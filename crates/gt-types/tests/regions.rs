//! M4.5: region (lifetime có tên) ở chữ ký — khai báo, elision, bảng `RegionSig` cho M6 (E0315, `docs/spec/regions.md`).
mod common;
use common::*;
use gt_types::{RegionRef, RegionSig, TypesOutput};

fn sig(out: &TypesOutput, key: &str) -> RegionSig {
    let node = out
        .fn_keys
        .iter()
        .find(|(_, k)| k.as_str() == key)
        .unwrap_or_else(|| panic!("không có hàm {key}: {:?}", out.fn_keys.values()))
        .0;
    out.regions[node].clone()
}

fn n(s: &str) -> RegionRef {
    RegionRef::Named(s.to_string())
}

#[test]
fn a_single_input_reference_gives_the_output_its_region() {
    let out = ok("ƒ first(xs: &[i32]) → &i32 ! ∅ { &xs[0] }");
    let s = sig(&out, "main::first");
    assert_eq!(s.inputs, vec![vec![RegionRef::Anon(0)]]);
    assert_eq!(s.output, vec![RegionRef::Anon(0)]);
    assert!(s.declared.is_empty());
}

#[test]
fn named_regions_are_recorded_in_order_of_appearance() {
    let out = ok("ƒ f<'a, 'b>(x: &'a i32, y: &'b i32) → &'a i32 ! ∅ { x }");
    let s = sig(&out, "main::f");
    assert_eq!(s.declared, ["a", "b"]);
    assert_eq!(s.inputs, vec![vec![n("a")], vec![n("b")]]);
    assert_eq!(s.output, vec![n("a")]);
}

#[test]
fn nested_references_record_every_region_position_in_preorder() {
    let out = ok("ƒ f<'a, 'b>(x: &'a &'b i32) → &'b i32 ! ∅ { *x }");
    let s = sig(&out, "main::f");
    assert_eq!(s.inputs, vec![vec![n("a"), n("b")]]);
    assert_eq!(s.output, vec![n("b")]);
    let out = ok("ƒ g<'a>(o: Option<&'a i32>, t: (&'a u8, u8)) ! ∅ { }");
    assert_eq!(
        sig(&out, "main::g").inputs,
        vec![vec![n("a")], vec![n("a")]]
    );
}

#[test]
fn no_input_reference_means_static_output() {
    let out = ok("ƒ name() → &str ! ∅ { \"grit\" }");
    assert_eq!(sig(&out, "main::name").output, vec![RegionRef::Static]);
    let out = ok("ƒ s(x: &'static str) → &'static str ! ∅ { x }");
    let s = sig(&out, "main::s");
    assert_eq!(s.inputs, vec![vec![RegionRef::Static]]);
    assert_eq!(s.output, vec![RegionRef::Static]);
}

#[test]
fn methods_count_self_as_the_first_input() {
    let out = ok(
        "▢ S { n: i32 } ⊐ S { ƒ get(&self) → &i32 ! ∅ { &self.n } ƒ own(self) → i32 ! ∅ { self.n } }",
    );
    let key = out
        .fn_keys
        .values()
        .find(|k| k.ends_with("::get"))
        .expect("get")
        .clone();
    let s = sig(&out, &key);
    assert_eq!(s.inputs, vec![vec![RegionRef::Anon(0)]]);
    assert_eq!(s.output, vec![RegionRef::Anon(0)]);
    let key = out
        .fn_keys
        .values()
        .find(|k| k.ends_with("::own"))
        .expect("own")
        .clone();
    assert_eq!(sig(&out, &key).inputs, vec![Vec::<RegionRef>::new()]);
}

#[test]
fn impl_and_trait_lifetimes_are_in_scope_of_their_members() {
    ok("▢ S { n: i32 } ⊐ <'a> S { ƒ f(&self, x: &'a i32) → &'a i32 ! ∅ { x } }");
    ok("⊏ T<'a> { ƒ f(&self, x: &'a i32) → &'a i32 ! ∅; }");
}

#[test]
fn every_type_position_is_checked_for_undeclared_lifetimes() {
    for bad in [
        "ƒ f(x: &'a i32) ! ∅ { }",
        "ƒ f(x: & &'a i32) ! ∅ { }",
        "ƒ f(x: Option<&'a i32>) ! ∅ { }",
        "ƒ f(x: (i32, &'a i32)) ! ∅ { }",
        "ƒ f(x: [&'a i32; 2]) ! ∅ { }",
        "ƒ f(x: &[&'a i32]) ! ∅ { }",
        "ƒ f() → &'a i32 ! ∅ { &1 }",
        "ƒ f() ! ∅ { x: &'a i32 ≔ &1; }",
        "ƒ f() ! ∅ { x ≔ 1 as u8; y: Option<&'a u8> ≔ None; }",
        "▢ S { r: &'a i32 }",
        "▢ S<'b> { r: &'a i32 }",
        "⊔ E { A(&'a i32) }",
        "▢ S { n: i32 } ⊐ S { ƒ f(&self, x: &'a i32) ! ∅ { } }",
        "⊏ T { ƒ f(&self, x: &'a i32) ! ∅; }",
    ] {
        err(bad, "E0315", 1);
    }
}

#[test]
fn declared_lifetimes_are_accepted_in_the_same_positions() {
    for good in [
        "ƒ f<'a>(x: &'a i32) ! ∅ { }",
        "ƒ f<'a>(x: & &'a i32) ! ∅ { }",
        "ƒ f<'a>(x: Option<&'a i32>) ! ∅ { }",
        "ƒ f<'a>(x: (i32, &'a i32)) ! ∅ { }",
        "ƒ f<'a>(x: [&'a i32; 2]) ! ∅ { }",
        "ƒ f<'a>(x: &[&'a i32]) ! ∅ { }",
        "ƒ f<'a>(x: &'a i32) ! ∅ { y: &'a i32 ≔ x; }",
        "▢ S<'a> { r: &'a i32 }",
        "⊔ E<'a> { A(&'a i32) }",
        "ƒ f() → &'static str ! ∅ { \"x\" }",
        "ƒ f() ! ∅ { x: &'static i32 ≔ &1; }",
    ] {
        ok(good);
    }
}

#[test]
fn lifetime_declarations_may_not_repeat_or_shadow() {
    err("ƒ f<'a, 'a>(x: &'a i32) ! ∅ { }", "E0315", 1);
    err("▢ S<'a, 'a> { r: &'a i32 }", "E0315", 1);
    err(
        "▢ S { n: i32 } ⊐ <'a> S { ƒ f<'a>(&self) ! ∅ { } }",
        "E0315",
        1,
    );
    // hai hàm khác nhau dùng lại cùng tên là bình thường
    ok("ƒ f<'a>(x: &'a i32) ! ∅ { } ƒ g<'a>(x: &'a i32) ! ∅ { }");
}

#[test]
fn elision_in_the_return_type_needs_exactly_one_input_reference() {
    ok("ƒ f(x: &i32) → &i32 ! ∅ { x }");
    ok("ƒ f(x: &'static i32) → &i32 ! ∅ { x }");
    ok("ƒ f() → &i32 ! ∅ { &1 }");
    for bad in [
        "ƒ f(x: &i32, y: &i32) → &i32 ! ∅ { x }",
        "ƒ f(x: & &i32) → &i32 ! ∅ { *x }",
        "ƒ f(x: Option<&i32>, y: &i32) → &i32 ! ∅ { y }",
        "ƒ f<'a>(x: &'a i32, y: &i32) → &i32 ! ∅ { x }",
        "▢ S { n: i32 } ⊐ S { ƒ f(&self, k: &i32) → &i32 ! ∅ { k } }",
    ] {
        err(bad, "E0315", 1);
    }
    // ghi tường minh thì hai tham chiếu vào là được
    ok("ƒ f<'a>(x: &'a i32, y: &i32) → &'a i32 ! ∅ { x }");
}

#[test]
fn elision_errors_are_reported_once_per_signature_and_mention_the_count() {
    let out = err(
        "ƒ f(x: &i32, y: &i32) → (&i32, &i32) ! ∅ { (x, y) }",
        "E0315",
        1,
    );
    assert!(
        out.report.diagnostics[0].message.contains("2 vị trí"),
        "{:?}",
        show(&out)
    );
}

#[test]
fn elided_fields_are_rejected_in_every_nested_field_position() {
    for bad in [
        "▢ S { r: &i32 }",
        "▢ S { r: Option<&i32> }",
        "▢ S { r: (u8, &i32) }",
        "▢ S { r: [&i32; 2] }",
        "⊔ E { A(&i32) }",
        "⊔ E { A(u8, Option<&u8>) }",
    ] {
        err(bad, "E0315", 1);
    }
}

#[test]
fn arena_borrows_follow_the_ordinary_elision_rule() {
    // `alloc(&self, v) → &T` có một tham chiếu vào (self): kết quả mang region của lần mượn Arena
    let out = ok("ƒ f<'r>(a: &'r Arena) → i32 ! {mem} { p ≔ a.alloc(41); *p +% 1 }");
    let s = sig(&out, "main::f");
    assert_eq!(s.declared, ["r"]);
    assert_eq!(s.inputs, vec![vec![n("r")]]);
}

#[test]
fn the_prelude_arena_signature_has_a_single_borrowed_region() {
    // chữ ký của prelude cũng đi qua cùng bảng: `Arena::alloc` là `ƒ alloc<T>(&self, v: T) → &T`
    let out = ok("ƒ f() ! ∅ { }");
    assert!(
        out.regions
            .values()
            .any(|s| s.inputs == vec![vec![RegionRef::Anon(0)], vec![]]
                && s.output == vec![RegionRef::Anon(0)]),
        "không thấy chữ ký alloc của prelude"
    );
}

const HOLDER: &str = "▢ Holder<'a> { r: &'a i32 } ▢ Pair<'a, T> { r: &'a T, v: T } ";

#[test]
fn lifetime_arguments_in_types_are_checked_adr_021() {
    ok(&format!(
        "{HOLDER}ƒ get<'a>(h: Holder<'a>) → &'a i32 ! ∅ {{ h.r }}"
    ));
    ok(&format!(
        "{HOLDER}ƒ get<'a>(p: Pair<'a, u8>) → u8 ! ∅ {{ p.v }}"
    ));
    ok(&format!("{HOLDER}▢ Outer<'a> {{ h: Holder<'a> }}"));
    ok(&format!("{HOLDER}ƒ f(h: Holder<'static>) ! ∅ {{ }}"));
    for bad in [
        "ƒ f(h: Holder<'x>) ! ∅ { }",
        "ƒ f<'a, 'b>(h: Holder<'a, 'b>) ! ∅ { }",
        "ƒ f<'a>(h: Option<'a, u8>) ! ∅ { }",
        "ƒ f<'a, T>(h: T<'a>) ! ∅ { }",
        "▢ Outer<'a> { h: Holder }",
        "▢ Outer { h: Pair<u8> }",
        "⊔ E { A(Holder) }",
    ] {
        err(&format!("{HOLDER}{bad}"), "E0315", 1);
    }
}

#[test]
fn elided_lifetime_arguments_count_as_input_positions() {
    // `Holder` bỏ trống ở chữ ký: một vị trí ẩn — đủ để elision đầu ra
    let out = ok(&format!("{HOLDER}ƒ f(h: Holder) → &i32 ! ∅ {{ h.r }}"));
    let s = sig(&out, "main::f");
    assert_eq!(s.inputs, vec![vec![RegionRef::Anon(0)]]);
    assert_eq!(s.output, vec![RegionRef::Anon(0)]);
    // `&Holder` có hai vị trí (`&` và `'a` của Holder) — đầu ra ẩn là mơ hồ
    err(
        &format!("{HOLDER}ƒ f(h: &Holder) → &i32 ! ∅ {{ h.r }}"),
        "E0315",
        1,
    );
    // ghi tên: thứ tự duyệt trước — `&` rồi tham số lifetime của Pair, rồi đối số kiểu
    let out = ok(&format!(
        "{HOLDER}ƒ g<'a, 'b>(p: &'a Pair<'b, &'a u8>) → &'b &'a u8 ! ∅ {{ p.r }}"
    ));
    let s = sig(&out, "main::g");
    assert_eq!(s.inputs, vec![vec![n("a"), n("b"), n("a")]]);
    assert_eq!(s.output, vec![n("b"), n("a")]);
}

#[test]
fn the_self_type_of_an_impl_contributes_region_positions_that_elision_does_not_count() {
    let src = "▢ H<'a> { r: &'a i32 } ⊐ <'a> H<'a> { ƒ get(&self) → &'a i32 ! ∅ { self.r } ƒ me(self) → Self ! ∅ { self } ƒ first(&self) → &i32 ! ∅ { &1 } }";
    let out = one(src);
    let get = out
        .fn_keys
        .iter()
        .find(|(_, k)| k.ends_with("::get"))
        .map(|(n, _)| *n)
        .expect("get");
    let s = &out.regions[&get];
    assert_eq!(s.inputs, vec![vec![RegionRef::Anon(0), n("a")]]);
    assert_eq!(s.input_self, vec![vec![false, true]]);
    assert_eq!(s.output, vec![n("a")]);
    let me = out
        .fn_keys
        .iter()
        .find(|(_, k)| k.ends_with("::me"))
        .map(|(n, _)| *n)
        .expect("me");
    let s = &out.regions[&me];
    assert_eq!(s.inputs, vec![vec![n("a")]]);
    assert_eq!(s.output, vec![n("a")]);
    assert_eq!(s.output_self, vec![true]);
    // `first(&self) → &i32`: elision chỉ đếm `&self` (một vị trí) — KHÔNG đếm `'a` của Self
    let first = out
        .fn_keys
        .iter()
        .find(|(_, k)| k.ends_with("::first"))
        .map(|(n, _)| *n)
        .expect("first");
    assert_eq!(out.regions[&first].output, vec![RegionRef::Anon(0)]);
}

#[test]
fn trait_impl_may_not_promise_less_about_lifetimes_than_the_trait_e0311() {
    let t = "⊏ T { ƒ pick<'b, 'c>(&self, y: &'b i32, z: &'c i32) → &'b i32 ! ∅; } ▢ S { n: i32 } ";
    ok(&format!(
        "{t}⊐ T for S {{ ƒ pick<'b, 'c>(&self, y: &'b i32, z: &'c i32) → &'b i32 ! ∅ {{ y }} }}"
    ));
    // impl hứa kết quả gắn với `z` thay vì `y`: người gọi qua trait tin chữ ký trait ⇒ không được
    err(
        &format!(
            "{t}⊐ T for S {{ ƒ pick<'b, 'c>(&self, y: &'b i32, z: &'c i32) → &'c i32 ! ∅ {{ z }} }}"
        ),
        "E0311",
        1,
    );
    // đổi tên lifetime nhưng cùng quan hệ: được
    ok(&format!(
        "{t}⊐ T for S {{ ƒ pick<'p, 'q>(&self, y: &'p i32, z: &'q i32) → &'p i32 ! ∅ {{ y }} }}"
    ));
    // impl hứa NHIỀU hơn (kết quả 'static) thì được
    ok(&format!(
        "{t}⊐ T for S {{ ƒ pick<'b, 'c>(&self, y: &'b i32, z: &'c i32) → &'static i32 ! ∅ {{ core::abort(\"n\") }} }}"
    ));
    // ngược lại: trait hứa 'static, impl chỉ hứa gắn với self ⇒ lỗi
    err(
        "⊏ U { ƒ k(&self) → &'static i32 ! ∅; } ▢ S { n: i32 } ⊐ U for S { ƒ k(&self) → &i32 ! ∅ { &self.n } }",
        "E0311",
        1,
    );
}
