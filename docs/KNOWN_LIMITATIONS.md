# KNOWN LIMITATIONS (cập nhật theo milestone)

## Sau M0
- Chưa có compiler. Mọi thứ trong `docs/spec/` là **bản nháp chờ duyệt**.
- Token đo bằng proxy (tiktoken), không phải tokenizer Claude (ADR-002).
- `tests/` chưa có runner; kỳ vọng nằm ở dòng `// expect:` đầu file và chưa được máy kiểm chứng. Các mẫu **chưa được parse bởi công cụ nào**, nên có thể lệch grammar ở chỗ nhỏ — lỗi lộ ở M2, khi đó phải sửa mẫu hoặc grammar và ghi lại.
- Một số mẫu fail dựa vào hàm chưa định nghĩa trong core (`root_fs`, `raw_read`, `Arena::new`) vì lib/core viết ở M12; mẫu chỉ nhằm minh hoạ mã lỗi, cần rà lại ở milestone sở hữu mã đó.
- Chưa có mẫu đa file (E0204 vòng phụ thuộc module) — thuộc M3.
