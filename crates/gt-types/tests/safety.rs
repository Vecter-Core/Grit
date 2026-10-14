//! Các kiểm tra an toàn trọng yếu của M4 (đối tượng của đột biến có mục tiêu ở cuối M4):
//! D1 không ép kiểu số ngầm (E0302), D4 cấm `==` trên số thực (E0303), D5 bất biến mặc định (E0308), ép kiểu `!` (ADR-018 §3).
mod common;
use common::*;

// ======================= D4: == và ≠ trên số thực =======================

#[test]
fn d4_float_equality_is_rejected_in_every_form() {
    for bad in [
        "ƒ f(a: f64, b: f64) → bool ! ∅ { a == b }",
        "ƒ f(a: f64, b: f64) → bool ! ∅ { a ≠ b }",
        "ƒ f(a: f32, b: f32) → bool ! ∅ { a == b }",
        "ƒ f(a: f64) → bool ! ∅ { a == 1.0 }",
        "ƒ f(a: f64) → bool ! ∅ { 1.0 == a }",
        "ƒ f() → bool ! ∅ { 1.0 == 2.0 }",
        "ƒ f(a: f64) → bool ! ∅ { a == 1.5f64 }",
        "ƒ f(a: f32) → bool ! ∅ { a ≠ a }",
    ] {
        err(bad, "E0303", 1);
    }
}

#[test]
fn d4_orderings_and_integer_equality_are_still_fine() {
    ok("ƒ f(a: f64, b: f64) → bool ! ∅ { a < b ∨ a <= b ∨ a > b ∨ a >= b }");
    ok("ƒ f(a: i32, b: i32) → bool ! ∅ { a == b ∧ a ≠ b }");
    ok("ƒ f(a: u8) → bool ! ∅ { a == 0 }");
    ok("ƒ f(a: bool, b: bool) → bool ! ∅ { a == b }");
    // so sánh số thực qua ngưỡng
    ok("ƒ f(a: f64, b: f64) → bool ! ∅ { a - b < 0.001 ∧ b - a < 0.001 }");
}

// ======================= D5: bất biến mặc định =======================

#[test]
fn d5_assignment_to_immutable_bindings_is_e0308() {
    for bad in [
        "ƒ f() ! ∅ { x ≔ 1; x ← 2; }",
        "ƒ f(x: i32) ! ∅ { x ← 2; }",
        "ƒ f(x: bool) ! ∅ { x ← true; }",
        "▢ P { a: i32 } ƒ f(p: P) ! ∅ { p.a ← 1; }",
        "▢ P { a: i32 } ƒ f() ! ∅ { p ≔ P { a: 0 }; p.a ← 1; }",
        "ƒ f(t: (i32, i32)) ! ∅ { t.0 ← 1; }",
        "ƒ f(a: [i32; 2]) ! ∅ { a[0] ← 1; }",
        "ƒ f(p: &i32) ! ∅ { *p ← 1; }",
        "▢ P { a: i32 } ƒ f(p: &P) ! ∅ { p.a ← 1; }",
        "ƒ f(xs: &[i32]) ! ∅ { xs[0] ← 1; }",
        "ƒ f() ! ∅ { ⟳ i in 0..3 { i ← 1; } }",
        "ƒ f(o: Option<i32>) ! ∅ { ⌥ o { Some(v) ↦ { v ← 1; }, None ↦ { } } }",
    ] {
        err(bad, "E0308", 1);
    }
}

#[test]
fn d5_mutable_places_accept_assignment() {
    for good in [
        "ƒ f() ! ∅ { μx ≔ 1; x ← 2; }",
        "ƒ f(μx: i32) ! ∅ { x ← 2; }",
        "▢ P { a: i32 } ƒ f(μp: P) ! ∅ { p.a ← 1; }",
        "▢ P { a: i32 } ƒ f() ! ∅ { μp ≔ P { a: 0 }; p.a ← 1; }",
        "ƒ f(μt: (i32, i32)) ! ∅ { t.0 ← 1; }",
        "ƒ f(μa: [i32; 2]) ! ∅ { a[0] ← 1; }",
        "ƒ f(p: &!i32) ! ∅ { *p ← 1; }",
        "▢ P { a: i32 } ƒ f(p: &!P) ! ∅ { p.a ← 1; }",
        "▢ P { a: i32 } ƒ f(p: &!P) ! ∅ { (*p).a ← 1; }",
        "ƒ f(xs: &![i32]) ! ∅ { xs[0] ← 1; }",
        "ƒ f(pp: &!&!i32) ! ∅ { **pp ← 2; }",
        "▢ P { x: i32 } ⊐ P { ƒ bump(&!self) ! ∅ { self.x ← self.x +% 1; } }",
    ] {
        ok(good);
    }
}

#[test]
fn d5_assignment_through_a_shared_reference_inside_a_mutable_binding_is_still_rejected() {
    // μ trên biến chứa &T không làm *p thay đổi được
    err("ƒ f(μp: &i32) ! ∅ { *p ← 1; }", "E0308", 1);
    // nhưng được gán lại chính biến
    ok("ƒ f(a: &i32, b: &i32) ! ∅ { μp ≔ a; p ← b; }");
}

#[test]
fn d5_assignment_through_self_needs_a_unique_self() {
    err(
        "▢ P { x: i32 } ⊐ P { ƒ bump(&self) ! ∅ { self.x ← 1; } }",
        "E0308",
        1,
    );
    err(
        "▢ P { x: i32 } ⊐ P { ƒ bump(self) ! ∅ { self.x ← 1; } }",
        "E0308",
        1,
    );
}

#[test]
fn d5_unique_borrow_of_an_immutable_place_is_e0308() {
    err("ƒ f() ! ∅ { x ≔ 1; p ≔ &!x; }", "E0308", 1);
    err("ƒ f(x: i32) ! ∅ { p ≔ &!x; }", "E0308", 1);
    err("▢ P { a: i32 } ƒ f(p: P) ! ∅ { q ≔ &!p.a; }", "E0308", 1);
    ok("ƒ f() ! ∅ { μx ≔ 1; p ≔ &!x; }");
    ok("ƒ f(p: &!i32) ! ∅ { q ≔ &!*p; }");
    err("ƒ f(p: &i32) ! ∅ { q ≔ &!*p; }", "E0308", 1);
    // mượn chung thì luôn được
    ok("ƒ f(x: i32) ! ∅ { p ≔ &x; }");
}

#[test]
fn d5_method_needing_unique_self_requires_a_mutable_receiver() {
    let defs = "▢ C { n: i32 } ⊐ C { ƒ inc(&!self) ! ∅ { self.n ← self.n + 1; } ƒ get(&self) → i32 ! ∅ { self.n } } ";
    ok(&format!(
        "{defs}ƒ f() ! ∅ {{ μc ≔ C {{ n: 0 }}; c.inc(); }}"
    ));
    err(
        &format!("{defs}ƒ f() ! ∅ {{ c ≔ C {{ n: 0 }}; c.inc(); }}"),
        "E0308",
        1,
    );
    err(&format!("{defs}ƒ f(c: C) ! ∅ {{ c.inc(); }}"), "E0308", 1);
    err(&format!("{defs}ƒ f(c: &C) ! ∅ {{ c.inc(); }}"), "E0308", 1);
    ok(&format!("{defs}ƒ f(c: &!C) ! ∅ {{ c.inc(); }}"));
    ok(&format!("{defs}ƒ f(c: C) → i32 ! ∅ {{ c.get() }}"));
    ok(&format!("{defs}ƒ f(c: &C) → i32 ! ∅ {{ c.get() }}"));
}

#[test]
fn d5_message_names_the_variable_and_suggests_mu() {
    let out = err("ƒ f() ! ∅ { total ≔ 1; total ← 2; }", "E0308", 1);
    let m = &out.report.diagnostics[0].message;
    assert!(m.contains("total") && m.contains('μ'), "{m}");
}

// ======================= kiểu `!` =======================

#[test]
fn never_coerces_at_coercion_points() {
    for good in [
        "ƒ f(c: bool) → i32 ! {div} { ⎇ c { 1 } ⇝ { core::abort(\"x\") } }",
        "ƒ f(c: bool) → i32 ! {div} { ⎇ c { core::abort(\"x\") } ⇝ { 1 } }",
        "ƒ f(o: Option<i32>) → i32 ! {div} { ⌥ o { Some(v) ↦ v, None ↦ core::abort(\"none\") } }",
        "ƒ f() → i32 ! {div} { x: i32 ≔ core::abort(\"x\"); x }",
        "ƒ f() ! {div} { g(core::abort(\"x\")); } ƒ g(x: i32) ! ∅ { }",
        "ƒ f() → i32 ! {div} { core::abort(\"x\") }",
        "ƒ f(c: bool) → i32 ! ∅ { ⎇ c { ⏎ 1; } 2 }",
        "ƒ f() → i32 ! ∅ { ⏎ 1; }",
        "ƒ f() → i32 ! ∅ { ⏎ 1; 2 }",
    ] {
        ok(good);
    }
}

#[test]
fn never_is_not_a_general_subtype() {
    // toán hạng của toán tử không phải điểm ép
    err("ƒ f() → i32 ! {div} { 1 + core::abort(\"x\") }", "E0301", 1);
    err(
        "ƒ f() → bool ! {div} { 1 == core::abort(\"x\") }",
        "E0301",
        1,
    );
}

#[test]
fn never_branches_merge_with_the_other_branches_type() {
    let out =
        ok("ƒ f(o: Option<u8>) ! {div} { x ≔ ⌥ o { Some(v) ↦ v, None ↦ core::abort(\"none\") }; }");
    assert_eq!(out.local_type("main::f", "x").as_deref(), Some("u8"));
    // mọi nhánh là `!` thì cả biểu thức là `!`
    let out =
        ok("ƒ f(c: bool) ! {div} { x ≔ ⎇ c { core::abort(\"a\") } ⇝ { core::abort(\"b\") }; }");
    assert_eq!(out.local_type("main::f", "x").as_deref(), Some("!"));
}

#[test]
fn a_block_with_a_diverging_statement_has_type_never() {
    ok("ƒ f() → i32 ! ∅ { ⏎ 1; }");
    ok("ƒ f(c: bool) → i32 ! {div} { ⎇ c { 1 } ⇝ { ⏎ 2; } }");
    ok("ƒ f() → i32 ! {div} { core::abort(\"x\"); }");
    // `() ` mà cần i32 khi KHÔNG phân kỳ: M5 (E0307); M4 không báo — nhưng có tail sai kiểu thì báo
    err("ƒ f() → i32 ! ∅ { true }", "E0301", 1);
}

#[test]
fn a_function_declared_to_return_never_must_have_a_never_body() {
    ok("ƒ die() → ! ! {div} { core::abort(\"x\") }");
    ok("ƒ die() → ! ! {div} { ⟳ { } }");
    err("ƒ die() → ! ! ∅ { 1 }", "E0301", 1);
    err("ƒ die() → ! ! ∅ { () }", "E0301", 1);
    // và gọi nó có kiểu `!`
    ok("ƒ die() → ! ! {div} { core::abort(\"x\") } ƒ f() → i32 ! {div} { die() }");
}

#[test]
fn never_is_rejected_as_a_type_argument_field_or_element_e0314() {
    err("ƒ f(x: Option<!>) ! ∅ { }", "E0314", 1);
    err("▢ S { x: ! }", "E0314", 1);
    err("⊔ E { A(!) }", "E0314", 1);
    err("ƒ f(a: [!; 2]) ! ∅ { }", "E0314", 1);
    err("ƒ f(a: (i32, !)) ! ∅ { }", "E0314", 1);
    // làm kiểu trả về thì được
    ok("ƒ f() → ! ! {div} { ⟳ { } }");
}
