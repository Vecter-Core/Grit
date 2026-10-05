#![allow(dead_code)]
use gt_fmt::format;

/// Định dạng, fail nếu bị từ chối.
pub fn fmt_ok(src: &str) -> String {
    match format("t.gt", src) {
        Ok(s) => s,
        Err(e) => panic!("gt-fmt từ chối:\n{src}\n{:?}", e.report.diagnostics),
    }
}

/// Mã lỗi khi gt-fmt từ chối.
pub fn fmt_err(src: &str) -> Vec<String> {
    match format("t.gt", src) {
        Ok(s) => panic!("đáng lẽ phải bị từ chối:\n{src}\n=> {s}"),
        Err(e) => e
            .report
            .diagnostics
            .iter()
            .map(|d| d.code.as_str().to_string())
            .collect(),
    }
}

/// Biểu thức trong thân hàm một dòng: trả về biểu thức đã định dạng.
pub fn fe(e: &str) -> String {
    let out = fmt_ok(&format!("ƒ f() → i32 ! ∅ {{ {e} }}"));
    let (pre, post) = ("ƒ f() → i32 ! ∅ { ", " }\n");
    assert!(
        out.starts_with(pre) && out.ends_with(post),
        "không còn là một dòng:\n{out}"
    );
    out[pre.len()..out.len() - post.len()].to_string()
}

/// Bỏ các dòng chỉ chứa comment `//` (để so sánh alias/canonical không bị khác ở phần ghi chú).
pub fn strip_comment_lines(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Che span trong chuỗi Debug của AST.
pub fn mask_spans(debug: &str) -> String {
    let mut out = String::new();
    let mut rest = debug;
    while let Some(i) = rest.find("Span {") {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        match tail.find('}') {
            Some(j) => {
                out.push_str("Span");
                rest = &tail[j + 1..];
            }
            None => rest = "",
        }
    }
    out.push_str(rest);
    out
}

pub fn ast_of(src: &str) -> String {
    let out = gt_parser::parse("t.gt", src);
    assert!(
        out.report.diagnostics.is_empty(),
        "{src}\n{:?}",
        out.report.diagnostics
    );
    mask_spans(&format!("{:?}", out.program))
}

pub fn comment_texts(src: &str) -> Vec<String> {
    gt_lexer::lex(src)
        .trivia
        .iter()
        .map(|t| t.text.clone())
        .collect()
}
