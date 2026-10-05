# Mã chẩn đoán dự kiến (M0) — D16/D17

ĐỎ `E` (exit 1) · VÀNG `S` (chạy tiếp) · GHI CHÚ `N` · ICE `E99xx` (exit 2).
Mỗi mã khi triển khai phải có `docs/errors/<mã>.md`, ≥3 test fail, ≥3 test pass (plan §0.5).
Cột "Test mẫu" là file trong `tests/fail|warn` đã có từ M0.

| Mã | Ý nghĩa | Milestone | Test mẫu (M0) |
|---|---|---|---|
| E0101 | Token bất ngờ / thiếu dấu / từ khoá cứng hoặc ký hiệu dành riêng (`self`, `Self`, `in`, `μ`, `\extern`, `\where`; comment khối `/* */`; `=` đứng riêng; `x as T < y` thiếu ngoặc, có gợi ý machine-applicable — Q16c) dùng sai chỗ | M2 | E0101_missing_brace, E0101_cast_then_less_than, E0101_mu_as_ident, E0101_star_dangling, E0101_self_as_var, E0101_self_as_var_ascii, E0101_extern_reserved, E0101_where_reserved, E0101_block_comment, E0101_compound_assign, E0101_char_multi, E0101_doc_comment_dangling, E0101_selftype_as_var, E0101_in_as_var, E0101_place_not_assignable, E0101_struct_in_contract, E0101_struct_in_loop_variant |
| E0102 | Lồng nhau quá sâu (> `MAX_DEPTH` = 256 mức, tính cả độ dài chuỗi toán tử/postfix) hoặc cây quá lớn. ĐỎ, không phải ICE. Gợi ý sửa: tách biểu thức con ra biến phụ bằng `≔` | M2 | E0102_too_deep, E0102_too_deep_blocks, E0102_too_deep_types |
| E0201 | Tên chưa định nghĩa | M3 | E0201_import_missing, E0201_missing_member_across_files, E0201_parent_names_not_inherited, E0201_self_outside_impl, E0201_undefined_name, E0201_undefined_type, E0201_use_before_def |
| E0202 | Định nghĩa trùng | M3 | E0202_duplicate_def, E0202_duplicate_param, E0202_duplicate_variant, E0202_fn_vs_struct, E0202_import_clash |
| E0203 | Truy cập item private | M3 | E0203_private_across_files, E0203_private_import, E0203_private_item, E0203_private_module_member, E0203_private_nested_sibling |
| E0204 | Vòng phụ thuộc module | M3 | E0204_cycle_via_imports, E0204_three_files_cycle, E0204_two_files_cycle |
| E0205 | Shadowing (D7) | M3 | E0205_shadow_import, E0205_shadow_loop_var, E0205_shadow_param, E0205_shadow_prelude, E0205_shadowing |
| E0301 | Sai kiểu / biểu thức chưa có ngữ nghĩa ở Phase 1 (vd range mở độc lập) | M4 | E0301_type_mismatch, E0301_open_range_standalone |
| E0302 | Ép kiểu ngầm số (D1) | M4 | E0302_implicit_cast |
| E0303 | `==` trên số thực (D4) | M4 | E0303_float_eq |
| E0304 | Trường struct/biến thể/pattern sai: thiếu, thừa, không tồn tại hoặc trùng (literal `P { … }`, pattern `P { … }`, truy cập `.x`) — Q20 | M4 | — |
| E0305 | `⌥` không đầy đủ (D6) | M4 | E0305_nonexhaustive |
| E0306 | Dùng biến chưa khởi tạo chắc chắn | M5 | — |
| E0307 | Thiếu đường trả về (gồm khối kết thúc bằng `;` mà hàm trả kiểu khác `()`) — **thuộc M5**, không phải parser (cần kiểu trả về và luồng điều khiển) | M5 | E0307_missing_return, E0307_trailing_semicolon |
| E0308 | Gán vào biến bất biến (D5) | M4 | E0308_assign_immutable |
| E0309 | Ràng buộc trait không thoả / không có phương thức / mơ hồ (Q20) | M4 | — |
| E0311 | `⊐` không hợp lệ: thiếu/thừa/khác chữ ký thành viên so với trait, hoặc hai impl chồng lấn (Q20) | M4 | — |
| E0312 | Đệ quy đa hình vượt ngưỡng instance (64) — `ƒ f<T>() { f<Vec<T>>() }` (ADR-018 §1, Q20) | M4 | — |
| E0313 | Kiểu lang dùng sai: tạo `Secret`/`Cap` bằng literal ngoài lib/core, `⊐ Copy for` chúng (Q20, Q21) | M4 | — |
| E0314 | Kiểu `!` ở vị trí không hợp lệ: tham số kiểu, trường, phần tử mảng (ADR-018 §3.6, Q20) | M4 | — |
| E0310 | Thiếu capability (D28) | M4.5 | E0310_missing_cap |
| E0401 | Dùng sau move | M6 | E0401_use_after_move |
| E0402 | Hai `&!` chồng nhau | M6 | E0402_double_unique |
| E0403 | `&!` chồng `&` | M6 | E0403_unique_vs_shared |
| E0404 | Trả tham chiếu tới local | M6 | E0404_return_local_ref |
| E0405 | Con trỏ thoát Arena (D27) | M6 | E0405_arena_escape |
| E0406 | Gán khi đang bị mượn | M6 | — |
| E0501 | Chia sẻ không `Send` qua luồng | M8 | — |
| E0502 | Vi phạm thứ tự khoá | M8 | — |
| E0601 | Obligation biên chắc chắn sai (tràn hằng) | M7 | E0601_const_overflow, E0601_literal_out_of_range |
| E0602 | Chia 0 chắc chắn | M7 | E0602_const_div_zero |
| E0603 | Chỉ số ngoài biên chắc chắn | M7 | E0603_const_oob |
| E0604 | Ép kiểu mất thông tin chắc chắn | M7 | — |
| E0701 | Vòng lặp thiếu `↓` (D12) | M8 | E0701_loop_no_variant, E0701_infinite_no_div |
| E0702 | `↓` không giảm nghiêm ngặt | M8 | E0702_variant_not_decreasing |
| E0703 | Đệ quy thiếu `↓` | M8 | — |
| E0801 | Hiệu ứng suy ra ⊄ khai báo | M8 | E0801_effect_undeclared |
| E0802 | Hàm `∅` gọi hàm có hiệu ứng | M8 | — |
| E0901 | Tiền điều kiện chắc chắn sai | M7 | E0901_precondition_false |
| E0902 | Obligation không kiểm lúc chạy được mà chưa giải (D25) | M7 | — |
| E1001 | Nghĩa vụ `⚠` chưa giải (D21); con trỏ thô `*□ T`/`*μ T` hoặc `#noreturn` dùng ngoài phạm vi cho phép (Q12, Q13) | M7 | E1001_unsafe_unproven, E1001_raw_ptr_outside_unsafe |
| E1002 | Thuộc tính `#lang` dùng ngoài `lib/core`/prelude (ADR-018 §5, Q21) | M4 | — |
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
| N0901 | Tiền điều kiện hạ thành runtime | M7 | — |

Ghi chú: plan ghi match-đầy-đủ và capability thuộc "E01xx/E03xx/E06xx" / "E03xx/E08xx" mơ hồ → đề xuất
E0305/E0310 (xem OPEN_QUESTIONS Q4). Các test `—` do milestone sở hữu viết (cần đa file/pass chưa có).
