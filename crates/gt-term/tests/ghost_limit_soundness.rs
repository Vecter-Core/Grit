//! GÓI 7 S2.a — FB-C nghiêm trọng (vòng KHÔNG dừng bị chấp nhận). Khi hàm có hơn `GHOST_MAX_VARS` (24) biến biến-thể vòng lặp, vòng
//! sau không có biến bóng riêng; biến bóng CŨ của cùng biến (chụp ở vòng trước, còn nằm trong trạng thái) làm `y < bóng(y)` đúng nếu y đã giảm
//! trước vòng, nên một vòng không bao giờ dừng được chứng minh "dừng". Test này phải ĐỎ trước bản sửa: chương trình phải bị từ chối (E0702).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::check_termination;

fn codes(src: &str) -> Vec<String> {
    let (t, m) = common::mir(src);
    assert!(m.report.diagnostics.is_empty(), "{:?}\n{src}", common::mir_codes(&m));
    let info = build_prog_info(&t, &m);
    let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
    o.report.diagnostics.iter().map(|d| d.code.as_str().to_string()).collect()
}

/// `fillers` vòng lót (mỗi vòng một biến), vòng đầu dùng y; sau đó y giảm 1 và vòng cuối KHÔNG đổi gì (không dừng)
fn program(fillers: usize) -> String {
    let mut s = String::from("ƒ f(p: u32) → () ! ∅\n  ⊳ p >= 1\n{\n  μy ≔ p;\n  μk0 ≔ 0u32; ⟳ k0 < 5 ↓ y + 5 - k0 { k0 ← k0 + 1; }\n");
    for k in 1..=fillers {
        s.push_str(&format!("  μk{k} ≔ 0u32; ⟳ k{k} < 5 ↓ 5 - k{k} {{ k{k} ← k{k} + 1; }}\n"));
    }
    s.push_str("  y ← y - 1;\n  μz ≔ 0u32;\n  ⟳ z < 5 ↓ y { }\n}\nƒ main() → i32 ! ∅ { f(1); 0 }\n");
    s
}

#[test]
fn nonterminating_loop_after_ghost_budget_is_rejected() {
    // 22 vòng lót + vòng đầu (2 biến: y, k0) = 24 biến bóng đã dùng hết khi tới vòng cuối
    let c = codes(&program(22));
    assert!(c.iter().any(|x| x == "E0702"), "vòng không dừng bị chấp nhận (FB-C nghiêm trọng): {c:?}");
}

#[test]
fn same_shape_with_few_loops_is_rejected_too() {
    // đối chứng: dưới giới hạn, bộ chứng minh vốn đã từ chối (có biến bóng riêng cho vòng cuối)
    let c = codes(&program(3));
    assert!(c.iter().any(|x| x == "E0702"), "{c:?}");
}
