//! MIR dạng text ổn định (ADR-020 §5): một dòng một lệnh, đánh số theo thứ tự tạo, không địa chỉ/băm/vị trí tuyệt đối.
use crate::mir::*;
use gt_ast::NodeId;
use gt_resolve::Def;
use gt_types::{AdtKind, IntTy, Ty, TypeDb};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Ngữ cảnh in: tên định nghĩa, bảng ADT, khoá của hàm/item theo NodeId.
pub struct PrintCx<'a> {
    pub defs: &'a [Def],
    pub db: &'a TypeDb,
    pub keys: &'a BTreeMap<NodeId, String>,
}

impl PrintCx<'_> {
    fn ty(&self, t: &Ty) -> String {
        t.render(self.defs)
    }

    fn key(&self, n: NodeId) -> String {
        self.keys
            .get(&n)
            .cloned()
            .unwrap_or_else(|| format!("?{}", n.raw()))
    }
}

/// In một thân.
#[must_use]
pub fn print_body(cx: &PrintCx<'_>, b: &Body) -> String {
    let mut s = String::new();
    let args: Vec<String> = (1..=b.arg_count)
        .map(|i| format!("_{i}: {}", cx.ty(&b.locals[i].ty)))
        .collect();
    let kw = match b.kind {
        BodyKind::Fn => "fn",
        BodyKind::Const => "const",
        BodyKind::Static => "static",
    };
    let _ = writeln!(
        s,
        "{kw} {}({}) -> {} {{",
        b.key,
        args.join(", "),
        cx.ty(b.return_ty())
    );
    for (i, l) in b.locals.iter().enumerate() {
        let m = if l.mutable { "mut " } else { "" };
        let what = match &l.kind {
            LocalKind::Return => "return".to_string(),
            LocalKind::Arg(Some(n)) => format!("arg `{n}`"),
            LocalKind::Arg(None) => "arg self".to_string(),
            LocalKind::User { name, .. } => format!("user `{name}`"),
            LocalKind::Temp => "temp".to_string(),
        };
        let regions = if l.regions.is_empty() {
            String::new()
        } else {
            let r: Vec<String> = l.regions.iter().map(|r| format!("'#{}", r.0)).collect();
            format!("; regions {}", r.join(" "))
        };
        let _ = writeln!(s, "    let {m}_{i}: {}; // {what}{regions}", cx.ty(&l.ty));
    }
    if !b.contracts.is_empty() {
        let _ = writeln!(s, "    // hợp đồng chưa hạ (M7): {}", b.contracts.len());
    }
    for (i, bb) in b.blocks.iter().enumerate() {
        let _ = writeln!(s, "    bb{i}: {{");
        for st in &bb.statements {
            let _ = writeln!(s, "        {}", statement(cx, b, st));
        }
        match &bb.terminator {
            Some(t) => {
                let _ = writeln!(s, "        {}", terminator(cx, b, &t.kind));
            }
            None => {
                let _ = writeln!(s, "        <thiếu terminator>");
            }
        }
        let _ = writeln!(s, "    }}");
    }
    s.push_str("}\n");
    s
}

fn statement(cx: &PrintCx<'_>, b: &Body, st: &Statement) -> String {
    match &st.kind {
        StatementKind::Assign(p, rv) => format!("{} = {};", place(p), rvalue(cx, b, rv)),
        StatementKind::StorageLive(l) => format!("StorageLive(_{});", l.0),
        StatementKind::StorageDead(l) => format!("StorageDead(_{});", l.0),
        StatementKind::FakeRead(c, p) => format!("FakeRead({c:?}, {});", place(p)),
        StatementKind::Nop => "nop;".to_string(),
    }
}

/// In một chỗ: `_3`, `(*_2)`, `_3.0`, `(_3 as #1)`, `_2[_5]`, `_2[_5.._6]`.
#[must_use]
pub fn place(p: &Place) -> String {
    let mut s = format!("_{}", p.local.0);
    for e in &p.projection {
        s = match e {
            PlaceElem::Deref => format!("(*{s})"),
            PlaceElem::Field(i, _) => format!("{s}.{i}"),
            PlaceElem::Index(l) => format!("{s}[_{}]", l.0),
            PlaceElem::Downcast(v) => format!("({s} as #{v})"),
            PlaceElem::Subslice { from, to } => match to {
                Some(t) => format!("{s}[_{}.._{}]", from.0, t.0),
                None => format!("{s}[_{}..]", from.0),
            },
        };
    }
    s
}

fn int_text(bits: u128, ty: IntTy) -> String {
    let w = ty.bits();
    let v = if ty.signed() {
        let shift = 128 - w;
        (((bits << shift) as i128) >> shift).to_string()
    } else {
        bits.to_string()
    };
    format!("{v}_{}", ty.name())
}

fn escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if c.is_control() => {
                let _ = write!(o, "\\u{{{:x}}}", c as u32);
            }
            c => o.push(c),
        }
    }
    o
}

fn constant(cx: &PrintCx<'_>, c: &Const) -> String {
    match c {
        Const::Int { bits, ty } => int_text(*bits, *ty),
        Const::Float { text, .. } => text.clone(),
        Const::Bool(b) => b.to_string(),
        Const::Char(c) => match char::from_u32(*c) {
            Some(ch) if !ch.is_control() && ch != '\'' && ch != '\\' => format!("'{ch}'"),
            _ => format!("'\\u{{{c:x}}}'"),
        },
        Const::Str(s) => format!("\"{}\"", escape(s)),
        Const::Unit => "()".to_string(),
        Const::FnItem { node, generics, .. } => {
            format!("fn {}{}", cx.key(*node), gens(cx, generics))
        }
        Const::Item { node, .. } => format!("item {}", cx.key(*node)),
    }
}

fn gens(cx: &PrintCx<'_>, g: &[Ty]) -> String {
    if g.is_empty() {
        String::new()
    } else {
        let v: Vec<String> = g.iter().map(|t| cx.ty(t)).collect();
        format!("::<{}>", v.join(", "))
    }
}

fn operand(cx: &PrintCx<'_>, o: &Operand) -> String {
    match o {
        Operand::Copy(p) => format!("copy {}", place(p)),
        Operand::Move(p) => format!("move {}", place(p)),
        Operand::Const(c) => format!("const {}", constant(cx, c)),
    }
}

fn ops(cx: &PrintCx<'_>, v: &[Operand]) -> String {
    v.iter()
        .map(|o| operand(cx, o))
        .collect::<Vec<_>>()
        .join(", ")
}

fn rvalue(cx: &PrintCx<'_>, b: &Body, rv: &Rvalue) -> String {
    let _ = b;
    match rv {
        Rvalue::Use(o) => operand(cx, o),
        Rvalue::Repeat(o, n) => format!("[{}; {n}]", operand(cx, o)),
        Rvalue::Ref(id, k, p) => {
            let m = if *k == BorrowKind::Unique { "!" } else { "" };
            format!("&{m}{} /*b{}*/", place(p), id.0)
        }
        Rvalue::Len(p) => format!("Len({})", place(p)),
        Rvalue::Cast(o, t) => format!("{} as {}", operand(cx, o), cx.ty(t)),
        Rvalue::CastFits(o, t) => format!("CastFits({} as {})", operand(cx, o), cx.ty(t)),
        Rvalue::BinaryOp(op, a, c) => {
            format!("{}({}, {})", op.name(), operand(cx, a), operand(cx, c))
        }
        Rvalue::CheckedBinaryOp(op, a, c) => {
            format!(
                "Checked{}({}, {})",
                op.name(),
                operand(cx, a),
                operand(cx, c)
            )
        }
        Rvalue::UnaryOp(UnOp::Not, o) => format!("Not({})", operand(cx, o)),
        Rvalue::UnaryOp(UnOp::Neg, o) => format!("Neg({})", operand(cx, o)),
        Rvalue::Discriminant(p) => format!("discriminant({})", place(p)),
        Rvalue::Aggregate(k, v) => match k {
            AggregateKind::Tuple => {
                if v.len() == 1 {
                    format!("({},)", ops(cx, v))
                } else {
                    format!("({})", ops(cx, v))
                }
            }
            AggregateKind::Array(_) => format!("[{}]", ops(cx, v)),
            AggregateKind::Adt(d, var, args) => {
                let adt_ty = cx.ty(&Ty::Adt(*d, args.clone()));
                match cx.db.adts.get(d).map(|a| &a.kind) {
                    Some(AdtKind::Struct(fs)) => {
                        let parts: Vec<String> = fs
                            .iter()
                            .zip(v)
                            .map(|(f, o)| format!("{}: {}", f.name, operand(cx, o)))
                            .collect();
                        format!("{adt_ty} {{ {} }}", parts.join(", "))
                    }
                    Some(AdtKind::Enum(vs)) => {
                        let name = vs
                            .get(*var as usize)
                            .map_or_else(|| format!("#{var}"), |x| x.name.clone());
                        if v.is_empty() {
                            format!("{adt_ty}::{name}")
                        } else {
                            format!("{adt_ty}::{name}({})", ops(cx, v))
                        }
                    }
                    None => format!("{adt_ty}#{var}({})", ops(cx, v)),
                }
            }
        },
    }
}

fn callee(cx: &PrintCx<'_>, c: &Callee) -> String {
    match c {
        Callee::Fn { node, generics } => format!("{}{}", cx.key(*node), gens(cx, generics)),
        Callee::TraitMethod {
            trait_def,
            member,
            self_ty,
            generics,
        } => {
            let tname = cx.defs[trait_def.index()].path.clone();
            let mname = cx
                .db
                .traits
                .get(trait_def)
                .and_then(|t| t.members.get(*member))
                .map_or_else(|| format!("#{member}"), |m| m.name.clone());
            format!(
                "<{} as {tname}>::{mname}{}",
                cx.ty(self_ty),
                gens(cx, generics)
            )
        }
        Callee::Value(o) => format!("({})", operand(cx, o)),
    }
}

fn assert_kind(cx: &PrintCx<'_>, k: &AssertKind) -> String {
    match k {
        AssertKind::Overflow(op, a, b) => {
            format!(
                "overflow {}({}, {})",
                op.name(),
                operand(cx, a),
                operand(cx, b)
            )
        }
        AssertKind::OverflowNeg(a) => format!("overflow_neg({})", operand(cx, a)),
        AssertKind::DivisionByZero(a) => format!("div_by_zero({})", operand(cx, a)),
        AssertKind::RemainderByZero(a) => format!("rem_by_zero({})", operand(cx, a)),
        AssertKind::IndexOutOfBounds { len, index } => format!(
            "index_out_of_bounds(len {}, index {})",
            operand(cx, len),
            operand(cx, index)
        ),
        AssertKind::SliceRange { from, to, len } => format!(
            "slice_range({}..{}, len {})",
            operand(cx, from),
            operand(cx, to),
            operand(cx, len)
        ),
        AssertKind::CastLossy { value, to } => {
            format!("cast_lossy({} as {})", operand(cx, value), cx.ty(to))
        }
    }
}

fn terminator(cx: &PrintCx<'_>, b: &Body, t: &TerminatorKind) -> String {
    let _ = b;
    match t {
        TerminatorKind::Goto(x) => format!("goto -> bb{};", x.0),
        TerminatorKind::SwitchInt {
            discr,
            targets,
            otherwise,
        } => {
            let mut parts: Vec<String> = targets
                .iter()
                .map(|(v, bb)| format!("{v}: bb{}", bb.0))
                .collect();
            parts.push(format!("otherwise: bb{}", otherwise.0));
            format!(
                "switchInt({}) -> [{}];",
                operand(cx, discr),
                parts.join(", ")
            )
        }
        TerminatorKind::Return => "return;".to_string(),
        TerminatorKind::Unreachable => "unreachable;".to_string(),
        TerminatorKind::Drop { place: p, target } => {
            format!("drop({}) -> bb{};", place(p), target.0)
        }
        TerminatorKind::Call {
            callee: c,
            args,
            dest,
            target,
            ..
        } => {
            let tgt = match target {
                Some(x) => format!("bb{}", x.0),
                None => "!".to_string(),
            };
            format!(
                "{} = {}({}) -> {tgt};",
                place(dest),
                callee(cx, c),
                ops(cx, args)
            )
        }
        TerminatorKind::Assert {
            cond,
            expected,
            kind,
            target,
        } => {
            let neg = if *expected { "" } else { "!" };
            format!(
                "assert({neg}{}, {}) -> bb{};",
                operand(cx, cond),
                assert_kind(cx, kind),
                target.0
            )
        }
    }
}
