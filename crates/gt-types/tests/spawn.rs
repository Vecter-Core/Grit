//! `spawn` và `Send`-like tự suy (M8, ADR-026 §2): E0501 khi đối số chứa tham chiếu, con trỏ thô, `Arena`, hoặc tham số kiểu trần.
mod common;
use common::*;

fn only(src: &str, code: &str) {
    let out = one(src);
    let c = codes(&out);
    assert!(
        !c.is_empty() && c.iter().all(|x| x == code),
        "kỳ vọng chỉ {code}, có {c:?}\n{src}"
    );
}

const WORKER: &str = "ƒ work(n: u32) ! ∅ { }\n";

#[test]
fn plain_data_can_be_spawned() {
    ok(&format!("{WORKER}ƒ main() ! ∅ {{ spawn(work, 5u32); }}"));
    ok(
        "▢ P { a: i32, b: (u8, bool) }\nƒ w(p: P) ! ∅ { }\nƒ main() ! ∅ { spawn(w, P { a: 1, b: (2, true) }); }",
    );
    ok("⊔ E { A(u32), B }\nƒ w(e: E) ! ∅ { }\nƒ main() ! ∅ { spawn(w, E::A(3)); }");
    ok("ƒ w(a: [u8; 4]) ! ∅ { }\nƒ main() ! ∅ { spawn(w, [1, 2, 3, 4]); }");
    // tham số kiểu có ràng buộc Send
    ok("ƒ w<T>(x: T) ! ∅ { }\nƒ run<T: Send>(x: T) ! ∅ { spawn(w, x); }");
}

#[test]
fn references_pointers_arenas_and_bare_params_are_not_send() {
    only(
        "ƒ w(r: &u32) ! ∅ { }\nƒ main() ! ∅ { x ≔ 5u32; spawn(w, &x); }",
        "E0501",
    );
    only(
        "ƒ w(r: &!u32) ! ∅ { }\nƒ main() ! ∅ { μx ≔ 5u32; spawn(w, &!x); }",
        "E0501",
    );
    // tham chiếu nằm sâu trong ADT
    only(
        "▢ H<'a> { r: &'a u32 }\nƒ w(h: H) ! ∅ { }\nƒ main() ! ∅ { x ≔ 5u32; spawn(w, H { r: &x }); }",
        "E0501",
    );
    only(
        "⊔ E<'a> { A(&'a u32), B }\nƒ w(e: E) ! ∅ { }\nƒ main() ! ∅ { spawn(w, E::B); }",
        "E0501",
    );
    // Arena
    only(
        "ƒ w(a: Arena) ! ∅ { }\nƒ run(a: Arena) ! ∅ { spawn(w, a); }",
        "E0501",
    );
    // tham số kiểu trần
    only(
        "ƒ w<T>(x: T) ! ∅ { }\nƒ run<T>(x: T) ! ∅ { spawn(w, x); }",
        "E0501",
    );
}

#[test]
fn send_is_automatic() {
    only("▢ S { v: i32 }\n⊐ Send for S { }", "E0313");
}

#[test]
fn composite_types_are_checked_through_generics_and_recursion() {
    // ADT generic: Send phụ thuộc đối số kiểu
    ok("▢ W<T> { v: T }\nƒ w(x: W<u32>) ! ∅ { }\nƒ main() ! ∅ { spawn(w, W { v: 1u32 }); }");
    only(
        "▢ W<T> { v: T }\nƒ w(x: W<&u32>) ! ∅ { }\nƒ main() ! ∅ { n ≔ 1u32; spawn(w, W { v: &n }); }",
        "E0501",
    );
    // ADT đệ quy (qua Option): không lặp vô hạn; sạch khi mọi trường sạch
    ok(
        "⊔ L { Nil, Cons(u32, Option<u32>) }\nƒ w(l: L) ! ∅ { }\nƒ main() ! ∅ { spawn(w, L::Nil); }",
    );
    // lồng sâu: tuple trong mảng trong ADT
    only(
        "▢ D<'a> { x: [(u8, &'a u32); 2] }\nƒ w(d: D) ! ∅ { }\nƒ run(d: D) ! ∅ { spawn(w, d); }",
        "E0501",
    );
    ok("▢ D { x: [(u8, u32); 2] }\nƒ w(d: D) ! ∅ { }\nƒ run(d: D) ! ∅ { spawn(w, d); }");
    // hàm tĩnh (không bắt biến) chuyển được
    ok("ƒ g(n: u32) ! ∅ { }\nƒ w(f: ƒ(u32) ! ∅) ! ∅ { }\nƒ main() ! ∅ { spawn(w, g); }");
    // con trỏ thô
    only(
        "ƒ w(p: *□ u32) ! ∅ { }\nƒ run(p: *□ u32) ! ∅ { spawn(w, p); }",
        "E0501",
    );
}

#[test]
fn send_bounds_belong_to_their_own_parameter() {
    // T chỉ có Copy, U có Send: x: T vẫn không Send
    only(
        "ƒ w<T>(x: T) ! ∅ { }\nƒ run<T: Copy, U: Send>(x: T, y: U) ! ∅ { spawn(w, x); }",
        "E0501",
    );
    ok("ƒ w<T>(x: T) ! ∅ { }\nƒ run<T: Copy, U: Send>(x: T, y: U) ! ∅ { spawn(w, y); }");
}

#[test]
fn a_clean_sibling_never_hides_a_dirty_field() {
    // hai ADT khác nhau, đều không đối số: ADT sạch đi trước không được che ADT bẩn
    only(
        "▢ Fine { v: u32 }\n▢ Bad<'a> { r: &'a u32 }\n▢ Top<'a> { a: Fine, b: Bad<'a> }\nƒ w(t: Top) ! ∅ { }\nƒ run(t: Top) ! ∅ { spawn(w, t); }",
        "E0501",
    );
    // cùng ADT với đối số kiểu khác nhau: bản sạch không được che bản bẩn
    only(
        "▢ W<T> { v: T }\n▢ Top<'a> { a: W<u32>, b: W<&'a u32> }\nƒ w(t: Top) ! ∅ { }\nƒ run(t: Top) ! ∅ { spawn(w, t); }",
        "E0501",
    );
    // tuple ngay ở gốc
    ok("ƒ w(t: (u8, u32)) ! ∅ { }\nƒ main() ! ∅ { spawn(w, (1u8, 2u32)); }");
    only(
        "ƒ w(t: (u8, &u32)) ! ∅ { }\nƒ run(t: (u8, &u32)) ! ∅ { spawn(w, t); }",
        "E0501",
    );
}

#[test]
fn recursive_types_terminate_the_check() {
    ok("▢ N { v: u32, next: Option<N> }\nƒ w(n: N) ! ∅ { }\nƒ run(n: N) ! ∅ { spawn(w, n); }");
    only(
        "▢ N<'a> { r: &'a u32, next: Option<N<'a>> }\nƒ w(n: N) ! ∅ { }\nƒ run(n: N) ! ∅ { spawn(w, n); }",
        "E0501",
    );
}

#[test]
fn polymorphic_recursion_is_cut_off_by_the_depth_guard() {
    // G<T> chứa G<(T, T)>: kiểu mở rộng vô hạn, `seen` không bao giờ trùng; chốt độ sâu phải dừng và từ chối (fail-closed)
    only(
        "▢ G<T> { v: T, g: Option<G<(T, T)>> }\nƒ w(x: G<u32>) ! ∅ { }\nƒ run(x: G<u32>) ! ∅ { spawn(w, x); }",
        "E0501",
    );
}

/// Đột biến sống sót (send.rs:67 `== → !=`): thông điệp phải gọi ĐÚNG tên đặc tính bị thiếu (Send vs Sync), không đảo nhau.
#[test]
fn the_message_names_the_missing_auto_trait_correctly() {
    let msgs = |src: &str| -> Vec<String> {
        one(src)
            .report
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect()
    };
    // thiếu Send (tham số kiểu trần được spawn)
    let m = msgs("ƒ w<T>(x: T) ! ∅ { }\nƒ run<T>(x: T) ! ∅ { spawn(w, x); }");
    assert!(
        m.iter().any(|s| s.contains("ràng buộc `Send`")),
        "thiếu Send mà thông điệp không nêu Send: {m:?}"
    );
    // thiếu Sync (static chứa Cell)
    let m = msgs("■ C: Cell<u32> ≔ Cell::new(1u32);");
    assert!(m.iter().any(|s| s.contains("Sync")), "{m:?}");
}
