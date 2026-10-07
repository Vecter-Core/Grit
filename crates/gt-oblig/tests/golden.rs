//! Golden "số check còn lại" (ADR-025 §8): `tests/golden/obligations/<ca>.gt` → `<ca>.txt` khớp từng byte; `GRIT_UPDATE_SNAPSHOTS=1`
//! để cập nhật có chủ đích (rồi ĐỌC `git diff`). Kiểm cả JSON "check còn lại": tất định và khớp số liệu của bảng.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::analysis::Status;
use gt_oblig::driver::{OblOutput, analyze_program, count_asserts, remaining_checks_json};
use gt_oblig::prover::ZoneProver;
use std::path::{Path, PathBuf};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/obligations")
}

fn run_case(src: &str) -> (gt_types::TypesOutput, gt_mir::MirOutput, OblOutput) {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}",
        common::mir_codes(&m)
    );
    let o = analyze_program(&t, &m, &mut ZoneProver::new(24));
    (t, m, o)
}

fn render(src: &str) -> String {
    let (_t, m, o) = run_case(src);
    let mut s = String::new();
    for (b, r) in m.bodies.iter().zip(&o.per_body) {
        if b.key.starts_with("$prelude") {
            continue;
        }
        let after = o
            .bodies
            .iter()
            .find(|x| x.key == b.key)
            .map_or(0, count_asserts);
        let (mut d, mut k, mut red, mut rej) = (0, 0, 0, 0);
        for j in &r.judged {
            match j.status {
                Status::Discharged { .. } => d += 1,
                Status::Lowered { .. } => k += 1,
                Status::DefinitelyFalse => red += 1,
                Status::Rejected { .. } => rej += 1,
            }
        }
        s.push_str(&format!(
            "fn {}: nghĩa vụ {} | xoá {d} | giữ lúc chạy {k} | ĐỎ {red} | e0902 {rej} | Assert {} → {after}\n",
            b.key,
            r.judged.len(),
            count_asserts(b)
        ));
        for j in &r.judged {
            let st = match &j.status {
                Status::Discharged { prover, .. } => format!("xoá ({prover})"),
                Status::Lowered { note } => format!("giữ {note}"),
                Status::DefinitelyFalse => "ĐỎ".to_string(),
                Status::Rejected { .. } => "e0902".to_string(),
            };
            s.push_str(&format!(
                "  {} {:<14} bb{:<3} {}\n",
                j.ob.id,
                j.ob.kind.name(),
                j.ob.block,
                st
            ));
        }
    }
    let st = &o.stats;
    s.push_str(&format!(
        "-- tổng: nghĩa vụ {} | xoá {} | giữ {} | ĐỎ {} | e0902 {} | Assert {} → {}\n",
        st.obligations,
        st.discharged,
        st.lowered,
        st.definitely_false,
        st.rejected,
        st.asserts_before,
        st.asserts_after
    ));
    s
}

#[test]
fn obligation_tables_match_golden_files() {
    let update = std::env::var("GRIT_UPDATE_SNAPSHOTS").is_ok_and(|v| v == "1");
    let mut cases: Vec<PathBuf> = std::fs::read_dir(dir())
        .expect("thư mục golden")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .collect();
    cases.sort();
    assert!(cases.len() >= 6, "chỉ {} ca golden", cases.len());
    let mut bad = Vec::new();
    for p in &cases {
        let src = std::fs::read_to_string(p)
            .expect("đọc")
            .replace("\r\n", "\n");
        let actual = render(&src);
        let want_path = p.with_extension("txt");
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
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn remaining_checks_json_is_deterministic_and_consistent() {
    for p in std::fs::read_dir(dir()).expect("dir") {
        let p = p.expect("entry").path();
        if !p.extension().is_some_and(|x| x == "gt") {
            continue;
        }
        let src = std::fs::read_to_string(&p)
            .expect("đọc")
            .replace("\r\n", "\n");
        let (_t, _m, o1) = run_case(&src);
        let (_t, _m, o2) = run_case(&src);
        let (j1, j2) = (remaining_checks_json(&o1), remaining_checks_json(&o2));
        assert_eq!(j1, j2, "{}: JSON không tất định", p.display());
        assert!(j1.starts_with("{\"schema\":1,\"totals\":{"), "{j1}");
        let st = &o1.stats;
        assert!(j1.contains(&format!("\"obligations\":{}", st.obligations)));
        assert!(j1.contains(&format!("\"remaining_runtime_checks\":{}", st.lowered)));
        assert_eq!(
            st.obligations,
            st.discharged
                + st.lowered
                + st.definitely_false
                + o1.per_body
                    .iter()
                    .flat_map(|r| &r.judged)
                    .filter(|j| matches!(j.status, Status::Rejected { .. }))
                    .count(),
            "{}: tổng nghĩa vụ không khớp",
            p.display()
        );
        // mỗi nghĩa vụ xoá được = một Assert ít hơn (hợp đồng chèn thêm Assert nên so sánh theo thân không hợp đồng)
        if st.rejected == 0 && !src.contains('⊳') && !src.contains('⊲') {
            assert_eq!(
                st.asserts_before - st.discharged,
                st.asserts_after,
                "{}: Assert trước/sau",
                p.display()
            );
        }
    }
}
