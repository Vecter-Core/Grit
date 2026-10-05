//! Loại token của Grit. **Tên token chính thức** (Giang duyệt 2026-10-05, ADR-010): tiền tố `KW_` cho từ khoá,
//! tên ký hiệu riêng cho ký hiệu toán học, `*_LIT` cho literal, `UNKNOWN` cho ký tự/đoạn không hợp lệ.
use gt_span::Span;

/// Một token kèm vị trí.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Loại trivia (văn bản không thuộc cú pháp nhưng PHẢI được giữ để gt-fmt (thư viện) không xoá).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriviaKind {
    /// `// …` (kể cả `////…`). Doc comment `///` KHÔNG phải trivia: nó là token `DOC_COMMENT`.
    LineComment,
}

/// Một mẩu trivia. `text` là văn bản nguồn NGUYÊN VĂN (gồm `//`, không gồm CR/LF cuối dòng).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub text: String,
    pub span: Span,
    /// `true` nếu trước comment trên dòng đó chỉ có khoảng trắng (comment đứng một mình trên dòng);
    /// `false` nếu đứng sau mã (comment cuối dòng). Dùng để `gt-fmt` gắn lại đúng chỗ.
    pub own_line: bool,
}

/// Loại token. Canonical Unicode và alias ASCII cho **cùng** loại (hai bề mặt, một ngữ nghĩa).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    // dấu câu
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Colon,
    ColonColon,
    Dot,
    DotDot,
    DotDotEq,
    Hash,
    // mũi tên / gán
    Arrow,
    Walrus,
    Assign,
    Bang,
    FatArrow,
    // số học
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusWrap,
    MinusWrap,
    StarWrap,
    PlusSat,
    MinusSat,
    StarSat,
    PlusChk,
    MinusChk,
    StarChk,
    // so sánh, mượn
    EqEq,
    Neq,
    Lt,
    Gt,
    Leq,
    Geq,
    Borrow,
    BorrowMut,
    // logic
    Land,
    Lor,
    Lnot,
    Implies,
    ForAll,
    Exists,
    InSet,
    // ký hiệu riêng
    LoopSym,
    Dec,
    Pre,
    Post,
    Inv,
    ProofSym,
    EmptySet,
    // từ khoá có ký hiệu canonical
    KwFn,
    KwMut,
    KwIf,
    KwElse,
    KwMatch,
    KwRet,
    KwStruct,
    KwEnum,
    KwMod,
    KwUse,
    KwPub,
    KwTrait,
    KwImpl,
    KwBreak,
    KwCont,
    KwType,
    KwConst,
    KwStatic,
    KwUnsafe,
    // dành riêng (ASCII-only) và từ khoá cứng ASCII
    KwExtern,
    KwWhere,
    KwSelfKw,
    KwSelfTy,
    KwAs,
    KwFor,
    KwIn,
    KwTrue,
    KwFalse,
    Underscore,
    // có giá trị (văn bản nguồn nguyên văn, trừ DocComment đã bỏ `///` và một khoảng trắng đầu)
    Ident(String),
    IntLit(String),
    FloatLit(String),
    StrLit(String),
    CharLit(String),
    Lifetime(String),
    DocComment(String),
    /// Ký tự hoặc đoạn nguồn không hợp lệ (luôn đi kèm một `LexError`); văn bản lấy lại từ span.
    Unknown,
    Eof,
}

impl TokenKind {
    /// Tên token chính thức, ổn định, dùng trong `tests/lexer/*.tokens`.
    #[must_use]
    pub fn name(&self) -> &'static str {
        use TokenKind::*;
        match self {
            LParen => "LPAREN",
            RParen => "RPAREN",
            LBrace => "LBRACE",
            RBrace => "RBRACE",
            LBracket => "LBRACKET",
            RBracket => "RBRACKET",
            Comma => "COMMA",
            Semi => "SEMI",
            Colon => "COLON",
            ColonColon => "COLONCOLON",
            Dot => "DOT",
            DotDot => "DOTDOT",
            DotDotEq => "DOTDOTEQ",
            Hash => "HASH",
            Arrow => "ARROW",
            Walrus => "WALRUS",
            Assign => "ASSIGN",
            Bang => "BANG",
            FatArrow => "FAT_ARROW",
            Plus => "PLUS",
            Minus => "MINUS",
            Star => "STAR",
            Slash => "SLASH",
            Percent => "PERCENT",
            PlusWrap => "PLUS_WRAP",
            MinusWrap => "MINUS_WRAP",
            StarWrap => "STAR_WRAP",
            PlusSat => "PLUS_SAT",
            MinusSat => "MINUS_SAT",
            StarSat => "STAR_SAT",
            PlusChk => "PLUS_CHK",
            MinusChk => "MINUS_CHK",
            StarChk => "STAR_CHK",
            EqEq => "EQ_EQ",
            Neq => "NEQ",
            Lt => "LT",
            Gt => "GT",
            Leq => "LEQ",
            Geq => "GEQ",
            Borrow => "BORROW",
            BorrowMut => "BORROW_MUT",
            Land => "LAND",
            Lor => "LOR",
            Lnot => "LNOT",
            Implies => "IMPLIES",
            ForAll => "FORALL",
            Exists => "EXISTS",
            InSet => "IN_SET",
            LoopSym => "LOOP_SYM",
            Dec => "DEC",
            Pre => "PRE",
            Post => "POST",
            Inv => "INV",
            ProofSym => "PROOF_SYM",
            EmptySet => "EMPTY_SET",
            KwFn => "KW_FN",
            KwMut => "KW_MUT",
            KwIf => "KW_IF",
            KwElse => "KW_ELSE",
            KwMatch => "KW_MATCH",
            KwRet => "KW_RET",
            KwStruct => "KW_STRUCT",
            KwEnum => "KW_ENUM",
            KwMod => "KW_MOD",
            KwUse => "KW_USE",
            KwPub => "KW_PUB",
            KwTrait => "KW_TRAIT",
            KwImpl => "KW_IMPL",
            KwBreak => "KW_BREAK",
            KwCont => "KW_CONT",
            KwType => "KW_TYPE",
            KwConst => "KW_CONST",
            KwStatic => "KW_STATIC",
            KwUnsafe => "KW_UNSAFE",
            KwExtern => "KW_EXTERN",
            KwWhere => "KW_WHERE",
            KwSelfKw => "KW_SELF_KW",
            KwSelfTy => "KW_SELF_TY",
            KwAs => "KW_AS",
            KwFor => "KW_FOR",
            KwIn => "KW_IN",
            KwTrue => "KW_TRUE",
            KwFalse => "KW_FALSE",
            Underscore => "UNDERSCORE",
            Ident(_) => "IDENT",
            IntLit(_) => "INT_LIT",
            FloatLit(_) => "FLOAT_LIT",
            StrLit(_) => "STR_LIT",
            CharLit(_) => "CHAR_LIT",
            Lifetime(_) => "LIFETIME",
            DocComment(_) => "DOC_COMMENT",
            Unknown => "UNKNOWN",
            Eof => "EOF",
        }
    }

    /// Dạng văn bản trong file `.tokens`: `NAME` hoặc `NAME(giá trị)`.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            TokenKind::Ident(v)
            | TokenKind::IntLit(v)
            | TokenKind::FloatLit(v)
            | TokenKind::StrLit(v)
            | TokenKind::CharLit(v)
            | TokenKind::Lifetime(v)
            | TokenKind::DocComment(v) => format!("{}({v})", self.name()),
            other => other.name().to_string(),
        }
    }
}

/// Bảng ký hiệu canonical ↔ alias ASCII (docs/spec/symbols.md). Nguồn duy nhất cho lexer.
/// Cột `ascii`: bí danh dạng `\tên` hoặc toán tử ASCII tương đương.
///
/// Danh sách tên Giang duyệt có thêm `KW_LOOP` và `KW_PROOF` nhưng `⟳`/`⊢` đã được gán `LOOP_SYM`/`PROOF_SYM`;
/// alias và canonical phải cho CÙNG một token nên chỉ dùng một tên cho mỗi khái niệm (ADR-010).
pub(crate) const SYMBOLS: &[(char, &str, TokenKind)] = &[
    ('ƒ', "\\fn", TokenKind::KwFn),
    ('≔', ":=", TokenKind::Walrus),
    ('μ', "\\mut", TokenKind::KwMut),
    ('←', "<-", TokenKind::Assign),
    ('→', "->", TokenKind::Arrow),
    ('⟳', "\\loop", TokenKind::LoopSym),
    ('↓', "\\dec", TokenKind::Dec),
    ('⊳', "\\pre", TokenKind::Pre),
    ('⊲', "\\post", TokenKind::Post),
    ('⋈', "\\inv", TokenKind::Inv),
    ('⚠', "\\unsafe", TokenKind::KwUnsafe),
    ('⊢', "\\proof", TokenKind::ProofSym),
    ('∅', "\\nil", TokenKind::EmptySet),
    ('⎇', "\\if", TokenKind::KwIf),
    ('⇝', "\\else", TokenKind::KwElse),
    ('⌥', "\\match", TokenKind::KwMatch),
    ('↦', "=>", TokenKind::FatArrow),
    ('⏎', "\\ret", TokenKind::KwRet),
    ('▢', "\\struct", TokenKind::KwStruct),
    ('⊔', "\\enum", TokenKind::KwEnum),
    ('▦', "\\mod", TokenKind::KwMod),
    ('⇲', "\\use", TokenKind::KwUse),
    ('◈', "\\pub", TokenKind::KwPub),
    ('⊏', "\\trait", TokenKind::KwTrait),
    ('⊐', "\\impl", TokenKind::KwImpl),
    ('⊠', "\\break", TokenKind::KwBreak),
    ('⊡', "\\cont", TokenKind::KwCont),
    ('≡', "\\type", TokenKind::KwType),
    ('□', "\\const", TokenKind::KwConst),
    ('■', "\\static", TokenKind::KwStatic),
    ('∧', "\\land", TokenKind::Land),
    ('∨', "\\lor", TokenKind::Lor),
    ('¬', "\\neg", TokenKind::Lnot),
    ('⇒', "\\implies", TokenKind::Implies),
    ('∀', "\\forall", TokenKind::ForAll),
    ('∃', "\\exists", TokenKind::Exists),
    ('≤', "<=", TokenKind::Leq),
    ('≥', ">=", TokenKind::Geq),
    ('≠', "!=", TokenKind::Neq),
    ('∈', "\\in", TokenKind::InSet),
];
