//! M4 bước (c) tiếp: generics, trait tĩnh, bảng impl, ràng buộc, phân giải phương thức, kiểm hợp lệ impl (E0309, E0311),
//! đơn hình hoá lazy (bảng instance, E0312) — ADR-018 §1, §2.
mod common;
use common::*;

const AREA: &str = "⊏ Area { ƒ area(&self) → i32 ! ∅; } ▢ Sq { w: i32 } ⊐ Area for Sq { ƒ area(&self) → i32 ! ∅ { self.w } } ";

#[test]
fn generic_functions_infer_their_arguments_from_use() {
    let out = ok(
        "ƒ id<T>(x: T) → T ! ∅ { x } ƒ f() ! ∅ { a ≔ id(7); b ≔ id(true); c ≔ id(1u8); d ≔ id(\"s\"); }",
    );
    assert_eq!(out.local_type("main::f", "a").as_deref(), Some("i32"));
    assert_eq!(out.local_type("main::f", "b").as_deref(), Some("bool"));
    assert_eq!(out.local_type("main::f", "c").as_deref(), Some("u8"));
    assert_eq!(out.local_type("main::f", "d").as_deref(), Some("&str"));
    // kiểu mong đợi chảy ngược vào tham số kiểu
    let out = ok("ƒ id<T>(x: T) → T ! ∅ { x } ƒ f() ! ∅ { a: u8 ≔ id(7); }");
    assert_eq!(out.local_type("main::f", "a").as_deref(), Some("u8"));
    // hai tham số kiểu
    ok(
        "ƒ pair<A, B>(a: A, b: B) → (A, B) ! ∅ { (a, b) } ƒ f() → (i32, bool) ! ∅ { pair(1, true) }",
    );
    err(
        "ƒ same<T>(a: T, b: T) → T ! ∅ { a } ƒ f() ! ∅ { same(1u8, 2u16); }",
        "E0302",
        1,
    );
    err(
        "ƒ same<T>(a: T, b: T) → T ! ∅ { a } ƒ f() ! ∅ { same(1, true); }",
        "E0301",
        1,
    );
}

#[test]
fn generic_bodies_are_checked_once_against_their_bounds() {
    // thân generic không biết gì về T ngoài ràng buộc
    err("ƒ f<T>(a: T, b: T) → bool ! ∅ { a == b }", "E0301", 1);
    err("ƒ f<T>(a: T, b: T) → T ! ∅ { a + b }", "E0301", 1);
    err("ƒ f<T>(a: T) → i32 ! ∅ { a.area() }", "E0309", 1);
    ok("ƒ f<T>(a: T) → T ! ∅ { a }");
    // trả về T chỉ khớp T, không khớp i32
    err("ƒ f<T>(a: T) → i32 ! ∅ { a }", "E0301", 1);
}

#[test]
fn unresolvable_generic_calls_are_reported() {
    err(
        "ƒ mk<T>() → Option<T> ! ∅ { None } ƒ f() ! ∅ { x ≔ mk(); }",
        "E0301",
        1,
    );
    ok("ƒ mk<T>() → Option<T> ! ∅ { None } ƒ f() ! ∅ { x: Option<u8> ≔ mk(); }");
}

#[test]
fn trait_bounds_are_checked_at_the_call_site_e0309() {
    let defs = format!("{AREA}ƒ total<T: Area>(t: &T) → i32 ! ∅ {{ t.area() }} ");
    ok(&format!("{defs}ƒ f(s: Sq) → i32 ! ∅ {{ total(&s) }}"));
    err(
        &format!("{defs}ƒ f(x: i32) → i32 ! ∅ {{ total(&x) }}"),
        "E0309",
        1,
    );
    // chuyển tiếp ràng buộc: hàm generic không có ràng buộc thì không gọi được `total`
    err(
        &format!("{defs}ƒ g<U>(u: &U) → i32 ! ∅ {{ total(u) }}"),
        "E0309",
        1,
    );
    ok(&format!(
        "{defs}ƒ g<U: Area>(u: &U) → i32 ! ∅ {{ total(u) }}"
    ));
}

#[test]
fn method_calls_resolve_through_inherent_impls_traits_and_bounds() {
    let out = ok(&format!(
        "{AREA}▢ P {{ x: i32 }} ⊐ P {{ ƒ get(&self) → i32 ! ∅ {{ self.x }} ƒ new() → Self ! ∅ {{ Self {{ x: 0 }} }} }} \
         ƒ f(s: Sq, p: &P) ! ∅ {{ a ≔ s.area(); b ≔ p.get(); c ≔ P::new(); d ≔ Sq::area(&s); e ≔ Area::area(&s); }}"
    ));
    for n in ["a", "b", "d", "e"] {
        assert_eq!(out.local_type("main::f", n).as_deref(), Some("i32"), "{n}");
    }
    assert_eq!(out.local_type("main::f", "c").as_deref(), Some("P"));
    // trên tham số kiểu có ràng buộc
    ok(&format!(
        "{AREA}ƒ g<T: Area>(t: &T) → i32 ! ∅ {{ t.area() }}"
    ));
    // phương thức không tồn tại / sai số đối số / sai kiểu đối số
    err(&format!("{AREA}ƒ f(s: Sq) ! ∅ {{ s.nope(); }}"), "E0309", 1);
    err(
        &format!("{AREA}ƒ f(s: Sq) ! ∅ {{ s.area(1); }}"),
        "E0301",
        1,
    );
    err(
        &format!(
            "{AREA}▢ P {{ x: i32 }} ⊐ P {{ ƒ set(&!self, v: i32) ! ∅ {{ self.x ← v; }} }} ƒ f(p: &!P) ! ∅ {{ p.set(true); }}"
        ),
        "E0301",
        1,
    );
}

#[test]
fn trait_static_functions_and_self_in_traits() {
    ok(
        "⊏ Make { ƒ make() → Self ! ∅; } ▢ S { x: i32 } ⊐ Make for S { ƒ make() → Self ! ∅ { S { x: 1 } } } ƒ f<T: Make>() → T ! ∅ { T::make() } ƒ g() → S ! ∅ { f() }",
    );
    ok(
        "⊏ Dup { ƒ dup(&self) → Self ! ∅; } ▢ S { x: i32 } ⊐ Dup for S { ƒ dup(&self) → Self ! ∅ { S { x: self.x } } }",
    );
}

#[test]
fn impl_generics_and_member_generics() {
    let out = ok(
        "▢ W<T> { v: T } ⊐ <T> W<T> { ƒ get(&self) → T ! ∅ { self.v } ƒ map2<U>(&self, u: U) → U ! ∅ { u } } \
         ƒ f(w: W<u8>) ! ∅ { a ≔ w.get(); b ≔ w.map2(true); }",
    );
    assert_eq!(out.local_type("main::f", "a").as_deref(), Some("u8"));
    assert_eq!(out.local_type("main::f", "b").as_deref(), Some("bool"));
    // impl với ràng buộc inline: chỉ khớp khi ràng buộc thoả
    ok(&format!(
        "{AREA}▢ H<T> {{ t: T }} ⊐ <T: Area> H<T> {{ ƒ total(&self) → i32 ! ∅ {{ self.t.area() }} }} ƒ f(h: H<Sq>) → i32 ! ∅ {{ h.total() }}"
    ));
}

// ======================= E0311: impl không hợp lệ =======================

#[test]
fn e0311_trait_impl_must_match_the_trait() {
    let t = "⊏ T { ƒ a(&self) → i32 ! ∅; ƒ b(&self, x: i32) ! ∅; } ▢ S { x: i32 } ";
    ok(&format!(
        "{t}⊐ T for S {{ ƒ a(&self) → i32 ! ∅ {{ 1 }} ƒ b(&self, x: i32) ! ∅ {{ }} }}"
    ));
    for bad in [
        "⊐ T for S { ƒ a(&self) → i32 ! ∅ { 1 } }", // thiếu b
        "⊐ T for S { ƒ a(&self) → i32 ! ∅ { 1 } ƒ b(&self, x: i32) ! ∅ { } ƒ c(&self) ! ∅ { } }", // thừa c
        "⊐ T for S { ƒ a(&self) → bool ! ∅ { true } ƒ b(&self, x: i32) ! ∅ { } }", // sai kiểu trả về
        "⊐ T for S { ƒ a(&self) → i32 ! ∅ { 1 } ƒ b(&self, x: bool) ! ∅ { } }", // sai kiểu tham số
        "⊐ T for S { ƒ a(&!self) → i32 ! ∅ { 1 } ƒ b(&self, x: i32) ! ∅ { } }", // sai kiểu self
        "⊐ T for S { ƒ a(&self) → i32 ! ∅ { 1 } ƒ b(&self) ! ∅ { } }",          // sai số tham số
    ] {
        err(&format!("{t}{bad}"), "E0311", 1);
    }
}

#[test]
fn e0311_overlapping_impls() {
    let t = "⊏ T { ƒ a(&self) ! ∅; } ▢ S { x: i32 } ";
    err(
        &format!("{t}⊐ T for S {{ ƒ a(&self) ! ∅ {{ }} }} ⊐ T for S {{ ƒ a(&self) ! ∅ {{ }} }}"),
        "E0311",
        1,
    );
    // impl generic chồng lấn với impl cụ thể
    err(
        &format!(
            "{t}⊐ <U> T for U {{ ƒ a(&self) ! ∅ {{ }} }} ⊐ T for S {{ ƒ a(&self) ! ∅ {{ }} }}"
        ),
        "E0311",
        1,
    );
    // hai kiểu khác nhau: không chồng lấn
    ok(&format!(
        "{t}▢ R {{ y: i32 }} ⊐ T for S {{ ƒ a(&self) ! ∅ {{ }} }} ⊐ T for R {{ ƒ a(&self) ! ∅ {{ }} }}"
    ));
    // inherent trùng tên hàm
    err(
        "▢ S { x: i32 } ⊐ S { ƒ a(&self) ! ∅ { } } ⊐ S { ƒ a(&self) ! ∅ { } }",
        "E0311",
        1,
    );
    ok("▢ S { x: i32 } ⊐ S { ƒ a(&self) ! ∅ { } } ⊐ S { ƒ b(&self) ! ∅ { } }");
}

#[test]
fn e0309_ambiguous_methods_from_two_traits() {
    let defs = "⊏ A { ƒ m(&self) ! ∅; } ⊏ B { ƒ m(&self) ! ∅; } ▢ S { x: i32 } ⊐ A for S { ƒ m(&self) ! ∅ { } } ⊐ B for S { ƒ m(&self) ! ∅ { } } ";
    err(&format!("{defs}ƒ f(s: S) ! ∅ {{ s.m(); }}"), "E0309", 1);
    // gọi tường minh qua trait thì được
    ok(&format!("{defs}ƒ f(s: S) ! ∅ {{ A::m(&s); B::m(&s); }}"));
}

#[test]
fn inherent_methods_win_over_trait_methods() {
    ok(
        "⊏ A { ƒ m(&self) → i32 ! ∅; } ▢ S { x: i32 } ⊐ A for S { ƒ m(&self) → i32 ! ∅ { 1 } } ⊐ S { ƒ m(&self) → bool ! ∅ { true } } ƒ f(s: S) → bool ! ∅ { s.m() }",
    );
}

// ======================= đơn hình hoá lazy (ADR-018 §1) =======================

#[test]
fn instances_are_collected_lazily_from_non_generic_roots() {
    let out = ok(
        "ƒ id<T>(x: T) → T ! ∅ { x } ƒ unused<T>(x: T) → T ! ∅ { x } ƒ main() ! ∅ { id(1u8); id(true); id(1u8); id(2u8); }",
    );
    assert_eq!(
        out.instance_names(),
        ["main::id<bool>", "main::id<u8>", "main::main"]
    );
}

#[test]
fn instances_follow_calls_transitively_and_through_trait_methods() {
    let out = ok(&format!(
        "{AREA}ƒ total<T: Area>(t: &T) → i32 ! ∅ {{ t.area() }} ƒ outer<U: Area>(u: &U) → i32 ! ∅ {{ total(u) }} ƒ main(s: Sq) → i32 ! ∅ {{ outer(&s) }}"
    ));
    let names = out.instance_names();
    assert!(names.contains(&"main::outer<Sq>".to_string()), "{names:?}");
    assert!(names.contains(&"main::total<Sq>".to_string()), "{names:?}");
    // lời gọi qua ràng buộc được giải thành hàm của impl cụ thể
    assert!(names.iter().any(|n| n.contains("area")), "{names:?}");
}

#[test]
fn the_instance_table_is_sorted_and_deterministic() {
    let src = "ƒ id<T>(x: T) → T ! ∅ { x } ƒ main() ! ∅ { id(true); id(1u8); id(2i64); id('c'); }";
    let a = ok(src).instance_names();
    let b = ok(src).instance_names();
    assert_eq!(a, b);
    let mut sorted = a.clone();
    sorted.sort();
    assert_eq!(a, sorted);
}

#[test]
fn e0312_polymorphic_recursion_is_cut_off_at_the_depth_limit() {
    err(
        "ƒ f<T>(x: T) ! ∅ { f((x,)); } ƒ main() ! ∅ { f(1u8); }",
        "E0312",
        1,
    );
    // đệ quy đa hình KHÔNG nở kiểu thì hữu hạn
    ok("ƒ f<T>(x: T, n: u32) ! ∅ { ⎇ n > 0 { f(x, n - 1); } } ƒ main() ! ∅ { f(1u8, 3); }");
    // nở nhưng không có gốc dùng nó: không có instance để cắt (hàm generic không được gọi)
    ok("ƒ f<T>(x: T) ! ∅ { f((x,)); }");
}

/// FB-C10: `⊐ <K: Copy> Copy for E<K>` phải được thấy khi `K` chỉ là tham số có ràng buộc `K: Copy` trong phạm vi
/// (trước đây `find_impl` kiểm ràng buộc impl với phạm vi RỖNG ⇒ `E<K>` bị coi là không-Copy ⇒ E0318 trong `Vec<E<K>>`).
#[test]
fn generic_copy_adt_is_copy_under_its_scope_bounds_fb_c10() {
    let defs = "▢ E<K> { k: K } ⊐ <K: Copy> Copy for E<K> { } ";
    ok(&format!(
        "{defs}ƒ f<'a, K: Copy>(v: &Vec<'a, E<K>>) → usize ! ∅ {{ v.len() }}"
    ));
    ok(&format!(
        "{defs}ƒ g<K: Copy>(e: E<K>) → (E<K>, E<K>) ! ∅ {{ (e, e) }}"
    ));
    // fail-closed: không có `K: Copy` trong phạm vi ⇒ vẫn E0318
    err(
        &format!("{defs}ƒ h<'a, K>(v: &Vec<'a, E<K>>) → usize ! ∅ {{ v.len() }}"),
        "E0318",
        1,
    );
}

/// FB-C13: `==`/`≠` chỉ nhận kiểu mà MỌI backend cài đúng: số nguyên/bool/char và `&str`. Tuple và `&T` (T ≠ str) được M4 chấp nhận trước đây
/// nhưng backend LLVM từ chối (`Eq trên Ref`) và thông dịch so ĐỊA CHỈ ⇒ hai bên bất đồng; nay là E0301 ngay ở cổng.
#[test]
fn equality_is_limited_to_scalars_and_str_fb_c13() {
    ok("ƒ f(a: &str, b: &str) → bool ! ∅ { a == b }");
    ok("ƒ f(a: &str, b: &str) → bool ! ∅ { a != b }");
    ok("ƒ f(a: u8, b: u8) → bool ! ∅ { a == b }");
    ok("ƒ f(a: char, b: char) → bool ! ∅ { a == b }");
    err(
        "ƒ f(a: (u32, bool), b: (u32, bool)) → bool ! ∅ { a == b }",
        "E0301",
        1,
    );
    err("ƒ f(a: &u32, b: &u32) → bool ! ∅ { a == b }", "E0301", 1);
    ok("ƒ f(a: &u32, b: &u32) → bool ! ∅ { *a == *b }");
}

/// FB-C14: `x ≔ 1.5; x as i32` (biến nhận literal số thực KHÔNG chú thích) lọt qua M4 chỉ với N0604 vì kiểu còn là biến suy diễn số thực
/// khi kiểm `as`; phải là E0319 như `x: f64`/`1.5f64` (ADR-035 Q70).
#[test]
fn float_to_int_cast_through_an_inferred_float_literal_is_e0319_fb_c14() {
    err("ƒ f() → i32 ! ∅ { x ≔ 1.5; x as i32 }", "E0319", 1);
    err("ƒ f() → i32 ! ∅ { (2.5) as i32 }", "E0319", 1);
    err("ƒ f(y: f64) → i32 ! ∅ { y as i32 }", "E0319", 1);
    ok("ƒ f() → f64 ! ∅ { x ≔ 1.5; x }");
    ok("ƒ f() → f64 ! ∅ { n ≔ 3; n as f64 }");
}

/// FB-C16 (kiểu (a), NGHIÊM TRỌNG): `⊐ <T> Copy for W<T> { }` (thiếu `T: Copy`) làm `W<Cap<Out>>`/`W<Arena>` thành Copy ⇒ nhân đôi capability/tay cầm Arena.
/// `validate_copy_impl` bỏ qua trường kiểu tham số (`Ty::Param ⇒ continue`) và thay tham số ADT bằng biến suy diễn (luôn "Copy"). Phải: mọi trường, SAU khi thế bằng
/// đối số của `self_ty` của impl, là Copy dưới các ràng buộc của CHÍNH impl.
#[test]
fn copy_impl_must_bound_every_type_parameter_it_stores_fb_c16() {
    err("▢ W<T> { v: T } ⊐ <T> Copy for W<T> { }", "E0311", 1);
    // ADT lồng: `In<T>` chỉ Copy khi `T: Copy` ⇒ `H<T>` cần ràng buộc đó trên impl của nó
    let nest = "▢ In<T> { v: T } ⊐ <T: Copy> Copy for In<T> { } ▢ H<T> { o: In<T> } ";
    err(&format!("{nest}⊐ <T> Copy for H<T> {{ }}"), "E0311", 1);
    ok(&format!("{nest}⊐ <T: Copy> Copy for H<T> {{ }}"));
    ok("▢ W<T> { v: T } ⊐ <T: Copy> Copy for W<T> { }");
    ok("▢ W<T> { v: T } ⊐ Copy for W<u32> { }");
    // thân không tham số nhưng impl cụ thể cho kiểu không Copy
    err("▢ W<T> { v: T } ⊐ Copy for W<Cap<Out>> { }", "E0311", 1);
}

/// FB-C17 (kiểu (a), NGHIÊM TRỌNG): `Arena` (và `Mutex`/`Atomic`/`Cell`) không bao giờ `Copy`: `⊐ Copy for Arena { }` nhân đôi Arena ⇒ `reset` bản này, đọc qua bản kia = dùng sau giải phóng.
#[test]
fn copy_is_forbidden_for_arena_and_interior_mutable_lang_types_fb_c17() {
    for t in ["Arena", "Mutex<u32, ()>", "Atomic<u32>", "Cell<u32>"] {
        let src = format!("⊐ Copy for {t} {{ }}");
        let c = codes(&one(&src));
        assert!(
            !c.is_empty() && c.iter().all(|x| x == "E0313" || x == "E0311"),
            "{t}: {c:?}"
        );
    }
}

/// FB-C17b: `Cell<T>::get` đòi `T: Copy` (nếu không, `Cell<Arena>::get` nhân đôi giá trị không Copy mà không cần impl Copy).
#[test]
fn cell_get_requires_copy_fb_c17() {
    err("ƒ f(c: &Cell<Arena>) ! {mem} { a ≔ c.get(); }", "E0309", 1);
    ok("ƒ f(c: &Cell<u32>) → u32 ! ∅ { c.get() }");
}

/// FB-C19 (kiểu (a)): `==` trên biến suy diễn TỔNG QUÁT (chưa biết kiểu lúc gặp `==`) lọt qua `eq_ok(Infer) = true`; sau đó biến được hợp nhất với `f64`/tuple ⇒ bỏ qua cấm D4 (E0303)
/// và cấm tuple (FB-C13). Phải kiểm lại SAU khi kiểu đã giải.
#[test]
fn equality_on_a_later_resolved_inference_variable_is_rechecked_fb_c19() {
    err(
        "ƒ f() → bool ! ∅ { μx ≔ None; μr ≔ ⌥ x { Some(v) ↦ v == v, None ↦ false }; x ← Some(1.5); r }",
        "E0303",
        1,
    );
    err(
        "ƒ f() → bool ! ∅ { μx ≔ None; μr ≔ ⌥ x { Some(v) ↦ v == v, None ↦ false }; x ← Some((1, 2)); r }",
        "E0301",
        1,
    );
    ok(
        "ƒ f() → bool ! ∅ { μx ≔ None; μr ≔ ⌥ x { Some(v) ↦ v == v, None ↦ false }; x ← Some(3); r }",
    );
}
