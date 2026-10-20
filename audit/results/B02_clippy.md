### B02_clippy — clippy -D warnings (17 crate thuần)

- **Trạng thái:** FAIL (mã thoát 101, 2s, bắt đầu 2026-10-20T11:17:04Z)
- **rustc/clippy:** 2 dòng `error`, 1 dòng `warning`

**Lỗi (dòng `error` đầu tiên):**
```
error: unused import: `std::process::Command`
 --> crates/gt-lexer/tests/leak_check.rs:6:5
  |
6 | use std::process::Command;
```

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
    Checking unicode-normalization v0.1.25
    Checking zmij v1.0.23
    Checking memchr v2.8.3
    Checking itoa v1.0.18
    Checking gt-lexer v0.1.0 (/home/user/Grit/crates/gt-lexer)
    Checking gt-diag v0.1.0 (/home/user/Grit/crates/gt-diag)
    Checking gt-ast v0.1.0 (/home/user/Grit/crates/gt-ast)
error: unused import: `std::process::Command`
 --> crates/gt-lexer/tests/leak_check.rs:6:5
  |
6 | use std::process::Command;
  |     ^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `-D unused-imports` implied by `-D warnings`
  = help: to override `-D warnings` add `#[allow(unused_imports)]`

error: could not compile `gt-lexer` (test "leak_check") due to 1 previous error
warning: build failed, waiting for other jobs to finish...
```
</details>
