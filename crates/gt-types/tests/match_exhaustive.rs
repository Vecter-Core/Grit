//! M4 bước (e): `⌥` đầy đủ theo kiểu (D6, E0305). Số nguyên/`char` vét cạn theo KHOẢNG (Q14); `str` và số thực cần `_`/tên (ADR-018 §5).
mod common;
use common::*;

fn m(scrut_ty: &str, arms: &str) -> String {
    format!(
        "⊔ E {{ A, B(i32), C(bool, bool) }} ▢ P {{ a: bool, b: bool }} ƒ f(x: {scrut_ty}) → i32 ! ∅ {{ ⌥ x {{ {arms} }} }}"
    )
}

#[test]
fn enum_must_cover_every_variant() {
    ok(&m("E", "A ↦ 1, B(_) ↦ 2, C(_, _) ↦ 3"));
    ok(&m("E", "A ↦ 1, B(n) ↦ n, C(a, b) ↦ 3"));
    ok(&m("E", "A ↦ 1, _ ↦ 2"));
    ok(&m("E", "n ↦ 1"));
    for bad in [
        "A ↦ 1, B(_) ↦ 2",
        "A ↦ 1",
        "B(_) ↦ 1, C(_, _) ↦ 2",
        "A ↦ 1, C(_, _) ↦ 2",
    ] {
        err(&m("E", bad), "E0305", 1);
    }
    // thông điệp nêu giá trị chưa được phủ
    let out = err(&m("E", "A ↦ 1, B(_) ↦ 2"), "E0305", 1);
    assert!(
        out.report.diagnostics[0].message.contains("C(_, _)"),
        "{}",
        out.report.diagnostics[0].message
    );
}

#[test]
fn bool_needs_both_values() {
    ok(&m("bool", "true ↦ 1, false ↦ 2"));
    ok(&m("bool", "true ↦ 1, _ ↦ 2"));
    err(&m("bool", "true ↦ 1"), "E0305", 1);
    err(&m("bool", "false ↦ 1"), "E0305", 1);
    let out = err(&m("bool", "true ↦ 1"), "E0305", 1);
    assert!(out.report.diagnostics[0].message.contains("false"));
}

#[test]
fn nested_patterns_are_checked_in_depth() {
    ok("ƒ f(o: Option<bool>) → i32 ! ∅ { ⌥ o { Some(true) ↦ 1, Some(false) ↦ 2, None ↦ 3 } }");
    err(
        "ƒ f(o: Option<bool>) → i32 ! ∅ { ⌥ o { Some(true) ↦ 1, None ↦ 3 } }",
        "E0305",
        1,
    );
    ok(
        "ƒ f(o: Option<Option<u8>>) → i32 ! ∅ { ⌥ o { Some(Some(_)) ↦ 1, Some(None) ↦ 2, None ↦ 3 } }",
    );
    err(
        "ƒ f(o: Option<Option<u8>>) → i32 ! ∅ { ⌥ o { Some(Some(_)) ↦ 1, None ↦ 3 } }",
        "E0305",
        1,
    );
}

#[test]
fn tuples_and_structs_are_covered_component_wise() {
    ok(
        "ƒ f(t: (bool, bool)) → i32 ! ∅ { ⌥ t { (true, true) ↦ 1, (true, false) ↦ 2, (false, _) ↦ 3 } }",
    );
    err(
        "ƒ f(t: (bool, bool)) → i32 ! ∅ { ⌥ t { (true, true) ↦ 1, (false, _) ↦ 3 } }",
        "E0305",
        1,
    );
    ok("ƒ f(t: (bool, i32)) → i32 ! ∅ { ⌥ t { (true, n) ↦ n, (false, _) ↦ 0 } }");
    ok(&m(
        "P",
        "P { a: true, b: true } ↦ 1, P { a: true, b: false } ↦ 2, P { a: false, .. } ↦ 3",
    ));
    err(
        &m("P", "P { a: true, b: true } ↦ 1, P { a: false, .. } ↦ 3"),
        "E0305",
        1,
    );
    ok(&m("P", "P { .. } ↦ 1"));
}

#[test]
fn integers_need_a_catch_all_unless_ranges_cover_the_whole_type() {
    ok(&m("i32", "0 ↦ 1, _ ↦ 2"));
    ok(&m("i32", "0 ↦ 1, n ↦ n"));
    err(&m("i32", "0 ↦ 1"), "E0305", 1);
    err(&m("i32", "0 ↦ 1, 1 ↦ 2, 2 ↦ 3"), "E0305", 1);
    // phủ bằng khoảng (Q14): đầy đủ không cần `_`
    ok(&m("i32", "..0 ↦ 0, 0..=9 ↦ 1, 10.. ↦ 2"));
    ok(&m("i32", "..1 ↦ 0, 1.. ↦ 1"));
    ok(&m("u8", "0..=127 ↦ 0, 128..=255 ↦ 1"));
    ok(&m("u8", "0.. ↦ 0"));
    ok(&m("i8", "-128..=-1 ↦ 0, 0..=127 ↦ 1"));
    // hở một giá trị / một đoạn
    err(&m("i32", "..0 ↦ 0, 1.. ↦ 2"), "E0305", 1); // thiếu 0
    err(&m("u8", "0..=127 ↦ 0, 129..=255 ↦ 1"), "E0305", 1); // thiếu 128
    err(&m("u8", "0..=254 ↦ 0"), "E0305", 1); // thiếu 255
    err(&m("i32", "..0 ↦ 0, 0..10 ↦ 1"), "E0305", 1); // 10.. chưa phủ
    // literal đơn lẻ lấp đủ miền nhỏ cũng được tính (khoảng một điểm)
    ok(&m("u8", "0 ↦ 0, 1..=255 ↦ 1"));
    // nửa khoảng mở kết thúc loại trừ
    ok(&m("u8", "0..128 ↦ 0, 128.. ↦ 1"));
}

#[test]
fn char_ranges_follow_the_same_rule() {
    ok(&m("char", "'a' ↦ 1, _ ↦ 2"));
    err(&m("char", "'a' ↦ 1"), "E0305", 1);
    ok(&m("char", "..'{' ↦ 0, '{'.. ↦ 1"));
    err(&m("char", "..'z' ↦ 0, '{'.. ↦ 1"), "E0305", 1);
}

#[test]
fn str_and_floats_need_a_catch_all() {
    ok(&m("&str", "\"a\" ↦ 1, _ ↦ 2"));
    ok(&m("&str", "\"a\" ↦ 1, s ↦ 2"));
    err(&m("&str", "\"a\" ↦ 1"), "E0305", 1);
    err(&m("&str", "\"a\" ↦ 1, \"b\" ↦ 2"), "E0305", 1);
    ok(&m("f64", "1.5 ↦ 1, _ ↦ 2"));
    err(&m("f64", "1.5 ↦ 1"), "E0305", 1);
}

#[test]
fn arms_after_a_catch_all_are_not_an_error_in_m4() {
    // mã chết là GHI CHÚ của M5, không phải lỗi M4
    ok(&m("bool", "_ ↦ 1, true ↦ 2"));
    ok(&m("E", "n ↦ 1, A ↦ 2"));
}

#[test]
fn match_on_a_reference_scrutinee_matches_through_the_reference_adr040() {
    // ADR-040 (Q87): trước đây E0301; nay khớp qua tham chiếu, v: &i32
    ok("ƒ f(o: &Option<i32>) → i32 ! ∅ { ⌥ o { Some(v) ↦ *v, None ↦ 0 } }");
    // pattern hằng trên tham chiếu vẫn là lỗi kiểu (không tương tác ngầm)
    err(
        "ƒ f(r: &i32) → i32 ! ∅ { ⌥ r { 0 ↦ 1, _ ↦ 2 } }",
        "E0301",
        1,
    );
    // biến bind là &i32, không phải i32
    err(
        "ƒ f(o: &Option<i32>) → i32 ! ∅ { ⌥ o { Some(v) ↦ v, None ↦ 0 } }",
        "E0301",
        1,
    );
}

#[test]
fn the_approved_range_pattern_sample_is_exhaustive() {
    // tests/pass/29_range_open_patterns.gt (Q14): ..0, 0..=9, 10.. phủ hết i32 mà không cần `_`
    ok("ƒ bucket(n: i32) → i32 ! ∅ { ⌥ n { ..0 ↦ 0, 0..=9 ↦ 1, 10.. ↦ 2 } }");
}

#[test]
fn nested_reference_pattern_reports_exactly_one_error_without_cascade() {
    // ADR-040: tầng tham chiếu thứ hai báo E0301 MỘT lần; biến bind trong mẫu lỗi có kiểu `Err` nên không sinh lỗi phụ khi dùng
    err(
        "▢ P { x: u32 }\nƒ f(o: &Option<&P>) → u32 ! ∅ { ⌥ o { Some(P { x }) ↦ *x, None ↦ 0 } }",
        "E0301",
        1,
    );
}
