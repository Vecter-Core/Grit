### C_gt-lexer — cargo test -p gt-lexer

- **Trạng thái:** PASS (mã thoát 0, 9s, bắt đầu 2026-10-20T11:19:16Z)
- **cargo test:** 25 đạt / 0 lỗi / 0 ignore trong 8 bộ test
- **rustc/clippy:** 0 dòng `error`, 1 dòng `warning`

<details><summary>Đuôi log (18 dòng cuối)</summary>

```

     Running tests/trivia.rs (target/debug/deps/trivia-313be78702edac98)

running 5 tests
test crlf_comment_span_excludes_cr ... ok
test edge_cases ... ok
test line_comments_are_kept_with_exact_text_and_placement ... ok
test trivia_are_in_source_order_and_do_not_overlap_tokens ... ok
test corpus_comment_count_is_preserved ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

   Doc-tests gt_lexer

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>
