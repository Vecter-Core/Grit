//! FAIL_OPEN_AUDIT B2/B3: các nhánh "thiếu dữ liệu" của `regions.rs` mà Phase 1 (M4/M5) KHÔNG BAO GIỜ sinh ra. Dựng bằng cách SỬA một thân MIR
//! thật (xoá slot region, đổi kiểu thành ADT không có trong cơ sở kiểu, chèn phép chiếu không khớp kiểu) rồi gọi `check_body` công khai.
//! Yêu cầu: thân nguyên vẹn KHÔNG báo gì (đối chứng), thân bị sửa PHẢI ra ICE E9901 (không được im lặng chấp nhận).
//! Hoàn nguyên bản sửa (B2/B3 cũ) ⇒ các test này ĐỎ — xem FIELD_BUGS FB-12.
mod common;
use gt_mir::{Body, Local, Operand, PlaceElem, Rvalue, StatementKind};
use gt_resolve::DefId;
use gt_types::Ty;

fn body_of(r: &common::Run, name: &str) -> Body {
    r.m.bodies
        .iter()
        .find(|b| b.key.ends_with(name))
        .unwrap_or_else(|| {
            panic!(
                "không thấy thân {name}: {:?}",
                r.m.bodies.iter().map(|b| &b.key).collect::<Vec<_>>()
            )
        })
        .clone()
}

fn codes(r: &common::Run, b: &Body) -> Vec<String> {
    gt_borrowck::check_body(&r.t, b)
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect()
}

fn ice(r: &common::Run, b: &Body) -> bool {
    codes(r, b).iter().any(|c| c == "E9901")
}

fn unknown_adt() -> Ty {
    Ty::Adt(DefId::from_index(1_000_000), Vec::new())
}

/// B2: `Deref` của một tham chiếu mà local không còn slot region nào ⇒ bó rỗng ⇒ thiếu ràng buộc reborrow.
#[test]
fn deref_of_reference_without_a_region_slot_is_an_ice() {
    let r = common::bc("ƒ f(x: &u32) → u32 ! ∅ { *x }");
    let b = body_of(&r, "f");
    assert!(
        codes(&r, &b).is_empty(),
        "đối chứng: thân nguyên vẹn phải sạch"
    );
    let mut bad = b.clone();
    assert!(
        matches!(bad.locals[1].ty, Ty::Ref(..)),
        "{:?}",
        bad.locals[1].ty
    );
    bad.locals[1].regions.clear();
    assert!(ice(&r, &bad), "{:?}", codes(&r, &bad));
}

/// B2: tổ hợp (phép chiếu, kiểu) lạ — `Field` trên một số nguyên — từng bị bỏ qua ⇒ slot sai lặng lẽ.
#[test]
fn projection_that_does_not_match_the_type_is_an_ice() {
    let r = common::bc("ƒ f(x: u32) → u32 ! ∅ { x }");
    let b = body_of(&r, "f");
    assert!(codes(&r, &b).is_empty(), "đối chứng");
    let mut bad = b.clone();
    let fty = bad.locals[1].ty.clone();
    let mut done = false;
    for bl in &mut bad.blocks {
        for s in &mut bl.statements {
            if let StatementKind::Assign(p, Rvalue::Use(Operand::Copy(_) | Operand::Move(_))) =
                &mut s.kind
                && p.local == Local(0)
                && !done
            {
                p.projection.push(PlaceElem::Field(0, fty.clone()));
                done = true;
            }
        }
    }
    assert!(done, "không có phép gán vào _0 để sửa");
    assert!(ice(&r, &bad), "{:?}", codes(&r, &bad));
}

/// B3: ADT không có trong cơ sở kiểu (số tham số lifetime không tra được) từng được coi là 0 ⇒ không ràng buộc.
#[test]
fn unknown_adt_in_assignment_is_an_ice() {
    let r = common::bc("ƒ f(x: u32) → u32 ! ∅ { x }");
    let b = body_of(&r, "f");
    assert!(codes(&r, &b).is_empty(), "đối chứng");
    let mut bad = b.clone();
    bad.locals[0].ty = unknown_adt();
    bad.locals[1].ty = unknown_adt();
    assert!(ice(&r, &bad), "{:?}", codes(&r, &bad));
}

/// B3: ADT lạ nằm BÊN TRONG `&!` — đường `adt_invariant` (từng trả "không bất biến" ⇒ nới).
#[test]
fn unknown_adt_behind_a_mutable_reference_is_an_ice() {
    let r = common::bc("ƒ f(x: &!u32) → u32 ! ∅ { y ≔ x; 0u32 }");
    let b = body_of(&r, "f");
    assert!(codes(&r, &b).is_empty(), "đối chứng");
    let mut bad = b.clone();
    for l in &mut bad.locals {
        if let Ty::Ref(_, inner) = &mut l.ty {
            **inner = unknown_adt();
        }
    }
    assert!(ice(&r, &bad), "{:?}", codes(&r, &bad));
}
