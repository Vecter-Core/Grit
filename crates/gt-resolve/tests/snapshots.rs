//! Snapshot test cho THÔNG BÁO LỖI của giải tên (E02xx): đầu ra người-đọc (không màu, nhiều tệp) và JSON của mọi chẩn đoán phải khớp từng
//! byte với `tests/snapshots_resolve/<ca>.snap`. Thiếu/khác → ĐỎ; cập nhật có chủ đích bằng `GRIT_UPDATE_SNAPSHOTS=1` rồi đọc `git diff`.
mod common;
use common::*;
use gt_diag::{RenderOptions, diagnostic_to_json, render_human_files};
use gt_span::SourceFile;
use std::path::{Path, PathBuf};

const OWNED: &[&str] = &["E0201", "E0202", "E0203", "E0204", "E0205"];

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests")
}

fn name(p: &Path) -> String {
    p.file_name().expect("tên").to_string_lossy().into_owned()
}

fn owned(n: &str) -> bool {
    OWNED.iter().any(|c| n.starts_with(&format!("{c}_")))
}

/// (tên ca, danh sách tệp (tên, nội dung)): tệp đơn dùng tên module `main.gt`.
fn cases() -> Vec<(String, Vec<(String, String)>)> {
    let mut v = Vec::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(tests_dir().join("fail"))
        .expect("fail")
        .map(|e| e.expect("entry").path())
        .collect();
    entries.sort();
    for p in entries {
        let n = name(&p);
        if p.is_dir() {
            if !owned(&n) {
                continue;
            }
            let mut files: Vec<(String, String)> = std::fs::read_dir(&p)
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
            files.sort();
            v.push((n, files));
        } else if p.extension().is_some_and(|x| x == "gt") && owned(&n) {
            let text = std::fs::read_to_string(&p)
                .expect("đọc")
                .replace("\r\n", "\n");
            v.push((
                n.trim_end_matches(".gt").to_string(),
                vec![("main.gt".to_string(), text)],
            ));
        }
    }
    v
}

fn snapshot_of(case: &str, files: &[(String, String)]) -> String {
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let out = run(&refs);
    let sources: Vec<SourceFile> = files
        .iter()
        .map(|(n, t)| SourceFile::new(n.clone(), t.clone()).expect("nhỏ"))
        .collect();
    let src_refs: Vec<&SourceFile> = sources.iter().collect();
    let mut s = format!("# {case}: {} chẩn đoán\n", out.report.diagnostics.len());
    for (i, d) in out.report.diagnostics.iter().enumerate() {
        s.push_str(&format!("\n## [{}] human\n", i + 1));
        s.push_str(&render_human_files(
            d,
            &src_refs,
            &RenderOptions { color: false },
        ));
        s.push_str(&format!("## [{}] json\n", i + 1));
        s.push_str(&diagnostic_to_json(d, &src_refs));
        s.push('\n');
    }
    s
}

fn snap_path(case: &str) -> PathBuf {
    tests_dir()
        .join("snapshots_resolve")
        .join(format!("{case}.snap"))
}

#[test]
fn resolver_error_messages_match_their_snapshots() {
    let update = std::env::var("GRIT_UPDATE_SNAPSHOTS").is_ok_and(|v| v == "1");
    let list = cases();
    assert!(list.len() >= 20, "chỉ có {} ca E02xx", list.len());
    let mut bad = Vec::new();
    for (case, files) in &list {
        let actual = snapshot_of(case, files);
        let sp = snap_path(case);
        if update {
            std::fs::create_dir_all(sp.parent().expect("cha")).expect("tạo thư mục");
            std::fs::write(&sp, &actual).expect("ghi");
            continue;
        }
        match std::fs::read_to_string(&sp) {
            Ok(want) if want.replace("\r\n", "\n") == actual => {}
            Ok(_) => bad.push(format!("{}: KHÁC snapshot", sp.display())),
            Err(_) => bad.push(format!("{}: THIẾU snapshot", sp.display())),
        }
    }
    assert!(
        bad.is_empty(),
        "{}\nnếu thay đổi là chủ đích: GRIT_UPDATE_SNAPSHOTS=1 cargo test -p gt-resolve --test snapshots, rồi xem git diff tests/snapshots_resolve",
        bad.join("\n")
    );
}

#[test]
fn every_resolver_snapshot_has_a_case_and_says_something() {
    let dir = tests_dir().join("snapshots_resolve");
    let wanted: Vec<String> = cases().iter().map(|(c, _)| format!("{c}.snap")).collect();
    let rd = std::fs::read_dir(&dir).unwrap_or_else(|_| panic!("thiếu thư mục {}", dir.display()));
    for e in rd {
        let p = e.expect("entry").path();
        let n = name(&p);
        assert!(wanted.contains(&n), "snapshot mồ côi: {n}");
        let body = std::fs::read_to_string(&p).expect("đọc");
        assert!(body.contains("error[E02"), "{n}: không có chẩn đoán E02xx");
    }
}
