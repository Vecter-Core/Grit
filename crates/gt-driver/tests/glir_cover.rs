//! ADR-P2-02 C3: đường hạ MIR→GLIR (gt-glirgen) trên MỌI mẫu `tests/pass`: mọi hàm hạ được phải qua VERIFIER GLIR (0 lỗi) và in-đọc-in ổn định;
//! hàm chưa hạ được được ĐẾM theo lý do (in bằng `--nocapture`). Độ phủ chỉ được tăng (ngưỡng dưới ghi ở cuối, nâng dần).
#![cfg(feature = "llvm")]
use gt_driver::{Options, check};
use gt_resolve::SourceInput;
use std::collections::BTreeMap;

#[test]
fn every_lowered_body_passes_the_glir_verifier_and_coverage_is_measured() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/pass");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .expect("dir")
        .map(|e| e.expect("e").path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .collect();
    files.sort();
    let (mut lowered, mut total) = (0usize, 0usize);
    let mut why: BTreeMap<String, usize> = BTreeMap::new();
    for p in files {
        let src = std::fs::read_to_string(&p).expect("đọc");
        let Ok(res) = check(
            &[SourceInput {
                name: "main.gt".to_string(),
                text: src,
            }],
            &Options::default(),
        ) else {
            continue;
        };
        let Some(art) = res.open_gate() else { continue };
        for b in &art.mir.bodies {
            if b.kind != gt_mir::BodyKind::Fn {
                continue;
            }
            total += 1;
            match gt_glirgen::lower_body(&art.types.db, b) {
                Ok(f) => {
                    lowered += 1;
                    let m = gt_glir::Module {
                        decls: Vec::new(),
                        funcs: vec![f],
                    };
                    let errs = gt_glir::verify(&m);
                    assert!(
                        errs.is_empty(),
                        "{} :: {}: {errs:?}\n{}",
                        p.display(),
                        b.key,
                        gt_glir::print(&m)
                    );
                    let text = gt_glir::print(&m);
                    let back = gt_glir::parse(&text).unwrap_or_else(|e| {
                        panic!("{}: in ra không đọc lại được: {e}\n{text}", b.key)
                    });
                    assert_eq!(back, m, "{}: đọc(in) ≠ gốc", b.key);
                }
                Err(u) => *why.entry(u.0).or_default() += 1,
            }
        }
    }
    let mut by: Vec<_> = why.into_iter().collect();
    by.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    eprintln!("GLIR_COVER lowered={lowered} total={total}");
    for (w, n) in by.iter().take(15) {
        eprintln!("  chưa hạ: {n:5}  {w}");
    }
    assert!(lowered >= 1, "không hạ được thân nào");
}
