# STACK_PROBE — nhu cầu stack thật của 6 công cụ dogfood và 4 capstone (GÓI 4 S2.a, 2026-11-19)

Công cụ: `tools/stack_probe.py` (thủ công, KHÔNG trong cổng). Số liệu thô: `docs/STACK_PROBE.jsonl` (một dòng mỗi chương trình × hồ sơ). Cơ chế đo CHỈ có ở bản dựng kiểm thử của runtime: `gt-rt --features test-fault` đọc `GRITRT_TEST_STACK_BYTES`; bản sản xuất bỏ qua biến này (test `spawn_failed.rs` chứng minh với công tắc ép lỗi cùng feature; cùng cấu trúc `cfg(feature = "test-fault")` cho `thread_stack_bytes`).

## Phương pháp
- Mỗi chương trình dựng ở `dev` và `release-o2` bằng runtime test-fault; chạy với stack **4, 8, 16, 32, 64 MiB** và **256 MiB** (mặc định sản xuất, làm đối chiếu).
- **Đạt** = mã thoát 0 và đầu ra (stdout + stderr) BẰNG đầu ra ở 256 MiB (dogfood), hoặc mọi ca `capstone/check.py --large` khớp bản tham chiếu Rust (capstone).
- Bộ tệp lớn nhất: `wc`, `wordfreq`, `sortlines`: 32 MiB văn bản (dòng 1–12 từ, ≈ 2 010 từ khác nhau); `sha256`: 32 MiB byte ngẫu nhiên; `stats`: 10⁶ số nguyên không âm; `json`: ≈ 13 MiB, mỗi phần tử lồng 99 tầng mảng; capstone: bộ ca `gen_cases.py --large` (K1 1 triệu dòng 23,6 MB, K2 300 000 dòng, K3 99 000 lệnh, K4 200 000 dòng).

## Kết quả (20 hàng, tất cả như nhau)
| chương trình | dev | release-o2 |
|---|---|---|
| wc, json, sha256, sortlines, wordfreq, stats | đạt ở MỌI cỡ thử (4, 8, 16, 32, 64, 256 MiB) | đạt ở mọi cỡ thử |
| csvstat, calc, kvstore, grep-lite (capstone) | đạt ở mọi cỡ thử | đạt ở mọi cỡ thử |

⇒ **Stack nhỏ nhất vẫn đúng ≤ 4 MiB** cho cả 10 chương trình ở cả hai hồ sơ (4 MiB là cỡ nhỏ nhất đã thử; cỡ tối thiểu thật có thể thấp hơn, CHƯA đo).

## Đối chứng dương (cơ chế đo thật sự có tác dụng)
Chương trình đệ quy `↓` sâu 2 000 000 tầng (`stack_depth`): thất bại ở ≤ 128 MiB, chạy ở 192 MiB (dev); thất bại ở ≤ 64 MiB, chạy ở 96 MiB (release-o2) — số đo ở `docs/KNOWN_LIMITATIONS.md` (mục GÓI 3). Vậy biến ghi đè hoạt động và stack nhỏ KHÔNG che được tràn.

## Kết luận và biên an toàn
- **Giữ 256 MiB.** Lý do: (a) các chương trình thật ở đây dùng ≤ 4 MiB, tức biên ≥ 64× so với 256 MiB; (b) nhưng 256 MiB là yêu cầu của chương trình đệ quy sâu hợp lệ đã được M8 chứng minh dừng (2 000 000 tầng cần > 128 MiB ở dev); hạ xuống sẽ làm test `deep_terminating_recursion_runs_natively` chết.
- 256 MiB chỉ là giữ chỗ địa chỉ; chi phí thật là số luồng × 256 MiB không gian địa chỉ ảo (xem `tools/stress_spawn.sh`, K ≤ 100 đã chạy được).
- **Giới hạn đo:** đầu vào là bộ lớn nhất đã chọn, không phải cận trên toán học; chương trình khác (đặc biệt đệ quy phụ thuộc đầu vào) có thể cần hơn.
