### E01_gtrt — runtime/gt-rt (runtime native, chỉ Windows x86_64)

- **Trạng thái:** BLOCKED (mã thoát 77, 0s, bắt đầu 2026-10-20T11:50:04Z)
- gt-rt dừng bằng compile_error! trên Linux: runtime native chỉ có cho x86_64 Windows (ADR-029)
- **rustc/clippy:** 2 dòng `error`, 0 dòng `warning`

**Lỗi (dòng `error` đầu tiên):**
```
error: gt-rt Phase 1 chỉ hỗ trợ x86_64 Windows (ADR-029); target khác phải được thêm có chủ đích
  --> src/lib.rs:11:1
   |
11 | / compile_error!(
```

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
== fmt gt-rt
FMT_OK
== dựng gt-rt (release)
   Compiling gt-rt v0.0.0 (/home/user/Grit/runtime/gt-rt)
error: gt-rt Phase 1 chỉ hỗ trợ x86_64 Windows (ADR-029); target khác phải được thêm có chủ đích
  --> src/lib.rs:11:1
   |
11 | / compile_error!(
12 | |     "gt-rt Phase 1 chỉ hỗ trợ x86_64 Windows (ADR-029); target khác phải được thêm có chủ đích"
13 | | );
   | |_^

error: could not compile `gt-rt` (lib) due to 1 previous error
SUMMARY: gt-rt dừng bằng compile_error! trên Linux: runtime native chỉ có cho x86_64 Windows (ADR-029)
```
</details>
