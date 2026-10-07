//! Các biên của `Interval` và `ExhaustiveProver`/`ZoneProver` mà test khác chưa chạm (phát hiện bằng đột biến, ADR-025 §8):
//! widening có ngưỡng, dư/phép nhân với 0 và vô cực, `fits`/`saturate`, đếm gán của bộ vét cạn (nguyên tử, ngưỡng điểm, nhân chứng).
use gt_oblig::I256;
use gt_oblig::interval::{B, Interval, Tri, Ty};
use gt_oblig::ir::{
    Budget, Checkability, ObKind, Obligation, Prop, Prover, Term, VarDecl, VarId, Verdict,
};
use gt_oblig::prover::{ExhaustiveProver, ZoneProver};

fn iv(a: i128, b: i128) -> Interval {
    Interval::new(B::from_i128(a), B::from_i128(b))
}
fn th(v: &[i128]) -> Vec<I256> {
    v.iter().map(|x| I256::from_i128(*x)).collect()
}

#[test]
fn widening_jumps_to_the_nearest_threshold_only_when_a_bound_moves() {
    let t = th(&[-100, -1, 0, 1, 10, 100]);
    // cận trên tăng: nhảy tới ngưỡng ≥ new.hi gần nhất; cận dưới đứng yên
    assert_eq!(iv(0, 5).widen(iv(0, 7), &t), iv(0, 10));
    assert_eq!(iv(0, 5).widen(iv(0, 10), &t), iv(0, 10));
    assert_eq!(iv(0, 5).widen(iv(0, 11), &t), iv(0, 100));
    // cận dưới giảm: nhảy tới ngưỡng ≤ new.lo gần nhất
    assert_eq!(iv(0, 5).widen(iv(-1, 5), &t), iv(-1, 5));
    assert_eq!(iv(0, 5).widen(iv(-2, 5), &t), iv(-100, 5));
    // không có ngưỡng đủ xa ⇒ vô cực
    let w = iv(0, 5).widen(iv(0, 1000), &t);
    assert_eq!(w.hi, B::PosInf);
    assert_eq!(iv(0, 5).widen(iv(-1000, 5), &t).lo, B::NegInf);
    // cận KHÔNG đổi (bằng nhau) thì giữ nguyên, kể cả khi có ngưỡng thấp hơn
    let t2 = th(&[0, 3]);
    assert_eq!(iv(5, 8).widen(iv(5, 8), &t2), iv(5, 8));
    assert_eq!(iv(5, 8).widen(iv(6, 7), &t2), iv(5, 8)); // new ⊂ self ⇒ giữ
    // ngưỡng đúng bằng cận mới được chọn (≤ / ≥ chứ không phải < / >)
    let t3 = th(&[0, 3, 4, 20, 30]);
    assert_eq!(iv(5, 8).widen(iv(3, 8), &t3), iv(3, 8));
    assert_eq!(iv(5, 8).widen(iv(5, 20), &t3), iv(5, 20));
}

#[test]
fn containment_needs_both_bounds() {
    assert!(iv(0, 5).contains(I256::from_i128(3)));
    assert!(iv(0, 5).contains(I256::from_i128(0)));
    assert!(iv(0, 5).contains(I256::from_i128(5)));
    assert!(!iv(0, 5).contains(I256::from_i128(10)));
    assert!(!iv(0, 5).contains(I256::from_i128(-10)));
}

#[test]
fn multiplication_signs_and_zero_times_infinity() {
    assert_eq!(iv(-3, 0).mul(iv(2, 5)), iv(-15, 0));
    assert_eq!(iv(-2, 3).mul(iv(-4, 5)), iv(-12, 15));
    assert_eq!(iv(2, 3).mul(iv(4, 5)), iv(8, 15));
    assert_eq!(iv(-3, -2).mul(iv(-5, -4)), iv(8, 15));
    assert_eq!(iv(0, 0).mul(Interval::top()), iv(0, 0));
    let m = iv(1, 2).mul(Interval::top());
    assert_eq!((m.lo, m.hi), (B::NegInf, B::PosInf));
    let p = Interval::new(B::Fin(I256::from_i128(1)), B::PosInf).mul(iv(2, 3));
    assert_eq!(p.lo, B::from_i128(2));
    assert_eq!(p.hi, B::PosInf);
}

#[test]
fn remainder_bounds_are_exact() {
    // a ≥ 0 và a < |d|min ⇒ r = a (không bị kéo về 0)
    assert_eq!(iv(2, 3).rem_trunc_nz(iv(7, 7)), Some(iv(2, 3)));
    assert_eq!(iv(0, 3).rem_trunc_nz(iv(7, 7)), Some(iv(0, 3)));
    // |r| ≤ |d|max − 1, dấu theo số bị chia
    assert_eq!(iv(-10, 10).rem_trunc_nz(iv(7, 7)), Some(iv(-6, 6)));
    assert_eq!(iv(-10, -2).rem_trunc_nz(iv(7, 7)), Some(iv(-6, 0)));
    assert_eq!(iv(2, 10).rem_trunc_nz(iv(-7, -7)), Some(iv(0, 6)));
    assert_eq!(iv(0, 100).rem_trunc_nz(iv(3, 5)), Some(iv(0, 4)));
    assert_eq!(iv(5, 100).rem_trunc_nz(iv(3, 5)), Some(iv(0, 4)));
    // chỉ có số chia 0 ⇒ None
    assert_eq!(iv(1, 5).rem_trunc_nz(iv(0, 0)), None);
    // số chia gồm cả âm lẫn dương: lấy |d| lớn nhất
    assert_eq!(iv(-100, 100).rem_trunc_nz(iv(-4, 9)), Some(iv(-8, 8)));
}

#[test]
fn fits_and_saturate_follow_the_type_range() {
    let u8t = Ty::new(8, false);
    assert_eq!(iv(0, 255).fits(u8t), Tri::Yes);
    assert_eq!(iv(10, 20).fits(u8t), Tri::Yes);
    assert_eq!(iv(-5, 300).fits(u8t), Tri::Maybe);
    assert_eq!(iv(256, 400).fits(u8t), Tri::No);
    assert_eq!(iv(-9, -1).fits(u8t), Tri::No);
    assert_eq!(iv(-5, 300).saturate(u8t), iv(0, 255));
    assert_eq!(iv(-5, -1).saturate(u8t), iv(0, 0));
    assert_eq!(iv(300, 400).saturate(u8t), iv(255, 255));
    assert_eq!(iv(3, 9).saturate(u8t), iv(3, 9));
    assert_eq!(iv(-200, 200).saturate(Ty::new(8, true)), iv(-128, 127));
}

// ---------------------------------------------------------------- bộ giải

fn ob(vars: &[(VarId, Ty)], goal: Prop, hyps: Vec<Prop>, atoms: &[VarId]) -> Obligation {
    Obligation {
        id: "t#0".into(),
        kind: ObKind::Overflow,
        body: "t".into(),
        block: 0,
        stmt: 0,
        span: (0, 0),
        vars: vars
            .iter()
            .map(|(id, ty)| VarDecl {
                id: *id,
                name: format!("v{id}"),
                ty: *ty,
            })
            .collect(),
        atoms: atoms.iter().map(|a| (*a, format!("a{a}"))).collect(),
        goal,
        hyps,
        runtime: Checkability::Runtime("test".into()),
    }
}

fn budget() -> Budget {
    Budget::new(1 << 40)
}

#[test]
fn zone_prover_default_capacity_keeps_relations() {
    let u4 = Ty::new(4, false);
    let rel = Prop::Lt(Term::var(1), Term::var(2));
    let o = ob(&[(1, u4), (2, u4)], rel.clone(), vec![rel], &[]);
    assert!(matches!(
        ZoneProver::default().prove(&o, &mut budget()),
        Verdict::Proved(_)
    ));
    assert!(matches!(
        ZoneProver::new(0).prove(&o, &mut budget()),
        Verdict::Proved(_)
    ));
}

#[test]
fn contradictory_hypotheses_prove_anything_in_the_zone_prover() {
    let u4 = Ty::new(4, false);
    // v1 < 0 với v1 không dấu: không thoả
    let o = ob(
        &[(1, u4)],
        Prop::False,
        vec![Prop::Lt(Term::var(1), Term::int(0))],
        &[],
    );
    match ZoneProver::new(8).prove(&o, &mut budget()) {
        Verdict::Proved(c) => assert!(c.0.contains("mâu thuẫn"), "{c:?}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn exhaustive_prover_evaluates_constants_and_rejects_undeclared_or_huge_domains() {
    let mut p = ExhaustiveProver::default();
    assert!(matches!(
        p.prove(&ob(&[], Prop::True, vec![], &[]), &mut budget()),
        Verdict::Proved(_)
    ));
    match p.prove(&ob(&[], Prop::False, vec![], &[]), &mut budget()) {
        Verdict::Refuted { everywhere, .. } => assert!(everywhere),
        other => panic!("{other:?}"),
    }
    // biến dùng mà chưa khai báo kiểu
    assert_eq!(
        p.prove(
            &ob(&[], Prop::Le(Term::int(0), Term::var(9)), vec![], &[]),
            &mut budget()
        ),
        Verdict::Unknown
    );
    // 20 bit còn được vét cạn, 21 bit thì từ chối
    let u20 = Ty::new(20, false);
    assert!(matches!(
        p.prove(
            &ob(
                &[(1, u20)],
                Prop::Le(Term::int(0), Term::var(1)),
                vec![],
                &[]
            ),
            &mut budget()
        ),
        Verdict::Proved(_)
    ));
    let u21 = Ty::new(21, false);
    assert_eq!(
        p.prove(
            &ob(
                &[(1, u21)],
                Prop::Le(Term::int(0), Term::var(1)),
                vec![],
                &[]
            ),
            &mut budget()
        ),
        Verdict::Unknown
    );
    // số điểm == ngưỡng vẫn được chạy
    let mut small = ExhaustiveProver { max_points: 16 };
    let u4 = Ty::new(4, false);
    assert!(matches!(
        small.prove(
            &ob(
                &[(1, u4)],
                Prop::Le(Term::int(0), Term::var(1)),
                vec![],
                &[]
            ),
            &mut budget()
        ),
        Verdict::Proved(_)
    ));
    let mut smaller = ExhaustiveProver { max_points: 15 };
    assert_eq!(
        smaller.prove(
            &ob(
                &[(1, u4)],
                Prop::Le(Term::int(0), Term::var(1)),
                vec![],
                &[]
            ),
            &mut budget()
        ),
        Verdict::Unknown
    );
}

#[test]
fn exhaustive_prover_enumerates_every_atom_assignment_and_reports_the_first_witness() {
    let mut p = ExhaustiveProver::default();
    let a = Prop::Atom(1);
    let b = Prop::Atom(2);
    // a ∨ ¬a: đúng với mọi gán
    let taut = Prop::Or(vec![a.clone(), a.clone().not()]);
    assert!(matches!(
        p.prove(&ob(&[], taut, vec![], &[1]), &mut budget()),
        Verdict::Proved(_)
    ));
    // a ∨ b: sai CHỈ ở (false, false) — phải tìm thấy, không phải "mọi nơi"
    match p.prove(
        &ob(&[], Prop::Or(vec![a.clone(), b.clone()]), vec![], &[1, 2]),
        &mut budget(),
    ) {
        Verdict::Refuted { everywhere, .. } => assert!(!everywhere),
        other => panic!("{other:?}"),
    }
    // a ∧ b: sai ở ba trong bốn gán
    match p.prove(
        &ob(&[], Prop::And(vec![a.clone(), b.clone()]), vec![], &[1, 2]),
        &mut budget(),
    ) {
        Verdict::Refuted { everywhere, .. } => assert!(!everywhere),
        other => panic!("{other:?}"),
    }
    // giả thiết a ∧ b, mục tiêu a: đúng
    assert!(matches!(
        p.prove(
            &ob(&[], a.clone(), vec![a.clone(), b.clone()], &[1, 2]),
            &mut budget()
        ),
        Verdict::Proved(_)
    ));
    // giả thiết ¬a, mục tiêu a: sai ở MỌI gán thoả giả thiết
    match p.prove(
        &ob(&[], a.clone(), vec![a.clone().not()], &[1]),
        &mut budget(),
    ) {
        Verdict::Refuted { everywhere, .. } => assert!(everywhere),
        other => panic!("{other:?}"),
    }
    // nhân chứng là gán vi phạm ĐẦU TIÊN theo thứ tự liệt kê (0, 1, 2, …): v1 < 3 sai đầu tiên ở v1 = 3
    let u3 = Ty::new(3, false);
    match p.prove(
        &ob(
            &[(1, u3)],
            Prop::Lt(Term::var(1), Term::int(3)),
            vec![],
            &[],
        ),
        &mut budget(),
    ) {
        Verdict::Refuted {
            witness,
            everywhere,
        } => {
            assert!(!everywhere);
            assert_eq!(witness.0, vec![(1, I256::from_i128(3))]);
        }
        other => panic!("{other:?}"),
    }
}
