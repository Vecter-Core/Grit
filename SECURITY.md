# Chính sách an toàn (SECURITY)

Grit là bản ứng viên `1.0.0-rc4` (CHƯA phải Seed). An toàn của nó dựa trên kiểm tra của trình biên dịch, corpus, oracle, đột biến và đọc mã cùng họ model; **chưa có kiểm toán độc lập**. Xem `docs/SAFETY_CLAIMS.md` và `docs/SEED_LIMITATIONS.md` để biết chính xác điều gì được và không được khẳng định.

## Điều cần báo
Quan tâm nhất là **chấp nhận sai (false negative)** của kiểm tra an toàn: `gritc` cho XANH (hoặc sinh/chạy sai) một chương trình mà theo `docs/spec/` phải bị từ chối, ví dụ truy cập ngoài biên, tràn số, chia 0, use-after-move, data race, vi phạm mượn/alias, hiệu ứng hoặc capability lọt qua, vòng lặp không dừng được chấp nhận, hỏng bộ nhớ trong mã sinh ra hay runtime.

## Cách báo
1. **Ưu tiên báo riêng**: dùng GitHub "Report a vulnerability" (Security → Private vulnerability reporting) của repo này, nếu đã được bật.
2. Nội dung: **mẫu tái hiện tối thiểu** (một tệp `.gt` nhỏ nhất có thể), lệnh đã chạy (`gritc check`/`run`, hồ sơ dev hay release), kết quả thực tế và kết quả mong đợi theo spec, phiên bản `gritc --version` và hệ điều hành.
3. Lỗi KHÔNG ảnh hưởng an toàn (từ chối oan, thông báo khó hiểu, hiệu năng) báo bằng Issue thường theo mẫu "user_bug".

Không gửi bí mật thật. Không có chương trình thưởng lỗi.
