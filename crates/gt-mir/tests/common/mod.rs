#![allow(dead_code)]
//! Dùng chung cho test gt-mir: fixture prelude của gt-types (một nguồn duy nhất) + hạ MIR + in.
#[path = "../../../gt-types/tests/common/mod.rs"]
mod tc;
pub use tc::*;

use gt_mir::{MirOutput, lower_program, print::PrintCx, print::print_body};
use gt_types::TypesOutput;

/// Kiểm kiểu sạch rồi hạ MIR.
pub fn mir(src: &str) -> (TypesOutput, MirOutput) {
    let t = ok(src);
    let m = lower_program(&t);
    (t, m)
}

pub fn mir_files(files: &[(&str, &str)]) -> (TypesOutput, MirOutput) {
    let t = run(files);
    assert!(t.report.diagnostics.is_empty(), "{:?}", show(&t));
    let m = lower_program(&t);
    (t, m)
}

pub fn mir_codes(m: &MirOutput) -> Vec<String> {
    m.report
        .diagnostics
        .iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

/// Hạ MIR phải sạch (không ICE, không E0307); trả văn bản MIR của hàm `key`.
pub fn mir_ok(src: &str, key: &str) -> String {
    let (t, m) = mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        mir_codes(&m)
    );
    text(&t, &m, key)
}

pub fn text(t: &TypesOutput, m: &MirOutput, key: &str) -> String {
    let cx = PrintCx {
        defs: t.defs(),
        db: &t.db,
        keys: &m.keys,
    };
    let b = m.body(key).unwrap_or_else(|| {
        panic!(
            "không có thân {key}: {:?}",
            m.bodies.iter().map(|b| &b.key).collect::<Vec<_>>()
        )
    });
    print_body(&cx, b)
}
