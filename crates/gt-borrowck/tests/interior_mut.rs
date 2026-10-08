//! ĐIỀU KIỆN CỨNG mutability bên trong (Giang 2026-10-07; GRIT_PLAN §7 điểm 10): `Cell<T>` (lang `cell`) ghi được qua `&self`, nên mọi vị trí
//! region bên trong nó phải BẤT BIẾN kể cả khi chỉ có `&` tới nó. Các chương trình dưới đây thử "nhét" tham chiếu sống ngắn vào ô sống lâu
//! qua nhiều đường chuyển kiểu (hiệp biến giả); borrowck PHẢI từ chối. Đối chứng hợp lệ PHẢI được chấp nhận. Dùng để giết đột biến 238
//! (`ty_invariant`: `&T` hiệp biến) của `regions.rs` và kiểm "tắt bản sửa" (xem PROGRESS).
mod common;
use common::*;

const HEAD: &str = "ƒ use_i(x: i32) ! ∅ { }\nƒ idg<T>(x: T) → T ! ∅ { x }\n▢ Cl<'a, 'b> { c: &'b Cell<&'a i32> }\n";

fn prog(body: &str) -> String {
    format!("{HEAD}ƒ main() ! ∅ {{\n{body}\n}}\n")
}

#[track_caller]
fn rejected(body: &str) {
    let src = prog(body);
    let r = bc(&src);
    assert!(
        !r.bc.diagnostics.is_empty(),
        "đáng lẽ bị TỪ CHỐI (tham chiếu sống ngắn lọt vào ô sống lâu):\n{src}"
    );
}

#[track_caller]
fn accepted(body: &str) {
    let src = prog(body);
    let r = bc(&src);
    assert!(
        r.bc.diagnostics.is_empty(),
        "đáng lẽ được chấp nhận: {:?}\n{src}",
        bc_show(&r)
    );
}

#[test]
fn a_short_lived_reference_cannot_be_stored_into_a_long_lived_cell() {
    // trực tiếp qua `&Cell`
    rejected(
        "μlong ≔ 1; cell ≔ Cell::new(&long); r ≔ &cell; { short ≔ 2; r.set(&short); }; use_i(*cell.get());",
    );
    // qua ADT giữ `&Cell` (hiệp biến giả của ADT ⇒ đột biến 238)
    rejected(
        "μlong ≔ 1; cell ≔ Cell::new(&long); cl ≔ Cl { c: &cell }; { short ≔ 2; cl2 ≔ cl; cl2.c.set(&short); }; use_i(*cell.get());",
    );
    // qua hàm generic
    rejected(
        "μlong ≔ 1; cell ≔ Cell::new(&long); cl ≔ Cl { c: &cell }; { short ≔ 2; cl2 ≔ idg(cl); cl2.c.set(&short); }; use_i(*cell.get());",
    );
    // qua tuple, Option, mảng
    rejected(
        "μlong ≔ 1; cell ≔ Cell::new(&long); t ≔ (&cell, 0); { short ≔ 2; t2 ≔ t; t2.0.set(&short); }; use_i(*cell.get());",
    );
    rejected(
        "μlong ≔ 1; cell ≔ Cell::new(&long); o ≔ Some(&cell); { short ≔ 2; ⌥ o { Some(q) ↦ { q.set(&short); }, None ↦ { } }; }; use_i(*cell.get());",
    );
    rejected(
        "μlong ≔ 1; cell ≔ Cell::new(&long); a ≔ [&cell, &cell]; { short ≔ 2; a[0].set(&short); }; use_i(*cell.get());",
    );
    // ghi đè bằng giá trị sống lâu SAU đó cũng không cứu được: kiểu của ô buộc mọi tham chiếu lưu vào sống tới lần dùng cuối của ô
    rejected(
        "μlong ≔ 1; cell ≔ Cell::new(&long); { short ≔ 2; cell.set(&short); cell.set(&long); }; use_i(*cell.get());",
    );
    // `&&Cell`
    rejected(
        "μlong ≔ 1; cell ≔ Cell::new(&long); r ≔ &cell; rr ≔ &r; { short ≔ 2; rr.set(&short); }; use_i(*cell.get());",
    );
}

#[test]
fn same_lifetime_stores_into_a_cell_are_accepted() {
    accepted("μa ≔ 1; μb ≔ 2; cell ≔ Cell::new(&a); cell.set(&b); use_i(*cell.get());");
    accepted(
        "μa ≔ 1; μb ≔ 2; cell ≔ Cell::new(&a); cl ≔ Cl { c: &cell }; cl.c.set(&b); use_i(*cell.get());",
    );
    accepted(
        "μa ≔ 1; μb ≔ 2; cell ≔ Cell::new(&a); t ≔ (&cell, 0); t.0.set(&b); use_i(*cell.get());",
    );
    // ô cũng sống ngắn như giá trị lưu vào: không treo
    accepted(
        "μlong ≔ 1; { short ≔ 2; cell ≔ Cell::new(&long); cell.set(&short); use_i(*cell.get()); };",
    );
}
