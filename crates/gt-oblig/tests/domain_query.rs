//! DomainQuery (M8, ADR-025 §7): khoảng/zone của biểu thức tại một điểm MIR, cho các mẫu `0..n`, đếm lên, đếm xuống.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_mir::{Local, LocalKind, Location};
use gt_oblig::analysis::DomainQuery;
use gt_oblig::driver::build_prog_info;
use gt_oblig::ir::Term;

fn local(b: &gt_mir::Body, name: &str) -> Local {
    let i = b
        .locals
        .iter()
        .position(|d| match &d.kind {
            LocalKind::User { name: n, .. } => n == name,
            LocalKind::Arg(Some(n)) => n == name,
            _ => false,
        })
        .unwrap_or_else(|| panic!("không có local {name}"));
    Local(i as u32)
}

/// (cận dưới, cận trên) dạng i128 (None = vô cực)
fn bounds(iv: &gt_oblig::Interval) -> (Option<i128>, Option<i128>) {
    use gt_oblig::B;
    let f = |b: &B| match b {
        B::Fin(v) => v.to_i128(),
        _ => None,
    };
    (f(&iv.lo), f(&iv.hi))
}

#[test]
fn counting_up_body_has_i_below_n_and_exit_has_i_at_least_n() {
    let src = "ƒ f(n: u32) → u32 ! ∅ { μi ≔ 0u32; ⟳ i < n { i ← i + 1; } i }";
    let (t, m) = common::mir(src);
    let info = build_prog_info(&t, &m);
    let b = m.bodies.iter().find(|b| b.key == "main::f").expect("f");
    let q = DomainQuery::new(b, &info).expect("phân tích");
    let head = b.loops[0].head;
    assert_eq!(
        bounds(
            &q.local_interval(
                Location {
                    block: head,
                    statement: 0
                },
                local(b, "i")
            )
            .expect("tới được")
        )
        .0,
        Some(0)
    );
    let d = Term::Sub(
        Box::new(Term::Var(local(b, "i").0)),
        Box::new(Term::Var(local(b, "n").0)),
    );
    // hai nhánh của đầu vòng: một nhánh có i - n ≤ -1 (vào thân), nhánh kia có i - n ≥ 0 (thoát)
    let succs = b.blocks[head.index()]
        .terminator
        .as_ref()
        .expect("terminator")
        .kind
        .successors();
    let mut body_side = false;
    let mut exit_side = false;
    for s in succs {
        let iv = q
            .interval_at(
                Location {
                    block: s,
                    statement: 0,
                },
                &d,
            )
            .expect("tới được");
        let (lo, hi) = bounds(&iv);
        body_side |= hi.is_some_and(|h| h <= -1);
        exit_side |= lo.is_some_and(|l| l >= 0);
    }
    assert!(body_side, "nhánh vào thân phải có i - n ≤ -1");
    assert!(exit_side, "nhánh thoát phải có i - n ≥ 0");
}

#[test]
fn counting_down_head_has_i_nonnegative_and_below_n() {
    let src = "ƒ f(n: u32) → u32 ! ∅ { μi ≔ n; ⟳ i > 0 { i ← i - 1; } i }";
    let (t, m) = common::mir(src);
    let info = build_prog_info(&t, &m);
    let b = m.bodies.iter().find(|b| b.key == "main::f").expect("f");
    let q = DomainQuery::new(b, &info).expect("phân tích");
    let loc = Location {
        block: b.loops[0].head,
        statement: 0,
    };
    let iv = q.local_interval(loc, local(b, "i")).expect("tới được");
    assert_eq!(bounds(&iv).0, Some(0));
    let d = Term::Sub(
        Box::new(Term::Var(local(b, "i").0)),
        Box::new(Term::Var(local(b, "n").0)),
    );
    assert_eq!(
        bounds(&q.interval_at(loc, &d).expect("tới được")).1,
        Some(0)
    );
}

#[test]
fn range_loop_variable_stays_below_the_bound() {
    let src = "ƒ f(n: u32) → u32 ! ∅ { μs ≔ 0u32; ⟳ i in 0..n { s ← s +% i; } s }";
    let (t, m) = common::mir(src);
    let info = build_prog_info(&t, &m);
    let b = m.bodies.iter().find(|b| b.key == "main::f").expect("f");
    let q = DomainQuery::new(b, &info).expect("phân tích");
    // mọi điểm tới được trong thân: biến đếm của for-in luôn ≥ 0 (kiểu u32) và có cận trên hữu hạn tại đầu vòng
    let loc = Location {
        block: b.loops[0].head,
        statement: 0,
    };
    assert!(q.reachable(loc));
    let mut seen = 0;
    for (bi, blk) in b.blocks.iter().enumerate() {
        let l = Location {
            block: gt_mir::BasicBlock(bi as u32),
            statement: 0,
        };
        if q.reachable(l) && !blk.statements.is_empty() {
            seen += 1;
        }
    }
    assert!(seen >= 2);
}

#[test]
fn unreachable_points_are_none() {
    let src = "ƒ f() → i32 ! ∅ { ⎇ false { 1 } ⇝ { 2 } }";
    let (t, m) = common::mir(src);
    let info = build_prog_info(&t, &m);
    let b = m.bodies.iter().find(|b| b.key == "main::f").expect("f");
    let q = DomainQuery::new(b, &info).expect("phân tích");
    let unreachable = (0..b.blocks.len())
        .filter(|&i| {
            !q.reachable(Location {
                block: gt_mir::BasicBlock(i as u32),
                statement: 0,
            })
        })
        .count();
    let reachable = (0..b.blocks.len())
        .filter(|&i| {
            q.reachable(Location {
                block: gt_mir::BasicBlock(i as u32),
                statement: 0,
            })
        })
        .count();
    assert!(reachable >= 1);
    assert!(unreachable + reachable == b.blocks.len());
}

#[test]
#[ignore]
fn dump_domain() {
    let src = std::env::var("GT_SRC").expect("GT_SRC");
    let (t, m) = common::mir(&src);
    let info = build_prog_info(&t, &m);
    let b = m.bodies.iter().find(|b| b.key == "main::f").expect("f");
    let q = DomainQuery::new(b, &info).expect("phân tích");
    let (i, n) = (local(b, "i").0, local(b, "n").0);
    let d = Term::Sub(Box::new(Term::Var(i)), Box::new(Term::Var(n)));
    for (bi, blk) in b.blocks.iter().enumerate() {
        for s in 0..=blk.statements.len() {
            let l = Location {
                block: gt_mir::BasicBlock(bi as u32),
                statement: s,
            };
            if let Some(iv) = q.interval_at(l, &Term::Var(i)) {
                let dv = q.interval_at(l, &d).expect("cùng điểm");
                eprintln!("bb{bi}:{s} i={:?} i-n={:?}", bounds(&iv), bounds(&dv));
                if s == blk.statements.len() {
                    eprintln!(
                        "bb{bi}: term {:?}",
                        blk.terminator.as_ref().map(|t| &t.kind)
                    );
                }
            }
        }
    }
}
