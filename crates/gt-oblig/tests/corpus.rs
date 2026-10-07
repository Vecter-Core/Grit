//! Cổng M7: mọi mẫu `pass` KHÔNG có lỗi từ nghĩa vụ (chỉ được có GHI CHÚ); mẫu `fail` thuộc M7 ra ĐÚNG mã sở hữu.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use common::*;
use gt_oblig::driver::analyze_program;
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

/// `None` nếu không qua M4/M5 (không thuộc M7).
fn run_all(p: &Path) -> Option<(gt_oblig::driver::OblOutput, gt_types::TypesOutput)> {
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
    let out = analyze_program(&t, &m, &mut ZoneProver::new(24));
    Some((out, t))
}

fn codes(o: &gt_oblig::driver::OblOutput) -> Vec<String> {
    o.report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect()
}

#[test]
fn every_pass_case_has_no_obligation_errors() {
    let (mut n, mut obs, mut disch, mut kept) = (0, 0, 0, 0);
    let mut bad = vec![];
    for p in entries("pass") {
        if p.extension().is_some_and(|x| x != "gt") {
            continue;
        }
        n += 1;
        let (o, _t) =
            run_all(&p).unwrap_or_else(|| panic!("{}: mẫu pass phải qua M4/M5", name(&p)));
        let errs: Vec<String> = codes(&o)
            .into_iter()
            .filter(|c| c.starts_with('E'))
            .collect();
        if !errs.is_empty() {
            bad.push(format!("{}: {:?}", name(&p), errs));
        }
        obs += o.stats.obligations;
        disch += o.stats.discharged;
        kept += o.stats.lowered;
    }
    eprintln!("pass: {n} ca, {obs} nghĩa vụ, {disch} xoá, {kept} giữ");
    assert!(n >= 70, "chỉ {n} ca pass");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

// E0901/E0902 thuộc bước hợp đồng (v)
const OWNED: &[&str] = &["E0601", "E0602", "E0603", "E0604"];

#[test]
fn fail_cases_of_m7_give_exactly_their_code() {
    let mut owned = 0;
    for p in entries("fail") {
        let n = name(&p);
        let Some(code) = OWNED.iter().find(|c| n.starts_with(&format!("{c}_"))) else {
            continue;
        };
        owned += 1;
        let Some((o, _t)) = run_all(&p) else {
            panic!("{n}: phải qua M4/M5")
        };
        let errs: Vec<String> = codes(&o)
            .into_iter()
            .filter(|c| c.starts_with('E'))
            .collect();
        assert!(
            !errs.is_empty() && errs.iter().all(|c| c == code),
            "{n}: kỳ vọng chỉ {code}, có {errs:?}"
        );
    }
    assert!(owned >= 4, "chỉ {owned} mẫu thuộc M7");
}

#[test]
#[ignore]
fn dump_kept() {
    for p in entries("pass") {
        if p.extension().is_some_and(|x| x != "gt") {
            continue;
        }
        let Some((o, _)) = run_all(&p) else { continue };
        for r in &o.per_body {
            for j in &r.judged {
                if matches!(j.status, gt_oblig::analysis::Status::Lowered { .. }) {
                    eprintln!(
                        "{} {} {} goal={} hyps={:?}",
                        name(&p),
                        j.ob.id,
                        j.ob.kind.name(),
                        j.ob.goal.to_text(),
                        j.ob.hyps.iter().map(|h| h.to_text()).collect::<Vec<_>>()
                    );
                }
            }
        }
    }
}
