//! `gt-fmt` với toán tử `e?` (edition 2, ADR-P2-09): `fmt(fmt(x)) == fmt(x)`, AST không đổi sau fmt, comment bảo toàn — trên mọi mẫu `tests/ed2/pass` và các dạng hậu tố/ưu tiên khó
//! (`x?`, `f(x)?`, `x.f?`, `x?.f`, `x?[0]`, `(a + b)?`, `-x?`, `&x?`, `x??`, `x? + 1`, comment sau `?`). Ở edition 1 cùng văn bản bị từ chối (E0101), không bị in lại âm thầm.
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
fn every_edition_2_pass_sample_roundtrips() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/ed2/pass");
    let mut n = 0;
    for e in std::fs::read_dir(&dir).expect("đọc tests/ed2/pass") {
        let p = e.expect("entry").path();
        if p.extension().is_some_and(|x| x == "gt") {
            let src = std::fs::read_to_string(&p).expect("đọc");
            check(&p.display().to_string(), &src);
            n += 1;
        }
    }
    assert!(n >= 4, "chỉ {n} mẫu");
}

#[test]
fn postfix_question_mark_keeps_precedence_and_chains() {
    for (label, body) in [
        ("đứng riêng", "ƒ f() ! ∅ { v ≔ x?; }"),
        ("sau lời gọi", "ƒ f() ! ∅ { v ≔ g(a, b)?; }"),
        ("sau trường", "ƒ f() ! ∅ { v ≔ a.b?; }"),
        ("trước trường", "ƒ f() ! ∅ { v ≔ a?.b; }"),
        ("trước chỉ số", "ƒ f() ! ∅ { v ≔ a?[0]; }"),
        ("lồng", "ƒ f() ! ∅ { v ≔ a??; }"),
        ("trong phép cộng", "ƒ f() ! ∅ { v ≔ a? + b?; }"),
        ("trên biểu thức cần ngoặc", "ƒ f() ! ∅ { v ≔ (a + b)?; }"),
        (
            "sau toán tử một ngôi",
            "ƒ f() ! ∅ { v ≔ -a?; w ≔ ¬b?; z ≔ &c?; }",
        ),
        ("hợp với `+?`", "ƒ f() ! ∅ { v ≔ (a +? b)?; w ≔ a +? b?; }"),
        ("không dấu cách thừa", "ƒ f() ! ∅ { v ≔ a ? ; }"),
        (
            "comment cuối dòng",
            "ƒ f() ! ∅ {\n  v ≔ a?; // sau ?\n  // trước dòng\n  w ≔ b?;\n}",
        ),
    ] {
        check(label, body);
    }
}

#[test]
fn the_same_text_is_refused_at_edition_1_not_silently_reformatted() {
    let e =
        gt_fmt::format_with_edition("t.gt", "ƒ f() ! ∅ { v ≔ a?; }", 1).expect_err("phải từ chối");
    assert!(
        e.report
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "E0101"),
        "{:?}",
        e.report.diagnostics
    );
}
