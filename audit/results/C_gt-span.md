### C_gt-span — cargo test -p gt-span

- **Trạng thái:** PASS (mã thoát 0, 1s, bắt đầu 2026-10-20T11:19:15Z)
- **cargo test:** 9 đạt / 0 lỗi / 0 ignore trong 3 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test file_id_is_ordered_hashable_and_indexes ... ok
test empty_file_and_name ... ok
test line_col_counts_unicode_scalars_not_bytes ... ok
test line_col_lf_and_crlf ... ok
test snippet_and_line_text ... ok
test source_map_assigns_file_ids_by_sorted_name_not_by_input_order ... ok
test source_map_rejects_duplicate_names_and_handles_empty ... ok
test span_basics ... ok
test span_new_rejects_inverted_range - should panic ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

   Doc-tests gt_span

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>
