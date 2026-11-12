//! ORACLE ngẫu nhiên cho vét cạn `⌥` trên số nguyên và `char` (luật bộ sinh, FB-14; lệnh Giang 2026-10-17 mục 4): MỌI chiều rộng (i8…i128, u8…u128, isize, usize) và `char`;
//! khoảng đóng/mở (`a..=b`, `a..b`, `a..`, `..b`), literal đơn, khoảng CHỒNG nhau, literal âm, biên MIN/MAX, và (có xác suất) nhánh `_`. Một phần chương trình là PHÂN HOẠCH miền
//! (cắt ngẫu nhiên rồi có thể làm lệch 1 đơn vị / bỏ một nhánh) nên cả hai loại vét cạn và gần-vét-cạn đều xuất hiện.
//! Oracle ĐỘC LẬP: hợp các khoảng (độ lệch `u128` so với giá trị nhỏ nhất, không tràn) có phủ trọn miền hợp lệ không. Miền `char` KHÔNG liền khối: 0xD800–0xDFFF không phải giá trị.
//! * mọi kiểu trừ `u128`: gt-types chấp nhận (không E0305) ⇔ oracle vét cạn (HAI CHIỀU, chính xác);
//! * `u128`: miền không mô hình hoá (ADR-041) ⇒ chỉ kiểm chiều AN TOÀN (chấp nhận ⇒ vét cạn); đếm riêng số ca bảo thủ (oracle vét cạn nhưng E0305).
//!
//! Giá trị luôn xử lý như độ lệch `u128` so với giá trị nhỏ nhất của kiểu (lật bit dấu cho `i128`) nên không dùng số học tràn.
//! Với `char`, miền không liền khối: 0xD800–0xDFFF không phải giá trị Unicode scalar hợp lệ.
mod common;

const N_DEFAULT: usize = 650;

#[derive(Clone, Copy)]
struct Ty {
    name: &'static str,
    bits: u32,
    signed: bool,
    is_char: bool,
}

const fn it(name: &'static str, bits: u32, signed: bool) -> Ty {
    Ty {
        name,
        bits,
        signed,
        is_char: false,
    }
}

const TYS: [Ty; 13] = [
    it("i8", 8, true),
    it("u8", 8, false),
    it("i16", 16, true),
    it("u16", 16, false),
    it("i32", 32, true),
    it("u32", 32, false),
    it("i64", 64, true),
    it("u64", 64, false),
    it("i128", 128, true),
    it("u128", 128, false),
    it("isize", 64, true),
    it("usize", 64, false),
    Ty {
        name: "char",
        bits: 21,
        signed: false,
        is_char: true,
    },
];

const GAP_LO: u128 = 0xD800;
const GAP_HI: u128 = 0xDFFF;

impl Ty {
    fn max_off(self) -> u128 {
        if self.is_char {
            0x10FFFF
        } else if self.bits == 128 {
            u128::MAX
        } else {
            (1u128 << self.bits) - 1
        }
    }
    /// các đoạn giá trị HỢP LỆ (độ lệch)
    fn domain(self) -> Vec<(u128, u128)> {
        if self.is_char {
            vec![(0, GAP_LO - 1), (GAP_HI + 1, 0x10FFFF)]
        } else {
            vec![(0, self.max_off())]
        }
    }
    fn valid(self, v: u128) -> bool {
        !(self.is_char && (GAP_LO..=GAP_HI).contains(&v)) && v <= self.max_off()
    }
    /// độ lệch → literal văn bản
    fn text(self, off: u128) -> String {
        if self.is_char {
            return format!("'\\u{{{off:X}}}'");
        }
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

/// độ lệch HỢP LỆ "thú vị" (dồn vào biên, nửa miền, lân cận; với `char` tránh khe)
fn pick(r: &mut Rng, t: Ty) -> u128 {
    let max = t.max_off();
    let mid = max / 2 + 1;
    let base = match r.below(8) {
        0 => 0,
        1 => max,
        2 => mid,
        3 => mid - 1,
        4 => 1,
        5 => max - 1,
        6 if t.is_char => GAP_LO - 1,
        _ => (u128::from(r.next()) << 64 | u128::from(r.next())) & max,
    };
    let v = match r.below(4) {
        0 => base.saturating_add(1).min(max),
        1 => base.saturating_sub(1),
        _ => base,
    };
    if t.valid(v) {
        v
    } else if t.is_char {
        if r.below(2) == 0 {
            GAP_LO - 1
        } else {
            GAP_HI + 1
        }
    } else {
        v
    }
}

#[derive(Clone, Copy, Debug)]
enum Arm {
    Wild,
    Lit(u128),
    Incl(u128, u128),
    Excl(u128, u128),
    From(u128),
    Upto(u128), // ..b (loại trừ b)
}

impl Arm {
    /// khoảng độ lệch mà nhánh phủ (rỗng ⇒ None)
    fn interval(self, max: u128) -> Option<(u128, u128)> {
        match self {
            Arm::Wild => Some((0, max)),
            Arm::Lit(a) => Some((a, a)),
            Arm::Incl(a, b) => (a <= b).then_some((a, b)),
            Arm::Excl(a, b) => (a < b).then(|| (a, b - 1)),
            Arm::From(a) => (a <= max).then_some((a, max)),
            Arm::Upto(b) => (b > 0).then(|| (0, b - 1)),
        }
    }
    fn text(self, t: Ty) -> String {
        match self {
            Arm::Wild => "_".to_string(),
            Arm::Lit(a) => t.text(a),
            Arm::Incl(a, b) => format!("{}..={}", t.text(a), t.text(b)),
            Arm::Excl(a, b) => format!("{}..{}", t.text(a), t.text(b)),
            Arm::From(a) => format!("{}..", t.text(a)),
            Arm::Upto(b) => format!("..{}", t.text(b)),
        }
    }
}

fn gen_arm(r: &mut Rng, t: Ty) -> Arm {
    let max = t.max_off();
    loop {
        match r.below(12) {
            0 => return Arm::Wild,
            1..=3 => return Arm::Lit(pick(r, t)),
            4..=6 => {
                let (a, b) = (pick(r, t), pick(r, t));
                return Arm::Incl(a.min(b), a.max(b));
            }
            7 | 8 => {
                let (a, b) = (pick(r, t), pick(r, t));
                if a != b {
                    return Arm::Excl(a.min(b), a.max(b));
                }
            }
            9 => return Arm::From(pick(r, t)),
            _ => {
                let b = pick(r, t);
                if b > 0 && b <= max {
                    return Arm::Upto(b);
                }
            }
        }
    }
}

/// PHÂN HOẠCH miền thành các khoảng liên tiếp (viết bằng kiểu khoảng ngẫu nhiên), rồi có thể làm lệch/bỏ/chồng.
fn gen_partition(r: &mut Rng, t: Ty) -> Vec<Arm> {
    let max = t.max_off();
    let k = 1 + r.below(3) as usize;
    let mut cuts: Vec<u128> = (0..k).map(|_| pick(r, t)).filter(|&c| c > 0).collect();
    cuts.sort_unstable();
    cuts.dedup();
    let mut arms = Vec::new();
    let mut lo = 0u128;
    for &c in &cuts {
        if c > lo {
            arms.push(if lo == 0 && r.below(2) == 0 {
                Arm::Upto(c)
            } else if r.below(3) == 0 {
                Arm::Excl(lo, c)
            } else {
                Arm::Incl(lo, c - 1)
            });
            lo = c;
        }
    }
    arms.push(if r.below(2) == 0 {
        Arm::From(lo)
    } else {
        Arm::Incl(lo, max)
    });
    // char: dùng đầu mút hợp lệ (đổi đầu mút nằm trong khe sang biên khe)
    if t.is_char {
        for a in &mut arms {
            *a = fix_char(*a);
        }
    }
    // làm lệch
    match r.below(5) {
        0 if arms.len() > 1 => {
            arms.remove(r.below(arms.len() as u64) as usize);
        }
        1 => {
            let i = r.below(arms.len() as u64) as usize;
            if let Arm::Incl(a, b) = arms[i]
                && b > a
                && t.valid(b - 1)
            {
                arms[i] = Arm::Incl(a, b - 1);
            }
        }
        2 => arms.push(gen_arm(r, t)), // chồng thêm một nhánh ngẫu nhiên
        _ => {}
    }
    arms
}

/// đầu mút nằm trong khe char ⇒ kéo ra biên hợp lệ gần nhất
fn fix_char(a: Arm) -> Arm {
    let f = |v: u128, up: bool| {
        if (GAP_LO..=GAP_HI).contains(&v) {
            if up { GAP_HI + 1 } else { GAP_LO - 1 }
        } else {
            v
        }
    };
    match a {
        Arm::Wild => a,
        Arm::Lit(x) => Arm::Lit(f(x, false)),
        Arm::Incl(x, y) => Arm::Incl(f(x, true), f(y, false)),
        Arm::Excl(x, y) => Arm::Excl(f(x, true), f(y, true)),
        Arm::From(x) => Arm::From(f(x, true)),
        Arm::Upto(y) => Arm::Upto(f(y, true)),
    }
}

/// hợp các khoảng có phủ trọn mọi đoạn của miền không (oracle độc lập, không tràn)
fn covers_domain(arms: &[Arm], t: Ty) -> bool {
    let max = t.max_off();
    let mut iv: Vec<(u128, u128)> = arms.iter().filter_map(|a| a.interval(max)).collect();
    iv.sort_unstable();
    let mut merged: Vec<(u128, u128)> = Vec::new();
    for (lo, hi) in iv {
        match merged.last_mut() {
            Some(last) if lo <= last.1.saturating_add(1) => last.1 = last.1.max(hi),
            _ => merged.push((lo, hi)),
        }
    }
    t.domain()
        .iter()
        .all(|&(dl, dh)| merged.iter().any(|&(l, h)| l <= dl && dh <= h))
}

#[derive(Default, Clone, Copy)]
struct Stat {
    programs: usize,
    oracle_exhaustive: usize,
    accepted: usize,
    conservative: usize,
}

fn run_one(r: &mut Rng, t: Ty, st: &mut Stat) {
    let arms: Vec<Arm> = if r.below(2) == 0 {
        gen_partition(r, t)
    } else {
        (0..1 + r.below(4)).map(|_| gen_arm(r, t)).collect()
    };
    let arms = if t.is_char {
        arms.into_iter().map(fix_char).collect()
    } else {
        arms
    };
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
    assert!(
        codes.iter().all(|c| c == "E0305"),
        "mã lạ {codes:?} cho {src}"
    );
    let accepted = codes.is_empty();
    let exhaustive = covers_domain(&arms, t);
    st.programs += 1;
    st.oracle_exhaustive += usize::from(exhaustive);
    st.accepted += usize::from(accepted);
    if t.name == "u128" {
        assert!(
            !accepted || exhaustive,
            "u128: gt-types CHẤP NHẬN nhưng oracle thấy khe hở: {src}"
        );
        st.conservative += usize::from(exhaustive && !accepted);
    } else {
        assert_eq!(
            accepted,
            exhaustive,
            "{}: gt-types {} nhưng oracle {}: {src}",
            t.name,
            if accepted { "chấp nhận" } else { "E0305" },
            if exhaustive {
                "vét cạn"
            } else {
                "có khe hở"
            }
        );
    }
}

#[test]
fn integer_and_char_match_exhaustiveness_agrees_with_an_independent_oracle() {
    let n: usize = std::env::var("GT_MATCH_PROGRAMS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(N_DEFAULT);
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let mut stats = [Stat::default(); 13];
    for i in 0..n {
        let k = i % TYS.len();
        run_one(&mut r, TYS[k], &mut stats[k]);
    }
    println!("match_oracle: n={n}");
    println!("  kiểu    chương trình  vét cạn(oracle)  chấp nhận(gt-types)  bảo thủ(u128)");
    for (t, s) in TYS.iter().zip(&stats) {
        println!(
            "  {:<7} {:>10} {:>14} {:>18} {:>14}",
            t.name, s.programs, s.oracle_exhaustive, s.accepted, s.conservative
        );
    }
    for (t, s) in TYS.iter().zip(&stats) {
        assert!(
            s.oracle_exhaustive > 0 && s.oracle_exhaustive < s.programs,
            "{}: bộ sinh phải ra cả ca vét cạn lẫn có khe hở ({} / {})",
            t.name,
            s.oracle_exhaustive,
            s.programs
        );
    }
}
