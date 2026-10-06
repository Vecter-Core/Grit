//! Đường dẫn dạng giá trị, lời gọi hàm, phương thức và hàm kết hợp; nghĩa vụ trait.
use super::*;
use crate::collect::AdtKind;
use crate::traits::{Pick, bounds_of, fresh_args, subst_with};

impl<'e, 'a> Checker<'e, 'a> {
    pub(crate) fn push_oblig(&mut self, ty: Ty, trait_def: DefId, node: NodeId, span: Span) {
        self.oblig.push(Oblig {
            ty,
            trait_def,
            node,
            span,
        });
    }

    pub(crate) fn path_value(&mut self, e: &Expr, p: &Path) -> Ty {
        let Some(res) = self.env.res.uses.get(&p.id).cloned() else {
            return Ty::Err;
        };
        match res {
            Res::Err => Ty::Err,
            Res::Local(l) => self.locals.get(&l).map_or(Ty::Err, |i| i.ty.clone()),
            Res::SelfValue => self.self_param_ty(),
            Res::Generic(..) | Res::SelfType => {
                self.err(
                    "E0301",
                    e.id,
                    e.span,
                    "tên kiểu không phải giá trị".to_string(),
                );
                Ty::Err
            }
            Res::Def(d) => self.def_value(e, d),
            Res::Assoc { base, member } => self.assoc_value(e, &base, &member),
        }
    }

    fn def_value(&mut self, e: &Expr, d: DefId) -> Ty {
        let kind = self.env.defs()[d.index()].kind;
        match kind {
            DefKind::Fn => {
                let Some((sig, _)) = self.env.fns.get(&d).cloned() else {
                    return Ty::Err;
                };
                let args = fresh_args(&mut self.infer, &sig.generics);
                for (t, tr) in bounds_of(&sig.generics, &args) {
                    self.push_oblig(t, tr, e.id, e.span);
                }
                let sub = |t: &Ty| subst_with(t, &sig.generics, &args);
                self.calls.push(CallRec {
                    caller: self.owner,
                    callee: Some(sig.node),
                    args: args.clone(),
                    trait_method: None,
                });
                Ty::Fn(
                    sig.params.iter().map(sub).collect(),
                    Box::new(sub(&sig.ret)),
                    sig.effects.clone(),
                )
            }
            DefKind::Const | DefKind::Static => self.env.consts.get(&d).cloned().unwrap_or(Ty::Err),
            DefKind::Variant => {
                let parent = self.env.defs()[d.index()].parent;
                let Some(adt) = parent.and_then(|p| self.env.adts.get(&p)).cloned() else {
                    return Ty::Err;
                };
                let AdtKind::Enum(vs) = &adt.kind else {
                    return Ty::Err;
                };
                let Some(v) = vs.iter().find(|v| v.def == d) else {
                    return Ty::Err;
                };
                let args = fresh_args(&mut self.infer, &adt.generics);
                let aty = Ty::Adt(adt.def, args.clone());
                if v.fields.is_empty() {
                    aty
                } else {
                    let fs: Vec<Ty> = v
                        .fields
                        .iter()
                        .map(|f| subst_with(f, &adt.generics, &args))
                        .collect();
                    Ty::Fn(fs, Box::new(aty), Vec::new())
                }
            }
            _ => {
                let name = self.env.defs()[d.index()].path.clone();
                self.err(
                    "E0301",
                    e.id,
                    e.span,
                    format!("`{name}` là {}, không phải giá trị", kind.as_str()),
                );
                Ty::Err
            }
        }
    }

    /// `Kiểu::tên` — hàm kết hợp (có thể có `self`, dạng gọi tường minh).
    fn assoc_value(&mut self, e: &Expr, base: &Res, member: &str) -> Ty {
        if member.contains("::") {
            self.err(
                "E0309",
                e.id,
                e.span,
                format!("không có hàm kết hợp `{member}`"),
            );
            return Ty::Err;
        }
        let base_ty = match base {
            Res::Def(d) if self.env.adts.contains_key(d) => {
                let n = self.env.adts[d].generics.len();
                let args: Vec<Ty> = (0..n).map(|_| self.fresh()).collect();
                Ty::Adt(*d, args)
            }
            Res::Def(d) if self.env.traits.contains_key(d) => {
                // `Trait::hàm(x)`: Self suy ra từ đối số
                let td = self.env.traits[d].clone();
                let Some(mi) = td.members.iter().position(|m| m.name == member) else {
                    self.err(
                        "E0309",
                        e.id,
                        e.span,
                        format!("trait không có hàm `{member}`"),
                    );
                    return Ty::Err;
                };
                let m = td.members[mi].clone();
                let self_ty = self.fresh();
                let targs = fresh_args(&mut self.infer, &m.generics);
                let sub = |t: &Ty| {
                    subst_with(
                        &crate::collect::Env::subst_self(t, td.node, &self_ty),
                        &m.generics,
                        &targs,
                    )
                };
                self.push_oblig(self_ty.clone(), *d, e.id, e.span);
                for (t, tr) in bounds_of(&m.generics, &targs) {
                    self.push_oblig(t, tr, e.id, e.span);
                }
                let mut ps: Vec<Ty> = Vec::new();
                match m.self_kind {
                    Some(SelfKind::Value) => ps.push(self_ty.clone()),
                    Some(SelfKind::Ref) => ps.push(Ty::Ref(false, Box::new(self_ty.clone()))),
                    Some(SelfKind::RefMut) => ps.push(Ty::Ref(true, Box::new(self_ty.clone()))),
                    None => {}
                }
                ps.extend(m.params.iter().map(&sub));
                self.methods.push((
                    e.id,
                    MethodRes::TraitMethod {
                        trait_def: *d,
                        member: mi,
                        self_ty: self_ty.clone(),
                    },
                ));
                self.calls.push(CallRec {
                    caller: self.owner,
                    callee: None,
                    args: targs.clone(),
                    trait_method: Some((*d, mi, self_ty.clone())),
                });
                return Ty::Fn(ps, Box::new(sub(&m.ret)), m.effects.clone());
            }
            Res::Generic(o, i) => Ty::Param(crate::ty::ParamId {
                owner: *o,
                index: *i,
            }),
            Res::SelfType => self.cx.self_ty.clone().unwrap_or(Ty::Err),
            _ => {
                self.err(
                    "E0309",
                    e.id,
                    e.span,
                    format!("không có hàm kết hợp `{member}`"),
                );
                return Ty::Err;
            }
        };
        let scope = self.scope.clone();
        match self
            .env
            .lookup_assoc(&mut self.infer, &base_ty, member, &scope)
        {
            Ok(pick) => self.apply_pick_types(e, &pick),
            Err(msg) => {
                self.err("E0309", e.id, e.span, msg);
                Ty::Err
            }
        }
    }

    /// Kiểu hàm của một ứng viên được gọi dạng `Kiểu::hàm` (có thể có tham số `self` đầu tiên).
    fn apply_pick_types(&mut self, e: &Expr, pick: &Pick) -> Ty {
        for (t, tr) in &pick.bounds {
            self.push_oblig(t.clone(), *tr, e.id, e.span);
        }
        self.record_pick(e.id, pick);
        let mut ps: Vec<Ty> = Vec::new();
        match pick.self_kind {
            Some(SelfKind::Value) => ps.push(pick.recv_self.clone()),
            Some(SelfKind::Ref) => ps.push(Ty::Ref(false, Box::new(pick.recv_self.clone()))),
            Some(SelfKind::RefMut) => ps.push(Ty::Ref(true, Box::new(pick.recv_self.clone()))),
            None => {}
        }
        ps.extend(pick.params.iter().cloned());
        Ty::Fn(ps, Box::new(pick.ret.clone()), Vec::new())
    }

    fn record_pick(&mut self, node: NodeId, pick: &Pick) {
        self.methods.push((node, pick.res.clone()));
        match &pick.res {
            MethodRes::Impl {
                impl_node,
                member,
                args,
            } => {
                let callee = self
                    .env
                    .impls
                    .iter()
                    .find(|i| i.node == *impl_node)
                    .and_then(|i| i.members.get(*member))
                    .map(|m| m.node);
                let mut all = args.clone();
                all.extend(pick.generics.iter().cloned());
                self.calls.push(CallRec {
                    caller: self.owner,
                    callee,
                    args: all,
                    trait_method: None,
                });
            }
            MethodRes::TraitMethod {
                trait_def,
                member,
                self_ty,
            } => {
                self.calls.push(CallRec {
                    caller: self.owner,
                    callee: None,
                    args: pick.generics.clone(),
                    trait_method: Some((*trait_def, *member, self_ty.clone())),
                });
            }
        }
    }

    // ---------------- lời gọi ----------------

    pub(crate) fn call(
        &mut self,
        e: &Expr,
        callee: &Expr,
        args: &[Expr],
        expect: Option<&Ty>,
    ) -> Ty {
        if let ExprKind::Field {
            base,
            name: FieldName::Named(n),
        } = &callee.kind
        {
            return self.method_call(e, base, n, args);
        }
        let ct = self.expr(callee, None);
        match self.infer.shallow(&ct) {
            Ty::Fn(params, ret, _) => {
                if params.len() != args.len() {
                    self.err(
                        "E0301",
                        e.id,
                        e.span,
                        format!("hàm nhận {} đối số, gặp {}", params.len(), args.len()),
                    );
                    for a in args {
                        let _ = self.expr(a, None);
                    }
                    return *ret;
                }
                if let Some(x) = expect {
                    // gợi ý ngược từ kiểu mong đợi giúp suy ra tham số kiểu của kết quả (không bắt buộc)
                    let r = self.infer.shallow(&ret);
                    if r.has_infer() && self.infer.probe(|inf| inf.unify(&r, x)) {
                        self.infer.unify(&r, x);
                    }
                }
                for (a, p) in args.iter().zip(&params) {
                    self.expr_expect(a, p);
                }
                *ret
            }
            Ty::Err => {
                for a in args {
                    let _ = self.expr(a, None);
                }
                Ty::Err
            }
            other => {
                let s = self.show(&other);
                self.err(
                    "E0301",
                    callee.id,
                    callee.span,
                    format!("`{s}` không phải hàm nên không gọi được"),
                );
                for a in args {
                    let _ = self.expr(a, None);
                }
                Ty::Err
            }
        }
    }

    fn method_call(&mut self, e: &Expr, base: &Expr, name: &Ident, args: &[Expr]) -> Ty {
        let bt = self.expr(base, None);
        let scope = self.scope.clone();
        let pick = match self
            .env
            .lookup_method(&mut self.infer, &bt, &name.name, &scope)
        {
            Ok(p) => p,
            Err(msg) => {
                let code = if msg.contains("chưa đủ thông tin") {
                    "E0301"
                } else {
                    "E0309"
                };
                self.err(code, name.id, name.span, msg);
                for a in args {
                    let _ = self.expr(a, None);
                }
                return Ty::Err;
            }
        };
        // phương thức `&!self` cần người nhận có thể thay đổi (D5)
        if pick.self_kind == Some(SelfKind::RefMut) {
            let recv = self.infer.shallow(&bt);
            let ok = match recv {
                Ty::Ref(m, _) => m,
                _ => !Self::is_place(base) || self.place_mutable(base),
            };
            if !ok {
                let n = self.place_name(base);
                self.err(
                    "E0308",
                    base.id,
                    base.span,
                    format!("phương thức cần mượn duy nhất `&!self` nhưng `{n}` bất biến (D5): khai báo bằng `μ`"),
                );
            }
        }
        for (t, tr) in &pick.bounds {
            self.push_oblig(t.clone(), *tr, e.id, e.span);
        }
        self.record_pick(e.id, &pick);
        if pick.params.len() != args.len() {
            self.err(
                "E0301",
                e.id,
                e.span,
                format!(
                    "phương thức `{}` nhận {} đối số, gặp {}",
                    name.name,
                    pick.params.len(),
                    args.len()
                ),
            );
            for a in args {
                let _ = self.expr(a, None);
            }
            return pick.ret;
        }
        for (a, p) in args.iter().zip(&pick.params) {
            self.expr_expect(a, p);
        }
        pick.ret
    }
}
