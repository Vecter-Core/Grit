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
    /// vị trí lời gọi (cho chẩn đoán E0318)
    pub span: Span,
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
    /// đã báo E0318 (Cell/Mutex) trong hàm này
    pub cell_drop_reported: bool,
    /// đã báo E0318 trong hàm này
    pub vec_drop_reported: bool,
    /// nút của biểu thức đang đứng ở vị trí NGƯỜI ĐƯỢC GỌI (`callee` của `Call`): đường dẫn tới intrinsic chỉ được phép ở đây (gọi trực tiếp)
    pub callee_pos: Option<NodeId>,
    /// `==`/`≠` có toán hạng là biến suy diễn CHƯA giải lúc gặp toán tử: kiểm lại ở `finish` (FB-C19)
    pub eq_deferred: Vec<(NodeId, Span, Ty)>,
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
    /// đang kiểm biểu thức `□`/`■` (không có hàm bao quanh để `?` thoát)
    pub in_const: bool,
    /// chế độ tham chiếu của pattern đang kiểm (ADR-040): `Some(duy nhất?)`
    pub pat_ref: Option<bool>,
    pub pat_modes: Vec<(NodeId, bool)>,
    /// ép unsize ghi nhận trong thân này (GÓI 12)
    pub unsize: Vec<(NodeId, bool)>,
    /// đang bind biến của pattern đã lỗi: biến nhận kiểu `Err` (chặn lỗi dây chuyền)
    pub pat_poison: bool,
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
            vec_drop_reported: false,
            cell_drop_reported: false,
            callee_pos: None,
            eq_deferred: Vec::new(),
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
            in_const: false,
            pat_ref: None,
            pat_modes: Vec::new(),
            unsize: Vec::new(),
            pat_poison: false,
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

    /// Ép UNSIZE (như Rust, GÓI 12) tại điểm ép kiểu của MỘT NÚT BIỂU THỨC: `&[T; N]` → `&[T]`, `&![T; N]` → `&![T]` hoặc `&[T]`.
    /// Chỉ khi cả hai phía đã là tham chiếu cụ thể (không đoán từ biến suy luận) và kiểu phần tử hợp nhất được. MIR hạ thành `&(*e)[0..]`
    /// (cùng dạng với lát con người dùng viết, nên mọi backend đã hỗ trợ). Không bao giờ ép `&[T; N]` → `&![T]` (thêm quyền ghi).
    pub(crate) fn try_unsize(&mut self, from: &Ty, to: &Ty, node: NodeId) -> bool {
        let (f, t) = (self.infer.shallow(from), self.infer.shallow(to));
        let (Ty::Ref(fm, fa), Ty::Ref(tm, ts)) = (&f, &t) else {
            return false;
        };
        if *tm && !*fm {
            return false;
        }
        let (Ty::Array(fe, _), Ty::Slice(te)) = (self.infer.shallow(fa), self.infer.shallow(ts))
        else {
            return false;
        };
        if !self.infer.unify(&fe, &te) {
            return false;
        }
        self.unsize.push((node, *tm));
        true
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
        // FB-C18: entry phải KHÔNG tham số (như `find_main` của codegen/runner); `main(x)` ở tệp khác không phải entry nên không được đúc `Cap`
        c.entry = f.impl_idx.is_none()
            && f.body.is_some()
            && f.sig.params.is_empty()
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
        c.in_const = true;
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
            has_contracts: false,
        });
    }

    // ---------------- kết thúc một thân ----------------

    /// `ty` có chứa `Cap<_>` ở đâu đó không?
    fn has_cap(&self, ty: &Ty) -> bool {
        let Some(cap) = self.lang(Lang::Cap) else {
            return false;
        };
        // đi vào trường của struct/enum người dùng (FB-C18b: `W { c: Cap<Fs> }` vẫn là capability); `seen` chặn kiểu đệ quy
        fn go(db: &crate::db::TypeDb, t: &Ty, cap: DefId, seen: &mut Vec<DefId>) -> bool {
            match t {
                Ty::Adt(d, args) => {
                    if *d == cap || args.iter().any(|a| go(db, a, cap, seen)) {
                        return true;
                    }
                    if seen.contains(d) {
                        return false;
                    }
                    seen.push(*d);
                    let Some(adt) = db.adts.get(d) else {
                        // ADT không có trong db (không nhất quán): KHÔNG được coi là "không phải capability" (E0310 sẽ bị bỏ qua) ⇒ bảo thủ (FAIL_OPEN_AUDIT G2.1.b)
                        return true;
                    };
                    let fields: Vec<Ty> = match &adt.kind {
                        crate::collect::AdtKind::Struct(fs) => {
                            fs.iter().map(|f| f.ty.clone()).collect()
                        }
                        crate::collect::AdtKind::Enum(vs) => {
                            vs.iter().flat_map(|v| v.fields.clone()).collect()
                        }
                    };
                    fields.iter().any(|f| {
                        go(
                            db,
                            &crate::traits::subst_with(f, &adt.generics, args),
                            cap,
                            seen,
                        )
                    })
                }
                Ty::Tuple(v) => v.iter().any(|a| go(db, a, cap, seen)),
                Ty::Array(t, _) | Ty::Slice(t) | Ty::Ref(_, t) | Ty::RawPtr(_, t) => {
                    go(db, t, cap, seen)
                }
                Ty::Fn(p, r, _) => p.iter().any(|a| go(db, a, cap, seen)) || go(db, r, cap, seen),
                _ => false,
            }
        }
        go(&self.env.db, &self.infer.resolve(ty), cap, &mut Vec::new())
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
        for (node, span, t) in std::mem::take(&mut self.eq_deferred) {
            self.check_eq_operand(node, span, &t);
        }
        let obligs = std::mem::take(&mut self.oblig);
        for o in obligs {
            let ty = self.infer.resolve(&o.ty);
            if ty.has_infer() {
                self.unresolved(o.node, o.span);
                continue;
            }
            let scope = self.scope.clone();
            let auto = if self.env.db.lang.get(&crate::collect::Lang::Send) == Some(&o.trait_def) {
                Some(crate::send::Auto::Send)
            } else if self.env.db.lang.get(&crate::collect::Lang::Sync) == Some(&o.trait_def) {
                Some(crate::send::Auto::Sync)
            } else {
                None
            };
            if let Some(which) = auto {
                if let Err(why) = self.env.auto_like(&ty, &scope, which) {
                    let s = self.show(&ty);
                    let msg = match which {
                        crate::send::Auto::Send => format!(
                            "kiểu `{s}` không `Send` nên không chuyển được sang luồng khác: {why}"
                        ),
                        crate::send::Auto::Sync => format!(
                            "kiểu `{s}` không `Sync` nên không chia sẻ được giữa các luồng: {why}"
                        ),
                    };
                    let code = if crate::send::is_internal(&why) {
                        "E9901"
                    } else {
                        "E0501"
                    };
                    self.err(code, o.node, o.span, msg);
                }
                continue;
            }
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
        for (n, m) in std::mem::take(&mut self.pat_modes) {
            self.env.out_pat_modes.insert(n, m);
        }
        for (n, m) in std::mem::take(&mut self.unsize) {
            self.env.out_unsize.insert(n, m);
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
        let scope = self.scope.clone();
        for mut c in calls {
            c.args = c.args.iter().map(|t| self.infer.resolve(t)).collect();
            self.check_vec_element(&c, &scope);
            if let Some((_, _, t)) = &mut c.trait_method {
                *t = self.infer.resolve(t);
            }
            self.env.out_calls.push(c);
        }
    }

    /// E0318 (ADR-032 phụ lục A, 3b): thành viên của `Vec<'a, T>` chỉ dùng được khi `T` KHÔNG cần drop (arena không chạy destructor).
    /// Tham số kiểu `T` chưa biết coi là cần drop trừ khi có ràng buộc `Copy` trong phạm vi (fail-closed).
    fn check_vec_element(&mut self, c: &CallRec, scope: &[Bound]) {
        let Some(callee) = c.callee else {
            return;
        };
        let Some(imp) = self
            .env
            .db
            .impls
            .iter()
            .find(|i| i.members.iter().any(|m| m.node == callee))
        else {
            return;
        };
        let Ty::Adt(d, _) = &imp.self_ty else {
            return;
        };
        let lang = self.env.db.adts.get(d).and_then(|a| a.lang);
        if lang == Some(Lang::Arena) {
            // ADR-C18: `Arena::alloc(v)` không bao giờ chạy destructor ⇒ giá trị có `drop` người dùng / `Owned` (trả khối heap) bị âm thầm
            // bỏ drop. Từ chối như Vec (E0318) khi kiểu đã biết; tham số kiểu thì chưa biết (KNOWN_LIMITATIONS: chỉ rò rỉ, không mất an toàn).
            if let Some(t) = c.args.first()
                && self.env.db.holds_glue_source(t)
            {
                let shown = t.render(self.env.defs());
                self.err(
                    "E0318",
                    c.caller,
                    c.span,
                    format!("`Arena::alloc` với `{shown}`: giá trị có Drop/`Owned` sẽ không bao giờ được drop (Arena không chạy destructor)"),
                );
            }
            return;
        }
        if matches!(lang, Some(Lang::Cell | Lang::Mutex)) {
            // FB-C42 (Gemini G-05): glue không đi vào được Cell/Mutex chứa giá trị có Drop/`Owned` — elaborate chỉ bắt kiểu CỤ THỂ, nên trong hàm
            // generic phải chặn ở đây: `T` còn là tham số không ràng buộc `Copy` (có thể là `Owned`/`Secret` khi instance hoá). Kiểu cụ thể do elaborate bắt
            // (và `■ Mutex<Secret>` hợp lệ: static không có drop ⇒ chỉ S203).
            if let Some(t) = c.args.first()
                && t.has_param()
                && !self.env.db.is_copy_resolved(t, scope)
                && !self.cell_drop_reported
            {
                self.cell_drop_reported = true;
                let shown = t.render(self.env.defs());
                self.err(
                    "E0318",
                    c.caller,
                    c.span,
                    format!("`Cell`/`Mutex` chứa `{shown}`: giá trị có Drop/`Owned` (hoặc tham số kiểu không `Copy`) không drop được qua kiểu lang đục"),
                );
            }
            return;
        }
        if lang != Some(Lang::Vec) {
            return;
        }
        let Some(t) = c.args.first() else {
            return;
        };
        let nd = match self.env.db.needs_drop_checked(t) {
            Ok(b) => b,
            Err(why) => {
                // db không nhất quán: KHÔNG được coi là "không cần drop" (E0318 sẽ bị bỏ qua) ⇒ ICE
                self.err("E9901", c.caller, c.span, why);
                return;
            }
        };
        if !self.vec_drop_reported && nd && !self.env.db.is_copy_resolved(t, scope) {
            self.vec_drop_reported = true; // một chẩn đoán cho mỗi hàm (tránh lặp ở `new`/`push`/…)
            let shown = t.render(self.env.defs());
            self.err(
                "E0318",
                c.caller,
                c.span,
                format!("`Vec<_, {shown}>`: `{shown}` cần drop (Drop tường minh/Arena/tham số kiểu không Copy) nhưng Vec trên Arena không chạy destructor"),
            );
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
                        if !self.try_unsize(&t, a, init.id) {
                            self.coerce(&t, a, init.id, init.span, init.span.end);
                        }
                        a.clone()
                    }
                    None => self.expr(init, None),
                };
                self.sized(&it, init.id, init.span);
                let diverged = self.infer.shallow(&it) == Ty::Never;
                self.declare_binding(name, it, *mutable, true);
                diverged
            }
            StmtKind::LetPat { pat, ty, init } => {
                // ADR-P2-10: kiểu vế phải như `≔` thường; pattern kiểm bằng ĐÚNG bộ `pat` + vét cạn của `⌥` với MỘT nhánh (không vét cạn ⇒ E0305)
                let annot = ty.as_ref().map(|t| {
                    let cx = self.cx.clone();
                    self.env.lower_ty(t, &cx)
                });
                // pattern phá cấu trúc không kết hợp được với ép UNSIZE (`&[T; N]` → `&[T]`) hay kiểu `!` nên không có nhánh try_unsize/phân kỳ ở đây (khác `≔` thường)
                let it = match &annot {
                    Some(a) => {
                        let a2 = a.clone();
                        let t = self.expr(init, Some(&a2));
                        // FB-P2-01/F5: `coerce` bỏ qua `!` nên vế phải phân kỳ lọt qua rồi M5 ICE (pattern tuple trên kiểu `!`); không chú thích thì đã E0301 ⇒ nhất quán
                        if self.infer.shallow(&t) == Ty::Never {
                            self.err(
                                "E0301",
                                init.id,
                                init.span,
                                "vế phải của `let` phân rã có kiểu `!` (phân kỳ): không có giá trị để phân rã".to_string(),
                            );
                        }
                        self.coerce(&t, a, init.id, init.span, init.span.end);
                        a.clone()
                    }
                    None => self.expr(init, None),
                };
                self.sized(&it, init.id, init.span);
                let ir = self.pat(pat, &it);
                let scrut = match self.infer.shallow(&it) {
                    Ty::Ref(_, inner) if self.destructures(pat) => *inner,
                    _ => it,
                };
                self.matches.push(MatchCheck {
                    node: s.id,
                    span: s.span,
                    scrut,
                    arms: vec![ir],
                    insert_at: s.span.end.saturating_sub(1),
                });
                false
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
