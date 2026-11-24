#![no_main]
//! Fuzz parser: với MỌI chuỗi UTF-8 — không ICE (panic nội bộ bị parser bắt thành E9901 nên phải kiểm ở đây),
//! luôn kết thúc, công việc tuyến tính theo độ dài, mọi Diagnostic có span hợp lệ và mã đã biết, và kết quả tất định.
use gt_parser::parse_file_with_edition;
use gt_span::FileId;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else { return };
    // FB-C49 (Gemini G-09a): fuzz CẢ edition 2 (`?`, `let` phân rã, or-pattern)
    for edition in [1u32, 2] {
    let out = parse_file_with_edition(FileId::new(0), "f.gt", src, edition);
    for d in &out.report.diagnostics {
        assert!(!d.code.is_ice(), "ICE: {}", d.message);
        assert!(d.code.is_known());
        assert!(d.span.start <= d.span.end && d.span.end as usize <= src.len());
        assert!(src.is_char_boundary(d.span.start as usize) && src.is_char_boundary(d.span.end as usize));
    }
    assert!(out.work <= 300 * (src.len() + 1), "công việc siêu tuyến tính: {} / {}", out.work, src.len());
    assert!(out.node_count >= 1);
    // tất định (D9): hai lần parse cho cùng chẩn đoán và cùng AST
    let again = parse_file_with_edition(FileId::new(0), "f.gt", src, edition);
    assert_eq!(format!("{:?}", out.program), format!("{:?}", again.program));
    assert_eq!(out.report, again.report);
    }
});
