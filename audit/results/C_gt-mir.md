### C_gt-mir — cargo test -p gt-mir

- **Trạng thái:** PASS (mã thoát 0, 6s, bắt đầu 2026-10-20T11:35:49Z)
- **cargo test:** 30 đạt / 0 lỗi / 0 ignore trong 11 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test duplicating_a_storage_dead_or_using_a_dead_local_is_detected ... ok
test fall_off_is_reported_only_when_reachable ... ok
test implicit_reborrow_replaces_move_of_a_unique_reference_place_at_coercion_points ... ok
test lowered_mir_is_valid ... ok
test literal_true_and_false_conditions_are_folded_but_nothing_else ... ok
test never_function_with_a_reachable_return_is_e0307 ... ok
test quantifier_outside_a_contract_is_a_type_error_not_an_ice ... ok
test removing_any_single_storage_dead_on_a_reachable_path_is_detected ... ok
test structural_errors_are_detected ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

   Doc-tests gt_mir

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>
