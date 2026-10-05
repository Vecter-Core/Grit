//! Duyệt AST: phân giải đường dẫn (biểu thức, kiểu, pattern), biến cục bộ, generics, `Self`/`self`, D7 (ADR-017 d).
use crate::model::{DefId, DefKind, LocalId, Res};
use crate::tables::{Found, Tables, Walked, seg_name};
use gt_ast::*;
use gt_span::{FileId, Span};

struct Local {
    name: String,
    id: LocalId,
    file: FileId,
    span: Span,
}

struct Gen {
    name: String,
    id: (NodeId, u32),
    file: FileId,
    span: Span,
}

pub(crate) struct Walker<'t> {
    t: &'t mut Tables,
    module: DefId,
    file: FileId,
    /// Khoá logic của item đang duyệt (đường dẫn đủ tên); dùng cho chẩn đoán logic và dạng chuẩn.
    site: String,
    ordinal: u32,
    owner: NodeId,
    locals: Vec<Vec<Local>>,
    next_local: u32,
    generics: Vec<Gen>,
    in_impl: bool,
    has_self: bool,
}

/// Duyệt mọi item của một module (đệ quy vào `▦`).
pub(crate) fn walk_items(t: &mut Tables, items: &[Item], module: DefId, file: FileId) {
    for item in items {
        match &item.kind {
            ItemKind::Mod(m) => {
                if let Some(&d) = t.item_def.get(&item.id) {
                    walk_items(t, &m.items, d, file);
                }
            }
            ItemKind::Use(_) => {}
            _ => {
                let mut w = Walker::new(t, module, file, item);
                w.item(item);
            }
        }
    }
}

fn res_text(t: &Tables, r: &Res) -> String {
    match r {
        Res::Def(d) => t.defs[d.index()].path.clone(),
        Res::Local(l) => format!("local#{}", l.index),
        Res::Generic(_, i) => format!("generic#{i}"),
        Res::SelfType => "Self".to_string(),
        Res::SelfValue => "self".to_string(),
        Res::Assoc { base, member } => format!("assoc({})::{member}", res_text(t, base)),
        Res::Err => "<lỗi>".to_string(),
    }
}

impl<'t> Walker<'t> {
    fn new(t: &'t mut Tables, module: DefId, file: FileId, item: &Item) -> Walker<'t> {
        let site = item_key(t, module, file, item);
        Walker {
            t,
            module,
            file,
            site,
            ordinal: 0,
            owner: item.id,
            locals: vec![Vec::new()],
            next_local: 0,
            generics: Vec::new(),
            in_impl: false,
            has_self: false,
        }
    }

    // ---------------- tiện ích ----------------

    fn err(&mut self, code: &str, span: Span, msg: String) {
        if let Some(d) = self.t.mk(code, self.file, span, msg) {
            let site = self.site.clone();
            self.t.push(code, d, &site);
        }
    }

    fn err_with(
        &mut self,
        code: &str,
        span: Span,
        msg: String,
        label: &str,
        other: (FileId, Span),
        other_msg: &str,
    ) {
        if let Some(d) = self.t.mk(code, self.file, span, msg) {
            let fname = self.t.fname(other.0);
            let d = d
                .with_span_label(label)
                .with_label(&fname, other.1, other_msg);
            let site = self.site.clone();
            self.t.push(code, d, &site);
        }
    }

    fn record(&mut self, node: NodeId, text: &str, res: Res) {
        self.ordinal += 1;
        let line = format!(
            "{} #{} {} -> {}",
            self.site,
            self.ordinal,
            text,
            res_text(self.t, &res)
        );
        self.t.canon.push(line);
        self.t.res.insert(node, res);
    }

    fn push_scope(&mut self) {
        self.locals.push(Vec::new());
    }

    fn pop_scope(&mut self) {
        self.locals.pop();
    }

    // ---------------- tên cấp module ----------------

    /// Tên `n` thấy được ở cấp module (bảng module hiện tại → module-tệp → prelude): vị trí định nghĩa đầu tiên.
    fn module_scope_site(&self, n: &str) -> Option<(FileId, Span)> {
        let site_of = |t: &Tables, d: DefId| (t.defs[d.index()].file, t.defs[d.index()].name_span);
        for container in [self.module, self.t.root, self.t.prelude] {
            if let Some(entries) = self.t.tables[container.index()].get(n) {
                return match entries.first()? {
                    crate::tables::Entry::Def(d) => Some(site_of(self.t, *d)),
                    crate::tables::Entry::Use(i) => {
                        Some((self.t.uses[*i].file, self.t.uses[*i].span))
                    }
                };
            }
        }
        None
    }

    /// Tra tên trần ở cấp module (không kiểm khả kiến): module hiện tại → module-tệp → prelude.
    fn module_scope_lookup(&mut self, n: &str) -> Found {
        for container in [self.module, self.t.root, self.t.prelude] {
            match self.t.lookup_in(container, n, None) {
                Found::Missing => {}
                other => return other,
            }
        }
        Found::Missing
    }

    // ---------------- biến cục bộ & generics (D7) ----------------

    fn find_local(&self, n: &str) -> Option<&Local> {
        self.locals
            .iter()
            .rev()
            .flat_map(|s| s.iter().rev())
            .find(|l| l.name == n)
    }

    fn find_generic(&self, n: &str) -> Option<&Gen> {
        self.generics.iter().rev().find(|g| g.name == n)
    }

    /// Kiểm luật D7 cho một tên mới; báo E0205 nếu che khuất. Trả về `true` nếu hợp lệ.
    fn check_shadow(&mut self, name: &str, span: Span, what: &str) -> bool {
        let prior: Option<(FileId, Span, &str)> = if let Some(l) = self.find_local(name) {
            Some((l.file, l.span, "biến cục bộ/tham số"))
        } else if let Some(g) = self.find_generic(name) {
            Some((g.file, g.span, "tham số generics"))
        } else {
            self.module_scope_site(name)
                .map(|(f, s)| (f, s, "tên cấp module (item, nhập hoặc prelude)"))
        };
        match prior {
            None => true,
            Some((f, s, kind)) => {
                let msg = format!("{what} `{name}` che khuất {kind} cùng tên (D7: cấm shadowing)");
                self.err_with("E0205", span, msg, "tên mới", (f, s), "bị che khuất ở đây");
                false
            }
        }
    }

    fn introduce_named(&mut self, name: &str, span: Span, node: NodeId, what: &str) -> LocalId {
        self.check_shadow(name, span, what);
        let id = LocalId {
            owner: self.owner,
            index: self.next_local,
        };
        self.next_local += 1;
        if let Some(scope) = self.locals.last_mut() {
            scope.push(Local {
                name: name.to_string(),
                id,
                file: self.file,
                span,
            });
        }
        self.t.decls.insert(node, id);
        let line = format!("{} decl {} -> local#{}", self.site, name, id.index);
        self.t.canon.push(line);
        id
    }

    fn introduce(&mut self, ident: &Ident, what: &str) -> LocalId {
        self.introduce_named(&ident.name, ident.span, ident.id, what)
    }

    /// Đưa tham số generics của item vào phạm vi (kiểm trùng E0202 và che khuất E0205), rồi phân giải các ràng buộc.
    fn enter_generics(&mut self, g: &Option<Generics>, owner: NodeId) {
        let Some(g) = g else { return };
        let mut seen: Vec<(String, Span)> = Vec::new();
        let mut idx = 0u32;
        let mut bounds: Vec<&Path> = Vec::new();
        for p in &g.params {
            if let GenericParamKind::Type { name, bounds: b } = &p.kind {
                if let Some((_, first)) = seen.iter().find(|(n, _)| *n == name.name) {
                    let first = *first;
                    self.err_with(
                        "E0202",
                        name.span,
                        format!("tham số generics `{}` được khai báo nhiều lần", name.name),
                        "khai báo lặp",
                        (self.file, first),
                        "khai báo đầu tiên",
                    );
                } else {
                    seen.push((name.name.clone(), name.span));
                    self.check_shadow(&name.name, name.span, "tham số generics");
                    self.generics.push(Gen {
                        name: name.name.clone(),
                        id: (owner, idx),
                        file: self.file,
                        span: name.span,
                    });
                    idx += 1;
                }
                bounds.extend(b.iter());
            }
        }
        for b in bounds {
            self.resolve_path(b);
        }
    }

    // ---------------- đường dẫn ----------------

    fn resolve_path(&mut self, path: &Path) -> Res {
        let segs: Vec<String> = path.segments.iter().map(|s| seg_name(&s.kind)).collect();
        let spans: Vec<Span> = path.segments.iter().map(|s| s.span).collect();
        let res = self.resolve_path_inner(path, &segs, &spans);
        self.record(path.id, &segs.join("::"), res.clone());
        res
    }

    fn with_assoc(base: Res, segs: &[String], from: usize) -> Res {
        if segs.len() > from {
            Res::Assoc {
                base: Box::new(base),
                member: segs[from..].join("::"),
            }
        } else {
            base
        }
    }

    fn resolve_path_inner(&mut self, path: &Path, segs: &[String], spans: &[Span]) -> Res {
        let Some(first) = path.segments.first() else {
            return Res::Err;
        };
        match &first.kind {
            SegmentKind::SelfType => {
                if !self.in_impl {
                    self.err(
                        "E0201",
                        first.span,
                        "`Self` chỉ dùng được bên trong `⊏` hoặc `⊐`".to_string(),
                    );
                    return Res::Err;
                }
                Self::with_assoc(Res::SelfType, segs, 1)
            }
            SegmentKind::SelfValue => {
                if !self.has_self {
                    self.err(
                        "E0201",
                        first.span,
                        "`self` chỉ dùng được trong hàm có tham số `self`".to_string(),
                    );
                    return Res::Err;
                }
                Self::with_assoc(Res::SelfValue, segs, 1)
            }
            SegmentKind::Name(n) => {
                if segs.len() == 1
                    && let Some(l) = self.find_local(n)
                {
                    return Res::Local(l.id);
                }
                if let Some(g) = self.find_generic(n) {
                    let base = Res::Generic(g.id.0, g.id.1);
                    return Self::with_assoc(base, segs, 1);
                }
                let start = match self.module_scope_lookup(n) {
                    Found::Def(d) => d,
                    Found::Silent => return Res::Err,
                    Found::Hidden { .. } | Found::Missing => {
                        let site = self.site.clone();
                        self.t.err_first_missing(self.file, spans[0], n, &site);
                        return Res::Err;
                    }
                };
                match self.t.walk_rest(start, &segs[1..], self.module) {
                    Ok(Walked::Def(d)) => {
                        self.t.note_edge(self.module, d, self.file, path.span);
                        Res::Def(d)
                    }
                    Ok(Walked::Assoc(base, at)) => {
                        self.t.note_edge(self.module, base, self.file, path.span);
                        Self::with_assoc(Res::Def(base), segs, 1 + at)
                    }
                    Err(e) => {
                        let site = self.site.clone();
                        self.t.report_walk_err(self.file, spans, 1, &e, &site);
                        Res::Err
                    }
                }
            }
        }
    }

    // ---------------- item ----------------

    fn item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Fn(f) => self.fn_like(&f.sig, Some(&f.body), item.id, false),
            ItemKind::Struct(s) => {
                self.enter_generics(&s.generics, item.id);
                let mut seen: Vec<(String, Span)> = Vec::new();
                for f in &s.fields {
                    if let Some((_, first)) = seen.iter().find(|(n, _)| *n == f.name.name) {
                        let first = *first;
                        self.err_with(
                            "E0202",
                            f.name.span,
                            format!(
                                "trường `{}` được khai báo nhiều lần trong struct",
                                f.name.name
                            ),
                            "khai báo lặp",
                            (self.file, first),
                            "khai báo đầu tiên",
                        );
                    } else {
                        seen.push((f.name.name.clone(), f.name.span));
                    }
                    self.ty(&f.ty);
                }
            }
            ItemKind::Enum(e) => {
                self.enter_generics(&e.generics, item.id);
                for v in &e.variants {
                    for t in &v.fields {
                        self.ty(t);
                    }
                }
            }
            ItemKind::Trait(t) => {
                self.enter_generics(&t.generics, item.id);
                let mut seen: Vec<(String, Span)> = Vec::new();
                let base = self.site.clone();
                for m in &t.members {
                    self.member_dup(&mut seen, &m.sig.name, "thành viên trait");
                    self.site = format!("{base}::{}", m.sig.name.name);
                    self.ordinal = 0;
                    let keep = self.generics.len();
                    self.fn_like(&m.sig, None, m.id, true);
                    self.generics.truncate(keep);
                }
            }
            ItemKind::Impl(i) => {
                self.enter_generics(&i.generics, item.id);
                if let Some(tp) = &i.trait_path {
                    self.resolve_path(tp);
                }
                self.ty(&i.self_ty);
                let mut seen: Vec<(String, Span)> = Vec::new();
                let base = self.site.clone();
                for m in &i.members {
                    self.member_dup(&mut seen, &m.func.sig.name, "thành viên impl");
                    self.site = format!("{base}::{}", m.func.sig.name.name);
                    self.ordinal = 0;
                    let keep = self.generics.len();
                    self.fn_like(&m.func.sig, Some(&m.func.body), m.id, true);
                    self.generics.truncate(keep);
                }
            }
            ItemKind::TypeAlias(a) => {
                self.enter_generics(&a.generics, item.id);
                self.ty(&a.ty);
            }
            ItemKind::Const(c) => {
                self.ty(&c.ty);
                self.expr(&c.value);
            }
            ItemKind::Static(c) => {
                self.ty(&c.ty);
                self.expr(&c.value);
            }
            ItemKind::Mod(_) | ItemKind::Use(_) => {}
        }
    }

    fn member_dup(&mut self, seen: &mut Vec<(String, Span)>, name: &Ident, what: &str) {
        if let Some((_, first)) = seen.iter().find(|(n, _)| *n == name.name) {
            let first = *first;
            self.err_with(
                "E0202",
                name.span,
                format!("{what} `{}` được khai báo nhiều lần", name.name),
                "khai báo lặp",
                (self.file, first),
                "khai báo đầu tiên",
            );
        } else {
            seen.push((name.name.clone(), name.span));
        }
    }

    /// Hàm tự do, thành viên trait (không thân) hoặc thành viên impl.
    fn fn_like(&mut self, sig: &FnSig, body: Option<&Block>, owner: NodeId, in_impl: bool) {
        self.owner = owner;
        self.locals = vec![Vec::new()];
        self.next_local = 0;
        self.in_impl = in_impl;
        self.has_self = sig
            .params
            .iter()
            .any(|p| matches!(p.kind, ParamKind::SelfParam(_)));
        self.enter_generics(&sig.generics, owner);
        let mut seen: Vec<(String, Span)> = Vec::new();
        for p in &sig.params {
            if let ParamKind::Named { name, ty, .. } = &p.kind {
                self.ty(ty);
                if let Some((_, first)) = seen.iter().find(|(n, _)| *n == name.name) {
                    let first = *first;
                    self.err_with(
                        "E0202",
                        name.span,
                        format!("tham số `{}` được khai báo nhiều lần", name.name),
                        "khai báo lặp",
                        (self.file, first),
                        "khai báo đầu tiên",
                    );
                } else {
                    seen.push((name.name.clone(), name.span));
                    self.introduce(name, "tham số");
                }
            }
        }
        if let Some(r) = &sig.ret {
            self.ty(r);
        }
        for c in &sig.contracts {
            if c.kind == ContractKind::Post {
                // `result` ngầm CHỈ trong ⊲ (OPEN_QUESTIONS Q18)
                self.push_scope();
                self.introduce_named("result", c.span, c.id, "biến `result` của hậu điều kiện");
                self.expr(&c.expr);
                self.pop_scope();
            } else {
                self.expr(&c.expr);
            }
        }
        if let Some(b) = body {
            self.block(b);
        }
    }

    // ---------------- kiểu ----------------

    fn ty(&mut self, t: &Type) {
        match &t.kind {
            TypeKind::Prim(_) | TypeKind::Never => {}
            TypeKind::SelfType => {
                if self.in_impl {
                    self.record(t.id, "Self", Res::SelfType);
                } else {
                    self.err(
                        "E0201",
                        t.span,
                        "`Self` chỉ dùng được bên trong `⊏` hoặc `⊐`".to_string(),
                    );
                    self.record(t.id, "Self", Res::Err);
                }
            }
            TypeKind::Path { path, args } => {
                self.resolve_path(path);
                for a in args {
                    self.ty(a);
                }
            }
            TypeKind::Ref { inner, .. }
            | TypeKind::Slice(inner)
            | TypeKind::RawPtr { inner, .. } => self.ty(inner),
            TypeKind::Array { elem, len } => {
                self.ty(elem);
                self.expr(len);
            }
            TypeKind::Tuple(v) => {
                for x in v {
                    self.ty(x);
                }
            }
            TypeKind::Fn { params, ret, .. } => {
                for x in params {
                    self.ty(x);
                }
                if let Some(r) = ret {
                    self.ty(r);
                }
            }
        }
    }

    // ---------------- khối, câu lệnh, biểu thức ----------------

    fn block(&mut self, b: &Block) {
        self.push_scope();
        for s in &b.stmts {
            self.stmt(s);
        }
        if let Some(t) = &b.tail {
            self.expr(t);
        }
        self.pop_scope();
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Let { name, ty, init, .. } => {
                if let Some(t) = ty {
                    self.ty(t);
                }
                self.expr(init);
                self.introduce(name, "biến");
            }
            StmtKind::Assign { target, value } => {
                self.expr(target);
                self.expr(value);
            }
            StmtKind::Expr { expr, .. } => self.expr(expr),
            StmtKind::Return(e) => {
                if let Some(e) = e {
                    self.expr(e);
                }
            }
            StmtKind::Break | StmtKind::Continue => {}
        }
    }

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Lit(_) => {}
            ExprKind::Path(p) => {
                self.resolve_path(p);
            }
            ExprKind::Unary { operand, .. } => self.expr(operand),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            ExprKind::Cast { expr, ty } => {
                self.expr(expr);
                self.ty(ty);
            }
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                for a in args {
                    self.expr(a);
                }
            }
            ExprKind::Field { base, .. } => self.expr(base),
            ExprKind::Index { base, index } => {
                self.expr(base);
                self.expr(index);
            }
            ExprKind::Range { start, end, .. } => {
                if let Some(s) = start {
                    self.expr(s);
                }
                if let Some(x) = end {
                    self.expr(x);
                }
            }
            ExprKind::Tuple(v) | ExprKind::Array(v) => {
                for x in v {
                    self.expr(x);
                }
            }
            ExprKind::ArrayRepeat { value, len } => {
                self.expr(value);
                self.expr(len);
            }
            ExprKind::StructLit { path, fields } => {
                self.resolve_path(path);
                for f in fields {
                    self.expr(&f.value);
                }
            }
            ExprKind::Block(b) => self.block(b),
            ExprKind::If {
                cond,
                then_block,
                else_branch,
            } => {
                self.expr(cond);
                self.block(then_block);
                if let Some(x) = else_branch {
                    self.expr(x);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.expr(scrutinee);
                for a in arms {
                    self.push_scope();
                    let mut seen = Vec::new();
                    self.pat(&a.pattern, &mut seen);
                    self.expr(&a.body);
                    self.pop_scope();
                }
            }
            ExprKind::Loop(l) => self.loop_expr(l),
            ExprKind::Unsafe { proofs, body } => {
                for p in proofs {
                    self.expr(p);
                }
                self.block(body);
            }
            ExprKind::Quantifier {
                var, domain, body, ..
            } => {
                self.expr(domain);
                self.push_scope();
                self.introduce(var, "biến lượng từ");
                self.expr(body);
                self.pop_scope();
            }
        }
    }

    fn loop_expr(&mut self, l: &Loop) {
        self.push_scope();
        match &l.kind {
            LoopKind::Infinite => {}
            LoopKind::While(c) => self.expr(c),
            LoopKind::ForIn { var, iter } => {
                self.expr(iter);
                self.introduce(var, "biến vòng lặp");
            }
        }
        if let Some(v) = &l.variant {
            self.expr(v);
        }
        for i in &l.invariants {
            self.expr(i);
        }
        self.block(&l.body);
        self.pop_scope();
    }

    // ---------------- pattern ----------------

    fn bind(&mut self, ident: &Ident, seen: &mut Vec<(String, Span)>) {
        // pattern chỉ có ident: biến thể enum nếu phân giải được tới một biến thể (ADR-017 mục 8)
        if let Found::Def(d) = self.module_scope_lookup(&ident.name)
            && self.t.defs[d.index()].kind == DefKind::Variant
        {
            self.record(ident.id, &ident.name, Res::Def(d));
            return;
        }
        if let Some((_, first)) = seen.iter().find(|(n, _)| *n == ident.name) {
            let first = *first;
            self.err_with(
                "E0202",
                ident.span,
                format!(
                    "tên `{}` được gán nhiều lần trong cùng một pattern",
                    ident.name
                ),
                "gán lặp",
                (self.file, first),
                "lần gán đầu tiên",
            );
            return;
        }
        seen.push((ident.name.clone(), ident.span));
        self.introduce(ident, "binding");
    }

    fn pat(&mut self, p: &Pattern, seen: &mut Vec<(String, Span)>) {
        match &p.kind {
            PatternKind::Wildcard
            | PatternKind::Rest
            | PatternKind::Lit(_)
            | PatternKind::Range { .. } => {}
            PatternKind::Binding(ident) => self.bind(ident, seen),
            PatternKind::Variant { path, args } => {
                self.resolve_path(path);
                for a in args {
                    self.pat(a, seen);
                }
            }
            PatternKind::Struct { path, fields, .. } => {
                self.resolve_path(path);
                for f in fields {
                    match &f.pattern {
                        Some(sub) => self.pat(sub, seen),
                        None => self.bind(&f.name, seen),
                    }
                }
            }
            PatternKind::Tuple(v) => {
                for x in v {
                    self.pat(x, seen);
                }
            }
        }
    }
}

/// Khoá logic của item: đường dẫn đủ tên; `⊐` dùng văn bản đầu đề + tên thành viên (không phụ thuộc thứ tự).
fn item_key(t: &Tables, module: DefId, file: FileId, item: &Item) -> String {
    if let Some(&d) = t.item_def.get(&item.id) {
        return t.defs[d.index()].path.clone();
    }
    let ItemKind::Impl(i) = &item.kind else {
        return t.defs[module.index()].path.clone();
    };
    let text = |s: Span| -> String {
        t.texts
            .get(file.index())
            .and_then(|x| x.get(s.start as usize..s.end as usize))
            .unwrap_or("")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let head = match &i.trait_path {
        Some(tp) => format!("{} for {}", text(tp.span), text(i.self_ty.span)),
        None => text(i.self_ty.span),
    };
    let mut members: Vec<&str> = i
        .members
        .iter()
        .map(|m| m.func.sig.name.name.as_str())
        .collect();
    members.sort_unstable();
    format!(
        "{}::impl {} [{}]",
        t.defs[module.index()].path,
        head,
        members.join(",")
    )
}
