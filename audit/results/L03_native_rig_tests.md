### L03_native_rig_tests — [bản sao 2] cargo test -p rig (rig build/run/test + 6 công cụ dogfood bằng exe native)

- **Trạng thái:** PASS (mã thoát 0, 32s, bắt đầu 2026-10-20T11:48:35Z)
- **cargo test:** 47 đạt / 0 lỗi / 3 ignore trong 10 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test failing_test_exits_1_and_names_the_abort ... ok
test fmt_rewrites_and_check_flags_unformatted_sources ... ok
test lock_missing_is_created_stale_is_always_refused_and_locked_flag_is_strict ... ok
test manifest_errors_are_fail_closed_with_clear_messages ... ok
test module_name_collisions_and_prefix_rule_are_loud_errors ... ok
test new_check_build_run_roundtrip ... ok
test builds_are_reproducible_byte_for_byte ... ok
test three_package_project_builds_runs_and_tests_with_diamond_loaded_once ... ok
test option_ord_in_another_package_uses_reference_patterns_passes_e0703_and_runs_correctly ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.84s

   Doc-tests rig

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>
