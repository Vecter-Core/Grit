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

## GÓI 11 — bổ sung (2026-11-20; Windows trừ khi ghi khác)
- **Cỡ enum thực (S2.c, ĐO tĩnh, Windows, `GRITC_DUMP_IR`, gritc release dựng từ rc6, hồ sơ dev):** số lần KIỂU `{ i64, [k x i64] }` xuất hiện trong IR (không phải số giá trị lúc chạy): json {k=0: 10, k=1: 18, k=2: 70, k=4: 111}; sha256 {0: 10, 1: 18}; sortlines {0: 64, 1: 18}; stats {0: 10, 1: 28}; wc {0: 10, 1: 18}; wordfreq {0: 52, 1: 18, 4: 10}. Cỡ = 8 + 8k byte. Tag `u8` chỉ co được enum không payload (k=0: 8 → 1 byte, tối đa 7 byte mỗi giá trị); enum có payload căn lề 8 vẫn đệm lại nên KHÔNG lợi. 4 capstone CHƯA đo.
- **S2.d oracle ghi thẳng vào đích khi abort giữa chừng (SUY RA từ đọc mã, không đo):** theo D3 abort dừng ngay, không có handler đọc bộ nhớ ⇒ trạng thái nửa vời không quan sát được ⇒ không cần đo.
- **`bench_profile.py` sau khi sửa (S0.c; Windows; trung vị 5 lần, gritc release rc6; cột "release" = hồ sơ `release`; nhiễu Defender):**
```text
bench      dựng dev/release      dev trung vị (min–max)       release trung vị (min–max)    tăng tốc trung vị
fib(35)    0,29 / 0,20 s         0,110 s (0,101–0,114)        0,124 s (0,110–0,128)         x0,89
wc         0,26 / 0,24 s         0,340 s (0,331–0,347)        0,339 s (0,313–0,369)         x1,00
json       0,74 / 0,73 s         0,205 s (0,191–0,219)        0,076 s (0,061–0,092)         x2,71
sha256     0,39 / 0,42 s         1,688 s (1,575–1,762)        2,606 s (2,319–2,648)         x0,65
sortlines  0,72 / 0,66 s         0,428 s (0,412–0,462)        0,376 s (0,350–0,411)         x1,14
stats      0,30 / 0,29 s         0,203 s (0,177–0,241)        1,206 s (1,181–1,207)         x0,17
wordfreq   0,42 / 0,42 s         7,504 s (7,468–7,757)        6,203 s (6,016–6,525)         x1,21
```
  Quan sát (ĐO, một lượt, KHÔNG kết luận nguyên nhân): `stats` và `sha256` ở hồ sơ `release` CHẬM hơn `dev` (x0,17 và x0,65). Lần đo trước trong cùng ngày (`bench_profile` đứt ở sortlines) cho fib(35) dev 1,097 s (so với 0,110 s ở đây): nhiễu máy rất lớn giữa hai lượt (~10 lần ở fib), nên các số này KHÔNG dùng để ra quyết định hiệu năng; cần đo lại hai lượt khi máy rảnh. Thêm `wordfreq` (đầu vào như `wc`).
- **Chưa làm (CHƯA ĐO):** S2.a (`memcpy` của `Vec::push`: cần bản gt-rt có bộ đếm), S2.b (profile sortlines: chưa kiểm `perf` trong WSL), S2.e (commit charge Arena), S2.f (chưa sửa `PHASE2_PLAN.md` ngoài dòng D4 đã CHƯA ĐỦ ĐIỀU KIỆN ở GÓI 10).
