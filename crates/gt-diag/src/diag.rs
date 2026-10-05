//! Kiểu `Diagnostic`, nhãn, gợi ý sửa (edit có cấu trúc).
use crate::code::{Code, Level};
use gt_span::Span;

/// Cách áp dụng một gợi ý. Fail-closed: chỉ `MachineApplicable` được phép áp dụng tự động.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applicability {
    MachineApplicable,
    MaybeIncorrect,
}

impl Applicability {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Applicability::MachineApplicable => "machine-applicable",
            Applicability::MaybeIncorrect => "maybe-incorrect",
        }
    }
}

/// Một thay đổi văn bản: thay `span` bằng `replacement` (rỗng = xoá; span rỗng = chèn).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub file: String,
    pub span: Span,
    pub replacement: String,
}

impl Edit {
    /// Edit trong tệp của chẩn đoán (tên tệp để trống; `Diagnostic::with_suggestion` điền).
    #[must_use]
    pub fn new(span: Span, replacement: impl Into<String>) -> Edit {
        Edit {
            file: String::new(),
            span,
            replacement: replacement.into(),
        }
    }
}

/// Một gợi ý sửa: tập edit áp dụng NGUYÊN TỬ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub message: String,
    pub applicability: Applicability,
    pub edits: Vec<Edit>,
}

impl Suggestion {
    #[must_use]
    pub fn new(
        message: impl Into<String>,
        applicability: Applicability,
        edits: Vec<Edit>,
    ) -> Suggestion {
        Suggestion {
            message: message.into(),
            applicability,
            edits,
        }
    }
}

/// Nhãn phụ trên một span (có thể ở tệp khác từ M3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub file: String,
    pub span: Span,
    pub message: String,
}

/// Một chẩn đoán. `level` luôn suy ra từ `code` (D16).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: Code,
    pub level: Level,
    pub message: String,
    pub file: String,
    pub span: Span,
    pub span_label: Option<String>,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    #[must_use]
    pub fn new(code: Code, file: &str, span: Span, message: impl Into<String>) -> Diagnostic {
        let level = code.level();
        Diagnostic {
            code,
            level,
            message: message.into(),
            file: file.to_string(),
            span,
            span_label: None,
            labels: Vec::new(),
            notes: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_span_label(mut self, text: impl Into<String>) -> Diagnostic {
        self.span_label = Some(text.into());
        self
    }

    #[must_use]
    pub fn with_label(mut self, file: &str, span: Span, message: impl Into<String>) -> Diagnostic {
        self.labels.push(Label {
            file: file.to_string(),
            span,
            message: message.into(),
        });
        self
    }

    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Diagnostic {
        self.notes.push(note.into());
        self
    }

    /// Thêm gợi ý; edit chưa có tên tệp được gán tệp của chẩn đoán.
    #[must_use]
    pub fn with_suggestion(mut self, mut s: Suggestion) -> Diagnostic {
        for e in &mut s.edits {
            if e.file.is_empty() {
                e.file.clone_from(&self.file);
            }
        }
        self.suggestions.push(s);
        self
    }
}
