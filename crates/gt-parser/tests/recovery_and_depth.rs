//! Khôi phục lỗi (luôn tiến về phía trước) và giới hạn độ sâu. Viết TRƯỚC khi có parser (TDD).
mod common;
use common::*;
use gt_ast::*;

#[test]
fn recovers_and_reports_many_errors_in_one_run() {
    let src = "\
ƒ a() → i32 ! ∅ { x ≔ ; 1 }
ƒ b() → i32 ! ∅ { 2 }
ƒ c() → i32 ! ∅ { y ≔ 1 z; 3 }
ƒ d() → i32 ! ∅ { ⏎ ; }
";
    let out = parse_src(src);
    let c = codes(&out);
    assert!(c.len() >= 2, "{:?}", out.report.diagnostics);
    assert!(c.iter().all(|x| x == "E0101"));
    // cả bốn hàm vẫn được parse (khôi phục trong khối), và hàm b nguyên vẹn
    assert_eq!(out.program.items.len(), 4);
    let ItemKind::Fn(b) = &out.program.items[1].kind else {
        panic!()
    };
    assert_eq!(sx(b.body.tail.as_ref().expect("tail")), "2");
    // mỗi lỗi có vị trí riêng, đúng thứ tự
    let starts: Vec<u32> = out
        .report
        .diagnostics
        .iter()
        .map(|d| d.span.start)
        .collect();
    assert!(starts.windows(2).all(|w| w[0] <= w[1]));
}

#[test]
fn resynchronizes_at_semicolon_and_closing_brace() {
    // lỗi giữa câu lệnh: bỏ tới `;` rồi tiếp tục; câu lệnh sau vẫn được parse
    let out = parse_src("ƒ f() → i32 ! ∅ { a ≔ ; b ≔ 2; b }");
    assert_eq!(codes(&out), ["E0101"]);
    let ItemKind::Fn(f) = &out.program.items[0].kind else {
        panic!()
    };
    assert_eq!(sblock(&f.body), "{(let b 2) tail:b}");
    // lỗi trong ngoặc lồng: đồng bộ ở `}` đóng của khối, không nuốt phần còn lại của tệp
    let out = parse_src("ƒ f() → i32 ! ∅ { g(1, ; } ƒ ok() → i32 ! ∅ { 1 }");
    assert!(!codes(&out).is_empty());
    assert!(
        out.program
            .items
            .iter()
            .any(|i| matches!(&i.kind, ItemKind::Fn(f) if f.sig.name.name == "ok"))
    );
}

#[test]
fn recovers_at_item_level() {
    // tiêu đề item hỏng, `}` lạc, rồi item hợp lệ
    let src = "ƒ ( ) { }\n}\nƒ ok() → i32 ! ∅ { 1 }\n";
    let out = parse_src(src);
    assert!(!codes(&out).is_empty());
    assert!(
        out.program
            .items
            .iter()
            .any(|i| matches!(&i.kind, ItemKind::Fn(f) if f.sig.name.name == "ok"))
    );
    // thiếu `}` ở cuối tệp: đúng một lỗi
    let out = parse_src("ƒ f() → i32 ! ∅ { 1 + 2");
    assert_eq!(codes(&out), ["E0101"]);
}

#[test]
fn match_arm_recovery() {
    let out = parse_src("ƒ f(x: i32) → i32 ! ∅ { ⌥ x { 1 ↦ ↦ 2, 3 ↦ 4 } }");
    assert!(!codes(&out).is_empty());
    assert_eq!(out.program.items.len(), 1);
}

#[test]
fn lexer_errors_become_diagnostics_without_duplicates() {
    // zero-width (E1202) và ký tự lạ (E0101) đến từ lexer; parser không báo thêm lỗi trùng ở token UNKNOWN
    let out = parse_src("ƒ f() → i32 ! ∅ { 1\u{200B} }");
    assert_eq!(codes(&out), ["E1202"]);
    let out = parse_src("ƒ f() → i32 ! ∅ { 1 @ 2 }");
    assert!(codes(&out).iter().all(|c| c == "E0101"));
    let spans: Vec<_> = out
        .report
        .diagnostics
        .iter()
        .map(|d| (d.span.start, d.span.end))
        .collect();
    let mut dedup = spans.clone();
    dedup.dedup();
    assert_eq!(spans, dedup);
    // NFC, bidi, homoglyph
    assert_eq!(
        codes(&parse_src("// \u{202E}\nƒ f() → i32 ! ∅ { 1 }")),
        ["E1201"]
    );
    assert_eq!(
        codes(&parse_src("ƒ f() → i32 ! ∅ { x\u{0430} ≔ 1; 1 }")),
        ["E1203"]
    );
    assert_eq!(
        codes(&parse_src("ƒ f() → i32 ! ∅ { cafe\u{0301} ≔ 1; 1 }")),
        ["E1204"]
    );
}

/// Mọi chẩn đoán có span hợp lệ (trong tệp, trên ranh giới ký tự) — dùng cho test độ bền.
fn assert_sane(src: &str) -> usize {
    let out = parse_src(src);
    for d in &out.report.diagnostics {
        assert!(
            d.span.start <= d.span.end && d.span.end as usize <= src.len(),
            "{src:?}: {:?}",
            d.span
        );
        assert!(
            src.is_char_boundary(d.span.start as usize)
                && src.is_char_boundary(d.span.end as usize),
            "{src:?}"
        );
        assert!(
            !d.code.is_ice(),
            "{src:?}: ICE (panic nội bộ bị bắt): {}",
            d.message
        );
        assert!(d.code.is_known(), "{src:?}: mã lạ {}", d.code);
    }
    out.work
}

/// Tính chất "luôn tiến về phía trước": chuỗi token ngẫu nhiên không bao giờ làm parser lặp vô hạn hay làm việc siêu tuyến tính.
/// Công việc (số lần gọi production) bị chặn theo độ dài đầu vào; nếu một vòng recovery không tiêu thụ token, test treo/vượt ngưỡng.
#[test]
fn parser_always_makes_progress_on_random_token_soup() {
    let atoms: Vec<&str> = vec![
        "ƒ", "f", "(", ")", "{", "}", "[", "]", ";", ",", ":", "::", ".", "..", "..=", "→", "!",
        "∅", "≔", "←", "μ", "x", "y", "1", "2.5", "\"s\"", "'a'", "'a", "+", "-", "*", "/", "&",
        "&!", "<", ">", "==", "≠", "∧", "∨", "¬", "⇒", "∈", "⎇", "⇝", "⌥", "↦", "⟳", "↓", "⋈",
        "in", "⚠", "⊢", "▢", "⊔", "⊏", "⊐", "▦", "⇲", "◈", "≡", "□", "■", "#", "⏎", "⊠", "⊡", "_",
        "self", "Self", "as", "for", "true", "⊳", "⊲", "/// d\n", "// c\n", "@", "=", "\\extern",
        "\\where", "∀", "∃", "\n", " ",
    ];
    let mut state: u64 = 0x1234_5678_9ABC_DEF1;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut worst = 0usize;
    for _ in 0..30_000 {
        let n = (next() % 60) as usize;
        let mut src = String::new();
        for _ in 0..n {
            src.push_str(atoms[(next() % atoms.len() as u64) as usize]);
            src.push(' ');
        }
        let work = assert_sane(&src);
        let bound = 300 * (src.len() + 1);
        assert!(work <= bound, "{src:?}: work {work} > {bound}");
        worst = worst.max(work);
    }
    assert!(worst > 0);
}

#[test]
fn truncated_valid_programs_never_hang() {
    // cắt chương trình hợp lệ ở MỌI vị trí byte (trên ranh giới ký tự): luôn kết thúc, chẩn đoán hợp lệ
    let full = "▢ P { x: i32 }\nƒ f(a: i32, μ b: &[i32]) → i32 ! {io} ⊳ a > 0 {\n  μs ≔ 0;\n  ⟳ x in b ↓ 3 { s ← s + *x; }\n  ⌥ a { 0 ↦ 1, _ ↦ 2 }\n}\n";
    for (i, _) in full.char_indices() {
        assert_sane(&full[..i]);
    }
}

// ---------- độ sâu ----------

fn nested(open: &str, core: &str, close: &str, n: usize) -> String {
    format!(
        "ƒ f() → i32 ! ∅ {{ {}{core}{} }}",
        open.repeat(n),
        close.repeat(n)
    )
}

#[test]
fn depth_limit_is_a_red_error_not_a_crash() {
    // trong giới hạn: qua
    assert!(codes(&parse_src(&nested("(", "1", ")", 100))).is_empty());
    assert!(codes(&parse_src(&nested("{ ", "1", " }", 100))).is_empty());
    // vượt giới hạn: ĐỎ E0102, không tràn stack, không lỗi E0101 rác
    for src in [
        nested("(", "1", ")", 300),
        nested("{ ", "1", " }", 300),
        nested("[", "1", "]", 300),
        nested("¬", "a", "", 300),
        nested("-(", "a", ")", 300),
        format!("ƒ f(p: {}i32) ! ∅ {{ }}", "&".repeat(300)),
        format!(
            "ƒ f(p: {}i32{}) ! ∅ {{ }}",
            "(".repeat(300),
            ")".repeat(300)
        ),
        format!(
            "ƒ f() → i32 ! ∅ {{ ⌥ x {{ {}a{} ↦ 1 }} }}",
            "(".repeat(300),
            ")".repeat(300)
        ),
        format!(
            "ƒ f() → i32 ! ∅ {{ {}1 ⇝ {{ 0 }}{} }}",
            "⎇ c { 1 } ⇝ ".repeat(300),
            ""
        ),
    ] {
        let c = codes(&parse_src(&src));
        assert_eq!(c, ["E0102"], "{}", &src[..src.len().min(60)]);
    }
    // cực đoan: 200 000 mức không được làm tràn stack
    let c = codes(&parse_src(&nested("(", "1", ")", 200_000)));
    assert_eq!(c, ["E0102"]);
}

#[test]
fn flat_chains_count_toward_depth() {
    // Q15e: chuỗi phẳng cũng tạo cây lệch trái sâu
    let chain = |n: usize| format!("ƒ f() → i32 ! ∅ {{ {} }}", vec!["1"; n].join(" + "));
    assert!(codes(&parse_src(&chain(100))).is_empty());
    assert_eq!(codes(&parse_src(&chain(300))), ["E0102"]);
    assert_eq!(codes(&parse_src(&chain(50_000))), ["E0102"]);
    let postfix = |n: usize| format!("ƒ f() → i32 ! ∅ {{ a{} }}", ".b".repeat(n));
    assert!(codes(&parse_src(&postfix(100))).is_empty());
    assert_eq!(codes(&parse_src(&postfix(300))), ["E0102"]);
    let calls = |n: usize| format!("ƒ f() → i32 ! ∅ {{ a{} }}", "(1)".repeat(n));
    assert_eq!(codes(&parse_src(&calls(300))), ["E0102"]);
    let casts = |n: usize| format!("ƒ f() → i32 ! ∅ {{ a{} }}", " as u8".repeat(n));
    assert_eq!(codes(&parse_src(&casts(300))), ["E0102"]);
}

#[test]
fn many_items_and_statements_are_flat_and_fine() {
    let many_stmts = format!("ƒ f() → i32 ! ∅ {{ {} 0 }}", "g(); ".repeat(5000));
    assert!(codes(&parse_src(&many_stmts)).is_empty());
    let many_items = "ƒ f() ! ∅ { }\n".repeat(3000);
    let out = parse_src(&many_items);
    assert!(codes(&out).is_empty());
    assert_eq!(out.program.items.len(), 3000);
}

#[test]
fn garbage_after_a_broken_item_is_reported_not_swallowed() {
    // item hỏng kết thúc ở `}`; `x` là rác mới và PHẢI có lỗi riêng, rồi item sau vẫn được parse
    let out = parse_src("ƒ ( ) { } x ƒ ok() → i32 ! ∅ { 1 }");
    assert!(
        out.report.diagnostics.len() >= 2,
        "{:?}",
        out.report.diagnostics
    );
    assert!(
        out.program
            .items
            .iter()
            .any(|i| matches!(&i.kind, ItemKind::Fn(f) if f.sig.name.name == "ok"))
    );
}
