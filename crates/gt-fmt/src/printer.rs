//! Printer: AST → văn bản chuẩn. Độ ưu tiên (cao = chặt hơn): range 1 < ⇒ 2 < ∨ 3 < ∧ 4 < so sánh 5 < cộng 6 < nhân 7 < `as` 8
//! < đơn nguyên 9 < postfix 10 < sơ cấp 11. Khối-biểu-thức (`⎇ ⌥ ⟳ ⚠`) và lượng từ có độ ưu tiên 0.
use gt_ast::*;
use gt_span::Span;

const INDENT: usize = 2;
const INLINE_MAX: usize = 60;

fn pad(n: usize) -> String {
    " ".repeat(n * INDENT)
}

/// Printer in văn bản CƠ SỞ (không có comment); comment được đặt sau bởi `attach::place` theo token.
/// Printer chỉ biết VỊ TRÍ comment để một khối/danh sách chứa comment không bao giờ bị in gọn một dòng.
pub(crate) struct Fmt {
    comment_starts: Vec<u32>,
}

impl Fmt {
    pub fn new(comment_starts: Vec<u32>) -> Fmt {
        Fmt { comment_starts }
    }

    /// Có comment bắt đầu trong khoảng mở `(lo, hi)`.
    fn has_comment_in(&self, lo: u32, hi: u32) -> bool {
        let i = self.comment_starts.partition_point(|&c| c <= lo);
        self.comment_starts.get(i).is_some_and(|&c| c < hi)
    }

    /// Một node trong danh sách: một dòng thụt lề.
    fn emit_node(indent: usize, text: &str) -> String {
        format!("{}{text}\n", pad(indent))
    }

    /// `{ … }` nhiều dòng cho một danh sách node (trường, biến thể, thành viên, nhánh, item).
    fn braced_list(indent: usize, entries: &[(Span, String)], blank_between: bool) -> String {
        if entries.is_empty() {
            return "{ }".to_string();
        }
        let mut out = String::from("{\n");
        for (i, (_, text)) in entries.iter().enumerate() {
            if i > 0 && blank_between {
                out.push('\n');
            }
            out.push_str(&Self::emit_node(indent + 1, text));
        }
        out.push_str(&pad(indent));
        out.push('}');
        out
    }

    // ---------- chương trình & item ----------

    pub fn program(&mut self, p: &Program) -> String {
        let mut out = String::new();
        for (i, it) in p.items.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            let text = self.item(it, 0);
            out.push_str(&Self::emit_node(0, &text));
        }
        out
    }

    fn docs(docs: &[DocComment]) -> Vec<String> {
        docs.iter()
            .map(|d| {
                if d.text.is_empty() {
                    "///".to_string()
                } else {
                    format!("/// {}", d.text)
                }
            })
            .collect()
    }

    fn join_lines(lines: Vec<String>, indent: usize) -> String {
        lines.join(&format!("\n{}", pad(indent)))
    }

    fn item(&mut self, it: &Item, indent: usize) -> String {
        let mut lines = Self::docs(&it.docs);
        for a in &it.attrs {
            lines.push(self.attribute(a));
        }
        let vis = if it.vis == Visibility::Public {
            "◈ "
        } else {
            ""
        };
        let body = match &it.kind {
            ItemKind::Fn(f) => self.fn_decl(vis, &f.sig, Some(&f.body), indent),
            ItemKind::Struct(s) => {
                let entries: Vec<(Span, String)> = s
                    .fields
                    .iter()
                    .map(|f| (f.span, self.field_def(f, indent + 1)))
                    .collect();
                format!(
                    "{vis}▢ {}{} {}",
                    s.name.name,
                    self.generics(&s.generics),
                    Self::braced_list(indent, &entries, false)
                )
            }
            ItemKind::Enum(e) => {
                let entries: Vec<(Span, String)> = e
                    .variants
                    .iter()
                    .map(|v| (v.span, self.variant(v, indent + 1)))
                    .collect();
                format!(
                    "{vis}⊔ {}{} {}",
                    e.name.name,
                    self.generics(&e.generics),
                    Self::braced_list(indent, &entries, false)
                )
            }
            ItemKind::Trait(t) => {
                let entries: Vec<(Span, String)> = t
                    .members
                    .iter()
                    .map(|m| (m.span, self.trait_member(m, indent + 1)))
                    .collect();
                format!(
                    "{vis}⊏ {}{} {}",
                    t.name.name,
                    self.generics(&t.generics),
                    Self::braced_list(indent, &entries, false)
                )
            }
            ItemKind::Impl(i) => {
                let entries: Vec<(Span, String)> = i
                    .members
                    .iter()
                    .map(|m| (m.span, self.impl_member(m, indent + 1)))
                    .collect();
                let head = match &i.trait_path {
                    Some(t) => format!("{} for {}", Self::path(t), self.ty(&i.self_ty)),
                    None => self.ty(&i.self_ty),
                };
                format!(
                    "{vis}⊐{} {head} {}",
                    self.generics(&i.generics),
                    Self::braced_list(indent, &entries, true)
                )
            }
            ItemKind::Mod(m) => {
                let entries: Vec<(Span, String)> = m
                    .items
                    .iter()
                    .map(|x| (x.span, self.item(x, indent + 1)))
                    .collect();
                format!(
                    "{vis}▦ {} {}",
                    m.name.name,
                    Self::braced_list(indent, &entries, true)
                )
            }
            ItemKind::Use(u) => {
                let names = if u.names.is_empty() {
                    String::new()
                } else {
                    format!(
                        "::{{{}}}",
                        u.names
                            .iter()
                            .map(|n| n.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                format!("{vis}⇲ {}{names};", Self::path(&u.path))
            }
            ItemKind::TypeAlias(t) => format!(
                "{vis}≡ {}{} ≔ {};",
                t.name.name,
                self.generics(&t.generics),
                self.ty(&t.ty)
            ),
            ItemKind::Const(c) => {
                let v = self.expr(&c.value, 0, false, indent);
                format!("{vis}□ {}: {} ≔ {v};", c.name.name, self.ty(&c.ty))
            }
            ItemKind::Static(c) => {
                let v = self.expr(&c.value, 0, false, indent);
                format!("{vis}■ {}: {} ≔ {v};", c.name.name, self.ty(&c.ty))
            }
        };
        lines.push(body);
        Self::join_lines(lines, indent)
    }

    fn attribute(&self, a: &Attribute) -> String {
        if a.args.is_empty() {
            return format!("#{}", a.name.name);
        }
        let args: Vec<String> = a
            .args
            .iter()
            .map(|x| match &x.kind {
                AttrArgKind::Ident(s) => s.clone(),
                AttrArgKind::Str(s) => format!("\"{s}\""),
            })
            .collect();
        format!("#{}({})", a.name.name, args.join(", "))
    }

    fn field_def(&mut self, f: &FieldDef, indent: usize) -> String {
        let mut lines = Self::docs(&f.docs);
        let vis = if f.vis == Visibility::Public {
            "◈ "
        } else {
            ""
        };
        lines.push(format!("{vis}{}: {},", f.name.name, self.ty(&f.ty)));
        Self::join_lines(lines, indent)
    }

    fn variant(&mut self, v: &Variant, indent: usize) -> String {
        let mut lines = Self::docs(&v.docs);
        if v.fields.is_empty() {
            lines.push(format!("{},", v.name.name));
        } else {
            let f: Vec<String> = v.fields.iter().map(|t| self.ty(t)).collect();
            lines.push(format!("{}({}),", v.name.name, f.join(", ")));
        }
        Self::join_lines(lines, indent)
    }

    fn trait_member(&mut self, m: &TraitMember, indent: usize) -> String {
        let mut lines = Self::docs(&m.docs);
        let head = self.fn_header(&m.sig, indent);
        let mut s = head;
        for c in &m.sig.contracts {
            let e = self.expr(&c.expr, 0, true, indent);
            s.push_str(&format!(" {} {e}", Self::contract_mark(c.kind)));
        }
        s.push(';');
        lines.push(s);
        Self::join_lines(lines, indent)
    }

    fn impl_member(&mut self, m: &ImplMember, indent: usize) -> String {
        let mut lines = Self::docs(&m.docs);
        lines.push(self.fn_decl("", &m.func.sig, Some(&m.func.body), indent));
        Self::join_lines(lines, indent)
    }

    fn contract_mark(k: ContractKind) -> &'static str {
        match k {
            ContractKind::Pre => "⊳",
            ContractKind::Post => "⊲",
            ContractKind::Variant => "↓",
        }
    }

    fn generics(&mut self, g: &Option<Generics>) -> String {
        let Some(g) = g else { return String::new() };
        let ps: Vec<String> = g
            .params
            .iter()
            .map(|p| match &p.kind {
                GenericParamKind::Lifetime(l) => format!("'{}", l.name),
                GenericParamKind::Type { name, bounds } => {
                    if bounds.is_empty() {
                        name.name.clone()
                    } else {
                        format!(
                            "{}: {}",
                            name.name,
                            bounds
                                .iter()
                                .map(Self::path)
                                .collect::<Vec<_>>()
                                .join(" + ")
                        )
                    }
                }
            })
            .collect();
        format!("<{}>", ps.join(", "))
    }

    fn effects(e: &Effects) -> String {
        match &e.kind {
            EffectsKind::Pure => "! ∅".to_string(),
            EffectsKind::Set(v) => {
                let items: Vec<String> = v
                    .iter()
                    .map(|x| match &x.arg {
                        Some(a) => format!("{}({})", x.name.name, a.name),
                        None => x.name.name.clone(),
                    })
                    .collect();
                format!("! {{{}}}", items.join(", "))
            }
        }
    }

    fn param(&mut self, p: &Param) -> String {
        match &p.kind {
            ParamKind::SelfParam(SelfKind::Value) => "self".to_string(),
            ParamKind::SelfParam(SelfKind::Ref) => "&self".to_string(),
            ParamKind::SelfParam(SelfKind::RefMut) => "&!self".to_string(),
            ParamKind::Named { mutable, name, ty } => {
                format!(
                    "{}{}: {}",
                    if *mutable { "μ" } else { "" },
                    name.name,
                    self.ty(ty)
                )
            }
        }
    }

    /// `[◈ ]ƒ tên<…>(tham số) → kiểu ! hiệu_ứng` (không gồm hợp đồng, thân hay `;`).
    fn fn_header(&mut self, sig: &FnSig, _indent: usize) -> String {
        let params: Vec<String> = sig.params.iter().map(|p| self.param(p)).collect();
        let ret = sig
            .ret
            .as_ref()
            .map_or(String::new(), |t| format!(" → {}", self.ty(t)));
        format!(
            "ƒ {}{}({}){ret} {}",
            sig.name.name,
            self.generics(&sig.generics),
            params.join(", "),
            Self::effects(&sig.effects)
        )
    }

    fn fn_decl(&mut self, vis: &str, sig: &FnSig, body: Option<&Block>, indent: usize) -> String {
        let mut s = format!("{vis}{}", self.fn_header(sig, indent));
        if sig.contracts.is_empty() {
            if let Some(b) = body {
                s.push(' ');
                s.push_str(&self.block(b, indent));
            }
            return s;
        }
        for c in &sig.contracts {
            let e = self.expr(&c.expr, 0, true, indent + 1);
            s.push_str(&format!(
                "\n{}{} {e}",
                pad(indent + 1),
                Self::contract_mark(c.kind)
            ));
        }
        if let Some(b) = body {
            s.push_str(&format!("\n{}{}", pad(indent), self.block(b, indent)));
        }
        s
    }

    // ---------- đường dẫn, kiểu, pattern ----------

    fn path(p: &Path) -> String {
        p.segments
            .iter()
            .map(|s| match &s.kind {
                SegmentKind::Name(n) => n.clone(),
                SegmentKind::SelfType => "Self".to_string(),
                SegmentKind::SelfValue => "self".to_string(),
            })
            .collect::<Vec<_>>()
            .join("::")
    }

    fn prim(p: PrimType) -> &'static str {
        match p {
            PrimType::I8 => "i8",
            PrimType::I16 => "i16",
            PrimType::I32 => "i32",
            PrimType::I64 => "i64",
            PrimType::I128 => "i128",
            PrimType::Isize => "isize",
            PrimType::U8 => "u8",
            PrimType::U16 => "u16",
            PrimType::U32 => "u32",
            PrimType::U64 => "u64",
            PrimType::U128 => "u128",
            PrimType::Usize => "usize",
            PrimType::F32 => "f32",
            PrimType::F64 => "f64",
            PrimType::Bool => "bool",
            PrimType::Char => "char",
            PrimType::Str => "str",
        }
    }

    fn ty(&mut self, t: &Type) -> String {
        match &t.kind {
            TypeKind::Prim(p) => Self::prim(*p).to_string(),
            TypeKind::SelfType => "Self".to_string(),
            TypeKind::Path {
                path,
                lifetimes,
                args,
            } => {
                if args.is_empty() && lifetimes.is_empty() {
                    Self::path(path)
                } else {
                    // lifetime trước, kiểu sau (ADR-021)
                    let mut a: Vec<String> =
                        lifetimes.iter().map(|l| format!("'{}", l.name)).collect();
                    a.extend(args.iter().map(|x| self.ty(x)));
                    format!("{}<{}>", Self::path(path), a.join(", "))
                }
            }
            TypeKind::Ref {
                lifetime,
                mutable,
                inner,
            } => {
                let lt = lifetime
                    .as_ref()
                    .map_or(String::new(), |l| format!("'{} ", l.name));
                format!("&{}{lt}{}", if *mutable { "!" } else { "" }, self.ty(inner))
            }
            TypeKind::Array { elem, len } => {
                let l = self.expr(len, 0, false, 0);
                format!("[{}; {l}]", self.ty(elem))
            }
            TypeKind::Slice(e) => format!("[{}]", self.ty(e)),
            TypeKind::Tuple(v) => match v.len() {
                0 => "()".to_string(),
                1 => format!("({},)", self.ty(&v[0])),
                _ => format!(
                    "({})",
                    v.iter().map(|x| self.ty(x)).collect::<Vec<_>>().join(", ")
                ),
            },
            TypeKind::RawPtr { mutable, inner } => {
                format!("*{} {}", if *mutable { "μ" } else { "□" }, self.ty(inner))
            }
            TypeKind::Fn {
                params,
                ret,
                effects,
            } => {
                let p: Vec<String> = params.iter().map(|x| self.ty(x)).collect();
                let r = ret
                    .as_ref()
                    .map_or(String::new(), |r| format!(" → {}", self.ty(r)));
                format!("ƒ({}){r} {}", p.join(", "), Self::effects(effects))
            }
            TypeKind::Never => "!".to_string(),
        }
    }

    fn lit(l: &Lit) -> String {
        let t = match &l.kind {
            LitKind::Int(t) | LitKind::Float(t) => t.clone(),
            LitKind::Str(t) => format!("\"{t}\""),
            LitKind::Char(t) => format!("'{t}'"),
            LitKind::Bool(b) => b.to_string(),
        };
        if l.negative { format!("-{t}") } else { t }
    }

    fn pat(&mut self, p: &Pattern) -> String {
        match &p.kind {
            PatternKind::Wildcard => "_".to_string(),
            PatternKind::Rest => "..".to_string(),
            PatternKind::Binding(i) => i.name.clone(),
            PatternKind::Lit(l) => Self::lit(l),
            PatternKind::Range {
                start,
                end,
                inclusive,
            } => format!(
                "{}{}{}",
                start.as_ref().map_or(String::new(), Self::lit),
                if *inclusive { "..=" } else { ".." },
                end.as_ref().map_or(String::new(), Self::lit)
            ),
            PatternKind::Variant { path, args } => {
                if args.is_empty() && path.segments.len() > 1 {
                    Self::path(path)
                } else {
                    format!(
                        "{}({})",
                        Self::path(path),
                        args.iter()
                            .map(|a| self.pat(a))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            PatternKind::Struct { path, fields, rest } => {
                let mut f: Vec<String> = fields
                    .iter()
                    .map(|x| match &x.pattern {
                        Some(sub) => format!("{}: {}", x.name.name, self.pat(sub)),
                        None => x.name.name.clone(),
                    })
                    .collect();
                if *rest {
                    f.push("..".to_string());
                }
                if f.is_empty() {
                    format!("{} {{ }}", Self::path(path))
                } else {
                    format!("{} {{ {} }}", Self::path(path), f.join(", "))
                }
            }
            PatternKind::Tuple(v) => match v.as_slice() {
                [] => "()".to_string(),
                [only] if matches!(only.kind, PatternKind::Rest) => "(..)".to_string(),
                [only] => format!("({},)", self.pat(only)),
                _ => format!(
                    "({})",
                    v.iter().map(|x| self.pat(x)).collect::<Vec<_>>().join(", ")
                ),
            },
        }
    }

    // ---------- khối & câu lệnh ----------

    fn block(&mut self, b: &Block, indent: usize) -> String {
        let (lo, hi) = (b.span.start, b.span.end.saturating_sub(1));
        let has_comments = self.has_comment_in(lo, hi);
        if !has_comments && b.stmts.is_empty() {
            match &b.tail {
                None => return "{ }".to_string(),
                Some(t) => {
                    let s = self.expr(t, 0, false, indent);
                    if !s.contains('\n') && s.chars().count() <= INLINE_MAX {
                        return format!("{{ {s} }}");
                    }
                }
            }
        }
        let mut entries: Vec<(Span, String)> = Vec::new();
        for s in &b.stmts {
            entries.push((s.span, self.stmt(s, indent + 1)));
        }
        if let Some(t) = &b.tail {
            entries.push((t.span, self.expr(t, 0, false, indent + 1)));
        }
        Self::braced_list(indent, &entries, false)
    }

    fn stmt(&mut self, s: &Stmt, indent: usize) -> String {
        match &s.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                init,
            } => {
                let t = ty
                    .as_ref()
                    .map_or(String::new(), |t| format!(": {}", self.ty(t)));
                let v = self.expr(init, 0, false, indent);
                format!("{}{}{t} ≔ {v};", if *mutable { "μ" } else { "" }, name.name)
            }
            StmtKind::Assign { target, value } => {
                let t = self.expr(target, 9, false, indent);
                let v = self.expr(value, 0, false, indent);
                format!("{t} ← {v};")
            }
            StmtKind::Expr { expr, semi } => {
                let e = self.expr(expr, 0, false, indent);
                if *semi { format!("{e};") } else { e }
            }
            StmtKind::Return(None) => "⏎;".to_string(),
            StmtKind::Return(Some(e)) => format!("⏎ {};", self.expr(e, 0, false, indent)),
            StmtKind::Break => "⊠;".to_string(),
            StmtKind::Continue => "⊡;".to_string(),
        }
    }

    // ---------- biểu thức ----------

    fn prec(e: &Expr) -> u8 {
        match &e.kind {
            ExprKind::Lit(l) => {
                if l.negative {
                    9
                } else {
                    11
                }
            }
            ExprKind::Path(_)
            | ExprKind::Tuple(_)
            | ExprKind::Array(_)
            | ExprKind::ArrayRepeat { .. }
            | ExprKind::StructLit { .. }
            | ExprKind::Block(_) => 11,
            ExprKind::Call { .. }
            | ExprKind::Field { .. }
            | ExprKind::Index { .. }
            | ExprKind::Try(_) => 10,
            ExprKind::Unary { .. } => 9,
            ExprKind::Cast { .. } => 8,
            ExprKind::Binary { op, .. } => Self::bin_prec(*op).0,
            ExprKind::Range { .. } => 1,
            ExprKind::If { .. }
            | ExprKind::Match { .. }
            | ExprKind::Loop(_)
            | ExprKind::Unsafe { .. }
            | ExprKind::Quantifier { .. } => 0,
        }
    }

    /// (độ ưu tiên, kết hợp: 0 trái, 1 phải, 2 không kết hợp, ký hiệu)
    fn bin_prec(op: BinaryOp) -> (u8, u8, &'static str) {
        use BinaryOp::*;
        match op {
            Implies => (2, 1, "⇒"),
            Or => (3, 0, "∨"),
            And => (4, 0, "∧"),
            Eq => (5, 2, "=="),
            Ne => (5, 2, "≠"),
            Lt => (5, 2, "<"),
            Gt => (5, 2, ">"),
            Le => (5, 2, "≤"),
            Ge => (5, 2, "≥"),
            InSet => (5, 2, "∈"),
            Add => (6, 0, "+"),
            Sub => (6, 0, "-"),
            AddWrap => (6, 0, "+%"),
            SubWrap => (6, 0, "-%"),
            AddSat => (6, 0, "+^"),
            SubSat => (6, 0, "-^"),
            AddChecked => (6, 0, "+?"),
            SubChecked => (6, 0, "-?"),
            Mul => (7, 0, "*"),
            Div => (7, 0, "/"),
            Rem => (7, 0, "%"),
            MulWrap => (7, 0, "*%"),
            MulSat => (7, 0, "*^"),
            MulChecked => (7, 0, "*?"),
        }
    }

    /// Biểu thức kết thúc bằng một kiểu (chuỗi `as T` ở cuối, kể cả qua toán hạng phải/đơn nguyên).
    /// Range không cần xét: vế trái kiểu Range (độ ưu tiên 1) luôn bị bọc ngoặc vì thấp hơn mọi toán hạng của `<`.
    fn ends_with_type(e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Cast { .. } => true,
            ExprKind::Binary { rhs, .. } => Self::ends_with_type(rhs),
            ExprKind::Unary { operand, .. } => Self::ends_with_type(operand),
            _ => false,
        }
    }

    /// In biểu thức; thêm ngoặc nếu độ ưu tiên < `min`, hoặc nếu là struct literal ở vị trí cấm (`ns`).
    fn expr(&mut self, e: &Expr, min: u8, ns: bool, indent: usize) -> String {
        let prec = Self::prec(e);
        let bare_struct_forbidden = ns && matches!(e.kind, ExprKind::StructLit { .. });
        let paren = prec < min || bare_struct_forbidden;
        let inner_ns = if paren { false } else { ns };
        let s = self.expr_inner(e, inner_ns, indent);
        if paren { format!("({s})") } else { s }
    }

    fn comma_list(&mut self, xs: &[Expr], indent: usize) -> String {
        xs.iter()
            .map(|x| self.expr(x, 0, false, indent))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn expr_inner(&mut self, e: &Expr, ns: bool, indent: usize) -> String {
        match &e.kind {
            ExprKind::Lit(l) => Self::lit(l),
            ExprKind::Path(p) => Self::path(p),
            ExprKind::Unary { op, operand } => {
                let sym = match op {
                    UnaryOp::Not => "¬",
                    UnaryOp::Neg => "-",
                    UnaryOp::Deref => "*",
                    UnaryOp::Borrow => "&",
                    UnaryOp::BorrowMut => "&!",
                };
                // `-` trước literal sẽ bị gấp thành literal âm khi đọc lại → bọc ngoặc để giữ đúng AST
                if matches!(op, UnaryOp::Neg) && matches!(operand.kind, ExprKind::Lit(_)) {
                    let o = self.expr_inner(operand, false, indent);
                    return format!("{sym}({o})");
                }
                format!("{sym}{}", self.expr(operand, 9, ns, indent))
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let (p, assoc, sym) = Self::bin_prec(*op);
                let (lmin, rmin) = match assoc {
                    0 => (p, p + 1),
                    1 => (p + 1, p),
                    _ => (p + 1, p + 1),
                };
                // `x as T < y`: `T <` bị đọc là tham số kiểu → vế trái kết thúc bằng kiểu phải bọc ngoặc khi toán tử là `<`
                let lhs_text = if matches!(op, BinaryOp::Lt) && Self::ends_with_type(lhs) {
                    format!("({})", self.expr_inner(lhs, false, indent))
                } else {
                    self.expr(lhs, lmin, ns, indent)
                };
                format!("{lhs_text} {sym} {}", self.expr(rhs, rmin, ns, indent))
            }
            ExprKind::Cast { expr, ty } => {
                format!("{} as {}", self.expr(expr, 8, ns, indent), self.ty(ty))
            }
            ExprKind::Call { callee, args } => {
                format!(
                    "{}({})",
                    self.expr(callee, 10, ns, indent),
                    self.comma_list(args, indent)
                )
            }
            ExprKind::Field { base, name } => {
                // `2.0` đọc lại là số thực: chỉ số tuple sau literal phải bọc ngoặc
                let literal_base = matches!(base.kind, ExprKind::Lit(_));
                let b = if literal_base && matches!(name, FieldName::Index(_)) {
                    format!("({})", self.expr_inner(base, false, indent))
                } else {
                    self.expr(base, 10, ns, indent)
                };
                match name {
                    FieldName::Named(n) => format!("{b}.{}", n.name),
                    FieldName::Index(i) => format!("{b}.{}", i.value),
                }
            }
            ExprKind::Try(inner) => format!("{}?", self.expr(inner, 10, ns, indent)),
            ExprKind::Index { base, index } => {
                format!(
                    "{}[{}]",
                    self.expr(base, 10, ns, indent),
                    self.expr(index, 0, false, indent)
                )
            }
            ExprKind::Range {
                start,
                end,
                inclusive,
            } => {
                let s = start
                    .as_ref()
                    .map_or(String::new(), |x| self.expr(x, 2, ns, indent));
                let op = if *inclusive { "..=" } else { ".." };
                let en = match end {
                    None => String::new(),
                    // trong ngữ cảnh cấm struct, `{` sau `..` bắt đầu khối chứ không phải cận cuối
                    Some(x) if ns && matches!(x.kind, ExprKind::Block(_)) => {
                        format!("({})", self.expr_inner(x, false, indent))
                    }
                    Some(x) => self.expr(x, 2, ns, indent),
                };
                format!("{s}{op}{en}")
            }
            ExprKind::Tuple(v) => match v.len() {
                0 => "()".to_string(),
                1 => format!("({},)", self.expr(&v[0], 0, false, indent)),
                _ => format!("({})", self.comma_list(v, indent)),
            },
            ExprKind::Array(v) => format!("[{}]", self.comma_list(v, indent)),
            ExprKind::ArrayRepeat { value, len } => {
                format!(
                    "[{}; {}]",
                    self.expr(value, 0, false, indent),
                    self.expr(len, 0, false, indent)
                )
            }
            ExprKind::StructLit { path, fields } => {
                if fields.is_empty() {
                    format!("{} {{ }}", Self::path(path))
                } else {
                    let f: Vec<String> = fields
                        .iter()
                        .map(|x| {
                            format!("{}: {}", x.name.name, self.expr(&x.value, 0, false, indent))
                        })
                        .collect();
                    format!("{} {{ {} }}", Self::path(path), f.join(", "))
                }
            }
            ExprKind::Block(b) => self.block(b, indent),
            ExprKind::If {
                cond,
                then_block,
                else_branch,
            } => {
                let mut s = format!(
                    "⎇ {} {}",
                    self.expr(cond, 0, true, indent),
                    self.block(then_block, indent)
                );
                if let Some(e) = else_branch {
                    s.push_str(" ⇝ ");
                    match &e.kind {
                        ExprKind::Block(b) => s.push_str(&self.block(b, indent)),
                        _ => s.push_str(&self.expr(e, 0, false, indent)),
                    }
                }
                s
            }
            ExprKind::Match { scrutinee, arms } => {
                let head = format!("⌥ {} ", self.expr(scrutinee, 0, true, indent));
                let entries: Vec<(Span, String)> = arms
                    .iter()
                    .map(|a| {
                        let body = self.expr(&a.body, 0, false, indent + 1);
                        (a.span, format!("{} ↦ {body},", self.pat(&a.pattern)))
                    })
                    .collect();
                format!("{head}{}", Self::braced_list(indent, &entries, false))
            }
            ExprKind::Loop(l) => {
                let mut s = String::from("⟳");
                match &l.kind {
                    LoopKind::Infinite => {}
                    LoopKind::While(c) => {
                        s.push_str(&format!(" {}", self.expr(c, 0, true, indent)))
                    }
                    LoopKind::ForIn { var, iter } => s.push_str(&format!(
                        " {} in {}",
                        var.name,
                        self.expr(iter, 0, true, indent)
                    )),
                }
                if let Some(v) = &l.variant {
                    s.push_str(&format!(" ↓ {}", self.expr(v, 0, true, indent)));
                }
                for i in &l.invariants {
                    s.push_str(&format!(" ⋈ {}", self.expr(i, 0, true, indent)));
                }
                s.push(' ');
                s.push_str(&self.block(&l.body, indent));
                s
            }
            ExprKind::Unsafe { proofs, body } => {
                let mut s = String::from("⚠");
                for p in proofs {
                    s.push_str(&format!(" ⊢ {}", self.expr(p, 0, true, indent)));
                }
                s.push(' ');
                s.push_str(&self.block(body, indent));
                s
            }
            ExprKind::Quantifier {
                kind,
                var,
                domain,
                body,
            } => {
                let q = if matches!(kind, QuantifierKind::ForAll) {
                    "∀"
                } else {
                    "∃"
                };
                format!(
                    "{q} {} ∈ {} : {}",
                    var.name,
                    self.expr(domain, 1, false, indent),
                    self.expr(body, 0, ns, indent)
                )
            }
        }
    }
}
