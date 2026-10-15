//! THĂM DÒ codegen cho ADR-040 (Q87) TRƯỚC khi viết gt-types: dựng tay MIR có chuỗi `Deref → Downcast → Field → Ref` bằng cách VÁ MIR của chương trình
//! thật (khớp mẫu theo giá trị trên `*r` với trường Copy): biến bind `x` đổi kiểu thành `&V`/`&!V`, phép gán bind thành `Rvalue::Ref` trên đúng vị trí
//! `(*r) as V . f`, mọi chỗ dùng `x` thêm `Deref` ở đầu. Chạy thông dịch (MIR đã vá) và exe LLVM (cùng MIR đã vá) rồi so với kết quả CHƯA vá.
#![cfg(feature = "llvm")]
#[path = "support/refpatch.rs"]
mod refpatch;
use refpatch::*;

const ENUMS: &str = "⊔ E { A(u32), B(u32, u32), C, D(u32, u32, u32) }\n";

#[test]
fn deref_downcast_field_ref_one_and_many_binds_shared() {
    let src = format!(
        "{ENUMS}ƒ f(r: &E) → u32 ! ∅ {{ ⌥ *r {{ E::A(x) ↦ x + 1, E::B(x, y) ↦ x * 10 + y, E::C ↦ 7, E::D(x, y, z) ↦ x * 100 + y * 10 + z }} }}
ƒ main() → u32 ! ∅ {{ f(&E::A(5)) + f(&E::B(3, 4)) * 2 + f(&E::C) + f(&E::D(1, 2, 3)) }}\n"
    );
    let (b, i, l, n) = probe(&src, "main::f", &["x", "y", "z"], false, false);
    println!("shared: gốc={b} đã vá thông dịch={i} đã vá llvm={l} số bind vá={n}");
    assert_eq!(n, 1 + 2 + 3);
    assert_eq!(b, i, "MIR đã vá khác MIR gốc (thông dịch)");
    assert_eq!(i, l, "thông dịch ≠ llvm");
}

#[test]
fn deref_downcast_field_ref_unique_with_write_through() {
    // `&!E`: ghi qua tham chiếu duy nhất ⇒ giá trị gốc đổi (đọc lại sau cuộc gọi)
    let src = format!(
        "{ENUMS}ƒ g(r: &!E) → u32 ! ∅ {{ ⌥ *r {{ E::A(x) ↦ x, E::B(x, y) ↦ y, E::C ↦ 0, E::D(x, y, z) ↦ z }} }}
ƒ h(r: &E) → u32 ! ∅ {{ ⌥ *r {{ E::A(x) ↦ x, E::B(x, y) ↦ x + y, E::C ↦ 0, E::D(x, y, z) ↦ x + y + z }} }}
ƒ main() → u32 ! ∅ {{ μv ≔ E::B(3, 4); a ≔ g(&!v); b ≔ h(&v); a * 1000 + b }}\n"
    );
    let (b, i, l, n) = probe(&src, "main::g", &["y", "x", "z"], true, true);
    println!("unique+ghi: gốc={b} đã vá thông dịch={i} đã vá llvm={l} số bind vá={n}");
    assert!(n >= 4);
    // gốc: g(&!v)=4, h(&v)=7 ⇒ 4007. Đã vá có ghi: bind `y` (trường 1) +100 ⇒ g đọc 104 rồi h đọc 3+104=107 ⇒ 104107... kết quả thật in ra ở trên;
    // yêu cầu: thông dịch ≡ llvm
    assert_eq!(i, l, "thông dịch ≠ llvm");
    assert_ne!(b, i, "phép ghi qua &! phải làm giá trị gốc đổi");
}

#[test]
fn deref_downcast_field_ref_slice_field() {
    let src = "⊔ S<'a> { N(&'a [u32]), M(u32, &'a [u32]) }
ƒ f<'a>(r: &S<'a>) → u32 ! ∅ { ⌥ *r { S::N(s) ↦ s[0] + s[1], S::M(k, s) ↦ k + s[2] } }
ƒ main() → u32 ! ∅ { a ≔ [10u32, 20, 30]; f(&S::N(&a[..])) + f(&S::M(5, &a[..])) }\n";
    let (b, i, l, n) = probe(src, "main::f", &["s", "k"], false, false);
    println!("lát: gốc={b} đã vá={i} llvm={l} n={n}");
    assert_eq!(n, 3);
    assert_eq!(b, i);
    assert_eq!(i, l);
    assert!(l.contains("65"), "{l} / {b}");
}

#[test]
fn deref_field_ref_struct_and_tuple_without_downcast() {
    let src = "▢ P { x: u32, y: u32 }
ƒ f(p: &P) → u32 ! ∅ { ⌥ *p { P { x, y } ↦ x * 10 + y } }
ƒ t(p: &(u32, u32)) → u32 ! ∅ { ⌥ *p { (a, b) ↦ a * 10 + b } }
ƒ main() → u32 ! ∅ { f(&P { x: 4, y: 2 }) + t(&(1, 3)) }\n";
    let (b, i, l, _) = probe(src, "main::f", &["x", "y"], false, false);
    assert_eq!(b, i);
    assert!(l.contains("55"), "{l}");
    let (b, i, l, _) = probe(src, "main::t", &["a", "b"], false, false);
    assert_eq!(b, i);
    assert!(l.contains("55"), "{l}");
}
