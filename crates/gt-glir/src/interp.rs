//! Thông dịch GLIR (Rust thuần, ĐỘC LẬP với gt-mir/gt-glirgen): oracle ngữ nghĩa cho đường hạ MIR→GLIR (ADR-P2-02 C3; thiết kế cố vấn Opus).
//! Giá trị nguyên là mẫu bit không dấu cắt theo độ rộng (u128). Hành vi KHÔNG xác định trong GLIR (chia 0, `MIN/-1`, dịch ≥ độ rộng, đọc slot chưa ghi,
//! gọi hàm) trả `Ub` — KHÔNG phải `Abort` — để một bước hạ quên chốt chặn lộ ra ngay (chương trình hạ đúng không bao giờ chạm `Ub`).
use crate::{BinOp, CastOp, CmpPred, Func, Inst, Opd, Term, Ty};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// giá trị trả về (mẫu bit đã cắt) hoặc `None` nếu `ret void`
    Value(Option<u128>),
    Abort(String),
    Ub(String),
    OutOfFuel,
}

#[derive(Clone, Debug)]
enum Mem {
    Int(u128),
    Agg(Vec<Option<Mem>>),
}

#[derive(Clone, Debug)]
enum Val {
    Int(u128),
    /// (slot, đường trường)
    Ptr(String, Vec<u32>),
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

fn width(t: &Ty) -> Option<u32> {
    t.bits()
}

fn empty(t: &Ty) -> Option<Mem> {
    match t {
        Ty::Struct(fs) => Some(Mem::Agg(fs.iter().map(empty).collect())),
        Ty::Array(e, n) if *n <= 64 => Some(Mem::Agg((0..*n).map(|_| empty(e)).collect())),
        _ => None,
    }
}

struct St<'a> {
    f: &'a Func,
    env: BTreeMap<String, Val>,
    slots: BTreeMap<String, Option<Mem>>,
}

impl St<'_> {
    fn opd(&self, o: &Opd) -> Result<Val, String> {
        match o {
            Opd::Val(v) => self
                .env
                .get(v)
                .cloned()
                .ok_or_else(|| format!("%{v} chưa có giá trị")),
            Opd::Slot(s) => Ok(Val::Ptr(s.clone(), Vec::new())),
            Opd::Int(t, n) => Ok(Val::Int(
                (*n as u128) & mask(width(t).ok_or("hằng không nguyên")?),
            )),
        }
    }
    fn int(&self, o: &Opd) -> Result<u128, String> {
        match self.opd(o)? {
            Val::Int(v) => Ok(v),
            Val::Ptr(..) => Err("cần số nguyên, có con trỏ".into()),
        }
    }
    fn cell(&mut self, p: &Val) -> Result<&mut Option<Mem>, String> {
        let Val::Ptr(s, path) = p else {
            return Err("không phải con trỏ".into());
        };
        let mut cur = self.slots.get_mut(s).ok_or("slot lạ")?;
        for i in path {
            let Some(Mem::Agg(fs)) = cur else {
                return Err("truy cập trường của ô chưa khởi tạo/không phải struct".into());
            };
            cur = fs.get_mut(*i as usize).ok_or("trường ngoài phạm vi")?;
        }
        Ok(cur)
    }
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
pub fn run(f: &Func, args: &[u128], mut fuel: u64) -> Outcome {
    let mut st = St {
        f,
        env: BTreeMap::new(),
        slots: BTreeMap::new(),
    };
    if args.len() != f.params.len() {
        return Outcome::Ub("sai số đối số".into());
    }
    for ((n, t), a) in f.params.iter().zip(args) {
        let Some(w) = width(t) else {
            return Outcome::Ub("tham số không nguyên".into());
        };
        st.env.insert(n.clone(), Val::Int(a & mask(w)));
    }
    for (n, t) in &f.slots {
        st.slots.insert(n.clone(), empty(t));
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
            if fuel == 0 {
                return Outcome::OutOfFuel;
            }
            fuel -= 1;
            let r: Result<(), String> = (|| {
                match i {
                    Inst::Phi { .. } => {}
                    Inst::Bin { d, op, ty, a, b } => {
                        let w = width(ty).ok_or("bin không nguyên")?;
                        let v = binop(*op, w, st.int(a)?, st.int(b)?)?;
                        st.env.insert(d.clone(), Val::Int(v));
                    }
                    Inst::ICmp { d, pred, ty, a, b } => {
                        let w = width(ty).ok_or("icmp không nguyên")?;
                        let v = cmp(*pred, w, st.int(a)?, st.int(b)?);
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
                        let (wf, wt) = (width(from).ok_or("cast")?, width(to).ok_or("cast")?);
                        let x = st.int(v)? & mask(wf);
                        let r = match op {
                            CastOp::Trunc => x & mask(wt),
                            CastOp::ZExt => x,
                            CastOp::SExt => (sext(x, wf) as u128) & mask(wt),
                        };
                        st.env.insert(d.clone(), Val::Int(r));
                    }
                    Inst::Load { d, ptr, .. } => {
                        let p = st.opd(ptr)?;
                        match st.cell(&p)? {
                            Some(Mem::Int(v)) => {
                                let v = *v;
                                st.env.insert(d.clone(), Val::Int(v));
                            }
                            _ => return Err("đọc ô chưa ghi hoặc không phải số nguyên".into()),
                        }
                    }
                    Inst::Store { val, ptr, .. } => {
                        let v = st.int(val)?;
                        let p = st.opd(ptr)?;
                        *st.cell(&p)? = Some(Mem::Int(v));
                    }
                    Inst::FieldPtr { d, ptr, idx, .. } => {
                        let Val::Ptr(s, mut path) = st.opd(ptr)? else {
                            return Err("fieldptr trên giá trị không phải con trỏ".into());
                        };
                        path.push(*idx);
                        st.env.insert(d.clone(), Val::Ptr(s, path));
                    }
                    Inst::Call { .. } => return Err("gọi hàm: GLIR interp chưa hỗ trợ".into()),
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
                let w = width(t).unwrap_or(128);
                return match st.int(v) {
                    Ok(x) => Outcome::Value(Some(x & mask(w))),
                    Err(e) => ub(e),
                };
            }
            Term::Abort(k) => return Outcome::Abort(k.clone()),
        }
        if fuel == 0 {
            return Outcome::OutOfFuel;
        }
        fuel -= 1;
    }
}
