//! `gt-fmt` với `let` phân rã (edition 2, ADR-P2-10): idempotent, AST không đổi, comment bảo toàn; ở edition 1 cùng văn bản bị từ chối (không in lại âm thầm).
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
fn every_edition_2_let_sample_roundtrips() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/ed2/pass");
    let mut n = 0;
    for e in std::fs::read_dir(&dir).expect("đọc tests/ed2/pass") {
        let p = e.expect("entry").path();
        let is_let = p
            .file_name()
            .is_some_and(|f| f.to_string_lossy().starts_with("let_"));
        if is_let && p.extension().is_some_and(|x| x == "gt") {
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
fn destructuring_let_forms_keep_their_shape() {
    for (label, body) in [
        ("tuple", "ƒ f() ! ∅ { (a, b) ≔ g(); }"),
        ("bỏ qua", "ƒ f() ! ∅ { (a, _) ≔ g(); (_, b) ≔ g(); }"),
        ("lồng", "ƒ f() ! ∅ { ((a, b), c) ≔ g(); }"),
        ("struct", "ƒ f() ! ∅ { P { x, y: z, .. } ≔ g(); }"),
        ("có kiểu", "ƒ f() ! ∅ { (a, b): (u32, u32) ≔ g(); }"),
        (
            "vế phải phức tạp",
            "ƒ f() ! ∅ { (a, b) ≔ h(1u32 + 2u32, g()?); }",
        ),
        (
            "comment",
            "ƒ f() ! ∅ {\n  (a, b) ≔ g(); // tách\n  // dòng riêng\n  (c, d) ≔ g();\n}",
        ),
    ] {
        check(label, body);
    }
}

#[test]
fn the_same_text_is_refused_at_edition_1() {
    let e = gt_fmt::format_with_edition("t.gt", "ƒ f() ! ∅ { (a, b) ≔ g(); }", 1)
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
