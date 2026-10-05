//! Lỗi của lexer. Mã lỗi theo docs/spec/error_codes.md (D16).
use gt_span::Span;

/// Mã lỗi mà lexer có thể phát ra.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorCode {
    /// Token bất ngờ / thiếu dấu / alias sai / literal sai.
    E0101,
    /// Ký tự bidi (D15).
    E1201,
    /// Ký tự vô hình / zero-width / điều khiển / khoảng trắng lạ.
    E1202,
    /// Định danh trộn bảng chữ (homoglyph).
    E1203,
    /// Nguồn không phải NFC.
    E1204,
}

impl ErrorCode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::E0101 => "E0101",
            ErrorCode::E1201 => "E1201",
            ErrorCode::E1202 => "E1202",
            ErrorCode::E1203 => "E1203",
            ErrorCode::E1204 => "E1204",
        }
    }
}

/// Một lỗi lexer. Lexer luôn tiếp tục sau lỗi để báo nhiều lỗi (D19).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexError {
    pub code: ErrorCode,
    pub message: String,
    pub span: Span,
}
