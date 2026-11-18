# TSAN_RESULTS — ThreadSanitizer trên IR của Grit (GÓI 4 S3, 2026-11-19)

**ĐÂY LÀ BUILD PHÂN TÍCH, KHÔNG PHẢI binary đã qua Guard cuối.** Quy trình (`tools/tsan_check.sh`, job CI `tsan`, chỉ Linux): `gritc build --profile=dev` sinh IR (đã qua Guard-pre) qua `GRITC_DUMP_IR` ⇒ `tools/tsan_inject.py` chèn thuộc tính `sanitize_thread` vào MỌI hàm định nghĩa ⇒ `clang-20 -fsanitize=thread` (Ubuntu 24.04, 20.1.2) ⇒ liên kết với `gt-rt` dựng bằng Rust nightly `-Zsanitizer=thread -Zbuild-std=std,panic_abort`. Thời gian job ≈ 2–3 phút (tsan only: 8 phút khi dựng lạnh).

## (a) Đối chứng — chạy TRƯỚC (đường kiểm có nghĩa)
| đối chứng | kết quả (CI, 2026-11-18, run `44241971280`) |
|---|---|
| `tests/tsan/race_pthread.ll`: hai luồng pthread cùng ghi một ô toàn cục không đồng bộ (đua dữ liệu đã biết, mức IR vì Grit không cho viết đua) | **CAUGHT** — 3/3 lần chạy báo `data race` (cả ở các run trước: 3/3) |
| `tests/tsan/race_gritrt_spawn.ll`: hai worker do `gritrt_spawn` của gt-rt tạo cùng ghi một ô | **CAUGHT** — 2/3 lần chạy báo (ở run khác 2/3 và 3/3): xác suất bắt mỗi lần chạy ≈ 70–100 %, KHÔNG phải 100 % |
| `tests/tsan/norace_atomic_pthread.ll`: cùng cấu trúc nhưng ghi NGUYÊN TỬ | **không báo (đúng)** 3/3 |
⇒ đường kiểm HOẠT ĐỘNG: TSan thấy đua dữ liệu của luồng do `gritrt_spawn` tạo và không báo sai với ghi nguyên tử.

## (b) Mẫu Grit thật (mỗi mẫu 50 lần chạy, tổng 400 lần)
| mẫu | tham chiếu `__tsan_` sau khi clang chèn (xác nhận có theo dõi) | thoát khác 0 | báo `data race` |
|---|---:|---:|---:|
| `tests/samples/threads.gt` (4 luồng, Atomic + Mutex tĩnh) | 49 | 0 | 0 |
| `tests/samples/two_locks.gt` (2 luồng, khoá lồng) | 52 | 0 | 0 |
| `tests/pass/E0501_ok_spawn_cell_by_value.gt` | 26 | 0 | 0 |
| `tests/pass/E0501_ok_spawn_enum_array.gt` | 47 | 0 | 0 |
| `tests/pass/E0501_ok_spawn_int.gt` | 26 | 0 | 0 |
| `tests/pass/E0501_ok_spawn_struct.gt` | 26 | 0 | 0 |
| `tests/pass/E0501_ok_static_atomic.gt` | 35 | 0 | 0 |
| `tests/pass/E0501_ok_static_mutex.gt` | 39 | 0 | 0 |
(`E0502_ok_spawn_worker.gt` không có `main` ⇒ không chạy được.)

## Phân loại báo cáo
Tổng báo cáo `data race` ở mẫu thật: **0** ⇒ không có gì để phân loại (đua thật / giả do khoá tự cài / giới hạn công cụ). Khoá tự cài: trên Linux `gt-rt` dùng khoá quay + nhường bằng atomic nên TSan hiểu quan hệ happens-before; **không cần** biệt lệ (suppression) và KHÔNG dùng suppression nào. SRWLOCK trên Windows không kiểm được bằng cách này.

## Kết luận (nói đúng mức)
**TSan: không phát hiện đua dữ liệu trong 8 mẫu × 50 lần chạy (400 lần) ở build phân tích này.** Đây KHÔNG phải "không có đua dữ liệu". Giới hạn: chỉ các lịch thực sự xảy ra; dev profile (có kiểm Arena); 8 chương trình nhỏ, 1–4 luồng; không thử bộ nhớ yếu (x86 TSO); chỉ Linux; Windows (SRWLOCK) chưa kiểm. Đối chứng cho thấy xác suất bắt một đua cố ý ≈ 70–100 % mỗi lần chạy.
