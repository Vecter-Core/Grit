//! Mỗi test ở đây giết một đột biến SỐNG SÓT của đợt đột biến có mục tiêu cuối M4 (D1, D4, D5, `!`) — xem PROGRESS.md.
mod common;
use common::*;

const DIE: &str = "ƒ die() → ! ! {div} { core::abort(\"x\") } ";

#[test]
fn unique_self_method_on_a_temporary_receiver_is_allowed_d5() {
    // người nhận không phải chỗ (giá trị tạm) thì không có "biến bất biến" để báo
    ok(
        "▢ C { n: i32 } ⊐ C { ƒ inc(&!self) ! ∅ { self.n ← self.n +% 1; } } ƒ mk() → C ! ∅ { C { n: 0 } } ƒ f() ! ∅ { mk().inc(); }",
    );
    ok(
        "▢ C { n: i32 } ⊐ C { ƒ inc(&!self) ! ∅ { self.n ← self.n +% 1; } } ƒ f() ! ∅ { C { n: 0 }.inc(); }",
    );
}

#[test]
fn e0314_does_not_cascade_into_further_errors() {
    // sau E0314 kiểu trường là `Err`, không phải `!`: dùng trường không sinh thêm lỗi toán hạng `!`
    err("▢ S { x: ! } ƒ f(s: S) → i32 ! ∅ { s.x + 1 }", "E0314", 1);
    err("ƒ f(o: Option<!>) → i32 ! ∅ { 1 }", "E0314", 1);
}

#[test]
fn int_and_float_literals_never_unify_d1() {
    err("ƒ f() ! ∅ { a ≔ 1 + 1.5; }", "E0302", 1);
    err("ƒ f() ! ∅ { a ≔ 1.5 + 1; }", "E0302", 1);
    err("ƒ f(c: bool) ! ∅ { a ≔ ⎇ c { 1 } ⇝ { 2.5 }; }", "E0302", 1);
    err("ƒ f() ! ∅ { a ≔ [1, 2.5]; }", "E0302", 1);
}

#[test]
fn never_as_an_operand_names_the_reason() {
    // không chỉ đúng mã: một chẩn đoán phải nói rõ `!` không phải điểm ép kiểu (không bị nhầm với "sai kiểu" thường)
    for body in ["die() + 1", "1 + die()"] {
        let out = one(&format!("{DIE}ƒ f() → i32 ! {{div}} {{ {body} }}"));
        assert!(
            !codes(&out).is_empty() && codes(&out).iter().all(|c| c == "E0301"),
            "{:?}",
            show(&out)
        );
        assert!(
            show(&out)
                .iter()
                .any(|m| m.contains("toán hạng của toán tử")),
            "{:?}",
            show(&out)
        );
    }
}

#[test]
fn a_leading_never_branch_does_not_decide_the_join_type() {
    // không có kiểu mong đợi: nhánh đầu là `!` không được biến cả biểu thức thành `!`
    let out = ok(&format!(
        "{DIE}ƒ f(c: bool) ! {{div}} {{ x ≔ ⎇ c {{ die() }} ⇝ {{ 1u8 }}; }}"
    ));
    assert_eq!(out.local_type("main::f", "x").as_deref(), Some("u8"));
    let out = ok(&format!(
        "{DIE}ƒ f(o: Option<u8>) ! {{div}} {{ x ≔ ⌥ o {{ None ↦ die(), Some(v) ↦ v }}; }}"
    ));
    assert_eq!(out.local_type("main::f", "x").as_deref(), Some("u8"));
}

#[test]
fn equality_on_aggregates_containing_floats_is_rejected_d4() {
    err(
        "ƒ f(a: (f64, i32), b: (f64, i32)) → bool ! ∅ { a == b }",
        "E0301",
        1,
    );
    err(
        "ƒ f(a: (i32, f32), b: (i32, f32)) → bool ! ∅ { a ≠ b }",
        "E0301",
        1,
    );
    err(
        "ƒ f(a: [f64; 2], b: [f64; 2]) → bool ! ∅ { a == b }",
        "E0301",
        1,
    );
    // tuple không float: cũng E0301 từ FB-C13 (tuple chưa hỗ trợ `==` ở mọi backend); số nguyên thì được
    err(
        "ƒ f(a: (i32, bool), b: (i32, bool)) → bool ! ∅ { a == b }",
        "E0301",
        1,
    );
    ok("ƒ f(a: i32, b: i32) → bool ! ∅ { a == b }");
}

#[test]
fn deref_of_a_non_reference_is_one_type_error_not_a_mutability_error() {
    // `*x` với `x: i32` là lỗi kiểu duy nhất; nó không được kéo thêm lỗi bất biến (D5) vì chỗ gán không xác định được
    let out = one("ƒ f(x: i32) ! ∅ { *x ← 1; }");
    assert_eq!(codes(&out), ["E0301"], "{:?}", show(&out));
}

#[test]
fn a_diverging_let_initializer_makes_the_block_diverge() {
    // `x ≔ die();` làm khối có kiểu `!`, nên khối gán được cho `i32`
    ok(&format!(
        "{DIE}ƒ f() → i32 ! {{div}} {{ y: i32 ≔ {{ x ≔ die(); }}; y }}"
    ));
    // đối chứng: khối không phân kỳ có kiểu `()` thì không gán được cho `i32`
    err("ƒ f() → i32 ! ∅ { y: i32 ≔ { x ≔ 1; }; y }", "E0301", 1);
}

#[test]
fn a_diverging_expression_statement_makes_the_block_diverge() {
    ok(&format!(
        "{DIE}ƒ f() → i32 ! {{div}} {{ y: i32 ≔ {{ die(); }}; y }}"
    ));
    ok("ƒ f() → i32 ! {div} { y: i32 ≔ { core::abort(\"a\"); }; y }");
    err("ƒ f() → i32 ! ∅ { y: i32 ≔ { 1; }; y }", "E0301", 1);
}
