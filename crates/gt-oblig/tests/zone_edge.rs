//! Các trường hợp biên của `Zone` mà test đơn vị trong `zone.rs` chưa chạm (phát hiện bằng đột biến, ADR-025 §8):
//! vùng RỖNG qua join/widen/leq/meet/drop_all, đường chéo sau `shift`, bất biến "chỉ siết, không nới" của đóng bao.
use gt_oblig::zone::Zone;

fn empty3() -> Zone {
    // dựng một vùng có ràng buộc thật rồi làm nó mâu thuẫn: ma trận còn "rác" cũ ⇒ phép toán không kiểm cờ sẽ lộ ra
    let mut z = Zone::new(3);
    z.add_le(0, 1, 4);
    z.add_le(1, 2, 5);
    z.add_le(1, 0, -10); // x1 − x0 ≤ −10 và x0 − x1 ≤ 4 ⇒ chu trình 4 − 10 < 0
    assert!(z.is_empty_set());
    z
}

fn sample() -> Zone {
    let mut z = Zone::new(3);
    z.add_le(0, 1, -1);
    z.add_le(2, 1, 3);
    z
}

#[test]
fn empty_zone_reports_minimum_bounds_and_ignores_updates() {
    let mut e = empty3();
    assert_eq!(e.get(0, 1), Some(i128::MIN));
    assert_eq!(e.get(2, 2), Some(i128::MIN));
    e.add_le(0, 2, 1);
    e.forget(1);
    e.shift(0, 3);
    e.assign_copy(0, 1, 0);
    e.drop_all();
    assert!(e.is_empty_set());
    assert_eq!(e.get(0, 1), Some(i128::MIN));
}

#[test]
fn join_with_an_empty_zone_is_the_other_zone() {
    let (e, s) = (empty3(), sample());
    assert_eq!(e.join(&s), s);
    assert_eq!(s.join(&e), s);
    let ee = e.join(&e);
    assert!(ee.is_empty_set());
}

#[test]
fn widen_with_an_empty_zone_is_the_other_zone() {
    let (e, s) = (empty3(), sample());
    assert_eq!(e.widen(&s), s);
    assert_eq!(s.widen(&e), s);
    assert!(e.widen(&e).is_empty_set());
}

#[test]
fn order_treats_the_empty_zone_as_bottom() {
    let (e, s) = (empty3(), sample());
    assert!(e.leq(&s));
    assert!(e.leq(&e));
    assert!(!s.leq(&e));
    assert!(s.leq(&s));
    // (None, Some) ⇒ không ⊑: ràng buộc có ở `o` nhưng `self` không có
    let free = Zone::new(3);
    assert!(!free.leq(&s));
    assert!(s.leq(&free));
}

#[test]
fn meet_with_an_empty_zone_empties_and_empty_stays_empty() {
    let (e, s) = (empty3(), sample());
    let mut a = s.clone();
    a.meet_with(&e);
    assert!(a.is_empty_set());
    let mut b = e.clone();
    b.meet_with(&s);
    assert!(b.is_empty_set());
    // meet hai vùng thường: siết cả hai
    let mut c = sample();
    let mut d = Zone::new(3);
    d.add_le(0, 2, 7);
    d.add_le(1, 0, 5);
    c.meet_with(&d);
    assert_eq!(c.get(0, 1), Some(-1));
    assert_eq!(c.get(0, 2), Some(7)); // không có đường chặt hơn: x1 − x2 chưa bị chặn
    assert_eq!(c.get(1, 0), Some(5));
}

#[test]
fn shift_leaves_the_diagonal_at_zero() {
    let mut z = Zone::new(3);
    z.add_le(0, 1, 2);
    z.shift(0, 5);
    assert_eq!(z.get(0, 0), Some(0));
    assert_eq!(z.get(0, 1), Some(7));
    assert_eq!(z.get(1, 0), None);
    let mut w = Zone::new(2);
    w.shift(1, -3);
    assert_eq!(w.get(1, 1), Some(0));
}

#[test]
fn forget_keeps_the_diagonal_and_clears_only_that_variable() {
    let mut z = sample();
    z.add_le(0, 2, 1);
    z.forget(1);
    assert_eq!(z.get(1, 1), Some(0));
    assert_eq!(z.get(0, 1), None);
    assert_eq!(z.get(1, 0), None);
    assert_eq!(z.get(2, 1), None);
    // ràng buộc không liên quan tới biến 1 còn nguyên
    assert_eq!(z.get(0, 2), Some(1));
}

#[test]
fn closure_only_tightens_never_loosens() {
    let mut z = Zone::new(3);
    z.add_le(0, 2, -5);
    // đường vòng cho x0 − x2 ≤ -3 (lỏng hơn -5) KHÔNG được thay -5
    z.add_le(0, 1, -1);
    z.add_le(1, 2, -2);
    assert_eq!(z.get(0, 2), Some(-5));
    // một ràng buộc mới chặt hơn thì thay
    z.add_le(1, 2, -9);
    assert_eq!(z.get(0, 2), Some(-10));
}

#[test]
fn widen_keeps_equal_bounds_and_drops_looser_ones() {
    let mut a = Zone::new(2);
    a.add_le(0, 1, -2);
    let mut same = Zone::new(2);
    same.add_le(0, 1, -2);
    assert_eq!(a.widen(&same).get(0, 1), Some(-2)); // b == a ⇒ giữ
    let mut looser = Zone::new(2);
    looser.add_le(0, 1, -1);
    assert_eq!(a.widen(&looser).get(0, 1), None); // b > a ⇒ bỏ
    let mut tighter = Zone::new(2);
    tighter.add_le(0, 1, -5);
    assert_eq!(a.widen(&tighter).get(0, 1), Some(-2)); // b < a ⇒ giữ a
}

#[test]
fn drop_all_leaves_only_the_diagonal() {
    let mut z = sample();
    z.drop_all();
    for i in 0..3 {
        for j in 0..3 {
            assert_eq!(
                z.get(i, j),
                if i == j { Some(0) } else { None },
                "({i},{j})"
            );
        }
    }
}

#[test]
fn self_difference_constraints() {
    let mut z = Zone::new(2);
    z.add_le(1, 1, 0); // x − x ≤ 0 luôn đúng
    assert!(!z.is_empty_set());
    z.add_le(1, 1, 7);
    assert!(!z.is_empty_set());
    z.add_le(0, 0, -1); // x − x ≤ −1 mâu thuẫn
    assert!(z.is_empty_set());
}
