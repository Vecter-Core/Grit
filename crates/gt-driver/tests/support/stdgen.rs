//! Bộ sinh chương trình dùng gói `std` (lib/std) với MÔ HÌNH Rust độc lập: mỗi CA (một hàm/impl của std) sinh một hàm Grit `cK() → u64` gồm nhiều lần gọi
//! với đầu vào ngẫu nhiên (thiên về biên) và giá trị kỳ vọng tính bằng chính `Ord`/`Option`/`Result` của Rust (bản tham chiếu độc lập). `main` gộp mọi ca.
//! Chế độ `edge`: chéo TOÀN BỘ tập giá trị biên của từng kiểu (MIN, MIN+1, -1, 0, 1, MAX-1, MAX, bằng nhau, tiền tố, rỗng).
#![allow(dead_code)]
#[path = "../../../gt-mir/tests/support/progen.rs"]
mod progen;
use progen::Rng;
use std::cmp::Ordering;

pub struct Case {
    pub idx: usize,
    pub name: &'static str,
    /// số lần gọi hàm std trong ca này
    pub calls: u64,
    /// số lần gọi dùng giá trị biên (đầu vào biên của kiểu)
    pub edge_calls: u64,
    pub func: String,
    pub model: u64,
}

pub struct Prog {
    pub helpers: String,
    pub funcs: Vec<(&'static str, String, u64, bool)>,
    pub src: String,
    pub expect: u64,
    pub cases: Vec<(&'static str, u64, u64)>,
}

fn fold(acc: u64, v: u64) -> u64 {
    acc.wrapping_mul(31).wrapping_add(v)
}

fn rk(o: Ordering) -> u64 {
    match o {
        Ordering::Less => 1,
        Ordering::Equal => 2,
        Ordering::Greater => 3,
    }
}

struct B {
    body: String,
    acc: u64,
    calls: u64,
    edge_calls: u64,
}

impl B {
    fn new() -> B {
        B {
            body: String::new(),
            acc: 0,
            calls: 0,
            edge_calls: 0,
        }
    }
    /// thêm `acc ← acc *% 31 +% <expr>` với giá trị kỳ vọng `v`; `edge` = đầu vào là giá trị biên
    fn push(&mut self, expr: &str, v: u64, edge: bool) {
        self.body
            .push_str(&format!("  acc ← acc *% 31u64 +% ({expr});\n"));
        self.acc = fold(self.acc, v);
        self.calls += 1;
        if edge {
            self.edge_calls += 1;
        }
    }
    fn finish(self, name: &'static str, idx: usize, mem: bool) -> Case {
        let eff = if mem { "{mem}" } else { "∅" };
        let arg = if mem { "alloc: &Cap<Alloc>" } else { "" };
        Case {
            idx,
            name,
            calls: self.calls,
            edge_calls: self.edge_calls,
            func: format!(
                "ƒ c{idx}({arg}) → u64 ! {eff} {{\n  μacc ≔ 0u64;\n{}  acc\n}}\n",
                self.body
            ),
            model: self.acc,
        }
    }
}

struct Ity {
    name: &'static str,
    min: i128,
    max: i128,
}

const ITYS: [Ity; 10] = [
    Ity {
        name: "u8",
        min: 0,
        max: u8::MAX as i128,
    },
    Ity {
        name: "u16",
        min: 0,
        max: u16::MAX as i128,
    },
    Ity {
        name: "u32",
        min: 0,
        max: u32::MAX as i128,
    },
    Ity {
        name: "u64",
        min: 0,
        max: u64::MAX as i128,
    },
    Ity {
        name: "usize",
        min: 0,
        max: u64::MAX as i128,
    },
    Ity {
        name: "i8",
        min: i8::MIN as i128,
        max: i8::MAX as i128,
    },
    Ity {
        name: "i16",
        min: i16::MIN as i128,
        max: i16::MAX as i128,
    },
    Ity {
        name: "i32",
        min: i32::MIN as i128,
        max: i32::MAX as i128,
    },
    Ity {
        name: "i64",
        min: i64::MIN as i128,
        max: i64::MAX as i128,
    },
    Ity {
        name: "isize",
        min: i64::MIN as i128,
        max: i64::MAX as i128,
    },
];

fn lit(t: &Ity, v: i128) -> String {
    if v < 0 {
        if v == t.min {
            format!("(-{}{} -% 1{})", t.max, t.name, t.name)
        } else {
            format!("(-{}{})", -v, t.name)
        }
    } else {
        format!("{v}{}", t.name)
    }
}

fn edges(t: &Ity) -> Vec<i128> {
    let mut e = vec![t.min, t.min + 1, 0, 1, t.max - 1, t.max];
    if t.min < 0 {
        e.push(-1);
    }
    e.sort();
    e.dedup();
    e
}

fn pick(rng: &mut Rng, t: &Ity) -> (i128, bool) {
    let e = edges(t);
    if rng.chance(60) {
        (e[rng.below(e.len() as u64) as usize], true)
    } else {
        let span = (t.max - t.min).min(1 << 62) as u64 + 1;
        let off = rng.below(span) as i128;
        // phần lớn mẫu ngẫu nhiên gần 0 để a == b xảy ra với kiểu nhỏ
        (
            if rng.chance(50) {
                t.min + off
            } else {
                off.min(t.max).max(t.min)
            },
            false,
        )
    }
}

const CHARS: [char; 10] = ['a', 'b', 'Z', '0', ' ', '~', '!', 'é', 'λ', '中'];
const STRS: [&str; 8] = ["", "a", "ab", "b", "aé", "aa", "abc", "é"];

fn clit(c: char) -> String {
    format!("'{c}'")
}

pub const HELPERS: &str = "ƒ rk(o: Ordering) → u64 ! ∅ { ⌥ o { Ordering::Less ↦ 1u64, Ordering::Equal ↦ 2u64, Ordering::Greater ↦ 3u64 } }
ƒ bn(b: bool) → u64 ! ∅ { ⎇ b { 1u64 } ⇝ { 0u64 } }
ƒ le(o: Ordering) → bool ! ∅ { ⌥ o { Ordering::Greater ↦ false, _ ↦ true } }
ƒ imp3(ab: bool, bc: bool, ac: bool) → bool ! ∅ { ⎇ ab { ⎇ bc { ac } ⇝ { true } } ⇝ { true } }
ƒ ou(k: u32, present: bool) → Option<u32> ! ∅ { ⎇ present { Some(k) } ⇝ { None } }
ƒ oo(k: u8, lvl: u8) → Option<Option<u8>> ! ∅ { ⎇ lvl == 0u8 { None } ⇝ { ⎇ lvl == 1u8 { Some(None) } ⇝ { Some(Some(k)) } } }
ƒ rv(k: u32, e: u8, ok: bool) → Result<u32, u8> ! ∅ { ⎇ ok { Ok(k) } ⇝ { Err(e) } }
ƒ rs(k: u8, e: i16, ok: bool) → Result<u8, i16> ! ∅ { ⎇ ok { Ok(k) } ⇝ { Err(e) } }
ƒ dbl(x: u32) → u32 ! ∅ { x *% 2u32 }
ƒ half(x: u32) → Option<u32> ! ∅ { ⎇ x % 2u32 == 0u32 { Some(x / 2u32) } ⇝ { None } }
ƒ rhalf(x: u32) → Result<u32, u8> ! ∅ { ⎇ x % 2u32 == 0u32 { Ok(x / 2u32) } ⇝ { Err(7u8) } }
ƒ tri(e: u8) → u16 ! ∅ { (e as u16) *% 3u16 }
ƒ on(o: Option<u32>) → u64 ! ∅ { ⌥ o { Some(v) ↦ (v as u64) + 1u64, None ↦ 0u64 } }
ƒ er(r: Result<u32, u8>) → u64 ! ∅ { ⌥ r { Ok(v) ↦ (v as u64) * 512u64 + 1u64, Err(e) ↦ (e as u64) * 2u64 } }
ƒ er16(r: Result<u32, u16>) → u64 ! ∅ { ⌥ r { Ok(v) ↦ (v as u64) * 131072u64 + 1u64, Err(e) ↦ (e as u64) * 2u64 } }
";

// ---- mô hình của các helper
fn ou(k: u32, p: bool) -> Option<u32> {
    if p { Some(k) } else { None }
}
fn oo(k: u8, lvl: u8) -> Option<Option<u8>> {
    match lvl {
        0 => None,
        1 => Some(None),
        _ => Some(Some(k)),
    }
}
fn rv(k: u32, e: u8, ok: bool) -> Result<u32, u8> {
    if ok { Ok(k) } else { Err(e) }
}
fn rs(k: u8, e: i16, ok: bool) -> Result<u8, i16> {
    if ok { Ok(k) } else { Err(e) }
}
fn on(o: Option<u32>) -> u64 {
    o.map_or(0, |v| v as u64 + 1)
}
fn er(r: Result<u32, u8>) -> u64 {
    match r {
        Ok(v) => v as u64 * 512 + 1,
        Err(e) => e as u64 * 2,
    }
}
fn er16(r: Result<u32, u16>) -> u64 {
    match r {
        Ok(v) => v as u64 * 131072 + 1,
        Err(e) => e as u64 * 2,
    }
}

type OptFn = Box<dyn Fn(u32, bool, u32, bool) -> (String, u64)>;
type ResFn = Box<dyn Fn(u32, u8, bool) -> (String, u64)>;

const REPS: usize = 8;

/// `calls` mỗi ca = REPS (ngẫu nhiên) hoặc toàn bộ chéo (edge)
/// `mem_only`: chỉ các ca dùng Arena (Text): thông dịch KHÔNG mô phỏng Arena/root_alloc nên chương trình này chỉ so mô hình ≡ exe (2 nguồn)
pub fn program(seed: u64, edge: bool, mem_only: bool) -> Prog {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03 | 1);
    let mut cases: Vec<Case> = Vec::new();
    // -------- Ord cho số nguyên
    for t in &ITYS {
        let mut b = B::new();
        if edge {
            let e = edges(t);
            for &x in &e {
                for &y in &e {
                    b.push(
                        &format!("rk({}.cmp(&{}))", lit(t, x), lit(t, y)),
                        rk(x.cmp(&y)),
                        true,
                    );
                }
            }
        } else {
            for _ in 0..REPS {
                let (x, ex) = pick(&mut rng, t);
                let (y, ey) = pick(&mut rng, t);
                let (y, ey) = if rng.chance(15) { (x, ex) } else { (y, ey) };
                b.push(
                    &format!("rk({}.cmp(&{}))", lit(t, x), lit(t, y)),
                    rk(x.cmp(&y)),
                    ex || ey,
                );
            }
        }
        cases.push(b.finish(
            Box::leak(format!("Ord for {}", t.name).into_boxed_str()),
            cases.len(),
            false,
        ));
    }
    // -------- char
    {
        let mut b = B::new();
        let n = if edge {
            CHARS.len() * CHARS.len()
        } else {
            REPS
        };
        for k in 0..n {
            let (x, y) = if edge {
                (CHARS[k / CHARS.len()], CHARS[k % CHARS.len()])
            } else {
                (
                    CHARS[rng.below(CHARS.len() as u64) as usize],
                    CHARS[rng.below(CHARS.len() as u64) as usize],
                )
            };
            b.push(
                &format!("rk({}.cmp(&{}))", clit(x), clit(y)),
                rk(x.cmp(&y)),
                true,
            );
        }
        cases.push(b.finish("Ord for char", cases.len(), false));
    }
    // -------- bool
    {
        let mut b = B::new();
        for x in [false, true] {
            for y in [false, true] {
                b.push(&format!("rk({x}.cmp(&{y}))"), rk(x.cmp(&y)), true);
            }
        }
        cases.push(b.finish("Ord for bool", cases.len(), false));
    }
    // -------- Ordering
    {
        let mut b = B::new();
        let os = [Ordering::Less, Ordering::Equal, Ordering::Greater];
        let name = |o: Ordering| match o {
            Ordering::Less => "Ordering::Less",
            Ordering::Equal => "Ordering::Equal",
            Ordering::Greater => "Ordering::Greater",
        };
        for x in os {
            for y in os {
                b.push(
                    &format!("rk({}.cmp(&{}))", name(x), name(y)),
                    rk(x.cmp(&y)),
                    true,
                );
            }
        }
        cases.push(b.finish("Ord for Ordering", cases.len(), false));
        for (mname, f) in [
            ("Ordering::reverse", 0),
            ("Ordering::is_lt", 1),
            ("Ordering::is_eq", 2),
            ("Ordering::is_gt", 3),
        ] {
            let mut b = B::new();
            for x in os {
                let (e, v) = match f {
                    0 => (format!("rk({}.reverse())", name(x)), rk(x.reverse())),
                    1 => (format!("bn({}.is_lt())", name(x)), x.is_lt() as u64),
                    2 => (format!("bn({}.is_eq())", name(x)), x.is_eq() as u64),
                    _ => (format!("bn({}.is_gt())", name(x)), x.is_gt() as u64),
                };
                b.push(&e, v, true);
            }
            cases.push(b.finish(mname, cases.len(), false));
        }
        let mut b = B::new();
        for x in os {
            for y in os {
                b.push(
                    &format!("rk({}.then({}))", name(x), name(y)),
                    rk(x.then(y)),
                    true,
                );
            }
        }
        cases.push(b.finish("Ordering::then", cases.len(), false));
    }
    // -------- Option<u32> (Ord và phương thức)
    let ovals = |rng: &mut Rng| -> (u32, bool) { (rng.below(6) as u32, rng.chance(35)) };
    {
        let mut b = B::new();
        let n = if edge { 36 } else { REPS };
        for k in 0..n {
            let (a, pa, c, pc) = if edge {
                (
                    (k / 6) as u32 % 3,
                    k / 6 >= 3,
                    (k % 6) as u32 % 3,
                    k % 6 >= 3,
                )
            } else {
                let (a, pa) = ovals(&mut rng);
                let (c, pc) = ovals(&mut rng);
                (a, !pa, c, !pc)
            };
            let (x, y) = (ou(a, pa), ou(c, pc));
            b.push(
                &format!("rk(ou({a}u32, {pa}).cmp(&ou({c}u32, {pc})))"),
                rk(x.cmp(&y)),
                edge,
            );
        }
        cases.push(b.finish("Ord for Option<T>", cases.len(), false));
    }
    {
        // Option<Option<u8>>: 3 cấp
        let mut b = B::new();
        let n = if edge { 9 * 9 } else { REPS };
        for k in 0..n {
            let (la, lb) = if edge {
                ((k / 9) as u8 % 9, (k % 9) as u8)
            } else {
                (rng.below(9) as u8, rng.below(9) as u8)
            };
            let (ka, kb) = (la / 3, lb / 3);
            let (la, lb) = (la % 3, lb % 3);
            let (x, y) = (oo(ka, la), oo(kb, lb));
            b.push(
                &format!("rk(oo({ka}u8, {la}u8).cmp(&oo({kb}u8, {lb}u8)))"),
                rk(x.cmp(&y)),
                edge,
            );
        }
        cases.push(b.finish("Ord for Option<Option<u8>>", cases.len(), false));
    }
    // -------- Result<u8, i16>
    {
        let mut b = B::new();
        let n = if edge { 4 * 4 } else { REPS };
        for k in 0..n {
            let (ia, ib) = if edge {
                ((k / 4) as u8, (k % 4) as u8)
            } else {
                (rng.below(4) as u8, rng.below(4) as u8)
            };
            let mk = |i: u8| -> (u8, i16, bool) {
                (i / 2, if i / 2 == 0 { -1 } else { 5 }, i.is_multiple_of(2))
            };
            let ((k1, e1, o1), (k2, e2, o2)) = (mk(ia), mk(ib));
            let (x, y) = (rs(k1, e1, o1), rs(k2, e2, o2));
            b.push(
                &format!("rk(rs({k1}u8, ({e1}i16), {o1}).cmp(&rs({k2}u8, ({e2}i16), {o2})))"),
                rk(x.cmp(&y)),
                edge,
            );
        }
        cases.push(b.finish("Ord for Result<T, E>", cases.len(), false));
    }
    // -------- tuple
    {
        let mut b = B::new();
        let t = &ITYS[7];
        for _ in 0..(if edge { 24 } else { REPS }) {
            let (a, _) = pick(&mut rng, &ITYS[0]);
            let (c, _) = pick(&mut rng, &ITYS[0]);
            let (a, c) = if edge { (a % 2, c % 2) } else { (a % 4, c % 4) }; // thành phần đầu hay bằng nhau
            let (x, _) = pick(&mut rng, t);
            let (y, _) = pick(&mut rng, t);
            b.push(
                &format!(
                    "rk(({}, {}).cmp(&({}, {})))",
                    lit(&ITYS[0], a),
                    lit(t, x),
                    lit(&ITYS[0], c),
                    lit(t, y)
                ),
                rk((a, x).cmp(&(c, y))),
                edge,
            );
        }
        cases.push(b.finish("Ord for (A, B)", cases.len(), false));
        let mut b = B::new();
        for _ in 0..(if edge { 24 } else { REPS }) {
            let (a, c) = (CHARS[rng.below(3) as usize], CHARS[rng.below(3) as usize]);
            let (p, q) = (rng.chance(50), rng.chance(50));
            let (u, v) = (rng.below(3) as i128, rng.below(3) as i128);
            b.push(
                &format!(
                    "rk(({}, {}, {p}).cmp(&({}, {}, {q})))",
                    lit(&ITYS[0], u),
                    clit(a),
                    lit(&ITYS[0], v),
                    clit(c)
                ),
                rk((u, a, p).cmp(&(v, c, q))),
                edge,
            );
        }
        cases.push(b.finish("Ord for (A, B, C)", cases.len(), false));
    }
    // -------- lát
    {
        let mut b = B::new();
        for _ in 0..(if edge { 40 } else { REPS }) {
            let (la, lb) = (rng.below(5) as usize, rng.below(5) as usize);
            let mut xs: Vec<u8> = (0..4).map(|_| rng.below(3) as u8).collect();
            let mut ys: Vec<u8> = xs.clone();
            if rng.chance(50) {
                ys[rng.below(4) as usize] = rng.below(3) as u8;
            }
            if rng.chance(20) {
                xs = vec![0, 0, 0, 0];
                ys = vec![0, 0, 0, 0];
            }
            let arr = |v: &[u8]| format!("[{}u8, {}u8, {}u8, {}u8]", v[0], v[1], v[2], v[3]);
            let id = b.calls;
            let body = format!(
                "{{ xa{id} ≔ {}; ya{id} ≔ {}; rk(xa{id}[..{la}usize].cmp(&ya{id}[..{lb}usize])) }}",
                arr(&xs),
                arr(&ys)
            );
            b.push(
                &body,
                rk(xs[..la].cmp(&ys[..lb])),
                edge || la == 0 || lb == 0,
            );
        }
        cases.push(b.finish("Ord for [T] (u8)", cases.len(), false));
        let mut b = B::new();
        for _ in 0..(if edge { 40 } else { REPS }) {
            let (la, lb) = (rng.below(4) as usize, rng.below(4) as usize);
            let xs: Vec<(u32, u8)> = (0..3)
                .map(|_| (rng.below(2) as u32, rng.below(2) as u8))
                .collect();
            let mut ys = xs.clone();
            if rng.chance(50) {
                ys[rng.below(3) as usize] = (rng.below(2) as u32, rng.below(2) as u8);
            }
            let arr = |v: &[(u32, u8)]| {
                format!(
                    "[({}u32, {}u8), ({}u32, {}u8), ({}u32, {}u8)]",
                    v[0].0, v[0].1, v[1].0, v[1].1, v[2].0, v[2].1
                )
            };
            let id = b.calls;
            let body = format!(
                "{{ xa{id} ≔ {}; ya{id} ≔ {}; rk(xa{id}[..{la}usize].cmp(&ya{id}[..{lb}usize])) }}",
                arr(&xs),
                arr(&ys)
            );
            b.push(
                &body,
                rk(xs[..la].cmp(&ys[..lb])),
                edge || la == 0 || lb == 0,
            );
        }
        cases.push(b.finish("Ord for [T] (tuple)", cases.len(), false));
    }
    // -------- Text
    {
        let mut b = B::new();
        let n = if edge { STRS.len() * STRS.len() } else { REPS };
        for k in 0..n {
            let (x, y) = if edge {
                (STRS[k / STRS.len()], STRS[k % STRS.len()])
            } else {
                (STRS[rng.below(8) as usize], STRS[rng.below(8) as usize])
            };
            let id = b.calls;
            let body = format!(
                "{{ μpa{id} ≔ alloc.arena(); μpb{id} ≔ alloc.arena(); μta{id} ≔ Text::new(&!pa{id}); μtb{id} ≔ Text::new(&!pb{id}); ta{id}.push_str(\"{x}\"); tb{id}.push_str(\"{y}\"); rk(ta{id}.cmp(&tb{id})) }}"
            );
            b.push(&body, rk(x.as_bytes().cmp(y.as_bytes())), true);
        }
        cases.push(b.finish("Ord for Text", cases.len(), true));
    }
    // -------- Option: phương thức
    let mut oprops: Vec<(&'static str, OptFn)> = Vec::new();
    oprops.push((
        "Option::is_none",
        Box::new(|a, pa, _, _| {
            (
                format!("bn(ou({a}u32, {pa}).is_none())"),
                ou(a, pa).is_none() as u64,
            )
        }),
    ));
    oprops.push((
        "Option::unwrap_or",
        Box::new(|a, pa, c, _| {
            (
                format!("(ou({a}u32, {pa}).unwrap_or({c}u32)) as u64"),
                ou(a, pa).unwrap_or(c) as u64,
            )
        }),
    ));
    oprops.push((
        "Option::map",
        Box::new(|a, pa, _, _| {
            (
                format!("on(ou({a}u32, {pa}).map(dbl))"),
                on(ou(a, pa).map(|v| v.wrapping_mul(2))),
            )
        }),
    ));
    oprops.push((
        "Option::and_then",
        Box::new(|a, pa, _, _| {
            (
                format!("on(ou({a}u32, {pa}).and_then(half))"),
                on(ou(a, pa).and_then(|v| if v % 2 == 0 { Some(v / 2) } else { None })),
            )
        }),
    ));
    oprops.push((
        "Option::or",
        Box::new(|a, pa, c, pc| {
            (
                format!("on(ou({a}u32, {pa}).or(ou({c}u32, {pc})))"),
                on(ou(a, pa).or(ou(c, pc))),
            )
        }),
    ));
    oprops.push((
        "Option::ok_or",
        Box::new(|a, pa, c, _| {
            (
                format!("er(ou({a}u32, {pa}).ok_or({}u8))", c % 200),
                er(ou(a, pa).ok_or((c % 200) as u8)),
            )
        }),
    ));
    for (name, f) in &oprops {
        let mut b = B::new();
        for k in 0..(if edge { 16 } else { REPS }) {
            let (a, pa, c, pc) = if edge {
                ((k % 4) as u32, k & 1 == 0, ((k / 4) % 4) as u32, k & 2 == 0)
            } else {
                (
                    rng.below(6) as u32,
                    rng.chance(60),
                    rng.below(6) as u32,
                    rng.chance(60),
                )
            };
            let (e, v) = f(a, pa, c, pc);
            b.push(&e, v, true);
        }
        cases.push(b.finish(name, cases.len(), false));
    }
    // -------- Result: phương thức
    let mut rprops: Vec<(&'static str, ResFn)> = Vec::new();
    rprops.push((
        "Result::is_ok",
        Box::new(|k, e, ok| {
            (
                format!("bn(rv({k}u32, {e}u8, {ok}).is_ok())"),
                rv(k, e, ok).is_ok() as u64,
            )
        }),
    ));
    rprops.push((
        "Result::is_err",
        Box::new(|k, e, ok| {
            (
                format!("bn(rv({k}u32, {e}u8, {ok}).is_err())"),
                rv(k, e, ok).is_err() as u64,
            )
        }),
    ));
    rprops.push((
        "Result::unwrap_or",
        Box::new(|k, e, ok| {
            (
                format!("(rv({k}u32, {e}u8, {ok}).unwrap_or(77u32)) as u64"),
                rv(k, e, ok).unwrap_or(77) as u64,
            )
        }),
    ));
    rprops.push((
        "Result::map",
        Box::new(|k, e, ok| {
            (
                format!("er(rv({k}u32, {e}u8, {ok}).map(dbl))"),
                er(rv(k, e, ok).map(|v| v.wrapping_mul(2))),
            )
        }),
    ));
    rprops.push((
        "Result::map_err",
        Box::new(|k, e, ok| {
            (
                format!("er16(rv({k}u32, {e}u8, {ok}).map_err(tri))"),
                er16(rv(k, e, ok).map_err(|x| x as u16 * 3)),
            )
        }),
    ));
    rprops.push((
        "Result::and_then",
        Box::new(|k, e, ok| {
            (
                format!("er(rv({k}u32, {e}u8, {ok}).and_then(rhalf))"),
                er(rv(k, e, ok).and_then(|v| if v % 2 == 0 { Ok(v / 2) } else { Err(7) })),
            )
        }),
    ));
    rprops.push((
        "Result::ok",
        Box::new(|k, e, ok| {
            (
                format!("on(rv({k}u32, {e}u8, {ok}).ok())"),
                on(rv(k, e, ok).ok()),
            )
        }),
    ));
    rprops.push((
        "Result::err",
        Box::new(|k, e, ok| {
            (
                format!("on(rv({k}u32, {e}u8, {ok}).err().map(tou32))"),
                rv(k, e, ok).err().map_or(0, |x| x as u64 + 1),
            )
        }),
    ));
    for (name, f) in &rprops {
        let mut b = B::new();
        for k in 0..(if edge { 16 } else { REPS }) {
            let (kk, e, ok) = if edge {
                ((k % 4) as u32, ((k / 4) % 4) as u8 + 1, k & 1 == 0)
            } else {
                (rng.below(6) as u32, rng.below(5) as u8, rng.chance(55))
            };
            let (ex, v) = f(kk, e, ok);
            b.push(&ex, v, true);
        }
        cases.push(b.finish(name, cases.len(), false));
    }
    // -------- min / max
    for (name, is_min) in [("min", true), ("max", false)] {
        let mut b = B::new();
        let f = if is_min { "std::min" } else { "std::max" };
        for t in [&ITYS[2], &ITYS[5]] {
            for _ in 0..(if edge { 6 } else { REPS / 2 }) {
                let (x, ex) = pick(&mut rng, t);
                let (y, ey) = if rng.chance(25) {
                    (x, ex)
                } else {
                    pick(&mut rng, t)
                };
                let v = if is_min { x.min(y) } else { x.max(y) };
                // so qua `cmp` của kiểu (kết quả là giá trị cùng kiểu): đưa về u64 bằng `as` rồi cộng bù để tránh trùng
                b.push(
                    &format!(
                        "(({f}({}, {}) as i64) + 2147483649i64) as u64",
                        lit(t, x),
                        lit(t, y)
                    ),
                    (v as i64 + 2147483649) as u64,
                    ex || ey,
                );
            }
        }
        for _ in 0..(if edge { 6 } else { 3 }) {
            let (x, y) = (CHARS[rng.below(10) as usize], CHARS[rng.below(10) as usize]);
            let v = if is_min { x.min(y) } else { x.max(y) };
            b.push(
                &format!("({f}({}, {}) as u32) as u64", clit(x), clit(y)),
                v as u64,
                true,
            );
        }
        let label: &'static str = if is_min {
            "std::min (u32, i8, char)"
        } else {
            "std::max (u32, i8, char)"
        };
        let _ = name;
        cases.push(b.finish(label, cases.len(), false));
    }
    // -------- tính chất đại số của Ord (phản xạ, đối xứng ngược, bắc cầu, nhất quán với `==` cho kiểu nguyên thuỷ)
    for t in [&ITYS[0], &ITYS[2], &ITYS[5], &ITYS[8]] {
        let mut b = B::new();
        for _ in 0..(if edge { 30 } else { REPS }) {
            let (x, _) = pick(&mut rng, t);
            let (y, _) = pick(&mut rng, t);
            let (z, _) = pick(&mut rng, t);
            let (x, y, z) = (lit(t, x), lit(t, y), lit(t, z));
            b.push(
                &format!(
                    "bn({x}.cmp(&{x}).is_eq()) + bn(rk({x}.cmp(&{y})) + rk({y}.cmp(&{x})) == 4u64) + bn(imp3(le({x}.cmp(&{y})), le({y}.cmp(&{z})), le({x}.cmp(&{z})))) + bn({x}.cmp(&{y}).is_eq() == ({x} == {y}))"
                ),
                4,
                true,
            );
        }
        cases.push(b.finish(
            Box::leak(format!("Ord properties ({})", t.name).into_boxed_str()),
            cases.len(),
            false,
        ));
    }
    cases.retain(|c| c.func.contains("alloc: &Cap<Alloc>") == mem_only);
    // đánh số lại sau khi lọc (tên hàm Grit `cN` theo vị trí mới)
    for (new, c) in cases.iter_mut().enumerate() {
        c.func = c
            .func
            .replacen(&format!("ƒ c{}(", c.idx), &format!("ƒ c{new}("), 1);
        c.idx = new;
    }
    // ---- gộp
    let mut src = String::from(HELPERS);
    src.push_str("ƒ tou32(x: u8) → u32 ! ∅ { x as u32 }\n");
    for c in &cases {
        src.push_str(&c.func);
    }
    if mem_only {
        src.push_str(
            "ƒ main() → u64 ! {mem} {
  alloc ≔ root_alloc();
  μacc ≔ 0u64;
",
        );
    } else {
        src.push_str(
            "ƒ main() → u64 ! ∅ {
  μacc ≔ 0u64;
",
        );
    }
    let mut total = 0u64;
    for (i, c) in cases.iter().enumerate() {
        let call = if c.func.contains("alloc: &Cap<Alloc>") {
            format!("c{i}(&alloc)")
        } else {
            format!("c{i}()")
        };
        src.push_str(&format!("  acc ← acc *% 1000003u64 +% {call};\n"));
        total = total.wrapping_mul(1000003).wrapping_add(c.model);
    }
    src.push_str("  acc\n}\n");
    Prog {
        helpers: format!(
            "{HELPERS}ƒ tou32(x: u8) → u32 ! ∅ {{ x as u32 }}
"
        ),
        funcs: cases
            .iter()
            .map(|c| {
                (
                    c.name,
                    c.func.clone(),
                    c.model,
                    c.func.contains("alloc: &Cap<Alloc>"),
                )
            })
            .collect(),
        src,
        expect: total,
        cases: cases
            .iter()
            .map(|c| (c.name, c.calls, c.edge_calls))
            .collect(),
    }
}
