//! Plan M4: "thông điệp lỗi có gợi ý sửa dạng edit". Áp gợi ý vào nguồn rồi kiểm lại: chương trình đã sửa phải sạch.
mod common;
use common::*;

fn apply(src: &str, code: &str) -> String {
    let out = err(src, code, 1);
    let d = &out.report.diagnostics[0];
    assert_eq!(d.suggestions.len(), 1, "{:?}", show(&out));
    gt_diag::apply_edits(src, &d.suggestions[0].edits).expect("áp dụng được")
}

#[test]
fn e0308_assign_to_immutable_let_suggests_mu() {
    let src = "ƒ f() → i32 ! ∅ { x ≔ 1; x ← 2; x }";
    let fixed = apply(src, "E0308");
    assert!(fixed.contains("μx ≔ 1;"), "{fixed}");
    ok(&fixed);
}

#[test]
fn e0308_assign_to_immutable_param_suggests_mu() {
    let src = "ƒ f(x: i32) → i32 ! ∅ { x ← 2; x }";
    let fixed = apply(src, "E0308");
    assert!(fixed.contains("ƒ f(μx: i32)"), "{fixed}");
    ok(&fixed);
}

#[test]
fn e0308_field_and_index_roots_get_the_same_fix() {
    let fixed = apply(
        "▢ P { n: i32 } ƒ f() ! ∅ { p ≔ P { n: 1 }; p.n ← 2; }",
        "E0308",
    );
    assert!(fixed.contains("μp ≔"), "{fixed}");
    ok(&fixed);
    let fixed = apply("ƒ f() ! ∅ { a ≔ [1, 2]; a[0] ← 3; }", "E0308");
    assert!(fixed.contains("μa ≔"), "{fixed}");
    ok(&fixed);
}

#[test]
fn e0308_unique_borrow_of_immutable_suggests_mu() {
    let fixed = apply("ƒ f() ! ∅ { x ≔ 1; p ≔ &!x; }", "E0308");
    assert!(fixed.contains("μx ≔ 1;"), "{fixed}");
    ok(&fixed);
}

#[test]
fn e0308_through_a_shared_reference_gets_no_mu_suggestion() {
    // nguyên nhân là tham chiếu `&T`, không phải khai báo của biến: thêm `μ` vào `p` sẽ không sửa được
    let out = err("ƒ f(p: &i32) ! ∅ { *p ← 1; }", "E0308", 1);
    assert!(out.report.diagnostics[0].suggestions.is_empty());
    let out = err("▢ P { n: i32 } ƒ f(p: &P) ! ∅ { p.n ← 1; }", "E0308", 1);
    assert!(out.report.diagnostics[0].suggestions.is_empty());
}

#[test]
fn e0308_pattern_bindings_get_no_mu_suggestion() {
    let out = err(
        "ƒ f(o: Option<i32>) ! ∅ { ⌥ o { Some(v) ↦ { v ← 1; }, None ↦ { } } }",
        "E0308",
        1,
    );
    assert!(out.report.diagnostics[0].suggestions.is_empty());
}

#[test]
fn e0305_suggests_a_catch_all_arm_after_the_last_arm() {
    let fixed = apply("ƒ f(b: bool) → i32 ! ∅ { ⌥ b { true ↦ 1 } }", "E0305");
    assert!(fixed.contains("true ↦ 1, _ ↦ …"), "{fixed}");
    ok(&fixed.replace('…', "0"));
}

#[test]
fn e0305_suggestion_works_with_a_trailing_comma_and_many_arms() {
    let fixed = apply("ƒ f(n: i32) → i32 ! ∅ { ⌥ n { 0 ↦ 1, 1 ↦ 2, } }", "E0305");
    ok(&fixed.replace('…', "0"));
    let fixed = apply(
        "⊔ E { A, B, C } ƒ f(e: E) → i32 ! ∅ { ⌥ e { A ↦ 1, B ↦ 2 } }",
        "E0305",
    );
    ok(&fixed.replace('…', "0"));
}
