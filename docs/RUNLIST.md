# RUNLIST — danh sách việc chạy (tạo mới 2026-10-21 trên nhánh `claude/gallant-darwin-m251cq`)

Trước đây tham chiếu "RUNLIST" nằm ở bản Windows chưa lên origin; tệp này dựng lại từ lệnh Seed 1.0 (bản sửa) và HANDOFF §6. Trạng thái: XONG / ĐANG / CHƯA / KHÔNG XONG (kèm phần đã làm). Cập nhật sau mỗi mục.

| Mã | Việc | Trạng thái | Ghi chú |
|---|---|---|---|
| R1 | preflight + quét NUL chạy được Linux và Windows; bundle sao lưu | ĐANG | `tools/preflight.sh`, `tools/nul_scan.py` (+ test), giai đoạn `nul` của `gate.sh`: XONG phần Linux; Windows chưa kiểm (`preflight.sh` có nhánh PowerShell cho RAM); bundle: làm ở mốc rc1 |
| R2 | hàm-làm-giá-trị: M6 qua giá trị, `⊳` qua giá trị, hiệu ứng, bảng INTRINSIC_AS_VALUE | XONG (2026-10-21, Linux) | bảng ĐO trước/sau; FB-C5, FB-C6 đã sửa (đỏ trước); M6 qua giá trị: bộ sinh `fn_value_ref` + 7 mẫu; ADR-C01/C02 |
| R3 | FAIL_OPEN_AUDIT rà hết bằng `tools/fail_open_scan.py` | CHƯA | S7.a |
| R4 | `GRITRT_ARENA_CHECK` chỉ bật thêm, không tắt được ở dev | XONG (2026-10-21, Linux) | test đảo `dev_check_cannot_be_switched_off_by_environment` (đỏ trước `963da34`, xanh `53bfb4d`); arena_dynamic 6/6, gt-rt 6/6 |
| R5 | flake `llvm_sort` ≥ 100 lần; std ở release/release-o2; Ord kiểu ghép; đo sortlines 5 lần | CHƯA | S7.b |
| R6 | Arena chia sẻ `&'a Arena` (ADR-038 / Q83) | XONG (2026-10-21, Linux) | ADR-C03; oracle `arena_vec`; FB-C9 tìm thấy và sửa |
| R7 | HashMap | CHƯA | S2 (capstone K3 cần) |
| R8 | READINESS: bảng §7 và sáu mục SEED 1.0 | CHƯA | S8.d |
| R9 | `tools/fuzz_loop.sh` tích luỹ có giới hạn (tối đa 2 giờ tổng; điều kiện #2 ghi "ĐANG TÍCH LUỸ") | CHƯA | S7.d |
| R10 | 69 đột biến "xoá thêm check" của M7 (`analysis.rs` 62, `driver.rs` 7) | CHƯA | S7.c |
| R11 | đọc mã đối kháng nhóm CAO bằng subagent (E_PROMPT.md; trạng thái TCB: "đã qua đọc mã đối kháng bằng subagent cùng họ model") | CHƯA | S7.e |

## Đã XONG trước nhánh này (giữ nguyên, từ HANDOFF)
M0–M11 có mã; std (47 hàm, 35 chương trình ba nguồn mỗi hàm); 6 công cụ dogfood; B1/B2-B3/D1; đột biến G1+FB-9.

## Đã XONG trên nhánh này (kiểm toán ngoài, xem FIELD_BUGS FB-C1…FB-C8)
F-01…F-08, port Linux, ngăn xếp 256 MiB, clippy sạch, CI/deny (chưa chạy trên GitHub).
