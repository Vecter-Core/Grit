//! Q16c: `x as T < y` là quy tắc NGÔN NGỮ (như Rust): `<` sau kiểu mở danh sách tham số kiểu. Parser phát E0101 kèm gợi ý
//! `machine-applicable` bọc `(x as T)`; áp dụng gợi ý cho chương trình hợp lệ có đúng AST dự kiến. Viết TRƯỚC (TDD).
use gt_diag::{Applicability, apply_edits};

fn wrap(e: &str) -> String {
    format!("ƒ f(x: i32, a: i64, b: i64, c: i64) → bool ! ∅ {{ {e} }}")
}

fn parse_diags(src: &str) -> Vec<gt_diag::Diagnostic> {
    gt_parser::parse("t.gt", src).report.diagnostics
}

/// Mỗi ca: (biểu thức sai, biểu thức đúng sau khi áp dụng gợi ý).
const CASES: &[(&str, &str)] = &[
    ("x as i64 < a", "(x as i64) < a"),
    ("a + b as i64 < c", "a + (b as i64) < c"),
    ("x as i32 as i64 < a", "(x as i32 as i64) < a"),
    ("x as m::T < a", "(x as m::T) < a"),
    ("-x as i64 < a", "(-x as i64) < a"),
    ("(x as i64 < a)", "((x as i64) < a)"),
    ("c ∧ x as i64 < a", "c ∧ (x as i64) < a"),
];

#[test]
fn cast_then_less_than_is_one_error_with_a_machine_applicable_suggestion() {
    for (bad, good) in CASES {
        let src = wrap(bad);
        let ds = parse_diags(&src);
        assert_eq!(ds.len(), 1, "{bad}: {ds:?}");
        let d = &ds[0];
        assert_eq!(d.code.as_str(), "E0101", "{bad}");
        // trỏ đúng vào `<`
        assert_eq!(
            &src[d.span.start as usize..d.span.end as usize],
            "<",
            "{bad}"
        );
        assert!(
            d.message.contains("as") && d.message.contains('('),
            "{bad}: {}",
            d.message
        );
        assert_eq!(d.suggestions.len(), 1, "{bad}");
        let s = &d.suggestions[0];
        assert_eq!(s.applicability, Applicability::MachineApplicable, "{bad}");
        assert_eq!(s.edits.len(), 2, "{bad}");
        // áp dụng gợi ý → đúng văn bản dự kiến, parse sạch
        let fixed = apply_edits(&src, &s.edits).expect("áp dụng được");
        assert_eq!(fixed, wrap(good), "{bad}");
        assert!(
            parse_diags(&fixed).is_empty(),
            "{bad} → {fixed}: {:?}",
            parse_diags(&fixed)
        );
    }
}

#[test]
fn applying_the_suggestion_gives_a_comparison_of_a_cast() {
    let fixed = {
        let src = wrap("x as i64 < a");
        let d = &parse_diags(&src)[0];
        apply_edits(&src, &d.suggestions[0].edits).unwrap()
    };
    let out = gt_parser::parse("t.gt", &fixed);
    let dbg = format!("{:?}", out.program);
    let lt = dbg.find("op: Lt").expect("Lt");
    let cast = dbg.find("Cast").expect("Cast");
    assert!(
        lt < cast,
        "phép so sánh phải là gốc, ép kiểu là toán hạng trái"
    );
}

#[test]
fn valid_forms_still_parse() {
    for ok in [
        "(x as i64) < a",
        "x as i64 > a",
        "x as i64 <= a",
        "x as i64 == a",
        "a < x as i64",
    ] {
        let src = wrap(ok);
        assert!(
            parse_diags(&src).is_empty(),
            "{ok}: {:?}",
            parse_diags(&src)
        );
    }
    // kiểu có tham số kiểu thật sự vẫn được chấp nhận sau `as`
    assert!(parse_diags(&wrap("{ v ≔ x as Vec<i64>; true }")).is_empty());
}

#[test]
fn a_real_generic_error_after_as_is_not_mistaken_for_the_lt_rule() {
    // sau `<` không có biểu thức nào → không thể là phép so sánh → lỗi cú pháp thường, KHÔNG có gợi ý bọc ngoặc
    for bad in ["{ v ≔ x as Vec< ; true }", "{ v ≔ x as Vec<, ; true }"] {
        let ds = parse_diags(&wrap(bad));
        assert!(!ds.is_empty(), "{bad}");
        assert!(ds.iter().all(|d| d.suggestions.is_empty()), "{bad}: {ds:?}");
    }
}

/// Mơ hồ vốn có: `x as Vec<i64 ;` trùng chuỗi token với `(x as Vec) < i64;` → coi là phép so sánh (đúng quy tắc ngôn ngữ).
#[test]
fn an_unclosed_generic_that_reads_as_a_comparison_is_reported_as_the_lt_rule() {
    let ds = parse_diags(&wrap("{ v ≔ x as Vec<i64 ; true }"));
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].suggestions.len(), 1);
}
