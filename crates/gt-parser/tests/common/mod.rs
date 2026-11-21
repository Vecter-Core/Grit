//! Công cụ dùng chung cho test parser còn lại (`corpus`, `depth_law`). Bộ in S-expression của AST đã bỏ cùng các test hình-dạng-AST (GÓI 12):
//! hành vi parse được kiểm bằng mẫu compile-pass/compile-fail của `crates/gt-driver/tests/corpus_e2e.rs`, không bằng hình dạng cây nội bộ.
#![allow(dead_code)]
use gt_parser::{ParseOutput, parse};

pub fn parse_src(src: &str) -> ParseOutput {
    parse("t.gt", src)
}

/// Che mọi `Span { start: N, end: M }` trong chuỗi Debug để so sánh hai AST không tính vị trí.
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
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}
