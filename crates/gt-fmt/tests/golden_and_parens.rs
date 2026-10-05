//! Golden và ngoặc theo độ ưu tiên (AST không có Paren nên printer phải tự thêm). Viết TRƯỚC khi có gt-fmt (TDD).
mod common;
use common::*;

// ---------- golden ----------

#[test]
fn simple_function_and_contract_layout() {
    assert_eq!(
        fmt_ok("ƒ  add( a:i32 ,b:i32 )→i32!∅{a+b}"),
        "ƒ add(a: i32, b: i32) → i32 ! ∅ { a + b }\n"
    );
    assert_eq!(
        fmt_ok("ƒ add(a:i32,b:i32)→i32!∅⊳a>=0∧b>=0{a+b}"),
        "ƒ add(a: i32, b: i32) → i32 ! ∅\n  ⊳ a ≥ 0 ∧ b ≥ 0\n{ a + b }\n"
    );
    assert_eq!(
        fmt_ok("ƒ fact(n:u32)→u64!∅⊳n>=0↓n{n as u64}"),
        "ƒ fact(n: u32) → u64 ! ∅\n  ⊳ n ≥ 0\n  ↓ n\n{ n as u64 }\n"
    );
    assert_eq!(fmt_ok("ƒ f()!∅{}"), "ƒ f() ! ∅ { }\n");
    assert_eq!(
        fmt_ok("◈ ƒ f<T:A+B,'a>(μ x:&'a T)!{io,lock(L)}{}"),
        "◈ ƒ f<T: A + B, 'a>(μx: &'a T) ! {io, lock(L)} { }\n"
    );
}

#[test]
fn statements_loops_match_if() {
    assert_eq!(
        fmt_ok("ƒ sum(xs:&[i64])→i64!∅{μs≔0;⟳ x in xs{s←s+*x;}s}"),
        "ƒ sum(xs: &[i64]) → i64 ! ∅ {\n  μs ≔ 0;\n  ⟳ x in xs {\n    s ← s + *x;\n  }\n  s\n}\n"
    );
    assert_eq!(
        fmt_ok("ƒ f(o:Option<i32>)→i32!∅{⌥ o{Some(v)↦v,None↦0}}"),
        "ƒ f(o: Option<i32>) → i32 ! ∅ {\n  ⌥ o {\n    Some(v) ↦ v,\n    None ↦ 0,\n  }\n}\n"
    );
    assert_eq!(
        fmt_ok("ƒ f(a:i32,b:i32)→i32!∅{⎇ a>=b{a}⇝{b}}"),
        "ƒ f(a: i32, b: i32) → i32 ! ∅ { ⎇ a ≥ b { a } ⇝ { b } }\n"
    );
    assert_eq!(
        fmt_ok("ƒ f(n:u32)!∅{⟳ i<n ↓ n-i ⋈ i>=0 {⊠;}}"),
        "ƒ f(n: u32) ! ∅ {\n  ⟳ i < n ↓ n - i ⋈ i ≥ 0 {\n    ⊠;\n  }\n}\n"
    );
    assert_eq!(fmt_ok("ƒ f()!∅{⏎;}"), "ƒ f() ! ∅ {\n  ⏎;\n}\n");
    assert_eq!(
        fmt_ok("ƒ f()→i32!∅{⎇ c{1}⇝⎇ d{2}⇝{3}}"),
        "ƒ f() → i32 ! ∅ { ⎇ c { 1 } ⇝ ⎇ d { 2 } ⇝ { 3 } }\n"
    );
}

#[test]
fn items_layout() {
    let src = "▢ P{◈ x:i32,y:i32}⊔ Shape{Circle(i32),Pair(i32,i32),Empty,}⊏ Area{ƒ area(&self)→i32!∅;}⊐ Area for Sq{ƒ area(&self)→i32!∅{self.w}}";
    let expected = "\
▢ P {
  ◈ x: i32,
  y: i32,
}

⊔ Shape {
  Circle(i32),
  Pair(i32, i32),
  Empty,
}

⊏ Area {
  ƒ area(&self) → i32 ! ∅;
}

⊐ Area for Sq {
  ƒ area(&self) → i32 ! ∅ { self.w }
}
";
    assert_eq!(fmt_ok(src), expected);
    assert_eq!(
        fmt_ok(
            "▦ m{◈ ƒ a()!∅{}ƒ b()!∅{}}⇲ m::a;⇲ a::b::{x,y};≡ Id≔u32;□ L:u32≔100;■ G:&str≔\"hi\";"
        ),
        "▦ m {\n  ◈ ƒ a() ! ∅ { }\n\n  ƒ b() ! ∅ { }\n}\n\n⇲ m::a;\n\n⇲ a::b::{x, y};\n\n≡ Id ≔ u32;\n\n□ L: u32 ≔ 100;\n\n■ G: &str ≔ \"hi\";\n"
    );
    assert_eq!(fmt_ok("▢ E{}⊐<T> W<T>{}"), "▢ E { }\n\n⊐<T> W<T> { }\n");
}

#[test]
fn doc_comments_and_attributes_follow_the_item() {
    assert_eq!(
        fmt_ok("///   Cộng.\n///\n#allow(S101,\"lý do\")\n#noreturn\n◈ ƒ f()!∅{}"),
        "///   Cộng.\n///\n#allow(S101, \"lý do\")\n#noreturn\n◈ ƒ f() ! ∅ { }\n"
    );
    assert_eq!(
        fmt_ok("▢ P{/// hoành độ\nx:i32}"),
        "▢ P {\n  /// hoành độ\n  x: i32,\n}\n"
    );
}

#[test]
fn types_render_canonically() {
    let t = |ty: &str| -> String {
        let out = fmt_ok(&format!("ƒ f(p: {ty}) ! ∅ {{ }}"));
        out.strip_prefix("ƒ f(p: ")
            .and_then(|s| s.strip_suffix(") ! ∅ { }\n"))
            .expect("dạng")
            .to_string()
    };
    for ty in [
        "i32",
        "&i32",
        "&!i32",
        "&'a i32",
        "&!'a i32",
        "&[i32]",
        "[u8; 4]",
        "()",
        "(i32,)",
        "(i32, bool)",
        "!",
        "*□ i32",
        "*μ i32",
        "Option<Vec<u8>>",
        "ƒ(i32, bool) → u8 ! ∅",
        "ƒ() ! {io}",
        "a::b::C<T>",
        "Self",
    ] {
        assert_eq!(t(ty), ty, "{ty}");
    }
    assert_eq!(t("(i32)"), "i32");
    assert_eq!(t(r"*\const i32"), "*□ i32"); // alias -> canonical
    assert_eq!(t(r"*\mut i32"), "*μ i32");
}

// ---------- ngoặc theo độ ưu tiên ----------

#[test]
fn parentheses_are_added_and_removed_by_precedence() {
    // phải thêm/giữ ngoặc
    for same in [
        "¬(a ∧ b)",
        "(a + b) * c",
        "a - (b - c)",
        "a / (b * c)",
        "(a < b) == c",
        "a ∧ (b ∨ c)",
        "(a ⇒ b) ⇒ c",
        "-(x as u8)",
        "(a as u8).f",
        "(-2).abs()",
        "-(128i8)",
        "(*p).x",
        "(&x).y",
        "(a..b).f",
        "a..(b..c)",
        "(∀ i ∈ r : p) ∧ q",
        "(⎇ c { 1 } ⇝ { 2 }) + 1",
        "(a,)",
        "()",
        "(a, b)",
        "-2.abs()",
        "*p.q",
        "¬¬a",
        "&!x",
        "-x as u8",
        "x as u8 as u16",
        "a ∈ b",
        "f(g(x), h)(y)",
        "xs[1..]",
        "xs[..2]",
        "xs[..]",
        "a.0.1",
    ] {
        assert_eq!(fe(same), same, "{same}");
    }
    // ngoặc thừa bị bỏ (AST không có Paren)
    for (input, expected) in [
        ("(((a)))", "a"),
        ("a + (b * c)", "a + b * c"),
        ("(a - b) - c", "a - b - c"),
        ("(a ∧ b) ∨ c", "a ∧ b ∨ c"),
        ("a ⇒ (b ⇒ c)", "a ⇒ b ⇒ c"),
        ("((a, b))", "(a, b)"),
        ("(a)", "a"),
        ("(-x) as u8", "-x as u8"),
        ("-(-5)", "-(-5)"),
        ("--5", "-(-5)"),
        ("(a + b) as u8", "(a + b) as u8"),
        ("a < b ∧ (c < d)", "a < b ∧ c < d"),
        ("(a.b).c", "a.b.c"),
        ("(f(x))(y)", "f(x)(y)"),
        ("¬(a)", "¬a"),
        ("(a) + (b)", "a + b"),
        ("(a ∈ b) ∧ c", "a ∈ b ∧ c"),
        ("(a as u8) < b", "(a as u8) < b"),
        ("(a + b as u8) < c", "(a + b as u8) < c"),
        ("(a as u8) <= b", "a as u8 ≤ b"),
        ("(a as u8) == b", "a as u8 == b"),
        ("(-(a as u8)) < b", "(-(a as u8)) < b"),
    ] {
        assert_eq!(fe(input), expected, "{input}");
    }
    // ASCII alias -> canonical, cùng ngoặc
    assert_eq!(fe(r"\neg (a \land b)"), "¬(a ∧ b)");
    assert_eq!(fe("a <= b"), "a ≤ b");
    assert_eq!(fe("a != b"), "a ≠ b");
    assert_eq!(fe("a >= b"), "a ≥ b");
}

#[test]
fn negative_literals_survive_reparse() {
    // gấp literal âm: in lại phải cho đúng cùng AST (bất biến, không chỉ chuỗi)
    for e in [
        "-128i8",
        "-(128i8)",
        "--5",
        "-2.abs()",
        "(-2).abs()",
        "-1.5",
        "a - 5",
        "a -5",
        "-x",
        "-(x)",
        "- 5",
    ] {
        let src = format!("ƒ f() → i32 ! ∅ {{ {e} }}");
        let out = fmt_ok(&src);
        assert_eq!(ast_of(&out), ast_of(&src), "{e} => {out}");
    }
}

#[test]
fn struct_literals_in_conditions_and_contracts_are_parenthesized() {
    // luật expr_ns: struct literal trần ở ⎇ ⌥ ⟳ và ở contract/↓/⋈/⊢ phải bọc ngoặc
    assert_eq!(
        fmt_ok("ƒ f()!∅{⎇ (P{x:1}) {1}⇝{2}}"),
        "ƒ f() ! ∅ { ⎇ (P { x: 1 }) { 1 } ⇝ { 2 } }\n"
    );
    assert_eq!(
        fmt_ok("ƒ f()→i32!∅{⌥ (P{x:1}) {_↦1}}"),
        "ƒ f() → i32 ! ∅ {\n  ⌥ (P { x: 1 }) {\n    _ ↦ 1,\n  }\n}\n"
    );
    assert_eq!(
        fmt_ok("ƒ f()!∅{⟳ (P{x:1}) {}}"),
        "ƒ f() ! ∅ { ⟳ (P { x: 1 }) { } }\n"
    );
    assert_eq!(
        fmt_ok("ƒ f()!∅{⟳ x in (P{x:1}) {}}"),
        "ƒ f() ! ∅ { ⟳ x in (P { x: 1 }) { } }\n"
    );
    assert_eq!(
        fmt_ok("ƒ f()→bool!∅⊳(P{x:1}) {true}"),
        "ƒ f() → bool ! ∅\n  ⊳ (P { x: 1 })\n{ true }\n"
    );
    assert_eq!(
        fmt_ok("ƒ f(n:u32)!∅↓(P{x:1}).x {}"),
        "ƒ f(n: u32) ! ∅\n  ↓ (P { x: 1 }).x\n{ }\n"
    );
    assert_eq!(
        fmt_ok("ƒ f(c:bool)!∅{⟳ c ↓ (P{x:1}).x ⋈ (P{x:2}).x {}}"),
        "ƒ f(c: bool) ! ∅ { ⟳ c ↓ (P { x: 1 }).x ⋈ (P { x: 2 }).x { } }\n"
    );
    assert_eq!(
        fmt_ok("ƒ f()!∅{⚠ ⊢ (P{x:1}) {x}}"),
        "ƒ f() ! ∅ { ⚠ ⊢ (P { x: 1 }) { x } }\n"
    );
    // sâu hơn trong biểu thức của điều kiện
    assert_eq!(
        fe("⎇ (P{x:1}).x == 1 {1}⇝{2}"),
        "⎇ (P { x: 1 }).x == 1 { 1 } ⇝ { 2 }"
    );
    // KHÔNG thêm ngoặc thừa khi ngữ cảnh đã đặt lại luật (đối số, ngoặc vuông, ngoặc)
    assert_eq!(fe("⎇ f(P{x:1}) {1}⇝{2}"), "⎇ f(P { x: 1 }) { 1 } ⇝ { 2 }");
    assert_eq!(fe("⎇ a[P{x:1}] {1}⇝{2}"), "⎇ a[P { x: 1 }] { 1 } ⇝ { 2 }");
    assert_eq!(
        fe("⎇ ((P{x:1}), 2).1 == 2 {1}⇝{2}"),
        "⎇ (P { x: 1 }, 2).1 == 2 { 1 } ⇝ { 2 }"
    );
    // ngoài điều kiện: ngoặc thừa bị bỏ
    assert_eq!(fe("(P{x:1})"), "P { x: 1 }");
    assert_eq!(
        fmt_ok("ƒ f()!∅{y≔(P{x:1});}"),
        "ƒ f() ! ∅ {\n  y ≔ P { x: 1 };\n}\n"
    );
}

#[test]
fn assignment_places_and_one_element_tuple() {
    assert_eq!(
        fmt_ok("ƒ f()!∅{(*p).x←1;}"),
        "ƒ f() ! ∅ {\n  (*p).x ← 1;\n}\n"
    );
    assert_eq!(fmt_ok("ƒ f()!∅{**pp←2;}"), "ƒ f() ! ∅ {\n  **pp ← 2;\n}\n");
    assert_eq!(fmt_ok("ƒ f()!∅{*p.q←1;}"), "ƒ f() ! ∅ {\n  *p.q ← 1;\n}\n");
    assert_eq!(
        fmt_ok("ƒ f()!∅{a.b[0].c←1;}"),
        "ƒ f() ! ∅ {\n  a.b[0].c ← 1;\n}\n"
    );
    assert_eq!(
        fmt_ok("ƒ f()!∅{self.x←1;}"),
        "ƒ f() ! ∅ {\n  self.x ← 1;\n}\n"
    );
    assert_eq!(
        fmt_ok("ƒ f()→(i32,)!∅{(1,)}"),
        "ƒ f() → (i32,) ! ∅ { (1,) }\n"
    );
    assert_eq!(fmt_ok("ƒ f()!∅{x≔(a,);}"), "ƒ f() ! ∅ {\n  x ≔ (a,);\n}\n");
}

#[test]
fn block_like_statements_keep_their_semicolon_status() {
    assert_eq!(
        fmt_ok("ƒ f()→i32!∅{⎇ c{1}⇝{2} 3}"),
        "ƒ f() → i32 ! ∅ {\n  ⎇ c { 1 } ⇝ { 2 }\n  3\n}\n"
    );
    assert_eq!(
        fmt_ok("ƒ f()→i32!∅{⎇ c{1}⇝{2}; 3}"),
        "ƒ f() → i32 ! ∅ {\n  ⎇ c { 1 } ⇝ { 2 };\n  3\n}\n"
    );
}

#[test]
fn patterns_print_and_reparse_to_the_same_ast() {
    // `V()` (biến thể một đoạn không đối số) KHÔNG được in thành `V` (sẽ đọc lại là binding)
    let out = fmt_ok("ƒ f(x: i32) → i32 ! ∅ { ⌥ x { V() ↦ 1, Shape::E ↦ 2, _ ↦ 3 } }");
    assert!(out.contains("V() ↦ 1"), "{out}");
    assert!(out.contains("Shape::E ↦ 2"), "{out}");

    for p in [
        "V()",
        "V",
        "Shape::E",
        "Shape::E()",
        "S(a, _)",
        "S(.., z)",
        "P { x, y: _, .. }",
        "P { }",
        "P { .. }",
        "(a,)",
        "(..)",
        "(a, ..)",
        "()",
        "-1",
        "'c'",
        "\"s\"",
        "true",
        "0..=9",
        "10..",
        "..0",
        "-5..-1",
        "_",
    ] {
        let src = format!("ƒ f(x: i32) → i32 ! ∅ {{ ⌥ x {{ {p} ↦ 1 }} }}");
        let out = fmt_ok(&src);
        assert_eq!(ast_of(&out), ast_of(&src), "{p} => {out}");
        assert_eq!(fmt_ok(&out), out, "{p}");
    }
}
