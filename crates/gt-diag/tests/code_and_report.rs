//! Mã lỗi, mức, bộ gom và phán quyết (D16, D17, D19). Viết TRƯỚC khi có cài đặt (TDD).
use gt_diag::{Code, Diagnostic, Diagnostics, Level};
use gt_span::Span;
use std::collections::BTreeSet;

fn d(code: &str, start: u32, end: u32, msg: &str) -> Diagnostic {
    Diagnostic::new(
        Code::parse(code).expect("mã hợp lệ"),
        "t.gt",
        Span::new(start, end),
        msg,
    )
}

#[test]
fn code_format_and_level() {
    // D16: E + 4 số = ĐỎ, S + 3 số = VÀNG, N + 4 số = GHI CHÚ
    assert_eq!(Code::parse("E0101").expect("ok").level(), Level::Error);
    assert_eq!(Code::parse("S101").expect("ok").level(), Level::Warn);
    assert_eq!(Code::parse("N0601").expect("ok").level(), Level::Note);
    assert_eq!(Code::parse("E0101").expect("ok").as_str(), "E0101");
    assert_eq!(Code::parse("S201").expect("ok").to_string(), "S201");
    for bad in [
        "", "E", "E101", "E01011", "S0101", "S10", "N601", "N06011", "X0101", "e0101", "E01a1",
        "E 101", "E0101 ", "EEEEE",
    ] {
        assert!(Code::parse(bad).is_err(), "{bad:?} phải bị từ chối");
    }
    // ICE = E99xx
    assert!(Code::parse("E9901").expect("ok").is_ice());
    assert!(Code::parse("E9999").expect("ok").is_ice());
    assert!(!Code::parse("E0101").expect("ok").is_ice());
    assert!(!Code::parse("S101").expect("ok").is_ice());
    // mức luôn suy ra từ mã, không thể mâu thuẫn
    let diag = d("S101", 0, 1, "m");
    assert_eq!(diag.level, Level::Warn);
}

#[test]
fn level_names_are_stable() {
    assert_eq!(Level::Error.as_str(), "error");
    assert_eq!(Level::Warn.as_str(), "warn");
    assert_eq!(Level::Note.as_str(), "note");
    assert!(Level::Error > Level::Warn && Level::Warn > Level::Note); // nặng hơn = lớn hơn
}

/// Bộ mã đã biết phải KHỚP docs/spec/error_codes.md (chống lệch tài liệu ↔ mã).
#[test]
fn known_codes_match_docs() {
    let md = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/spec/error_codes.md"
    ))
    .expect("error_codes.md");
    let mut doc: BTreeSet<String> = BTreeSet::new();
    for line in md.lines() {
        let cell = line
            .trim()
            .trim_start_matches('|')
            .split('|')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if Code::parse(&cell).is_ok() {
            doc.insert(cell);
        }
    }
    assert!(doc.len() >= 40, "không đọc được bảng mã lỗi: {}", doc.len());
    let code: BTreeSet<String> = gt_diag::known_codes()
        .iter()
        .map(|c| c.as_str().to_string())
        .collect();
    assert_eq!(
        code, doc,
        "gt_diag::known_codes() lệch docs/spec/error_codes.md"
    );
    assert!(Code::parse("E0101").expect("ok").is_known());
    assert!(!Code::parse("E7777").expect("ok").is_known());
}

#[test]
fn builder_fields() {
    let diag = d("E0101", 3, 5, "msg")
        .with_span_label("nhãn chính")
        .with_label("t.gt", Span::new(0, 1), "nhãn phụ")
        .with_note("ghi chú 1")
        .with_note("ghi chú 2");
    assert_eq!(diag.span, Span::new(3, 5));
    assert_eq!(diag.span_label.as_deref(), Some("nhãn chính"));
    assert_eq!(diag.labels.len(), 1);
    assert_eq!(diag.notes, ["ghi chú 1", "ghi chú 2"]);
    assert!(diag.suggestions.is_empty());
    assert_eq!(diag.file, "t.gt");
}

#[test]
fn report_sorts_deterministically_and_dedupes() {
    let build = |order: &[usize]| {
        let all = [
            d("E0201", 10, 12, "b"),
            d("E0101", 0, 2, "a"),
            d("E0101", 0, 2, "a"), // trùng hệt
            d("E0301", 10, 12, "c"),
            d("S101", 5, 6, "s"),
        ];
        let mut bag = Diagnostics::new(100);
        for &i in order {
            bag.push(all[i].clone());
        }
        bag.finish()
    };
    let r1 = build(&[0, 1, 2, 3, 4]);
    let r2 = build(&[4, 3, 2, 1, 0]);
    assert_eq!(r1, r2, "thứ tự chèn không được ảnh hưởng kết quả (D19)");
    let codes: Vec<_> = r1
        .diagnostics
        .iter()
        .map(|x| x.code.as_str().to_string())
        .collect();
    assert_eq!(codes, ["E0101", "S101", "E0201", "E0301"]); // theo vị trí, rồi mã; bản trùng bị bỏ
    assert_eq!(r1.truncated, 0);
}

#[test]
fn report_limit_truncates_but_never_hides_ice() {
    let mut bag = Diagnostics::new(2);
    for i in 0..5u32 {
        bag.push(d("E0101", i, i + 1, "e"));
    }
    bag.push(d("E9901", 100, 101, "ICE"));
    let r = bag.finish();
    let n_ice = r.diagnostics.iter().filter(|x| x.code.is_ice()).count();
    assert_eq!(n_ice, 1, "ICE không bao giờ bị cắt");
    assert_eq!(r.diagnostics.len(), 3); // 2 lỗi thường + 1 ICE
    assert_eq!(r.truncated, 3);
    assert_eq!(r.exit_code(false), 2);
}

#[test]
fn exit_code_d17() {
    let report = |codes: &[&str]| {
        let mut bag = Diagnostics::new(50);
        for (i, c) in codes.iter().enumerate() {
            bag.push(d(c, i as u32, i as u32 + 1, "m"));
        }
        bag.finish()
    };
    // 0 = qua (kể cả VÀNG và GHI CHÚ), không có gì
    assert_eq!(report(&[]).exit_code(false), 0);
    assert_eq!(report(&["S101"]).exit_code(false), 0);
    assert_eq!(report(&["N0601", "S201"]).exit_code(false), 0);
    // 1 = có ĐỎ
    assert_eq!(report(&["E0101"]).exit_code(false), 1);
    assert_eq!(report(&["S101", "E0301", "N0601"]).exit_code(false), 1);
    // 2 = ICE, thắng cả ĐỎ
    assert_eq!(report(&["E9901"]).exit_code(false), 2);
    assert_eq!(report(&["E0101", "E9901"]).exit_code(false), 2);
    // --deny-yellow nâng VÀNG thành ĐỎ (plan §5); không đổi GHI CHÚ
    assert_eq!(report(&["S101"]).exit_code(true), 1);
    assert_eq!(report(&["N0601"]).exit_code(true), 0);
    // đếm
    let r = report(&["E0101", "E0201", "S101", "N0601", "N0602", "N0901"]);
    assert_eq!((r.errors(), r.warnings(), r.notes()), (2, 1, 3));
    assert!(r.has_errors());
    assert!(!report(&["S101"]).has_errors());
}

/// D19: đầu ra không phụ thuộc thứ tự đầu vào — thử MỌI hoán vị của 6 chẩn đoán (kể cả hai tệp, cùng vị trí khác mã).
#[test]
fn every_input_permutation_gives_identical_report_and_json() {
    use gt_span::SourceFile;
    let mk = |code: &str, file: &str, s: u32, e: u32, m: &str| {
        Diagnostic::new(Code::parse(code).expect("mã"), file, Span::new(s, e), m)
    };
    let all = [
        mk("E0301", "a.gt", 4, 6, "x"),
        mk("E0101", "a.gt", 4, 6, "y"),
        mk("E0101", "a.gt", 4, 5, "z"),
        mk("S101", "a.gt", 0, 1, "w"),
        mk("E0201", "b.gt", 0, 2, "v"),
        mk("N0601", "b.gt", 0, 2, "u"),
    ];
    let file_a = SourceFile::new("a.gt", "0123456789".to_string()).expect("nhỏ");
    let file_b = SourceFile::new("b.gt", "ab".to_string()).expect("nhỏ");
    // sinh hoán vị bằng thuật toán Heap
    fn permutations(n: usize, a: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if n == 1 {
            out.push(a.clone());
            return;
        }
        for i in 0..n {
            permutations(n - 1, a, out);
            let j = if n.is_multiple_of(2) { i } else { 0 };
            a.swap(j, n - 1);
        }
    }
    let mut idx: Vec<usize> = (0..all.len()).collect();
    let mut perms = Vec::new();
    permutations(all.len(), &mut idx, &mut perms);
    assert_eq!(perms.len(), 720);
    let render = |order: &[usize]| {
        let mut bag = Diagnostics::new(100);
        for &i in order {
            bag.push(all[i].clone());
        }
        let r = bag.finish();
        let json = r.to_json(&[&file_a, &file_b], false);
        (r, json)
    };
    let (base_report, base_json) = render(&perms[0]);
    for p in &perms[1..] {
        let (r, j) = render(p);
        assert_eq!(r, base_report, "hoán vị {p:?}");
        assert_eq!(j, base_json, "hoán vị {p:?}");
    }
    // thứ tự mong đợi: theo (tệp, bắt đầu, kết thúc, mã, thông điệp)
    let got: Vec<String> = base_report
        .diagnostics
        .iter()
        .map(|d| format!("{}:{}:{}:{}", d.file, d.span.start, d.span.end, d.code))
        .collect();
    assert_eq!(
        got,
        [
            "a.gt:0:1:S101",
            "a.gt:4:5:E0101",
            "a.gt:4:6:E0101",
            "a.gt:4:6:E0301",
            "b.gt:0:2:E0201",
            "b.gt:0:2:N0601"
        ]
    );
}
