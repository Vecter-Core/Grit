//! Độ ưu tiên, tính kết hợp, literal âm, struct literal trong điều kiện. Viết TRƯỚC khi có parser (TDD).
mod common;
use common::*;

#[test]
fn arithmetic_precedence_and_associativity() {
    assert_eq!(expr("a + b * c"), "(+ a (* b c))");
    assert_eq!(expr("a * b + c"), "(+ (* a b) c)");
    assert_eq!(expr("a - b - c"), "(- (- a b) c)");
    assert_eq!(expr("a / b / c"), "(/ (/ a b) c)");
    assert_eq!(expr("a % b * c"), "(* (% a b) c)");
    assert_eq!(expr("a +% b *^ c"), "(+% a (*^ b c))");
    assert_eq!(expr("a +? b -^ c"), "(-^ (+? a b) c)");
    assert_eq!(expr("(a + b) * c"), "(* (+ a b) c)"); // ngoặc nhóm không vào AST
    assert_eq!(expr("((a))"), "a");
}

#[test]
fn logic_and_comparison() {
    assert_eq!(expr("a < b ∧ c < d ∨ e"), "(or (and (< a b) (< c d)) e)");
    assert_eq!(expr("a ∨ b ∧ c"), "(or a (and b c))");
    assert_eq!(expr("¬a ∧ b"), "(and (not a) b)");
    assert_eq!(expr("¬(a ∧ b)"), "(not (and a b))");
    assert_eq!(expr("a ⇒ b ⇒ c"), "(=> a (=> b c))"); // kết hợp phải (Q15c)
    assert_eq!(expr("a ∨ b ⇒ c"), "(=> (or a b) c)");
    assert_eq!(expr("a + 1 <= b * 2"), "(<= (+ a 1) (* b 2))");
    assert_eq!(expr("a ∈ b"), "(in a b)"); // cùng tầng so sánh (Q15c)
    // ASCII alias cho cùng kết quả
    assert_eq!(expr("a \\land b \\lor c"), expr("a ∧ b ∨ c"));
    assert_eq!(expr("\\neg a"), expr("¬a"));
    assert_eq!(expr("a != b"), expr("a ≠ b"));
}

#[test]
fn comparison_is_non_associative() {
    for bad in [
        "a < b < c",
        "a == b == c",
        "a < b == c",
        "a ∈ b ∈ c",
        "a <= b >= c",
        "a != b != c",
    ] {
        assert_eq!(expr_err(bad)[0], "E0101", "{bad}");
    }
    // nhưng nối bằng ∧ thì được, và ngoặc cũng được
    assert_eq!(expr("a < b ∧ b < c"), "(and (< a b) (< b c))");
    assert_eq!(expr("(a < b) == c"), "(== (< a b) c)");
}

#[test]
fn unary_cast_and_deref_vs_mul() {
    assert_eq!(expr("-x as u8"), "(as (neg x) U8)"); // unary chặt hơn `as`
    assert_eq!(expr("x as u8 + 1"), "(+ (as x U8) 1)");
    assert_eq!(expr("x as u8 as u16"), "(as (as x U8) U16)");
    assert_eq!(expr("*p * *q"), "(* (deref p) (deref q))");
    assert_eq!(expr("a ** b"), "(* a (deref b))");
    assert_eq!(expr("&!x"), "(&! x)");
    assert_eq!(expr("&x.y"), "(& (. x y))"); // postfix chặt hơn unary
    assert_eq!(expr("*p.q"), "(deref (. p q))");
    assert_eq!(expr("¬¬a"), "(not (not a))");
}

#[test]
fn negative_literals_fold_only_directly() {
    assert_eq!(expr("-128i8"), "-128i8"); // gấp vào literal
    assert_eq!(expr("- 5"), "-5");
    assert_eq!(expr("-1.5"), "-1.5");
    assert_eq!(expr("-0xFFu8"), "-0xFFu8");
    assert_eq!(expr("-(128i8)"), "(neg 128i8)"); // ngoặc: KHÔNG gấp (để M7 báo tràn)
    assert_eq!(expr("-x"), "(neg x)");
    assert_eq!(expr("--5"), "(neg -5)");
    assert_eq!(expr("-2.abs()"), "(neg (call (. 2 abs)))"); // postfix chặt hơn: không gấp
    assert_eq!(expr("a - 5"), "(- a 5)"); // trừ nhị phân không bị gấp
    assert_eq!(expr("a -5"), "(- a 5)");
}

#[test]
fn ranges() {
    assert_eq!(expr("a..b"), "(.. a b)");
    assert_eq!(expr("a..=b"), "(..= a b)");
    assert_eq!(expr("a.."), "(.. a _)");
    assert_eq!(expr("..b"), "(.. _ b)");
    assert_eq!(expr(".."), "(.. _ _)");
    assert_eq!(expr("a + 1..b * 2"), "(.. (+ a 1) (* b 2))"); // range thấp nhất
    assert_eq!(expr("xs[1..]"), "(idx xs (.. 1 _))");
    assert_eq!(expr("xs[..2]"), "(idx xs (.. _ 2))");
    assert_eq!(expr("xs[..]"), "(idx xs (.. _ _))");
    assert_eq!(expr("&xs[0..=1]"), "(& (idx xs (..= 0 1)))");
    assert_eq!(expr("1..2"), "(.. 1 2)");
    assert_eq!(expr_err("a..=")[0], "E0101"); // ..= bắt buộc có cận cuối
}

#[test]
fn postfix_tuples_arrays() {
    assert_eq!(expr("f(x)(y)"), "(call (call f x) y)");
    assert_eq!(expr("f()"), "(call f)");
    assert_eq!(expr("f(a, b,)"), "(call f a b)"); // dấu phẩy cuối được phép
    assert_eq!(expr("t.0.1"), "(. (. t 0) 1)");
    assert_eq!(expr("a.b(c).d"), "(. (call (. a b) c) d)");
    assert_eq!(expr("a[0][1]"), "(idx (idx a 0) 1)");
    assert_eq!(expr("()"), "(tuple)");
    assert_eq!(expr("(a,)"), "(tuple a)");
    assert_eq!(expr("(a, b)"), "(tuple a b)");
    assert_eq!(expr("[1, 2, 3]"), "[1 2 3]");
    assert_eq!(expr("[0; 8]"), "[0; 8]");
    assert_eq!(expr("[]"), "[]");
    assert_eq!(expr("a::b::c"), "a::b::c");
    assert_eq!(expr("self.w"), "(. self w)"); // Q15b
    assert_eq!(expr("Self::new()"), "(call Self::new)");
    assert_eq!(expr("true"), "true");
    assert_eq!(expr("\"hi\""), "\"hi\"");
    assert_eq!(expr("'x'"), "'x'");
}

#[test]
fn struct_literals_and_the_no_struct_rule() {
    assert_eq!(expr("Point { x: 1, y: 2 }"), "(struct Point (x 1) (y 2))");
    assert_eq!(expr("Self { x: 0 }"), "(struct Self (x 0))");
    assert_eq!(expr("Point { }"), "(struct Point )");
    // trong điều kiện: tên rồi `{` là khối, không phải struct literal
    assert_eq!(
        expr("⎇ p { 1 } ⇝ { 2 }"),
        "(if p {tail:1} (block {tail:2}))"
    );
    // struct literal trần trong điều kiện bị từ chối; bọc ngoặc thì được
    assert_eq!(expr_err("⎇ Point { x: 1 } { 1 } ⇝ { 2 }")[0], "E0101");
    assert_eq!(
        expr("⎇ (Point { x: 1 }).x == 1 { 1 } ⇝ { 2 }"),
        "(if (== (. (struct Point (x 1)) x) 1) {tail:1} (block {tail:2}))"
    );
    // đối tượng của ⌥ và điều kiện của ⟳ cũng vậy
    assert_eq!(expr("⌥ p { _ ↦ 1 }"), "(match p [_ => 1])");
    assert_eq!(expr_err("⌥ Point { x: 1 } { _ ↦ 1 }")[0], "E0101");
    assert_eq!(expr_err("⟳ Point { x: 1 } { }")[0], "E0101");
    // ngoặc/ngoặc vuông/lời gọi đặt lại luật
    assert_eq!(
        expr("⎇ f(Point { x: 1 }) { 1 } ⇝ { 2 }"),
        "(if (call f (struct Point (x 1))) {tail:1} (block {tail:2}))"
    );
}

#[test]
fn quantifier_and_chain_depth() {
    assert_eq!(
        expr("∀ i ∈ 0..n : a[i] >= 0"),
        "(forall i (.. 0 n) (>= (idx a i) 0))"
    );
    assert_eq!(
        expr("∃ i ∈ r : p(i) ∧ q"),
        "(exists i r (and (call p i) q))"
    );
    assert_eq!(expr("\\forall i \\in r : p"), expr("∀ i ∈ r : p"));
}
