//! M4 bước (c): kiểu hàm, lời gọi, suy luận cục bộ bidirectional, toán tử, luồng điều khiển.
mod common;
use common::*;

#[test]
fn calls_check_arity_and_argument_types() {
    let defs = "ƒ add(a: i32, b: i32) → i32 ! ∅ { a + b } ";
    ok(&format!("{defs}ƒ f() → i32 ! ∅ {{ add(1, 2) }}"));
    err(&format!("{defs}ƒ f() → i32 ! ∅ {{ add(1) }}"), "E0301", 1);
    err(
        &format!("{defs}ƒ f() → i32 ! ∅ {{ add(1, 2, 3) }}"),
        "E0301",
        1,
    );
    err(
        &format!("{defs}ƒ f() → i32 ! ∅ {{ add(1, true) }}"),
        "E0301",
        1,
    );
    err(
        &format!("{defs}ƒ f() → i32 ! ∅ {{ add(1, 2i64) }}"),
        "E0302",
        1,
    );
    // gọi thứ không phải hàm
    err("ƒ f(x: i32) ! ∅ { x(1); }", "E0301", 1);
}

#[test]
fn return_types_are_checked_at_every_return_point() {
    ok("ƒ f() → i32 ! ∅ { 1 }");
    err("ƒ f() → i32 ! ∅ { true }", "E0301", 1);
    err("ƒ f() → i32 ! ∅ { 1i64 }", "E0302", 1);
    ok("ƒ f(c: bool) → i32 ! ∅ { ⎇ c { ⏎ 1; } 2 }");
    err("ƒ f(c: bool) → i32 ! ∅ { ⎇ c { ⏎ true; } 2 }", "E0301", 1);
    err("ƒ f() → i32 ! ∅ { ⏎; }", "E0301", 1);
    ok("ƒ f() ! ∅ { ⏎; }");
    err("ƒ f() ! ∅ { ⏎ 1; }", "E0301", 1);
    // hàm trả `()`: tail `()` hoặc không có tail
    ok("ƒ f() ! ∅ { }");
    ok("ƒ f() → () ! ∅ { () }");
    err("ƒ f() → () ! ∅ { 1 }", "E0301", 1);
    // khối kết thúc bằng `;` mà hàm trả kiểu khác (): E0307 của M5, M4 không báo
    ok("ƒ f() → i32 ! ∅ { 1; }");
}

#[test]
fn function_values_and_fn_types() {
    ok(
        "ƒ inc(x: i32) → i32 ! ∅ { x + 1 } ƒ ap(f: ƒ(i32) → i32 ! ∅, v: i32) → i32 ! ∅ { f(v) } ƒ g() → i32 ! ∅ { ap(inc, 1) }",
    );
    // hiệu ứng là một phần của kiểu hàm: hàm có {io} không gán được vào ƒ(…) ! ∅
    err(
        "ƒ io_fn(x: i32) → i32 ! {io} { x } ƒ ap(f: ƒ(i32) → i32 ! ∅) ! ∅ { } ƒ g() ! ∅ { ap(io_fn); }",
        "E0301",
        1,
    );
    err(
        "ƒ inc(x: i32) → i32 ! ∅ { x } ƒ ap(f: ƒ(i32, i32) → i32 ! ∅) ! ∅ { } ƒ g() ! ∅ { ap(inc); }",
        "E0301",
        1,
    );
}

#[test]
fn recursion_and_mutual_calls_use_declared_signatures() {
    ok("ƒ fact(n: u32) → u64 ! ∅ ↓ n { ⎇ n == 0 { 1 } ⇝ { n as u64 *% fact(n - 1) } }");
    ok(
        "ƒ even(n: u32) → bool ! ∅ { ⎇ n == 0 { true } ⇝ { odd(n - 1) } } ƒ odd(n: u32) → bool ! ∅ { ⎇ n == 0 { false } ⇝ { even(n - 1) } }",
    );
}

#[test]
fn if_conditions_and_branch_types() {
    ok("ƒ f(c: bool) → i32 ! ∅ { ⎇ c { 1 } ⇝ { 2 } }");
    err(
        "ƒ f(c: bool) → i32 ! ∅ { ⎇ c { 1 } ⇝ { true } }",
        "E0301",
        1,
    );
    err("ƒ f(x: i32) → i32 ! ∅ { ⎇ x { 1 } ⇝ { 2 } }", "E0301", 1);
    // không else: nhánh then phải là ()
    ok("ƒ f(c: bool) ! ∅ { ⎇ c { } }");
    err("ƒ f(c: bool) ! ∅ { ⎇ c { 1 } }", "E0301", 1);
    // else-if
    ok("ƒ f(a: bool, b: bool) → i32 ! ∅ { ⎇ a { 1 } ⇝ ⎇ b { 2 } ⇝ { 3 } }");
    err(
        "ƒ f(a: bool, b: bool) → i32 ! ∅ { ⎇ a { 1 } ⇝ ⎇ b { 2 } }",
        "E0301",
        2, // else-if thiếu else cuối: then-block lệch `()` và nhánh ngoài lệch
    );
}

#[test]
fn match_arms_join_and_scrutinee_types() {
    ok("ƒ f(o: Option<i32>) → i32 ! ∅ { ⌥ o { Some(v) ↦ v, None ↦ 0 } }");
    err(
        "ƒ f(o: Option<i32>) → i32 ! ∅ { ⌥ o { Some(v) ↦ v, None ↦ true } }",
        "E0301",
        1,
    );
    ok("ƒ f(x: i32) → bool ! ∅ { ⌥ x { 0 ↦ true, _ ↦ false } }");
    // binding của nhánh có kiểu của scrutinee
    let out = ok("ƒ f(o: Option<u8>) ! ∅ { ⌥ o { Some(v) ↦ { w ≔ v; }, None ↦ { } } }");
    assert_eq!(out.local_type("main::f", "v").as_deref(), Some("u8"));
}

#[test]
fn loops_and_loop_variables() {
    ok("ƒ f(n: u32) ! ∅ { μi ≔ 0u32; ⟳ i < n ↓ n - i { i ← i + 1; } }");
    err("ƒ f(n: u32) ! ∅ { ⟳ n { } }", "E0301", 1);
    err(
        "ƒ f(n: u32) ! ∅ { μi ≔ 0u32; ⟳ i < n ↓ true { i ← i + 1; } }",
        "E0301",
        1,
    );
    // for-in range: biến có kiểu của cận
    let out = ok("ƒ f() ! ∅ { ⟳ i in 0..3u8 { x ≔ i; } }");
    assert_eq!(out.local_type("main::f", "i").as_deref(), Some("u8"));
    // for-in slice cho &T
    let out = ok("ƒ f(xs: &[i64]) → i64 ! ∅ { μs ≔ 0; ⟳ x in xs { s ← s +% *x; } s }");
    assert_eq!(out.local_type("main::f", "x").as_deref(), Some("&i64"));
    assert_eq!(out.local_type("main::f", "s").as_deref(), Some("i64"));
    err(
        "ƒ f(xs: &[i64]) → i64 ! ∅ { μs ≔ 0; ⟳ x in xs { s ← s +% x; } s }",
        "E0301",
        1,
    );
    err("ƒ f(x: i32) ! ∅ { ⟳ i in x { } }", "E0301", 1);
    // invariant phải là bool
    err(
        "ƒ f(n: u32) ! ∅ { μi ≔ 0u32; ⟳ i < n ⋈ 1 { i ← i + 1; } }",
        "E0301",
        1,
    );
}

#[test]
fn break_and_continue_only_inside_loops() {
    ok("ƒ f() ! ∅ { ⟳ { ⊠; } }");
    ok("ƒ f(n: u32) ! ∅ { μi ≔ 0u32; ⟳ i < n ↓ n - i { ⎇ i == 3 { ⊠; } i ← i + 1; ⊡; } }");
    err("ƒ f() ! ∅ { ⊠; }", "E0301", 1);
    err("ƒ f() ! ∅ { ⊡; }", "E0301", 1);
}

#[test]
fn infinite_loop_without_break_has_type_never_otherwise_unit() {
    // ⟳ { } không thoát: kiểu `!`, nên là thân hợp lệ của hàm `→ i32` và `→ !`
    ok("ƒ f() → i32 ! {div} { ⟳ { } }");
    ok("ƒ f() → ! ! {div} { ⟳ { } }");
    // có ⊠: kiểu `()`
    err("ƒ f() → i32 ! {div} { ⟳ { ⊠; } }", "E0301", 1);
    ok("ƒ f() ! {div} { ⟳ { ⊠; } }");
}

#[test]
fn operators_follow_d1_d4_rules() {
    ok("ƒ f(a: i32, b: i32) → i32 ! ∅ { a + b - a * b / a % b }");
    ok("ƒ f(a: u8, b: u8) → u8 ! ∅ { a +% b -% a *% b }");
    ok("ƒ f(a: u8, b: u8) → u8 ! ∅ { a +^ b -^ a *^ b }");
    ok("ƒ f(a: f64, b: f64) → f64 ! ∅ { a + b - a * b / a }");
    ok("ƒ f(a: bool, b: bool) → bool ! ∅ { (a ∧ b) ∨ ¬a ⇒ b }");
    ok("ƒ f(a: i32, b: i32) → bool ! ∅ { a < b ∧ a <= b ∧ a > b ∧ a >= b ∧ a == b ∧ a ≠ b }");
    ok("ƒ f(a: char, b: char) → bool ! ∅ { a < b ∧ a == b }");
    ok("ƒ f(a: f64, b: f64) → bool ! ∅ { a < b ∨ a >= b }");
    ok("ƒ f(a: i32) → i32 ! ∅ { -a }");
    ok("ƒ f(a: f64) → f64 ! ∅ { -a }");
    for bad in [
        "ƒ f(a: bool, b: bool) → bool ! ∅ { a + b }",
        "ƒ f(a: f64, b: f64) → f64 ! ∅ { a +% b }",
        "ƒ f(a: f64, b: f64) → f64 ! ∅ { a *^ b }",
        "ƒ f(a: i32, b: i32) → bool ! ∅ { a ∧ b }",
        "ƒ f(a: i32) → bool ! ∅ { ¬a }",
        "ƒ f(a: bool, b: bool) → bool ! ∅ { a < b }",
        "ƒ f(a: u8) → u8 ! ∅ { -a }",
        "ƒ f(a: bool) → bool ! ∅ { -a }",
        "ƒ f(a: &str, b: &str) → bool ! ∅ { a < b }",
    ] {
        let out = one(bad);
        assert!(
            !codes(&out).is_empty() && codes(&out).iter().all(|c| c == "E0301"),
            "{bad}: {:?}",
            show(&out)
        );
    }
    err("ƒ f(a: i32, b: i64) → i32 ! ∅ { a + b }", "E0302", 1);
    err("ƒ f(a: i32, b: i64) → bool ! ∅ { a < b }", "E0302", 1);
}

#[test]
fn checked_arithmetic_returns_the_lang_option() {
    let out = ok("ƒ f(a: u8, b: u8) ! ∅ { x ≔ a +? b; y ≔ a -? b; z ≔ a *? b; }");
    for n in ["x", "y", "z"] {
        assert_eq!(
            out.local_type("main::f", n).as_deref(),
            Some("Option<u8>"),
            "{n}"
        );
    }
    err("ƒ f(a: f64, b: f64) ! ∅ { a +? b; }", "E0301", 1);
}

#[test]
fn equality_is_limited_to_comparable_types() {
    ok("ƒ f(a: &str, b: &str) → bool ! ∅ { a == b }");
    // tuple: CHƯA (FB-C13: backend LLVM không cài, thông dịch từng khác ngữ nghĩa) ⇒ E0301 ngay ở cổng
    err("ƒ f(a: (i32, bool), b: (i32, bool)) → bool ! ∅ { a == b }", "E0301", 1);
    err(
        "▢ P { x: i32 } ƒ f(a: P, b: P) → bool ! ∅ { a == b }",
        "E0301",
        1,
    );
    err("ƒ f(a: i32, b: bool) → bool ! ∅ { a == b }", "E0301", 1);
}

#[test]
fn references_deref_and_borrow() {
    let out = ok("ƒ f(p: &i32, q: &!i32) ! ∅ { a ≔ *p; b ≔ *q; μx ≔ 1; c ≔ &x; d ≔ &!x; }");
    assert_eq!(out.local_type("main::f", "a").as_deref(), Some("i32"));
    assert_eq!(out.local_type("main::f", "c").as_deref(), Some("&i32"));
    assert_eq!(out.local_type("main::f", "d").as_deref(), Some("&!i32"));
    err("ƒ f(x: i32) → i32 ! ∅ { *x }", "E0301", 1);
    ok("ƒ f(pp: &&i32) → i32 ! ∅ { **pp }");
    // &T và &!T là kiểu khác nhau
    err(
        "ƒ g(p: &!i32) ! ∅ { } ƒ f(p: &i32) ! ∅ { g(p); }",
        "E0301",
        1,
    );
    err(
        "ƒ g(p: &i32) ! ∅ { } ƒ f(p: &!i32) ! ∅ { g(p); }",
        "E0301",
        1,
    );
}

#[test]
fn quantifiers_and_set_membership_in_contracts() {
    ok("ƒ f(a: &[i32], n: usize) ! ∅ ⊳ ∀ i ∈ 0..n : i < n { }");
    // `∈` có độ ưu tiên so sánh, range thấp nhất: miền phải đặt trong ngoặc (`x ∈ (0..10)`)
    ok("ƒ f(x: i32) ! ∅ ⊳ x ∈ (0..10) { }");
    err("ƒ f(x: i32) ! ∅ ⊳ x ∈ 5 { }", "E0301", 1);
    err("ƒ f(x: i32) ! ∅ ⊳ x ∈ (0..10u8) { }", "E0302", 1);
    err("ƒ f(x: i32) ! ∅ ⊳ x + 1 { }", "E0301", 1);
    err("ƒ f(x: i32) → i32 ! ∅ ⊲ result { x }", "E0301", 1);
    ok("ƒ f(x: i32) → i32 ! ∅ ⊲ result >= x { x + 1 }");
}

#[test]
fn open_ranges_are_only_slice_indices() {
    ok("ƒ f(xs: &[i32]) ! ∅ { a ≔ &xs[1..]; b ≔ &xs[..2]; c ≔ &xs[..]; d ≔ &xs[0..=1]; }");
    err("ƒ f() ! ∅ { r ≔ 1..; }", "E0301", 1);
    err("ƒ f() ! ∅ { r ≔ ..2; }", "E0301", 1);
    err("ƒ f() ! ∅ { r ≔ ..; }", "E0301", 1);
    err("ƒ f() ! ∅ { r ≔ 1..true; }", "E0301", 1);
    err("ƒ f() ! ∅ { r ≔ 1.0..2.0; }", "E0301", 1);
}

#[test]
fn unresolved_types_are_reported_not_guessed() {
    // `None` không có ngữ cảnh: không đủ thông tin
    err("ƒ f() ! ∅ { x ≔ None; }", "E0301", 1);
    // có chú thích thì được
    ok("ƒ f() ! ∅ { x: Option<u8> ≔ None; }");
}

#[test]
fn deeply_nested_expressions_do_not_overflow() {
    let mut e = String::from("1");
    for _ in 0..120 {
        e = format!("({e} + 1)");
    }
    ok(&format!("ƒ f() → i32 ! ∅ {{ {e} }}"));
}
