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
    pub compound_loops: usize,
}

pub struct Prog {
    pub src: String,
    pub ty: Ity,
    /// số tham số của `main::entry`
    pub entry_params: usize,
    /// khoảng [lo, hi] mà tiền điều kiện `⊳` của `entry` cho phép cho từng tham số (không có `⊳` ⇒ cả kiểu)
    pub entry_ranges: Vec<(i128, i128)>,
}

struct Fx {
    name: String,
    params: usize,
    has_slice: bool,
    /// tham số (chỉ số) và khoảng [lo, hi] mà `⊳` cho phép: lời gọi thường truyền đối số trong khoảng đó
    pre_range: Option<(usize, i128, i128)>,
}

pub struct OGen {
    pub rng: Rng,
    ty: Ity,
    /// chế độ "an toàn": phần lớn phép toán dùng `+% -% *%` / có bảo vệ, để chương trình chạy sâu hơn
    safe: bool,
    /// chế độ "nghiêm": mọi phép có nghĩa vụ đều bọc trong bảo vệ chứng minh được VÀ thật sự đúng lúc chạy, để chạy tới cuối hàm
    strict: bool,
    pub stats: GenStats,
    counter: usize,
    helpers: Vec<Fx>,
    /// R2a (lệnh Giang 2026-10-17): mọi lời gọi hàm phụ đi qua GIÁ TRỊ HÀM (`hv_hK ≔ hK;` đầu mỗi thân, gọi `hv_hK(...)`). KHÔNG đổi dãy số ngẫu nhiên.
    via_value: bool,
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
        let safe = rng.chance(75);
        let strict = safe && rng.chance(50);
        OGen {
            rng,
            ty,
            safe,
            strict,
            stats: GenStats::default(),
            counter: 0,
            helpers: Vec::new(),
            via_value: false,
        }
    }

    /// như `new` nhưng mọi lời gọi hàm phụ qua giá trị hàm (cùng dãy số ngẫu nhiên ⇒ cùng chương trình, chỉ đổi cách gọi)
    pub fn with_values(seed: u64) -> OGen {
        let mut g = OGen::new(seed);
        g.via_value = true;
        g
    }

    /// các phép gán giá trị hàm đầu thân: một cho MỖI hàm phụ đã định nghĩa
    fn value_bindings(&self) -> String {
        if !self.via_value {
            return String::new();
        }
        self.helpers
            .iter()
            .map(|h| format!("hv_{0} ≔ {0}; ", h.name))
            .collect()
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

    /// phép `+ - *` kiểm tra bọc trong bảo vệ vừa chứng minh được vừa đúng lúc chạy (không tràn)
    fn guarded_arith(&mut self, sc: &Scope) -> String {
        let (a, b) = (self.atom(sc), self.atom(sc));
        let ty = self.ty;
        let half = |lo: i128, hi: i128| (lo / 2, hi / 2);
        let (op, guard) = match self.rng.below(3) {
            0 => {
                let (lo, hi) = half(ty.min(), ty.max());
                (
                    "+",
                    format!(
                        "{a} >= {l} ∧ {a} <= {h} ∧ {b} >= {l} ∧ {b} <= {h}",
                        l = ty.lit(lo),
                        h = ty.lit(hi)
                    ),
                )
            }
            1 if ty.signed => {
                let (lo, hi) = half(ty.min(), ty.max());
                (
                    "-",
                    format!(
                        "{a} >= {l} ∧ {a} <= {h} ∧ {b} >= {l} ∧ {b} <= {h}",
                        l = ty.lit(lo),
                        h = ty.lit(hi)
                    ),
                )
            }
            1 => ("-", format!("{a} >= {b}")),
            _ => {
                let k = if ty.signed {
                    (ty.bits - 1) / 2
                } else {
                    ty.bits / 2
                };
                let lim = (1i128 << k) - 1;
                let lo = if ty.signed { -lim } else { 0 };
                (
                    "*",
                    format!(
                        "{a} >= {l} ∧ {a} <= {h} ∧ {b} >= {l} ∧ {b} <= {h}",
                        l = ty.lit(lo),
                        h = ty.lit(lim)
                    ),
                )
            }
        };
        format!("(⎇ {guard} {{ ({a} {op} {b}) }} ⇝ {{ {a} }})")
    }

    fn expr(&mut self, sc: &Scope, depth: u32) -> String {
        if depth == 0 {
            return self.atom(sc);
        }
        match self.rng.below(100) {
            0..=24 => self.atom(sc),
            25..=49 => {
                if self.strict && self.rng.chance(60) {
                    return self.guarded_arith(sc);
                }
                let wrap = self.strict || (self.safe && self.rng.chance(65));
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
                if self.strict || (self.safe && self.rng.chance(70)) {
                    // mẫu số có bảo vệ: chứng minh được ≠ 0 (nghiêm: cả ≠ -1 cho số có dấu)
                    let d = self.atom(sc);
                    let z = self.ty.lit(0);
                    let extra = if self.strict && self.ty.signed {
                        format!(" ∧ {d} != {}", self.ty.lit(-1))
                    } else {
                        String::new()
                    };
                    format!(
                        "(⎇ {d} != {z}{extra} {{ {} {op} {d} }} ⇝ {{ {} }})",
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
                if self.strict {
                    let a = self.atom(sc);
                    return format!(
                        "(⎇ {a} > {} {{ (-{a}) }} ⇝ {{ {a} }})",
                        self.ty.lit(self.ty.min())
                    );
                }
                format!("(-{})", self.expr(sc, depth - 1))
            }
            61..=68 => {
                // chuyển đổi qua kiểu hẹp/khác dấu rồi về `T`
                self.stats.casts += 1;
                let vi = self.rng.below(6) as usize;
                let via = ["u8", "i8", "u16", "i16", "u32", "i32"][vi];
                let e = self.expr(sc, depth - 1);
                if self.strict || (self.safe && self.rng.chance(75)) {
                    // chế độ an toàn: chỉ ép khi giá trị nằm trong khoảng của kiểu trung gian (kiểu chứng minh được)
                    let (vb, vs) = [
                        (8u32, false),
                        (8, true),
                        (16, false),
                        (16, true),
                        (32, false),
                        (32, true),
                    ][vi];
                    let via_t = Ity {
                        name: via,
                        bits: vb,
                        signed: vs,
                    };
                    let (lo, hi) = (
                        via_t.min().max(self.ty.min()),
                        via_t.max().min(self.ty.max()),
                    );
                    return format!(
                        "(⎇ {e} >= {} ∧ {e} <= {} {{ (({e} as {via}) as {}) }} ⇝ {{ {e} }})",
                        self.ty.lit(lo),
                        self.ty.lit(hi),
                        self.ty.name
                    );
                }
                format!("(({e} as {via}) as {})", self.ty.name)
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
                if self.strict || self.rng.chance(if self.safe { 85 } else { 45 }) {
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
                self.call_helper(sc, k, depth, false)
                    .unwrap_or_else(|| self.atom(sc))
            }
            _ => self.atom(sc),
        }
    }

    /// lời gọi hàm phụ thứ `k` với đối số ngẫu nhiên (và hai lát nếu hàm nhận lát)
    fn call_helper(
        &mut self,
        sc: &Scope,
        k: usize,
        depth: u32,
        valid_slices: bool,
    ) -> Option<String> {
        let (name, params, has_slice) = {
            let h = &self.helpers[k];
            (h.name.clone(), h.params, h.has_slice)
        };
        let mut args: Vec<String> = Vec::new();
        let pre_range = self.helpers[k].pre_range;
        for j in 0..params {
            if let Some((pi, lo, hi)) = pre_range
                && pi == j
                && self.rng.chance(75)
            {
                // đối số thoả ⊳ của hàm gọi (kể cả các biên của nó): lời gọi chạy tới cuối thay vì abort ngay
                let edges = [lo, lo + 1, hi - 1, hi, lo + (hi - lo) / 2];
                let e = edges[self.rng.below(edges.len() as u64) as usize];
                args.push(self.ty.lit(e.clamp(lo, hi)));
                continue;
            }
            args.push(self.expr(sc, depth.saturating_sub(1)));
        }
        if has_slice {
            // hàm phụ có HAI tham số lát `(μxs, zs)`: truyền hai lát (cùng hoặc khác mảng)
            let (a1, a2) = (
                self.slice_arg(sc, valid_slices)?,
                self.slice_arg(sc, valid_slices)?,
            );
            self.stats.slice_calls += 1;
            args.insert(0, a2);
            args.insert(0, a1);
        }
        if self.via_value {
            return Some(format!("hv_{name}({})", args.join(", ")));
        }
        Some(format!("{name}({})", args.join(", ")))
    }

    /// một biểu thức kiểu `&[T]`: cắt mảng cục bộ hoặc chuyển tiếp một tham số lát
    fn slice_arg(&mut self, sc: &Scope, valid: bool) -> Option<String> {
        if let Some((a, n)) = sc.arrays.first().cloned()
            && (sc.slices.is_empty() || self.rng.chance(70))
        {
            // cắt lát [0..k] với k có thể vượt độ dài (nghĩa vụ SliceRange)
            let k = if valid || self.strict || self.rng.chance(60) {
                // lát hợp lệ, độ dài tuỳ ý trong [1, n] để hai lát có thể KHÁC độ dài
                (1 + self.rng.below(n as u64)).to_string()
            } else if self.rng.chance(50) {
                format!("{}", n + 1)
            } else {
                // cận cắt là biến (nghĩa vụ chưa chứng minh được)
                let v = self.var(sc);
                self.index(sc, &v)
            };
            return Some(format!("&{a}[0..{k}]"));
        }
        if sc.slices.is_empty() {
            return None;
        }
        Some(sc.slices[self.rng.below(sc.slices.len() as u64) as usize].clone())
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
                    let lim = if self.strict {
                        // vòng ngắn: chạy tới cuối thay vì cạn nhiên liệu
                        let hi = (self.ty.max()).min(12);
                        self.ty.lit(1 + self.rng.below(hi as u64) as i128)
                    } else if self.rng.chance(50) {
                        self.lit()
                    } else {
                        self.var(sc)
                    };
                    // đếm lên từ 0; biến thể = lim - i (thuộc M8, ở đây chỉ để qua parser)
                    let zero = self.ty.lit(0);
                    let one = self.ty.lit(1);
                    let acc = sc.muts[self.rng.below(sc.muts.len() as u64) as usize].clone();
                    let inv = if self.rng.chance(70) {
                        self.stats.invariants += 1;
                        // một nửa bất biến là `i <= lim` (chứng minh được và đúng lúc chạy), nửa kia hằng ngẫu nhiên
                        let k = if self.rng.chance(50) {
                            lim.clone()
                        } else {
                            self.lit()
                        };
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
                    let wrap = if self.rng.chance(if self.safe { 10 } else { 50 }) {
                        "+"
                    } else {
                        "+%"
                    };
                    // điều kiện ghép của vòng lặp (hạ thẳng thành nhánh, M5-fix 2026-10-08): `∧`/`∨` với một so sánh bất kỳ
                    let guard = if self.rng.chance(35) {
                        self.stats.compound_loops += 1;
                        let c2 = self.cond(&s1, 1);
                        let op = "∧"; // `∨` có thể lặp vô hạn (nhiên liệu cạn ⇒ ca bị bỏ)
                        format!("{i} < {lim} {op} {c2}")
                    } else {
                        format!("{i} < {lim}")
                    };
                    out.push_str(&format!(
                        "μ{i} ≔ {zero}; ⟳ {guard} ↓ {lim} - {i}{inv} {{ {acc} ← {acc} {wrap} {body}; {i} ← {i} + {one}; }} "
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
                93..=96 if !sc.slices.is_empty() => {
                    // bí danh bất biến của một lát
                    let from = sc.slices[self.rng.below(sc.slices.len() as u64) as usize].clone();
                    let al = self.fresh("al");
                    out.push_str(&format!("{al} ≔ {from}; "));
                    sc.slices.push(al);
                }
                97..=99
                    if sc.slices.iter().any(|x| x == "xs")
                        && sc.slices.iter().any(|x| x == "zs") =>
                {
                    out.push_str("xs ← zs; ");
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

    fn contracts(
        &mut self,
        params: &[String],
        has_ret_post: bool,
    ) -> (String, Option<(usize, i128, i128)>) {
        let mut out = String::new();
        let mut range = None;
        if params.is_empty() {
            return (out, range);
        }
        if self.rng.chance(if self.strict {
            4
        } else if self.safe {
            12
        } else {
            30
        }) {
            self.stats.pre += 1;
            let pi = self.rng.below(params.len() as u64) as usize;
            let p = &params[pi];
            let (tmin, tmax) = (self.ty.min(), self.ty.max());
            let b = self.ty.boundary();
            let k = b[self.rng.below(b.len() as u64) as usize];
            let form = match self.rng.below(4) {
                0 => {
                    if k < tmax {
                        range = Some((pi, k + 1, tmax));
                    }
                    format!("{p} > {}", self.ty.lit(k))
                }
                1 => {
                    range = Some((pi, tmin, k));
                    format!("{p} <= {}", self.ty.lit(k))
                }
                2 => {
                    let hi = (k + 1 + self.rng.below(30) as i128).min(tmax);
                    range = Some((pi, k, hi));
                    format!("{p} >= {} ∧ {p} <= {}", self.ty.lit(k), self.ty.lit(hi))
                }
                _ => {
                    range = Some((
                        pi,
                        if k < tmax { k + 1 } else { k - 1 },
                        if k < tmax { k + 1 } else { k - 1 },
                    ));
                    format!("{p} != {}", self.ty.lit(k))
                }
            };
            out.push_str(&format!(" ⊳ {form}"));
        }
        if has_ret_post
            && self.rng.chance(if self.strict {
                4
            } else if self.safe {
                15
            } else {
                40
            })
        {
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
        (out, range)
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
                // `μxs` có thể bị gán lại (đối chứng cho chuỗi bí danh của tham số lát), `zs` thì không
                params.insert(0, format!("zs: &[{t}]"));
                params.insert(0, format!("μxs: &[{t}]"));
                sc.slices.push("xs".into());
                sc.slices.push("zs".into());
            }
            let (mut ctr, pre_range) = self.contracts(&pnames, true);
            let n = 2 + self.rng.below(3) as usize;
            // mẫu "trả nguyên tham số": thân không có `⏎` sớm, đuôi `p0`, hậu điều kiện về `p0` (chứng minh được và đúng lúc chạy)
            let id_tail = self.rng.chance(25);
            if id_tail {
                self.stats.post += 1;
                let rel = ["==", ">=", "<="][self.rng.below(3) as usize];
                ctr.push_str(&format!(" ⊲ result {rel} p0"));
            }
            // kịch bản "bí danh rồi gán lại tham số lát" (luật bộ sinh grit-contract §3.3, lỗ hổng #3 của M7)
            let mut pre = String::new();
            if has_slice && self.rng.chance(40) {
                pre.push_str("al0 ≔ xs; xs ← zs; ");
                sc.slices.push("al0".into());
                // truy cập `xs` (đã là `zs`) sau khi chỉ kiểm độ dài của bí danh (= xs cũ)
                let e = self.expr(&sc, 1);
                let ix = self.index(&sc, &e);
                let fb = self.atom(&sc);
                let w = self.fresh("w");
                pre.push_str(&format!(
                    "{w} ≔ ⎇ {ix} < al0.len() {{ xs[{ix}] }} ⇝ {{ {fb} }}; "
                ));
            }
            let vb = self.value_bindings();
            let body = format!("{vb}{pre}{}", self.stmts(&mut sc, 2, n, !id_tail));
            let tail = if id_tail {
                "p0".to_string()
            } else {
                self.expr(&sc, 2)
            };
            src.push_str(&format!(
                "ƒ {name}({}) → {t} ! ∅{ctr} {{ {body}{tail} }}\n",
                params.join(", ")
            ));
            self.helpers.push(Fx {
                name,
                params: np,
                has_slice,
                pre_range,
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
        // tiền điều kiện của `entry`: bộ chạy CHỈ cấp đầu vào thoả nó (không ai kiểm `⊳` của `entry`), kể cả các biên của nó
        let mut ranges = vec![(self.ty.min(), self.ty.max()); entry_params];
        let mut entry_pre = String::new();
        if self.rng.chance(65) {
            let b = self.ty.boundary();
            let mut conj: Vec<String> = Vec::new();
            for (k, r) in ranges.iter_mut().enumerate() {
                if self.rng.chance(80) {
                    let lo = b[self.rng.below(b.len() as u64) as usize];
                    let span = 1 + self.rng.below(40) as i128;
                    let hi = (lo + span).min(self.ty.max());
                    *r = (lo, hi);
                    conj.push(format!(
                        "a{k} >= {} ∧ a{k} <= {}",
                        self.ty.lit(lo),
                        self.ty.lit(hi)
                    ));
                }
            }
            if !conj.is_empty() {
                self.stats.pre += 1;
                entry_pre = format!(" ⊳ {}", conj.join(" ∧ "));
            }
        }
        let n = 3 + self.rng.below(4) as usize;
        // gọi MỌI hàm phụ ngay đầu (trước các phép có thể abort), trên mảng `ar0` của đầu vào: để mọi hàm đều được chạy
        let mut calls = format!("{}ar0 ≔ [{}]; ", self.value_bindings(), pn.join(", "));
        sc.arrays.push(("ar0".into(), entry_params));
        for k in 0..self.helpers.len() {
            if let Some(c) = self.call_helper(&sc, k, 1, true) {
                let v = self.fresh("c");
                calls.push_str(&format!("{v} ≔ {c}; "));
            }
        }
        let body = format!("{calls}{}", self.stmts(&mut sc, 3, n, true));
        let tail = self.expr(&sc, 3);
        src.push_str(&format!(
            "ƒ entry({}) → {t} ! ∅{entry_pre} {{ {body}{tail} }}\n",
            params.join(", ")
        ));
        Prog {
            src,
            ty: self.ty,
            entry_params,
            entry_ranges: ranges,
        }
    }
}
