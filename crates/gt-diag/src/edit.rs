//! Áp dụng edit có cấu trúc lên văn bản.
use crate::diag::Edit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditError {
    /// Span vượt quá văn bản.
    OutOfBounds,
    /// Span cắt giữa một ký tự UTF-8.
    NotCharBoundary,
    /// Hai edit chồng nhau, hoặc cùng điểm bắt đầu (thứ tự mơ hồ) — từ chối (fail-closed).
    Overlap,
}

/// Áp dụng NGUYÊN TỬ mọi edit lên `text` (độc lập thứ tự đưa vào). Lỗi → không áp dụng gì cả.
pub fn apply_edits(text: &str, edits: &[Edit]) -> Result<String, EditError> {
    let mut sorted: Vec<&Edit> = edits.iter().collect();
    sorted.sort_by_key(|e| (e.span.start, e.span.end));
    for e in &sorted {
        let (s, t) = (e.span.start as usize, e.span.end as usize);
        if s > t || t > text.len() {
            return Err(EditError::OutOfBounds);
        }
        if !text.is_char_boundary(s) || !text.is_char_boundary(t) {
            return Err(EditError::NotCharBoundary);
        }
    }
    for w in sorted.windows(2) {
        if w[1].span.start < w[0].span.end || w[1].span.start == w[0].span.start {
            return Err(EditError::Overlap);
        }
    }
    let mut out = String::with_capacity(text.len());
    let mut pos = 0usize;
    for e in sorted {
        out.push_str(&text[pos..e.span.start as usize]);
        out.push_str(&e.replacement);
        pos = e.span.end as usize;
    }
    out.push_str(&text[pos..]);
    Ok(out)
}
