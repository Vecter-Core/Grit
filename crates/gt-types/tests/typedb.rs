//! M5 bước 1 (ADR-020 §1, §9): giao diện gt-types → bước hạ MIR — `TypeDb` (`needs_drop`, `is_copy_resolved`),
//! bảng chỗ gọi (`sites`), phạm vi ràng buộc (`scope_of`), danh sách thân (`bodies`), `Drop` (Q26) và `RangeInclusive`.
mod common;
use common::*;
use gt_types::{BodyKind, CallTarget, Ty, TypesOutput};

fn node_of(out: &TypesOutput, key: &str) -> gt_ast::NodeId {
    *out.fn_keys
        .iter()
        .find(|(_, k)| k.as_str() == key)
        .unwrap_or_else(|| panic!("không có {key}"))
        .0
}

fn local(out: &TypesOutput, key: &str, name: &str) -> Ty {
    out.locals
        .values()
        .find(|(k, n, _)| k == key && n == name)
        .map(|(_, _, t)| t.clone())
        .unwrap_or_else(|| panic!("không có {name}"))
}

#[test]
fn needs_drop_follows_adr_020_q26() {
    let out = ok("▢ P { x: i32 } ▢ Q { b: Buf } ⊔ E { A, B(Buf) } \
                  ƒ f(a: i32, p: P, q: Q, e: E, b: Buf, r: &Buf, ar: Arena, t: (u8, Buf), arr: [Buf; 2], z: [Buf; 0]) ! ∅ { }");
    let k = "main::f";
    let db = &out.db;
    for (name, want) in [
        ("a", false),
        ("p", false),
        ("q", true),
        ("e", true),
        ("b", true),
        ("r", false),
        ("ar", true),
        ("t", true),
        ("arr", true),
        ("z", false),
    ] {
        assert_eq!(db.needs_drop(&local(&out, k, name)), want, "{name}");
    }
}

#[test]
fn type_parameters_need_drop_in_polymorphic_mir() {
    let out = ok("ƒ f<T>(x: T) ! ∅ { }");
    assert!(out.db.needs_drop(&local(&out, "main::f", "x")));
}

#[test]
fn is_copy_respects_the_bounds_in_scope() {
    let out = ok("ƒ f<T: Copy, U>(x: T, y: U, z: &u8, w: (i32, bool)) ! ∅ { }");
    let n = node_of(&out, "main::f");
    let scope = out.scope_of(n);
    assert!(
        out.db
            .is_copy_resolved(&local(&out, "main::f", "x"), &scope)
    );
    assert!(
        !out.db
            .is_copy_resolved(&local(&out, "main::f", "y"), &scope)
    );
    assert!(
        out.db
            .is_copy_resolved(&local(&out, "main::f", "z"), &scope)
    );
    assert!(
        out.db
            .is_copy_resolved(&local(&out, "main::f", "w"), &scope)
    );
    // ngoài phạm vi ràng buộc thì T không Copy
    assert!(!out.db.is_copy_resolved(&local(&out, "main::f", "x"), &[]));
}

#[test]
fn impl_bounds_are_in_scope_of_members() {
    let out = ok("▢ W<T> { v: T } ⊐ <T: Copy> W<T> { ƒ get(&self) → T ! ∅ { self.v } }");
    let (n, _) = out
        .fn_keys
        .iter()
        .find(|(_, k)| k.ends_with("::get"))
        .expect("get");
    assert_eq!(out.scope_of(*n).len(), 1);
}

#[test]
fn call_sites_record_target_generics_and_receiver_adjustment() {
    let out = ok("ƒ id<T>(x: T) → T ! ∅ { x } \
                  ▢ S { n: i32 } ⊐ S { ƒ get(&self) → i32 ! ∅ { self.n } } \
                  ƒ f(s: &&S) → i32 ! ∅ { a ≔ id(1u8); s.get() }");
    let id_node = node_of(&out, "main::id");
    let sites: Vec<_> = out.sites.values().collect();
    assert!(
        sites.iter().any(|s| s.target == CallTarget::Fn(id_node)
            && s.generics == vec![Ty::Int(gt_types::IntTy::U8)]),
        "{sites:?}"
    );
    let m = sites
        .iter()
        .find(|s| s.method_syntax)
        .expect("lời gọi phương thức");
    assert_eq!(m.derefs, 2);
    assert_eq!(m.self_kind, Some(gt_ast::SelfKind::Ref));
}

#[test]
fn trait_methods_on_type_parameters_stay_unresolved_until_mono() {
    let out = ok("⊏ A { ƒ area(&self) → i32 ! ∅; } ƒ f<T: A>(t: &T) → i32 ! ∅ { t.area() }");
    assert!(
        out.sites
            .values()
            .any(|s| matches!(s.target, CallTarget::TraitMethod { .. }) && s.method_syntax)
    );
}

#[test]
fn bodies_list_functions_members_consts_and_statics_but_not_the_prelude() {
    let out =
        ok("□ C: u8 ≔ 1; ■ S: u8 ≔ 2; ▢ P { n: i32 } ⊐ P { ƒ m(&self) ! ∅ { } } ƒ f() ! ∅ { }");
    let kinds: Vec<(BodyKind, bool)> = out
        .bodies
        .iter()
        .map(|b| (b.kind, b.impl_node.is_some()))
        .collect();
    assert_eq!(kinds.len(), 4, "{:?}", out.bodies);
    assert!(kinds.contains(&(BodyKind::Const, false)));
    assert!(kinds.contains(&(BodyKind::Static, false)));
    assert!(kinds.contains(&(BodyKind::Fn, true)));
    assert!(kinds.contains(&(BodyKind::Fn, false)));
    assert!(out.bodies.iter().all(|b| b.node.file().raw() != u32::MAX));
    // thứ tự tất định theo NodeId
    let mut sorted = out.bodies.clone();
    sorted.sort_by_key(|b| b.node);
    assert_eq!(sorted, out.bodies);
}

#[test]
fn user_drop_impls_outside_the_narrow_rule_are_rejected_e0313() {
    // ADR-C18: Drop người dùng được phép (kiểu cùng tệp, không tham số kiểu…); kiểu có tham số KIỂU thì vẫn E0313 (corpus phủ các luật còn lại)
    err(
        "▢ S<T> { n: T } ⊐ <T> Drop for S<T> { ƒ drop(&!self) ! ∅ { } }",
        "E0313",
        1,
    );
    err("#lang(drop) ⊏ MyDrop { }", "E1002", 1);
}

#[test]
fn inclusive_ranges_have_their_own_type() {
    assert_eq!(
        local_type_of("ƒ f() ! ∅ { x ≔ 1..=3; }", "x"),
        "RangeInclusive<i32>"
    );
    assert_eq!(local_type_of("ƒ f() ! ∅ { x ≔ 1..3; }", "x"), "Range<i32>");
    // dùng được như range thường: lặp, ∈
    ok("ƒ f() ! ∅ { r ≔ 0u8..=9; ⟳ i in r { } }");
    ok("ƒ f(x: u8) → bool ! ∅ { r ≔ 0u8..=9; x ∈ r }");
}

fn local_type_of(src: &str, name: &str) -> String {
    ok(src).local_type("main::f", name).expect("biến")
}
