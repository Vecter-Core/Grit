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
                let Some(adt) = self.adts.get(d) else {
                    return false;
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

    /// Tìm impl của `trait_def` cho `ty` (đã giải). Trả (chỉ số impl, tham số impl) nếu có đúng một.
    pub(crate) fn find_impl(
        &self,
        infer: &mut InferCtx,
        trait_def: DefId,
        ty: &Ty,
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
                        .all(|b| self.satisfies_bound(inf, &a, b.trait_def, &[], depth + 1))
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
        let (i, args) = self.find_impl(&mut inf, trait_def, ty, 0)?;
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
                self.find_impl(infer, trait_def, &ty, depth).is_some()
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
                self.find_impl(infer, copy, &ty, depth + 1).is_some()
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

    /// Có `⊐ Drop for` khớp kiểu này không (impl của lang item `drop`).
    #[must_use]
    pub fn has_drop_impl(&self, ty: &Ty) -> bool {
        let Some(&drop) = self.lang.get(&Lang::Drop) else {
            return false;
        };
        let mut inf = InferCtx::new();
        self.find_impl(&mut inf, drop, ty, 0).is_some()
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
