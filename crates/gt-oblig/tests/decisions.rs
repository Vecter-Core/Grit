//! Quyết định XOÁ / GIỮ / ĐỎ của bộ phân tích, kiểm CHÍNH XÁC trên chương trình nhỏ (lệnh Giang 2026-10-08, mục 4b): mỗi ca được thiết kế
//! sao cho kết quả của MỘT nghĩa vụ phụ thuộc trực tiếp vào đúng một hàm chuyển / một cạnh / một quy tắc theo dõi. Ca "xoá" nếu đột
//! biến làm phân tích YẾU đi; ca "giữ" (đối chứng) nếu đột biến làm phân tích MẠNH quá mức (nguy hiểm: xoá nhầm check).
//! Ký hiệu: `x` xoá, `g` giữ lúc chạy, `d` ĐỎ (chắc chắn sai), `r` bị bác (E0902).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::analysis::Status;
use gt_oblig::driver::analyze_program;
use gt_oblig::prover::ZoneProver;

/// Danh sách "loại:trạng thái" của các nghĩa vụ trong `main::f`, theo thứ tự id.
fn run(src: &str) -> String {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        common::mir_codes(&m)
    );
    let o = analyze_program(&t, &m, &mut ZoneProver::new(24));
    let r = o
        .per_body
        .iter()
        .find(|r| r.key == "main::f")
        .expect("main::f");
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

fn check(cases: &[(&str, &str)]) {
    let mut bad = Vec::new();
    for (src, want) in cases {
        let got = run(src);
        if got != *want {
            bad.push(format!("kỳ vọng [{want}], thực tế [{got}]\n  {src}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn wrapping_and_saturating_results_keep_exact_ranges_only_when_no_wrap_happens() {
    check(&[
        // a ∈ [0,4] ⇒ a +% 250 ∈ [250,254] (không quấn) ⇒ b + 1 vừa, b + 2 thì không
        (
            "ƒ f(a: u8) → u8 ! ∅ { ⎇ a < 5 { b ≔ a +% 250; b + 1 } ⇝ { 0 } }",
            "overflow:x",
        ),
        (
            "ƒ f(a: u8) → u8 ! ∅ { ⎇ a < 5 { b ≔ a +% 250; b + 2 } ⇝ { 0 } }",
            "overflow:g",
        ),
        // a ∈ [0,9] ⇒ a +% 250 CÓ THỂ quấn ⇒ kết quả mất cận ⇒ b + 1 phải giữ
        (
            "ƒ f(a: u8) → u8 ! ∅ { ⎇ a < 10 { b ≔ a +% 250; b + 1 } ⇝ { 0 } }",
            "overflow:g",
        ),
        // bão hoà: không chạm trần thì chính xác
        (
            "ƒ f(a: u8) → u8 ! ∅ { ⎇ a < 10 { b ≔ a +^ 200; b + 46 } ⇝ { 0 } }",
            "overflow:x",
        ),
        (
            "ƒ f(a: u8) → u8 ! ∅ { ⎇ a < 10 { b ≔ a +^ 200; b + 47 } ⇝ { 0 } }",
            "overflow:g",
        ),
        // bão hoà chạm trần: b = 255 chắc chắn khi a ≥ 100 ⇒ b + 1 chắc chắn tràn
        (
            "ƒ f(a: u8) → u8 ! ∅ { ⎇ a >= 100 { b ≔ a +^ 200; b + 1 } ⇝ { 0 } }",
            "overflow:d",
        ),
        // trừ bão hoà về 0
        (
            "ƒ f(a: u8) → u8 ! ∅ { ⎇ a < 10 { b ≔ a -^ 20; b + 255 } ⇝ { 0 } }",
            "overflow:x",
        ),
        // b = 0 chắc chắn; c = b + 255 = 255 chắc chắn; c + 1 chắc chắn tràn
        (
            "ƒ f(a: u8) → u8 ! ∅ { ⎇ a < 10 { b ≔ a -^ 20; c ≔ b + 255; c + 1 } ⇝ { 0 } }",
            "overflow:x,overflow:d",
        ),
    ]);
}

#[test]
fn division_and_remainder_results_are_bounded_by_the_divisor() {
    check(&[
        (
            "ƒ f(a: u8) → u8 ! ∅ { r ≔ a % 7; r + 249 }",
            "rem_zero:x,overflow:x",
        ),
        (
            "ƒ f(a: u8) → u8 ! ∅ { r ≔ a % 7; r + 250 }",
            "rem_zero:x,overflow:g",
        ),
        (
            "ƒ f(a: u8) → u8 ! ∅ { q ≔ a / 4; q + 192 }",
            "div_zero:x,overflow:x",
        ),
        (
            "ƒ f(a: u8) → u8 ! ∅ { q ≔ a / 4; q + 193 }",
            "div_zero:x,overflow:g",
        ),
        // số bị chia nhỏ hơn số chia ⇒ dư = chính nó
        (
            "ƒ f(a: u8) → u8 ! ∅ { ⎇ a < 5 { r ≔ a % 9; r + 251 } ⇝ { 0 } }",
            "rem_zero:x,overflow:x",
        ),
        (
            "ƒ f(a: u8) → u8 ! ∅ { ⎇ a < 5 { r ≔ a % 9; r + 252 } ⇝ { 0 } }",
            "rem_zero:x,overflow:g",
        ),
        // số chia âm / số bị chia âm (dấu theo số bị chia)
        (
            "ƒ f(a: i8) → i8 ! ∅ { r ≔ a % -3; r + 125 }",
            "rem_zero:x,div_overflow:x,overflow:x",
        ),
        (
            "ƒ f(a: i8) → i8 ! ∅ { r ≔ a % -3; r + 126 }",
            "rem_zero:x,div_overflow:x,overflow:g",
        ),
        (
            "ƒ f(a: i8) → i8 ! ∅ { r ≔ a % -3; r - 126 }",
            "rem_zero:x,div_overflow:x,overflow:x",
        ),
        (
            "ƒ f(a: i8) → i8 ! ∅ { r ≔ a % -3; r - 127 }",
            "rem_zero:x,div_overflow:x,overflow:g",
        ),
    ]);
}

#[test]
fn negation_casts_and_comparisons_feed_later_obligations() {
    check(&[
        (
            "ƒ f(x: i8) → i8 ! ∅ { ⎇ x > -6 { ⎇ x < 6 { y ≔ -x; y + 122 } ⇝ { 0 } } ⇝ { 0 } }",
            "neg_min:x,overflow:x",
        ),
        (
            "ƒ f(x: i8) → i8 ! ∅ { ⎇ x > -6 { ⎇ x < 6 { y ≔ -x; y + 123 } ⇝ { 0 } } ⇝ { 0 } }",
            "neg_min:x,overflow:g",
        ),
        // -x với x chưa biết
        ("ƒ f(x: i8) → i8 ! ∅ { -x }", "neg_min:g"),
        // ép kiểu hẹp trong khoảng đã chứng minh, rồi dùng kết quả
        (
            "ƒ f(a: i32) → i32 ! ∅ { ⎇ a >= 0 { ⎇ a <= 100 { b ≔ a as u8; (b as i32) + 2147483547 } ⇝ { 0 } } ⇝ { 0 } }",
            "cast_lossy:x,overflow:x",
        ),
        (
            "ƒ f(a: i32) → i32 ! ∅ { ⎇ a >= 0 { ⎇ a <= 100 { b ≔ a as u8; (b as i32) + 2147483548 } ⇝ { 0 } } ⇝ { 0 } }",
            "cast_lossy:x,overflow:g",
        ),
        // ép kiểu ngoài khoảng: chắc chắn sai
        (
            "ƒ f(a: i32) → u8 ! ∅ { ⎇ a > 300 { a as u8 } ⇝ { 0 } }",
            "cast_lossy:d",
        ),
        (
            "ƒ f(a: i32) → u8 ! ∅ { ⎇ a < 0 { a as u8 } ⇝ { 0 } }",
            "cast_lossy:d",
        ),
        ("ƒ f(a: i32) → u8 ! ∅ { a as u8 }", "cast_lossy:g"),
        // i64 → f64 chính xác khi |x| ≤ 2^53 (biên đúng 2^53 và 2^53 + 1; ADR-035 §B)
        (
            "ƒ f(a: i64) → f64 ! ∅ { ⎇ a >= -9007199254740992 { ⎇ a <= 9007199254740992 { a as f64 } ⇝ { 0.0 } } ⇝ { 0.0 } }",
            "cast_lossy:x",
        ),
        (
            "ƒ f(a: i64) → f64 ! ∅ { ⎇ a >= -9007199254740992 { ⎇ a <= 9007199254740993 { a as f64 } ⇝ { 0.0 } } ⇝ { 0.0 } }",
            "cast_lossy:g",
        ),
        (
            "ƒ f(a: i64) → f64 ! ∅ { ⎇ a >= -9007199254740993 { ⎇ a <= 9007199254740992 { a as f64 } ⇝ { 0.0 } } ⇝ { 0.0 } }",
            "cast_lossy:g",
        ),
        (
            "ƒ f(a: i64) → f64 ! ∅ { ⎇ a > 9007199254740993 { a as f64 } ⇝ { 0.0 } }",
            "cast_lossy:d",
        ),
        ("ƒ f(a: i64) → f64 ! ∅ { a as f64 }", "cast_lossy:g"),
        // so sánh lưu vào biến bool rồi rẽ nhánh
        (
            "ƒ f(x: u8) → u8 ! ∅ { c ≔ x < 10; ⎇ c { x + 246 } ⇝ { 0 } }",
            "overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { c ≔ x < 10; ⎇ c { x + 247 } ⇝ { 0 } }",
            "overflow:g",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { c ≔ x < 10; ⎇ ¬c { x + 0 } ⇝ { 0 } }",
            "overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { c ≔ x >= 10; ⎇ c { x - 10 } ⇝ { 0 } }",
            "overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { c ≔ x >= 10; ⎇ ¬c { x - 10 } ⇝ { 0 } }",
            "overflow:d",
        ),
        // ∧ / ∨
        // điều kiện ghép được hạ THẲNG thành nhánh (M5-fix 2026-10-08): nhánh "đúng" của `∧` biết cả hai vế
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ x >= 10 ∧ x < 20 { x - 10 } ⇝ { 0 } }",
            "overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ x >= 10 ∧ x < 20 { x + 235 } ⇝ { 0 } }",
            "overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ x >= 10 ∧ x < 20 { x + 237 } ⇝ { 0 } }",
            "overflow:g",
        ),
        // nhánh "sai" của `∧` chỉ biết ¬a ∨ ¬b (không suy ra gì chắc chắn)
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ x >= 10 ∧ x < 20 { 0 } ⇝ { x - 10 } }",
            "overflow:g",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ x >= 10 ∨ x < 20 { x - 10 } ⇝ { 0 } }",
            "overflow:g",
        ),
        // nhánh "sai" của `∨` biết ¬a ∧ ¬b
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ x < 10 ∨ x >= 20 { 0 } ⇝ { x - 10 } }",
            "overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ x < 10 ∨ x >= 20 { 0 } ⇝ { x + 235 } }",
            "overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ x < 10 ∨ x >= 20 { 0 } ⇝ { x + 237 } }",
            "overflow:g",
        ),
        // ¬(a ∧ b) ≡ ¬a ∨ ¬b: nhánh sai biết a ∧ b
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ ¬(x >= 10 ∧ x < 20) { 0 } ⇝ { x - 10 } }",
            "overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ ¬(x >= 10 ∧ x < 20) { x - 10 } ⇝ { 0 } }",
            "overflow:g",
        ),
        // lồng nhau
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ (x >= 10 ∧ x < 20) ∧ x != 15 { x - 10 } ⇝ { 0 } }",
            "overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ x < 5 ∨ (x >= 10 ∧ x < 20) { 0 } ⇝ { x - 5 } }",
            "overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ (x < 5 ∨ x >= 20) ∨ x == 7 { 0 } ⇝ { x - 5 } }",
            "overflow:x",
        ),
        // vế phải chỉ được đánh giá khi vế trái đúng/sai (ngắn mạch): nghĩa vụ ở vế phải dùng kết quả vế trái
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { ⎇ i < xs.len() ∧ xs[i] > 3 { 1 } ⇝ { 0 } }",
            "index:x",
        ),
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { ⎇ i >= xs.len() ∨ xs[i] > 3 { 1 } ⇝ { 0 } }",
            "index:x",
        ),
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { ⎇ i < xs.len() ∨ xs[i] > 3 { 1 } ⇝ { 0 } }",
            "index:d",
        ),
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { ⎇ i >= xs.len() ∧ xs[i] > 3 { 1 } ⇝ { 0 } }",
            "index:d",
        ),
        // vòng lặp có điều kiện ghép
        (
            "ƒ f(xs: &[u8]) → usize ! ∅ { μi ≔ 0; ⟳ i < xs.len() ∧ xs[i] > 0 ↓ xs.len() - i { i ← i + 1; } i }",
            "index:x,overflow:x",
        ),
        // dùng điều kiện như GIÁ TRỊ vẫn hạ như cũ (không refine)
        (
            "ƒ f(x: u8) → u8 ! ∅ { c ≔ x >= 10 ∧ x < 20; ⎇ c { x - 10 } ⇝ { 0 } }",
            "overflow:g",
        ),
    ]);
}

#[test]
fn chains_of_checked_operations_use_the_refined_ranges() {
    check(&[
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ x < 100 { y ≔ x + 1; z ≔ y + 1; z + 154 } ⇝ { 0 } }",
            "overflow:x,overflow:x,overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ x < 100 { y ≔ x + 1; z ≔ y + 1; z + 155 } ⇝ { 0 } }",
            "overflow:x,overflow:x,overflow:g",
        ),
        // a - b sau khi biết a ≥ b
        (
            "ƒ f(a: u8, b: u8) → u8 ! ∅ { ⎇ a >= b { a - b } ⇝ { 0 } }",
            "overflow:x",
        ),
        (
            "ƒ f(a: u8, b: u8) → u8 ! ∅ { ⎇ a > b { a - b } ⇝ { 0 } }",
            "overflow:x",
        ),
        (
            "ƒ f(a: u8, b: u8) → u8 ! ∅ { ⎇ a < b { a - b } ⇝ { 0 } }",
            "overflow:d",
        ),
        ("ƒ f(a: u8, b: u8) → u8 ! ∅ { a - b }", "overflow:g"),
        // tổng đã qua kiểm tra không cho phép suy ra hiệu (đối chứng: bộ giải KHÔNG biết c - b = a)
        (
            "ƒ f(a: u8, b: u8) → u8 ! ∅ { c ≔ a + b; c - b }",
            "overflow:g,overflow:g",
        ),
        // hai biến bằng nhau qua gán
        (
            "ƒ f(a: u8) → u8 ! ∅ { b ≔ a; ⎇ a >= 5 { b - 5 } ⇝ { 0 } }",
            "overflow:x",
        ),
        (
            "ƒ f(a: u8) → u8 ! ∅ { μb ≔ a; b ← b + 0; ⎇ a >= 5 { b - 5 } ⇝ { 0 } }",
            "overflow:x,overflow:x",
        ),
        // gán lại phá quan hệ
        (
            "ƒ f(a: u8, c: u8) → u8 ! ∅ { μb ≔ a; b ← c; ⎇ a >= 5 { b - 5 } ⇝ { 0 } }",
            "overflow:g",
        ),
    ]);
}

#[test]
fn slice_lengths_relations_and_assertion_narrowing() {
    check(&[
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { n ≔ xs.len(); ⎇ i < n { xs[i] } ⇝ { 0 } }",
            "index:x",
        ),
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { n ≔ xs.len(); ⎇ i <= n { xs[i] } ⇝ { 0 } }",
            "index:g",
        ),
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { ⎇ i < xs.len() { xs[i] } ⇝ { 0 } }",
            "index:x",
        ),
        // quan hệ truyền qua phép gán
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { j ≔ i; ⎇ j < xs.len() { xs[i] } ⇝ { 0 } }",
            "index:x",
        ),
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { μj ≔ i; ⎇ j < xs.len() { j ← j + 1; xs[i] } ⇝ { 0 } }",
            "overflow:x,index:x",
        ),
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { μj ≔ i; ⎇ j < xs.len() { j ← j + 1; xs[j] } ⇝ { 0 } }",
            "overflow:x,index:g",
        ),
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { μj ≔ i; ⎇ j < xs.len() { j ← j + 5; xs[j] } ⇝ { 0 } }",
            "overflow:g,index:g",
        ),
        // sau khi xs[i] đạt: i < len ⇒ i + 1 không tràn
        (
            "ƒ f(xs: &[u8], i: usize) → usize ! ∅ { v ≔ xs[i]; i + 1 }",
            "index:g,overflow:x",
        ),
        (
            "ƒ f(xs: &[u8], i: usize) → usize ! ∅ { i + 1 }",
            "overflow:g",
        ),
        // chỉ số hằng
        (
            "ƒ f(xs: &[u8]) → u8 ! ∅ { ⎇ xs.len() >= 3 { xs[2] } ⇝ { 0 } }",
            "index:x",
        ),
        (
            "ƒ f(xs: &[u8]) → u8 ! ∅ { ⎇ xs.len() >= 3 { xs[3] } ⇝ { 0 } }",
            "index:g",
        ),
        (
            "ƒ f(xs: &[u8]) → u8 ! ∅ { ⎇ xs.len() == 0 { xs[0] } ⇝ { 0 } }",
            "index:d",
        ),
        // mảng độ dài hằng
        ("ƒ f() → u8 ! ∅ { a ≔ [1u8, 2, 3]; a[2] }", "index:x"),
        ("ƒ f() → u8 ! ∅ { a ≔ [1u8, 2, 3]; a[3] }", "index:d"),
        (
            "ƒ f(i: usize) → u8 ! ∅ { a ≔ [1u8, 2, 3]; ⎇ i < 3 { a[i] } ⇝ { 0 } }",
            "index:x",
        ),
        (
            "ƒ f(i: usize) → u8 ! ∅ { a ≔ [1u8, 2, 3]; ⎇ i < 4 { a[i] } ⇝ { 0 } }",
            "index:g",
        ),
        // lát con
        (
            "ƒ f(xs: &[u8]) → usize ! ∅ { ⎇ xs.len() >= 4 { ys ≔ &xs[1..3]; ys.len() } ⇝ { 0 } }",
            "slice_range:x",
        ),
        (
            "ƒ f(xs: &[u8]) → usize ! ∅ { ⎇ xs.len() >= 3 { ys ≔ &xs[1..4]; ys.len() } ⇝ { 0 } }",
            "slice_range:g",
        ),
    ]);
}

#[test]
fn alias_tracking_and_reborrow_roots() {
    check(&[
        // biến bị mượn &! ở đâu đó KHÔNG được theo dõi: x + 10 phải giữ dù x ≔ 250 (đối chứng: nếu theo dõi sẽ báo ĐỎ sai)
        (
            "ƒ bump(p: &!u8) ! ∅ { *p ← 0u8; }\nƒ f() → u8 ! ∅ { μx ≔ 250u8; bump(&!x); x + 10 }",
            "overflow:g",
        ),
        // không bị mượn: theo dõi chính xác
        ("ƒ f() → u8 ! ∅ { μx ≔ 250u8; x + 10 }", "overflow:d"),
        ("ƒ f() → u8 ! ∅ { μx ≔ 250u8; x + 5 }", "overflow:x"),
        // bí danh của tham số lát (gán đúng một lần) dùng chung độ dài gốc
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { ys ≔ xs; ⎇ i < ys.len() { xs[i] } ⇝ { 0 } }",
            "index:x",
        ),
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { ys ≔ xs; ⎇ i < xs.len() { ys[i] } ⇝ { 0 } }",
            "index:x",
        ),
        // bí danh bị gán LẠI ⇒ không còn là bí danh của gốc (đối chứng quan trọng)
        (
            "ƒ f(xs: &[u8], zs: &[u8], i: usize) → u8 ! ∅ { μys ≔ xs; ys ← zs; ⎇ i < ys.len() { xs[i] } ⇝ { 0 } }",
            "index:g",
        ),
        (
            "ƒ f(xs: &[u8], zs: &[u8], i: usize) → u8 ! ∅ { μys ≔ xs; ys ← zs; ⎇ i < zs.len() { ys[i] } ⇝ { 0 } }",
            "index:g", // sau `ys ← zs` độ dài của ys chưa được biết bằng zs (sound, chưa đầy đủ)
        ),
        // gốc là tham số bị gán lại ⇒ không dùng làm gốc
        (
            "ƒ f(μxs: &[u8], zs: &[u8], i: usize) → u8 ! ∅ { ys ≔ xs; xs ← zs; ⎇ i < ys.len() { xs[i] } ⇝ { 0 } }",
            "index:g", // LỖ HỔNG đã sửa: tham số có gán ngầm lúc vào ⇒ KHÔNG phải mắt xích đơn-gán của chuỗi bí danh
        ),
        // hai lát độc lập: độ dài của lát này không suy ra lát kia
        (
            "ƒ f(xs: &[u8], zs: &[u8], i: usize) → u8 ! ∅ { ⎇ i < zs.len() { xs[i] } ⇝ { 0 } }",
            "index:g",
        ),
    ]);
}

#[test]
fn loops_widening_narrowing_and_relations() {
    check(&[
        (
            "ƒ f() → u8 ! ∅ { μi ≔ 0u8; ⟳ i < 200 ↓ 200 - i { i ← i + 1; } i }",
            "overflow:x",
        ),
        (
            "ƒ f() → u8 ! ∅ { μi ≔ 0u8; ⟳ i < 200 ↓ 200 - i { i ← i + 2; } i }",
            "overflow:x",
        ),
        (
            "ƒ f() → u8 ! ∅ { μi ≔ 0u8; ⟳ i < 255 ↓ 255 - i { i ← i + 1; } i }",
            "overflow:x",
        ),
        (
            "ƒ f() → u8 ! ∅ { μi ≔ 0u8; ⟳ i <= 255 ↓ 255 - i { i ← i + 1; } i }",
            "overflow:g",
        ),
        (
            "ƒ f() → u8 ! ∅ { μi ≔ 10u8; ⟳ i > 0 ↓ i { i ← i - 1; } i }",
            "overflow:x",
        ),
        (
            "ƒ f() → u8 ! ∅ { μi ≔ 10u8; ⟳ i >= 0 ↓ i { i ← i - 1; } i }",
            "overflow:g",
        ),
        // số lần lặp không biết ⇒ tích luỹ phải giữ (đối chứng)
        (
            "ƒ f() → u8 ! ∅ { μi ≔ 0u8; μs ≔ 0u8; ⟳ i < 10 ↓ 10 - i { s ← s + 1; i ← i + 1; } s }",
            "overflow:g,overflow:x",
        ),
        // hai biến đếm cùng nhịp: quan hệ j = i được giữ qua đầu vòng
        // hằng khởi tạo không tạo quan hệ ⇒ j + 1 phải giữ (chưa đầy đủ, sound)
        (
            "ƒ f(n: usize) → usize ! ∅ { μi ≔ 0; μj ≔ 0; ⟳ i < n ↓ n - i { i ← i + 1; j ← j + 1; } j }",
            "overflow:x,overflow:g",
        ),
        // j ≔ i tạo quan hệ j = i, giữ qua đầu vòng ⇒ j < n
        (
            "ƒ f(n: usize) → usize ! ∅ { μi ≔ 0; μj ≔ i; ⟳ i < n ↓ n - i { i ← i + 1; j ← j + 1; } j }",
            "overflow:x,overflow:x",
        ),
        // nhịp khác nhau: j tăng gấp đôi ⇒ không còn quan hệ
        (
            "ƒ f(n: usize) → usize ! ∅ { μi ≔ 0; μj ≔ 0; ⟳ i < n ↓ n - i { i ← i + 1; j ← j + 2; } j }",
            "overflow:x,overflow:g",
        ),
        // vòng lặp lồng nhau
        (
            "ƒ f() → u8 ! ∅ { μi ≔ 0u8; ⟳ i < 10 ↓ 10 - i { μj ≔ 0u8; ⟳ j < 10 ↓ 10 - j { j ← j + 1; } i ← i + 1; } i }",
            "overflow:x,overflow:x",
        ),
        // thoát bằng ⊠ (break) không làm yếu điều đã biết
        (
            "ƒ f() → u8 ! ∅ { μi ≔ 0u8; ⟳ i < 100 ↓ 100 - i { ⎇ i > 50 { ⊠; } i ← i + 1; } i }",
            "overflow:x",
        ),
        // vòng lặp vô hạn có lối thoát: không kết luận gì về i
        // vòng vô hạn thoát khi i > 3: tại phép cộng i ≤ 3 (i bị chặn bởi lối thoát)
        (
            "ƒ f() → u8 ! ∅ { μi ≔ 0u8; ⟳ { i ← i + 1; ⎇ i > 3 { ⊠; } } i }",
            "overflow:x",
        ),
    ]);
}

#[test]
fn calls_overwrite_destinations_and_unmodelled_calls_are_unknown() {
    check(&[
        // đích của lời gọi bị quên: b không còn là 1
        (
            "ƒ g(x: u8) → u8 ! ∅ { x }\nƒ f(a: u8) → u8 ! ∅ { μb ≔ 1u8; b ← g(a); b + 1 }",
            "overflow:g",
        ),
        (
            "ƒ g(x: u8) → u8 ! ∅ { x }\nƒ f(a: u8) → u8 ! ∅ { b ≔ g(a); b + 0 }",
            "overflow:x",
        ),
        // kết quả lời gọi không biết ⇒ phép trừ phải giữ
        (
            "ƒ g(x: u8) → u8 ! ∅ { x }\nƒ f(a: u8) → u8 ! ∅ { b ≔ g(a); a - b }",
            "overflow:g",
        ),
        // tham số hợp đồng: ⊲ result == n cho phép suy ra b = a
        (
            "ƒ g(n: u8) → u8 ! ∅ ⊲ result == n { n }\nƒ f(a: u8) → u8 ! ∅ { b ≔ g(a); a - b }",
            "overflow:x",
        ),
        (
            "ƒ g(n: u8) → u8 ! ∅ ⊲ result <= n { n }\nƒ f(a: u8) → u8 ! ∅ { b ≔ g(a); a - b }",
            "overflow:x",
        ),
        (
            "ƒ g(n: u8) → u8 ! ∅ ⊲ result >= n { n }\nƒ f(a: u8) → u8 ! ∅ { b ≔ g(a); a - b }",
            "overflow:g",
        ),
        // x = g(x): giá trị cũ của x đã mất ⇒ KHÔNG được thế (đối chứng)
        // ⊲ result == n + 1 là sự thật về kết quả (thân g sai ⇒ g bị bác riêng): result ≥ 1 nên a - 1 không tràn
        (
            "ƒ g(n: u8) → u8 ! ∅ ⊲ result == n + 1 { n }\nƒ f(μa: u8) → u8 ! ∅ { a ← g(a); a - 1 }",
            "overflow:x",
        ),
        (
            "ƒ g(n: u8) → u8 ! ∅ ⊲ result <= n { n }\nƒ f(μa: u8) → u8 ! ∅ { a ← g(a); a - 1 }",
            "overflow:g",
        ),
    ]);
}

#[test]
#[ignore]
fn dump_one() {
    let src = std::env::var("GT_SRC").expect("GT_SRC");
    let (t, m) = common::mir(&src);
    let o = analyze_program(&t, &m, &mut ZoneProver::new(24));
    for r in &o.per_body {
        for j in &r.judged {
            eprintln!(
                "{} {} {:?}\n goal {}\n hyps {:?}",
                j.ob.id,
                j.ob.kind.name(),
                j.status,
                j.ob.goal.to_text(),
                j.ob.hyps.iter().map(|h| h.to_text()).collect::<Vec<_>>()
            );
        }
    }
    eprintln!("{}", common::text(&t, &m, "main::f"));
}
