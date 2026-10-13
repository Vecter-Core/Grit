//! "Bảo toàn ngữ nghĩa khi hạ MIR" (plan §9; ADR-020 §7, §9): chương trình ngẫu nhiên → kết quả kỳ vọng do BỘ ĐÁNH GIÁ AST
//! RIÊNG của bộ sinh tính → so với thông dịch MIR (kể cả LOẠI abort: tràn, chia 0, -MIN, ép mất thông tin).
//! Thông dịch và bộ sinh là công cụ test, không thuộc TCB.
mod common;
mod support;
use common::*;
use gt_mir::interp::{Interp, Outcome, V};
use support::progen::{Gen, Res, call, print};

const PROGRAMS: u64 = 400;

#[test]
fn lowering_preserves_the_meaning_of_random_programs() {
    let mut aborts = 0;
    let mut values = 0;
    let mut place_then_effect = 0u64;
    for seed in 1..=PROGRAMS {
        let mut g = Gen::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let fns = g.program();
        place_then_effect += g.place_then_effect as u64;
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
    // độ phủ: toán hạng là CHỖ đứng trước một toán hạng ghi đúng chỗ đó (lỗi M5 đã lọt qua 400 chương trình cũ vì bộ sinh
    // không có biểu thức có tác dụng phụ)
    assert!(
        place_then_effect >= PROGRAMS / 2,
        "chỉ {place_then_effect} biểu thức chỗ-rồi-tác-dụng-phụ trong {PROGRAMS} chương trình"
    );
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

/// Toán hạng là CHỖ phải được đọc ở lúc đánh giá của nó, không phải lúc dùng: biểu thức đứng sau có thể ghi vào chỗ đó
/// (lỗi tìm thấy khi viết corpus M6: `g(x, { x ← 2; 3 })` đọc x = 2).
#[test]
fn place_operands_are_read_before_later_operands_can_change_them() {
    for (src, want) in [
        (
            "ƒ g(a: i32, b: i32) → i32 ! ∅ { a } ƒ entry() → i32 ! ∅ { μx ≔ 1; g(x, { x ← 2; 3 }) }",
            1,
        ),
        ("ƒ entry() → i32 ! ∅ { μx ≔ 1; x - { x ← 5; 0 } }", 1),
        ("ƒ entry() → i32 ! ∅ { μx ≔ 1; x +% { x ← 9; 10 } }", 11),
        (
            "ƒ entry() → i32 ! ∅ { μx ≔ 1; t ≔ (x, { x ← 7; 0 }); t.0 }",
            1,
        ),
        (
            "ƒ entry() → i32 ! ∅ { μx ≔ 1; a ≔ [x, { x ← 7; 0 }]; a[0] }",
            1,
        ),
        (
            "▢ P { a: i32, b: i32 } ƒ entry() → i32 ! ∅ { μx ≔ 1; P { a: x, b: { x ← 3; 0 } }.a }",
            1,
        ),
        ("ƒ entry() → bool ! ∅ { μx ≔ 1; x < { x ← 0; 2 } }", 1),
    ] {
        let (_, m) = mir(src);
        match Interp::new(&m, 10_000).call_key("main::entry", Vec::new()) {
            Outcome::Value(V::Int(v, _)) => assert_eq!(v, want, "{src}"),
            Outcome::Value(V::Bool(b)) => assert_eq!(i128::from(b), want, "{src}"),
            other => panic!("{src}: {other:?}"),
        }
    }
}

/// ADR-022 §5 (đã duyệt): người nhận THEO GIÁ TRỊ bị tiêu thụ SAU các đối số (cùng nguyên tắc autoref muộn, Q28).
/// Khác với toán hạng đối số, nó KHÔNG được chụp trước: `p.add({ p ← P { a: 100 }; 5 })` đọc `p` mới.
#[test]
fn by_value_receiver_is_consumed_after_the_arguments() {
    let src = "▢ P { a: i32 } ⊐ P { ƒ add(self, k: i32) → i32 ! ∅ { self.a + k } } ƒ entry() → i32 ! ∅ { μp ≔ P { a: 1 }; p.add({ p ← P { a: 100 }; 5 }) }";
    let (_, m) = mir(src);
    match Interp::new(&m, 10_000).call_key("main::entry", Vec::new()) {
        Outcome::Value(V::Int(v, _)) => assert_eq!(v, 105, "{src}"),
        other => panic!("{src}: {other:?}"),
    }
}

#[test]
fn spawn_runs_the_function_with_its_argument() {
    // công cụ test: spawn chạy tuần tự trong thông dịch (ngoài TCB). Worker abort khi n == 7 chứng tỏ nó thật sự chạy với đối số.
    let src = "ƒ work(n: u32) ! ∅ { z ≔ 1u32 / (n - 7u32); }
ƒ go(n: u32) → u32 ! ∅ { spawn(work, n); 1u32 }";
    let (_t, m) = mir(src);
    let mut it = Interp::new(&m, 10_000);
    assert!(matches!(
        it.call_key("main::go", vec![V::Int(9, gt_types::IntTy::U32)]),
        Outcome::Value(V::Int(1, _))
    ));
    let mut it = Interp::new(&m, 10_000);
    assert!(matches!(
        it.call_key("main::go", vec![V::Int(7, gt_types::IntTy::U32)]),
        Outcome::Abort(_)
    ));
}

#[test]
fn mutex_atomic_and_cell_intrinsics_run_sequentially() {
    // công cụ test (ngoài TCB): ngữ nghĩa tuần tự, không phải bằng chứng cho codegen
    let src = "#level(\"1\")\n▢ Lo { }\n\
        ƒ addn(v: &!u32, n: u32) → u32 ! ∅ { *v ← *v +% n; *v }\n\
        ƒ go(n: u32) → u32 ! {lock(Lo)} {\n\
          m: Mutex<u32, Lo> ≔ Mutex::new(10u32);\n\
          a ≔ Atomic::new(5u32);\n\
          c ≔ Cell::new(7u32);\n\
          r1 ≔ m.with(addn, n);\n\
          r2 ≔ m.with(addn, n);\n\
          a.fetch_add(n, AtomicOrdering::SeqCst);\n\
          c.set(c.get() +% 1u32);\n\
          r2 +% a.load(AtomicOrdering::SeqCst) +% c.get() +% r1\n\
        }";
    let (_t, m) = mir(src);
    let mut it = Interp::new(&m, 100_000);
    // n = 3: r1 = 13, r2 = 16, a = 8, c = 8 ⇒ 16 + 8 + 8 + 13 = 45
    assert!(matches!(
        it.call_key("main::go", vec![V::Int(3, gt_types::IntTy::U32)]),
        Outcome::Value(V::Int(45, _))
    ));
}
