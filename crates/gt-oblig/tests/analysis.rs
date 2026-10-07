//! Phân tích trên chương trình thật (ADR-025 §3–§5): nghĩa vụ nào xoá được, nào giữ, nào bị bác.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::analysis::{Status, analyze_body, summarize};
use gt_oblig::prover::ZoneProver;

fn report(src: &str, key: &str) -> gt_oblig::analysis::BodyReport {
    let (_t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}",
        common::mir_codes(&m)
    );
    let b = m.body(key).unwrap_or_else(|| panic!("no body {key}"));
    analyze_body(b, &m.keys, &mut ZoneProver::new(24))
}

fn show(r: &gt_oblig::analysis::BodyReport) -> String {
    r.judged
        .iter()
        .map(|j| format!("{} {} {:?}", j.ob.id, j.ob.kind.name(), j.status))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn sum_loop_index_is_discharged_but_add_is_kept() {
    let r = report(
        "ƒ sum(xs: &[i64]) → i64 ! ∅ {\n μs ≔ 0;\n μi ≔ 0;\n ⟳ i < xs.len() ↓ xs.len() - i {\n s ← s + xs[i];\n i ← i + 1;\n }\n s\n}\n",
        "main::sum",
    );
    assert!(r.ice.is_none(), "{:?}", r.ice);
    let (d, l, f, x) = summarize(&r);
    eprintln!("{}", show(&r));
    assert_eq!((f, x), (0, 0));
    assert!(d >= 2, "index và i+1 phải xoá được\n{}", show(&r));
    assert_eq!(l, 1, "s + xs[i] phải giữ\n{}", show(&r));
    assert!(
        r.judged
            .iter()
            .any(|j| matches!(j.status, Status::Lowered { note: "N0601" }))
    );
}

#[test]
fn dump() {
    let src = "ƒ sum(xs: &[i64]) → i64 ! ∅ {\n μs ≔ 0;\n μi ≔ 0;\n ⟳ i < xs.len() ↓ xs.len() - i {\n s ← s + xs[i];\n i ← i + 1;\n }\n s\n}\n";
    let (t, m) = common::mir(src);
    eprintln!("{}", common::text(&t, &m, "main::sum"));
    let r = report(src, "main::sum");
    for j in &r.judged {
        eprintln!(
            "{}\n goal {}\n hyps {:?}",
            j.ob.id,
            j.ob.goal.to_text(),
            j.ob.hyps.iter().map(|h| h.to_text()).collect::<Vec<_>>()
        );
    }
}
