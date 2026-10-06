//! Bất biến của MIR do bước hạ sinh ra (ADR-020 §9, Q25): trên MỌI mẫu `pass`, mẫu golden và mẫu của M6+, mọi lần đọc một
//! local đều sau một lần gán trên mọi đường (vi phạm = ICE); liveness/khởi tạo giải được (hội tụ) trên mọi thân.
#[path = "../../gt-types/tests/common/mod.rs"]
mod tc;
use gt_dataflow::*;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests")
}

fn gt_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("dir")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .collect();
    v.sort();
    v
}

#[test]
fn lowered_mir_satisfies_the_assignment_invariant_and_analyses_converge() {
    let mut files = gt_files(&root().join("pass"));
    files.extend(gt_files(&root().join("golden/mir")));
    files.extend(gt_files(&root().join("fail")).into_iter().filter(|p| {
        p.file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("E04"))
    }));
    let mut bodies = 0;
    for p in &files {
        let src = std::fs::read_to_string(p)
            .expect("đọc")
            .replace("\r\n", "\n");
        let t = tc::one(&src);
        if !t.report.diagnostics.is_empty() {
            continue;
        }
        let m = gt_mir::lower_program(&t);
        assert!(
            m.report.diagnostics.is_empty(),
            "{}: {:?}",
            p.display(),
            m.report.diagnostics
        );
        let r = check_invariants(&t, &m);
        assert!(
            r.diagnostics.is_empty(),
            "{}: {:?}",
            p.display(),
            r.diagnostics
        );
        for b in &m.bodies {
            bodies += 1;
            let mp = MovePaths::build(b);
            solve(&MaybeInit { paths: &mp }, b).expect("hội tụ");
            solve(&MaybeUninit { paths: &mp }, b).expect("hội tụ");
            solve(&Liveness, b).expect("hội tụ");
        }
    }
    assert!(bodies >= 120, "chỉ {bodies} thân");
}
