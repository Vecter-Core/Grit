//! Thông dịch MIR cỡ nhỏ — BỘ CHẠY THAM CHIẾU, không thuộc TCB (ADR-020 §9a; ADR-028: backend `interpreter` của `gritc run` ở M9, M10 thay bằng binary LLVM). Phạm vi (§9c): số nguyên, bool, rẽ nhánh, vòng lặp,
//! gọi hàm, struct/tuple/enum, tham chiếu tới chỗ. `Assert` sai ⇒ abort với loại nghĩa vụ (§9d). Đọc chỗ đã `StorageDead`
//! hoặc chưa gán ⇒ lỗi của MIR (`Bug`), tới `Unreachable` ⇒ `Bug`.
use crate::*;
use gt_ast::NodeId;
use gt_types::{IntTy, ParamId, Ty, TypesOutput};

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
    /// môi trường kiểu của instance đang chạy (tham số, đối số) — chỉ cần để phân giải gọi qua trait (ADR-032 §2)
    env: (Vec<ParamId>, Vec<Ty>),
}

pub struct Interp<'a> {
    pub mir: &'a MirOutput,
    frames: Vec<Frame>,
    fuel: u64,
    /// bảng kiểu (impl) — cần để chạy gọi qua trait; không có thì gọi qua trait là `Bug`
    types: Option<&'a TypesOutput>,
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
            types: None,
            asserts: Vec::new(),
        }
    }

    /// Cho thông dịch dùng bảng impl của M4 (MỘT nguồn với codegen: `TypeDb::resolve_trait_method`).
    #[must_use]
    pub fn with_types(mut self, t: &'a TypesOutput) -> Self {
        self.types = Some(t);
        self
    }

    pub fn call_key(&mut self, key: &str, args: Vec<V>) -> Outcome {
        let Some(b) = self.mir.body(key) else {
            return Outcome::Bug(format!("không có thân {key}"));
        };
        self.run(b, args)
    }

    fn run(&mut self, b: &'a Body, args: Vec<V>) -> Outcome {
        self.run_env(b, args, (Vec::new(), Vec::new()))
    }

    fn run_env(&mut self, b: &'a Body, args: Vec<V>, env: (Vec<ParamId>, Vec<Ty>)) -> Outcome {
        if self.frames.len() > 200 {
            return Outcome::Bug("đệ quy quá sâu".to_string());
        }
        let mut locals = vec![V::Uninit; b.locals.len()];
        for (i, a) in args.into_iter().enumerate() {
            locals[i + 1] = a;
        }
        self.frames.push(Frame { locals, env });
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

    /// thế tham số kiểu của khung hiện tại vào `ty`
    fn sub_env(&self, ty: &Ty) -> Ty {
        let Some(fr) = self.frames.last() else {
            return ty.clone();
        };
        let (ps, args) = &fr.env;
        ty.subst(&|p| {
            ps.iter()
                .position(|q| *q == p)
                .and_then(|i| args.get(i).cloned())
        })
    }

    /// môi trường kiểu của callee: tham số của `node` ↦ đối số đã thế theo khung hiện tại
    fn callee_env(&self, node: NodeId, generics: &[Ty]) -> (Vec<ParamId>, Vec<Ty>) {
        match self.types {
            Some(t) => (
                t.db.fn_params(node),
                generics.iter().map(|g| self.sub_env(g)).collect(),
            ),
            None => (Vec::new(), Vec::new()),
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
                    let (node, env) = match callee {
                        Callee::Fn { node, generics } => {
                            let env = self.callee_env(*node, generics);
                            (*node, env)
                        }
                        Callee::Value(o) => match self.operand(o) {
                            Ok(V::Fn(n)) => (n, (Vec::new(), Vec::new())),
                            Ok(other) => return Flow::Bug(format!("gọi giá trị {other:?}")),
                            Err(e) => return e,
                        },
                        Callee::TraitMethod {
                            trait_def,
                            member,
                            self_ty,
                            generics,
                        } => {
                            let Some(t) = self.types else {
                                return Flow::Bug(
                                    "gọi qua trait cần bảng kiểu (Interp::with_types)".to_string(),
                                );
                            };
                            let st = self.sub_env(self_ty);
                            let ga: Vec<Ty> = generics.iter().map(|g| self.sub_env(g)).collect();
                            let Some((n, all)) =
                                t.db.resolve_trait_method(*trait_def, *member, &st, &ga)
                            else {
                                return Flow::Bug(format!(
                                    "E9901: không phân giải được phương thức trait trên `{st:?}`"
                                ));
                            };
                            (n, (t.db.fn_params(n), all))
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
                        match self.lang_ops(&k, &vals) {
                            Ok(Some(v)) => {
                                if let Err(e) = self.write(dest, v) {
                                    return e;
                                }
                                match target {
                                    Some(x) => {
                                        cur = *x;
                                        continue;
                                    }
                                    None => return Flow::Bug("intrinsic trả về `!`".to_string()),
                                }
                            }
                            Ok(None) => {}
                            Err(e) => return e,
                        }
                        if k == "$prelude::spawn"
                            && let [V::Fn(fnode), arg] = vals.as_slice()
                        {
                            // `spawn(f, a)` (công cụ test, NGOÀI TCB; M10 phải cài bằng luồng thật): chạy `f(a)` ngay, tuần tự và tất định
                            let Some(wb) = self.mir.bodies.iter().find(|x| x.node == *fnode) else {
                                return Flow::Bug("spawn: không có thân MIR của hàm".to_string());
                            };
                            match self.run(wb, vec![arg.clone()]) {
                                Outcome::Value(_) => {}
                                Outcome::Abort(k) => return Flow::Abort(k),
                                Outcome::Bug(m) => return Flow::Bug(m),
                                Outcome::OutOfFuel => return Flow::Fuel,
                            }
                            if let Err(e) = self.write(dest, V::Unit) {
                                return e;
                            }
                            match target {
                                Some(x) => {
                                    cur = *x;
                                    continue;
                                }
                                None => return Flow::Bug("spawn trả về `!`".to_string()),
                            }
                        }
                        return Flow::Bug(format!("không có thân MIR cho {k}"));
                    };
                    match self.run_env(cb, vals, env) {
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
        self.set_at(fr, local, &path, val)
    }

    /// Ghi `val` vào (khung, local, đường đi).
    fn set_at(&mut self, fr: usize, local: usize, path: &[Step], val: V) -> Result<(), Flow> {
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
        set(&mut self.frames[fr].locals[local], path, val).map_err(Flow::Bug)
    }

    /// Lang item đồng thời/mutability bên trong (`Cell`, `Mutex`, `Atomic`): CÔNG CỤ TEST, NGOÀI TCB, thực thi TUẦN TỰ. Ngữ nghĩa này KHÔNG
    /// phải bằng chứng cho ngữ nghĩa codegen (M10 phải dùng atomics/khoá LLVM thật; ADR-027 §1). `Ok(None)`: không phải intrinsic loại này.
    /// `Vec<'a, T>` (ADR-032 phụ lục A) trên thông dịch: giá trị `Adt(0, [arena, Array(phần tử), len, cap])`; `Array` luôn có đúng `len`
    /// phần tử. Không mô phỏng hết Arena/tràn kích thước (thông dịch không có bố cục): các abort đó chỉ kiểm trên exe.
    fn vec_op(&mut self, m: &str, vals: &[V]) -> Result<V, Flow> {
        let bug = |t: &str| Flow::Bug(format!("Vec::{m}: {t}"));
        match (m, vals) {
            ("new", [arena]) => Ok(V::Adt(
                0,
                vec![
                    arena.clone(),
                    V::Array(Vec::new()),
                    V::Int(0, IntTy::Usize),
                    V::Int(0, IntTy::Usize),
                ],
            )),
            ("len", [V::Ref(fr, l, path)]) => {
                let mut p = path.clone();
                p.push(Step::Field(2));
                self.get(*fr, *l, &p)
            }
            ("as_slice" | "as_mut_slice", [V::Ref(fr, l, path)]) => {
                let mut p = path.clone();
                p.push(Step::Field(1));
                Ok(V::Ref(*fr, *l, p))
            }
            ("reserve", [V::Ref(fr, l, path), V::Int(n, _)]) => {
                // chỉ ghi nhận dung lượng (thông dịch không có bố cục/giới hạn bộ nhớ)
                let mut cp = path.clone();
                cp.push(Step::Field(3));
                let V::Int(cap, t) = self.get(*fr, *l, &cp)? else {
                    return Err(bug("cap không phải số nguyên"));
                };
                if *n > cap {
                    self.set_at(*fr, *l, &cp, V::Int(*n, t))?;
                }
                Ok(V::Unit)
            }
            ("push", [V::Ref(fr, l, path), x]) => {
                let mut items = path.clone();
                items.push(Step::Field(1));
                let V::Array(mut a) = self.get(*fr, *l, &items)? else {
                    return Err(bug("trường phần tử không phải mảng"));
                };
                a.push(x.clone());
                let n = a.len() as i128;
                self.set_at(*fr, *l, &items, V::Array(a))?;
                let mut lp = path.clone();
                lp.push(Step::Field(2));
                self.set_at(*fr, *l, &lp, V::Int(n, IntTy::Usize))?;
                let mut cp = path.clone();
                cp.push(Step::Field(3));
                let V::Int(cap, t) = self.get(*fr, *l, &cp)? else {
                    return Err(bug("cap không phải số nguyên"));
                };
                if n > cap {
                    self.set_at(*fr, *l, &cp, V::Int((cap * 2).max(4), t))?;
                }
                Ok(V::Unit)
            }
            _ => Err(bug("đối số không đúng dạng")),
        }
    }

    fn lang_ops(&mut self, key: &str, vals: &[V]) -> Result<Option<V>, Flow> {
        // phép bit u32 của prelude (Q9): thân không có MIR; M7 đã chèn Assert `n < 32` ở chỗ gọi nên n ≥ 32 không tới được đây
        if let Some(name) = key.strip_prefix("$prelude::")
            && matches!(
                name,
                "band" | "bor" | "bxor" | "bnot" | "shl" | "shr" | "rotr" | "rotl"
            )
        {
            let get = |i: usize| match vals.get(i) {
                Some(V::Int(x, t)) => Ok((*x as u64 & 0xFFFF_FFFF, *t)),
                _ => Err(Flow::Bug(format!("{name}: đối số không phải số nguyên"))),
            };
            let (a, t) = get(0)?;
            let m = 0xFFFF_FFFFu64;
            let r = match name {
                "bnot" => !a & m,
                "band" | "bor" | "bxor" => {
                    let (b, _) = get(1)?;
                    match name {
                        "band" => a & b,
                        "bor" => a | b,
                        _ => a ^ b,
                    }
                }
                _ => {
                    let (n, _) = get(1)?;
                    if n >= 32 {
                        return Err(Flow::Bug(format!("{name}: n ≥ 32 không có Assert")));
                    }
                    match name {
                        "shl" => (a << n) & m,
                        "shr" => a >> n,
                        "rotr" => ((a >> n) | (a << (32 - n))) & m,
                        _ => ((a << n) | (a >> (32 - n))) & m,
                    }
                }
            };
            return Ok(Some(V::Int(i128::from(r), t)));
        }
        let is = |name: &str| key.contains(name);
        let m = key.rsplit("::").next().unwrap_or("");
        if key.starts_with("$prelude::Vec<") {
            return self.vec_op(m, vals).map(Some);
        }
        if key == "$prelude::Arena::new" || key == "$prelude::Arena::reset" {
            // Arena của thông dịch: tay cầm 0, không giới hạn bộ nhớ; `reset` không làm gì (M6 đã bảo đảm không còn tham chiếu vào nội dung)
            return Ok(Some(if m == "new" {
                V::Adt(0, vec![V::Int(0, IntTy::I32)])
            } else {
                V::Unit
            }));
        }
        if !(is("Cell") || is("Mutex") || is("Atomic")) {
            return Ok(None);
        }
        if m == "new" {
            return match vals {
                [v] => Ok(Some(V::Adt(0, vec![v.clone()]))),
                _ => Err(Flow::Bug("new: sai số đối số".to_string())),
            };
        }
        let Some(V::Ref(fr, l, path)) = vals.first() else {
            return Err(Flow::Bug(format!(
                "{key}: người nhận không phải tham chiếu"
            )));
        };
        let mut field = path.clone();
        field.push(Step::Field(0));
        let cur = self.get(*fr, *l, &field)?;
        let int_op = |old: &V, d: &V, sub: bool| -> Result<V, Flow> {
            match (old, d) {
                (V::Int(a, t), V::Int(b, _)) => {
                    Ok(V::Int(wrap(if sub { a - b } else { a + b }, *t), *t))
                }
                _ => Err(Flow::Bug(
                    "fetch_add/sub trên giá trị không nguyên".to_string(),
                )),
            }
        };
        match m {
            "get" | "load" => Ok(Some(cur)),
            "set" | "store" => {
                let v = vals
                    .get(1)
                    .cloned()
                    .ok_or(Flow::Bug("thiếu giá trị".to_string()))?;
                self.set_at(*fr, *l, &field, v)?;
                Ok(Some(V::Unit))
            }
            "swap" => {
                let v = vals
                    .get(1)
                    .cloned()
                    .ok_or(Flow::Bug("thiếu giá trị".to_string()))?;
                self.set_at(*fr, *l, &field, v)?;
                Ok(Some(cur))
            }
            "fetch_add" | "fetch_sub" => {
                let d = vals.get(1).ok_or(Flow::Bug("thiếu giá trị".to_string()))?;
                let nv = int_op(&cur, d, m == "fetch_sub")?;
                self.set_at(*fr, *l, &field, nv)?;
                Ok(Some(cur))
            }
            "compare_exchange" => {
                let (Some(want), Some(newv)) = (vals.get(1), vals.get(2)) else {
                    return Err(Flow::Bug("compare_exchange thiếu đối số".to_string()));
                };
                if &cur == want {
                    self.set_at(*fr, *l, &field, newv.clone())?;
                    Ok(Some(V::Adt(0, vec![cur])))
                } else {
                    Ok(Some(V::Adt(1, vec![cur])))
                }
            }
            "with" => {
                // `Mutex::with(&self, f, a)`: chạy `f(&!inner, a)` tại chỗ (khoá giữ trong lời gọi)
                let (Some(V::Fn(fnode)), Some(arg)) = (vals.get(1), vals.get(2)) else {
                    return Err(Flow::Bug("with: thiếu hàm hoặc đối số".to_string()));
                };
                let Some(wb) = self.mir.bodies.iter().find(|x| x.node == *fnode) else {
                    return Err(Flow::Bug("with: không có thân MIR của hàm".to_string()));
                };
                let inner = V::Ref(*fr, *l, field);
                match self.run(wb, vec![inner, arg.clone()]) {
                    Outcome::Value(v) => Ok(Some(v)),
                    Outcome::Abort(k) => Err(Flow::Abort(k)),
                    Outcome::Bug(b) => Err(Flow::Bug(b)),
                    Outcome::OutOfFuel => Err(Flow::Fuel),
                }
            }
            _ => Ok(None),
        }
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
