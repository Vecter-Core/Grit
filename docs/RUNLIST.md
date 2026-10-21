# RUNLIST — danh sách việc chạy (tạo mới 2026-10-21 trên nhánh `claude/gallant-darwin-m251cq`)

Trước đây tham chiếu "RUNLIST" nằm ở bản Windows chưa lên origin; tệp này dựng lại từ lệnh Seed 1.0 (bản sửa) và HANDOFF §6. Trạng thái: XONG / ĐANG / CHƯA / KHÔNG XONG (kèm phần đã làm). Cập nhật sau mỗi mục.

| Mã | Việc | Trạng thái | Ghi chú |
|---|---|---|---|
| R1 | preflight + quét NUL chạy được Linux và Windows; bundle sao lưu | ĐANG | `tools/preflight.sh`, `tools/nul_scan.py` (+ test), giai đoạn `nul` của `gate.sh`: XONG phần Linux; Windows chưa kiểm (`preflight.sh` có nhánh PowerShell cho RAM); bundle: làm ở mốc rc1 |
| R2 | hàm-làm-giá-trị: M6 qua giá trị, `⊳` qua giá trị, hiệu ứng, bảng INTRINSIC_AS_VALUE | ĐANG | đã ĐO bảng (docs/INTRINSIC_AS_VALUE.md); mở: FB-C5 (hiệu ứng hàm kết hợp), FB-C6 (`⊳`), vùng mượn (S1) |
| R3 | FAIL_OPEN_AUDIT rà hết bằng `tools/fail_open_scan.py` | CHƯA | S7.a |
| R4 | `GRITRT_ARENA_CHECK` chỉ bật thêm, không tắt được ở dev | CHƯA | S1.b |
| R5 | flake `llvm_sort` ≥ 100 lần; std ở release/release-o2; Ord kiểu ghép; đo sortlines 5 lần | CHƯA | S7.b |
| R6 | Arena chia sẻ `&'a Arena` (ADR-038 / Q83) | CHƯA | S2; nếu không đạt, HashMap thiết kế quanh `&!` độc quyền (ghi vào AI_GUIDE) |
| R7 | HashMap | CHƯA | S2 (capstone K3 cần) |
| R8 | READINESS: bảng §7 và sáu mục SEED 1.0 | CHƯA | S8.d |
| R9 | `tools/fuzz_loop.sh` tích luỹ có giới hạn (tối đa 2 giờ tổng; điều kiện #2 ghi "ĐANG TÍCH LUỸ") | CHƯA | S7.d |
| R10 | 69 đột biến "xoá thêm check" của M7 (`analysis.rs` 62, `driver.rs` 7) | CHƯA | S7.c |
| R11 | đọc mã đối kháng nhóm CAO bằng subagent (E_PROMPT.md; trạng thái TCB: "đã qua đọc mã đối kháng bằng subagent cùng họ model") | CHƯA | S7.e |

## Đã XONG trước nhánh này (giữ nguyên, từ HANDOFF)
M0–M11 có mã; std (47 hàm, 35 chương trình ba nguồn mỗi hàm); 6 công cụ dogfood; B1/B2-B3/D1; đột biến G1+FB-9.

## Đã XONG trên nhánh này (kiểm toán ngoài, xem FIELD_BUGS FB-C1…FB-C8)
F-01…F-08, port Linux, ngăn xếp 256 MiB, clippy sạch, CI/deny (chưa chạy trên GitHub).
