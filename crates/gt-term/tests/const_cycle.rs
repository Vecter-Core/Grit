//! GÓI 12 P3.a — FB-C36 (nghiêm trọng, âm tính giả, D12): vòng tham chiếu QUA HẰNG `□`. Backend LLVM hạ mỗi lần dùng `□ C` thành MỘT LỜI GỌI hàm tính `C`
//! (`gt-codegen-llvm` `Const::Item` ⇒ `build_call`), nên `□ C ≔ C`, `□ A ≔ B; □ B ≔ A` hay `□ C ≔ f()` với `f` đọc `C` là ĐỆ QUY VÔ HẠN. `gt-term` chỉ
//! dựng đồ thị từ thân `BodyKind::Fn` ⇒ `gritc check` XANH, `gritc build` sinh exe, chạy thì tràn ngăn xếp (đo ngày 2026-11-20, Windows: exit 127 khi chạy
//! thẳng, `gritc run` ⇒ mã thoát 101; thông dịch: lỗi nội bộ "hằng ngoài phạm vi thông dịch" — mã thoát 2 cho MỌI hằng có initializer không phải literal).
//! Mọi chương trình dưới đây có chu trình đi qua một thân `Const` ⇒ KHÔNG dừng ⇒ PHẢI bị ĐỎ (E0703). Bộ sinh tự quyết dừng/không (không hỏi gritc).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::check_termination;

fn codes(src: &str) -> Vec<String> {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        common::mir_codes(&m)
    );
    let info = build_prog_info(&t, &m);
    let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
    o.report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect()
}

fn rejected(src: &str) {
    let c = codes(src);
    assert!(
        c.iter().any(|x| x == "E0703"),
        "vòng qua hằng được CHẤP NHẬN (FB-C36): {c:?}\n{src}"
    );
}

#[test]
fn a_const_that_reads_itself_is_rejected() {
    rejected("□ C: u32 ≔ C;\nƒ main() → i32 ! ∅ { r ≔ C; 0 }\n");
}

#[test]
fn two_consts_reading_each_other_are_rejected() {
    rejected("□ A: u32 ≔ B;\n□ B: u32 ≔ A;\nƒ main() → i32 ! ∅ { r ≔ A; 0 }\n");
}

#[test]
fn a_const_initialised_by_a_function_that_reads_the_const_is_rejected() {
    rejected("ƒ f() → u32 ! ∅ { C }\n□ C: u32 ≔ f();\nƒ main() → i32 ! ∅ { r ≔ C; 0 }\n");
}

#[test]
fn a_const_cycle_through_a_precondition_is_rejected() {
    rejected(
        "ƒ f(n: u32) → u32 ! ∅ ⊳ C > 0u32 { n }\n□ C: u32 ≔ f(1u32);\nƒ main() → i32 ! ∅ { r ≔ C; 0 }\n",
    );
}

/// Đối chứng: không có chu trình thì KHÔNG đỏ oan.
#[test]
fn acyclic_consts_stay_accepted() {
    let c = codes(
        "□ A: u32 ≔ 1u32;\n□ B: u32 ≔ A;\nƒ g() → u32 ! ∅ { B }\n□ D: u32 ≔ g();\nƒ main() → i32 ! ∅ { r ≔ D; 0 }\n",
    );
    assert!(c.is_empty(), "{c:?}");
}

#[test]
fn a_const_calling_a_function_that_reads_another_const_stays_accepted() {
    let c = codes(
        "□ K: u32 ≔ 5u32;\nƒ f() → u32 ! ∅ { K }\n□ C: u32 ≔ f();\nƒ main() → i32 ! ∅ { r ≔ C; 0 }\n",
    );
    assert!(c.is_empty(), "{c:?}");
}

/// Đột biến GÓI 12 (`mut.py`, 5 ca sống sót ở `const_cycles`) ⇒ test giết: hằng là toán hạng PHẢI của phép toán, là ĐỐI SỐ của lời gọi, và vòng qua phương
/// thức trait (cạnh `TraitMethod` nối tới mọi impl).
#[test]
fn a_const_as_the_right_operand_of_an_operator_is_rejected() {
    rejected(
        "□ C: u32 ≔ 1u32 + C;
ƒ main() → i32 ! ∅ { r ≔ C; 0 }
",
    );
}

#[test]
fn a_const_passed_as_a_call_argument_is_rejected() {
    rejected(
        "ƒ id(x: u32) → u32 ! ∅ { x }
□ C: u32 ≔ id(C);
ƒ main() → i32 ! ∅ { r ≔ C; 0 }
",
    );
}

#[test]
fn a_const_cycle_through_a_trait_method_is_rejected() {
    rejected(
        "⊏ T { ƒ v(&self) → u32 ! ∅; }
▢ S { a: u32 }
⊐ T for S { ƒ v(&self) → u32 ! ∅ { C } }
ƒ h<X: T>(x: &X) → u32 ! ∅ { x.v() }
ƒ g() → u32 ! ∅ { s ≔ S { a: 1u32 }; h(&s) }
□ C: u32 ≔ g();
ƒ main() → i32 ! ∅ { r ≔ C; 0 }
",
    );
}

#[test]
fn a_trait_method_that_does_not_read_the_const_stays_accepted() {
    let c = codes(
        "⊏ T { ƒ v(&self) → u32 ! ∅; }
▢ S { a: u32 }
⊐ T for S { ƒ v(&self) → u32 ! ∅ { self.a } }
ƒ h<X: T>(x: &X) → u32 ! ∅ { x.v() }
ƒ g() → u32 ! ∅ { s ≔ S { a: 1u32 }; h(&s) }
□ C: u32 ≔ g();
ƒ main() → i32 ! ∅ { r ≔ C; 0 }
",
    );
    assert!(c.is_empty(), "{c:?}");
}

const TYPES: [&str; 4] = ["u32", "i32", "u64", "i64"];

fn lit(ty: &str) -> String {
    format!("1{ty}")
}

fn pads(k: usize, ty: &str) -> String {
    (0..k)
        .map(|i| format!("ƒ pad{i}() → {ty} ! ∅ {{ {} }}\n", lit(ty)))
        .collect()
}

/// 5 hình dạng × 32 chương trình (4 kiểu × 8 số hàm đệm). Mô hình phía bộ sinh: có chu trình qua thân `Const` ⇒ không dừng ⇒ PHẢI đỏ.
fn cyclic_family(shape: usize, ty_i: usize, pad: usize) -> String {
    let ty = TYPES[ty_i];
    let p = pads(pad, ty);
    let main = "ƒ main() → i32 ! ∅ { r ≔ C0; 0 }\n";
    match shape {
        // 0: tự tham chiếu
        0 => format!("{p}□ C0: {ty} ≔ C0;\n{main}"),
        // 1: vòng hằng độ dài 2..=5 (độ dài chọn theo pad)
        1 => {
            let n = 2 + pad % 4;
            let mut s = p;
            for i in 0..n {
                s += &format!("□ C{i}: {ty} ≔ C{};\n", (i + 1) % n);
            }
            s + main
        }
        // 2: C0 ≔ f0(); f_i gọi f_{i+1}; f_last đọc C0 (chuỗi hàm dài 1..=4)
        2 => {
            let n = 1 + pad % 4;
            let mut s = p;
            for i in 0..n {
                let body = if i + 1 == n {
                    "C0".to_string()
                } else {
                    format!("f{}()", i + 1)
                };
                s += &format!("ƒ f{i}() → {ty} ! ∅ {{ {body} }}\n");
            }
            s + &format!("□ C0: {ty} ≔ f0();\n{main}")
        }
        // 3: vòng qua tiền điều kiện của hàm: f ⊳ C0 > 0, C0 ≔ f(1)
        3 => {
            let zero = format!("0{ty}");
            format!(
                "{p}ƒ f(n: {ty}) → {ty} ! ∅ ⊳ C0 > {zero} {{ n }}
□ C0: {ty} ≔ f({});
{main}",
                lit(ty)
            )
        }
        // 4: hằng thứ hai đọc hằng vòng: C1 ≔ C0 + vòng C0 ≔ g() với g đọc C1
        _ => format!("{p}ƒ g() → {ty} ! ∅ {{ C1 }}\n□ C1: {ty} ≔ C0;\n□ C0: {ty} ≔ g();\n{main}"),
    }
}

/// Số ca bị chấp nhận SAI (bản sửa tắt ⇒ phải là 160/160; bật ⇒ 0).
#[test]
fn generated_const_cycles_are_all_rejected() {
    let mut accepted = Vec::new();
    let mut total = 0;
    for shape in 0..5 {
        let mut per_shape = 0;
        for (ty_i, ty) in TYPES.iter().enumerate() {
            for pad in 0..8 {
                let src = cyclic_family(shape, ty_i, pad);
                total += 1;
                per_shape += 1;
                let c = codes(&src);
                if !c.iter().any(|x| x == "E0703") {
                    accepted.push(format!("hình {shape}, {ty}, pad {pad}"));
                }
            }
        }
        assert_eq!(per_shape, 32);
    }
    assert_eq!(total, 160);
    assert!(
        accepted.is_empty(),
        "{} / {total} chương trình vòng-qua-hằng bị chấp nhận (FB-C36): {accepted:?}",
        accepted.len()
    );
}

/// Đối chứng sinh: cùng kiểu/đệm nhưng KHÔNG có chu trình ⇒ không có E0703.
#[test]
fn generated_acyclic_controls_are_not_rejected() {
    for ty in TYPES {
        for pad in 0..8 {
            let src = format!(
                "{}□ K: {ty} ≔ {};\nƒ f() → {ty} ! ∅ {{ K }}\n□ C0: {ty} ≔ f();\nƒ main() → i32 ! ∅ {{ r ≔ C0; 0 }}\n",
                pads(pad, ty),
                lit(ty)
            );
            let c = codes(&src);
            assert!(c.is_empty(), "{c:?}\n{src}");
        }
    }
}
