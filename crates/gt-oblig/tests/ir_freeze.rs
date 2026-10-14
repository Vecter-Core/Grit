//! ADR-039: đóng băng Obligation IR. (1) Ba nghĩa vụ điển hình của Phase 2 biểu diễn được và tuần tự hoá CÓ CẤU TRÚC, tất định; (2) quy tắc chỉ-thêm-khoá:
//! văn bản JSON cũ là TIỀN TỐ của văn bản mới; (3) các nút mới (`Forall/Exists/Pred/Uf`) KHÔNG BAO GIỜ được hai bộ giải hiện có "chứng minh" (fail-closed);
//! (4) ca (c) `i < xs.len()` kèm giả thiết là nghĩa vụ M7 dùng hằng ngày: cả hai bộ giải chứng minh được.
use gt_oblig::interval::Ty;
use gt_oblig::ir::{
    Binder, Budget, Checkability, ObKind, Obligation, Prop, Prover, Term, VarDecl, Verdict,
};
use gt_oblig::prover::{ExhaustiveProver, ZoneProver};

fn usize_ty() -> Ty {
    Ty::new(64, false)
}

fn ob(vars: &[(u32, &str, Ty)], goal: Prop, hyps: Vec<Prop>) -> Obligation {
    Obligation {
        id: "t::f#0".into(),
        kind: ObKind::Invariant,
        body: "t::f".into(),
        block: 1,
        stmt: 0,
        span: (0, 0),
        vars: vars
            .iter()
            .map(|(id, n, ty)| VarDecl {
                id: *id,
                name: (*n).to_string(),
                ty: *ty,
            })
            .collect(),
        atoms: vec![],
        goal,
        hyps,
        runtime: Checkability::NotCheckable("Phase 2".into()),
    }
}

fn uf(n: &str, args: Vec<Term>) -> Term {
    Term::Uf(n.into(), args)
}

fn pred(n: &str, args: Vec<Term>) -> Prop {
    Prop::Pred(n.into(), args)
}

/// (a) bất biến vòng lặp có lượng từ: `∀ j ∈ [0, i): a[j] ≤ a[j+1]` và `∃ j ∈ [0, n): a[j] == x`
fn paper_a() -> Vec<Obligation> {
    let (a, i, n, x, j) = (1, 2, 3, 4, 10);
    let sel = |idx: Term| uf("select", vec![Term::var(a), idx]);
    let sorted = Prop::Forall(
        Box::new(Binder {
            var: j,
            lo: Term::int(0),
            hi: Term::var(i),
            hi_inclusive: false,
        }),
        Box::new(Prop::Le(
            sel(Term::var(j)),
            sel(Term::add(Term::var(j), Term::int(1))),
        )),
    );
    let found = Prop::Exists(
        Box::new(Binder {
            var: j,
            lo: Term::int(0),
            hi: Term::var(n),
            hi_inclusive: false,
        }),
        Box::new(Prop::Eq(sel(Term::var(j)), Term::var(x))),
    );
    let vars = [
        (a, "a", usize_ty()),
        (i, "i", usize_ty()),
        (n, "n", usize_ty()),
        (x, "x", usize_ty()),
    ];
    vec![
        ob(&vars, sorted, vec![Prop::Le(Term::var(i), Term::var(n))]),
        ob(&vars, found, vec![Prop::Lt(Term::int(0), Term::var(n))]),
    ]
}

/// (b) hợp lệ con trỏ cho `Vec::push` nhánh `len < cap`: ghi tại `ptr + len*size` hợp lệ
fn paper_b() -> Obligation {
    let (ptr, len, cap, size) = (1, 2, 3, 4);
    let off = Term::mul(Term::var(len), Term::var(size));
    ob(
        &[
            (ptr, "ptr", usize_ty()),
            (len, "len", usize_ty()),
            (cap, "cap", usize_ty()),
            (size, "size_of_T", usize_ty()),
        ],
        pred("valid_write", vec![Term::var(ptr), off, Term::var(size)]),
        vec![
            Prop::Le(Term::var(len), Term::var(cap)),
            Prop::Lt(Term::var(len), Term::var(cap)),
            pred(
                "valid_range",
                vec![Term::var(ptr), Term::mul(Term::var(cap), Term::var(size))],
            ),
            Prop::Eq(uf("size_of", vec![Term::int(0)]), Term::var(size)),
        ],
    )
}

/// (c) `i < ys.len()` kèm giả thiết `0 ≤ i`, `i < xs.len()`, `xs.len() == ys.len()` (độ dài là biến nguyên)
fn paper_c(ty: Ty) -> Obligation {
    let (i, lx, ly) = (1, 2, 3);
    ob(
        &[(i, "i", ty), (lx, "xs.len", ty), (ly, "ys.len", ty)],
        Prop::Lt(Term::var(i), Term::var(ly)),
        vec![
            Prop::Le(Term::int(0), Term::var(i)),
            Prop::Lt(Term::var(i), Term::var(lx)),
            Prop::Eq(Term::var(lx), Term::var(ly)),
        ],
    )
}

#[test]
fn paper_exercise_a_quantified_invariants_serialize_structurally() {
    let obs = paper_a();
    let j = obs[0].to_json();
    assert!(
        j.contains("\"goal_ast\":{\"op\":\"forall\",\"var\":10,\"lo\":{\"op\":\"int\",\"v\":\"0\"},\"hi\":{\"op\":\"var\",\"id\":2},\"hi_incl\":false,\"body\":{\"op\":\"le\""),
        "{j}"
    );
    assert!(j.contains("\"op\":\"uf\",\"name\":\"select\""), "{j}");
    let k = obs[1].to_json();
    assert!(
        k.contains("\"goal_ast\":{\"op\":\"exists\",\"var\":10"),
        "{k}"
    );
    // văn bản cho người đọc vẫn có
    assert!(
        j.contains("\"goal\":\"forall v10 in [0, v2): select(v1, v10) <= select(v1, (v10 + 1))\""),
        "{j}"
    );
    // tất định
    assert_eq!(j, paper_a()[0].to_json());
    assert_eq!(k, paper_a()[1].to_json());
}

#[test]
fn paper_exercise_b_pointer_validity_serializes_structurally() {
    let o = paper_b();
    let j = o.to_json();
    assert!(
        j.contains("\"goal_ast\":{\"op\":\"pred\",\"name\":\"valid_write\",\"args\":[{\"op\":\"var\",\"id\":1},{\"op\":\"mul\""),
        "{j}"
    );
    assert!(j.contains("\"hyps_ast\":[{\"op\":\"le\""), "{j}");
    assert!(
        j.contains("{\"op\":\"pred\",\"name\":\"valid_range\""),
        "{j}"
    );
    assert!(
        j.contains("{\"op\":\"uf\",\"name\":\"size_of\",\"args\":[{\"op\":\"int\",\"v\":\"0\"}]}"),
        "{j}"
    );
    assert_eq!(j, paper_b().to_json());
}

#[test]
fn paper_exercise_c_length_relation_is_proved_by_both_provers_without_any_change() {
    // ZoneProver: kiểu usize đầy đủ
    let o = paper_c(usize_ty());
    let v = ZoneProver::new(0).prove(&o, &mut Budget::new(100_000));
    assert!(matches!(v, Verdict::Proved(_)), "{v:?}");
    // ExhaustiveProver: kiểu nhỏ (4 bit) để liệt kê được; kết quả phải CÙNG: chứng minh
    let small = paper_c(Ty::new(4, false));
    let v = ExhaustiveProver::default().prove(&small, &mut Budget::new(100_000));
    assert!(matches!(v, Verdict::Proved(_)), "{v:?}");
    // đối chứng: bỏ giả thiết `xs.len == ys.len` ⇒ KHÔNG chứng minh được (và vét cạn thấy phản ví dụ)
    let mut weak = paper_c(Ty::new(4, false));
    weak.hyps.pop();
    let v = ExhaustiveProver::default().prove(&weak, &mut Budget::new(100_000));
    assert!(matches!(v, Verdict::Refuted { .. }), "{v:?}");
    let v = ZoneProver::new(0).prove(&weak, &mut Budget::new(100_000));
    assert!(!matches!(v, Verdict::Proved(_)), "{v:?}");
}

#[test]
fn new_nodes_are_never_proved_by_the_existing_provers_fail_closed() {
    let mut all: Vec<Obligation> = paper_a();
    all.push(paper_b());
    // mục tiêu chứa nút mới: PHẢI Unknown với cả hai bộ giải, cho dù giả thiết nào
    for o in &all {
        let z = ZoneProver::new(0).prove(o, &mut Budget::new(100_000));
        assert!(
            matches!(z, Verdict::Unknown),
            "Zone: {z:?}\n{}",
            o.to_json()
        );
        let e = ExhaustiveProver::default().prove(o, &mut Budget::new(100_000));
        assert!(
            matches!(e, Verdict::Unknown),
            "Exhaustive: {e:?}\n{}",
            o.to_json()
        );
    }
    // nút mới LẶN trong `Not`/`Or`/`And` cũng không được làm ra `Proved`
    let nested = ob(
        &[(1, "x", usize_ty())],
        Prop::Or(vec![
            Prop::Not(Box::new(pred("valid", vec![Term::var(1)]))),
            pred("valid", vec![Term::var(1)]),
        ]),
        vec![],
    );
    let z = ZoneProver::new(0).prove(&nested, &mut Budget::new(100_000));
    assert!(!matches!(z, Verdict::Proved(_)), "Zone: {z:?}");
    let e = ExhaustiveProver::default().prove(&nested, &mut Budget::new(100_000));
    assert!(!matches!(e, Verdict::Proved(_)), "Exhaustive: {e:?}");
    // giả thiết chứa nút mới mà mục tiêu số học thuần vẫn đúng: không được `Refuted` (giả thiết lạ không phải bằng chứng sai)
    let mut c = paper_c(usize_ty());
    c.hyps.push(pred("opaque", vec![Term::var(1)]));
    let z = ZoneProver::new(0).prove(&c, &mut Budget::new(100_000));
    assert!(!matches!(z, Verdict::Refuted { .. }), "Zone: {z:?}");
    let e = ExhaustiveProver::default().prove(&c, &mut Budget::new(100_000));
    assert!(!matches!(e, Verdict::Refuted { .. }), "Exhaustive: {e:?}");
}

#[test]
fn old_json_text_is_a_prefix_of_the_new_text_only_keys_were_appended() {
    let o = Obligation {
        id: "main::f#0".into(),
        kind: ObKind::Overflow,
        body: "main::f".into(),
        block: 1,
        stmt: 2,
        span: (3, 9),
        vars: vec![VarDecl {
            id: 1,
            name: "x\"q".into(),
            ty: Ty::new(8, true),
        }],
        atoms: vec![],
        goal: Prop::Le(Term::var(1), Term::int(127)),
        hyps: vec![Prop::Le(Term::int(0), Term::var(1))],
        runtime: Checkability::Runtime("assert:overflow".into()),
    };
    // văn bản trước ADR-039 (đóng đinh nguyên văn)
    let old = "{\"id\":\"main::f#0\",\"kind\":\"overflow\",\"body\":\"main::f\",\"block\":1,\"stmt\":2,\"span\":[3,9],\"vars\":[{\"id\":1,\"name\":\"x\\\"q\",\"ty\":\"i8\"}],\"atoms\":[],\"goal\":\"v1 <= 127\",\"hyps\":[\"0 <= v1\"],\"runtime\":{\"checkable\":true,\"via\":\"assert:overflow\"}";
    let new = o.to_json();
    assert!(new.starts_with(old), "khoá cũ bị đổi:\n{new}");
    assert!(
        new[old.len()..].starts_with(",\"goal_ast\":{\"op\":\"le\""),
        "khoá mới phải được NỐI vào cuối: {}",
        &new[old.len()..]
    );
    assert!(new.ends_with("]}"), "{new}");
}

#[test]
fn budget_is_counted_in_steps_and_a_solver_with_no_budget_gives_unknown_not_proved() {
    let o = paper_c(usize_ty());
    let v = ZoneProver::new(0).prove(&o, &mut Budget::new(0));
    assert!(!matches!(v, Verdict::Proved(_)), "{v:?}");
    let mut b = Budget::new(10);
    assert!(b.spend(10));
    assert!(!b.spend(1));
    assert_eq!(b.steps, 0);
}

/// Bắt đột biến "nút lạ được đánh giá như hằng/đúng": dùng kiểu NHỎ (4 bit) để `ExhaustiveProver` THỰC SỰ liệt kê (kiểu 64 bit vượt `max_points` nên luôn `Unknown`
/// và che lỗi), và mục tiêu mà một cách đánh giá sai (Uf = 0, Pred = đúng, Forall = đúng) sẽ làm ra `Proved`/`Refuted` sai.
#[test]
fn unknown_nodes_are_not_evaluated_as_constants_even_when_the_domain_is_small() {
    let small = Ty::new(4, false);
    let vars = [(1, "x", small)];
    let f = || uf("f", vec![Term::var(1)]);
    let goals = vec![
        // Uf không được coi là 0: `f(x) <= 0` KHÔNG đúng với mọi gán
        Prop::Le(f(), Term::int(0)),
        // Uf trong dạng tuyến tính không được biến mất: `x + f(x) <= x`
        Prop::Le(Term::add(Term::var(1), f()), Term::var(1)),
        // Uf không bằng 0 và không khác 0 một cách tuỳ tiện
        Prop::Eq(f(), Term::int(0)),
        Prop::Ne(f(), f()),
        // Pred / Forall / Exists không được coi là đúng
        pred("p", vec![Term::var(1)]),
        Prop::Forall(
            Box::new(Binder {
                var: 9,
                lo: Term::int(0),
                hi: Term::int(2),
                hi_inclusive: false,
            }),
            Box::new(Prop::False),
        ),
        Prop::Exists(
            Box::new(Binder {
                var: 9,
                lo: Term::int(0),
                hi: Term::int(2),
                hi_inclusive: false,
            }),
            Box::new(Prop::False),
        ),
    ];
    for g in goals {
        let o = ob(&vars, g, vec![]);
        for (name, v) in [
            (
                "zone",
                ZoneProver::new(0).prove(&o, &mut Budget::new(100_000)),
            ),
            (
                "exhaustive",
                ExhaustiveProver::default().prove(&o, &mut Budget::new(100_000)),
            ),
        ] {
            assert!(
                matches!(v, Verdict::Unknown),
                "{name} phải Unknown cho nút lạ, được {v:?}
{}",
                o.to_json()
            );
        }
    }
    // giả thiết chứa nút lạ không được làm trạng thái "chết" (nếu chết thì chứng minh được mọi thứ): mục tiêu SAI vẫn không được `Proved`
    for h in [
        pred("p", vec![Term::var(1)]),
        Prop::Exists(
            Box::new(Binder {
                var: 9,
                lo: Term::int(0),
                hi: Term::int(2),
                hi_inclusive: false,
            }),
            Box::new(Prop::False),
        ),
    ] {
        let o = ob(&vars, Prop::Lt(Term::var(1), Term::int(0)), vec![h]);
        let z = ZoneProver::new(0).prove(&o, &mut Budget::new(100_000));
        assert!(!matches!(z, Verdict::Proved(_)), "zone: {z:?}");
        let e = ExhaustiveProver::default().prove(&o, &mut Budget::new(100_000));
        assert!(!matches!(e, Verdict::Proved(_)), "exhaustive: {e:?}");
    }
}
