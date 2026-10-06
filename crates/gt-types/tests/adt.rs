//! M4 bước (b): struct/enum/tuple/mảng; kiểm trường (E0304); khả kiến trường (E0203); generics của ADT.
mod common;
use common::*;

const PT: &str = "▢ P { x: i32, y: i32 } ";

#[test]
fn struct_literals_fields_and_types() {
    let out = ok(&format!(
        "{PT}ƒ f() → i32 ! ∅ {{ p ≔ P {{ x: 1, y: 2 }}; q ≔ p.x; p.y }}"
    ));
    assert_eq!(out.local_type("main::f", "p").as_deref(), Some("P"));
    assert_eq!(out.local_type("main::f", "q").as_deref(), Some("i32"));
    // thứ tự trường tuỳ ý; viết tắt không có (grammar không có shorthand ở literal)
    ok(&format!("{PT}ƒ f() ! ∅ {{ p ≔ P {{ y: 2, x: 1 }}; }}"));
}

#[test]
fn e0304_missing_extra_unknown_and_duplicate_fields_in_literals() {
    for bad in [
        "p ≔ P { x: 1 };",             // thiếu
        "p ≔ P { };",                  // thiếu cả hai
        "p ≔ P { x: 1, y: 2, z: 3 };", // thừa (không tồn tại)
        "p ≔ P { x: 1, y: 2, x: 3 };", // trùng
        "p ≔ P { x: 1, w: 2 };",       // sai tên (và thiếu y) — hai lỗi
    ] {
        let out = one(&format!("{PT}ƒ f() ! ∅ {{ {bad} }}"));
        assert!(
            !codes(&out).is_empty() && codes(&out).iter().all(|c| c == "E0304"),
            "{bad}: {:?}",
            show(&out)
        );
    }
    err(
        &format!("{PT}ƒ f() ! ∅ {{ p ≔ P {{ x: 1 }}; }}"),
        "E0304",
        1,
    );
    err(
        &format!("{PT}ƒ f() ! ∅ {{ p ≔ P {{ x: 1, y: 2, z: 3 }}; }}"),
        "E0304",
        1,
    );
    err(
        &format!("{PT}ƒ f() ! ∅ {{ p ≔ P {{ x: 1, y: 2, x: 3 }}; }}"),
        "E0304",
        1,
    );
}

#[test]
fn e0304_missing_field_has_a_machine_applicable_looking_suggestion() {
    let out = err(
        &format!("{PT}ƒ f() ! ∅ {{ p ≔ P {{ x: 1 }}; }}"),
        "E0304",
        1,
    );
    let d = &out.report.diagnostics[0];
    assert!(d.message.contains('y'));
    assert_eq!(d.suggestions.len(), 1);
    assert!(d.suggestions[0].edits[0].replacement.contains("y: …"));
}

#[test]
fn field_value_types_are_checked() {
    err(
        &format!("{PT}ƒ f() ! ∅ {{ p ≔ P {{ x: true, y: 2 }}; }}"),
        "E0301",
        1,
    );
    err(
        &format!("{PT}ƒ f() ! ∅ {{ p ≔ P {{ x: 1i64, y: 2 }}; }}"),
        "E0302",
        1,
    );
}

#[test]
fn field_access() {
    ok(&format!(
        "{PT}ƒ f(p: P, q: &P, r: &!P) → i32 ! ∅ {{ p.x + q.y + r.x }}"
    ));
    err(&format!("{PT}ƒ f(p: P) → i32 ! ∅ {{ p.z }}"), "E0304", 1);
    err("ƒ f(x: i32) → i32 ! ∅ { x.y }", "E0304", 1);
    // nested
    ok("▢ A { b: B } ▢ B { c: i32 } ƒ f(a: A) → i32 ! ∅ { a.b.c }");
    // enum không có trường
    err("⊔ E { A } ƒ f(e: E) ! ∅ { e.x; }", "E0304", 1);
}

#[test]
fn tuple_fields_and_indexing() {
    let out = ok("ƒ f(t: (i32, bool), n: ((i32, i32), i32)) ! ∅ { a ≔ t.0; b ≔ t.1; c ≔ n.0.1; }");
    assert_eq!(out.local_type("main::f", "a").as_deref(), Some("i32"));
    assert_eq!(out.local_type("main::f", "b").as_deref(), Some("bool"));
    assert_eq!(out.local_type("main::f", "c").as_deref(), Some("i32"));
    err("ƒ f(t: (i32, bool)) ! ∅ { t.2; }", "E0304", 1);
    err("ƒ f(t: (i32,)) ! ∅ { t.1; }", "E0304", 1);
}

#[test]
fn arrays_and_slices_index_with_usize() {
    let out =
        ok("ƒ f(a: [i32; 3], s: &[u8]) ! ∅ { x ≔ a[0]; y ≔ s[1]; z ≔ &s[0..2]; w ≔ &a[..]; }");
    assert_eq!(out.local_type("main::f", "x").as_deref(), Some("i32"));
    assert_eq!(out.local_type("main::f", "y").as_deref(), Some("u8"));
    assert_eq!(out.local_type("main::f", "z").as_deref(), Some("&[u8]"));
    assert_eq!(out.local_type("main::f", "w").as_deref(), Some("&[i32]"));
    err("ƒ f(a: [i32; 3]) ! ∅ { a[1i32]; }", "E0302", 1);
    err("ƒ f(a: [i32; 3]) ! ∅ { a[true]; }", "E0301", 1);
    err("ƒ f(x: i32) ! ∅ { x[0]; }", "E0301", 1);
}

#[test]
fn enum_variants_construct_and_have_the_enum_type() {
    let out = ok(
        "⊔ Shape { Dot, Circle(i32), Pair(i32, bool) } ƒ f() ! ∅ { a ≔ Shape::Dot; b ≔ Shape::Circle(1); c ≔ Shape::Pair(1, true); d ≔ Dot; }",
    );
    for n in ["a", "b", "c", "d"] {
        assert_eq!(
            out.local_type("main::f", n).as_deref(),
            Some("Shape"),
            "{n}"
        );
    }
    err("⊔ E { A(i32) } ƒ f() ! ∅ { x ≔ E::A(true); }", "E0301", 1);
    err("⊔ E { A(i32) } ƒ f() ! ∅ { x ≔ E::A(1, 2); }", "E0301", 1);
    err("⊔ E { A(i32) } ƒ f() ! ∅ { x ≔ E::A(); }", "E0301", 1);
}

#[test]
fn generic_adts_substitute_their_parameters() {
    let out = ok(
        "▢ W<T> { v: T } ⊔ O<T> { N, S(T) } ƒ f() ! ∅ { a ≔ W { v: 1u8 }; b ≔ a.v; c ≔ S(true); d ≔ N; e: O<i64> ≔ d; }",
    );
    assert_eq!(out.local_type("main::f", "a").as_deref(), Some("W<u8>"));
    assert_eq!(out.local_type("main::f", "b").as_deref(), Some("u8"));
    assert_eq!(out.local_type("main::f", "c").as_deref(), Some("O<bool>"));
    assert_eq!(out.local_type("main::f", "e").as_deref(), Some("O<i64>"));
    // `d ≔ N` không đủ thông tin nếu không có chỗ ràng buộc
    err("⊔ O<T> { N, S(T) } ƒ f() ! ∅ { d ≔ N; }", "E0301", 1);
    // sai số đối số kiểu
    err("▢ W<T> { v: T } ƒ f(a: W) ! ∅ { }", "E0301", 1);
    err("▢ W<T> { v: T } ƒ f(a: W<i32, i32>) ! ∅ { }", "E0301", 1);
    err("▢ P { x: i32 } ƒ f(a: P<i32>) ! ∅ { }", "E0301", 1);
}

#[test]
fn struct_patterns_check_fields() {
    ok(&format!(
        "{PT}ƒ f(p: P) → i32 ! ∅ {{ ⌥ p {{ P {{ x, y: _ }} ↦ x }} }}"
    ));
    ok(&format!(
        "{PT}ƒ f(p: P) → i32 ! ∅ {{ ⌥ p {{ P {{ x, .. }} ↦ x }} }}"
    ));
    err(
        &format!("{PT}ƒ f(p: P) → i32 ! ∅ {{ ⌥ p {{ P {{ x }} ↦ x }} }}"),
        "E0304",
        1,
    ); // thiếu y, không có ..
    err(
        &format!("{PT}ƒ f(p: P) → i32 ! ∅ {{ ⌥ p {{ P {{ x, z, .. }} ↦ x }} }}"),
        "E0304",
        1,
    );
}

#[test]
fn variant_and_tuple_patterns_check_arity_and_types() {
    ok("⊔ E { A(i32, bool), B } ƒ f(e: E) → i32 ! ∅ { ⌥ e { A(n, _) ↦ n, B ↦ 0 } }");
    ok("⊔ E { A(i32, bool), B } ƒ f(e: E) → i32 ! ∅ { ⌥ e { A(n, ..) ↦ n, B ↦ 0 } }");
    err(
        "⊔ E { A(i32, bool), B } ƒ f(e: E) → i32 ! ∅ { ⌥ e { A(n) ↦ n, B ↦ 0 } }",
        "E0304",
        1,
    );
    err(
        "⊔ E { A(i32), B } ƒ f(e: E) → i32 ! ∅ { ⌥ e { A(n, m) ↦ n, B ↦ 0 } }",
        "E0304",
        1,
    );
    err(
        "⊔ E { A(i32), B } ƒ f(e: E) → i32 ! ∅ { ⌥ e { A ↦ 1, B ↦ 0 } }",
        "E0304",
        1,
    );
    err(
        "⊔ E { A(i32), B } ƒ f(e: E) → i32 ! ∅ { ⌥ e { B(x) ↦ 1, A(_) ↦ 0 } }",
        "E0304",
        1,
    );
    ok("ƒ f(t: (i32, bool)) → i32 ! ∅ { ⌥ t { (a, _) ↦ a } }");
    ok("ƒ f(t: (i32, bool, u8)) → i32 ! ∅ { ⌥ t { (a, ..) ↦ a } }");
    err(
        "ƒ f(t: (i32, bool)) → i32 ! ∅ { ⌥ t { (a, b, c) ↦ a } }",
        "E0304",
        1,
    );
    // pattern sai kiểu
    err(
        "⊔ E { A } ⊔ F { B } ƒ f(e: E) → i32 ! ∅ { ⌥ e { B ↦ 1 } }",
        "E0301",
        1,
    );
    err(
        "ƒ f(x: i32) → i32 ! ∅ { ⌥ x { true ↦ 1, _ ↦ 2 } }",
        "E0301",
        1,
    );
}

#[test]
fn private_fields_are_private_to_their_module_e0203() {
    // cùng module: được; khác tệp: E0203 ở literal, truy cập và pattern
    ok("▦ m { ◈ ▢ S { a: i32 } ◈ ƒ mk() → S ! ∅ { S { a: 1 } } ƒ rd(s: S) → i32 ! ∅ { s.a } }");
    let f = |b: &str| run(&[("a.gt", "◈ ▢ S { x: i32, ◈ y: i32 }"), ("b.gt", b)]);
    let out = f("ƒ g() ! ∅ { s ≔ a::S { x: 1, y: 2 }; }");
    assert_eq!(codes(&out), ["E0203"], "{:?}", show(&out));
    let out = f("ƒ g(s: a::S) → i32 ! ∅ { s.x }");
    assert_eq!(codes(&out), ["E0203"], "{:?}", show(&out));
    let out = f("ƒ g(s: a::S) → i32 ! ∅ { ⌥ s { a::S { x, .. } ↦ x } }");
    assert_eq!(codes(&out), ["E0203"], "{:?}", show(&out));
    // trường ◈ thì được
    let out = f("ƒ g(s: a::S) → i32 ! ∅ { s.y }");
    assert!(out.report.diagnostics.is_empty(), "{:?}", show(&out));
}

#[test]
fn type_aliases_expand() {
    let out =
        ok("≡ Id ≔ u32; ≡ Pair<T> ≔ (T, T); ƒ f(a: Id, b: Pair<bool>) ! ∅ { x ≔ a; y ≔ b.0; }");
    assert_eq!(out.local_type("main::f", "x").as_deref(), Some("u32"));
    assert_eq!(out.local_type("main::f", "y").as_deref(), Some("bool"));
    err("≡ A ≔ A; ƒ f(a: A) ! ∅ { }", "E0301", 1);
    err("≡ Pair<T> ≔ (T, T); ƒ f(a: Pair) ! ∅ { }", "E0301", 1);
}

#[test]
fn constants_and_statics_have_their_declared_types() {
    ok("□ LIMIT: u32 ≔ 100; ■ GREETING: &str ≔ \"hi\"; ƒ f(x: u32) → bool ! ∅ { x <= LIMIT }");
    err("□ LIMIT: u32 ≔ true;", "E0301", 1);
    err("□ LIMIT: u32 ≔ 1i64;", "E0302", 1);
    // (literal ngoài phạm vi là E0601 của M7, không phải M4)
    ok("□ LIMIT: u8 ≔ 300;");
}
