### D01_native_probe — Dựng nguyên trạng gt-driver/rig/gt-llvm (cần LLVM 20 + Windows)

- **Trạng thái:** BLOCKED (mã thoát 77, 4s, bắt đầu 2026-10-20T11:50:00Z)
- 4/4 lệnh dựng nguyên trạng thất bại
- **rustc/clippy:** 8 dòng `error`, 2 dòng `warning`

**Lỗi (dòng `error` đầu tiên):**
```
error: failed to run custom build command for `gt-llvm v0.1.0 (/home/user/Grit/crates/gt-llvm)`
note: To improve backtraces for build dependencies, set the CARGO_PROFILE_DEV_BUILD_OVERRIDE_DEBUG=true environment variable to enable debug information generation.

Caused by:
error[E0433]: cannot find module or crate `gt_codegen_llvm` in this scope
  --> crates/gt-driver/src/cli.rs:73:53
   |
73 |             "--profile=release-o2" => cli.profile = gt_codegen_llvm::Opt::ReleaseO2,
error[E0433]: cannot find module or crate `gt_codegen_llvm` in this scope
  --> crates/gt-driver/src/cli.rs:25:14
   |
25 |     profile: gt_codegen_llvm::Opt,
error[E0433]: cannot find module or crate `gt_codegen_llvm` in this scope
  --> crates/gt-driver/src/cli.rs:73:53
   |
73 |             "--profile=release-o2" => cli.profile = gt_codegen_llvm::Opt::ReleaseO2,
error[E0433]: cannot find module or crate `gt_codegen_llvm` in this scope
  --> crates/gt-driver/src/cli.rs:25:14
   |
25 |     profile: gt_codegen_llvm::Opt,
error: failed to run custom build command for `gt-llvm v0.1.0 (/home/user/Grit/crates/gt-llvm)`
note: To improve backtraces for build dependencies, set the CARGO_PROFILE_DEV_BUILD_OVERRIDE_DEBUG=true environment variable to enable debug information generation.

Caused by:
```

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
Caused by:
  process didn't exit successfully: `/home/user/Grit/target/debug/build/gt-llvm-da143656b7cdde03/build-script-build` (exit status: 101)
  --- stdout
  cargo:rerun-if-env-changed=GRIT_LLVM_DIR

  --- stderr

  thread 'main' (27166) panicked at crates/gt-llvm/build.rs:11:5:
  không thấy D:/Tools/llvm-20.1.8/lib/LLVM-C.lib — GRIT_LLVM_DIR sai?
  stack backtrace:
     0: __rustc::rust_begin_unwind
     1: core::panicking::panic_fmt
     2: build_script_build::main
     3: <fn() as core::ops::function::FnOnce<()>>::call_once
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
warning: build failed, waiting for other jobs to finish...
>>> THẤT BẠI: -p gt-llvm 
SUMMARY: 4/4 lệnh dựng nguyên trạng thất bại
```
</details>
