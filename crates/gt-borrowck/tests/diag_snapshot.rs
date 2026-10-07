//! Snapshot của NHÃN và VỊ TRÍ chẩn đoán E0401–E0407 trên toàn bộ mẫu xấu (hai đột biến chỉ đổi vị trí chẩn đoán cho thấy trước đó
//! chưa test nào khoá điều này). Mỗi dòng: mẫu | mã | văn bản được gạch chân | nhãn (văn bản nhãn @ văn bản được gạch) | thông điệp.
//! Đổi chủ đích: `GRIT_UPDATE_SNAPSHOTS=1 cargo test -p gt-borrowck --test diag_snapshot`, rồi đọc `git diff tests/golden/borrowck`.
mod common;
use common::*;
use std::path::{Path, PathBuf};

fn clip(s: &str) -> String {
    let one: String = s.chars().map(|c| if c == '\n' { '⏎' } else { c }).collect();
    if one.chars().count() > 70 {
        format!("{}…", one.chars().take(70).collect::<String>())
    } else {
        one
    }
}

fn render() -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fail");
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("thư mục")
        .map(|e| e.expect("e").path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "gt")
                && p.file_stem()
                    .is_some_and(|n| n.to_string_lossy().starts_with("E040"))
        })
        .collect();
    files.sort();
    let mut out = String::new();
    for p in files {
        let name = p.file_stem().expect("tên").to_string_lossy().to_string();
        let src = std::fs::read_to_string(&p)
            .expect("đọc")
            .replace("\r\n", "\n");
        let r = bc(&src);
        let at = |s: gt_span::Span| clip(&src[s.start as usize..s.end as usize]);
        for d in &r.bc.diagnostics {
            let labels: Vec<String> = d
                .labels
                .iter()
                .map(|l| format!("{} @ `{}`", l.message, at(l.span)))
                .collect();
            out.push_str(&format!(
                "{name} | {} | `{}` | [{}] | {}\n",
                d.code,
                at(d.span),
                labels.join("; "),
                d.message
            ));
        }
    }
    out
}

#[test]
fn diagnostics_of_e0401_to_e0407_match_the_snapshot() {
    let actual = render();
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/borrowck/diagnostics.txt");
    if std::env::var("GRIT_UPDATE_SNAPSHOTS").is_ok_and(|v| v == "1") {
        std::fs::create_dir_all(path.parent().expect("cha")).expect("tạo thư mục");
        std::fs::write(&path, &actual).expect("ghi");
        return;
    }
    let want = std::fs::read_to_string(&path)
        .expect("thiếu snapshot (chạy với GRIT_UPDATE_SNAPSHOTS=1)")
        .replace("\r\n", "\n");
    assert!(
        actual == want,
        "chẩn đoán khác snapshot; nếu chủ đích: GRIT_UPDATE_SNAPSHOTS=1 rồi xem git diff"
    );
    // mọi chẩn đoán phải có vị trí không rỗng và (trừ E0407, không có khoản mượn) nhãn trỏ vào chỗ nguồn gốc
    for l in actual.lines() {
        let f: Vec<&str> = l.split(" | ").collect();
        assert!(f[2] != "``", "vị trí rỗng: {l}");
        if !f[1].starts_with("E0407")
            && !f[1].starts_with("E0401")
            && !f[4].starts_with("tham chiếu có region")
        {
            assert!(f[3] != "[]", "thiếu nhãn: {l}");
        }
    }
}
