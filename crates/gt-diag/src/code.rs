//! Mã chẩn đoán (D16) và mức (ĐỎ/VÀNG/GHI CHÚ).
use std::fmt;

/// Mức chẩn đoán. Thứ tự: nặng hơn = lớn hơn (`Error > Warn > Note`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// GHI CHÚ (trắng) — không ảnh hưởng chạy.
    Note,
    /// VÀNG — vẫn cho chạy (chỉ lộ secret, plan §5).
    Warn,
    /// ĐỎ — không sinh binary, không chạy.
    Error,
}

impl Level {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warn => "warn",
            Level::Note => "note",
        }
    }
}

/// Mã không đúng định dạng D16.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeError(pub String);

impl fmt::Display for CodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "mã chẩn đoán không hợp lệ: {:?} (cần E+4 số, S+3 số hoặc N+4 số)",
            self.0
        )
    }
}

/// Mã chẩn đoán đã kiểm định dạng: `E`+4 số (ĐỎ), `S`+3 số (VÀNG), `N`+4 số (GHI CHÚ).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code(String);

impl Code {
    /// Kiểm định dạng. Không kiểm "đã biết" — xem [`Code::is_known`].
    pub fn parse(s: &str) -> Result<Code, CodeError> {
        let b = s.as_bytes();
        let digits = |r: &[u8]| r.iter().all(u8::is_ascii_digit);
        let ok = match b.first() {
            Some(b'E') | Some(b'N') => b.len() == 5 && digits(&b[1..]),
            Some(b'S') => b.len() == 4 && digits(&b[1..]),
            _ => false,
        };
        if ok {
            Ok(Code(s.to_string()))
        } else {
            Err(CodeError(s.to_string()))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Mức suy ra từ tiền tố (D16) — không thể mâu thuẫn với mã.
    #[must_use]
    pub fn level(&self) -> Level {
        match self.0.as_bytes()[0] {
            b'E' => Level::Error,
            b'S' => Level::Warn,
            _ => Level::Note,
        }
    }

    /// Lỗi nội bộ compiler (E99xx) — exit 2 (D17).
    #[must_use]
    pub fn is_ice(&self) -> bool {
        self.0.starts_with("E99")
    }

    /// Mã có trong `docs/spec/error_codes.md`.
    #[must_use]
    pub fn is_known(&self) -> bool {
        KNOWN.contains(&self.0.as_str())
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Bộ mã đã biết; test `known_codes_match_docs` giữ cho khớp `docs/spec/error_codes.md`.
const KNOWN: &[&str] = &[
    "E0101", "E0102", "E0201", "E0202", "E0203", "E0204", "E0205", "E0301", "E0302", "E0303",
    "E0304", "E0305", "E0307", "E0308", "E0309", "E0310", "E0311", "E0312", "E0313", "E0314",
    "E0315", "E0316", "E0401", "E0402", "E0403", "E0404", "E0405", "E0406", "E0407", "E0501",
    "E0502", "E0503", "E0601", "E0602", "E0603", "E0604", "E0701", "E0702", "E0703", "E0801",
    "E0802", "E0901", "E0902", "E1001", "E1002", "E1101", "E1201", "E1202", "E1203", "E1204",
    "E1301", "E1401", "E9901", "N0601", "N0602", "N0603", "N0604", "N0605", "N0901", "N0902",
    "N0903", "N0904", "N1401", "S101", "S102", "S201", "S202",
];

/// Danh sách mã đã biết.
#[must_use]
pub fn known_codes() -> Vec<Code> {
    KNOWN.iter().filter_map(|s| Code::parse(s).ok()).collect()
}
