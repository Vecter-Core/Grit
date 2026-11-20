# NEXT_PRODUCT_BATCH — thay đổi MÃ SẢN PHẨM đang chờ (lập GÓI 10, 2026-11-20)

Mã sản phẩm (`crates/`, `runtime/`, `lib/`) ĐÓNG BĂNG sau cổng xanh rc6 (`16c37f0`). Danh sách này gom mọi thay đổi cần làm; áp dụng MỘT LẦN khi có kích hoạt, rồi MỘT nhãn rc7 và MỘT cổng.
**Kích hoạt:** (a) phát hiện hỏng bộ nhớ hoặc âm tính giả (false negative) của kiểm tra an toàn: sửa NGAY (test đỏ trước, `FIELD_BUGS.md`), nâng rc7, cổng lại; hoặc (b) quyết định của kiến trúc sư/Giang.

| # | Thay đổi | Lý do | Nguồn | Trạng thái |
|---|---|---|---|---|
| 1 | Chú thích `{detach}` trong chữ ký prelude `spawn` (`lib/core/prelude.gt:15`) không khớp hành vi (không được thi hành) | tài liệu đã đồng bộ theo hành vi (GÓI 10); mã vẫn mang chú thích sai | ADR-C12 | CHỜ — sửa prelude đổi hợp đồng TCB (cần duyệt) |
| 2 | Đường siết spawn: cho phép khai `{detach}` ở mã người dùng (E1002 hiện chặn) rồi bắt buộc hàm gọi `spawn` khai `detach` | `! ∅` hiện không bảo đảm "không có luồng" | ADR-C12 | CHỜ — cần ADR riêng, làm TRƯỚC kênh có kiểu |
| 3 | `declassify` không có `Cap<Secret>` ⇒ lỗi kiểu ĐỎ (hiện chỉ cảnh báo S201) | D28: dùng năng lực cần `Cap` | ADR-C13 | HOÃN — thay đổi phá vỡ; bán kính chưa đo bằng chạy thật (27 tệp theo `grep`) |
| 4 | Xoá `Secret<T>` có bảo đảm (`secret_wipe`) | hiện không có chỗ nào xoá | `PHASE2_ADR_DRAFTS.md` ADR-P2-03 | ĐỀ XUẤT |
| 5 | Phát hiện từ S1 (cổng Linux WSL) | — | GÓI 10 S1 | KHÔNG có lỗi mã: cổng Linux GREEN. Chỉ lệch môi trường (toolchain, `python`, `libffi-dev`) — sửa `tools/setup_cloud.sh` là việc công cụ, không phải mã sản phẩm |
| 6 | Phát hiện từ S3: FB-C34 tắt bản sửa ⇒ ĐỎ (đúng). Đột biến S1d: 27/110 sống sót; cần THÊM TEST (tests/ của gt-term) để giết ~22 ca hướng chấp-nhận-thêm | GÓI 10 S3.a/b | `docs/MUT_GT_TERM_GOI10.md` | CHỜ (thêm test không đổi mã sản phẩm; làm được ngay khi có phiên) |
| 7 | Kết quả R11c (Opus đọc mã) | — | sau này | CHƯA CÓ |

Số mục đang chờ thật (1–4): **4** (mục 5–7 là chỗ giữ chỗ).

## Bổ sung GÓI 11 (2026-11-20)
| # | Thay đổi | Lý do | Nguồn | Trạng thái |
|---|---|---|---|---|
| 8 | Vòng khởi tạo static qua lời gọi hàm (`■ X ≔ f()`, `f` đọc `X`) hiện `gritc check` XANH nhưng backend LLVM từ chối và thông dịch báo lỗi nội bộ: thêm kiểm đồ thị khởi tạo static ở M4/M8 để ĐỎ ngay ở `check` | nhất quán D12/fail-closed ở giai đoạn kiểm, không chờ backend | `docs/CONTROL_TRANSFER_INVENTORY.md` hàng 13 | CHỜ (không phải âm tính giả chạy được: backend từ chối) |
| 9 | 25 đột biến gt-term còn sống: thêm test giết (đặc biệt nhánh `None => true` ở `dyn_fn_arg`, `takes_fn_param`, `has_back_edge` 0 lần trong 974 đầu vào) | `docs/MUT_GT_TERM_GOI10.md` | GÓI 11 S1 | CHỜ (chỉ test) |
| (đã làm) | FB-C35 sửa trong GÓI 11 (commit `7c6334c`, rc7 `41ae5e7`) — KHÔNG còn chờ | — | FIELD_BUGS | XONG |
