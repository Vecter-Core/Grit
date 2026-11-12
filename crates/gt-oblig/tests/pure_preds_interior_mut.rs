//! Điều kiện cứng mutability bên trong (Giang 2026-10-07): vị từ trên kiểu CÓ THỂ chứa ô ghi được qua `&` (lang `cell`) KHÔNG vào
//! `pure_preds` của M7 — kết quả của chúng đổi được sau `cell.set(..)` nên không được coi là sự kiện ổn định.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;

/// khoá hàm đầy đủ → có nằm trong `pure_preds` không
fn is_pure(src: &str, key: &str) -> bool {
    let (t, m) = common::mir(src);
    let info = build_prog_info(&t, &m);
    let node = m
        .keys
        .iter()
        .find(|(_, k)| k.as_str() == key)
        .map(|(n, _)| *n)
        .unwrap_or_else(|| {
            panic!(
                "không có hàm {key}: {:?}",
                m.keys.values().collect::<Vec<_>>()
            )
        });
    info.pure_preds.contains_key(&node)
}

#[test]
fn plain_predicates_stay_pure() {
    assert!(is_pure("ƒ p(x: i32) → bool ! ∅ { x > 0 }", "main::p"));
    assert!(is_pure("ƒ p(x: &i32) → bool ! ∅ { *x > 0 }", "main::p"));
    assert!(is_pure(
        "▢ S { v: i32 }\nƒ p(x: &S) → bool ! ∅ { x.v > 0 }",
        "main::p"
    ));
}

#[test]
fn predicates_over_interior_mutable_types_are_not_pure() {
    // trực tiếp, qua tham chiếu, qua ADT, qua tuple/mảng/Option
    assert!(!is_pure(
        "ƒ p(x: &Cell<i32>) → bool ! ∅ { true }",
        "main::p"
    ));
    assert!(!is_pure("ƒ p(x: Cell<i32>) → bool ! ∅ { true }", "main::p"));
    assert!(!is_pure(
        "▢ H { c: Cell<i32> }\nƒ p(x: &H) → bool ! ∅ { true }",
        "main::p"
    ));
    assert!(!is_pure(
        "▢ H<T> { v: T }\nƒ p(x: &H<Cell<i32>>) → bool ! ∅ { true }",
        "main::p"
    ));
    assert!(!is_pure(
        "ƒ p(x: (i32, &Cell<i32>)) → bool ! ∅ { true }",
        "main::p"
    ));
    assert!(!is_pure(
        "ƒ p(x: Option<&Cell<i32>>) → bool ! ∅ { true }",
        "main::p"
    ));
    // tham số kiểu KHÔNG ràng buộc: thân không quan sát được nội dung ⇒ vẫn thuần; CÓ ràng buộc ⇒ bảo thủ
    assert!(is_pure("ƒ p<T>(x: &T) → bool ! ∅ { true }", "main::p"));
    assert!(!is_pure(
        "⊏ Tr { ƒ m(&self) → bool ! ∅; }
ƒ p<T: Tr>(x: &T) → bool ! ∅ { true }",
        "main::p"
    ));
    assert!(is_pure(
        "ƒ p(x: &Option<i32>) → bool ! ∅ { true }",
        "main::p"
    ));
    // phương thức `&self` của kiểu chứa ô
    assert!(!is_pure(
        "▢ H { c: Cell<i32> }\n⊐ H { ƒ p(&self) → bool ! ∅ { true } }",
        "main::H::p"
    ));
    assert!(is_pure(
        "▢ H { v: i32 }\n⊐ H { ƒ p(&self) → bool ! ∅ { true } }",
        "main::H::p"
    ));
}

#[test]
fn a_cell_among_several_fields_variants_and_impl_members_is_still_found() {
    // ô nằm cùng các trường khác
    assert!(!is_pure(
        "▢ H { n: i32, c: Cell<i32>, m: i32 }\nƒ p(x: &H) → bool ! ∅ { true }",
        "main::p"
    ));
    // ô ở một biến thể của enum
    assert!(!is_pure(
        "⊔ E { A(i32), B(Cell<i32>) }\nƒ p(x: &E) → bool ! ∅ { true }",
        "main::p"
    ));
    // ADT đệ quy (qua tham chiếu: đệ quy theo giá trị bị M4 chặn bằng E0301) có ô / không có ô
    assert!(!is_pure(
        "▢ N<'a> { next: Option<&'a N<'a>>, c: Cell<i32> }\nƒ p(x: &N) → bool ! ∅ { true }",
        "main::p"
    ));
    assert!(is_pure(
        "▢ N<'a> { next: Option<&'a N<'a>>, v: i32 }\nƒ p(x: &N) → bool ! ∅ { true }",
        "main::p"
    ));
    // impl nhiều thành viên: kiểu của `self` vẫn được xét
    assert!(!is_pure(
        "▢ H { c: Cell<i32> }\n⊐ H { ƒ a(&self) → i32 ! ∅ { 1 } ƒ p(&self) → bool ! ∅ { true } ƒ b(&self) → i32 ! ∅ { 2 } }",
        "main::H::p"
    ));
}
