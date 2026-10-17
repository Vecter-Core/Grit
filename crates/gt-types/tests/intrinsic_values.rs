//! FAIL_OPEN_AUDIT tổng quát hoá S-E (lệnh Giang 2026-10-17 mục 2): hàm intrinsic của lib/core (thân prelude KHÔNG có MIR) KHÔNG được dùng làm GIÁ TRỊ: mọi nghĩa vụ theo lớp
//! (precondition `n < 32`, E0310, E0405, E0502, E0501…) được sinh ở điểm gọi TRỰC TIẾP nên gọi qua con trỏ né kiểm tra của cả lớp; LLVM từ chối ("giá trị hàm không có thân MIR"),
//! thông dịch có chỗ chạy được. Phase 1: từ chối ở M4 bằng E0313 (riêng `Atomic` giữ E0503 của gt-conc). Hàm viết bằng Grit, gọi TRỰC TIẾP intrinsic, cú pháp phương thức: vẫn dùng được.
mod common;
use common::*;

fn only(name: &str, src: &str, code: &str) {
    let out = one(src);
    let c = codes(&out);
    assert!(
        !c.is_empty() && c.iter().all(|x| x == code),
        "{name}: kỳ vọng chỉ {code}, có {c:?} {:?}
{src}",
        show(&out)
    );
}

#[test]
fn intrinsics_used_as_function_values_are_rejected() {
    only(
        "shl",
        r#"ƒ main() → u32 ! ∅ {
  f: ƒ(u32, u32) → u32 ! ∅ ≔ shl;
  f(1u32, 40u32)
}
"#,
        "E0313",
    );
    only(
        "shr",
        r#"ƒ main() → u32 ! ∅ {
  f: ƒ(u32, u32) → u32 ! ∅ ≔ shr;
  f(1u32, 40u32)
}
"#,
        "E0313",
    );
    only(
        "rotl",
        r#"ƒ main() → u32 ! ∅ {
  f: ƒ(u32, u32) → u32 ! ∅ ≔ rotl;
  f(1u32, 40u32)
}
"#,
        "E0313",
    );
    only(
        "rotr",
        r#"ƒ main() → u32 ! ∅ {
  f: ƒ(u32, u32) → u32 ! ∅ ≔ rotr;
  f(1u32, 40u32)
}
"#,
        "E0313",
    );
    only(
        "band",
        r#"ƒ main() → u32 ! ∅ {
  f: ƒ(u32, u32) → u32 ! ∅ ≔ band;
  f(6u32, 3u32)
}
"#,
        "E0313",
    );
    only(
        "bnot",
        r#"ƒ main() → u32 ! ∅ {
  f: ƒ(u32) → u32 ! ∅ ≔ bnot;
  f(6u32)
}
"#,
        "E0313",
    );
    only(
        "core_abort",
        r#"ƒ main() ! {div} {
  f: ƒ(&str) → ! ! {div} ≔ core::abort;
  f("x")
}
"#,
        "E0313",
    );
    only(
        "core_exit",
        r#"ƒ main() ! {div} {
  f: ƒ(u32) → ! ! {div} ≔ core::exit;
  f(3u32)
}
"#,
        "E0313",
    );
    only(
        "core_fuel_left",
        r#"ƒ main() ! ∅ {
  f: ƒ(u64) ! ∅ ≔ core::fuel_left;
  f(0u64);
}
"#,
        "E0313",
    );
    only(
        "core_f64_from_bits",
        r#"ƒ main() → bool ! ∅ {
  f: ƒ(u64) → f64 ! ∅ ≔ core::f64_from_bits;
  x ≔ f(0u64);
  x.is_nan()
}
"#,
        "E0313",
    );
    only(
        "raw_read",
        r#"ƒ main() → i32 ! ∅ {
  f: ƒ(usize) → i32 ! ∅ ≔ raw_read;
  f(0usize)
}
"#,
        "E0313",
    );
    only(
        "cell_get",
        r#"ƒ main() → u32 ! ∅ {
  c ≔ Cell::new(1u32);
  f: ƒ(&Cell<u32>) → u32 ! ∅ ≔ Cell::get;
  f(&c)
}
"#,
        "E0313",
    );
    only(
        "option_is_some",
        r#"ƒ main() → bool ! ∅ {
  f: ƒ(&Option<u32>) → bool ! ∅ ≔ Option::is_some;
  o ≔ Some(1u32);
  f(&o)
}
"#,
        "E0313",
    );
    only(
        "option_unwrap",
        r#"ƒ main() → u32 ! ∅ {
  f: ƒ(Option<u32>) → u32 ! ∅ ≔ Option::unwrap;
  f(None)
}
"#,
        "E0313",
    );
    only(
        "vec_len",
        r#"ƒ main() → usize ! {mem} {
  μa ≔ Arena::new();
  μv: Vec<u32> ≔ Vec::new(&!a);
  f: ƒ(&Vec<u32>) → usize ! ∅ ≔ Vec::len;
  f(&v)
}
"#,
        "E0313",
    );
    only(
        "vec_as_slice",
        r#"ƒ main() → usize ! {mem} {
  μa ≔ Arena::new();
  μv ≔ Vec::new(&!a);
  v.push(1u32);
  s: ƒ(&Vec<u32>) → &[u32] ! ∅ ≔ Vec::as_slice;
  x ≔ s(&v);
  x.len()
}
"#,
        "E0313",
    );
    only(
        "cap_out_write",
        r#"ƒ main() ! {io} {
  o ≔ root_out();
  f: ƒ(&Cap<Out>, u64) ! {io} ≔ Cap::write_u64;
  f(&o, 7u64);
}
"#,
        "E0313",
    );
    only(
        "cap_clock_now",
        r#"ƒ main() → u64 ! {io} {
  c ≔ root_clock();
  f: ƒ(&Cap<Clock>) → u64 ! {io} ≔ Cap::now;
  f(&c)
}
"#,
        "E0313",
    );
    only(
        "cap_alloc_arena",
        r#"ƒ main() ! {mem} {
  c ≔ root_alloc();
  f: ƒ(&Cap<Alloc>) → Arena ! {mem} ≔ Cap::arena;
  a ≔ f(&c);
}
"#,
        "E0313",
    );
    only(
        "host_fs",
        r#"ƒ main() ! {io} {
  f: ƒ() → Cap<Fs> ! {io} ≔ Host::fs;
  c ≔ f();
}
"#,
        "E0313",
    );
    only(
        "file_len",
        r#"ƒ main() ! {io} {
  fs ≔ root_fs();
  fl ≔ fs.read("/nonexistent");
  b: ƒ(&File) → usize ! ∅ ≔ File::len;
  x ≔ b(&fl);
}
"#,
        "E0313",
    );
    only(
        "file_bytes",
        r#"ƒ main() ! {io} {
  fs ≔ root_fs();
  fl ≔ fs.read("/nonexistent");
  b: ƒ(&File) → &[u8] ! ∅ ≔ File::bytes;
  x ≔ b(&fl);
}
"#,
        "E0313",
    );
    only(
        "buf_mk",
        r#"ƒ main() ! ∅ {
  f: ƒ() → Buf ! ∅ ≔ mk_buf;
  b ≔ f();
}
"#,
        "E0313",
    );
    only(
        "arena_new",
        r#"ƒ main() ! {mem} {
  f: ƒ() → Arena ! {mem} ≔ Arena::new;
  a ≔ f();
}
"#,
        "E0313",
    );
    only(
        "text_push_char",
        r#"ƒ main() ! {mem} {
  μa ≔ Arena::new();
  μt ≔ Text::new(&!a);
  p: ƒ(&!Text, char) ! {mem} ≔ Text::push_char;
  p(&!t, 'x');
}
"#,
        "E0313",
    );
    only(
        "vec_reserve_huge",
        r#"ƒ main() ! {mem} {
  μa ≔ Arena::new();
  μv ≔ Vec::new(&!a);
  r: ƒ(&!Vec<u64>, usize) ! {mem} ≔ Vec::reserve;
  r(&!v, 1152921504606846976usize);
}
"#,
        "E0313",
    );
}

#[test]
fn direct_calls_method_syntax_and_user_functions_as_values_still_work() {
    // gọi TRỰC TIẾP intrinsic (đường dẫn và phương thức)
    ok("ƒ main() → u32 ! ∅ { shl(1u32, 3u32) }");
    ok("ƒ main() → u32 ! ∅ { a ≔ Atomic::new(5u32); Atomic::load(&a, AtomicOrdering::Acquire) }");
    ok("ƒ main() → u32 ! ∅ { c ≔ Cell::new(2u32); c.get() }");
    // hàm viết bằng Grit (thường, kể cả kết hợp) dùng làm giá trị
    ok("ƒ inc(x: u32) → u32 ! ∅ { x +% 1 }
ƒ main() → u32 ! ∅ { f ≔ inc; f(1u32) }");
    ok("▢ B { v: u32 }
⊐ B {
  ƒ get(&self) → u32 ! ∅ { self.v }
}
ƒ main() → u32 ! ∅ {
  b ≔ B { v: 3u32 };
  f: ƒ(&B) → u32 ! ∅ ≔ B::get;
  f(&b)
}
");
    // hàm thường bọc intrinsic
    ok("ƒ my_shl(x: u32) → u32 ! ∅ { shl(x, 2u32) }
ƒ main() → u32 ! ∅ { f ≔ my_shl; f(1u32) }");
}
