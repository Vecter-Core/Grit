//! FAIL_OPEN_AUDIT (needs_drop): ADT không có trong db khi hạ MIR phải là ICE (E9901), không phải "không cần drop". Kiểm: test này XANH ngay cả TRƯỚC khi sửa `needs_drop`
//! (đường tra cứu ADT khác của gt-mir đã ICE) ⇒ nó khoá hành vi, KHÔNG chứng minh riêng `needs_drop`; phần `needs_drop` được test ở gt-types/db.rs.
mod common;
use common::*;

#[test]
fn local_of_an_adt_missing_from_the_database_is_an_ice_not_a_silent_no_drop() {
    let src = "▢ B { v: i32 }\nƒ mk() → B ! ∅ { B { v: 0 } }\nƒ f() ! ∅ { b ≔ mk(); }";
    let mut t = ok(src);
    // đối chứng: nguyên vẹn hạ sạch
    let m = gt_mir::lower_program(&t);
    assert!(m.report.diagnostics.is_empty(), "{:?}", mir_codes(&m));
    let b = *t
        .db
        .adts
        .keys()
        .find(|d| t.defs()[d.index()].path.ends_with("::B"))
        .expect("ADT B");
    t.db.adts.remove(&b);
    let m = gt_mir::lower_program(&t);
    let codes = mir_codes(&m);
    assert!(
        codes.iter().any(|c| c.starts_with("E9901")),
        "ADT lạ không gây ICE: {codes:?}"
    );
}
