//! Điều kiện dừng (M8 nửa 1; semantics §21): E0701 / E0702 / E0703 trên chương trình nhỏ, kết quả CHÍNH XÁC.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_oblig::termination::check_termination;

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

#[test]
fn counting_loops_are_inferred_or_verified() {
    ok(
        "ƒ f(xs: &[i64]) → i64 ! ∅ { μs ≔ 0; μi ≔ 0; ⟳ i < xs.len() ↓ xs.len() - i { s ← s +% xs[i]; i ← i + 1; } s }",
    );
    // tự suy từ điều kiện, không `↓`
    ok("ƒ f(n: u32) → u32 ! ∅ { μi ≔ 0u32; ⟳ i < n { i ← i + 1; } i }");
    ok("ƒ f(n: u32) → u32 ! ∅ { μi ≔ n; ⟳ i > 0 { i ← i - 1; } i }");
    ok("ƒ f(n: u32) → u32 ! ∅ { μi ≔ 0u32; ⟳ i < n { i ← i + 2; } i }");
    ok("ƒ f(n: u32) → u32 ! ∅ { μi ≔ 0u32; ⟳ i <= n { i ← i + 1; } i }");
    ok("ƒ f(n: u32) → u32 ! ∅ { μi ≔ n; ⟳ i >= 1 { i ← i - 1; } i }");
    // một vế của `∧` đủ; vế kia tuỳ ý
    ok("ƒ f(xs: &[u8]) → usize ! ∅ { μi ≔ 0; ⟳ i < xs.len() ∧ xs[i] > 0 { i ← i + 1; } i }");
    ok("ƒ f(n: u32, k: u32) → u32 ! ∅ { μi ≔ 0u32; ⟳ k > 5 ∧ i < n { i ← i + 1; } i }");
    // `↓` có dấu, bị chặn dưới nhờ điều kiện
    ok("ƒ f() → i32 ! ∅ { μi ≔ 10i32; ⟳ i > 0 ↓ i { i ← i - 1; } i }");
    // for-in hữu hạn tự giảm
    ok("ƒ f(n: u32) → u32 ! ∅ { μs ≔ 0u32; ⟳ i in 0..n { s ← s +% i; } s }");
    ok("ƒ f(xs: &[u32]) → u32 ! ∅ { μs ≔ 0u32; ⟳ x in xs { s ← s +% *x; } s }");
    // vô hạn có `div`
    ok("ƒ f() → () ! {div} { ⟳ { } }");
    // lồng nhau: mỗi vòng có biến thể riêng
    ok(
        "ƒ f(n: u32) → u32 ! ∅ { μi ≔ 0u32; ⟳ i < n { μj ≔ 0u32; ⟳ j < 3 { j ← j + 1; } i ← i + 1; } i }",
    );
}

#[test]
fn loops_without_provable_progress_are_red() {
    // không có `↓`, thân không tiến
    only("ƒ f(n: u32) → () ! ∅ { μi ≔ 0u32; ⟳ i < n { } }", "E0701");
    // thân đi NGƯỢC chiều
    only(
        "ƒ f(n: u32) → () ! ∅ { μi ≔ 5u32; ⟳ i < n { i ← i +% 0; } }",
        "E0701",
    );
    // bước do hàm khác quyết định (mẫu có sẵn trong tests/fail)
    only(
        "ƒ next(i: u32) → u32 ! ∅ { i }\nƒ f(n: u32) → () ! ∅ { μi ≔ 0; ⟳ i < n { i ← next(i); } }",
        "E0701",
    );
    // điều kiện không phải so sánh theo biến nguyên
    only(
        "ƒ g(i: u32) → bool ! ∅ { i < 3 }\nƒ f() → () ! ∅ { μi ≔ 0u32; ⟳ g(i) { i ← i + 1; } }",
        "E0701",
    );
    // `!=` không dùng để suy biến thể
    only(
        "ƒ f(n: u32) → () ! ∅ { μi ≔ 0u32; ⟳ i != n { i ← i + 1; } }",
        "E0701",
    );
    // vô hạn thiếu `div`
    only("ƒ f() → () ! ∅ { ⟳ { } }", "E0701");
    // `∨` không cho vế nào chắc chắn tiếp tục
    only(
        "ƒ f(n: u32, k: u32) → () ! ∅ { μi ≔ 0u32; ⟳ i < n ∨ k > 0 { i ← i + 1; } }",
        "E0701",
    );
}

#[test]
fn given_variants_must_decrease_and_be_bounded() {
    // không giảm (mẫu có sẵn trong tests/fail)
    only(
        "ƒ f() → () ! ∅ { μi ≔ 0; ⟳ i < 10 ↓ i { i ← i +% 1; } }",
        "E0702",
    );
    // đứng yên
    only(
        "ƒ f(n: u32) → () ! ∅ { μi ≔ 0u32; ⟳ i < n ↓ n - i { } }",
        "E0702",
    );
    // giảm nhưng KHÔNG bị chặn dưới (có thể âm khi vẫn tiếp tục)
    only(
        "ƒ f() → () ! ∅ { μi ≔ 5i32; ⟳ i > -100 ↓ i { i ← i - 1; } }",
        "E0702",
    );
    // biểu thức ngoài dạng hỗ trợ
    only(
        "ƒ f(n: u32) → () ! ∅ { μi ≔ 0u32; ⟳ i < n ↓ n * n { i ← i + 1; } }",
        "E0702",
    );
    // biến thể đúng nhưng thân đổi biến khác
    only(
        "ƒ f(n: u32, m: u32) → () ! ∅ { μi ≔ 0u32; μj ≔ 0u32; ⟳ i < n ↓ m - j { j ← j + 1; } }",
        "E0702",
    );
}

#[test]
fn recursion_needs_a_decreasing_variant_or_div() {
    ok("ƒ fact(n: u32) → u64 ! ∅ ↓ n { ⎇ n == 0 { 1 } ⇝ { n as u64 *% fact(n - 1) } }");
    ok("ƒ f(n: u32) → u32 ! {div} { f(n) }");
    ok("ƒ g(n: u32) → u32 ! ∅ { n }");
    // mutual: even/odd
    ok(
        "ƒ even(n: u32) → bool ! ∅ ↓ n { ⎇ n == 0 { true } ⇝ { odd(n - 1) } }\nƒ odd(n: u32) → bool ! ∅ ↓ n { ⎇ n == 0 { false } ⇝ { even(n - 1) } }",
    );
    // thiếu `↓`
    only("ƒ f(n: u32) → u32 ! ∅ { f(n) }", "E0703");
    only(
        "ƒ even(n: u32) → bool ! ∅ { odd(n) }\nƒ odd(n: u32) → bool ! ∅ { even(n) }",
        "E0703",
    );
    // chỉ một hàm của chu trình thiếu `↓`
    let c = codes("ƒ even(n: u32) → bool ! ∅ ↓ n { odd(n) }\nƒ odd(n: u32) → bool ! ∅ { even(n) }");
    assert!(c.contains(&"E0703".to_string()), "{c:?}");
    // có `↓` nhưng lời gọi không giảm
    only("ƒ f(n: u32) → u32 ! ∅ ↓ n { f(n) }", "E0702");
    // có `↓` nhưng lời gọi giảm mà không bị chặn dưới (n - 1 có thể âm khi n = 0)
    only("ƒ f(n: i32) → i32 ! ∅ ↓ n { f(n - 1) }", "E0702");
    // tham số bị gán lại thì biến thể "lúc vào" không còn: bị bác
    only(
        "ƒ f(μn: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { n ← n - 1; f(n) } }",
        "E0702",
    );
}

#[test]
#[ignore]
fn dump_term() {
    let src = std::env::var("GT_SRC").expect("GT_SRC");
    let (t, m) = common::mir(&src);
    let info = build_prog_info(&t, &m);
    let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
    for r in &o.per_body {
        for j in &r.judged {
            if j.tag.is_some() || j.ob.kind == gt_oblig::ir::ObKind::Variant {
                eprintln!(
                    "{} {:?} {:?}\n goal {}\n hyps {:?}",
                    j.ob.id,
                    j.ob.kind,
                    j.status,
                    j.ob.goal.to_text(),
                    j.ob.hyps.iter().map(|h| h.to_text()).collect::<Vec<_>>()
                );
            }
        }
    }
}

#[test]
fn methods_multiplication_and_leading_conjuncts() {
    // đệ quy qua phương thức trong impl
    ok(
        "▢ S { v: i32 }\n⊐ S { ƒ down(&self, n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { self.down(n - 1) } } }",
    );
    only(
        "▢ S { v: i32 }\n⊐ S { ƒ down(&self, n: u32) → u32 ! ∅ { self.down(n) } }",
        "E0703",
    );
    // vế đầu của `∧` không dịch được (lời gọi / chỉ số) không che vế sau
    ok(
        "ƒ g(i: u32) → bool ! ∅ { true }\nƒ f(n: u32) → u32 ! ∅ { μi ≔ 0u32; ⟳ g(i) ∧ i < n { i ← i + 1; } i }",
    );
    ok("ƒ f(xs: &[u32], n: u32) → u32 ! ∅ { μi ≔ 0u32; ⟳ xs[0] > 0 ∧ i < n { i ← i + 1; } i }");
}

#[test]
fn call_graph_components_are_exact() {
    // chu trình 3 hàm, không hàm nào có ↓: cả ba bị báo, hàm gọi vào chu trình thì không
    let c = codes(
        "ƒ a(n: u32) → u32 ! ∅ { b(n) }\nƒ b(n: u32) → u32 ! ∅ { c(n) }\nƒ c(n: u32) → u32 ! ∅ { a(n) }\nƒ d(n: u32) → u32 ! ∅ { a(n) }",
    );
    assert_eq!(c, vec!["E0703"; 3], "{c:?}");
    // kim cương không đệ quy: không được gộp thành một thành phần
    ok(
        "ƒ h(n: u32) → u32 ! ∅ { n }\nƒ g(n: u32) → u32 ! ∅ { h(n) }\nƒ f(n: u32) → u32 ! ∅ { g(n) + h(n) }\nƒ m(n: u32) → u32 ! ∅ { f(n) + g(n) }",
    );
    // hai thành phần độc lập, chỉ một cái có lỗi
    let c = codes(
        "ƒ p(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { q(n - 1) } }\nƒ q(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { p(n - 1) } }\nƒ r(n: u32) → u32 ! ∅ { r(n) }",
    );
    assert_eq!(c, vec!["E0703"], "{c:?}");
    // chu trình nằm sau một nhánh đã duyệt
    let c = codes(
        "ƒ top(n: u32) → u32 ! ∅ { leaf(n) + x(n) }\nƒ leaf(n: u32) → u32 ! ∅ { n }\nƒ x(n: u32) → u32 ! ∅ { y(n) + leaf(n) }\nƒ y(n: u32) → u32 ! ∅ { x(n) }",
    );
    assert_eq!(c, vec!["E0703"; 2], "{c:?}");
}

#[test]
fn infinite_loop_with_variant_still_needs_div() {
    let c = codes("ƒ f(n: u32) → () ! ∅ { μi ≔ 0u32; ⟳ ↓ n { i ← i + 1; } }");
    assert!(c.contains(&"E0701".to_string()), "{c:?}");
}

#[test]
fn indirect_recursion_is_rejected_even_with_a_variant() {
    // qua giá trị hàm: chu trình không kiểm được biến thể giảm
    let c = codes("ƒ f(n: u32) → u32 ! ∅ ↓ n { μg: ƒ(u32) → u32 ! ∅ ≔ f; g(n) }");
    assert!(c.contains(&"E0703".to_string()), "{c:?}");
    // qua phương thức trait
    let c = codes(
        "⊏ T { ƒ m(&self, n: u32) → u32 ! ∅; }\n▢ S { v: i32 }\nƒ r<X: T>(x: &X, n: u32) → u32 ! ∅ ↓ n { x.m(n) }\n⊐ T for S { ƒ m(&self, n: u32) → u32 ! ∅ ↓ n { r(self, n) } }",
    );
    assert!(c.contains(&"E0703".to_string()), "{c:?}");
    // trait nhưng không có chu trình: sạch
    ok(
        "⊏ T { ƒ m(&self, n: u32) → u32 ! ∅; }\n▢ S { v: i32 }\nƒ r<X: T>(x: &X, n: u32) → u32 ! ∅ { x.m(n) }\n⊐ T for S { ƒ m(&self, n: u32) → u32 ! ∅ { n } }",
    );
}

#[test]
fn recursive_variant_never_uses_a_modified_parameter() {
    // tham số bị gán lại TĂNG: `↓ n` theo giá trị hiện tại sẽ "giảm" giả
    only(
        "ƒ f(μn: u32) → u32 ! ∅ ↓ n { n ← n +% 5; ⎇ n == 0 { 0 } ⇝ { f(n - 1) } }",
        "E0702",
    );
    // tham số bị sửa qua `&!` (không có phép gán trực tiếp)
    only(
        "ƒ f(μn: u32) → u32 ! ∅ ↓ n { p ≔ &!n; *p ← *p +% 5; ⎇ n == 0 { 0 } ⇝ { f(n - 1) } }",
        "E0702",
    );
    // hai biến trong biến thể, chỉ một bị gán lại: vẫn bị bác (phải TẤT CẢ nguyên vẹn)
    only(
        "ƒ f(n: u32, μm: u32) → u32 ! ∅ ↓ n + m { m ← m +% 5; ⎇ n == 0 { 0 } ⇝ { f(n - 1, m) } }",
        "E0702",
    );
    // tham số không đụng tới: sạch
    ok("ƒ f(n: u32, m: u32) → u32 ! ∅ ↓ n + m { ⎇ n == 0 { 0 } ⇝ { f(n - 1, m) } }");
}

#[test]
fn calls_between_distinct_recursive_components_need_no_decrease() {
    // f và g đều tự đệ quy nhưng KHÔNG thuộc cùng chu trình: f gọi g với cùng n là hợp lệ
    ok(
        "ƒ g(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { g(n - 1) } }\nƒ f(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { f(n - 1) + g(n) } }",
    );
}
