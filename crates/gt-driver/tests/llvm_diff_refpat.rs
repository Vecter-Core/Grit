//! Bộ so sánh codegen cho KHỚP MẪU QUA THAM CHIẾU (ADR-040, Q87): chương trình từ `refgen` (có MÔ HÌNH Rust độc lập) chạy ở thông dịch VÀ exe LLVM ở hồ sơ
//! `GT_PROFILE` (dev | release | release-o2); cả ba (mô hình, thông dịch, LLVM) phải bằng nhau; ca ĐỎ có chủ ý phải ra đúng mã.
//! Lô: `GT_DIFF_START=1 GT_DIFF_COUNT=25 GT_DIFF_OUT=D:/Grit/docs/RELEASE_O2_DIFF_RESULTS.jsonl (đường dẫn TUYỆT ĐỐI) GT_PROFILE=release-o2 cargo test -p gt-driver --test llvm_diff_refpat -- --ignored batch`
#![cfg(feature = "llvm")]
#[path = "support/refgen.rs"]
mod refgen;

use gt_driver::native::{build_exe_profile, run_exe_timeout};
use gt_driver::pipeline::{attestation, mir_dump};
use gt_driver::runner::run_main;
use gt_driver::{Options, RunEnd, check};
use gt_resolve::SourceInput;
use refgen::{Expect, program};
use std::io::Write;
use std::time::Duration;

fn profile() -> (gt_codegen_llvm::Opt, &'static str) {
    match std::env::var("GT_PROFILE").as_deref() {
        Ok("release") => (gt_codegen_llvm::Opt::Release, "release"),
        Ok("release-o2") => (gt_codegen_llvm::Opt::ReleaseO2, "release-o2"),
        _ => (gt_codegen_llvm::Opt::Dev, "dev"),
    }
}

#[derive(Default, Debug)]
struct Tally {
    programs: u64,
    equal: u64,
    expected_red: u64,
    mismatch: u64,
    detail: Vec<String>,
    forms: std::collections::BTreeMap<&'static str, u64>,
}

fn one(seed: u64, dir: &std::path::Path, t: &mut Tally) {
    let p = program(seed);
    t.programs += 1;
    let inputs = [SourceInput {
        name: "main.gt".to_string(),
        text: p.src.clone(),
    }];
    let mut res = check(&inputs, &Options::default()).expect("check");
    match &p.expect {
        Expect::Red(code) => {
            let got: Vec<String> = res
                .report
                .diagnostics
                .iter()
                .filter(|d| d.level == gt_diag::Level::Error)
                .map(|d| d.code.as_str().to_string())
                .collect();
            if !res.verdict.allows_output() && got.first().map(String::as_str) == Some(*code) {
                t.expected_red += 1;
            } else {
                t.mismatch += 1;
                t.detail.push(format!(
                    "#{seed}: kỳ vọng ĐỎ {code}, nhận {got:?}\n{}",
                    p.src
                ));
            }
        }
        Expect::Value(want) => {
            if !res.verdict.allows_output() {
                t.mismatch += 1;
                let d: Vec<String> = res
                    .report
                    .diagnostics
                    .iter()
                    .map(|d| format!("{} {}", d.code.as_str(), d.message))
                    .collect();
                t.detail
                    .push(format!("#{seed}: bị ĐỎ/ICE {d:?}\n{}", p.src));
                return;
            }
            let (opt, _) = profile();
            let att = {
                let art = res.open_gate().expect("cổng");
                attestation(&res, &mir_dump(art))
            };
            let exe = dir.join(format!("r{seed}.exe"));
            let art = res.take_gated().expect("cổng");
            if let Err(e) = build_exe_profile(&art, &exe, true, &att, opt) {
                t.mismatch += 1;
                t.detail.push(format!(
                    "#{seed}: backend lỗi (mã {}): {e}\n{}",
                    e.exit_code(),
                    p.src
                ));
                return;
            }
            let llvm = run_exe_timeout(&exe, Some(Duration::from_secs(20)));
            let _ = std::fs::remove_file(&exe);
            let interp = run_main(art);
            let iv = interp.value.as_deref().and_then(|s| {
                s.strip_prefix("Int(")
                    .and_then(|x| x.split(',').next())
                    .map(str::to_string)
            });
            let lv = llvm.value.clone();
            let w = Some(want.to_string());
            if interp.end == RunEnd::Normal && llvm.end == RunEnd::Normal && iv == w && lv == w {
                t.equal += 1;
            } else {
                t.mismatch += 1;
                t.detail.push(format!(
                    "#{seed}: mô hình {want} ≠ thông dịch {:?}/{iv:?} / llvm {:?}/{lv:?}\n{}",
                    interp.end, llvm.end, p.src
                ));
            }
        }
    }
}

fn count_forms(t: &mut Tally, p: &refgen::RefProg) {
    // độ phủ theo loại: chỉ tính chương trình ĐÃ SO SÁNH (bằng mô hình ở cả thông dịch và LLVM) hoặc ĐỎ đúng mã
    let set: std::collections::BTreeSet<&'static str> = p.forms.iter().copied().collect();
    for f in set {
        *t.forms.entry(f).or_default() += 1;
    }
}

fn run(start: u64, count: u64) -> Tally {
    let dir = std::env::temp_dir().join(format!("gt-refpat-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo thư mục");
    let mut t = Tally::default();
    for s in start..start + count {
        let before = t.equal + t.expected_red;
        one(s, &dir, &mut t);
        if t.equal + t.expected_red > before {
            count_forms(&mut t, &program(s));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    t
}

#[test]
fn a_small_default_run_agrees_with_the_model_in_the_chosen_profile() {
    let t = run(1, 12);
    println!("{t:?}");
    assert_eq!(t.mismatch, 0, "{}", t.detail.join("\n=====\n"));
    assert!(t.equal >= 8 && t.expected_red >= 2, "{t:?}");
}

#[test]
#[ignore = "chạy theo lô bằng GT_DIFF_START/COUNT/OUT"]
fn batch() {
    let start: u64 = std::env::var("GT_DIFF_START")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let count: u64 = std::env::var("GT_DIFF_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(25);
    let t = run(start, count);
    let line = format!(
        "{{\"suite\":\"refpat\",\"v\":2,\"profile\":\"{}\",\"start\":{start},\"count\":{count},\"programs\":{},\"equal\":{},\"expected_red\":{},\"mismatch\":{},\"forms\":{:?}}}",
        profile().1,
        t.programs,
        t.equal,
        t.expected_red,
        t.mismatch,
        t.forms
    );
    println!("{line}");
    if let Some(out) = std::env::var_os("GT_DIFF_OUT") {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(out)
            .expect("mở tệp kết quả");
        writeln!(f, "{line}").expect("ghi");
    }
    assert_eq!(t.mismatch, 0, "{}", t.detail.join("\n=====\n"));
}

/// ca ĐỎ có chủ ý (không phụ thuộc hồ sơ/backend): mỗi loại ≥ 30 chương trình phải ra ĐÚNG mã đầu tiên
#[test]
fn intentional_red_programs_are_rejected_with_the_expected_code_at_least_30_per_kind() {
    let mut per: std::collections::BTreeMap<&'static str, u64> = std::collections::BTreeMap::new();
    for seed in 1..=240u64 {
        let p = refgen::red(seed);
        let Expect::Red(code) = &p.expect else {
            panic!("không phải ca đỏ")
        };
        let inputs = [SourceInput {
            name: "main.gt".to_string(),
            text: p.src.clone(),
        }];
        let res = check(&inputs, &Options::default()).expect("check");
        let got: Vec<String> = res
            .report
            .diagnostics
            .iter()
            .filter(|d| d.level == gt_diag::Level::Error)
            .map(|d| d.code.as_str().to_string())
            .collect();
        assert!(!res.verdict.allows_output(), "{}", p.src);
        assert_eq!(got.first().map(String::as_str), Some(*code), "{}", p.src);
        *per.entry(p.forms[0]).or_default() += 1;
    }
    println!("{per:?}");
    assert_eq!(per.len(), 4);
    assert!(per.values().all(|n| *n >= 30), "{per:?}");
}
