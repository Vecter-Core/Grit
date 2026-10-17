//! Hiệu ứng (M8 nửa 1; semantics §4): E0801 (suy ra ⊄ khai báo), E0802 (chữ ký không tương thích), kết quả CHÍNH XÁC.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_effects::check_effects;

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

// ---------------------------------------------------------------- khoá (ADR-027 §3)

const LV: &str = "#level(\"10\")\n▢ Lo { }\n#level(\"20\")\n▢ Hi { }\n#level(\"30\")\n▢ Top { }\n";

#[test]
fn lock_effects_are_inferred_and_must_be_declared() {
    // khai đúng lock(Lo)
    ok(&format!(
        "{LV}ƒ bump(v: &!u32, n: u32) → u32 ! ∅ {{ *v ← *v +% n; *v }}\nƒ go(m: &Mutex<u32, Lo>) → u32 ! {{lock(Lo)}} {{ m.with(bump, 1u32) }}"
    ));
    // thiếu khai báo lock(Lo)
    only(
        &format!(
            "{LV}ƒ bump(v: &!u32, n: u32) → u32 ! ∅ {{ *v }}\nƒ go(m: &Mutex<u32, Lo>) → u32 ! ∅ {{ m.with(bump, 1u32) }}"
        ),
        "E0801",
    );
    // khai nhầm cấp khác
    only(
        &format!(
            "{LV}ƒ bump(v: &!u32, n: u32) → u32 ! ∅ {{ *v }}\nƒ go(m: &Mutex<u32, Lo>) → u32 ! {{lock(Hi)}} {{ m.with(bump, 1u32) }}"
        ),
        "E0801",
    );
}

const STATICS: &str = "■ LO: Mutex<u32, Lo> ≔ Mutex::new(0u32);
■ HI: Mutex<u32, Hi> ≔ Mutex::new(0u32);
■ TOP: Mutex<u32, Top> ≔ Mutex::new(0u32);
ƒ leaf(v: &!u32, n: u32) → u32 ! ∅ { *v }
";

#[test]
fn nested_locks_must_have_strictly_increasing_levels() {
    // trong khoá Lo lấy khoá Hi (cấp cao hơn): hợp lệ
    ok(&format!(
        "{LV}{STATICS}ƒ inner(v: &!u32, n: u32) → u32 ! {{lock(Hi)}} {{ HI.with(leaf, 0u32) }}
ƒ go() → u32 ! {{lock(Hi), lock(Lo)}} {{ LO.with(inner, 0u32) }}"
    ));
    // ba tầng tăng dần
    ok(&format!(
        "{LV}{STATICS}ƒ t2(v: &!u32, n: u32) → u32 ! {{lock(Top)}} {{ TOP.with(leaf, 0u32) }}
ƒ t1(v: &!u32, n: u32) → u32 ! {{lock(Hi), lock(Top)}} {{ HI.with(t2, 0u32) }}
ƒ go() → u32 ! {{lock(Hi), lock(Lo), lock(Top)}} {{ LO.with(t1, 0u32) }}"
    ));
    // đảo ngược thứ tự: trong Hi lấy Lo
    only(
        &format!(
            "{LV}{STATICS}ƒ inner(v: &!u32, n: u32) → u32 ! {{lock(Lo)}} {{ LO.with(leaf, 0u32) }}
ƒ go() → u32 ! {{lock(Hi), lock(Lo)}} {{ HI.with(inner, 0u32) }}"
        ),
        "E0502",
    );
    // khoá lại cùng cấp (tự deadlock)
    only(
        &format!(
            "{LV}{STATICS}ƒ inner(v: &!u32, n: u32) → u32 ! {{lock(Lo)}} {{ LO.with(leaf, 0u32) }}
ƒ go() → u32 ! {{lock(Lo)}} {{ LO.with(inner, 0u32) }}"
        ),
        "E0502",
    );
    // đảo ngược qua nhiều tầng hàm
    only(
        &format!(
            "{LV}{STATICS}ƒ deep(v: &!u32, n: u32) → u32 ! {{lock(Lo)}} {{ LO.with(leaf, 0u32) }}
ƒ mid(v: &!u32, n: u32) → u32 ! {{lock(Lo)}} {{ deep(v, n) }}
ƒ go() → u32 ! {{lock(Hi), lock(Lo)}} {{ HI.with(mid, 0u32) }}"
        ),
        "E0502",
    );
}

#[test]
fn locks_sequentially_in_any_order_are_fine_and_spawn_does_not_nest() {
    // lần lượt (không lồng) theo thứ tự bất kỳ
    ok(&format!(
        "{LV}{STATICS}ƒ go() ! {{lock(Hi), lock(Lo)}} {{ HI.with(leaf, 0u32); LO.with(leaf, 0u32); }}"
    ));
    // worker của spawn lấy khoá thấp trong khi người gọi giữ khoá cao: không lồng, không phải E0502
    ok(&format!(
        "{LV}{STATICS}ƒ w(n: u32) ! {{lock(Lo)}} {{ LO.with(leaf, n); }}
ƒ go() ! {{lock(Hi)}} {{ HI.with(spawner, 0u32); }}
ƒ spawner(v: &!u32, n: u32) → u32 ! ∅ {{ spawn(w, 1u32); 0u32 }}"
    ));
}

#[test]
fn polymorphic_argument_effects_reach_the_caller() {
    // spawn: hiệu ứng io/mem của worker phải được người gọi khai báo
    only(
        "ƒ w(n: u32) ! {io} { }\nƒ go() ! ∅ { spawn(w, 1u32); }",
        "E0801",
    );
    ok("ƒ w(n: u32) ! {io} { }\nƒ go() ! {io} { spawn(w, 1u32); }");
    // worker có cả lock lẫn mem: lock bị bỏ (không lồng), mem vẫn truyền — kể cả khi lock đứng trước trong thứ tự
    let src = format!(
        "{LV}{STATICS}ƒ w(n: u32) ! {{lock(Lo), mem}} {{ LO.with(leaf, n); }}\nƒ go() ! {{{{}}}} {{ spawn(w, 1u32); }}"
    )
    .replace("{{}}", "∅");
    only(&src, "E0801");
    ok(&format!(
        "{LV}{STATICS}ƒ w(n: u32) ! {{lock(Lo), mem}} {{ LO.with(leaf, n); }}\nƒ go() ! {{mem}} {{ spawn(w, 1u32); }}"
    ));
    // Mutex::with: hiệu ứng của hàm bên trong (ngoài lock) truyền vào người gọi
    only(
        &format!(
            "{LV}{STATICS}ƒ f(v: &!u32, n: u32) → u32 ! {{io}} {{ *v }}\nƒ go() → u32 ! {{lock(Lo)}} {{ LO.with(f, 0u32) }}"
        ),
        "E0801",
    );
    ok(&format!(
        "{LV}{STATICS}ƒ f(v: &!u32, n: u32) → u32 ! {{io}} {{ *v }}\nƒ go() → u32 ! {{io, lock(Lo)}} {{ LO.with(f, 0u32) }}"
    ));
}

#[test]
fn calling_a_function_that_declares_lock_directly_is_fine() {
    ok(&format!(
        "{LV}{STATICS}ƒ a() ! {{lock(Lo)}} {{ LO.with(leaf, 0u32); }}\nƒ b() ! {{lock(Lo)}} {{ a(); }}"
    ));
}

#[test]
fn two_level_types_with_the_same_short_name_are_ambiguous() {
    only(
        "▦ m1 { #level(\"1\") ◈ ▢ X { } }\n▦ m2 { #level(\"2\") ◈ ▢ X { } }",
        "E0502",
    );
}

/// FAIL_OPEN_AUDIT (gt-effects dòng 50): thân hàm KHÔNG tìm thấy chữ ký (`sig_by_node` = None) từng bị `continue` bỏ qua ⇒ hiệu ứng của thân không được kiểm.
/// db không nhất quán ⇒ phải là ICE E9901 (dựng trong bộ nhớ: đổi `node` của thân thành NodeId không có trong db).
#[test]
fn body_without_a_signature_is_an_internal_error_not_skipped() {
    let (t, mut m) = common::mir("ƒ out() → () ! {io} { }\nƒ f() → () ! ∅ { out(); }");
    // đối chứng: nguyên vẹn ⇒ E0801 (gọi io trong hàm ∅)
    assert!(
        check_effects(&t, &m)
            .report
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "E0801")
    );
    let i = m
        .bodies
        .iter()
        .position(|b| b.key.ends_with("::f"))
        .expect("thân f");
    m.bodies[i].node = gt_ast::NodeId::new(gt_span::FileId::new(0), 9_999_999);
    let codes: Vec<String> = check_effects(&t, &m)
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    assert!(
        codes.iter().any(|c| c == "E9901"),
        "thân không có chữ ký bị bỏ qua lặng lẽ: {codes:?}"
    );
}

/// FB-15 (lỗi THẬT, đạt tới được từ Grit): hàm KẾT HỢP dùng làm giá trị (`B::talk`) từng mang kiểu hàm với hiệu ứng RỖNG (`apply_pick_types` gán `Vec::new()`), nên gọi qua biến
/// làm mất hiệu ứng khai báo (`io`, `mem`…) và một hàm `! ∅` thực hiện được I/O mà không bị E0801.
#[test]
fn associated_function_used_as_a_value_keeps_its_declared_effects() {
    let prelude = "▢ B { v: u32 }\n⊐ B {\n  ƒ talk(&self) ! {io} { }\n}\n";
    // gọi trực tiếp (đối chứng): E0801
    only(&format!("{prelude}ƒ g(b: &B) ! ∅ {{ b.talk(); }}"), "E0801");
    // lấy làm giá trị rồi gọi: cũng PHẢI là E0801
    only(
        &format!("{prelude}ƒ g(b: &B) ! ∅ {{\n  f ≔ B::talk;\n  f(b)\n}}"),
        "E0801",
    );
    // khai báo đủ hiệu ứng thì sạch
    ok(&format!(
        "{prelude}ƒ g(b: &B) ! {{io}} {{\n  f ≔ B::talk;\n  f(b)\n}}"
    ));
}
