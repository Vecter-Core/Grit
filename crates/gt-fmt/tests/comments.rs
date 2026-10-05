//! Comment: gắn đúng chỗ, giữ nguyên số lượng/nội dung/thứ tự, và TỪ CHỐI (E1301) khi không gắn được. Viết TRƯỚC (TDD).
mod common;
use common::*;

#[test]
fn own_line_trailing_and_dangling_comments_in_a_block() {
    assert_eq!(
        fmt_ok("// header\nƒ f()!∅{\n// before stmt\nx≔1; // trailing\n// dangling at end\n}\n"),
        "// header\nƒ f() ! ∅ {\n  // before stmt\n  x ≔ 1;  // trailing\n  // dangling at end\n}\n"
    );
    // comment sau `{` cùng dòng
    assert_eq!(
        fmt_ok("ƒ g()!∅{ // after open\n 1 }"),
        "ƒ g() ! ∅ { // after open\n  1\n}\n"
    );
    // comment trước tail
    assert_eq!(
        fmt_ok("ƒ g()→i32!∅{\n// why\n1}"),
        "ƒ g() → i32 ! ∅ {\n  // why\n  1\n}\n"
    );
    // comment sau tail cùng dòng
    assert_eq!(
        fmt_ok("ƒ g()→i32!∅{\n1 // one\n}"),
        "ƒ g() → i32 ! ∅ {\n  1  // one\n}\n"
    );
}

#[test]
fn comments_between_items_and_at_file_edges() {
    assert_eq!(
        fmt_ok("ƒ a()!∅{}\n// about b\nƒ b()!∅{}"),
        "ƒ a() ! ∅ { }\n\n// about b\nƒ b() ! ∅ { }\n"
    );
    assert_eq!(
        fmt_ok("ƒ a()!∅{} // note\nƒ b()!∅{}"),
        "ƒ a() ! ∅ { }  // note\n\nƒ b() ! ∅ { }\n"
    );
    assert_eq!(
        fmt_ok("ƒ a()!∅{}\n// the end\n"),
        "ƒ a() ! ∅ { }\n// the end\n"
    );
    assert_eq!(fmt_ok("// only\n"), "// only\n");
    assert_eq!(fmt_ok("// a\n// b\n"), "// a\n// b\n");
    assert_eq!(fmt_ok(""), "");
    assert_eq!(
        fmt_ok("// a\n\n\n// b\nƒ a()!∅{}"),
        "// a\n// b\nƒ a() ! ∅ { }\n"
    );
    // CRLF vào, LF ra
    assert_eq!(fmt_ok("// c\r\nƒ a()!∅{}\r\n"), "// c\nƒ a() ! ∅ { }\n");
}

#[test]
fn comments_in_struct_enum_trait_impl_mod_and_match() {
    assert_eq!(
        fmt_ok("▢ P{ // open\n x:i32, // tx\n // before y\n y:i32\n}"),
        "▢ P { // open\n  x: i32,  // tx\n  // before y\n  y: i32,\n}\n"
    );
    assert_eq!(
        fmt_ok("⊔ E{\n// first\nA, // a\nB\n// last\n}"),
        "⊔ E {\n  // first\n  A,  // a\n  B,\n  // last\n}\n"
    );
    assert_eq!(
        fmt_ok("⊏ T{\n// m\nƒ f(&self)!∅; // sig\n}"),
        "⊏ T {\n  // m\n  ƒ f(&self) ! ∅;  // sig\n}\n"
    );
    assert_eq!(
        fmt_ok("⊐ T{\n// m1\nƒ f()!∅{}\n// m2\nƒ g()!∅{}\n}"),
        "⊐ T {\n  // m1\n  ƒ f() ! ∅ { }\n\n  // m2\n  ƒ g() ! ∅ { }\n}\n"
    );
    assert_eq!(
        fmt_ok("▦ m{\n// in\nƒ a()!∅{}\n}"),
        "▦ m {\n  // in\n  ƒ a() ! ∅ { }\n}\n"
    );
    assert_eq!(
        fmt_ok("ƒ f(o:i32)→i32!∅{⌥ o{\n// first\n1↦1, // one\n_↦2 }}"),
        "ƒ f(o: i32) → i32 ! ∅ {\n  ⌥ o {\n    // first\n    1 ↦ 1,  // one\n    _ ↦ 2,\n  }\n}\n"
    );
    // comment trong khối lồng của biểu thức
    assert_eq!(
        fmt_ok("ƒ f(c:bool)→i32!∅{⎇ c{ // yes\n1}⇝{2}}"),
        "ƒ f(c: bool) → i32 ! ∅ {\n  ⎇ c { // yes\n    1\n  } ⇝ { 2 }\n}\n"
    );
}

#[test]
fn comments_that_cannot_be_placed_are_refused_not_dropped_or_moved() {
    for bad in [
        "ƒ f()→i32!∅{1+ // c\n2}",              // giữa biểu thức
        "ƒ f(a:i32, // c\nb:i32)!∅{}",          // giữa tham số
        "/// d\n// c\nƒ f()!∅{}",               // giữa doc comment và item
        "ƒ f()!∅ // c\n{}",                     // giữa chữ ký và thân
        "ƒ f(c:bool)→i32!∅{⎇ c{1} // x\n⇝{2}}", // giữa then và else
        "ƒ f(p:&// c\ni32)!∅{}",                // trong kiểu
        "▢ P{x:i32 // c\n,y:i32}", // trước dấu phẩy trường (sau kiểu, trước `,`) vẫn thuộc node => chấp nhận? xem dưới
    ] {
        let codes = fmt_err_or_skip(bad);
        if let Some(c) = codes {
            assert_eq!(c, ["E1301"], "{bad}");
        }
    }
    // một lỗi cho mỗi comment không gắn được
    assert_eq!(
        fmt_err("ƒ f()→i32!∅{1+ // a\n2+ // b\n3}"),
        ["E1301", "E1301"]
    );
}

/// Trường hợp biên: một số vị trí vẫn hợp lệ — hàm trả Some(mã) nếu bị từ chối, None nếu định dạng được (khi đó kiểm bảo toàn comment).
fn fmt_err_or_skip(src: &str) -> Option<Vec<String>> {
    match gt_fmt::format("t.gt", src) {
        Ok(out) => {
            assert_eq!(
                comment_texts(&out),
                comment_texts(src),
                "comment bị đổi/mất:\n{src}\n=> {out}"
            );
            None
        }
        Err(e) => Some(
            e.report
                .diagnostics
                .iter()
                .map(|d| d.code.as_str().to_string())
                .collect(),
        ),
    }
}

#[test]
fn refusal_diagnostic_points_at_the_comment_and_explains() {
    let src = "ƒ f()→i32!∅{1+ // c\n2}";
    let e = gt_fmt::format("t.gt", src).expect_err("từ chối");
    let d = &e.report.diagnostics[0];
    assert_eq!(d.code.as_str(), "E1301");
    assert_eq!(&src[d.span.start as usize..d.span.end as usize], "// c");
    assert!(
        d.notes.iter().any(|n| n.contains("dòng riêng")),
        "{:?}",
        d.notes
    );
}

#[test]
fn syntax_errors_are_refused_with_the_parser_diagnostics() {
    assert_eq!(fmt_err("ƒ f() → i32 ! ∅ { 1 + }"), ["E0101"]);
    assert_eq!(fmt_err("ƒ f() → i32 ! ∅ { 1\u{200B} }"), ["E1202"]);
}

#[test]
fn comment_count_content_and_order_are_preserved() {
    let src = "// h1\n// h2\nƒ a()!∅{ // o\n// m\nx≔1; // t\n// d\n}\n// between\n▢ P{ // so\nx:i32, // t\n}\n// end\n";
    let out = fmt_ok(src);
    assert_eq!(comment_texts(&out), comment_texts(src));
    assert_eq!(comment_texts(&out).len(), 10);
    // idempotent kể cả với comment
    assert_eq!(fmt_ok(&out), out);
}

/// Tính chất tổng quát: chèn `// zz` sau MỖI token của mọi file pass → kết quả chỉ có hai khả năng:
/// (a) định dạng được và mọi comment còn nguyên (số lượng, nội dung, thứ tự), (b) bị từ chối chỉ bằng E1301.
/// Không bao giờ được mất/đổi comment, không ICE, không mã lỗi khác.
#[test]
fn a_comment_inserted_after_any_token_is_kept_or_refused_never_lost() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/pass");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .collect();
    files.sort();
    let (mut kept, mut refused) = (0u32, 0u32);
    for p in files {
        let src = std::fs::read_to_string(&p).unwrap().replace("\r\n", "\n");
        let toks = gt_lexer::lex(&src).tokens;
        let mut ends: Vec<usize> = toks
            .iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    gt_lexer::TokenKind::DocComment(_) | gt_lexer::TokenKind::Eof
                )
            })
            .map(|t| t.span.end as usize)
            .collect();
        ends.dedup();
        for end in ends {
            // chỉ chèn ở cuối dòng của chính token (phần còn lại của dòng được đẩy xuống dòng sau)
            let mut s = src.clone();
            s.insert_str(end, " // zz\n");
            // chèn xuống dòng có thể đổi cú pháp (vd `t.` + xuống dòng + `0.1` thành số thực) → bỏ qua, không phải lỗi của fmt
            if !gt_parser::parse("t.gt", &s).report.diagnostics.is_empty() {
                continue;
            }
            let before = comment_texts(&s);
            match gt_fmt::format("t.gt", &s) {
                Ok(out) => {
                    assert_eq!(
                        comment_texts(&out),
                        before,
                        "comment đổi/mất: {} @{end}\n{out}",
                        p.display()
                    );
                    kept += 1;
                }
                Err(e) => {
                    let codes: Vec<_> = e
                        .report
                        .diagnostics
                        .iter()
                        .map(|d| d.code.as_str().to_string())
                        .collect();
                    assert!(
                        !codes.is_empty() && codes.iter().all(|c| c == "E1301"),
                        "{} @{end}: {codes:?}",
                        p.display()
                    );
                    refused += 1;
                }
            }
        }
    }
    eprintln!("chèn comment sau từng token: giữ {kept}, từ chối E1301 {refused}");
    // cả hai nhánh đều phải xảy ra (nếu không, test không kiểm gì)
    assert!(kept > 100 && refused > 100, "kept={kept} refused={refused}");
}

#[test]
fn comments_glued_to_the_end_of_a_node_are_trailing_not_refused() {
    // không có khoảng trắng giữa node và `//`: vẫn là comment cuối dòng của node đó
    assert_eq!(
        fmt_ok("ƒ a()!∅{}// n\nƒ b()!∅{}"),
        "ƒ a() ! ∅ { }  // n\n\nƒ b() ! ∅ { }\n"
    );
    assert_eq!(
        fmt_ok("ƒ g()→i32!∅{\nx≔1;// s\n1// t\n}"),
        "ƒ g() → i32 ! ∅ {\n  x ≔ 1;  // s\n  1  // t\n}\n"
    );
    assert_eq!(
        fmt_ok("▢ P{ x:i32,// a\ny:i32// b\n}"),
        "▢ P {\n  x: i32,  // a\n  y: i32,  // b\n}\n"
    );
}

/// GIỚI HẠN ĐÃ BIẾT (docs/KNOWN_LIMITATIONS.md): các vị trí dưới đây là chỗ người viết rất hay đặt comment nhưng gt-fmt
/// CHƯA có quy tắc gắn → từ chối (E1301), không xoá, không dời. Test khoá hành vi hiện tại: khi mở rộng quy tắc gắn, sửa test này có chủ đích.
#[test]
fn known_limitation_comments_in_signature_parts_and_call_args_are_refused() {
    for (what, src) in [
        (
            "sau dòng contract",
            "ƒ f(x: i32) → i32 ! ∅\n  ⊳ x > 0 // pre\n{ x }\n",
        ),
        (
            "giữa các contract (dòng riêng)",
            "ƒ f(x: i32) → i32 ! ∅\n  // pre\n  ⊳ x > 0\n{ x }\n",
        ),
        (
            "sau tham số",
            "ƒ f(\n  a: i32, // a\n  b: i32,\n) ! ∅ { }\n",
        ),
        (
            "trong danh sách đối số nhiều dòng",
            "ƒ f() ! ∅ {\n  g(\n    1, // one\n    2,\n  );\n}\n",
        ),
        ("trong generics", "ƒ f<T // t\n>() ! ∅ { }\n"),
        (
            "trước ⇝",
            "ƒ f(c: bool) → i32 ! ∅ {\n  ⎇ c { 1 }\n  // else case\n  ⇝ { 2 }\n}\n",
        ),
    ] {
        assert_eq!(fmt_err(src), ["E1301"], "{what}");
    }
}
