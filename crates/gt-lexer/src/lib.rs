#![forbid(unsafe_code)]
//! gt-lexer (M1): lexer hai bề mặt, một ngữ nghĩa.
//!
//! Chấp nhận cả canonical Unicode (`ƒ ≔ μ ← →`…) lẫn alias ASCII (`\fn := \mut <- ->`…) và cho **cùng chuỗi token**.
//! Chặn ký tự nguy hiểm theo D15 (bidi E1201, vô hình E1202, homoglyph E1203, không NFC E1204).
//! Đặc tả: `docs/spec/symbols.md`, `docs/spec/semantics.md` §12–§15, §19, §24.
mod error;
mod lexer;
mod token;

pub use error::{ErrorCode, LexError};
pub use gt_span::Span;
pub use lexer::{LexOutput, lex, render_tokens};
pub use token::{Token, TokenKind, Trivia, TriviaKind};
