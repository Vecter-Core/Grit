# PHASE2_MEASUREMENTS — đo cho Giai đoạn 2 (GÓI 10 S2; 2026-11-20)

Trạng thái (cập nhật sau khi Giang đóng WSL `OpenClawGateway`, preflight PASS): đã đo được mục 1, 5, 8 và một phần 6; các mục 2, 3, 4, 7, 9 CHƯA ĐO (lý do ở từng dòng). Số nào cũng ghi nguồn ĐO hay SUY RA. Máy: i5-12450H, Windows 11; nhiễu Windows Defender; một cargo tại một thời điểm, `CARGO_BUILD_JOBS=2`. `gritc` bản release dựng từ `08a4931` (mã sản phẩm = rc6).

| # | Phép đo | Kết quả | Nhãn |
|---|---|---|---|
| 1 | Dựng `gritc` release nguội (`cargo build --release -p gt-driver`, clone sạch) | **1 m 39,8 s** (một lần; không có min/trung vị). Tệp `gritc.exe` 5 854 720 B, trùng cỡ với bản dựng trước ở cây chính | ĐO |
| 1 | `bench_profile.py 7` với gritc release (trung vị 7 lần, giây; cột "release" = hồ sơ `release`) | fib(35): dev 1,097 / release 1,109 · wc: 1,319 / 1,304 · json: 1,196 / 1,073 · sha256: 2,440 / 3,337 (release CHẬM hơn dev; chưa giải thích, có thể nhiễu; CHƯA lặp) · đầu ra dev = release ở cả 4. min không in được vì bench dừng ở `sortlines` | ĐO |
| 1 | `bench_profile.py` với `sortlines` và `stats` | **LỖI CÔNG CỤ:** `gritc build examples/sortlines/src` ⇒ `E0309 không có phương thức cmp` (thư mục không có gói `std`; `bench_sortlines_g2.py` sao chép `lib/std/src` cạnh nguồn nên chạy được). Bench này hỏng ở `sortlines` từ trước; CHƯA sửa công cụ | ĐO (lỗi) |
| 5 | `bench_sortlines_g2.py` (33 554 438 B, 1 636 921 dòng; 5 lần; giây) | grit dev: trung vị 2,831 (min 2,632) · grit release-o2: 1,253 (min 1,250) · Rust -O0: 2,207 (min 2,085) · Rust -O: 0,968 (min 0,917). SHA-256 đầu ra GIỐNG nhau ở cả 4 (`45459d06…ef51`). Tỷ lệ release-o2 / Rust -O ≈ 1,29 (SUY RA từ trung vị) | ĐO |
| 5 | `arena_scaling` (hồ sơ theo script, CHƯA xác minh) | 100 000 dòng 0,179 s; 400 000: 0,597 s; 1 636 921: 2,741 s (trung vị 3 lần) | ĐO |
| 8 | `cargo build --no-default-features -p gt-driver` | **XANH** (5,0 s trên cache phụ thuộc có sẵn, nên KHÔNG phải thời gian nguội); `gritc --version` ⇒ "backend mặc định: interpreter"; chạy với PATH KHÔNG có LLVM: `gritc check` trên **433/433** tệp `tests/pass/*.gt` ⇒ green/yellow, 0 ca khác | ĐO |
| 2 | Profile `sortlines` (WPR/xperf) | CHƯA ĐO (không có công cụ profile trên máy; không cài thêm) | — |
| 3 | Phần `memcpy` trong `Vec::push` | CHƯA ĐO (cần bộ đếm trong bản dựng thử runtime). SUY RA từ mã: tăng ×2 sàn 4 ⇒ tổng byte sao chép ≤ cỡ cuối (khuếch đại khấu hao ≤ 2×); `sortlines` gọi `reserve` một lần nên không đại diện. D4 vẫn CHƯA ĐỦ ĐIỀU KIỆN | SUY RA |
| 4 | Cỡ enum thực trong IR | KHÔNG ĐO được: `gritc build` không có tuỳ chọn phát IR (`--help`); cần thêm mã hoặc test mới | — |
| 6 | Đảo đối số | 0 ca xác nhận trong 4 nhật ký (xem bên dưới) | ĐO (đọc) |
| 7 | Oracle "ghi thẳng vào đích" | KHÔNG ĐO được ngoài cây | — |
| 9 | Commit charge Arena đặt trước | CHƯA ĐO | — |

Phép đo 6 (từ lần đọc đầu, không đổi): 0 ca đảo đối số xác nhận trong `docs/USER_BUGS.md` (0 mục), `APPS_LOG.md`, `CAPSTONE_LOG.md`, `RUN_REPORTS.md`; 15 mẫu từ khoá đều 0 khớp. Cảnh báo: lỗi đảo hai đối số cùng kiểu KHÔNG phải lỗi biên dịch, chỉ lộ thành đầu ra sai; nhật ký rất ngắn nên 0 không chứng minh lỗi không xảy ra. N9 giữ HOÃN.
