### C00_build — Dựng 17 crate thuần (mọi target, kể cả test)

- **Trạng thái:** PASS (mã thoát 0, 0s, bắt đầu 2026-10-20T11:19:15Z)
- **rustc/clippy:** 0 dòng `error`, 1 dòng `warning`

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
warning: unused import: `std::process::Command`
 --> crates/gt-lexer/tests/leak_check.rs:6:5
  |
6 | use std::process::Command;
  |     ^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: `gt-lexer` (test "leak_check") generated 1 warning (run `cargo fix --test "leak_check" -p gt-lexer` to apply 1 suggestion)
    Finished `dev` profile [unoptimized] target(s) in 0.05s
```
</details>
