# PHASE2_MEASUREMENTS — đo cho Giai đoạn 2 (GÓI 10 S2; 2026-11-20)

Trạng thái: **CHƯA ĐO** các phép cần cargo/bench/gate: `tools/preflight.sh` ĐỎ cả 3 lần, ở 3 thời điểm khác nhau trong gói (bộ nhớ ảo khả dụng 2 119 232 → 2 643 856 KB < 4 GB; `Win32_OperatingSystem.FreeVirtualMemory` ≈ 2,19 GB trên tổng 33,2 GB). Nghi nguyên nhân (CHƯA KIỂM): tiến trình `vmmemWSL` giữ ≈ 4 GB commit vì phân phối WSL `OpenClawGateway` đang chạy (không phải của agent; agent KHÔNG tắt: `wsl --shutdown` sẽ dừng nó). Theo điều kiện dừng (b) không chạy lô nặng. Không có số đo nào dưới đây được bịa.

| # | Phép đo (FACTCHECK mục 10) | Kết quả | Nhãn |
|---|---|---|---|
| 1 | Thời gian dựng `gritc` release; bench với gritc release | CHƯA ĐO (cần cargo; preflight đỏ) | — |
| 2 | Profile `sortlines` | CHƯA ĐO | — |
| 3 | Phần `memcpy` trong `Vec::push` | CHƯA ĐO. Hệ quả cho D4 (`PHASE2_PLAN.md`): chưa đủ điều kiện | — |
| 4 | Cỡ enum thực trong IR của công cụ | CHƯA ĐO (cần `gritc build` + đọc `.ll`; chưa chạy). Từ ĐỌC MÃ: `8 + 8k` byte, cỡ mọi biến thể bằng nhau (`PHASE2_FACTCHECK.md` mục 2) | SUY RA từ mã, không phải đo |
| 5 | Dev vs release-o2 vs Rust `-O`, hai lượt (lưu ý `bench_profile.py` không có `wordfreq`) | CHƯA ĐO | — |
| 6 | Tần suất lỗi đảo đối số (`USER_BUGS.md`, `apps/`) | **0 ca xác nhận** trong `docs/USER_BUGS.md` (23 dòng, chỉ có mẫu, 0 mục), `docs/APPS_LOG.md`, `docs/CAPSTONE_LOG.md`, `docs/RUN_REPORTS.md`. Tìm 15 mẫu từ khoá (swap, đổi chỗ, thứ tự đối số, nhầm đối số, argument order, sai thứ tự, đối số ngược, đảo hai, hoán vị, …) ⇒ 0 khớp. "đảo" ở `RUN_REPORTS.md:42,133` là "hoàn nguyên", không phải đảo đối số. Các dòng nhắc "đối số" (`APPS_LOG.md:12,30`, `CAPSTONE_LOG.md:19,22`) là ca thiếu/thừa đối số dòng lệnh, `&![T]`→`&[T]` (E0301), mượn hai pha: KHÔNG phải đảo đối số. | ĐO (đọc từng dòng khớp) |
| 7 | Oracle ghi thẳng vào đích khi abort giữa chừng | KHÔNG ĐO được ngoài cây (cần bản thử `lower.rs` + cargo) | — |
| 8 | `cargo build --no-default-features -p gt-driver` (không LLVM) | CHƯA ĐO (cargo) | — |
| 9 | Commit charge của Arena đặt trước | KHÔNG ĐO (không an toàn khi bộ nhớ ảo đã gần cạn: nguyên mẫu `VirtualAlloc` thêm áp lực lên đúng tài nguyên đang thiếu) | — |

**Cảnh báo đánh giá (mục 6):** lỗi đảo hai đối số cùng kiểu KHÔNG phải lỗi biên dịch, chỉ lộ thành đầu ra sai; đếm lỗi biên dịch luôn ra 0 và vô nghĩa. Con số 0 ở trên là "chưa thấy ca đảo nào trong các nhật ký đã đọc" — các nhật ký rất ngắn (35 và 28 dòng, hai phiên capstone chưa có hai bản sạch theo ngưỡng của N9), nên đây KHÔNG phải bằng chứng rằng lỗi không xảy ra. N9 giữ HOÃN theo `PHASE2_PLAN.md`.
**Máy:** i5-12450H, Windows 11 (số của `BENCHMARK.md`); Windows Defender làm lệch lần chạy đầu ⇒ dùng min. Khi preflight xanh: chạy `tools/bench_profile.py` (min và trung vị) theo `PHASE2_FACTCHECK.md` mục 10.
