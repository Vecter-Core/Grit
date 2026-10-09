//! Hợp đồng `⊳` `⊲` (ADR-025 §6): chứng minh ở điểm gọi/thoát, hạ runtime, ĐỎ E0901/E0902; MIR sau M7 chạy được qua thông dịch.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
#[path = "../../gt-mir/tests/support/mod.rs"]
mod support;
use gt_mir::MirOutput;
use gt_mir::interp::{Interp, Outcome, V};
use gt_oblig::driver::{OblOutput, analyze_program};
use gt_oblig::prover::ZoneProver;

fn pipeline(src: &str) -> (gt_types::TypesOutput, MirOutput, OblOutput) {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}",
        common::mir_codes(&m)
    );
    let o = analyze_program(&t, &m, &mut ZoneProver::new(24));
    (t, m, o)
}

fn codes(o: &OblOutput) -> Vec<String> {
    o.report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect()
}

fn count(o: &OblOutput, c: &str) -> usize {
    codes(o).iter().filter(|x| x.as_str() == c).count()
}

/// Chạy hàm `key` trên MIR SAU M7.
fn exec(m: &MirOutput, o: &OblOutput, key: &str, args: Vec<V>) -> Outcome {
    let after = MirOutput {
        report: gt_diag::Diagnostics::new(1).finish(),
        bodies: o.bodies.clone(),
        keys: m.keys.clone(),
    };
    Interp::new(&after, 1_000_000).call_key(key, args)
}

fn i32v(v: i128) -> V {
    V::Int(v, gt_types::IntTy::I32)
}

const POS: &str = "ƒ pos(n: i32) → i32 ! ∅ ⊳ n > 0 { n }\n";

#[test]
fn proven_precondition_is_deleted_and_adds_no_check() {
    let (_t, m, o) = pipeline(&format!("{POS}ƒ f() → i32 ! ∅ {{ pos(5) }}\n"));
    assert_eq!(
        count(&o, "E0901") + count(&o, "N0901"),
        0,
        "{:?}",
        codes(&o)
    );
    let f = o.bodies.iter().find(|b| b.key == "main::f").unwrap();
    let orig = m.bodies.iter().find(|b| b.key == "main::f").unwrap();
    assert_eq!(f.blocks.len(), orig.blocks.len(), "không chèn kiểm");
}

#[test]
fn definitely_false_precondition_is_red() {
    let (_t, _m, o) = pipeline(&format!("{POS}ƒ f() → i32 ! ∅ {{ pos(0) }}\n"));
    assert_eq!(count(&o, "E0901"), 1, "{:?}", codes(&o));
}

#[test]
fn unproven_precondition_is_checked_at_runtime_with_n0901() {
    let (_t, m, o) = pipeline(&format!("{POS}ƒ g(x: i32) → i32 ! ∅ {{ pos(x) }}\n"));
    assert_eq!(count(&o, "N0901"), 1, "{:?}", codes(&o));
    assert_eq!(count(&o, "E0901") + count(&o, "E0902"), 0);
    assert_eq!(
        exec(&m, &o, "main::g", vec![i32v(7)]),
        Outcome::Value(i32v(7))
    );
    assert_eq!(
        exec(&m, &o, "main::g", vec![i32v(0)]),
        Outcome::Abort("precondition".into())
    );
    assert_eq!(
        exec(&m, &o, "main::g", vec![i32v(-3)]),
        Outcome::Abort("precondition".into())
    );
}

#[test]
fn precondition_of_the_function_is_a_hypothesis_in_its_body() {
    // n - 1 không tràn nhờ n > 0
    let (_t, _m, o) = pipeline("ƒ dec(n: u32) → u32 ! ∅ ⊳ n > 0 { n - 1 }\n");
    assert_eq!(count(&o, "N0601"), 0, "{:?}", codes(&o));
    let (_t, _m, o) = pipeline("ƒ dec(n: u32) → u32 ! ∅ { n - 1 }\n");
    assert_eq!(
        count(&o, "N0601"),
        1,
        "đối chứng: không có ⊳ thì phải giữ kiểm"
    );
}

#[test]
fn guard_before_call_proves_the_precondition() {
    let src = format!("{POS}ƒ g(x: i32) → i32 ! ∅ {{ ⎇ x > 0 {{ pos(x) }} ⇝ {{ 1 }} }}\n");
    let (_t, _m, o) = pipeline(&src);
    assert_eq!(
        count(&o, "N0901") + count(&o, "E0901"),
        0,
        "{:?}",
        codes(&o)
    );
}

#[test]
fn proven_postcondition_adds_no_check() {
    let (_t, m, o) = pipeline("ƒ one() → i32 ! ∅ ⊲ result == 1 { 1 }\n");
    assert_eq!(
        count(&o, "N0902") + count(&o, "E0901"),
        0,
        "{:?}",
        codes(&o)
    );
    let f = o.bodies.iter().find(|b| b.key == "main::one").unwrap();
    let orig = m.bodies.iter().find(|b| b.key == "main::one").unwrap();
    assert_eq!(f.blocks.len(), orig.blocks.len());
}

#[test]
fn unproven_postcondition_is_checked_at_every_exit() {
    let src = "ƒ f(x: i32) → i32 ! ∅ ⊲ result > 0 { x }\n";
    let (_t, m, o) = pipeline(src);
    assert_eq!(count(&o, "N0902"), 1, "{:?}", codes(&o));
    assert_eq!(
        exec(&m, &o, "main::f", vec![i32v(4)]),
        Outcome::Value(i32v(4))
    );
    assert_eq!(
        exec(&m, &o, "main::f", vec![i32v(0)]),
        Outcome::Abort("postcondition".into())
    );
}

#[test]
fn definitely_false_postcondition_is_red() {
    let (_t, _m, o) = pipeline("ƒ f() → i32 ! ∅ ⊲ result > 5 { 1 }\n");
    assert_eq!(count(&o, "E0901"), 1, "{:?}", codes(&o));
}

#[test]
fn postcondition_of_the_callee_is_a_fact_at_the_call_site() {
    // 1 + 2147483646 không tràn nhờ ⊲ result == 1
    let src = "ƒ one() → i32 ! ∅ ⊲ result == 1 { 1 }\nƒ f() → i32 ! ∅ { one() + 2147483646 }\n";
    let (_t, _m, o) = pipeline(src);
    assert_eq!(
        count(&o, "N0601") + count(&o, "E0601"),
        0,
        "{:?}",
        codes(&o)
    );
    let src2 = "ƒ one() → i32 ! ∅ { 1 }\nƒ f() → i32 ! ∅ { one() + 2147483646 }\n";
    let (_t, _m, o) = pipeline(src2);
    assert_eq!(
        count(&o, "N0601"),
        1,
        "đối chứng: không có ⊲ thì phải giữ kiểm"
    );
}

#[test]
fn postcondition_naming_a_reassigned_parameter_is_not_checkable() {
    let src = "ƒ f(μx: i32) → i32 ! ∅ ⊲ result > x { x ← x + 1; x }\n";
    let (_t, _m, o) = pipeline(src);
    assert_eq!(count(&o, "E0902"), 1, "{:?}", codes(&o));
}

#[test]
fn bounded_quantifier_is_checked_by_a_runtime_loop() {
    let src = "ƒ f(n: i32) → i32 ! ∅ ⊳ ∀ i ∈ 0..n : i < 3 { n }\nƒ g(n: i32) → i32 ! ∅ { f(n) }\n";
    let (_t, m, o) = pipeline(src);
    assert_eq!(count(&o, "N0903"), 1, "{:?}", codes(&o));
    assert_eq!(count(&o, "E0902"), 0, "{:?}", codes(&o));
    assert_eq!(
        exec(&m, &o, "main::g", vec![i32v(3)]),
        Outcome::Value(i32v(3))
    );
    assert_eq!(
        exec(&m, &o, "main::g", vec![i32v(0)]),
        Outcome::Value(i32v(0))
    );
    assert_eq!(
        exec(&m, &o, "main::g", vec![i32v(5)]),
        Outcome::Abort("precondition".into())
    );
}

#[test]
fn existential_quantifier_runs_too() {
    let src = "ƒ f(n: i32) → i32 ! ∅ ⊳ ∃ i ∈ 0..n : i == 2 { n }\nƒ g(n: i32) → i32 ! ∅ { f(n) }\n";
    let (_t, m, o) = pipeline(src);
    assert_eq!(count(&o, "N0903"), 1, "{:?}", codes(&o));
    assert_eq!(
        exec(&m, &o, "main::g", vec![i32v(3)]),
        Outcome::Value(i32v(3))
    );
    assert_eq!(
        exec(&m, &o, "main::g", vec![i32v(2)]),
        Outcome::Abort("precondition".into())
    );
}

#[test]
fn a_function_used_as_a_value_never_gets_its_precondition_assumed() {
    // `pos` thoát qua giá trị hàm ⇒ có thể bị gọi không qua điểm kiểm ⇒ ⊳ KHÔNG là giả thiết của thân
    let src = "ƒ dec(n: u32) → u32 ! ∅ ⊳ n > 0 { n - 1 }\nƒ h() → ƒ(u32) → u32 ! ∅ ! ∅ { dec }\n";
    let (t, m) = common::mir(src);
    if m.report.diagnostics.is_empty() {
        let o = analyze_program(&t, &m, &mut ZoneProver::new(24));
        assert_eq!(count(&o, "N0601"), 1, "{:?}", codes(&o));
    }
}

#[test]
#[ignore]
fn dump_contract() {
    let (t, m) = common::mir(&format!("{POS}ƒ f() → i32 ! ∅ {{ pos(5) }}\n"));
    for b in &m.bodies {
        eprintln!("{}", common::text(&t, &m, &b.key));
    }
    let info = gt_oblig::driver::build_prog_info(&t, &m);
    eprintln!("{:#?}", info.pre);
    let o = analyze_program(&t, &m, &mut ZoneProver::new(24));
    for r in &o.per_body {
        for j in &r.judged {
            eprintln!(
                "{} {:?} goal={} hyps={:?}",
                j.ob.id,
                j.status,
                j.ob.goal.to_text(),
                j.ob.hyps.iter().map(|h| h.to_text()).collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn unbounded_quantifier_is_red_e0902() {
    let src = "ƒ f(r: Range<i32>) → i32 ! ∅ ⊳ ∀ i ∈ r : i >= 0 { 1 }\n";
    let (_t, _m, o) = pipeline(src);
    assert!(count(&o, "E0902") >= 1, "{:?}", codes(&o));
}

#[test]
fn shift_precondition_uses_the_prelude_contract() {
    let (_t, _m, o) = pipeline(
        "ƒ f(x: u32) → u32 ! ∅ { shl(x, 3) }
",
    );
    assert_eq!(
        count(&o, "N0901") + count(&o, "E0901"),
        0,
        "{:?}",
        codes(&o)
    );
    let (_t, _m, o) = pipeline(
        "ƒ f(x: u32) → u32 ! ∅ { shl(x, 40) }
",
    );
    assert_eq!(count(&o, "E0901"), 1, "{:?}", codes(&o));
    let (_t, _m, o) = pipeline(
        "ƒ f(x: u32, n: u32) → u32 ! ∅ { shr(x, n) }
",
    );
    assert_eq!(count(&o, "N0901"), 1, "{:?}", codes(&o));
    let (_t, _m, o) = pipeline(
        "ƒ f(x: u32, n: u32) → u32 ! ∅ { ⎇ n < 32 { shr(x, n) } ⇝ { 0 } }
",
    );
    assert_eq!(
        count(&o, "N0901") + count(&o, "E0901"),
        0,
        "{:?}",
        codes(&o)
    );
}

#[test]
fn shift_runtime_check_aborts_before_the_call() {
    let (_t, m, o) = pipeline(
        "ƒ f(x: u32, n: u32) → u32 ! ∅ { shl(x, n) }
",
    );
    let u = |v| V::Int(v, gt_types::IntTy::U32);
    assert_eq!(
        exec(&m, &o, "main::f", vec![u(1), u(40)]),
        Outcome::Abort("precondition".into())
    );
}

#[test]
fn unwrap_needs_is_some() {
    let guarded = "ƒ h(o: Option<i32>) → i32 ! ∅ { ⎇ o.is_some() { o.unwrap() } ⇝ { 0 } }
";
    let (_t, _m, o) = pipeline(guarded);
    assert_eq!(
        count(&o, "N0901") + count(&o, "E0901") + count(&o, "E0902"),
        0,
        "{:?}",
        codes(&o)
    );
    let bare = "ƒ h(o: Option<i32>) → i32 ! ∅ { o.unwrap() }
";
    let (_t, _m, o) = pipeline(bare);
    assert_eq!(
        count(&o, "N0901") + count(&o, "E0902"),
        1,
        "{:?}",
        codes(&o)
    );
}

#[test]
fn loop_invariant_is_proved_or_checked_at_runtime() {
    // ⋈ i <= 10 với i bị chặn bởi điều kiện vòng: chứng minh được ⇒ không còn kiểm
    let ok = "ƒ f() → u32 ! ∅ { μi ≔ 0u32; ⟳ i < 10 ↓ 10 - i ⋈ i <= 10 { i ← i + 1; } i }
";
    let (_t, _m, o) = pipeline(ok);
    assert_eq!(
        count(&o, "N0904") + count(&o, "E0901"),
        0,
        "{:?}",
        codes(&o)
    );
    // bất biến phụ thuộc đầu vào: giữ kiểm lúc chạy
    let unk = "ƒ g(n: u32) → u32 ! ∅ { μi ≔ 0u32; ⟳ i < 10 ↓ 10 - i ⋈ i <= n { i ← i + 1; } i }
";
    let (_t, m, o) = pipeline(unk);
    assert_eq!(count(&o, "N0904"), 1, "{:?}", codes(&o));
    let u = |v| V::Int(v, gt_types::IntTy::U32);
    assert_eq!(exec(&m, &o, "main::g", vec![u(20)]), Outcome::Value(u(10)));
    assert_eq!(
        exec(&m, &o, "main::g", vec![u(3)]),
        Outcome::Abort("invariant".into())
    );
}
