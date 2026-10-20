### B02_clippy — clippy -D warnings (17 crate thuần)

- **Trạng thái:** FAIL (mã thoát 101, 13s, bắt đầu 2026-10-20T11:50:51Z)
- **rustc/clippy:** 4 dòng `error`, 1 dòng `warning`

**Lỗi (dòng `error` đầu tiên):**
```
error: unused import: `std::process::Command`
 --> crates/gt-lexer/tests/leak_check.rs:6:5
  |
6 | use std::process::Command;
error: doc list item without indentation
 --> crates/gt-types/tests/match_oracle.rs:5:5
  |
5 | //! Giá trị luôn xử lý như độ lệch `u128` so với giá trị nhỏ nhất của kiểu (lật bit dấu cho `i128`) nên không dùng số học tràn.
```

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
  = help: if this is supposed to be its own paragraph, add a blank line
  = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.97.0/index.html#doc_lazy_continuation
  = note: `-D clippy::doc-lazy-continuation` implied by `-D clippy::all`
  = help: to override `-D clippy::all` add `#[allow(clippy::doc_lazy_continuation)]`
help: indent this line
  |
5 | //!   Giá trị luôn xử lý như độ lệch `u128` so với giá trị nhỏ nhất của kiểu (lật bit dấu cho `i128`) nên không dùng số học tràn.
  |     ++

error: could not compile `gt-types` (test "match_oracle") due to 1 previous error
    Checking gt-dataflow v0.1.0 (/home/user/Grit/crates/gt-dataflow)
    Checking gt-oblig v0.1.0 (/home/user/Grit/crates/gt-oblig)
    Checking gt-effects v0.1.0 (/home/user/Grit/crates/gt-effects)
    Checking gt-borrowck v0.1.0 (/home/user/Grit/crates/gt-borrowck)
    Checking gt-conc v0.1.0 (/home/user/Grit/crates/gt-conc)
    Checking gt-secrets v0.1.0 (/home/user/Grit/crates/gt-secrets)
    Checking gt-term v0.1.0 (/home/user/Grit/crates/gt-term)
    Checking gt-span v0.1.0 (/home/user/Grit/crates/gt-span)
```
</details>
