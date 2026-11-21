//! Hai bề mặt (canonical Unicode và alias ASCII) cho CÙNG AST, và trivia đi kèm AST. Việc "mọi mẫu corpus pass/fail đúng mã" đã chuyển sang
//! `crates/gt-driver/tests/corpus_e2e.rs` (GÓI 12): E2E qua `gritc`, không phụ thuộc API parser.
mod common;
use common::*;
use std::path::Path;

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
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
