//! Adapter `LexError → Diagnostic` (ADR-011: đặt ở gt-parser) và mô tả token cho thông điệp lỗi.
use gt_diag::{Code, Diagnostic};
use gt_lexer::{LexError, TokenKind};

/// Chuyển lỗi lexer thành `Diagnostic` (mã E0101/E12xx đã nằm trong bảng mã).
pub fn lex_error(file: &str, e: &LexError) -> Option<Diagnostic> {
    let code = Code::parse(e.code.as_str()).ok()?;
    Some(Diagnostic::new(code, file, e.span, e.message.clone()))
}

/// Mô tả token cho thông điệp "mong đợi …, gặp …".
pub fn describe(k: &TokenKind) -> String {
    let sym = match k {
        TokenKind::Eof => return "hết tệp".to_string(),
        TokenKind::Unknown => return "ký tự không hợp lệ".to_string(),
        TokenKind::Ident(n) => return format!("`{n}`"),
        TokenKind::IntLit(t) | TokenKind::FloatLit(t) => return format!("số `{t}`"),
        TokenKind::StrLit(_) => return "chuỗi".to_string(),
        TokenKind::CharLit(_) => return "ký tự".to_string(),
        TokenKind::Lifetime(n) => return format!("lifetime `'{n}`"),
        TokenKind::DocComment(_) => return "doc comment".to_string(),
        TokenKind::LParen => "(",
        TokenKind::RParen => ")",
        TokenKind::LBrace => "{",
        TokenKind::RBrace => "}",
        TokenKind::LBracket => "[",
        TokenKind::RBracket => "]",
        TokenKind::Comma => ",",
        TokenKind::Semi => ";",
        TokenKind::Colon => ":",
        TokenKind::ColonColon => "::",
        TokenKind::Dot => ".",
        TokenKind::DotDot => "..",
        TokenKind::DotDotEq => "..=",
        TokenKind::Walrus => "≔",
        TokenKind::Assign => "←",
        TokenKind::Arrow => "→",
        TokenKind::FatArrow => "↦",
        TokenKind::Bang => "!",
        TokenKind::KwFn => "ƒ",
        TokenKind::KwMut => "μ",
        TokenKind::KwExtern => return "`\\extern` (từ khoá dành riêng cho Phase 2)".to_string(),
        TokenKind::KwWhere => return "`\\where` (từ khoá dành riêng cho Phase 2)".to_string(),
        TokenKind::KwSelfKw => return "`self` (từ khoá cứng)".to_string(),
        TokenKind::KwSelfTy => return "`Self` (từ khoá cứng)".to_string(),
        TokenKind::KwIn => return "`in` (từ khoá cứng)".to_string(),
        TokenKind::KwAs => return "`as` (từ khoá cứng)".to_string(),
        TokenKind::KwFor => return "`for` (từ khoá cứng)".to_string(),
        other => return format!("token {}", other.name()),
    };
    format!("`{sym}`")
}
