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
#lang(range_inclusive)
▢ RangeInclusive<T> { start: T, end: T }
#lang(copy)
⊏ Copy { }
#lang(send)
⊏ Send { }
ƒ spawn<A: Send>(f: ƒ(A) ! {detach}, a: A) ! {detach} { }
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
#lang(cell)
▢ Cell<T> { v: T }
⊐ <T> Cell<T> {
  ƒ new(v: T) → Cell<T> ! ∅ { }
  ƒ set(&self, x: T) ! ∅ { }
  ƒ get(&self) → T ! ∅ { }
}
#lang(drop)
⊏ Drop { ƒ drop(&!self) ! ∅; }
▢ Buf { n: i32 }
⊐ Drop for Buf { ƒ drop(&!self) ! ∅ { } }
ƒ mk_buf() → Buf ! ∅ { }
▢ Guard { ◈ inner: Buf, ◈ n: i32 }
⊐ Drop for Guard { ƒ drop(&!self) ! ∅ { } }
ƒ mk_guard() → Guard ! ∅ { }
#lang(secret)
▢ Secret<T> { v: T }
⊐ <T> Secret<T> { ƒ declassify(self) → T ! ∅ { } }
#lang(cap)
▢ Cap<T> { v: T }
▢ Fs { x: i32 }
▢ Net { x: i32 }
▢ Env { x: i32 }
▢ Clock { x: i32 }
▢ Alloc { x: i32 }
▢ Host { x: i32 }
#lang(arena)
▢ Arena { x: i32 }
▢ File { x: i32 }
⊐ <T> [T] { ƒ len(&self) → usize ! ∅ { } }
⊐ File { ƒ len(&self) → usize ! ∅ { } }
⊐ <T> Option<T> {
  ƒ is_some(&self) → bool ! ∅ { }
  ƒ unwrap(self) → T ! ∅ ⊳ self.is_some() { }
}
ƒ shl(x: u32, n: u32) → u32 ! ∅ ⊳ n < 32 { }
ƒ shr(x: u32, n: u32) → u32 ! ∅ ⊳ n < 32 { }
⊐ Cap<Fs> {
  ƒ read(&self, p: &str) → File ! {io} { }
  ƒ sub(&self, p: &str) → Cap<Fs> ! {io} { }
}
⊐ Cap<Net> { ƒ connect(&self, h: &str) → File ! {io} { } }
⊐ Cap<Env> { ƒ var(&self, n: &str) → usize ! {io} { } }
⊐ Cap<Clock> { ƒ now(&self) → u64 ! {io} { } }
⊐ Cap<Alloc> { ƒ arena(&self) → Arena ! {mem} { } }
⊐ Host { ƒ fs() → Cap<Fs> ! {io} { } }
⊐ Arena { ƒ new() → Arena ! {mem} { } ƒ alloc<T>(&self, v: T) → &T ! {mem} { } ƒ reset(&!self) ! {mem} { } }
▦ core { ◈ ƒ abort(msg: &str) → ! ! {div} { } }
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
