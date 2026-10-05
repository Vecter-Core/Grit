//! Comment (ADR-016, Q16b): mỗi comment gắn vào TOKEN gần nhất — `own_line` vào token kế tiếp, cuối dòng vào token liền trước —
//! printer phát comment tại vị trí token và ép xuống dòng sau comment cuối dòng. E1301 chỉ còn là phương án cuối (xác minh thất bại).
//! Viết TRƯỚC (TDD). Nghiệm thu: chèn `// zz` sau từng token của corpus → 0 ca bị từ chối.
mod common;
use common::*;

#[test]
fn own_line_trailing_and_dangling_comments_in_a_block() {
    assert_eq!(
        fmt_ok("// header\nƒ f()!∅{\n// before stmt\nx≔1; // trailing\n// dangling at end\n}\n"),
        "// header\nƒ f() ! ∅ {\n  // before stmt\n  x ≔ 1;  // trailing\n  // dangling at end\n}\n"
    );
    // comment sau `{` cùng dòng: khối chứa comment không bao giờ in một dòng; comment cuối dòng cách `{` hai khoảng trắng
    assert_eq!(
        fmt_ok("ƒ g()!∅{ // after open\n 1 }"),
        "ƒ g() ! ∅ {  // after open\n  1\n}\n"
    );
    assert_eq!(
        fmt_ok("ƒ g()→i32!∅{\n// why\n1}"),
        "ƒ g() → i32 ! ∅ {\n  // why\n  1\n}\n"
    );
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
        "▢ P {  // open\n  x: i32,  // tx\n  // before y\n  y: i32,\n}\n"
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
    assert_eq!(
        fmt_ok("ƒ f(c:bool)→i32!∅{⎇ c{ // yes\n1}⇝{2}}"),
        "ƒ f(c: bool) → i32 ! ∅ {\n  ⎇ c {  // yes\n    1\n  } ⇝ { 2 }\n}\n"
    );
}

/// Q16b: các vị trí trước đây bị từ chối nay được đặt theo cơ chế token chung (không vá từng chỗ).
#[test]
fn comments_in_signature_parts_arguments_and_expressions_are_placed_by_the_token_rule() {
    // sau dòng contract / giữa các contract
    assert_eq!(
        fmt_ok("ƒ f(x: i32) → i32 ! ∅\n  ⊳ x > 0 // pre\n  ⊲ result > 0 // post\n{ x }\n"),
        "ƒ f(x: i32) → i32 ! ∅\n  ⊳ x > 0  // pre\n  ⊲ result > 0  // post\n{ x }\n"
    );
    assert_eq!(
        fmt_ok("ƒ f(x: i32) → i32 ! ∅\n  // pre\n  ⊳ x > 0\n{ x }\n"),
        "ƒ f(x: i32) → i32 ! ∅\n  // pre\n  ⊳ x > 0\n{ x }\n"
    );
    // sau tham số (dấu phẩy thừa cuối danh sách bị bỏ như thường lệ)
    assert_eq!(
        fmt_ok("ƒ f(\n  a: i32, // a\n  b: i32,\n) ! ∅ { }\n"),
        "ƒ f(a: i32,  // a\n  b: i32) ! ∅ { }\n"
    );
    // đối số nhiều dòng
    assert_eq!(
        fmt_ok("ƒ f() ! ∅ {\n  g(\n    1, // one\n    2,\n  );\n}\n"),
        "ƒ f() ! ∅ {\n  g(1,  // one\n    2);\n}\n"
    );
    // generics
    assert_eq!(
        fmt_ok("ƒ f<T // t\n>() ! ∅ { }\n"),
        "ƒ f<T  // t\n  >() ! ∅ { }\n"
    );
    // trước ⇝
    assert_eq!(
        fmt_ok("ƒ f(c: bool) → i32 ! ∅ {\n  ⎇ c { 1 }\n  // else case\n  ⇝ { 2 }\n}\n"),
        "ƒ f(c: bool) → i32 ! ∅ {\n  ⎇ c { 1 }\n    // else case\n    ⇝ { 2 }\n}\n"
    );
    // giữa biểu thức: comment cuối dòng sau toán tử ép xuống dòng
    assert_eq!(
        fmt_ok("ƒ f()→i32!∅{1+ // c\n2}"),
        "ƒ f() → i32 ! ∅ {\n  1 +  // c\n    2\n}\n"
    );
    // giữa doc comment và item
    assert_eq!(
        fmt_ok("/// d\n// c\nƒ f()!∅{}"),
        "/// d\n// c\nƒ f() ! ∅ { }\n"
    );
    // trong kiểu, trước dấu phẩy của trường
    assert_eq!(
        fmt_ok("ƒ f(p:&// c\ni32)!∅{}"),
        "ƒ f(p: &  // c\n  i32) ! ∅ { }\n"
    );
    // comment cuối dòng gắn vào token LIỀN TRƯỚC (`i32`), dấu phẩy nguồn đứng sau nó nên bị ép xuống dòng theo (quy tắc, không phải luật riêng)
    assert_eq!(
        fmt_ok("▢ P{x:i32 // c\n,y:i32}"),
        "▢ P {\n  x: i32  // c\n    ,\n  y: i32,\n}\n"
    );
    // nhiều comment cùng neo: giữ thứ tự
    // dòng tiếp theo thụt +2 so với dòng hiện tại (quy tắc duy nhất; chuỗi nhiều lần ép dòng tạo bậc thang)
    assert_eq!(
        fmt_ok("ƒ f()→i32!∅{1+ // a\n// b\n2+ // c\n3}"),
        "ƒ f() → i32 ! ∅ {\n  1 +  // a\n    // b\n    2 +  // c\n      3\n}\n"
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
    assert_eq!(fmt_ok(&out), out);
}

fn pass_files() -> Vec<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/pass");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .collect();
    files.sort();
    files
}

/// Kiểm một nguồn đã chèn comment: định dạng được (không từ chối), comment giữ nguyên số lượng/nội dung/thứ tự,
/// fmt idempotent, và AST của đầu ra == AST của nguồn.
fn check_comment_source(label: &str, s: &str) {
    let out = match gt_fmt::format("t.gt", s) {
        Ok(o) => o,
        Err(e) => panic!("{label}: BỊ TỪ CHỐI {:?}\n{s}", e.codes()),
    };
    assert_eq!(
        comment_texts(&out),
        comment_texts(s),
        "{label}: comment đổi/mất\n{out}"
    );
    assert_eq!(
        gt_fmt::format("t.gt", &out).expect("fmt lần 2"),
        out,
        "{label}: không idempotent\n{out}"
    );
    assert_eq!(ast_of(&out), ast_of(s), "{label}: AST đổi\n{out}");
}

/// NGHIỆM THU Q16b (test thường trực): chèn comment sau MỖI token của mọi file pass → 0 ca bị từ chối.
/// Hai dạng: cuối dòng (`tok // zz⏎`) và dòng riêng (`tok⏎// zz⏎`).
#[test]
fn a_comment_inserted_after_any_token_is_never_refused_and_never_lost() {
    let (mut n, mut skipped) = (0u32, 0u32);
    for p in pass_files() {
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
            for ins in [" // zz\n", "\n// zz\n"] {
                let mut s = src.clone();
                s.insert_str(end, ins);
                // chèn xuống dòng có thể đổi chính cú pháp nguồn (vd `t.` + xuống dòng + `0.1` thành số thực): không phải ca của fmt
                if !gt_parser::parse("t.gt", &s).report.diagnostics.is_empty() {
                    skipped += 1;
                    continue;
                }
                check_comment_source(&format!("{} @{end} {ins:?}", p.display()), &s);
                n += 1;
            }
        }
    }
    assert!(n > 3000, "chỉ kiểm {n} ca");
    assert!(
        skipped < 100,
        "bỏ qua quá nhiều ca nguồn không hợp lệ: {skipped}"
    );
}

/// Comment ở nhiều vị trí cùng lúc (mỗi token một comment) vẫn đặt được, giữ thứ tự, idempotent.
#[test]
fn a_comment_after_every_token_at_once_is_kept_in_order() {
    for p in pass_files().into_iter().take(12) {
        let src = std::fs::read_to_string(&p).unwrap().replace("\r\n", "\n");
        let toks = gt_lexer::lex(&src).tokens;
        let mut s = String::new();
        let mut pos = 0usize;
        for (k, t) in toks
            .iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    gt_lexer::TokenKind::DocComment(_) | gt_lexer::TokenKind::Eof
                )
            })
            .enumerate()
        {
            s.push_str(&src[pos..t.span.end as usize]);
            pos = t.span.end as usize;
            s.push_str(&format!(" // c{k}\n"));
        }
        s.push_str(&src[pos..]);
        if !gt_parser::parse("t.gt", &s).report.diagnostics.is_empty() {
            continue;
        }
        check_comment_source(&p.display().to_string(), &s);
    }
}

/// Token đóng `)` `]` `}`: comment dòng riêng đứng trước chúng thụt sâu hơn token đóng; `(` nguồn khớp thì comment đứng SAU nó.
#[test]
fn own_line_comments_before_closers_and_after_matched_openers() {
    assert_eq!(
        fmt_ok("ƒ f() ! ∅ {\n  g(1\n  // c\n  );\n}\n"),
        "ƒ f() ! ∅ {\n  g(1\n    // c\n  );\n}\n"
    );
    assert_eq!(
        fmt_ok("ƒ f() ! ∅ {\n  x ≔ [1, 2\n  // c\n  ];\n}\n"),
        "ƒ f() ! ∅ {\n  x ≔ [1, 2\n    // c\n  ];\n}\n"
    );
    assert_eq!(fmt_ok("ƒ a() ! ∅ {\n// c\n}\n"), "ƒ a() ! ∅ {\n  // c\n}\n");
    // comment dòng riêng SAU `(` khớp được phát sau `(`, không trước nó
    assert_eq!(
        fmt_ok("ƒ f() ! ∅ {\n  g(\n  // c\n  1);\n}\n"),
        "ƒ f() ! ∅ {\n  g(\n    // c\n    1);\n}\n"
    );
    // một comment trong tệp rỗng ngoài comment; tệp chỉ có comment không kết thúc bằng dòng trống thừa
    assert_eq!(fmt_ok("// a"), "// a\n");
}
