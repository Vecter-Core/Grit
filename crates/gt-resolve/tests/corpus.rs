//! Chạy resolver THẬT trên corpus `tests/`: mọi mẫu `pass` phải phân giải sạch; mẫu `fail` E02xx phải ra ĐÚNG mã đó; mẫu `fail`/`warn`
//! của milestone sau KHÔNG được bị M3 báo thay (chúng là chương trình tên-đúng); ca nhiều tệp (ADR-017b) theo tệp `expect`.
mod common;
use common::*;
use std::path::{Path, PathBuf};

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests")
}

fn gt_files(sub: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(tests_dir().join(sub))
        .expect("thư mục")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .collect();
    v.sort();
    v
}

fn case_dirs(sub: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(tests_dir().join(sub))
        .expect("thư mục")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.is_dir())
        .collect();
    v.sort();
    v
}

fn name(p: &Path) -> String {
    p.file_name().expect("tên").to_string_lossy().into_owned()
}

fn show(out: &gt_resolve::ResolveOutput) -> Vec<String> {
    out.report
        .diagnostics
        .iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

#[test]
fn every_pass_file_resolves_cleanly() {
    let list = gt_files("pass");
    assert!(list.len() >= 30);
    let mut bad = vec![];
    for p in &list {
        let src = std::fs::read_to_string(p).expect("đọc");
        let out = one(&src);
        if !out.report.diagnostics.is_empty() {
            bad.push(format!("{}: {:?}", name(p), show(&out)));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

const OWNED: &[&str] = &["E0201", "E0202", "E0203", "E0204", "E0205"];

#[test]
fn fail_files_of_this_milestone_give_exactly_their_code_and_others_are_not_reported_here() {
    let mut owned = 0;
    let mut bad = vec![];
    for p in gt_files("fail") {
        let n = name(&p);
        let src = std::fs::read_to_string(&p).expect("đọc");
        if !gt_parser::parse("t.gt", &src).report.diagnostics.is_empty() {
            continue; // lỗi cú pháp: thuộc parser
        }
        let out = one(&src);
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
                // mẫu của milestone sau: phải là chương trình tên-đúng
                let m3: Vec<_> = got.iter().filter(|g| OWNED.contains(&g.as_str())).collect();
                if !m3.is_empty() {
                    bad.push(format!("{n}: M3 báo thay milestone sau: {:?}", show(&out)));
                }
            }
        }
    }
    assert!(owned >= 4, "chỉ có {owned} mẫu fail của M3");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn warn_files_are_not_blocked_by_name_resolution() {
    for p in gt_files("warn") {
        let src = std::fs::read_to_string(&p).expect("đọc");
        if !gt_parser::parse("t.gt", &src).report.diagnostics.is_empty() {
            continue;
        }
        let out = one(&src);
        assert!(
            out.report.diagnostics.is_empty(),
            "{}: {:?}",
            name(&p),
            show(&out)
        );
    }
}

/// Ca nhiều tệp: thư mục chứa `*.gt` (tên tệp = tên module) và tệp `expect` một dòng.
#[test]
fn multi_file_cases_follow_their_expect_line() {
    let mut count = 0;
    let mut bad = vec![];
    for sub in ["pass", "fail"] {
        for dir in case_dirs(sub) {
            let exp = std::fs::read_to_string(dir.join("expect")).expect("expect");
            let line = exp
                .lines()
                .find(|l| l.starts_with("expect:"))
                .expect("dòng expect")
                .trim()
                .to_string();
            let mut files: Vec<(String, String)> = std::fs::read_dir(&dir)
                .expect("dir")
                .map(|e| e.expect("entry").path())
                .filter(|p| p.extension().is_some_and(|x| x == "gt"))
                .map(|p| {
                    (
                        name(&p),
                        std::fs::read_to_string(&p)
                            .expect("đọc")
                            .replace("\r\n", "\n"),
                    )
                })
                .collect();
            files.sort();
            let refs: Vec<(&str, &str)> = files
                .iter()
                .map(|(n, t)| (n.as_str(), t.as_str()))
                .collect();
            let out = run(&refs);
            let got: Vec<String> = out
                .report
                .diagnostics
                .iter()
                .map(|d| d.code.as_str().to_string())
                .collect();
            count += 1;
            if line == "expect: pass" {
                if !got.is_empty() {
                    bad.push(format!("{}: cần sạch, ra {:?}", name(&dir), show(&out)));
                }
            } else {
                let code = line.trim_start_matches("expect: fail ").to_string();
                if got.is_empty() || got.iter().any(|g| *g != code) {
                    bad.push(format!(
                        "{}: cần chỉ {code}, ra {:?}",
                        name(&dir),
                        show(&out)
                    ));
                }
            }
        }
    }
    assert!(count >= 8, "chỉ có {count} ca nhiều tệp");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
