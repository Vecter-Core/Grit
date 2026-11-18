//! GÓI 6 S2/S3: hàm có hơn 24 biến vòng lặp vượt giới hạn của bộ chứng minh (GHOST_MAX_VARS = 24); vòng đúng bị từ chối oan bằng E0702 (fail-closed). Chẩn đoán PHẢI nói rõ nguyên nhân
//! và gợi ý tách hàm (trước đây chỉ nói "biến thể không giảm", dễ hiểu nhầm là lỗi của chương trình). Vòng sai trong hàm NHỎ không được kèm lời nhắc này (không gây nhiễu).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::check_termination;

fn messages(src: &str) -> Vec<(String, String)> {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        common::mir_codes(&m)
    );
    let info = build_prog_info(&t, &m);
    let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
    o.report
        .diagnostics
        .iter()
        .map(|d| (d.code.as_str().to_string(), d.message.clone()))
        .collect()
}

/// hàm với `n` vòng đúng, mỗi vòng biến riêng, có `↓` tường minh
fn loops(n: usize) -> String {
    let mut body = String::new();
    for k in 1..=n {
        body.push_str(&format!(
            "  μi{k} ≔ 0u32; ⟳ i{k} < 3 ↓ 3 - i{k} {{ i{k} ← i{k} + 1; }}\n"
        ));
    }
    format!("ƒ f() → () ! ∅ {{\n{body}}}\n")
}

#[test]
fn up_to_the_limit_is_accepted() {
    assert!(messages(&loops(24)).is_empty());
}

#[test]
fn past_the_limit_is_rejected_with_a_message_that_names_the_cause() {
    let m = messages(&loops(26));
    assert!(
        !m.is_empty(),
        "26 vòng đúng bị từ chối oan (fail-closed) — nếu đã nâng giới hạn, cập nhật test và docs/LIMITS_AUDIT.md"
    );
    for (code, msg) in &m {
        assert_eq!(code, "E0702");
        assert!(
            msg.contains("tách") && msg.contains("24"),
            "thiếu lời nhắc giới hạn/gợi ý tách hàm: {msg}"
        );
    }
}

#[test]
fn a_genuinely_bad_loop_in_a_small_function_gets_no_limit_hint() {
    let m = messages("ƒ f() → () ! ∅ { μi ≔ 0u32; ⟳ i < 3 ↓ 3 - i { } }\n");
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].0, "E0702");
    assert!(
        !m[0].1.contains("tách"),
        "lời nhắc giới hạn không được xuất hiện ở hàm nhỏ: {}",
        m[0].1
    );
}
