//! Đồ thị gọi theo INSTANCE của M8 (ADR-033 phụ lục): các quyết định mới phải có test BẮT đột biến (lệnh Giang 2026-10-14, 1d).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::check_termination;

fn codes(src: &str) -> Vec<String> {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        common::mir_codes(&m)
    );
    let info = build_prog_info(&t, &m);
    let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
    let mut v: Vec<String> = o
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    v.sort();
    v
}

fn ok(src: &str) {
    let c = codes(src);
    assert!(c.is_empty(), "kỳ vọng sạch, có {c:?}\n{src}");
}

fn only(src: &str, code: &str) {
    let c = codes(src);
    assert!(
        !c.is_empty() && c.iter().all(|x| x == code),
        "kỳ vọng chỉ {code}, có {c:?}\n{src}"
    );
}

/// Hàm đệ quy tương hỗ cùng một SCC dùng CHUNG số hiệu: biến thể phải giảm nghiêm ngặt trên MỌI cạnh của SCC (nếu số hiệu bị tách thì M7
/// không còn kiểm cạnh nào ⇒ chấp nhận đệ quy tương hỗ không giảm = FAIL-OPEN).
#[test]
fn mutual_recursion_variants_are_checked_across_the_whole_scc() {
    only(
        "ƒ ping(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { pong(n) } }\nƒ pong(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { ping(n - 1) } }",
        "E0702",
    );
    // ba hàm, chỉ cạnh cuối không giảm
    only(
        "ƒ a(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { b(n - 1) } }\nƒ b(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { c(n - 1) } }\nƒ c(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { a(n) } }",
        "E0702",
    );
    // giảm trên mọi cạnh: sạch
    ok(
        "ƒ ping(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { pong(n - 1) } }\nƒ pong(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { ping(n - 1) } }",
    );
}

/// Một hàm đệ quy có `div` đứng TRƯỚC một hàm đệ quy thường: hàm sau vẫn phải bị kiểm (không dừng ở hàm `div`).
#[test]
fn a_div_function_does_not_hide_later_recursive_functions() {
    only(
        "ƒ r1(n: u32) → u32 ! {div} { r1(n) }\nƒ r2(n: u32) → u32 ! ∅ { r2(n) }",
        "E0703",
    );
    // hàm không đệ quy đứng trước, hàm đệ quy sau
    only(
        "ƒ plain(n: u32) → u32 ! ∅ { n }\nƒ rec(n: u32) → u32 ! ∅ { rec(n) }",
        "E0703",
    );
}

/// Hàm vừa có cạnh TRỰC TIẾP giảm đúng, vừa có cạnh qua phương thức trait nằm trong cùng SCC: cạnh trait không kiểm được biến thể nên
/// PHẢI bị báo E0703 (không được chấp nhận chỉ vì còn một cạnh trực tiếp).
#[test]
fn a_trait_edge_in_the_scc_is_flagged_even_next_to_a_checked_direct_edge() {
    only(
        "⊏ T { ƒ t(&self, n: u32) → u32 ! ∅; }\n▢ S { }\n⊐ T for S { ƒ t(&self, n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { x ≔ S { }; f(&x, n - 1) } } }\nƒ f<U: T>(u: &U, n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { u.t(n - 1) +% f(u, n - 1) } }\nƒ main() → u32 ! ∅ { s ≔ S { }; f(&s, 3) }",
        "E0703",
    );
}

/// Phán quyết theo đồ thị instance: chuỗi hữu hạn hợp lệ, vòng thật bị từ chối (kể cả qua nhiều tầng và hai trait).
#[test]
fn instance_graph_verdicts() {
    let w = "▢ W<T> { i: T }\n";
    ok(&format!(
        "⊏ A {{ ƒ a(&self) → i32 ! ∅; }}\n▢ S {{ }}\n{w}⊐ A for S {{ ƒ a(&self) → i32 ! ∅ {{ 1 }} }}\n⊐ <T: A> A for W<T> {{ ƒ a(&self) → i32 ! ∅ {{ self.i.a() +% 10 }} }}\nƒ main() → i32 ! ∅ {{ x ≔ W {{ i: W {{ i: S {{ }} }} }}; x.a() }}"
    ));
    let cyc = codes(&format!(
        "⊏ A {{ ƒ a(&self) → i32 ! ∅; }}\n▢ X {{ }}\n{w}⊐ A for X {{ ƒ a(&self) → i32 ! ∅ {{ y ≔ W {{ i: W {{ i: X {{ }} }} }}; y.a() }} }}\n⊐ <T: A> A for W<T> {{ ƒ a(&self) → i32 ! ∅ {{ self.i.a() }} }}\nƒ main() → i32 ! ∅ {{ x ≔ X {{ }}; x.a() }}"
    ));
    assert!(
        !cyc.is_empty() && cyc.iter().all(|c| c == "E0703"),
        "{cyc:?}"
    );
}

/// FB-C15 (kiểu (a), M8): hàm dùng làm giá trị qua `[f; N]` (`Rvalue::Repeat`) không được ghi nhận là "thoát" ⇒ không có cạnh tới `f` ⇒ đệ quy qua giá trị hàm không bị E0703.
/// Đối chứng: `g ≔ f; g(n)` (Use) bị E0703.
#[test]
fn recursion_through_a_repeated_fn_value_is_still_e0703_fb_c15() {
    only(
        "ƒ f(n: u32) → u32 ! ∅ { μa ≔ [f; 2]; μg ≔ a[0]; g(n) } ƒ main() ! ∅ { μr ≔ f(1); }",
        "E0703",
    );
    only(
        "ƒ f(n: u32) → u32 ! ∅ { μg ≔ f; g(n) } ƒ main() ! ∅ { μr ≔ f(1); }",
        "E0703",
    );
}

/// Generic specialization first encountered as a repeated function value must enter the instance graph too.
#[test]
fn generic_recursion_through_a_repeated_fn_value_is_e0703() {
    only(
        "ƒ f<T: Copy>(n: u32, x: T) → u32 ! ∅ { μa ≔ [f; 2]; μg ≔ a[0]; g(n, x) } ƒ main() → u32 ! ∅ { f(1u32, 2u32) }",
        "E0703",
    );
}
