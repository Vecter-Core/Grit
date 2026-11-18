# USER_BUGS — lỗi gặp khi DÙNG Grit thật (GÓI 5 S4)

Ghi mục mới ở CUỐI tệp theo mẫu. `python tools/triage_user_bugs.py` đếm mục theo loại.

## Hai loại và cách xử lý
* **CHẤP NHẬN SAI (nghiêm trọng)** — `gritc` chấp nhận (xanh/vàng) hoặc chạy sai một chương trình mà theo tài liệu/spec phải bị từ chối hoặc phải cho kết quả khác. Đây là **false negative**, loại nghiêm trọng nhất. Người xử lý: (1) thêm chương trình tối thiểu vào corpus VĨNH VIỄN (`tests/fail/` hoặc `tests/pass/` kèm kỳ vọng), (2) viết test ĐỎ trước, (3) sửa ở commit riêng, (4) ghi FB-C vào `docs/FIELD_BUGS.md`. Không đóng mục khi chưa làm đủ bốn bước.
* **TỪ CHỐI OAN** — `gritc` từ chối (đỏ `E…`) chương trình đúng. Đây là fail-closed (an toàn nhưng phiền): xếp theo mức phiền (chặn việc / có cách né / hiếm), KHÔNG chặn tiến độ; nếu có cách né thì ghi vào `docs/SEED_LIMITATIONS.md`.

## Mẫu mục (sao chép, điền, đặt dưới dòng `<!-- MỤC MỚI Ở DƯỚI -->`)
```
### UB-<số>: <tiêu đề ngắn>
- Ngày: YYYY-MM-DD
- Chương trình: apps/<tên>/src/<tệp>.gt (hoặc đường dẫn tệp tái hiện)
- Hành vi mong đợi: ...
- Hành vi thực tế: ... (lệnh, mã thoát, đầu ra rút gọn)
- Mã lỗi: E.... hoặc "không có" (chấp nhận sai)
- Loại: từ chối oan | chấp nhận sai (nghiêm trọng)
- Mức phiền (chỉ từ chối oan): chặn việc | có cách né | hiếm
- Tệp tái hiện tối thiểu: tests/user_bugs/UB-<số>.gt
- Trạng thái: mở | đang xử lý | đóng (kèm commit, FB-C nếu có)
```

<!-- MỤC MỚI Ở DƯỚI -->
