//! ADR-P2-04: (1) test ĐỘC HẠI — mọi chứng chỉ sai/giả phải bị kernel TỪ CHỐI; (2) oracle soundness — nghĩa vụ tuyến tính sinh ngẫu nhiên trên biến
//! 3–4 bit: mỗi lần kernel CHẤP NHẬN chứng chỉ, bộ giải VÉT CẠN độc lập (gt-oblig, không chung mã với kernel) phải xác nhận không có phản ví dụ.
use gt_kernel::{CaseCert, FarkasCert, check, find_cert};
use gt_oblig::Ty;
use gt_oblig::ir::{
    Budget, Checkability, ObKind, Obligation, Prop, Prover, Term, VarDecl, Verdict,
};
use gt_oblig::prover::ExhaustiveProver;

fn ob(vars: &[(u32, u32, bool)], hyps: Vec<Prop>, goal: Prop) -> Obligation {
    Obligation {
        id: "t#0".into(),
        kind: ObKind::Index,
        body: "t".into(),
        block: 0,
        stmt: 0,
        span: (0, 0),
        vars: vars
            .iter()
            .map(|(id, bits, signed)| VarDecl {
                id: *id,
                name: format!("v{id}"),
                ty: Ty::new(*bits, *signed),
            })
            .collect(),
        atoms: Vec::new(),
        goal,
        hyps,
        runtime: Checkability::Runtime("test".into()),
    }
}

/// hệ số của một ca Farkas: (chỉ số ràng buộc, hệ số)
type Mults = Vec<(usize, i128)>;

fn v(i: u32) -> Term {
    Term::Var(i)
}

/// x ≤ 5 ⊢ x < 10 (x: u8)
fn simple() -> Obligation {
    ob(
        &[(0, 8, false)],
        vec![Prop::Le(v(0), Term::int(5))],
        Prop::Lt(v(0), Term::int(10)),
    )
}

#[test]
fn a_correct_certificate_is_found_and_accepted() {
    let o = simple();
    let c = find_cert(&o, &mut 10_000).expect("phải tìm được");
    assert_eq!(check(&o, &c), Ok(()));
    // mã hoá ⇄ giải mã giữ nguyên
    assert_eq!(FarkasCert::decode(&c.encode()), Ok(c));
}

#[test]
fn forged_or_mismatched_certificates_are_rejected() {
    let o = simple();
    let good = find_cert(&o, &mut 10_000).expect("phải tìm được");
    let mut bad = Vec::new();
    // hệ số âm
    let CaseCert::Farkas(m0) = good.cases[0].clone() else {
        panic!("ca 0 phải là Farkas")
    };
    let with = |f: &dyn Fn(&mut Mults)| {
        let mut m = m0.clone();
        f(&mut m);
        FarkasCert {
            cases: vec![CaseCert::Farkas(m)],
        }
    };
    bad.push(("hệ số âm", with(&|m| m[0].1 = -m[0].1)));
    bad.push((
        "thiếu ràng buộc",
        with(&|m| {
            m.pop();
        }),
    ));
    bad.push(("hệ số lệch", with(&|m| m[0].1 *= 2)));
    bad.push(("chỉ số ngoài ca", with(&|m| m[0].0 = 999)));
    bad.push(("tràn", with(&|m| m[0].1 = i128::MAX)));
    bad.push((
        "mâu thuẫn nguyên tử bịa",
        FarkasCert {
            cases: vec![CaseCert::Atom(0)],
        },
    ));
    // thừa / thiếu ca
    let mut c = good.clone();
    c.cases.push(CaseCert::Farkas(vec![]));
    bad.push(("thừa ca", c));
    bad.push(("không ca nào", FarkasCert { cases: vec![] }));
    // chứng chỉ rỗng cho ca (tổng 0 ≤ 0 không mâu thuẫn)
    bad.push((
        "ca rỗng",
        FarkasCert {
            cases: vec![CaseCert::Farkas(vec![])],
        },
    ));
    for (why, c) in bad {
        assert!(
            check(&o, &c).is_err(),
            "{why}: kernel CHẤP NHẬN chứng chỉ sai {c:?}"
        );
    }
    // chứng chỉ đúng của nghĩa vụ này đem dùng cho nghĩa vụ SAI (x ≤ 5 ⊢ x < 3) ⇒ từ chối
    let wrong = ob(
        &[(0, 8, false)],
        vec![Prop::Le(v(0), Term::int(5))],
        Prop::Lt(v(0), Term::int(3)),
    );
    assert!(check(&wrong, &good).is_err());
    assert!(find_cert(&wrong, &mut 10_000).is_none_or(|c| check(&wrong, &c).is_err()));
    // giải mã chặt
    for s in [
        "",
        "farkas:v2:0*1",
        "farkas:v1:0*",
        "farkas:v1:a*1",
        "farkas:v1:0*1,",
        "farkas:v1:0*-x",
        "farkas:v1:@x",
    ] {
        assert!(
            FarkasCert::decode(s).is_err(),
            "giải mã nhận chuỗi sai {s:?}"
        );
    }
}

#[test]
fn what_the_kernel_does_not_understand_only_weakens_never_proves_more() {
    // x*y là biến mờ: x ≤ 5, y ≤ 5 ⊬ x*y ≤ 25 bằng Farkas (cần phi tuyến) — không được "chứng minh"
    let o = ob(
        &[(0, 8, false), (1, 8, false)],
        vec![Prop::Le(v(0), Term::int(5)), Prop::Le(v(1), Term::int(5))],
        Prop::Le(Term::Mul(Box::new(v(0)), Box::new(v(1))), Term::int(25)),
    );
    assert!(find_cert(&o, &mut 10_000).is_none_or(|c| check(&o, &c).is_err()));
    // nguyên tử logic trong mục tiêu: không được chứng minh từ không có gì, cũng không từ nguyên tử KHÁC
    let o = ob(&[(0, 8, false)], vec![], Prop::Atom(7));
    assert!(find_cert(&o, &mut 10_000).is_none_or(|c| check(&o, &c).is_err()));
    let o = ob(&[(0, 8, false)], vec![Prop::Atom(6)], Prop::Atom(7));
    assert!(find_cert(&o, &mut 10_000).is_none_or(|c| check(&o, &c).is_err()));
    assert!(
        check(
            &o,
            &FarkasCert {
                cases: vec![CaseCert::Atom(7)]
            }
        )
        .is_err()
    );
    // nhưng `a ⊢ a` thì được (mâu thuẫn a ∧ ¬a)
    let o = ob(&[(0, 8, false)], vec![Prop::Atom(7)], Prop::Atom(7));
    let c = find_cert(&o, &mut 10_000).expect("a ⊢ a");
    assert_eq!(check(&o, &c), Ok(()));
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn coef(&mut self) -> i128 {
        self.below(7) as i128 - 3
    }
}

fn term(r: &mut Rng, nv: u32) -> Term {
    let mut t = Term::int(r.below(9) as i128 - 4);
    for i in 0..nv {
        let c = r.coef();
        if c != 0 {
            t = Term::Add(
                Box::new(t),
                Box::new(Term::Mul(Box::new(Term::int(c)), Box::new(v(i)))),
            );
        }
    }
    t
}

fn atom(r: &mut Rng, nv: u32) -> Prop {
    let (a, b) = (term(r, nv), term(r, nv));
    match r.below(4) {
        0 => Prop::Le(a, b),
        1 => Prop::Lt(a, b),
        2 => Prop::Eq(a, b),
        _ => Prop::Ne(a, b),
    }
}

fn prop(r: &mut Rng, nv: u32) -> Prop {
    match r.below(8) {
        // nguyên tử logic (luật mâu thuẫn `a ∧ ¬a` của kernel)
        6 => Prop::Atom(100 + r.below(2) as u32),
        7 => Prop::Not(Box::new(Prop::Atom(100 + r.below(2) as u32))),
        0 => Prop::Or(vec![atom(r, nv), atom(r, nv)]),
        1 => Prop::Not(Box::new(atom(r, nv))),
        2 => Prop::And(vec![atom(r, nv), atom(r, nv)]),
        _ => atom(r, nv),
    }
}

#[test]
fn every_kernel_accepted_certificate_agrees_with_exhaustive_enumeration() {
    let (mut accepted, mut exhaustive_proved, mut total) = (0u32, 0u32, 0u32);
    for seed in 1..=3000u64 {
        let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let nv = 1 + r.below(3) as u32;
        let vars: Vec<(u32, u32, bool)> = (0..nv)
            .map(|i| (i, 3 + r.below(2) as u32, r.below(2) == 0))
            .collect();
        let hyps: Vec<Prop> = (0..r.below(4)).map(|_| prop(&mut r, nv)).collect();
        let goal = prop(&mut r, nv);
        let o = ob(&vars, hyps, goal);
        total += 1;
        let ex = ExhaustiveProver::default().prove(&o, &mut Budget::new(1 << 20));
        let ex_proved = matches!(ex, Verdict::Proved(_));
        if ex_proved {
            exhaustive_proved += 1;
        }
        if let Some(c) = find_cert(&o, &mut 20_000)
            && check(&o, &c).is_ok()
        {
            accepted += 1;
            assert!(
                ex_proved,
                "#{seed}: kernel chấp nhận nhưng vét cạn có phản ví dụ: {ex:?}\n{o:?}\n{c:?}"
            );
        }
    }
    eprintln!("tổng {total}, vét cạn chứng minh {exhaustive_proved}, kernel chấp nhận {accepted}");
    // chống bộ sinh rỗng: phải có đủ ca chứng minh được để phép thử có nghĩa
    assert!(
        accepted >= 300,
        "quá ít chứng chỉ được chấp nhận ({accepted}/{total}) — bộ sinh hoặc bộ tìm hỏng?"
    );
}
