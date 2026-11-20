//! GÓI 11 S1.b: test giết các đột biến SỐNG SÓT của `gt-term` (docs/MUT_GT_TERM_GOI10.md). Mỗi test khẳng định KẾT QUẢ suy từ hợp đồng D12 (đệ quy
//! không có `↓`/`div` phải bị ĐỎ; không bỏ cạnh vì thiếu thông tin), không chỉ chạm dòng. Các chốt toàn cục (`set_*_for_test`) nằm trong MỘT test
//! để không đua với nhau (cùng tiến trình).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::{check_termination, conservative_fallbacks, set_graph_limit_for_test};

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

fn rejected(src: &str) {
    let c = codes(src);
    assert!(
        c.iter().any(|x| x == "E0703" || x == "E0702"),
        "đệ quy không dừng được CHẤP NHẬN: {c:?}\n{src}"
    );
}

fn accepted(src: &str) {
    let c = codes(src);
    assert!(c.is_empty(), "kỳ vọng sạch, có {c:?}\n{src}");
}

/// Cận số nút ĐÚNG TỪNG NÚT (`>=`, dòng 309): số nút N của chương trình chỉ-hàm-không-generic = số thân hàm không tham số kiểu. Cận = N: không
/// lùi (đủ chỗ); cận = N-1: PHẢI lùi về đồ thị bảo thủ (bộ đếm `conservative_fallbacks` tăng).
#[test]
fn graph_limit_is_exact_to_the_node() {
    let src = "ƒ a() ! ∅ { }\nƒ b() ! ∅ { a(); }\nƒ main() ! ∅ { b(); }\n";
    let (t, m) = common::mir(src);
    let info = build_prog_info(&t, &m);
    let n = m
        .bodies
        .iter()
        .filter(|b| b.kind == gt_mir::BodyKind::Fn && t.db.fn_params(b.node).is_empty())
        .count();
    assert!(n >= 3, "n = {n}");
    let run = || {
        let before = conservative_fallbacks();
        let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
        assert!(o.report.diagnostics.is_empty(), "{:?}", o.report.diagnostics);
        conservative_fallbacks() - before
    };
    set_graph_limit_for_test(n);
    let at_limit = run();
    set_graph_limit_for_test(n - 1);
    let below = run();
    set_graph_limit_for_test(0);
    assert_eq!(at_limit, 0, "cận = N không được làm lùi bảo thủ");
    assert!(below >= 1, "cận = N-1 phải làm lùi về bảo thủ");
}

/// Đệ quy chỉ đi qua MỘT loại cạnh phải bị ĐỎ: không loại cạnh nào được rơi mất khi dựng đồ thị instance.
#[test]
fn recursion_through_each_edge_kind_is_rejected() {
    // cạnh trực tiếp qua instance generic (thân generic gọi lại chính nó)
    rejected("ƒ g<T: Copy>(x: T, n: u32) → u32 ! ∅ { g(x, n) }\nƒ main() → u32 ! ∅ { g(1u32, 2u32) }\n");
    // hai instance khác nhau của cùng một hàm generic trong một vòng (g<u32> → g<u32> qua h)
    rejected("ƒ h<T: Copy>(x: T, n: u32) → u32 ! ∅ { g(x, n) }\nƒ g<T: Copy>(x: T, n: u32) → u32 ! ∅ { h(x, n) }\nƒ main() → u32 ! ∅ { g(1u32, 2u32) }\n");
    // phương thức trait (cạnh gián tiếp)
    rejected("⊏ T { ƒ t(&self, n: u32) → u32 ! ∅; }\n▢ S { }\n⊐ T for S { ƒ t(&self, n: u32) → u32 ! ∅ { x ≔ S { }; x.t(n) } }\nƒ main() → u32 ! ∅ { x ≔ S { }; x.t(1u32) }\n");
    // hàm dùng làm giá trị rồi gọi (cạnh gián tiếp tới hàm bị thoát)
    rejected("ƒ f(n: u32) → u32 ! ∅ { μg ≔ f; g(n) }\nƒ main() → u32 ! ∅ { f(1u32) }\n");
    // tham số hàm: hàm gọi tham số, main đưa chính nó
    rejected("ƒ app(h: ƒ(u32) → u32 ! ∅, n: u32) → u32 ! ∅ { h(n) }\nƒ f(n: u32) → u32 ! ∅ { app(f, n) }\nƒ main() → u32 ! ∅ { f(1u32) }\n");
    // đệ quy qua `spawn` với worker là chính hàm gọi (đối số FnItem của callee không thân)
    rejected("ƒ f(n: u64) → () ! ∅ { spawn(f, n); }\nƒ main() → i32 ! ∅ { f(1u64); 0 }\n");
}

/// Đối chứng (không đỏ oan): hàm không đệ quy dùng làm giá trị/đối số/trait vẫn được chấp nhận.
#[test]
fn non_recursive_uses_of_each_edge_kind_stay_accepted() {
    accepted("ƒ g<T: Copy>(x: T) → T ! ∅ { x }\nƒ main() → u32 ! ∅ { g(1u32) }\n");
    accepted("⊏ T { ƒ t(&self) → u32 ! ∅; }\n▢ S { }\n⊐ T for S { ƒ t(&self) → u32 ! ∅ { 1u32 } }\nƒ main() → u32 ! ∅ { x ≔ S { }; x.t() }\n");
    accepted("ƒ f(n: u32) → u32 ! ∅ { n }\nƒ main() → u32 ! ∅ { μg ≔ f; g(1u32) }\n");
    accepted("ƒ w(n: u64) → () ! ∅ { }\nƒ main() → i32 ! ∅ { spawn(w, 1u64); 0 }\n");
}
