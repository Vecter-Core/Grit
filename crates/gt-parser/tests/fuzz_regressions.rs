//! Mọi đầu vào từng làm sập fuzz parser (hoặc vi phạm bất biến của fuzz target) được giữ lại ở `tests/fuzz_regressions/`
//! và kiểm LẠI đúng các bất biến của `fuzz/fuzz_targets/parse.rs` trong test thường trực.
use gt_parser::parse;

fn check(name: &str, src: &str) {
    let out = parse("f.gt", src);
    for d in &out.report.diagnostics {
        assert!(!d.code.is_ice(), "{name}: ICE: {}", d.message);
        assert!(d.code.is_known(), "{name}");
        assert!(
            d.span.start <= d.span.end && d.span.end as usize <= src.len(),
            "{name}: span"
        );
        assert!(
            src.is_char_boundary(d.span.start as usize)
                && src.is_char_boundary(d.span.end as usize),
            "{name}: span cắt giữa ký tự"
        );
    }
    assert!(
        out.work <= 300 * (src.len() + 1),
        "{name}: công việc siêu tuyến tính {} / {}",
        out.work,
        src.len()
    );
    assert!(out.node_count >= 1, "{name}");
    let again = parse("f.gt", src);
    assert_eq!(
        format!("{:?}", out.program),
        format!("{:?}", again.program),
        "{name}: không tất định"
    );
    assert_eq!(out.report, again.report, "{name}: không tất định");
}

#[test]
fn every_fuzz_regression_input_satisfies_the_fuzz_invariants() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fuzz_regressions");
    let mut n = 0;
    for e in std::fs::read_dir(&dir).expect("thư mục regression") {
        let p = e.expect("entry").path();
        let Ok(src) = std::fs::read_to_string(&p) else {
            continue;
        };
        check(&p.display().to_string(), &src);
        n += 1;
    }
    assert!(n >= 1, "thư mục regression rỗng");
}
