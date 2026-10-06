//! M4 bước (a): kiểu nguyên thuỷ + kiểu của literal; phạm vi literal; không ép kiểu số ngầm (D1).
mod common;
use common::*;

fn local_ty(body: &str, name: &str) -> String {
    let out = ok(&format!("ƒ f() ! ∅ {{ {body} }}"));
    out.local_type("main::f", name)
        .unwrap_or_else(|| panic!("không có biến {name}"))
}

#[test]
fn unsuffixed_literals_default_to_i32_and_f64() {
    for (body, want) in [
        ("x ≔ 1;", "i32"),
        ("x ≔ 0;", "i32"),
        ("x ≔ 0xFF;", "i32"),
        ("x ≔ 0b1010;", "i32"),
        ("x ≔ 1_000;", "i32"),
        ("x ≔ 1.5;", "f64"),
        ("x ≔ 1e3;", "f64"),
        ("x ≔ 2.5e-3;", "f64"),
    ] {
        assert_eq!(local_ty(body, "x"), want, "{body}");
    }
}

#[test]
fn suffixed_literals_have_exactly_that_type() {
    for (body, want) in [
        ("x ≔ 1i8;", "i8"),
        ("x ≔ 1i16;", "i16"),
        ("x ≔ 1i32;", "i32"),
        ("x ≔ 1i64;", "i64"),
        ("x ≔ 1i128;", "i128"),
        ("x ≔ 1isize;", "isize"),
        ("x ≔ 1u8;", "u8"),
        ("x ≔ 1u16;", "u16"),
        ("x ≔ 1u32;", "u32"),
        ("x ≔ 1u64;", "u64"),
        ("x ≔ 1u128;", "u128"),
        ("x ≔ 1usize;", "usize"),
        ("x ≔ 0xFFu8;", "u8"),
        ("x ≔ 0o17u8;", "u8"),
        ("x ≔ 0b1010_0101u8;", "u8"),
        ("x ≔ 1_000_000i64;", "i64"),
        ("x ≔ 1f32;", "f32"),
        ("x ≔ 1f64;", "f64"),
        ("x ≔ 2.5f32;", "f32"),
        ("x ≔ 2.5e-3f64;", "f64"),
    ] {
        assert_eq!(local_ty(body, "x"), want, "{body}");
    }
}

#[test]
fn other_literal_kinds() {
    for (body, want) in [
        ("x ≔ true;", "bool"),
        ("x ≔ false;", "bool"),
        ("x ≔ 'a';", "char"),
        ("x ≔ \"s\";", "&str"),
        ("x ≔ ();", "()"),
        ("x ≔ (1, true);", "(i32, bool)"),
        ("x ≔ (7,);", "(i32,)"),
        ("x ≔ [1, 2, 3];", "[i32; 3]"),
        ("x ≔ [0u8; 4];", "[u8; 4]"),
        ("x ≔ 1..2;", "Range<i32>"),
        ("x ≔ 1u8..=9;", "RangeInclusive<u8>"),
    ] {
        assert_eq!(local_ty(body, "x"), want, "{body}");
    }
}

#[test]
fn literals_take_their_type_from_use() {
    // vế còn lại / tham số / trả về / mảng quyết định kiểu của literal
    assert_eq!(local_ty("a ≔ 1u8; x ≔ a + 2;", "x"), "u8");
    assert_eq!(local_ty("a ≔ 1i64; μs ≔ 0; s ← s + a;", "s"), "i64");
    assert_eq!(local_ty("x ≔ [1u8, 2, 3];", "x"), "[u8; 3]");
    let out = ok("ƒ take(b: u16) ! ∅ { } ƒ f() ! ∅ { x ≔ 5; take(x); }");
    assert_eq!(out.local_type("main::f", "x").as_deref(), Some("u16"));
    let out = ok("ƒ f() → u64 ! ∅ { x ≔ 5; x }");
    assert_eq!(out.local_type("main::f", "x").as_deref(), Some("u64"));
    assert_eq!(local_ty("x: i64 ≔ 7;", "x"), "i64");
    assert_eq!(local_ty("x: f32 ≔ 1.5;", "x"), "f32");
}

#[test]
fn literal_range_is_not_a_type_error_it_belongs_to_m7_e0601() {
    // semantics.md §1: literal ngoài phạm vi kiểu là lỗi biên chắc chắn E0601 (M7), không phải E0301.
    ok(
        "ƒ f() ! ∅ { a ≔ 255u8; b ≔ -128i8; c ≔ 127i8; d ≔ 0u8; e ≔ 4294967295u32; g ≔ 18446744073709551615u64; }",
    );
    ok("ƒ f() ! ∅ { a ≔ 2147483647; b ≔ -2147483648; }");
    for later in [
        "x ≔ 256u8;",
        "x ≔ 128i8;",
        "x: u8 ≔ 300;",
        "x ≔ 2147483648;",
    ] {
        ok(&format!("ƒ f() ! ∅ {{ {later} }}"));
    }
    // vượt cả u128 thì không kiểu nào chứa được: không biểu diễn được ngay tại M4
    err(
        "ƒ f() ! ∅ { x ≔ 340282366920938463463374607431768211456u128; }",
        "E0301",
        1,
    );
}

#[test]
fn no_implicit_numeric_conversion_between_different_types_d1() {
    for bad in [
        "a ≔ 1i32; b: i64 ≔ a;",
        "a ≔ 1u8; b: u16 ≔ a;",
        "a ≔ 1i64; b: i32 ≔ a;",
        "a ≔ 1.5f32; b: f64 ≔ a;",
        "a ≔ 1i32; b: f64 ≔ a;",
        "b: f64 ≔ 1;",
        "b: i32 ≔ 1.0;",
        "a ≔ 1i32; b ≔ 2i64; c ≔ a + b;",
        "a ≔ 1u8; b ≔ 2i8; c ≔ a +% b;",
    ] {
        err(&format!("ƒ f() ! ∅ {{ {bad} }}"), "E0302", 1);
    }
    // cùng kiểu, hoặc ép tường minh: được
    ok("ƒ f() ! ∅ { a ≔ 1i32; b: i64 ≔ a as i64; c ≔ a as f64; d ≔ 2.5f64 as i32; }");
}

#[test]
fn e0302_comes_with_a_machine_readable_explicit_cast_suggestion() {
    let out = err("ƒ f() ! ∅ { a ≔ 1i32; b: i64 ≔ a; }", "E0302", 1);
    let d = &out.report.diagnostics[0];
    assert_eq!(d.suggestions.len(), 1);
    assert!(d.suggestions[0].message.contains("as i64"));
    assert_eq!(d.suggestions[0].edits.len(), 1);
    assert_eq!(d.suggestions[0].edits[0].replacement, " as i64");
    // áp dụng gợi ý cho chương trình hợp lệ
    let src = "ƒ f() ! ∅ { a ≔ 1i32; b: i64 ≔ a; }";
    let fixed = gt_diag::apply_edits(src, &d.suggestions[0].edits).expect("áp dụng được");
    assert!(fixed.contains("b: i64 ≔ a as i64;"), "{fixed}");
    ok(&fixed);
}

#[test]
fn other_literal_mismatches_are_e0301() {
    for bad in [
        "x: i32 ≔ true;",
        "x: bool ≔ 1;",
        "x: char ≔ \"a\";",
        "x: &str ≔ 'a';",
        "x: i32 ≔ \"s\";",
        "x: () ≔ 1;",
        "x: (i32, i32) ≔ (1, true);",
        "x: [i32; 2] ≔ [1, 2, 3];",
        "x: u8 ≔ 'a';",
    ] {
        err(&format!("ƒ f() ! ∅ {{ {bad} }}"), "E0301", 1);
    }
}

#[test]
fn unsized_types_need_a_reference() {
    err("ƒ f(xs: &[i32]) ! ∅ { x ≔ *xs; }", "E0301", 1);
    ok("ƒ f(xs: &[i32]) ! ∅ { x ≔ xs; y ≔ &xs[0..1]; }");
}

#[test]
fn array_lengths_are_compile_time_constants() {
    ok("□ N: usize ≔ 4; ƒ f() ! ∅ { x ≔ [0; 4]; y ≔ [0u8; N]; z: [i32; 6] ≔ [0; N + 2]; }");
    err("ƒ f(n: usize) ! ∅ { x ≔ [0; n]; }", "E0301", 1);
    err("■ M: usize ≔ 3; ƒ f(a: [i32; M]) ! ∅ { }", "E0301", 1);
}

#[test]
fn casts_follow_the_allowed_table() {
    ok(
        "ƒ f() ! ∅ { a ≔ 1i32 as u8; b ≔ 1u8 as char; c ≔ 'a' as u32; d ≔ true as i32; e ≔ 1.5 as i32; g ≔ 3 as f32; }",
    );
    for bad in [
        "x ≔ \"s\" as i32;",
        "x ≔ 1i32 as bool;",
        "x ≔ 'a' as bool;",
        "x ≔ 1i32 as char;",
        "x ≔ (1, 2) as i32;",
    ] {
        err(&format!("ƒ f() ! ∅ {{ {bad} }}"), "E0301", 1);
    }
}
