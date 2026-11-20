//! GÓI 13 B1 — chốt thử nghiệm `set_drop_trait_edges_for_test` (chỉ dùng ở oracle vi phân): bật ⇒ cạnh trait của đồ thị instance BỊ BỎ ⇒ đệ quy qua phương thức trait
//! không còn bị báo (chứng minh chốt có tác dụng; đột biến `if !DROP_TRAIT_EDGES` → luôn đúng làm chốt vô hiệu). Tắt ⇒ E0703. Tiến trình riêng (chốt toàn cục).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::{check_termination, set_drop_trait_edges_for_test};

fn codes(src: &str) -> Vec<String> {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}
{src}",
        common::mir_codes(&m)
    );
    let info = build_prog_info(&t, &m);
    let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
    let mut v: Vec<String> = o
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    v.sort();
    v
}

const VIA_TRAIT: &str = "⊏ T { ƒ m(&self, n: u32) → u32 ! ∅; }\n▢ S { v: i32 }\nƒ r<X: T>(x: &X, n: u32) → u32 ! ∅ ↓ n { x.m(n) }\n⊐ T for S { ƒ m(&self, n: u32) → u32 ! ∅ ↓ n { r(self, n) } }";

#[test]
fn the_drop_trait_edges_hook_changes_the_verdict_and_is_restored() {
    let src = format!(
        "{VIA_TRAIT}
ƒ main() → u32 ! ∅ {{ s ≔ S {{ v: 1 }}; r(&s, 3) }}
"
    );
    let normal = codes(&src);
    assert!(normal.iter().any(|x| x == "E0703"), "{normal:?}");
    set_drop_trait_edges_for_test(true);
    let hooked = codes(&src);
    set_drop_trait_edges_for_test(false);
    assert!(
        !hooked.iter().any(|x| x == "E0703"),
        "chốt không có tác dụng: {hooked:?}"
    );
    assert_eq!(codes(&src), normal);
}
