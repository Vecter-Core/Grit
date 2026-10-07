//! Thông dịch MIR cỡ nhỏ — CÔNG CỤ TEST, không thuộc TCB (ADR-020 §9a). Phạm vi (§9c): số nguyên, bool, rẽ nhánh, vòng lặp,
//! gọi hàm, struct/tuple/enum, tham chiếu tới chỗ. `Assert` sai ⇒ abort với loại nghĩa vụ (§9d). Đọc chỗ đã `StorageDead`
//! hoặc chưa gán ⇒ lỗi của MIR (`Bug`), tới `Unreachable` ⇒ `Bug`.
#![allow(dead_code)]
use gt_ast::NodeId;
use gt_mir::*;
use gt_types::{IntTy, Ty};

#[derive(Clone, Debug, PartialEq)]
pub enum V {
    Int(i128, IntTy),
    Bool(bool),
    Char(u32),
    Str(String),
    Unit,
    Tuple(Vec<V>),
    Array(Vec<V>),
    Adt(u32, Vec<V>),
    /// Tham chiếu: (khung, local, đường đi trong giá trị).
    Ref(usize, usize, Vec<Step>),
    Fn(NodeId),
    Uninit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Field(usize),
    Index(usize),
    Sub(usize, usize),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Value(V),
    Abort(String),
    Bug(String),
    OutOfFuel,
}

fn wrap(v: i128, t: IntTy) -> i128 {
    let bits = t.bits();
    if bits == 128 {
        return v;
    }
    let m = (1i128 << bits) - 1;
    let u = v & m;
    if t.signed() && (u >> (bits - 1)) & 1 == 1 {
        u - (1i128 << bits)
    } else {
        u
    }
}

fn in_range(v: i128, t: IntTy) -> bool {
    if t.bits() == 128 {
        return true;
    }
    v >= t.min_val() && (v < 0 || (v as u128) <= t.max_val())
}

fn from_bits(bits: u128, t: IntTy) -> i128 {
    if t.signed() {
        let shift = 128 - t.bits();
        ((bits << shift) as i128) >> shift
    } else {
        bits as i128
    }
}

struct Frame {
    locals: Vec<V>,
}

pub struct Interp<'a> {
    pub mir: &'a MirOutput,
    frames: Vec<Frame>,
    fuel: u64,
    /// vết các `Assert` đã chạy: (khoá thân, khối, đạt?) — để oracle M7 kiểm "tới được ⇒ sai"
    pub asserts: Vec<(String, u32, bool)>,
}

enum Flow {
    Ret,
    Abort(String),
    Bug(String),
    Fuel,
}

impl<'a> Interp<'a> {
    #[must_use]
    pub fn new(mir: &'a MirOutput, fuel: u64) -> Self {
        Interp {
            mir,
            frames: Vec::new(),
            fuel,
            asserts: Vec::new(),
        }
    }

    pub fn call_key(&mut self, key: &str, args: Vec<V>) -> Outcome {
        let Some(b) = self.mir.body(key) else {
            return Outcome::Bug(format!("không có thân {key}"));
        };
        self.run(b, args)
    }

    fn run(&mut self, b: &'a Body, args: Vec<V>) -> Outcome {
        if self.frames.len() > 200 {
            return Outcome::Bug("đệ quy quá sâu".to_string());
        }
        let mut locals = vec![V::Uninit; b.locals.len()];
        for (i, a) in args.into_iter().enumerate() {
            locals[i + 1] = a;
        }
        self.frames.push(Frame { locals });
        let r = self.exec(b);
        let fr = self.frames.pop();
        match r {
            Flow::Ret => match fr.map(|f| f.locals[0].clone()) {
                Some(V::Uninit) | None => Outcome::Bug("trả về `_0` chưa gán".to_string()),
                Some(v) => Outcome::Value(v),
            },
            Flow::Abort(k) => Outcome::Abort(k),
            Flow::Bug(m) => Outcome::Bug(m),
            Flow::Fuel => Outcome::OutOfFuel,
        }
    }

    fn top(&self) -> usize {
        self.frames.len() - 1
    }

    fn exec(&mut self, b: &'a Body) -> Flow {
        let mut cur = ENTRY;
        loop {
            if self.fuel == 0 {
                return Flow::Fuel;
            }
            self.fuel -= 1;
            let bd = &b.blocks[cur.index()];
            for s in &bd.statements {
                if let Err(e) = self.stmt(s) {
                    return e;
                }
            }
            let Some(t) = &bd.terminator else {
                return Flow::Bug("khối thiếu terminator".to_string());
            };
            match &t.kind {
                TerminatorKind::Goto(x) => cur = *x,
                TerminatorKind::Return => return Flow::Ret,
                TerminatorKind::Unreachable => {
                    return Flow::Bug(format!("tới `unreachable` ở bb{}", cur.0));
                }
                TerminatorKind::Drop { target, .. } => cur = *target,
                TerminatorKind::SwitchInt {
                    discr,
                    targets,
                    otherwise,
                } => {
                    let v = match self.operand(discr) {
                        Ok(v) => v,
                        Err(e) => return e,
                    };
                    let n: u128 = match v {
                        V::Bool(x) => u128::from(x),
                        V::Int(i, t) => {
                            (i as u128)
                                & if t.bits() == 128 {
                                    u128::MAX
                                } else {
                                    (1u128 << t.bits()) - 1
                                }
                        }
                        V::Char(c) => u128::from(c),
                        other => return Flow::Bug(format!("switchInt trên {other:?}")),
                    };
                    cur = targets
                        .iter()
                        .find(|(k, _)| *k == n)
                        .map_or(*otherwise, |(_, b)| *b);
                }
                TerminatorKind::Assert {
                    cond,
                    expected,
                    kind,
                    target,
                } => match self.operand(cond) {
                    Ok(V::Bool(x)) if x == *expected => {
                        self.asserts.push((b.key.clone(), cur.0, true));
                        cur = *target;
                    }
                    Ok(V::Bool(_)) => {
                        self.asserts.push((b.key.clone(), cur.0, false));
                        return Flow::Abort(kind.name().to_string());
                    }
                    Ok(o) => return Flow::Bug(format!("assert trên {o:?}")),
                    Err(e) => return e,
                },
                TerminatorKind::Call {
                    callee,
                    args,
                    dest,
                    target,
                    ..
                } => {
                    let mut vals = Vec::new();
                    for a in args {
                        match self.operand(a) {
                            Ok(v) => vals.push(v),
                            Err(e) => return e,
                        }
                    }
                    let node = match callee {
                        Callee::Fn { node, .. } => *node,
                        Callee::Value(o) => match self.operand(o) {
                            Ok(V::Fn(n)) => n,
                            Ok(other) => return Flow::Bug(format!("gọi giá trị {other:?}")),
                            Err(e) => return e,
                        },
                        Callee::TraitMethod { .. } => {
                            return Flow::Bug(
                                "gọi qua trait (ngoài phạm vi thông dịch)".to_string(),
                            );
                        }
                    };
                    let Some(cb) = self.mir.bodies.iter().find(|x| x.node == node) else {
                        let k = self.mir.keys.get(&node).cloned().unwrap_or_default();
                        if k.ends_with("core::abort") {
                            return Flow::Abort("abort".to_string());
                        }
                        if k == "$prelude::[T0]::len"
                            && let [V::Ref(fr, l, path)] = vals.as_slice()
                        {
                            // `<[T]>::len(&s)` của prelude (thân prelude không có MIR)
                            let n = match self.get(*fr, *l, path) {
                                Ok(V::Array(x)) => x.len() as i128,
                                Ok(other) => return Flow::Bug(format!("len trên {other:?}")),
                                Err(e) => return e,
                            };
                            if let Err(e) = self.write(dest, V::Int(n, IntTy::Usize)) {
                                return e;
                            }
                            match target {
                                Some(x) => {
                                    cur = *x;
                                    continue;
                                }
                                None => return Flow::Bug("len trả về `!`".to_string()),
                            }
                        }
                        return Flow::Bug(format!("không có thân MIR cho {k}"));
                    };
                    match self.run(cb, vals) {
                        Outcome::Value(v) => {
                            if let Err(e) = self.write(dest, v) {
                                return e;
                            }
                        }
                        Outcome::Abort(k) => return Flow::Abort(k),
                        Outcome::Bug(m) => return Flow::Bug(m),
                        Outcome::OutOfFuel => return Flow::Fuel,
                    }
                    match target {
                        Some(x) => cur = *x,
                        None => return Flow::Bug("hàm `→ !` trả về".to_string()),
                    }
                }
            }
        }
    }

    fn stmt(&mut self, s: &Statement) -> Result<(), Flow> {
        match &s.kind {
            StatementKind::Assign(p, rv) => {
                let v = self.rvalue(rv)?;
                self.write(p, v)
            }
            StatementKind::StorageLive(l) | StatementKind::StorageDead(l) => {
                let t = self.top();
                self.frames[t].locals[l.index()] = V::Uninit;
                Ok(())
            }
            StatementKind::FakeRead(..) | StatementKind::Nop => Ok(()),
        }
    }

    /// Phân giải một chỗ thành (khung, local, đường đi).
    fn resolve(&self, p: &Place) -> Result<(usize, usize, Vec<Step>), Flow> {
        let mut fr = self.top();
        let mut local = p.local.index();
        let mut path: Vec<Step> = Vec::new();
        for e in &p.projection {
            match e {
                PlaceElem::Deref => {
                    let v = self.get(fr, local, &path)?;
                    match v {
                        V::Ref(f2, l2, p2) => {
                            fr = f2;
                            local = l2;
                            path = p2;
                        }
                        other => return Err(Flow::Bug(format!("deref {other:?}"))),
                    }
                }
                PlaceElem::Field(i, _) => path.push(Step::Field(*i as usize)),
                PlaceElem::Downcast(_) => {}
                PlaceElem::Index(l) => {
                    let i = self.local_usize(*l)?;
                    path.push(Step::Index(i));
                }
                PlaceElem::Subslice { from, to } => {
                    let a = self.local_usize(*from)?;
                    let b = match to {
                        Some(t) => self.local_usize(*t)?,
                        None => match self.get(fr, local, &path)? {
                            V::Array(v) => v.len(),
                            other => return Err(Flow::Bug(format!("subslice {other:?}"))),
                        },
                    };
                    path.push(Step::Sub(a, b));
                }
            }
        }
        Ok((fr, local, path))
    }

    fn local_usize(&self, l: Local) -> Result<usize, Flow> {
        match &self.frames[self.top()].locals[l.index()] {
            V::Int(i, _) => usize::try_from(*i).map_err(|_| Flow::Bug("chỉ số âm".to_string())),
            other => Err(Flow::Bug(format!("chỉ số {other:?}"))),
        }
    }

    fn get(&self, fr: usize, local: usize, path: &[Step]) -> Result<V, Flow> {
        let mut v = self.frames[fr].locals[local].clone();
        for st in path {
            v = match (st, v) {
                (Step::Field(i), V::Tuple(x) | V::Adt(_, x)) => x
                    .get(*i)
                    .cloned()
                    .ok_or_else(|| Flow::Bug("trường".to_string()))?,
                (Step::Index(i), V::Array(x)) => x
                    .get(*i)
                    .cloned()
                    .ok_or_else(|| Flow::Bug("chỉ số ngoài biên KHÔNG có assert".to_string()))?,
                (Step::Sub(a, b), V::Array(x)) => V::Array(
                    x.get(*a..*b)
                        .ok_or_else(|| Flow::Bug("lát ngoài biên".to_string()))?
                        .to_vec(),
                ),
                (_, V::Uninit) => {
                    return Err(Flow::Bug(format!("đọc _{local} chưa gán / đã chết")));
                }
                (s, o) => return Err(Flow::Bug(format!("bước {s:?} trên {o:?}"))),
            };
        }
        if v == V::Uninit {
            return Err(Flow::Bug(format!("đọc _{local} chưa gán / đã chết")));
        }
        Ok(v)
    }

    fn write(&mut self, p: &Place, val: V) -> Result<(), Flow> {
        let (fr, local, path) = self.resolve(p)?;
        // Downcast + Field trên enum: ghi vào trường của biến thể; ghi cả biến thể khi không có đường
        fn set(slot: &mut V, path: &[Step], val: V) -> Result<(), String> {
            match path.split_first() {
                None => {
                    *slot = val;
                    Ok(())
                }
                Some((Step::Field(i), rest)) => match slot {
                    V::Tuple(x) | V::Adt(_, x) => set(x.get_mut(*i).ok_or("trường")?, rest, val),
                    _ => Err("ghi trường vào giá trị không phải aggregate".to_string()),
                },
                Some((Step::Index(i), rest)) => match slot {
                    V::Array(x) => set(x.get_mut(*i).ok_or("chỉ số")?, rest, val),
                    _ => Err("ghi chỉ số vào giá trị không phải mảng".to_string()),
                },
                Some((Step::Sub(..), _)) => Err("ghi vào lát".to_string()),
            }
        }
        set(&mut self.frames[fr].locals[local], &path, val).map_err(Flow::Bug)
    }

    fn operand(&self, o: &Operand) -> Result<V, Flow> {
        match o {
            Operand::Copy(p) | Operand::Move(p) => {
                let (fr, l, path) = self.resolve(p)?;
                self.get(fr, l, &path)
            }
            Operand::Const(c) => Ok(match c {
                Const::Int { bits, ty } => V::Int(from_bits(*bits, *ty), *ty),
                Const::Bool(b) => V::Bool(*b),
                Const::Char(c) => V::Char(*c),
                Const::Str(s) => V::Str(s.clone()),
                Const::Unit => V::Unit,
                Const::FnItem { node, .. } => V::Fn(*node),
                Const::Float { .. } | Const::Item { .. } => {
                    return Err(Flow::Bug("hằng ngoài phạm vi thông dịch".to_string()));
                }
            }),
        }
    }

    fn rvalue(&mut self, rv: &Rvalue) -> Result<V, Flow> {
        Ok(match rv {
            Rvalue::Use(o) => self.operand(o)?,
            Rvalue::Repeat(o, n) => V::Array(vec![self.operand(o)?; *n as usize]),
            Rvalue::Ref(_, _, p) => {
                let (fr, l, path) = self.resolve(p)?;
                V::Ref(fr, l, path)
            }
            Rvalue::Len(p) => {
                let (fr, l, path) = self.resolve(p)?;
                match self.get(fr, l, &path)? {
                    V::Array(x) => V::Int(x.len() as i128, IntTy::Usize),
                    other => return Err(Flow::Bug(format!("Len {other:?}"))),
                }
            }
            Rvalue::Discriminant(p) => {
                let (fr, l, path) = self.resolve(p)?;
                match self.get(fr, l, &path)? {
                    V::Adt(v, _) => V::Int(i128::from(v), IntTy::Usize),
                    other => return Err(Flow::Bug(format!("discriminant {other:?}"))),
                }
            }
            Rvalue::Cast(o, t) => cast(self.operand(o)?, t, false)?,
            Rvalue::CastFits(o, t) => {
                let v = self.operand(o)?;
                V::Bool(cast(v, t, true).is_ok())
            }
            Rvalue::UnaryOp(op, o) => match (op, self.operand(o)?) {
                (UnOp::Not, V::Bool(b)) => V::Bool(!b),
                (UnOp::Neg, V::Int(i, t)) => V::Int(wrap(-i, t), t),
                (_, o) => return Err(Flow::Bug(format!("unop trên {o:?}"))),
            },
            Rvalue::BinaryOp(op, a, b) => binop(*op, self.operand(a)?, self.operand(b)?)?,
            Rvalue::CheckedBinaryOp(op, a, b) => match (self.operand(a)?, self.operand(b)?) {
                (V::Int(x, t), V::Int(y, _)) => {
                    let exact = match op {
                        BinOp::Add => x.checked_add(y),
                        BinOp::Sub => x.checked_sub(y),
                        BinOp::Mul => x.checked_mul(y),
                        _ => return Err(Flow::Bug("checked op lạ".to_string())),
                    };
                    match exact {
                        Some(r) if in_range(r, t) => V::Tuple(vec![V::Int(r, t), V::Bool(false)]),
                        Some(r) => V::Tuple(vec![V::Int(wrap(r, t), t), V::Bool(true)]),
                        None => V::Tuple(vec![V::Int(0, t), V::Bool(true)]),
                    }
                }
                (a, b) => return Err(Flow::Bug(format!("checked trên {a:?} {b:?}"))),
            },
            Rvalue::Aggregate(k, ops) => {
                let mut vs = Vec::new();
                for o in ops {
                    vs.push(self.operand(o)?);
                }
                match k {
                    AggregateKind::Tuple => {
                        if vs.is_empty() {
                            V::Unit
                        } else {
                            V::Tuple(vs)
                        }
                    }
                    AggregateKind::Array(_) => V::Array(vs),
                    AggregateKind::Adt(_, var, _) => V::Adt(*var, vs),
                }
            }
        })
    }
}

fn binop(op: BinOp, a: V, b: V) -> Result<V, Flow> {
    Ok(match (a, b) {
        (V::Int(x, t), V::Int(y, _)) => match op {
            BinOp::Add => V::Int(wrap(x + y, t), t),
            BinOp::Sub => V::Int(wrap(x - y, t), t),
            BinOp::Mul => V::Int(wrap(x.wrapping_mul(y), t), t),
            BinOp::Div => {
                if y == 0 {
                    return Err(Flow::Bug("chia 0 KHÔNG có assert".to_string()));
                }
                V::Int(wrap(x / y, t), t)
            }
            BinOp::Rem => {
                if y == 0 {
                    return Err(Flow::Bug("% 0 KHÔNG có assert".to_string()));
                }
                V::Int(wrap(x % y, t), t)
            }
            BinOp::AddWrap => V::Int(wrap(x.wrapping_add(y), t), t),
            BinOp::SubWrap => V::Int(wrap(x.wrapping_sub(y), t), t),
            BinOp::MulWrap => V::Int(wrap(x.wrapping_mul(y), t), t),
            BinOp::AddSat | BinOp::SubSat | BinOp::MulSat => {
                let r = match op {
                    BinOp::AddSat => x.saturating_add(y),
                    BinOp::SubSat => x.saturating_sub(y),
                    _ => x.saturating_mul(y),
                };
                let lo = t.min_val();
                let hi = i128::try_from(t.max_val()).unwrap_or(i128::MAX);
                V::Int(r.clamp(lo, hi), t)
            }
            BinOp::Eq => V::Bool(x == y),
            BinOp::Ne => V::Bool(x != y),
            BinOp::Lt => V::Bool(x < y),
            BinOp::Le => V::Bool(x <= y),
            BinOp::Gt => V::Bool(x > y),
            BinOp::Ge => V::Bool(x >= y),
            BinOp::BitAnd => return Err(Flow::Bug("BitAnd trên số".to_string())),
        },
        (V::Bool(x), V::Bool(y)) => match op {
            BinOp::Eq => V::Bool(x == y),
            BinOp::Ne => V::Bool(x != y),
            BinOp::BitAnd => V::Bool(x && y),
            _ => return Err(Flow::Bug("phép bool lạ".to_string())),
        },
        (a, b) => match op {
            BinOp::Eq => V::Bool(a == b),
            BinOp::Ne => V::Bool(a != b),
            _ => return Err(Flow::Bug(format!("binop {op:?} trên {a:?} {b:?}"))),
        },
    })
}

/// `strict`: chỉ thành công nếu không mất thông tin (CastFits).
fn cast(v: V, t: &Ty, strict: bool) -> Result<V, Flow> {
    match (v, t) {
        (V::Int(x, _), Ty::Int(to)) => {
            if strict && !in_range(x, *to) {
                return Err(Flow::Bug("mất thông tin".to_string()));
            }
            Ok(V::Int(wrap(x, *to), *to))
        }
        (V::Bool(b), Ty::Int(to)) => Ok(V::Int(i128::from(b), *to)),
        (V::Char(c), Ty::Int(to)) => {
            if strict && !in_range(i128::from(c), *to) {
                return Err(Flow::Bug("mất thông tin".to_string()));
            }
            Ok(V::Int(wrap(i128::from(c), *to), *to))
        }
        (V::Int(x, _), Ty::Char) => Ok(V::Char(u32::try_from(x).unwrap_or(0))),
        (v, t) => Err(Flow::Bug(format!("ép {v:?} sang {t:?}"))),
    }
}
