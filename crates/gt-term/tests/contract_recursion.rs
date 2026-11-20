//! GÓI 11 S1.f — FB-C35 (nghiêm trọng, âm tính giả, D12): đệ quy QUA THÂN HỢP ĐỒNG. Kiểm tra `⊳`/`⊲` chạy lúc chạy (chèn SAU M8) và gọi hàm người
//! dùng; `gt-term` chỉ dựng đồ thị từ thân `BodyKind::Fn` nên cạnh "hàm → hàm được gọi trong hợp đồng của nó" bị bỏ. `ƒ f(n) ⊳ g(n)` với `g` gọi lại `f`
//! từng được chấp nhận (`gritc check` XANH) nhưng không dừng khi chạy (thông dịch: hết nhiên liệu, mã 71). Các chương trình dưới đây PHẢI bị ĐỎ (E0703).
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
        c.iter().any(|x| x == "E0703" || x == "E0702"),
        "đệ quy qua hợp đồng được CHẤP NHẬN (FB-C35): {c:?}\n{src}"
    );
}

#[test]
fn recursion_through_a_precondition_is_rejected() {
    rejected(
        "ƒ f(n: u32) → u32 ! ∅ ⊳ g(n) { n }\nƒ g(n: u32) → bool ! ∅ { f(n) > 0u32 }\nƒ main() → i32 ! ∅ { r ≔ f(1u32); 0 }\n",
    );
}

#[test]
fn recursion_through_a_postcondition_is_rejected() {
    rejected(
        "ƒ f(n: u32) → u32 ! ∅ ⊲ g(result) { n }\nƒ g(n: u32) → bool ! ∅ { f(n) > 0u32 }\nƒ main() → i32 ! ∅ { r ≔ f(1u32); 0 }\n",
    );
}

#[test]
fn a_contract_calling_its_own_function_is_rejected() {
    rejected("ƒ f(n: u32) → u32 ! ∅ ⊳ f(n) > 0u32 { n }\nƒ main() → i32 ! ∅ { r ≔ f(1u32); 0 }\n");
}

#[test]
fn recursion_through_a_contract_in_a_longer_cycle_is_rejected() {
    rejected(
        "ƒ f(n: u32) → u32 ! ∅ ⊳ g(n) { n }\nƒ g(n: u32) → bool ! ∅ { h(n) }\nƒ h(n: u32) → bool ! ∅ { f(n) > 0u32 }\nƒ main() → i32 ! ∅ { r ≔ f(1u32); 0 }\n",
    );
}

/// Đối chứng: hợp đồng gọi hàm KHÔNG quay lại thì vẫn được chấp nhận (không đỏ oan).
#[test]
fn a_contract_calling_a_non_recursive_helper_stays_accepted() {
    let c = codes(
        "ƒ pos(n: u32) → bool ! ∅ { n > 0u32 }\nƒ f(n: u32) → u32 ! ∅ ⊳ pos(n) { n }\nƒ main() → i32 ! ∅ { r ≔ f(1u32); 0 }\n",
    );
    assert!(c.is_empty(), "{c:?}");
}

/// Bộ sinh hình dạng (GÓI 11 S1.e): đệ quy qua hợp đồng với độ dài chu trình 1–4, kiểu tham số khác nhau và 0–7 hàm đệm; MỖI hình dạng 32 chương trình.
/// Mô hình phía bộ sinh: chương trình có chu trình qua hợp đồng và KHÔNG có `↓`/`div` ⇒ không dừng ⇒ PHẢI bị ĐỎ (không gọi gritc để quyết).
fn contract_family(shape: usize, ty: &str, pad: usize) -> String {
    let mut s = String::new();
    // hàm gọi trực tiếp một hàm khác trong thân: chuỗi k bước quay lại f
    let chain = |hops: usize| -> String {
        // g1 → g2 → ... → f (hops hàm trung gian, hàm cuối gọi f)
        let mut o = String::new();
        for i in 1..=hops {
            let callee = if i == hops {
                "f".to_string()
            } else {
                format!("g{}", i + 1)
            };
            if i == hops {
                o.push_str(&format!("ƒ g{i}(n: {ty}) → bool ! ∅ {{ f(n) > 0{ty} }}\n"));
            } else {
                o.push_str(&format!("ƒ g{i}(n: {ty}) → bool ! ∅ {{ {callee}(n) }}\n"));
            }
        }
        o
    };
    match shape {
        0 => {
            s.push_str(&format!("ƒ f(n: {ty}) → {ty} ! ∅ ⊳ g1(n) {{ n }}\n"));
            s.push_str(&chain(1));
        }
        1 => {
            s.push_str(&format!("ƒ f(n: {ty}) → {ty} ! ∅ ⊲ g1(result) {{ n }}\n"));
            s.push_str(&chain(1));
        }
        2 => {
            s.push_str(&format!("ƒ f(n: {ty}) → {ty} ! ∅ ⊳ g1(n) {{ n }}\n"));
            s.push_str(&chain(3));
        }
        3 => s.push_str(&format!("ƒ f(n: {ty}) → {ty} ! ∅ ⊳ f(n) > 0{ty} {{ n }}\n")),
        _ => {
            s.push_str(&format!("ƒ f(n: {ty}) → {ty} ! ∅ ⊲ g1(result) {{ n }}\n"));
            s.push_str(&chain(4));
        }
    }
    for k in 0..pad {
        s.push_str(&format!("ƒ pad{k}(n: {ty}) → {ty} ! ∅ {{ n }}\n"));
    }
    s.push_str(&format!("ƒ main() → i32 ! ∅ {{ r ≔ f(1{ty}); 0 }}\n"));
    s
}

#[test]
fn contract_recursion_family_is_always_rejected() {
    let tys = ["u32", "u64", "i32", "u8"];
    let mut n = 0usize;
    for shape in 0..5 {
        let mut per = 0usize;
        for ty in tys {
            for pad in 0..8 {
                rejected(&contract_family(shape, ty, pad));
                per += 1;
                n += 1;
            }
        }
        assert!(per >= 30, "hình dạng {shape}: chỉ {per} chương trình");
    }
    assert_eq!(n, 160);
}
