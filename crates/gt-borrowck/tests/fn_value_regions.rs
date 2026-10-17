//! R2a (lệnh Giang 2026-10-17): M6 qua GIÁ TRỊ HÀM. Kiểu hàm trong `Ty` KHÔNG mang region (ADR-019/020) ⇒ khi gọi qua giá trị hàm (`Callee::Value`) borrowck không có `RegionSig`
//! và đi đường BẢO THỦ: kết quả có thể mượn từ MỌI đối số, mọi đối số outlive nhau (ADR-042). Test khoá hành vi: ca SAI qua giá trị hàm vẫn bị từ chối đúng mã (sound);
//! ca đúng của `pick2` qua giá trị hàm bị từ chối oan (không complete, có ghi chép); ca đúng đơn giản vẫn qua. Oracle động: kịch bản `fn_value_call` ở `oracle.rs`.
mod common;
use common::*;

const IDR: &str = "ƒ idr<'a>(x: &'a i32) → &'a i32 ! ∅ { x }\nƒ use_i(x: i32) ! ∅ { }\n";
const PICK: &str =
    "ƒ pick<'a>(x: &'a i32, y: &'a i32) → &'a i32 ! ∅ { x }\nƒ use_i(x: i32) ! ∅ { }\n";
const PICK2: &str =
    "ƒ pick2<'a, 'b>(x: &'a i32, y: &'b i32) → &'a i32 ! ∅ { x }\nƒ use_i(x: i32) ! ∅ { }\n";

fn codes_of(src: &str) -> Vec<String> {
    let r = bc(src);
    bc_codes(&r)
}

#[test]
fn wrong_programs_are_rejected_through_function_values_with_the_right_code() {
    // kết quả sống lâu hơn biến cục bộ (trực tiếp và qua giá trị): E0404
    assert_eq!(
        codes_of(&format!(
            "{IDR}ƒ main() ! ∅ {{\n  r ≔ {{ y ≔ 5; idr(&y) }};\n  use_i(*r);\n}}"
        )),
        vec!["E0404"]
    );
    assert_eq!(
        codes_of(&format!(
            "{IDR}ƒ main() ! ∅ {{\n  f ≔ idr;\n  r ≔ {{ y ≔ 5; f(&y) }};\n  use_i(*r);\n}}"
        )),
        vec!["E0404"]
    );
    // hai đối số, kết quả buộc với cả hai
    assert_eq!(
        codes_of(&format!(
            "{PICK}ƒ main() ! ∅ {{\n  f ≔ pick;\n  a ≔ 1;\n  r ≔ {{ b ≔ 2; f(&a, &b) }};\n  use_i(*r);\n}}"
        )),
        vec!["E0404"]
    );
    // truyền hàm qua tham số kiểu hàm
    assert_eq!(
        codes_of(&format!(
            "ƒ apply(f: ƒ(&i32) → &i32 ! ∅, x: &i32) → &i32 ! ∅ {{ f(x) }}\n{IDR}ƒ main() ! ∅ {{\n  r ≔ {{ y ≔ 5; apply(idr, &y) }};\n  use_i(*r);\n}}"
        )),
        vec!["E0404"]
    );
    // hai `&!` cùng một biến, hoặc `&` rồi `&!`, qua giá trị hàm
    assert_eq!(
        codes_of(
            "ƒ two(a: &!i32, b: &!i32) ! ∅ { *a ← 1; *b ← 2; }\nƒ main() ! ∅ {\n  f ≔ two;\n  μx ≔ 0;\n  f(&!x, &!x);\n}"
        ),
        vec!["E0402"]
    );
    assert_eq!(
        codes_of(
            "ƒ mix(a: &i32, b: &!i32) ! ∅ { *b ← *a; }\nƒ main() ! ∅ {\n  f ≔ mix;\n  μx ≔ 0;\n  f(&x, &!x);\n}"
        ),
        vec!["E0403"]
    );
    // reset Arena khi kết quả của lời gọi qua giá trị còn sống: E0405
    assert_eq!(
        codes_of(&format!(
            "{IDR}ƒ main() ! {{mem}} {{\n  μa ≔ Arena::new();\n  p ≔ a.alloc(7);\n  f ≔ idr;\n  r ≔ f(p);\n  a.reset();\n  use_i(*r);\n}}"
        )),
        vec!["E0405"]
    );
}

#[test]
fn pick2_through_a_function_value_is_rejected_conservatively_but_directly_it_passes() {
    // trực tiếp: RegionSig nói kết quả chỉ mượn từ `x` ⇒ qua
    assert_eq!(
        codes_of(&format!(
            "{PICK2}ƒ main() ! ∅ {{\n  a ≔ 1;\n  r ≔ {{ b ≔ 2; pick2(&a, &b) }};\n  use_i(*r);\n}}"
        )),
        Vec::<String>::new()
    );
    // qua giá trị hàm: KHÔNG có RegionSig ⇒ bảo thủ ⇒ E0404 (sound, không complete; ADR-042)
    assert_eq!(
        codes_of(&format!(
            "{PICK2}ƒ main() ! ∅ {{\n  f ≔ pick2;\n  a ≔ 1;\n  r ≔ {{ b ≔ 2; f(&a, &b) }};\n  use_i(*r);\n}}"
        )),
        vec!["E0404"]
    );
}

#[test]
fn correct_simple_programs_still_pass_through_function_values() {
    assert_eq!(
        codes_of(&format!(
            "{IDR}ƒ main() ! ∅ {{\n  f ≔ idr;\n  x ≔ 5;\n  r ≔ f(&x);\n  use_i(*r);\n}}"
        )),
        Vec::<String>::new()
    );
    assert_eq!(
        codes_of(&format!(
            "ƒ apply(f: ƒ(&i32) → &i32 ! ∅, x: &i32) → &i32 ! ∅ {{ f(x) }}\n{IDR}ƒ main() ! ∅ {{\n  x ≔ 5;\n  r ≔ apply(idr, &x);\n  use_i(*r);\n}}"
        )),
        Vec::<String>::new()
    );
}
