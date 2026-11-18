# DECISION_COVERAGE — độ phủ nhánh của hàm quyết định của checker (GÓI 4 S4, tuỳ chọn)

**Trạng thái: CHƯA CHẠY.** Lý do: mục này ưu tiên thấp và ngân sách phút CI hạn chế (≈ 2 000 phút/tháng; chạy test cả workspace dưới `cargo llvm-cov --branch` chậm 2–5 lần, mà cổng Linux thường đã tốn khoảng một giờ). Theo lệnh, chuyển thành công cụ chạy thủ công: `tools/coverage.sh` (cần `cargo-llvm-cov` và Rust nightly; KHÔNG sửa mã để tăng phần trăm).

## Việc cần làm khi chạy
1. `bash tools/coverage.sh gt-oblig gt-borrowck gt-types gt-term gt-effects gt-guard` ⇒ `target/coverage/<crate>.json`.
2. Với mỗi hàm quyết định đã liệt kê ở `docs/E_PROMPT_OPUS.md` (bộ quyết định xoá/giữ check ở `gt-oblig`; `conflicts`, `regions`, `relate` ở `gt-borrowck`; `send.rs`, `select_impl`, khớp mẫu, E0313 ở `gt-types`; `gt-term`; `gt-effects`; `gt-guard`): liệt kê nhánh KHÔNG được chạy qua.
3. Mỗi nhánh: một thí nghiệm (chương trình `.gt` hoặc test dựng) chạm tới nó, hoặc ghi lý do không đạt tới (vd nhánh phòng thủ ICE `E9901` bất khả đạt tới).
4. Ghi kết quả vào bảng dưới (chưa có dòng nào).

| crate | hàm | nhánh không phủ | thí nghiệm hoặc lý do | kết quả |
|---|---|---|---|---|
| (chưa chạy) | | | | |

**CHƯA KIỂM:** mọi phần trăm độ phủ; không có số nào trong tài liệu này.
