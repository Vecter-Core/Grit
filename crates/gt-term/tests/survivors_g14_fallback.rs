//! GÓI 14 D1 — `dyn_fn_arg`: một đối số HẰNG (`Operand::Const`) không bao giờ là "giá trị hàm chưa biết". `spawn(w, 5u64)` (cả hai đối số là hằng) PHẢI dựng được đồ thị
//! instance, KHÔNG lùi về đồ thị bảo thủ. Đột biến `Operand::Const(_) => false` → `true` làm mọi lời gọi `spawn(hằng…)` lùi về bảo thủ (đếm `conservative_fallbacks`).
//! MỘT test duy nhất trong tệp: bộ đếm là toàn cục theo tiến trình.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::{check_termination, conservative_fallbacks};

#[test]
fn spawn_with_constant_arguments_keeps_the_instance_graph() {
    let src = "ƒ w(n: u64) → () ! ∅ { }\nƒ main() → i32 ! ∅ { spawn(w, 5u64); 0 }\n";
    let (t, m) = common::mir(src);
    assert!(m.report.diagnostics.is_empty());
    let info = build_prog_info(&t, &m);
    let before = conservative_fallbacks();
    let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
    assert!(
        o.report.diagnostics.is_empty(),
        "{:?}",
        o.report.diagnostics
    );
    assert_eq!(
        conservative_fallbacks(),
        before,
        "lùi về đồ thị bảo thủ oan"
    );
}
