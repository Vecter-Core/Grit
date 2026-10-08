#![allow(dead_code)]
//! Dùng chung cho test gt-resolve: fixture prelude (ADR-017c — resolver KHÔNG có tên nào viết cứng) và các hàm chạy.
use gt_resolve::{Prelude, ResolveOutput, SourceInput, resolve_program};

/// Prelude giả lập `lib/core` (M12). Mọi tên Option/Some/None/… đến từ đây, không từ resolver.
pub const PRELUDE_SRC: &str = "\
⊔ Option<T> { None, Some(T) }
⊔ Result<T, E> { Ok(T), Err(E) }
⊏ Copy { }
⊏ Drop { }
⊏ Send { }
ƒ spawn<A: Send>(f: ƒ(A) ! {detach}, a: A) ! {detach} { }
#lang(cell)
▢ Cell<T> { v: T }
⊐ <T> Cell<T> {
  ƒ new(v: T) → Cell<T> ! ∅ { }
  ƒ set(&self, x: T) ! ∅ { }
  ƒ get(&self) → T ! ∅ { }
}
#lang(sync)
⊏ Sync { }
#lang(ordering)
⊔ Ordering { Relaxed, Acquire, Release, AcqRel, SeqCst }
#lang(atomic_int)
⊏ AtomicInt { }
⊐ AtomicInt for bool { }
⊐ AtomicInt for i32 { }
⊐ AtomicInt for u32 { }
⊐ AtomicInt for i64 { }
⊐ AtomicInt for u64 { }
⊐ AtomicInt for usize { }
#lang(atomic)
▢ Atomic<T: AtomicInt> { v: T }
⊐ <T: AtomicInt> Atomic<T> {
  ƒ new(v: T) → Atomic<T> ! ∅ { }
  ƒ load(&self, o: Ordering) → T ! ∅ { }
  ƒ store(&self, v: T, o: Ordering) ! ∅ { }
  ƒ swap(&self, v: T, o: Ordering) → T ! ∅ { }
  ƒ fetch_add(&self, v: T, o: Ordering) → T ! ∅ { }
  ƒ fetch_sub(&self, v: T, o: Ordering) → T ! ∅ { }
  ƒ compare_exchange(&self, cur: T, new: T, s: Ordering, f: Ordering) → Result<T, T> ! ∅ { }
}
#lang(mutex)
▢ Mutex<T, L> { v: T }
⊐ <T, L> Mutex<T, L> {
  ƒ new(v: T) → Mutex<T, L> ! ∅ { }
  ƒ with<R: Send, Arg: Send>(&self, f: ƒ(&!T, Arg) → R ! {poly}, a: Arg) → R ! {poly, lock(L)} { }
}
▢ Buf { n: i32 }
ƒ mk_buf() → Buf ! ∅ { }
▢ Guard { ◈ inner: Buf, ◈ n: i32 }
ƒ mk_guard() → Guard ! ∅ { }
▢ Range<T> { start: T, end: T }
⊐ <T> Option<T> {
  ƒ is_some(&self) → bool ! ∅ { }
  ƒ unwrap(self) → T ! ∅ ⊳ self.is_some() { }
}
ƒ shl(x: u32, n: u32) → u32 ! ∅ ⊳ n < 32 { }
ƒ shr(x: u32, n: u32) → u32 ! ∅ ⊳ n < 32 { }
▢ Secret<T> { v: T }
▢ Cap<T> { v: T }
▢ Fs { x: i32 }
▢ Net { x: i32 }
▢ Env { x: i32 }
▢ Clock { x: i32 }
▢ Alloc { x: i32 }
▢ Host { x: i32 }
▢ File { x: i32 }
▢ Arena { x: i32 }
▦ core {
  ◈ ƒ abort() ! ∅ { }
  ƒ hidden() ! ∅ { }
}
ƒ root_fs() → Cap<Fs> ! ∅ { }
ƒ root_net() → Cap<Net> ! ∅ { }
ƒ root_env() → Cap<Env> ! ∅ { }
ƒ root_clock() → Cap<Clock> ! ∅ { }
ƒ root_alloc() → Cap<Alloc> ! ∅ { }
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
