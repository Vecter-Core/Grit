//! So sánh codegen với PHÉP BIT và số học quấn vòng (lệnh 2026-10-13, 2d): biểu thức u32 ngẫu nhiên gồm band/bor/bxor/bnot/shl/shr/rotr/rotl
//! (lượng dịch hằng, bị che `band(x, 31)`, hoặc THÔ ⇒ có thể vi phạm `n < 32`), `+% -% *%` (quấn) và `+ - *` (tràn ⇒ abort).
//! BA nguồn độc lập phải khớp: (1) thông dịch MIR, (2) exe LLVM thật, (3) bộ đánh giá Rust của chính cây biểu thức (u32 wrapping/checked).
//! Chạy theo lô: `GT_BITS_START=1 GT_BITS_COUNT=25 GT_BITS_OUT=docs/CODEGEN_DIFF_RESULTS.jsonl cargo test -p gt-driver --test llvm_diff_bits -- --ignored batch`
#![cfg(feature = "llvm")]
#[path = "../../gt-mir/tests/support/progen.rs"]
mod progen;

use gt_driver::native::{build_exe, run_exe_timeout};
use gt_driver::pipeline::attestation;
use gt_driver::runner::run_main;
use gt_driver::{Options, RunEnd, check};
use gt_resolve::SourceInput;
use std::io::Write;
use std::time::Duration;

#[derive(Clone, Debug)]
enum Amt {
    /// lượng dịch hằng 0..31 (phải được M7 chứng minh `n < 32`)
    Const(u32),
    /// `band(e, 31)`: luôn hợp lệ nhưng cần chứng minh `band` ≤ 31 (M7 không biết ⇒ giữ kiểm tra lúc chạy)
    Masked(Box<E>),
    /// thô: có thể ≥ 32 ⇒ abort `precondition`
    Raw(Box<E>),
}

#[derive(Clone, Debug)]
enum E {
    Var(usize),
    Lit(u32),
    And(Box<E>, Box<E>),
    Or(Box<E>, Box<E>),
    Xor(Box<E>, Box<E>),
    Not(Box<E>),
    Shl(Box<E>, Amt),
    Shr(Box<E>, Amt),
    Rotr(Box<E>, Amt),
    Rotl(Box<E>, Amt),
    AddW(Box<E>, Box<E>),
    SubW(Box<E>, Box<E>),
    MulW(Box<E>, Box<E>),
    Add(Box<E>, Box<E>),
    Sub(Box<E>, Box<E>),
    Mul(Box<E>, Box<E>),
}

const VARS: [&str; 3] = ["a", "b", "c"];
const EDGE: [u32; 8] = [
    0,
    1,
    0xFFFF_FFFF,
    0x8000_0000,
    0x7FFF_FFFF,
    31,
    32,
    0xDEAD_BEEF,
];

fn bx(e: E) -> Box<E> {
    Box::new(e)
}

fn gen_expr(rng: &mut progen::Rng, depth: u32) -> E {
    if depth == 0 || rng.chance(15) {
        return if rng.chance(60) {
            E::Var(rng.below(3) as usize)
        } else if rng.chance(50) {
            E::Lit(EDGE[rng.below(EDGE.len() as u64) as usize])
        } else {
            E::Lit(rng.next() as u32)
        };
    }
    let d = depth - 1;
    let amt = |rng: &mut progen::Rng| match rng.below(5) {
        0 | 1 => Amt::Const(rng.below(32) as u32),
        2 | 3 => Amt::Masked(bx(gen_expr(rng, d))),
        _ => Amt::Raw(bx(gen_expr(rng, d))),
    };
    match rng.below(16) {
        0 => E::And(bx(gen_expr(rng, d)), bx(gen_expr(rng, d))),
        1 => E::Or(bx(gen_expr(rng, d)), bx(gen_expr(rng, d))),
        2 => E::Xor(bx(gen_expr(rng, d)), bx(gen_expr(rng, d))),
        3 => E::Not(bx(gen_expr(rng, d))),
        4 => E::Shl(bx(gen_expr(rng, d)), amt(rng)),
        5 => E::Shr(bx(gen_expr(rng, d)), amt(rng)),
        6 => E::Rotr(bx(gen_expr(rng, d)), amt(rng)),
        7 => E::Rotl(bx(gen_expr(rng, d)), amt(rng)),
        8 | 9 => E::AddW(bx(gen_expr(rng, d)), bx(gen_expr(rng, d))),
        10 => E::SubW(bx(gen_expr(rng, d)), bx(gen_expr(rng, d))),
        11 => E::MulW(bx(gen_expr(rng, d)), bx(gen_expr(rng, d))),
        12 => E::Add(bx(gen_expr(rng, d)), bx(gen_expr(rng, d))),
        13 => E::Sub(bx(gen_expr(rng, d)), bx(gen_expr(rng, d))),
        14 => E::Mul(bx(gen_expr(rng, d)), bx(gen_expr(rng, d))),
        _ => E::Xor(bx(gen_expr(rng, d)), bx(gen_expr(rng, d))),
    }
}

fn amt_src(a: &Amt) -> String {
    match a {
        Amt::Const(n) => format!("{n}u32"),
        Amt::Masked(e) => format!("band({}, 31u32)", src(e)),
        Amt::Raw(e) => src(e),
    }
}

fn src(e: &E) -> String {
    match e {
        E::Var(i) => VARS[*i].to_string(),
        E::Lit(v) => format!("0x{v:08x}u32"),
        E::And(a, b) => format!("band({}, {})", src(a), src(b)),
        E::Or(a, b) => format!("bor({}, {})", src(a), src(b)),
        E::Xor(a, b) => format!("bxor({}, {})", src(a), src(b)),
        E::Not(a) => format!("bnot({})", src(a)),
        E::Shl(a, n) => format!("shl({}, {})", src(a), amt_src(n)),
        E::Shr(a, n) => format!("shr({}, {})", src(a), amt_src(n)),
        E::Rotr(a, n) => format!("rotr({}, {})", src(a), amt_src(n)),
        E::Rotl(a, n) => format!("rotl({}, {})", src(a), amt_src(n)),
        E::AddW(a, b) => format!("({} +% {})", src(a), src(b)),
        E::SubW(a, b) => format!("({} -% {})", src(a), src(b)),
        E::MulW(a, b) => format!("({} *% {})", src(a), src(b)),
        E::Add(a, b) => format!("({} + {})", src(a), src(b)),
        E::Sub(a, b) => format!("({} - {})", src(a), src(b)),
        E::Mul(a, b) => format!("({} * {})", src(a), src(b)),
    }
}

/// Đánh giá tham chiếu bằng Rust: `Err(loại abort)` theo thứ tự đánh giá trái → phải, đối số trước rồi mới kiểm tiền điều kiện/tràn.
fn eval_amt(a: &Amt, v: &[u32; 3]) -> Result<u32, &'static str> {
    match a {
        Amt::Const(n) => Ok(*n),
        Amt::Masked(e) => Ok(eval(e, v)? & 31),
        Amt::Raw(e) => eval(e, v),
    }
}

fn eval(e: &E, v: &[u32; 3]) -> Result<u32, &'static str> {
    let two = |a: &E, b: &E| -> Result<(u32, u32), &'static str> { Ok((eval(a, v)?, eval(b, v)?)) };
    let shift = |a: &E, n: &Amt| -> Result<(u32, u32), &'static str> {
        let x = eval(a, v)?;
        let n = eval_amt(n, v)?;
        if n >= 32 {
            Err("precondition")
        } else {
            Ok((x, n))
        }
    };
    Ok(match e {
        E::Var(i) => v[*i],
        E::Lit(x) => *x,
        E::And(a, b) => {
            let (x, y) = two(a, b)?;
            x & y
        }
        E::Or(a, b) => {
            let (x, y) = two(a, b)?;
            x | y
        }
        E::Xor(a, b) => {
            let (x, y) = two(a, b)?;
            x ^ y
        }
        E::Not(a) => !eval(a, v)?,
        E::Shl(a, n) => {
            let (x, n) = shift(a, n)?;
            x << n
        }
        E::Shr(a, n) => {
            let (x, n) = shift(a, n)?;
            x >> n
        }
        E::Rotr(a, n) => {
            let (x, n) = shift(a, n)?;
            x.rotate_right(n)
        }
        E::Rotl(a, n) => {
            let (x, n) = shift(a, n)?;
            x.rotate_left(n)
        }
        E::AddW(a, b) => {
            let (x, y) = two(a, b)?;
            x.wrapping_add(y)
        }
        E::SubW(a, b) => {
            let (x, y) = two(a, b)?;
            x.wrapping_sub(y)
        }
        E::MulW(a, b) => {
            let (x, y) = two(a, b)?;
            x.wrapping_mul(y)
        }
        E::Add(a, b) => {
            let (x, y) = two(a, b)?;
            x.checked_add(y).ok_or("overflow")?
        }
        E::Sub(a, b) => {
            let (x, y) = two(a, b)?;
            x.checked_sub(y).ok_or("overflow")?
        }
        E::Mul(a, b) => {
            let (x, y) = two(a, b)?;
            x.checked_mul(y).ok_or("overflow")?
        }
    })
}

fn norm_interp(v: &Option<String>) -> Option<String> {
    let s = v.as_deref()?;
    s.strip_prefix("Int(")
        .and_then(|i| i.split(',').next())
        .map(str::to_string)
}

#[derive(Default)]
struct Tally {
    programs: u64,
    red_programs: u64,
    runs: u64,
    equal3: u64,
    mismatch: u64,
    normal: u64,
    aborts: std::collections::BTreeMap<String, u64>,
    skipped: std::collections::BTreeMap<String, u64>,
    detail: Vec<String>,
}

fn run_one(seed: u64, t: &mut Tally, dir: &std::path::Path) {
    let mut rng = progen::Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xB175 | 1);
    let e = gen_expr(&mut rng, 4);
    let body = src(&e);
    t.programs += 1;
    for k in 0..4 {
        t.runs += 1;
        let vals: [u32; 3] = [0, 1, 2].map(|_| {
            if rng.chance(60) {
                EDGE[rng.below(EDGE.len() as u64) as usize]
            } else {
                rng.next() as u32
            }
        });
        let source = format!(
            "ƒ entry(a: u32, b: u32, c: u32) → u32 ! ∅ {{ {body} }}\nƒ main() → u32 ! ∅ {{ entry({}u32, {}u32, {}u32) }}\n",
            vals[0], vals[1], vals[2]
        );
        let mut res = match check(
            &[SourceInput {
                name: "main.gt".into(),
                text: source.clone(),
            }],
            &Options::default(),
        ) {
            Ok(r) => r,
            Err(err) => {
                t.mismatch += 1;
                t.detail
                    .push(format!("#{seed}: đầu vào không hợp lệ: {err}\n{source}"));
                continue;
            }
        };
        if !res.verdict.allows_output() {
            let code = res
                .report
                .diagnostics
                .iter()
                .find(|d| d.level == gt_diag::Level::Error)
                .map_or("ICE".to_string(), |d| d.code.as_str().to_string());
            *t.skipped.entry(format!("ĐỎ {code}")).or_default() += 1;
            if code == "E9901" {
                t.mismatch += 1;
                t.detail.push(format!("#{seed}: ICE\n{source}"));
            }
            if k == 0 {
                t.red_programs += 1;
                return;
            }
            continue;
        }
        let exe = dir.join(format!("b{seed}.exe"));
        {
            let art = res.open_gate().expect("cổng mở");
            let att = attestation(&res, &gt_driver::pipeline::mir_dump(art));
            if let Err(err) = build_exe(art, &exe, true, &att) {
                t.mismatch += 1;
                t.detail.push(format!("#{seed}: llvm lỗi {err}\n{source}"));
                continue;
            }
        }
        let llvm = run_exe_timeout(&exe, Some(Duration::from_secs(20)));
        let _ = std::fs::remove_file(&exe);
        let interp = run_main(res.take_gated().expect("sản phẩm"));
        let want = eval(&e, &vals);
        let same = match (&want, &interp.end, &llvm.end) {
            (Ok(w), RunEnd::Normal, RunEnd::Normal) => {
                norm_interp(&interp.value).as_deref() == Some(&w.to_string())
                    && llvm.value.as_deref() == Some(&w.to_string())
            }
            (Err(w), RunEnd::Aborted(a), RunEnd::Aborted(b)) => a == w && b == w,
            _ => false,
        };
        if same {
            t.equal3 += 1;
            match &want {
                Ok(_) => t.normal += 1,
                Err(w) => *t.aborts.entry((*w).to_string()).or_default() += 1,
            }
        } else {
            t.mismatch += 1;
            t.detail.push(format!(
                "#{seed} {vals:?}: tham chiếu Rust {want:?} / thông dịch {:?} {:?} / llvm {:?} {:?}\n{source}",
                interp.end, interp.value, llvm.end, llvm.value
            ));
        }
    }
}

fn summary(t: &Tally) -> String {
    format!(
        "{{\"programs\":{},\"red_programs\":{},\"runs\":{},\"equal_3way\":{},\"mismatch\":{},\"normal\":{},\"aborts\":{:?},\"skipped\":{:?}}}",
        t.programs, t.red_programs, t.runs, t.equal3, t.mismatch, t.normal, t.aborts, t.skipped
    )
}

fn env_u64(k: &str, d: u64) -> u64 {
    std::env::var(k)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(d)
}

fn run_batch(start: u64, count: u64) -> Tally {
    let dir = std::env::temp_dir().join(format!("gt-bits-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo");
    let mut t = Tally::default();
    for s in start..start + count {
        run_one(s, &mut t, &dir);
    }
    let _ = std::fs::remove_dir_all(&dir);
    t
}

#[test]
#[ignore = "chạy theo lô bằng GT_BITS_START/COUNT/OUT"]
fn batch() {
    let (start, count) = (env_u64("GT_BITS_START", 1), env_u64("GT_BITS_COUNT", 25));
    let t = run_batch(start, count);
    let line = format!(
        "{{\"suite\":\"bits-3way\",\"start\":{start},\"count\":{count},\"result\":{}}}",
        summary(&t)
    );
    println!("{line}");
    if let Some(out) = std::env::var_os("GT_BITS_OUT") {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(out)
            .expect("mở");
        writeln!(f, "{line}").expect("ghi");
        for d in &t.detail {
            writeln!(f, "  DETAIL {d}").expect("ghi");
        }
    }
    assert_eq!(t.mismatch, 0, "bất đồng: {:#?}", t.detail);
}

/// Lô nhỏ cố định cho cổng đầy đủ.
#[test]
fn tiny_fixed_batch() {
    let t = run_batch(1, 6);
    println!("{}", summary(&t));
    assert_eq!(t.mismatch, 0, "bất đồng: {:#?}", t.detail);
    assert!(
        t.equal3 >= 8,
        "quá ít phép so sánh có nghĩa: {}",
        summary(&t)
    );
}
