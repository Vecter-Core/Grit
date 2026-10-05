//! Test chi tiết của lexer: số, chuỗi, comment, CRLF, Unicode nguy hiểm, span, và độ bền (không panic).
use gt_lexer::{ErrorCode, TokenKind, lex, render_tokens};

fn toks(src: &str) -> Vec<String> {
    let out = lex(src);
    assert!(
        out.errors.is_empty(),
        "không mong đợi lỗi cho {src:?}: {:?}",
        out.errors
    );
    render_tokens(&out)
}

fn codes(src: &str) -> Vec<ErrorCode> {
    lex(src).errors.iter().map(|e| e.code).collect()
}

#[test]
fn numbers() {
    assert_eq!(toks("1_000_000"), ["INT_LIT(1_000_000)"]);
    assert_eq!(
        toks("0xFFu8 0o17 0b1010_0101u8 255u8"),
        [
            "INT_LIT(0xFFu8)",
            "INT_LIT(0o17)",
            "INT_LIT(0b1010_0101u8)",
            "INT_LIT(255u8)"
        ]
    );
    assert_eq!(
        toks("1.5 2.5e-3f64 1e3 1E+3 1f32 1f64"),
        [
            "FLOAT_LIT(1.5)",
            "FLOAT_LIT(2.5e-3f64)",
            "FLOAT_LIT(1e3)",
            "FLOAT_LIT(1E+3)",
            "FLOAT_LIT(1f32)",
            "FLOAT_LIT(1f64)"
        ]
    );
    assert_eq!(toks("0x1e3"), ["INT_LIT(0x1e3)"]); // hex, không phải float
    assert_eq!(
        toks("1_000i128 7u128 3usize 2isize"),
        [
            "INT_LIT(1_000i128)",
            "INT_LIT(7u128)",
            "INT_LIT(3usize)",
            "INT_LIT(2isize)"
        ]
    );
    // range không phải float
    assert_eq!(toks("1..2"), ["INT_LIT(1)", "DOTDOT", "INT_LIT(2)"]);
    assert_eq!(toks("1..=2"), ["INT_LIT(1)", "DOTDOTEQ", "INT_LIT(2)"]);
    assert_eq!(toks("1.foo"), ["INT_LIT(1)", "DOT", "IDENT(foo)"]);
    // chỉ số tuple
    assert_eq!(
        toks("t.0.1"),
        ["IDENT(t)", "DOT", "INT_LIT(0)", "DOT", "INT_LIT(1)"]
    );
    assert_eq!(toks("t.2"), ["IDENT(t)", "DOT", "INT_LIT(2)"]);
    // '-' là toán tử prefix riêng, không dính vào literal
    assert_eq!(toks("-128i8"), ["MINUS", "INT_LIT(128i8)"]);
    // lỗi: gạch dưới liền hậu tố / đầu / cuối, hậu tố lạ, chữ số sai cơ số, thiếu chữ số
    for bad in [
        "1000_u32", "1_", "0x", "0b102", "1.5i32", "1e3i32", "12abc", "0xFFz", "1__0",
    ] {
        assert_eq!(codes(bad).first(), Some(&ErrorCode::E0101), "{bad}");
    }
    // số ngay sau chữ: là phần của định danh, không phải số
    assert_eq!(toks("x1 _2"), ["IDENT(x1)", "IDENT(_2)"]);
}

#[test]
fn strings_and_chars() {
    assert_eq!(toks("\"hi\""), ["STR_LIT(hi)"]);
    assert_eq!(
        toks("\"tab\\there \\u{1F600} \\\" \\\\\""),
        ["STR_LIT(tab\\there \\u{1F600} \\\" \\\\)"]
    );
    assert_eq!(toks("\"ƒ ≔ tiếng Việt\""), ["STR_LIT(ƒ ≔ tiếng Việt)"]); // ký hiệu trong chuỗi không bị tách
    assert_eq!(
        toks("\"// không phải comment\""),
        ["STR_LIT(// không phải comment)"]
    );
    for bad in [
        "\"chưa đóng",
        "\"xuống\ndòng\"",
        "\"xuống\r\ndòng\"",
        "\"\\q\"",
        "\"\\u{110000}\"",
        "\"\\u{}\"",
        "\"\\u41\"",
        "\"\\u{D800}\"",
    ] {
        assert_eq!(codes(bad).first(), Some(&ErrorCode::E0101), "{bad:?}");
    }
    // lexer tiếp tục sau chuỗi hỏng
    assert!(render_tokens(&lex("\"a\nb")).contains(&"IDENT(b)".to_string()));
}

#[test]
fn comments() {
    assert_eq!(toks("// thường\nx"), ["IDENT(x)"]);
    assert_eq!(toks("//// bốn gạch\nx"), ["IDENT(x)"]);
    assert_eq!(toks("/// doc\nx"), ["DOC_COMMENT(doc)", "IDENT(x)"]);
    assert_eq!(
        toks("///   hai khoảng\nx"),
        ["DOC_COMMENT(  hai khoảng)", "IDENT(x)"]
    ); // chỉ bỏ MỘT khoảng trắng đầu
    assert_eq!(toks("///\nx"), ["DOC_COMMENT()", "IDENT(x)"]);
    assert_eq!(toks("x // cuối dòng"), ["IDENT(x)"]);
    assert_eq!(
        toks("/// doc crlf\r\nx"),
        ["DOC_COMMENT(doc crlf)", "IDENT(x)"]
    );
    // "/*" và "*/" luôn lỗi, kể cả dính liền (a /*p phải viết a / *p)
    assert_eq!(codes("/* c */"), [ErrorCode::E0101, ErrorCode::E0101]);
    assert_eq!(codes("a /*p"), [ErrorCode::E0101]);
    assert_eq!(toks("a / *p"), ["IDENT(a)", "SLASH", "STAR", "IDENT(p)"]);
    assert_eq!(codes("*/"), [ErrorCode::E0101]);
    // "/*" bên trong comment hoặc chuỗi chỉ là chữ
    assert_eq!(toks("// /* không sao */\nx"), ["IDENT(x)"]);
    assert_eq!(toks("\"/* */\""), ["STR_LIT(/* */)"]);
}

#[test]
fn operators_maximal_munch() {
    assert_eq!(
        toks("+ - * / %"),
        ["PLUS", "MINUS", "STAR", "SLASH", "PERCENT"]
    );
    assert_eq!(
        toks("+% -% *% +^ -^ *^ +? -? *?"),
        [
            "PLUS_WRAP",
            "MINUS_WRAP",
            "STAR_WRAP",
            "PLUS_SAT",
            "MINUS_SAT",
            "STAR_SAT",
            "PLUS_CHK",
            "MINUS_CHK",
            "STAR_CHK"
        ]
    );
    assert_eq!(
        toks("== != <= >= < > => -> <- := ::"),
        [
            "EQ_EQ",
            "NEQ",
            "LEQ",
            "GEQ",
            "LT",
            "GT",
            "FAT_ARROW",
            "ARROW",
            "ASSIGN",
            "WALRUS",
            "COLONCOLON"
        ]
    );
    assert_eq!(toks("& &! !"), ["BORROW", "BORROW_MUT", "BANG"]);
    assert_eq!(
        toks("! ! {div}"),
        ["BANG", "BANG", "LBRACE", "IDENT(div)", "RBRACE"]
    );
    assert_eq!(toks("a ** b"), ["IDENT(a)", "STAR", "STAR", "IDENT(b)"]); // semantics §14
    assert_eq!(toks("x<-1"), ["IDENT(x)", "ASSIGN", "INT_LIT(1)"]); // khớp dài nhất (semantics §24)
    assert_eq!(toks("x < -1"), ["IDENT(x)", "LT", "MINUS", "INT_LIT(1)"]);
    assert_eq!(toks("# . .. ..="), ["HASH", "DOT", "DOTDOT", "DOTDOTEQ"]);
    assert_eq!(toks("_ _x"), ["UNDERSCORE", "IDENT(_x)"]);
    // ký tự không thuộc ngôn ngữ
    for bad in ["@", "$", "?", "~", "^", "|", "`", "=", "⊤", "⊥", "😀"] {
        assert_eq!(codes(bad), [ErrorCode::E0101], "{bad}");
    }
}

#[test]
fn crlf_and_whitespace() {
    assert_eq!(
        toks("a\r\nb\tc  d\n"),
        ["IDENT(a)", "IDENT(b)", "IDENT(c)", "IDENT(d)"]
    );
    assert_eq!(codes("a\rb"), [ErrorCode::E1202]); // \r lẻ
    assert_eq!(codes("a\u{00A0}b"), [ErrorCode::E1202]); // NBSP
    assert_eq!(codes("a\u{2028}b"), [ErrorCode::E1202]);
    assert_eq!(codes("a\u{0007}b"), [ErrorCode::E1202]); // điều khiển
    assert_eq!(codes("a\u{0085}b"), [ErrorCode::E1202]); // C1
}

#[test]
fn dangerous_unicode() {
    // bidi ở mọi nơi: mã, comment, chuỗi
    assert_eq!(codes("a \u{202E} b"), [ErrorCode::E1201]);
    assert_eq!(codes("// \u{2066} x\ny"), [ErrorCode::E1201]);
    assert_eq!(codes("\"\u{202A}\""), [ErrorCode::E1201]);
    for c in [
        '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}', '\u{2066}', '\u{2067}',
        '\u{2068}', '\u{2069}',
    ] {
        assert_eq!(
            codes(&format!("x{c}y")),
            [ErrorCode::E1201],
            "U+{:04X}",
            c as u32
        );
    }
    // vô hình / zero-width, kể cả BOM đầu file
    for c in [
        '\u{200B}', '\u{200C}', '\u{200D}', '\u{2060}', '\u{FEFF}', '\u{00AD}',
    ] {
        assert_eq!(
            codes(&format!("ab{c}cd")),
            [ErrorCode::E1202],
            "U+{:04X}",
            c as u32
        );
    }
    assert_eq!(codes("\u{FEFF}x"), [ErrorCode::E1202]);
    assert_eq!(codes("\"a\u{200B}b\""), [ErrorCode::E1202]); // cả trong chuỗi
    // trộn bảng chữ trong một định danh
    assert_eq!(codes("x\u{0430}"), [ErrorCode::E1203]); // Latin + Cyrillic
    assert_eq!(codes("\u{03B1}x"), [ErrorCode::E1203]); // Greek + Latin
    assert_eq!(codes("'x\u{0430}"), [ErrorCode::E1203]); // cả tên lifetime
    // MICRO SIGN giả μ
    assert_eq!(codes("\u{00B5}s"), [ErrorCode::E1203,]);
    // một bảng chữ duy nhất thì hợp lệ (tiếng Việt, Cyrillic thuần, Hy Lạp thuần)
    assert_eq!(
        toks("tiếng_việt привет αβγ x1_é"),
        [
            "IDENT(tiếng_việt)",
            "IDENT(привет)",
            "IDENT(αβγ)",
            "IDENT(x1_é)"
        ]
    );
    // không phải NFC: "e" + U+0301
    assert_eq!(codes("cafe\u{0301}"), [ErrorCode::E1204]);
    assert!(lex("caf\u{00E9}").errors.is_empty());
    // nhiều lỗi được báo cùng lúc, theo thứ tự vị trí (D19)
    let out = lex("a\u{200B} b\u{202E} c\u{0430}x");
    let cs: Vec<_> = out.errors.iter().map(|e| e.code).collect();
    assert_eq!(cs, [ErrorCode::E1202, ErrorCode::E1201, ErrorCode::E1203]);
    assert!(
        out.errors
            .windows(2)
            .all(|w| w[0].span.start <= w[1].span.start)
    );
}

#[test]
fn spans_slice_back_to_source() {
    let src = "ƒ add(a: i32) → i32 ! ∅ { 'x' + \"s\" + 0xFFu8 /// d\n }";
    let out = lex(src);
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    let texts: Vec<&str> = out
        .tokens
        .iter()
        .map(|t| &src[t.span.start as usize..t.span.end as usize])
        .collect();
    assert_eq!(
        texts,
        [
            "ƒ", "add", "(", "a", ":", "i32", ")", "→", "i32", "!", "∅", "{", "'x'", "+", "\"s\"",
            "+", "0xFFu8", "/// d", "}", ""
        ]
    );
    assert_eq!(out.tokens.last().map(|t| &t.kind), Some(&TokenKind::Eof));
    let last = out.tokens.last().expect("Eof").span;
    assert_eq!(
        (last.start as usize, last.end as usize),
        (src.len(), src.len())
    );
}

/// Độ bền (KHÔNG thay thế fuzz thật): hàng chục nghìn chuỗi giả ngẫu nhiên, xác định (xorshift, seed cố định),
/// lấy từ bảng ký tự "khó chịu". Lexer không được panic, span phải hợp lệ/đơn điệu, luôn kết thúc bằng Eof.
#[test]
fn never_panics_and_spans_are_sane() {
    let alphabet: Vec<char> = "ƒ≔μ←→⟳↓⊳⊲⋈∧∨¬⇒∀∃≤≥≠∈∅⚠⊢⎇⇝⌥↦⏎▢⊔▦⇲◈⊏⊐⊠⊡≡□■ \t\n\r\\'\"/*.-+<>=!&%:;,#(){}[]_0123456789abcxeEuUfFiIsS\u{b5}\u{3bc}\u{430}\u{301}\u{200b}\u{202e}\u{a0}\u{7}\u{feff}é😀\u{10ffff}"
        .chars()
        .collect();
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..40_000 {
        let len = (next() % 41) as usize;
        let src: String = (0..len)
            .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
            .collect();
        let out = lex(&src);
        assert_eq!(
            out.tokens.last().map(|t| &t.kind),
            Some(&TokenKind::Eof),
            "{src:?}"
        );
        let mut prev_end = 0u32;
        for t in &out.tokens {
            assert!(
                t.span.start >= prev_end && t.span.start <= t.span.end,
                "{src:?}: span lệch {:?}",
                t.span
            );
            assert!(
                src.is_char_boundary(t.span.start as usize)
                    && src.is_char_boundary(t.span.end as usize),
                "{src:?}"
            );
            assert!(t.span.end as usize <= src.len(), "{src:?}");
            prev_end = t.span.end;
        }
        for e in &out.errors {
            assert!(
                e.span.end as usize <= src.len()
                    && src.is_char_boundary(e.span.start as usize)
                    && src.is_char_boundary(e.span.end as usize),
                "{src:?}"
            );
        }
    }
}
