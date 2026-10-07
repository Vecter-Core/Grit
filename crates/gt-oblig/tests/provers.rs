//! Hai bộ giải trên CÙNG một Obligation (ADR-025 §12): bộ giải chính (khoảng+zone) không được "chứng minh" nghĩa vụ mà bộ giải vét cạn
//! tìm ra phản ví dụ, và không được báo "chắc chắn sai" khi vét cạn thấy có gán làm mục tiêu đúng. Hai bộ bất đồng = bug ở một trong hai.
//! Với giả thiết CHỈ là hộp khoảng, các nghĩa vụ số học/so sánh phải CHÍNH XÁC (chứng minh ⇔ vét cạn chứng minh).
use gt_oblig::int::I256;
use gt_oblig::interval::Ty;
use gt_oblig::ir::{
    Budget, Checkability, ObKind, Obligation, Prop, Prover, Term, VarDecl, Verdict,
};
use gt_oblig::prover::{ExhaustiveProver, ZoneProver};

fn ob(vars: &[(u32, Ty)], goal: Prop, hyps: Vec<Prop>) -> Obligation {
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
        atoms: vec![],
        goal,
        hyps,
        runtime: Checkability::Runtime("test".into()),
    }
}

fn xorshift(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

#[derive(Default, Debug)]
struct Stats {
    cases: u64,
    both_proved: u64,
    main_incomplete: u64,
    both_refuted: u64,
    unknown_both: u64,
    /// hệ giả thiết mâu thuẫn mà bộ chính chưa phát hiện (chỉ mất chính xác, báo 'sai' trên mã chết: fail-closed)
    vacuous_refuted: u64,
}

fn compare(o: &Obligation, exact: bool, st: &mut Stats) {
    let m = ZoneProver::new(8).prove(o, &mut Budget::new(1 << 40));
    let e = ExhaustiveProver::default().prove(o, &mut Budget::new(1 << 40));
    st.cases += 1;
    let ctx = || {
        format!(
            "goal: {}\nhyps: {:?}",
            o.goal.to_text(),
            o.hyps.iter().map(Prop::to_text).collect::<Vec<_>>()
        )
    };
    match (&m, &e) {
        (Verdict::Proved(_), Verdict::Proved(_)) => st.both_proved += 1,
        (Verdict::Proved(c), Verdict::Refuted { witness, .. }) => {
            panic!(
                "KHÔNG SOUND: bộ giải chính chứng minh ({c:?}) nhưng vét cạn có phản ví dụ {witness:?}\n{}",
                ctx()
            )
        }
        (
            Verdict::Refuted {
                everywhere: true, ..
            },
            Verdict::Refuted {
                everywhere: true, ..
            },
        ) => st.both_refuted += 1,
        (
            Verdict::Refuted {
                everywhere: true, ..
            },
            Verdict::Proved(c),
        ) if c.0.contains("không thoả") && !exact => {
            st.vacuous_refuted += 1;
        }
        (
            Verdict::Refuted {
                everywhere: true, ..
            },
            other,
        ) => {
            panic!(
                "bộ giải chính báo CHẮC CHẮN SAI nhưng vét cạn: {other:?}\n{}",
                ctx()
            )
        }
        (
            Verdict::Refuted {
                everywhere: false, ..
            },
            _,
        ) => panic!("bộ giải chính không được báo 'có phản ví dụ'"),
        (Verdict::Unknown, Verdict::Proved(_)) => {
            st.main_incomplete += 1;
            assert!(
                !exact,
                "phải CHÍNH XÁC với giả thiết hộp: chính Unknown, vét cạn Proved\n{}",
                ctx()
            );
        }
        (Verdict::Unknown, Verdict::Refuted { everywhere, .. }) => {
            if *everywhere {
                st.main_incomplete += 1;
                assert!(
                    !exact,
                    "phải CHÍNH XÁC với giả thiết hộp: chính Unknown, vét cạn 'sai mọi nơi'\n{}",
                    ctx()
                );
            } else {
                st.unknown_both += 1;
            }
        }
        (_, Verdict::Unknown) => st.unknown_both += 1,
    }
}

fn boxes(ty: Ty) -> Vec<(i128, i128)> {
    let (lo, hi) = (
        ty.min().to_i128().unwrap_or(0),
        ty.max().to_i128().unwrap_or(0),
    );
    let mut v = Vec::new();
    for a in lo..=hi {
        for b in a..=hi {
            v.push((a, b));
        }
    }
    v
}

fn box_hyps(var: u32, b: (i128, i128), ty: Ty) -> Vec<Prop> {
    let mut h = Vec::new();
    if b.0 > ty.min().to_i128().unwrap_or(0) {
        h.push(Prop::Le(Term::int(b.0), Term::var(var)));
    }
    if b.1 < ty.max().to_i128().unwrap_or(0) {
        h.push(Prop::Le(Term::var(var), Term::int(b.1)));
    }
    h
}

fn in_range(ty: Ty, t: Term) -> Prop {
    Prop::And(vec![
        Prop::Le(Term::Int(ty.min()), t.clone()),
        Prop::Le(t, Term::Int(ty.max())),
    ])
}

#[test]
fn arithmetic_and_comparison_obligations_are_exact_for_box_hypotheses() {
    let mut st = Stats::default();
    for bits in 2..=4u32 {
        for signed in [false, true] {
            let ty = Ty::new(bits, signed);
            let bx = boxes(ty);
            let mut seed = 0x9E37_79B9 + u64::from(bits);
            for (i, bx1) in bx.iter().enumerate() {
                for (j, bx2) in bx.iter().enumerate() {
                    // đủ cho bits ≤ 3; bits = 4: lấy mẫu (136² = 18k cặp × nhiều mục tiêu quá nặng)
                    if bits == 4 && !(xorshift(&mut seed).is_multiple_of(9)) && !(i == 0 || j == 0)
                    {
                        continue;
                    }
                    let mut hyps = box_hyps(1, *bx1, ty);
                    hyps.extend(box_hyps(2, *bx2, ty));
                    let x = Term::var(1);
                    let y = Term::var(2);
                    let vars = [(1, ty), (2, ty)];
                    for goal in [
                        in_range(ty, Term::add(x.clone(), y.clone())),
                        in_range(ty, Term::sub(x.clone(), y.clone())),
                        in_range(ty, Term::mul(x.clone(), y.clone())),
                        Prop::Ne(y.clone(), Term::int(0)),
                        Prop::Lt(x.clone(), y.clone()),
                        Prop::Le(x.clone(), y.clone()),
                        Prop::Eq(x.clone(), y.clone()),
                        Prop::Ne(x.clone(), y.clone()),
                    ] {
                        compare(&ob(&vars, goal, hyps.clone()), true, &mut st);
                    }
                    if signed {
                        // −x và MIN / −1
                        compare(
                            &ob(
                                &vars,
                                Prop::Ne(x.clone(), Term::Int(ty.min())),
                                hyps.clone(),
                            ),
                            true,
                            &mut st,
                        );
                        let g = Prop::Not(Box::new(Prop::And(vec![
                            Prop::Eq(x.clone(), Term::Int(ty.min())),
                            Prop::Eq(y.clone(), Term::int(-1)),
                        ])));
                        compare(&ob(&vars, g, hyps.clone()), true, &mut st);
                    }
                }
            }
            // ép kiểu mất thông tin: x thuộc kiểu RỘNG, đích hẹp hơn một bit
            if bits >= 3 {
                let narrow = Ty::new(bits - 1, signed);
                for bx1 in &bx {
                    compare(
                        &ob(
                            &[(1, ty)],
                            in_range(narrow, Term::var(1)),
                            box_hyps(1, *bx1, ty),
                        ),
                        true,
                        &mut st,
                    );
                }
            }
        }
    }
    eprintln!("{st:?}");
    assert!(st.both_proved > 1000 && st.both_refuted > 100, "{st:?}");
}

#[test]
fn zone_relations_are_sound_against_exhaustive_enumeration() {
    let mut st = Stats::default();
    let mut seed = 0xC0FF_EE11u64;
    for bits in 2..=4u32 {
        for signed in [false, true] {
            let ty = Ty::new(bits, signed);
            let (lo, hi) = (
                ty.min().to_i128().unwrap_or(0),
                ty.max().to_i128().unwrap_or(0),
            );
            let span = hi - lo;
            for _ in 0..1500 {
                let r = |seed: &mut u64, m: i128| {
                    lo + i128::from((xorshift(seed) % (m as u64 + 1)) as u32)
                };
                let c = -span + i128::from((xorshift(&mut seed) % (2 * span as u64 + 1)) as u32);
                let mut hyps = vec![Prop::Le(
                    Term::sub(Term::var(1), Term::var(2)),
                    Term::int(c),
                )];
                if xorshift(&mut seed).is_multiple_of(2) {
                    let k = r(&mut seed, span);
                    hyps.push(Prop::Le(Term::var(1), Term::int(k)));
                }
                if xorshift(&mut seed).is_multiple_of(2) {
                    let k = r(&mut seed, span);
                    hyps.push(Prop::Le(Term::int(k), Term::var(2)));
                }
                if xorshift(&mut seed).is_multiple_of(3) {
                    // quan hệ thứ hai x−z, z−y (đóng bao qua biến thứ ba)
                    let c2 =
                        -span + i128::from((xorshift(&mut seed) % (2 * span as u64 + 1)) as u32);
                    hyps.push(Prop::Le(
                        Term::sub(Term::var(2), Term::var(3)),
                        Term::int(c2),
                    ));
                }
                let x = Term::var(1);
                let y = Term::var(2);
                let z = Term::var(3);
                let k = r(&mut seed, span);
                let goals = [
                    Prop::Lt(x.clone(), y.clone()),
                    Prop::Le(x.clone(), y.clone()),
                    Prop::Eq(x.clone(), y.clone()),
                    Prop::Ne(x.clone(), y.clone()),
                    Prop::Lt(x.clone(), z.clone()),
                    Prop::Le(Term::add(x.clone(), Term::int(1)), y.clone()),
                    Prop::Le(Term::add(x.clone(), Term::int(1)), Term::Int(ty.max())),
                    Prop::Le(x.clone(), Term::int(k)),
                    Prop::And(vec![
                        Prop::Le(Term::Int(ty.min()), Term::sub(x.clone(), Term::int(1))),
                        Prop::Lt(x.clone(), y.clone()),
                    ]),
                ];
                for g in goals {
                    compare(
                        &ob(&[(1, ty), (2, ty), (3, ty)], g, hyps.clone()),
                        false,
                        &mut st,
                    );
                }
            }
        }
    }
    eprintln!("{st:?}");
    assert!(st.both_proved > 500, "{st:?}");
}

#[test]
fn random_boolean_combinations_are_sound() {
    let mut st = Stats::default();
    let mut seed = 0xDEAD_BEEFu64;
    let ty = Ty::new(3, true);
    let (lo, hi) = (
        ty.min().to_i128().unwrap_or(0),
        ty.max().to_i128().unwrap_or(0),
    );
    let span = (hi - lo) as u64;
    let mut rt = |seed: &mut u64| -> Term {
        match xorshift(seed) % 5 {
            0 => Term::var(1),
            1 => Term::var(2),
            2 => Term::add(Term::var(1), Term::var(2)),
            3 => Term::sub(Term::var(1), Term::int(1)),
            _ => Term::int(lo + i128::from((xorshift(seed) % (span + 1)) as u32)),
        }
    };
    let rp = |seed: &mut u64, rt: &mut dyn FnMut(&mut u64) -> Term| -> Prop {
        let (a, b) = (rt(seed), rt(seed));
        match xorshift(seed) % 5 {
            0 => Prop::Le(a, b),
            1 => Prop::Lt(a, b),
            2 => Prop::Eq(a, b),
            3 => Prop::Ne(a, b),
            _ => Prop::Not(Box::new(Prop::Lt(a, b))),
        }
    };
    for _ in 0..6000 {
        let mut hyps = Vec::new();
        for _ in 0..(xorshift(&mut seed) % 4) {
            let p = rp(&mut seed, &mut rt);
            hyps.push(if xorshift(&mut seed).is_multiple_of(4) {
                Prop::Or(vec![p, rp(&mut seed, &mut rt)])
            } else {
                p
            });
        }
        let g = {
            let p = rp(&mut seed, &mut rt);
            match xorshift(&mut seed) % 3 {
                0 => p,
                1 => Prop::And(vec![p, rp(&mut seed, &mut rt)]),
                _ => Prop::Or(vec![p, rp(&mut seed, &mut rt)]),
            }
        };
        compare(&ob(&[(1, ty), (2, ty)], g, hyps), false, &mut st);
    }
    eprintln!("{st:?}");
    assert!(st.cases >= 6000);
    let _ = I256::ZERO;
}
