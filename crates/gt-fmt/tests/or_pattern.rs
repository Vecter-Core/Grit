//! `gt-fmt` với or-pattern (edition 2, ADR-P2-11): idempotent, AST không đổi, comment bảo toàn; ở edition 1 cùng văn bản bị từ chối.
mod common;
use common::*;
use std::path::Path;

fn check(label: &str, src: &str) {
    let out = gt_fmt::format_with_edition("t.gt", src, 2)
        .unwrap_or_else(|e| panic!("{label}: gt-fmt từ chối: {:?}", e.report.diagnostics));
    assert_eq!(
        ast_of_edition(&out, 2),
        ast_of_edition(src, 2),
        "{label}: AST đổi sau fmt\n--- đầu ra ---\n{out}"
    );
    let again = gt_fmt::format_with_edition("t.gt", &out, 2).expect("fmt lần hai");
    assert_eq!(again, out, "{label}: không idempotent");
    assert_eq!(
        comment_texts_edition(&out, 2),
        comment_texts_edition(src, 2),
        "{label}: comment bị đổi"
    );
}

#[test]
fn every_edition_2_or_sample_roundtrips() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/ed2/pass");
    let mut n = 0;
    for e in std::fs::read_dir(&dir).expect("đọc tests/ed2/pass") {
        let p = e.expect("entry").path();
        let is_or = p
            .file_name()
            .is_some_and(|f| f.to_string_lossy().starts_with("or_"));
        if is_or && p.extension().is_some_and(|x| x == "gt") {
            check(
                &p.display().to_string(),
                &std::fs::read_to_string(&p).expect("đọc"),
            );
            n += 1;
        }
    }
    assert!(n >= 4, "chỉ {n} mẫu");
}

#[test]
fn or_pattern_forms_keep_their_shape() {
    for (label, body) in [
        (
            "literal",
            "ƒ f(x: u32) ! ∅ { ⌥ x { 1 | 2 | 3 ↦ 1, _ ↦ 0 } }",
        ),
        (
            "khoảng",
            "ƒ f(x: u32) ! ∅ { ⌥ x { 0..=9 | 20.. ↦ 1, _ ↦ 0 } }",
        ),
        (
            "lồng trong biến thể",
            "ƒ f(o: Option<u32>) ! ∅ { ⌥ o { Some(1 | 2) | None ↦ 1, _ ↦ 0 } }",
        ),
        (
            "lồng trong tuple",
            "ƒ f(t: (u32, u32)) ! ∅ { ⌥ t { (0 | 1, 2 | 3) ↦ 1, _ ↦ 0 } }",
        ),
        (
            "không dấu cách",
            "ƒ f(x: u32) ! ∅ { ⌥ x { 1|2 ↦ 1, _ ↦ 0 } }",
        ),
        ("trong let phân rã", "ƒ f() ! ∅ { (0 | 1, b) ≔ g(); }"),
        (
            "comment",
            "ƒ f(x: u32) ! ∅ {\n  ⌥ x {\n    1 | 2 ↦ 1, // hai\n    _ ↦ 0,\n  }\n}",
        ),
    ] {
        check(label, body);
    }
}

#[test]
fn the_same_text_is_refused_at_edition_1() {
    let e = gt_fmt::format_with_edition("t.gt", "ƒ f(x: u32) ! ∅ { ⌥ x { 1 | 2 ↦ 1, _ ↦ 0 } }", 1)
        .expect_err("phải từ chối");
    assert!(
        e.report
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "E0101"),
        "{:?}",
        e.report.diagnostics
    );
}
