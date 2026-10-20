//! Nhiều toán hạng của CÙNG MỘT câu lệnh: move của toán hạng đứng trước phải được thấy khi kiểm toán hạng đứng sau (bộ kiểm toán phát hiện
//! `(b, b)`, `two(b, b)`, `[b, b]`… từng lọt XANH ⇒ nhân đôi `&!`/`Cap`/tay cầm Arena). Mỗi dạng toán hạng × mỗi loại giá trị không-Copy.
mod common;
use common::*;

const DECLS: &str = "▢ B { v: i32 }\n▢ P { a: B, b: B }\nƒ two<T>(a: T, b: T) ! ∅ { }\nƒ mixed(a: B, n: i32) ! ∅ { }\n";

/// `(tên kiểu, giá trị lấy từ tham số x)`: kiểu không-Copy cần kiểm (struct, tham chiếu khả biến, tham số kiểu).
fn affine_params() -> Vec<(&'static str, &'static str)> {
    vec![("B", "B"), ("&!i32", "&!i32"), ("T", "T")]
}

#[test]
fn moving_the_same_value_twice_in_one_statement_is_rejected() {
    // (dạng, mẫu thân hàm dùng `x`: kiểu `$`)
    let forms = [
        "ƒ f$G(x: $) ! ∅ { t ≔ (x, x); }",
        "ƒ f$G(x: $) ! ∅ { t ≔ [x, x]; }",
        "ƒ f$G(x: $) ! ∅ { two(x, x); }",
        "ƒ f$G(x: $) → ($, $) ! ∅ { (x, x) }",
    ];
    for (ty, _) in affine_params() {
        for form in forms {
            let g = if ty == "T" { "<T>" } else { "" };
            let src = format!("{DECLS}{}", form.replace("$G", g).replace('$', ty));
            // tham chiếu khả biến: cùng một vi phạm có thể hiện là "dùng sau move" (E0401) hoặc "hai &! cùng lúc" (E0402)
            let r = bc(&src);
            let c = bc_codes(&r);
            assert!(
                c.len() == 1 && (c[0] == "E0401" || (ty == "&!i32" && c[0] == "E0402")),
                "{src}\n{:?}",
                bc_show(&r)
            );
        }
    }
}

#[test]
fn struct_literal_and_read_after_move_in_one_statement_are_rejected() {
    bad(
        &format!("{DECLS}ƒ f(b: B) ! ∅ {{ t ≔ P {{ a: b, b: b }}; }}"),
        "E0401",
        1,
    );
    bad(
        &format!("{DECLS}ƒ f(b: B) ! ∅ {{ mixed(b, b.v); }}"),
        "E0401",
        1,
    );
    bad(
        &format!("{DECLS}ƒ f(b: B) ! ∅ {{ t ≔ (b, b.v); }}"),
        "E0401",
        1,
    );
}

#[test]
fn valid_twins_stay_accepted() {
    // đọc trước rồi mới move; hai giá trị khác nhau; kiểu Copy dùng hai lần
    good(&format!(
        "{DECLS}ƒ f(b: B) ! ∅ {{ mixed2(b.v, b); }}\nƒ mixed2(n: i32, b: B) ! ∅ {{ }}"
    ));
    good(&format!(
        "{DECLS}ƒ f(a: B, b: B) ! ∅ {{ t ≔ (a.v, 1); two(a, b); }}"
    ));
    good(&format!("{DECLS}ƒ f(n: i32) → (i32, i32) ! ∅ {{ (n, n) }}"));
}
