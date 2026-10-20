### C_gt-stack — cargo test -p gt-stack

- **Trạng thái:** PASS (mã thoát 0, 0s, bắt đầu 2026-10-20T11:19:38Z)
- **cargo test:** 6 đạt / 0 lỗi / 0 ignore trong 3 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
     Running tests/stack.rs (target/debug/deps/stack-a9a081d1a0a09b1a)

running 6 tests
test nested_calls_each_get_their_own_big_stack ... ok
test deep_recursion_that_would_overflow_a_default_stack_works ... ok
test panic_with_non_string_payload_still_reports ... ok
test panics_become_errors_not_crashes ... ok
test returns_the_value_and_may_borrow_from_the_caller ... ok
test stack_constant_is_large_enough_for_the_depth_contract ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

   Doc-tests gt_stack

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>
