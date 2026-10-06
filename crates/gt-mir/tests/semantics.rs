//! "Bảo toàn ngữ nghĩa khi hạ MIR" (plan §9; ADR-020 §7, §9): chương trình ngẫu nhiên → kết quả kỳ vọng do BỘ ĐÁNH GIÁ AST
//! RIÊNG của bộ sinh tính → so với thông dịch MIR (kể cả LOẠI abort: tràn, chia 0, -MIN, ép mất thông tin).
//! Thông dịch và bộ sinh là công cụ test, không thuộc TCB.
mod common;
mod support;
use common::*;
use support::interp::{Interp, Outcome, V};
use support::progen::{Gen, Res, call, print};

const PROGRAMS: u64 = 400;

#[test]
fn lowering_preserves_the_meaning_of_random_programs() {
    let mut aborts = 0;
    let mut values = 0;
    for seed in 1..=PROGRAMS {
        let mut g = Gen::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let fns = g.program();
        let src = print(&fns);
        let t = one(&src);
        assert!(
            t.report.diagnostics.is_empty(),
            "#{seed}: chương trình sinh ra không qua M4: {:?}\n{src}",
            show(&t)
        );
        let m = gt_mir::lower_program(&t);
        assert!(
            m.report.diagnostics.is_empty(),
            "#{seed}: {:?}\n{src}",
            mir_codes(&m)
        );
        let want = call(fns.len() - 1, Vec::new(), &fns);
        let got = Interp::new(&m, 1_000_000).call_key("main::entry", Vec::new());
        match (&want, &got) {
            (Res::Value(w), Outcome::Value(V::Int(g2, _))) if i128::from(*w) == *g2 => values += 1,
            (Res::Abort(k), Outcome::Abort(k2)) if *k == k2.as_str() => aborts += 1,
            _ => panic!("#{seed}: kỳ vọng {want:?}, MIR cho {got:?}\n{src}"),
        }
    }
    // bộ sinh phải phủ được cả hai loại kết quả
    assert!(
        values >= PROGRAMS / 5,
        "chỉ {values} chương trình trả giá trị"
    );
    assert!(aborts >= PROGRAMS / 20, "chỉ {aborts} chương trình abort");
}

#[test]
fn every_abort_kind_is_reached_and_matches() {
    let cases = [
        ("ƒ entry() → i32 ! ∅ { x ≔ 2147483647; x + 1 }", "overflow"),
        ("ƒ entry() → i32 ! ∅ { x ≔ 0; 7 / x }", "div_by_zero"),
        ("ƒ entry() → i32 ! ∅ { x ≔ 0; 7 % x }", "rem_by_zero"),
        (
            "ƒ entry() → i32 ! ∅ { x ≔ -2147483648; -x }",
            "overflow_neg",
        ),
        (
            "ƒ entry() → i32 ! ∅ { x ≔ -2147483648; x / (-1) }",
            "overflow",
        ),
        (
            "ƒ entry() → i32 ! ∅ { x ≔ 300; (x as u8) as i32 }",
            "cast_lossy",
        ),
        (
            "ƒ entry() → i32 ! ∅ { a ≔ [1, 2, 3]; i ≔ 3usize; a[i] }",
            "index_out_of_bounds",
        ),
        (
            "ƒ entry() → i32 ! ∅ { a ≔ [1, 2, 3]; s ≔ &a[2..1]; 0 }",
            "slice_range",
        ),
    ];
    for (src, kind) in cases {
        let (_, m) = mir(src);
        let got = Interp::new(&m, 10_000).call_key("main::entry", Vec::new());
        assert_eq!(got, Outcome::Abort(kind.to_string()), "{src}");
    }
    // không abort: phép bao quanh/bão hoà/kiểm tra, ép không mất
    for (src, want) in [
        (
            "ƒ entry() → i32 ! ∅ { x ≔ 2147483647; x +% 1 }",
            -2_147_483_648,
        ),
        (
            "ƒ entry() → i32 ! ∅ { x ≔ 2147483647; x +^ 1 }",
            2_147_483_647,
        ),
        (
            "ƒ entry() → i32 ! ∅ { x ≔ 2147483647; ⌥ x +? 1 { Some(v) ↦ v, None ↦ -7 } }",
            -7,
        ),
        ("ƒ entry() → i32 ! ∅ { x ≔ 200; (x as u8) as i32 }", 200),
        (
            "ƒ entry() → i32 ! ∅ { a ≔ [1, 2, 3]; s ≔ &a[1..=2]; μt ≔ 0; ⟳ v in s { t ← t + *v; } t }",
            5,
        ),
    ] {
        let (_, m) = mir(src);
        match Interp::new(&m, 10_000).call_key("main::entry", Vec::new()) {
            Outcome::Value(V::Int(v, _)) => assert_eq!(v, want, "{src}"),
            other => panic!("{src}: {other:?}"),
        }
    }
}

#[test]
fn evaluation_order_is_left_to_right_including_struct_literals_in_written_order() {
    // nếu thứ tự đánh giá sai, abort đầu tiên sẽ khác loại
    for (src, kind) in [
        (
            "ƒ entry() → i32 ! ∅ { z ≔ 0; m ≔ -2147483648; (7 / z) + (-m) }",
            "div_by_zero",
        ),
        (
            "ƒ entry() → i32 ! ∅ { z ≔ 0; m ≔ -2147483648; (-m) + (7 / z) }",
            "overflow_neg",
        ),
        (
            "▢ P { a: i32, b: i32 } ƒ entry() → i32 ! ∅ { z ≔ 0; m ≔ -2147483648; P { b: 7 / z, a: -m }.a }",
            "div_by_zero",
        ),
        (
            "▢ P { a: i32, b: i32 } ƒ entry() → i32 ! ∅ { z ≔ 0; m ≔ -2147483648; P { a: -m, b: 7 / z }.a }",
            "overflow_neg",
        ),
    ] {
        let (_, m) = mir(src);
        assert_eq!(
            Interp::new(&m, 10_000).call_key("main::entry", Vec::new()),
            Outcome::Abort(kind.to_string()),
            "{src}"
        );
    }
}
