//! Corpus M6 (ADR-023 §7): mọi mẫu `tests/fail/E04*.gt` bị chặn ĐÚNG mã (và chỉ mã đó); mọi mẫu `tests/pass/*.gt` qua sạch.
mod common;
use common::*;
use std::path::{Path, PathBuf};

fn files(kind: &str) -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join(kind);
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("thư mục corpus")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "gt"))
        .collect();
    v.sort();
    v
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p)
        .expect("đọc")
        .replace("\r\n", "\n")
}

/// Mã M6 nhận được từ một chương trình; M4/M5 phải sạch (corpus M6 chỉ thử borrowck).
fn borrowck_codes(name: &str, src: &str) -> Vec<String> {
    let t = one(src);
    assert!(
        t.report.diagnostics.is_empty(),
        "{name}: M4 báo {:?}",
        show(&t)
    );
    let mut m = gt_mir::lower_program(&t);
    assert!(m.report.diagnostics.is_empty(), "{name}: M5 báo lỗi");
    // như pipeline: elaboration drop (ADR-C18) chạy giữa M5 và M6; lỗi của nó (E0407 ở hợp đồng) cũng là lỗi sở hữu
    let e = gt_dataflow::elaborate_program(&t, &mut m);
    let r = gt_borrowck::check_program(&t, &m);
    e.diagnostics
        .iter()
        .chain(r.diagnostics.iter())
        .map(|d| d.code.as_str().to_string())
        .collect()
}

#[test]
fn every_bad_sample_is_rejected_with_exactly_its_code() {
    let mut per_code: std::collections::BTreeMap<String, usize> = Default::default();
    let mut problems = vec![];
    for p in files("fail") {
        let name = p.file_stem().expect("tên").to_string_lossy().to_string();
        if !name.starts_with("E04") {
            continue;
        }
        let code = name[..5].to_string();
        let got = borrowck_codes(&name, &read(&p));
        if got.is_empty() || got.iter().any(|c| *c != code) {
            problems.push(format!("{name}: cần chỉ {code}, nhận {got:?}"));
        }
        *per_code.entry(code).or_default() += 1;
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert_eq!(
        gt_borrowck::defensive_counters(),
        (0, 0),
        "đường dự phòng của regions.rs bị dùng"
    );
    let total: usize = per_code.values().sum();
    assert!(
        total >= 155,
        "corpus xấu chỉ có {total} mẫu (ADR-023 §7: ≥155): {per_code:?}"
    );
    for c in [
        "E0401", "E0402", "E0403", "E0404", "E0405", "E0406", "E0407",
    ] {
        assert!(
            per_code.get(c).copied().unwrap_or(0) >= 15,
            "{c} chỉ có {:?} mẫu: {per_code:?}",
            per_code.get(c)
        );
    }
}

#[test]
fn every_good_sample_is_accepted() {
    let mut problems = vec![];
    let mut n = 0;
    for p in files("pass") {
        let name = p.file_stem().expect("tên").to_string_lossy().to_string();
        let src = read(&p);
        // chỉ các mẫu qua được M4 (một số mẫu pass cũ chỉ để thử M2/M3 không có thân hợp lệ ở M4 thì đã bị bắt ở test M4)
        let t = one(&src);
        assert!(
            t.report.diagnostics.is_empty(),
            "{name}: M4 báo {:?}",
            show(&t)
        );
        let got = borrowck_codes(&name, &src);
        if !got.is_empty() {
            problems.push(format!("{name}: chặn nhầm {got:?}"));
        }
        n += 1;
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert_eq!(
        gt_borrowck::defensive_counters(),
        (0, 0),
        "đường dự phòng của regions.rs bị dùng"
    );
    assert!(n >= 155 + 77, "corpus đúng chỉ có {n} mẫu");
}

/// Nhóm theo bảng ADR-023 §7: mỗi mẫu `bc` mang dòng `// group: Gn`; mỗi nhóm cần đủ số mẫu xấu và mẫu đúng.
#[test]
fn every_group_of_adr_023_table_meets_its_minimum() {
    // (nhóm, tối thiểu xấu, tối thiểu đúng) — ADR-023 §7
    let table = [
        ("G1", 30, 30),
        ("G2", 20, 20),
        ("G3", 20, 20),
        ("G4", 15, 15),
        ("G5", 20, 20),
        ("G6", 20, 20),
        ("G7", 15, 15),
        ("G8", 15, 15),
    ];
    let mut bad: std::collections::BTreeMap<String, usize> = Default::default();
    let mut good: std::collections::BTreeMap<String, usize> = Default::default();
    for (kind, acc) in [("fail", &mut bad), ("pass", &mut good)] {
        for p in files(kind) {
            let name = p.file_stem().expect("tên").to_string_lossy().to_string();
            let src = read(&p);
            let Some(g) = src.lines().find_map(|l| l.strip_prefix("// group: ")) else {
                continue; // mẫu cũ của các milestone trước không có nhóm
            };
            assert!(
                table.iter().any(|(t, ..)| *t == g.trim()),
                "{name}: nhóm lạ `{g}`"
            );
            *acc.entry(g.trim().to_string()).or_default() += 1;
        }
    }
    eprintln!("nhóm: xấu {bad:?}");
    eprintln!("nhóm: đúng {good:?}");
    for (g, min_bad, min_good) in table {
        let (b, o) = (
            bad.get(g).copied().unwrap_or(0),
            good.get(g).copied().unwrap_or(0),
        );
        assert!(b >= min_bad, "{g}: chỉ {b} mẫu xấu (cần ≥{min_bad})");
        assert!(o >= min_good, "{g}: chỉ {o} mẫu đúng (cần ≥{min_good})");
    }
}
