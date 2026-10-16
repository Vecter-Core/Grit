//! FB-8 (2026-10-16): thông dịch MIR không cài so sánh THỨ TỰ trên `char` (`<` `≤` `>` `≥`): `binop Lt trên Char(..) Char(..)` ⇒ `Flow::Bug`,
//! trong khi LLVM chạy đúng (phát hiện bởi bộ kiểm std `Ord for char`, ba nguồn). Test hồi quy: mọi so sánh trên char phải cho đúng kết quả.
mod common;
use common::*;
use gt_mir::interp::{Interp, Outcome, V};

fn run(body: &str) -> Outcome {
    let t = one(&format!("ƒ f() → bool ! ∅ {{ {body} }}"));
    assert!(t.report.diagnostics.is_empty(), "{:?}", show(&t));
    let m = gt_mir::lower_program(&t);
    assert!(m.report.diagnostics.is_empty(), "{:?}", mir_codes(&m));
    Interp::new(&m, 100_000).call_key("main::f", Vec::new())
}

#[test]
fn ordering_comparisons_on_char_are_evaluated_like_code_points() {
    let cases = [
        ("'a' < 'b'", true),
        ("'b' < 'a'", false),
        ("'a' < 'a'", false),
        ("'a' <= 'a'", true),
        ("'b' <= 'a'", false),
        ("'b' > 'a'", true),
        ("'a' > 'a'", false),
        ("'a' >= 'a'", true),
        ("'a' >= 'b'", false),
        ("'中' > 'é'", true),
        ("'é' < '中'", true),
        ("'a' == 'a'", true),
        ("'a' != 'b'", true),
    ];
    for (src, want) in cases {
        assert_eq!(run(src), Outcome::Value(V::Bool(want)), "{src}");
    }
}
