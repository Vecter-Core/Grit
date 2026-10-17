//! R3 / FB-17: `⊐ Copy for` phải khai báo `T: Copy` cho MỌI tham số xuất hiện trong trường (ma trận hình dạng trường × có/không ràng buộc).
//! Trước bản sửa, `⊐ <T> Copy for W<T>` (không ràng buộc) được chấp nhận vì trường được kiểm bằng biến suy luận ⇒ `W<Cap<…>>`/`W<Arena>` thành Copy.
mod common;
use common::*;

const SHAPES: &[&str] = &[
    "T",
    "(T, u8)",
    "(u8, (T,))",
    "[T; 2]",
    "Option<T>",
    "Option<(T, u8)>",
];
/// hình dạng mà `T: Copy` của impl ĐƯỢC công nhận (không đi qua impl của ADT khác)
const SHAPES_DIRECT: &[&str] = &["T", "(T, u8)", "(u8, (T,))", "[T; 2]"];

#[test]
fn unbounded_copy_impl_is_rejected_for_every_field_shape() {
    for sh in SHAPES {
        err(
            &format!("▢ W<T> {{ v: {sh} }} ⊐ <T> Copy for W<T> {{ }} ƒ main() ! ∅ {{ }}"),
            "E0311",
            1,
        );
        err(
            &format!("⊔ E<T> {{ A({sh}), N }} ⊐ <T> Copy for E<T> {{ }} ƒ main() ! ∅ {{ }}"),
            "E0311",
            1,
        );
    }
}

#[test]
fn bounded_copy_impl_is_accepted_for_direct_field_shapes() {
    for sh in SHAPES_DIRECT {
        ok(&format!(
            "▢ W<T> {{ v: {sh} }} ⊐ <T: Copy> Copy for W<T> {{ }} ƒ main() ! ∅ {{ }}"
        ));
        ok(&format!(
            "⊔ E<T> {{ A({sh}), N }} ⊐ <T: Copy> Copy for E<T> {{ }} ƒ main() ! ∅ {{ }}"
        ));
    }
}

#[test]
fn unused_parameter_needs_no_bound_and_concrete_arguments_are_checked() {
    ok("▢ M<T> { n: u8 } ⊐ <T> Copy for M<T> { } ƒ main() ! ∅ { }");
    ok("▢ W<T> { v: T } ⊐ Copy for W<u8> { } ƒ main() ! ∅ { }");
    err(
        "▢ W<T> { v: T } ⊐ Copy for W<Arena> { } ƒ main() ! ∅ { }",
        "E0311",
        1,
    );
}

#[test]
fn known_incompleteness_bound_is_not_propagated_through_another_adts_copy_impl() {
    // KHÔNG hoàn chỉnh (không phải mất an toàn): `find_impl` kiểm ràng buộc của impl ADT khác với scope RỖNG ⇒ `Option<T>` với `T: Copy` của impl
    // vẫn bị coi là không Copy ⇒ bị từ chối oan. Ghi trong KNOWN_LIMITATIONS; đổi thành `ok` khi `find_impl` nhận scope.
    err(
        "▢ W<T> { v: Option<T> } ⊐ <T: Copy> Copy for W<T> { } ƒ main() ! ∅ { }",
        "E0311",
        1,
    );
}
