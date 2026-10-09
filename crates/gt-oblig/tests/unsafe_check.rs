//! E1001 (D21, Q13): mọi `⚠` ở mã người dùng và con trỏ thô ngoài `⚠` đều ĐỎ; mã an toàn thì sạch.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::unsafe_check::check_unsafe;

fn n(src: &str) -> usize {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        common::mir_codes(&m)
    );
    let d = check_unsafe(&t, &m);
    assert!(d.iter().all(|x| x.code.as_str() == "E1001"));
    d.len()
}

#[test]
fn every_unsafe_block_is_red() {
    assert_eq!(n("ƒ f(p: usize) → i32 ! ∅ { ⚠ { raw_read(p) } }"), 1);
    assert_eq!(n("ƒ f() ! ∅ { ⚠ { } }"), 1);
    // hai khối ⚠ riêng ⇒ hai chẩn đoán
    assert_eq!(
        n("ƒ f(p: usize) ! ∅ { ⚠ { raw_read(p); } ⚠ { raw_read(p); } }"),
        2
    );
    // ⚠ lồng nhau: báo một lần cho khối ngoài cùng
    assert_eq!(n("ƒ f(p: usize) ! ∅ { ⚠ { ⚠ { raw_read(p); } } }"), 1);
    // trong phương thức impl
    assert_eq!(
        n("▢ S { v: i32 }\n⊐ S { ƒ m(&self, p: usize) → i32 ! ∅ { ⚠ { raw_read(p) } } }"),
        1
    );
    // trong nhánh / vòng lặp
    assert_eq!(
        n("ƒ f(p: usize, c: bool) ! ∅ { ⎇ c { ⚠ { raw_read(p); } } }"),
        1
    );
}

#[test]
fn raw_pointer_types_outside_unsafe_are_red() {
    assert_eq!(n("ƒ f(p: *□ i32) → i32 ! ∅ { 0 }"), 1);
    assert_eq!(n("ƒ f(p: *μ i32) ! ∅ { }"), 1);
    assert_eq!(n("ƒ f() → *□ i32 ! ∅ { f() }"), 1);
    assert_eq!(n("▢ H { p: *□ i32 }"), 1);
    assert_eq!(n("⊔ E { A(*μ u8), B }"), 1);
    assert_eq!(n("□ P: usize ≔ 0;\nƒ f(x: &*□ i32) ! ∅ { }"), 1);
    assert_eq!(n("▢ S { v: i32 }\n⊐ S { ƒ m(&self, p: *□ u8) ! ∅ { } }"), 1);
    // lồng trong kiểu khác
    assert_eq!(n("ƒ f(p: Option<*□ i32>) ! ∅ { }"), 1);
    assert_eq!(n("ƒ f(p: (u8, *μ u8)) ! ∅ { }"), 1);
    assert_eq!(n("ƒ f(p: [*□ u8; 2]) ! ∅ { }"), 1);
}

#[test]
fn safe_programs_are_clean() {
    assert_eq!(n("ƒ f(x: &i32) → i32 ! ∅ { *x }"), 0);
    assert_eq!(
        n("▢ H { n: i32, s: usize }\nƒ f(h: &H) → i32 ! ∅ { h.n }"),
        0
    );
    assert_eq!(n("ƒ f(p: usize) → usize ! ∅ { p + 1 }"), 0);
    assert_eq!(n("ƒ f(a: &[u8]) → usize ! ∅ { a.len() }"), 0);
    assert_eq!(
        n("▢ S { v: i32 }\n⊐ S { ƒ m(&self) → i32 ! ∅ { self.v } }"),
        0
    );
    // prelude (lib/core) có `raw_read(usize)` nhưng KHÔNG bị kiểm
    assert_eq!(n("ƒ f() ! ∅ { }"), 0);
}

#[test]
fn trait_members_function_types_and_deep_types() {
    // chữ ký trait (không thân)
    assert_eq!(n("⊏ T { ƒ m(&self, p: *□ u8) ! ∅; }"), 1);
    assert_eq!(n("⊏ T { ƒ m(&self) → *μ u8 ! ∅; }"), 1);
    assert_eq!(n("⊏ T { ƒ m(&self, p: usize) ! ∅; }"), 0);
    // kiểu hàm chứa con trỏ thô (tham số hoặc kết quả)
    assert_eq!(n("ƒ f(g: ƒ(*□ u8) ! ∅) ! ∅ { }"), 1);
    assert_eq!(n("ƒ f(g: ƒ(u8) → *□ u8 ! ∅) ! ∅ { }"), 1);
    assert_eq!(n("ƒ f(g: ƒ(u8) → u8 ! ∅) ! ∅ { }"), 0);
    // kiểu lồng RẤT sâu: con trỏ thô ở đáy vẫn bị thấy (và quá sâu thì từ chối, fail-closed)
    let deep = |inner: &str| {
        let mut ty = inner.to_string();
        for _ in 0..40 {
            ty = format!("Option<{ty}>");
        }
        format!("ƒ f(x: {ty}) ! ∅ {{ }}")
    };
    assert_eq!(n(&deep("*□ u8")), 1);
    assert_eq!(
        n(&deep("u8")),
        1,
        "kiểu quá sâu không đoán: coi là có con trỏ thô"
    );
}

#[test]
fn the_diagnostic_points_inside_the_unsafe_block() {
    let src = "ƒ f(p: usize) ! ∅ { a ≔ 1; ⚠ { b ≔ 2; raw_read(p); } c ≔ 3; }";
    let (t, m) = common::mir(src);
    let d = check_unsafe(&t, &m);
    assert_eq!(d.len(), 1);
    let (open, close) = (src.find('⚠').expect("⚠"), src.find("} c").expect("}"));
    let start = d[0].span.start as usize;
    assert!(
        start > open && start < close,
        "span {start} phải nằm trong khối ⚠ [{open}, {close}]"
    );
    // khối lồng: vẫn trỏ vào khối ngoài cùng
    let src = "ƒ f(p: usize) ! ∅ { ⚠ { a ≔ 1; ⚠ { raw_read(p); } } }";
    let (t, m) = common::mir(src);
    let d = check_unsafe(&t, &m);
    assert_eq!(d.len(), 1);
    let start = d[0].span.start as usize;
    assert!(start > src.find('⚠').expect("⚠"), "{start}");
}
