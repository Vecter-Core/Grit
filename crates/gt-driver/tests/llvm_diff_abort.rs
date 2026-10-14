//! So sánh codegen NHẮM VÀO ABORT ÍT PHỦ (lệnh Giang 2026-10-13, mục 2): chương trình nhỏ thiên về chỉ số sát biên, chia/dư cho 0,
//! tiền điều kiện bị vi phạm. Mỗi chương trình chạy trên vài đầu vào (mỗi đầu vào = một exe); kết quả + loại abort phải KHỚP thông dịch.
//! Bất đồng nào là bug ⇒ thêm seed vào `PERMANENT` và sửa. Chạy theo lô, kết quả ghi nối vào tệp sau mỗi lô:
//!   `GT_ABORT_KIND=index|divrem|pre GT_ABORT_START=1 GT_ABORT_COUNT=25 GT_ABORT_OUT=docs/CODEGEN_DIFF_RESULTS.jsonl \
//!    cargo test -p gt-driver --test llvm_diff_abort -- --ignored batch`
#![cfg(feature = "llvm")]
#[path = "../../gt-oblig/tests/support/obgen.rs"]
mod obgen;

/// hồ sơ do bộ chạy chọn (`GT_PROFILE=release` ⇒ chạy lại cả bộ ở hồ sơ release, ADR-036 Q77); mặc định dev
fn test_profile() -> gt_codegen_llvm::Opt {
    if std::env::var("GT_PROFILE").is_ok_and(|v| v == "release") {
        gt_codegen_llvm::Opt::Release
    } else {
        gt_codegen_llvm::Opt::Dev
    }
}

#[path = "../../gt-mir/tests/support/progen.rs"]
mod progen;

use gt_driver::native::{build_exe_profile, run_exe_timeout};
use gt_driver::pipeline::attestation;
use gt_driver::runner::run_main;
use gt_driver::{Options, RunEnd, check};
use gt_resolve::SourceInput;
use std::io::Write;
use std::time::Duration;

/// Seed (kind, seed) đã từng lộ bất đồng thật.
const PERMANENT: &[(&str, u64)] = &[];

const INTS: [obgen::Ity; 9] = obgen::TYPES;

fn lit(ty: obgen::Ity, v: i128) -> String {
    if ty.signed && v == ty.min() {
        format!("(-{}{} -% 1{})", ty.max(), ty.name, ty.name)
    } else {
        ty.lit(v)
    }
}

fn norm_interp(v: &Option<String>) -> Option<String> {
    let s = v.as_deref()?;
    if s == "Unit" {
        return None;
    }
    if let Some(inner) = s.strip_prefix("Int(") {
        return inner.split(',').next().map(str::to_string);
    }
    Some(s.to_string())
}

/// Một chương trình đích: nguồn (không có `main`), danh sách tham số của `entry` + các bộ đầu vào + kiểu trả về.
struct Prog {
    src: String,
    ret: obgen::Ity,
    /// kiểu của từng tham số của `entry`
    params: Vec<obgen::Ity>,
    inputs: Vec<Vec<i128>>,
}

const USIZE: obgen::Ity = obgen::Ity {
    name: "usize",
    bits: 64,
    signed: false,
};

fn pick<'a, T>(rng: &mut progen::Rng, v: &'a [T]) -> &'a T {
    &v[rng.below(v.len() as u64) as usize]
}

fn array_lit(rng: &mut progen::Rng, ty: obgen::Ity, n: usize) -> String {
    let edge = [ty.min(), ty.max(), 0, 1, -1];
    let elems: Vec<String> = (0..n)
        .map(|_| {
            let v = *pick(rng, &edge);
            lit(ty, v.clamp(ty.min(), ty.max()))
        })
        .collect();
    format!("[{}]", elems.join(", "))
}

/// Chỉ số sát biên: mảng/lát độ dài n; chỉ số n-1 (hợp lệ), n, n+1, 0, 2^32, MAX; biểu thức chỉ số đổi kiểu bao quanh.
fn gen_index(rng: &mut progen::Rng) -> Prog {
    let ty = *pick(rng, &INTS);
    let n = 1 + rng.below(8) as usize;
    let arr = array_lit(rng, ty, n);
    let idx = *pick(
        rng,
        &["i", "(i +% 1usize)", "(i -% 1usize)", "(i *% 2usize)"],
    );
    let src = if rng.chance(50) {
        format!(
            "ƒ entry(i: usize) → {t} ! ∅ {{ a ≔ {arr}; a[{idx}] }}\n",
            t = ty.name
        )
    } else {
        format!(
            "ƒ g(xs: &[{t}], i: usize) → {t} ! ∅ {{ xs[{idx}] }}\nƒ entry(i: usize) → {t} ! ∅ {{ a ≔ {arr}; g(&a[..], i) }}\n",
            t = ty.name
        )
    };
    let nn = n as i128;
    let mut ins: Vec<Vec<i128>> = [
        0,
        nn - 1,
        nn,
        nn + 1,
        nn - 2,
        4_294_967_296,
        i128::from(u64::MAX),
    ]
    .iter()
    .map(|v| vec![(*v).max(0)])
    .collect();
    ins.truncate(6);
    Prog {
        src,
        ret: ty,
        params: vec![USIZE],
        inputs: ins,
    }
}

/// Chia/dư: mẫu số 0 (trực tiếp hoặc do phép trừ/nhân bao quanh), MIN / -1.
fn gen_divrem(rng: &mut progen::Rng) -> Prog {
    let ty = *pick(rng, &INTS);
    let op = *pick(rng, &["/", "%"]);
    let c = 1 + rng.below(100) as i128;
    let c = c.min(ty.max());
    let den = match rng.below(4) {
        0 => "b".to_string(),
        1 => format!("(b -% {})", lit(ty, c)),
        2 => "(b *% b)".to_string(),
        _ => format!("(b +% {})", lit(ty, 1)),
    };
    let src = format!(
        "ƒ entry(a: {t}, b: {t}) → {t} ! ∅ {{ a {op} {den} }}\n",
        t = ty.name
    );
    let mut bs = vec![0, 1, c, c + 1, ty.max()];
    if ty.signed {
        bs.extend([-1, ty.min(), -c]);
    }
    let a_vals = [ty.min(), ty.max(), 0, 1, 7];
    let ins: Vec<Vec<i128>> = bs
        .iter()
        .take(6)
        .map(|b| {
            let a = *pick(rng, &a_vals);
            vec![a.clamp(ty.min(), ty.max()), (*b).clamp(ty.min(), ty.max())]
        })
        .collect();
    Prog {
        src,
        ret: ty,
        params: vec![ty, ty],
        inputs: ins,
    }
}

/// Tiền điều kiện bị vi phạm lúc chạy (M7 không chứng minh được ở chỗ gọi vì đối số là tham số của `entry`).
fn gen_pre(rng: &mut progen::Rng) -> Prog {
    let ty = *pick(rng, &INTS);
    let span = (ty.max() - ty.min()).min(1000);
    let k = (ty.min() + rng.below(span as u64 + 1) as i128).clamp(ty.min(), ty.max());
    let k = if rng.chance(50) {
        ty.max().min(k.abs() + 3)
    } else {
        k
    };
    let pre = match rng.below(4) {
        0 => format!("p < {}", lit(ty, k)),
        1 => format!("p >= {}", lit(ty, k)),
        2 => format!("p != {}", lit(ty, k)),
        _ => format!(
            "p <= {} ∧ p >= {}",
            lit(ty, k),
            lit(ty, (k - 5).max(ty.min()))
        ),
    };
    let t = ty.name;
    let src = if rng.chance(40) {
        // kèm tham số lát (tổ hợp từng gây FB-1)
        format!(
            "ƒ h(xs: &[{t}], p: {t}) → {t} ! ∅ ⊳ {pre} {{ p }}\nƒ entry(a: {t}) → {t} ! ∅ {{ ar ≔ [a, a]; h(&ar[..], a) }}\n"
        )
    } else {
        format!("ƒ h(p: {t}) → {t} ! ∅ ⊳ {pre} {{ p }}\nƒ entry(a: {t}) → {t} ! ∅ {{ h(a) }}\n")
    };
    let cands = [k - 1, k, k + 1, ty.min(), ty.max(), 0];
    let ins: Vec<Vec<i128>> = cands
        .iter()
        .map(|v| vec![(*v).clamp(ty.min(), ty.max())])
        .collect();
    Prog {
        src,
        ret: ty,
        params: vec![ty],
        inputs: ins,
    }
}

#[derive(Default, Debug)]
struct Tally {
    programs: u64,
    red_programs: u64,
    runs: u64,
    equal: u64,
    mismatch: u64,
    unsupported: u64,
    timeout: u64,
    interp_limit: u64,
    normal: u64,
    aborts: std::collections::BTreeMap<String, u64>,
    skipped: std::collections::BTreeMap<String, u64>,
    detail: Vec<String>,
}

fn run_prog(kind: &str, seed: u64, t: &mut Tally, dir: &std::path::Path) {
    let mut rng = progen::Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xA5A5 | 1);
    let p = match kind {
        "index" => gen_index(&mut rng),
        "divrem" => gen_divrem(&mut rng),
        _ => gen_pre(&mut rng),
    };
    t.programs += 1;
    for (k, input) in p.inputs.iter().enumerate() {
        t.runs += 1;
        let args: Vec<String> = input
            .iter()
            .zip(&p.params)
            .map(|(v, ty)| lit(*ty, *v))
            .collect();
        let src = format!(
            "{}ƒ main() → {} ! ∅ {{ entry({}) }}\n",
            p.src,
            p.ret.name,
            args.join(", ")
        );
        let inputs = [SourceInput {
            name: "main.gt".to_string(),
            text: src.clone(),
        }];
        let mut res = match check(&inputs, &Options::default()) {
            Ok(r) => r,
            Err(e) => {
                t.mismatch += 1;
                t.detail.push(format!(
                    "#{kind}{seed} {input:?}: đầu vào không hợp lệ: {e}\n{src}"
                ));
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
                t.mismatch += 1; // ICE trên chương trình hợp lệ = bug
                t.detail.push(format!("#{kind}{seed}: ICE\n{src}"));
            }
            if k == 0 {
                t.red_programs += 1;
                return;
            }
            continue;
        }
        let exe = dir.join(format!("a{seed}.exe"));
        {
            let art = res.open_gate().expect("cổng mở");
            let att = attestation(&res, &gt_driver::pipeline::mir_dump(art));
            if let Err(e) = build_exe_profile(art, &exe, true, &att, test_profile()) {
                if e.exit_code() == 3 {
                    t.unsupported += 1;
                } else {
                    t.mismatch += 1;
                    t.detail
                        .push(format!("#{kind}{seed} {input:?}: llvm lỗi {e}\n{src}"));
                }
                continue;
            }
        }
        let llvm = run_exe_timeout(&exe, Some(Duration::from_secs(20)));
        let _ = std::fs::remove_file(&exe);
        let interp = run_main(res.take_gated().expect("sản phẩm"));
        if interp.end == RunEnd::Limit {
            t.interp_limit += 1;
            continue;
        }
        if llvm.end == RunEnd::Limit {
            t.timeout += 1;
            t.detail
                .push(format!("#{kind}{seed} {input:?}: exe quá hạn\n{src}"));
            continue;
        }
        let same = match (&interp.end, &llvm.end) {
            (RunEnd::Normal, RunEnd::Normal) => norm_interp(&interp.value) == llvm.value,
            (RunEnd::Aborted(a), RunEnd::Aborted(b)) => a == b,
            _ => false,
        };
        if same {
            t.equal += 1;
            match &llvm.end {
                RunEnd::Aborted(kd) => *t.aborts.entry(kd.clone()).or_default() += 1,
                _ => t.normal += 1,
            }
        } else {
            t.mismatch += 1;
            t.detail.push(format!(
                "#{kind}{seed} {input:?}: thông dịch {:?}/{:?} ≠ llvm {:?}/{:?}\n{src}",
                interp.end, interp.value, llvm.end, llvm.value
            ));
        }
    }
}

fn summary(t: &Tally) -> String {
    format!(
        "{{\"programs\":{},\"red_programs\":{},\"runs\":{},\"equal\":{},\"mismatch\":{},\"normal\":{},\"aborts\":{:?},\"unsupported\":{},\"timeout\":{},\"interp_limit\":{},\"skipped\":{:?}}}",
        t.programs,
        t.red_programs,
        t.runs,
        t.equal,
        t.mismatch,
        t.normal,
        t.aborts,
        t.unsupported,
        t.timeout,
        t.interp_limit,
        t.skipped
    )
}

fn env_u64(k: &str, d: u64) -> u64 {
    std::env::var(k)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(d)
}

/// Lô nhắm đích (chạy tay). Mặc định bỏ qua.
#[test]
#[ignore = "chạy theo lô bằng GT_ABORT_KIND/START/COUNT/OUT"]
fn batch() {
    let kind = std::env::var("GT_ABORT_KIND").unwrap_or_else(|_| "index".to_string());
    let start = env_u64("GT_ABORT_START", 1);
    let count = env_u64("GT_ABORT_COUNT", 25);
    let dir = std::env::temp_dir().join(format!("gt-abort-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo");
    let mut t = Tally::default();
    for s in start..start + count {
        run_prog(&kind, s, &mut t, &dir);
    }
    let _ = std::fs::remove_dir_all(&dir);
    let line = format!(
        "{{\"suite\":\"abort-targeted\",\"kind\":\"{kind}\",\"start\":{start},\"count\":{count},\"result\":{}}}",
        summary(&t)
    );
    println!("{line}");
    if let Some(out) = std::env::var_os("GT_ABORT_OUT") {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(out)
            .expect("mở tệp kết quả");
        writeln!(f, "{line}").expect("ghi");
        for d in &t.detail {
            writeln!(f, "  DETAIL {d}").expect("ghi");
        }
    }
    assert_eq!(t.mismatch, 0, "bất đồng: {:#?}", t.detail);
}

/// Seed đã lộ bất đồng + lô rất nhỏ cố định cho cổng đầy đủ.
#[test]
fn permanent_and_tiny_batch() {
    let dir = std::env::temp_dir().join(format!("gt-abort-t-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo");
    let mut t = Tally::default();
    for (k, s) in PERMANENT {
        run_prog(k, *s, &mut t, &dir);
    }
    for k in ["index", "divrem", "pre"] {
        for s in 1..=2 {
            run_prog(k, s, &mut t, &dir);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    println!("{}", summary(&t));
    assert_eq!(t.mismatch, 0, "bất đồng: {:#?}", t.detail);
    assert!(t.equal >= 10, "quá ít phép so sánh: {}", summary(&t));
}
