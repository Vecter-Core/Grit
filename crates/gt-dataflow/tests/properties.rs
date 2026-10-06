//! Test thuộc tính (ADR-020 §3, §7; PRNG tự viết, không proptest — ADR-009): trên CFG ngẫu nhiên,
//! (1) bộ giải (RPO + hàng đợi) cho ĐÚNG điểm cố định nhỏ nhất của một bộ giải ngây thơ viết độc lập (lặp theo chỉ số
//! tới khi không đổi); (2) kết quả không phụ thuộc cách đánh số khối; (3) tất định.
mod common;
use common::*;
use gt_dataflow::*;
use gt_mir::*;
use gt_types::Ty;

const CASES: u64 = 300;

fn random_body(rng: &mut Rng) -> Body {
    let nb = 2 + rng.below(10) as usize;
    let args = rng.below(3) as usize;
    let nl = 1 + args + 1 + rng.below(5) as usize;
    let tys: Vec<Ty> = (0..nl).map(|_| i32t()).collect();
    let mut blocks = Vec::new();
    for _ in 0..nb {
        let mut sts = Vec::new();
        for _ in 0..rng.below(5) {
            let a = rng.below(nl as u64) as u32;
            let b = rng.below(nl as u64) as u32;
            let s = match rng.below(6) {
                0 => assign(a, Rvalue::Use(c(1))),
                1 => assign(a, use_copy(b)),
                2 => assign(a, Rvalue::Use(Operand::Move(l(b)))),
                3 if a as usize > args => st(StatementKind::StorageLive(Local(a))),
                4 if a as usize > args => st(StatementKind::StorageDead(Local(a))),
                _ => st(StatementKind::FakeRead(FakeReadCause::LetInit, l(b))),
            };
            sts.push(s);
        }
        let t = match rng.below(5) {
            0 => TerminatorKind::Return,
            1 => TerminatorKind::Goto(bb(rng.below(nb as u64) as u32)),
            2 => TerminatorKind::Call {
                callee: Callee::Value(Operand::Copy(l(rng.below(nl as u64) as u32))),
                args: vec![Operand::Move(l(rng.below(nl as u64) as u32))],
                arg_spans: vec![],
                dest: l(rng.below(nl as u64) as u32),
                target: Some(bb(rng.below(nb as u64) as u32)),
            },
            3 => TerminatorKind::Drop {
                place: l(rng.below(nl as u64) as u32),
                target: bb(rng.below(nb as u64) as u32),
            },
            _ => TerminatorKind::SwitchInt {
                discr: Operand::Copy(l(rng.below(nl as u64) as u32)),
                targets: vec![(0, bb(rng.below(nb as u64) as u32))],
                otherwise: bb(rng.below(nb as u64) as u32),
            },
        };
        blocks.push((sts, t));
    }
    body(tys, args, blocks)
}

fn reachable(b: &Body) -> Vec<bool> {
    let mut r = vec![false; b.blocks.len()];
    let mut stack = vec![0usize];
    while let Some(i) = stack.pop() {
        if r[i] {
            continue;
        }
        r[i] = true;
        for s in b.blocks[i]
            .terminator
            .as_ref()
            .expect("t")
            .kind
            .successors()
        {
            stack.push(s.index());
        }
    }
    r
}

/// Bộ giải ngây thơ, viết độc lập với `solve`: lặp mọi khối tới được theo chỉ số tới khi không đổi.
fn naive<A: Analysis>(a: &A, b: &Body) -> Vec<A::Domain> {
    let n = b.blocks.len();
    let reach = reachable(b);
    let mut st: Vec<A::Domain> = (0..n).map(|_| a.bottom(b)).collect();
    match A::DIRECTION {
        Direction::Forward => {
            a.initialize_start(b, &mut st[0]);
            loop {
                let mut changed = false;
                for i in 0..n {
                    if !reach[i] {
                        continue;
                    }
                    let mut s = st[i].clone();
                    for (k, x) in b.blocks[i].statements.iter().enumerate() {
                        a.statement_effect(
                            &mut s,
                            x,
                            Location {
                                block: bb(i as u32),
                                statement: k,
                            },
                        );
                    }
                    let t = b.blocks[i].terminator.as_ref().expect("t");
                    a.terminator_effect(
                        &mut s,
                        t,
                        Location {
                            block: bb(i as u32),
                            statement: 99,
                        },
                    );
                    for sc in t.kind.successors() {
                        let mut e = s.clone();
                        if let TerminatorKind::Call {
                            dest,
                            target: Some(tg),
                            ..
                        } = &t.kind
                            && *tg == sc
                        {
                            a.call_return_effect(&mut e, dest);
                        }
                        changed |= st[sc.index()].join(&e);
                    }
                }
                if !changed {
                    return st;
                }
            }
        }
        Direction::Backward => {
            let mut before: Vec<A::Domain> = (0..n).map(|_| a.bottom(b)).collect();
            loop {
                let mut changed = false;
                for i in 0..n {
                    if !reach[i] {
                        continue;
                    }
                    let t = b.blocks[i].terminator.as_ref().expect("t");
                    let mut s = a.bottom(b);
                    for sc in t.kind.successors() {
                        s.join(&before[sc.index()]);
                    }
                    st[i] = s.clone();
                    a.terminator_effect(
                        &mut s,
                        t,
                        Location {
                            block: bb(i as u32),
                            statement: 99,
                        },
                    );
                    for (k, x) in b.blocks[i].statements.iter().enumerate().rev() {
                        a.statement_effect(
                            &mut s,
                            x,
                            Location {
                                block: bb(i as u32),
                                statement: k,
                            },
                        );
                    }
                    if before[i] != s {
                        before[i] = s;
                        changed = true;
                    }
                }
                if !changed {
                    return st;
                }
            }
        }
    }
}

fn check_against_naive<A: Analysis>(a: &A, b: &Body, label: &str)
where
    A::Domain: std::fmt::Debug,
{
    let r = solve(a, b).expect("hội tụ");
    let want = naive(a, b);
    let reach = reachable(b);
    for i in 0..b.blocks.len() {
        if reach[i] {
            assert_eq!(r.state[i], want[i], "{label}: khối {i}\n{b:#?}");
        }
    }
}

#[test]
fn the_solver_finds_the_same_fixpoint_as_a_naive_independent_solver() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for case in 0..CASES {
        let b = random_body(&mut rng);
        let mp = MovePaths::build(&b);
        check_against_naive(&MaybeInit { paths: &mp }, &b, &format!("#{case} MaybeInit"));
        check_against_naive(
            &MaybeUninit { paths: &mp },
            &b,
            &format!("#{case} MaybeUninit"),
        );
        check_against_naive(
            &DefinitelyAssigned,
            &b,
            &format!("#{case} DefinitelyAssigned"),
        );
        check_against_naive(&Liveness, &b, &format!("#{case} Liveness"));
    }
}

/// Đánh số lại khối (giữ `bb0` là entry) theo hoán vị `perm` (cũ → mới).
fn renumber(b: &Body, perm: &[usize]) -> Body {
    let mut nb = b.clone();
    let m = |x: BasicBlock| bb(perm[x.index()] as u32);
    let mut blocks = vec![None; b.blocks.len()];
    for (i, blk) in b.blocks.iter().enumerate() {
        let mut blk = blk.clone();
        if let Some(t) = &mut blk.terminator {
            match &mut t.kind {
                TerminatorKind::Goto(x) => *x = m(*x),
                TerminatorKind::SwitchInt {
                    targets, otherwise, ..
                } => {
                    for (_, x) in targets.iter_mut() {
                        *x = m(*x);
                    }
                    *otherwise = m(*otherwise);
                }
                TerminatorKind::Drop { target, .. } | TerminatorKind::Assert { target, .. } => {
                    *target = m(*target)
                }
                TerminatorKind::Call {
                    target: Some(x), ..
                } => *x = m(*x),
                _ => {}
            }
        }
        blocks[perm[i]] = Some(blk);
    }
    nb.blocks = blocks.into_iter().map(|x| x.expect("đủ")).collect();
    nb
}

#[test]
fn results_do_not_depend_on_block_numbering() {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    for case in 0..CASES {
        let b = random_body(&mut rng);
        let n = b.blocks.len();
        // hoán vị ngẫu nhiên giữ 0 → 0
        let mut perm: Vec<usize> = (0..n).collect();
        for i in (2..n).rev() {
            let j = 1 + rng.below(i as u64) as usize;
            perm.swap(i, j);
        }
        let b2 = renumber(&b, &perm);
        let r1 = solve(&Liveness, &b).expect("hội tụ");
        let r2 = solve(&Liveness, &b2).expect("hội tụ");
        let d1 = solve(&DefinitelyAssigned, &b).expect("hội tụ");
        let d2 = solve(&DefinitelyAssigned, &b2).expect("hội tụ");
        let reach = reachable(&b);
        for i in 0..n {
            if reach[i] {
                assert_eq!(r1.state[i], r2.state[perm[i]], "#{case} liveness khối {i}");
                assert_eq!(d1.state[i], d2.state[perm[i]], "#{case} assigned khối {i}");
            }
        }
    }
}

#[test]
fn solving_is_deterministic_and_bounded() {
    let mut rng = Rng(42);
    for _ in 0..CASES {
        let b = random_body(&mut rng);
        let a = solve(&Liveness, &b).expect("hội tụ");
        let c2 = solve(&Liveness, &b).expect("hội tụ");
        assert_eq!(a, c2);
        assert!(a.steps <= (b.blocks.len() + 1) * 256 + 4096);
    }
}

#[test]
fn bitset_join_is_idempotent_commutative_and_monotone() {
    let mut rng = Rng(7);
    for _ in 0..CASES {
        let n = 1 + rng.below(150) as usize;
        let mut a = BitSet::empty(n);
        let mut b = BitSet::empty(n);
        for _ in 0..rng.below(40) {
            a.insert(rng.below(n as u64) as usize);
            b.insert(rng.below(n as u64) as usize);
        }
        let mut ab = a.clone();
        ab.join(&b);
        let mut ba = b.clone();
        ba.join(&a);
        assert_eq!(ab, ba);
        let mut again = ab.clone();
        assert!(!again.join(&b), "join lần hai không được đổi");
        assert_eq!(again, ab);
        for i in a.iter() {
            assert!(ab.contains(i), "join không được làm mất phần tử");
        }
        let mut d = Dual(ab.clone());
        let before = d.clone();
        d.join(&Dual(a.clone()));
        for i in d.0.iter() {
            assert!(before.0.contains(i) && a.contains(i));
        }
    }
}
