//! Test gt-span (M1) — viết trước khi có cài đặt (TDD).
use gt_span::{FileId, LineCol, SourceFile, SourceMap, SourceMapError, Span};

#[test]
fn span_basics() {
    let s = Span::new(2, 5);
    assert_eq!(s.start, 2);
    assert_eq!(s.end, 5);
    assert_eq!(s.len(), 3);
    assert!(!s.is_empty());
    assert!(Span::new(4, 4).is_empty());
    assert_eq!(Span::new(1, 3).to(Span::new(6, 9)), Span::new(1, 9));
    assert_eq!(Span::new(6, 9).to(Span::new(1, 3)), Span::new(1, 9));
}

#[test]
#[should_panic]
fn span_new_rejects_inverted_range() {
    // start > end là lỗi lập trình trong compiler (ICE), không được âm thầm chấp nhận.
    let _ = Span::new(5, 2);
}

#[test]
fn line_col_lf_and_crlf() {
    // "ab\ncd\r\nef": byte 0..2 'a''b', 2 '\n', 3..5 'c''d', 5 '\r', 6 '\n', 7..9 'e''f'
    let f = SourceFile::new("t.gt", "ab\ncd\r\nef".to_string()).expect("nhỏ");
    assert_eq!(f.line_col(0), Some(LineCol { line: 1, col: 1 }));
    assert_eq!(f.line_col(2), Some(LineCol { line: 1, col: 3 })); // chính ký tự '\n'
    assert_eq!(f.line_col(3), Some(LineCol { line: 2, col: 1 }));
    assert_eq!(f.line_col(5), Some(LineCol { line: 2, col: 3 })); // '\r'
    assert_eq!(f.line_col(7), Some(LineCol { line: 3, col: 1 }));
    assert_eq!(f.line_col(9), Some(LineCol { line: 3, col: 3 })); // EOF hợp lệ
    assert_eq!(f.line_col(10), None); // quá cuối file
}

#[test]
fn line_col_counts_unicode_scalars_not_bytes() {
    // "ƒ x": 'ƒ' chiếm 2 byte; 'x' ở byte 3 nhưng là cột 3 (ký tự thứ 3).
    let f = SourceFile::new("t.gt", "ƒ x".to_string()).expect("nhỏ");
    assert_eq!(f.line_col(3), Some(LineCol { line: 1, col: 3 }));
    // offset nằm giữa một ký tự nhiều byte không hợp lệ
    assert_eq!(f.line_col(1), None);
}

#[test]
fn snippet_and_line_text() {
    let f = SourceFile::new("t.gt", "ab\ncd\r\nef".to_string()).expect("nhỏ");
    assert_eq!(f.snippet(Span::new(3, 5)), Some("cd"));
    assert_eq!(f.snippet(Span::new(0, 9)), Some("ab\ncd\r\nef"));
    assert_eq!(f.snippet(Span::new(0, 99)), None);
    assert_eq!(f.line_text(1), Some("ab"));
    assert_eq!(f.line_text(2), Some("cd")); // bỏ cả "\r\n"
    assert_eq!(f.line_text(3), Some("ef"));
    assert_eq!(f.line_text(4), None);
    assert_eq!(f.line_count(), 3);
    // span cắt giữa ký tự nhiều byte
    let g = SourceFile::new("u.gt", "ƒx".to_string()).expect("nhỏ");
    assert_eq!(g.snippet(Span::new(1, 2)), None);
    assert_eq!(g.snippet(Span::new(0, 2)), Some("ƒ"));
}

#[test]
fn empty_file_and_name() {
    let f = SourceFile::new("e.gt", String::new()).expect("nhỏ");
    assert_eq!(f.name(), "e.gt");
    assert_eq!(f.text(), "");
    assert_eq!(f.len(), 0);
    assert_eq!(f.line_count(), 1);
    assert_eq!(f.line_col(0), Some(LineCol { line: 1, col: 1 }));
    assert_eq!(f.line_text(1), Some(""));
}

// ---------- FileId / SourceMap (ADR-017a) ----------

fn sf(name: &str, text: &str) -> SourceFile {
    SourceFile::new(name, text.to_string()).expect("nhỏ")
}

#[test]
fn source_map_assigns_file_ids_by_sorted_name_not_by_input_order() {
    let a = SourceMap::new(vec![sf("b.gt", "2"), sf("a.gt", "1"), sf("c.gt", "3")]).expect("map");
    let b = SourceMap::new(vec![sf("c.gt", "3"), sf("a.gt", "1"), sf("b.gt", "2")]).expect("map");
    for m in [&a, &b] {
        assert_eq!(m.len(), 3);
        assert_eq!(m.id_of("a.gt"), Some(FileId::new(0)));
        assert_eq!(m.id_of("b.gt"), Some(FileId::new(1)));
        assert_eq!(m.id_of("c.gt"), Some(FileId::new(2)));
        assert_eq!(m.id_of("zzz.gt"), None);
        assert_eq!(m.get(FileId::new(1)).map(SourceFile::name), Some("b.gt"));
        assert_eq!(m.get(FileId::new(9)).map(SourceFile::name), None);
    }
    let names: Vec<&str> = a.iter().map(|(_, f)| f.name()).collect();
    assert_eq!(names, ["a.gt", "b.gt", "c.gt"]);
    let ids: Vec<u32> = a.iter().map(|(id, _)| id.raw()).collect();
    assert_eq!(ids, [0, 1, 2]);
}

#[test]
fn source_map_rejects_duplicate_names_and_handles_empty() {
    let e = SourceMap::new(vec![sf("a.gt", "1"), sf("a.gt", "2")]).expect_err("trùng");
    assert_eq!(e, SourceMapError::DuplicateName("a.gt".to_string()));
    let m = SourceMap::new(vec![]).expect("rỗng hợp lệ");
    assert!(m.is_empty());
    assert_eq!(m.len(), 0);
}

#[test]
fn file_id_is_ordered_hashable_and_indexes() {
    assert!(FileId::new(1) < FileId::new(2));
    assert_eq!(FileId::new(7).index(), 7);
    assert_eq!(FileId::new(7).raw(), 7);
    let mut set = std::collections::HashSet::new();
    set.insert(FileId::new(1));
    assert!(set.contains(&FileId::new(1)));
}
