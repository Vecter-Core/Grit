//! In GLIR ra văn bản chuẩn (tất định). `print(parse(print(m))) == print(m)`.
use crate::{
    AtomicOrd, BinOp, Block, CastOp, CmpPred, Decl, FBinOp, FCmpPred, FUnOp, Func, Inst, Module,
    Opd, RmwOp, Term, Ty,
};
use std::fmt::Write;

#[must_use]
pub fn ty(t: &Ty) -> String {
    match t {
        Ty::I1 => "i1".into(),
        Ty::I8 => "i8".into(),
        Ty::I16 => "i16".into(),
        Ty::I32 => "i32".into(),
        Ty::I64 => "i64".into(),
        Ty::I128 => "i128".into(),
        Ty::F64 => "f64".into(),
        Ty::Ptr => "ptr".into(),
        Ty::Void => "void".into(),
        Ty::Struct(fs) => format!("{{{}}}", fs.iter().map(ty).collect::<Vec<_>>().join(", ")),
        Ty::Array(e, n) => format!("[{n} x {}]", ty(e)),
    }
}

fn opd(o: &Opd) -> String {
    match o {
        Opd::Val(v) => format!("%{v}"),
        Opd::Slot(s) => format!("${s}"),
        Opd::Global(g) => format!("@{g}"),
        Opd::Null => "null".into(),
        Opd::Int(t, n) => format!("{} {n}", ty(t)),
        Opd::F64(b) => format!("f64 0x{b:016x}"),
    }
}

pub(crate) const BINOPS: [(BinOp, &str); 13] = [
    (BinOp::Add, "add"),
    (BinOp::Sub, "sub"),
    (BinOp::Mul, "mul"),
    (BinOp::UDiv, "udiv"),
    (BinOp::SDiv, "sdiv"),
    (BinOp::URem, "urem"),
    (BinOp::SRem, "srem"),
    (BinOp::And, "and"),
    (BinOp::Or, "or"),
    (BinOp::Xor, "xor"),
    (BinOp::Shl, "shl"),
    (BinOp::LShr, "lshr"),
    (BinOp::AShr, "ashr"),
];
pub(crate) const PREDS: [(CmpPred, &str); 10] = [
    (CmpPred::Eq, "eq"),
    (CmpPred::Ne, "ne"),
    (CmpPred::Ult, "ult"),
    (CmpPred::Ule, "ule"),
    (CmpPred::Ugt, "ugt"),
    (CmpPred::Uge, "uge"),
    (CmpPred::Slt, "slt"),
    (CmpPred::Sle, "sle"),
    (CmpPred::Sgt, "sgt"),
    (CmpPred::Sge, "sge"),
];
pub(crate) const CASTS: [(CastOp, &str); 7] = [
    (CastOp::ZExt, "zext"),
    (CastOp::SExt, "sext"),
    (CastOp::Trunc, "trunc"),
    (CastOp::SIToFP, "sitofp"),
    (CastOp::UIToFP, "uitofp"),
    (CastOp::FPToSI, "fptosi"),
    (CastOp::BitCast, "bitcast"),
];
pub(crate) const FBINOPS: [(FBinOp, &str); 4] = [
    (FBinOp::Add, "fadd"),
    (FBinOp::Sub, "fsub"),
    (FBinOp::Mul, "fmul"),
    (FBinOp::Div, "fdiv"),
];
pub(crate) const ORDS: [(AtomicOrd, &str); 5] = [
    (AtomicOrd::Monotonic, "monotonic"),
    (AtomicOrd::Acquire, "acquire"),
    (AtomicOrd::Release, "release"),
    (AtomicOrd::AcqRel, "acq_rel"),
    (AtomicOrd::SeqCst, "seq_cst"),
];
pub(crate) const RMWOPS: [(RmwOp, &str); 3] = [
    (RmwOp::Xchg, "xchg"),
    (RmwOp::Add, "add"),
    (RmwOp::Sub, "sub"),
];
pub(crate) const FUNOPS: [(FUnOp, &str); 2] = [(FUnOp::Neg, "fneg"), (FUnOp::Sqrt, "fsqrt")];
pub(crate) const FPREDS: [(FCmpPred, &str); 7] = [
    (FCmpPred::Oeq, "oeq"),
    (FCmpPred::Olt, "olt"),
    (FCmpPred::Ole, "ole"),
    (FCmpPred::Ogt, "ogt"),
    (FCmpPred::Oge, "oge"),
    (FCmpPred::Une, "une"),
    (FCmpPred::Uno, "uno"),
];

fn name<T: Copy + PartialEq>(tbl: &[(T, &'static str)], v: T) -> &'static str {
    tbl.iter().find(|(k, _)| *k == v).map_or("?", |(_, s)| s)
}

fn inst(i: &Inst) -> String {
    match i {
        Inst::Bin { d, op, ty: t, a, b } => format!(
            "%{d} = bin {} {} {}, {}",
            name(&BINOPS, *op),
            ty(t),
            opd(a),
            opd(b)
        ),
        Inst::ICmp {
            d,
            pred,
            ty: t,
            a,
            b,
        } => format!(
            "%{d} = icmp {} {} {}, {}",
            name(&PREDS, *pred),
            ty(t),
            opd(a),
            opd(b)
        ),
        Inst::FBin { d, op, ty: t, a, b } => format!(
            "%{d} = fbin {} {} {}, {}",
            name(&FBINOPS, *op),
            ty(t),
            opd(a),
            opd(b)
        ),
        Inst::FUn { d, op, ty: t, v } => {
            format!("%{d} = funop {} {} {}", name(&FUNOPS, *op), ty(t), opd(v))
        }
        Inst::FCmp {
            d,
            pred,
            ty: t,
            a,
            b,
        } => format!(
            "%{d} = fcmp {} {} {}, {}",
            name(&FPREDS, *pred),
            ty(t),
            opd(a),
            opd(b)
        ),
        Inst::AtomicLoad { d, ord, ty: t, ptr } => {
            format!(
                "%{d} = atomicload {} {} {}",
                name(&ORDS, *ord),
                ty(t),
                opd(ptr)
            )
        }
        Inst::AtomicStore {
            ord,
            ty: t,
            val,
            ptr,
        } => format!(
            "atomicstore {} {} {}, {}",
            name(&ORDS, *ord),
            ty(t),
            opd(val),
            opd(ptr)
        ),
        Inst::AtomicRmw {
            d,
            op,
            ord,
            ty: t,
            ptr,
            val,
        } => format!(
            "%{d} = atomicrmw {} {} {} {}, {}",
            name(&RMWOPS, *op),
            name(&ORDS, *ord),
            ty(t),
            opd(ptr),
            opd(val)
        ),
        Inst::CmpXchg {
            d,
            succ,
            fail,
            ty: t,
            ptr,
            cmp,
            new,
        } => format!(
            "%{d} = cmpxchg {} {} {} {}, {}, {}",
            name(&ORDS, *succ),
            name(&ORDS, *fail),
            ty(t),
            opd(ptr),
            opd(cmp),
            opd(new)
        ),
        Inst::Select { d, ty: t, c, a, b } => {
            format!("%{d} = select {} {}, {}, {}", ty(t), opd(c), opd(a), opd(b))
        }
        Inst::Cast { d, op, from, v, to } => format!(
            "%{d} = cast {} {} {} to {}",
            name(&CASTS, *op),
            ty(from),
            opd(v),
            ty(to)
        ),
        Inst::Load { d, ty: t, ptr } => format!("%{d} = load {} {}", ty(t), opd(ptr)),
        Inst::Store { ty: t, val, ptr } => format!("store {} {}, {}", ty(t), opd(val), opd(ptr)),
        Inst::IndexPtr { d, elem, ptr, idx } => {
            format!("%{d} = indexptr {} {}, {}", ty(elem), opd(ptr), opd(idx))
        }
        Inst::FieldPtr { d, ty: t, ptr, idx } => {
            format!("%{d} = fieldptr {} {}, {idx}", ty(t), opd(ptr))
        }
        Inst::Call {
            d,
            ret,
            callee,
            args,
        } => {
            let a = args.iter().map(opd).collect::<Vec<_>>().join(", ");
            match d {
                Some(d) => format!("%{d} = call {} @{callee}({a})", ty(ret)),
                None => format!("call {} @{callee}({a})", ty(ret)),
            }
        }
        Inst::Phi { d, ty: t, ins } => format!(
            "%{d} = phi {} {}",
            ty(t),
            ins.iter()
                .map(|(v, b)| format!("[{}, {b}]", opd(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn term(t: &Term) -> String {
    match t {
        Term::Br(b) => format!("br {b}"),
        Term::CondBr(c, a, b) => format!("condbr {}, {a}, {b}", opd(c)),
        Term::Ret(None) => "ret void".into(),
        Term::Ret(Some((t, v))) => format!("ret {} {}", ty(t), opd(v)),
        Term::Abort(k) => format!("abort {k}"),
    }
}

fn block(b: &Block, out: &mut String) {
    let _ = writeln!(out, "{}:", b.name);
    for i in &b.insts {
        let _ = writeln!(out, "  {}", inst(i));
    }
    let _ = writeln!(out, "  {}", term(&b.term));
}

fn decl(d: &Decl) -> String {
    format!(
        "declare @{}({}) -> {}",
        d.name,
        d.params.iter().map(ty).collect::<Vec<_>>().join(", "),
        ty(&d.ret)
    )
}

fn func(f: &Func, out: &mut String) {
    let ps = f
        .params
        .iter()
        .map(|(n, t)| format!("%{n}: {}", ty(t)))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(out, "fn @{}({ps}) -> {} {{", f.name, ty(&f.ret));
    for (n, t) in &f.slots {
        let _ = writeln!(out, "slot ${n}: {}", ty(t));
    }
    for b in &f.blocks {
        block(b, out);
    }
    let _ = writeln!(out, "}}");
}

#[must_use]
pub fn print(m: &Module) -> String {
    let mut out = String::from("glir 1\n");
    for g in &m.globals {
        let hex: String = g.bytes.iter().map(|b| format!("{b:02x}")).collect();
        let _ = writeln!(
            out,
            "global @{} = {}",
            g.name,
            if hex.is_empty() { "-" } else { &hex }
        );
    }
    for d in &m.decls {
        let _ = writeln!(out, "{}", decl(d));
    }
    for f in &m.funcs {
        func(f, &mut out);
    }
    out
}
