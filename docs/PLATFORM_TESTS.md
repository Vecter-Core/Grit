# PLATFORM_TESTS — test chỉ-một-nền và test bị `#[ignore]` (GÓI 5 S2.a, 2026-11-19)

`tools/platform_tests.py` quét tĩnh mọi hàm `#[test]` có thuộc tính `cfg(windows|unix|target_os|not(...))` hoặc `#[ignore]` và so với bảng dưới (dòng bảng bắt đầu bằng ``| `đường_dẫn::hàm` |``); lệch ⇒ ĐỎ (`tools/test_platform_tests.py`, trong giai đoạn `expect` của cổng). `#[ignore]` PHẢI có lý do.

## Số liệu thật
* Windows `cargo test --workspace`: 1027 test đạt (HANDOFF/RUN_REPORTS); Linux CI: 1024. Chênh 3 = ba test `cfg(windows)` của workspace dưới đây (`llvm.rs` ×2, `fs_read_io_error.rs` ×1). Danh sách `--list` thật mỗi nền và diff: xem mục "Diff `--list`" cuối tệp.
* Test `#[ignore]` không chạy ở cổng; mỗi cái có lý do và cách chạy tay.

## Chỉ một nền (`cfg`)
| test | nền | lý do |
|---|---|---|
| `crates/gt-driver/tests/fs_read_io_error.rs::a_locked_file_aborts_io_error` | chỉ Windows | khoá độc quyền tệp bằng `share_mode(0)` (API Windows); Linux không có khoá bắt buộc tương đương |
| `crates/gt-driver/tests/llvm.rs::built_exe_does_not_depend_on_llvm_dll` | chỉ Windows | dùng `dumpbin.exe` của MSVC để kiểm exe không phụ thuộc `LLVM-C.dll` |
| `crates/gt-driver/tests/llvm.rs::check_runs_without_llvm_on_path` | chỉ Windows | khái niệm `LLVM-C.dll` trên PATH chỉ có ở Windows (Linux liên kết `libLLVM-20.so` lúc dựng) |
| `runtime/gt-rt/src/lib.rs::lone_surrogate_argument_is_lossily_replaced_with_valid_utf8` | chỉ Windows | đối số UTF-16 có surrogate lẻ (`OsStringExt` của Windows); gt-rt là workspace riêng, KHÔNG nằm trong 1027/1024 |
| `runtime/gt-rt/src/lib.rs::invalid_unix_argument_bytes_are_lossily_replaced_with_valid_utf8` | chỉ unix | đối số byte không hợp lệ UTF-8 (`OsStringExt` của unix); gt-rt là workspace riêng |

(Các `cfg` TRONG thân một test — `argv_e2e.rs`, `leak_check.rs`, `gt-rt` kiểm `NUL` — không tạo test riêng nền nào; cùng một tên test chạy ở cả hai nền với nhánh mã khác nhau.)

## Bị `#[ignore]` (lý do ghi ngay trong thuộc tính)
| test | lý do / cách chạy |
|---|---|
| `crates/gt-driver/tests/llvm_diff.rs::batch` | chạy theo lô bằng `GT_DIFF_START/COUNT/OUT` (oracle vi phân, tool `tools/oracle.py`) |
| `crates/gt-driver/tests/llvm_diff_abort.rs::batch` | theo lô bằng `GT_ABORT_KIND/START/COUNT/OUT` |
| `crates/gt-driver/tests/llvm_diff_bits.rs::batch` | theo lô bằng `GT_BITS_START/COUNT/OUT` |
| `crates/gt-driver/tests/llvm_diff_gtv.rs::batch` | theo lô bằng `GT_GTV_START/COUNT/OUT` |
| `crates/gt-driver/tests/llvm_diff_refpat.rs::batch` | theo lô bằng `GT_DIFF_START/COUNT/OUT` |
| `crates/rig/tests/dogfood_sortlines.rs::perf_report_against_rust_sort_by` | đo hiệu năng: `--ignored --nocapture` |
| `crates/rig/tests/dogfood_stats.rs::perf_report_against_rust` | đo hiệu năng: `--ignored --nocapture` |
| `crates/rig/tests/dogfood_wordfreq.rs::perf_report_against_rust` | đo hiệu năng: `--ignored --nocapture` |

Các máy in gỡ lỗi `dump_*`/`probe_*` (không khẳng định gì) đã xoá ở GÓI 12; năm `batch` là oracle chạy tay theo lô (xem `docs/HISTORY.md`).

## Cập nhật GÓI 6
Danh sách `--list` Windows nay 1059 mục (thêm 3 test `gt-term/tests/limit_hint.rs` và 9 test của GÓI 7 (`ghost_limit_soundness.rs`, `ghost_wide_oracle.rs`, `struct_pat_privacy.rs`, `spawn_recursion.rs`)); Linux = Windows trừ 3 test chỉ-Windows (CI bước `PLATFORM_DIFF_OK`). Bước diff bắt được ngay danh sách cũ khi thêm test (CI run `44365492150` đỏ ở bước này dù cổng GREEN): phải thêm tên test mới vào `docs/platform_tests_windows.list`.
