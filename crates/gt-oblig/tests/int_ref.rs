//! `I256` đối chiếu với số học tham chiếu độc lập (`i128`/`u128` gốc của Rust) và các hằng thập phân tính ngoài (Python):
//! cộng/trừ/nhân/chia/dư/so sánh/đổi kiểu/in thập phân, đúng ở các biên 64/128/255 bit. Dấu-độ lớn (sign-magnitude) nên
//! phải kiểm riêng số 0 âm, tràn qua 128 bit, và các nhánh nhanh của phép nhân.
use gt_oblig::I256;

fn edge() -> Vec<i128> {
    let mut v = vec![
        0i128,
        1,
        -1,
        2,
        -2,
        3,
        7,
        10,
        255,
        256,
        -255,
        -256,
        i128::MAX,
        i128::MIN,
        i128::MIN + 1,
        i128::MAX - 1,
    ];
    for k in [31u32, 32, 63, 64, 65, 100, 126] {
        let p = 1i128 << k;
        v.extend([p, p - 1, p + 1, -p, -p + 1, -p - 1]);
    }
    v.sort_unstable();
    v.dedup();
    v
}

fn dec(v: i128) -> String {
    v.to_string()
}

#[test]
fn conversions_round_trip_and_reject_out_of_range() {
    for v in edge() {
        assert_eq!(I256::from_i128(v).to_i128(), Some(v), "{v}");
        assert_eq!(I256::from(v), I256::from_i128(v));
        assert_eq!(I256::from_i128(v).is_neg(), v < 0, "{v}");
        assert_eq!(I256::from_i128(v).is_zero(), v == 0);
        assert_eq!(I256::from_i128(v).to_u128(), u128::try_from(v).ok(), "{v}");
        assert_eq!(
            I256::from_i128(v).negate().to_i128(),
            v.checked_neg(),
            "{v}"
        );
        assert_eq!(I256::from_i128(v).abs().to_i128(), v.checked_abs(), "{v}");
    }
    assert_eq!(I256::from_u128(u128::MAX).to_i128(), None);
    assert_eq!(I256::from_u128(u128::MAX).to_u128(), Some(u128::MAX));
    assert_eq!(I256::from_u128(i128::MAX as u128 + 1).to_i128(), None);
    assert_eq!(
        I256::from_u128(i128::MAX as u128).to_i128(),
        Some(i128::MAX)
    );
    // -2^127 vừa i128, -2^127 - 1 thì không
    let m = I256::from_i128(i128::MIN);
    assert_eq!(m.to_i128(), Some(i128::MIN));
    assert_eq!(
        m.checked_sub(I256::from_i128(1))
            .expect("vừa 256")
            .to_i128(),
        None
    );
    // không có số 0 âm
    assert_eq!(I256::ZERO.negate(), I256::ZERO);
    assert!(!I256::ZERO.negate().is_neg());
    assert_eq!(
        I256::from_i128(5)
            .checked_sub(I256::from_i128(5))
            .expect("0"),
        I256::ZERO
    );
    assert!(
        !I256::from_i128(5)
            .checked_sub(I256::from_i128(5))
            .expect("0")
            .is_neg()
    );
    assert!(
        !I256::from_i128(-5)
            .checked_add(I256::from_i128(5))
            .expect("0")
            .is_neg()
    );
}

#[test]
fn add_sub_mul_match_i128_when_it_fits_and_stay_exact_beyond() {
    let e = edge();
    for &a in &e {
        for &b in &e {
            let (x, y) = (I256::from_i128(a), I256::from_i128(b));
            let s = x.checked_add(y).expect("256 bit đủ cho tổng hai i128");
            let d = x.checked_sub(y).expect("256 bit đủ cho hiệu hai i128");
            assert_eq!(s.to_i128(), a.checked_add(b), "{a} + {b}");
            assert_eq!(d.to_i128(), a.checked_sub(b), "{a} - {b}");
            // vòng về: (a + b) - b = a, (a - b) + b = a (kể cả khi vượt i128)
            assert_eq!(s.checked_sub(y), Some(x), "{a} + {b} - {b}");
            assert_eq!(d.checked_add(y), Some(x), "{a} - {b} + {b}");
            assert_eq!(x.cmp(&y), a.cmp(&b), "{a} ? {b}");
            assert_eq!(x < y, a < b);
            match a.checked_mul(b) {
                Some(p) => assert_eq!(
                    x.checked_mul(y).expect("vừa").to_i128(),
                    Some(p),
                    "{a} * {b}"
                ),
                None => {
                    // vượt i128 nhưng thường vẫn vừa 256 bit: (a*b)/b = a khi cả hai vừa 128 bit magnitude
                    if let Some(p) = x.checked_mul(y) {
                        assert_eq!(p.is_neg(), (a < 0) != (b < 0), "dấu {a} * {b}");
                        assert_eq!(p.to_i128(), None, "{a} * {b} không được vừa i128");
                    }
                }
            }
        }
    }
}

#[test]
fn division_and_remainder_truncate_toward_zero() {
    let e = edge();
    for &a in &e {
        for &b in &e {
            if b == 0 {
                assert_eq!(I256::from_i128(a).div_trunc(I256::from_i128(b)), None);
                assert_eq!(I256::from_i128(a).rem_trunc(I256::from_i128(b)), None);
                continue;
            }
            if a == i128::MIN && b == -1 {
                continue; // tràn i128 gốc; I256 vẫn tính được: kiểm riêng bên dưới
            }
            let (x, y) = (I256::from_i128(a), I256::from_i128(b));
            assert_eq!(
                x.div_trunc(y).and_then(I256::to_i128),
                Some(a / b),
                "{a} / {b}"
            );
            assert_eq!(
                x.rem_trunc(y).and_then(I256::to_i128),
                Some(a % b),
                "{a} % {b}"
            );
        }
    }
    let q = I256::from_i128(i128::MIN)
        .div_trunc(I256::from_i128(-1))
        .expect("2^127");
    assert_eq!(q.to_decimal(), "170141183460469231731687303715884105728");
    // số bị chia/ số chia rộng hơn 128 bit: không hỗ trợ ⇒ None (không bao giờ sai)
    let wide = I256::pow2(130).expect("2^130");
    assert_eq!(wide.div_trunc(I256::from_i128(3)), None);
    assert_eq!(wide.rem_trunc(I256::from_i128(3)), None);
    assert_eq!(I256::from_i128(3).div_trunc(wide), None);
    assert_eq!(I256::from_i128(3).rem_trunc(wide), None);
    // chỉ MỘT trong hai rộng: vẫn None (kiểm từng vế của `||`)
    assert_eq!(wide.div_trunc(wide), None);
}

#[test]
fn multiplication_fast_paths_with_a_wide_operand() {
    let w = I256::pow2(130).expect("2^130");
    let zero = I256::ZERO;
    let one = I256::from_i128(1);
    let neg1 = I256::from_i128(-1);
    let two = I256::from_i128(2);
    for (a, b) in [(w, zero), (zero, w)] {
        assert_eq!(a.checked_mul(b), Some(zero));
        assert!(!a.checked_mul(b).expect("0").is_neg());
    }
    assert_eq!(w.checked_mul(one), Some(w));
    assert_eq!(one.checked_mul(w), Some(w));
    assert_eq!(w.checked_mul(neg1), Some(w.negate()));
    assert_eq!(neg1.checked_mul(w), Some(w.negate()));
    assert_eq!(w.negate().checked_mul(neg1), Some(w));
    assert_eq!(w.checked_mul(two), None);
    assert_eq!(two.checked_mul(w), None);
    assert_eq!(w.checked_mul(w), None);
    assert_eq!(w.negate().checked_mul(w), None);
    // dấu của tích hai số hẹp
    let (a, b) = (I256::from_i128(-3), I256::from_i128(4));
    assert_eq!(a.checked_mul(b).and_then(I256::to_i128), Some(-12));
    assert_eq!(a.checked_mul(a).and_then(I256::to_i128), Some(9));
}

#[test]
fn big_products_and_powers_print_exactly() {
    let p64 = I256::pow2(64).expect("2^64");
    assert_eq!(p64.to_decimal(), "18446744073709551616");
    assert_eq!(
        p64.checked_mul(p64).expect("2^128").to_decimal(),
        "340282366920938463463374607431768211456"
    );
    let p127 = I256::pow2(127).expect("2^127");
    assert_eq!(
        p127.checked_mul(p127).expect("2^254").to_decimal(),
        "28948022309329048855892746252171976963317496166410141009864396001978282409984"
    );
    assert_eq!(
        I256::pow2(200).expect("2^200").to_decimal(),
        "1606938044258990275541962092341162602522202993782792835301376"
    );
    assert_eq!(
        I256::pow2(255).expect("2^255").to_decimal(),
        "57896044618658097711785492504343953926634992332820282019728792003956564819968"
    );
    assert_eq!(
        I256::pow2(255).expect("2^255").negate().to_decimal(),
        "-57896044618658097711785492504343953926634992332820282019728792003956564819968"
    );
    let m = I256::from_u128(u128::MAX);
    assert_eq!(
        m.checked_mul(m).expect("(2^128-1)^2").to_decimal(),
        "115792089237316195423570985008687907852589419931798687112530834793049593217025"
    );
    assert_eq!(
        m.checked_mul(I256::pow2(126).expect("2^126"))
            .expect("vừa")
            .to_decimal(),
        "28948022309329048855892746252171976963232425574679906393998552350120340357120"
    );
    assert_eq!(
        I256::pow2(127).expect("2^127").to_decimal(),
        "170141183460469231731687303715884105728"
    );
    assert_eq!(
        I256::pow2(128).expect("2^128").to_decimal(),
        "340282366920938463463374607431768211456"
    );
    assert_eq!(I256::pow2(0).expect("1").to_decimal(), "1");
    assert_eq!(I256::pow2(256), None);
    assert!(I256::pow2(255).is_some());
    assert!(I256::pow2(128).is_some());
    assert_eq!(I256::pow2(127).and_then(I256::to_i128), None);
    assert_eq!(I256::pow2(126).and_then(I256::to_i128), Some(1i128 << 126));
}

#[test]
fn decimal_printing_matches_the_native_printer() {
    for v in edge() {
        assert_eq!(I256::from_i128(v).to_decimal(), dec(v));
    }
    // biên của khối 10^19 và các luỹ thừa của 10
    for (v, s) in [
        (10_000_000_000_000_000_000u128, "10000000000000000000"),
        (9_999_999_999_999_999_999u128, "9999999999999999999"),
        (
            100_000_000_000_000_000_000_000_000_000_000_000_000u128,
            "100000000000000000000000000000000000000",
        ),
        (
            100_000_000_000_000_000_000_000_000_000_000_000_007u128,
            "100000000000000000000000000000000000007",
        ),
        (10_000_000_000_000_000_001u128, "10000000000000000001"),
        (u128::MAX, "340282366920938463463374607431768211455"),
        (1, "1"),
        (10, "10"),
        (100, "100"),
    ] {
        assert_eq!(I256::from_u128(v).to_decimal(), s);
    }
    let ten19 = I256::from_u128(10_000_000_000_000_000_000);
    assert_eq!(
        ten19
            .checked_mul(ten19)
            .and_then(|x| x.checked_mul(ten19))
            .expect("10^57")
            .to_decimal(),
        "1000000000000000000000000000000000000000000000000000000000"
    );
    assert_eq!(I256::ZERO.to_decimal(), "0");
}

#[test]
fn addition_carries_across_the_128_bit_boundary_in_both_signs() {
    let m = I256::from_u128(u128::MAX);
    let one = I256::from_i128(1);
    let up = m.checked_add(one).expect("2^128");
    assert_eq!(up, I256::pow2(128).expect("2^128"));
    assert_eq!(up.checked_sub(one), Some(m));
    let down = m.negate().checked_sub(one).expect("-2^128");
    assert_eq!(down, I256::pow2(128).expect("2^128").negate());
    assert_eq!(down.checked_add(one), Some(m.negate()));
    // trừ qua số 0 đổi dấu
    assert_eq!(
        I256::from_i128(3)
            .checked_sub(I256::from_i128(10))
            .and_then(I256::to_i128),
        Some(-7)
    );
    assert_eq!(
        I256::from_i128(-3)
            .checked_add(I256::from_i128(10))
            .and_then(I256::to_i128),
        Some(7)
    );
    assert_eq!(
        I256::from_i128(-10)
            .checked_add(I256::from_i128(3))
            .and_then(I256::to_i128),
        Some(-7)
    );
    // tràn 256 bit ⇒ None, không quấn vòng
    let max = I256::pow2(255)
        .expect("2^255")
        .checked_add(
            I256::pow2(255)
                .expect("2^255")
                .checked_sub(one)
                .expect("-1"),
        )
        .expect("2^256-1");
    assert_eq!(max.checked_add(one), None);
    assert_eq!(max.negate().checked_sub(one), None);
}
