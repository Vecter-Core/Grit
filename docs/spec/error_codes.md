# Mã chẩn đoán dự kiến (M0) — D16/D17

ĐỎ `E` (exit 1) · VÀNG `S` (chạy tiếp) · GHI CHÚ `N` · ICE `E99xx` (exit 2).
Mỗi mã khi triển khai phải có `docs/errors/<mã>.md`, ≥3 test fail, ≥3 test pass (plan §0.5).
Cột "Test mẫu" là file trong `tests/fail|warn` đã có từ M0.

| Mã | Ý nghĩa | Milestone | Test mẫu (M0) |
|---|---|---|---|
| E0101 | Token bất ngờ / thiếu dấu / từ khoá cứng hoặc ký hiệu dành riêng (`self`, `Self`, `in`, `μ`, `\extern`, `\where`; comment khối `/* */`; `=` đứng riêng; `x as T < y` thiếu ngoặc, có gợi ý machine-applicable — Q16c) dùng sai chỗ | M2 | E0101_missing_brace, E0101_cast_then_less_than, E0101_mu_as_ident, E0101_star_dangling, E0101_self_as_var, E0101_self_as_var_ascii, E0101_extern_reserved, E0101_where_reserved, E0101_block_comment, E0101_compound_assign, E0101_char_multi, E0101_doc_comment_dangling, E0101_selftype_as_var, E0101_in_as_var, E0101_place_not_assignable, E0101_struct_in_contract, E0101_struct_in_loop_variant, E0101_lifetime_after_type_arg |
| E0102 | Lồng nhau quá sâu (> `MAX_DEPTH` = 256 mức, tính cả độ dài chuỗi toán tử/postfix) hoặc cây quá lớn. ĐỎ, không phải ICE. Gợi ý sửa: tách biểu thức con ra biến phụ bằng `≔` | M2 | E0102_too_deep, E0102_too_deep_blocks, E0102_too_deep_types |
| E0201 | Tên chưa định nghĩa | M3 | E0201_import_missing, E0201_missing_member_across_files, E0201_parent_names_not_inherited, E0201_self_outside_impl, E0201_undefined_name, E0201_undefined_type, E0201_use_before_def |
| E0202 | Định nghĩa trùng | M3 | E0202_duplicate_def, E0202_duplicate_param, E0202_duplicate_variant, E0202_fn_vs_struct, E0202_import_clash |
| E0203 | Truy cập item private | M3 | E0203_private_across_files, E0203_private_import, E0203_private_item, E0203_private_module_member, E0203_private_nested_sibling |
| E0204 | Vòng phụ thuộc module | M3 | E0204_cycle_via_imports, E0204_three_files_cycle, E0204_two_files_cycle |
| E0205 | Shadowing (D7) | M3 | E0205_shadow_import, E0205_shadow_loop_var, E0205_shadow_param, E0205_shadow_prelude, E0205_shadowing |
| E0301 | Sai kiểu / biểu thức chưa có ngữ nghĩa ở Phase 1 (vd range mở độc lập) | M4 | E0301_break_outside_loop, E0301_if_condition_not_bool, E0301_open_range_standalone, E0301_return_type_mismatch, E0301_type_mismatch, E0301_wrong_arity |
| E0302 | Ép kiểu ngầm số (D1) | M4 | E0302_implicit_cast, E0302_int_to_float, E0302_mixed_operands, E0302_narrowing_argument |
| E0303 | `==` trên số thực (D4) | M4 | E0303_float_eq, E0303_float_literal_eq, E0303_float_ne |
| E0304 | Trường struct/biến thể/pattern sai: thiếu, thừa, không tồn tại hoặc trùng (literal `P { … }`, pattern `P { … }`, truy cập `.x`) — Q20 | M4 | E0304_duplicate_field, E0304_missing_field, E0304_pattern_missing_field, E0304_tuple_index_out_of_range, E0304_unknown_field |
| E0305 | `⌥` không đầy đủ (D6) | M4 | E0305_int_without_wildcard, E0305_missing_bool_branch, E0305_nonexhaustive, E0305_range_gap, E0305_str_without_wildcard |
| E0307 | Thiếu đường trả về (gồm khối kết thúc bằng `;` mà hàm trả kiểu khác `()`) — **thuộc M5**, không phải parser (cần kiểu trả về và luồng điều khiển) | M5 | E0307_missing_return, E0307_trailing_semicolon, E0307_match_arm_falls_through, E0307_return_only_inside_loop, E0307_last_if_without_else |
| E0308 | Gán vào biến bất biến (D5) | M4 | E0308_assign_immutable, E0308_assign_param, E0308_assign_through_shared_ref, E0308_method_needs_mut_receiver, E0308_unique_borrow_of_immutable |
| E0309 | Ràng buộc trait không thoả / không có phương thức / mơ hồ (Q20) | M4 | E0309_ambiguous_method, E0309_bound_not_satisfied, E0309_no_such_method |
| E0311 | `⊐` không hợp lệ: thiếu/thừa/khác chữ ký thành viên so với trait, hoặc hai impl chồng lấn (Q20) | M4 | E0311_copy_non_copy_field, E0311_missing_trait_method, E0311_overlapping_impls, E0311_signature_mismatch |
| E0312 | Đệ quy đa hình vượt ngưỡng instance (64) — `ƒ f<T>() { f<Vec<T>>() }` (ADR-018 §1, Q20) | M4 | E0312_array_growth, E0312_option_growth, E0312_polymorphic_recursion |
| E0313 | Kiểu lang dùng sai: tạo `Secret`/`Cap` bằng literal ngoài lib/core, `⊐ Copy for` chúng, `⊐ Drop for` ngoài lib/core (ADR-020 §9) (Q20, Q21) | M4 | E0313_copy_for_cap, E0313_copy_for_secret, E0313_forge_cap, E0313_forge_secret |
| E0314 | Kiểu `!` ở vị trí không hợp lệ: tham số kiểu, trường, phần tử mảng (ADR-018 §3.6, Q20) | M4 | E0314_never_array_element, E0314_never_field, E0314_never_type_argument |
| E0310 | Thiếu capability: gọi nguồn capability gốc (hàm trả `Cap<…>` mà không nhận `Cap<…>`) ngoài `main` (D28, D29) | M4.5 | E0310_missing_cap, E0310_as_argument, E0310_assoc_root, E0310_both_branches, E0310_bound_to_local, E0310_const_initializer, E0310_fn_as_value, E0310_helper_called_from_main, E0310_in_generic_fn, E0310_in_impl_method, E0310_in_trait_impl_method, E0310_in_tuple, E0310_in_variant, E0310_inside_if_branch, E0310_inside_while_body, E0310_main_in_nested_module, E0310_method_named_main, E0310_missing_cap, E0310_returned_as_is, E0310_root_alloc, E0310_root_clock, E0310_root_env, E0310_root_net, E0310_root_then_split, E0310_static_initializer, E0310_two_roots |
| E0315 | Lifetime/region trong chữ ký không hợp lệ: chưa khai báo, khai báo trùng/che, trường thiếu lifetime, elision mơ hồ ở kiểu trả về (ADR-019) | M4.5 | E0315_ambiguous_elision, E0315_duplicate_lifetime, E0315_enum_field_undeclared, E0315_field_elision, E0315_let_annotation, E0315_method_two_refs, E0315_shadow_impl_lifetime, E0315_undeclared_in_return, E0315_undeclared_lifetime, E0315_lifetime_arg_count, E0315_lifetime_arg_undeclared, E0315_field_elided_lifetime_arg |
| E0316 | Lượng từ `∀`/`∃` ngoài hợp đồng (chỉ hợp lệ trong `⊳ ⊲ ↓ ⋈`) | M4 | E0316_forall_in_body, E0316_exists_in_condition, E0316_quantifier_in_let |
| E0401 | Dùng sau move | M6 | E0401_use_after_move |
| E0402 | Hai `&!` chồng nhau | M6 | E0402_double_unique |
| E0403 | `&!` chồng `&` | M6 | E0403_unique_vs_shared |
| E0404 | Tham chiếu sống lâu hơn giá trị bị mượn: trả tham chiếu tới local/tạm/tham số theo giá trị, biến phạm vi ngoài giữ tham chiếu tới biến phạm vi trong, hoặc vi phạm region của chữ ký (ADR-023 §8) | M6 | E0404_return_local_ref |
| E0405 | Con trỏ thoát Arena (D27) | M6 | E0405_arena_escape |
| E0406 | Gán khi đang bị mượn | M6 | — |
| E0407 | Move ra khỏi chỗ đi qua `*`/`[i]`/lát, hoặc move một trường ra khỏi giá trị có `⊐ Drop for`/`Arena` (ADR-023 §3, §8) | M6 | — |
| E0501 | Chia sẻ không `Send` qua luồng | M8 | — |
| E0502 | Vi phạm thứ tự khoá | M8 | — |
| E0601 | Obligation biên chắc chắn sai (tràn hằng) | M7 | E0601_const_overflow, E0601_literal_out_of_range |
| E0602 | Chia 0 chắc chắn | M7 | E0602_const_div_zero |
| E0603 | Chỉ số ngoài biên chắc chắn | M7 | E0603_const_oob |
| E0604 | Ép kiểu mất thông tin chắc chắn | M7 | — |
| E0701 | Vòng lặp thiếu `↓` (D12) | M8 | E0701_loop_no_variant, E0701_infinite_no_div, E0701_disjunction_cond |
| E0702 | `↓` không giảm nghiêm ngặt hoặc không bị chặn dưới | M8 | E0702_variant_not_decreasing, E0702_variant_unbounded, E0702_variant_stationary |
| E0703 | Đệ quy thiếu `↓` (hoặc `div`) | M8 | E0703_self_recursion, E0703_mutual_recursion, E0703_partial_cycle |
| E0801 | Hiệu ứng suy ra ⊄ khai báo (kể cả hàm `∅` gọi hàm có hiệu ứng) | M8 | E0801_effect_undeclared |
| E0802 | Thành viên impl khai hiệu ứng ngoài tập mà thành viên trait cho phép | M8 | E0802_impl_more_effects |
| E0901 | Hợp đồng chắc chắn sai: `⊳` tại lời gọi, `⊲` ở điểm thoát, `⋈` ở đầu vòng lặp (ADR-025 §6) | M7 | E0901_precondition_false, E0901_shift_too_far, E0901_postcondition_false, E0901_invariant_false |
| E0902 | Obligation không kiểm lúc chạy được mà chưa giải (D25): lượng từ không có biên `a..b`; `⊳` cần gọi lúc chạy mà đối số không `Copy`; `⊲` nhắc tham số bị gán lại | M7 | E0902_quantifier_over_range_value, E0902_noncopy_argument_unproven_pre, E0902_post_names_reassigned_param |
| E1001 | Nghĩa vụ `⚠` chưa giải (D21); con trỏ thô `*□ T`/`*μ T` hoặc `#noreturn` dùng ngoài phạm vi cho phép (Q12, Q13) | M7 | E1001_unsafe_unproven, E1001_raw_ptr_outside_unsafe |
| E1002 | Thuộc tính `#lang` dùng ngoài `lib/core`/prelude (ADR-018 §5, Q21) | M4 | E1002_lang_on_fn, E1002_lang_on_struct, E1002_lang_on_trait |
| E1101 | IR Guard: attestation lệch | M10 | — |
| E1201 | Ký tự bidi (D15) | M1 | E1201_bidi |
| E1202 | Ký tự zero-width/vô hình | M1 | E1202_zero_width |
| E1203 | Định danh trộn bảng chữ/homoglyph | M1 | E1203_homoglyph |
| E1204 | Nguồn không phải NFC | M1 | E1204_not_nfc |
| E1301 | gt-fmt: không đặt được comment (bước xác minh sau khi chèn comment theo token thất bại: token hoặc dãy comment đổi). PHƯƠNG ÁN CUỐI — theo Q16b mọi vị trí comment thông thường đều được đặt; gặp mã này là lỗi của gt-fmt cần báo cáo. ĐỎ — duyệt Giang 2026-10-06 (nhóm E13xx: định dạng) Không có mẫu `.gt` (không đầu vào hợp lệ nào kích hoạt); kiểm bằng unit test `verify_rejects_changed_tokens_or_changed_comment_sequence` trong gt-fmt. | M2 (gt-fmt) | — |
| E9901 | ICE (lỗi nội bộ compiler; gồm panic trong parser bị bắt, hết bộ nhớ khi tạo luồng parser) — exit 2, không bao giờ chạy | M2 (parser) / M9 (cổng phán quyết) | — |
| S101 | Secret nằm trong nguồn | M9 | S101_key_in_source |
| S201 | `declassify` / Secret chảy ra sink | M9 | — |
| N0601 | Check tràn hạ thành runtime | M7 | pass/02_sum_loop |
| N0602 | Check biên mảng hạ thành runtime | M7 | — |
| N0603 | Check chia/% (mẫu số ≠ 0, `MIN / -1`) hạ thành runtime | M7 | — |
| N0604 | Check ép kiểu mất thông tin hạ thành runtime | M7 | — |
| N0605 | Phân tích bị giảm độ chính xác: vượt `ZONE_MAX_VARS` biến cho miền quan hệ, hoặc hết ngân sách bước (D20) | M7 | — |
| N0901 | Tiền điều kiện hạ thành runtime | M7 | — |
| N0902 | Hậu điều kiện `⊲` hạ thành runtime ở mọi điểm thoát | M7 | — |
| N0903 | Lượng từ có biên hạ thành vòng lặp lúc chạy | M7 | — |
| N0904 | Bất biến `⋈` hạ thành runtime | M7 | — |

Ghi chú: plan ghi match-đầy-đủ và capability thuộc "E01xx/E03xx/E06xx" / "E03xx/E08xx" mơ hồ → đề xuất
E0305/E0310 (xem OPEN_QUESTIONS Q4). Các test `—` do milestone sở hữu viết (cần đa file/pass chưa có).

**Mã đã nghỉ (không tái sử dụng số):** E0306 ("dùng biến chưa khởi tạo") — D3 cấm biến chưa khởi tạo và `≔` luôn có vế khởi tạo nên không có nguồn kích hoạt; khởi tạo chắc chắn là bất biến của MIR, vi phạm là ICE (ADR-020 §9, Q25). Mở lại nếu Phase 2 thêm khởi tạo trì hoãn.
