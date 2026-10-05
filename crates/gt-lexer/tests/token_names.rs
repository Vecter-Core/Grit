//! Khoá bộ tên token CHÍNH THỨC (Giang duyệt 2026-10-05, ADR-010) và hành vi token `UNKNOWN`.
use gt_lexer::{ErrorCode, lex, render_tokens};
use std::collections::BTreeSet;

/// Danh sách tên Giang duyệt.
const OFFICIAL: &[&str] = &[
    // từ khoá
    "KW_MUT",
    "KW_IN",
    "KW_WHERE",
    "KW_EXTERN",
    "KW_IF",
    "KW_ELSE",
    "KW_MATCH",
    "KW_LOOP",
    "KW_BREAK",
    "KW_CONT",
    "KW_RET",
    "KW_FN",
    "KW_STRUCT",
    "KW_ENUM",
    "KW_TRAIT",
    "KW_IMPL",
    "KW_MOD",
    "KW_USE",
    "KW_PUB",
    "KW_TYPE",
    "KW_CONST",
    "KW_STATIC",
    "KW_UNSAFE",
    "KW_PROOF",
    "KW_SELF_KW",
    "KW_SELF_TY",
    "KW_AS",
    "KW_FOR",
    "KW_TRUE",
    "KW_FALSE",
    // ký hiệu
    "ARROW",
    "FAT_ARROW",
    "WALRUS",
    "ASSIGN",
    "LOOP_SYM",
    "DEC",
    "PRE",
    "POST",
    "INV",
    "PROOF_SYM",
    "EMPTY_SET",
    "IMPLIES",
    "IN_SET",
    "FORALL",
    "EXISTS",
    "LEQ",
    "GEQ",
    "NEQ",
    "LAND",
    "LOR",
    "LNOT",
    "BORROW",
    "BORROW_MUT",
    // dấu câu
    "SEMI",
    "COLON",
    "COMMA",
    "DOT",
    "DOTDOT",
    "DOTDOTEQ",
    "LBRACE",
    "RBRACE",
    "LPAREN",
    "RPAREN",
    "LBRACKET",
    "RBRACKET",
    "COLONCOLON",
    "HASH",
    "STAR",
    "SLASH",
    "PERCENT",
    "PLUS",
    "MINUS",
    "LT",
    "GT",
    "EQ_EQ",
    // đặc biệt
    "LIFETIME",
    "DOC_COMMENT",
    "INT_LIT",
    "FLOAT_LIT",
    "STR_LIT",
    "CHAR_LIT",
    "IDENT",
    "EOF",
    "UNKNOWN",
];

/// Tên không có trong danh sách của Giang nhưng lexer cần (agent đặt theo cùng quy ước; ADR-010 xin xác nhận).
const EXTRA: &[&str] = &[
    "BANG",
    "UNDERSCORE",
    "PLUS_WRAP",
    "MINUS_WRAP",
    "STAR_WRAP",
    "PLUS_SAT",
    "MINUS_SAT",
    "STAR_SAT",
    "PLUS_CHK",
    "MINUS_CHK",
    "STAR_CHK",
];

/// Tên trong danh sách nhưng cố ý KHÔNG dùng: `⟳` và `⊢` đã có `LOOP_SYM` / `PROOF_SYM` (alias và canonical phải ra CÙNG một token).
const UNUSED_BY_DESIGN: &[&str] = &["KW_LOOP", "KW_PROOF"];

fn name_of(rendered: &str) -> &str {
    rendered.split('(').next().unwrap_or(rendered)
}

/// Một chương trình chứa mọi loại token.
const EVERYTHING: &str = "\\fn \\mut \\if \\else \\match \\ret \\struct \\enum \\mod \\use \\pub \\trait \\impl \\break \\cont \\type \\const \\static \\unsafe \\extern \\where \
    self Self as for in true false _ \
    := <- -> => \\loop \\dec \\pre \\post \\inv \\proof \\nil \\implies \\in \\forall \\exists <= >= != \\land \\lor \\neg & &! == < > \
    + - * / % +% -% *% +^ -^ *^ +? -? *? ; : , . .. ..= { } ( ) [ ] :: # ! \
    ident 'life 12 1.5 \"s\" 'c' /// doc\n";

#[test]
fn every_emitted_name_is_official() {
    let out = lex(EVERYTHING);
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    let allowed: BTreeSet<&str> = OFFICIAL.iter().chain(EXTRA).copied().collect();
    let got: BTreeSet<String> = render_tokens(&out)
        .iter()
        .map(|t| name_of(t).to_string())
        .collect();
    for n in &got {
        assert!(
            allowed.contains(n.as_str()),
            "tên token không chính thức: {n}"
        );
    }
    // mọi tên chính thức (trừ hai tên cố ý không dùng, EOF, UNKNOWN) đều được lexer phát ra
    for n in OFFICIAL.iter().chain(EXTRA) {
        if UNUSED_BY_DESIGN.contains(n) || ["EOF", "UNKNOWN"].contains(n) {
            continue;
        }
        assert!(
            got.contains(*n),
            "lexer không bao giờ phát tên chính thức {n}"
        );
    }
}

#[test]
fn old_provisional_names_are_gone() {
    let old = [
        "KEYWORD_IN",
        "KEYWORD_WHERE",
        "KEYWORD_EXTERN",
        "KEYWORD_SELF",
        "KEYWORD_SELF_TYPE",
        "KEYWORD_AS",
        "KEYWORD_FOR",
        "KEYWORD_TRUE",
        "KEYWORD_FALSE",
        "MUT",
        "FN",
        "LOOP",
        "ASSIGN_LET",
        "MATCH_ARM",
        "EMPTYSET",
        "TYPE_ALIAS",
        "EQEQ",
        "AMP",
        "AMP_BANG",
        "NE",
        "LE",
        "GE",
        "AND",
        "OR",
        "NOT",
        "INT",
        "FLOAT",
        "STR",
        "CHAR",
        "IF",
        "ELSE",
        "MATCH",
        "RETURN",
        "BREAK",
        "CONTINUE",
        "STRUCT",
        "ENUM",
        "MOD",
        "USE",
        "PUB",
        "TRAIT",
        "IMPL",
        "CONST",
        "STATIC",
        "UNSAFE",
        "PROOF",
    ];
    let out = lex(EVERYTHING);
    for t in render_tokens(&out) {
        assert!(!old.contains(&name_of(&t)), "còn tên tạm cũ: {t}");
    }
}

fn all(src: &str) -> (Vec<String>, Vec<ErrorCode>) {
    let out = lex(src);
    (
        render_tokens(&out),
        out.errors.iter().map(|e| e.code).collect(),
    )
}

#[test]
fn unknown_token_for_invalid_input() {
    // ký tự không thuộc ngôn ngữ -> UNKNOWN + E0101 (parser có điểm neo để khôi phục lỗi)
    assert_eq!(
        all("@"),
        (vec!["UNKNOWN".to_string()], vec![ErrorCode::E0101])
    );
    assert_eq!(all("a ⊤ b").0, ["IDENT(a)", "UNKNOWN", "IDENT(b)"]);
    // '=' đứng riêng
    assert_eq!(all("a = b").0, ["IDENT(a)", "UNKNOWN", "IDENT(b)"]);
    // alias không tồn tại
    assert_eq!(all("\\foo x").0, ["UNKNOWN", "IDENT(x)"]);
    // comment khối
    assert_eq!(all("/* c */").0, ["UNKNOWN", "IDENT(c)", "UNKNOWN"]);
    // literal số hỏng
    assert_eq!(all("1000_u32;").0, ["UNKNOWN", "SEMI"]);
    // chuỗi / char hỏng
    assert_eq!(all("\"abc").0, ["UNKNOWN"]);
    assert_eq!(all("'ab'").0, ["LIFETIME(ab)", "UNKNOWN"]);
    assert_eq!(all("''").0, ["UNKNOWN"]);
    // µ giả μ
    assert_eq!(
        all("\u{b5}s"),
        (
            vec!["UNKNOWN".to_string(), "IDENT(s)".to_string()],
            vec![ErrorCode::E1203]
        )
    );
    // mỗi lỗi có đúng một UNKNOWN tại chỗ đó; chuỗi hợp lệ không bao giờ có UNKNOWN
    assert!(
        !render_tokens(&lex("ƒ f() → i32 ! ∅ { 1 }"))
            .iter()
            .any(|t| t == "UNKNOWN")
    );
}
