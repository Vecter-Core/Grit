### G01_driver_tests — [bản sao] cargo test -p gt-driver --no-default-features

- **Trạng thái:** FAIL (mã thoát 101, 21s, bắt đầu 2026-10-20T11:32:51Z)
- **cargo test:** 38 đạt / 9 lỗi / 0 ignore trong 29 bộ test
- **rustc/clippy:** 4 dòng `error`, 0 dòng `warning`

**Test lỗi (9):**
- `directory_input_and_version`
- `guard_post_rejects_every_injected_fault`
- `guard_pre_still_runs_before_the_optimizer_in_release`
- `release_reports_two_ir_hashes_dev_none`
- `release_does_not_change_the_remaining_checks_report`
- `every_reachable_abort_kind_is_the_same_in_dev_and_release`
- `constant_folded_floats_are_ieee_exact_in_release`
- `nan_signed_zero_infinity_subnormal_behave_per_ieee_in_release`
- `float_traps_match_ieee_per_operation_in_release`

<details><summary>Chi tiết panic (tối đa 12)</summary>

`directory_input_and_version`
```
thread 'directory_input_and_version' (27668) panicked at crates/gt-driver/tests/cli.rs:452:5:
gritc 0.1.0 (backend mặc định: interpreter)
stack backtrace:
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: core::panicking::panic_display::<alloc::string::String>
```
`guard_post_rejects_every_injected_fault`
```
thread 'guard_post_rejects_every_injected_fault' (20413) panicked at crates/gt-driver/tests/release_guard.rs:56:9:
assertion `left == right` failed: post:drop-abort: gritc: bản gritc này dựng không có backend llvm (--no-default-features)
dùng: gritc <lệnh> …
  gritc check <tệp|thư mục> [--json] [--deny-yellow] [--checks=all|proven (mặc định: all; proven là THỬ NGHIỆM: xoá check đã chứng minh, in ghi chú N0606, không dùng với release)]
  gritc build <tệp|thư mục> [-o ra.exe|ra.gtmir] [--backend=llvm|interpreter (mặc định: llvm nếu bản dựng có)] [--profile=dev|release|release-o2 (mặc định: dev; release và release-o2 đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh: ADR-036 §9)] [--json] [--deny-yellow] [--checks=all|
  gritc run   <tệp|thư mục> [--backend=llvm|interpreter (mặc định: llvm nếu bản dựng có)] [--profile=dev|release|release-o2 (mặc định: dev; release và release-o2 đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh: ADR-036 §9)] [--json] [--deny-yellow] [--checks=all|proven (mặc định: all
```
`guard_pre_still_runs_before_the_optimizer_in_release`
```
thread 'guard_pre_still_runs_before_the_optimizer_in_release' (20414) panicked at crates/gt-driver/tests/release_guard.rs:89:9:
assertion `left == right` failed: ir:unreachable: gritc: bản gritc này dựng không có backend llvm (--no-default-features)
dùng: gritc <lệnh> …
  gritc check <tệp|thư mục> [--json] [--deny-yellow] [--checks=all|proven (mặc định: all; proven là THỬ NGHIỆM: xoá check đã chứng minh, in ghi chú N0606, không dùng với release)]
  gritc build <tệp|thư mục> [-o ra.exe|ra.gtmir] [--backend=llvm|interpreter (mặc định: llvm nếu bản dựng có)] [--profile=dev|release|release-o2 (mặc định: dev; release và release-o2 đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh: ADR-036 §9)] [--json] [--deny-yellow] [--checks=all|
  gritc run   <tệp|thư mục> [--backend=llvm|interpreter (mặc định: llvm nếu bản dựng có)] [--profile=dev|release|release-o2 (mặc định: dev; release và release-o2 đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh: ADR-036 §9)] [--json] [--deny-yellow] [--checks=all|proven (mặc định: all
```
`release_reports_two_ir_hashes_dev_none`
```
thread 'release_reports_two_ir_hashes_dev_none' (20842) panicked at crates/gt-driver/tests/release_guard.rs:124:28:
thiếu dòng băm: gritc: bản gritc này dựng không có backend llvm (--no-default-features)
dùng: gritc <lệnh> …
  gritc check <tệp|thư mục> [--json] [--deny-yellow] [--checks=all|proven (mặc định: all; proven là THỬ NGHIỆM: xoá check đã chứng minh, in ghi chú N0606, không dùng với release)]
  gritc build <tệp|thư mục> [-o ra.exe|ra.gtmir] [--backend=llvm|interpreter (mặc định: llvm nếu bản dựng có)] [--profile=dev|release|release-o2 (mặc định: dev; release và release-o2 đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh: ADR-036 §9)] [--json] [--deny-yellow] [--checks=all|
  gritc run   <tệp|thư mục> [--backend=llvm|interpreter (mặc định: llvm nếu bản dựng có)] [--profile=dev|release|release-o2 (mặc định: dev; release và release-o2 đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh: ADR-036 §9)] [--json] [--deny-yellow] [--checks=all|proven (mặc định: all
```
`release_does_not_change_the_remaining_checks_report`
```
thread 'release_does_not_change_the_remaining_checks_report' (20841) panicked at crates/gt-driver/tests/release_guard.rs:103:5:
assertion `left == right` failed
  left: Some(3)
 right: Some(0)
stack backtrace:
   0: __rustc::rust_begin_unwind
```
`every_reachable_abort_kind_is_the_same_in_dev_and_release`
```
thread 'every_reachable_abort_kind_is_the_same_in_dev_and_release' (12303) panicked at crates/gt-driver/tests/release_traps.rs:40:9:
JSON hỏng (EOF while parsing a value at line 1 column 0): 
stderr: gritc: bản gritc này dựng không có backend llvm (--no-default-features)
dùng: gritc <lệnh> …
  gritc check <tệp|thư mục> [--json] [--deny-yellow] [--checks=all|proven (mặc định: all; proven là THỬ NGHIỆM: xoá check đã chứng minh, in ghi chú N0606, không dùng với release)]
  gritc build <tệp|thư mục> [-o ra.exe|ra.gtmir] [--backend=llvm|interpreter (mặc định: llvm nếu bản dựng có)] [--profile=dev|release|release-o2 (mặc định: dev; release và release-o2 đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh: ADR-036 §9)] [--json] [--deny-yellow] [--checks=all|
```
`constant_folded_floats_are_ieee_exact_in_release`
```
thread 'constant_folded_floats_are_ieee_exact_in_release' (12302) panicked at crates/gt-driver/tests/release_traps.rs:40:9:
JSON hỏng (EOF while parsing a value at line 1 column 0): 
stderr: gritc: bản gritc này dựng không có backend llvm (--no-default-features)
dùng: gritc <lệnh> …
  gritc check <tệp|thư mục> [--json] [--deny-yellow] [--checks=all|proven (mặc định: all; proven là THỬ NGHIỆM: xoá check đã chứng minh, in ghi chú N0606, không dùng với release)]
  gritc build <tệp|thư mục> [-o ra.exe|ra.gtmir] [--backend=llvm|interpreter (mặc định: llvm nếu bản dựng có)] [--profile=dev|release|release-o2 (mặc định: dev; release và release-o2 đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh: ADR-036 §9)] [--json] [--deny-yellow] [--checks=all|
```
`nan_signed_zero_infinity_subnormal_behave_per_ieee_in_release`
```
thread 'nan_signed_zero_infinity_subnormal_behave_per_ieee_in_release' (12497) panicked at crates/gt-driver/tests/release_traps.rs:40:9:
JSON hỏng (EOF while parsing a value at line 1 column 0): 
stderr: gritc: bản gritc này dựng không có backend llvm (--no-default-features)
dùng: gritc <lệnh> …
  gritc check <tệp|thư mục> [--json] [--deny-yellow] [--checks=all|proven (mặc định: all; proven là THỬ NGHIỆM: xoá check đã chứng minh, in ghi chú N0606, không dùng với release)]
  gritc build <tệp|thư mục> [-o ra.exe|ra.gtmir] [--backend=llvm|interpreter (mặc định: llvm nếu bản dựng có)] [--profile=dev|release|release-o2 (mặc định: dev; release và release-o2 đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh: ADR-036 §9)] [--json] [--deny-yellow] [--checks=all|
```
`float_traps_match_ieee_per_operation_in_release`
```
thread 'float_traps_match_ieee_per_operation_in_release' (12496) panicked at crates/gt-driver/tests/release_traps.rs:40:9:
JSON hỏng (EOF while parsing a value at line 1 column 0): 
stderr: gritc: bản gritc này dựng không có backend llvm (--no-default-features)
dùng: gritc <lệnh> …
  gritc check <tệp|thư mục> [--json] [--deny-yellow] [--checks=all|proven (mặc định: all; proven là THỬ NGHIỆM: xoá check đã chứng minh, in ghi chú N0606, không dùng với release)]
  gritc build <tệp|thư mục> [-o ra.exe|ra.gtmir] [--backend=llvm|interpreter (mặc định: llvm nếu bản dựng có)] [--profile=dev|release|release-o2 (mặc định: dev; release và release-o2 đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh: ADR-036 §9)] [--json] [--deny-yellow] [--checks=all|
```
</details>

**Lỗi (dòng `error` đầu tiên):**
```
error: test failed, to rerun pass `-p gt-driver --test cli`
     Running tests/fs_read_io_error.rs (target/debug/deps/fs_read_io_error-c5a4bc2c96307825)

running 0 tests
error: test failed, to rerun pass `-p gt-driver --test release_guard`
     Running tests/release_policy.rs (target/debug/deps/release_policy-5780f25041e8e3bb)

running 6 tests
error: test failed, to rerun pass `-p gt-driver --test release_traps`
     Running tests/std_lib.rs (target/debug/deps/std_lib-5ccb3edfc5905398)

running 0 tests
error: 3 targets failed:
    `-p gt-driver --test cli`
    `-p gt-driver --test release_guard`
    `-p gt-driver --test release_traps`
```

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/vec_overflow.rs (target/debug/deps/vec_overflow-fc47c77d6746284f)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests gt_driver

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 3 targets failed:
    `-p gt-driver --test cli`
    `-p gt-driver --test release_guard`
    `-p gt-driver --test release_traps`
```
</details>
