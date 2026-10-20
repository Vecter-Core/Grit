### C_gt-conc — cargo test -p gt-conc

- **Trạng thái:** PASS (mã thoát 0, 1s, bắt đầu 2026-10-20T11:43:59Z)
- **cargo test:** 4 đạt / 0 lỗi / 0 ignore trong 3 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ordering.rs (target/debug/deps/ordering-c5f79655ddf00f6a)

running 4 tests
test atomic_methods_used_as_function_values_are_rejected ... ok
test only_atomic_methods_are_rejected_as_function_values ... ok
test atomic_orderings_must_be_literal_and_valid ... ok
test ordering_tracing_is_exact_and_scans_every_call ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

   Doc-tests gt_conc

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>
