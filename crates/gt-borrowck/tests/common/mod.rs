#![allow(dead_code)]
//! Dùng chung cho test gt-borrowck: fixture prelude của gt-types (một nguồn duy nhất) + hạ MIR + borrowck.
#[path = "../../../gt-types/tests/common/mod.rs"]
mod tc;
pub use tc::*;

use gt_diag::Report;
use gt_types::TypesOutput;

pub struct Run {
    pub t: TypesOutput,
    pub m: gt_mir::MirOutput,
    pub bc: Report,
}

pub fn pipeline(files: &[(&str, &str)]) -> Run {
    let t = run(files);
    assert!(t.report.diagnostics.is_empty(), "M4 báo: {:?}", show(&t));
    let mut m = gt_mir::lower_program(&t);
    assert!(
        m.report.diagnostics.is_empty(),
        "M5 báo: {:?}",
        m.report
            .diagnostics
            .iter()
            .map(|d| format!("{} {}", d.code, d.message))
            .collect::<Vec<_>>()
    );
    // như pipeline của gritc: elaboration drop (ADR-C18) giữa M5 và M6; chẩn đoán của nó (E0407 ở hợp đồng) tính là của sở hữu
    let e = gt_dataflow::elaborate_program(&t, &mut m);
    let mut bc = gt_borrowck::check_program(&t, &m);
    bc.diagnostics.splice(0..0, e.diagnostics);
    Run { t, m, bc }
}

pub fn bc(src: &str) -> Run {
    pipeline(&[("main.gt", src)])
}

pub fn bc_codes(r: &Run) -> Vec<String> {
    r.bc.diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect()
}

pub fn bc_show(r: &Run) -> Vec<String> {
    r.bc.diagnostics
        .iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

/// Phải qua borrowck sạch.
pub fn good(src: &str) -> Run {
    let r = bc(src);
    assert!(
        r.bc.diagnostics.is_empty(),
        "đáng lẽ được chấp nhận: {:?}\n{src}",
        bc_show(&r)
    );
    r
}

/// Phải ra đúng `n` chẩn đoán, đều mang mã `code`.
pub fn bad(src: &str, code: &str, n: usize) -> Run {
    let r = bc(src);
    assert_eq!(
        bc_codes(&r),
        vec![code.to_string(); n],
        "{src}\n{:?}",
        bc_show(&r)
    );
    r
}
