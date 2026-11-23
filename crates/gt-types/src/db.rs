//! `TypeDb`: các bảng kết quả của gt-types (ADT, trait, chữ ký, impl, lang item) — chỉ đọc sau M4, dùng chung với các
//! pass sau (bước hạ MIR, M5; ADR-020 §1). Truy vấn `is_copy`/`needs_drop`/`satisfies_bound` nằm ở đây để Env và MIR dùng chung.
use crate::collect::{AdtDef, AdtKind, FnSigInfo, ImplDef, Lang, TraitDef};
use crate::infer::InferCtx;
use crate::traits::{Bound, fresh_args, subst_with};
use crate::ty::Ty;
use gt_ast::NodeId;
use gt_resolve::DefId;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct TypeDb {
    pub adts: BTreeMap<DefId, AdtDef>,
    pub traits: BTreeMap<DefId, TraitDef>,
    /// Chữ ký hàm (không thuộc impl/trait) + module chứa.
    pub fns: BTreeMap<DefId, (FnSigInfo, DefId)>,
    pub consts: BTreeMap<DefId, Ty>,
    pub impls: Vec<ImplDef>,
    pub lang: BTreeMap<Lang, DefId>,
    /// `Drop` NGƯỜI DÙNG đã qua kiểm của M4 (ADR-C18): ADT → thành viên `drop` của impl (thân có MIR, instance không đối số kiểu).
    /// Chỉ các mục ở đây được drop glue GỌI; `⊐ Drop for` của prelude (`Buf`, `Guard`: thân rỗng, không có MIR) không có mặt.
    pub user_drops: BTreeMap<DefId, NodeId>,
    /// Chương trình có tạo `Owned<_>` không (một biểu thức có kiểu nhắc tới `Owned`; giá trị Owned CHỈ sinh từ `Owned::new` của prelude vì
    /// literal bị cấm). Cùng `user_drops` quyết định có cần drop glue: không có cả hai ⇒ mọi glue rỗng (ADR-C18).
    pub owned_used: bool,
    /// Chương trình có biểu thức mang kiểu nhắc tới `Secret` (ADR-P2-03): bật glue xoá + xoá-sau-move.
    pub secret_used: bool,
}

/// Các trường cần glue của một struct/tuple/biến thể: (chỉ số trường, kiểu trường, glue), theo thứ tự khai báo.
pub type GlueFields = Vec<(u32, Ty, Glue)>;

/// Drop glue của một kiểu CỤ THỂ (ADR-C18): việc cần làm khi một giá trị kiểu đó hết đời. Một nguồn sự thật cho thông dịch, codegen,
/// M6 (drop có ý nghĩa là một lần dùng) và M8 (cạnh gọi ngầm tới `drop`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Glue {
    /// không có gì để chạy
    Nothing,
    /// gọi `drop` người dùng (thành viên impl `NodeId`, instance không đối số) trên chính chỗ, RỒI glue của các trường
    Call(NodeId, Box<Glue>),
    /// `Owned<T>` (ADR-C18 bước 2): glue của `T` tại khối heap mà chỗ trỏ tới, RỒI trả khối (`gritrt_heap_free(ptr, size_of T, align_of T)`)
    Free(Ty, Box<Glue>),
    /// `Secret<T>` (ADR-P2-03): glue của `T` (trường `v`) RỒI xoá toàn bộ byte của `Secret<T>` tại chỗ (`gritrt_secret_wipe`)
    Wipe(Box<Glue>),
    /// struct/tuple: các trường có glue ≠ `Nothing`, theo thứ tự khai báo: (chỉ số trường, kiểu trường, glue)
    Fields(GlueFields),
    /// enum: chỉ biến thể đang có chạy; (chỉ số biến thể, các trường có glue của biến thể đó)
    Variants(Vec<(u32, GlueFields)>),
    /// mảng `[T; n]`: glue của từng phần tử theo chỉ số tăng
    Array(u64, Ty, Box<Glue>),
}

impl Glue {
    #[must_use]
    pub fn is_nothing(&self) -> bool {
        matches!(self, Glue::Nothing)
    }

    /// Mọi `drop` người dùng mà glue này có thể gọi (cho đồ thị gọi của M8 và bảng instance).
    pub fn calls(&self, out: &mut Vec<NodeId>) {
        match self {
            Glue::Nothing => {}
            Glue::Call(n, g) => {
                if !out.contains(n) {
                    out.push(*n);
                }
                g.calls(out);
            }
            Glue::Free(_, g) | Glue::Wipe(g) => g.calls(out),
            Glue::Fields(fs) => fs.iter().for_each(|(_, _, g)| g.calls(out)),
            Glue::Variants(vs) => vs
                .iter()
                .for_each(|(_, fs)| fs.iter().for_each(|(_, _, g)| g.calls(out))),
            Glue::Array(_, _, g) => g.calls(out),
        }
    }
}

impl TypeDb {
    /// Kiểu có thể chứa MUTABILITY BÊN TRONG (lang `cell`, và sau này Mutex/Atomic*) không? Dùng để loại vị từ khỏi `pure_preds` của M7:
    /// kết quả vị từ trên `&Cell` đổi được sau `cell.set(..)` nên không được coi là sự kiện ổn định. `generics` = tham số kiểu trong tầm
    /// (của hàm và của impl). Tham số kiểu KHÔNG có ràng buộc trait không bị quan sát được nội dung (tính tham số: thân không gọi được gì
    /// trên nó) nên không tính; tham số CÓ ràng buộc (một impl của trait có thể đọc ô) và biến suy luận tính là CÓ THỂ (fail-closed).
    #[must_use]
    pub fn may_have_interior_mut(&self, ty: &Ty, generics: &[&crate::collect::GParam]) -> bool {
        self.imut(ty, Some(generics), &mut Vec::new())
    }

    /// `params`: `Some(generics)` = tra ràng buộc; `None` = đang duyệt trường của ADT (tham số của ADT đã được kiểm qua đối số).
    fn imut(
        &self,
        ty: &Ty,
        params: Option<&[&crate::collect::GParam]>,
        seen: &mut Vec<DefId>,
    ) -> bool {
        match ty {
            Ty::Infer(_) => true,
            Ty::Param(p) => params.is_some_and(|gs| {
                gs.iter()
                    .find(|g| g.id == *p)
                    .is_none_or(|g| !g.bounds.is_empty())
            }),
            Ty::Tuple(v) => v.iter().any(|t| self.imut(t, params, seen)),
            Ty::Array(t, _) | Ty::Slice(t) | Ty::Ref(_, t) | Ty::RawPtr(_, t) => {
                self.imut(t, params, seen)
            }
            Ty::Adt(d, args) => {
                // ADT không có trong db (db không nhất quán): KHÔNG được coi là "không có mutability bên trong" — bảo thủ `true` (vị từ không còn được mô hình hoá thuần)
                let Some(adt) = self.adts.get(d) else {
                    return true;
                };
                if adt.lang.is_some_and(Lang::interior_mutable) {
                    return true;
                }
                if args.iter().any(|t| self.imut(t, params, seen)) {
                    return true;
                }
                if seen.contains(d) {
                    return false;
                }
                seen.push(*d);
                let fields: Vec<&Ty> = match &adt.kind {
                    AdtKind::Struct(fs) => fs.iter().map(|f| &f.ty).collect(),
                    AdtKind::Enum(vs) => vs.iter().flat_map(|v| v.fields.iter()).collect(),
                };
                fields.into_iter().any(|t| self.imut(t, None, seen))
            }
            _ => false,
        }
    }

    /// Tìm impl của `trait_def` cho `ty` (đã giải); `scope` là các ràng buộc có hiệu lực cho tham số kiểu trong `ty`
    /// (FB-C10: ràng buộc inline của impl được kiểm TRONG phạm vi này, không phải phạm vi rỗng). Trả (chỉ số impl, tham số impl) nếu có đúng một.
    pub(crate) fn find_impl(
        &self,
        infer: &mut InferCtx,
        trait_def: DefId,
        ty: &Ty,
        scope: &[Bound],
        depth: u32,
    ) -> Option<(usize, Vec<Ty>)> {
        if depth > 8 {
            return None;
        }
        let mut found = Vec::new();
        for (i, imp) in self.impls.iter().enumerate() {
            if imp.trait_ref.as_ref().map(|t| t.trait_def) != Some(trait_def) {
                continue;
            }
            let ok = infer.probe(|inf| {
                let args = fresh_args(inf, &imp.generics);
                let st = subst_with(&imp.self_ty, &imp.generics, &args);
                if !inf.unify(&st, ty) {
                    return false;
                }
                // ràng buộc inline của impl phải thoả
                imp.generics.iter().zip(&args).all(|(g, a)| {
                    let a = inf.resolve(a);
                    g.bounds
                        .iter()
                        .all(|b| self.satisfies_bound(inf, &a, b.trait_def, scope, depth + 1))
                })
            });
            if ok {
                found.push(i);
            }
        }
        if found.len() != 1 {
            return None;
        }
        let imp = &self.impls[found[0]];
        let args = fresh_args(infer, &imp.generics);
        let st = subst_with(&imp.self_ty, &imp.generics, &args);
        infer.unify(&st, ty);
        Some((found[0], args))
    }

    /// MỘT NGUỒN SỰ THẬT chọn impl (ADR-032 §2): impl duy nhất của `trait_def` cho kiểu CỤ THỂ `ty` (không tham số/biến suy diễn),
    /// trả (chỉ số impl, đối số của impl đã giải). `None` nếu không có hoặc không duy nhất. Mono, codegen và thông dịch dùng hàm này;
    /// KHÔNG nơi nào tự cài khớp kiểu.
    #[must_use]
    pub fn select_impl(&self, trait_def: DefId, ty: &Ty) -> Option<(usize, Vec<Ty>)> {
        if select_impl_fault() || ty.has_param() || ty.has_infer() {
            return None;
        }
        if select_impl_first() {
            // CHỈ ĐỂ KIỂM ĐỘ NHẠY CỦA BỘ SINH (ADR-034): chọn impl ĐẦU của trait, bỏ qua kiểu
            let i = self
                .impls
                .iter()
                .position(|m| m.trait_ref.as_ref().map(|t| t.trait_def) == Some(trait_def))?;
            let mut inf = InferCtx::new();
            let args = fresh_args(&mut inf, &self.impls[i].generics);
            return Some((i, args.iter().map(|a| inf.resolve(a)).collect()));
        }
        let mut inf = InferCtx::new();
        let (i, args) = self.find_impl(&mut inf, trait_def, ty, &[], 0)?;
        Some((i, args.iter().map(|a| inf.resolve(a)).collect()))
    }

    /// Phân giải `trait_def::member` trên `self_ty` cụ thể thành (NodeId thành viên của impl, đối số instance = đối số impl ++ `method_args`).
    #[must_use]
    pub fn resolve_trait_method(
        &self,
        trait_def: DefId,
        member: usize,
        self_ty: &Ty,
        method_args: &[Ty],
    ) -> Option<(NodeId, Vec<Ty>)> {
        let name = &self.traits.get(&trait_def)?.members.get(member)?.name;
        let (ii, mut all) = self.select_impl(trait_def, self_ty)?;
        let m = self.impls[ii].members.iter().find(|m| &m.name == name)?;
        all.extend(method_args.iter().cloned());
        Some((m.node, all))
    }

    /// Thứ tự tham số kiểu của instance của hàm/thành viên `node`: generics của impl (nếu có) rồi của hàm.
    #[must_use]
    pub fn fn_params(&self, node: NodeId) -> Vec<crate::ty::ParamId> {
        for imp in &self.impls {
            for m in &imp.members {
                if m.node == node {
                    return imp
                        .generics
                        .iter()
                        .chain(m.generics.iter())
                        .map(|g| g.id)
                        .collect();
                }
            }
        }
        for (sig, _) in self.fns.values() {
            if sig.node == node {
                return sig.generics.iter().map(|g| g.id).collect();
            }
        }
        Vec::new()
    }

    /// `ty: trait_def` có thoả không? `scope` là các ràng buộc có hiệu lực (cho tham số kiểu).
    pub(crate) fn satisfies_bound(
        &self,
        infer: &mut InferCtx,
        ty: &Ty,
        trait_def: DefId,
        scope: &[Bound],
        depth: u32,
    ) -> bool {
        let ty = infer.resolve(ty);
        match &ty {
            Ty::Err => true,
            Ty::Infer(_) => true, // chưa biết: hoãn (bộ giải nghĩa vụ chạy lại sau khi mặc định hoá)
            Ty::Param(_) => scope
                .iter()
                .any(|b| b.trait_def == trait_def && infer.resolve(&b.ty) == ty),
            _ => {
                if self.lang.get(&Lang::Copy) == Some(&trait_def) {
                    return self.is_copy(infer, &ty, scope, depth);
                }
                self.find_impl(infer, trait_def, &ty, scope, depth)
                    .is_some()
            }
        }
    }

    /// `is_copy` cho M6 (ADR-018 §2): nguyên thuỷ số/bool/char, `&T`, con trỏ thô, hàm, `!`; tuple/mảng toàn phần tử Copy;
    /// ADT chỉ khi có `⊐ Copy for` hợp lệ; `&!T`, `Secret<_>`, `Cap<_>` không bao giờ.
    pub(crate) fn is_copy(
        &self,
        infer: &mut InferCtx,
        ty: &Ty,
        scope: &[Bound],
        depth: u32,
    ) -> bool {
        let ty = infer.resolve(ty);
        match &ty {
            Ty::Int(_)
            | Ty::Float(_)
            | Ty::Bool
            | Ty::Char
            | Ty::Never
            | Ty::Err
            | Ty::Fn(..)
            | Ty::RawPtr(..) => true,
            Ty::Ref(false, _) => true,
            Ty::Ref(true, _) | Ty::Str | Ty::Slice(_) => false,
            Ty::Tuple(v) => v.iter().all(|t| self.is_copy(infer, t, scope, depth + 1)),
            Ty::Array(t, _) => self.is_copy(infer, t, scope, depth + 1),
            Ty::Infer(_) => true,
            Ty::Param(_) => {
                let Some(&copy) = self.lang.get(&Lang::Copy) else {
                    return false;
                };
                scope
                    .iter()
                    .any(|b| b.trait_def == copy && infer.resolve(&b.ty) == ty)
            }
            Ty::Adt(d, _) => {
                if matches!(
                    self.adts.get(d).and_then(|a| a.lang),
                    Some(Lang::Secret | Lang::Cap)
                ) {
                    return false;
                }
                let Some(&copy) = self.lang.get(&Lang::Copy) else {
                    return false;
                };
                self.find_impl(infer, copy, &ty, scope, depth + 1).is_some()
            }
        }
    }

    /// Giá trị kiểu này có cần `Drop` tường minh không (ADR-020 §9, Q26): ADT có `⊐ Drop for` (chỉ lib/core), `Arena` (luôn),
    /// tuple/mảng/ADT chứa thành phần cần drop; tham số kiểu: coi là cần (MIR đa hình). `Copy`, tham chiếu, nguyên thuỷ: không.
    /// ADT không có trong db (db không nhất quán) ⇒ coi là CẦN drop (bảo thủ, không bao giờ "không cần"); dùng [`TypeDb::needs_drop_checked`] khi cần BÁO lỗi nội bộ.
    #[must_use]
    pub fn needs_drop(&self, ty: &Ty) -> bool {
        self.needs_drop_inner(ty, 0, &mut None)
    }

    /// Như [`TypeDb::needs_drop`] nhưng `Err` (lý do) nếu gặp ADT không có trong db — người gọi báo ICE E9901 (FAIL_OPEN_AUDIT).
    pub fn needs_drop_checked(&self, ty: &Ty) -> Result<bool, String> {
        let mut fault = None;
        let r = self.needs_drop_inner(ty, 0, &mut fault);
        match fault {
            Some(f) => Err(f),
            None => Ok(r),
        }
    }

    fn needs_drop_inner(&self, ty: &Ty, depth: u32, fault: &mut Option<String>) -> bool {
        if depth > 32 {
            return true; // kiểu đệ quy sâu bất thường: an toàn là coi như cần drop
        }
        match ty {
            Ty::Int(_)
            | Ty::Float(_)
            | Ty::Bool
            | Ty::Char
            | Ty::Str
            | Ty::Never
            | Ty::Err
            | Ty::Fn(..)
            | Ty::RawPtr(..)
            | Ty::Ref(..)
            | Ty::Infer(_) => false,
            Ty::Param(_) => true,
            Ty::Tuple(v) => v.iter().any(|t| self.needs_drop_inner(t, depth + 1, fault)),
            Ty::Array(t, n) => *n > 0 && self.needs_drop_inner(t, depth + 1, fault),
            Ty::Slice(t) => self.needs_drop_inner(t, depth + 1, fault),
            Ty::Adt(d, args) => {
                let Some(adt) = self.adts.get(d) else {
                    fault.get_or_insert_with(|| {
                        format!("ADT {d:?} không có trong cơ sở kiểu (needs_drop)")
                    });
                    return true;
                };
                if adt.lang == Some(Lang::Arena) {
                    return true;
                }
                if self.has_drop_impl(ty) {
                    return true;
                }
                match &adt.kind {
                    AdtKind::Struct(fs) => fs.iter().any(|f| {
                        self.needs_drop_inner(
                            &subst_with(&f.ty, &adt.generics, args),
                            depth + 1,
                            fault,
                        )
                    }),
                    AdtKind::Enum(vs) => vs.iter().any(|v| {
                        v.fields.iter().any(|f| {
                            self.needs_drop_inner(
                                &subst_with(f, &adt.generics, args),
                                depth + 1,
                                fault,
                            )
                        })
                    }),
                }
            }
        }
    }

    /// Drop glue của kiểu `ty` (ADR-C18). Kiểu phải CỤ THỂ (không tham số/biến suy diễn) — người gọi thế tham số của instance trước.
    /// `Err` (lý do): kiểu còn tham số, ADT không có trong db, kiểu quá sâu, hoặc kiểu lang ĐỤC (Vec/Text/Arena/Mutex/Atomic/Cell/Secret/Cap)
    /// chứa giá trị có Drop người dùng — biểu diễn của chúng không phải trường thường nên glue không đi vào được (fail-closed).
    pub fn drop_glue(&self, ty: &Ty) -> Result<Glue, String> {
        if !self.glue_possible() {
            // glue chỉ làm việc khi có `drop` người dùng hoặc `Owned`: không có cả hai ⇒ mọi glue rỗng (chương trình cũ không đổi gì)
            return Ok(Glue::Nothing);
        }
        self.glue_inner(ty, 0)
    }

    /// Glue của kiểu có thể CÒN tham số: tham số kiểu coi là "có thể có glue" (`true`). Dùng ở MIR đa hình (M6: drop là một lần dùng).
    #[must_use]
    pub fn may_have_glue(&self, ty: &Ty) -> bool {
        if !self.glue_possible() {
            return false;
        }
        if ty.has_param() || ty.has_infer() {
            return self.needs_drop(ty);
        }
        !matches!(self.drop_glue(ty), Ok(Glue::Nothing))
    }

    fn glue_fields(&self, tys: impl Iterator<Item = Ty>, depth: u32) -> Result<GlueFields, String> {
        let mut out = Vec::new();
        for (i, t) in tys.enumerate() {
            let g = self.glue_inner(&t, depth + 1)?;
            if !g.is_nothing() {
                out.push((u32::try_from(i).unwrap_or(u32::MAX), t, g));
            }
        }
        Ok(out)
    }

    fn glue_inner(&self, ty: &Ty, depth: u32) -> Result<Glue, String> {
        if depth > 1024 {
            // độ sâu bất thường ⇒ lỗi (không bao giờ coi là "không có glue")
            return Err("kiểu lồng quá sâu cho drop glue".to_string());
        }
        match ty {
            Ty::Int(_)
            | Ty::Float(_)
            | Ty::Bool
            | Ty::Char
            | Ty::Str
            | Ty::Never
            | Ty::Err
            | Ty::Fn(..)
            | Ty::RawPtr(..)
            | Ty::Ref(..)
            | Ty::Slice(_) => Ok(Glue::Nothing),
            Ty::Param(_) | Ty::Infer(_) => Err(format!("drop glue của kiểu chưa cụ thể `{ty:?}`")),
            Ty::Tuple(v) => {
                let fs = self.glue_fields(v.iter().cloned(), depth)?;
                Ok(if fs.is_empty() {
                    Glue::Nothing
                } else {
                    Glue::Fields(fs)
                })
            }
            Ty::Array(t, n) => {
                let g = self.glue_inner(t, depth + 1)?;
                Ok(if g.is_nothing() || *n == 0 {
                    Glue::Nothing
                } else {
                    Glue::Array(*n, (**t).clone(), Box::new(g))
                })
            }
            Ty::Adt(d, args) => {
                let Some(adt) = self.adts.get(d) else {
                    return Err(format!("ADT {d:?} không có trong cơ sở kiểu (drop_glue)"));
                };
                if adt.lang == Some(Lang::Owned) {
                    // M4 cấm đệ quy qua `Owned` (E0301) ⇒ đệ quy ở đây dừng
                    let Some(t) = args.first() else {
                        return Err("`Owned` thiếu đối số kiểu".to_string());
                    };
                    let inner = self.glue_inner(t, depth + 1)?;
                    return Ok(Glue::Free(t.clone(), Box::new(inner)));
                }
                if adt.lang == Some(Lang::Secret) {
                    let Some(t) = args.first() else {
                        return Err("`Secret` thiếu đối số kiểu".to_string());
                    };
                    return Ok(Glue::Wipe(Box::new(self.glue_inner(t, depth + 1)?)));
                }
                if let Some(l) = adt.lang
                    && matches!(
                        l,
                        Lang::Vec
                            | Lang::Text
                            | Lang::Arena
                            | Lang::Mutex
                            | Lang::Atomic
                            | Lang::Cell
                            | Lang::Cap
                    )
                {
                    for a in args {
                        if !self.glue_inner(a, depth + 1)?.is_nothing() {
                            return Err(format!(
                                "kiểu lang đục `{l:?}` chứa giá trị có Drop người dùng: chưa hỗ trợ (ADR-C18)"
                            ));
                        }
                    }
                    return Ok(Glue::Nothing);
                }
                let inner = match &adt.kind {
                    AdtKind::Struct(fs) => {
                        let fs = self.glue_fields(
                            fs.iter().map(|f| subst_with(&f.ty, &adt.generics, args)),
                            depth,
                        )?;
                        if fs.is_empty() {
                            Glue::Nothing
                        } else {
                            Glue::Fields(fs)
                        }
                    }
                    AdtKind::Enum(vs) => {
                        let mut out = Vec::new();
                        for (vi, v) in vs.iter().enumerate() {
                            let fs = self.glue_fields(
                                v.fields.iter().map(|f| subst_with(f, &adt.generics, args)),
                                depth,
                            )?;
                            if !fs.is_empty() {
                                out.push((u32::try_from(vi).unwrap_or(u32::MAX), fs));
                            }
                        }
                        if out.is_empty() {
                            Glue::Nothing
                        } else {
                            Glue::Variants(out)
                        }
                    }
                };
                Ok(match self.user_drops.get(d) {
                    Some(n) => Glue::Call(*n, Box::new(inner)),
                    None => inner,
                })
            }
        }
    }

    /// Có nguồn drop glue nào trong chương trình không (`drop` người dùng hoặc `Owned`). Không ⇒ mọi glue rỗng.
    #[must_use]
    pub fn glue_possible(&self) -> bool {
        !self.user_drops.is_empty() || self.owned_used || self.secret_used
    }

    /// Kiểu có THỂ chứa `Secret` (ADR-P2-03: chỗ như vậy bị xoá sau khi move ra). Tham số kiểu ⇒ có thể (nếu chương trình dùng Secret).
    #[must_use]
    pub fn may_contain_secret(&self, ty: &Ty) -> bool {
        if !self.secret_used {
            return false;
        }
        self.contains_lang(ty, Lang::Secret, true, 0)
    }

    /// `ty` (đã CỤ THỂ) có chứa `Secret` không — để backend quyết định xoá theo kiểu đã thế.
    #[must_use]
    pub fn contains_secret(&self, ty: &Ty) -> bool {
        self.contains_lang(ty, Lang::Secret, false, 0)
    }

    fn contains_lang(&self, ty: &Ty, l: Lang, params: bool, depth: u32) -> bool {
        if depth > 64 {
            return true;
        }
        match ty {
            Ty::Param(_) | Ty::Infer(_) => params,
            Ty::Tuple(v) => v
                .iter()
                .any(|t| self.contains_lang(t, l, params, depth + 1)),
            Ty::Array(t, n) => *n > 0 && self.contains_lang(t, l, params, depth + 1),
            Ty::Adt(d, args) => {
                let Some(adt) = self.adts.get(d) else {
                    return true;
                };
                if adt.lang == Some(l) {
                    return true;
                }
                if adt.lang == Some(Lang::Owned) {
                    // nội dung nằm ở khối heap, không ở chỗ này: xoá chỗ không xoá được khối (Owned<Secret> tự xoá khi drop qua glue)
                    return false;
                }
                let fields: Vec<Ty> = match &adt.kind {
                    AdtKind::Struct(fs) => fs
                        .iter()
                        .map(|f| subst_with(&f.ty, &adt.generics, args))
                        .collect(),
                    AdtKind::Enum(vs) => vs
                        .iter()
                        .flat_map(|v| v.fields.iter().map(|f| subst_with(f, &adt.generics, args)))
                        .collect(),
                };
                fields
                    .iter()
                    .any(|t| self.contains_lang(t, l, params, depth + 1))
            }
            _ => false,
        }
    }

    /// Kiểu (đã cụ thể hoặc còn tham số) có CHỨA theo cấu trúc một nguồn drop glue không: ADT có `drop` người dùng đã đăng ký, hoặc `Owned`.
    /// Không phụ thuộc `owned_used` (dùng được trong lúc M4 kiểm thân). Tham số kiểu ⇒ `false` (chưa biết; xem KNOWN_LIMITATIONS).
    #[must_use]
    pub fn holds_glue_source(&self, ty: &Ty) -> bool {
        self.holds_glue_inner(ty, 0)
    }

    fn holds_glue_inner(&self, ty: &Ty, depth: u32) -> bool {
        if depth > 64 {
            return true;
        }
        match ty {
            Ty::Tuple(v) => v.iter().any(|t| self.holds_glue_inner(t, depth + 1)),
            Ty::Array(t, n) => *n > 0 && self.holds_glue_inner(t, depth + 1),
            Ty::Adt(d, args) => {
                let Some(adt) = self.adts.get(d) else {
                    return true;
                };
                if matches!(adt.lang, Some(Lang::Owned | Lang::Secret))
                    || self.user_drops.contains_key(d)
                {
                    return true;
                }
                if args.iter().any(|a| self.holds_glue_inner(a, depth + 1)) {
                    return true;
                }
                let fields: Vec<Ty> = match &adt.kind {
                    AdtKind::Struct(fs) => fs
                        .iter()
                        .map(|f| subst_with(&f.ty, &adt.generics, args))
                        .collect(),
                    AdtKind::Enum(vs) => vs
                        .iter()
                        .flat_map(|v| v.fields.iter().map(|f| subst_with(f, &adt.generics, args)))
                        .collect(),
                };
                fields.iter().any(|t| self.holds_glue_inner(t, depth + 1))
            }
            _ => false,
        }
    }

    /// Có `⊐ Drop for` khớp kiểu này không (impl của lang item `drop`).
    #[must_use]
    pub fn has_drop_impl(&self, ty: &Ty) -> bool {
        let Some(&drop) = self.lang.get(&Lang::Drop) else {
            return false;
        };
        let mut inf = InferCtx::new();
        self.find_impl(&mut inf, drop, ty, &[], 0).is_some()
    }

    /// `is_copy` với các ràng buộc `scope` của hàm đang xét (kiểu đã giải hết — dùng sau M4).
    #[must_use]
    pub fn is_copy_resolved(&self, ty: &Ty, scope: &[Bound]) -> bool {
        let mut inf = InferCtx::new();
        self.is_copy(&mut inf, ty, scope, 0)
    }

    /// Chữ ký của hàm/thành viên theo NodeId (hàm tự do, thành viên impl, thành viên trait).
    #[must_use]
    pub fn sig_by_node(&self, node: NodeId) -> Option<&FnSigInfo> {
        if let Some((s, _)) = self.fns.values().find(|(s, _)| s.node == node) {
            return Some(s);
        }
        for imp in &self.impls {
            if let Some(m) = imp.members.iter().find(|m| m.node == node) {
                return Some(m);
            }
        }
        for t in self.traits.values() {
            if let Some(m) = t.members.iter().find(|m| m.node == node) {
                return Some(m);
            }
        }
        None
    }
}

thread_local! {
    static SELECT_FAULT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static SELECT_FIRST: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Chốt kiểm thử độ nhạy (ADR-034): `select_impl` luôn chọn impl ĐẦU của trait trên luồng hiện tại. KHÔNG dùng ngoài test.
pub fn set_select_impl_first_for_test(on: bool) {
    SELECT_FIRST.with(|c| c.set(on));
}

fn select_impl_first() -> bool {
    SELECT_FIRST.with(std::cell::Cell::get)
}

/// Chốt kiểm thử (ADR-032 phụ lục A, 5b): làm `select_impl` luôn trả `None` trên luồng hiện tại để chứng minh đường ICE E9901 của
/// codegen/thông dịch. KHÔNG dùng ngoài test.
pub fn set_select_impl_fault_for_test(on: bool) {
    SELECT_FAULT.with(|c| c.set(on));
}

fn select_impl_fault() -> bool {
    SELECT_FAULT.with(std::cell::Cell::get)
}

#[cfg(test)]
mod drop_tests {
    use crate::collect::Lang;
    use crate::testkit::with_env;
    use crate::ty::Ty;
    use gt_resolve::DefId;

    /// FAIL_OPEN_AUDIT: ADT không có trong db KHÔNG được coi là "không cần drop" (nuôi E0318 và các quyết định drop của MIR): phải bảo thủ.
    #[test]
    fn unknown_adt_is_never_assumed_not_to_need_drop() {
        with_env("ƒ f() → u32 ! ∅ { 1 }", |env| {
            let unknown = Ty::Adt(DefId::from_index(1_000_000), Vec::new());
            assert!(
                env.db.needs_drop(&unknown),
                "ADT lạ bị coi là không cần drop"
            );
            // lồng trong tuple/mảng cũng vậy
            assert!(
                env.db
                    .needs_drop(&Ty::Tuple(vec![Ty::Bool, unknown.clone()]))
            );
            // API có kiểm: báo lỗi nội bộ (người gọi ICE E9901), kể cả khi ADT lạ nằm lồng bên trong
            assert!(env.db.needs_drop_checked(&unknown).is_err());
            assert!(
                env.db
                    .needs_drop_checked(&Ty::Tuple(vec![Ty::Bool, unknown.clone()]))
                    .is_err()
            );
            assert_eq!(env.db.needs_drop_checked(&Ty::Bool), Ok(false));
            // R3 (FAIL_OPEN_AUDIT db.rs `imut`): ADT không có trong db KHÔNG được coi là "không có mutability bên trong" (M7 sẽ coi vị từ là thuần)
            assert!(
                env.db.may_have_interior_mut(&unknown, &[]),
                "ADT lạ bị coi là không có mutability bên trong"
            );
            let arena = *env.db.lang.get(&Lang::Arena).expect("Arena");
            assert!(
                env.db.needs_drop(&Ty::Adt(arena, Vec::new())),
                "đối chứng: Arena cần drop"
            );
        });
    }
}

#[cfg(test)]
mod imut_tests {
    use crate::testkit::with_env;
    use crate::ty::Ty;
    use gt_resolve::DefId;

    /// FB-C24 (đọc mã đối kháng nhóm M7/M8): ADT không có trong db KHÔNG được coi là "không có mutability bên trong" (vị từ trên nó sẽ bị coi là sự kiện ổn định).
    #[test]
    fn unknown_adt_is_assumed_to_have_interior_mutability_fb_c24() {
        with_env("ƒ f() → u32 ! ∅ { 1 }", |env| {
            let unknown = Ty::Adt(DefId::from_index(1_000_000), Vec::new());
            assert!(env.db.may_have_interior_mut(&unknown, &[]));
            assert!(
                env.db
                    .may_have_interior_mut(&Ty::Tuple(vec![Ty::Bool, unknown.clone()]), &[])
            );
            assert!(!env.db.may_have_interior_mut(&Ty::Bool, &[]));
        });
    }
}
