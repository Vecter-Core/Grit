//! GÓI 7 FB-C33 (nghiêm trọng, D12): đệ quy QUA `spawn` (hàm worker được truyền làm đối số của intrinsic không có thân) không được xét như cạnh gọi, nên
//! `ƒ f(n) ↓ n { spawn(f, n); }` (biến thể không giảm) và vòng tương hỗ qua `spawn` không `↓`/`div` từng được chấp nhận (mỗi lượt để lại một luồng: không dừng).
//! Phát hiện bởi bộ tấn công `attack/` (phiên kiểm toán); tái hiện độc lập ở GÓI 7. Test này ĐỎ trước bản sửa: các chương trình phải bị từ chối (E0703/E0702).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::check_termination;

fn codes(src: &str) -> Vec<String> {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        common::mir_codes(&m)
    );
    let info = build_prog_info(&t, &m);
    let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
    o.report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect()
}

fn rejected(src: &str) {
    let c = codes(src);
    assert!(
        c.iter().any(|x| x == "E0703" || x == "E0702"),
        "đệ quy qua spawn được chấp nhận (FB-C33): {c:?}\n{src}"
    );
}

#[test]
fn self_recursion_through_spawn_without_variant_is_rejected() {
    rejected("ƒ f(n: u64) → () ! ∅ { spawn(f, n); }\nƒ main() → i32 ! ∅ { f(1u64); 0 }\n");
}

#[test]
fn self_recursion_through_spawn_with_non_decreasing_variant_is_rejected() {
    rejected("ƒ f(n: u64) → () ! ∅ ↓ n { spawn(f, n); }\nƒ main() → i32 ! ∅ { f(1u64); 0 }\n");
}

#[test]
fn mutual_recursion_through_spawn_is_rejected() {
    rejected(
        "ƒ a(n: u64) → () ! ∅ { spawn(b, n); }\nƒ b(n: u64) → () ! ∅ { a(n); }\nƒ main() → i32 ! ∅ { a(1u64); 0 }\n",
    );
}

#[test]
fn spawning_a_non_recursive_worker_is_still_accepted() {
    // đối chứng: spawn hàm lá không đệ quy phải còn được chấp nhận (không từ chối quá tay)
    let c = codes(
        "ƒ w(n: u64) → () ! ∅ { }\nƒ run() → () ! ∅ { spawn(w, 1u64); }\nƒ main() → i32 ! ∅ { run(); 0 }\n",
    );
    assert!(c.is_empty(), "{c:?}");
}
