//! M4 bước (d): `Secret<T>`/`Cap<T>` ở mức kiểu (không Copy, không tạo bằng literal ngoài lib/core, không `⊐ Copy for`),
//! `Copy` và `#lang` (E0313, E0311, E1002) — ADR-018 §2, §5.
mod common;
use common::*;

const DUP: &str = "ƒ dup<T: Copy>(x: T) → T ! ∅ { x } ";

#[test]
fn copy_is_known_for_primitives_references_and_aggregates() {
    for good in [
        "ƒ f() ! ∅ { dup(1); dup(1u8); dup(1.5); dup(true); dup('c'); dup(()); }",
        "ƒ f(x: i32) ! ∅ { dup(&x); }",
        "ƒ f() ! ∅ { dup((1, true)); dup([1u8, 2]); dup(\"s\"); }",
        "ƒ g(x: i32) → i32 ! ∅ { x } ƒ f() ! ∅ { dup(g); }",
    ] {
        ok(&format!("{DUP}{good}"));
    }
}

#[test]
fn unique_references_and_unsized_and_non_copy_adts_are_not_copy_e0309() {
    for bad in [
        "ƒ f(x: &!i32) ! ∅ { dup(x); }",
        "▢ P { x: i32 } ƒ f(p: P) ! ∅ { dup(p); }",
        "⊔ E { A } ƒ f(e: E) ! ∅ { dup(e); }",
        "ƒ f() ! ∅ { dup((1, Some(2))); }",
        "▢ Q { x: i32 } ƒ f(q: Q) ! ∅ { dup((1, q)); }",
    ] {
        err(&format!("{DUP}{bad}"), "E0309", 1);
    }
}

#[test]
fn user_types_are_copy_only_through_a_valid_copy_impl() {
    ok(&format!(
        "{DUP}▢ P {{ x: i32, y: bool }} ⊐ Copy for P {{ }} ƒ f(p: P) ! ∅ {{ dup(p); }}"
    ));
    ok(&format!(
        "{DUP}⊔ E {{ A, B(i32) }} ⊐ Copy for E {{ }} ƒ f(e: E) ! ∅ {{ dup(e); }}"
    ));
    // trường không Copy làm impl không hợp lệ
    err("▢ Q { x: i32 } ▢ P { q: Q } ⊐ Copy for P { }", "E0311", 1);
    err("▢ P<'a> { x: &!'a i32 } ⊐ Copy for P { }", "E0311", 1);
    // Copy cho kiểu không phải ADT của người dùng
    err("⊐ Copy for i32 { }", "E0311", 1);
    // generic: trường kiểu tham số do ràng buộc của impl quyết định
    ok(&format!(
        "{DUP}▢ W<T> {{ v: T }} ⊐ <T: Copy> Copy for W<T> {{ }} ƒ f(w: W<u8>) ! ∅ {{ dup(w); }}"
    ));
    err(
        &format!(
            "{DUP}▢ W<T> {{ v: T }} ⊐ <T: Copy> Copy for W<T> {{ }} ƒ f(w: W<&!u8>) ! ∅ {{ dup(w); }}"
        ),
        "E0309",
        1,
    );
}

#[test]
fn secret_and_cap_are_never_copy_and_cannot_be_declared_copy() {
    // không Copy (ràng buộc không thoả)
    err(
        &format!("{DUP}ƒ f(k: Secret<u64>) ! ∅ {{ dup(k); }}"),
        "E0309",
        1,
    );
    err(
        &format!("{DUP}ƒ f(c: Cap<Fs>) ! ∅ {{ dup(c); }}"),
        "E0309",
        1,
    );
    // &Cap thì Copy (tham chiếu chung)
    ok(&format!("{DUP}ƒ f(c: &Cap<Fs>) ! ∅ {{ dup(c); }}"));
    // không thể khai báo Copy cho chúng
    err("⊐ Copy for Secret<u64> { }", "E0313", 1);
    err("⊐ Copy for Cap<Fs> { }", "E0313", 1);
    err("⊐ <T> Copy for Secret<T> { }", "E0313", 1);
}

#[test]
fn secret_and_cap_cannot_be_forged_with_a_literal_outside_the_core() {
    err("ƒ f() ! ∅ { k ≔ Secret { v: 1u64 }; }", "E0313", 1);
    err("ƒ f() ! ∅ { c ≔ Cap { v: 1 }; }", "E0313", 1);
    // nhận từ nguồn hợp lệ thì được
    ok("ƒ main() ! ∅ { c ≔ root_fs(); }");
    ok("ƒ pass_through(k: Secret<u64>) → Secret<u64> ! ∅ { k }");
    // kiểu thường thì tạo được
    ok("▢ P { x: i32 } ƒ f() ! ∅ { p ≔ P { x: 1 }; }");
}

#[test]
fn capability_methods_come_from_the_prelude_impls() {
    let out =
        ok("ƒ read_len(fs: &Cap<Fs>, path: &str) → usize ! {io} { f ≔ fs.read(path); f.len() }");
    assert_eq!(
        out.local_type("main::read_len", "f").as_deref(),
        Some("File")
    );
    // Cap<Net> không có `read` (impl chỉ cho Cap<Fs>)
    err("ƒ f(c: &Cap<Net>) ! ∅ { c.read(\"x\"); }", "E0309", 1);
}

#[test]
fn lang_attribute_is_rejected_outside_the_prelude_e1002() {
    err("#lang(copy) ⊏ MyCopy { }", "E1002", 1);
    err("#lang(secret) ▢ S { x: i32 }", "E1002", 1);
    err("#lang(option) ⊔ O<T> { N, S(T) }", "E1002", 1);
    err("#lang(range) ▢ R<T> { a: T }", "E1002", 1);
    // trên hàm
    err("#lang(cap) ƒ f() ! ∅ { }", "E1002", 1);
    // thuộc tính khác thì không bị ảnh hưởng
    ok("#allow(S101, \"lý do\") ƒ f() ! ∅ { }");
    // tên lang lạ cũng là E1002 (dùng ngoài prelude)
    err("#lang(whatever) ▢ S { x: i32 }", "E1002", 1);
}

#[test]
fn a_lang_attribute_does_not_make_a_user_type_special() {
    // dù bị E1002, kiểu người dùng không trở thành Secret/Cap (không bị cấm literal vì lý do lang)
    let out = one("#lang(secret) ▢ S { x: i32 } ƒ f() ! ∅ { s ≔ S { x: 1 }; }");
    assert_eq!(codes(&out), ["E1002"], "{:?}", show(&out));
}
