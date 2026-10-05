//! Snapshot test cho THÔNG BÁO LỖI (plan §6.2, tiêu chí hoàn thành M2): với mỗi file `tests/fail/E01xx|E12xx`, đầu ra người-đọc
//! (không màu) và JSON của mọi chẩn đoán phải khớp từng byte với `tests/snapshots/<tên>.snap`.
//!
//! * Snapshot thiếu hoặc khác → test ĐỎ kèm chỉ dẫn; không bao giờ tự cập nhật ngầm.
//! * Cập nhật CÓ CHỦ ĐÍCH: `GRIT_UPDATE_SNAPSHOTS=1 cargo test -p gt-parser --test snapshots`, rồi đọc `git diff tests/snapshots`.
//! * Không có snapshot mồ côi (không file fail tương ứng).
use gt_diag::{RenderOptions, diagnostic_to_json, render_human};
use gt_span::SourceFile;
use std::path::{Path, PathBuf};

/// Mã thuộc phạm vi lexer/parser (cùng danh sách với `corpus.rs`).
const OWNED: &[&str] = &["E0101", "E0102", "E1201", "E1202", "E1203", "E1204"];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests")
}

fn owned_fail_files() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(repo().join("fail"))
        .expect("tests/fail")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .filter(|p| {
            let n = p.file_name().expect("tên").to_string_lossy().into_owned();
            OWNED.iter().any(|c| n.starts_with(&format!("{c}_")))
        })
        .collect();
    v.sort();
    v
}

fn snapshot_of(p: &Path) -> String {
    let name = p.file_name().expect("tên").to_string_lossy().into_owned();
    let text = std::fs::read_to_string(p)
        .expect("đọc")
        .replace("\r\n", "\n");
    let label = format!("tests/fail/{name}");
    let out = gt_parser::parse(&label, &text);
    let file = SourceFile::new(label.clone(), text).expect("nhỏ");
    let mut s = String::new();
    s.push_str(&format!(
        "# {label}: {} chẩn đoán\n",
        out.report.diagnostics.len()
    ));
    for (i, d) in out.report.diagnostics.iter().enumerate() {
        s.push_str(&format!("\n## [{}] human\n", i + 1));
        s.push_str(&render_human(d, &file, &RenderOptions { color: false }));
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(&format!("## [{}] json\n", i + 1));
        s.push_str(&diagnostic_to_json(d, &[&file]));
        s.push('\n');
    }
    s
}

fn snap_path(p: &Path) -> PathBuf {
    let n = p.file_stem().expect("tên").to_string_lossy().into_owned();
    repo().join("snapshots").join(format!("{n}.snap"))
}

#[test]
fn error_messages_match_their_snapshots() {
    let update = std::env::var("GRIT_UPDATE_SNAPSHOTS").is_ok_and(|v| v == "1");
    let files = owned_fail_files();
    assert!(
        files.len() >= 20,
        "chỉ có {} file fail của lexer/parser",
        files.len()
    );
    let mut bad = Vec::new();
    for p in &files {
        let actual = snapshot_of(p);
        let sp = snap_path(p);
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
        "{}\nnếu thay đổi là chủ đích: GRIT_UPDATE_SNAPSHOTS=1 cargo test -p gt-parser --test snapshots, rồi xem git diff tests/snapshots",
        bad.join("\n")
    );
}

#[test]
fn every_snapshot_has_a_fail_file_and_says_something() {
    let dir = repo().join("snapshots");
    let wanted: Vec<String> = owned_fail_files()
        .iter()
        .map(|p| {
            snap_path(p)
                .file_name()
                .expect("tên")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let Ok(rd) = std::fs::read_dir(&dir) else {
        panic!("thiếu thư mục {}", dir.display());
    };
    for e in rd {
        let p = e.expect("entry").path();
        let n = p.file_name().expect("tên").to_string_lossy().into_owned();
        assert!(wanted.contains(&n), "snapshot mồ côi: {n}");
        let body = std::fs::read_to_string(&p).expect("đọc");
        assert!(
            body.contains("error[E"),
            "{n}: snapshot không có chẩn đoán nào"
        );
    }
}
