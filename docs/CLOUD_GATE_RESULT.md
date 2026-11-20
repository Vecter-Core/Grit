# CLOUD_GATE_RESULT — kết quả cổng Linux

**Trạng thái (GÓI 10, 2026-11-20): cổng Linux ĐÃ CHẠY CỤC BỘ bằng WSL2 (KHÔNG phải CI, KHÔNG phải phiên đám mây). `docs/CLOUD_GATE_PROMPT.md` giữ làm phương án khi GitHub trở lại.**

```text
GATE_RESULT=GREEN            GATE_PLATFORM=linux
GATE_TREE=b828b9812b7a6bc39d2ed6c1b42edf90c52c5d784188e8f62602c04d4335ffa4   (KHỚP với cổng Windows rc6)
Commit: 16c37f0100dcc3345b1fdd5acb2b3af9fb66ed6d (thẻ seed-1.0-rc6), clone sạch /root/grit_lin trong WSL
TEST_PASSED=1042 FAILED=0 SUITES=213   (Windows: 1045/0/213; chênh 3 test là test điều kiện nền tảng; CHƯA đối chiếu từng test)
DOC_TEST: 220 khối grit, 0 sai · ADVERSARIAL: 3 chương trình, 0 sai · FAIL_OPEN_SCAN: 479 mục, 0 MỚI
NUL_OK (2099 tệp) · DELETIONS_OK (193) · TRACK_OK · RUN_REPORTS_INDEX_OK · corpus nhất quán với error_codes.md
Giai đoạn đạt: gtrt, fmt, clippy -D warnings, audit (cargo-audit 0.23.0), test (22 đơn vị), expect, nul, doc, deletions, failopen, track
```
Môi trường: WSL 2.7.14.0, Ubuntu 24.04.5 LTS, nhân 6.18.33.2-microsoft-standard-WSL2, 12 lõi, ≈ 7,7 GB RAM; rustc **1.98.1** (cùng Windows); LLVM = gói hệ thống `libllvm20` (20.1.2; Windows dùng 20.1.8); `CARGO_BUILD_JOBS=2`.

## Đường tới xanh (nói thẳng)
1. Lần đầu (rustc 1.100.0 mới nhất): clippy ĐỎ (`gt-mir`, lint mới của bản 1.100) và audit ĐỎ (chưa cài `cargo-audit`). Đây là lệch TOOLCHAIN, không phải sửa mã; đưa WSL về rustc 1.98.1 rồi chạy lại TỪ ĐẦU (`GATE_RESET=1`).
2. `gt-driver::std_lib::std_ord_int_file_matches_generator` ĐỎ: test gọi `python`, Ubuntu chỉ có `python3` ⇒ cài `python-is-python3`; sau đó 183/183.
3. `cargo build` ban đầu lỗi liên kết `-lffi`: cần `libffi-dev` (cùng `libzstd-dev zlib1g-dev libxml2-dev`).
**Lỗ hổng của `tools/setup_cloud.sh` (CHƯA sửa):** không cài `libffi-dev`, `python-is-python3`, `rustfmt`/`clippy`, `cargo-audit`, và không ghim rustc 1.98.1. Ghi vào việc kế.
