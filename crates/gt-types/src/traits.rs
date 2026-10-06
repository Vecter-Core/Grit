//! Impl, ràng buộc trait, `is_copy`, phân giải phương thức (ADR-018 §2).
use crate::collect::{Env, FnSigInfo, GParam, ImplDef, Lang};
use crate::infer::InferCtx;
use crate::ty::{ParamId, SELF_INDEX, Ty};
use gt_ast::SelfKind;
use gt_resolve::DefId;

/// Ràng buộc `T: Trait` đang có hiệu lực trong thân hàm đang kiểm.
#[derive(Clone, Debug)]
pub struct Bound {
    pub ty: Ty,
    pub trait_def: DefId,
}

/// Cách một lời gọi phương thức/hàm kết hợp được phân giải.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MethodRes {
    /// Hàm của một `⊐` (impl_idx = chỉ số trong bảng impl; member = chỉ số thành viên); `args` = tham số kiểu của impl.
    Impl {
        impl_node: gt_ast::NodeId,
        member: usize,
        args: Vec<Ty>,
    },
    /// Gọi qua ràng buộc/trait: chọn impl cụ thể được hoãn tới lúc thu thập instance.
    TraitMethod {
        trait_def: DefId,
        member: usize,
        self_ty: Ty,
    },
}

/// Một ứng viên phương thức đã dò xong.
#[derive(Clone, Debug)]
pub struct Pick {
    pub res: MethodRes,
    pub self_kind: Option<SelfKind>,
    /// Kiểu `Self` tại bước deref đó (đã hợp nhất với đầu đề impl).
    pub recv_self: Ty,
    pub params: Vec<Ty>,
    pub ret: Ty,
    pub generics: Vec<Ty>,
    pub bounds: Vec<(Ty, DefId)>,
}

pub(crate) fn subst_with(ty: &Ty, gens: &[GParam], args: &[Ty]) -> Ty {
    ty.subst(&|p| {
        gens.iter()
            .position(|g| g.id == p)
            .and_then(|i| args.get(i).cloned())
    })
}

pub(crate) fn fresh_args(infer: &mut InferCtx, gens: &[GParam]) -> Vec<Ty> {
    gens.iter().map(|_| infer.fresh_var()).collect()
}

impl Env<'_> {
    /// `Self` của trait `t` được thay bằng `self_ty`.
    pub(crate) fn subst_self(ty: &Ty, trait_node: gt_ast::NodeId, self_ty: &Ty) -> Ty {
        ty.subst(&|p| (p.owner == trait_node && p.index == SELF_INDEX).then(|| self_ty.clone()))
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

    /// Chuỗi autoderef của người nhận: `[T, *T, **T, …]`.
    fn deref_chain(&self, infer: &InferCtx, recv: &Ty) -> Vec<Ty> {
        let mut out = vec![infer.resolve(recv)];
        loop {
            let last = out.last().cloned().unwrap_or(Ty::Err);
            match last {
                Ty::Ref(_, inner) => out.push(infer.resolve(&inner)),
                _ => break,
            }
            if out.len() > 8 {
                break;
            }
        }
        out
    }

    /// Dò phương thức `name` cho người nhận `recv` (autoderef; inherent trước, rồi trait, rồi ràng buộc của tham số kiểu).
    /// `Err(msg)` nếu không có hoặc mơ hồ.
    pub(crate) fn lookup_method(
        &self,
        infer: &mut InferCtx,
        recv: &Ty,
        name: &str,
        scope: &[Bound],
    ) -> Result<Pick, String> {
        let chain = self.deref_chain(infer, recv);
        self.lookup_chain(infer, &chain, name, scope)
    }

    /// Dò hàm kết hợp `Kiểu::tên` (không autoderef).
    pub(crate) fn lookup_assoc(
        &self,
        infer: &mut InferCtx,
        base: &Ty,
        name: &str,
        scope: &[Bound],
    ) -> Result<Pick, String> {
        let chain = vec![infer.resolve(base)];
        self.lookup_chain(infer, &chain, name, scope)
    }

    fn lookup_chain(
        &self,
        infer: &mut InferCtx,
        chain: &[Ty],
        name: &str,
        scope: &[Bound],
    ) -> Result<Pick, String> {
        for u in chain {
            if matches!(u, Ty::Infer(_)) {
                return Err("kiểu của người nhận chưa đủ thông tin để chọn phương thức; hãy thêm chú thích kiểu".to_string());
            }
            let mut picks: Vec<Pick> = Vec::new();
            // A + B: impl (inherent và trait) có thành viên `name` cho kiểu u
            for imp in &self.impls {
                let Some(mi) = imp.members.iter().position(|m| m.name == name) else {
                    continue;
                };
                if let Some(p) = self.try_impl_member(infer, imp, mi, u) {
                    picks.push(p);
                }
            }
            // C: ràng buộc trên tham số kiểu
            if let Ty::Param(_) = u {
                for b in scope {
                    if infer.resolve(&b.ty) != *u {
                        continue;
                    }
                    if let Some(td) = self.traits.get(&b.trait_def)
                        && let Some(mi) = td.members.iter().position(|m| m.name == name)
                    {
                        let m = &td.members[mi];
                        let targs = fresh_args(infer, &m.generics);
                        let sub = |t: &Ty| {
                            subst_with(&Self::subst_self(t, td.node, u), &m.generics, &targs)
                        };
                        picks.push(Pick {
                            res: MethodRes::TraitMethod {
                                trait_def: b.trait_def,
                                member: mi,
                                self_ty: u.clone(),
                            },
                            self_kind: m.self_kind,
                            recv_self: u.clone(),
                            params: m.params.iter().map(&sub).collect(),
                            ret: sub(&m.ret),
                            bounds: bounds_of(&m.generics, &targs),
                            generics: targs,
                        });
                    }
                }
            }
            match picks.len() {
                0 => continue,
                1 => return Ok(picks.remove(0)),
                _ => {
                    // inherent trước (trait_ref None)
                    let inherent: Vec<&Pick> = picks
                        .iter()
                        .filter(|p| matches!(&p.res, MethodRes::Impl { impl_node, .. } if self.impls.iter().any(|i| i.node == *impl_node && i.trait_ref.is_none())))
                        .collect();
                    if inherent.len() == 1 {
                        return Ok(inherent[0].clone());
                    }
                    return Err(format!("phương thức `{name}` mơ hồ: có nhiều ứng viên"));
                }
            }
        }
        Err(format!("không có phương thức `{name}` cho kiểu này"))
    }

    pub(crate) fn try_impl_member(
        &self,
        infer: &mut InferCtx,
        imp: &ImplDef,
        mi: usize,
        u: &Ty,
    ) -> Option<Pick> {
        let m: &FnSigInfo = &imp.members[mi];
        let ok = infer.probe(|inf| {
            let args = fresh_args(inf, &imp.generics);
            let st = subst_with(&imp.self_ty, &imp.generics, &args);
            inf.unify(&st, u)
        });
        if !ok {
            return None;
        }
        let args = fresh_args(infer, &imp.generics);
        let st = subst_with(&imp.self_ty, &imp.generics, &args);
        infer.unify(&st, u);
        let margs = fresh_args(infer, &m.generics);
        let sub = |t: &Ty| subst_with(&subst_with(t, &imp.generics, &args), &m.generics, &margs);
        let params: Vec<Ty> = m.params.iter().map(&sub).collect();
        let ret = sub(&m.ret);
        let mut bounds = bounds_of(&imp.generics, &args);
        bounds.extend(bounds_of(&m.generics, &margs));
        Some(Pick {
            res: MethodRes::Impl {
                impl_node: imp.node,
                member: mi,
                args,
            },
            self_kind: m.self_kind,
            recv_self: u.clone(),
            params,
            ret,
            generics: margs,
            bounds,
        })
    }
}

pub(crate) fn bounds_of(gens: &[GParam], args: &[Ty]) -> Vec<(Ty, DefId)> {
    let mut out = Vec::new();
    for (g, a) in gens.iter().zip(args) {
        for b in &g.bounds {
            out.push((a.clone(), b.trait_def));
        }
    }
    out
}

/// Ràng buộc có hiệu lực từ danh sách generics (kiểu `Param`).
pub(crate) fn scope_bounds(gens: &[&GParam]) -> Vec<Bound> {
    let mut out = Vec::new();
    for g in gens {
        for b in &g.bounds {
            out.push(Bound {
                ty: Ty::Param(g.id),
                trait_def: b.trait_def,
            });
        }
    }
    out
}

#[allow(dead_code)]
pub(crate) fn param_ty(owner: gt_ast::NodeId, index: u32) -> Ty {
    Ty::Param(ParamId { owner, index })
}
