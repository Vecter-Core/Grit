//! FAIL_OPEN_AUDIT S-B (FB-14, lỗi THẬT đạt tới được từ Grit): `⌥` trên số nguyên 128 bit. Trước đây literal > `i128::MAX` bị KẸP hoặc coi là "không giới hạn" và miền `u128`
//! bị cắt ở `i128::MAX`, nên `⌥` thiếu nhánh trên `u128` được coi là vét cạn (chạy ra `abort: unreachable`). Nay: literal không bao giờ bị kẹp; `u128` có miền "vô hạn"
//! (chỉ nhánh `_`/tên phủ được ⇒ E0305 kèm gợi ý); `i128` có miền chính xác.
mod common;
use common::{err, ok};

fn m(ty: &str, arms: &str) -> String {
    format!("ƒ f(x: {ty}) → u32 ! ∅ {{ ⌥ x {{ {arms} }} }}")
}

#[test]
fn u128_match_missing_values_above_i128_max_is_not_exhaustive() {
    // dừng đúng ở i128::MAX: còn thiếu mọi giá trị lớn hơn
    err(
        &m("u128", "0..=170141183460469231731687303715884105727 ↦ 1u32"),
        "E0305",
        1,
    );
    // dừng ở 2·10^38 (> i128::MAX nhưng < u128::MAX)
    err(
        &m("u128", "0..=200000000000000000000000000000000000000 ↦ 1u32"),
        "E0305",
        1,
    );
    // hai nhánh sát nhau quanh i128::MAX, vẫn thiếu phía trên
    err(
        &m(
            "u128",
            "0..=170141183460469231731687303715884105726 ↦ 1u32, 170141183460469231731687303715884105727 ↦ 2u32",
        ),
        "E0305",
        1,
    );
    // thiếu nhánh thường
    err(&m("u128", "0..=5 ↦ 1u32"), "E0305", 1);
}

#[test]
fn u128_match_with_a_catch_all_is_fine_even_with_huge_literals() {
    ok(&m(
        "u128",
        "340282366920938463463374607431768211455 ↦ 1u32, _ ↦ 2u32",
    ));
    ok(&m(
        "u128",
        "200000000000000000000000000000000000000 ↦ 1u32, n ↦ 2u32",
    ));
}

#[test]
fn literal_beyond_u128_is_an_error_not_a_clamp() {
    err(
        &m(
            "u128",
            "340282366920938463463374607431768211456 ↦ 1u32, _ ↦ 2u32",
        ),
        "E0301",
        1,
    );
}

#[test]
fn i128_match_uses_exact_bounds_including_the_minimum() {
    // phủ kín bằng hai khoảng chính xác
    ok(&m(
        "i128",
        "-170141183460469231731687303715884105728..=0 ↦ 1u32, 1.. ↦ 2u32",
    ));
    // `..i128::MIN` (loại trừ) KHÔNG phủ gì cả: phải thiếu
    err(
        &m("i128", "..-170141183460469231731687303715884105728 ↦ 1u32"),
        "E0305",
        1,
    );
    // chỉ nửa dưới
    err(
        &m(
            "i128",
            "-170141183460469231731687303715884105728..=0 ↦ 1u32",
        ),
        "E0305",
        1,
    );
    // literal i128::MIN đứng riêng không đủ
    err(
        &m("i128", "-170141183460469231731687303715884105728 ↦ 1u32"),
        "E0305",
        1,
    );
}
