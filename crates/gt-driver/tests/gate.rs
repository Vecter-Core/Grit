//! CỔNG PHÁN QUYẾT trong tiến trình (ADR-028 §3, §5): ĐỎ/ICE ⇒ không có sản phẩm để ghi/chạy; VÀNG ⇒ có; pass bắt buộc; mặc định `--checks=all`;
//! corpus của mọi milestone chạy qua driver THẬT; báo cáo "check còn lại"; phân vùng mã thoát.
#[path = "../../gt-types/tests/common/mod.rs"]
mod fixture;

use gt_driver::pipeline::{PassRecord, PassStatus, REQUIRED_PASSES};
use gt_driver::render::{Phase, json_report};
use gt_driver::runner::{run_main, run_main_with};
use gt_driver::{
    DEFAULT_CHECKS, Fault, Options, RunEnd, Verdict, check, check_exit_code, decide, run_exit_code,
};
use gt_oblig::driver::Checks;
use gt_resolve::SourceInput;
use std::path::PathBuf;

fn input(src: &str) -> Vec<SourceInput> {
    vec![SourceInput {
        name: "main.gt".to_string(),
        text: src.to_string(),
    }]
}

fn verdict_of(src: &str, opts: &Options) -> Verdict {
    check(&input(src), opts).expect("đầu vào hợp lệ").verdict
}

const GOOD: &str = "ƒ main() → u32 ! ∅ { 1u32 + 2u32 }";
const RED: &str = "ƒ main() → u8 ! ∅ { x: u8 ≔ 300; x }";
const YELLOW: &str = "ƒ main() → &str ! ∅ { k ≔ \"AKIAIOSFODNN7EXAMPLE\"; k }";

/// Prelude có ĐÚNG MỘT nguồn: `lib/core/prelude.gt`. Hai fixture test (gt-types, gt-resolve) phải `include_str!` đúng tệp đó và KHÔNG còn bản chép tay
/// (trước đây có 3 bản; bản thứ ba của gt-resolve bị bỏ sót khi thêm `Out`/`root_out` và chỉ lộ ở cổng đầy đủ).
#[test]
fn prelude_has_a_single_source() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = std::fs::read_to_string(root.join("lib/core/prelude.gt")).expect("đọc prelude");
    assert_eq!(
        file.replace("\r\n", "\n"),
        gt_driver::pipeline::prelude_text().replace("\r\n", "\n"),
        "driver phải nhúng đúng lib/core/prelude.gt"
    );
    for fx in [
        "crates/gt-types/tests/common/mod.rs",
        "crates/gt-resolve/tests/common/mod.rs",
    ] {
        let t = std::fs::read_to_string(root.join(fx)).expect("đọc fixture");
        assert!(
            t.contains("include_str!(\"../../../../lib/core/prelude.gt\")"),
            "{fx} phải đọc prelude từ lib/core/prelude.gt"
        );
    }
    // không có bản chép tay nào khác trong mã Rust: dấu hiệu = chữ ký `root_fs` viết nguyên văn
    let mut stack = vec![root.join("crates")];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).expect("đọc thư mục").flatten() {
            let p = e.path();
            if p.is_dir() {
                if !matches!(e.file_name().to_str(), Some("target" | "fuzz")) {
                    stack.push(p);
                }
            } else if p.extension().is_some_and(|x| x == "rs") && !p.ends_with("gate.rs") {
                let t = std::fs::read_to_string(&p).unwrap_or_default();
                assert!(
                    !t.contains("ƒ root_fs() → Cap<Fs>"),
                    "{} chứa một bản chép tay của prelude",
                    p.display()
                );
            }
        }
    }
}

#[test]
fn default_checks_mode_is_keep_all() {
    assert_eq!(DEFAULT_CHECKS, Checks::All);
    assert_eq!(Options::default().checks, Checks::All);
    assert!(!Options::default().deny_yellow);
    assert!(Options::default().fault.is_none());
}

#[test]
fn red_and_ice_leave_nothing_to_write_or_run() {
    // ĐỎ
    let mut res = check(&input(RED), &Options::default()).expect("ok");
    assert_eq!(res.verdict, Verdict::Red);
    assert!(res.open_gate().is_none());
    assert!(res.take_gated().is_none(), "ĐỎ: không được lấy sản phẩm");
    assert_eq!(check_exit_code(res.verdict), 1);
    // ICE bằng cách chèn panic vào MỌI pass: mỗi lần đều ICE (kể cả pass cuối)
    for pass in REQUIRED_PASSES {
        let opts = Options {
            fault: Some(Fault::Panic(pass.to_string())),
            ..Options::default()
        };
        let mut r = check(&input(GOOD), &opts).expect("ok");
        assert_eq!(
            r.verdict,
            Verdict::Ice,
            "panic trong `{pass}` phải thành ICE"
        );
        assert!(r.open_gate().is_none(), "{pass}: ICE mà vẫn mở cổng");
        assert!(r.take_gated().is_none());
        assert_eq!(check_exit_code(r.verdict), 2);
        assert!(
            r.report
                .diagnostics
                .iter()
                .any(|d| d.code.as_str() == "E9901")
        );
    }
}

#[test]
fn skipping_a_required_pass_is_ice_never_green() {
    for pass in [
        "m4.types",
        "m5.mir",
        "m6.borrowck",
        "m7.obligations",
        "m8.termination",
        "m8.effects",
        "m8.conc",
        "m9.secrets",
        "selftest",
    ] {
        let opts = Options {
            fault: Some(Fault::Skip(pass.to_string())),
            ..Options::default()
        };
        let v = verdict_of(GOOD, &opts);
        assert_eq!(v, Verdict::Ice, "bỏ `{pass}` mà báo sạch là fail-open");
    }
}

#[test]
fn hard_conditions_e0601_and_e0307_are_required_passes() {
    assert!(REQUIRED_PASSES.contains(&"m7.const_bounds"));
    assert!(REQUIRED_PASSES.contains(&"m5.return_paths"));
    // báo cáo SẠCH nhưng thiếu pass bắt buộc ⇒ ICE
    let empty = gt_diag::Diagnostics::new(10).finish();
    let all_done: Vec<PassRecord> = REQUIRED_PASSES
        .iter()
        .map(|n| PassRecord {
            name: n,
            status: PassStatus::Done,
        })
        .collect();
    assert_eq!(decide(&empty, &all_done, false), Verdict::Green);
    for missing in REQUIRED_PASSES {
        let without: Vec<PassRecord> = all_done
            .iter()
            .filter(|p| p.name != missing)
            .cloned()
            .collect();
        assert_eq!(
            decide(&empty, &without, false),
            Verdict::Ice,
            "thiếu {missing}"
        );
        let skipped: Vec<PassRecord> = all_done
            .iter()
            .map(|p| PassRecord {
                name: p.name,
                status: if p.name == missing {
                    PassStatus::Skipped
                } else {
                    PassStatus::Done
                },
            })
            .collect();
        assert_eq!(
            decide(&empty, &skipped, false),
            Verdict::Ice,
            "bỏ {missing}"
        );
    }
    // một pass PHỤ hỏng (mọi pass bắt buộc đều xong, báo cáo sạch) vẫn là ICE: không được coi là "sạch"
    let mut with_extra = all_done.clone();
    with_extra.push(PassRecord {
        name: "extra.optional",
        status: PassStatus::Failed,
    });
    assert_eq!(decide(&empty, &with_extra, false), Verdict::Ice);
    let mut with_skipped = all_done.clone();
    with_skipped.push(PassRecord {
        name: "extra.optional",
        status: PassStatus::Skipped,
    });
    assert_eq!(
        decide(&empty, &with_skipped, false),
        Verdict::Green,
        "pass phụ bị bỏ qua không làm hỏng phán quyết"
    );
    // một pass hỏng ⇒ ICE dù báo cáo sạch
    let mut failed = all_done.clone();
    failed[3].status = PassStatus::Failed;
    assert_eq!(decide(&empty, &failed, false), Verdict::Ice);
}

#[test]
fn yellow_opens_the_gate_and_deny_yellow_closes_it() {
    let mut res = check(&input(YELLOW), &Options::default()).expect("ok");
    assert_eq!(res.verdict, Verdict::Yellow);
    assert_eq!(check_exit_code(res.verdict), 0);
    assert!(res.report.warnings() >= 1);
    assert!(res.open_gate().is_some(), "VÀNG vẫn cho chạy");
    let art = res.take_gated().expect("sản phẩm");
    let rr = run_main(art);
    assert_eq!(rr.end, RunEnd::Normal);
    // --deny-yellow nâng VÀNG thành ĐỎ: không sản phẩm
    let opts = Options {
        deny_yellow: true,
        ..Options::default()
    };
    let mut res = check(&input(YELLOW), &opts).expect("ok");
    assert_eq!(res.verdict, Verdict::Red);
    assert_eq!(check_exit_code(res.verdict), 1);
    assert!(res.open_gate().is_none());
    assert!(res.take_gated().is_none());
}

#[test]
fn run_outcomes_and_the_partitioned_exit_codes() {
    // bình thường
    let mut r = check(&input(GOOD), &Options::default()).expect("ok");
    let rr = run_main(r.take_gated().expect("art"));
    assert_eq!(rr.end, RunEnd::Normal);
    assert_eq!(run_exit_code(&rr.end), 0);
    assert!(rr.value.as_deref().is_some_and(|v| v.contains("Int(3")));
    // abort lúc chạy: tràn cộng trong vòng lặp mà bộ giải không chứng minh được (check được giữ)
    let abort = "ƒ main() → u8 ! ∅ { μs ≔ 250u8; ⟳ i in 0..10u32 { s ← s + 1; } s }";
    let mut r = check(&input(abort), &Options::default()).expect("ok");
    assert!(
        r.verdict.allows_output(),
        "{:?}",
        r.report
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect::<Vec<_>>()
    );
    let rr = run_main(r.take_gated().expect("art"));
    assert!(matches!(rr.end, RunEnd::Aborted(_)), "{:?}", rr.end);
    assert_eq!(run_exit_code(&rr.end), 70);
    // hết giới hạn của bộ chạy
    let spin = "ƒ main() ! {div} { ⟳ { } }";
    let mut r = check(&input(spin), &Options::default()).expect("ok");
    let rr = run_main_with(r.take_gated().expect("art"), 10_000);
    assert_eq!(rr.end, RunEnd::Limit);
    assert_eq!(run_exit_code(&rr.end), 71);
    // KHÔNG có main
    let mut r = check(&input("ƒ f() → u32 ! ∅ { 1u32 }"), &Options::default()).expect("ok");
    let rr = run_main(r.take_gated().expect("art"));
    assert!(matches!(rr.end, RunEnd::Internal(_)));
}

#[test]
fn exit_code_zones_do_not_overlap() {
    use gt_driver::RunEnd::{Aborted, Internal, Limit, Normal, ProgramExit};
    let check_zone = [
        check_exit_code(Verdict::Green),
        check_exit_code(Verdict::Yellow),
        check_exit_code(Verdict::Red),
        check_exit_code(Verdict::Ice),
    ];
    assert_eq!(check_zone, [0, 0, 1, 2]);
    // chương trình KHÔNG BAO GIỜ có thể ra mã 1, 2 hay 3 (vùng của gritc): "chương trình trả 1" ≠ "gritc báo ĐỎ"
    assert_eq!(run_exit_code(&Normal), 0);
    assert_eq!(run_exit_code(&ProgramExit(0)), 0);
    assert_eq!(run_exit_code(&Aborted("overflow".into())), 70);
    assert_eq!(run_exit_code(&Limit), 71);
    for s in 1..=255u8 {
        let c = run_exit_code(&ProgramExit(s));
        assert!(
            (101..=200).contains(&c),
            "trạng thái {s} ↦ {c} ra ngoài vùng chương trình"
        );
    }
    assert_eq!(
        run_exit_code(&ProgramExit(1)),
        101,
        "chương trình trả 1 ⇒ 101, không phải 1"
    );
    // lỗi nội bộ của bộ chạy là lỗi của gritc ⇒ 2
    assert_eq!(run_exit_code(&Internal("x".into())), 2);
}

// ----------------------------------------------------------------- corpus qua driver thật

fn tests_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests")
}

fn gt_files(sub: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(tests_dir().join(sub))
        .expect("thư mục")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "gt"))
        .collect();
    v.sort();
    v
}

fn read(p: &std::path::Path) -> String {
    std::fs::read_to_string(p)
        .expect("đọc")
        .replace("\r\n", "\n")
}

fn name_of(p: &std::path::Path) -> String {
    p.file_name().expect("tên").to_string_lossy().into_owned()
}

#[test]
fn every_fail_sample_is_red_through_the_real_driver_with_its_code() {
    let mut n = 0;
    let mut bad = Vec::new();
    for p in gt_files("fail") {
        let src = read(&p);
        let Some(first) = src.lines().next() else {
            continue;
        };
        let Some(code) = first.strip_prefix("// expect: fail ") else {
            continue;
        };
        let code = code.trim();
        let name = name_of(&p);
        let inputs = vec![SourceInput {
            name: "main.gt".to_string(),
            text: src.clone(),
        }];
        let Ok(res) = check(&inputs, &Options::default()) else {
            // lỗi đầu vào của resolver (vd tệp lỗi ký tự) cũng là từ chối
            n += 1;
            continue;
        };
        n += 1;
        let codes: Vec<&str> = res
            .report
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect();
        if res.verdict != Verdict::Red && res.verdict != Verdict::Ice {
            bad.push(format!(
                "{name}: kỳ vọng ĐỎ {code}, phán quyết {:?}",
                res.verdict
            ));
        } else if !codes.contains(&code) {
            bad.push(format!("{name}: thiếu mã {code}, có {codes:?}"));
        } else if res.open_gate().is_some() {
            bad.push(format!("{name}: ĐỎ mà cổng mở"));
        }
    }
    assert!(n >= 300, "chỉ {n} mẫu fail");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn hard_condition_samples_are_refused_by_the_driver() {
    let mut e0601 = 0;
    let mut e0307 = 0;
    for p in gt_files("fail") {
        let name = name_of(&p);
        let want = if name.starts_with("E0601_") {
            e0601 += 1;
            "E0601"
        } else if name.starts_with("E0307_") {
            e0307 += 1;
            "E0307"
        } else {
            continue;
        };
        let res = check(&input(&read(&p)), &Options::default()).expect("ok");
        assert_eq!(res.verdict, Verdict::Red, "{name}");
        assert!(
            res.report
                .diagnostics
                .iter()
                .any(|d| d.code.as_str() == want),
            "{name}"
        );
        assert!(
            res.open_gate().is_none(),
            "{name}: phải từ chối, không sinh sản phẩm"
        );
    }
    assert!(e0601 >= 3 && e0307 >= 1, "mẫu E0601={e0601}, E0307={e0307}");
}

#[test]
fn every_pass_sample_is_accepted_by_the_driver() {
    let mut n = 0;
    let mut bad = Vec::new();
    for p in gt_files("pass") {
        let src = read(&p);
        let res = check(&input(&src), &Options::default()).expect("ok");
        n += 1;
        if !res.verdict.allows_output() {
            bad.push(format!(
                "{}: {:?} {:?}",
                name_of(&p),
                res.verdict,
                res.report
                    .diagnostics
                    .iter()
                    .map(|d| d.code.as_str())
                    .collect::<Vec<_>>()
            ));
        }
    }
    assert!(n >= 300, "chỉ {n} mẫu pass");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn every_warn_sample_is_yellow_and_still_opens_the_gate() {
    let mut n = 0;
    for p in gt_files("warn") {
        let src = read(&p);
        let Some(first) = src.lines().next() else {
            continue;
        };
        let Some(code) = first.strip_prefix("// expect: warn ") else {
            continue;
        };
        let res = check(&input(&src), &Options::default()).expect("ok");
        assert_eq!(res.verdict, Verdict::Yellow, "{}", name_of(&p));
        assert!(
            res.report
                .diagnostics
                .iter()
                .any(|d| d.code.as_str() == code.trim()),
            "{}",
            name_of(&p)
        );
        assert!(res.open_gate().is_some());
        n += 1;
    }
    assert!(n >= 5, "chỉ {n} mẫu warn");
}

// ----------------------------------------------------------------- báo cáo check còn lại

const PROVABLE: &str = "ƒ main() → u32 ! ∅ { xs ≔ [1u32, 2u32, 3u32]; μs ≔ 0u32; μi ≔ 0; ⟳ i < 3 { s ← s +% xs[i]; i ← i + 1; } s }\nƒ g(a: u8) → u32 ! ∅ { ⎇ a < 100 { (a + 1) as u32 } ⇝ { 0 } }";

#[test]
fn remaining_checks_report_differs_between_modes_exactly_as_specified() {
    let all = check(&input(PROVABLE), &Options::default()).expect("ok");
    let proven = check(
        &input(PROVABLE),
        &Options {
            checks: Checks::Proven,
            ..Options::default()
        },
    )
    .expect("ok");
    let (ra, rp) = (
        all.checks_report.clone().expect("báo cáo"),
        proven.checks_report.clone().expect("báo cáo"),
    );
    assert_eq!(ra.mode, "all");
    assert_eq!(rp.mode, "proven");
    // ở `all`: không xoá gì, `would_delete` = đúng những gì `proven` xoá
    assert!(ra.deleted.is_empty());
    assert!(
        !ra.would_delete.is_empty(),
        "chương trình mẫu có check chứng minh được"
    );
    assert_eq!(ra.would_delete.len(), rp.deleted.len());
    let ids =
        |v: &[gt_driver::render::CheckItem]| v.iter().map(|c| c.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&ra.would_delete), ids(&rp.deleted));
    // ở `proven`: không còn `would_delete`
    assert!(rp.would_delete.is_empty());
    assert!(!rp.deleted.is_empty());
    // check giữ vì chưa chứng minh được có mặt ở cả hai chế độ như nhau
    let unproven = |r: &gt_driver::render::ChecksReport| {
        r.kept
            .iter()
            .filter(|c| c.status == "kept_not_proven")
            .count()
    };
    assert_eq!(unproven(&ra), unproven(&rp));
    // MIR: `all` giữ đủ Assert, `proven` ít hơn
    let n = |r: &gt_driver::CheckResult| -> usize {
        r.open_gate()
            .expect("mở")
            .obl
            .bodies
            .iter()
            .map(gt_oblig::driver::count_asserts)
            .sum()
    };
    assert!(n(&all) > n(&proven));
}

#[test]
fn json_report_carries_backend_phase_checks_and_is_well_formed() {
    let res = check(&input(PROVABLE), &Options::default()).expect("ok");
    let j = json_report(&res, false, Phase::Check, None, Some("abc123"));
    assert!(j.contains("\"backend\":\"interpreter\""), "{j}");
    assert!(j.contains("\"phase\":\"check\""));
    assert!(j.contains("\"verdict\":\"green\""));
    assert!(j.contains("\"checks\":{\"mode\":\"all\""));
    assert!(j.contains("\"would_delete\":[{"));
    assert!(j.contains("\"attestation\":\"abc123\""));
    assert!(j.contains("\"schema_version\":1"));
    assert!(well_formed(&j), "JSON không hợp lệ: {j}");
    // chạy: có khối `run`
    let mut r = check(&input(PROVABLE), &Options::default()).expect("ok");
    let rr = run_main(r.take_gated().expect("art"));
    let j = json_report(&r, false, Phase::Run, Some(&rr), None);
    assert!(
        j.contains("\"phase\":\"run\"") && j.contains("\"run\":{\"exit_status\":0"),
        "{j}"
    );
    assert!(well_formed(&j));
    // ĐỎ vẫn có backend và JSON hợp lệ
    let red = check(&input(RED), &Options::default()).expect("ok");
    let j = json_report(&red, false, Phase::Check, None, None);
    assert!(j.contains("\"verdict\":\"red\"") && j.contains("\"backend\":\"interpreter\""));
    assert!(well_formed(&j));
}

/// Bộ kiểm JSON tối giản (đệ quy xuống): đủ để bắt dấu phẩy thừa/thiếu, ngoặc lệch, chuỗi chưa đóng.
fn well_formed(s: &str) -> bool {
    fn ws(b: &[u8], i: &mut usize) {
        while *i < b.len() && matches!(b[*i], b' ' | b'\n' | b'\r' | b'\t') {
            *i += 1;
        }
    }
    fn val(b: &[u8], i: &mut usize) -> bool {
        ws(b, i);
        if *i >= b.len() {
            return false;
        }
        match b[*i] {
            b'{' => {
                *i += 1;
                ws(b, i);
                if *i < b.len() && b[*i] == b'}' {
                    *i += 1;
                    return true;
                }
                loop {
                    ws(b, i);
                    if !string(b, i) {
                        return false;
                    }
                    ws(b, i);
                    if *i >= b.len() || b[*i] != b':' {
                        return false;
                    }
                    *i += 1;
                    if !val(b, i) {
                        return false;
                    }
                    ws(b, i);
                    match b.get(*i) {
                        Some(b',') => *i += 1,
                        Some(b'}') => {
                            *i += 1;
                            return true;
                        }
                        _ => return false,
                    }
                }
            }
            b'[' => {
                *i += 1;
                ws(b, i);
                if *i < b.len() && b[*i] == b']' {
                    *i += 1;
                    return true;
                }
                loop {
                    if !val(b, i) {
                        return false;
                    }
                    ws(b, i);
                    match b.get(*i) {
                        Some(b',') => *i += 1,
                        Some(b']') => {
                            *i += 1;
                            return true;
                        }
                        _ => return false,
                    }
                }
            }
            b'"' => string(b, i),
            b't' | b'f' | b'n' => {
                for lit in [&b"true"[..], b"false", b"null"] {
                    if b[*i..].starts_with(lit) {
                        *i += lit.len();
                        return true;
                    }
                }
                false
            }
            _ => {
                let st = *i;
                while *i < b.len()
                    && matches!(b[*i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                {
                    *i += 1;
                }
                *i > st
            }
        }
    }
    fn string(b: &[u8], i: &mut usize) -> bool {
        if b.get(*i) != Some(&b'"') {
            return false;
        }
        *i += 1;
        while *i < b.len() {
            match b[*i] {
                b'\\' => *i += 2,
                b'"' => {
                    *i += 1;
                    return true;
                }
                _ => *i += 1,
            }
        }
        false
    }
    let b = s.as_bytes();
    let mut i = 0;
    let ok = val(b, &mut i);
    ws(b, &mut i);
    ok && i == b.len()
}

#[test]
fn well_formed_checker_itself_rejects_broken_json() {
    assert!(well_formed("{\"a\":[1,2,{\"b\":null}],\"c\":\"x\\\"y\"}"));
    assert!(!well_formed("{\"a\":[1,2,]}"));
    assert!(!well_formed("{\"a\":1,}"));
    assert!(!well_formed("{\"a\":\"x}"));
    assert!(!well_formed("{\"a\":1}}"));
}

#[test]
fn n1401_samples_emit_their_note_and_s_pass_samples_stay_silent() {
    for p in gt_files("pass") {
        let name = name_of(&p);
        let res = check(&input(&read(&p)), &Options::default()).expect("ok");
        let codes: Vec<&str> = res
            .report
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect();
        if name.starts_with("N1401_") {
            assert!(
                codes.contains(&"N1401"),
                "{name}: thiếu ghi chú N1401, có {codes:?}"
            );
            assert_eq!(res.verdict, Verdict::Green, "{name}");
        }
        for c in ["S101", "S102", "S201", "S202"] {
            if name.starts_with(&format!("{c}_ok_")) {
                assert!(
                    !codes.contains(&c),
                    "{name}: không được phát {c}: {codes:?}"
                );
                assert_eq!(res.verdict, Verdict::Green, "{name}: {codes:?}");
            }
        }
    }
}

#[test]
fn allowed_findings_are_reported_for_audit_in_json() {
    let src = "#allow(S101, \"khoá mẫu của tài liệu\")\nƒ f() → &str ! ∅ { k ≔ \"AKIAIOSFODNN7EXAMPLE\"; k }";
    let res = check(&input(src), &Options::default()).expect("ok");
    assert_eq!(res.verdict, Verdict::Green);
    assert_eq!(res.allowed.len(), 1);
    let j = json_report(&res, false, Phase::Check, None, None);
    assert!(
        j.contains("\"allowed\":[{\"code\":\"S101\",\"reason\":\"khoá mẫu của tài liệu\""),
        "{j}"
    );
    assert!(well_formed(&j));
}

fn status_of(res: &gt_driver::CheckResult, name: &str) -> PassStatus {
    res.passes
        .iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("không có pass {name}"))
        .status
}

#[test]
fn injected_faults_hit_exactly_the_named_alias_pass() {
    for (alias, other) in [
        ("m5.return_paths", "m7.const_bounds"),
        ("m7.const_bounds", "m5.return_paths"),
    ] {
        let skip = Options {
            fault: Some(Fault::Skip(alias.to_string())),
            ..Options::default()
        };
        let r = check(&input(GOOD), &skip).expect("ok");
        assert_eq!(status_of(&r, alias), PassStatus::Skipped, "{alias}");
        assert_eq!(
            status_of(&r, other),
            PassStatus::Done,
            "{other} không được bị ảnh hưởng"
        );
        let panic = Options {
            fault: Some(Fault::Panic(alias.to_string())),
            ..Options::default()
        };
        let r = check(&input(GOOD), &panic).expect("ok");
        assert_eq!(status_of(&r, alias), PassStatus::Failed, "{alias}");
        assert_eq!(status_of(&r, other), PassStatus::Done, "{other}");
        assert_eq!(r.verdict, Verdict::Ice);
    }
    // không lỗi chèn: mọi pass xong
    let r = check(&input(GOOD), &Options::default()).expect("ok");
    assert!(r.passes.iter().all(|p| p.status == PassStatus::Done));
}

#[test]
fn full_gritc_json_has_exactly_the_schema_properties() {
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../docs/spec/diag_schema.json"),
        )
        .expect("đọc schema"),
    )
    .expect("schema hợp lệ");
    let mut want: Vec<String> = schema["properties"]
        .as_object()
        .expect("properties")
        .keys()
        .cloned()
        .collect();
    want.sort();
    // báo cáo ĐẦY ĐỦ: pha chạy (có `run`) và có attestation
    let mut r = check(&input(PROVABLE), &Options::default()).expect("ok");
    let rr = run_main(r.take_gated().expect("art"));
    let j = json_report(&r, false, Phase::Run, Some(&rr), Some("abc"));
    let v: serde_json::Value = serde_json::from_str(&j).expect("JSON hợp lệ");
    let mut got: Vec<String> = v.as_object().expect("obj").keys().cloned().collect();
    got.sort();
    assert_eq!(got, want, "khóa cấp cao của gritc lệch schema");
    // khối `checks`/`run` có đúng các khóa đã mô tả
    for k in ["mode", "kept", "deleted", "would_delete"] {
        assert!(v["checks"].get(k).is_some(), "checks.{k}");
    }
    for k in ["exit_status", "aborted", "abort_kind", "value"] {
        assert!(v["run"].get(k).is_some(), "run.{k}");
    }
    // pha check: không có `run` và không có `attestation` (đều tuỳ chọn)
    let c = check(&input(PROVABLE), &Options::default()).expect("ok");
    let v: serde_json::Value =
        serde_json::from_str(&json_report(&c, false, Phase::Check, None, None)).expect("JSON");
    assert!(v.get("run").is_none() && v.get("attestation").is_none());
    for k in ["backend", "phase", "verdict", "passes", "checks", "allowed"] {
        assert!(v.get(k).is_some(), "{k} phải luôn có");
    }
}
