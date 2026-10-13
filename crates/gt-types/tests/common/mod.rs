#![allow(dead_code)]
//! Dùng chung cho test gt-types: fixture prelude phong phú (giả lập `lib/core`, M12) và hàm chạy.
use gt_resolve::{Prelude, SourceInput};
use gt_types::{TypesOutput, check_program};

/// MỘT nguồn prelude duy nhất cho gritc và mọi test: `lib/core/prelude.gt` (giả lập `lib/core`, M12). Không có bản chép tay (khoá bởi `gt-driver/tests/gate.rs::prelude_has_a_single_source`).
pub const PRELUDE_SRC: &str = include_str!("../../../../lib/core/prelude.gt");

pub fn prelude() -> Prelude {
    Prelude::from_source(PRELUDE_SRC).expect("prelude hợp lệ")
}

pub fn run(files: &[(&str, &str)]) -> TypesOutput {
    let inputs: Vec<SourceInput> = files
        .iter()
        .map(|(n, t)| SourceInput {
            name: (*n).to_string(),
            text: (*t).to_string(),
        })
        .collect();
    check_program(&inputs, &prelude()).expect("đầu vào hợp lệ")
}

pub fn one(src: &str) -> TypesOutput {
    run(&[("main.gt", src)])
}

pub fn codes(out: &TypesOutput) -> Vec<String> {
    out.report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect()
}

pub fn show(out: &TypesOutput) -> Vec<String> {
    out.report
        .diagnostics
        .iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

/// Phải qua sạch.
pub fn ok(src: &str) -> TypesOutput {
    let out = one(src);
    assert!(
        out.report.diagnostics.is_empty(),
        "đáng lẽ sạch: {:?}\n{src}",
        show(&out)
    );
    out
}

/// Phải ra đúng `n` chẩn đoán, tất cả mang mã `code`.
pub fn err(src: &str, code: &str, n: usize) -> TypesOutput {
    let out = one(src);
    assert_eq!(
        codes(&out),
        vec![code.to_string(); n],
        "{src}\n{:?}",
        show(&out)
    );
    out
}
