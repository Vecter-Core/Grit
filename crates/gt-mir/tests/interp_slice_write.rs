//! FB-C12 (2026-10-21): thông dịch MIR không ghi được QUA LÁT (`s[i] ← x` với `s = &!a[from..to]`): `Flow::Bug("ghi vào lát")` ⇒ ICE
//! "MIR không hợp lệ", trong khi LLVM chạy đúng (phát hiện bởi bộ kiểm std2 `[T]::sort`). Ghi qua lát phải sửa đúng cửa sổ và không đụng ngoài nó.
mod common;
use common::*;
use gt_mir::interp::{Interp, Outcome, V};

fn run(src: &str) -> Outcome {
    let t = one(src);
    assert!(t.report.diagnostics.is_empty(), "{:?}", show(&t));
    let m = gt_mir::lower_program(&t);
    assert!(m.report.diagnostics.is_empty(), "{:?}", mir_codes(&m));
    Interp::new(&m, 100_000).call_key("main::f", Vec::new())
}

#[test]
fn writing_through_a_mutable_array_slice_updates_only_the_window_fb_c12() {
    let src = "ƒ set(s: &![u32], i: usize, x: u32) ! ∅ { s[i] ← x; }
ƒ f() → u32 ! ∅ { μa ≔ [1u32, 2u32, 3u32, 4u32, 5u32]; set(&!a[1usize..4usize], 1usize, 90u32); a[0usize] + a[1usize] * 10u32 + a[2usize] * 100u32 + a[3usize] * 1000u32 + a[4usize] * 10000u32 }";
    // a = [1,2,90,4,5] ⇒ 1 + 20 + 9000 + 4000 + 50000
    assert_eq!(run(src), Outcome::Value(V::Int(63021, gt_types::IntTy::U32)));
}
