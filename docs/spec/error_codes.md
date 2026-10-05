# Mã chẩn đoán dự kiến (M0) — D16/D17

ĐỎ `E` (exit 1) · VÀNG `S` (chạy tiếp) · GHI CHÚ `N` · ICE `E99xx` (exit 2).
Mỗi mã khi triển khai phải có `docs/errors/<mã>.md`, ≥3 test fail, ≥3 test pass (plan §0.5).
Cột "Test mẫu" là file trong `tests/fail|warn` đã có từ M0.

| Mã | Ý nghĩa | Milestone | Test mẫu (M0) |
|---|---|---|---|
| E0101 | Token bất ngờ / thiếu dấu | M2 | E0101_missing_brace |
| E0201 | Tên chưa định nghĩa | M3 | E0201_undefined_name |
| E0202 | Định nghĩa trùng | M3 | E0202_duplicate_def |
| E0203 | Truy cập item private | M3 | E0203_private_item |
| E0204 | Vòng phụ thuộc module | M3 | — (cần đa file, M3) |
| E0205 | Shadowing (D7) | M4 | E0205_shadowing |
| E0301 | Sai kiểu | M4 | E0301_type_mismatch |
| E0302 | Ép kiểu ngầm số (D1) | M4 | E0302_implicit_cast |
| E0303 | `==` trên số thực (D4) | M4 | E0303_float_eq |
| E0305 | `⌥` không đầy đủ (D6) | M5 | E0305_nonexhaustive |
| E0306 | Dùng biến chưa khởi tạo chắc chắn | M5 | — |
| E0307 | Thiếu đường trả về | M5 | E0307_missing_return |
| E0308 | Gán vào biến bất biến (D5) | M4 | E0308_assign_immutable |
| E0310 | Thiếu capability (D28) | M4.5 | E0310_missing_cap |
| E0401 | Dùng sau move | M6 | E0401_use_after_move |
| E0402 | Hai `&!` chồng nhau | M6 | E0402_double_unique |
| E0403 | `&!` chồng `&` | M6 | E0403_unique_vs_shared |
| E0404 | Trả tham chiếu tới local | M6 | E0404_return_local_ref |
| E0405 | Con trỏ thoát Arena (D27) | M6 | E0405_arena_escape |
| E0406 | Gán khi đang bị mượn | M6 | — |
| E0501 | Chia sẻ không `Send` qua luồng | M8 | — |
| E0502 | Vi phạm thứ tự khoá | M8 | — |
| E0601 | Obligation biên chắc chắn sai (tràn hằng) | M7 | E0601_const_overflow |
| E0602 | Chia 0 chắc chắn | M7 | E0602_const_div_zero |
| E0603 | Chỉ số ngoài biên chắc chắn | M7 | E0603_const_oob |
| E0604 | Ép kiểu mất thông tin chắc chắn | M7 | — |
| E0701 | Vòng lặp thiếu `↓` (D12) | M8 | E0701_loop_no_variant |
| E0702 | `↓` không giảm nghiêm ngặt | M8 | E0702_variant_not_decreasing |
| E0703 | Đệ quy thiếu `↓` | M8 | — |
| E0801 | Hiệu ứng suy ra ⊄ khai báo | M8 | E0801_effect_undeclared |
| E0802 | Hàm `∅` gọi hàm có hiệu ứng | M8 | — |
| E0901 | Tiền điều kiện chắc chắn sai | M7 | E0901_precondition_false |
| E0902 | Obligation không kiểm lúc chạy được mà chưa giải (D25) | M7 | — |
| E1001 | Nghĩa vụ `⚠` chưa giải (D21) | M7 | E1001_unsafe_unproven |
| E1101 | IR Guard: attestation lệch | M10 | — |
| E1201 | Ký tự bidi (D15) | M1 | E1201_bidi |
| E1202 | Ký tự zero-width/vô hình | M1 | E1202_zero_width |
| E1203 | Định danh trộn bảng chữ/homoglyph | M1 | E1203_homoglyph |
| E1204 | Nguồn không phải NFC | M1 | E1204_not_nfc |
| E9901 | ICE | M9 | — |
| S101 | Secret nằm trong nguồn | M9 | `tests/warn/S101_key_in_source` |
| S201 | `declassify` / Secret chảy ra sink | M9 | — |
| N0601 | Check tràn hạ thành runtime | M7 | `tests/pass/02_sum_loop` (expect note) |
| N0602 | Check biên mảng hạ thành runtime | M7 | — |
| N0901 | Tiền điều kiện hạ thành runtime | M7 | — |

Ghi chú: plan ghi match-đầy-đủ và capability thuộc "E01xx/E03xx/E06xx" / "E03xx/E08xx" mơ hồ → đề xuất
E0305/E0310 (xem OPEN_QUESTIONS Q4). Các test `—` do milestone sở hữu viết (cần đa file/pass chưa có).
