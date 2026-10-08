//! Hiệu ứng (M8 nửa 1; semantics §4): E0801 (suy ra ⊄ khai báo), E0802 (chữ ký không tương thích), kết quả CHÍNH XÁC.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::effects::check_effects;

fn codes(src: &str) -> Vec<String> {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        common::mir_codes(&m)
    );
    let o = check_effects(&t, &m);
    let mut v: Vec<String> = o
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    v.sort();
    v
}

fn ok(src: &str) {
    let c = codes(src);
    assert!(c.is_empty(), "kỳ vọng sạch, có {c:?}\n{src}");
}

fn only(src: &str, code: &str) {
    let c = codes(src);
    assert!(
        !c.is_empty() && c.iter().all(|x| x == code),
        "kỳ vọng chỉ {code}, có {c:?}\n{src}"
    );
}

const IO: &str = "ƒ out() → () ! {io} { }\n";

#[test]
fn declared_must_cover_inferred() {
    ok(&format!("{IO}ƒ f() → () ! {{io}} {{ out(); }}"));
    ok(&format!("{IO}ƒ f() → () ! {{io, mem}} {{ out(); }}"));
    ok("ƒ f() → i32 ! ∅ { 1 }");
    only(&format!("{IO}ƒ f() → () ! ∅ {{ out(); }}"), "E0801");
    only(&format!("{IO}ƒ f() → () ! {{mem}} {{ out(); }}"), "E0801");
    ok("ƒ f() → () ! ∅ { ⟳ { } }"); // vòng vô hạn thiếu div là E0701 (termination), không phải E0801
    ok("ƒ f() → () ! {div} { ⟳ { } }");
}

#[test]
fn effects_propagate_through_call_chains_and_recursion() {
    // hai tầng: g gọi out (io) khai đúng, f gọi g mà khai ∅
    only(
        &format!("{IO}ƒ g() → () ! {{io}} {{ out(); }}\nƒ f() → () ! ∅ {{ g(); }}"),
        "E0801",
    );
    ok(&format!(
        "{IO}ƒ g() → () ! {{io}} {{ out(); }}\nƒ f() → () ! {{io}} {{ g(); }}"
    ));
    // đệ quy tương hỗ: hiệu ứng io đi qua chu trình
    only(
        &format!(
            "{IO}ƒ a(n: u32) → () ! {{io}} ↓ n {{ ⎇ n == 0 {{ out(); }} ⇝ {{ b(n - 1); }} }}\nƒ b(n: u32) → () ! ∅ ↓ n {{ a(n); }}"
        ),
        "E0801",
    );
    // `div` truyền qua lời gọi
    only(
        "ƒ d() → () ! {div} { ⟳ { } }\nƒ f() → () ! ∅ { d(); }",
        "E0801",
    );
}

#[test]
fn effects_through_function_values() {
    ok(&format!(
        "{IO}ƒ f(g: ƒ() → () ! {{io}}) → () ! {{io}} {{ g(); }}"
    ));
    only("ƒ f(g: ƒ() → () ! {io}) → () ! ∅ { g(); }", "E0801");
    only("ƒ f(g: ƒ() → () ! {io, mem}) → () ! {io} { g(); }", "E0801");
    ok("ƒ f(g: ƒ() → () ! ∅) → () ! ∅ { g(); }");
}

#[test]
fn effects_through_trait_methods() {
    let tr = "⊏ T { ƒ m(&self) ! {io}; }\n";
    only(
        &format!("{tr}ƒ f<X: T>(x: &X) → () ! ∅ {{ x.m(); }}"),
        "E0801",
    );
    ok(&format!("{tr}ƒ f<X: T>(x: &X) → () ! {{io}} {{ x.m(); }}"));
}

#[test]
fn signatures_must_be_compatible() {
    // impl khai nhiều hiệu ứng hơn trait
    only(
        "⊏ T { ƒ m(&self) ! ∅; }\n▢ S { v: i32 }\n⊐ T for S { ƒ m(&self) ! {io} { } }",
        "E0802",
    );
    ok("⊏ T { ƒ m(&self) ! {io}; }\n▢ S { v: i32 }\n⊐ T for S { ƒ m(&self) ! ∅ { } }");
}

#[test]
fn inherent_impls_and_extra_members_do_not_hide_violations() {
    // impl thường (không trait) đứng trước, rồi impl trait vi phạm
    only(
        "⊏ T { ƒ m(&self) ! ∅; }\n▢ S { v: i32 }\n⊐ S { ƒ k(&self) ! {io} { } }\n⊐ T for S { ƒ m(&self) ! {io} { } }",
        "E0802",
    );
    // thành viên impl đứng sau một thành viên hợp lệ khác
    only(
        "⊏ T { ƒ a(&self) ! {io}; ƒ b(&self) ! ∅; }\n▢ S { v: i32 }\n⊐ T for S { ƒ a(&self) ! {io} { } ƒ b(&self) ! {mem} { } }",
        "E0802",
    );
}

#[test]
fn messages_name_declared_and_inferred_effects() {
    let src = format!("{IO}ƒ f() → () ! ∅ {{ out(); }}\nƒ g() → () ! {{mem}} {{ out(); }}");
    let (t, m) = common::mir(&src);
    let o = check_effects(&t, &m);
    let msgs: Vec<String> = o
        .report
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert!(
        msgs.iter().any(|x| x.contains("∅") && x.contains("`io`")),
        "{msgs:?}"
    );
    assert!(
        msgs.iter()
            .any(|x| x.contains("{mem}") && x.contains("`io`")),
        "{msgs:?}"
    );
}
