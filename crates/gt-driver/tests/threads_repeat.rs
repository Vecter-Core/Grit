//! H8 (TCB `Mutex::{new,with}`, `spawn`): chạy LẶP exe THẬT ≥ 300 lần liên tiếp mỗi mẫu, mỗi hồ sơ (dev, release-o2): mẫu `threads` (4 luồng, Atomic + Mutex tĩnh) luôn ra 8000;
//! mẫu `two_locks` (2 luồng khoá lồng hai Mutex theo cấp tăng Lo→Hi) luôn ra 4000 và KHÔNG treo (hết giờ 30 s ⇒ lỗi); mẫu khoá sai thứ tự bị E0502 chặn TĨNH.
//! Kết quả từng lô ghi nối vào `STD_OUT`-kiểu: biến `THREADS_OUT=<đường dẫn tuyệt đối .jsonl>`.
//! Đây là bằng chứng THỰC NGHIỆM (không có ThreadSanitizer/loom): KHÔNG chứng minh vắng đua dữ liệu.
#![cfg(feature = "llvm")]
use gt_driver::native::{build_exe_profile, run_exe_timeout};
use gt_driver::pipeline::{attestation, mir_dump};
use gt_driver::{Options, RunEnd, check};
use gt_resolve::SourceInput;
use std::io::Write;
use std::time::{Duration, Instant};

const THREADS: &str = include_str!("../../../tests/samples/threads.gt");
const TWO_LOCKS: &str = include_str!("../../../tests/samples/two_locks.gt");

fn expect_of(src: &str) -> String {
    src.lines()
        .find_map(|l| l.strip_prefix("// expect-result: "))
        .expect("expect-result")
        .trim()
        .to_string()
}

fn build(src: &str, opt: gt_codegen_llvm::Opt, tag: &str) -> std::path::PathBuf {
    let inputs = [SourceInput {
        name: "main.gt".to_string(),
        text: src.to_string(),
    }];
    let mut res = check(&inputs, &Options::default()).expect("check");
    assert!(
        res.verdict.allows_output(),
        "{:?}",
        res.report
            .diagnostics
            .iter()
            .map(|d| (d.code.as_str().to_string(), d.message.clone()))
            .collect::<Vec<_>>()
    );
    let att = {
        let art = res.open_gate().expect("cổng");
        attestation(&res, &mir_dump(art))
    };
    let art = res.take_gated().expect("cổng");
    let dir = std::env::temp_dir().join(format!("gt-thr-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo");
    let exe = dir.join(format!("{tag}.exe"));
    build_exe_profile(&art, &exe, true, &att, opt).expect("dựng");
    exe
}

fn repeat(name: &str, src: &str, profile: &str, opt: gt_codegen_llvm::Opt, runs: usize) {
    let exe = build(src, opt, &format!("{name}_{}", profile.replace('-', "_")));
    let want = expect_of(src);
    let mut bad: Vec<String> = Vec::new();
    let (mut min, mut max) = (Duration::MAX, Duration::ZERO);
    let t0 = Instant::now();
    for k in 0..runs {
        let t = Instant::now();
        let r = run_exe_timeout(&exe, Some(Duration::from_secs(30)));
        let dt = t.elapsed();
        min = min.min(dt);
        max = max.max(dt);
        if r.end != RunEnd::Normal || r.value.as_deref() != Some(want.as_str()) {
            bad.push(format!("lần {k}: {:?} {:?}", r.end, r.value));
        }
    }
    let line = format!(
        "{{\"suite\":\"threads\",\"sample\":\"{name}\",\"profile\":\"{profile}\",\"runs\":{runs},\"bad\":{},\"min_ms\":{},\"max_ms\":{},\"total_s\":{:.1}}}",
        bad.len(),
        min.as_millis(),
        max.as_millis(),
        t0.elapsed().as_secs_f64()
    );
    println!("{line}");
    if let Some(out) = std::env::var_os("THREADS_OUT") {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(out)
            .expect("mở");
        writeln!(f, "{line}").expect("ghi");
    }
    assert!(
        bad.is_empty(),
        "{name}/{profile}: {} lần sai (kỳ vọng {want}): {:?}",
        bad.len(),
        &bad[..bad.len().min(5)]
    );
}

fn runs() -> usize {
    std::env::var("THREADS_RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300)
}

#[test]
fn threads_sample_always_gives_8000_over_300_consecutive_runs() {
    repeat("threads", THREADS, "dev", gt_codegen_llvm::Opt::Dev, runs());
    repeat(
        "threads",
        THREADS,
        "release-o2",
        gt_codegen_llvm::Opt::ReleaseO2,
        runs(),
    );
}

#[test]
fn two_ordered_locks_sample_always_gives_4000_and_never_hangs_over_300_runs() {
    repeat(
        "two_locks",
        TWO_LOCKS,
        "dev",
        gt_codegen_llvm::Opt::Dev,
        runs(),
    );
    repeat(
        "two_locks",
        TWO_LOCKS,
        "release-o2",
        gt_codegen_llvm::Opt::ReleaseO2,
        runs(),
    );
}

#[test]
fn locking_in_the_wrong_order_is_rejected_statically_with_e0502() {
    // đảo thứ tự: giữ Hi rồi lấy Lo
    let bad = TWO_LOCKS
        .replace(
            "ƒ inner(v: &!u64, n: u64) → u64 ! {lock(Hi)} {\n  *v ← *v +% 1;\n  HI.with(bump_hi, 1u64)\n}",
            "ƒ inner(v: &!u64, n: u64) → u64 ! {lock(Lo)} {\n  *v ← *v +% 1;\n  LO.with(bump_hi, 1u64)\n}",
        )
        .replace("LO.with(inner, 0u64);\n    i ← i + 1;", "HI.with(inner, 0u64);\n    i ← i + 1;");
    assert_ne!(bad, TWO_LOCKS, "thay thế không áp dụng");
    let inputs = [SourceInput {
        name: "main.gt".to_string(),
        text: bad,
    }];
    let res = check(&inputs, &Options::default()).expect("check");
    assert!(!res.verdict.allows_output());
    let codes: Vec<String> = res
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    assert!(codes.iter().any(|c| c == "E0502"), "{codes:?}");
}
