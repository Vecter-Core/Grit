//! Thông dịch GLIR theo mô hình BYTE (Rust thuần, ĐỘC LẬP với gt-mir/gt-glirgen): oracle ngữ nghĩa cho đường hạ MIR→GLIR (ADR-P2-02; thiết kế cố vấn Opus 2026-11-25).
//! Mỗi slot là một vùng byte có cỡ do `layout` (bố cục GLIR) quyết định; con trỏ là `(vùng, offset)` và mang NGUỒN GỐC (provenance) khi nằm trong bộ nhớ.
//! Hành vi KHÔNG xác định trả `Ub` (KHÔNG phải `Abort`) để một bước hạ quên chốt chặn lộ ra: chia 0, `MIN/-1`, dịch ≥ độ rộng, đọc byte chưa ghi, con trỏ ngoài vùng,
//! đọc con trỏ từ byte không phải mảnh con trỏ nguyên vẹn, gọi hàm ngoài module. Chương trình hạ đúng không bao giờ chạm `Ub`.
use crate::layout::{field_offset, layout};
use crate::{BinOp, CastOp, CmpPred, Func, Inst, Module, Opd, Term, Ty};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// giá trị trả về (mẫu bit đã cắt) hoặc `None` nếu `ret void`
    Value(Option<u128>),
    /// giá trị tổng hợp trả về (struct/mảng) dưới dạng byte (`None` = byte chưa ghi/đệm)
    Agg(Vec<Option<u8>>),
    /// con trỏ trả về (chỉ có nghĩa trong cùng một lần chạy: `(vùng, offset)`)
    Ptr(usize, u64),
    Abort(String),
    Ub(String),
    OutOfFuel,
}

/// Đối số của hàm GLIR: số nguyên (mẫu bit) hoặc giá trị tổng hợp dạng byte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg {
    Int(u128),
    Bytes(Vec<Option<u8>>),
    /// con trỏ tới một vùng mới có nội dung ban đầu `bytes` (để thử hàm nhận `&T`/`&!T`)
    Ref(Vec<Option<u8>>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Byte {
    Uninit,
    B(u8),
    /// mảnh `k` (0..8) của con trỏ `(vùng, offset)`
    P(usize, u64, u8),
}

#[derive(Clone, Debug)]
enum Val {
    Int(u128),
    Ptr(usize, u64),
    /// giá trị tổng hợp (struct/mảng) dưới dạng byte, kể cả byte chưa ghi
    Agg(Vec<Byte>),
}

fn mask(w: u32) -> u128 {
    if w >= 128 {
        u128::MAX
    } else {
        (1u128 << w) - 1
    }
}

fn sext(v: u128, w: u32) -> i128 {
    if w >= 128 {
        v as i128
    } else if (v >> (w - 1)) & 1 == 1 {
        (v | !mask(w)) as i128
    } else {
        v as i128
    }
}

struct St<'a> {
    f: &'a Func,
    env: BTreeMap<String, Val>,
    /// tên slot → vùng
    slot: BTreeMap<String, usize>,
    mem: &'a mut Vec<Vec<Byte>>,
    /// tên global → vùng (các vùng `< nro` là CHỈ ĐỌC)
    gl: &'a BTreeMap<String, usize>,
    nro: usize,
}

impl St<'_> {
    fn opd(&self, o: &Opd) -> Result<Val, String> {
        match o {
            Opd::Val(v) => self
                .env
                .get(v)
                .cloned()
                .ok_or_else(|| format!("%{v} chưa có giá trị")),
            Opd::Slot(s) => Ok(Val::Ptr(*self.slot.get(s).ok_or("slot lạ")?, 0)),
            Opd::Global(g) => Ok(Val::Ptr(*self.gl.get(g).ok_or("global lạ")?, 0)),
            Opd::Int(t, n) => Ok(Val::Int(
                (*n as u128) & mask(t.bits().ok_or("hằng không nguyên")?),
            )),
        }
    }
    fn int(&self, o: &Opd) -> Result<u128, String> {
        match self.opd(o)? {
            Val::Int(v) => Ok(v),
            Val::Ptr(..) | Val::Agg(_) => Err("cần số nguyên".into()),
        }
    }
    fn ptr(&self, o: &Opd) -> Result<(usize, u64), String> {
        match self.opd(o)? {
            Val::Ptr(a, off) => Ok((a, off)),
            Val::Int(_) | Val::Agg(_) => Err("cần con trỏ".into()),
        }
    }
    fn range(&self, a: usize, off: u64, n: u64) -> Result<std::ops::Range<usize>, String> {
        let len = self.mem[a].len() as u64;
        if off.checked_add(n).is_none_or(|e| e > len) {
            return Err(format!("truy cập [{off}, {off}+{n}) ngoài vùng cỡ {len}"));
        }
        Ok(off as usize..(off + n) as usize)
    }
    fn read(&self, a: usize, off: u64, t: &Ty) -> Result<Vec<Byte>, String> {
        let n = layout(t).size;
        let r = self.range(a, off, n)?;
        Ok(self.mem[a][r].to_vec())
    }
    fn write(&mut self, a: usize, off: u64, bytes: &[Byte]) -> Result<(), String> {
        if a < self.nro {
            return Err("ghi vào vùng chỉ-đọc (global)".into());
        }
        let r = self.range(a, off, bytes.len() as u64)?;
        self.mem[a][r].copy_from_slice(bytes);
        Ok(())
    }
}

fn int_bytes(v: u128, n: u64) -> Vec<Byte> {
    (0..n)
        .map(|i| Byte::B(((v >> (8 * i)) & 0xff) as u8))
        .collect()
}

fn decode_int(bs: &[Byte], w: u32) -> Result<u128, String> {
    let mut v = 0u128;
    for (i, b) in bs.iter().enumerate() {
        match b {
            Byte::B(x) => v |= u128::from(*x) << (8 * i),
            Byte::Uninit => return Err("đọc byte chưa ghi".into()),
            Byte::P(..) => return Err("đọc số nguyên từ mảnh con trỏ".into()),
        }
    }
    Ok(v & mask(w))
}

fn decode_ptr(bs: &[Byte]) -> Result<(usize, u64), String> {
    let mut first = None;
    for (i, b) in bs.iter().enumerate() {
        match b {
            Byte::P(a, off, k) if usize::from(*k) == i && first.is_none_or(|f| f == (*a, *off)) => {
                first = Some((*a, *off))
            }
            Byte::Uninit => return Err("đọc con trỏ từ byte chưa ghi".into()),
            _ => return Err("đọc con trỏ từ byte không phải mảnh con trỏ nguyên vẹn".into()),
        }
    }
    first.ok_or_else(|| "con trỏ rỗng".into())
}

fn binop(op: BinOp, w: u32, a: u128, b: u128) -> Result<u128, String> {
    let m = mask(w);
    let (sa, sb) = (sext(a, w), sext(b, w));
    let min = if w >= 128 {
        i128::MIN
    } else {
        -(1i128 << (w - 1))
    };
    Ok(match op {
        BinOp::Add => a.wrapping_add(b) & m,
        BinOp::Sub => a.wrapping_sub(b) & m,
        BinOp::Mul => a.wrapping_mul(b) & m,
        BinOp::UDiv => {
            if b == 0 {
                return Err("udiv cho 0".into());
            }
            a / b
        }
        BinOp::URem => {
            if b == 0 {
                return Err("urem cho 0".into());
            }
            a % b
        }
        BinOp::SDiv => {
            if sb == 0 || (sa == min && sb == -1) {
                return Err("sdiv: chia 0 hoặc MIN/-1".into());
            }
            (sa / sb) as u128 & m
        }
        BinOp::SRem => {
            if sb == 0 || (sa == min && sb == -1) {
                return Err("srem: chia 0 hoặc MIN%-1".into());
            }
            (sa % sb) as u128 & m
        }
        BinOp::And => a & b,
        BinOp::Or => a | b,
        BinOp::Xor => a ^ b,
        BinOp::Shl | BinOp::LShr | BinOp::AShr => {
            if b >= u128::from(w) {
                return Err("dịch ≥ độ rộng".into());
            }
            let s = b as u32;
            match op {
                BinOp::Shl => (a << s) & m,
                BinOp::LShr => a >> s,
                _ => ((sa >> s) as u128) & m,
            }
        }
    })
}

fn cmp(p: CmpPred, w: u32, a: u128, b: u128) -> bool {
    let (sa, sb) = (sext(a, w), sext(b, w));
    match p {
        CmpPred::Eq => a == b,
        CmpPred::Ne => a != b,
        CmpPred::Ult => a < b,
        CmpPred::Ule => a <= b,
        CmpPred::Ugt => a > b,
        CmpPred::Uge => a >= b,
        CmpPred::Slt => sa < sb,
        CmpPred::Sle => sa <= sb,
        CmpPred::Sgt => sa > sb,
        CmpPred::Sge => sa >= sb,
    }
}

/// Chạy `f` với đối số nguyên `args` (mẫu bit; sẽ được cắt theo kiểu tham số). `fuel` = số lệnh tối đa.
#[must_use]
pub fn run(f: &Func, args: &[u128], fuel: u64) -> Outcome {
    let m = Module {
        globals: Vec::new(),
        decls: Vec::new(),
        funcs: vec![f.clone()],
    };
    run_module(&m, &f.name, args, fuel)
}

/// Như `run` nhưng gọi được hàm khác trong `m` (đệ quy tới độ sâu 128, quá ⇒ `OutOfFuel`; hàm `declare` ngoài module ⇒ `Ub`).
#[must_use]
pub fn run_module(m: &Module, name: &str, args: &[u128], fuel: u64) -> Outcome {
    let a: Vec<Arg> = args.iter().map(|x| Arg::Int(*x)).collect();
    run_module_args(m, name, &a, fuel)
}

/// Như `run_module` với đối số có thể là giá trị tổng hợp (không có `Arg::Ref`).
#[must_use]
pub fn run_module_args(m: &Module, name: &str, args: &[Arg], fuel: u64) -> Outcome {
    run_module_refs(m, name, args, fuel).0
}

/// Như `run_module_args`, thêm: với mỗi `Arg::Ref` (theo thứ tự) trả NỘI DUNG cuối của vùng nó trỏ tới (để so với giá trị sau lời gọi của thông dịch MIR).
#[must_use]
pub fn run_module_refs(
    m: &Module,
    name: &str,
    args: &[Arg],
    mut fuel: u64,
) -> (Outcome, Vec<Vec<Option<u8>>>) {
    let Some(f) = m.funcs.iter().find(|f| f.name == name) else {
        return (Outcome::Ub(format!("không có hàm @{name}")), Vec::new());
    };
    let mut mem: Vec<Vec<Byte>> = Vec::new();
    // các global chiếm các vùng đầu (chỉ đọc)
    let mut gl: BTreeMap<String, usize> = BTreeMap::new();
    for g in &m.globals {
        gl.insert(g.name.clone(), mem.len());
        mem.push(g.bytes.iter().map(|b| Byte::B(*b)).collect());
    }
    let nro = mem.len();
    let mut refs: Vec<usize> = Vec::new();
    let vals: Vec<Val> = args
        .iter()
        .map(|a| match a {
            Arg::Int(x) => Val::Int(*x),
            Arg::Bytes(bs) => {
                Val::Agg(bs.iter().map(|b| b.map_or(Byte::Uninit, Byte::B)).collect())
            }
            Arg::Ref(bs) => {
                mem.push(bs.iter().map(|b| b.map_or(Byte::Uninit, Byte::B)).collect());
                refs.push(mem.len() - 1);
                Val::Ptr(mem.len() - 1, 0)
            }
        })
        .collect();
    let out = exec(m, f, &vals, &mut fuel, 0, &mut mem, &Glob { gl, nro });
    let fin = refs
        .iter()
        .map(|i| {
            mem[*i]
                .iter()
                .map(|b| match b {
                    Byte::B(x) => Some(*x),
                    _ => None,
                })
                .collect()
        })
        .collect();
    (out, fin)
}

/// Môi trường chung của một lần chạy: tên global → vùng và số vùng chỉ-đọc đứng đầu.
struct Glob {
    gl: BTreeMap<String, usize>,
    nro: usize,
}

fn exec(
    m: &Module,
    f: &Func,
    args: &[Val],
    fuel: &mut u64,
    depth: u32,
    mem: &mut Vec<Vec<Byte>>,
    glob: &Glob,
) -> Outcome {
    let mut st = St {
        f,
        env: BTreeMap::new(),
        slot: BTreeMap::new(),
        mem,
        gl: &glob.gl,
        nro: glob.nro,
    };
    if args.len() != f.params.len() {
        return Outcome::Ub("sai số đối số".into());
    }
    for ((n, t), a) in f.params.iter().zip(args) {
        let v = match (t, a) {
            (t, Val::Int(x)) if t.is_int() => Val::Int(x & mask(t.bits().unwrap_or(128))),
            (Ty::Struct(_) | Ty::Array(..), Val::Agg(bs)) if bs.len() as u64 == layout(t).size => {
                Val::Agg(bs.clone())
            }
            (Ty::Ptr, Val::Ptr(a, o)) => Val::Ptr(*a, *o),
            _ => return Outcome::Ub("tham số không khớp kiểu".into()),
        };
        st.env.insert(n.clone(), v);
    }
    for (n, t) in &f.slots {
        st.slot.insert(n.clone(), st.mem.len());
        st.mem.push(vec![Byte::Uninit; layout(t).size as usize]);
    }
    let ub = |m: String| Outcome::Ub(m);
    let mut cur = 0usize;
    let mut prev: Option<usize> = None;
    loop {
        let b = &st.f.blocks[cur];
        // phi chạy đồng thời theo khối tiền nhiệm
        let mut phis: Vec<(String, Val)> = Vec::new();
        for i in &b.insts {
            if let Inst::Phi { d, ins, .. } = i {
                let Some(p) = prev else {
                    return ub("phi ở khối vào".into());
                };
                let Some((v, _)) = ins.iter().find(|(_, bn)| *bn == st.f.blocks[p].name) else {
                    return ub("phi thiếu nguồn".into());
                };
                match st.opd(v) {
                    Ok(x) => phis.push((d.clone(), x)),
                    Err(e) => return ub(e),
                }
            }
        }
        for (d, v) in phis {
            st.env.insert(d, v);
        }
        for i in &b.insts {
            if *fuel == 0 {
                return Outcome::OutOfFuel;
            }
            *fuel -= 1;
            if let Inst::Call {
                d,
                callee,
                args: cargs,
                ..
            } = i
            {
                if depth >= 128 {
                    return Outcome::OutOfFuel;
                }
                let mut vals = Vec::new();
                for a in cargs {
                    match st.opd(a) {
                        Ok(v) => vals.push(v),
                        Err(e) => return ub(e),
                    }
                }
                let Some(g) = m.funcs.iter().find(|g| g.name == *callee) else {
                    return ub(format!("gọi @{callee} ngoài module"));
                };
                match exec(m, g, &vals, fuel, depth + 1, st.mem, glob) {
                    Outcome::Value(v) => {
                        if let (Some(d), Some(v)) = (d, v) {
                            st.env.insert(d.clone(), Val::Int(v));
                        }
                    }
                    Outcome::Ptr(a, o) => {
                        if let Some(d) = d {
                            st.env.insert(d.clone(), Val::Ptr(a, o));
                        }
                    }
                    Outcome::Agg(bs) => {
                        if let Some(d) = d {
                            let bytes =
                                bs.iter().map(|b| b.map_or(Byte::Uninit, Byte::B)).collect();
                            st.env.insert(d.clone(), Val::Agg(bytes));
                        }
                    }
                    other => return other,
                }
                continue;
            }
            let r: Result<(), String> = (|| {
                match i {
                    Inst::Phi { .. } | Inst::Call { .. } => {}
                    Inst::Bin { d, op, ty, a, b } => {
                        let w = ty.bits().ok_or("bin không nguyên")?;
                        let v = binop(*op, w, st.int(a)?, st.int(b)?)?;
                        st.env.insert(d.clone(), Val::Int(v));
                    }
                    Inst::ICmp { d, pred, ty, a, b } => {
                        let v = if *ty == Ty::Ptr {
                            let (pa, pb) = (st.ptr(a)?, st.ptr(b)?);
                            match pred {
                                CmpPred::Eq => pa == pb,
                                CmpPred::Ne => pa != pb,
                                _ => return Err("so sánh thứ tự trên con trỏ".into()),
                            }
                        } else {
                            let w = ty.bits().ok_or("icmp không nguyên")?;
                            cmp(*pred, w, st.int(a)?, st.int(b)?)
                        };
                        st.env.insert(d.clone(), Val::Int(u128::from(v)));
                    }
                    Inst::Select { d, c, a, b, .. } => {
                        let v = if st.int(c)? & 1 == 1 {
                            st.opd(a)?
                        } else {
                            st.opd(b)?
                        };
                        st.env.insert(d.clone(), v);
                    }
                    Inst::Cast { d, op, from, v, to } => {
                        let (wf, wt) = (from.bits().ok_or("cast")?, to.bits().ok_or("cast")?);
                        let x = st.int(v)? & mask(wf);
                        let r = match op {
                            CastOp::Trunc => x & mask(wt),
                            CastOp::ZExt => x,
                            CastOp::SExt => (sext(x, wf) as u128) & mask(wt),
                        };
                        st.env.insert(d.clone(), Val::Int(r));
                    }
                    Inst::Load { d, ty, ptr } => {
                        let (a, off) = st.ptr(ptr)?;
                        let bs = st.read(a, off, ty)?;
                        let v = match ty {
                            Ty::Ptr => {
                                let (pa, po) = decode_ptr(&bs)?;
                                Val::Ptr(pa, po)
                            }
                            t if t.is_int() => Val::Int(decode_int(&bs, t.bits().unwrap_or(128))?),
                            Ty::Struct(_) | Ty::Array(..) => Val::Agg(bs),
                            _ => return Err("load kiểu không hỗ trợ".into()),
                        };
                        st.env.insert(d.clone(), v);
                    }
                    Inst::Store { ty, val, ptr } => {
                        let (a, off) = st.ptr(ptr)?;
                        let bytes = match (ty, st.opd(val)?) {
                            (Ty::Ptr, Val::Ptr(pa, po)) => {
                                (0..8u8).map(|k| Byte::P(pa, po, k)).collect()
                            }
                            (t, Val::Int(v)) if t.is_int() => int_bytes(v, layout(t).size),
                            (Ty::Struct(_) | Ty::Array(..), Val::Agg(bs))
                                if bs.len() as u64 == layout(ty).size =>
                            {
                                bs
                            }
                            _ => return Err("store: kiểu/giá trị không khớp".into()),
                        };
                        st.write(a, off, &bytes)?;
                    }
                    Inst::FieldPtr { d, ty, ptr, idx } => {
                        let (a, off) = st.ptr(ptr)?;
                        let fo = field_offset(ty, *idx).ok_or("fieldptr ngoài struct")?;
                        let total = layout(ty).size;
                        if off + total > st.mem[a].len() as u64 {
                            return Err("fieldptr: struct vượt vùng".into());
                        }
                        st.env.insert(d.clone(), Val::Ptr(a, off + fo));
                    }
                    Inst::IndexPtr { d, elem, ptr, idx } => {
                        let (a, off) = st.ptr(ptr)?;
                        let i = sext(st.int(idx)?, 64);
                        let n = i128::from(layout(elem).size) * i;
                        let no = i128::from(off) + n;
                        if no < 0 || no > st.mem[a].len() as i128 {
                            return Err("indexptr ra ngoài vùng".into());
                        }
                        st.env.insert(d.clone(), Val::Ptr(a, no as u64));
                    }
                }
                Ok(())
            })();
            if let Err(e) = r {
                return ub(e);
            }
        }
        let next = |n: &str| st.f.blocks.iter().position(|x| x.name == n);
        match &b.term {
            Term::Br(t) => {
                let Some(n) = next(t) else {
                    return ub("đích lạ".into());
                };
                prev = Some(cur);
                cur = n;
            }
            Term::CondBr(c, t, e) => {
                let v = match st.int(c) {
                    Ok(v) => v,
                    Err(e) => return ub(e),
                };
                let Some(n) = next(if v & 1 == 1 { t } else { e }) else {
                    return ub("đích lạ".into());
                };
                prev = Some(cur);
                cur = n;
            }
            Term::Ret(None) => return Outcome::Value(None),
            Term::Ret(Some((t, v))) => {
                if *t == Ty::Ptr {
                    return match st.opd(v) {
                        Ok(Val::Ptr(a, o)) => Outcome::Ptr(a, o),
                        Ok(_) => ub("ret ptr không phải con trỏ".into()),
                        Err(e) => ub(e),
                    };
                }
                if matches!(t, Ty::Struct(_) | Ty::Array(..)) {
                    return match st.opd(v) {
                        Ok(Val::Agg(bs)) => {
                            let mut out = Vec::new();
                            for b in bs {
                                match b {
                                    Byte::B(x) => out.push(Some(x)),
                                    Byte::Uninit => out.push(None),
                                    Byte::P(..) => return ub("trả về giá trị chứa con trỏ".into()),
                                }
                            }
                            Outcome::Agg(out)
                        }
                        Ok(_) => ub("ret tổng hợp không phải giá trị tổng hợp".into()),
                        Err(e) => ub(e),
                    };
                }
                let w = t.bits().unwrap_or(128);
                return match st.int(v) {
                    Ok(x) => Outcome::Value(Some(x & mask(w))),
                    Err(e) => ub(e),
                };
            }
            Term::Abort(k) => return Outcome::Abort(k.clone()),
        }
        if *fuel == 0 {
            return Outcome::OutOfFuel;
        }
        *fuel -= 1;
    }
}
