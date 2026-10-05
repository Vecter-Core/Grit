//! Trivia (comment `//`) — phải được GIỮ LẠI để `gt-fmt` không xoá comment của người dùng.
//! Viết TRƯỚC khi sửa lexer (TDD): lexer M1 hiện bỏ hẳn `//`.
use gt_lexer::{TokenKind, TriviaKind, lex, render_tokens};

fn texts(src: &str) -> Vec<(String, bool)> {
    lex(src)
        .trivia
        .iter()
        .map(|t| (t.text.clone(), t.own_line))
        .collect()
}

#[test]
fn line_comments_are_kept_with_exact_text_and_placement() {
    let src = "// a\nx // b\n  // c\ny\n";
    let out = lex(src);
    assert!(out.errors.is_empty());
    assert_eq!(
        texts(src),
        [
            ("// a".to_string(), true),
            ("// b".to_string(), false),
            ("// c".to_string(), true)
        ]
    );
    // token không bị ảnh hưởng: comment KHÔNG nằm trong luồng token (parser không phải lọc)
    assert_eq!(render_tokens(&out), ["IDENT(x)", "IDENT(y)"]);
    // span cắt lại đúng văn bản nguồn
    for t in &out.trivia {
        assert_eq!(&src[t.span.start as usize..t.span.end as usize], t.text);
        assert_eq!(t.kind, TriviaKind::LineComment);
    }
}

#[test]
fn edge_cases() {
    // "////" (≥4 dấu /) là comment thường (không phải doc)
    assert_eq!(texts("//// bốn\nx"), [("//// bốn".to_string(), true)]);
    // comment cuối tệp không có xuống dòng; CRLF không lọt '\r' vào văn bản
    assert_eq!(texts("x // cuối"), [("// cuối".to_string(), false)]);
    assert_eq!(texts("x // crlf\r\ny"), [("// crlf".to_string(), false)]);
    // comment rỗng
    assert_eq!(texts("//\nx"), [("//".to_string(), true)]);
    // thụt đầu dòng bằng tab/dấu cách vẫn là own_line
    assert_eq!(texts("\t  // t\nx"), [("// t".to_string(), true)]);
    // "//" trong chuỗi KHÔNG phải comment
    assert!(texts("\"// không phải\"").is_empty());
    // "///" là doc comment: vẫn là TOKEN, không phải trivia
    let out = lex("/// doc\nx");
    assert!(out.trivia.is_empty());
    assert_eq!(render_tokens(&out), ["DOC_COMMENT(doc)", "IDENT(x)"]);
    // không có comment
    assert!(lex("x y").trivia.is_empty());
    // comment chứa ký tự Unicode/ký hiệu Grit giữ nguyên
    assert_eq!(
        texts("// ƒ ≔ μ tiếng Việt 😀\n"),
        [("// ƒ ≔ μ tiếng Việt 😀".to_string(), true)]
    );
}

#[test]
fn trivia_are_in_source_order_and_do_not_overlap_tokens() {
    let src = "a // 1\n// 2\nb // 3\n";
    let out = lex(src);
    let spans: Vec<_> = out.trivia.iter().map(|t| t.span).collect();
    assert!(spans.windows(2).all(|w| w[0].end <= w[1].start));
    for t in &out.trivia {
        for k in out.tokens.iter().filter(|k| k.kind != TokenKind::Eof) {
            assert!(
                t.span.end <= k.span.start || k.span.end <= t.span.start,
                "comment chồng token"
            );
        }
    }
}

/// Mọi dòng có `//` trong corpus phải cho đúng một mục (trivia hoặc DOC_COMMENT): không comment nào bị nuốt.
#[test]
fn corpus_comment_count_is_preserved() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests");
    let mut checked = 0;
    for sub in ["pass", "fail", "warn", "lexer"] {
        for e in std::fs::read_dir(root.join(sub)).expect("dir") {
            let p = e.expect("entry").path();
            if p.extension().is_none_or(|x| x != "gt") {
                continue;
            }
            let src = std::fs::read_to_string(&p).expect("đọc");
            // corpus không có "//" trong chuỗi, nên đếm theo dòng là chính xác
            let expected = src.lines().filter(|l| l.contains("//")).count();
            let out = lex(&src);
            let docs = out
                .tokens
                .iter()
                .filter(|t| matches!(t.kind, TokenKind::DocComment(_)))
                .count();
            assert_eq!(
                out.trivia.len() + docs,
                expected,
                "{}: số comment không khớp",
                p.display()
            );
            checked += 1;
        }
    }
    assert!(checked >= 80, "chỉ kiểm {checked} file");
}

/// Với CRLF, span của comment phải khớp đúng `text` (không chứa CR cuối dòng).
#[test]
fn crlf_comment_span_excludes_cr() {
    let src = "x // crlf\r\n// own\r\ny";
    let out = lex(src);
    assert_eq!(out.trivia.len(), 2);
    for t in &out.trivia {
        assert_eq!(&src[t.span.start as usize..t.span.end as usize], t.text);
        assert!(!t.text.contains('\r'));
    }
}
