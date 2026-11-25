//! Verifier GLIR (Rust thuần, không phụ thuộc LLVM). Trả danh sách lỗi `Gxxxx: …` (rỗng ⇒ hợp lệ). Mã:
//! G01xx cấu trúc khối · G02xx định nghĩa/dùng SSA, slot · G03xx kiểu · G04xx phi.
use crate::{Func, Inst, Module, Opd, Term, Ty};
use std::collections::{BTreeMap, BTreeSet};

struct Cx<'a> {
    m: &'a Module,
    f: &'a Func,
    errs: Vec<String>,
}

impl Cx<'_> {
    fn err(&mut self, code: &str, msg: String) {
        self.errs.push(format!("{code}: @{}: {msg}", self.f.name));
    }
}

fn term_targets(t: &Term) -> Vec<&str> {
    match t {
        Term::Br(b) => vec![b],
        Term::CondBr(_, a, b) => vec![a, b],
        Term::Ret(_) | Term::Abort(_) => vec![],
    }
}

fn inst_def(i: &Inst) -> Option<&str> {
    match i {
        Inst::Bin { d, .. }
        | Inst::ICmp { d, .. }
        | Inst::Select { d, .. }
        | Inst::Cast { d, .. }
        | Inst::Load { d, .. }
        | Inst::FieldPtr { d, .. }
        | Inst::Phi { d, .. } => Some(d),
        Inst::Call { d, .. } => d.as_deref(),
        Inst::Store { .. } => None,
    }
}

fn ret_ty(i: &Inst) -> Option<Ty> {
    match i {
        Inst::Bin { ty, .. }
        | Inst::Select { ty, .. }
        | Inst::Load { ty, .. }
        | Inst::Phi { ty, .. } => Some(ty.clone()),
        Inst::ICmp { .. } => Some(Ty::I1),
        Inst::Cast { to, .. } => Some(to.clone()),
        Inst::FieldPtr { .. } => Some(Ty::Ptr),
        Inst::Call { ret, .. } => Some(ret.clone()),
        Inst::Store { .. } => None,
    }
}

pub fn verify(m: &Module) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for f in &m.funcs {
        if !seen.insert(&f.name) {
            out.push(format!("G0104: hàm `@{}` định nghĩa hai lần", f.name));
        }
        out.extend(verify_fn(m, f));
    }
    out
}

fn verify_fn(m: &Module, f: &Func) -> Vec<String> {
    let mut cx = Cx {
        m,
        f,
        errs: Vec::new(),
    };
    if f.blocks.is_empty() {
        cx.err("G0101", "hàm không có khối nào".into());
        return cx.errs;
    }
    let names: Vec<&str> = f.blocks.iter().map(|b| b.name.as_str()).collect();
    let idx: BTreeMap<&str, usize> = names.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    if idx.len() != names.len() {
        cx.err("G0102", "tên khối trùng".into());
        return cx.errs;
    }
    let mut slots: BTreeMap<&str, &Ty> = BTreeMap::new();
    for (n, t) in &f.slots {
        if slots.insert(n, t).is_some() {
            cx.err("G0203", format!("slot `${n}` khai báo hai lần"));
        }
    }
    // cạnh
    let n = f.blocks.len();
    let mut succ: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut pred: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, b) in f.blocks.iter().enumerate() {
        for t in term_targets(&b.term) {
            match idx.get(t) {
                Some(&j) => {
                    succ[i].push(j);
                    pred[j].push(i);
                }
                None => cx.err(
                    "G0103",
                    format!("khối `{}` nhảy tới `{t}` không tồn tại", b.name),
                ),
            }
        }
    }
    // tới được + dominator (tập, lặp tới điểm cố định; hàm nhỏ)
    let mut reach = vec![false; n];
    let mut stack = vec![0usize];
    while let Some(i) = stack.pop() {
        if !std::mem::replace(&mut reach[i], true) {
            stack.extend(succ[i].iter().copied());
        }
    }
    let all: BTreeSet<usize> = (0..n).filter(|i| reach[*i]).collect();
    let mut dom: Vec<BTreeSet<usize>> = (0..n)
        .map(|i| {
            if i == 0 {
                BTreeSet::from([0])
            } else {
                all.clone()
            }
        })
        .collect();
    loop {
        let mut changed = false;
        for i in 1..n {
            if !reach[i] {
                continue;
            }
            let mut d: Option<BTreeSet<usize>> = None;
            for &p in pred[i].iter().filter(|p| reach[**p]) {
                d = Some(match d {
                    None => dom[p].clone(),
                    Some(x) => x.intersection(&dom[p]).copied().collect(),
                });
            }
            let mut d = d.unwrap_or_default();
            d.insert(i);
            if d != dom[i] {
                dom[i] = d;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // định nghĩa: tên → (khối, chỉ số lệnh; param = (0, -1)), kiểu
    let mut def: BTreeMap<String, (usize, isize, Ty)> = BTreeMap::new();
    for (p, t) in &f.params {
        if def.insert(p.clone(), (0, -1, t.clone())).is_some() {
            cx.err("G0201", format!("`%{p}` định nghĩa hai lần"));
        }
    }
    for (bi, b) in f.blocks.iter().enumerate() {
        for (ii, i) in b.insts.iter().enumerate() {
            if let (Some(d), Some(t)) = (inst_def(i), ret_ty(i))
                && def.insert(d.to_string(), (bi, ii as isize, t)).is_some()
            {
                cx.err("G0201", format!("`%{d}` định nghĩa hai lần"));
            }
        }
    }
    let opd_ty =
        |cx: &mut Cx, o: &Opd, bi: usize, ii: isize, phi_pred: Option<usize>| -> Option<Ty> {
            match o {
                Opd::Int(t, _) => Some(t.clone()),
                Opd::Slot(s) => {
                    if slots.contains_key(s.as_str()) {
                        Some(Ty::Ptr)
                    } else {
                        cx.err("G0204", format!("slot `${s}` chưa khai báo"));
                        None
                    }
                }
                Opd::Val(v) => match def.get(v) {
                    None => {
                        cx.err("G0202", format!("`%{v}` chưa định nghĩa"));
                        None
                    }
                    Some((db, di, t)) => {
                        let ok = match phi_pred {
                            Some(p) => !reach[p] || dom[p].contains(db),
                            None => {
                                !reach[bi]
                                    || (*db == bi && *di < ii)
                                    || (*db != bi && dom[bi].contains(db))
                            }
                        };
                        if !ok {
                            cx.err("G0205", format!("`%{v}` không trội (dominate) chỗ dùng"));
                        }
                        Some(t.clone())
                    }
                },
            }
        };
    for (bi, b) in f.blocks.iter().enumerate() {
        let mut past_phi = false;
        for (ii, i) in b.insts.iter().enumerate() {
            let ii = ii as isize;
            if matches!(i, Inst::Phi { .. }) {
                if past_phi {
                    cx.err("G0402", format!("phi không ở đầu khối `{}`", b.name));
                }
            } else {
                past_phi = true;
            }
            check_inst(&mut cx, i, bi, ii, &opd_ty, f, &pred);
        }
        match &b.term {
            Term::CondBr(c, ..) => {
                if let Some(t) = opd_ty(&mut cx, c, bi, b.insts.len() as isize, None)
                    && t != Ty::I1
                {
                    cx.err("G0306", format!("condbr cần i1, có {t:?}"));
                }
            }
            Term::Ret(r) => match (r, &f.ret) {
                (None, Ty::Void) => {}
                (Some((t, v)), rt) if t == rt => {
                    if let Some(vt) = opd_ty(&mut cx, v, bi, b.insts.len() as isize, None)
                        && vt != *t
                    {
                        cx.err("G0303", format!("ret: toán hạng {vt:?} ≠ {t:?}"));
                    }
                }
                _ => cx.err("G0303", format!("ret không khớp kiểu trả về {:?}", f.ret)),
            },
            Term::Br(_) | Term::Abort(_) => {}
        }
    }
    cx.errs
}

fn check_inst(
    cx: &mut Cx,
    i: &Inst,
    bi: usize,
    ii: isize,
    opd_ty: &dyn Fn(&mut Cx, &Opd, usize, isize, Option<usize>) -> Option<Ty>,
    f: &Func,
    pred: &[Vec<usize>],
) {
    let expect = |cx: &mut Cx, o: &Opd, want: &Ty| {
        if let Some(t) = opd_ty(cx, o, bi, ii, None)
            && t != *want
        {
            cx.err("G0301", format!("toán hạng kiểu {t:?}, cần {want:?}"));
        }
    };
    match i {
        Inst::Bin { ty, a, b, .. } => {
            if !ty.is_int() {
                cx.err("G0304", format!("bin trên kiểu không nguyên {ty:?}"));
            }
            expect(cx, a, ty);
            expect(cx, b, ty);
        }
        Inst::ICmp { ty, a, b, .. } => {
            if !ty.is_int() && *ty != Ty::Ptr {
                cx.err("G0304", format!("icmp trên kiểu {ty:?}"));
            }
            expect(cx, a, ty);
            expect(cx, b, ty);
        }
        Inst::Select { ty, c, a, b, .. } => {
            expect(cx, c, &Ty::I1);
            expect(cx, a, ty);
            expect(cx, b, ty);
        }
        Inst::Cast {
            op, from, v, to, ..
        } => {
            expect(cx, v, from);
            match (from.bits(), to.bits()) {
                (Some(x), Some(y)) => {
                    let ok = match op {
                        crate::CastOp::Trunc => y < x,
                        crate::CastOp::ZExt | crate::CastOp::SExt => y > x,
                    };
                    if !ok {
                        cx.err("G0305", format!("cast {from:?}→{to:?} sai hướng độ rộng"));
                    }
                }
                _ => cx.err("G0305", "cast chỉ trên số nguyên".into()),
            }
        }
        Inst::Load { ptr, .. } => expect(cx, ptr, &Ty::Ptr),
        Inst::Store { ty, val, ptr } => {
            expect(cx, val, ty);
            expect(cx, ptr, &Ty::Ptr);
        }
        Inst::FieldPtr { ty, ptr, idx, .. } => {
            expect(cx, ptr, &Ty::Ptr);
            match ty {
                Ty::Struct(fs) if (*idx as usize) < fs.len() => {}
                _ => cx.err("G0307", format!("fieldptr {idx} ngoài struct {ty:?}")),
            }
        }
        Inst::Call {
            ret, callee, args, ..
        } => {
            let sig =
                cx.m.decls
                    .iter()
                    .find(|d| d.name == *callee)
                    .map(|d| (d.params.clone(), d.ret.clone()))
                    .or_else(|| {
                        cx.m.funcs.iter().find(|g| g.name == *callee).map(|g| {
                            (
                                g.params.iter().map(|p| p.1.clone()).collect(),
                                g.ret.clone(),
                            )
                        })
                    });
            match sig {
                None => cx.err("G0302", format!("gọi `@{callee}` không khai báo")),
                Some((ps, r)) => {
                    if r != *ret {
                        cx.err(
                            "G0302",
                            format!("`@{callee}` trả {r:?}, lời gọi ghi {ret:?}"),
                        );
                    }
                    if ps.len() != args.len() {
                        cx.err(
                            "G0302",
                            format!("`@{callee}` nhận {} đối số, có {}", ps.len(), args.len()),
                        );
                    } else {
                        for (a, p) in args.iter().zip(&ps) {
                            expect(cx, a, p);
                        }
                    }
                }
            }
        }
        Inst::Phi { ty, ins, .. } => {
            let names: BTreeSet<&str> = ins.iter().map(|(_, b)| b.as_str()).collect();
            let want: BTreeSet<&str> = pred[bi]
                .iter()
                .map(|p| f.blocks[*p].name.as_str())
                .collect();
            if names != want || names.len() != ins.len() {
                cx.err(
                    "G0401",
                    format!("phi: khối vào {names:?} ≠ khối tiền nhiệm {want:?}"),
                );
            }
            for (v, b) in ins {
                if let Some(p) = f.blocks.iter().position(|x| x.name == *b)
                    && let Some(t) = opd_ty(cx, v, bi, ii, Some(p))
                    && t != *ty
                {
                    cx.err("G0301", format!("phi: toán hạng {t:?}, cần {ty:?}"));
                }
            }
        }
    }
}
