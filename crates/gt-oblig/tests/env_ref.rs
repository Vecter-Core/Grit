//! `Env` (miền khoảng + zone + nguyên tử) kiểm TRỰC TIẾP, không qua bộ giải (phát hiện bằng đột biến, ADR-025 §8): ngữ nghĩa của
//! `assume` cho từng dạng mệnh đề, phát hiện mâu thuẫn (`dead`), lan truyền khoảng↔zone, `assign`, `join/widen/leq` với trạng thái chết
//! và nguyên tử. Mỗi ca khẳng định cận CHÍNH XÁC (không chỉ "sound") để đột biến làm lỏng/siết sai bị bắt.
use gt_oblig::I256;
use gt_oblig::env::Env;
use gt_oblig::interval::{B, Interval, Tri, Ty};
use gt_oblig::ir::{Prop, Term};

fn u4() -> Ty {
    Ty::new(4, false)
}
fn i4() -> Ty {
    Ty::new(4, true)
}
fn iv(a: i128, b: i128) -> Interval {
    Interval::new(B::from_i128(a), B::from_i128(b))
}
fn x() -> Term {
    Term::var(1)
}
fn y() -> Term {
    Term::var(2)
}
fn k(v: i128) -> Term {
    Term::int(v)
}
/// môi trường với x, y ∈ u4; `zone`: x, y là biến zone
fn env2(zone: bool) -> Env {
    let zs: Vec<u32> = if zone { vec![1, 2] } else { vec![] };
    let mut e = Env::new(&zs);
    e.declare(1, u4());
    e.declare(2, u4());
    e
}
fn dead_after(p: Prop, pos: bool) -> bool {
    let mut e = env2(true);
    e.assume(&p, pos);
    e.is_dead()
}

#[test]
fn constants_decide_liveness() {
    assert!(!dead_after(Prop::True, true));
    assert!(dead_after(Prop::True, false));
    assert!(dead_after(Prop::False, true));
    assert!(!dead_after(Prop::False, false));
}

#[test]
fn negated_comparisons_are_strict_or_not_correctly() {
    // ¬(x ≤ 5) ⇒ x ≥ 6 ; ¬(x < 5) ⇒ x ≥ 5
    let mut e = env2(true);
    e.assume(&Prop::Le(x(), k(5)), false);
    assert_eq!(e.interval(1), iv(6, 15));
    let mut e = env2(true);
    e.assume(&Prop::Lt(x(), k(5)), false);
    assert_eq!(e.interval(1), iv(5, 15));
    // dạng dương
    let mut e = env2(true);
    e.assume(&Prop::Le(x(), k(5)), true);
    assert_eq!(e.interval(1), iv(0, 5));
    let mut e = env2(true);
    e.assume(&Prop::Lt(x(), k(5)), true);
    assert_eq!(e.interval(1), iv(0, 4));
    // đối xứng: 5 ≤ x và ¬(5 < x)
    let mut e = env2(true);
    e.assume(&Prop::Le(k(5), x()), true);
    assert_eq!(e.interval(1), iv(5, 15));
    let mut e = env2(true);
    e.assume(&Prop::Lt(k(5), x()), false);
    assert_eq!(e.interval(1), iv(0, 5));
}

#[test]
fn equality_and_disequality() {
    let mut e = env2(true);
    e.assume(&Prop::Eq(x(), k(5)), true);
    assert_eq!(e.interval(1), iv(5, 5));
    let mut e = env2(true);
    e.assume(&Prop::Ne(x(), k(5)), false); // ¬(x ≠ 5) ⇔ x = 5
    assert_eq!(e.interval(1), iv(5, 5));
    let mut e = env2(true);
    e.assume(&Prop::Eq(x(), k(5)), false); // x ≠ 5: không cắt giữa khoảng
    assert_eq!(e.interval(1), iv(0, 15));
    // x ≠ cận ⇒ cắt cận
    let mut e = env2(true);
    e.assume(&Prop::Ne(x(), k(0)), true);
    assert_eq!(e.interval(1), iv(1, 15));
    let mut e = env2(true);
    e.assume(&Prop::Ne(x(), k(15)), true);
    assert_eq!(e.interval(1), iv(0, 14));
    // hệ số âm: −x ≠ −15 ⇔ x ≠ 15 ; −x ≠ 0 ⇔ x ≠ 0
    let mut e = env2(true);
    e.assume(&Prop::Ne(Term::Neg(Box::new(x())), k(-15)), true);
    assert_eq!(e.interval(1), iv(0, 14));
    let mut e = env2(true);
    e.assume(&Prop::Ne(Term::Neg(Box::new(x())), k(0)), true);
    assert_eq!(e.interval(1), iv(1, 15));
    // hằng khác hằng, hay một vế bằng chính vế kia ⇒ mâu thuẫn
    assert!(dead_after(Prop::Ne(k(4), k(4)), true));
    assert!(!dead_after(Prop::Ne(k(4), k(5)), true));
    let mut e = env2(true);
    e.assume(&Prop::Eq(x(), k(4)), true);
    e.assume(&Prop::Ne(x(), k(4)), true);
    assert!(e.is_dead());
    // x ≠ x (cùng biểu thức, kể cả x+1 ≠ x+1) ⇒ mâu thuẫn dù x chưa biết
    assert!(dead_after(Prop::Ne(x(), x()), true));
    assert!(dead_after(
        Prop::Ne(Term::add(x(), k(1)), Term::add(x(), k(1))),
        true
    ));
    assert!(!dead_after(
        Prop::Ne(Term::add(x(), k(1)), Term::add(x(), k(2))),
        true
    ));
    // strict không được lẫn vào phát hiện bằng-nhau: x ∈ [4,4], y ∈ [4,4] ⇒ x ≠ y mâu thuẫn; x ∈ [3,4], y = 4 thì không
    let mut e = env2(false);
    e.assume(&Prop::Eq(x(), k(4)), true);
    e.assume(&Prop::Eq(y(), k(4)), true);
    e.assume(&Prop::Ne(x(), y()), true);
    assert!(e.is_dead());
    let mut e = env2(false);
    e.assume(&Prop::Le(k(3), x()), true);
    e.assume(&Prop::Le(x(), k(4)), true);
    e.assume(&Prop::Eq(y(), k(4)), true);
    e.assume(&Prop::Ne(x(), y()), true);
    assert!(!e.is_dead());
}

#[test]
fn boolean_structure_of_assumptions() {
    // ¬(a ∨ b) ⇒ ¬a ∧ ¬b
    let mut e = env2(true);
    e.assume(
        &Prop::Or(vec![Prop::Lt(x(), k(3)), Prop::Lt(y(), k(4))]),
        false,
    );
    assert_eq!(e.interval(1), iv(3, 15));
    assert_eq!(e.interval(2), iv(4, 15));
    // a ∧ b
    let mut e = env2(true);
    e.assume(
        &Prop::And(vec![Prop::Lt(x(), k(3)), Prop::Lt(y(), k(4))]),
        true,
    );
    assert_eq!(e.interval(1), iv(0, 2));
    assert_eq!(e.interval(2), iv(0, 3));
    // ¬(a ∧ b) ⇒ ¬a ∨ ¬b: nếu a chắc chắn đúng thì ¬b
    let mut e = env2(true);
    e.assume(&Prop::Le(x(), k(2)), true);
    e.assume(
        &Prop::And(vec![Prop::Lt(x(), k(3)), Prop::Lt(y(), k(4))]),
        false,
    );
    assert_eq!(e.interval(2), iv(4, 15));
    // a ∨ b với a chắc chắn sai ⇒ b
    let mut e = env2(true);
    e.assume(&Prop::Le(k(10), x()), true);
    e.assume(
        &Prop::Or(vec![Prop::Lt(x(), k(3)), Prop::Lt(y(), k(4))]),
        true,
    );
    assert_eq!(e.interval(2), iv(0, 3));
    // a ∨ b mà cả hai chắc chắn sai ⇒ chết
    let mut e = env2(true);
    e.assume(&Prop::Le(k(10), x()), true);
    e.assume(&Prop::Le(k(10), y()), true);
    e.assume(
        &Prop::Or(vec![Prop::Lt(x(), k(3)), Prop::Lt(y(), k(4))]),
        true,
    );
    assert!(e.is_dead());
    // a ∨ b chưa quyết được: chỉ giữ thông tin CHUNG (x < 8 ở cả hai nhánh)
    let mut e = env2(true);
    e.assume(
        &Prop::Or(vec![
            Prop::And(vec![Prop::Lt(x(), k(8)), Prop::Lt(y(), k(2))]),
            Prop::And(vec![Prop::Lt(x(), k(6)), Prop::Lt(y(), k(9))]),
        ]),
        true,
    );
    assert_eq!(e.interval(1), iv(0, 7));
    assert_eq!(e.interval(2), iv(0, 8));
    // ¬¬a
    let mut e = env2(true);
    e.assume(
        &Prop::Not(Box::new(Prop::Not(Box::new(Prop::Lt(x(), k(3)))))),
        true,
    );
    assert_eq!(e.interval(1), iv(0, 2));
}

#[test]
fn contradictions_are_detected_by_intervals_zone_and_atoms() {
    // khoảng
    let mut e = env2(false);
    e.assume(&Prop::Le(x(), k(2)), true);
    e.assume(&Prop::Le(k(5), x()), true);
    assert!(e.is_dead());
    let mut e = env2(false);
    e.assume(&Prop::Le(k(5), x()), true);
    e.assume(&Prop::Le(x(), k(2)), true);
    assert!(e.is_dead());
    // tĩnh sai ngay từ đầu: x ∈ [8,9] rồi giả thiết x ≤ 5
    let mut e = env2(false);
    e.assume(&Prop::Le(k(8), x()), true);
    e.assume(&Prop::Lt(x(), k(10)), true);
    e.assume(&Prop::Le(x(), k(5)), true);
    assert!(e.is_dead());
    // hằng so với hằng
    assert!(dead_after(Prop::Le(k(5), k(3)), true));
    assert!(dead_after(Prop::Lt(k(3), k(3)), true));
    assert!(!dead_after(Prop::Le(k(3), k(3)), true));
    // zone: chu trình âm
    let mut e = env2(true);
    e.assume(&Prop::Lt(x(), y()), true);
    e.assume(&Prop::Le(y(), x()), true);
    assert!(e.is_dead());
    // nguyên tử xung đột
    let mut e = env2(true);
    e.assume(&Prop::Atom(7), true);
    assert!(!e.is_dead());
    assert_eq!(e.atom(7), Some(true));
    e.assume(&Prop::Atom(7), true);
    assert!(!e.is_dead());
    e.assume(&Prop::Atom(7), false);
    assert!(e.is_dead());
    let mut e = env2(true);
    e.assume(&Prop::Atom(7), false);
    assert_eq!(e.atom(7), Some(false));
    e.forget_atom(7);
    assert_eq!(e.atom(7), None);
    e.set_atom(8, true);
    assert_eq!(e.atom(8), Some(true));
}

#[test]
fn intervals_and_zone_propagate_into_each_other() {
    // x − y ≤ −1 (zone), y ≤ 3 (khoảng) ⇒ x ≤ 2
    let mut e = env2(true);
    e.assume(&Prop::Le(Term::sub(x(), y()), k(-1)), true);
    e.assume(&Prop::Le(y(), k(3)), true);
    assert_eq!(e.interval(1), iv(0, 2));
    // x ≥ 4 ⇒ y ≥ 5
    let mut e = env2(true);
    e.assume(&Prop::Le(Term::sub(x(), y()), k(-1)), true);
    e.assume(&Prop::Le(k(4), x()), true);
    assert_eq!(e.interval(2), iv(5, 15));
    // quan hệ đồng thời với phép gán: y := x + 2 ⇒ y − x ≤ 2 và x − y ≤ −2
    let mut e = env2(true);
    e.assign(2, &Term::add(x(), k(2)));
    assert_eq!(e.zone_bound(2, 1), Some(2));
    assert_eq!(e.zone_bound(1, 2), Some(-2));
    assert_eq!(e.interval(2), iv(2, 15));
    assert_eq!(e.interval(1), iv(0, 13));
    // x := x + 1 dịch quan hệ
    e.assign(1, &Term::add(x(), k(1)));
    assert_eq!(e.zone_bound(2, 1), Some(1));
    // quên một biến xoá quan hệ và đưa về khoảng của kiểu
    e.forget(2);
    assert_eq!(e.zone_bound(2, 1), None);
    assert_eq!(e.interval(2), iv(0, 15));
}

#[test]
fn nonlinear_and_non_zone_comparisons_use_intervals_exactly() {
    // x < y*z, với y, z ∈ [1,3]: x ≤ 8
    let mut e = Env::new(&[]);
    for v in [1, 2, 3] {
        e.declare(v, u4());
    }
    e.assume(&Prop::Le(k(1), y()), true);
    e.assume(&Prop::Le(y(), k(3)), true);
    e.assume(&Prop::Le(k(1), Term::var(3)), true);
    e.assume(&Prop::Le(Term::var(3), k(3)), true);
    let prod = Term::mul(y(), Term::var(3));
    let mut a = e.clone();
    a.assume(&Prop::Lt(x(), prod.clone()), true);
    assert_eq!(a.interval(1), iv(0, 8));
    // y*z < x ⇒ x ≥ 2
    let mut b = e.clone();
    b.assume(&Prop::Lt(prod.clone(), x()), true);
    assert_eq!(b.interval(1), iv(2, 15));
    // không chặt: x ≤ y*z ⇒ x ≤ 9 ; y*z ≤ x ⇒ x ≥ 1
    let mut c = e.clone();
    c.assume(&Prop::Le(x(), prod.clone()), true);
    assert_eq!(c.interval(1), iv(0, 9));
    let mut d = e.clone();
    d.assume(&Prop::Le(prod, x()), true);
    assert_eq!(d.interval(1), iv(1, 15));
}

#[test]
fn difference_and_sum_shapes_without_zone_variables() {
    // x − y ≤ −3 (x, y không ở zone): hi(x) ≤ hi(y) − 3 = 12 ; lo(y) ≥ lo(x) + 3 = 3
    let mut e = env2(false);
    e.assume(&Prop::Le(Term::sub(x(), y()), k(-3)), true);
    assert_eq!(e.interval(1), iv(0, 12));
    assert_eq!(e.interval(2), iv(3, 15));
    // y − x ≤ 2 (đảo vai)
    let mut e = env2(false);
    e.assume(&Prop::Le(Term::sub(y(), x()), k(-4)), true);
    assert_eq!(e.interval(2), iv(0, 11));
    assert_eq!(e.interval(1), iv(4, 15));
    // x + y ≤ 5 ⇒ x ≤ 5, y ≤ 5 (khoảng không âm)
    let mut e = env2(false);
    e.assume(&Prop::Le(Term::add(x(), y()), k(5)), true);
    assert_eq!(e.interval(1), iv(0, 5));
    assert_eq!(e.interval(2), iv(0, 5));
    let mut e = env2(false);
    e.assume(&Prop::Le(k(2), y()), true);
    e.assume(&Prop::Le(Term::add(x(), y()), k(5)), true);
    assert_eq!(e.interval(1), iv(0, 3));
    // x + y ≥ 20 ⇒ x ≥ 5 (vì y ≤ 15), y ≥ 5
    let mut e = env2(false);
    e.assume(&Prop::Le(k(20), Term::add(x(), y())), true);
    assert_eq!(e.interval(1), iv(5, 15));
    assert_eq!(e.interval(2), iv(5, 15));
    // hệ số 2: bỏ qua thay vì làm tròn sai
    let mut e = env2(false);
    e.assume(&Prop::Le(Term::add(x(), x()), k(9)), true);
    assert_eq!(e.interval(1), iv(0, 15)); // hệ số ≠ ±1 chưa được dùng (sound, không chính xác)
}

#[test]
fn assign_outside_the_type_range_is_unreachable() {
    let mut e = env2(true);
    e.assign(1, &k(300));
    assert!(e.is_dead());
    let mut e = env2(true);
    e.assign(1, &k(-1));
    assert!(e.is_dead());
    let mut e = env2(true);
    e.assign_interval(1, iv(20, 30));
    assert!(e.is_dead());
    let mut e = env2(true);
    e.assign_interval(1, iv(3, 30));
    assert_eq!(e.interval(1), iv(3, 15));
    let mut e = env2(true);
    e.assign(1, &k(7));
    assert_eq!(e.interval(1), iv(7, 7));
    // phép gán không biểu diễn được bằng zone: vẫn đúng khoảng
    e.assign(2, &Term::mul(x(), k(2)));
    assert_eq!(e.interval(2), iv(14, 14));
    // i4 có dấu: gán ngoài [-8,7]
    let mut e = Env::new(&[1]);
    e.declare(1, i4());
    e.assign(1, &k(8));
    assert!(e.is_dead());
}

#[test]
fn join_widen_and_order_with_dead_states_and_atoms() {
    let mut live = env2(true);
    live.assume(&Prop::Lt(x(), k(5)), true);
    live.assume(&Prop::Atom(3), true);
    let mut dead = env2(true);
    dead.assume(&Prop::False, true);
    assert!(dead.is_dead());
    // dead là đáy
    assert!(dead.leq(&live));
    assert!(dead.leq(&dead));
    assert!(!live.leq(&dead));
    assert_eq!(dead.join(&live).interval(1), live.interval(1));
    assert_eq!(live.join(&dead).interval(1), live.interval(1));
    assert_eq!(dead.join(&live).atom(3), Some(true));
    assert_eq!(live.join(&dead).atom(3), Some(true));
    let th = vec![I256::from_i128(0), I256::from_i128(15)];
    assert_eq!(dead.widen(&live, &th).interval(1), live.interval(1));
    assert_eq!(live.widen(&dead, &th).interval(1), live.interval(1));
    assert_eq!(live.widen(&dead, &th).atom(3), Some(true));
    // join/widen chỉ giữ nguyên tử CHUNG (cùng giá trị)
    let mut a = env2(true);
    a.assume(&Prop::Atom(1), true);
    a.assume(&Prop::Atom(2), true);
    a.assume(&Prop::Atom(3), false);
    let mut b = env2(true);
    b.assume(&Prop::Atom(1), true);
    b.assume(&Prop::Atom(2), false);
    let j = a.join(&b);
    assert_eq!(j.atom(1), Some(true));
    assert_eq!(j.atom(2), None);
    assert_eq!(j.atom(3), None);
    let w = a.widen(&b, &th);
    assert_eq!(w.atom(1), Some(true));
    assert_eq!(w.atom(2), None);
    assert_eq!(w.atom(3), None);
    // thứ tự: cần ĐỒNG THỜI khoảng, zone và nguyên tử
    let mut strong = env2(true);
    strong.assume(&Prop::Lt(x(), k(5)), true);
    strong.assume(&Prop::Lt(x(), y()), true);
    strong.assume(&Prop::Atom(9), true);
    let mut weak_iv = env2(true);
    weak_iv.assume(&Prop::Lt(x(), y()), true);
    weak_iv.assume(&Prop::Atom(9), true);
    let mut weak_zone = env2(true);
    weak_zone.assume(&Prop::Lt(x(), k(5)), true);
    weak_zone.assume(&Prop::Atom(9), true);
    let mut weak_atom = env2(true);
    weak_atom.assume(&Prop::Lt(x(), k(5)), true);
    weak_atom.assume(&Prop::Lt(x(), y()), true);
    assert!(strong.leq(&weak_iv) && strong.leq(&weak_zone) && strong.leq(&weak_atom));
    assert!(!weak_iv.leq(&strong), "thiếu cận khoảng");
    assert!(!weak_zone.leq(&strong), "thiếu quan hệ zone");
    assert!(!weak_atom.leq(&strong), "thiếu nguyên tử");
    // join giữ thông tin chung của hai nhánh
    let mut p = env2(true);
    p.assume(&Prop::Le(x(), k(3)), true);
    let mut q = env2(true);
    q.assume(&Prop::Le(k(10), x()), true);
    assert_eq!(p.join(&q).interval(1), iv(0, 15));
    let mut r = env2(true);
    r.assume(&Prop::Le(x(), k(7)), true);
    assert_eq!(p.join(&r).interval(1), iv(0, 7));
}

#[test]
fn eval_prop_is_three_valued_and_exact_on_boxes() {
    let mut e = env2(true);
    e.assume(&Prop::Le(k(3), x()), true);
    e.assume(&Prop::Le(x(), k(5)), true);
    assert_eq!(e.eval_prop(&Prop::True), Tri::Yes);
    assert_eq!(e.eval_prop(&Prop::False), Tri::No);
    assert_eq!(e.eval_prop(&Prop::Le(x(), k(5))), Tri::Yes);
    assert_eq!(e.eval_prop(&Prop::Lt(x(), k(3))), Tri::No);
    assert_eq!(e.eval_prop(&Prop::Lt(x(), k(5))), Tri::Maybe);
    assert_eq!(e.eval_prop(&Prop::Lt(x(), k(6))), Tri::Yes);
    assert_eq!(e.eval_prop(&Prop::Le(k(6), x())), Tri::No);
    // Or: một nhánh Yes ⇒ Yes ; mọi nhánh No ⇒ No ; còn lại Maybe
    assert_eq!(
        e.eval_prop(&Prop::Or(vec![Prop::False, Prop::Le(x(), k(5))])),
        Tri::Yes
    );
    assert_eq!(
        e.eval_prop(&Prop::Or(vec![Prop::False, Prop::Lt(x(), k(3))])),
        Tri::No
    );
    assert_eq!(
        e.eval_prop(&Prop::Or(vec![Prop::Lt(x(), k(5)), Prop::Lt(x(), k(3))])),
        Tri::Maybe
    );
    // And
    assert_eq!(
        e.eval_prop(&Prop::And(vec![Prop::True, Prop::Le(x(), k(5))])),
        Tri::Yes
    );
    assert_eq!(
        e.eval_prop(&Prop::And(vec![Prop::True, Prop::Lt(x(), k(3))])),
        Tri::No
    );
    assert_eq!(
        e.eval_prop(&Prop::And(vec![Prop::Le(x(), k(5)), Prop::Lt(x(), k(5))])),
        Tri::Maybe
    );
    // nguyên tử
    assert_eq!(e.eval_prop(&Prop::Atom(1)), Tri::Maybe);
    e.assume(&Prop::Atom(1), true);
    assert_eq!(e.eval_prop(&Prop::Atom(1)), Tri::Yes);
    e.assume(&Prop::Atom(2), false);
    assert_eq!(e.eval_prop(&Prop::Atom(2)), Tri::No);
    assert_eq!(e.eval_prop(&Prop::Not(Box::new(Prop::Atom(2)))), Tri::Yes);
}
