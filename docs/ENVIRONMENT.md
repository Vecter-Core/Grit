# Hướng dẫn môi trường (Windows 11, `x86_64-pc-windows-msvc`)

Tệp này gom MỌI thứ môi trường mà phiên sau cần biết. Chi tiết bẫy lịch sử ở `docs/KNOWN_LIMITATIONS.md` mục "Môi trường build trên Windows".

## 1. Máy và dung lượng (đo 2026-10-09)
| | |
|---|---|
| CPU / RAM | i5-12450H (12 luồng logic) / 15,6 GB RAM vật lý |
| Ổ C: | 275,6 GB; **~10 GB trống** (chứa `%TEMP%` mặc định và **paging file**) |
| Ổ D: | 200,4 GB; ~151 GB trống — repo, `target/`, công cụ ở đây |
| Paging file | `C:\pagefile.sys`, tự quản lý, cấp ~20,9 GB; bộ nhớ ảo (commit limit) tổng ~36 GB |

## 2. LLVM (cho M10; CHƯA dùng trong mã)
- **Phiên bản:** LLVM **20.1.8**, bản chính thức `clang+llvm-20.1.8-x86_64-pc-windows-msvc.tar.xz` (940 MB tải về). KHÔNG build LLVM từ nguồn.
- **Vị trí cố định:** `D:\Tools\llvm-20.1.8` (đã trích sẵn tập tệp cần thiết, 662 MB: `bin\llvm-config.exe`, `bin\LLVM-C.dll`, `lib\*.lib` (236 tệp), `include\llvm`, `include\llvm-c`).
- **Biến môi trường:**
  - `GRIT_LLVM_DIR=D:\Tools\llvm-20.1.8` — build script của `gt-codegen-llvm` đọc biến này; thiếu ⇒ lỗi rõ ràng, không đoán đường dẫn.
  - `PATH` phải chứa `%GRIT_LLVM_DIR%\bin` lúc CHẠY (cần `LLVM-C.dll`).
- **Cách liên kết (đã kiểm thật, ADR-029 §2):** liên kết ĐỘNG `LLVM-C.lib` ↔ `LLVM-C.dll`. Hai đường mặc định của `llvm-sys` GÃY với bản chính thức này: tĩnh đòi `libxml2s.lib` (không kèm), động đòi `LLVM-20.dll` (không kèm; chỉ có `LLVM-C.dll`). Vì vậy phải dùng `no-llvm-linking` + chỉ thị liên kết riêng. Dự phòng: tạo `libxml2s.lib` rỗng bằng `lib.exe` rồi liên kết tĩnh.
- **Kiểm nhanh:** `D:\Tools\llvm-20.1.8\bin\llvm-config.exe --version` ⇒ `20.1.8`; `--host-target` ⇒ `x86_64-pc-windows-msvc`.
- **Băm SHA-256 (đã tự tính, khớp số GitHub công bố):** gói `clang+llvm-20.1.8-x86_64-pc-windows-msvc.tar.xz` (939 541 912 byte) = `f229769f11d6a6edc8ada599c0cda964b7dee6ab1a08c6cf9dd7f513e85b107f`. Tệp đã trích: `bin\LLVM-C.dll` = `b84e81d2ffa8fb030b58f1d9ad75ef3ab37f63cd094993c302d9efb34242eaf1`; `bin\llvm-config.exe` = `1a3dd9f3e6769ccd5d7a27135ef85e58f0cb4be3db6765a1c6086b77fb326884`; `lib\LLVM-C.lib` = `bd7e55563f21a9d2e4c0a5cdb48354d6d99e348fd93edef72eb1a647ae311be9`. Tải: `https://github.com/llvm/llvm-project/releases/download/llvmorg-20.1.8/clang%2Bllvm-20.1.8-x86_64-pc-windows-msvc.tar.xz`; trích chọn lọc: `tar -xJf - --strip-components=1 --wildcards '*/bin/llvm-config.exe' '*/bin/LLVM-C.dll' '*/lib/*.lib' '*/include/llvm/*' '*/include/llvm-c/*'`.
- **Khi chạy:** `gritc` (backend LLVM) cần `LLVM-C.dll` trên `PATH`. **Binary do `gritc` sinh cho chương trình người dùng KHÔNG phụ thuộc `LLVM-C.dll`** (LLVM chỉ sinh object; exe = object + gt-rt, kiểm bằng `dumpbin /dependents` ở test M10). Nạp DLL trễ (`/DELAYLOAD`): `gritc check/fmt/explain` chạy được dù PATH không có LLVM.
- **`libxml2s.lib` rỗng = WORKAROUND dự phòng** (chỉ nếu liên kết tĩnh phải dùng); hiện KHÔNG dùng.

## 3. Quy tắc build/test (Giang, 2026-10-08; áp dụng MỌI lúc)
- Không chạy hai tiến trình `cargo` cùng lúc. `CARGO_BUILD_JOBS=2`; test với `--test-threads=2` (`t.sh`, `tools/gate.sh` đã đặt sẵn).
- Đột biến (`tools/mut.py`): **1 worker**, lô nhỏ, kết quả ghi nối vào `.jsonl` ngay sau mỗi đột biến. Bản sao làm việc `D:\Grit_mut\w0` chép `crates/`, `tests/`, `lib/`, `docs/` (cần `docs/errors` cho `gritc explain` và `lib/core/prelude.gt` cho `gritc`).
- `GT_FULL8=1` (vét cạn 8 bit đầy đủ, ~11 phút) chỉ chạy MỘT lần khi máy rảnh.
- `PYTHONIOENCODING=utf-8` cho mọi script Python in tiếng Việt; không dùng `\` trong chuỗi Python gửi qua heredoc shell (dùng công cụ Write/Edit).

## 4. WinError 1455 — thiếu BỘ NHỚ ẢO, không phải thiếu ổ đĩa
Lỗi `os error 1455` ("The paging file is too small for this operation to complete") nghĩa là **commit limit = RAM + paging file** đã cạn, KHÔNG phải ổ đĩa đầy. Gặp khi biên dịch/liên kết song song (đặc biệt LLVM, `rustc` nhiều crate cùng lúc) hoặc khi chạy nhiều việc nặng.
- **Người dùng kiểm cài đặt paging file (Giang đã báo 2026-10-12: sẽ TỰ chuyển paging file sang D: và restart):** Settings → System → About → Advanced system settings → Performance → Advanced → Virtual memory. Khuyến nghị: để hệ thống tự quản lý NHƯNG giữ ổ chứa paging file còn ≥ 15 GB trống, hoặc đặt cỡ cố định ≥ 24 GB, **ưu tiên chuyển paging file sang ổ D:** (ổ C: chỉ còn ~10 GB nên paging file không giãn được). Đây là cài đặt hệ thống — công cụ tự động KHÔNG đổi.
- Giữ `CARGO_BUILD_JOBS` thấp (≤ 2), đóng chương trình nặng trước khi liên kết LLVM, không chạy hai bản dựng nặng cùng lúc. `t.sh` thử lại tự động khi gặp `LNK1104/LNK1108/1455`.

## 5. Ổ C: đầy
Triệu chứng: `No space left on device`, `LNK1104`/`LNK1108`. Cách làm đã chốt: `TEMP`/`TMP`/`TMPDIR` trỏ về `D:\Grit\target\tmp` (xem `tools/gate.sh`, `t.sh`). Giải phóng chỗ khi cần: thư mục cài đặt .NET/VS cũ trong `%TEMP%`.
