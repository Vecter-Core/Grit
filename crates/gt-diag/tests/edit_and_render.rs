//! Edit có cấu trúc và dạng người đọc (golden). Viết TRƯỚC khi có cài đặt (TDD).
use gt_diag::{
    Applicability, Code, Diagnostic, Edit, EditError, RenderOptions, Suggestion, apply_edits,
    render_human,
};
use gt_span::{SourceFile, Span};

fn file(text: &str) -> SourceFile {
    SourceFile::new("t.gt", text.to_string()).expect("nhỏ")
}

fn diag(code: &str, start: u32, end: u32, msg: &str) -> Diagnostic {
    Diagnostic::new(
        Code::parse(code).expect("mã"),
        "t.gt",
        Span::new(start, end),
        msg,
    )
}

fn plain() -> RenderOptions {
    RenderOptions { color: false }
}

// ---------- apply_edits ----------

#[test]
fn edits_apply_in_any_order() {
    let text = "abcdef";
    let a = Edit::new(Span::new(0, 1), "X");
    let b = Edit::new(Span::new(3, 5), "YY");
    assert_eq!(
        apply_edits(text, &[a.clone(), b.clone()]).expect("ok"),
        "XbcYYf"
    );
    assert_eq!(apply_edits(text, &[b, a]).expect("ok"), "XbcYYf");
    // chèn (span rỗng) và xoá (thay bằng rỗng)
    assert_eq!(
        apply_edits(text, &[Edit::new(Span::new(2, 2), "--")]).expect("ok"),
        "ab--cdef"
    );
    assert_eq!(
        apply_edits(text, &[Edit::new(Span::new(1, 3), "")]).expect("ok"),
        "adef"
    );
    // không có edit
    assert_eq!(apply_edits(text, &[]).expect("ok"), text);
    // kề nhau (không chồng)
    assert_eq!(
        apply_edits(
            text,
            &[
                Edit::new(Span::new(0, 2), "1"),
                Edit::new(Span::new(2, 4), "2")
            ]
        )
        .expect("ok"),
        "12ef"
    );
}

#[test]
fn edits_reject_bad_input() {
    let text = "ƒ x";
    // ngoài tệp
    assert_eq!(
        apply_edits(text, &[Edit::new(Span::new(0, 99), "")]),
        Err(EditError::OutOfBounds)
    );
    // cắt giữa ký tự nhiều byte ('ƒ' = 2 byte)
    assert_eq!(
        apply_edits(text, &[Edit::new(Span::new(1, 2), "")]),
        Err(EditError::NotCharBoundary)
    );
    // chồng nhau
    assert_eq!(
        apply_edits(
            "abcdef",
            &[
                Edit::new(Span::new(0, 3), "a"),
                Edit::new(Span::new(2, 4), "b")
            ]
        ),
        Err(EditError::Overlap)
    );
    // hai lần chèn cùng một điểm: thứ tự mơ hồ → từ chối (fail-closed)
    assert_eq!(
        apply_edits(
            "abc",
            &[
                Edit::new(Span::new(1, 1), "x"),
                Edit::new(Span::new(1, 1), "y")
            ]
        ),
        Err(EditError::Overlap)
    );
}

// ---------- render: golden ----------

#[test]
fn single_line_with_label() {
    let f = file("ƒ f( { 1 }\n");
    let d = diag("E0101", 6, 7, "mong đợi `)`").with_span_label("thiếu dấu đóng");
    let expected = "\
error[E0101]: mong đợi `)`
 --> t.gt:1:6
  |
1 | ƒ f( { 1 }
  |      ^ thiếu dấu đóng
";
    assert_eq!(render_human(&d, &f, &plain()), expected);
}

#[test]
fn caret_width_follows_span_in_chars() {
    let f = file("ƒ ≔ 12345\n");
    // "12345" bắt đầu ở byte 7 (ƒ=2, ' '=1, ≔=3, ' '=1), col = 5
    let d = diag("E0301", 7, 12, "sai kiểu");
    let expected = "\
error[E0301]: sai kiểu
 --> t.gt:1:5
  |
1 | ƒ ≔ 12345
  |     ^^^^^
";
    assert_eq!(render_human(&d, &f, &plain()), expected);
}

#[test]
fn empty_span_and_eof() {
    let f = file("ab\ncd");
    // span rỗng giữa dòng: một dấu ^
    let mid = diag("E0101", 1, 1, "chèn ở đây");
    let expected_mid = "\
error[E0101]: chèn ở đây
 --> t.gt:1:2
  |
1 | ab
  |  ^
";
    assert_eq!(render_human(&mid, &f, &plain()), expected_mid);
    // EOF: offset = độ dài tệp, dòng cuối không có xuống dòng
    let eof = diag("E0101", 5, 5, "hết tệp đột ngột");
    let expected_eof = "\
error[E0101]: hết tệp đột ngột
 --> t.gt:2:3
  |
2 | cd
  |   ^
";
    assert_eq!(render_human(&eof, &f, &plain()), expected_eof);
}

#[test]
fn multi_line_span() {
    let f = file("a\nbb\nccc\n");
    let d = diag("E0305", 0, 7, "khối").with_span_label("ở đây");
    let expected = "\
error[E0305]: khối
 --> t.gt:1:1
  |
1 | a
  | ^
2 | bb
  | ^^
3 | ccc
  | ^^ ở đây
";
    assert_eq!(render_human(&d, &f, &plain()), expected);
}

#[test]
fn very_long_span_is_elided() {
    let f = file("1\n2\n3\n4\n5\n6\n7\n");
    // span phủ dòng 1..6
    let d = diag("E0305", 0, 11, "dài");
    let out = render_human(&d, &f, &plain());
    assert!(out.contains("1 | 1\n"), "{out}");
    assert!(out.contains("2 | 2\n"), "{out}");
    assert!(out.contains("...\n"), "{out}");
    assert!(out.contains("6 | 6\n"), "{out}");
    assert!(
        !out.contains("3 | 3\n") && !out.contains("4 | 4\n"),
        "{out}"
    );
}

#[test]
fn line_number_width_grows() {
    let mut text = String::new();
    for i in 1..=12 {
        text.push_str(&format!("l{i}\n"));
    }
    let f = file(&text);
    // dòng 10 bắt đầu sau 9 dòng "lN\n" (3 byte mỗi dòng) = byte 27
    let d = diag("E0101", 27, 29, "x");
    let expected = "\
error[E0101]: x
  --> t.gt:10:1
   |
10 | l10
   | ^^
";
    assert_eq!(render_human(&d, &f, &plain()), expected);
}

#[test]
fn crlf_and_tabs_render_cleanly() {
    let f = file("a\r\n\tb\r\n");
    // 'b' ở byte 4 (a=0, \r=1, \n=2, \t=3, b=4), col 2 trên dòng 2; tab hiển thị thành 1 dấu cách
    let d = diag("E0101", 4, 5, "b");
    let expected = "\
error[E0101]: b
 --> t.gt:2:2
  |
2 |  b
  |  ^
";
    assert_eq!(render_human(&d, &f, &plain()), expected);
}

#[test]
fn levels_notes_secondary_labels_and_suggestions() {
    let f = file("let a;\nlet b;\n");
    let warn = diag("S101", 4, 5, "khoá nằm trong nguồn");
    assert!(render_human(&warn, &f, &plain()).starts_with("warn[S101]: khoá nằm trong nguồn\n"));
    let note = diag("N0601", 4, 5, "check tràn hạ thành runtime");
    assert!(
        render_human(&note, &f, &plain()).starts_with("note[N0601]: check tràn hạ thành runtime\n")
    );

    let d = diag("E0401", 11, 12, "dùng sau khi move")
        .with_span_label("dùng ở đây")
        .with_label("t.gt", Span::new(4, 5), "đã move ở đây")
        .with_note("giá trị không phải Copy")
        .with_suggestion(Suggestion::new(
            "bọc bằng clone",
            Applicability::MaybeIncorrect,
            vec![Edit::new(Span::new(11, 12), "b.clone()")],
        ));
    let expected = "\
error[E0401]: dùng sau khi move
 --> t.gt:2:5
  |
1 | let a;
  |     - đã move ở đây
2 | let b;
  |     ^ dùng ở đây
  = note: giá trị không phải Copy
  = help: bọc bằng clone (maybe-incorrect)
          - 2:5-2:6: `b` -> `b.clone()`
";
    assert_eq!(render_human(&d, &f, &plain()), expected);
}

#[test]
fn out_of_range_span_does_not_panic() {
    let f = file("abc");
    let d = diag("E0101", 50, 60, "span hỏng");
    let out = render_human(&d, &f, &plain());
    assert!(out.starts_with("error[E0101]: span hỏng\n"), "{out}");
    assert!(out.contains("span ngoài tệp"), "{out}");
}

#[test]
fn color_only_adds_ansi() {
    let f = file("ƒ f( { 1 }\n");
    let d = diag("E0101", 6, 7, "m").with_span_label("l").with_note("n");
    let plain_out = render_human(&d, &f, &plain());
    let color_out = render_human(&d, &f, &RenderOptions { color: true });
    assert!(!plain_out.contains('\u{1b}'));
    assert!(color_out.contains("\u{1b}[1;31m"), "ĐỎ dùng đỏ đậm");
    // bỏ ANSI thì phải y hệt bản thuần (cấu trúc không đổi)
    let mut stripped = String::new();
    let mut chars = color_out.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for n in chars.by_ref() {
                if n == 'm' {
                    break;
                }
            }
        } else {
            stripped.push(c);
        }
    }
    assert_eq!(stripped, plain_out);
    // vàng và trắng
    let w = diag("S101", 0, 1, "w");
    assert!(render_human(&w, &f, &RenderOptions { color: true }).contains("\u{1b}[1;33m"));
    let n = diag("N0601", 0, 1, "n");
    assert!(render_human(&n, &f, &RenderOptions { color: true }).contains("\u{1b}[1;37m"));
}

#[test]
fn rendering_is_deterministic() {
    let f = file("ƒ f( { 1 }\n");
    let d = diag("E0101", 6, 7, "m").with_span_label("l");
    assert_eq!(
        render_human(&d, &f, &plain()),
        render_human(&d, &f, &plain())
    );
}

#[test]
fn span_ending_at_line_start_does_not_show_next_line() {
    let f = file("a\nbb\n");
    // span [0,2) phủ "a\n": kết thúc đúng đầu dòng 2 → chỉ hiển thị dòng 1
    let d = diag("E0101", 0, 2, "m");
    let expected = "\
error[E0101]: m
 --> t.gt:1:1
  |
1 | a
  | ^
";
    assert_eq!(render_human(&d, &f, &plain()), expected);
}

// ---------- nhiều tệp (ADR-017a) ----------

fn named(name: &str, text: &str) -> SourceFile {
    SourceFile::new(name, text.to_string()).expect("nhỏ")
}

fn cross_file() -> (Diagnostic, SourceFile, SourceFile) {
    let b = named("b.gt", "ƒ g() ! ∅ { a::secret(); }\n");
    let a = named("a.gt", "ƒ secret() ! ∅ { }\n");
    let u = b.text().find("secret").expect("secret") as u32;
    let d0 = a.text().find("secret").expect("secret") as u32;
    let d = Diagnostic::new(
        Code::parse("E0203").expect("mã"),
        "b.gt",
        Span::new(u, u + 6),
        "`secret` là private",
    )
    .with_span_label("private")
    .with_label("a.gt", Span::new(d0, d0 + 6), "định nghĩa ở đây")
    .with_suggestion(Suggestion::new(
        "thêm `◈` trước định nghĩa",
        Applicability::MaybeIncorrect,
        vec![Edit {
            file: "a.gt".to_string(),
            span: Span::new(0, 0),
            replacement: "◈ ".to_string(),
        }],
    ));
    (d, b, a)
}

#[test]
fn a_label_in_another_file_is_drawn_as_its_own_snippet_block() {
    let (d, b, a) = cross_file();
    let out = gt_diag::render_human_files(&d, &[&b, &a], &plain());
    assert_eq!(
        out,
        "error[E0203]: `secret` là private\n --> b.gt:1:16\n  |\n1 | ƒ g() ! ∅ { a::secret(); }\n  |                ^^^^^^ private\n ::: a.gt:1:3\n  |\n1 | ƒ secret() ! ∅ { }\n  |   ------ định nghĩa ở đây\n  = help: thêm `◈` trước định nghĩa (maybe-incorrect)\n          - 1:1-1:1: `` -> `◈ `\n"
    );
}

#[test]
fn a_label_in_a_file_that_is_not_provided_is_listed_as_text_not_dropped() {
    let (d, b, _a) = cross_file();
    let out = gt_diag::render_human_files(&d, &[&b], &plain());
    assert!(out.contains("= label: định nghĩa ở đây (a.gt)"), "{out}");
    assert!(
        out.contains("a.gt"),
        "edit ở tệp thiếu vẫn được nhắc: {out}"
    );
    assert!(!out.contains(":::"));
}

#[test]
fn single_file_rendering_is_unchanged_by_the_multi_file_renderer() {
    let f = file("ƒ f( { 1 }\n");
    let d = diag("E0101", 4, 5, "mong đợi `)`");
    assert_eq!(
        render_human(&d, &f, &plain()),
        gt_diag::render_human_files(&d, &[&f], &plain())
    );
}
