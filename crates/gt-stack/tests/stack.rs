//! gt-stack: helper chạy-trong-luồng-stack-lớn dùng chung cho MỌI pass đệ quy trên AST (ADR-015). Viết TRƯỚC (TDD).
use gt_stack::{BIG_STACK_BYTES, StackError, run_with_big_stack};

#[test]
fn returns_the_value_and_may_borrow_from_the_caller() {
    let data = [1u64, 2, 3];
    let sum = run_with_big_stack(|| data.iter().sum::<u64>()).expect("chạy được");
    assert_eq!(sum, 6);
    // trả về dữ liệu sở hữu
    let s = run_with_big_stack(|| "xin chào".to_string()).expect("ok");
    assert_eq!(s, "xin chào");
}

#[test]
fn panics_become_errors_not_crashes() {
    let e = run_with_big_stack(|| -> u32 { panic!("hỏng rồi") }).expect_err("phải là lỗi");
    assert_eq!(e, StackError::Panic("hỏng rồi".to_string()));
    // panic có định dạng (String)
    let n = 7;
    let e = run_with_big_stack(move || -> u32 { panic!("n = {n}") }).expect_err("phải là lỗi");
    assert_eq!(e, StackError::Panic("n = 7".to_string()));
    // sau panic vẫn dùng được
    assert_eq!(run_with_big_stack(|| 1 + 1).expect("ok"), 2);
    assert!(e.to_string().contains("n = 7"));
}

#[test]
fn panic_with_non_string_payload_still_reports() {
    let e = run_with_big_stack(|| -> u32 { std::panic::panic_any(42u32) }).expect_err("lỗi");
    assert!(matches!(e, StackError::Panic(_)));
}

/// Đệ quy tiêu tốn ~24 MiB stack (gấp >10 lần stack mặc định 2 MiB của luồng test) phải chạy được trong helper.
#[test]
fn deep_recursion_that_would_overflow_a_default_stack_works() {
    fn rec(n: u32) -> u64 {
        let pad = [n as u8; 256];
        let pad = std::hint::black_box(pad);
        if n == 0 {
            u64::from(pad[0])
        } else {
            rec(n - 1) + u64::from(pad[7] & 1)
        }
    }
    // ~40_000 khung × ~700 byte (debug) ≈ 28 MiB: >10 lần stack mặc định 2 MiB, vẫn < 64 MiB
    let r = run_with_big_stack(|| rec(40_000)).expect("không tràn stack");
    assert!(r <= 40_000);
}

#[test]
fn stack_constant_is_large_enough_for_the_depth_contract() {
    // MAX_DEPTH = 256 mức × (nhiều khung × vài KiB ở bản debug) phải vừa; 64 MiB có dư địa lớn
    const { assert!(BIG_STACK_BYTES >= 32 * 1024 * 1024) };
}

#[test]
fn nested_calls_each_get_their_own_big_stack() {
    let v = run_with_big_stack(|| run_with_big_stack(|| 5).expect("trong")).expect("ngoài");
    assert_eq!(v, 5);
}
