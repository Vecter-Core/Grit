//! Chỉ tiêu 8.B (ADR-P2-04): tỉ lệ chứng minh của bộ giải Phase 1 (zone) mà kernel XÁC NHẬN độc lập bằng chứng chỉ Farkas, đo trên TOÀN BỘ mẫu
//! `tests/pass` một tệp (M7, `--checks=all`). Kernel không được từ chối chứng chỉ nào của bộ tìm (lỗi bộ tìm), và tỉ lệ không được tụt dưới mức đã đo.
use gt_kernel::CertifiedProver;
use gt_oblig::driver::{Checks, analyze_program_with};
use gt_oblig::prover::ZoneProver;
use gt_resolve::{Prelude, SourceInput};
use std::path::Path;

#[test]
fn kernel_confirms_a_measured_share_of_phase1_proofs_on_the_corpus() {
    let prelude = Prelude::from_source(&gt_driver::pipeline::prelude_text()).expect("prelude");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/pass");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("đọc corpus")
        .map(|e| e.expect("e").path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .collect();
    files.sort();
    let mut prover = CertifiedProver::new(ZoneProver::new(24));
    prover.measure = true;
    let mut programs = 0;
    for f in files {
        let text = std::fs::read_to_string(&f)
            .expect("đọc")
            .replace("\r\n", "\n");
        let inputs = [SourceInput {
            name: "main.gt".into(),
            text,
        }];
        let Ok(t) = gt_types::check_program(&inputs, &prelude) else {
            continue;
        };
        if t.report.errors() > 0 {
            continue;
        }
        let mut m = gt_mir::lower_program(&t);
        if m.report.errors() > 0 {
            continue;
        }
        let _ = gt_dataflow::elaborate_program(&t, &mut m);
        let before = prover.stats;
        let _ = analyze_program_with(&t, &m, &mut prover, Checks::All);
        let after = prover.stats;
        if after.inner_proved - before.inner_proved
            != after.inner_kernel_confirmed - before.inner_kernel_confirmed
        {
            eprintln!("CHƯA XÁC NHẬN: {}", f.display());
        }
        programs += 1;
    }
    let s = prover.stats;
    let pct = if s.inner_proved == 0 {
        0.0
    } else {
        100.0 * s.inner_kernel_confirmed as f64 / s.inner_proved as f64
    };
    eprintln!(
        "chương trình {programs}; zone chứng minh {}; kernel xác nhận {} ({pct:.1}%); chỉ Farkas {}; bị kernel từ chối {}",
        s.inner_proved, s.inner_kernel_confirmed, s.farkas_only, s.rejected
    );
    assert!(programs >= 400, "chỉ {programs} chương trình được đo");
    assert_eq!(s.rejected, 0, "bộ tìm đưa chứng chỉ mà kernel từ chối");
    assert!(s.inner_proved > 0);
    // đo 2026-11-23: 105/105 (100%) — gritc chạy chế độ CHỈ TIN KERNEL nên tụt dưới 100% trên corpus là hồi quy (chứng minh bị mất)
    assert_eq!(
        s.inner_kernel_confirmed, s.inner_proved,
        "kernel không xác nhận được mọi chứng minh của zone trên corpus"
    );
}
