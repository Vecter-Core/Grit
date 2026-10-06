//! Bộ sinh chương trình Grit ngẫu nhiên + BỘ ĐÁNH GIÁ AST RIÊNG (ADR-020 §9b): kết quả kỳ vọng tính trên AST nhỏ của chính
//! bộ sinh, độc lập với bước hạ MIR. Phạm vi (§9c): i32, bool, `⎇`, `⌥` (số nguyên, Option), `⟳ in` có `⊠/⊡/⏎` xuyên phạm vi,
//! gọi hàm (không đệ quy), tuple, struct (thứ tự viết ≠ thứ tự khai báo), Option. Abort: tràn, chia/% 0, -MIN, ép mất thông tin.
#![allow(dead_code)]

pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
    pub fn chance(&mut self, pct: u64) -> bool {
        self.below(100) < pct
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    AddWrap,
}

#[derive(Clone, Copy, Debug)]
pub enum Cmp {
    Lt,
    Le,
    Eq,
    Ne,
}

#[derive(Clone, Debug)]
pub enum E {
    Lit(i64),
    Var(String),
    Bin(Op, Box<E>, Box<E>),
    Neg(Box<E>),
    If(Box<C>, Box<E>, Box<E>),
    Call(usize, Vec<E>),
    /// `(a, b).k`
    TupleGet(Box<E>, Box<E>, u8),
    /// `P { b: e2, a: e1 }.a/b` — viết `b` trước để kiểm thứ tự đánh giá theo nguồn
    StructGet(Box<E>, Box<E>, bool),
    /// `⌥ e { k ↦ x, …, _ ↦ d }`
    MatchInt(Box<E>, Vec<(i64, E)>, Box<E>),
    /// `⌥ (⎇ c { Some(e) } ⇝ { None }) { Some(v) ↦ s, None ↦ n }`
    MatchOpt(Box<C>, Box<E>, String, Box<E>, Box<E>),
    /// `((e as u8) as i32)`
    CastU8(Box<E>),
    /// `{ v ← e1; e2 }`: biểu thức có tác dụng phụ lên biến `v` (thứ tự đánh giá của toán hạng là CHỖ đứng trước nó).
    Eff(String, Box<E>, Box<E>),
}

#[derive(Clone, Debug)]
pub enum C {
    Lit(bool),
    Cmp(Cmp, Box<E>, Box<E>),
    And(Box<C>, Box<C>),
    Or(Box<C>, Box<C>),
    Not(Box<C>),
}

#[derive(Clone, Debug)]
pub enum S {
    Let(String, E),
    Assign(String, E),
    If(C, Vec<S>, Vec<S>),
    For(String, u8, Vec<S>),
    Ret(E),
    Break,
    Continue,
}

#[derive(Clone, Debug)]
pub struct F {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<S>,
    pub tail: E,
}

pub struct Gen {
    pub rng: Rng,
    counter: usize,
    pub fns: Vec<F>,
    /// biến gán được (`μ`) trong phạm vi đang sinh — cho `E::Eff`
    cur_muts: Vec<String>,
    /// số biểu thức "toán hạng là chỗ + toán hạng sau ghi đúng chỗ đó" đã sinh (độ phủ)
    pub place_then_effect: usize,
}

const I32_MIN: i64 = i32::MIN as i64;
const I32_MAX: i64 = i32::MAX as i64;

impl Gen {
    pub fn new(seed: u64) -> Gen {
        Gen {
            rng: Rng(seed | 1),
            counter: 0,
            fns: Vec::new(),
            cur_muts: Vec::new(),
            place_then_effect: 0,
        }
    }

    fn fresh(&mut self, p: &str) -> String {
        self.counter += 1;
        format!("{p}{}", self.counter)
    }

    fn lit(&mut self) -> i64 {
        match self.rng.below(10) {
            0 => I32_MAX - self.rng.below(3) as i64,
            1 => I32_MIN + self.rng.below(3) as i64,
            2 => 0,
            3 => -1,
            4 => 1 << (20 + self.rng.below(10)),
            _ => self.rng.below(200) as i64 - 100,
        }
    }

    fn expr(&mut self, vars: &[String], depth: u32, fidx: usize) -> E {
        if depth == 0 || self.rng.chance(25) {
            return if !vars.is_empty() && self.rng.chance(60) {
                E::Var(vars[self.rng.below(vars.len() as u64) as usize].clone())
            } else {
                E::Lit(self.lit())
            };
        }
        let d = depth - 1;
        if !self.cur_muts.is_empty() && self.rng.chance(12) {
            return self.place_then_effect_expr(vars, d, fidx);
        }
        match self.rng.below(12) {
            0..=3 => {
                let op = [Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Rem, Op::AddWrap]
                    [self.rng.below(6) as usize];
                E::Bin(
                    op,
                    Box::new(self.expr(vars, d, fidx)),
                    Box::new(self.expr(vars, d, fidx)),
                )
            }
            4 => E::Neg(Box::new(self.expr(vars, d, fidx))),
            5 => E::If(
                Box::new(self.cond(vars, d, fidx)),
                Box::new(self.expr(vars, d, fidx)),
                Box::new(self.expr(vars, d, fidx)),
            ),
            6 if fidx > 0 => {
                let j = self.rng.below(fidx as u64) as usize;
                let n = self.fns[j].params.len();
                E::Call(j, (0..n).map(|_| self.expr(vars, d, fidx)).collect())
            }
            7 => E::TupleGet(
                Box::new(self.expr(vars, d, fidx)),
                Box::new(self.expr(vars, d, fidx)),
                self.rng.below(2) as u8,
            ),
            8 => E::StructGet(
                Box::new(self.expr(vars, d, fidx)),
                Box::new(self.expr(vars, d, fidx)),
                self.rng.chance(50),
            ),
            9 => {
                let n = 1 + self.rng.below(3);
                let mut arms = Vec::new();
                let mut used = Vec::new();
                for _ in 0..n {
                    let k = self.rng.below(5) as i64 - 2;
                    if used.contains(&k) {
                        continue;
                    }
                    used.push(k);
                    arms.push((k, self.expr(vars, d, fidx)));
                }
                E::MatchInt(
                    Box::new(self.expr(vars, d, fidx)),
                    arms,
                    Box::new(self.expr(vars, d, fidx)),
                )
            }
            10 => {
                let v = self.fresh("o");
                let c = self.cond(vars, d, fidx);
                let inner = self.expr(vars, d, fidx);
                let mut vs = vars.to_vec();
                vs.push(v.clone());
                let some = self.expr(&vs, d, fidx);
                let none = self.expr(vars, d, fidx);
                E::MatchOpt(
                    Box::new(c),
                    Box::new(inner),
                    v,
                    Box::new(some),
                    Box::new(none),
                )
            }
            11 => E::CastU8(Box::new(self.expr(vars, d, fidx))),
            _ => E::Lit(self.lit()),
        }
    }

    /// Toán hạng đầu là CHỖ `x`, toán hạng sau là `{ x ← e1; e2 }`: đánh giá trái sang phải đọc `x` TRƯỚC khi bị ghi.
    fn place_then_effect_expr(&mut self, vars: &[String], d: u32, fidx: usize) -> E {
        let x = self.cur_muts[self.rng.below(self.cur_muts.len() as u64) as usize].clone();
        let e1 = self.expr(vars, d.min(1), fidx);
        let e2 = self.expr(vars, d.min(1), fidx);
        let eff = E::Eff(x.clone(), Box::new(e1), Box::new(e2));
        let place = E::Var(x);
        self.place_then_effect += 1;
        match self.rng.below(5) {
            0 => E::Bin(Op::AddWrap, Box::new(place), Box::new(eff)),
            1 => E::Bin(Op::Sub, Box::new(place), Box::new(eff)),
            2 => E::TupleGet(Box::new(place), Box::new(eff), self.rng.below(2) as u8),
            // thứ tự viết: `b` trước ⇒ b = place, a = eff
            3 => E::StructGet(Box::new(eff), Box::new(place), self.rng.chance(50)),
            _ => {
                let two: Vec<usize> = (0..fidx)
                    .filter(|j| self.fns[*j].params.len() >= 2)
                    .collect();
                if let Some(&j) = two.get(self.rng.below(two.len().max(1) as u64) as usize) {
                    let n = self.fns[j].params.len();
                    let mut args = vec![place, eff];
                    while args.len() < n {
                        args.push(E::Lit(self.lit()));
                    }
                    E::Call(j, args)
                } else {
                    E::Bin(Op::AddWrap, Box::new(place), Box::new(eff))
                }
            }
        }
    }

    fn cond(&mut self, vars: &[String], depth: u32, fidx: usize) -> C {
        let d = depth.saturating_sub(1);
        match self.rng.below(7) {
            0 => C::Lit(self.rng.chance(50)),
            1 if depth > 0 => C::And(
                Box::new(self.cond(vars, d, fidx)),
                Box::new(self.cond(vars, d, fidx)),
            ),
            2 if depth > 0 => C::Or(
                Box::new(self.cond(vars, d, fidx)),
                Box::new(self.cond(vars, d, fidx)),
            ),
            3 if depth > 0 => C::Not(Box::new(self.cond(vars, d, fidx))),
            _ => {
                let c = [Cmp::Lt, Cmp::Le, Cmp::Eq, Cmp::Ne][self.rng.below(4) as usize];
                C::Cmp(
                    c,
                    Box::new(self.expr(vars, d, fidx)),
                    Box::new(self.expr(vars, d, fidx)),
                )
            }
        }
    }

    fn stmts(
        &mut self,
        vars: &mut Vec<String>,
        muts: &mut Vec<String>,
        depth: u32,
        in_loop: bool,
        fidx: usize,
    ) -> Vec<S> {
        let n = self.rng.below(4);
        let mut out = Vec::new();
        for _ in 0..n {
            self.cur_muts = muts.clone();
            let s = match self.rng.below(8) {
                0 | 1 => {
                    let v = self.fresh("v");
                    let e = self.expr(vars, 2, fidx);
                    vars.push(v.clone());
                    muts.push(v.clone());
                    S::Let(v, e)
                }
                2 if !muts.is_empty() => {
                    let v = muts[self.rng.below(muts.len() as u64) as usize].clone();
                    S::Assign(v, self.expr(vars, 2, fidx))
                }
                3 if depth > 0 => {
                    let c = self.cond(vars, 1, fidx);
                    let (mut v1, mut m1) = (vars.clone(), muts.clone());
                    let a = self.stmts(&mut v1, &mut m1, depth - 1, in_loop, fidx);
                    let (mut v2, mut m2) = (vars.clone(), muts.clone());
                    let b = self.stmts(&mut v2, &mut m2, depth - 1, in_loop, fidx);
                    S::If(c, a, b)
                }
                4 if depth > 0 => {
                    let i = self.fresh("i");
                    let k = self.rng.below(4) as u8;
                    let (mut v1, mut m1) = (vars.clone(), muts.clone());
                    v1.push(i.clone());
                    let body = self.stmts(&mut v1, &mut m1, depth - 1, true, fidx);
                    S::For(i, k, body)
                }
                5 if in_loop => {
                    let c = self.cond(vars, 1, fidx);
                    let jump = if self.rng.chance(50) {
                        S::Break
                    } else {
                        S::Continue
                    };
                    S::If(c, vec![jump], vec![])
                }
                6 if self.rng.chance(30) => {
                    let c = self.cond(vars, 1, fidx);
                    S::If(c, vec![S::Ret(self.expr(vars, 2, fidx))], vec![])
                }
                _ => continue,
            };
            out.push(s);
        }
        out
    }

    /// Sinh một chương trình: vài hàm phụ (gọi các hàm trước nó), rồi `entry()`.
    pub fn program(&mut self) -> Vec<F> {
        self.fns.clear();
        let nf = 1 + self.rng.below(3) as usize;
        for k in 0..=nf {
            let np = if k == nf {
                0
            } else {
                self.rng.below(3) as usize
            };
            let params: Vec<String> = (0..np).map(|_| self.fresh("p")).collect();
            let mut vars = params.clone();
            let mut muts = Vec::new();
            let body = self.stmts(&mut vars, &mut muts, 2, false, k);
            self.cur_muts = muts.clone();
            let tail = self.expr(&vars, 3, k);
            let name = if k == nf {
                "entry".to_string()
            } else {
                format!("f{k}")
            };
            self.fns.push(F {
                name,
                params,
                body,
                tail,
            });
        }
        self.fns.clone()
    }
}

// ======================= in ra Grit =======================

fn pe(e: &E, fns: &[F]) -> String {
    match e {
        E::Lit(v) => {
            if *v < 0 {
                format!("({v})")
            } else {
                v.to_string()
            }
        }
        E::Var(v) => v.clone(),
        E::Bin(op, a, b) => {
            let o = match op {
                Op::Add => "+",
                Op::Sub => "-",
                Op::Mul => "*",
                Op::Div => "/",
                Op::Rem => "%",
                Op::AddWrap => "+%",
            };
            format!("({} {o} {})", pe(a, fns), pe(b, fns))
        }
        E::Neg(a) => format!("(-{})", pe(a, fns)),
        E::If(c, a, b) => format!(
            "(⎇ {} {{ {} }} ⇝ {{ {} }})",
            pc(c, fns),
            pe(a, fns),
            pe(b, fns)
        ),
        E::Call(j, args) => format!(
            "{}({})",
            fns[*j].name,
            args.iter()
                .map(|x| pe(x, fns))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        E::TupleGet(a, b, k) => format!("({}, {}).{k}", pe(a, fns), pe(b, fns)),
        E::StructGet(a, b, take_a) => {
            format!(
                "(P {{ b: {}, a: {} }}).{}",
                pe(b, fns),
                pe(a, fns),
                if *take_a { "a" } else { "b" }
            )
        }
        E::MatchInt(s, arms, d) => {
            let mut parts: Vec<String> = arms
                .iter()
                .map(|(k, x)| {
                    let kk = if *k < 0 {
                        format!("{k}")
                    } else {
                        k.to_string()
                    };
                    format!("{kk} ↦ {}", pe(x, fns))
                })
                .collect();
            parts.push(format!("_ ↦ {}", pe(d, fns)));
            format!("(⌥ {} {{ {} }})", pe(s, fns), parts.join(", "))
        }
        E::MatchOpt(c, inner, v, some, none) => format!(
            "(⌥ (⎇ {} {{ Some({}) }} ⇝ {{ None }}) {{ Some({v}) ↦ {}, None ↦ {} }})",
            pc(c, fns),
            pe(inner, fns),
            pe(some, fns),
            pe(none, fns)
        ),
        E::CastU8(a) => format!("(({} as u8) as i32)", pe(a, fns)),
        E::Eff(v, e1, e2) => format!("{{ {v} ← {}; {} }}", pe(e1, fns), pe(e2, fns)),
    }
}

fn pc(c: &C, fns: &[F]) -> String {
    match c {
        C::Lit(b) => b.to_string(),
        C::Cmp(op, a, b) => {
            let o = match op {
                Cmp::Lt => "<",
                Cmp::Le => "<=",
                Cmp::Eq => "==",
                Cmp::Ne => "≠",
            };
            format!("({} {o} {})", pe(a, fns), pe(b, fns))
        }
        C::And(a, b) => format!("({} ∧ {})", pc(a, fns), pc(b, fns)),
        C::Or(a, b) => format!("({} ∨ {})", pc(a, fns), pc(b, fns)),
        C::Not(a) => format!("(¬{})", pc(a, fns)),
    }
}

fn ps(s: &S, fns: &[F], ind: usize, out: &mut String) {
    let pad = "  ".repeat(ind);
    match s {
        S::Let(v, e) => out.push_str(&format!("{pad}μ{v} ≔ {};\n", pe(e, fns))),
        S::Assign(v, e) => out.push_str(&format!("{pad}{v} ← {};\n", pe(e, fns))),
        S::If(c, a, b) => {
            out.push_str(&format!("{pad}⎇ {} {{\n", pc(c, fns)));
            for x in a {
                ps(x, fns, ind + 1, out);
            }
            if b.is_empty() {
                out.push_str(&format!("{pad}}}\n"));
            } else {
                out.push_str(&format!("{pad}}} ⇝ {{\n"));
                for x in b {
                    ps(x, fns, ind + 1, out);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
        }
        S::For(i, k, body) => {
            out.push_str(&format!("{pad}⟳ {i} in 0..{k} {{\n"));
            for x in body {
                ps(x, fns, ind + 1, out);
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        S::Ret(e) => out.push_str(&format!("{pad}⏎ {};\n", pe(e, fns))),
        S::Break => out.push_str(&format!("{pad}⊠;\n")),
        S::Continue => out.push_str(&format!("{pad}⊡;\n")),
    }
}

pub fn print(fns: &[F]) -> String {
    let mut out = String::from("▢ P { a: i32, b: i32 }\n");
    for f in fns {
        let ps_: Vec<String> = f.params.iter().map(|p| format!("{p}: i32")).collect();
        out.push_str(&format!("ƒ {}({}) → i32 ! ∅ {{\n", f.name, ps_.join(", ")));
        for s in &f.body {
            ps(s, fns, 1, &mut out);
        }
        out.push_str(&format!("  {}\n}}\n", pe(&f.tail, fns)));
    }
    out
}

// ======================= bộ đánh giá AST riêng =======================

#[derive(Clone, Debug, PartialEq)]
pub enum Res {
    Value(i64),
    Abort(&'static str),
}

enum Flow {
    Normal,
    Ret(i64),
    Break,
    Continue,
    Abort(&'static str),
}

type Env = Vec<(String, i64)>;

fn lookup(env: &Env, v: &str) -> i64 {
    env.iter()
        .rev()
        .find(|(n, _)| n == v)
        .map_or(0, |(_, x)| *x)
}

fn ev(e: &E, env: &mut Env, fns: &[F]) -> Result<i64, &'static str> {
    Ok(match e {
        E::Lit(v) => *v,
        E::Var(v) => lookup(env, v),
        E::Bin(op, a, b) => {
            let x = ev(a, env, fns)?;
            let y = ev(b, env, fns)?;
            let r = match op {
                Op::Add => x + y,
                Op::Sub => x - y,
                Op::Mul => x * y,
                Op::AddWrap => return Ok(i64::from((x as i32).wrapping_add(y as i32))),
                Op::Div | Op::Rem => {
                    if y == 0 {
                        return Err(if matches!(op, Op::Div) {
                            "div_by_zero"
                        } else {
                            "rem_by_zero"
                        });
                    }
                    if x == I32_MIN && y == -1 {
                        return Err("overflow");
                    }
                    if matches!(op, Op::Div) { x / y } else { x % y }
                }
            };
            if !(I32_MIN..=I32_MAX).contains(&r) {
                return Err("overflow");
            }
            r
        }
        E::Neg(a) => {
            let x = ev(a, env, fns)?;
            if x == I32_MIN {
                return Err("overflow_neg");
            }
            -x
        }
        E::If(c, a, b) => {
            if evc(c, env, fns)? {
                ev(a, env, fns)?
            } else {
                ev(b, env, fns)?
            }
        }
        E::Call(j, args) => {
            let mut vals = Vec::new();
            for a in args {
                vals.push(ev(a, env, fns)?);
            }
            match call(*j, vals, fns) {
                Res::Value(v) => v,
                Res::Abort(k) => return Err(k),
            }
        }
        E::TupleGet(a, b, k) => {
            let x = ev(a, env, fns)?;
            let y = ev(b, env, fns)?;
            if *k == 0 { x } else { y }
        }
        E::StructGet(a, b, take_a) => {
            // thứ tự VIẾT: b trước, a sau
            let y = ev(b, env, fns)?;
            let x = ev(a, env, fns)?;
            if *take_a { x } else { y }
        }
        E::MatchInt(s, arms, d) => {
            let v = ev(s, env, fns)?;
            match arms.iter().find(|(k, _)| *k == v) {
                Some((_, x)) => ev(x, env, fns)?,
                None => ev(d, env, fns)?,
            }
        }
        E::MatchOpt(c, inner, v, some, none) => {
            let opt = if evc(c, env, fns)? {
                Some(ev(inner, env, fns)?)
            } else {
                None
            };
            match opt {
                Some(x) => {
                    env.push((v.clone(), x));
                    let r = ev(some, env, fns);
                    env.pop();
                    r?
                }
                None => ev(none, env, fns)?,
            }
        }
        E::CastU8(a) => {
            let x = ev(a, env, fns)?;
            if !(0..=255).contains(&x) {
                return Err("cast_lossy");
            }
            x
        }
        E::Eff(v, e1, e2) => {
            let x = ev(e1, env, fns)?;
            if let Some(slot) = env.iter_mut().rev().find(|(n, _)| n == v) {
                slot.1 = x;
            }
            ev(e2, env, fns)?
        }
    })
}

fn evc(c: &C, env: &mut Env, fns: &[F]) -> Result<bool, &'static str> {
    Ok(match c {
        C::Lit(b) => *b,
        C::Cmp(op, a, b) => {
            let x = ev(a, env, fns)?;
            let y = ev(b, env, fns)?;
            match op {
                Cmp::Lt => x < y,
                Cmp::Le => x <= y,
                Cmp::Eq => x == y,
                Cmp::Ne => x != y,
            }
        }
        C::And(a, b) => evc(a, env, fns)? && evc(b, env, fns)?,
        C::Or(a, b) => evc(a, env, fns)? || evc(b, env, fns)?,
        C::Not(a) => !evc(a, env, fns)?,
    })
}

fn exec(ss: &[S], env: &mut Env, fns: &[F]) -> Flow {
    exec_in(ss, env, fns, true)
}

/// `scoped = false`: thân hàm — giữ biến để tính biểu thức đuôi.
fn exec_in(ss: &[S], env: &mut Env, fns: &[F], scoped: bool) -> Flow {
    let mark = env.len();
    for s in ss {
        let f = match s {
            S::Let(v, e) => match ev(e, env, fns) {
                Ok(x) => {
                    env.push((v.clone(), x));
                    Flow::Normal
                }
                Err(k) => Flow::Abort(k),
            },
            S::Assign(v, e) => match ev(e, env, fns) {
                Ok(x) => {
                    if let Some(slot) = env.iter_mut().rev().find(|(n, _)| n == v) {
                        slot.1 = x;
                    }
                    Flow::Normal
                }
                Err(k) => Flow::Abort(k),
            },
            S::If(c, a, b) => match evc(c, env, fns) {
                Ok(true) => exec(a, env, fns),
                Ok(false) => exec(b, env, fns),
                Err(k) => Flow::Abort(k),
            },
            S::For(i, k, body) => {
                let mut out = Flow::Normal;
                for x in 0..i64::from(*k) {
                    env.push((i.clone(), x));
                    let f = exec(body, env, fns);
                    env.pop();
                    match f {
                        Flow::Normal | Flow::Continue => {}
                        Flow::Break => break,
                        other => {
                            out = other;
                            break;
                        }
                    }
                }
                out
            }
            S::Ret(e) => match ev(e, env, fns) {
                Ok(x) => Flow::Ret(x),
                Err(k) => Flow::Abort(k),
            },
            S::Break => Flow::Break,
            S::Continue => Flow::Continue,
        };
        if !matches!(f, Flow::Normal) {
            env.truncate(mark);
            return f;
        }
    }
    if scoped {
        env.truncate(mark);
    }
    Flow::Normal
}

pub fn call(j: usize, args: Vec<i64>, fns: &[F]) -> Res {
    let f = &fns[j];
    let mut env: Env = f.params.iter().cloned().zip(args).collect();
    match exec_in(&f.body, &mut env, fns, false) {
        Flow::Ret(v) => Res::Value(v),
        Flow::Abort(k) => Res::Abort(k),
        Flow::Normal | Flow::Break | Flow::Continue => match ev(&f.tail, &mut env, fns) {
            Ok(v) => Res::Value(v),
            Err(k) => Res::Abort(k),
        },
    }
}
