### C_gt-types — cargo test -p gt-types

- **Trạng thái:** PASS (mã thoát 0, 34s, bắt đầu 2026-10-20T11:35:15Z)
- **cargo test:** 174 đạt / 0 lỗi / 0 ignore trong 19 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test bodies_list_functions_members_consts_and_statics_but_not_the_prelude ... ok
test call_sites_record_target_generics_and_receiver_adjustment ... ok
test impl_bounds_are_in_scope_of_members ... ok
test is_copy_respects_the_bounds_in_scope ... ok
test needs_drop_follows_adr_020_q26 ... ok
test inclusive_ranges_have_their_own_type ... ok
test trait_methods_on_type_parameters_stay_unresolved_until_mono ... ok
test type_parameters_need_drop_in_polymorphic_mir ... ok
test user_drop_impls_are_rejected_in_phase_1_e0313 ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

   Doc-tests gt_types

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>
