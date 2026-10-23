//! Seed 1.0 S7.c — đột biến NGUY HIỂM của M7 (`assign_local`, `judge_posts`, `assume_posts`, `walk_paths`, `canon_*`, `subst_prop_k`, `pred_key`,
//! `extract_contract`; `docs/M7_DANGER_MUTANTS.md`): mỗi ca được thiết kế để kết quả của MỘT nghĩa vụ phụ thuộc trực tiếp vào đúng một điều kiện của hàm
//! quyết định xoá/giữ. Ký hiệu như `decisions.rs`: `x` xoá (đã chứng minh), `g` giữ lúc chạy, `d` ĐỎ, `r` bị bác (E0902).
//! Ca "x" bảo vệ hướng PHÂN TÍCH YẾU đi; ca "g/r" (đối chứng) bảo vệ hướng NGUY HIỂM (đột biến làm bộ giải xoá nhầm một check).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::analysis::Status;
use gt_oblig::driver::analyze_program;
use gt_oblig::prover::ZoneProver;

fn run_key(src: &str, key: &str) -> String {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        common::mir_codes(&m)
    );
    let o = analyze_program(&t, &m, &mut ZoneProver::new(24));
    let r = o.per_body.iter().find(|r| r.key == key).unwrap_or_else(|| {
        panic!(
            "không có {key}: {:?}",
            o.per_body.iter().map(|r| &r.key).collect::<Vec<_>>()
        )
    });
    assert!(r.ice.is_none(), "{:?}", r.ice);
    r.judged
        .iter()
        .map(|j| {
            let s = match j.status {
                Status::Discharged { .. } => 'x',
                Status::Lowered { .. } => 'g',
                Status::DefinitelyFalse => 'd',
                Status::Rejected { .. } => 'r',
            };
            format!("{}:{s}", j.ob.kind.name())
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// `GRIT_RECORD=1`: in kết quả thật thay vì so (dùng khi thiết kế ca mới).
fn check(name: &str, cases: &[(&str, &str)]) {
    let record = std::env::var("GRIT_RECORD").is_ok_and(|v| v == "1");
    let mut bad = Vec::new();
    for (i, (src, want)) in cases.iter().enumerate() {
        let got = run_key(src, "main::f");
        if record {
            println!("RECORD {name}#{i}: [{got}]  {}", src.replace('\n', " ⏎ "));
        } else if got != *want {
            bad.push(format!(
                "{name}#{i}: kỳ vọng [{want}], thực tế [{got}]\n  {src}"
            ));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn assign_local_decisions() {
    check(
        "assign_local",
        &[
            // ép số→số từ toán hạng KHÔNG theo dõi (a bị `&!`) ⇒ "không biết", không phải [0,1] của ép bool→số: nhánh `b > 1` còn sống
            (
                "ƒ f(μa: u32) → u64 ! ∅ { r ≔ &!a; *r ← 5; b ≔ a as u64; ⎇ b > 1 { b - 2 } ⇝ { 0 } }",
                "overflow:x",
            ),
            // `&s.b` (có phép chiếu) KHÔNG phải tham chiếu tới cả `s` ⇒ vị từ của `s.a` không suy ra cho `s.b`
            (
                "▢ S { a: Option<i32>, b: Option<i32> }\nƒ f(s: S) → i32 ! ∅ { ⎇ s.a.is_some() { s.b.unwrap() } ⇝ { 0 } }",
                "pre:r",
            ),
            // gán lại lát `s` ⇒ quên độ dài cũ của chính `s`
            (
                "ƒ f(a: &[u8], b: &[u8]) → u8 ! ∅ { μs ≔ a; ⎇ s.len() > 3 { s ← b; s[3] } ⇝ { 0 } }",
                "index:g",
            ),
        ],
    );
}

#[test]
fn pred_key_decisions() {
    check(
        "pred_key",
        &[
            // toán hạng có phép chiếu (`s.x`) không có khoá vị từ; `pos(s.x)` đúng KHÔNG suy ra `pos(s.y)`
            (
                "▢ S { x: i32, y: i32 }\nƒ pos(n: i32) → bool ! ∅ { n > 0 }\nƒ chk(n: i32) → i32 ! ∅ ⊳ pos(n) { n }\nƒ f(s: S) → i32 ! ∅ { ⎇ pos(s.x) { chk(s.y) } ⇝ { 0 } }",
                "pre:r",
            ),
            // local bị `&!` không có khoá vị từ (có thể bị đổi qua tham chiếu)
            (
                "ƒ clear(x: &!Option<i32>) ! ∅ { *x ← None; }\nƒ f(μo: Option<i32>) → i32 ! ∅ { clear(&!o); ⎇ o.is_some() { clear(&!o); o.unwrap() } ⇝ { 0 } }",
                "pre:r",
            ),
        ],
    );
}

/// `key < 1024`: local thứ 1024 trở đi KHÔNG có khoá vị từ (id nguyên tử mã hoá `pi*1024 + key` sẽ trùng ô của vị từ kế). Quét số biến đệm quanh 1024:
/// `o` ở local `pad + 1` (hiện tại; tạm cho `Some(1)` nằm SAU `o`), nên 1000..=1022 ⇒ chứng minh được (23 ca), 1023..=1029 (local 1024..=1030) ⇒ bị bác (7 ca).
/// Biên chính xác ở local 1024: đột biến `<` → `<=` làm ca pad = 1023 thành "chứng minh" ⇒ 6 bác / 24 chứng minh. (Nếu cách hạ MIR đổi số hiệu local
/// thì ĐẾM sẽ lệch: chỉnh lại hằng 23/7 cùng bài toán, KHÔNG nới.)
#[test]
fn locals_from_1024_have_no_predicate_key() {
    let (mut rejected, mut proven) = (Vec::new(), Vec::new());
    for pad in 1000..1030 {
        let mut src = String::from("ƒ f() → i32 ! ∅ {\n");
        for k in 0..pad {
            src.push_str(&format!("  v{k} ≔ 0u8;\n"));
        }
        src.push_str("  o ≔ Some(1);\n  ⎇ o.is_some() { o.unwrap() } ⇝ { 0 }\n}\n");
        match run_key(&src, "main::f").as_str() {
            "pre:r" => rejected.push(pad),
            "pre:x" => proven.push(pad),
            other => panic!("pad {pad}: {other}"),
        }
    }
    assert_eq!(rejected, (1023..1030).collect::<Vec<_>>());
    assert_eq!(proven, (1000..1023).collect::<Vec<_>>());
}

#[test]
fn postcondition_mapping_decisions() {
    check(
        "post_map",
        &[
            // tham số KHÔNG được ⊲ nhắc mà bị gán lại không làm hỏng ⊲ (m không được coi là "được nhắc")
            (
                "ƒ f(n: u8, μm: u8) → u8 ! ∅ ⊲ result == n { m ← 3; n }",
                "post:x",
            ),
            // tham số đầu không được nhắc không được chặn việc ánh xạ tham số sau
            ("ƒ f(n: u8, m: u8) → u8 ! ∅ ⊲ result == m { m }", "post:x"),
            // nguyên tử bool của tham số (cuối / không cuối) giữ nguyên nghĩa trong ⊲
            ("ƒ f(n: u8, b: bool) → bool ! ∅ ⊳ b ⊲ b { b }", "post:x"),
            ("ƒ f(b: bool, n: u8) → bool ! ∅ ⊳ b ⊲ b { b }", "post:x"),
            // vị từ thuần trên tham số; tham số khác bị gán lại không làm hỏng
            (
                "ƒ pos(n: i32) → bool ! ∅ { n > 0 }\nƒ f(n: i32) → i32 ! ∅ ⊳ pos(n) ⊲ pos(n) { n }",
                "post:x",
            ),
            (
                "ƒ pos(n: i32) → bool ! ∅ { n > 0 }\nƒ f(n: i32, μm: i32) → i32 ! ∅ ⊳ pos(n) ⊲ pos(n) { m ← 3; n }",
                "post:x",
            ),
            // local KHÔNG phải tham số (không Copy) không được tính vào "tham số sao chép được"
            (
                "ƒ f(n: u8) → u8 ! ∅ ⊲ result > 0 { b ≔ mk_buf(); n }",
                "post:g",
            ),
        ],
    );
}

#[test]
fn assume_posts_decisions() {
    check(
        "assume_posts",
        &[
            // `x ← add2(x, y)` — đối số nhắc `dest` (x) ⇒ không thế `⊲ result == a + b` (sẽ ra x == x + y ⇒ y == 0 ⇒ 255 / y ĐỎ giả)
            (
                "ƒ add2(a: u8, b: u8) → u8 ! ∅ ⊳ a < 100 ⊳ b < 100 ⊲ result == a + b { a + b }\nƒ f(μx: u8, y: u8) → u8 ! ∅ { x ← add2(x, y); 255 / y }",
                "pre:g,pre:g,div_zero:g",
            ),
            // đối số là MỘT biểu thức nhiều biến, chỉ một trong số đó là dest
            (
                "ƒ id1(a: u8) → u8 ! ∅ ⊳ a < 100 ⊲ result == a { a }\nƒ f(μx: u8, y: u8) → u8 ! ∅ { x ← id1(x + y); 255 / y }",
                "overflow:g,pre:g,div_zero:g",
            ),
            (
                "ƒ inc(n: u8) → u8 ! ∅ ⊳ n < 255 ⊲ result == n + 1 { n + 1 }\nƒ f(μx: u8) → u8 ! ∅ { x ← inc(x); x - 1 }",
                "pre:g,overflow:x",
            ),
            // đích bị `&!` (không theo dõi): KHÔNG được ghi sự kiện về nó (có thể bị đổi qua tham chiếu sau đó)
            (
                "ƒ one() → u8 ! ∅ ⊲ result > 0 { 1 }\nƒ zero(p: &!u8) ! ∅ { *p ← 0; }\nƒ f() → u8 ! ∅ { μy ≔ 5u8; zero(&!y); y ← one(); zero(&!y); 255 / y }",
                "div_zero:g",
            ),
            // kết quả bool của callee (⊲ result) thành sự kiện về biến đích
            (
                "ƒ g(n: u8, m: u8) → bool ! ∅ ⊲ result { true }\nƒ need(c: bool) → u8 ! ∅ ⊳ c { 1 }\nƒ f(x: u8) → u8 ! ∅ { b ≔ g(x, x); need(b) }",
                "pre:x",
            ),
        ],
    );
}

#[test]
fn contract_extraction_decisions() {
    check(
        "extract",
        &[
            // nhánh chết (`n >= 0` với u8 luôn đúng) không làm hỏng việc trích hợp đồng
            (
                "ƒ f(n: u8) → u8 ! ∅ ⊳ n >= 0 ∧ n < 10 { n + 245 }",
                "overflow:x",
            ),
            // khối hợp lưu (join) đi tới từ hai đường không phải vòng lặp: ⊳ ⇒ 4 ≤ n < 10
            (
                "ƒ f(n: u8) → u8 ! ∅ ⊳ (n > 5 ∨ n > 3) ∧ n < 10 { n - 4 }",
                "overflow:x",
            ),
            // nguyên tử riêng đầu tiên của hợp đồng (opaque) được ánh xạ
            (
                "ƒ f(n: u8) → u8 ! ∅ ⊳ n > 0 ∧ n % 2 == 0 { n - 1 }",
                "overflow:x",
            ),
            // điều kiện đường đi được ghi (`record_pc`): ⊳ (n > 5 ? n < 3 : n > 0) = 0 < n ≤ 5
            (
                "ƒ f(n: u8) → u8 ! ∅ ⊳ ⎇ n > 5 { n < 3 } ⇝ { n > 0 } { n + 250 }",
                "overflow:x",
            ),
            // độ dài của tham số lát CUỐI trong hợp đồng
            (
                "ƒ f(n: u8, xs: &[u8]) → u8 ! ∅ ⊳ xs.len() > 0 { xs[0] }",
                "index:x",
            ),
        ],
    );
}
