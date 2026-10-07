//! Bộ sinh chương trình số nguyên cho ORACLE mức chương trình của M7 (ADR-025 §8; luật bộ sinh grit-contract §3.3).
//! Công cụ test, không thuộc TCB. Mỗi chương trình dùng MỘT kiểu số nguyên `T` (9 kiểu: độ rộng 8/16/32/64, có/không dấu, `usize`),
//! gồm: phép toán có nghĩa vụ (`+ - * / %`, `-x`, `as`), rẽ nhánh với điều kiện đóng vai bảo vệ, vòng lặp đếm có `⋈`, mảng/lát
//! và chỉ số, hợp đồng `⊳ ⊲` trên hàm phụ, lời gọi giữa các hàm. Đầu vào của `entry` là các số nguyên `T` (biên + ngẫu nhiên).
#![allow(dead_code)]
use super::progen::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ity {
    pub name: &'static str,
    pub bits: u32,
    pub signed: bool,
}

pub const TYPES: [Ity; 9] = [
    Ity {
        name: "i8",
        bits: 8,
        signed: true,
    },
    Ity {
        name: "u8",
        bits: 8,
        signed: false,
    },
    Ity {
        name: "i16",
        bits: 16,
        signed: true,
    },
    Ity {
        name: "u16",
        bits: 16,
        signed: false,
    },
    Ity {
        name: "i32",
        bits: 32,
        signed: true,
    },
    Ity {
        name: "u32",
        bits: 32,
        signed: false,
    },
    Ity {
        name: "i64",
        bits: 64,
        signed: true,
    },
    Ity {
        name: "u64",
        bits: 64,
        signed: false,
    },
    Ity {
        name: "usize",
        bits: 64,
        signed: false,
    },
];

impl Ity {
    pub fn min(self) -> i128 {
        if self.signed {
            -(1i128 << (self.bits - 1))
        } else {
            0
        }
    }
    pub fn max(self) -> i128 {
        if self.signed {
            (1i128 << (self.bits - 1)) - 1
        } else {
            (1i128 << self.bits) - 1
        }
    }
    /// literal có hậu tố
    pub fn lit(self, v: i128) -> String {
        format!("{v}{}", self.name)
    }
    /// giá trị biên + ngẫu nhiên để làm đầu vào / hằng
    pub fn boundary(self) -> Vec<i128> {
        let (lo, hi) = (self.min(), self.max());
        let mut v = vec![lo, lo + 1, hi - 1, hi, 0, 1, 2, 3, 5, 7, 10, 100];
        if self.signed {
            v.extend([-1, -2, -3, -10]);
        }
        v.retain(|x| *x >= lo && *x <= hi);
        v.sort_unstable();
        v.dedup();
        v
    }
}

/// Độ phủ theo cấu trúc sinh (không phải kết quả phân tích).
#[derive(Default, Debug, Clone)]
pub struct GenStats {
    pub programs: usize,
    pub loops: usize,
    pub invariants: usize,
    pub arrays: usize,
    pub slice_calls: usize,
    pub pre: usize,
    pub post: usize,
    pub casts: usize,
    pub divs: usize,
    pub negs: usize,
}

pub struct Prog {
    pub src: String,
    pub ty: Ity,
    /// số tham số của `main::entry`
    pub entry_params: usize,
}

struct Fx {
    name: String,
    params: usize,
    has_slice: bool,
}

pub struct OGen {
    pub rng: Rng,
    ty: Ity,
    /// chế độ "an toàn": phần lớn phép toán dùng `+% -% *%` / có bảo vệ, để chương trình chạy sâu hơn
    safe: bool,
    pub stats: GenStats,
    counter: usize,
    helpers: Vec<Fx>,
}

struct Scope {
    /// biến nguyên đọc được
    vars: Vec<String>,
    /// biến gán được
    muts: Vec<String>,
    /// mảng cục bộ: (tên, độ dài)
    arrays: Vec<(String, usize)>,
    /// tham số lát (tên)
    slices: Vec<String>,
}

impl OGen {
    pub fn new(seed: u64) -> OGen {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let ty = TYPES[rng.below(TYPES.len() as u64) as usize];
        let safe = rng.chance(55);
        OGen {
            rng,
            ty,
            safe,
            stats: GenStats::default(),
            counter: 0,
            helpers: Vec::new(),
        }
    }

    fn fresh(&mut self, p: &str) -> String {
        self.counter += 1;
        format!("{p}{}", self.counter)
    }

    fn lit(&mut self) -> String {
        let b = self.ty.boundary();
        // thiên về hằng nhỏ để điều kiện/bảo vệ có ý nghĩa
        let v = if self.rng.chance(70) {
            b[self.rng.below(b.len() as u64) as usize]
        } else {
            let span = (self.ty.max() - self.ty.min()).min(1 << 20) as u64;
            (self.ty.min().max(-(1 << 19)) + self.rng.below(span + 1) as i128).min(self.ty.max())
        };
        let v = v.clamp(self.ty.min(), self.ty.max());
        self.ty.lit(v)
    }

    fn var(&mut self, sc: &Scope) -> String {
        if sc.vars.is_empty() {
            self.lit()
        } else {
            sc.vars[self.rng.below(sc.vars.len() as u64) as usize].clone()
        }
    }

    fn atom(&mut self, sc: &Scope) -> String {
        if self.rng.chance(60) {
            self.var(sc)
        } else {
            self.lit()
        }
    }

    /// chỉ số `usize` an toàn về kiểu (có thể ngoài biên: đó là mục đích)
    fn index(&mut self, sc: &Scope, e: &str) -> String {
        let _ = sc;
        if self.ty.name == "usize" {
            e.to_string()
        } else {
            self.stats.casts += 1;
            format!("({e} as usize)")
        }
    }

    fn expr(&mut self, sc: &Scope, depth: u32) -> String {
        if depth == 0 {
            return self.atom(sc);
        }
        match self.rng.below(100) {
            0..=24 => self.atom(sc),
            25..=49 => {
                let wrap = self.safe && self.rng.chance(65);
                let op = if wrap {
                    ["+%", "-%", "*%"][self.rng.below(3) as usize]
                } else {
                    ["+", "-", "*"][self.rng.below(3) as usize]
                };
                format!(
                    "({} {op} {})",
                    self.expr(sc, depth - 1),
                    self.expr(sc, depth - 1)
                )
            }
            50..=55 => {
                self.stats.divs += 1;
                let op = ["/", "%"][self.rng.below(2) as usize];
                if self.ty.signed && self.rng.chance(12) {
                    // MIN / -1 luôn tràn (E0601) hoặc, qua biến, chưa chắc
                    let m = self.ty.lit(self.ty.min());
                    let mo = self.ty.lit(-1);
                    let l = if self.rng.chance(50) {
                        m
                    } else {
                        self.atom(sc)
                    };
                    return format!("({l} {op} {mo})");
                }
                if self.safe && self.rng.chance(70) {
                    // mẫu số có bảo vệ: chứng minh được ≠ 0 (số có dấu vẫn còn MIN / -1)
                    let d = self.atom(sc);
                    let z = self.ty.lit(0);
                    format!(
                        "(⎇ {d} != {z} {{ {} {op} {d} }} ⇝ {{ {} }})",
                        self.expr(sc, depth - 1),
                        self.atom(sc)
                    )
                } else {
                    format!(
                        "({} {op} {})",
                        self.expr(sc, depth - 1),
                        self.expr(sc, depth - 1)
                    )
                }
            }
            56..=60 if self.ty.signed => {
                self.stats.negs += 1;
                format!("(-{})", self.expr(sc, depth - 1))
            }
            61..=68 => {
                // chuyển đổi qua kiểu hẹp/khác dấu rồi về `T`
                self.stats.casts += 1;
                let via = ["u8", "i8", "u16", "i16", "u32", "i32"][self.rng.below(6) as usize];
                format!(
                    "(({} as {via}) as {})",
                    self.expr(sc, depth - 1),
                    self.ty.name
                )
            }
            69..=80 => {
                let c = self.cond(sc, depth - 1);
                format!(
                    "(⎇ {c} {{ {} }} ⇝ {{ {} }})",
                    self.expr(sc, depth - 1),
                    self.expr(sc, depth - 1)
                )
            }
            81..=88 if !sc.arrays.is_empty() || !sc.slices.is_empty() => {
                let i = self.expr(sc, depth - 1);
                let ix = self.index(sc, &i);
                if self.rng.chance(45) {
                    // chỉ số có bảo vệ bằng độ dài (chứng minh được)
                    let fb = self.atom(sc);
                    if !sc.slices.is_empty() && (sc.arrays.is_empty() || self.rng.chance(50)) {
                        let n = sc.slices[self.rng.below(sc.slices.len() as u64) as usize].clone();
                        return format!("(⎇ {ix} < {n}.len() {{ {n}[{ix}] }} ⇝ {{ {fb} }})");
                    }
                    let (n, len) =
                        sc.arrays[self.rng.below(sc.arrays.len() as u64) as usize].clone();
                    return format!("(⎇ {ix} < {len}usize {{ {n}[{ix}] }} ⇝ {{ {fb} }})");
                }
                if !sc.arrays.is_empty() && (sc.slices.is_empty() || self.rng.chance(50)) {
                    let (n, _) = sc.arrays[self.rng.below(sc.arrays.len() as u64) as usize].clone();
                    format!("{n}[{ix}]")
                } else {
                    let n = sc.slices[self.rng.below(sc.slices.len() as u64) as usize].clone();
                    format!("{n}[{ix}]")
                }
            }
            89..=99 if !self.helpers.is_empty() => {
                let k = self.rng.below(self.helpers.len() as u64) as usize;
                let (name, params, has_slice) = {
                    let h = &self.helpers[k];
                    (h.name.clone(), h.params, h.has_slice)
                };
                let mut args: Vec<String> = Vec::new();
                for _ in 0..params {
                    args.push(self.expr(sc, depth - 1));
                }
                if has_slice {
                    if let Some((a, n)) = sc.arrays.first().cloned() {
                        self.stats.slice_calls += 1;
                        // cắt lát [0..k] với k có thể vượt độ dài (nghĩa vụ SliceRange)
                        let k = if self.rng.chance(40) {
                            n.to_string()
                        } else if self.rng.chance(50) {
                            format!("{}", n + 1)
                        } else {
                            // cận cắt là biến (nghĩa vụ chưa chứng minh được)
                            let v = self.var(sc);
                            self.index(sc, &v)
                        };
                        args.insert(0, format!("&{a}[0..{k}]"));
                    } else if let Some(s) = sc.slices.first() {
                        self.stats.slice_calls += 1;
                        args.insert(0, s.clone());
                    } else {
                        return self.atom(sc);
                    }
                }
                format!("{name}({})", args.join(", "))
            }
            _ => self.atom(sc),
        }
    }

    fn cond(&mut self, sc: &Scope, depth: u32) -> String {
        let cmp = ["<", "<=", ">", ">=", "=="][self.rng.below(5) as usize];
        let a = self.expr(sc, depth.min(1));
        let b = self.expr(sc, depth.min(1));
        let base = format!("{a} {cmp} {b}");
        match self.rng.below(10) {
            0 => format!("({base} ∧ {} < {})", self.atom(sc), self.atom(sc)),
            1 => format!("({base} ∨ {} >= {})", self.atom(sc), self.atom(sc)),
            2 => format!("(¬({base}))"),
            _ => format!("({base})"),
        }
    }

    fn stmts(&mut self, sc: &mut Scope, depth: u32, n: usize, in_fn_with_ret: bool) -> String {
        let mut out = String::new();
        for _ in 0..n {
            match self.rng.below(100) {
                0..=24 => {
                    let v = self.fresh("v");
                    let e = self.expr(sc, 2);
                    out.push_str(&format!("μ{v} ≔ {e}; "));
                    sc.vars.push(v.clone());
                    sc.muts.push(v);
                }
                25..=44 if !sc.muts.is_empty() => {
                    let v = sc.muts[self.rng.below(sc.muts.len() as u64) as usize].clone();
                    let e = self.expr(sc, 2);
                    out.push_str(&format!("{v} ← {e}; "));
                }
                45..=59 if depth > 0 => {
                    let c = self.cond(sc, 1);
                    let mut s1 = Scope {
                        vars: sc.vars.clone(),
                        muts: sc.muts.clone(),
                        arrays: sc.arrays.clone(),
                        slices: sc.slices.clone(),
                    };
                    let a = self.stmts(&mut s1, depth - 1, 2, false);
                    let mut s2 = Scope {
                        vars: sc.vars.clone(),
                        muts: sc.muts.clone(),
                        arrays: sc.arrays.clone(),
                        slices: sc.slices.clone(),
                    };
                    let b = self.stmts(&mut s2, depth - 1, 1, false);
                    out.push_str(&format!("⎇ {c} {{ {a}}} ⇝ {{ {b}}} "));
                }
                60..=74 if depth > 0 && !sc.muts.is_empty() => {
                    self.stats.loops += 1;
                    let i = self.fresh("i");
                    let lim = if self.rng.chance(50) {
                        self.lit()
                    } else {
                        self.var(sc)
                    };
                    // đếm lên từ 0; biến thể = lim - i (thuộc M8, ở đây chỉ để qua parser)
                    let zero = self.ty.lit(0);
                    let one = self.ty.lit(1);
                    let acc = sc.muts[self.rng.below(sc.muts.len() as u64) as usize].clone();
                    let inv = if self.rng.chance(40) {
                        self.stats.invariants += 1;
                        let k = self.lit();
                        format!(" ⋈ {i} <= {k}")
                    } else {
                        String::new()
                    };
                    let mut s1 = Scope {
                        vars: sc.vars.clone(),
                        muts: sc.muts.clone(),
                        arrays: sc.arrays.clone(),
                        slices: sc.slices.clone(),
                    };
                    s1.vars.push(i.clone());
                    let body = self.expr(&s1, 2);
                    let wrap = if self.rng.chance(50) { "+" } else { "+%" };
                    out.push_str(&format!(
                        "μ{i} ≔ {zero}; ⟳ {i} < {lim} ↓ {lim} - {i}{inv} {{ {acc} ← {acc} {wrap} {body}; {i} ← {i} + {one}; }} "
                    ));
                }
                75..=84 if sc.arrays.len() < 2 && self.ty.bits >= 8 => {
                    self.stats.arrays += 1;
                    let n = 1 + self.rng.below(4) as usize;
                    let a = self.fresh("a");
                    let elems: Vec<String> = (0..n).map(|_| self.atom(sc)).collect();
                    out.push_str(&format!("{a} ≔ [{}]; ", elems.join(", ")));
                    sc.arrays.push((a, n));
                }
                85..=92 if in_fn_with_ret => {
                    let c = self.cond(sc, 1);
                    let e = self.expr(sc, 2);
                    out.push_str(&format!("⎇ {c} {{ ⏎ {e}; }} "));
                }
                _ => {
                    let v = self.fresh("w");
                    let e = self.expr(sc, 2);
                    out.push_str(&format!("{v} ≔ {e}; "));
                    sc.vars.push(v);
                }
            }
        }
        out
    }

    fn contracts(&mut self, params: &[String], has_ret_post: bool) -> String {
        let mut out = String::new();
        if params.is_empty() {
            return out;
        }
        if self.rng.chance(45) {
            self.stats.pre += 1;
            let p = &params[self.rng.below(params.len() as u64) as usize];
            let k = self.lit();
            let form = match self.rng.below(4) {
                0 => format!("{p} > {k}"),
                1 => format!("{p} <= {k}"),
                2 => format!("{p} >= {} ∧ {p} <= {}", self.lit(), self.lit()),
                _ => format!("{p} != {k}"),
            };
            out.push_str(&format!(" ⊳ {form}"));
        }
        if has_ret_post && self.rng.chance(40) {
            self.stats.post += 1;
            let k = self.lit();
            let p = &params[self.rng.below(params.len() as u64) as usize];
            let form = match self.rng.below(3) {
                0 => format!("result >= {k}"),
                1 => format!("result <= {p}"),
                _ => format!("result != {k}"),
            };
            out.push_str(&format!(" ⊲ {form}"));
        }
        out
    }

    pub fn program(&mut self) -> Prog {
        self.stats.programs += 1;
        let t = self.ty.name;
        let mut src = String::new();
        let nh = 1 + self.rng.below(3) as usize;
        for _ in 0..nh {
            let name = self.fresh("h");
            let has_slice = self.rng.chance(35);
            let np = 1 + self.rng.below(2) as usize;
            let pnames: Vec<String> = (0..np).map(|k| format!("p{k}")).collect();
            let mut sc = Scope {
                vars: pnames.clone(),
                muts: Vec::new(),
                arrays: Vec::new(),
                slices: Vec::new(),
            };
            let mut params: Vec<String> = pnames.iter().map(|p| format!("{p}: {t}")).collect();
            if has_slice {
                params.insert(0, format!("xs: &[{t}]"));
                sc.slices.push("xs".into());
            }
            let ctr = self.contracts(&pnames, true);
            let n = 2 + self.rng.below(3) as usize;
            let body = self.stmts(&mut sc, 2, n, true);
            let tail = self.expr(&sc, 2);
            src.push_str(&format!(
                "ƒ {name}({}) → {t} ! ∅{ctr} {{ {body}{tail} }}\n",
                params.join(", ")
            ));
            self.helpers.push(Fx {
                name,
                params: np,
                has_slice,
            });
        }
        let entry_params = 3;
        let pn: Vec<String> = (0..entry_params).map(|k| format!("a{k}")).collect();
        let mut sc = Scope {
            vars: pn.clone(),
            muts: Vec::new(),
            arrays: Vec::new(),
            slices: Vec::new(),
        };
        let params: Vec<String> = pn.iter().map(|p| format!("{p}: {t}")).collect();
        let n = 3 + self.rng.below(4) as usize;
        let body = self.stmts(&mut sc, 3, n, true);
        let tail = self.expr(&sc, 3);
        src.push_str(&format!(
            "ƒ entry({}) → {t} ! ∅ {{ {body}{tail} }}\n",
            params.join(", ")
        ));
        Prog {
            src,
            ty: self.ty,
            entry_params,
        }
    }
}
