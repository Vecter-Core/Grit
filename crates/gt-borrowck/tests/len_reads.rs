//! `Len` của MẢNG có kích thước biết lúc biên dịch không đọc chỗ; `Len` của lát vẫn là một lần đọc (đọc siêu dữ liệu).
mod common;
use common::*;

fn codes(src: &str) -> Vec<String> {
    let mut c = bc_codes(&bc(src));
    c.sort();
    c
}

#[test]
fn array_length_is_a_compile_time_constant_and_reads_nothing() {
    // `&!a` còn sống rồi `&!a[0]`: chỉ xung đột `&!`/`&!` (E0402); kiểm chỉ số không thêm lần đọc nào.
    let c = codes("ƒ f() ! ∅ { μa ≔ [1, 2]; p ≔ &!a; q ≔ &!a[0]; *q ← 1; p[1] ← 2; }");
    assert_eq!(c, vec!["E0402"], "{c:?}");
    // vòng lặp qua `&mảng` (Len của mảng) khi có `&!` khác còn sống vẫn chỉ báo xung đột thật
    good("ƒ f() → i32 ! ∅ { μa ≔ [1, 2]; μt ≔ 0; ⟳ i in 0..2 { t ← t +% a[i as usize]; } t }");
}

#[test]
fn slice_length_is_still_a_read() {
    // cùng hình dạng nhưng trên lát: `Len(*s)` đọc siêu dữ liệu của `*s` khi reborrow `&!*s` còn sống ⇒ thêm E0403
    let c = codes("ƒ f(s: &![i32]) ! ∅ { p ≔ &!*s; q ≔ &!s[0]; *q ← 1; p[1] ← 2; }");
    assert_eq!(c, vec!["E0402", "E0403"], "{c:?}");
    // đọc độ dài lát qua gốc khi `&!` reborrow còn dùng: lỗi
    bad(
        "ƒ f(s: &![i32]) → usize ! ∅ { p ≔ &!*s; n ≔ s.len(); p[0] ← 1; n }",
        "E0403",
        1,
    );
    // và hợp lệ khi reborrow đã hết
    good("ƒ f(s: &![i32]) → usize ! ∅ { p ≔ &!*s; p[0] ← 1; s.len() }");
}
