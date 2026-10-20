### G02_rig_tests — [bản sao] cargo test -p rig --no-default-features

- **Trạng thái:** FAIL (mã thoát 101, 6s, bắt đầu 2026-10-20T11:33:12Z)
- **cargo test:** 10 đạt / 37 lỗi / 3 ignore trong 10 bộ test
- **rustc/clippy:** 8 dòng `error`, 0 dòng `warning`

**Test lỗi (37):**
- `wc_error_paths_are_controlled`
- `wc_handles_a_large_file_and_reports_throughput`
- `wc_written_in_grit_matches_the_reference_on_many_files`
- `json_tool_counts_and_depth_hand_checked`
- `deep_nesting_is_a_controlled_error_not_a_stack_overflow`
- `json_tool_matches_reference_and_serde_on_hand_written_documents`
- `json_tool_error_paths_and_missing_args`
- `mutation_fuzz_against_reference_and_serde`
- `json_tool_on_large_files_reports_throughput`
- `nist_vectors_hard_coded`
- `large_files_match_hashlib_and_report_throughput`
- `padding_boundaries_match_hashlib`
- `random_content_200_files_match_hashlib`
- `sha256_error_paths`
- `arena_formula_is_exact_and_exhaustion_is_controlled`
- `error_paths_use_stderr_and_exit_code_2`
- `four_mib_of_empty_lines`
- `one_million_short_lines`
- `one_very_long_line_of_1_mib`
- `random_text_1_mib_and_32_mib`
- `small_cases_match_the_reference_byte_for_byte`
- `bad_tokens_and_too_large_values_are_controlled_errors`
- `random_and_large_inputs_match_the_reference`
- `small_cases_match_the_reference_byte_for_byte`
- `invalid_utf8_is_a_controlled_error_not_an_abort`
- `large_files_match_the_reference`
- `many_distinct_words`
- `small_cases_match_the_reference_byte_for_byte`
- `builds_are_reproducible_byte_for_byte`
- `add_edits_manifest_relocks_and_rolls_back_on_failure`
- `cross_package_generic_trait_forwarder_passes_e0703_and_runs_correctly`
- `failing_test_exits_1_and_names_the_abort`
- `lock_missing_is_created_stale_is_always_refused_and_locked_flag_is_strict`
- `exit_codes_gritc_passthrough_and_rig_own_range`
- `option_ord_in_another_package_uses_reference_patterns_passes_e0703_and_runs_correctly`
- `new_check_build_run_roundtrip`
- `three_package_project_builds_runs_and_tests_with_diamond_loaded_once`

<details><summary>Chi tiết panic (tối đa 12)</summary>

`wc_error_paths_are_controlled`
```
thread 'wc_error_paths_are_controlled' (14955) panicked at crates/rig/tests/dogfood.rs:56:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:12:27
   |
12 |     ⎇ c == 10u8 { lines ← lines + 1; } ⇝ { }
   |                           ^^^^^^^^^
```
`wc_handles_a_large_file_and_reports_throughput`
```
thread 'wc_handles_a_large_file_and_reports_throughput' (14956) panicked at crates/rig/tests/dogfood.rs:56:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:12:27
   |
12 |     ⎇ c == 10u8 { lines ← lines + 1; } ⇝ { }
   |                           ^^^^^^^^^
```
`wc_written_in_grit_matches_the_reference_on_many_files`
```
thread 'wc_written_in_grit_matches_the_reference_on_many_files' (15508) panicked at crates/rig/tests/dogfood.rs:56:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:12:27
   |
12 |     ⎇ c == 10u8 { lines ← lines + 1; } ⇝ { }
   |                           ^^^^^^^^^
```
`json_tool_counts_and_depth_hand_checked`
```
thread 'json_tool_counts_and_depth_hand_checked' (15765) panicked at crates/rig/tests/dogfood_json.rs:42:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:16:19
   |
16 |   ⎇ is_digit(c) { (c as u32) - 48u32 } ⇝ { ⎇ c >= 97u8 { (c as u32) - 87u32 } ⇝ { (c as u32) - 55u32 } }
   |                   ^^^^^^^^^^^^^^^^^^
```
`deep_nesting_is_a_controlled_error_not_a_stack_overflow`
```
thread 'deep_nesting_is_a_controlled_error_not_a_stack_overflow' (15764) panicked at crates/rig/tests/dogfood_json.rs:42:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:16:19
   |
16 |   ⎇ is_digit(c) { (c as u32) - 48u32 } ⇝ { ⎇ c >= 97u8 { (c as u32) - 87u32 } ⇝ { (c as u32) - 55u32 } }
   |                   ^^^^^^^^^^^^^^^^^^
```
`json_tool_matches_reference_and_serde_on_hand_written_documents`
```
thread 'json_tool_matches_reference_and_serde_on_hand_written_documents' (17705) panicked at crates/rig/tests/dogfood_json.rs:42:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:16:19
   |
16 |   ⎇ is_digit(c) { (c as u32) - 48u32 } ⇝ { ⎇ c >= 97u8 { (c as u32) - 87u32 } ⇝ { (c as u32) - 55u32 } }
   |                   ^^^^^^^^^^^^^^^^^^
```
`json_tool_error_paths_and_missing_args`
```
thread 'json_tool_error_paths_and_missing_args' (17704) panicked at crates/rig/tests/dogfood_json.rs:42:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:16:19
   |
16 |   ⎇ is_digit(c) { (c as u32) - 48u32 } ⇝ { ⎇ c >= 97u8 { (c as u32) - 87u32 } ⇝ { (c as u32) - 55u32 } }
   |                   ^^^^^^^^^^^^^^^^^^
```
`mutation_fuzz_against_reference_and_serde`
```
thread 'mutation_fuzz_against_reference_and_serde' (19463) panicked at crates/rig/tests/dogfood_json.rs:42:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:16:19
   |
16 |   ⎇ is_digit(c) { (c as u32) - 48u32 } ⇝ { ⎇ c >= 97u8 { (c as u32) - 87u32 } ⇝ { (c as u32) - 55u32 } }
   |                   ^^^^^^^^^^^^^^^^^^
```
`json_tool_on_large_files_reports_throughput`
```
thread 'json_tool_on_large_files_reports_throughput' (19449) panicked at crates/rig/tests/dogfood_json.rs:42:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:16:19
   |
16 |   ⎇ is_digit(c) { (c as u32) - 48u32 } ⇝ { ⎇ c >= 97u8 { (c as u32) - 87u32 } ⇝ { (c as u32) - 55u32 } }
   |                   ^^^^^^^^^^^^^^^^^^
```
`nist_vectors_hard_coded`
```
thread 'nist_vectors_hard_coded' (21232) panicked at crates/rig/tests/dogfood_sha256.rs:41:5:
assertion `left == right` failed: rig build: note[N0602]: không chứng minh được chỉ số/khoảng trong biên: giữ kiểm tra lúc chạy
  --> main.gt:14:15
   |
14 |   bor(bor(shl(b[o] as u32, 24u32), shl(b[o + 1] as u32, 16u32)), bor(shl(b[o + 2] as u32, 8u32), b[o + 3] as u32))
   |               ^^^^
```
`large_files_match_hashlib_and_report_throughput`
```
thread 'large_files_match_hashlib_and_report_throughput' (21230) panicked at crates/rig/tests/dogfood_sha256.rs:41:5:
assertion `left == right` failed: rig build: note[N0602]: không chứng minh được chỉ số/khoảng trong biên: giữ kiểm tra lúc chạy
  --> main.gt:14:15
   |
14 |   bor(bor(shl(b[o] as u32, 24u32), shl(b[o + 1] as u32, 16u32)), bor(shl(b[o + 2] as u32, 8u32), b[o + 3] as u32))
   |               ^^^^
```
`padding_boundaries_match_hashlib`
```
thread 'padding_boundaries_match_hashlib' (22138) panicked at crates/rig/tests/dogfood_sha256.rs:41:5:
assertion `left == right` failed: rig build: note[N0602]: không chứng minh được chỉ số/khoảng trong biên: giữ kiểm tra lúc chạy
  --> main.gt:14:15
   |
14 |   bor(bor(shl(b[o] as u32, 24u32), shl(b[o + 1] as u32, 16u32)), bor(shl(b[o + 2] as u32, 8u32), b[o + 3] as u32))
   |               ^^^^
```
</details>

**Lỗi (dòng `error` đầu tiên):**
```
error: test failed, to rerun pass `-p rig --test dogfood`
     Running tests/dogfood_json.rs (target/debug/deps/dogfood_json-9a0a78e0696b1576)

running 6 tests
error: test failed, to rerun pass `-p rig --test dogfood_json`
     Running tests/dogfood_sha256.rs (target/debug/deps/dogfood_sha256-6d2b73e3d16234f4)

running 5 tests
error: test failed, to rerun pass `-p rig --test dogfood_sha256`
     Running tests/dogfood_sortlines.rs (target/debug/deps/dogfood_sortlines-f10adfaea3fef02f)

running 8 tests
error: test failed, to rerun pass `-p rig --test dogfood_sortlines`
     Running tests/dogfood_stats.rs (target/debug/deps/dogfood_stats-85f7110243405591)

running 4 tests
error: test failed, to rerun pass `-p rig --test dogfood_stats`
     Running tests/dogfood_wordfreq.rs (target/debug/deps/dogfood_wordfreq-b3b6d6a2a87eb5cd)

running 6 tests
error: test failed, to rerun pass `-p rig --test dogfood_wordfreq`
     Running tests/rig.rs (target/debug/deps/rig-36e069ca0be0dbbc)

running 14 tests
error: test failed, to rerun pass `-p rig --test rig`
   Doc-tests rig

running 0 tests
error: 7 targets failed:
    `-p rig --test dogfood`
    `-p rig --test dogfood_json`
    `-p rig --test dogfood_sha256`
```

<details><summary>Đuôi log (18 dòng cuối)</summary>

```

test result: FAILED. 5 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s

error: test failed, to rerun pass `-p rig --test rig`
   Doc-tests rig

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 7 targets failed:
    `-p rig --test dogfood`
    `-p rig --test dogfood_json`
    `-p rig --test dogfood_sha256`
    `-p rig --test dogfood_sortlines`
    `-p rig --test dogfood_stats`
    `-p rig --test dogfood_wordfreq`
    `-p rig --test rig`
```
</details>
