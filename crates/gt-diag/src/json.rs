//! Xuất JSON có version. Viết tay để tất định (thứ tự khoá cố định) và không thêm phụ thuộc;
//! tính hợp lệ được kiểm bằng serde_json (dev-dependency) trong test.
use crate::diag::{Diagnostic, Edit, Label, Suggestion};
use crate::report::Report;
use gt_span::{SourceFile, Span};
use std::fmt::Write as _;

/// Phiên bản schema (docs/spec/diag_schema.json). Cùng version chỉ được THÊM trường.
pub const SCHEMA_VERSION: u32 = 1;

fn esc(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn pos(file: Option<&SourceFile>, offset: u32, out: &mut String) {
    let _ = write!(out, "{{\"offset\":{offset},\"line\":");
    match file.and_then(|f| f.line_col(offset)) {
        Some(lc) => {
            let _ = write!(out, "{},\"col\":{}}}", lc.line, lc.col);
        }
        None => out.push_str("null,\"col\":null}"),
    }
}

fn span(file: Option<&SourceFile>, s: Span, out: &mut String) {
    out.push_str("{\"start\":");
    pos(file, s.start, out);
    out.push_str(",\"end\":");
    pos(file, s.end, out);
    out.push('}');
}

fn find<'a>(files: &[&'a SourceFile], name: &str) -> Option<&'a SourceFile> {
    files.iter().copied().find(|f| f.name() == name)
}

fn label(l: &Label, files: &[&SourceFile], out: &mut String) {
    out.push_str("{\"file\":");
    esc(&l.file, out);
    out.push_str(",\"span\":");
    span(find(files, &l.file), l.span, out);
    out.push_str(",\"message\":");
    esc(&l.message, out);
    out.push('}');
}

fn edit(e: &Edit, files: &[&SourceFile], out: &mut String) {
    out.push_str("{\"file\":");
    esc(&e.file, out);
    out.push_str(",\"span\":");
    span(find(files, &e.file), e.span, out);
    out.push_str(",\"replacement\":");
    esc(&e.replacement, out);
    out.push('}');
}

fn suggestion(s: &Suggestion, files: &[&SourceFile], out: &mut String) {
    out.push_str("{\"message\":");
    esc(&s.message, out);
    let _ = write!(
        out,
        ",\"applicability\":\"{}\",\"edits\":[",
        s.applicability.as_str()
    );
    for (i, e) in s.edits.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        edit(e, files, out);
    }
    out.push_str("]}");
}

fn list<T>(items: &[T], out: &mut String, mut f: impl FnMut(&T, &mut String)) {
    out.push('[');
    for (i, it) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        f(it, out);
    }
    out.push(']');
}

/// JSON của một chẩn đoán (đối tượng `diagnostic` trong schema).
#[must_use]
pub fn diagnostic_to_json(d: &Diagnostic, files: &[&SourceFile]) -> String {
    let mut out = String::new();
    write_diagnostic(d, files, &mut out);
    out
}

fn write_diagnostic(d: &Diagnostic, files: &[&SourceFile], out: &mut String) {
    let _ = write!(
        out,
        "{{\"code\":\"{}\",\"level\":\"{}\",\"message\":",
        d.code,
        d.level.as_str()
    );
    esc(&d.message, out);
    out.push_str(",\"file\":");
    esc(&d.file, out);
    out.push_str(",\"span\":");
    span(find(files, &d.file), d.span, out);
    out.push_str(",\"span_label\":");
    match &d.span_label {
        Some(t) => esc(t, out),
        None => out.push_str("null"),
    }
    out.push_str(",\"labels\":");
    list(&d.labels, out, |l, o| label(l, files, o));
    out.push_str(",\"notes\":");
    list(&d.notes, out, |n, o| esc(n, o));
    out.push_str(",\"suggestions\":");
    list(&d.suggestions, out, |s, o| suggestion(s, files, o));
    out.push('}');
}

impl Report {
    /// JSON đầy đủ (`schema_version` 1). `files`: các tệp nguồn để quy offset ra dòng/cột.
    #[must_use]
    pub fn to_json(&self, files: &[&SourceFile], deny_yellow: bool) -> String {
        let mut out = String::new();
        let _ = write!(
            out,
            "{{\"schema_version\":{SCHEMA_VERSION},\"tool\":\"gritc\",\"diagnostics\":"
        );
        list(&self.diagnostics, &mut out, |d, o| {
            write_diagnostic(d, files, o)
        });
        let _ = write!(
            out,
            ",\"summary\":{{\"errors\":{},\"warnings\":{},\"notes\":{},\"truncated\":{},\"exit_code\":{}}}}}",
            self.errors(),
            self.warnings(),
            self.notes(),
            self.truncated,
            self.exit_code(deny_yellow)
        );
        out
    }
}
