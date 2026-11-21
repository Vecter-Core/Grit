//! GÓI 14 D1 — giết thêm đột biến gt-term còn sống (xem docs/MUT_GT_TERM_GOI10.md "GÓI 14 D1"). Mỗi test XANH trên mã gốc.
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

/// Hàm đệ quy KHÔNG ĐƯỢC GỌI từ đâu (cũng không là giá trị): chỉ gốc khởi tạo `fn_params().is_empty()` của đồ thị instance thấy được nó ⇒ phán quyết dừng
/// PHỤ THUỘC vào việc gieo gốc cho MỌI hàm không generic. Đột biến `if fn_params(..).is_empty()` → `false` làm mất cạnh tự gọi ⇒ chấp nhận vòng vô hạn.
#[test]
fn an_uncalled_non_generic_recursive_fn_is_still_rejected() {
    let c = codes("ƒ f(n: u32) → u32 ! ∅ { f(n) }\nƒ main() → i32 ! ∅ { 0 }\n");
    assert!(c.contains(&"E0703".to_string()), "{c:?}");
}

#[test]
fn an_uncalled_mutual_recursion_is_still_rejected() {
    let c = codes(
        "ƒ a(n: u32) → u32 ! ∅ { b(n) }\nƒ b(n: u32) → u32 ! ∅ { a(n) }\nƒ main() → i32 ! ∅ { 0 }\n",
    );
    assert!(c.contains(&"E0703".to_string()), "{c:?}");
}

#[test]
fn an_uncalled_acyclic_fn_is_accepted() {
    let c = codes("ƒ f(n: u32) → u32 ! ∅ { n }\nƒ main() → i32 ! ∅ { 0 }\n");
    assert!(c.is_empty(), "{c:?}");
}

/// Hàm generic đệ quy QUA giá trị hàm: `g` dùng chính nó làm giá trị (`h ≔ g`) hoặc truyền nó làm đối số hàm (`ap(g, …)`). Việc phát hiện instance của hàm-giá-trị
/// (`ops.push(o)` cho `Use`/`Cast`/`Repeat`, `ops.extend(cargs)` cho lời gọi) phải còn; thiếu ⇒ cạnh vòng bị mất. (Tắt hai dòng đó ở instance_graph.)
#[test]
fn generic_recursion_through_a_fn_value_bound_to_a_local_is_rejected() {
    let c =
        codes("ƒ g<T>(x: T) → u32 ! ∅ { h ≔ g; h(x) }\nƒ main() → i32 ! ∅ { r ≔ g(1u32); 0 }\n");
    assert!(c.contains(&"E0703".to_string()), "{c:?}");
}

#[test]
fn generic_recursion_through_a_fn_value_passed_as_argument_is_rejected() {
    let c = codes(
        "ƒ ap(f: ƒ(u32) → u32 ! ∅, n: u32) → u32 ! ∅ { f(n) }\nƒ g<T>(x: T) → u32 ! ∅ { ap(g, 1u32) }\nƒ main() → i32 ! ∅ { r ≔ g(1u32); 0 }\n",
    );
    assert!(c.contains(&"E0703".to_string()), "{c:?}");
}

/// Instance của hàm generic CHỈ xuất hiện như GIÁ TRỊ (`h ≔ k`, hay `ap(k, …)`): vòng tự gọi của `k` chỉ thấy được nếu instance đó được phát hiện qua toán hạng hằng hàm.
#[test]
fn a_generic_fn_instantiated_only_as_a_value_keeps_its_self_recursion_visible() {
    let c =
        codes("ƒ k<T>(x: T) → u32 ! ∅ { k(x) }\nƒ main() → i32 ! ∅ { h ≔ k; r ≔ h(1u32); 0 }\n");
    assert!(c.contains(&"E0703".to_string()), "{c:?}");
}

#[test]
fn a_generic_fn_instantiated_only_as_a_call_argument_keeps_its_self_recursion_visible() {
    let c = codes(
        "ƒ ap(f: ƒ(u32) → u32 ! ∅, n: u32) → u32 ! ∅ { f(n) }\nƒ k<T>(x: T) → u32 ! ∅ { k(x) }\nƒ main() → i32 ! ∅ { r ≔ ap(k, 1u32); 0 }\n",
    );
    assert!(c.contains(&"E0703".to_string()), "{c:?}");
}
