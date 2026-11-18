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

## GÓI 5 S1 — độ nhạy đo bằng TỈ LỆ PHÁT HIỆN (CI run 44283688809, Linux, clang 20.1.2, mỗi đối chứng 30 lần chạy)
| đối chứng | loại | phát hiện |
|---|---|---|
| `race_pthread` (ghi toàn cục, 2 pthread) | dương | 30/30 |
| `race_gritrt_spawn` (2 worker do `gritrt_spawn`) | dương | 13/30 (≈ 0,43) — YẾU NHẤT |
| `race_narrow_ir` (cửa sổ HẸP: chỉ ghi ở vòng cuối của 200 000 vòng, qua `gritrt_spawn`) | dương | 17/30 |
| `grit_two_locks_nolock` (`two_locks.gt` → IR của gritc → `tsan_inject --drop-locks` → TSan) | dương, QUA PIPELINE GRIT | 16/30 |
| `grit_narrow_droplock_nolock` (`tests/tsan/narrow_droplock.gt`, một va chạm ở vòng cuối, bỏ khoá) | dương hẹp, QUA PIPELINE GRIT | 15/30 |
| `norace_atomic_pthread`, `grit_two_locks_lock`, `grit_narrow_droplock_lock` | ÂM (cùng chương trình, KHÔNG bỏ khoá) | 0/30 mỗi cái (đúng) |

* **Cơ chế bỏ khoá (S1.a) nằm ở công cụ kiểm thử**, không ở `gritc`/`gt-rt`: `tools/tsan_inject.py --drop-locks` xoá lệnh gọi `gritrt_mutex_lock/unlock` khỏi IR do gritc sinh (tương đương `Mutex::with` bỏ khoá). Làm vậy KHÔNG đổi mã sản phẩm (giữ rc3); `tools/test_tsan_inject.py` kiểm công cụ và kiểm `gt-rt`, `gt-driver`, `gt-codegen-llvm` không chứa cơ chế bỏ khoá. ĐIỂM KHÁC với đề (feature trong gt-rt): hỏi kiến trúc sư ở báo cáo cuối.
* **Kết luận độ nhạy:** TSan CÓ bắt race do Mutex bỏ khoá qua đúng đường dựng Grit (16/30 và 15/30 lần chạy; không phải 100 %), kể cả khi chỉ có MỘT va chạm hẹp. Tỉ lệ phát hiện yếu nhất ≈ 13/30.
* **Công thức:** nếu tỉ lệ phát hiện mỗi lần chạy là p thì xác suất bỏ sót qua n lần là (1−p)^n, CHỈ với race CÙNG DẠNG (hai luồng ghi/đọc một ô không đồng bộ). Với p = 13/30 và n = 50 lần/mẫu: (17/30)^50 ≈ 4,6·10⁻¹³; n = 400: ≈ 10⁻⁹⁹. Đây là xác suất "bỏ sót nếu mẫu có race dạng đó", KHÔNG phải xác suất mẫu không có race, và KHÔNG áp cho race tinh vi hơn (bộ nhớ yếu/thứ tự atomic, ATOMIC-01; phụ thuộc lịch luồng hiếm).
* **"400 lần không phát hiện"** (8 mẫu × 50, 0 báo cáo, CI cùng run) nay đi kèm: tỉ lệ phát hiện yếu nhất 13/30 trên race dạng đơn giản; vẫn chỉ nghĩa "không phát hiện trong các mẫu đã chạy".
* Còn giới hạn: chỉ Linux; build phân tích (không phải binary qua Guard cuối); gt-rt dựng TSan riêng; không kiểm bộ nhớ yếu; Windows (SRWLOCK) CHƯA KIỂM.
