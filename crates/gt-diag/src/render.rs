//! Xuất dạng người đọc (rustc-style, thuần văn bản; màu ANSI là tuỳ chọn và chỉ thêm mã màu, không đổi cấu trúc).
use crate::code::Level;
use crate::diag::{Diagnostic, Edit};
use gt_span::{SourceFile, Span};

/// Tuỳ chọn xuất. `color = false` (mặc định cho đầu ra tất định/CI).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderOptions {
    pub color: bool,
}

const RESET: &str = "\u{1b}[0m";
const SECONDARY: &str = "\u{1b}[1;36m";

fn level_color(l: Level) -> &'static str {
    match l {
        Level::Error => "\u{1b}[1;31m",
        Level::Warn => "\u{1b}[1;33m",
        Level::Note => "\u{1b}[1;37m",
    }
}

fn paint(opts: &RenderOptions, color: &str, text: &str) -> String {
    if opts.color && !text.is_empty() {
        format!("{color}{text}{RESET}")
    } else {
        text.to_string()
    }
}

/// Nhãn đã quy ra dòng/cột để vẽ.
struct Placed {
    primary: bool,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    text: String,
}

fn place(file: &SourceFile, span: Span, primary: bool, text: &str) -> Option<Placed> {
    let s = file.line_col(span.start)?;
    let e = file.line_col(span.end)?;
    // span kết thúc đúng đầu một dòng sau → không tính dòng đó
    let (end_line, end_col) = if e.line > s.line && e.col == 1 {
        let prev = e.line - 1;
        (
            prev,
            file.line_text(prev)
                .map_or(1, |t| t.chars().count() as u32 + 1),
        )
    } else {
        (e.line, e.col)
    };
    Some(Placed {
        primary,
        start_line: s.line,
        start_col: s.col,
        end_line,
        end_col,
        text: text.to_string(),
    })
}

fn display_line(file: &SourceFile, n: u32) -> String {
    file.line_text(n)
        .unwrap_or("")
        .chars()
        .map(|c| if c == '\t' { ' ' } else { c })
        .collect()
}

fn digits(mut n: u32) -> usize {
    let mut d = 1;
    while n >= 10 {
        n /= 10;
        d += 1;
    }
    d
}

fn escape_inline(s: &str) -> String {
    s.replace('\n', "\\n").replace('\r', "\\r")
}

fn edit_line(file: &SourceFile, e: &Edit) -> String {
    let (Some(s), Some(t), Some(old)) = (
        file.line_col(e.span.start),
        file.line_col(e.span.end),
        file.snippet(e.span),
    ) else {
        return "- span ngoài tệp".to_string();
    };
    format!(
        "- {}:{}-{}:{}: `{}` -> `{}`",
        s.line,
        s.col,
        t.line,
        t.col,
        escape_inline(old),
        escape_inline(&e.replacement)
    )
}

/// Xuất một chẩn đoán ở dạng người đọc.
#[must_use]
pub fn render_human(d: &Diagnostic, file: &SourceFile, opts: &RenderOptions) -> String {
    let lc = level_color(d.level);
    let mut out = String::new();
    out.push_str(&paint(
        opts,
        lc,
        &format!("{}[{}]", d.level.as_str(), d.code),
    ));
    out.push_str(&format!(": {}\n", d.message));

    // gom nhãn vẽ được; nhãn ở tệp khác (hoặc ngoài tệp) chỉ liệt kê dạng văn bản
    let mut placed: Vec<Placed> = Vec::new();
    let mut loose: Vec<String> = Vec::new();
    let primary_placed = if d.file == file.name() {
        place(file, d.span, true, d.span_label.as_deref().unwrap_or(""))
    } else {
        None
    };
    if let Some(p) = primary_placed {
        placed.push(p);
    }
    for l in &d.labels {
        match (l.file == file.name())
            .then(|| place(file, l.span, false, &l.message))
            .flatten()
        {
            Some(p) => placed.push(p),
            None => loose.push(format!("label: {} ({})", l.message, l.file)),
        }
    }
    placed.sort_by_key(|p| (p.start_line, p.start_col, p.end_line, p.end_col, !p.primary));

    let max_line = placed.iter().map(|p| p.end_line).max().unwrap_or(1);
    let w = digits(max_line);
    let gutter = " ".repeat(w);

    // dòng vị trí
    match placed.iter().find(|p| p.primary) {
        Some(p) => out.push_str(&format!(
            "{gutter}--> {}:{}:{}\n",
            d.file, p.start_line, p.start_col
        )),
        None => out.push_str(&format!(
            "{gutter}--> {}: span ngoài tệp ({}..{})\n",
            d.file, d.span.start, d.span.end
        )),
    }

    if !placed.is_empty() {
        out.push_str(&format!("{gutter} |\n"));
        let mut last_printed: Option<u32> = None;
        for p in &placed {
            let marker = if p.primary { '^' } else { '-' };
            let mcolor = if p.primary { lc } else { SECONDARY };
            let span_lines = p.end_line - p.start_line + 1;
            let lines: Vec<Option<u32>> = if span_lines > 4 {
                vec![
                    Some(p.start_line),
                    Some(p.start_line + 1),
                    None,
                    Some(p.end_line),
                ]
            } else {
                (p.start_line..=p.end_line).map(Some).collect()
            };
            for item in lines {
                let Some(n) = item else {
                    out.push_str("...\n");
                    last_printed = None;
                    continue;
                };
                if last_printed != Some(n) {
                    if last_printed.is_some_and(|prev| n > prev + 1) {
                        out.push_str("...\n");
                    }
                    out.push_str(&format!("{n:>w$} | {}\n", display_line(file, n), w = w));
                    last_printed = Some(n);
                }
                let len = display_line(file, n).chars().count() as u32;
                let (from, to) = if p.start_line == p.end_line {
                    (p.start_col, p.end_col)
                } else if n == p.start_line {
                    (p.start_col, len + 1)
                } else if n == p.end_line {
                    (1, p.end_col)
                } else {
                    (1, len + 1)
                };
                let count = to.saturating_sub(from).max(1) as usize;
                let carets: String = std::iter::repeat_n(marker, count).collect();
                let mut row = format!("{gutter} | {}", " ".repeat((from - 1) as usize));
                row.push_str(&paint(opts, mcolor, &carets));
                if n == p.end_line && !p.text.is_empty() {
                    row.push(' ');
                    row.push_str(&p.text);
                }
                out.push_str(&row);
                out.push('\n');
            }
        }
    }
    for l in &loose {
        out.push_str(&format!("{gutter} = {l}\n"));
    }
    for n in &d.notes {
        out.push_str(&format!("{gutter} = note: {n}\n"));
    }
    for s in &d.suggestions {
        out.push_str(&format!(
            "{gutter} = help: {} ({})\n",
            s.message,
            s.applicability.as_str()
        ));
        let indent = " ".repeat(w + " = help: ".len());
        for e in &s.edits {
            out.push_str(&format!("{indent}{}\n", edit_line(file, e)));
        }
    }
    out
}
