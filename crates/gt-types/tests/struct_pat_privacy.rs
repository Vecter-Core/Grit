//! GÓI 7 S2.c (pat.rs struct_pat): pattern struct nêu trường RIÊNG TƯ của struct ở module khác phải bị E0203; trường `◈` (công khai) thì được. Thay cho mẫu corpus
//! `tests/fail/E0203_*` (tiền tố E0203 bị `gt-resolve` coi là mã của resolver; xem docs/DELETIONS.md).
mod common;
use common::*;

#[test]
fn struct_pattern_naming_a_private_field_of_another_module_is_rejected() {
    err(
        "▦ m { ◈ ▢ S { x: i32 } ◈ ƒ mk() → S ! ∅ { S { x: 1 } } } ƒ f() → i32 ! ∅ { s ≔ m::mk(); ⌥ s { m::S { x } ↦ x } }",
        "E0203",
        1,
    );
}

#[test]
fn struct_pattern_naming_a_public_field_is_accepted() {
    ok(
        "▦ m { ◈ ▢ S { ◈ x: i32 } ◈ ƒ mk() → S ! ∅ { S { x: 1 } } } ƒ f() → i32 ! ∅ { s ≔ m::mk(); ⌥ s { m::S { x } ↦ x } }",
    );
}
