//! Cổng M8 (điều kiện dừng, hiệu ứng, thứ tự khoá, memory ordering): mọi mẫu `pass` KHÔNG có E07xx/E08xx/E0502/E0503; mẫu `fail` thuộc M8 ra ĐÚNG mã sở hữu.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use common::*;
use gt_oblig::prover::ZoneProver;
use std::path::{Path, PathBuf};

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests")
}

fn entries(sub: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(tests_dir().join(sub))
        .expect("thư mục")
        .map(|e| e.expect("entry").path())
        .collect();
    v.sort();
    v
}

fn name(p: &Path) -> String {
    p.file_name().expect("tên").to_string_lossy().into_owned()
}

fn files_of(p: &Path) -> Vec<(String, String)> {
    let rd = |q: &Path| {
        std::fs::read_to_string(q)
            .expect("đọc")
            .replace("\r\n", "\n")
    };
    if p.is_dir() {
        let mut v: Vec<(String, String)> = std::fs::read_dir(p)
            .expect("dir")
            .map(|e| e.expect("entry").path())
            .filter(|q| q.extension().is_some_and(|x| x == "gt"))
            .map(|q| (name(&q), rd(&q)))
            .collect();
        v.sort();
        v
    } else {
        vec![("main.gt".to_string(), rd(p))]
    }
}

// ---- M8: điều kiện dừng và hiệu ứng ----

const OWNED_M8: &[&str] = &[
    "E0701", "E0702", "E0703", "E0801", "E0802", "E0502", "E0503",
];

/// `None` nếu không qua M4/M5. Mã lỗi M8 (điều kiện dừng + hiệu ứng) của một mẫu.
fn m8_codes(p: &Path) -> Option<Vec<String>> {
    let files = files_of(p);
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let t = run(&refs);
    if !t.report.diagnostics.is_empty() {
        return None;
    }
    let m = gt_mir::lower_program(&t);
    if !m.report.diagnostics.is_empty() {
        return None;
    }
    let info = gt_oblig::driver::build_prog_info(&t, &m);
    let term = gt_term::check_termination(&t, &m, &info, &mut ZoneProver::new(24));
    let eff = gt_effects::check_effects(&t, &m);
    let mut orders = gt_diag::Diagnostics::new(200);
    gt_conc::check_orderings(&t, &m, &mut orders);
    let orders = orders.finish();
    let mut v: Vec<String> = term
        .report
        .diagnostics
        .iter()
        .chain(eff.report.diagnostics.iter())
        .chain(orders.diagnostics.iter())
        .map(|d| d.code.as_str().to_string())
        .collect();
    v.sort();
    Some(v)
}

#[test]
fn fail_cases_of_m8_give_exactly_their_code() {
    let mut owned = 0;
    for p in entries("fail") {
        let n = name(&p);
        let Some(code) = OWNED_M8.iter().find(|c| n.starts_with(&format!("{c}_"))) else {
            continue;
        };
        owned += 1;
        let Some(codes) = m8_codes(&p) else {
            panic!("{n}: phải qua M4/M5")
        };
        assert!(
            !codes.is_empty() && codes.iter().all(|c| c == code),
            "{n}: kỳ vọng chỉ {code}, có {codes:?}"
        );
    }
    assert!(owned >= 15, "chỉ {owned} mẫu thuộc M8");
}

#[test]
fn every_pass_case_has_no_termination_or_effect_errors() {
    let mut n = 0;
    let mut bad = vec![];
    for p in entries("pass") {
        if p.extension().is_some_and(|x| x != "gt") {
            continue;
        }
        n += 1;
        let codes = m8_codes(&p).unwrap_or_else(|| panic!("{}: mẫu pass phải qua M4/M5", name(&p)));
        if !codes.is_empty() {
            bad.push(format!("{}: {:?}", name(&p), codes));
        }
    }
    assert!(n >= 70, "chỉ {n} ca pass");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
