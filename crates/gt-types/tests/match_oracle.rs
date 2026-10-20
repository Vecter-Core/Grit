//! ORACLE ngẫu nhiên cho vét cạn `⌥` trên số nguyên (luật bộ sinh, FB-14): sinh `⌥` với các nhánh literal/khoảng ngẫu nhiên CÓ CHỦ Ý dồn vào biên (0, 1, 2^(n-1) ± 1, MAX-1, MAX…),
//! so phán quyết E0305 của gt-types với một oracle độc lập:
//! * `u8`/`i8`: duyệt TOÀN BỘ 256 giá trị ⇒ "vét cạn" phải KHỚP CHÍNH XÁC (hai chiều);
//! * `u128`/`i128`: không duyệt hết được ⇒ kiểm chiều AN TOÀN: nếu gt-types chấp nhận (không E0305) thì MỌI điểm quan trọng (đầu mút ± 1, 0, MAX…) phải được một nhánh phủ.
//!
//! Giá trị luôn xử lý như độ lệch `u128` so với giá trị nhỏ nhất của kiểu (lật bit dấu cho `i128`) nên không dùng số học tràn.
mod common;

const N_DEFAULT: usize = 400;

#[derive(Clone, Copy)]
struct Ty {
    name: &'static str,
    bits: u32,
    signed: bool,
}

const TYS: [Ty; 4] = [
    Ty {
        name: "u8",
        bits: 8,
        signed: false,
    },
    Ty {
        name: "i8",
        bits: 8,
        signed: true,
    },
    Ty {
        name: "u128",
        bits: 128,
        signed: false,
    },
    Ty {
        name: "i128",
        bits: 128,
        signed: true,
    },
];

impl Ty {
    fn max_off(self) -> u128 {
        if self.bits == 128 {
            u128::MAX
        } else {
            (1u128 << self.bits) - 1
        }
    }
    /// độ lệch (so với giá trị nhỏ nhất của kiểu) → văn bản literal thập phân
    fn text(self, off: u128) -> String {
        if !self.signed {
            return off.to_string();
        }
        if self.bits == 128 {
            ((off ^ (1u128 << 127)) as i128).to_string()
        } else {
            (off as i128 - (1i128 << (self.bits - 1))).to_string()
        }
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// độ lệch "thú vị": dồn vào biên của miền, nửa miền và lân cận
fn pick(r: &mut Rng, t: Ty) -> u128 {
    let max = t.max_off();
    let mid = max / 2 + 1;
    let base = match r.below(7) {
        0 => 0,
        1 => max,
        2 => mid,
        3 => mid - 1,
        4 => 1,
        5 => max - 1,
        _ => (u128::from(r.next()) << 64 | u128::from(r.next())) & max,
    };
    match r.below(4) {
        0 => base.saturating_add(1).min(max),
        1 => base.saturating_sub(1),
        _ => base,
    }
}

#[derive(Clone, Copy, Debug)]
enum Arm {
    Lit(u128),
    Incl(u128, u128),
    Excl(u128, u128),
    From(u128),
    Upto(u128), // ..b (loại trừ b)
}

impl Arm {
    fn covers(self, v: u128, max: u128) -> bool {
        match self {
            Arm::Lit(a) => v == a,
            Arm::Incl(a, b) => a <= v && v <= b,
            Arm::Excl(a, b) => a <= v && v < b,
            Arm::From(a) => a <= v && v <= max,
            Arm::Upto(b) => v < b,
        }
    }
    fn text(self, t: Ty) -> String {
        match self {
            Arm::Lit(a) => t.text(a),
            Arm::Incl(a, b) => format!("{}..={}", t.text(a), t.text(b)),
            Arm::Excl(a, b) => format!("{}..{}", t.text(a), t.text(b)),
            Arm::From(a) => format!("{}..", t.text(a)),
            Arm::Upto(b) => format!("..{}", t.text(b)),
        }
    }
    fn points(self, out: &mut Vec<u128>) {
        let ends: Vec<u128> = match self {
            Arm::Lit(a) | Arm::From(a) | Arm::Upto(a) => vec![a],
            Arm::Incl(a, b) | Arm::Excl(a, b) => vec![a, b],
        };
        for e in ends {
            out.extend([e.saturating_sub(1), e, e.saturating_add(1)]);
        }
    }
}

fn gen_arm(r: &mut Rng, t: Ty) -> Arm {
    let max = t.max_off();
    loop {
        match r.below(5) {
            0 => return Arm::Lit(pick(r, t)),
            1 => {
                let (a, b) = (pick(r, t), pick(r, t));
                return Arm::Incl(a.min(b), a.max(b));
            }
            2 => {
                let (a, b) = (pick(r, t), pick(r, t));
                if a != b {
                    return Arm::Excl(a.min(b), a.max(b));
                }
            }
            3 => return Arm::From(pick(r, t)),
            _ => {
                let b = pick(r, t);
                if b > 0 && b <= max {
                    return Arm::Upto(b);
                }
            }
        }
    }
}

fn run_one(r: &mut Rng, t: Ty, stats: &mut [usize; 4]) {
    let n = 1 + r.below(4) as usize;
    let arms: Vec<Arm> = (0..n).map(|_| gen_arm(r, t)).collect();
    let body: Vec<String> = arms
        .iter()
        .enumerate()
        .map(|(i, a)| format!("{} ↦ {}u32", a.text(t), i + 1))
        .collect();
    let src = format!(
        "ƒ f(x: {}) → u32 ! ∅ {{ ⌥ x {{ {} }} }}",
        t.name,
        body.join(", ")
    );
    let out = common::one(&src);
    let codes = common::codes(&out);
    let non_exhaustive = codes.iter().any(|c| c == "E0305");
    assert!(
        codes.iter().all(|c| c == "E0305"),
        "mã lạ {codes:?} cho {src}"
    );
    let max = t.max_off();
    if t.bits == 8 {
        let all = (0..=max).all(|v| arms.iter().any(|a| a.covers(v, max)));
        assert_eq!(!non_exhaustive, all, "u8/i8 phải khớp chính xác: {src}");
        stats[usize::from(all)] += 1;
    } else {
        let mut pts = vec![0, 1, max - 1, max, max / 2, max / 2 + 1];
        for a in &arms {
            a.points(&mut pts);
        }
        let missing = pts
            .iter()
            .copied()
            .filter(|&v| v <= max)
            .find(|&v| !arms.iter().any(|a| a.covers(v, max)));
        if !non_exhaustive {
            assert!(
                missing.is_none(),
                "gt-types CHẤP NHẬN nhưng giá trị (độ lệch) {missing:?} không nhánh nào phủ: {src}"
            );
            stats[2] += 1;
        } else {
            stats[3] += 1;
        }
    }
}

#[test]
fn integer_match_exhaustiveness_agrees_with_an_independent_oracle() {
    let n: usize = std::env::var("GT_MATCH_PROGRAMS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(N_DEFAULT);
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    // [8-bit vét cạn, 8-bit không, 128-bit chấp nhận, 128-bit từ chối]
    let mut stats = [0usize; 4];
    for i in 0..n {
        run_one(&mut r, TYS[i % 4], &mut stats);
    }
    println!(
        "match_oracle: n={n} 8bit(không vét cạn={}, vét cạn={}) 128bit(chấp nhận={}, từ chối={})",
        stats[0], stats[1], stats[2], stats[3]
    );
    assert!(
        stats[1] > 0 && stats[0] > 0,
        "bộ sinh phải ra cả hai loại 8-bit: {stats:?}"
    );
}
