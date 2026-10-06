//! Công cụ dùng chung cho test parser: dựng chương trình bọc, in S-expression của AST.
#![allow(dead_code)]
use gt_ast::*;
use gt_parser::{ParseOutput, parse};

pub fn parse_src(src: &str) -> ParseOutput {
    parse("t.gt", src)
}

pub fn codes(out: &ParseOutput) -> Vec<String> {
    out.report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect()
}

/// Bọc biểu thức vào thân hàm, parse, trả về S-expression của biểu thức cuối khối. Fail nếu có lỗi.
pub fn expr(e: &str) -> String {
    let out = parse_src(&format!("ƒ f() → i32 ! ∅ {{ {e} }}"));
    assert!(
        out.report.diagnostics.is_empty(),
        "{e:?}: {:?}",
        out.report.diagnostics
    );
    let body = fn_body(&out.program);
    match &body.tail {
        Some(t) => sx(t),
        None => panic!("{e:?}: không có biểu thức cuối khối"),
    }
}

/// Mã lỗi khi parse biểu thức (phải có lỗi).
pub fn expr_err(e: &str) -> Vec<String> {
    let out = parse_src(&format!("ƒ f() → i32 ! ∅ {{ {e} }}"));
    let c = codes(&out);
    assert!(!c.is_empty(), "{e:?}: đáng lẽ phải có lỗi");
    c
}

pub fn fn_body(p: &Program) -> &Block {
    match &p.items[0].kind {
        ItemKind::Fn(f) => &f.body,
        other => panic!("không phải hàm: {other:?}"),
    }
}

/// Các câu lệnh + biểu thức cuối của thân hàm, dạng S-expression.
pub fn block_of(src_body: &str) -> String {
    let out = parse_src(&format!("ƒ f() → i32 ! ∅ {{ {src_body} }}"));
    assert!(
        out.report.diagnostics.is_empty(),
        "{src_body:?}: {:?}",
        out.report.diagnostics
    );
    sblock(fn_body(&out.program))
}

pub fn sblock(b: &Block) -> String {
    let mut parts: Vec<String> = b.stmts.iter().map(sstmt).collect();
    if let Some(t) = &b.tail {
        parts.push(format!("tail:{}", sx(t)));
    }
    format!("{{{}}}", parts.join(" "))
}

fn sstmt(s: &Stmt) -> String {
    match &s.kind {
        StmtKind::Let {
            mutable,
            name,
            ty,
            init,
        } => {
            format!(
                "(let{} {}{} {})",
                if *mutable { "!" } else { "" },
                name.name,
                if ty.is_some() { ":T" } else { "" },
                sx(init)
            )
        }
        StmtKind::Assign { target, value } => format!("(set {} {})", sx(target), sx(value)),
        StmtKind::Expr { expr, semi } => {
            format!("({} {})", if *semi { "expr;" } else { "expr" }, sx(expr))
        }
        StmtKind::Return(e) => format!(
            "(ret{})",
            e.as_ref().map_or(String::new(), |e| format!(" {}", sx(e)))
        ),
        StmtKind::Break => "(break)".to_string(),
        StmtKind::Continue => "(continue)".to_string(),
    }
}

pub fn spath(p: &Path) -> String {
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

fn slit(l: &Lit) -> String {
    let t = match &l.kind {
        LitKind::Int(t) | LitKind::Float(t) => t.clone(),
        LitKind::Str(t) => format!("\"{t}\""),
        LitKind::Char(t) => format!("'{t}'"),
        LitKind::Bool(b) => b.to_string(),
    };
    if l.negative { format!("-{t}") } else { t }
}

fn bin(op: BinaryOp) -> &'static str {
    use BinaryOp::*;
    match op {
        Add => "+",
        Sub => "-",
        Mul => "*",
        Div => "/",
        Rem => "%",
        AddWrap => "+%",
        SubWrap => "-%",
        MulWrap => "*%",
        AddSat => "+^",
        SubSat => "-^",
        MulSat => "*^",
        AddChecked => "+?",
        SubChecked => "-?",
        MulChecked => "*?",
        Eq => "==",
        Ne => "!=",
        Lt => "<",
        Gt => ">",
        Le => "<=",
        Ge => ">=",
        And => "and",
        Or => "or",
        Implies => "=>",
        InSet => "in",
    }
}

fn un(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Not => "not",
        UnaryOp::Neg => "neg",
        UnaryOp::Deref => "deref",
        UnaryOp::Borrow => "&",
        UnaryOp::BorrowMut => "&!",
    }
}

fn opt(e: &Option<Box<Expr>>) -> String {
    e.as_ref().map_or("_".to_string(), |e| sx(e))
}

/// S-expression của biểu thức.
pub fn sx(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Lit(l) => slit(l),
        ExprKind::Path(p) => spath(p),
        ExprKind::Unary { op, operand } => format!("({} {})", un(*op), sx(operand)),
        ExprKind::Binary { op, lhs, rhs } => format!("({} {} {})", bin(*op), sx(lhs), sx(rhs)),
        ExprKind::Cast { expr, ty } => format!("(as {} {})", sx(expr), sty(ty)),
        ExprKind::Call { callee, args } => {
            let a: Vec<String> = args.iter().map(sx).collect();
            if a.is_empty() {
                format!("(call {})", sx(callee))
            } else {
                format!("(call {} {})", sx(callee), a.join(" "))
            }
        }
        ExprKind::Field { base, name } => match name {
            FieldName::Named(n) => format!("(. {} {})", sx(base), n.name),
            FieldName::Index(i) => format!("(. {} {})", sx(base), i.value),
        },
        ExprKind::Index { base, index } => format!("(idx {} {})", sx(base), sx(index)),
        ExprKind::Range {
            start,
            end,
            inclusive,
        } => {
            format!(
                "({} {} {})",
                if *inclusive { "..=" } else { ".." },
                opt(start),
                opt(end)
            )
        }
        ExprKind::Tuple(v) => {
            let a: Vec<String> = v.iter().map(sx).collect();
            if a.is_empty() {
                "(tuple)".to_string()
            } else {
                format!("(tuple {})", a.join(" "))
            }
        }
        ExprKind::Array(v) => {
            let a: Vec<String> = v.iter().map(sx).collect();
            format!("[{}]", a.join(" "))
        }
        ExprKind::ArrayRepeat { value, len } => format!("[{}; {}]", sx(value), sx(len)),
        ExprKind::StructLit { path, fields } => {
            let f: Vec<String> = fields
                .iter()
                .map(|f| format!("({} {})", f.name.name, sx(&f.value)))
                .collect();
            format!("(struct {} {})", spath(path), f.join(" "))
        }
        ExprKind::Block(b) => format!("(block {})", sblock(b)),
        ExprKind::If {
            cond,
            then_block,
            else_branch,
        } => match else_branch {
            Some(e) => format!("(if {} {} {})", sx(cond), sblock(then_block), sx(e)),
            None => format!("(if {} {})", sx(cond), sblock(then_block)),
        },
        ExprKind::Match { scrutinee, arms } => {
            let a: Vec<String> = arms
                .iter()
                .map(|a| format!("[{} => {}]", spat(&a.pattern), sx(&a.body)))
                .collect();
            format!("(match {} {})", sx(scrutinee), a.join(" "))
        }
        ExprKind::Loop(l) => {
            let kind = match &l.kind {
                LoopKind::Infinite => "inf".to_string(),
                LoopKind::While(c) => format!("while {}", sx(c)),
                LoopKind::ForIn { var, iter } => format!("for {} in {}", var.name, sx(iter)),
            };
            let var = l
                .variant
                .as_ref()
                .map_or(String::new(), |v| format!(" dec {}", sx(v)));
            let inv: String = l
                .invariants
                .iter()
                .map(|i| format!(" inv {}", sx(i)))
                .collect();
            format!("(loop {kind}{var}{inv} {})", sblock(&l.body))
        }
        ExprKind::Unsafe { proofs, body } => {
            let p: String = proofs.iter().map(|p| format!(" proof {}", sx(p))).collect();
            format!("(unsafe{p} {})", sblock(body))
        }
        ExprKind::Quantifier {
            kind,
            var,
            domain,
            body,
        } => format!(
            "({} {} {} {})",
            if matches!(kind, QuantifierKind::ForAll) {
                "forall"
            } else {
                "exists"
            },
            var.name,
            sx(domain),
            sx(body)
        ),
    }
}

pub fn sty(t: &Type) -> String {
    match &t.kind {
        TypeKind::Prim(p) => format!("{p:?}"),
        TypeKind::SelfType => "Self".to_string(),
        TypeKind::Path {
            path,
            lifetimes,
            args,
        } => {
            if args.is_empty() && lifetimes.is_empty() {
                spath(path)
            } else {
                let mut all: Vec<String> =
                    lifetimes.iter().map(|l| format!("'{}", l.name)).collect();
                all.extend(args.iter().map(sty));
                format!("{}<{}>", spath(path), all.join(","))
            }
        }
        TypeKind::Ref {
            lifetime,
            mutable,
            inner,
        } => format!(
            "&{}{}{}",
            if *mutable { "!" } else { "" },
            lifetime
                .as_ref()
                .map_or(String::new(), |l| format!("'{} ", l.name)),
            sty(inner)
        ),
        TypeKind::Array { elem, len } => format!("[{}; {}]", sty(elem), sx(len)),
        TypeKind::Slice(e) => format!("[{}]", sty(e)),
        TypeKind::Tuple(v) => format!("({})", v.iter().map(sty).collect::<Vec<_>>().join(",")),
        TypeKind::RawPtr { mutable, inner } => {
            format!("*{} {}", if *mutable { "mut" } else { "const" }, sty(inner))
        }
        TypeKind::Fn { params, ret, .. } => format!(
            "fn({})->{}",
            params.iter().map(sty).collect::<Vec<_>>().join(","),
            ret.as_ref().map_or("()".to_string(), |r| sty(r))
        ),
        TypeKind::Never => "!".to_string(),
    }
}

pub fn spat(p: &Pattern) -> String {
    match &p.kind {
        PatternKind::Wildcard => "_".to_string(),
        PatternKind::Rest => "..".to_string(),
        PatternKind::Binding(i) => i.name.clone(),
        PatternKind::Lit(l) => slit(l),
        PatternKind::Range {
            start,
            end,
            inclusive,
        } => format!(
            "{}{}{}",
            start.as_ref().map_or(String::new(), slit),
            if *inclusive { "..=" } else { ".." },
            end.as_ref().map_or(String::new(), slit)
        ),
        PatternKind::Variant { path, args } => {
            if args.is_empty() {
                spath(path)
            } else {
                format!(
                    "{}({})",
                    spath(path),
                    args.iter().map(spat).collect::<Vec<_>>().join(",")
                )
            }
        }
        PatternKind::Struct { path, fields, rest } => {
            let mut f: Vec<String> = fields
                .iter()
                .map(|f| match &f.pattern {
                    Some(p) => format!("{}:{}", f.name.name, spat(p)),
                    None => f.name.name.clone(),
                })
                .collect();
            if *rest {
                f.push("..".to_string());
            }
            format!("{}{{{}}}", spath(path), f.join(","))
        }
        PatternKind::Tuple(v) => format!("({})", v.iter().map(spat).collect::<Vec<_>>().join(",")),
    }
}

/// Che mọi `Span { start: N, end: M }` trong chuỗi Debug để so sánh hai AST không tính vị trí.
pub fn mask_spans(debug: &str) -> String {
    let mut out = String::new();
    let mut rest = debug;
    while let Some(i) = rest.find("Span {") {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        match tail.find('}') {
            Some(j) => {
                out.push_str("Span");
                rest = &tail[j + 1..];
            }
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}
