//! Kiểm hợp lệ của `⊐`: khớp trait (E0311), chồng lấn (E0311), `Copy` cho kiểu lang/trường không Copy (E0313/E0311).
use crate::collect::{AdtKind, Env, FnSigInfo, ImplDef, Lang};
use crate::infer::InferCtx;
use crate::traits::{fresh_args, subst_with};
use crate::ty::Ty;

impl Env<'_> {
    pub(crate) fn validate_impls(&mut self) {
        let n = self.db.impls.len();
        for i in 0..n {
            self.validate_trait_impl(i);
        }
        for i in 0..n {
            for j in (i + 1)..n {
                self.check_overlap(i, j);
            }
        }
    }

    fn validate_trait_impl(&mut self, idx: usize) {
        let imp = self.db.impls[idx].clone();
        let Some(tr) = imp.trait_ref.clone() else {
            return;
        };
        let Some(td) = self.db.traits.get(&tr.trait_def).cloned() else {
            // db không nhất quán: không được bỏ qua kiểm impl (FAIL_OPEN_AUDIT R3) ⇒ lỗi nội bộ
            self.err(
                "E9901",
                imp.node,
                imp.span,
                "lỗi nội bộ: trait của `⊐ … for` không có trong cơ sở kiểu".to_string(),
            );
            return;
        };
        let node = imp.node;
        let tname = self.defs()[tr.trait_def.index()].path.clone();
        // Drop người dùng (ADR-C18 bước 1; trước đó chỉ lib/core — ADR-020 §9): luật hẹp để mọi drop có sẵn instance và gọi ngầm an toàn
        if self.db.lang.get(&Lang::Drop) == Some(&tr.trait_def) && !self.is_prelude(node.file()) {
            match self.user_drop_violation(&imp) {
                Some(why) => {
                    self.err("E0313", node, imp.span, format!("`⊐ Drop for …`: {why}"));
                    return;
                }
                None => {
                    if let (Ty::Adt(d, _), Some(m)) = (&imp.self_ty, imp.members.first()) {
                        self.db.user_drops.insert(*d, m.node);
                    }
                }
            }
        }
        // Send: trait tự động (M8), suy theo cấu trúc; không cho viết impl
        if td.lang == Some(Lang::Send) || td.lang == Some(Lang::Sync) {
            self.err(
                "E0313",
                node,
                imp.span,
                "`Send`/`Sync` là trait tự động suy theo cấu trúc kiểu: không được viết `⊐ Send for …`/`⊐ Sync for …`"
                    .to_string(),
            );
            return;
        }
        // Copy: không cho kiểu lang; mọi trường phải Copy
        if td.lang == Some(Lang::Copy) {
            self.validate_copy_impl(&imp, node);
        }
        for m in &imp.members {
            if !td.members.iter().any(|t| t.name == m.name) {
                self.err(
                    "E0311",
                    m.node,
                    m.span,
                    format!("`⊐ {tname}`: trait không có hàm `{}`", m.name),
                );
            }
        }
        for t in &td.members {
            match imp.members.iter().find(|m| m.name == t.name) {
                None => {
                    self.err(
                        "E0311",
                        node,
                        imp.span,
                        format!("`⊐ {tname}` thiếu hàm `{}` của trait", t.name),
                    );
                }
                Some(m) => {
                    if !self.sig_matches(t, m, td.node, &imp.self_ty) {
                        let sname = self.render_sig(m);
                        self.err(
                            "E0311",
                            m.node,
                            m.span,
                            format!("hàm `{}` trong `⊐ {tname}` không khớp chữ ký của trait (nhận được `{sname}`)", m.name),
                        );
                    } else if !self.regions_compatible(t.node, m.node) {
                        self.err(
                            "E0311",
                            m.node,
                            m.span,
                            format!("hàm `{}` trong `⊐ {tname}` hứa về lifetime ít hơn trait: region của kết quả phụ thuộc vào đầu vào khác với chữ ký của trait", m.name),
                        );
                    }
                }
            }
        }
    }

    /// Lý do `⊐ Drop for` của người dùng bị từ chối (ADR-C18), hoặc `None` nếu hợp lệ:
    /// (1) đích là struct/enum khai trong CÙNG tệp (luật mồ côi: không đổi hành vi drop của kiểu thư viện/kiểu lang);
    /// (2) không có tham số KIỂU ở kiểu lẫn impl (tham số vùng được): instance của `drop` luôn là một instance gốc, có sẵn ở bảng instance;
    /// (3) kiểu không `Copy` (chép bit rồi drop hai lần); (4) `drop` là `! ∅` và không có hợp đồng (gọi ngầm: không có điểm gọi để kiểm `⊳`).
    fn user_drop_violation(&self, imp: &ImplDef) -> Option<String> {
        let Ty::Adt(d, args) = &imp.self_ty else {
            return Some("chỉ cho struct/enum khai trong cùng tệp".to_string());
        };
        let Some(adt) = self.db.adts.get(d) else {
            // db không nhất quán: KHÔNG được coi là hợp lệ (fail-closed)
            return Some("lỗi nội bộ: ADT không có trong cơ sở kiểu".to_string());
        };
        let def = &self.defs()[d.index()];
        if adt.lang.is_some() || def.file != imp.node.file() {
            return Some(format!(
                "chỉ cho struct/enum khai trong CÙNG tệp với impl; `{}` thì không (ADR-C18)",
                def.path
            ));
        }
        if !adt.generics.is_empty() || !imp.generics.is_empty() || !args.is_empty() {
            return Some("kiểu có tham số KIỂU chưa được có Drop ở Giai đoạn 1 (tham số vùng `'a` thì được) — ADR-C18".to_string());
        }
        if self.db.is_copy_resolved(&imp.self_ty, &[]) {
            return Some("kiểu `Copy` không được có Drop (chép bit rồi drop hai lần)".to_string());
        }
        for m in &imp.members {
            if !m.effects.is_empty() {
                return Some(format!(
                    "`drop` phải là `! ∅` (được gọi ngầm ở mọi hàm, kể cả hàm `! ∅`); khai báo `{{{}}}`",
                    m.effects.join(", ")
                ));
            }
            if m.has_contracts {
                return Some("`drop` không được có hợp đồng `⊳`/`⊲` (được gọi ngầm: không có điểm gọi để kiểm)".to_string());
            }
        }
        None
    }

    /// Chữ ký REGION của impl không được hứa ít hơn trait (ADR-023 §4): với mỗi vị trí kết quả, tập đầu vào mà region của nó
    /// ràng buộc theo impl phải là tập con của tập theo trait (người gọi qua trait tin chữ ký trait). Không tính vị trí của `Self`.
    fn regions_compatible(
        &self,
        trait_member: gt_ast::NodeId,
        impl_member: gt_ast::NodeId,
    ) -> bool {
        use crate::region::RegionRef;
        let (Some(ts), Some(is)) = (
            self.region_sigs.get(&trait_member),
            self.region_sigs.get(&impl_member),
        ) else {
            // thiếu chữ ký region: KHÔNG coi là tương thích (fail-closed; collect luôn ghi cho mọi hàm/thành viên)
            return false;
        };
        let flat = |s: &crate::region::RegionSig| -> Vec<RegionRef> {
            s.inputs
                .iter()
                .zip(&s.input_self)
                .flat_map(|(rs, fs)| {
                    rs.iter()
                        .zip(fs)
                        .filter(|(_, f)| !**f)
                        .map(|(r, _)| r.clone())
                })
                .collect()
        };
        let outs = |s: &crate::region::RegionSig| -> Vec<RegionRef> {
            s.output
                .iter()
                .zip(&s.output_self)
                .filter(|(_, f)| !**f)
                .map(|(r, _)| r.clone())
                .collect()
        };
        let (ti, ii, to, io) = (flat(ts), flat(is), outs(ts), outs(is));
        if ti.len() != ii.len() || to.len() != io.len() {
            return false;
        }
        let rel = |ins: &[RegionRef], o: &RegionRef| -> Option<Vec<usize>> {
            if *o == RegionRef::Static {
                return None;
            }
            Some(
                ins.iter()
                    .enumerate()
                    .filter(|(_, r)| *r == o)
                    .map(|(i, _)| i)
                    .collect(),
            )
        };
        to.iter()
            .zip(&io)
            .all(|(a, b)| match (rel(&ti, a), rel(&ii, b)) {
                (_, None) => true,
                (None, Some(_)) => false,
                (Some(rt), Some(ri)) => ri.iter().all(|x| rt.contains(x)),
            })
    }

    fn render_sig(&self, m: &FnSigInfo) -> String {
        let ps: Vec<String> = m.params.iter().map(|p| p.render(self.defs())).collect();
        format!(
            "ƒ {}({}) → {}",
            m.name,
            ps.join(", "),
            m.ret.render(self.defs())
        )
    }

    /// Chữ ký thành viên impl khớp chữ ký trait (Self := kiểu impl; tham số generics của hàm thế theo thứ tự).
    fn sig_matches(
        &self,
        t: &FnSigInfo,
        m: &FnSigInfo,
        trait_node: gt_ast::NodeId,
        self_ty: &Ty,
    ) -> bool {
        if t.self_kind != m.self_kind
            || t.params.len() != m.params.len()
            || t.generics.len() != m.generics.len()
        {
            return false;
        }
        let map = |ty: &Ty| -> Ty {
            let s = Self::subst_self(ty, trait_node, self_ty);
            s.subst(&|p| {
                t.generics
                    .iter()
                    .position(|g| g.id == p)
                    .map(|i| Ty::Param(m.generics[i].id))
            })
        };
        t.params.iter().zip(&m.params).all(|(a, b)| map(a) == *b) && map(&t.ret) == m.ret
    }

    fn validate_copy_impl(&mut self, imp: &ImplDef, node: gt_ast::NodeId) {
        let Ty::Adt(d, self_args) = &imp.self_ty else {
            if !matches!(imp.self_ty, Ty::Err) {
                self.err(
                    "E0311",
                    node,
                    imp.span,
                    "`⊐ Copy for` chỉ áp dụng cho struct/enum do người dùng định nghĩa".to_string(),
                );
            }
            return;
        };
        let Some(adt) = self.db.adts.get(d).cloned() else {
            // db không nhất quán: KHÔNG được bỏ qua kiểm trường (FAIL_OPEN_AUDIT R3) ⇒ lỗi nội bộ
            self.err(
                "E9901",
                node,
                imp.span,
                "lỗi nội bộ: ADT của `⊐ Copy for` không có trong cơ sở kiểu".to_string(),
            );
            return;
        };
        if matches!(
            adt.lang,
            Some(
                Lang::Secret
                    | Lang::Cap
                    | Lang::Vec
                    | Lang::Owned
                    | Lang::Text
                    | Lang::Arena
                    | Lang::Mutex
                    | Lang::Atomic
                    | Lang::Cell
            )
        ) {
            let name = self.defs()[d.index()].path.clone();
            self.err(
                "E0313",
                node,
                imp.span,
                format!("không thể `⊐ Copy for {name}`: kiểu capability/secret không bao giờ Copy (D13, D28)"),
            );
            return;
        }
        let tys: Vec<Ty> = match &adt.kind {
            AdtKind::Struct(fs) => fs.iter().map(|f| f.ty.clone()).collect(),
            AdtKind::Enum(vs) => vs.iter().flat_map(|v| v.fields.clone()).collect(),
        };
        // Thay tham số của ADT bằng đối số thật của `self_ty` và kiểm Copy dưới các ràng buộc của chính impl.
        let scope = crate::traits::scope_bounds(&imp.generics.iter().collect::<Vec<_>>());
        let mut infer = InferCtx::new();
        for t in tys {
            let ft = subst_with(&t, &adt.generics, self_args);
            if !self.is_copy_ignoring_self(&mut infer, &ft, *d, &scope) {
                let s = ft.render(self.defs());
                self.err(
                    "E0311",
                    node,
                    imp.span,
                    format!("`⊐ Copy` không hợp lệ: trường kiểu `{s}` không phải Copy (tham số kiểu cần ràng buộc `T: Copy` trên impl)"),
                );
                return;
            }
        }
    }

    /// `is_copy` nhưng coi chính ADT đang xét là Copy (tránh vòng tự tham chiếu).
    fn is_copy_ignoring_self(
        &self,
        infer: &mut InferCtx,
        ty: &Ty,
        own: gt_resolve::DefId,
        scope: &[crate::traits::Bound],
    ) -> bool {
        if matches!(ty, Ty::Adt(d, _) if *d == own) {
            return true;
        }
        self.db.is_copy(infer, ty, scope, 0)
    }

    /// Hai impl chồng lấn: cùng trait mà đầu đề hợp nhất được; hoặc cùng kiểu (inherent) và trùng tên thành viên.
    fn check_overlap(&mut self, i: usize, j: usize) {
        let (a, b) = (self.db.impls[i].clone(), self.db.impls[j].clone());
        let same_trait = match (&a.trait_ref, &b.trait_ref) {
            (Some(x), Some(y)) => x.trait_def == y.trait_def,
            (None, None) => true,
            _ => false,
        };
        if !same_trait {
            return;
        }
        let mut infer = InferCtx::new();
        let (ga, gb) = (
            fresh_args(&mut infer, &a.generics),
            fresh_args(&mut infer, &b.generics),
        );
        let (ta, tb) = (
            subst_with(&a.self_ty, &a.generics, &ga),
            subst_with(&b.self_ty, &b.generics, &gb),
        );
        if !infer.unify(&ta, &tb) {
            return;
        }
        if a.trait_ref.is_none() {
            let shared: Vec<&str> = a
                .members
                .iter()
                .filter(|m| b.members.iter().any(|n| n.name == m.name))
                .map(|m| m.name.as_str())
                .collect();
            if shared.is_empty() {
                return;
            }
            self.err(
                "E0311",
                b.node,
                b.span,
                format!(
                    "hai `⊐` cho cùng một kiểu trùng hàm `{}`",
                    shared.join(", ")
                ),
            );
        } else {
            let tn = self.defs()[a.trait_ref.as_ref().map_or(0, |t| t.trait_def.index())]
                .path
                .clone();
            self.err(
                "E0311",
                b.node,
                b.span,
                format!("hai `⊐ {tn} for …` chồng lấn: cùng một kiểu có thể khớp cả hai"),
            );
        }
    }
}
