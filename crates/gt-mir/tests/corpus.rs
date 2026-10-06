//! Cổng M5: mọi mẫu `pass` hạ được MIR, qua validator, không E0307/ICE; mẫu `fail` E0307 ra ĐÚNG mã đó.
mod common;
use common::*;
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
    if p.is_dir() {
        let mut v: Vec<(String, String)> = std::fs::read_dir(p)
            .expect("dir")
            .map(|e| e.expect("entry").path())
            .filter(|q| q.extension().is_some_and(|x| x == "gt"))
            .map(|q| {
                (
                    name(&q),
                    std::fs::read_to_string(&q)
                        .expect("đọc")
                        .replace("\r\n", "\n"),
                )
            })
            .collect();
        v.sort();
        v
    } else {
        vec![(
            "main.gt".to_string(),
            std::fs::read_to_string(p)
                .expect("đọc")
                .replace("\r\n", "\n"),
        )]
    }
}

fn lower(p: &Path) -> (gt_types::TypesOutput, gt_mir::MirOutput) {
    let files = files_of(p);
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let t = run(&refs);
    let m = gt_mir::lower_program(&t);
    (t, m)
}

#[test]
fn every_pass_case_lowers_validates_and_is_complete() {
    let mut bad = vec![];
    let mut n = 0;
    let mut bodies = 0;
    for p in entries("pass") {
        if p.extension().is_some_and(|x| x != "gt") {
            continue;
        }
        n += 1;
        let (t, m) = lower(&p);
        assert!(
            t.report.diagnostics.is_empty(),
            "{}: {:?}",
            name(&p),
            show(&t)
        );
        if !m.report.diagnostics.is_empty() {
            bad.push(format!("{}: {:?}", name(&p), mir_codes(&m)));
        }
        assert_eq!(m.bodies.len(), t.bodies.len(), "{}", name(&p));
        bodies += m.bodies.len();
    }
    assert!(n >= 70, "chỉ {n} ca pass");
    assert!(bodies >= 100, "chỉ {bodies} thân");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn e0307_fail_cases_give_exactly_e0307() {
    let mut owned = 0;
    for p in entries("fail") {
        let n = name(&p);
        if !n.starts_with("E0307_") {
            continue;
        }
        owned += 1;
        let (t, m) = lower(&p);
        assert!(
            t.report.diagnostics.is_empty(),
            "{n}: M4 báo {:?}",
            show(&t)
        );
        let codes: Vec<String> = m
            .report
            .diagnostics
            .iter()
            .map(|d| d.code.as_str().to_string())
            .collect();
        assert!(
            !codes.is_empty() && codes.iter().all(|c| c == "E0307"),
            "{n}: {:?}",
            mir_codes(&m)
        );
    }
    assert!(owned >= 3, "chỉ {owned} mẫu E0307");
}

#[test]
fn fail_cases_of_later_milestones_lower_without_mir_errors() {
    // mẫu của M6+ là chương trình đúng kiểu: MIR phải dựng được (M6 đọc chính MIR này)
    for p in entries("fail") {
        let n = name(&p);
        if !(n.starts_with("E04")
            || n.starts_with("E06")
            || n.starts_with("E07")
            || n.starts_with("E08"))
        {
            continue;
        }
        let (t, m) = lower(&p);
        if !t.report.diagnostics.is_empty() {
            continue; // không qua M4 (vd dùng cú pháp chưa có): không thuộc M5
        }
        assert!(m.report.diagnostics.is_empty(), "{n}: {:?}", mir_codes(&m));
    }
}
