//! Cổng M4: mọi mẫu `pass` kiểm kiểu sạch; mẫu `fail` E03xx/E1002 ra ĐÚNG mã đó; mẫu của milestone sau không bị M4 báo thay.
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

fn run_case(p: &Path) -> gt_types::TypesOutput {
    let files = files_of(p);
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    run(&refs)
}

#[test]
fn every_pass_case_typechecks_cleanly() {
    let mut bad = vec![];
    let mut n = 0;
    for p in entries("pass") {
        if p.extension().is_some_and(|x| x != "gt") {
            continue;
        }
        n += 1;
        let out = run_case(&p);
        if !out.report.diagnostics.is_empty() {
            bad.push(format!("{}: {:?}", name(&p), show(&out)));
        }
    }
    assert!(n >= 37, "chỉ có {n} ca pass");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

const OWNED: &[&str] = &[
    "E0301", "E0302", "E0303", "E0304", "E0305", "E0308", "E0309", "E0311", "E0312", "E0313",
    "E0310", "E0314", "E0315", "E0316", "E0501", "E1002",
];

#[test]
fn fail_cases_of_this_milestone_give_exactly_their_code_and_others_are_not_reported_here() {
    let mut owned = 0;
    let mut bad = vec![];
    for p in entries("fail") {
        let n = name(&p);
        let single = p.extension().is_some_and(|x| x == "gt");
        if !single && !p.is_dir() {
            continue;
        }
        let src_ok = if single {
            gt_parser::parse("t.gt", &std::fs::read_to_string(&p).expect("đọc"))
                .report
                .diagnostics
                .is_empty()
        } else {
            true
        };
        if !src_ok {
            // lỗi cú pháp: thuộc parser — nhưng mẫu của M4 mà không parse được là mẫu hỏng, không được bỏ qua lặng lẽ
            assert!(
                !OWNED.iter().any(|c| n.starts_with(&format!("{c}_"))),
                "{n}: mẫu của M4 có lỗi cú pháp"
            );
            continue;
        }
        let out = run_case(&p);
        let got: Vec<String> = out
            .report
            .diagnostics
            .iter()
            .map(|d| d.code.as_str().to_string())
            .collect();
        match OWNED.iter().find(|c| n.starts_with(&format!("{c}_"))) {
            Some(code) => {
                owned += 1;
                if got.is_empty() || got.iter().any(|g| g != code) {
                    bad.push(format!("{n}: cần chỉ {code}, ra {:?}", show(&out)));
                }
            }
            None => {
                // mẫu của milestone khác: M4 không được báo mã thuộc M4 thay họ (trừ khi tên mẫu nói rõ)
                let m4: Vec<&String> = got.iter().filter(|g| OWNED.contains(&g.as_str())).collect();
                let resolver_or_parser =
                    n.starts_with("E01") || n.starts_with("E02") || n.starts_with("E12");
                if !m4.is_empty() && !resolver_or_parser {
                    bad.push(format!("{n}: M4 báo thay milestone khác: {:?}", show(&out)));
                }
            }
        }
    }
    assert!(owned >= 20, "chỉ có {owned} mẫu fail của M4");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn warn_cases_are_not_blocked_by_type_errors() {
    for p in entries("warn") {
        if p.extension().is_some_and(|x| x == "gt") {
            let out = run_case(&p);
            assert!(
                out.report.diagnostics.is_empty(),
                "{}: {:?}",
                name(&p),
                show(&out)
            );
        }
    }
}
