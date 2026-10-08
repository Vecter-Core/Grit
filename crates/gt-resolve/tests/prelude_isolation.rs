//! M3-fix: mã của PRELUDE chỉ thấy prelude. Tên tệp/module của người dùng (vd `x.gt`) KHÔNG được bị D7 so với tham số trong chữ ký
//! prelude (E0205 oan); trong khi D7 vẫn áp dụng đầy đủ cho mã người dùng. Gốc lỗi: tra tên cấp module luôn gồm gốc người dùng, kể cả khi
//! đang duyệt prelude (phát hiện khi thêm `spawn<A>(f, a: A)` và gặp tệp `a.gt` trong test gt-types).
mod common;
use common::*;

#[test]
fn user_file_and_module_names_do_not_clash_with_prelude_parameters() {
    // prelude có tham số `x`, `n` (shl/shr) và `p` (raw_read)
    assert_ok(&[("x.gt", "ƒ g() ! ∅ { }")]);
    assert_ok(&[("n.gt", "ƒ g() ! ∅ { }")]);
    assert_ok(&[("p.gt", "ƒ g() ! ∅ { }")]);
    assert_ok(&[("main.gt", "▦ x { ◈ ƒ g() ! ∅ { } }")]);
    assert_ok(&[("main.gt", "ƒ n() ! ∅ { }")]);
    assert_ok(&[
        ("main.gt", "▦ p { } ƒ f() ! ∅ { }"),
        ("x.gt", "ƒ h() ! ∅ { }"),
    ]);
}

#[test]
fn shadowing_is_still_forbidden_in_user_code() {
    // tham số của người dùng che tên prelude hoặc tên module của người dùng
    assert_err(&[("main.gt", "ƒ f(shl: i32) ! ∅ { }")], "E0205", 1);
    assert_err(&[("x.gt", "ƒ f(x: i32) ! ∅ { }")], "E0205", 1);
    assert_err(&[("main.gt", "▦ m { } ƒ f(m: i32) ! ∅ { }")], "E0205", 1);
    assert_err(&[("main.gt", "ƒ f(raw_read: i32) ! ∅ { }")], "E0205", 1);
}
