//! Độ bền: không panic (không ICE E9901), luôn kết thúc, tất định — trên chương trình ngẫu nhiên (kể cả bị phá cú pháp) và trên
//! cây rất sâu (độ sâu tối đa hợp lệ). Fuzz không bắt buộc ở M3 (yêu cầu của Giang); đây là test độ bền thay thế.
mod common;
use common::*;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const FRAGMENTS: &[&str] = &[
    "ƒ f() ! ∅ { }",
    "ƒ g(x: i32) → i32 ! ∅ { x }",
    "▢ P { a: i32 }",
    "⊔ E { A, B(i32) }",
    "⇲ a::f;",
    "⇲ a::{f, g};",
    "◈ ƒ h() ! ∅ { f(); a::g(); }",
    "▦ m { ƒ f() ! ∅ { } }",
    "⊐ P { ƒ n() → Self ! ∅ { Self { a: 1 } } }",
    "⊏ T { ƒ s(&self) ! ∅; }",
    "ƒ w(o: Option<i32>) → i32 ! ∅ { ⌥ o { Some(v) ↦ v, None ↦ 0 } }",
    "ƒ l() ! ∅ { x ≔ 1; ⟳ i in 0..3 { y ≔ i; } }",
    "ƒ z() ! ∅ { q ≔ undefined; }",
    "□ C: i32 ≔ 1;",
    "≡ A ≔ i32;",
];

fn random_source(r: &mut Rng) -> String {
    let n = 1 + r.below(8);
    let mut parts: Vec<String> = (0..n)
        .map(|_| FRAGMENTS[r.below(FRAGMENTS.len())].to_string())
        .collect();
    // đôi khi phá hỏng văn bản: xoá/cắt/lặp ký tự (chương trình có lỗi cú pháp)
    if r.below(3) == 0 {
        let i = r.below(parts.len());
        let chars: Vec<char> = parts[i].chars().collect();
        if !chars.is_empty() {
            let cut = r.below(chars.len());
            parts[i] = chars[..cut].iter().collect();
        }
    }
    parts.join("\n")
}

#[test]
fn random_programs_never_panic_terminate_and_are_deterministic() {
    let mut r = Rng(0xA076_1D64_78BD_642F);
    let (mut clean, mut broken) = (0, 0);
    for _ in 0..1500 {
        let nfiles = 1 + r.below(3);
        let files: Vec<(String, String)> = (0..nfiles)
            .map(|i| (format!("{}.gt", ["a", "b", "c"][i]), random_source(&mut r)))
            .collect();
        let refs: Vec<(&str, &str)> = files
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        let a = run(&refs);
        let b = run(&refs);
        assert!(
            a.report.diagnostics.iter().all(|d| !d.code.is_ice()),
            "ICE: {:?}\n{files:?}",
            a.report.diagnostics
        );
        assert_eq!(a.canonical(), b.canonical());
        assert_eq!(a.canonical_diagnostics(), b.canonical_diagnostics());
        if a.report.diagnostics.is_empty() {
            clean += 1;
        } else {
            broken += 1;
        }
    }
    assert!(clean > 50 && broken > 50, "sạch={clean} lỗi={broken}");
}

#[test]
fn the_deepest_valid_nesting_is_resolved_without_overflowing_the_stack() {
    // biểu thức lồng 250 mức: parser chấp nhận, resolver (đệ quy trên AST) PHẢI chạy trong luồng stack lớn (ADR-015)
    let mut e = String::from("x");
    for _ in 0..120 {
        e = format!("({e} + x)");
    }
    let src = format!("ƒ f(x: i32) → i32 ! ∅ {{ {e} }}");
    assert_ok(&[("main.gt", &src)]);
    // khối lồng sâu
    let mut b = String::from("x");
    for _ in 0..100 {
        b = format!("{{ {b} }}");
    }
    let src = format!("ƒ f(x: i32) → i32 ! ∅ {b}");
    assert_ok(&[("main.gt", &src)]);
    // cùng tên ở các khối lồng: E0205 đúng một lần mỗi tên mới, không tràn
    let mut s = String::new();
    for _ in 0..100 {
        s = format!("{{ x ≔ 1; {s} }}");
    }
    let src = format!("ƒ f() ! ∅ {s}");
    let out = one(&src);
    assert_eq!(codes(&out).len(), 99);
}

#[test]
fn syntax_errors_stop_resolution_and_report_only_parser_diagnostics() {
    let out = run(&[
        ("a.gt", "ƒ f() ! ∅ { 1 + }"),
        ("b.gt", "ƒ g() ! ∅ { nope(); }"),
    ]);
    // chỉ có chẩn đoán của parser (E0101), không có E0201 từ tệp b: AST không hoàn chỉnh thì không phân giải (fail-closed)
    assert_eq!(codes(&out), ["E0101"]);
    assert!(out.defs.is_empty());
}

#[test]
fn invalid_inputs_are_rejected_up_front() {
    use gt_resolve::{InputError, SourceInput, resolve_program};
    let mk = |n: &str| SourceInput {
        name: n.to_string(),
        text: String::new(),
    };
    let p = prelude();
    assert_eq!(
        resolve_program(&[mk("a.gt"), mk("a.gt")], &p).err(),
        Some(InputError::DuplicateFileName("a.gt".to_string()))
    );
    assert!(matches!(
        resolve_program(&[mk("a-b.gt")], &p),
        Err(InputError::BadModuleName(_))
    ));
    assert!(matches!(
        resolve_program(&[mk("1a.gt")], &p),
        Err(InputError::BadModuleName(_))
    ));
    assert!(matches!(
        resolve_program(&[mk("noext")], &p),
        Err(InputError::BadModuleName(_))
    ));
    // tên module là từ khoá cứng cũng không hợp lệ
    assert!(matches!(
        resolve_program(&[mk("self.gt")], &p),
        Err(InputError::BadModuleName(_))
    ));
    // rỗng hợp lệ
    assert!(resolve_program(&[], &p).is_ok());
}

/// G1: `module_file_name_for` luôn trả tên mà `resolve_program` chấp nhận; tên đã hợp lệ giữ nguyên.
#[test]
fn module_file_name_for_always_yields_an_accepted_name() {
    use gt_resolve::{SourceInput, module_file_name_for, resolve_program};
    for (input, want) in [
        ("main.gt", Some("main.gt")),
        ("plain.gt", Some("plain.gt")),
        ("01_add.gt", Some("m_01_add.gt")),
        ("my-prog.gt", Some("my_prog.gt")),
        ("in.gt", Some("in_.gt")),
        ("self.gt", Some("self_.gt")),
        (".gt", Some("m_.gt")),
        ("x y.gt", Some("x_y.gt")),
        ("noext", Some("noext.gt")),
        ("tên.gt", None),
    ] {
        let got = module_file_name_for(input);
        if let Some(w) = want {
            assert_eq!(got, w, "{input}");
        }
        let r = resolve_program(
            &[SourceInput {
                name: got.clone(),
                text: String::new(),
            }],
            &prelude(),
        );
        assert!(r.is_ok(), "{input} -> {got}: {:?}", r.err());
    }
}
