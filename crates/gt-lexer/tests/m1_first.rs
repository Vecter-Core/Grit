//! Bốn test đầu tiên của M1 (viết TRƯỚC code lexer — TDD):
//! round_trip_alias_canonical, mu_split, lifetime_vs_char, for_in_tokens.
//! Bảng alias trong test này là "oracle" độc lập, dựng từ docs/spec/symbols.md (không import từ lexer).
use gt_lexer::{ErrorCode, lex, render_tokens};

fn toks(src: &str) -> Vec<String> {
    let out = lex(src);
    assert!(
        out.errors.is_empty(),
        "không mong đợi lỗi lexer cho {src:?}: {:?}",
        out.errors
    );
    render_tokens(&out)
}

fn codes(src: &str) -> Vec<ErrorCode> {
    lex(src).errors.iter().map(|e| e.code).collect()
}

/// (canonical, ascii alias, tên token) — symbols.md. Backslash viết đôi vì là chuỗi Rust thường.
const PAIRS: &[(&str, &str, &str)] = &[
    ("ƒ", "\\fn", "KW_FN"),
    ("≔", ":=", "WALRUS"),
    ("μ", "\\mut", "KW_MUT"),
    ("←", "<-", "ASSIGN"),
    ("→", "->", "ARROW"),
    ("⟳", "\\loop", "LOOP_SYM"),
    ("↓", "\\dec", "DEC"),
    ("⊳", "\\pre", "PRE"),
    ("⊲", "\\post", "POST"),
    ("⋈", "\\inv", "INV"),
    ("⚠", "\\unsafe", "KW_UNSAFE"),
    ("⊢", "\\proof", "PROOF_SYM"),
    ("∅", "\\nil", "EMPTY_SET"),
    ("⎇", "\\if", "KW_IF"),
    ("⇝", "\\else", "KW_ELSE"),
    ("⌥", "\\match", "KW_MATCH"),
    ("↦", "=>", "FAT_ARROW"),
    ("⏎", "\\ret", "KW_RET"),
    ("▢", "\\struct", "KW_STRUCT"),
    ("⊔", "\\enum", "KW_ENUM"),
    ("▦", "\\mod", "KW_MOD"),
    ("⇲", "\\use", "KW_USE"),
    ("◈", "\\pub", "KW_PUB"),
    ("⊏", "\\trait", "KW_TRAIT"),
    ("⊐", "\\impl", "KW_IMPL"),
    ("⊠", "\\break", "KW_BREAK"),
    ("⊡", "\\cont", "KW_CONT"),
    ("≡", "\\type", "KW_TYPE"),
    ("□", "\\const", "KW_CONST"),
    ("■", "\\static", "KW_STATIC"),
    ("∧", "\\land", "LAND"),
    ("∨", "\\lor", "LOR"),
    ("¬", "\\neg", "LNOT"),
    ("⇒", "\\implies", "IMPLIES"),
    ("∀", "\\forall", "FORALL"),
    ("∃", "\\exists", "EXISTS"),
    ("≤", "<=", "LEQ"),
    ("≥", ">=", "GEQ"),
    ("≠", "!=", "NEQ"),
    ("∈", "\\in", "IN_SET"),
];

#[test]
fn round_trip_alias_canonical() {
    for (canon, ascii, name) in PAIRS {
        assert_eq!(toks(canon), vec![name.to_string()], "canonical {canon:?}");
        assert_eq!(toks(ascii), vec![name.to_string()], "ascii {ascii:?}");
    }
    // alias chỉ-ASCII, không có canonical
    assert_eq!(toks("\\extern"), ["KW_EXTERN"]);
    assert_eq!(toks("\\where"), ["KW_WHERE"]);

    // cả chương trình: hai bề mặt phải cho cùng chuỗi token (khác nhau duy nhất là span)
    let canonical = "ƒ add(a: i32, b: i32) → i32 ! ∅ ⊳ a ≥ 0 ∧ b ≥ 0 { a + b }";
    let ascii = "\\fn add(a: i32, b: i32) -> i32 ! \\nil \\pre a >= 0 \\land b >= 0 { a + b }";
    assert_eq!(toks(canonical), toks(ascii));

    // file mẫu thật trong corpus: 01_add.gt (canonical) và 11_ascii_alias.gt (ASCII)
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/pass/");
    let a = std::fs::read_to_string(format!("{root}01_add.gt")).expect("01_add.gt");
    let b = std::fs::read_to_string(format!("{root}11_ascii_alias.gt")).expect("11_ascii_alias.gt");
    assert_eq!(toks(&a), toks(&b));

    // alias sai: không có trong bảng, hoặc dính chữ phía sau -> E0101
    assert_eq!(codes("\\foo"), [ErrorCode::E0101]);
    assert_eq!(codes("\\Fn"), [ErrorCode::E0101]);
    assert_eq!(codes("\\fnx"), [ErrorCode::E0101]);
    assert_eq!(codes("\\"), [ErrorCode::E0101]);
}

#[test]
fn mu_split() {
    // Q5: "μs" phải là hai token MUT và IDENT(s), không phải IDENT("μs")
    assert_eq!(toks("μs"), ["KW_MUT", "IDENT(s)"]);
    assert_eq!(toks("μ s"), ["KW_MUT", "IDENT(s)"]);
    assert_eq!(toks("\\mut s"), ["KW_MUT", "IDENT(s)"]);
    assert_eq!(toks("μμx"), ["KW_MUT", "KW_MUT", "IDENT(x)"]);
    // μ giữa chữ cũng bị tách (hệ quả đã duyệt của Q5): parser sẽ từ chối ở M2
    assert_eq!(toks("aμb"), ["IDENT(a)", "KW_MUT", "IDENT(b)"]);
    assert_eq!(toks("xμ"), ["IDENT(x)", "KW_MUT"]);
    // chuỗi token đầy đủ như tests/lexer/mu_split.tokens
    assert_eq!(
        toks("ƒ f() → i32 ! ∅ { μs ≔ 1; μ t ≔ 2; aμb }"),
        [
            "KW_FN",
            "IDENT(f)",
            "LPAREN",
            "RPAREN",
            "ARROW",
            "IDENT(i32)",
            "BANG",
            "EMPTY_SET",
            "LBRACE",
            "KW_MUT",
            "IDENT(s)",
            "WALRUS",
            "INT_LIT(1)",
            "SEMI",
            "KW_MUT",
            "IDENT(t)",
            "WALRUS",
            "INT_LIT(2)",
            "SEMI",
            "IDENT(a)",
            "KW_MUT",
            "IDENT(b)",
            "RBRACE"
        ]
    );
    // bất biến: không bao giờ có IDENT chứa 'μ' (ƒ cũng là ký hiệu dành riêng nên cũng không nằm trong IDENT)
    for src in ["μs", "aμb", "μμμ", "abcμdef", "xƒy", "μ_x", "x_μ", "αμβ"] {
        let out = lex(src);
        for t in render_tokens(&out) {
            assert!(
                !(t.starts_with("IDENT(") && (t.contains('μ') || t.contains('ƒ'))),
                "{src:?} -> {t}"
            );
        }
    }
}

#[test]
fn lifetime_vs_char() {
    // quy tắc Giang: 'x' (có dấu đóng) = char; 'a (không dấu đóng) = lifetime
    assert_eq!(toks("'a'"), ["CHAR_LIT(a)"]);
    assert_eq!(toks("'\\n'"), ["CHAR_LIT(\\n)"]);
    assert_eq!(toks("'\\''"), ["CHAR_LIT(\\')"]);
    assert_eq!(toks("'\\u{41}'"), ["CHAR_LIT(\\u{41})"]);
    assert_eq!(toks("'é'"), ["CHAR_LIT(é)"]);
    assert_eq!(toks("'a"), ["LIFETIME(a)"]);
    assert_eq!(toks("'abc"), ["LIFETIME(abc)"]);
    assert_eq!(
        toks("<'a, 'b>"),
        ["LT", "LIFETIME(a)", "COMMA", "LIFETIME(b)", "GT"]
    );
    assert_eq!(toks("&'a i32"), ["BORROW", "LIFETIME(a)", "IDENT(i32)"]);
    // như tests/lexer/lifetime_vs_char.tokens
    assert_eq!(
        toks("f<'a, 'b>('x', '\\n', '\\'', 'é') 'abc"),
        [
            "IDENT(f)",
            "LT",
            "LIFETIME(a)",
            "COMMA",
            "LIFETIME(b)",
            "GT",
            "LPAREN",
            "CHAR_LIT(x)",
            "COMMA",
            "CHAR_LIT(\\n)",
            "COMMA",
            "CHAR_LIT(\\')",
            "COMMA",
            "CHAR_LIT(é)",
            "RPAREN",
            "LIFETIME(abc)"
        ]
    );
    // char nhiều ký tự / rỗng / lẻ -> E0101
    assert_eq!(codes("'ab'"), [ErrorCode::E0101]);
    assert_eq!(codes("''"), [ErrorCode::E0101]);
    assert_eq!(codes("'"), [ErrorCode::E0101]);
    assert_eq!(codes("'\\q'"), [ErrorCode::E0101]); // escape lạ
    // lexer vẫn phải tiếp tục sau lỗi và không panic
    let out = lex("'ab' x");
    assert!(render_tokens(&out).contains(&"IDENT(x)".to_string()));
}

#[test]
fn for_in_tokens() {
    // như tests/lexer/for_in_tokens.tokens
    assert_eq!(
        toks("⟳ x in xs ↓ n - 1 { index }"),
        [
            "LOOP_SYM",
            "IDENT(x)",
            "KW_IN",
            "IDENT(xs)",
            "DEC",
            "IDENT(n)",
            "MINUS",
            "INT_LIT(1)",
            "LBRACE",
            "IDENT(index)",
            "RBRACE"
        ]
    );
    assert_eq!(
        toks("\\loop x in xs {}"),
        [
            "LOOP_SYM",
            "IDENT(x)",
            "KW_IN",
            "IDENT(xs)",
            "LBRACE",
            "RBRACE"
        ]
    );
    // "in" chỉ khớp nguyên từ
    for ident in ["index", "inx", "in_", "_in", "ins", "xin"] {
        assert_eq!(toks(ident), [format!("IDENT({ident})")], "{ident}");
    }
    // in trần (từ khoá) khác ∈ / \in (toán tử thuộc)
    assert_eq!(toks("in"), ["KW_IN"]);
    assert_eq!(toks("∈"), ["IN_SET"]);
    assert_eq!(toks("x \\in s"), ["IDENT(x)", "IN_SET", "IDENT(s)"]);
    // các từ khoá cứng khác
    assert_eq!(
        toks("self Self as for true false in"),
        [
            "KW_SELF_KW",
            "KW_SELF_TY",
            "KW_AS",
            "KW_FOR",
            "KW_TRUE",
            "KW_FALSE",
            "KW_IN"
        ]
    );
    assert_eq!(toks("Selfish selfie"), ["IDENT(Selfish)", "IDENT(selfie)"]);
}
