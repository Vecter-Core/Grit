//! Golden MIR (plan M5: "in được MIR dạng text ổn định"): `tests/golden/mir/<ca>.gt` → `<ca>.mir` khớp từng byte.
//! Cập nhật có chủ đích: `GRIT_UPDATE_SNAPSHOTS=1 cargo test -p gt-mir --test golden`, rồi ĐỌC `git diff`.
mod common;
use common::*;
use std::path::{Path, PathBuf};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/mir")
}

fn render(src: &str) -> String {
    let (t, m) = mir(src);
    assert!(m.report.diagnostics.is_empty(), "{:?}", mir_codes(&m));
    let mut s = String::new();
    for b in &m.bodies {
        s.push_str(&text(&t, &m, &b.key));
        s.push('\n');
    }
    s
}

#[test]
fn mir_text_matches_golden_files() {
    let update = std::env::var("GRIT_UPDATE_SNAPSHOTS").is_ok_and(|v| v == "1");
    let mut cases: Vec<PathBuf> = std::fs::read_dir(dir())
        .expect("thư mục golden")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .collect();
    cases.sort();
    assert!(cases.len() >= 9, "chỉ {} ca golden", cases.len());
    let mut bad = Vec::new();
    for p in &cases {
        let src = std::fs::read_to_string(p)
            .expect("đọc")
            .replace("\r\n", "\n");
        let actual = render(&src);
        let want_path = p.with_extension("mir");
        if update {
            std::fs::write(&want_path, &actual).expect("ghi");
            continue;
        }
        match std::fs::read_to_string(&want_path) {
            Ok(w) if w.replace("\r\n", "\n") == actual => {}
            Ok(_) => bad.push(format!("{}: khác golden", want_path.display())),
            Err(_) => bad.push(format!("{}: THIẾU golden", want_path.display())),
        }
    }
    assert!(
        bad.is_empty(),
        "{}\nnếu thay đổi là chủ đích: GRIT_UPDATE_SNAPSHOTS=1 cargo test -p gt-mir --test golden, rồi xem git diff tests/golden/mir",
        bad.join("\n")
    );
}

#[test]
fn printing_is_deterministic() {
    for p in std::fs::read_dir(dir()).expect("dir") {
        let p = p.expect("entry").path();
        if p.extension().is_some_and(|x| x == "gt") {
            let src = std::fs::read_to_string(&p).expect("đọc");
            assert_eq!(render(&src), render(&src), "{}", p.display());
        }
    }
}
