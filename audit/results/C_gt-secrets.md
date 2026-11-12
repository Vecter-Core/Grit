### C_gt-secrets — cargo test -p gt-secrets

- **Trạng thái:** PASS (mã thoát 0, 1s, bắt đầu 2026-10-20T11:49:59Z)
- **cargo test:** 15 đạt / 0 lỗi / 0 ignore trong 3 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test s101_in_comment_and_non_matches ... ok
test s101_known_patterns_in_string_literals ... ok
test s102_high_entropy_literal_assigned_to_secret_like_name ... ok
test s102_non_matches ... ok
test s201_declassify_and_s202_flow_to_io_sinks ... ok
test scanning_continues_past_non_matching_names_and_invalid_allows ... ok
test pattern_boundaries_are_exact ... ok
test taint_needs_only_one_tainted_local_among_many_operands ... ok
test taint_flows_through_many_shapes_and_only_real_declassify_counts ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s

   Doc-tests gt_secrets

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>
