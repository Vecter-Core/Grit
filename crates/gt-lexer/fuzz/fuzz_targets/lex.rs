#![no_main]
//! Fuzz lexer: với MỌI chuỗi UTF-8, lexer không panic/treo và trả span hợp lệ, đơn điệu, kết thúc bằng Eof.
use gt_lexer::{TokenKind, lex};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else { return };
    let out = lex(src);
    assert_eq!(out.tokens.last().map(|t| &t.kind), Some(&TokenKind::Eof));
    let mut prev_end = 0u32;
    for t in &out.tokens {
        assert!(t.span.start >= prev_end && t.span.start <= t.span.end);
        assert!(src.is_char_boundary(t.span.start as usize) && src.is_char_boundary(t.span.end as usize));
        assert!(t.span.end as usize <= src.len());
        prev_end = t.span.end;
    }
    // trivia (comment `//`): span hợp lệ, đúng thứ tự, text khớp nguồn, một dòng
    let mut prev_trivia_end = 0u32;
    for t in &out.trivia {
        assert!(t.span.start >= prev_trivia_end && t.span.end as usize <= src.len());
        assert!(src.is_char_boundary(t.span.start as usize) && src.is_char_boundary(t.span.end as usize));
        assert_eq!(&src[t.span.start as usize..t.span.end as usize], t.text);
        assert!(t.text.starts_with("//") && !t.text.contains('\n'));
        prev_trivia_end = t.span.end;
    }
    for e in &out.errors {
        assert!(e.span.end as usize <= src.len());
        assert!(src.is_char_boundary(e.span.start as usize) && src.is_char_boundary(e.span.end as usize));
    }
    // quyết định phải xác định: lex hai lần cho cùng kết quả (D9)
    assert_eq!(out, lex(src));
});
