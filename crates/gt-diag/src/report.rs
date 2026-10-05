//! Bộ gom chẩn đoán: thứ tự xác định, bỏ trùng, giới hạn số lượng (D19), phán quyết (D17).
use crate::code::Level;
use crate::diag::Diagnostic;

/// Bộ gom. `limit` = số chẩn đoán thường tối đa giữ lại (ICE luôn được giữ).
#[derive(Debug)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
    limit: usize,
}

/// Kết quả cuối: đã sắp xếp, đã bỏ trùng, đã cắt theo giới hạn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    /// Số chẩn đoán bị cắt do giới hạn.
    pub truncated: usize,
}

impl Diagnostics {
    #[must_use]
    pub fn new(limit: usize) -> Diagnostics {
        Diagnostics {
            items: Vec::new(),
            limit,
        }
    }

    pub fn push(&mut self, d: Diagnostic) {
        self.items.push(d);
    }

    /// Sắp theo (tệp, vị trí, mã, thông điệp), bỏ bản trùng hệt, cắt theo giới hạn (không bao giờ cắt ICE).
    #[must_use]
    pub fn finish(self) -> Report {
        let mut v = self.items;
        v.sort_by(|a, b| {
            (
                &a.file,
                a.span.start,
                a.span.end,
                a.code.as_str(),
                &a.message,
            )
                .cmp(&(
                    &b.file,
                    b.span.start,
                    b.span.end,
                    b.code.as_str(),
                    &b.message,
                ))
        });
        v.dedup();
        let total = v.len();
        let mut kept = Vec::new();
        let mut normal = 0usize;
        for d in v {
            if d.code.is_ice() {
                kept.push(d);
            } else if normal < self.limit {
                normal += 1;
                kept.push(d);
            }
        }
        let truncated = total - kept.len();
        Report {
            diagnostics: kept,
            truncated,
        }
    }
}

impl Report {
    fn count(&self, level: Level) -> usize {
        self.diagnostics.iter().filter(|d| d.level == level).count()
    }

    #[must_use]
    pub fn errors(&self) -> usize {
        self.count(Level::Error)
    }

    #[must_use]
    pub fn warnings(&self) -> usize {
        self.count(Level::Warn)
    }

    #[must_use]
    pub fn notes(&self) -> usize {
        self.count(Level::Note)
    }

    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.errors() > 0
    }

    /// D17: 0 = qua (có thể kèm VÀNG), 1 = có ĐỎ, 2 = ICE. `deny_yellow` nâng VÀNG thành ĐỎ.
    #[must_use]
    pub fn exit_code(&self, deny_yellow: bool) -> i32 {
        if self.diagnostics.iter().any(|d| d.code.is_ice()) {
            2
        } else if self.errors() > 0 || (deny_yellow && self.warnings() > 0) {
            1
        } else {
            0
        }
    }
}
