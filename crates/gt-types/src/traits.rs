//! Impl, ràng buộc trait, `is_copy`, phân giải phương thức (ADR-018 §2).
use crate::collect::{Env, FnSigInfo, GParam, ImplDef};
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
    /// Số lần deref người nhận trước khi khớp (autoderef).
    pub derefs: usize,
    /// Kiểu `Self` tại bước deref đó (đã hợp nhất với đầu đề impl).
    pub recv_self: Ty,
    pub params: Vec<Ty>,
    pub ret: Ty,
    /// Hiệu ứng khai báo của hàm (đã sắp) — phải đi theo `Ty::Fn` khi `Kiểu::hàm` được dùng làm GIÁ TRỊ (FB-C5), nếu không hiệu ứng bị mất.
    pub effects: Vec<String>,
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
        for (derefs, u) in chain.iter().enumerate() {
            if matches!(u, Ty::Infer(_)) {
                return Err("kiểu của người nhận chưa đủ thông tin để chọn phương thức; hãy thêm chú thích kiểu".to_string());
            }
            let mut picks: Vec<Pick> = Vec::new();
            // A + B: impl (inherent và trait) có thành viên `name` cho kiểu u
            for imp in &self.db.impls {
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
                    if let Some(td) = self.db.traits.get(&b.trait_def)
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
                            derefs: 0,
                            recv_self: u.clone(),
                            params: m.params.iter().map(&sub).collect(),
                            ret: sub(&m.ret),
                            effects: m.effects.clone(),
                            bounds: bounds_of(&m.generics, &targs),
                            generics: targs,
                        });
                    }
                }
            }
            for p in &mut picks {
                p.derefs = derefs;
            }
            match picks.len() {
                0 => continue,
                1 => return Ok(picks.remove(0)),
                _ => {
                    // inherent trước (trait_ref None)
                    let inherent: Vec<&Pick> = picks
                        .iter()
                        .filter(|p| matches!(&p.res, MethodRes::Impl { impl_node, .. } if self.db.impls.iter().any(|i| i.node == *impl_node && i.trait_ref.is_none())))
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
            derefs: 0,
            recv_self: u.clone(),
            params,
            ret,
            effects: m.effects.clone(),
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
