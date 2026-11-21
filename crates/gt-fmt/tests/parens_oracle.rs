//! Thêm/bỏ ngoặc theo "thước đo độc lập": parser. Với mọi cặp (biểu thức con × ngữ cảnh):
//! viết hai nguồn — có ngoặc quanh biểu thức con (`with`) và không (`flat`).
//! - Nếu `flat` parse sạch VÀ cho đúng cùng AST với `with` → ngoặc thừa → fmt(`with`) phải bỏ ngoặc (== fmt(`flat`)).
//! - Ngược lại ngoặc là cần → fmt(`with`) phải GIỮ đúng số ngoặc, và đọc lại cho cùng AST.
//!
//! Không nhúng bảng ưu tiên của printer vào test: mọi sai lệch (độ ưu tiên, kết hợp, `as T <`, tuple-index sau literal…) bị phát hiện
//! bởi chính parser. Viết sau khi đột biến sống sót cho thấy bảng kiểm thủ công quá thưa (xem HISTORY.md).
mod common;
use common::*;

const BIN: &[&str] = &[
    "⇒", "∨", "∧", "==", "≠", "<", ">", "≤", "≥", "∈", "+", "-", "+%", "-%", "+^", "-^", "+?",
    "-?", "*", "/", "%", "*%", "*^", "*?",
];

/// Biểu thức con (không ngoặc ngoài). Toán hạng là path đơn để chỉ kiểm cấu trúc.
fn inners() -> Vec<String> {
    let mut v: Vec<String> = BIN.iter().map(|op| format!("a {op} b")).collect();
    for s in [
        "¬a",
        "-a",
        "*a",
        "&a",
        "&!a",
        "a as u8",
        "a as u8 as u16",
        "a..b",
        "a..=b",
        "..b",
        "a..",
        "a.f",
        "a.0",
        "a[i]",
        "f(a)",
        "-1",
        "1",
        "a ∧ b ∨ c",
        "a ⇒ b ⇒ c",
        "-(1)",
        "-a.f",
        "¬a ∧ b",
        "(a,)",
        "[a, b]",
        "a as u8 < b",
        "x.0.1",
    ] {
        v.push(s.to_string());
    }
    v
}

/// Ngữ cảnh: `X` là chỗ đặt biểu thức con.
fn contexts() -> Vec<String> {
    let mut v = Vec::new();
    for op in BIN {
        v.push(format!("X {op} c"));
        v.push(format!("c {op} X"));
    }
    for s in [
        "¬X",
        "-X",
        "*X",
        "&X",
        "&!X",
        "X as u8",
        "X.f",
        "X.0",
        "X[i]",
        "X(i)",
        "X..",
        "..X",
        "X..c",
        "c..X",
        "X..=c",
        "c..=X",
        "(X,)",
        "[X, c]",
        "f(X)",
        "X as u8 < c",
        "c < X",
        "¬X ∧ c",
    ] {
        v.push(s.to_string());
    }
    v
}

fn parses_clean(src: &str) -> bool {
    gt_parser::parse("t.gt", src).report.diagnostics.is_empty()
}

fn wrap(e: &str) -> String {
    format!("ƒ f() → i32 ! ∅ {{ {e} }}")
}

#[test]
fn parentheses_match_the_parser_oracle_for_every_operand_in_every_context() {
    let (mut kept, mut dropped) = (0u32, 0u32);
    for inner in inners() {
        for ctx in contexts() {
            let with = ctx.replacen('X', &format!("({inner})"), 1);
            let flat = ctx.replacen('X', &inner, 1);
            let (ws, fs) = (wrap(&with), wrap(&flat));
            if !parses_clean(&ws) {
                continue; // tổ hợp không hợp lệ về mặt cú pháp/ngữ nghĩa mặt chữ (vd `a.0` sau literal) → không phải ca kiểm
            }
            let out = fe(&with);
            let redundant = parses_clean(&fs) && ast_of(&fs) == ast_of(&ws);
            if redundant {
                assert_eq!(out, fe(&flat), "ngoặc thừa phải được bỏ: {with}");
                // (`-(-1)` luôn in dạng chuẩn `-(-1)` — quy tắc Neg-literal, ADR-014 — nên không đếm ngoặc ca đó)
                if !out.contains("-(") {
                    assert_eq!(
                        out.matches('(').count(),
                        flat.matches('(').count(),
                        "{with} → {out}"
                    );
                }
                dropped += 1;
            } else {
                assert_eq!(
                    ast_of(&wrap(&out)),
                    ast_of(&ws),
                    "in ra đọc lại khác AST: {with} → {out}"
                );
                assert_eq!(
                    out.matches('(').count(),
                    with.matches('(').count(),
                    "ngoặc cần thiết phải giữ đúng (không thêm, không bớt): {with} → {out}"
                );
                kept += 1;
            }
            assert_eq!(fe(&out), out, "không idempotent: {with}");
        }
    }
    // test phải thực sự kiểm cả hai nhánh, nhiều ca
    assert!(kept > 500 && dropped > 500, "kept={kept} dropped={dropped}");
}

/// `ns` (cấm struct literal trần) phải đi qua mọi toán hạng, nhưng được đặt lại trong ngoặc/ngoặc vuông/đối số.
#[test]
fn bare_struct_literal_restriction_reaches_every_operand_and_resets_inside_brackets() {
    let s = "P { x: 1 }";
    let cases = [
        // (nguồn, kỳ vọng phần điều kiện sau `⎇ `)
        (
            format!("⎇ ({s}) == c {{ 1 }} ⇝ {{ 2 }}"),
            format!("({s}) == c"),
        ),
        (
            format!("⎇ c == ({s}) {{ 1 }} ⇝ {{ 2 }}"),
            format!("c == ({s})"),
        ),
        (format!("⎇ ¬({s}).b {{ 1 }} ⇝ {{ 2 }}"), format!("¬({s}).b")),
        (format!("⎇ -({s}).b {{ 1 }} ⇝ {{ 2 }}"), format!("-({s}).b")),
        (
            format!("⎇ ({s}).b[0] {{ 1 }} ⇝ {{ 2 }}"),
            format!("({s}).b[0]"),
        ),
        (
            format!("⎇ ({s}) as u8 == c {{ 1 }} ⇝ {{ 2 }}"),
            format!("({s}) as u8 == c"),
        ),
        (
            format!("⎇ c ∧ ({s}).b {{ 1 }} ⇝ {{ 2 }}"),
            format!("c ∧ ({s}).b"),
        ),
        (format!("⎇ f({s}) {{ 1 }} ⇝ {{ 2 }}"), format!("f({s})")), // đối số: ns được đặt lại
        (format!("⎇ ({s}) {{ 1 }} ⇝ {{ 2 }}"), format!("({s})")),
        (
            format!("⎇ ¬(a ∨ {s}.b) {{ 1 }} ⇝ {{ 2 }}"),
            format!("¬(a ∨ {s}.b)"),
        ), // trong ngoặc: ns đặt lại
        (format!("⎇ [{s}][0] {{ 1 }} ⇝ {{ 2 }}"), format!("[{s}][0]")),
        (format!("⎇ c[{s}.b] {{ 1 }} ⇝ {{ 2 }}"), format!("c[{s}.b]")),
        (format!("⎇ ({s},).0 {{ 1 }} ⇝ {{ 2 }}"), format!("({s},).0")),
        (
            "⎇ 0..({ 1 }) { 1 } ⇝ { 2 }".to_string(),
            "0..({ 1 })".to_string(),
        ),
    ];
    for (src, want) in cases {
        let out = fe(&src);
        assert_eq!(out, format!("⎇ {want} {{ 1 }} ⇝ {{ 2 }}"), "{src}");
        assert_eq!(ast_of(&wrap(&out)), ast_of(&wrap(&src)), "{src}");
    }
}

/// Hợp đồng của thành viên trait (nhánh code riêng với hợp đồng của hàm có thân).
#[test]
fn struct_literal_in_a_trait_member_contract_is_parenthesized() {
    assert_eq!(
        fmt_ok("⊏ T { ƒ f() → bool ! ∅ ⊳ (P { x: 1 }).x == 1; }"),
        "⊏ T {
  ƒ f() → bool ! ∅ ⊳ (P { x: 1 }).x == 1;
}
"
    );
    assert_eq!(
        fmt_ok("⊏ T { ƒ f(p: P) → bool ! ∅ ⊳ g(P { x: 1 }); }"),
        "⊏ T {
  ƒ f(p: P) → bool ! ∅ ⊳ g(P { x: 1 });
}
"
    );
}
