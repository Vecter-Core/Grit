//! CHÈN LỖI (lệnh Giang 2026-10-15, B.2/B.3): nếu nghĩa vụ biến thể của M7 bị MẤT/cắt (mô phỏng bằng `set_drop_variant_obligations_for_test`), gt-term KHÔNG được coi
//! tập nghĩa vụ rỗng là "đã chứng minh dừng". Trước khi sửa, gt-term chấp nhận vòng lặp đứng yên trong tình huống này (`all`/`is_empty` trên tập rỗng).
//! Chạy trong TIẾN TRÌNH RIÊNG (cờ chèn lỗi là toàn cục) nên chỉ có MỘT test chứa mọi ca, tuần tự.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::analysis::set_drop_variant_obligations_for_test;
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

#[test]
fn lost_obligations_are_never_read_as_a_termination_proof() {
    let stationary = "ƒ f(n: u32) → () ! ∅ { μi ≔ 0u32; ⟳ i < n ↓ n - i { } }";
    let no_progress = "ƒ f(n: u32) → () ! ∅ { μi ≔ 0u32; ⟳ i < n { } }";
    let valid_explicit = "ƒ f(n: u32) → () ! ∅ { μi ≔ 0u32; ⟳ i < n ↓ n - i { i ← i + 1; } }";
    let valid_inferred = "ƒ f(n: u32) → () ! ∅ { μi ≔ 0u32; ⟳ i < n { i ← i + 1; } }";
    // đối chứng (bình thường): sai bị bác, đúng được chấp nhận
    set_drop_variant_obligations_for_test(false);
    assert_eq!(codes(stationary), vec!["E0702"]);
    assert_eq!(codes(no_progress), vec!["E0701"]);
    assert!(codes(valid_explicit).is_empty());
    assert!(codes(valid_inferred).is_empty());
    // MẤT nghĩa vụ: KHÔNG được chấp nhận nữa — cả vòng sai VÀ vòng đúng đều bị từ chối (không có bằng chứng)
    set_drop_variant_obligations_for_test(true);
    assert_eq!(
        codes(stationary),
        vec!["E0702"],
        "vòng đứng yên bị CHẤP NHẬN khi nghĩa vụ bị mất ⇒ fail-open"
    );
    assert_eq!(
        codes(no_progress),
        vec!["E0701"],
        "vòng không tiến bị CHẤP NHẬN khi nghĩa vụ bị mất ⇒ fail-open"
    );
    assert_eq!(codes(valid_explicit), vec!["E0702"]);
    assert_eq!(codes(valid_inferred), vec!["E0701"]);
    // vòng MỘT KHỐI (đầu vòng tự quay về chính nó) đứng yên: khi nghĩa vụ bị mất vẫn phải bị từ chối (tiêu chí cạnh lùi phải tính cả cạnh tự quay)
    let single_block = "ƒ f(n: u32) → () ! {div} { ⟳ ↓ n { } }";
    set_drop_variant_obligations_for_test(false);
    assert_eq!(codes(single_block), vec!["E0702"]);
    set_drop_variant_obligations_for_test(true);
    assert_eq!(
        codes(single_block),
        vec!["E0702"],
        "vòng một khối đứng yên bị CHẤP NHẬN khi nghĩa vụ bị mất"
    );
    // vòng lặp mà M7 KHẲNG ĐỊNH không có cạnh lùi khả thi (thân không bao giờ vào được) VẪN được chấp nhận dù nghĩa vụ bị mất (không phụ thuộc nghĩa vụ)
    let dead_body = "ƒ f() → () ! ∅ { μi ≔ 19u8; ⟳ i <= 16u8 ∧ i < 12u8 ↓ i { i ← i + 1; } }";
    assert!(codes(dead_body).is_empty());
    set_drop_variant_obligations_for_test(false);
    assert!(codes(dead_body).is_empty());
}
