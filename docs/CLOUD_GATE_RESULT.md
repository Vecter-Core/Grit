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

## Cổng Linux rc7 (GÓI 12 P1, 2026-11-20; WSL2 cục bộ, KHÔNG phải CI)
- **GATE_RESULT=GREEN** trên commit `4ef044e893fb6a365a914dffc11426246788e90c` (= commit cổng Windows rc7). **`GATE_TREE=c0d7b2679410dbe6b2e0ba29045f6139d4166bd410defed075d9cb362c489380` KHỚP cổng Windows rc7** (cùng cây).
- Số liệu: `TEST_PASSED=1051 FAILED=0 SUITES=215` (Windows rc7: 1054/0/215). DOC_TEST 220/0, ADVERSARIAL 3/0, FAIL_OPEN 479 mục/0 mới, NUL_OK 2127, DELETIONS_OK 193, TRACK_OK, clippy `-D warnings` và `cargo audit` đạt, fmt OK. gt-term 51, gt-driver 183 (Windows 186), rig 48.
- Môi trường: Ubuntu 24.04.5 LTS, nhân 6.18.33.2-microsoft-standard-WSL2, rustc 1.98.1 (ghim; không còn lỗi clippy 1.100), clippy 0.1.98, `cargo-audit` 0.23.0; `setup_cloud.sh --check` ⇒ `SETUP_CHECK=OK`. RAM WSL ≈7,7 GB. Clone `~/grit_g12` (trong hệ tệp Linux, không chạy trong `/mnt/d`), `core.autocrlf=false`.
- Thời gian: ≈ 64 phút từ lúc chạy (14:25 → 15:29 giờ WSL), 4 lô (lô đầu + 3 lô tiếp, ngân sách 840 s mỗi lệnh, dừng ở ranh giới đơn vị); đơn vị `gt-driver` chạy MỘT cục (cổng rc7 chưa có bản chia; thời gian riêng của nó trên Linux: không ghi được từ nhật ký, CHƯA ĐO).
- **Chênh lệch số test (đã giải thích, tên cụ thể):** 1054 − 1051 = 3 = đúng 3 test `#[cfg(windows)]` của `gt-driver` (183 so với 186): `fs_read_io_error::a_locked_file_aborts_io_error`, `llvm::built_exe_does_not_depend_on_llvm_dll`, `llvm::check_runs_without_llvm_on_path` (`tools/platform_tests.py --list`, `docs/PLATFORM_TESTS.md`).
- **Phạm vi:** cổng này chứng minh rc7 `4ef044e` XANH trên Linux. Nó KHÔNG chứng minh `Vecter-Core/main` (`e8a8ae5`): cây đó khác đúng một dòng `license` ở `runtime/gt-rt/Cargo.toml` (xem `REMOTE_STATE.md` mục 6) và chưa chạy cổng. Không phải rc8 (rc8: cổng Windows riêng, xem READINESS).

## Cổng Linux rc8 (GÓI 13 C, 2026-11-21/22; WSL2 cục bộ, KHÔNG phải CI)
- **GATE_RESULT=GREEN** trên `78d0c40` (clone `~/grit_g13b`, trong hệ tệp Linux, `core.autocrlf=false`; qua `tools/gate_clean.sh`, 840 s/lô). **`GATE_TREE=c7bdef2613d5c442209769ce47d8fc3d9ca72aa2a1212c4c102d022552e8eec4`**. `TEST_PASSED=1074 FAILED=0 SUITES=220`. DOC_TEST 220/0, ADVERSARIAL 3/0, FAIL_OPEN 480 mục/0 mới, NUL_OK 2147, DELETIONS_OK 194, TRACK_OK, clippy `-D warnings` và audit đạt.
- Môi trường: Ubuntu 24.04.5, nhân 6.18.33.2-microsoft-standard-WSL2, rustc 1.98.1, clippy 0.1.98, cargo-audit 0.23.0.
- Thời gian gt-driver (đơn vị chia theo mục tiêu): tổng 1374 s (≈ 22,9 phút, gồm biên dịch từng bộ); dài nhất `hashmap_collide` 362 s, `std2_lib` 247, `std_lib` 239, `shared_arena_diff` 104 (Windows rc8: 581/322/306/213; Linux nhanh hơn ≈ 1,6–2×). **Đơn vị `gt-fmt` chạy 1024 s (17 phút) trên Linux — VƯỢT 15 phút** (gói `gt-fmt` chưa nằm trong `GATE_SPLIT_PKGS`); đề xuất thêm `gt-fmt` vào danh sách chia (công cụ, chưa làm). gt-term 424 s (69 test, 19 bộ).
- **Chênh lệch số test so Windows:** Windows cùng commit: xem READINESS GÓI 13. Chênh dự kiến = 3 test `cfg(windows)` của gt-driver (`fs_read_io_error::a_locked_file_aborts_io_error`, `llvm::built_exe_does_not_depend_on_llvm_dll`, `llvm::check_runs_without_llvm_on_path`; `tools/platform_tests.py`).
- Lần chạy đầu trên `a327924` ĐỎ (gt-resolve, 2 test): tôi thêm `tests/fail/E0203_private_field_in_pattern.gt`, nhưng tiền tố E0203 thuộc `gt-resolve` — lặp lại đúng lỗi GÓI 7 đã ghi ở `DELETIONS.md`. Đã gỡ (`78d0c40`, ghi DELETIONS).
