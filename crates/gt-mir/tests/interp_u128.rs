//! R3 / FB-18 (2026-10-18): thông dịch MIR lưu `u128` bằng bit trong `i128` rồi so sánh/chia/kiểm tràn như SỐ CÓ DẤU ⇒ giá trị ≥ 2^127 cho kết quả SAI LẶNG LẼ
//! (`u128::MAX > 5` ⇒ false; `u128::MAX / 2` ⇒ 0; `a + a` với a = 1e38 ⇒ báo tràn oan) trong khi LLVM đúng. Kỳ vọng tính ĐỘC LẬP bằng `u128` của Rust.
mod common;
use common::*;
use gt_mir::interp::{Interp, Outcome, V};

fn run(body: &str) -> Outcome {
    let t = one(&format!("ƒ f() → bool ! ∅ {{ {body} }}"));
    assert!(t.report.diagnostics.is_empty(), "{:?}", show(&t));
    let m = gt_mir::lower_program(&t);
    assert!(m.report.diagnostics.is_empty(), "{:?}", mir_codes(&m));
    Interp::new(&m, 100_000).call_key("main::f", Vec::new())
}

const MAX: &str = "340282366920938463463374607431768211455u128";
const HALF: &str = "170141183460469231731687303715884105727u128";
const E38: &str = "100000000000000000000000000000000000000u128";
const E38X2: &str = "200000000000000000000000000000000000000u128";

#[test]
fn u128_values_above_i128_max_compare_divide_and_check_overflow_unsigned() {
    // kỳ vọng độc lập bằng u128 của Rust (cùng hằng số)
    let (m, h, e) = (u128::MAX, i128::MAX as u128, 10u128.pow(38));
    assert_eq!((m / 2, m % 10, e + e), (h, 5, 2 * 10u128.pow(38)));
    let cases: Vec<(String, bool)> = vec![
        (format!("x ≔ {MAX}; x > 5u128"), true),
        (format!("x ≔ {MAX}; x < 5u128"), false),
        (format!("x ≔ {MAX}; x >= x"), true),
        (format!("x ≔ {MAX}; 5u128 <= x"), true),
        (format!("x ≔ {MAX}; x / 2u128 == {HALF}"), true),
        (format!("x ≔ {MAX}; x % 10u128 == 5u128"), true),
        (format!("x ≔ {MAX}; (x / 3u128) * 3u128 == x"), true),
        (format!("x ≔ {MAX}; x +^ 1u128 == x"), true),
        (format!("x ≔ {MAX}; 0u128 -^ x == 0u128"), true),
        (format!("a ≔ {E38}; a + a == {E38X2}"), true),
        (
            format!("a ≔ {E38}; a * 3u128 == 300000000000000000000000000000000000000u128"),
            true,
        ),
        (format!("a ≔ {E38X2}; a - {E38} == {E38}"), true),
    ];
    for (src, want) in cases {
        assert_eq!(run(&src), Outcome::Value(V::Bool(want)), "{src}");
    }
}

#[test]
fn lossy_casts_between_u128_and_i128_are_detected_by_value() {
    // u128::MAX không vừa i128; i128 âm không vừa u128; 2^127-1 vừa i128
    let abort = |src: String| match run(&src) {
        Outcome::Abort(_) => (),
        o => panic!("{src}: kỳ vọng abort, có {o:?}"),
    };
    abort(format!("x ≔ {MAX}; y ≔ x as i128; y < 0i128"));
    abort("n ≔ 0i128 - 1i128; y ≔ n as u128; y > 0u128".to_string());
    assert_eq!(
        run(&format!("x ≔ {HALF}; y ≔ x as i128; y > 0i128")),
        Outcome::Value(V::Bool(true))
    );
    abort(format!("x ≔ {MAX}; y ≔ x as u64; y == 0u64"));
}
