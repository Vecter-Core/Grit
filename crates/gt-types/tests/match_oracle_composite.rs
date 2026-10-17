//! ORACLE cho vét cạn `⌥` trên kiểu GHÉP (lệnh Giang 2026-10-17 mục 4a): `bool`, `u8`, `Option<T>` lồng, tuple, enum người dùng (`E { A, B(bool), C(u8) }`) và struct (`S { x: bool, y: u8 }`),
//! độ sâu ≤ 2. Oracle ĐỘC LẬP: LIỆT KÊ toàn bộ giá trị của kiểu và xem có nhánh nào khớp mỗi giá trị; gt-types chấp nhận (không E0305) ⇔ mọi giá trị được khớp (HAI CHIỀU, chính xác vì miền hữu hạn).
//! Nhánh được sinh theo "họ phủ" (None+Some(...), A|B(..)|C(..), tích chéo của tuple/struct) rồi làm lệch (bỏ một nhánh, thêm nhánh ngẫu nhiên) nên cả vét cạn lẫn gần-vét-cạn đều xuất hiện.
mod common;

#[derive(Clone, Debug, PartialEq)]
enum Ty {
    Bool,
    U8,
    Opt(Box<Ty>),
    Tup(Box<Ty>, Box<Ty>),
    E,
    S,
}

#[derive(Clone, Debug)]
enum Val {
    B(bool),
    N(u8),
    None,
    Some(Box<Val>),
    Tup(Box<Val>, Box<Val>),
    EA,
    EB(bool),
    EC(u8),
    S(bool, u8),
}

#[derive(Clone, Debug)]
enum Pat {
    Wild,
    Bool(bool),
    Range(u8, u8),
    Lit(u8),
    NoneP,
    SomeP(Box<Pat>),
    TupP(Box<Pat>, Box<Pat>),
    VarA,
    VarB(Box<Pat>),
    VarC(Box<Pat>),
    StructP(Box<Pat>, Box<Pat>),
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

fn ty_text(t: &Ty) -> String {
    match t {
        Ty::Bool => "bool".into(),
        Ty::U8 => "u8".into(),
        Ty::Opt(x) => format!("Option<{}>", ty_text(x)),
        Ty::Tup(a, b) => format!("({}, {})", ty_text(a), ty_text(b)),
        Ty::E => "E".into(),
        Ty::S => "S".into(),
    }
}

fn size(t: &Ty) -> u64 {
    match t {
        Ty::Bool => 2,
        Ty::U8 => 256,
        Ty::Opt(x) => 1 + size(x),
        Ty::Tup(a, b) => size(a) * size(b),
        Ty::E => 1 + 2 + 256,
        Ty::S => 2 * 256,
    }
}

fn values(t: &Ty) -> Vec<Val> {
    match t {
        Ty::Bool => vec![Val::B(false), Val::B(true)],
        Ty::U8 => (0..=255u8).map(Val::N).collect(),
        Ty::Opt(x) => {
            let mut v = vec![Val::None];
            v.extend(values(x).into_iter().map(|y| Val::Some(Box::new(y))));
            v
        }
        Ty::Tup(a, b) => {
            let (va, vb) = (values(a), values(b));
            let mut out = Vec::with_capacity(va.len() * vb.len());
            for x in &va {
                for y in &vb {
                    out.push(Val::Tup(Box::new(x.clone()), Box::new(y.clone())));
                }
            }
            out
        }
        Ty::E => {
            let mut v = vec![Val::EA, Val::EB(false), Val::EB(true)];
            v.extend((0..=255u8).map(Val::EC));
            v
        }
        Ty::S => {
            let mut v = Vec::new();
            for b in [false, true] {
                for n in 0..=255u8 {
                    v.push(Val::S(b, n));
                }
            }
            v
        }
    }
}

fn matches(p: &Pat, v: &Val) -> bool {
    match (p, v) {
        (Pat::Wild, _) => true,
        (Pat::Bool(a), Val::B(b)) | (Pat::Bool(a), Val::EB(b)) => a == b,
        (Pat::Range(lo, hi), Val::N(n)) | (Pat::Range(lo, hi), Val::EC(n)) => lo <= n && n <= hi,
        (Pat::Lit(a), Val::N(n)) | (Pat::Lit(a), Val::EC(n)) => a == n,
        (Pat::NoneP, Val::None) => true,
        (Pat::SomeP(q), Val::Some(x)) => matches(q, x),
        (Pat::TupP(p1, p2), Val::Tup(a, b)) => matches(p1, a) && matches(p2, b),
        (Pat::VarA, Val::EA) => true,
        (Pat::VarB(q), Val::EB(b)) => matches(q, &Val::B(*b)),
        (Pat::VarC(q), Val::EC(n)) => matches(q, &Val::N(*n)),
        (Pat::StructP(p1, p2), Val::S(b, n)) => {
            matches(p1, &Val::B(*b)) && matches(p2, &Val::N(*n))
        }
        _ => false,
    }
}

fn pat_text(p: &Pat) -> String {
    match p {
        Pat::Wild => "_".into(),
        Pat::Bool(b) => b.to_string(),
        Pat::Range(lo, hi) => format!("{lo}..={hi}"),
        Pat::Lit(a) => a.to_string(),
        Pat::NoneP => "None".into(),
        Pat::SomeP(q) => format!("Some({})", pat_text(q)),
        Pat::TupP(a, b) => format!("({}, {})", pat_text(a), pat_text(b)),
        Pat::VarA => "E::A".into(),
        Pat::VarB(q) => format!("E::B({})", pat_text(q)),
        Pat::VarC(q) => format!("E::C({})", pat_text(q)),
        Pat::StructP(a, b) => format!("S {{ x: {}, y: {} }}", pat_text(a), pat_text(b)),
    }
}

fn gen_ty(r: &mut Rng, depth: u32) -> Ty {
    loop {
        let t = match r.below(if depth == 0 { 4 } else { 8 }) {
            0 => Ty::Bool,
            1 => Ty::U8,
            2 => Ty::E,
            3 => Ty::S,
            4 | 5 => Ty::Opt(Box::new(gen_ty(r, depth - 1))),
            _ => Ty::Tup(
                Box::new(gen_ty(r, depth - 1)),
                Box::new(gen_ty(r, depth - 1)),
            ),
        };
        if size(&t) <= 70_000 {
            return t;
        }
    }
}

/// một nhánh ngẫu nhiên đúng kiểu
fn rand_pat(r: &mut Rng, t: &Ty) -> Pat {
    if r.below(6) == 0 {
        return Pat::Wild;
    }
    match t {
        Ty::Bool => Pat::Bool(r.below(2) == 0),
        Ty::U8 => {
            let (a, b) = (r.below(256) as u8, r.below(256) as u8);
            if r.below(3) == 0 {
                Pat::Lit(a)
            } else {
                Pat::Range(a.min(b), a.max(b))
            }
        }
        Ty::Opt(x) => {
            if r.below(3) == 0 {
                Pat::NoneP
            } else {
                Pat::SomeP(Box::new(rand_pat(r, x)))
            }
        }
        Ty::Tup(a, b) => Pat::TupP(Box::new(rand_pat(r, a)), Box::new(rand_pat(r, b))),
        Ty::E => match r.below(3) {
            0 => Pat::VarA,
            1 => Pat::VarB(Box::new(rand_pat(r, &Ty::Bool))),
            _ => Pat::VarC(Box::new(rand_pat(r, &Ty::U8))),
        },
        Ty::S => Pat::StructP(
            Box::new(rand_pat(r, &Ty::Bool)),
            Box::new(rand_pat(r, &Ty::U8)),
        ),
    }
}

/// HỌ PHỦ: tập nhánh phủ trọn kiểu `t` (có thể dùng `_`), kích thước nhỏ
fn cover(r: &mut Rng, t: &Ty) -> Vec<Pat> {
    if r.below(5) == 0 {
        return vec![Pat::Wild];
    }
    match t {
        Ty::Bool => vec![Pat::Bool(true), Pat::Bool(false)],
        Ty::U8 => {
            let mut cuts: Vec<u16> = (0..1 + r.below(3))
                .map(|_| 1 + r.below(255) as u16)
                .collect();
            cuts.sort_unstable();
            cuts.dedup();
            let mut v = Vec::new();
            let mut lo = 0u16;
            for c in cuts {
                v.push(Pat::Range(lo as u8, (c - 1) as u8));
                lo = c;
            }
            v.push(Pat::Range(lo as u8, 255));
            v
        }
        Ty::Opt(x) => {
            let mut v = vec![Pat::NoneP];
            v.extend(cover(r, x).into_iter().map(|p| Pat::SomeP(Box::new(p))));
            v
        }
        Ty::Tup(a, b) => {
            let (ca, cb) = (cover(r, a), cover(r, b));
            let mut v = Vec::new();
            for x in ca.iter().take(3) {
                for y in &cb {
                    v.push(Pat::TupP(Box::new(x.clone()), Box::new(y.clone())));
                }
            }
            // phần còn lại của ca (take(3) có thể cắt bớt): thêm bằng Wild ở cột sau để vẫn phủ
            for x in ca.iter().skip(3) {
                v.push(Pat::TupP(Box::new(x.clone()), Box::new(Pat::Wild)));
            }
            v
        }
        Ty::E => {
            let mut v = vec![Pat::VarA];
            v.extend(
                cover(r, &Ty::Bool)
                    .into_iter()
                    .map(|p| Pat::VarB(Box::new(p))),
            );
            v.extend(
                cover(r, &Ty::U8)
                    .into_iter()
                    .map(|p| Pat::VarC(Box::new(p))),
            );
            v
        }
        Ty::S => {
            let (cb, cn) = (cover(r, &Ty::Bool), cover(r, &Ty::U8));
            let mut v = Vec::new();
            for x in &cb {
                for y in &cn {
                    v.push(Pat::StructP(Box::new(x.clone()), Box::new(y.clone())));
                }
            }
            v
        }
    }
}

#[derive(Default)]
struct Stat {
    programs: usize,
    exhaustive: usize,
}

#[test]
fn composite_match_exhaustiveness_agrees_with_enumeration() {
    let n: usize = std::env::var("GT_MATCH_COMPOSITE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(260);
    let mut r = Rng(0xD1B5_4A32_D192_ED03);
    let (mut total, mut exh) = (0usize, 0usize);
    let mut by_shape: std::collections::BTreeMap<String, Stat> = Default::default();
    for _ in 0..n {
        let t = gen_ty(&mut r, 2);
        let mut rows = cover(&mut r, &t);
        match r.below(5) {
            0 if rows.len() > 1 => {
                rows.remove(r.below(rows.len() as u64) as usize);
            }
            1 => {
                let extra = rand_pat(&mut r, &t);
                rows.push(extra);
            }
            2 => rows = (0..1 + r.below(4)).map(|_| rand_pat(&mut r, &t)).collect(),
            _ => {}
        }
        rows.truncate(10);
        let body: Vec<String> = rows
            .iter()
            .enumerate()
            .map(|(i, p)| format!("{} ↦ {}u32", pat_text(p), i + 1))
            .collect();
        let src = format!(
            "⊔ E {{ A, B(bool), C(u8) }}\n▢ S {{ x: bool, y: u8 }}\nƒ f(v: {}) → u32 ! ∅ {{ ⌥ v {{ {} }} }}",
            ty_text(&t),
            body.join(", ")
        );
        let out = common::one(&src);
        let codes = common::codes(&out);
        assert!(codes.iter().all(|c| c == "E0305"), "mã lạ {codes:?}\n{src}");
        let accepted = codes.is_empty();
        let vals = values(&t);
        let exhaustive = vals.iter().all(|v| rows.iter().any(|p| matches(p, v)));
        assert_eq!(
            accepted,
            exhaustive,
            "gt-types {} nhưng liệt kê {}:\n{src}",
            if accepted { "chấp nhận" } else { "E0305" },
            if exhaustive {
                "vét cạn"
            } else {
                "có khe hở"
            }
        );
        total += 1;
        exh += usize::from(exhaustive);
        let shape = match &t {
            Ty::Bool => "bool",
            Ty::U8 => "u8",
            Ty::Opt(_) => "Option<..>",
            Ty::Tup(..) => "tuple",
            Ty::E => "enum",
            Ty::S => "struct",
        };
        let st = by_shape.entry(shape.to_string()).or_default();
        st.programs += 1;
        st.exhaustive += usize::from(exhaustive);
    }
    println!("match_oracle_composite: n={total}, vét cạn={exh}");
    for (k, s) in &by_shape {
        println!(
            "  {k:<12} {:>4} chương trình, {:>4} vét cạn",
            s.programs, s.exhaustive
        );
    }
    assert!(
        exh > 0 && exh < total,
        "bộ sinh phải ra cả hai loại ({exh}/{total})"
    );
}
