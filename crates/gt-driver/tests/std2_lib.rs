//! Bộ kiểm của phần std MỚI (Seed 1.0 S2: Vec API, byte/số, sắp xếp, băm, HashMap, in số): mỗi hàm được gọi từ chương trình sinh ngẫu nhiên QUA CÙNG CỔNG gritc như mã người dùng
//! rồi so BA NGUỒN: mô hình Rust độc lập ≡ thông dịch MIR ≡ exe LLVM. Ca dùng Arena (Vec/HashMap) chỉ so mô hình ≡ exe (thông dịch không mô phỏng Arena).
//! 32 chương trình ngẫu nhiên + 3 chương trình chéo biên (`edge`) cho mỗi nhóm; mỗi hàm phải xuất hiện trong ≥ 30 chương trình.
//! Hồ sơ: `GT_PROFILE=dev|release|release-o2`. Kết quả: `STD2_OUT=<đường dẫn tuyệt đối .jsonl>` ghi nối một dòng bảng.
#![cfg(feature = "llvm")]
#[path = "../../gt-mir/tests/support/progen.rs"]
mod progen;

use gt_driver::native::{build_exe_profile, run_exe_timeout};
use gt_driver::pipeline::{attestation, mir_dump};
use gt_driver::runner::run_main;
use gt_driver::{Options, RunEnd, check};
use gt_resolve::SourceInput;
use progen::Rng;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::Write;
use std::time::Duration;

const STD: &str = include_str!("../../../lib/std/src/std.gt");
const STD_ORD: &str = include_str!("../../../lib/std/src/std_ord.gt");
const STD_ORD_INT: &str = include_str!("../../../lib/std/src/std_ord_int.gt");

fn profile() -> (gt_codegen_llvm::Opt, &'static str) {
    match std::env::var("GT_PROFILE").as_deref() {
        Ok("release") => (gt_codegen_llvm::Opt::Release, "release"),
        Ok("release-o2") => (gt_codegen_llvm::Opt::ReleaseO2, "release-o2"),
        _ => (gt_codegen_llvm::Opt::Dev, "dev"),
    }
}

fn inputs(main: &str) -> Vec<SourceInput> {
    [
        ("std.gt", STD),
        ("std_ord.gt", STD_ORD),
        ("std_ord_int.gt", STD_ORD_INT),
        ("main.gt", main),
    ]
    .iter()
    .map(|(n, t)| SourceInput {
        name: (*n).to_string(),
        text: (*t).to_string(),
    })
    .collect()
}

fn norm(v: &Option<String>) -> Option<String> {
    let s = v.as_deref()?;
    if let Some(inner) = s.strip_prefix("Int(") {
        return inner.split(',').next().map(str::to_string);
    }
    Some(s.to_string())
}

/// (giá trị thông dịch, giá trị llvm) hoặc lỗi cổng/chạy
fn run3(
    main: &str,
    tag: &str,
    interp_too: bool,
) -> Result<(Option<String>, Option<String>), String> {
    let mut res = check(&inputs(main), &Options::default()).map_err(|e| e.to_string())?;
    if !res.verdict.allows_output() {
        let d: Vec<String> = res
            .report
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str().starts_with('E'))
            .take(6)
            .map(|d| format!("{} {} @{}", d.code.as_str(), d.message, d.span.start))
            .collect();
        return Err(format!("cổng ĐỎ/ICE: {d:?}"));
    }
    let att = {
        let art = res.open_gate().expect("cổng mở");
        attestation(&res, &mir_dump(art))
    };
    let dir = std::env::temp_dir().join(format!("gt-std2lib-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo thư mục");
    let exe = dir.join(format!("{tag}.exe"));
    let art = res.take_gated().expect("cổng");
    build_exe_profile(&art, &exe, true, &att, profile().0)
        .map_err(|e| format!("backend lỗi (mã {}): {e}", e.exit_code()))?;
    let llvm = run_exe_timeout(&exe, Some(Duration::from_secs(60)));
    let _ = std::fs::remove_file(&exe);
    if !interp_too {
        if llvm.end != RunEnd::Normal {
            return Err(format!("exe kết thúc bất thường: {:?}", llvm.end));
        }
        return Ok((llvm.value.clone(), llvm.value));
    }
    let interp = run_main(art);
    if interp.end != RunEnd::Normal || llvm.end != RunEnd::Normal {
        return Err(format!(
            "kết thúc bất thường: thông dịch {:?}, llvm {:?}",
            interp.end, llvm.end
        ));
    }
    Ok((norm(&interp.value), llvm.value))
}

// ------------------------------------------------------------------ mô hình Rust độc lập
fn fold(acc: u64, v: u64) -> u64 {
    acc.wrapping_mul(31).wrapping_add(v)
}

fn fold_bytes(b: &[u8]) -> u64 {
    let mut a = b.len() as u64;
    for &x in b {
        a = a.wrapping_mul(31).wrapping_add(x as u64 + 1);
    }
    a
}

fn rk(o: Ordering) -> u64 {
    match o {
        Ordering::Less => 1,
        Ordering::Equal => 2,
        Ordering::Greater => 3,
    }
}

fn is_ws(c: u8) -> bool {
    c == 32 || c == 9 || c == 10 || c == 13
}

fn m_find(s: &[u8], p: &[u8]) -> Option<usize> {
    if p.is_empty() {
        return Some(0);
    }
    if p.len() > s.len() {
        return None;
    }
    (0..=s.len() - p.len()).find(|&i| &s[i..i + p.len()] == p)
}

fn m_trim(s: &[u8]) -> &[u8] {
    let a = s.iter().position(|&c| !is_ws(c)).unwrap_or(s.len());
    let e = s.iter().rposition(|&c| !is_ws(c)).map_or(0, |i| i + 1);
    if a < e { &s[a..e] } else { &s[0..0] }
}

/// mã lỗi: Empty 1, Invalid 2, Overflow 3 (khớp thứ tự `ParseError` của std)
fn m_pdig(s: &[u8]) -> Result<u64, u64> {
    if s.is_empty() {
        return Err(1);
    }
    if s.iter().any(|c| !c.is_ascii_digit()) {
        return Err(2);
    }
    let mut v: u128 = 0;
    for &c in s {
        v = v * 10 + (c - b'0') as u128;
        if v > u64::MAX as u128 {
            return Err(3);
        }
    }
    Ok(v as u64)
}

fn m_pu64(s: &[u8]) -> u64 {
    match m_pdig(s) {
        Ok(v) => v,
        Err(k) => 1_000_000_000 + k,
    }
}

fn m_pi64(s: &[u8]) -> u64 {
    let r: Result<i64, u64> = if s.first() == Some(&b'-') {
        m_pdig(&s[1..]).and_then(|m| {
            if m > 1 << 63 {
                Err(3)
            } else if m == 1 << 63 {
                Ok(i64::MIN)
            } else {
                Ok(-(m as i64))
            }
        })
    } else {
        m_pdig(s).and_then(|m| {
            if m > i64::MAX as u64 {
                Err(3)
            } else {
                Ok(m as i64)
            }
        })
    };
    match r {
        Ok(v) => zz(v),
        Err(k) => 1_000_000_000 + k,
    }
}

fn m_hash_bytes(b: &[u8]) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for &x in b {
        h = h.wrapping_mul(1099511628211).wrapping_add(x as u64 + 1);
    }
    h.wrapping_mul(11400714819323198485)
}

fn zz(v: i64) -> u64 {
    if v >= 0 {
        (v as u64).wrapping_mul(2)
    } else {
        (((-(v + 1)) as u64).wrapping_mul(2)).wrapping_add(1)
    }
}

fn oi(o: Option<usize>) -> u64 {
    o.map_or(0, |v| v as u64 + 1)
}

// ------------------------------------------------------------------ sinh đầu vào
fn glit(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '\t' => o.push_str("\\t"),
            '\n' => o.push_str("\\n"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

const NUM_EDGE: [&str; 22] = [
    "",
    "0",
    "-",
    "-0",
    "1",
    "-1",
    "007",
    "18446744073709551615",
    "18446744073709551616",
    "9223372036854775807",
    "9223372036854775808",
    "-9223372036854775808",
    "-9223372036854775809",
    "99999999999999999999",
    "12a",
    "a12",
    " 1",
    "+1",
    "1 ",
    "--1",
    "1-",
    "4294967296",
];

fn numstr(rng: &mut Rng, edge_idx: Option<usize>) -> String {
    if let Some(i) = edge_idx {
        return NUM_EDGE[i % NUM_EDGE.len()].to_string();
    }
    if rng.chance(40) {
        return NUM_EDGE[rng.below(NUM_EDGE.len() as u64) as usize].to_string();
    }
    let mut s = String::new();
    if rng.chance(30) {
        s.push('-');
    }
    for _ in 0..rng.below(22) {
        s.push((b'0' + rng.below(10) as u8) as char);
    }
    if rng.chance(10) {
        s.push(['x', ' ', '\t', '.'][rng.below(4) as usize]);
    }
    s
}

const ALPHA: [char; 7] = ['a', 'b', 'c', ' ', '\t', '\n', 'a'];

fn wstr(rng: &mut Rng, maxlen: u64) -> String {
    (0..rng.below(maxlen + 1))
        .map(|_| ALPHA[rng.below(ALPHA.len() as u64) as usize])
        .collect()
}

fn u64_edge(rng: &mut Rng) -> u64 {
    let e = [
        0u64,
        1,
        2,
        9,
        10,
        255,
        256,
        u32::MAX as u64,
        u64::MAX - 1,
        u64::MAX,
        1 << 63,
    ];
    if rng.chance(60) {
        e[rng.below(e.len() as u64) as usize]
    } else {
        rng.next() >> rng.below(64)
    }
}

fn i64_edge(rng: &mut Rng) -> i64 {
    let e = [
        i64::MIN,
        i64::MIN + 1,
        -10,
        -1,
        0,
        1,
        9,
        i64::MAX - 1,
        i64::MAX,
    ];
    if rng.chance(60) {
        e[rng.below(e.len() as u64) as usize]
    } else {
        (rng.next() >> rng.below(64)) as i64 * if rng.chance(50) { -1 } else { 1 }
    }
}

fn i64lit(v: i64) -> String {
    if v == i64::MIN {
        "(-9223372036854775807i64 - 1i64)".to_string()
    } else if v < 0 {
        format!("(-{}i64)", -v)
    } else {
        format!("{v}i64")
    }
}

// ------------------------------------------------------------------ chương trình
const HELPERS: &str = "ƒ bn(b: bool) → u64 ! ∅ { ⎇ b { 1u64 } ⇝ { 0u64 } }
ƒ rk(o: Ordering) → u64 ! ∅ { ⌥ o { Ordering::Less ↦ 1u64, Ordering::Equal ↦ 2u64, Ordering::Greater ↦ 3u64 } }
ƒ ord(k: u8) → Ordering ! ∅ { ⎇ k == 0u8 { Ordering::Less } ⇝ { ⎇ k == 1u8 { Ordering::Equal } ⇝ { Ordering::Greater } } }
ƒ oz(o: Option<usize>) → u64 ! ∅ { ⌥ o { Some(v) ↦ (v as u64) +% 1u64, None ↦ 0u64 } }
ƒ ou(o: Option<u64>) → u64 ! ∅ { ⌥ o { Some(v) ↦ v +% 1u64, None ↦ 0u64 } }
ƒ ci(v: i64) → u64 ! ∅ { ⎇ v >= 0i64 { (v as u64) *% 2u64 } ⇝ { (((0i64 - (v + 1i64)) as u64) *% 2u64) +% 1u64 } }
ƒ oi(o: Option<i64>) → u64 ! ∅ { ⌥ o { Some(v) ↦ ci(v) +% 1u64, None ↦ 0u64 } }
ƒ pe(r: Result<u64, std::ParseError>) → u64 ! ∅ { ⌥ r { Ok(v) ↦ v, Err(e) ↦ ⌥ e { std::ParseError::Empty ↦ 1000000001u64, std::ParseError::Invalid ↦ 1000000002u64, std::ParseError::Overflow ↦ 1000000003u64 } } }
ƒ pei(r: Result<i64, std::ParseError>) → u64 ! ∅ { ⌥ r { Ok(v) ↦ ci(v), Err(e) ↦ ⌥ e { std::ParseError::Empty ↦ 1000000001u64, std::ParseError::Invalid ↦ 1000000002u64, std::ParseError::Overflow ↦ 1000000003u64 } } }
ƒ fold_bytes(b: &[u8]) → u64 ! ∅ {
  μacc ≔ b.len() as u64;
  μi ≔ 0usize;
  ⟳ i < b.len() ↓ b.len() - i { acc ← acc *% 31u64 +% ((b[i] as u64) + 1u64); i ← i + 1usize; }
  acc
}
ƒ fold_u64s(b: &[u64]) → u64 ! ∅ {
  μacc ≔ b.len() as u64;
  μi ≔ 0usize;
  ⟳ i < b.len() ↓ b.len() - i { acc ← acc *% 31u64 +% b[i]; i ← i + 1usize; }
  acc
}
ƒ fu(n: u64) → u64 ! ∅ { μbuf ≔ [0u8; 24]; s ≔ std::fmt_u64(&!buf, n); fold_bytes(&buf[s..24usize]) }
ƒ fi(n: i64) → u64 ! ∅ { μbuf ≔ [0u8; 24]; s ≔ std::fmt_i64(&!buf, n); fold_bytes(&buf[s..24usize]) }
ƒ fh(n: u64) → u64 ! ∅ { μbuf ≔ [0u8; 24]; s ≔ std::fmt_hex(&!buf, n); fold_bytes(&buf[s..24usize]) }
ƒ bcmp(a: &str, b: &str) → u64 ! ∅ { k1 ≔ std::Bytes { b: a.as_bytes() }; k2 ≔ std::Bytes { b: b.as_bytes() }; rk(k1.cmp(&k2)) }
ƒ bhash(a: &str) → u64 ! ∅ { k ≔ std::Bytes { b: a.as_bytes() }; k.hash() }
ƒ sig_u32(a: &![u32]) → u64 ! ∅ {
  a.sort();
  μacc ≔ a.len() as u64; μi ≔ 0usize;
  ⟳ i < a.len() ↓ a.len() - i { acc ← acc *% 31u64 +% (a[i] as u64); i ← i + 1usize; }
  acc
}
ƒ sig_i64(a: &![i64]) → u64 ! ∅ {
  a.sort();
  μacc ≔ a.len() as u64; μi ≔ 0usize;
  ⟳ i < a.len() ↓ a.len() - i { acc ← acc *% 31u64 +% ci(a[i]); i ← i + 1usize; }
  acc
}
ƒ sig_u8(a: &![u8]) → u64 ! ∅ {
  a.sort();
  μacc ≔ a.len() as u64; μi ≔ 0usize;
  ⟳ i < a.len() ↓ a.len() - i { acc ← acc *% 31u64 +% (a[i] as u64); i ← i + 1usize; }
  acc
}
";

struct Case {
    name: &'static str,
    mem: bool,
    /// hàm Grit phụ trợ (mỗi ca sort một hàm cho mỗi mảng) đặt TRƯỚC `cK`
    defs: String,
    func: String,
    model: u64,
    /// các hàm std thực sự được gọi (đếm cho bảng ≥ 30 chương trình)
    used: Vec<&'static str>,
}

struct Prog {
    cases: Vec<Case>,
    src: String,
    helpers: String,
    expect: u64,
}

/// ca thuần: dãy `acc ← acc *% 31 +% (expr)` với giá trị kỳ vọng của mô hình
struct Pure {
    body: String,
    defs: String,
    acc: u64,
    n: usize,
}

impl Pure {
    fn new() -> Pure {
        Pure {
            body: String::new(),
            defs: String::new(),
            acc: 0,
            n: 0,
        }
    }
    fn push(&mut self, expr: &str, v: u64) {
        self.body
            .push_str(&format!("  acc ← acc *% 31u64 +% ({expr});\n"));
        self.acc = fold(self.acc, v);
        self.n += 1;
    }
    fn finish(self, name: &'static str, idx: usize) -> Case {
        Case {
            name,
            mem: false,
            defs: self.defs,
            func: format!(
                "ƒ c{idx}() → u64 ! ∅ {{\n  μacc ≔ 0u64;\n{}  acc\n}}\n",
                self.body
            ),
            model: self.acc,
            used: vec![name],
        }
    }
}

fn t(
    out: &mut Vec<(&'static str, Pure)>,
    reps: usize,
    name: &'static str,
    f: &mut dyn FnMut(usize, &mut Pure),
) {
    let mut p = Pure::new();
    for k in 0..reps {
        f(k, &mut p);
    }
    out.push((name, p));
}

fn pure_cases(rng: &mut Rng, edge: bool) -> Vec<Pure> {
    let reps = if edge { NUM_EDGE.len() } else { 14 };
    let mut out: Vec<(&'static str, Pure)> = Vec::new();
    // ---- lát byte: starts_with/ends_with/find/find_byte/trim_ascii
    let mut r1 = Rng(rng.next());
    t(&mut out, reps, "[u8]::starts_with", &mut |k, p| {
        let s = wstr(&mut r1, 6);
        let pat = if r1.chance(50) {
            s.chars().take(r1.below(4) as usize).collect()
        } else {
            wstr(&mut r1, 3)
        };
        let _ = k;
        p.push(
            &format!(
                "bn({}.as_bytes().starts_with({}.as_bytes()))",
                glit(&s),
                glit(&pat)
            ),
            s.as_bytes().starts_with(pat.as_bytes()) as u64,
        );
    });
    let mut r2 = Rng(rng.next());
    t(&mut out, reps, "[u8]::ends_with", &mut |_, p| {
        let s = wstr(&mut r2, 6);
        let pat: String = if r2.chance(50) {
            let n = r2.below(4) as usize;
            s.chars()
                .skip(s.chars().count().saturating_sub(n))
                .collect()
        } else {
            wstr(&mut r2, 3)
        };
        p.push(
            &format!(
                "bn({}.as_bytes().ends_with({}.as_bytes()))",
                glit(&s),
                glit(&pat)
            ),
            s.as_bytes().ends_with(pat.as_bytes()) as u64,
        );
    });
    let mut r3 = Rng(rng.next());
    t(&mut out, reps, "[u8]::find", &mut |_, p| {
        let s = wstr(&mut r3, 8);
        let pat = if r3.chance(60) && !s.is_empty() {
            let a = r3.below(s.len() as u64) as usize;
            let b = (a + r3.below(4) as usize).min(s.len());
            s[a..b].to_string()
        } else {
            wstr(&mut r3, 3)
        };
        p.push(
            &format!(
                "oz({}.as_bytes().find({}.as_bytes()))",
                glit(&s),
                glit(&pat)
            ),
            oi(m_find(s.as_bytes(), pat.as_bytes())),
        );
    });
    let mut r4 = Rng(rng.next());
    t(&mut out, reps, "[u8]::find_byte", &mut |_, p| {
        let s = wstr(&mut r4, 8);
        let from = r4.below(10) as usize;
        let b = [b'a', b'b', b'c', b' ', b'\t', b'z'][r4.below(6) as usize];
        let m = if from >= s.len() {
            None
        } else {
            s.as_bytes()[from..]
                .iter()
                .position(|&c| c == b)
                .map(|i| i + from)
        };
        p.push(
            &format!("oz({}.as_bytes().find_byte({from}usize, {b}u8))", glit(&s)),
            oi(m),
        );
    });
    let mut r5 = Rng(rng.next());
    t(&mut out, reps, "[u8]::trim_ascii", &mut |_, p| {
        let s = wstr(&mut r5, 9);
        p.push(
            &format!("fold_bytes({}.as_bytes().trim_ascii())", glit(&s)),
            fold_bytes(m_trim(s.as_bytes())),
        );
    });
    // ---- phân tích số
    let mut r6 = Rng(rng.next());
    t(&mut out, reps, "[u8]::parse_u64", &mut |k, p| {
        let s = numstr(&mut r6, edge.then_some(k));
        p.push(
            &format!("pe({}.as_bytes().parse_u64())", glit(&s)),
            m_pu64(s.as_bytes()),
        );
    });
    let mut r7 = Rng(rng.next());
    t(&mut out, reps, "[u8]::parse_i64", &mut |k, p| {
        let s = numstr(&mut r7, edge.then_some(k));
        p.push(
            &format!("pei({}.as_bytes().parse_i64())", glit(&s)),
            m_pi64(s.as_bytes()),
        );
    });
    let mut r8 = Rng(rng.next());
    t(&mut out, reps, "str::parse_u64", &mut |k, p| {
        let s = numstr(&mut r8, edge.then_some(k));
        p.push(
            &format!("pe({}.parse_u64())", glit(&s)),
            m_pu64(s.as_bytes()),
        );
    });
    let mut r9 = Rng(rng.next());
    t(&mut out, reps, "str::parse_i64", &mut |k, p| {
        let s = numstr(&mut r9, edge.then_some(k));
        p.push(
            &format!("pei({}.parse_i64())", glit(&s)),
            m_pi64(s.as_bytes()),
        );
    });
    // ---- str
    let mut r10 = Rng(rng.next());
    t(&mut out, reps, "str::starts_with", &mut |_, p| {
        let s = wstr(&mut r10, 6);
        let pat: String = s.chars().take(r10.below(4) as usize).collect();
        let pat = if r10.chance(30) {
            wstr(&mut r10, 3)
        } else {
            pat
        };
        p.push(
            &format!("bn({}.starts_with({}))", glit(&s), glit(&pat)),
            s.starts_with(&pat) as u64,
        );
    });
    let mut r11 = Rng(rng.next());
    t(&mut out, reps, "str::ends_with", &mut |_, p| {
        let s = wstr(&mut r11, 6);
        let n = r11.below(4) as usize;
        let pat: String = s
            .chars()
            .skip(s.chars().count().saturating_sub(n))
            .collect();
        let pat = if r11.chance(30) {
            wstr(&mut r11, 3)
        } else {
            pat
        };
        p.push(
            &format!("bn({}.ends_with({}))", glit(&s), glit(&pat)),
            s.ends_with(&pat) as u64,
        );
    });
    let mut r12 = Rng(rng.next());
    t(&mut out, reps, "str::find", &mut |_, p| {
        let s = wstr(&mut r12, 8);
        let pat = wstr(&mut r12, 3);
        p.push(
            &format!("oz({}.find({}))", glit(&s), glit(&pat)),
            oi(m_find(s.as_bytes(), pat.as_bytes())),
        );
    });
    // ---- in số
    let mut r13 = Rng(rng.next());
    t(&mut out, reps, "fmt_u64", &mut |_, p| {
        let n = u64_edge(&mut r13);
        p.push(&format!("fu({n}u64)"), fold_bytes(n.to_string().as_bytes()));
    });
    let mut r14 = Rng(rng.next());
    t(&mut out, reps, "fmt_i64", &mut |_, p| {
        let n = i64_edge(&mut r14);
        p.push(
            &format!("fi({})", i64lit(n)),
            fold_bytes(n.to_string().as_bytes()),
        );
    });
    let mut r15 = Rng(rng.next());
    t(&mut out, reps, "fmt_hex", &mut |_, p| {
        let n = u64_edge(&mut r15);
        p.push(
            &format!("fh({n}u64)"),
            fold_bytes(format!("{n:x}").as_bytes()),
        );
    });
    // ---- số học kiểm tràn
    let mut r16 = Rng(rng.next());
    t(&mut out, reps, "abs_i64", &mut |_, p| {
        let x = i64_edge(&mut r16);
        p.push(
            &format!("oi(std::abs_i64({}))", i64lit(x)),
            x.checked_abs().map_or(0, |v| zz(v).wrapping_add(1)),
        );
    });
    let mut r17 = Rng(rng.next());
    t(&mut out, reps, "pow_u64", &mut |_, p| {
        let b = [0u64, 1, 2, 3, 10, 255, u64::MAX, r17.below(20)][r17.below(8) as usize];
        let e = r17.below(70) as u32;
        p.push(
            &format!("ou(std::pow_u64({b}u64, {e}u32))"),
            b.checked_pow(e).map_or(0, |v| v.wrapping_add(1)),
        );
    });
    // ---- sắp xếp: mỗi mảng một hàm phụ
    let mut r18 = Rng(rng.next());
    let mut sortn = 0usize;
    let mut idx_base = 0usize;
    for (name, kind) in [
        ("[T]::sort (u32)", 0),
        ("[T]::sort (i64)", 1),
        ("[T]::sort (u8)", 2),
    ] {
        let mut p = Pure::new();
        for _ in 0..reps {
            let n = 1 + r18.below(12) as usize;
            let (lits, model): (Vec<String>, u64) = match kind {
                0 => {
                    let mut v: Vec<u32> = (0..n)
                        .map(|_| {
                            [0, 1, u32::MAX, r18.below(6) as u32, r18.next() as u32]
                                [r18.below(5) as usize]
                        })
                        .collect();
                    let l = v.iter().map(|x| format!("{x}u32")).collect();
                    v.sort();
                    (
                        l,
                        v.iter()
                            .fold(n as u64, |a, &x| a.wrapping_mul(31).wrapping_add(x as u64)),
                    )
                }
                1 => {
                    let mut v: Vec<i64> = (0..n)
                        .map(|_| {
                            if r18.chance(50) {
                                i64_edge(&mut r18)
                            } else {
                                r18.below(5) as i64 - 2
                            }
                        })
                        .collect();
                    let l = v.iter().map(|&x| i64lit(x)).collect();
                    v.sort();
                    (
                        l,
                        v.iter()
                            .fold(n as u64, |a, &x| a.wrapping_mul(31).wrapping_add(zz(x))),
                    )
                }
                _ => {
                    let mut v: Vec<u8> = (0..n)
                        .map(|_| {
                            [0, 255, r18.below(5) as u8, r18.next() as u8][r18.below(4) as usize]
                        })
                        .collect();
                    let l = v.iter().map(|x| format!("{x}u8")).collect();
                    v.sort();
                    (
                        l,
                        v.iter()
                            .fold(n as u64, |a, &x| a.wrapping_mul(31).wrapping_add(x as u64)),
                    )
                }
            };
            let (ty, sig) = [("u32", "sig_u32"), ("i64", "sig_i64"), ("u8", "sig_u8")][kind];
            sortn += 1;
            let fname = format!("srt{}_{}", idx_base, sortn);
            p.defs.push_str(&format!(
                "ƒ {fname}() → u64 ! ∅ {{ μa ≔ [{}]; {sig}(&!a[0usize..{n}usize]) }}\n",
                lits.join(", ")
            ));
            let _ = ty;
            p.push(&format!("{fname}()"), model);
        }
        out.push((name, p));
        idx_base += 1;
    }
    // ---- băm
    let mut r19 = Rng(rng.next());
    t(&mut out, reps, "hash_bytes", &mut |_, p| {
        let s = wstr(&mut r19, 8);
        p.push(
            &format!("std::hash_bytes({}.as_bytes())", glit(&s)),
            m_hash_bytes(s.as_bytes()),
        );
    });
    let mut r20 = Rng(rng.next());
    t(&mut out, reps, "Hash for u64/usize", &mut |_, p| {
        let n = u64_edge(&mut r20);
        p.push(
            &format!("({n}u64).hash() +% ({n}usize).hash()"),
            n.wrapping_mul(11400714819323198485).wrapping_mul(2),
        );
    });
    let mut r21 = Rng(rng.next());
    t(&mut out, reps, "Bytes (Ord, Hash)", &mut |_, p| {
        let a = wstr(&mut r21, 4);
        let b = if r21.chance(30) {
            a.clone()
        } else {
            wstr(&mut r21, 4)
        };
        p.push(
            &format!("bcmp({}, {})", glit(&a), glit(&b)),
            rk(a.as_bytes().cmp(b.as_bytes())),
        );
        p.push(&format!("bhash({})", glit(&a)), m_hash_bytes(a.as_bytes()));
    });
    let mut r22 = Rng(rng.next());
    t(
        &mut out,
        reps,
        "Ordering::reverse/then/is_*",
        &mut |_, p| {
            let (a, b) = (r22.below(3) as u8, r22.below(3) as u8);
            let o = |k: u8| [Ordering::Less, Ordering::Equal, Ordering::Greater][k as usize];
            p.push(&format!("rk(ord({a}u8).reverse())"), rk(o(a).reverse()));
            p.push(
                &format!("rk(ord({a}u8).then(ord({b}u8)))"),
                rk(o(a).then(o(b))),
            );
            p.push(
            &format!("bn(ord({a}u8).is_lt()) + 2u64 * bn(ord({a}u8).is_eq()) + 4u64 * bn(ord({a}u8).is_gt())"),
            o(a).is_lt() as u64 + 2 * o(a).is_eq() as u64 + 4 * o(a).is_gt() as u64,
        );
        },
    );
    let mut r23 = Rng(rng.next());
    t(&mut out, reps, "is_ascii_ws", &mut |_, p| {
        let c = if r23.chance(50) {
            [9u8, 10, 13, 32, 11, 12, 0, 255][r23.below(8) as usize]
        } else {
            r23.next() as u8
        };
        p.push(&format!("bn(std::is_ascii_ws({c}u8))"), is_ws(c) as u64);
    });
    // đặt tên: giữ thứ tự, trả Pure kèm tên qua `defs` marker
    out.into_iter()
        .map(|(n, mut p)| {
            p.defs = format!("// @{n}\n{}", p.defs);
            p
        })
        .collect()
}

/// các ca dùng Arena: dãy câu lệnh trên Vec/HashMap thật cùng mô hình Rust; chỉ so mô hình ≡ exe
struct Mem {
    body: String,
    acc: u64,
    used: BTreeMap<&'static str, u64>,
}

impl Mem {
    fn new() -> Mem {
        Mem {
            body: String::new(),
            acc: 0,
            used: BTreeMap::new(),
        }
    }
    fn stmt(&mut self, s: &str) {
        self.body.push_str(&format!("  {s}\n"));
    }
    fn fold(&mut self, name: &'static str, expr: &str, v: u64) {
        self.body
            .push_str(&format!("  acc ← acc *% 31u64 +% ({expr});\n"));
        self.acc = fold(self.acc, v);
        *self.used.entry(name).or_default() += 1;
    }
    fn func(&self, idx: usize) -> String {
        format!(
            "ƒ c{idx}() → u64 ! {{mem}} {{\n  μar ≔ Arena::new();\n  μacc ≔ 0u64;\n{}  acc\n}}\n",
            self.body
        )
    }
}

fn mem_vec_u64(rng: &mut Rng, uid: &mut usize) -> Mem {
    let mut m = Mem::new();
    m.stmt("μv ≔ Vec::new(&ar);");
    let mut model: Vec<u64> = Vec::new();
    let val =
        |rng: &mut Rng| -> u64 { [0u64, 1, 2, 3, u64::MAX, rng.below(6)][rng.below(6) as usize] };
    let nops = 20 + rng.below(25);
    for step in 0..nops {
        if step == nops / 2 {
            // mỗi chương trình có ít nhất một `clear` giữa chừng (để mọi hàm xuất hiện ≥ 30 chương trình)
            m.stmt("v.clear();");
            model.clear();
            *m.used.entry("Vec::clear").or_default() += 1;
            continue;
        }
        let n = model.len();
        match rng.below(14) {
            0..=3 => {
                let x = val(rng);
                m.stmt(&format!("v.push({x}u64);"));
                model.push(x);
                *m.used.entry("Vec::push").or_default() += 1;
            }
            4 => {
                m.fold(
                    "Vec::pop",
                    "ou(v.pop())",
                    model.pop().map_or(0, |v| v.wrapping_add(1)),
                );
            }
            5 => {
                let i = rng.below(n as u64 + 3) as usize;
                m.fold(
                    "Vec::get",
                    &format!("ou(v.get({i}usize))"),
                    model.get(i).map_or(0, |v| v.wrapping_add(1)),
                );
            }
            6 => {
                m.fold(
                    "Vec::first",
                    "ou(v.first())",
                    model.first().map_or(0, |v| v.wrapping_add(1)),
                );
                m.fold(
                    "Vec::last",
                    "ou(v.last())",
                    model.last().map_or(0, |v| v.wrapping_add(1)),
                );
            }
            7 => {
                m.fold(
                    "Vec::len/is_empty",
                    "(v.len() as u64) + 7u64 * bn(v.is_empty())",
                    n as u64 + 7 * model.is_empty() as u64,
                );
            }
            8 | 9 => {
                let i = rng.below(n as u64 + 3) as usize;
                let x = val(rng);
                m.stmt(&format!("v.insert({i}usize, {x}u64);"));
                model.insert(i.min(n), x);
                *m.used.entry("Vec::insert").or_default() += 1;
            }
            10 => {
                let i = rng.below(n as u64 + 2) as usize;
                let r = if i < n { Some(model.remove(i)) } else { None };
                m.fold(
                    "Vec::remove",
                    &format!("ou(v.remove({i}usize))"),
                    r.map_or(0, |v| v.wrapping_add(1)),
                );
            }
            11 => {
                let x = val(rng);
                m.fold(
                    "Vec::contains",
                    &format!("bn(v.contains(&{x}u64))"),
                    model.contains(&x) as u64,
                );
            }
            12 => {
                let k = 1 + rng.below(3) as usize;
                let xs: Vec<u64> = (0..k).map(|_| val(rng)).collect();
                *uid += 1;
                let e = format!("e{uid}");
                let lits: Vec<String> = xs.iter().map(|x| format!("{x}u64")).collect();
                m.stmt(&format!("{e} ≔ [{}];", lits.join(", ")));
                m.stmt(&format!("v.extend_from_slice(&{e}[0usize..{k}usize]);"));
                model.extend(xs);
                *m.used.entry("Vec::extend_from_slice").or_default() += 1;
            }
            _ => {
                if rng.chance(25) {
                    m.stmt("v.clear();");
                    model.clear();
                    *m.used.entry("Vec::clear").or_default() += 1;
                }
            }
        }
    }
    m.fold("Vec::as_slice", "fold_u64s(v.as_slice())", {
        let mut a = model.len() as u64;
        for &x in &model {
            a = a.wrapping_mul(31).wrapping_add(x);
        }
        a
    });
    m
}

fn mem_vec_bytes(rng: &mut Rng) -> Mem {
    let mut m = Mem::new();
    m.stmt("μv ≔ Vec::new(&ar);");
    let mut model: Vec<u8> = Vec::new();
    for _ in 0..(8 + rng.below(12)) {
        match rng.below(4) {
            0 => {
                let s = wstr(rng, 5);
                m.stmt(&format!("v.push_str({});", glit(&s)));
                model.extend(s.bytes());
                *m.used.entry("Vec<u8>::push_str").or_default() += 1;
            }
            1 => {
                let n = u64_edge(rng);
                m.stmt(&format!("v.push_u64({n}u64);"));
                model.extend(n.to_string().bytes());
                *m.used.entry("Vec<u8>::push_u64").or_default() += 1;
            }
            2 => {
                let n = i64_edge(rng);
                m.stmt(&format!("v.push_i64({});", i64lit(n)));
                model.extend(n.to_string().bytes());
                *m.used.entry("Vec<u8>::push_i64").or_default() += 1;
            }
            _ => {
                let b = rng.next() as u8;
                m.stmt(&format!("v.push({b}u8);"));
                model.push(b);
                *m.used.entry("Vec::push").or_default() += 1;
            }
        }
    }
    m.fold(
        "Vec::as_slice",
        "fold_bytes(v.as_slice())",
        fold_bytes(&model),
    );
    m
}

fn mem_map_u64(rng: &mut Rng, churn: bool) -> Mem {
    let mut m = Mem::new();
    m.stmt("μm ≔ std::HashMap::new(&ar);");
    let pool = [8u64, 40, 200][rng.below(3) as usize];
    let mut model: BTreeMap<u64, u64> = BTreeMap::new();
    let nops = if churn { 120 } else { 60 + rng.below(100) };
    for _ in 0..nops {
        let k = if rng.chance(10) {
            [0u64, u64::MAX, 1 << 40][rng.below(3) as usize]
        } else {
            rng.below(pool)
        };
        let w = rng.below(100);
        let ins = if churn { 50 } else { 45 };
        if w < ins {
            let v = rng.next() >> rng.below(64);
            m.stmt(&format!("m.insert({k}u64, {v}u64);"));
            model.insert(k, v);
            *m.used.entry("HashMap::insert").or_default() += 1;
        } else if w < ins + 25 {
            let had = model.remove(&k).is_some();
            m.fold(
                "HashMap::remove",
                &format!("bn(m.remove(&{k}u64))"),
                had as u64,
            );
        } else if w < ins + 45 {
            m.fold(
                "HashMap::get",
                &format!("ou(m.get(&{k}u64))"),
                model.get(&k).map_or(0, |v| v.wrapping_add(1)),
            );
        } else if w < ins + 52 {
            m.fold(
                "HashMap::contains_key",
                &format!("bn(m.contains_key(&{k}u64))"),
                model.contains_key(&k) as u64,
            );
        } else {
            m.fold("HashMap::len", "(m.len() as u64)", model.len() as u64);
        }
    }
    // duyệt: tổng không phụ thuộc thứ tự + số entry sống
    let sum = model.iter().fold(0u64, |s, (k, v)| {
        s.wrapping_add(k.wrapping_mul(1000003).wrapping_add(*v))
    });
    m.stmt("μsum ≔ 0u64;");
    m.stmt("μlive ≔ 0u64;");
    m.stmt("μi ≔ 0usize;");
    m.stmt("μn ≔ m.entry_count();");
    m.stmt("⟳ i < n ↓ n - i { ⌥ m.entry_at(i) { Some(e) ↦ { sum ← sum +% (e.0 *% 1000003u64 +% e.1); live ← live + 1u64; }, None ↦ { } } i ← i + 1usize; }");
    m.fold(
        "HashMap::entry_at/entry_count",
        "sum +% live *% 7919u64",
        sum.wrapping_add(model.len() as u64 * 7919),
    );
    m
}

const WORDS: [&str; 10] = ["", "a", "ab", "abc", "b", "ba", "zz", "a b", "a\tb", "key"];

fn mem_map_bytes(rng: &mut Rng, uid: &mut usize) -> Mem {
    let mut m = Mem::new();
    m.stmt("μm ≔ std::HashMap::new(&ar);");
    let mut model: BTreeMap<&'static str, u64> = BTreeMap::new();
    for _ in 0..(30 + rng.below(50)) {
        let w = WORDS[rng.below(WORDS.len() as u64) as usize];
        *uid += 1;
        let kn = format!("k{uid}");
        m.stmt(&format!(
            "{kn} ≔ std::Bytes {{ b: {}.as_bytes() }};",
            glit(w)
        ));
        let r = rng.below(100);
        if r < 50 {
            let v = rng.next() >> rng.below(64);
            m.stmt(&format!("m.insert({kn}, {v}u64);"));
            model.insert(w, v);
            *m.used.entry("HashMap<Bytes>::insert").or_default() += 1;
        } else if r < 70 {
            let had = model.remove(w).is_some();
            m.fold(
                "HashMap<Bytes>::remove",
                &format!("bn(m.remove(&{kn}))"),
                had as u64,
            );
        } else {
            m.fold(
                "HashMap<Bytes>::get",
                &format!("ou(m.get(&{kn}))"),
                model.get(w).map_or(0, |v| v.wrapping_add(1)),
            );
        }
    }
    m.fold(
        "HashMap<Bytes>::len",
        "(m.len() as u64)",
        model.len() as u64,
    );
    m
}

fn program(seed: u64, edge: bool, mem_only: bool) -> Prog {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut cases: Vec<Case> = Vec::new();
    if !mem_only {
        for p in pure_cases(&mut rng, edge) {
            let idx = cases.len();
            let name_line = p.defs.lines().next().unwrap_or("").to_string();
            let name: &'static str = Box::leak(
                name_line
                    .trim_start_matches("// @")
                    .to_string()
                    .into_boxed_str(),
            );
            let mut p = p;
            p.defs = p.defs.lines().skip(1).map(|l| format!("{l}\n")).collect();
            cases.push(p.finish(name, idx));
        }
    } else {
        let mut uid = 0usize;
        let mk = |name: &'static str, m: Mem, cases: &mut Vec<Case>| {
            let idx = cases.len();
            let used: Vec<&'static str> = m.used.keys().copied().collect();
            cases.push(Case {
                name,
                mem: true,
                defs: String::new(),
                func: m.func(idx),
                model: m.acc,
                used,
            });
        };
        mk("Vec<u64> ops", mem_vec_u64(&mut rng, &mut uid), &mut cases);
        mk("Vec<u64> ops", mem_vec_u64(&mut rng, &mut uid), &mut cases);
        mk("Vec<u8> text", mem_vec_bytes(&mut rng), &mut cases);
        mk("HashMap<u64,u64>", mem_map_u64(&mut rng, false), &mut cases);
        mk(
            "HashMap<u64,u64> churn",
            mem_map_u64(&mut rng, true),
            &mut cases,
        );
        mk(
            "HashMap<Bytes,u64>",
            mem_map_bytes(&mut rng, &mut uid),
            &mut cases,
        );
    }
    let mut src = String::from(HELPERS);
    for c in &cases {
        src.push_str(&c.defs);
        src.push_str(&c.func);
    }
    src.push_str(&format!(
        "ƒ main() → u64 ! {} {{\n  μacc ≔ 0u64;\n",
        if mem_only { "{mem}" } else { "∅" }
    ));
    let mut total = 0u64;
    for (i, c) in cases.iter().enumerate() {
        src.push_str(&format!("  acc ← acc *% 1000003u64 +% c{i}();\n"));
        total = total.wrapping_mul(1000003).wrapping_add(c.model);
    }
    src.push_str("  acc\n}\n");
    Prog {
        cases,
        src,
        helpers: HELPERS.to_string(),
        expect: total,
    }
}

/// chạy riêng từng ca khi chương trình lệch, để chỉ ra hàm nào sai
fn diagnose(p: &Prog, seed: u64) -> String {
    let mut out = String::new();
    for (i, c) in p.cases.iter().enumerate() {
        let eff = if c.mem { "{mem}" } else { "∅" };
        let src = format!(
            "{}{}{}ƒ main() → u64 ! {eff} {{ c{i}() }}\n",
            p.helpers, c.defs, c.func
        );
        match run3(&src, &format!("d{seed}_{i}"), !c.mem) {
            Ok((a, b)) => {
                let w = Some(c.model.to_string());
                if a != w || b != w {
                    out.push_str(&format!(
                        "  LỆCH {}: mô hình {}, thông dịch {a:?}, llvm {b:?}\n",
                        c.name, c.model
                    ));
                }
            }
            Err(e) => out.push_str(&format!("  LỖI {}: {e}\n", c.name)),
        }
    }
    out
}

#[test]
fn new_std_functions_match_the_independent_rust_model_in_three_sources() {
    let random: u64 = std::env::var("STD2_RANDOM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(32);
    let edge_n: u64 = 3;
    let mut table: BTreeMap<&'static str, u64> = BTreeMap::new(); // tên → số chương trình xanh có gọi
    let mut fails: Vec<String> = Vec::new();
    for k in 0..random + edge_n {
        let edge = k >= random;
        let seed = if edge { 9000 + k } else { 1 + k };
        for mem_only in [false, true] {
            let p = program(seed, edge, mem_only);
            match run3(&p.src, &format!("p{seed}{mem_only}"), !mem_only) {
                Ok((a, b)) => {
                    let w = Some(p.expect.to_string());
                    if a == w && b == w {
                        let mut seen: Vec<&'static str> = Vec::new();
                        for c in &p.cases {
                            for u in &c.used {
                                if !seen.contains(u) {
                                    seen.push(u);
                                }
                            }
                        }
                        for u in seen {
                            *table.entry(u).or_default() += 1;
                        }
                    } else {
                        fails.push(format!(
                            "#{seed}{}{}: mô hình {} ≠ thông dịch {a:?} / llvm {b:?}\n{}",
                            if edge { " (edge)" } else { "" },
                            if mem_only { " (mem)" } else { "" },
                            p.expect,
                            diagnose(&p, seed)
                        ));
                    }
                }
                Err(e) => fails.push(format!(
                    "#{seed}{}: {e}\n{}",
                    if mem_only { " (mem)" } else { "" },
                    p.src
                )),
            }
        }
    }
    println!("{:<36} {:>9}", "hàm/impl của std", "chương trình");
    for (n, pr) in &table {
        println!("{n:<36} {pr:>9}");
    }
    if let Some(out) = std::env::var_os("STD2_OUT") {
        let rows: Vec<String> = table
            .iter()
            .map(|(n, pr)| format!("[\"{n}\",{pr}]"))
            .collect();
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(out)
            .expect("mở tệp kết quả");
        writeln!(
            f,
            "{{\"suite\":\"std2\",\"profile\":\"{}\",\"random\":{random},\"edge\":{edge_n},\"fails\":{},\"rows\":[{}]}}",
            profile().1,
            fails.len(),
            rows.join(",")
        )
        .expect("ghi");
    }
    assert!(fails.is_empty(), "{}", fails.join("\n=====\n"));
    for (n, pr) in &table {
        assert!(*pr >= 30, "{n}: chỉ {pr} chương trình (< 30)");
    }
}
