//! `Body.loops` (M8, ADR-026): đầu vòng là đích của mọi cạnh lùi; hình dạng, biến thể `↓` và điều kiện được ghi lại.
mod common;
use common::*;
use gt_mir::{LoopShape, TerminatorKind};

fn body(src: &str, key: &str) -> gt_mir::Body {
    let (_t, m) = mir(src);
    assert!(m.report.diagnostics.is_empty(), "{:?}", mir_codes(&m));
    m.body(key).expect("thân").clone()
}

fn preds(b: &gt_mir::Body, target: gt_mir::BasicBlock) -> usize {
    b.blocks
        .iter()
        .filter(|bb| {
            bb.terminator
                .as_ref()
                .is_some_and(|t| t.kind.successors().contains(&target))
        })
        .count()
}

#[test]
fn every_loop_head_is_recorded_with_shape_variant_and_condition() {
    let b = body(
        "ƒ f(n: u32) → u32 ! ∅ { μi ≔ 0u32; ⟳ i < n ↓ n - i { i ← i + 1; } i }",
        "main::f",
    );
    assert_eq!(b.loops.len(), 1);
    let l = &b.loops[0];
    assert_eq!(l.shape, LoopShape::While);
    assert!(l.variant.is_some() && l.cond.is_some());
    assert!(preds(&b, l.head) >= 2, "đầu vòng có cạnh vào và cạnh lùi");

    let b = body(
        "ƒ f() → u32 ! {div} { μi ≔ 0u32; ⟳ { i ← i + 1; ⎇ i > 3 { ⊠; } } i }",
        "main::f",
    );
    assert_eq!((b.loops.len(), b.loops[0].shape), (1, LoopShape::Infinite));
    assert!(b.loops[0].variant.is_none() && b.loops[0].cond.is_none());

    let b = body(
        "ƒ f(n: u32) → u32 ! ∅ { μs ≔ 0u32; ⟳ i in 0..n { s ← s +% i; } s }",
        "main::f",
    );
    assert_eq!((b.loops.len(), b.loops[0].shape), (1, LoopShape::ForRange));
    let b = body(
        "ƒ f(xs: &[u32]) → u32 ! ∅ { μs ≔ 0u32; ⟳ x in xs { s ← s +% *x; } s }",
        "main::f",
    );
    assert_eq!((b.loops.len(), b.loops[0].shape), (1, LoopShape::ForSlice));
    let b = body(
        "ƒ f(n: u32) → u32 ! ∅ { μs ≔ 0u32; ⟳ i in 0..=n { s ← s +% i; } s }",
        "main::f",
    );
    assert_eq!((b.loops.len(), b.loops[0].shape), (1, LoopShape::ForRange));
}

#[test]
fn nested_loops_and_compound_conditions_keep_every_head() {
    let b = body(
        "ƒ f(n: u32) → u32 ! ∅ { μi ≔ 0u32; ⟳ i < n ∧ i < 100 { μj ≔ 0u32; ⟳ j < 3 { j ← j + 1; } i ← i + 1; } i }",
        "main::f",
    );
    assert_eq!(b.loops.len(), 2);
    for l in &b.loops {
        assert_eq!(l.shape, LoopShape::While);
        assert!(preds(&b, l.head) >= 2);
        // đầu vòng chỉ được nhảy tới từ terminator `Goto` (không phải rẽ nhánh)
        assert!(b.blocks.iter().any(|bb| matches!(
            bb.terminator.as_ref().map(|t| &t.kind),
            Some(TerminatorKind::Goto(h)) if *h == l.head
        )));
    }
    assert_ne!(b.loops[0].head, b.loops[1].head);
}

#[test]
fn var_map_links_resolver_ids_to_locals() {
    let b = body(
        "ƒ f(n: u32) → u32 ! ∅ { μi ≔ 0u32; ⟳ i < n { i ← i + 1; } i }",
        "main::f",
    );
    assert!(b.var_map.len() >= 2, "{:?}", b.var_map);
    // mỗi local trong bảng là tham số hoặc biến người dùng
    for (_, l) in &b.var_map {
        let k = &b.locals[l.index()].kind;
        assert!(
            matches!(
                k,
                gt_mir::LocalKind::Arg(_) | gt_mir::LocalKind::User { .. }
            ),
            "{k:?}"
        );
    }
}
