//! Lô 2 của chương trình golden quyết định (lệnh Giang 2026-10-08, mục 4b): rẽ nhánh theo số nguyên, quên giá trị cũ khi gán,
//! nguyên tử vị từ, ánh xạ hợp đồng, thành viên trait, ép bool→số, JSON "check còn lại". Ký hiệu như `decisions.rs`.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::analysis::Status;
use gt_oblig::driver::{analyze_program, remaining_checks_json};
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

fn check(cases: &[(&str, &str)]) {
    let mut bad = Vec::new();
    for (src, want) in cases {
        let got = run_key(src, "main::f");
        if got != *want {
            bad.push(format!("kỳ vọng [{want}], thực tế [{got}]\n  {src}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn integer_switch_refines_each_arm_and_the_otherwise_edge() {
    check(&[
        // nhánh `3 ↦` biết x = 3
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⌥ x { 3 ↦ x + 252, _ ↦ 0 } }",
            "overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⌥ x { 3 ↦ x + 253, _ ↦ 0 } }",
            "overflow:d",
        ),
        // nhánh còn lại biết x ≠ mọi hằng đã liệt kê
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⌥ x { 0 ↦ 0, _ ↦ x - 1 } }",
            "overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⌥ x { 0 ↦ 0, 1 ↦ 0, _ ↦ x - 2 } }",
            "overflow:x",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⌥ x { 0 ↦ 0, 2 ↦ 0, _ ↦ x - 2 } }",
            "overflow:g",
        ),
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⌥ x { 5 ↦ 0, _ ↦ x - 1 } }",
            "overflow:g",
        ),
        // x ∈ [0,1] và x ≠ 0 ⇒ x = 1
        (
            "ƒ f(x: u8) → u8 ! ∅ { ⎇ x < 2 { ⌥ x { 0 ↦ 0, _ ↦ x + 254 } } ⇝ { 0 } }",
            "overflow:x",
        ),
    ]);
}

#[test]
fn assigning_an_unknown_value_forgets_everything_known_about_the_variable() {
    check(&[
        // x ← xs[0]: giá trị mới không biết; nếu nhớ x = 5 thì x + 250 sẽ bị xoá NHẦM (đối chứng)
        (
            "ƒ f(xs: &[u8]) → u8 ! ∅ { ⎇ xs.len() > 0 { μx ≔ 5u8; x ← xs[0]; x + 250 } ⇝ { 0 } }",
            "index:x,overflow:g",
        ),
        (
            "ƒ f(xs: &[u8]) → u8 ! ∅ { ⎇ xs.len() > 0 { μx ≔ 5u8; x + 250 } ⇝ { 0 } }",
            "overflow:x",
        ),
        // quan hệ cũ của biến bị gán lại cũng mất
        (
            "ƒ f(xs: &[u8], i: usize, k: usize) → u8 ! ∅ { ⎇ i < xs.len() { μj ≔ i; j ← k; xs[j] } ⇝ { 0 } }",
            "index:g",
        ),
        // j ← xs.len() là chắc chắn ngoài biên
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { ⎇ i < xs.len() { μj ≔ i; j ← xs.len(); xs[j] } ⇝ { 0 } }",
            "index:d",
        ),
    ]);
}

#[test]
fn bool_to_integer_cast_and_wrapping_results_with_relations() {
    check(&[
        ("ƒ f(b: bool) → u8 ! ∅ { (b as u8) + 254 }", "overflow:x"),
        ("ƒ f(b: bool) → u8 ! ∅ { (b as u8) + 255 }", "overflow:g"),
        // i +% 1 không quấn khi i < 100 ⇒ giữ quan hệ j = i + 1 ⇒ j < len ⇒ i < len
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { ⎇ i < 100 { j ≔ i +% 1; ⎇ j < xs.len() { xs[i] } ⇝ { 0 } } ⇝ { 0 } }",
            "index:x",
        ),
        // i có thể = MAX ⇒ i +% 1 có thể quấn ⇒ KHÔNG được giữ quan hệ (đối chứng quan trọng)
        (
            "ƒ f(xs: &[u8], i: usize) → u8 ! ∅ { j ≔ i +% 1; ⎇ j < xs.len() { xs[i] } ⇝ { 0 } }",
            "index:g",
        ),
    ]);
}

#[test]
fn predicate_atoms_die_when_their_argument_is_reassigned() {
    let pre = "ƒ f(μo: Option<i32>, p: Option<i32>) → i32 ! ∅";
    check(&[
        // is_some(o) đúng, rồi o bị gán lại ⇒ phải bác (Option không Copy ⇒ E0902), không được coi là đã chứng minh
        (
            &format!("{pre} {{ ⎇ o.is_some() {{ o ← p; o.unwrap() }} ⇝ {{ 0 }} }}"),
            "pre:r",
        ),
        (
            &format!("{pre} {{ ⎇ o.is_some() {{ o.unwrap() }} ⇝ {{ 0 }} }}"),
            "pre:x",
        ),
        // vị từ của biến KHÁC không suy ra cho o
        (
            &format!("{pre} {{ ⎇ p.is_some() {{ o.unwrap() }} ⇝ {{ 0 }} }}"),
            "pre:r",
        ),
    ]);
}

#[test]
fn postcondition_mapping_of_parameters_slices_and_copy_requirements() {
    check(&[
        // ⊲ nhắc độ dài lát của tham số ⇒ ánh xạ LEN_BASE
        (
            "ƒ f(xs: &[u8]) → usize ! ∅ ⊲ result == xs.len() { xs.len() }",
            "post:x",
        ),
        (
            "ƒ f(xs: &[u8]) → usize ! ∅ ⊲ result == xs.len() { 0 }",
            "post:g",
        ),
        // hậu điều kiện phụ thuộc tham số kiểu số
        ("ƒ f(n: u8) → u8 ! ∅ ⊲ result == n { n }", "post:x"),
        ("ƒ f(n: u8) → u8 ! ∅ ⊲ result == n { 7 }", "post:g"),
        // tham số không Copy mà ⊲ chưa chứng minh ⇒ không gọi được thân hợp đồng lúc chạy
        ("ƒ f(b: Buf, n: u8) → u8 ! ∅ ⊲ result > 0 { n }", "post:r"),
        // kiểu trả về không Copy
        ("ƒ f(n: u8) → Buf ! ∅ ⊲ n > 0 { mk_buf() }", "post:r"),
        // tham số bị gán lại: KHÔNG chứng minh bằng giá trị hiện tại (result == x đúng với x hiện tại nhưng sai với x lúc vào)
        ("ƒ f(μx: u8) → u8 ! ∅ ⊲ result == x { x ← 5; x }", "post:r"),
        ("ƒ f(μx: u8) → u8 ! ∅ ⊲ result != x { x ← 5; x }", "post:r"),
        // mọi thứ Copy thì hạ runtime
        ("ƒ f(n: u8) → u8 ! ∅ ⊲ result > 0 { n }", "post:g"),
    ]);
}

#[test]
fn preconditions_are_hypotheses_only_for_directly_called_functions() {
    let tr = "⊏ Tr { ƒ m(&self, n: u8) → u8 ! ∅; }\n▢ S { x: u8 }\n";
    // thành viên của `⊐ Trait for` có thể bị gọi qua trait (không qua điểm kiểm) ⇒ ⊳ KHÔNG là giả thiết
    let via_trait =
        format!("{tr}⊐ Tr for S {{ ƒ m(&self, n: u8) → u8 ! ∅ ⊳ n > 0 {{ n - 1 }} }}\n");
    assert_eq!(
        run_key(&via_trait, "main::<S as main::Tr>::m"),
        "overflow:g"
    );
    // phương thức thuần của `⊐ S` chỉ gọi trực tiếp ⇒ ⊳ là giả thiết
    let inherent = format!("{tr}⊐ S {{ ƒ m2(&self, n: u8) → u8 ! ∅ ⊳ n > 0 {{ n - 1 }} }}\n");
    assert_eq!(run_key(&inherent, "main::S::m2"), "overflow:x");
}

#[test]
fn remaining_checks_json_has_the_exact_documented_shape() {
    let (t, m) = common::mir("ƒ f(a: u8) → u8 ! ∅ { ⎇ a < 5 { a + 1 } ⇝ { a + 1 } }");
    let o = analyze_program(&t, &m, &mut ZoneProver::new(24));
    let j = remaining_checks_json(&o);
    assert!(
        j.starts_with("{\"schema\":1,\"totals\":{\"obligations\":2,\"discharged\":1,\"remaining_runtime_checks\":1,\"rejected\":0,\"definitely_false\":0,\"literal_errors\":0,\"asserts_before\":2,\"asserts_after\":1},\"bodies\":[{\"key\":\"main::f\",\"obligations\":[{\"status\":\"discharged\",\"prover\":\"interval+zone\",\"cert\":"),
        "{j}"
    );
    assert!(j.contains("{\"status\":\"runtime_check\",\"note\":\"N0601\",\"ob\":{\"id\":\"main::f#1\",\"kind\":\"overflow\",\"body\":\"main::f\""), "{j}");
    assert!(j.ends_with("]}]}"), "{j}");
    // tất định
    let o2 = analyze_program(&t, &m, &mut ZoneProver::new(24));
    assert_eq!(j, remaining_checks_json(&o2));
}
