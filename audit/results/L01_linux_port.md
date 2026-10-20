### L01_linux_port — [bản sao 2] port Linux (vá 5 tệp): gt-rt + gritc có backend LLVM 20 + liên kết bằng cc

- **Trạng thái:** PASS (mã thoát 0, 26s, bắt đầu 2026-10-20T11:44:03Z)
- port Linux: vá 5 tệp (gt-llvm/build.rs, gt-driver/build.rs+native.rs, rig/build.rs, gt-rt/src/lib.rs, +62/−26 dòng); LLVM 18.1 hệ thống thay vì 20.1.8
- **cargo test:** 6 đạt / 0 lỗi / 0 ignore trong 2 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
patching file crates/gt-driver/build.rs
patching file crates/gt-driver/src/native.rs
patching file crates/gt-llvm/build.rs
patching file crates/rig/build.rs
patching file runtime/gt-rt/src/lib.rs
== gt-rt (staticlib, Linux)
   Compiling gt-rt v0.0.0 (/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit_linux/runtime/gt-rt)
    Finished `release` profile [optimized] target(s) in 0.54s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
== gritc có backend LLVM
   Compiling itoa v1.0.18
   Compiling memchr v2.8.3
    Finished `dev` profile [unoptimized] target(s) in 24.12s
gritc: phán quyết green; 0 lỗi, 0 cảnh báo, 1 ghi chú; backend: llvm
chạy xong (backend: llvm); main trả về 45
/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/n1.exe: ELF 64-bi
SUMMARY: port Linux: vá 5 tệp (gt-llvm/build.rs, gt-driver/build.rs+native.rs, rig/build.rs, gt-rt/src/lib.rs, +62/−26 dòng); LLVM 18.1 hệ thống thay vì 20.1.8
```
</details>
