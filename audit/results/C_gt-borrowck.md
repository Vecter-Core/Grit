### C_gt-borrowck — cargo test -p gt-borrowck

- **Trạng thái:** PASS (mã thoát 0, 204s, bắt đầu 2026-10-20T11:36:00Z)
- **cargo test:** 57 đạt / 0 lỗi / 0 ignore trong 11 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test alias_through_function_calls_and_returned_references ... ok
test alias_through_generics ... ok
test alias_through_match_and_partial_moves ... ok
test alias_through_reborrow_chains ... ok
test invariance_survives_adt_construction ... ok
test arena_versions_are_invalidated_by_every_operation ... ok
test moves_out_of_places_that_cannot_be_moved_from ... ok
test unique_references_cannot_be_duplicated_by_copying_or_moving_around ... ok
test references_stored_in_structs_and_collections ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s

   Doc-tests gt_borrowck

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>
