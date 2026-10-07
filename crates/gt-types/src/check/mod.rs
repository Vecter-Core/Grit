//! Kiểm kiểu thân hàm: suy luận bidirectional cục bộ (ADR-018; `semantics.md` §1, §22).
mod call;
mod expr;
mod pat;
pub(crate) use pat::decode_char;

use crate::collect::{Env, FnBody, FnSigInfo, GParam, Lang, LowerCx};
use crate::exhaust::Pat as XPat;
use crate::infer::InferCtx;
use crate::traits::{Bound, MethodRes, scope_bounds};
use crate::ty::{IntTy, Ty};
use gt_ast::*;
use gt_diag::{Applicability, Suggestion};
use gt_resolve::{DefId, DefKind, LocalId, Res};
use gt_span::Span;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(crate) struct LocalInfo {
    pub ty: Ty,
    pub mutable: bool,
    pub name: String,
    /// Chỗ chèn `μ` nếu khai báo là `≔`/tham số (để gợi ý sửa E0308); `None` với binding trong pattern, biến vòng lặp.
    pub mut_hint: Option<(NodeId, u32)>,
}

/// Một chỗ dùng hàm/phương thức có thể sinh capability gốc (E0310, D28/D29): kiểm sau khi kiểu đã giải xong.
pub(crate) struct CapUse {
    pub params: Vec<Ty>,
    pub ret: Ty,
    pub node: NodeId,
    pub span: Span,
}

pub(crate) struct Oblig {
    pub ty: Ty,
    pub trait_def: DefId,
    pub node: NodeId,
    pub span: Span,
}

pub(crate) struct MatchCheck {
    pub node: NodeId,
    pub span: Span,
    pub scrut: Ty,
    pub arms: Vec<XPat>,
    /// Chỗ chèn nhánh `_` gợi ý (ngay sau nhánh cuối).
    pub insert_at: u32,
}

/// Hàm được một chỗ dùng (đường dẫn hàm, `Kiểu::hàm`, lời gọi phương thức) chỉ tới — cho bước hạ MIR (ADR-020 §1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallTarget {
    /// Hàm tự do hoặc thành viên impl (NodeId của hàm).
    Fn(NodeId),
    /// Qua trait trên kiểu chưa chọn impl (tham số kiểu): giải khi đơn hình hoá.
    TraitMethod {
        trait_def: DefId,
        member: usize,
        self_ty: Ty,
    },
}

/// Thông tin một chỗ dùng hàm, đã giải kiểu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallSite {
    pub target: CallTarget,
    /// Đối số kiểu: với thành viên impl là tham số của impl rồi tham số của chính hàm.
    pub generics: Vec<Ty>,
    /// `self` của phương thức (nếu có).
    pub self_kind: Option<SelfKind>,
    /// Số lần deref người nhận (lời gọi dạng `x.m()`); 0 với các dạng khác.
    pub derefs: usize,
    /// Có phải dạng `x.m(…)` (người nhận ngầm là đối số đầu)?
    pub method_syntax: bool,
}

/// Một lời gọi trong thân hàm (cho bộ thu thập instance, ADR-018 §1).
#[derive(Clone, Debug)]
pub struct CallRec {
    pub caller: NodeId,
    /// Hàm được gọi (NodeId của item/thành viên) hoặc `None` nếu là lời gọi qua trait trên kiểu chưa biết impl.
    pub callee: Option<NodeId>,
    pub args: Vec<Ty>,
    pub trait_method: Option<(DefId, usize, Ty)>,
}

pub(crate) struct Checker<'e, 'a> {
    pub env: &'e mut Env<'a>,
    pub infer: InferCtx,
    pub module: DefId,
    pub owner: NodeId,
    pub key: String,
    pub ret: Ty,
    pub cx: LowerCx,
    pub self_kind: Option<SelfKind>,
    pub locals: BTreeMap<LocalId, LocalInfo>,
    pub loops: Vec<bool>,
    pub scope: Vec<Bound>,
    pub oblig: Vec<Oblig>,
    pub types: BTreeMap<NodeId, Ty>,
    pub methods: Vec<(NodeId, MethodRes)>,
    pub matches: Vec<MatchCheck>,
    pub calls: Vec<CallRec>,
    pub cap_uses: Vec<CapUse>,
    pub sites: Vec<(NodeId, CallSite)>,
    /// Hàm này là `main` (nguồn duy nhất của capability gốc, D29).
    pub entry: bool,
    pub in_unsafe: u32,
    /// đang kiểm một biểu thức hợp đồng (`⊳ ⊲ ↓ ⋈`): chỗ DUY NHẤT `∀/∃` hợp lệ (E0316)
    pub in_contract: u32,
    pub unresolved_reported: bool,
}

impl<'e, 'a> Checker<'e, 'a> {
    pub(crate) fn new(
        env: &'e mut Env<'a>,
        module: DefId,
        owner: NodeId,
        key: String,
    ) -> Checker<'e, 'a> {
        Checker {
            env,
            infer: InferCtx::new(),
            module,
            owner,
            key,
            ret: Ty::unit(),
            cx: LowerCx::default(),
            self_kind: None,
            locals: BTreeMap::new(),
            loops: Vec::new(),
            scope: Vec::new(),
            oblig: Vec::new(),
            types: BTreeMap::new(),
            methods: Vec::new(),
            matches: Vec::new(),
            calls: Vec::new(),
            cap_uses: Vec::new(),
            sites: Vec::new(),
            entry: false,
            in_unsafe: 0,
            in_contract: 0,
            unresolved_reported: false,
        }
    }

    // ---------------- tiện ích ----------------

    pub(crate) fn show(&self, t: &Ty) -> String {
        self.infer.resolve(t).render(self.env.defs())
    }

    pub(crate) fn err(&mut self, code: &str, node: NodeId, span: Span, msg: String) {
        self.env.err(code, node, span, msg);
    }

    pub(crate) fn err_sugg(
        &mut self,
        code: &str,
        node: NodeId,
        span: Span,
        msg: String,
        s: Suggestion,
    ) {
        self.env.err_sugg(code, node, span, msg, s);
    }

    pub(crate) fn lang(&self, l: Lang) -> Option<DefId> {
        self.env.db.lang.get(&l).copied()
    }

    pub(crate) fn fresh(&mut self) -> Ty {
        self.infer.fresh_var()
    }

    pub(crate) fn is_within(&self, a: DefId, anc: DefId) -> bool {
        let mut cur = Some(a);
        while let Some(c) = cur {
            if c == anc {
                return true;
            }
            cur = self.env.defs()[c.index()].parent;
        }
        false
    }

    /// `Option<T>` của lang item (cho `+? -? *?`).
    pub(crate) fn option_of(&mut self, t: Ty, node: NodeId, span: Span) -> Ty {
        match self.lang(Lang::Option) {
            Some(d) => Ty::Adt(d, vec![t]),
            None => {
                self.err(
                    "E0301",
                    node,
                    span,
                    "thiếu lang item `option` trong prelude (cần cho `+?`, `-?`, `*?`)".to_string(),
                );
                Ty::Err
            }
        }
    }

    pub(crate) fn range_of(&mut self, t: Ty, inclusive: bool, node: NodeId, span: Span) -> Ty {
        let l = if inclusive {
            Lang::RangeInclusive
        } else {
            Lang::Range
        };
        match self.lang(l) {
            Some(d) => Ty::Adt(d, vec![t]),
            None => {
                self.err(
                    "E0301",
                    node,
                    span,
                    format!(
                        "thiếu lang item `{}` trong prelude",
                        if inclusive {
                            "range_inclusive"
                        } else {
                            "range"
                        }
                    ),
                );
                Ty::Err
            }
        }
    }

    pub(crate) fn declare_local(&mut self, ident: &Ident, ty: Ty, mutable: bool) {
        self.declare_binding(ident, ty, mutable, false);
    }

    /// `hintable`: khai báo bằng `≔`/tham số, nên có thể sửa bằng cách thêm `μ` ngay trước tên.
    pub(crate) fn declare_binding(&mut self, ident: &Ident, ty: Ty, mutable: bool, hintable: bool) {
        if let Some(&l) = self.env.res.decls.get(&ident.id) {
            self.locals.insert(
                l,
                LocalInfo {
                    ty,
                    mutable,
                    name: ident.name.clone(),
                    mut_hint: (hintable && !mutable).then_some((ident.id, ident.span.start)),
                },
            );
        }
    }

    /// Gốc cục bộ của một chỗ gán chỉ đi qua trường/chỉ số của GIÁ TRỊ (không qua tham chiếu): thêm `μ` vào khai báo mới sửa được.
    fn mut_fix_for(&self, place: &Expr) -> Option<Suggestion> {
        let mut e = place;
        loop {
            match &e.kind {
                ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => {
                    if matches!(
                        self.types.get(&base.id).map(|t| self.infer.resolve(t)),
                        Some(Ty::Ref(..) | Ty::RawPtr(..))
                    ) {
                        return None;
                    }
                    e = base;
                }
                ExprKind::Path(p) => {
                    let Some(Res::Local(l)) = self.env.res.uses.get(&p.id) else {
                        return None;
                    };
                    let (node, at) = self.locals.get(l)?.mut_hint?;
                    return Some(Suggestion::new(
                        "khai báo biến là có thể thay đổi",
                        Applicability::MaybeIncorrect,
                        vec![gt_diag::Edit {
                            file: self.env.fname(node.file()),
                            span: Span::new(at, at),
                            replacement: "μ".to_string(),
                        }],
                    ));
                }
                _ => return None,
            }
        }
    }

    /// Báo E0308 (D5) kèm gợi ý thêm `μ` nếu sửa được.
    pub(crate) fn immutable_err(&mut self, place: &Expr, msg: String) {
        match self.mut_fix_for(place) {
            Some(s) => self.err_sugg("E0308", place.id, place.span, msg, s),
            None => self.err("E0308", place.id, place.span, msg),
        }
    }

    // ---------------- ép kiểu (coercion) ----------------

    /// `from` gán được vào `to` tại một điểm ép kiểu: `!` ép được sang mọi kiểu (ADR-018 §3.1); còn lại hợp nhất.
    pub(crate) fn coerce(&mut self, from: &Ty, to: &Ty, node: NodeId, span: Span, end: u32) {
        let f = self.infer.shallow(from);
        if f == Ty::Never {
            return;
        }
        if self.infer.unify(&f, to) {
            return;
        }
        self.mismatch(&f, to, node, span, end);
    }

    pub(crate) fn mismatch(&mut self, from: &Ty, to: &Ty, node: NodeId, span: Span, end: u32) {
        let (a, b) = (self.infer.resolve(from), self.infer.resolve(to));
        let numeric = |t: &Ty, inf: &InferCtx| match t {
            Ty::Int(_) | Ty::Float(_) => true,
            Ty::Infer(i) => matches!(
                inf.kind_of(*i),
                crate::infer::VarKind::Int | crate::infer::VarKind::Float
            ),
            _ => false,
        };
        let (sa, sb) = (self.show(&a), self.show(&b));
        if numeric(&a, &self.infer) && numeric(&b, &self.infer) {
            // D1: không ép kiểu số ngầm
            let target = sb.clone();
            let fname = self.env.fname(node.file());
            let sugg = crate::collect::as_suggestion(&fname, end, &target);
            self.err_sugg(
                "E0302",
                node,
                span,
                format!("ép kiểu số ngầm bị cấm (D1): giá trị kiểu `{sa}` được dùng ở chỗ cần `{sb}`; viết `as {sb}` nếu đó là ý định"),
                sugg,
            );
        } else {
            self.err(
                "E0301",
                node,
                span,
                format!("sai kiểu: cần `{sb}`, gặp `{sa}`"),
            );
        }
    }

    // ---------------- hàm ----------------

    pub(crate) fn check_fn(
        env: &'e mut Env<'a>,
        f: &FnBody<'a>,
        impl_gens: &[GParam],
        self_ty: Option<Ty>,
    ) {
        let mut c = Checker::new(env, f.module, f.node, f.key.clone());
        c.ret = f.sig.ret.clone();
        c.cx = LowerCx {
            self_ty,
            lts: f.lts.clone(),
            no_elide: false,
            self_slots: Vec::new(),
        };
        c.self_kind = f.sig.self_kind;
        // entry: hàm `main` cấp cao của một tệp (`tệp::main`), không phải thành viên impl/trait
        c.entry = f.impl_idx.is_none()
            && f.body.is_some()
            && f.key.split("::").count() == 2
            && f.key.ends_with("::main");
        let all: Vec<&GParam> = impl_gens.iter().chain(f.sig.generics.iter()).collect();
        c.scope = scope_bounds(&all);
        c.bind_params(f);
        for ct in &f.ast_sig.contracts {
            c.contract(ct);
        }
        if let Some(body) = f.body {
            let rt = c.ret.clone();
            let bt = c.block(body, Some(&rt));
            // khối không có tail và không phân kỳ mà hàm trả kiểu khác `()`: E0307 thuộc M5 (cần luồng điều khiển)
            // (cả khi đuôi là `⎇` không `⇝`: nhánh thiếu cũng là đường không trả giá trị — M5)
            let open_end = body.tail.as_ref().is_none_or(|t| {
                matches!(
                    t.kind,
                    ExprKind::If {
                        else_branch: None,
                        ..
                    }
                )
            });
            let silent = open_end && bt.is_unit() && !rt.is_unit();
            if !silent {
                let end = body.tail.as_ref().map_or(body.span.end, |t| t.span.end);
                let sp = body.tail.as_ref().map_or(body.span, |t| t.span);
                c.coerce(&bt, &rt, body.id, sp, end);
            }
        }
        c.finish(&f.sig);
    }

    /// Chỉ kiểm kiểu CÁC HỢP ĐỒNG của một hàm prelude (thân của prelude không được kiểm; ADR-025 §6).
    pub(crate) fn check_fn_contracts(
        env: &'e mut Env<'a>,
        f: &FnBody<'a>,
        impl_gens: &[GParam],
        self_ty: Option<Ty>,
    ) {
        let mut c = Checker::new(env, f.module, f.node, f.key.clone());
        c.ret = f.sig.ret.clone();
        c.cx = LowerCx {
            self_ty,
            lts: f.lts.clone(),
            no_elide: false,
            self_slots: Vec::new(),
        };
        c.self_kind = f.sig.self_kind;
        let all: Vec<&GParam> = impl_gens.iter().chain(f.sig.generics.iter()).collect();
        c.scope = scope_bounds(&all);
        c.bind_params(f);
        for ct in &f.ast_sig.contracts {
            c.contract(ct);
        }
        c.finish(&f.sig);
    }

    fn bind_params(&mut self, f: &FnBody<'a>) {
        let mut i = 0usize;
        for p in &f.ast_sig.params {
            if let ParamKind::Named { mutable, name, .. } = &p.kind {
                let ty = f.sig.params.get(i).cloned().unwrap_or(Ty::Err);
                self.declare_binding(name, ty, *mutable, true);
                i += 1;
            }
        }
    }

    fn contract(&mut self, c: &Contract) {
        self.in_contract += 1;
        self.contract_inner(c);
        self.in_contract -= 1;
    }

    fn contract_inner(&mut self, c: &Contract) {
        match c.kind {
            ContractKind::Post => {
                // `result` ngầm (Q18)
                if let Some(&l) = self.env.res.decls.get(&c.id) {
                    self.locals.insert(
                        l,
                        LocalInfo {
                            ty: self.ret.clone(),
                            mutable: false,
                            name: "result".to_string(),
                            mut_hint: None,
                        },
                    );
                }
                self.expr_expect(&c.expr, &Ty::Bool);
            }
            ContractKind::Pre => self.expr_expect(&c.expr, &Ty::Bool),
            ContractKind::Variant => {
                let t = self.expr(&c.expr, None);
                let r = self.infer.shallow(&t);
                let ok = matches!(r, Ty::Int(_) | Ty::Err)
                    || matches!(r, Ty::Infer(i) if self.infer.kind_of(i) == crate::infer::VarKind::Int);
                if !ok {
                    let s = self.show(&t);
                    self.err(
                        "E0301",
                        c.expr.id,
                        c.expr.span,
                        format!("biến thể `↓` phải có kiểu số nguyên, gặp `{s}`"),
                    );
                }
            }
        }
    }

    /// Kiểm một biểu thức hằng/static: `value` phải ép được sang kiểu khai báo.
    pub(crate) fn check_const(
        env: &'e mut Env<'a>,
        key: String,
        module: DefId,
        owner: NodeId,
        ty: Ty,
        value: &Expr,
    ) {
        let mut c = Checker::new(env, module, owner, key);
        c.ret = ty.clone();
        c.expr_expect(value, &ty);
        c.finish(&FnSigInfo {
            node: owner,
            name: String::new(),
            generics: Vec::new(),
            self_kind: None,
            params: Vec::new(),
            ret: ty,
            effects: Vec::new(),
            span: value.span,
        });
    }

    // ---------------- kết thúc một thân ----------------

    /// `ty` có chứa `Cap<_>` ở đâu đó không?
    fn has_cap(&self, ty: &Ty) -> bool {
        let Some(cap) = self.lang(Lang::Cap) else {
            return false;
        };
        fn go(t: &Ty, cap: DefId) -> bool {
            match t {
                Ty::Adt(d, args) => *d == cap || args.iter().any(|a| go(a, cap)),
                Ty::Tuple(v) => v.iter().any(|a| go(a, cap)),
                Ty::Array(t, _) | Ty::Slice(t) | Ty::Ref(_, t) | Ty::RawPtr(_, t) => go(t, cap),
                Ty::Fn(p, r, _) => p.iter().any(|a| go(a, cap)) || go(r, cap),
                _ => false,
            }
        }
        go(&self.infer.resolve(ty), cap)
    }

    /// D28/D29: hàm trả `Cap<…>` mà không nhận `Cap<…>` nào là NGUỒN capability gốc; chỉ `main` được gọi nó.
    fn check_caps(&mut self) {
        let uses = std::mem::take(&mut self.cap_uses);
        if self.entry {
            return;
        }
        for u in uses {
            if self.has_cap(&u.ret) && !u.params.iter().any(|p| self.has_cap(p)) {
                let rt = self.show(&u.ret);
                self.err(
                    "E0310",
                    u.node,
                    u.span,
                    format!("thiếu capability: lời gọi này tạo `{rt}` từ hư không (nguồn capability gốc); chỉ `main` được gọi nó — các hàm khác phải nhận `Cap<…>` qua tham số (D28, D29)"),
                );
            }
        }
    }

    fn finish(&mut self, _sig: &FnSigInfo) {
        // nghĩa vụ trait lần 1 (trước khi mặc định hoá), rồi mặc định hoá literal, rồi lần cuối
        self.infer.default_literals();
        let obligs = std::mem::take(&mut self.oblig);
        for o in obligs {
            let ty = self.infer.resolve(&o.ty);
            if ty.has_infer() {
                self.unresolved(o.node, o.span);
                continue;
            }
            let scope = self.scope.clone();
            if !self
                .env
                .db
                .satisfies_bound(&mut self.infer, &ty, o.trait_def, &scope, 0)
            {
                let (s, tn) = (
                    self.show(&ty),
                    self.env.defs()[o.trait_def.index()].path.clone(),
                );
                self.err(
                    "E0309",
                    o.node,
                    o.span,
                    format!("kiểu `{s}` không thoả ràng buộc `{tn}` (không có impl)"),
                );
            }
        }
        self.check_caps();
        // vét cạn
        let ms = std::mem::take(&mut self.matches);
        for m in ms {
            let st = self.infer.resolve(&m.scrut);
            if st.has_infer() {
                self.unresolved(m.node, m.span);
                continue;
            }
            if let Some(w) = self.env.match_missing(&st, &m.arms) {
                let s = self.show(&st);
                let at = m.insert_at;
                let sugg = Suggestion::new(
                    "thêm nhánh bắt phần còn lại",
                    Applicability::MaybeIncorrect,
                    vec![gt_diag::Edit {
                        file: self.env.fname(m.node.file()),
                        span: Span::new(at, at),
                        replacement: ", _ ↦ …".to_string(),
                    }],
                );
                self.err_sugg(
                    "E0305",
                    m.node,
                    m.span,
                    format!("`⌥` không đầy đủ (D6) trên kiểu `{s}`: chưa phủ `{w}`; thêm nhánh `_` hoặc nhánh phủ phần còn lại"),
                    sugg,
                );
            }
        }
        // ghi kết quả
        let types = std::mem::take(&mut self.types);
        for (n, t) in types {
            let r = self.infer.resolve(&t);
            if r.has_infer() {
                // biến còn lại sau mặc định hoá: không suy ra được
                self.unresolved(n, Span::new(0, 0));
            }
            self.env.out_types.insert(n, r);
        }
        let locals = std::mem::take(&mut self.locals);
        for (l, info) in locals {
            let r = self.infer.resolve(&info.ty);
            if r.has_infer() {
                self.unresolved_local(&info.name);
            }
            self.env
                .out_locals
                .insert(l, (self.key.clone(), info.name, r));
        }
        let ms = std::mem::take(&mut self.methods);
        for (n, m) in ms {
            self.env.out_methods.insert(n, m);
        }
        let sites = std::mem::take(&mut self.sites);
        for (n, mut s) in sites {
            s.generics = s.generics.iter().map(|t| self.infer.resolve(t)).collect();
            if let CallTarget::TraitMethod { self_ty, .. } = &mut s.target {
                *self_ty = self.infer.resolve(self_ty);
            }
            self.env.out_sites.insert(n, s);
        }
        let calls = std::mem::take(&mut self.calls);
        for mut c in calls {
            c.args = c.args.iter().map(|t| self.infer.resolve(t)).collect();
            if let Some((_, _, t)) = &mut c.trait_method {
                *t = self.infer.resolve(t);
            }
            self.env.out_calls.push(c);
        }
    }

    fn unresolved(&mut self, node: NodeId, span: Span) {
        if self.unresolved_reported {
            return;
        }
        self.unresolved_reported = true;
        let key = self.key.clone();
        self.err(
            "E0301",
            node,
            span,
            format!("không suy ra được kiểu trong `{key}`; hãy thêm chú thích kiểu hoặc đối số kiểu tường minh"),
        );
    }

    fn unresolved_local(&mut self, name: &str) {
        if self.unresolved_reported {
            return;
        }
        self.unresolved_reported = true;
        let (key, owner) = (self.key.clone(), self.owner);
        self.err(
            "E0301",
            owner,
            Span::new(0, 0),
            format!("không suy ra được kiểu của `{name}` trong `{key}`; hãy thêm chú thích kiểu"),
        );
    }

    // ---------------- khối và câu lệnh ----------------

    pub(crate) fn block(&mut self, b: &Block, expect: Option<&Ty>) -> Ty {
        let mut diverges = false;
        for s in &b.stmts {
            if self.stmt(s) {
                diverges = true;
            }
        }
        let t = match &b.tail {
            Some(tail) => {
                // chưa ép ở đây: nơi gọi quyết định điểm ép (để báo lỗi đúng chỗ)
                match expect {
                    Some(e) => {
                        let e = e.clone();
                        self.expr(tail, Some(&e))
                    }
                    None => self.expr(tail, None),
                }
            }
            None if diverges => Ty::Never,
            None => Ty::unit(),
        };
        self.types.insert(b.id, t.clone());
        t
    }

    /// Trả về `true` nếu câu lệnh phân kỳ (kiểu `!`).
    fn stmt(&mut self, s: &Stmt) -> bool {
        match &s.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                init,
            } => {
                let annot = ty.as_ref().map(|t| {
                    let cx = self.cx.clone();
                    self.env.lower_ty(t, &cx)
                });
                let it = match &annot {
                    Some(a) => {
                        let a2 = a.clone();
                        let t = self.expr(init, Some(&a2));
                        self.coerce(&t, a, init.id, init.span, init.span.end);
                        a.clone()
                    }
                    None => self.expr(init, None),
                };
                self.sized(&it, init.id, init.span);
                let diverged = self.infer.shallow(&it) == Ty::Never;
                self.declare_binding(name, it, *mutable, true);
                diverged
            }
            StmtKind::Assign { target, value } => {
                let tt = self.expr(target, None);
                self.expr_expect(value, &tt);
                if !self.place_mutable(target) {
                    let n = self.place_name(target);
                    self.immutable_err(
                        target,
                        format!("gán vào `{n}` bất biến (D5): khai báo bằng `μ` (hoặc dùng tham chiếu `&!`) nếu cần thay đổi"),
                    );
                }
                false
            }
            StmtKind::Expr { expr, semi } => {
                let t = self.expr(expr, None);
                let r = self.infer.shallow(&t);
                if r == Ty::Never {
                    return true;
                }
                if !*semi {
                    // câu lệnh dạng khối không có `;`: phải có kiểu `()`
                    let unit = Ty::unit();
                    self.coerce(&t, &unit, expr.id, expr.span, expr.span.end);
                }
                false
            }
            StmtKind::Return(e) => {
                match e {
                    Some(e) => {
                        let rt = self.ret.clone();
                        self.expr_expect(e, &rt);
                    }
                    None => {
                        let rt = self.infer.resolve(&self.ret);
                        if !rt.is_unit() && rt != Ty::Err {
                            let s2 = self.show(&rt);
                            self.err(
                                "E0301",
                                s.id,
                                s.span,
                                format!("`⏎` không có giá trị trong hàm trả `{s2}`"),
                            );
                        }
                    }
                }
                true
            }
            StmtKind::Break | StmtKind::Continue => {
                if self.loops.is_empty() {
                    self.err(
                        "E0301",
                        s.id,
                        s.span,
                        "`⊠`/`⊡` chỉ dùng được trong vòng lặp".to_string(),
                    );
                } else if matches!(s.kind, StmtKind::Break)
                    && let Some(l) = self.loops.last_mut()
                {
                    *l = true;
                }
                true
            }
        }
    }

    pub(crate) fn sized(&mut self, t: &Ty, node: NodeId, span: Span) {
        let r = self.infer.resolve(t);
        if matches!(r, Ty::Slice(_) | Ty::Str) {
            let s = self.show(&r);
            self.err(
                "E0301",
                node,
                span,
                format!("kiểu `{s}` không biết kích thước; chỉ dùng được sau `&`/`&!`"),
            );
        }
    }

    // ---------------- chỗ gán / mượn ----------------

    fn place_name(&self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Path(p) => match self.env.res.uses.get(&p.id) {
                Some(Res::Local(l)) => self
                    .locals
                    .get(l)
                    .map_or("?".to_string(), |i| i.name.clone()),
                Some(Res::SelfValue) => "self".to_string(),
                _ => "?".to_string(),
            },
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => self.place_name(base),
            ExprKind::Unary { operand, .. } => self.place_name(operand),
            _ => "?".to_string(),
        }
    }

    /// Chỗ (place) này có thể thay đổi không? (D5)
    pub(crate) fn place_mutable(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Path(p) => match self.env.res.uses.get(&p.id) {
                Some(Res::Local(l)) => self.locals.get(l).is_none_or(|i| i.mutable),
                Some(Res::SelfValue) => false,
                _ => false,
            },
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => {
                match self.types.get(&base.id).map(|t| self.infer.resolve(t)) {
                    Some(Ty::Ref(m, _)) => m,
                    Some(Ty::Err) => true,
                    _ => self.place_mutable(base),
                }
            }
            ExprKind::Unary {
                op: UnaryOp::Deref,
                operand,
            } => match self.types.get(&operand.id).map(|t| self.infer.resolve(t)) {
                Some(Ty::Ref(m, _) | Ty::RawPtr(m, _)) => m,
                // deref của kiểu không phải con trỏ đã là lỗi kiểu riêng (E0301): không kéo thêm lỗi bất biến
                _ => true,
            },
            _ => false,
        }
    }

    pub(crate) fn is_place(e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Path(_) => true,
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => Self::is_place(base),
            ExprKind::Unary {
                op: UnaryOp::Deref, ..
            } => true,
            _ => false,
        }
    }

    pub(crate) fn self_param_ty(&self) -> Ty {
        let st = self.cx.self_ty.clone().unwrap_or(Ty::Err);
        match self.self_kind {
            Some(SelfKind::Value) => st,
            Some(SelfKind::Ref) => Ty::Ref(false, Box::new(st)),
            Some(SelfKind::RefMut) => Ty::Ref(true, Box::new(st)),
            None => Ty::Err,
        }
    }
}
