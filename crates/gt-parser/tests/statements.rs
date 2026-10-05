//! Câu lệnh, vòng lặp, match/pattern, unsafe. Viết TRƯỚC khi có parser (TDD).
mod common;
use common::*;

#[test]
fn simple_statements() {
    assert_eq!(block_of("x ≔ 1; x"), "{(let x 1) tail:x}");
    assert_eq!(block_of("μx ≔ 1; x"), "{(let! x 1) tail:x}");
    assert_eq!(block_of("x: i32 ≔ 1; x"), "{(let x:T 1) tail:x}");
    assert_eq!(block_of("x := 1; x"), block_of("x ≔ 1; x")); // alias ASCII
    assert_eq!(
        block_of("*p ← *p + 1; 0"),
        "{(set (deref p) (+ (deref p) 1)) tail:0}"
    );
    assert_eq!(
        block_of("a.b[0] ← 1; 0"),
        "{(set (idx (. a b) 0) 1) tail:0}"
    );
    assert_eq!(block_of("x <- 1; 0"), block_of("x ← 1; 0"));
    assert_eq!(block_of("⏎ 1;"), "{(ret 1)}");
    assert_eq!(block_of("⏎;"), "{(ret)}");
    assert_eq!(block_of("⊠; ⊡; 0"), "{(break) (continue) tail:0}");
    assert_eq!(block_of("f(1); 2"), "{(expr; (call f 1)) tail:2}");
    assert_eq!(block_of("x;"), "{(expr; x)}"); // `;` => câu lệnh, không có biểu thức cuối (Q8)
    assert_eq!(block_of(""), "{}");
}

#[test]
fn assignment_target_must_be_a_place() {
    for bad in ["x + y ← 1;", "f(x) ← 1;", "1 ← 2;", "(a, b) ← c;"] {
        assert_eq!(expr_err(bad)[0], "E0101", "{bad}");
    }
}

#[test]
fn block_like_statements_and_tail_rule() {
    // đứng ngay trước `}` => biểu thức cuối (giá trị khối)
    assert_eq!(
        block_of("⎇ c { 1 } ⇝ { 2 }"),
        "{tail:(if c {tail:1} (block {tail:2}))}"
    );
    // đứng trước thứ khác => câu lệnh, không cần `;` (semi = false)
    assert_eq!(
        block_of("⎇ c { 1 } ⇝ { 2 } 3"),
        "{(expr (if c {tail:1} (block {tail:2}))) tail:3}"
    );
    // `;` tuỳ chọn sau khối-biểu-thức (semi = true)
    assert_eq!(
        block_of("⎇ c { 1 } ⇝ { 2 }; 3"),
        "{(expr; (if c {tail:1} (block {tail:2}))) tail:3}"
    );
    assert_eq!(
        block_of("⎇ a { 1 } ⇝ ⎇ b { 2 } ⇝ { 3 }"),
        "{tail:(if a {tail:1} (if b {tail:2} (block {tail:3})))}"
    );
    // khối-biểu-thức ở đầu câu lệnh kết thúc tại `}` của nó: `+ 1` phía sau là lỗi
    assert_eq!(expr_err("⎇ c { 1 } ⇝ { 2 } + 1")[0], "E0101");
    // khối-biểu-thức dùng làm giá trị ở vị trí biểu thức
    assert_eq!(
        block_of("x ≔ ⎇ c { 1 } ⇝ { 2 }; x"),
        "{(let x (if c {tail:1} (block {tail:2}))) tail:x}"
    );
    assert_eq!(
        block_of("x ≔ { 1 }; x"),
        "{(let x (block {tail:1})) tail:x}"
    );
}

#[test]
fn loops() {
    assert_eq!(block_of("⟳ { } 0"), "{(expr (loop inf {})) tail:0}");
    assert_eq!(
        block_of("⟳ i < n { } 0"),
        "{(expr (loop while (< i n) {})) tail:0}"
    );
    assert_eq!(
        block_of("⟳ i < n ↓ n - i ⋈ i >= 0 ⋈ n >= 0 { } 0"),
        "{(expr (loop while (< i n) dec (- n i) inv (>= i 0) inv (>= n 0) {})) tail:0}"
    );
    assert_eq!(
        block_of("⟳ x in xs { } 0"),
        "{(expr (loop for x in xs {})) tail:0}"
    );
    assert_eq!(
        block_of("⟳ x in 0..n ↓ n { } 0"),
        "{(expr (loop for x in (.. 0 n) dec n {})) tail:0}"
    );
    assert_eq!(
        block_of("⟳ ↓ n { } 0"),
        "{(expr (loop inf dec n {})) tail:0}"
    );
    assert_eq!(
        block_of("\\loop x in xs { } 0"),
        block_of("⟳ x in xs { } 0")
    );
    // `x` không theo sau bởi `in` => là điều kiện
    assert_eq!(block_of("⟳ x { } 0"), "{(expr (loop while x {})) tail:0}");
    // thân có break/continue
    assert_eq!(
        block_of("⟳ c { ⊠; } 0"),
        "{(expr (loop while c {(break)})) tail:0}"
    );
    // for-in cần tên biến hợp lệ
    assert_eq!(expr_err("⟳ 1 in xs { }")[0], "E0101");
    assert_eq!(expr_err("⟳ in in xs { }")[0], "E0101");
}

#[test]
fn match_and_patterns() {
    assert_eq!(
        expr("⌥ o { Some(v) ↦ v, None ↦ 0, }"),
        "(match o [Some(v) => v] [None => 0])"
    );
    assert_eq!(
        expr("⌥ o { Some(v) ↦ v, None ↦ 0 }"),
        expr("⌥ o { Some(v) ↦ v, None ↦ 0, }")
    );
    assert_eq!(
        expr("⌥ n { ..0 ↦ 0, 0..=9 ↦ 1, 10.. ↦ 2, -1 ↦ 3, _ ↦ 4 }"),
        "(match n [..0 => 0] [0..=9 => 1] [10.. => 2] [-1 => 3] [_ => 4])"
    );
    assert_eq!(expr("⌥ t { (a, ..) ↦ a }"), "(match t [(a,..) => a])");
    assert_eq!(expr("⌥ t { (..) ↦ 0 }"), "(match t [(..) => 0])");
    assert_eq!(
        expr("⌥ s { Shape::Dot(Point { x, y: _ }) ↦ x, Shape::Pair(a, _) ↦ a, Shape::Empty ↦ 0 }"),
        "(match s [Shape::Dot(Point{x,y:_}) => x] [Shape::Pair(a,_) => a] [Shape::Empty => 0])"
    );
    assert_eq!(
        expr("⌥ p { Point { x, .. } ↦ x }"),
        "(match p [Point{x,..} => x])"
    );
    assert_eq!(
        expr("⌥ c { 'a' ↦ 1, \"s\" ↦ 2, true ↦ 3 }"),
        "(match c ['a' => 1] [\"s\" => 2] [true => 3])"
    );
    assert_eq!(
        expr("⌥ x { A ↦ { 1 }, B ↦ 2 }"),
        "(match x [A => (block {tail:1})] [B => 2])"
    );
    assert_eq!(expr("⌥ x { (a) ↦ a }"), "(match x [a => a])"); // ngoặc nhóm trong pattern
    assert_eq!(expr("\\match x { _ => 1 }"), expr("⌥ x { _ ↦ 1 }"));
    // lỗi: `..` đơn lẻ ở mức cao nhất, thiếu mũi tên, hai rest
    for bad in ["⌥ x { .. ↦ 1 }", "⌥ x { a 1 }", "⌥ x { (.., ..) ↦ 1 }"] {
        assert_eq!(expr_err(bad)[0], "E0101", "{bad}");
    }
}

#[test]
fn unsafe_blocks() {
    assert_eq!(expr("⚠ ⊢ p ⊢ q { x }"), "(unsafe proof p proof q {tail:x})");
    assert_eq!(expr("⚠ { x }"), "(unsafe {tail:x})");
    assert_eq!(block_of("⚠ { x } 1"), "{(expr (unsafe {tail:x})) tail:1}");
}

#[test]
fn reserved_and_hard_keywords_are_rejected() {
    for bad in [
        "self ≔ 1;",
        "Self ≔ 1;",
        "in ≔ 1;",
        "μ ≔ 1;",
        "\\extern",
        "\\where",
        "as ≔ 1;",
        "for ≔ 1;",
        "true ≔ 1;",
    ] {
        assert!(!expr_err(&format!("{bad} 0"))[0].is_empty(), "{bad}");
        assert_eq!(expr_err(&format!("{bad} 0"))[0], "E0101", "{bad}");
    }
    assert_eq!(expr_err("aμb")[0], "E0101"); // μ giữa chữ (Q5)
    // `=` đứng riêng, và toán tử gán kết hợp
    assert_eq!(expr_err("x = 1;")[0], "E0101");
    assert_eq!(expr_err("x += 1;")[0], "E0101");
    // comment khối
    assert_eq!(expr_err("/* c */ 1")[0], "E0101");
}

#[test]
fn doc_comment_in_wrong_place_is_an_error() {
    let out = parse_src("ƒ f() → i32 ! ∅ {\n  1\n  /// không gắn vào đâu\n}\n");
    assert_eq!(codes(&out), ["E0101"]);
}

#[test]
fn assignment_places_accept_deref_chains_and_self() {
    // Q15i: (*p).x, **pp, self.x và các dạng tương tự (ngoặc nhóm không vào AST)
    assert_eq!(
        block_of("(*p).x ← 1; 0"),
        "{(set (. (deref p) x) 1) tail:0}"
    );
    assert_eq!(
        block_of("**pp ← 2; 0"),
        "{(set (deref (deref pp)) 2) tail:0}"
    );
    assert_eq!(block_of("*p.q ← 1; 0"), "{(set (deref (. p q)) 1) tail:0}");
    assert_eq!(block_of("self.x ← 1; 0"), "{(set (. self x) 1) tail:0}");
    assert_eq!(
        block_of("a[0].b[1] ← 1; 0"),
        "{(set (idx (. (idx a 0) b) 1) 1) tail:0}"
    );
    assert_eq!(block_of("(a) ← 1; 0"), "{(set a 1) tail:0}");
    // không phải vị trí → E0101
    for bad in [
        "g().x ← 1;",
        "1.x ← 2;",
        "(a + b).x ← 1;",
        "&x ← 1;",
        "-x ← 1;",
        "a::b ← 1;",
        "[1][0] ← 1;",
        "(a, b).0 ← 1;",
    ] {
        assert_eq!(expr_err(bad)[0], "E0101", "{bad}");
    }
}

#[test]
fn expr_ns_applies_before_every_block() {
    // ↓ ⋈ ⊢ : tên rồi `{` là khối, không phải struct literal (Q15a)
    assert_eq!(
        block_of("⟳ c ↓ n { } 0"),
        "{(expr (loop while c dec n {})) tail:0}"
    );
    assert_eq!(
        block_of("⟳ c ⋈ n { } 0"),
        "{(expr (loop while c inv n {})) tail:0}"
    );
    assert_eq!(expr("⚠ ⊢ p { x }"), "(unsafe proof p {tail:x})");
    // struct literal trần bị từ chối ở từng vị trí; bọc ngoặc thì được
    for bad in [
        "⟳ c ↓ P { x: 1 } { }",
        "⟳ c ⋈ P { x: 1 } { }",
        "⚠ ⊢ P { x: 1 } { x }",
        "⟳ x in P { x: 1 } { }",
    ] {
        assert_eq!(expr_err(bad)[0], "E0101", "{bad}");
    }
    assert_eq!(
        block_of("⟳ c ↓ (P { x: 1 }).x { } 0"),
        "{(expr (loop while c dec (. (struct P (x 1)) x) {})) tail:0}"
    );
    assert_eq!(
        block_of("⟳ c ⋈ f(P { x: 1 }) { } 0"),
        "{(expr (loop while c inv (call f (struct P (x 1))) {})) tail:0}"
    );
}
