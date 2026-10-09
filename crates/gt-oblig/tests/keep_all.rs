//! Chế độ GIỮ MỌI CHECK (`Checks::All`, ADR-028 §2; GRIT_PLAN §7 điểm 10): không xoá `Assert` nào kể cả đã chứng minh; chèn cả kiểm hợp đồng
//! đã chứng minh mà kiểm được lúc chạy; nghĩa vụ không chứng minh được và không kiểm được lúc chạy vẫn ĐỎ ở MỌI chế độ (D25).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use common::*;
use gt_mir::interp::{Interp, Outcome, V};
use gt_oblig::driver::{Checks, OblOutput, analyze_program_with, count_asserts};
use gt_oblig::prover::ZoneProver;
use std::path::PathBuf;

fn analyze(src: &str, mode: Checks) -> (OblOutput, gt_mir::MirOutput) {
    let (t, m) = mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        mir_codes(&m)
    );
    let o = analyze_program_with(&t, &m, &mut ZoneProver::new(24), mode);
    (o, m)
}

fn total_asserts(m: &gt_mir::MirOutput) -> usize {
    m.bodies.iter().map(count_asserts).sum()
}

#[test]
fn keep_all_deletes_no_assert_while_proven_mode_does() {
    // mọi phép cộng/chỉ số dưới đây đều chứng minh được
    let src = "ƒ f(xs: &[u32]) → u32 ! ∅ { μs ≔ 0u32; μi ≔ 0; ⟳ i < xs.len() { s ← s +% xs[i]; i ← i + 1; } s }\nƒ g(a: u8) → u32 ! ∅ { ⎇ a < 100 { (a + 1) as u32 } ⇝ { 0 } }";
    let (all, m) = analyze(src, Checks::All);
    let (proven, _) = analyze(src, Checks::Proven);
    let before = total_asserts(&m);
    let after_all: usize = all.bodies.iter().map(count_asserts).sum();
    let after_proven: usize = proven.bodies.iter().map(count_asserts).sum();
    assert!(before > 0);
    assert_eq!(after_all, before, "giữ mọi check: số Assert không đổi");
    assert!(
        after_proven < before,
        "chế độ proven phải xoá được ít nhất một check ({after_proven} < {before})"
    );
    assert_eq!(
        all.stats.obligations, proven.stats.obligations,
        "cùng số nghĩa vụ được phân tích"
    );
    assert_eq!(
        all.stats.discharged, proven.stats.discharged,
        "cùng số nghĩa vụ chứng minh được"
    );
}

#[test]
fn keep_all_also_inserts_runtime_checks_for_proven_contracts() {
    // `⊳ x > 0` chứng minh được tại nơi gọi ⇒ proven: không chèn; all: vẫn chèn kiểm hợp đồng lúc chạy
    let src = "ƒ pos(x: i32) → i32 ! ∅ ⊳ x > 0 { x }\nƒ caller() → i32 ! ∅ { pos(5) }";
    let (all, m) = analyze(src, Checks::All);
    let (proven, _) = analyze(src, Checks::Proven);
    let n = |o: &OblOutput| o.bodies.iter().map(count_asserts).sum::<usize>();
    assert!(
        n(&all) > n(&proven),
        "all phải có thêm kiểm hợp đồng: {} vs {}",
        n(&all),
        n(&proven)
    );
    assert!(n(&all) >= total_asserts(&m));
}

#[test]
fn both_modes_agree_on_behaviour_for_valid_runs_and_on_aborts() {
    let src = "ƒ div(a: u32, b: u32) → u32 ! ∅ { ⎇ b > 0 { a / b } ⇝ { 0 } }\nƒ idx(xs: &[u32], i: usize) → u32 ! ∅ { xs[i] }";
    for mode in [Checks::All, Checks::Proven] {
        let (o, _) = analyze(src, mode);
        let m2 = gt_mir::MirOutput {
            bodies: o.bodies.clone(),
            ..mir(src).1
        };
        let mut it = Interp::new(&m2, 10_000);
        assert!(matches!(
            it.call_key(
                "main::div",
                vec![
                    V::Int(9, gt_types::IntTy::U32),
                    V::Int(3, gt_types::IntTy::U32)
                ]
            ),
            Outcome::Value(V::Int(3, _))
        ));
        let mut it = Interp::new(&m2, 10_000);
        assert!(matches!(
            it.call_key(
                "main::div",
                vec![
                    V::Int(9, gt_types::IntTy::U32),
                    V::Int(0, gt_types::IntTy::U32)
                ]
            ),
            Outcome::Value(V::Int(0, _))
        ));
    }
}

fn fail_samples() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fail");
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("thư mục")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .filter(|p| {
            let n = p.file_name().expect("tên").to_string_lossy().into_owned();
            ["E0601_", "E0602_", "E0603_", "E0604_", "E0901_", "E0902_"]
                .iter()
                .any(|c| n.starts_with(c))
        })
        .collect();
    v.sort();
    v
}

#[test]
fn red_obligations_stay_red_with_identical_codes_in_every_mode() {
    let mut n = 0;
    for p in fail_samples() {
        let src = std::fs::read_to_string(&p)
            .expect("đọc")
            .replace("\r\n", "\n");
        let t = one(&src);
        if !t.report.diagnostics.is_empty() {
            continue;
        }
        let m = gt_mir::lower_program(&t);
        if !m.report.diagnostics.is_empty() {
            continue;
        }
        let codes = |mode| {
            let o = analyze_program_with(&t, &m, &mut ZoneProver::new(24), mode);
            let mut v: Vec<String> = o
                .report
                .diagnostics
                .iter()
                .filter(|d| d.code.as_str().starts_with('E'))
                .map(|d| d.code.as_str().to_string())
                .collect();
            v.sort();
            v
        };
        let (a, b) = (codes(Checks::All), codes(Checks::Proven));
        assert!(
            !a.is_empty(),
            "{}: mẫu ĐỎ phải vẫn ĐỎ ở chế độ all",
            p.display()
        );
        assert_eq!(a, b, "{}: mã ĐỎ phải như nhau ở mọi chế độ", p.display());
        n += 1;
    }
    assert!(n >= 20, "chỉ {n} mẫu ĐỎ M7");
}
