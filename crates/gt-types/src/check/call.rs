//! Đường dẫn dạng giá trị, lời gọi hàm, phương thức và hàm kết hợp; nghĩa vụ trait.
use super::*;
use crate::collect::AdtKind;
use crate::traits::{Pick, bounds_of, fresh_args, subst_with};

impl<'e, 'a> Checker<'e, 'a> {
    /// Hiệu ứng của một đối số hàm không có tác dụng phụ khi dò: `Path` tới hàm tự do hoặc biến cục bộ kiểu hàm.
    fn fn_arg_effects(&mut self, a: &Expr) -> Option<Vec<String>> {
        let ExprKind::Path(p) = &a.kind else {
            return None;
        };
        match self.env.res.uses.get(&p.id)?.clone() {
            Res::Def(d) if self.env.defs()[d.index()].kind == DefKind::Fn => {
                self.env.db.fns.get(&d).map(|(s, _)| s.effects.clone())
            }
            Res::Local(l) => {
                let t = self.locals.get(&l)?.ty.clone();
                match self.infer.shallow(&t) {
                    Ty::Fn(_, _, e) => Some(e),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Tham số kiểu hàm có hiệu ứng đa hình (`poly`/`detach`, chỉ ở prelude): lấy hiệu ứng của đối số thay cho marker (ADR-027 §3).
    fn poly_param(&mut self, a: &Expr, p: &Ty) -> Ty {
        let ps = self.infer.shallow(p);
        if let Ty::Fn(params, ret, eff) = &ps
            && eff.len() == 1
            && (eff[0] == "poly" || eff[0] == "detach")
            && let Some(e) = self.fn_arg_effects(a)
        {
            return Ty::Fn(params.clone(), ret.clone(), e);
        }
        ps
    }

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
                let Some((sig, _)) = self.env.db.fns.get(&d).cloned() else {
                    return Ty::Err;
                };
                self.reject_intrinsic_value(e, sig.node);
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
                    span: e.span,
                });
                self.sites.push((
                    e.id,
                    CallSite {
                        target: CallTarget::Fn(sig.node),
                        generics: args.clone(),
                        self_kind: sig.self_kind,
                        derefs: 0,
                        method_syntax: false,
                    },
                ));
                let ps: Vec<Ty> = sig.params.iter().map(sub).collect();
                let rt = sub(&sig.ret);
                self.cap_uses.push(CapUse {
                    params: ps.clone(),
                    ret: rt.clone(),
                    node: e.id,
                    span: e.span,
                });
                Ty::Fn(ps, Box::new(rt), sig.effects.clone())
            }
            DefKind::Const | DefKind::Static => {
                self.env.db.consts.get(&d).cloned().unwrap_or(Ty::Err)
            }
            DefKind::Variant => {
                let parent = self.env.defs()[d.index()].parent;
                let Some(adt) = parent.and_then(|p| self.env.db.adts.get(&p)).cloned() else {
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
            Res::Def(d) if self.env.db.adts.contains_key(d) => {
                let n = self.env.db.adts[d].generics.len();
                let args: Vec<Ty> = (0..n).map(|_| self.fresh()).collect();
                Ty::Adt(*d, args)
            }
            Res::Def(d) if self.env.db.traits.contains_key(d) => {
                // `Trait::hàm(x)`: Self suy ra từ đối số
                let td = self.env.db.traits[d].clone();
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
                    span: e.span,
                });
                self.sites.push((
                    e.id,
                    CallSite {
                        target: CallTarget::TraitMethod {
                            trait_def: *d,
                            member: mi,
                            self_ty: self_ty.clone(),
                        },
                        generics: targs.clone(),
                        self_kind: m.self_kind,
                        derefs: 0,
                        method_syntax: false,
                    },
                ));
                let rt = sub(&m.ret);
                self.cap_uses.push(CapUse {
                    params: ps.clone(),
                    ret: rt.clone(),
                    node: e.id,
                    span: e.span,
                });
                return Ty::Fn(ps, Box::new(rt), m.effects.clone());
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
    /// Hàm của lib/core (thân prelude KHÔNG có MIR = intrinsic) dùng làm GIÁ TRỊ (không phải người được gọi trực tiếp) ⇒ E0313: mọi nghĩa vụ theo lớp (precondition,
    /// E0310, E0405, E0502…) được sinh ở điểm gọi trực tiếp nên gọi qua con trỏ né kiểm tra của cả lớp (FAIL_OPEN_AUDIT tổng quát hoá S-E). `Atomic` giữ E0503 của gt-conc.
    fn reject_intrinsic_value(&mut self, e: &Expr, callee_node: NodeId) {
        if crate::ALLOW_INTRINSIC_VALUES
            || self.callee_pos == Some(e.id)
            || !self.env.is_prelude(callee_node.file())
        {
            return;
        }
        let atomic = self.env.db.impls.iter().any(|i| {
            i.members.iter().any(|m| m.node == callee_node)
                && matches!(&i.self_ty, Ty::Adt(d, _) if self.env.db.adts.get(d).and_then(|a| a.lang) == Some(crate::collect::Lang::Atomic))
        });
        if atomic {
            return;
        }
        // nguồn capability gốc (`root_*`) ngoài `main`: E0310 ở `check_caps` đã chặn; không báo trùng (giữ nguyên mã E0310 của corpus)
        if !self.entry
            && let Some(sig) = self.env.db.sig_by_node(callee_node)
            && self.has_cap(&sig.ret)
            && !sig.params.iter().any(|p| self.has_cap(p))
        {
            return;
        }
        let name = self
            .env
            .db
            .sig_by_node(callee_node)
            .map_or_else(|| "?".to_string(), |s| s.name.clone());
        self.err(
            "E0313",
            e.id,
            e.span,
            format!("hàm intrinsic `{name}` của lib/core không được dùng làm giá trị hàm (gọi qua con trỏ né kiểm tra theo lớp: precondition, capability, arena, khoá…); gọi trực tiếp"),
        );
    }

    fn apply_pick_types(&mut self, e: &Expr, pick: &Pick) -> Ty {
        if let MethodRes::Impl {
            impl_node, member, ..
        } = &pick.res
            && let Some(node) = self
                .env
                .db
                .impls
                .iter()
                .find(|i| i.node == *impl_node)
                .and_then(|i| i.members.get(*member))
                .map(|m| m.node)
        {
            self.reject_intrinsic_value(e, node);
        }
        for (t, tr) in &pick.bounds {
            self.push_oblig(t.clone(), *tr, e.id, e.span);
        }
        self.record_pick(e.id, e.span, pick);
        let mut ps: Vec<Ty> = Vec::new();
        match pick.self_kind {
            Some(SelfKind::Value) => ps.push(pick.recv_self.clone()),
            Some(SelfKind::Ref) => ps.push(Ty::Ref(false, Box::new(pick.recv_self.clone()))),
            Some(SelfKind::RefMut) => ps.push(Ty::Ref(true, Box::new(pick.recv_self.clone()))),
            None => {}
        }
        ps.extend(pick.params.iter().cloned());
        self.cap_uses.push(CapUse {
            params: ps.clone(),
            ret: pick.ret.clone(),
            node: e.id,
            span: e.span,
        });
        Ty::Fn(ps, Box::new(pick.ret.clone()), self.pick_effects(pick))
    }

    /// Hiệu ứng KHAI BÁO của thành viên mà `pick` chọn (FB-15: trước đây kiểu hàm của hàm kết hợp dùng làm giá trị mang hiệu ứng RỖNG ⇒ gọi qua biến mất `io`/`mem`…).
    fn pick_effects(&self, pick: &Pick) -> Vec<String> {
        match &pick.res {
            MethodRes::Impl {
                impl_node, member, ..
            } => self
                .env
                .db
                .impls
                .iter()
                .find(|i| i.node == *impl_node)
                .and_then(|i| i.members.get(*member))
                .map(|m| m.effects.clone()),
            MethodRes::TraitMethod {
                trait_def, member, ..
            } => self
                .env
                .db
                .traits
                .get(trait_def)
                .and_then(|td| td.members.get(*member))
                .map(|m| m.effects.clone()),
        }
        // không tra được thành viên (db không nhất quán): KHÔNG đoán là `∅`; dùng một hiệu ứng không bao giờ khai báo được để E0801/E0301 chặn (fail-closed)
        .unwrap_or_else(|| vec!["?unknown".to_string()])
    }

    fn record_pick(&mut self, node: NodeId, span: gt_span::Span, pick: &Pick) {
        self.record_pick_site(node, pick, false);
        self.methods.push((node, pick.res.clone()));
        match &pick.res {
            MethodRes::Impl {
                impl_node,
                member,
                args,
            } => {
                let callee = self
                    .env
                    .db
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
                    span,
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
                    span,
                });
            }
        }
    }

    fn record_pick_site(&mut self, node: NodeId, pick: &Pick, method_syntax: bool) {
        let (target, generics) = match &pick.res {
            MethodRes::Impl {
                impl_node,
                member,
                args,
            } => {
                let callee = self
                    .env
                    .db
                    .impls
                    .iter()
                    .find(|i| i.node == *impl_node)
                    .and_then(|i| i.members.get(*member))
                    .map(|m| m.node);
                let Some(callee) = callee else {
                    return;
                };
                let mut all = args.clone();
                all.extend(pick.generics.iter().cloned());
                (CallTarget::Fn(callee), all)
            }
            MethodRes::TraitMethod {
                trait_def,
                member,
                self_ty,
            } => (
                CallTarget::TraitMethod {
                    trait_def: *trait_def,
                    member: *member,
                    self_ty: self_ty.clone(),
                },
                pick.generics.clone(),
            ),
        };
        let site = CallSite {
            target,
            generics,
            self_kind: pick.self_kind,
            derefs: pick.derefs,
            method_syntax,
        };
        if let Some(slot) = self.sites.iter_mut().find(|(n, _)| *n == node) {
            slot.1 = site;
        } else {
            self.sites.push((node, site));
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
        let prev = self.callee_pos.replace(callee.id);
        let ct = self.expr(callee, None);
        self.callee_pos = prev;
        match self.infer.shallow(&ct) {
            Ty::Fn(params, ret, _) => {
                // gọi QUA GIÁ TRỊ hàm (tham số/biến/biểu thức kiểu `ƒ(…)`): hàm đích không còn thấy được ở đây, nên chữ ký của giá trị là
                // thứ duy nhất để nhận ra nguồn capability gốc (D28/D29). Đường dẫn tới hàm item/`Kiểu::hàm` đã được ghi ở chỗ phân giải đường dẫn.
                let named_item = matches!(&callee.kind, ExprKind::Path(p)
                    if matches!(self.env.res.uses.get(&p.id), Some(Res::Def(_) | Res::Assoc { .. })));
                if !named_item {
                    self.cap_uses.push(CapUse {
                        params: params.clone(),
                        ret: (*ret).clone(),
                        node: e.id,
                        span: e.span,
                    });
                }
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
                    let p = self.poly_param(a, p);
                    self.expr_expect(a, &p);
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
        self.record_pick(e.id, e.span, &pick);
        self.record_pick_site(e.id, &pick, true);
        let mut cap_ps = vec![pick.recv_self.clone()];
        cap_ps.extend(pick.params.iter().cloned());
        self.cap_uses.push(CapUse {
            params: cap_ps,
            ret: pick.ret.clone(),
            node: e.id,
            span: e.span,
        });
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
            let p = self.poly_param(a, p);
            self.expr_expect(a, &p);
        }
        pick.ret
    }
}
