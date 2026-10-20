### C_gt-dataflow — cargo test -p gt-dataflow

- **Trạng thái:** PASS (mã thoát 0, 4s, bắt đầu 2026-10-20T11:35:56Z)
- **cargo test:** 14 đạt / 0 lỗi / 0 ignore trong 5 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test definitely_assigned_is_an_all_paths_analysis ... ok
test call_destination_is_initialized_only_on_the_return_edge ... ok
test drop_deinitializes_and_is_exempt_from_the_assignment_invariant ... ok
test liveness_is_fixed_on_the_counting_loop ... ok
test moves_deinitialize_and_partial_moves_touch_only_the_subtree ... ok
test maybe_init_and_maybe_uninit_on_the_diamond ... ok
test return_uses_the_return_place ... ok
test reverse_postorder_is_fixed_and_skips_unreachable_blocks ... ok
test the_same_cfg_with_both_branches_assigning_is_clean ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests gt_dataflow

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>
