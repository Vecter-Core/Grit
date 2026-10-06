//! Kiểm hợp lệ của `⊐`: khớp trait (E0311), chồng lấn (E0311), `Copy` cho kiểu lang/trường không Copy (E0313/E0311).
use crate::collect::{AdtKind, Env, FnSigInfo, ImplDef, Lang};
use crate::infer::InferCtx;
use crate::traits::{fresh_args, subst_with};
use crate::ty::Ty;

impl Env<'_> {
    pub(crate) fn validate_impls(&mut self) {
        let n = self.impls.len();
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
        let imp = self.impls[idx].clone();
        let Some(tr) = imp.trait_ref.clone() else {
            return;
        };
        let Some(td) = self.traits.get(&tr.trait_def).cloned() else {
            return;
        };
        let node = imp.node;
        let tname = self.defs()[tr.trait_def.index()].path.clone();
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
                    }
                }
            }
        }
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
        let Ty::Adt(d, _) = &imp.self_ty else {
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
        let Some(adt) = self.adts.get(d).cloned() else {
            return;
        };
        if matches!(adt.lang, Some(Lang::Secret | Lang::Cap)) {
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
        let mut infer = InferCtx::new();
        let gens: Vec<Ty> = fresh_args(&mut infer, &adt.generics);
        for t in tys {
            let ft = subst_with(&t, &adt.generics, &gens);
            let ft = match &ft {
                Ty::Param(_) => continue, // trường kiểu tham số: ràng buộc `T: Copy` do impl tự khai báo
                _ => ft,
            };
            if !self.is_copy_ignoring_self(&mut infer, &ft, *d) {
                let s = ft.render(self.defs());
                self.err(
                    "E0311",
                    node,
                    imp.span,
                    format!("`⊐ Copy` không hợp lệ: trường kiểu `{s}` không phải Copy"),
                );
                return;
            }
        }
    }

    /// `is_copy` nhưng coi chính ADT đang xét là Copy (tránh vòng tự tham chiếu).
    fn is_copy_ignoring_self(&self, infer: &mut InferCtx, ty: &Ty, own: gt_resolve::DefId) -> bool {
        if matches!(ty, Ty::Adt(d, _) if *d == own) {
            return true;
        }
        self.is_copy(infer, ty, &[], 0)
    }

    /// Hai impl chồng lấn: cùng trait mà đầu đề hợp nhất được; hoặc cùng kiểu (inherent) và trùng tên thành viên.
    fn check_overlap(&mut self, i: usize, j: usize) {
        let (a, b) = (self.impls[i].clone(), self.impls[j].clone());
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
