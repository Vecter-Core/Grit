#![allow(dead_code)]
//! Dùng chung cho test gt-resolve: fixture prelude (ADR-017c — resolver KHÔNG có tên nào viết cứng) và các hàm chạy.
use gt_resolve::{Prelude, ResolveOutput, SourceInput, resolve_program};

/// Prelude giả lập `lib/core` (M12). Mọi tên Option/Some/None/… đến từ đây, không từ resolver.
pub const PRELUDE_SRC: &str = "\
⊔ Option<T> { None, Some(T) }
⊔ Result<T, E> { Ok(T), Err(E) }
▢ Secret<T> { v: T }
▢ Cap<T> { v: T }
▢ Fs { x: i32 }
▢ Net { x: i32 }
▢ Arena { x: i32 }
▦ core {
  ◈ ƒ abort() ! ∅ { }
  ƒ hidden() ! ∅ { }
}
ƒ root_fs() → Cap<Fs> ! ∅ { }
ƒ raw_read(p: usize) → i32 ! ∅ { }
";

pub fn prelude() -> Prelude {
    Prelude::from_source(PRELUDE_SRC).expect("prelude hợp lệ")
}

pub fn run(files: &[(&str, &str)]) -> ResolveOutput {
    let inputs: Vec<SourceInput> = files
        .iter()
        .map(|(n, t)| SourceInput {
            name: (*n).to_string(),
            text: (*t).to_string(),
        })
        .collect();
    resolve_program(&inputs, &prelude()).expect("đầu vào hợp lệ")
}

/// Một tệp `main.gt`.
pub fn one(src: &str) -> ResolveOutput {
    run(&[("main.gt", src)])
}

pub fn codes(out: &ResolveOutput) -> Vec<String> {
    out.report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect()
}

/// Phải qua sạch (không chẩn đoán nào).
pub fn assert_ok(files: &[(&str, &str)]) {
    let out = run(files);
    assert!(
        out.report.diagnostics.is_empty(),
        "đáng lẽ sạch: {:?}\n{files:?}",
        out.report
            .diagnostics
            .iter()
            .map(|d| format!("{} {}", d.code, d.message))
            .collect::<Vec<_>>()
    );
}

/// Phải ra ĐÚNG `n` chẩn đoán, tất cả mang mã `code`.
pub fn assert_err(files: &[(&str, &str)], code: &str, n: usize) -> ResolveOutput {
    let out = run(files);
    let got = codes(&out);
    assert_eq!(
        got,
        vec![code.to_string(); n],
        "{files:?}\n{:?}",
        out.report.diagnostics
    );
    out
}
