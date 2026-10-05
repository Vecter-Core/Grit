//! LUẬT độ sâu/stack (semantics §26), không chỉ vài trường hợp: với MỖI loại lồng, độ sâu cực đại parse được
//! (ở bản DEBUG, tức bản tốn stack nhất) phải chạm sát `MAX_DEPTH` mà không ICE/tràn stack, và cực đại + 1 phải cho `E0102`.
mod common;
use common::*;
use gt_ast::MAX_DEPTH;

fn wrap(e: &str) -> String {
    format!("ƒ f() → i32 ! ∅ {{ {e} }}")
}

fn rep(s: &str, n: usize) -> String {
    s.repeat(n)
}

/// (tên, bộ sinh chương trình sâu `n` mức)
type Gen = Box<dyn Fn(usize) -> String>;

fn kinds() -> Vec<(&'static str, Gen)> {
    let mut v: Vec<(&'static str, Gen)> = Vec::new();
    macro_rules! k {
        ($name:expr, $f:expr) => {
            v.push(($name, Box::new($f)));
        };
    }
    // biểu thức lồng
    k!("expr/paren", |n| wrap(&format!(
        "{}1{}",
        rep("(", n),
        rep(")", n)
    )));
    k!("expr/array", |n| wrap(&format!(
        "{}1{}",
        rep("[", n),
        rep("]", n)
    )));
    k!("expr/tuple1", |n| wrap(&format!(
        "{}1{}",
        rep("(", n),
        rep(",)", n)
    )));
    k!("expr/not", |n| wrap(&format!("{}a", rep("¬", n))));
    k!("expr/neg", |n| wrap(&format!("{}a", rep("-", n))));
    k!("expr/deref", |n| wrap(&format!("{}p", rep("*", n))));
    k!("expr/borrow", |n| wrap(&format!("{}p", rep("&", n))));
    k!("expr/call-args", |n| wrap(&format!(
        "{}1{}",
        rep("f(", n),
        rep(")", n)
    )));
    k!("expr/index", |n| wrap(&format!(
        "{}1{}",
        rep("a[", n),
        rep("]", n)
    )));
    k!("expr/struct-lit", |n| wrap(&format!(
        "{}1{}",
        rep("P { x: ", n),
        rep(" }", n)
    )));
    k!("expr/quantifier", |n| wrap(&format!(
        "{}p",
        rep("∀ i ∈ r : ", n)
    )));
    // khối-biểu-thức lồng
    k!("block/expr", |n| wrap(&format!(
        "{}1{}",
        rep("{ ", n),
        rep(" }", n)
    )));
    k!("block/if-then", |n| wrap(&format!(
        "{}1{}",
        rep("⎇ c { ", n),
        rep(" } ⇝ { 0 }", n)
    )));
    k!("block/else-if-chain", |n| wrap(&format!(
        "{}{{ 0 }}",
        rep("⎇ c { 1 } ⇝ ", n)
    )));
    k!("block/match-arm", |n| wrap(&format!(
        "{}1{}",
        rep("⌥ x { _ ↦ ", n),
        rep(" }", n)
    )));
    k!("block/loop", |n| wrap(&format!(
        "{}1{}",
        rep("⟳ c { ", n),
        rep(" }", n)
    )));
    k!("block/unsafe", |n| wrap(&format!(
        "{}1{}",
        rep("⚠ { ", n),
        rep(" }", n)
    )));
    // chuỗi phẳng (cây lệch trái/phải)
    k!("flat/binary", |n| wrap(&vec!["1"; n + 1].join(" + ")));
    k!("flat/implies-right", |n| wrap(
        &vec!["a"; n + 1].join(" ⇒ ")
    ));
    k!("flat/postfix-field", |n| wrap(&format!(
        "a{}",
        rep(".b", n)
    )));
    k!("flat/postfix-call", |n| wrap(&format!(
        "a{}",
        rep("(1)", n)
    )));
    k!("flat/cast", |n| wrap(&format!("a{}", rep(" as u8", n))));
    // kiểu
    k!("type/ref", |n| format!(
        "ƒ f(p: {}i32) ! ∅ {{ }}",
        rep("&", n)
    ));
    k!("type/tuple1", |n| format!(
        "ƒ f(p: {}i32{}) ! ∅ {{ }}",
        rep("(", n),
        rep(",)", n)
    ));
    k!("type/paren", |n| format!(
        "ƒ f(p: {}i32{}) ! ∅ {{ }}",
        rep("(", n),
        rep(")", n)
    ));
    k!("type/array", |n| format!(
        "ƒ f(p: {}i32{}) ! ∅ {{ }}",
        rep("[", n),
        rep("; 1]", n)
    ));
    k!("type/generic", |n| format!(
        "ƒ f(p: {}i32{}) ! ∅ {{ }}",
        rep("A<", n),
        rep(">", n)
    ));
    k!("type/fn", |n| format!(
        "ƒ f(p: {}i32{}) ! ∅ {{ }}",
        rep("ƒ() → ", n),
        rep(" ! ∅", n)
    ));
    // pattern
    k!("pat/paren", |n| wrap(&format!(
        "⌥ x {{ {}y{} ↦ 1 }}",
        rep("(", n),
        rep(")", n)
    )));
    k!("pat/tuple1", |n| wrap(&format!(
        "⌥ x {{ {}y{} ↦ 1 }}",
        rep("(", n),
        rep(",)", n)
    )));
    k!("pat/variant", |n| wrap(&format!(
        "⌥ x {{ {}y{} ↦ 1 }}",
        rep("S(", n),
        rep(")", n)
    )));
    // module lồng
    k!("item/mod", |n| format!(
        "{}ƒ g() ! ∅ {{ }}{}",
        rep("▦ m { ", n),
        rep(" }", n)
    ));
    // trộn lẫn nhiều loại
    k!("mixed/expr", |n| {
        let opens = ["(", "[", "{ ", "f(", "⎇ c { ", "-", "¬"];
        let closes = [")", "]", " }", ")", " } ⇝ { 0 }", "", ""];
        let mut o = String::new();
        let mut c = Vec::new();
        for i in 0..n {
            o.push_str(opens[i % opens.len()]);
            c.push(closes[i % closes.len()]);
        }
        c.reverse();
        wrap(&format!("{o}1{}", c.concat()))
    });
    k!("mixed/type-pattern-expr", |n| {
        // xen kẽ: khối, match + pattern, ép kiểu sang kiểu lồng
        let mut s = String::new();
        let mut tail = String::new();
        for i in 0..n {
            match i % 3 {
                0 => {
                    s.push_str("{ ");
                    tail.insert_str(0, " }");
                }
                1 => {
                    s.push_str("⌥ x { (y) ↦ ");
                    tail.insert_str(0, " }");
                }
                _ => {
                    s.push('(');
                    tail.insert(0, ')');
                }
            }
        }
        wrap(&format!("{s}1{tail}"))
    });
    v
}

/// Chạy `n` mức; trả (số chẩn đoán, mã đầu tiên, max_depth).
fn run(src: &str) -> (usize, String, u32, bool) {
    let out = parse_src(src);
    let first = out
        .report
        .diagnostics
        .first()
        .map(|d| d.code.as_str().to_string())
        .unwrap_or_default();
    let ice = out.report.diagnostics.iter().any(|d| d.code.is_ice());
    (out.report.diagnostics.len(), first, out.max_depth, ice)
}

#[test]
fn depth_law_every_nesting_kind_reaches_the_limit_in_debug_without_crash() {
    let limit = MAX_DEPTH as usize;
    let mut summary = vec![];
    for (name, gen_src) in kinds() {
        // tìm n_ok: lớn nhất mà parse sạch; n_ok + 1 phải là E0102
        let mut n_ok = 0usize;
        let mut depth_at_ok = 0u32;
        let mut failed_at = None;
        for n in 1..=limit + 10 {
            let (count, first, depth, ice) = run(&gen_src(n));
            assert!(
                !ice,
                "{name} n={n}: ICE (panic nội bộ) — luật stack bị vi phạm"
            );
            if count == 0 {
                assert!(
                    failed_at.is_none(),
                    "{name}: không đơn điệu (n={n} parse được sau khi một n nhỏ hơn đã lỗi)"
                );
                n_ok = n;
                depth_at_ok = depth;
            } else if failed_at.is_none() {
                assert_eq!(
                    first, "E0102",
                    "{name} n={n}: lỗi đầu tiên phải là E0102, có {first} (bộ sinh sai?)"
                );
                assert_eq!(
                    count, 1,
                    "{name} n={n}: E0102 phải là chẩn đoán DUY NHẤT (dừng phân tích)"
                );
                failed_at = Some(n);
            }
        }
        assert!(n_ok >= 1, "{name}: không có độ sâu nào parse được");
        assert_eq!(
            failed_at,
            Some(n_ok + 1),
            "{name}: cực đại + 1 phải là E0102"
        );
        // độ sâu đã chạm sát giới hạn: bộ đếm phản ánh ĐÚNG mức stack được dùng
        assert!(
            depth_at_ok <= MAX_DEPTH && depth_at_ok + 6 >= MAX_DEPTH,
            "{name}: ở n_ok={n_ok} độ sâu chạm {depth_at_ok}, phải sát {MAX_DEPTH}"
        );
        summary.push(format!("{name}: n_ok={n_ok} depth={depth_at_ok}"));
    }
    assert!(summary.len() >= 30, "chỉ kiểm {} loại lồng", summary.len());
    println!("{}", summary.join("\n"));
}

#[test]
fn e0102_message_suggests_splitting_with_walrus() {
    let out = parse_src(&wrap(&format!("{}1{}", rep("(", 400), rep(")", 400))));
    let d = &out.report.diagnostics[0];
    assert_eq!(d.code.as_str(), "E0102");
    assert!(d.notes.iter().any(|n| n.contains('≔')), "{:?}", d.notes);
}

#[test]
fn max_depth_constant_lives_in_one_place() {
    // MAX_DEPTH chỉ được định nghĩa trong gt-ast; không crate nào khác tự đặt hằng giới hạn riêng
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut defs = vec![];
    for krate in std::fs::read_dir(&root).expect("crates/") {
        let src = krate.expect("entry").path().join("src");
        if !src.is_dir() {
            continue;
        }
        for f in std::fs::read_dir(&src).expect("src/") {
            let p = f.expect("entry").path();
            if p.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&p).expect("đọc");
                for (i, l) in text.lines().enumerate() {
                    if l.contains("const MAX_DEPTH") || l.contains("const PARSER_MAX") {
                        defs.push(format!("{}:{}", p.display(), i + 1));
                    }
                }
            }
        }
    }
    assert_eq!(
        defs.len(),
        1,
        "MAX_DEPTH phải được định nghĩa đúng MỘT chỗ: {defs:?}"
    );
    assert!(defs[0].contains("gt-ast"), "{defs:?}");
}

#[test]
fn deep_but_valid_ast_can_be_dropped_and_compared_on_the_default_test_stack() {
    // cây sâu sát giới hạn phải Drop/so sánh được trên stack mặc định của luồng gọi (2 MiB ở test)
    let src = wrap(&format!("{}1{}", rep("(", 200), rep(")", 200)));
    let a = parse_src(&src);
    let b = parse_src(&src);
    assert!(a.report.diagnostics.is_empty());
    assert_eq!(a.program, b.program);
    drop(a);
    drop(b);
}
