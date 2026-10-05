//! Chạy parser THẬT trên corpus `tests/`: 30 file pass phải parse không lỗi; file fail thuộc phạm vi parser phải ra ĐÚNG mã;
//! file fail thuộc milestone sau (kiểu, borrow, …) phải PARSE ĐƯỢC (parser không báo thay phần của milestone sau).
mod common;
use common::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn files(sub: &str) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join(sub);
    let mut v: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("{}: {e}", root.display()))
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .collect();
    v.sort();
    assert!(!v.is_empty());
    v
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn name(p: &Path) -> String {
    p.file_name().expect("tên").to_string_lossy().into_owned()
}

#[test]
fn all_pass_files_parse_without_any_diagnostic() {
    let list = files("pass");
    assert!(list.len() >= 30, "corpus pass có {} file", list.len());
    let mut failures = vec![];
    for p in &list {
        let out = parse_src(&read(p));
        if !out.report.diagnostics.is_empty() {
            let d = &out.report.diagnostics[0];
            failures.push(format!(
                "{}: {} {} @{}..{}",
                name(p),
                d.code,
                d.message,
                d.span.start,
                d.span.end
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "file pass bị parser chặn:\n{}",
        failures.join("\n")
    );
}

#[test]
fn warn_files_parse_without_diagnostic() {
    for p in files("warn") {
        let out = parse_src(&read(&p));
        assert!(
            out.report.diagnostics.is_empty(),
            "{}: {:?}",
            name(&p),
            out.report.diagnostics
        );
    }
}

/// Mã mà tầng lexer+parser sở hữu. Mọi mã khác thuộc milestone sau.
const OWNED: &[&str] = &["E0101", "E0102", "E1201", "E1202", "E1203", "E1204"];

#[test]
fn fail_files_get_exactly_the_expected_code_from_the_parser_or_parse_cleanly() {
    let mut failures = vec![];
    let mut owned_checked = 0;
    let mut later_checked = 0;
    for p in files("fail") {
        let src = read(&p);
        let header = src.lines().next().unwrap_or_default().to_string();
        let expected = header
            .split_whitespace()
            .nth(3)
            .unwrap_or_default()
            .to_string();
        let out = parse_src(&src);
        let got: BTreeSet<String> = codes(&out).into_iter().collect();
        if OWNED.contains(&expected.as_str()) {
            owned_checked += 1;
            if got != BTreeSet::from([expected.clone()]) {
                failures.push(format!("{}: mong đợi {{{expected}}}, có {got:?}", name(&p)));
            }
        } else {
            later_checked += 1;
            if !got.is_empty() {
                failures.push(format!(
                    "{}: thuộc milestone sau ({expected}) nhưng parser báo {got:?}: {:?}",
                    name(&p),
                    out.report.diagnostics[0].message
                ));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    assert!(
        owned_checked >= 15 && later_checked >= 20,
        "owned {owned_checked}, later {later_checked}"
    );
}

/// Hai bề mặt cho cùng AST (không tính vị trí): ASCII alias và canonical.
#[test]
fn alias_and_canonical_parse_to_the_same_ast() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/pass");
    let a = parse_src(&read(&root.join("01_add.gt")));
    let b = parse_src(&read(&root.join("11_ascii_alias.gt")));
    assert!(a.report.diagnostics.is_empty() && b.report.diagnostics.is_empty());
    assert_eq!(
        mask_spans(&format!("{:?}", a.program)),
        mask_spans(&format!("{:?}", b.program))
    );
    assert_eq!(a.node_count, b.node_count);
    // và với các cặp viết tay
    let pairs = [
        (
            "ƒ f(μ x: i32) → i32 ! ∅ ⊳ x ≥ 0 ∧ x ≠ 3 { ⟳ x < 3 ↓ 3 - x { x ← x + 1; } ⎇ x ≤ 2 { 1 } ⇝ { 2 } }",
            "\\fn f(\\mut x: i32) -> i32 ! \\nil \\pre x >= 0 \\land x != 3 { \\loop x < 3 \\dec 3 - x { x <- x + 1; } \\if x <= 2 { 1 } \\else { 2 } }",
        ),
        (
            "⊔ E { A(i32) } ƒ g(e: E) → i32 ! ∅ { ⌥ e { A(v) ↦ v } }",
            "\\enum E { A(i32) } \\fn g(e: E) -> i32 ! \\nil { \\match e { A(v) => v } }",
        ),
        (
            "▢ P { x: i32 } ⊐ Q for P { ƒ h(&self) ! ∅ { ⚠ ⊢ p { } } }",
            "\\struct P { x: i32 } \\impl Q for P { \\fn h(&self) ! \\nil { \\unsafe \\proof p { } } }",
        ),
    ];
    for (c, s) in pairs {
        let (x, y) = (parse_src(c), parse_src(s));
        assert!(
            x.report.diagnostics.is_empty(),
            "{c}: {:?}",
            x.report.diagnostics
        );
        assert!(
            y.report.diagnostics.is_empty(),
            "{s}: {:?}",
            y.report.diagnostics
        );
        assert_eq!(
            mask_spans(&format!("{:?}", x.program)),
            mask_spans(&format!("{:?}", y.program)),
            "{c}"
        );
    }
}

#[test]
fn trivia_is_returned_alongside_the_ast() {
    let out = parse_src("// a\nƒ f() ! ∅ { } // b\n");
    assert!(out.report.diagnostics.is_empty());
    assert_eq!(
        out.trivia
            .iter()
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>(),
        ["// a", "// b"]
    );
}
