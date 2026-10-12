# Benchmark nền của backend llvm (M10)

Đo bằng `python tools/bench.py 9` ngày 2026-10-13, máy i5-12450H / 15,6 GB, Windows 11, gritc **debug** (chỉ ảnh hưởng cột "dựng"), exe sinh ra bởi LLVM 20.1.8 **-O0** (hồ sơ `dev`, mọi `Assert` giữ, `--checks=all`). Tệp: `tests/bench/*.gt`; tham chiếu Rust: `tools/bench_ref/*.rs` (cùng thuật toán, cùng kiểm tràn/biên).

| bench | công việc | dựng (gritc debug) | exe LLVM -O0 trung vị / min (9 lần) | Mops/s (theo min) | thông dịch (thu nhỏ) | thông dịch Mops/s | Rust -C opt-level=0 (trung vị / min) | Rust -O (trung vị / min) |
|---|---|---|---|---|---|---|---|---|
| loop_sum | 400,000,000 | 0.20 s | 0.52 s / 0.50 s | 804.1 | 2.41 s (1,000,000) | 0.42 | 0.92 / 0.83 s | 0.03 / 0.02 s |
| fib | 29,860,703 | 0.21 s | 1.10 s / 1.08 s | 27.6 | 0.22 s (57,313) | 0.26 | 0.17 / 0.16 s | 1.05 / 0.05 s |
| slice_sum | 200,000,000 | 0.19 s | 0.52 s / 0.49 s | 409.7 | 0.30 s (1,000) | 0.00 | 0.60 / 0.51 s | 0.05 / 0.04 s |
| threads | 4,000,000 | 0.22 s | 1.27 s / 0.28 s | 14.5 | — | — | — | — |

## Đọc số cho đúng (đừng suy diễn quá)
- **Nhiễu lớn trên máy này.** Lần chạy đầu của một exe mới thường chậm thêm ~1 s (quét tệp của Windows Defender); script bỏ một lần khởi động nóng nhưng trung vị vẫn bị kéo (vd `Rust -O fib`: 1.05 s trung vị vs 0.05 s min; `threads`: 1.27 s vs 0.28 s). **Dùng cột `min` làm mốc tốt nhất**, trung vị là trải nghiệm điển hình trên máy này.
- **Đây là mốc NỀN, không phải so sánh ngôn ngữ.** `-O0` của Grit vẫn là mã "mọi local là alloca", tuple kiểm tràn lưu/đọc qua bộ nhớ; chậm hơn Rust `-O` cỡ 10–50× ở vòng lặp số học và ~20× ở `fib` (1.08 s vs 0.05 s). So với Rust `-C opt-level=0`: tương đương ở `loop_sum`/`slice_sum`, chậm ~6,7× ở `fib` (0.16 s) vì Grit trả tuple `{T, i1}` qua bộ nhớ cho mỗi phép kiểm tràn và gọi hàm qua alloca.
- **Thông dịch tham chiếu** chỉ ~0.2–0.4 triệu thao tác/s (chậm ~2000× so với exe -O0 ở `loop_sum`); `slice_sum` thu nhỏ còn 1 000 phần tử vì thông dịch sao chép mảng khi `Use` (không đại diện cho backend). Thông dịch KHÔNG hỗ trợ `static`/`spawn` (bỏ qua ở `threads`).
- **`threads`**: 4 luồng HĐH thật, 2 triệu `Mutex::with` + 2 triệu `fetch_add`; 14,5 triệu thao tác khoá+nguyên tử/s (theo min). Kết quả luôn 4 000 000.
- **Chưa có hồ sơ `release` (-O2).** ADR-029 §7 chỉ cho phép khi `dev` ≡ `release` trên bộ đối chiếu; IR Guard phải chạy SAU tối ưu (LLVM có thể sinh `nsw`/`unreachable`). Việc để sau, cần Giang duyệt hướng.
- Thời gian dựng ~0.2 s là của `gritc` debug (cổng M4–M9 + codegen + liên kết); `gritc` release sẽ nhanh hơn, chưa đo.
