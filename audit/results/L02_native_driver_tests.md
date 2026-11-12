### L02_native_driver_tests — [bản sao 2] cargo test -p gt-driver (backend LLVM thật: llvm*, release*, threads, vec, utf8…)

- **Trạng thái:** FAIL (mã thoát 101, 237s, bắt đầu 2026-10-20T11:44:37Z)
- **cargo test:** 135 đạt / 5 lỗi / 5 ignore trong 29 bộ test
- **rustc/clippy:** 2 dòng `error`, 0 dòng `warning`

**Test lỗi (5):**
- `built_exe_does_not_depend_on_llvm_dll`
- `check_runs_without_llvm_on_path`
- `golden_ir_checked_add_and_abort_blocks`
- `golden_ir_slice_bounds`
- `golden_ir_static_atomic_and_mutex`

<details><summary>Chi tiết panic (tối đa 12)</summary>

`built_exe_does_not_depend_on_llvm_dll`
```
thread 'built_exe_does_not_depend_on_llvm_dll' (28991) panicked at crates/gt-driver/tests/llvm.rs:325:77:
dumpbin
stack backtrace:
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: core::option::expect_failed
```
`check_runs_without_llvm_on_path`
```
thread 'check_runs_without_llvm_on_path' (31572) panicked at crates/gt-driver/tests/llvm.rs:394:5:
assertion `left == right` failed: gritc: phán quyết green; 0 lỗi, 0 cảnh báo, 0 ghi chú; backend: llvm
gritc: đã ghi /tmp/gt-llvm-test-28986-15-nollvm/p.exe (backend: llvm)
  left: Some(0)
 right: Some(3)
stack backtrace:
```
`golden_ir_checked_add_and_abort_blocks`
```
thread 'golden_ir_checked_add_and_abort_blocks' (1569) panicked at crates/gt-driver/tests/llvm.rs:708:9:
assertion `left == right` failed: IR của `checked_add` đổi so với golden /tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit_linux/crates/gt-driver/../../tests/golden_ll/checked_add.ll
  left: "; ModuleID = 'grit'\nsource_filename = \"grit\"\ntarget datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128\"\ntarget triple = \"x86_64-pc-linux-gnu\"\n\n@gt.attestation = constant [65 x i8] c\"2e255f837911913202f74bc2c9cca3081c612effeaf5d761506356
 right: "; ModuleID = 'grit'\nsource_filename = \"grit\"\ntarget datalayout = \"e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128\"\ntarget triple = \"x86_64-pc-windows-msvc\"\n\n@gt.attestation = constant [65 x i8] c\"2e255f837911913202f74bc2c9cca3081c612effeaf5d761506
stack backtrace:
   0: __rustc::rust_begin_unwind
```
`golden_ir_slice_bounds`
```
thread 'golden_ir_slice_bounds' (1997) panicked at crates/gt-driver/tests/llvm.rs:708:9:
assertion `left == right` failed: IR của `slice_sum` đổi so với golden /tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit_linux/crates/gt-driver/../../tests/golden_ll/slice_sum.ll
  left: "; ModuleID = 'grit'\nsource_filename = \"grit\"\ntarget datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128\"\ntarget triple = \"x86_64-pc-linux-gnu\"\n\n@gt.attestation = constant [65 x i8] c\"7f0a8dbb6c00f094ade96593421462406510be455cec8ebb38cf8a
 right: "; ModuleID = 'grit'\nsource_filename = \"grit\"\ntarget datalayout = \"e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128\"\ntarget triple = \"x86_64-pc-windows-msvc\"\n\n@gt.attestation = constant [65 x i8] c\"7f0a8dbb6c00f094ade96593421462406510be455cec8ebb38c
stack backtrace:
   0: __rustc::rust_begin_unwind
```
`golden_ir_static_atomic_and_mutex`
```
thread 'golden_ir_static_atomic_and_mutex' (2444) panicked at crates/gt-driver/tests/llvm.rs:708:9:
assertion `left == right` failed: IR của `static_atomic` đổi so với golden /tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit_linux/crates/gt-driver/../../tests/golden_ll/static_atomic.ll
  left: "; ModuleID = 'grit'\nsource_filename = \"grit\"\ntarget datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128\"\ntarget triple = \"x86_64-pc-linux-gnu\"\n\n@\"static.prog::COUNT\" = global { i32 } zeroinitializer\n@gt.attestation = constant [65 x i8]
 right: "; ModuleID = 'grit'\nsource_filename = \"grit\"\ntarget datalayout = \"e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128\"\ntarget triple = \"x86_64-pc-windows-msvc\"\n\n@\"static.prog::COUNT\" = global { i32 } zeroinitializer\n@gt.attestation = constant [65 x 
stack backtrace:
   0: __rustc::rust_begin_unwind
```
</details>

**Lỗi (dòng `error` đầu tiên):**
```
error: test failed, to rerun pass `-p gt-driver --test llvm`
     Running tests/llvm_diff.rs (target/debug/deps/llvm_diff-59cca6caaea4235a)

running 2 tests
error: 1 target failed:
    `-p gt-driver --test llvm`
```

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.52s

     Running tests/vec_overflow.rs (target/debug/deps/vec_overflow-4b25924a0547a34e)

running 2 tests
test small_reserve_and_push_do_not_false_alarm ... ok
test forced_multiplication_overflow_aborts_overflow_and_the_boundary_is_exact ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.30s

   Doc-tests gt_driver

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 1 target failed:
    `-p gt-driver --test llvm`
```
</details>
