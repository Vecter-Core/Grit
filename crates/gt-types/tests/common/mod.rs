#![allow(dead_code)]
//! Dùng chung cho test gt-types: fixture prelude phong phú (giả lập `lib/core`, M12) và hàm chạy.
use gt_resolve::{Prelude, SourceInput};
use gt_types::{TypesOutput, check_program};

/// Prelude cho corpus M4. Các kiểu lang mang `#lang(…)` (chỉ hợp lệ ở đây — ADR-018 §5). Thân hàm của prelude KHÔNG được kiểm
/// (trusted intrinsic); chỉ chữ ký có nghĩa.
pub const PRELUDE_SRC: &str = "\
#lang(option)
⊔ Option<T> { None, Some(T) }
⊔ Result<T, E> { Ok(T), Err(E) }
#lang(range)
▢ Range<T> { start: T, end: T }
#lang(copy)
⊏ Copy { }
#lang(secret)
▢ Secret<T> { v: T }
#lang(cap)
▢ Cap<T> { v: T }
▢ Fs { x: i32 }
▢ Net { x: i32 }
▢ Arena { x: i32 }
▢ File { x: i32 }
⊐ <T> [T] { ƒ len(&self) → usize ! ∅ { } }
⊐ File { ƒ len(&self) → usize ! ∅ { } }
⊐ Cap<Fs> { ƒ read(&self, p: &str) → File ! {io} { } }
⊐ Arena { ƒ new() → Arena ! {mem} { } ƒ alloc<T>(&self, v: T) → &T ! {mem} { } }
▦ core { ◈ ƒ abort(msg: &str) → ! ! {div} { } }
ƒ root_fs() → Cap<Fs> ! ∅ { }
ƒ raw_read(p: usize) → i32 ! ∅ { }
";

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
