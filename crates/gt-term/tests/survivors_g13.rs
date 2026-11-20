//! GÓI 13 B1 — test giết đột biến sống sót của `gt-term` (chặt thêm: vòng KHÔNG có cạnh lùi lặp tối đa một lần ⇒ không cần `↓`/`div`).
//! Mã gốc PHẢI xanh; đột biến `has_back_edge` ép "có cạnh lùi" PHẢI đỏ (đòi `↓` oan).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::check_termination;

fn codes(src: &str) -> Vec<String> {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}
{src}",
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

#[test]
fn a_loop_whose_body_always_breaks_needs_no_variant() {
    for body in [
        "ƒ f() → u32 ! ∅ { μi ≔ 0u32; ⟳ i < 5u32 { ⊠; } i }
ƒ main() → i32 ! ∅ { r ≔ f(); 0 }
",
        "ƒ f() → u32 ! ∅ { ⟳ { ⊠; } 1u32 }
ƒ main() → i32 ! ∅ { r ≔ f(); 0 }
",
        "ƒ f(n: u32) → u32 ! ∅ { μi ≔ n; ⟳ i > 0u32 { ⊠; } i }
ƒ main() → i32 ! ∅ { r ≔ f(3u32); 0 }
",
    ] {
        let c = codes(body);
        assert!(
            c.is_empty(),
            "{c:?}
{body}"
        );
    }
}

/// Đối chứng: vòng CÓ cạnh lùi mà thiếu `↓` vẫn đỏ (E0701).
#[test]
fn a_loop_with_a_back_edge_and_no_variant_is_still_rejected() {
    let c = codes(
        "ƒ f() → u32 ! ∅ { μi ≔ 0u32; ⟳ i < 5u32 { ⎇ i == 3u32 { ⊠; } } i }
ƒ main() → i32 ! ∅ { r ≔ f(); 0 }
",
    );
    assert!(c.iter().any(|x| x == "E0701"), "{c:?}");
}
