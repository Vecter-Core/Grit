//! Q87 (ADR-040) bước 2–3: khớp mẫu qua tham chiếu bằng gt-types + hạ MIR THẬT, so đáp số với bộ chuẩn của thăm dò (`ref_pattern_probe.rs`:
//! 204, 65, 104207, 55) ở CẢ thông dịch và exe LLVM, và đối chiếu ĐỘC LẬP cấu trúc khoản vay của MIR sinh từ mẫu với MIR vá tay / chương trình `&(*r).f` viết tay.
#![cfg(feature = "llvm")]
#[path = "support/refpatch.rs"]
mod refpatch;
use refpatch::*;

pub const ENUMS: &str = "⊔ E { A(u32), B(u32, u32), C, D(u32, u32, u32) }\n";

pub fn enum_shared() -> String {
    format!(
        "{ENUMS}ƒ f(r: &E) → u32 ! ∅ {{ ⌥ r {{ E::A(x) ↦ *x + 1, E::B(x, y) ↦ *x * 10 + *y, E::C ↦ 7, E::D(x, y, z) ↦ *x * 100 + *y * 10 + *z }} }}
ƒ main() → u32 ! ∅ {{ f(&E::A(5)) + f(&E::B(3, 4)) * 2 + f(&E::C) + f(&E::D(1, 2, 3)) }}\n"
    )
}
pub fn enum_unique() -> String {
    format!(
        "{ENUMS}ƒ g(r: &!E) → u32 ! ∅ {{ ⌥ r {{ E::A(x) ↦ {{ *x ← *x +% 100; *x }}, E::B(x, y) ↦ {{ *x ← *x +% 100; *y ← *y +% 100; *y }}, E::C ↦ 0, E::D(x, y, z) ↦ {{ *x ← *x +% 100; *y ← *y +% 100; *z ← *z +% 100; *z }} }} }}
ƒ h(r: &E) → u32 ! ∅ {{ ⌥ r {{ E::A(x) ↦ *x, E::B(x, y) ↦ *x + *y, E::C ↦ 0, E::D(x, y, z) ↦ *x + *y + *z }} }}
ƒ main() → u32 ! ∅ {{ μv ≔ E::B(3, 4); a ≔ g(&!v); b ≔ h(&v); a * 1000 + b }}\n"
    )
}
pub fn slice_src() -> String {
    "⊔ S<'a> { N(&'a [u32]), M(u32, &'a [u32]) }
ƒ f<'a>(r: &S<'a>) → u32 ! ∅ { ⌥ r { S::N(s) ↦ (*s)[0] + (*s)[1], S::M(k, s) ↦ *k + (*s)[2] } }
ƒ main() → u32 ! ∅ { a ≔ [10u32, 20, 30]; f(&S::N(&a[..])) + f(&S::M(5, &a[..])) }\n"
        .to_string()
}
pub fn struct_tuple() -> String {
    "▢ P { x: u32, y: u32 }
ƒ f(p: &P) → u32 ! ∅ { ⌥ p { P { x, y } ↦ *x * 10 + *y } }
ƒ t(p: &(u32, u32)) → u32 ! ∅ { ⌥ p { (a, b) ↦ *a * 10 + *b } }
ƒ main() → u32 ! ∅ { f(&P { x: 4, y: 2 }) + t(&(1, 3)) }\n"
        .to_string()
}

#[test]
fn real_lowering_gives_the_probe_answers_on_interpreter_and_llvm() {
    for (src, want) in [
        (enum_shared(), "204"),
        (enum_unique(), "104207"),
        (slice_src(), "65"),
        (struct_tuple(), "55"),
    ] {
        let (i, l) = run_both(&src);
        println!("{want}: thông dịch={i} llvm={l}");
        assert_eq!(i, l, "{src}");
        assert!(i.contains(want), "kỳ vọng {want}, nhận {i}\n{src}");
    }
}

/// ĐỐI CHIẾU ĐỘC LẬP (bước 3): khoản vay của MIR sinh từ MẪU ≡ khoản vay của MIR vá tay (enum/lát: không viết tay được vì `Downcast` không có trong
/// ngôn ngữ nguồn) về nguồn, loại, điểm sinh, điểm chết, số chỗ dùng.
#[test]
fn enum_and_slice_borrows_equal_the_hand_patched_mir() {
    let by_value_shared = format!(
        "{ENUMS}ƒ f(r: &E) → u32 ! ∅ {{ ⌥ *r {{ E::A(x) ↦ x + 1, E::B(x, y) ↦ x * 10 + y, E::C ↦ 7, E::D(x, y, z) ↦ x * 100 + y * 10 + z }} }}
ƒ main() → u32 ! ∅ {{ f(&E::A(5)) }}\n"
    );
    let real = sigs_of(&enum_shared(), "main::f");
    let hand = sigs_of_patched(&by_value_shared, "main::f", &["x", "y", "z"], false, false);
    println!("enum shared: {real:?}");
    assert_eq!(real.len(), 6);
    assert_eq!(real, hand);
    let by_value_slice = "⊔ S<'a> { N(&'a [u32]), M(u32, &'a [u32]) }
ƒ f<'a>(r: &S<'a>) → u32 ! ∅ { ⌥ *r { S::N(s) ↦ s[0] + s[1], S::M(k, s) ↦ k + s[2] } }
ƒ main() → u32 ! ∅ { a ≔ [10u32, 20, 30]; f(&S::N(&a[..])) }\n";
    let real = sigs_of(&slice_src(), "main::f");
    let hand = sigs_of_patched(by_value_slice, "main::f", &["s", "k"], false, false);
    println!("slice: {real:?}");
    assert_eq!(real.len(), 3);
    assert_eq!(real, hand);
    // &! (có ghi): bản tay do `patch` chèn thêm câu lệnh ghi nên vị trí lệch; so nguồn + loại + số khoản vay
    let real = sigs_of(&enum_unique(), "main::g");
    let by_value_unique = format!(
        "{ENUMS}ƒ g(r: &!E) → u32 ! ∅ {{ ⌥ *r {{ E::A(x) ↦ x, E::B(x, y) ↦ y, E::C ↦ 0, E::D(x, y, z) ↦ z }} }}
ƒ main() → u32 ! ∅ {{ μv ≔ E::B(3, 4); g(&!v) }}\n"
    );
    let hand = sigs_of_patched(&by_value_unique, "main::g", &["x", "y", "z"], true, true);
    let key = |v: &Vec<BorrowSig>| {
        v.iter()
            .map(|s| (s.source.clone(), s.unique))
            .collect::<Vec<_>>()
    };
    println!("enum unique: {real:?}");
    assert_eq!(real.len(), 6);
    assert_eq!(key(&real), key(&hand));
    assert!(real.iter().all(|s| s.unique));
}

/// struct/tuple: `&(*r).f` VIẾT TAY ≡ mẫu về nguồn, loại, số chỗ dùng (hình dạng CFG khác nên không so toạ độ)
#[test]
fn struct_and_tuple_borrows_equal_hand_written_reborrows() {
    let proj = |v: Vec<BorrowSig>| {
        let mut x: Vec<(String, bool, usize)> = v
            .into_iter()
            .map(|s| (s.source, s.unique, s.uses))
            .collect();
        x.sort();
        x
    };
    let pat = "▢ P { x: u32, y: u32 }
ƒ f(p: &P) → u32 ! ∅ { ⌥ p { P { x, y } ↦ *x * 10 + *y } }
ƒ t(p: &(u32, u32)) → u32 ! ∅ { ⌥ p { (a, b) ↦ *a * 10 + *b } }
ƒ w(p: &!P) → u32 ! ∅ { ⌥ p { P { x, y } ↦ { *x ← 1; *y ← 2; *x + *y } } }
ƒ main() → u32 ! ∅ { 0 }\n";
    let hand = "▢ P { x: u32, y: u32 }
ƒ f(p: &P) → u32 ! ∅ { x ≔ &(*p).x; y ≔ &(*p).y; *x * 10 + *y }
ƒ t(p: &(u32, u32)) → u32 ! ∅ { a ≔ &(*p).0; b ≔ &(*p).1; *a * 10 + *b }
ƒ w(p: &!P) → u32 ! ∅ { x ≔ &!(*p).x; y ≔ &!(*p).y; *x ← 1; *y ← 2; *x + *y }
ƒ main() → u32 ! ∅ { 0 }\n";
    for k in ["main::f", "main::t", "main::w"] {
        // `x ≔ &(*p).f` viết tay hạ qua MỘT biến tạm rồi chép vào `x` (thêm đúng 1 chỗ dùng của biến giữ khoản vay); mẫu ghi thẳng vào biến bind
        let b = proj(sigs_of(hand, k))
            .into_iter()
            .map(|(s, u, n)| (s, u, n - 1))
            .collect::<Vec<_>>();
        let a = proj(sigs_of(pat, k));
        println!("{k}: {a:?}");
        assert_eq!(a.len(), 2);
        assert_eq!(a, b, "{k}");
    }
}
