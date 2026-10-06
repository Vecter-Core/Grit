//! Bất biến của gt-fmt trên toàn corpus và trên biểu thức ngẫu nhiên: parse(fmt(parse(x))) == parse(x), fmt idempotent,
//! alias/canonical cho cùng đầu ra, comment được bảo toàn. Viết TRƯỚC (TDD).
mod common;
use common::*;
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
    v
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn name(p: &Path) -> String {
    p.file_name().expect("tên").to_string_lossy().into_owned()
}

/// Kiểm 4 bất biến trên một nguồn hợp lệ.
fn check_invariants(label: &str, src: &str) {
    let out = match gt_fmt::format("t.gt", src) {
        Ok(o) => o,
        Err(e) => panic!("{label}: gt-fmt từ chối: {:?}", e.report.diagnostics),
    };
    // 1. parse(fmt(parse(x))) == parse(x) (không tính span)
    assert_eq!(
        ast_of(&out),
        ast_of(src),
        "{label}: AST đổi sau fmt\n--- đầu ra ---\n{out}"
    );
    // 2. idempotent
    let again = fmt_ok(&out);
    assert_eq!(again, out, "{label}: không idempotent");
    // 3. comment: số lượng, nội dung, thứ tự
    assert_eq!(
        comment_texts(&out),
        comment_texts(src),
        "{label}: comment bị đổi"
    );
    // 4. đầu ra kết thúc bằng đúng một xuống dòng (hoặc rỗng)
    assert!(
        out.is_empty() || (out.ends_with('\n') && !out.ends_with("\n\n")),
        "{label}: kết thúc dòng sai"
    );
}

#[test]
fn every_pass_file_roundtrips_and_is_idempotent() {
    let list = files("pass");
    assert!(list.len() >= 30);
    for p in &list {
        check_invariants(&format!("pass/{}", name(p)), &read(p));
    }
}

#[test]
fn warn_files_and_parseable_fail_files_roundtrip_too() {
    let mut count = 0;
    for sub in ["warn", "fail"] {
        for p in files(sub) {
            let src = read(&p);
            if !gt_parser::parse("t.gt", &src).report.diagnostics.is_empty() {
                continue; // lỗi cú pháp: gt-fmt từ chối (kiểm riêng)
            }
            check_invariants(&format!("{sub}/{}", name(&p)), &src);
            count += 1;
        }
    }
    assert!(count >= 30, "chỉ kiểm {count} file");
}

#[test]
fn files_with_syntax_errors_are_refused() {
    let mut refused = 0;
    for p in files("fail") {
        let src = read(&p);
        if gt_parser::parse("t.gt", &src).report.diagnostics.is_empty() {
            continue;
        }
        assert!(
            gt_fmt::format("t.gt", &src).is_err(),
            "{}: đáng lẽ phải bị từ chối",
            name(&p)
        );
        refused += 1;
    }
    assert!(refused >= 15, "chỉ {refused} file lỗi cú pháp");
}

#[test]
fn alias_and_canonical_give_identical_output() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/pass");
    // cặp trong corpus: 01_add (canonical) và 11_ascii_alias — chỉ khác dòng ghi chú
    let a = fmt_ok(&strip_comment_lines(&read(&root.join("01_add.gt"))));
    let b = fmt_ok(&strip_comment_lines(&read(&root.join("11_ascii_alias.gt"))));
    assert_eq!(a, b);
    let pairs = [
        (
            "ƒ f(μ x: i32) → i32 ! ∅ ⊳ x ≥ 0 ∧ x ≠ 3 { ⟳ x < 3 ↓ 3 - x { x ← x + 1; } ⎇ x ≤ 2 { 1 } ⇝ { 2 } }",
            r"\fn f(\mut x: i32) -> i32 ! \nil \pre x >= 0 \land x != 3 { \loop x < 3 \dec 3 - x { x <- x + 1; } \if x <= 2 { 1 } \else { 2 } }",
        ),
        (
            "⊔ E { A(i32) } ƒ g(e: E) → i32 ! ∅ { ⌥ e { A(v) ↦ v } }",
            r"\enum E { A(i32) } \fn g(e: E) -> i32 ! \nil { \match e { A(v) => v } }",
        ),
        (
            "▢ P { x: i32 } ⊐ Q for P { ƒ h(&self) ! ∅ { ⚠ ⊢ p { } } }",
            r"\struct P { x: i32 } \impl Q for P { \fn h(&self) ! \nil { \unsafe \proof p { } } }",
        ),
        (
            "≡ Id ≔ u32; □ L: u32 ≔ 1; ■ S: u32 ≔ 2; ▦ m { } ⇲ a::b;",
            r"\type Id := u32; \const L: u32 := 1; \static S: u32 := 2; \mod m { } \use a::b;",
        ),
        (
            "ƒ f(p: *□ i32, q: *μ i32) ! ∅ { }",
            r"\fn f(p: *\const i32, q: *\mut i32) ! \nil { }",
        ),
        (
            "ƒ f(n: u32) ! ∅ ↓ n { ⟳ x in 0..n ↓ n ⋈ n ≥ 0 { ⊠; ⊡; } ⏎; }",
            r"\fn f(n: u32) ! \nil \dec n { \loop x in 0..n \dec n \inv n >= 0 { \break; \cont; } \ret; }",
        ),
        (
            "ƒ f(a: bool, b: bool) → bool ! ∅ ⊳ ∀ i ∈ r : a ⇒ ¬b ∨ b { a ∈ r }",
            r"\fn f(a: bool, b: bool) -> bool ! \nil \pre \forall i \in r : a \implies \neg b \lor b { a \in r }",
        ),
    ];
    for (c, s) in pairs {
        assert_eq!(fmt_ok(c), fmt_ok(s), "{c}");
    }
}

#[test]
fn already_canonical_corpus_files_keep_every_comment_line() {
    // tổng số comment trong corpus pass được bảo toàn (kiểm số lượng toàn cục, ngoài từng file)
    let mut before = 0;
    let mut after = 0;
    for p in files("pass") {
        let src = read(&p);
        before += comment_texts(&src).len();
        after += comment_texts(&fmt_ok(&src)).len();
    }
    assert_eq!(before, after);
    assert!(before > 30);
}

// ---------- property: biểu thức ngẫu nhiên ----------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[(self.next() % xs.len() as u64) as usize]
    }
    fn chance(&mut self, n: u64) -> bool {
        self.next().is_multiple_of(n)
    }
}

fn ws(r: &mut Rng) -> &'static str {
    if r.chance(3) { "" } else { " " }
}

/// Sinh biểu thức với ngoặc ngẫu nhiên (thừa hoặc cần) và khoảng trắng ngẫu nhiên. Struct literal chỉ xuất hiện trong ngoặc.
fn gen_expr(r: &mut Rng, depth: u32) -> String {
    if depth == 0 || r.chance(5) {
        return r
            .pick(&[
                "a",
                "b",
                "x.y",
                "p",
                "1",
                "2.5",
                "-3",
                "-128i8",
                "true",
                "'c'",
                "\"s\"",
                "self.w",
                "t.0",
                "Self::new()",
                "f()",
            ])
            .to_string();
    }
    let d = depth - 1;
    match r.next() % 14 {
        0..=4 => {
            let op = r.pick(&[
                "+", "-", "*", "/", "%", "+%", "-^", "*?", "<", "<=", "==", "!=", "≥", "≠", "∧",
                "∨", "⇒", "∈", "\\land", "\\lor",
            ]);
            let (l, rr) = (gen_expr(r, d), gen_expr(r, d));
            let (lp, rp) = (r.chance(2), r.chance(2));
            let l = if lp { format!("({l})") } else { l };
            let rr = if rp { format!("({rr})") } else { rr };
            format!("{l}{}{op}{}{rr}", ws(r), ws(r))
        }
        5 => format!(
            "{}{}",
            r.pick(&["-", "¬", "*", "&", "&!", "\\neg "]),
            paren_maybe(r, d)
        ),
        6 => format!(
            "{} as {}",
            paren_maybe(r, d),
            r.pick(&["u8", "i64", "&i32", "(i32, u8)"])
        ),
        7 => format!(
            "({}){}",
            gen_expr(r, d),
            r.pick(&[".f", ".0", "[1]", "(x)", "[..]", "[1..]"])
        ),
        8 => format!("({}, {})", gen_expr(r, d), gen_expr(r, d)),
        9 => format!("({},)", gen_expr(r, d)),
        10 => format!("[{}, {}]", gen_expr(r, d), gen_expr(r, d)),
        11 => format!(
            "({}){}{}",
            gen_expr(r, d),
            r.pick(&["..", "..="]),
            paren_maybe(r, d)
        ),
        12 => format!("(P {{ x: {}, y: {} }})", gen_expr(r, d), gen_expr(r, d)),
        _ => format!("(∀ i ∈ r : {})", gen_expr(r, d)),
    }
}

fn paren_maybe(r: &mut Rng, d: u32) -> String {
    let e = gen_expr(r, d);
    if r.chance(2) { format!("({e})") } else { e }
}

#[test]
fn random_expressions_roundtrip_with_correct_parentheses() {
    let mut r = Rng(0xDEAD_BEEF_CAFE_F00D);
    let (mut parsed, mut total) = (0, 0);
    for i in 0..6000 {
        let (e1, e2, e3) = (
            gen_expr(&mut r, 4),
            gen_expr(&mut r, 4),
            gen_expr(&mut r, 3),
        );
        // dùng cả vị trí thường lẫn các vị trí cấm struct literal trần (điều kiện, hợp đồng, ↓, ⋈)
        let src = match i % 4 {
            0 => format!("ƒ f() → i32 ! ∅ {{ x ≔ {e1}; {e2} }}"),
            1 => format!("ƒ f() → i32 ! ∅ {{ ⎇ {e1} {{ {e2} }} ⇝ {{ {e3} }} }}"),
            2 => format!("ƒ f() → i32 ! ∅ ⊳ {e1} ↓ {e2} {{ ⟳ {e3} ⋈ {e1} {{ ⊠; }} 0 }}"),
            _ => format!("ƒ f() → i32 ! ∅ {{ ⌥ {e1} {{ _ ↦ {e2} }} }}"),
        };
        total += 1;
        if !gt_parser::parse("t.gt", &src).report.diagnostics.is_empty() {
            continue; // nguồn ngẫu nhiên không hợp lệ (vd so sánh nối) — bỏ qua
        }
        parsed += 1;
        check_invariants(&format!("random#{i}: {src}"), &src);
    }
    assert!(
        parsed * 10 >= total * 4,
        "chỉ {parsed}/{total} nguồn ngẫu nhiên hợp lệ — bộ sinh quá yếu"
    );
}

#[test]
fn lifetime_arguments_in_types_roundtrip_adr_021() {
    // parse(fmt(x)) == parse(x) và idempotent trên mọi vị trí kiểu có đối số lifetime
    for src in [
        "▢ Holder<'a> { r: &'a i32 }\n",
        "▢ Pair<'a, T> { r: &'a T, v: T }\n",
        "ƒ get<'a>(h: Holder<'a>) → &'a i32 ! ∅ { h.r }\n",
        "ƒ f<'a, 'b>(x: Two<'a, 'b, u8, Option<Holder<'b>>>) ! ∅ { }\n",
        "ƒ f<'a>(x: &'a Holder<'a>) ! ∅ { y: Holder<'a> ≔ mk(x); }\n",
        "⊐ <'a> Holder<'a> { ƒ get(&self) → &'a i32 ! ∅ { self.r } }\n",
        "≡ H<'a> ≔ Holder<'a>;\n",
        "ƒ f(x:Pair<'a,u8>)!∅{}",
    ] {
        check_invariants(src, src);
    }
    assert_eq!(
        fmt_ok("ƒ f(x:Pair<'a,u8>)!∅{}"),
        "ƒ f(x: Pair<'a, u8>) ! ∅ { }\n"
    );
}
