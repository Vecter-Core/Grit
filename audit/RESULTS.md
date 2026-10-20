# KẾT QUẢ CHẠY TOÀN BỘ DỰ ÁN GRIT (tự sinh — đừng sửa tay; ghi chú tay đặt ở `audit/results/<ID>.note.md`)

- Mã nguồn được chạy: `86c4955 2026-10-17 Initial commit: Push full Grit project` (nhánh làm việc hiện tại; bộ chạy KHÔNG sửa tệp nào của dự án).
- Môi trường: Linux container (xem giai đoạn A00). Dự án được thiết kế cho **Windows x86_64 MSVC + LLVM 20.1.8** (docs/ENVIRONMENT.md).
- Cách chạy/tiếp tục: `bash audit/run_all.sh` (tự bỏ qua giai đoạn đã xong; xem `audit/PLAN.md`).

## Tổng hợp

| Trạng thái | Số giai đoạn |
|---|---|
| PASS | 12 |
| FAIL | 2 |
| PENDING | 26 |

PASS = chạy xong và đạt · FAIL = chạy được nhưng lỗi/thiếu · BLOCKED = KHÔNG chạy được trong môi trường này (có bằng chứng) · TIMEOUT = quá hạn · SKIP = bỏ qua có chủ ý · PENDING = chưa tới lượt.

## Bảng giai đoạn

| ID | Tên | Trạng thái | Thoát | Giây |
|---|---|---|---|---|
| A00_env | Môi trường: công cụ, phiên bản, tài nguyên | **PASS** | 0 | 0 |
| A01_fetch | Tải phụ thuộc theo Cargo.lock (--locked) | **PASS** | 0 | 0 |
| A02_inventory | Kiểm kê dự án: crate, tệp, LOC, tệp nhị phân bị theo dõi, CI/deny | **PASS** | 0 | 1 |
| B01_fmt | cargo fmt --all --check | **PASS** | 0 | 1 |
| B02_clippy | clippy -D warnings (17 crate thuần) | **FAIL** | 101 | 2 |
| B03_audit | cargo audit (lỗ hổng phụ thuộc) | **PASS** | 0 | 129 |
| B04_ci_deny | CI (ci.yml) và deny.toml có tồn tại không | **FAIL** | 1 | 0 |
| B05_codes | Mọi mã chẩn đoán đã biết có tài liệu + ≥3 mẫu fail + ≥3 mẫu pass (GRIT_PLAN §0.5) | PENDING | | |
| C00_build | Dựng 17 crate thuần (mọi target, kể cả test) | **PASS** | 0 | 0 |
| C_gt-span | cargo test -p gt-span | **PASS** | 0 | 1 |
| C_gt-lexer | cargo test -p gt-lexer | **PASS** | 0 | 9 |
| C_gt-diag | cargo test -p gt-diag | **PASS** | 0 | 0 |
| C_gt-ast | cargo test -p gt-ast | **PASS** | 0 | 0 |
| C_gt-parser | cargo test -p gt-parser | **PASS** | 0 | 12 |
| C_gt-stack | cargo test -p gt-stack | **PASS** | 0 | 0 |
| C_gt-fmt | cargo test -p gt-fmt | PENDING | | |
| C_gt-resolve | cargo test -p gt-resolve | PENDING | | |
| C_gt-types | cargo test -p gt-types | PENDING | | |
| C_gt-mir | cargo test -p gt-mir | PENDING | | |
| C_gt-dataflow | cargo test -p gt-dataflow | PENDING | | |
| C_gt-borrowck | cargo test -p gt-borrowck | PENDING | | |
| C_gt-oblig | cargo test -p gt-oblig | PENDING | | |
| C_gt-effects | cargo test -p gt-effects | PENDING | | |
| C_gt-conc | cargo test -p gt-conc | PENDING | | |
| C_gt-term | cargo test -p gt-term | PENDING | | |
| C_gt-secrets | cargo test -p gt-secrets | PENDING | | |
| D01_native_probe | Dựng nguyên trạng gt-driver/rig/gt-llvm (cần LLVM 20 + Windows) | PENDING | | |
| E01_gtrt | runtime/gt-rt (runtime native, chỉ Windows x86_64) | PENDING | | |
| F01_expect | tools/check_expectations.py (nhất quán corpus + mã lỗi) | PENDING | | |
| F02_tool_tests | unittest của tools/*.py | PENDING | | |
| G00_copy_patch | Bản sao NGOÀI dự án + vá tối thiểu để gritc dựng không cần LLVM | PENDING | | |
| G01_driver_tests | [bản sao] cargo test -p gt-driver --no-default-features | PENDING | | |
| G02_rig_tests | [bản sao] cargo test -p rig --no-default-features | PENDING | | |
| G03_corpus_cli | [bản sao] gritc check trên toàn bộ tests/{pass,fail,warn} | PENDING | | |
| G04_run_programs | [bản sao] gritc run: mẫu, bench, 6 công cụ examples/ | PENDING | | |
| G05_fmt_idempotent | [bản sao] gritc fmt: tính lũy đẳng trên corpus pass | PENDING | | |
| G06_determinism | [bản sao] gritc build hai lần ⇒ cùng băm (D9) | PENDING | | |
| G07_explain | [bản sao] gritc explain cho mọi mã có tài liệu | PENDING | | |
| H01_llvm_probe | Thử lấy LLVM 20 cho Linux (khả năng port) | PENDING | | |
| I01_fuzz_probe | Công cụ fuzz (nightly + cargo-fuzz) có sẵn không | PENDING | | |

## Chi tiết từng giai đoạn

### A00_env — Môi trường: công cụ, phiên bản, tài nguyên

- **Trạng thái:** PASS (mã thoát 0, 0s, bắt đầu 2026-10-20T11:17:02Z)
- Linux x86_64, 1.97.0, LLVM hệ thống 18.1.3; dự án ghim LLVM 20.1.8 + Windows MSVC

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
18.1.3
== tài nguyên
cpu=4
               total        used        free      shared  buff/cache   available
Mem:           16093         523       14598          14        1207       15569
/dev/vda        252G  9.5G   30G  25% /
== git
86c49553dcf08bb8ef301da57771bce5815ee521 2026-10-17 15:54:16 +0700 Initial commit: Push full Grit project
số commit nhìn thấy: 50 (kho nông)
== công cụ tuỳ chọn
THIẾU: cargo-audit
THIẾU: cargo-deny
THIẾU: cargo-fuzz
THIẾU: cargo-mutants
THIẾU: wine
có: clang
có: lld
SUMMARY: Linux x86_64, 1.97.0, LLVM hệ thống 18.1.3; dự án ghim LLVM 20.1.8 + Windows MSVC
```
</details>


### A01_fetch — Tải phụ thuộc theo Cargo.lock (--locked)

- **Trạng thái:** PASS (mã thoát 0, 0s, bắt đầu 2026-10-20T11:17:02Z)

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
```
</details>


### A02_inventory — Kiểm kê dự án: crate, tệp, LOC, tệp nhị phân bị theo dõi, CI/deny

- **Trạng thái:** PASS (mã thoát 0, 1s, bắt đầu 2026-10-20T11:17:02Z)
- workspace 22 crate; 1477 tệp theo dõi

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
== CI / deny / tệp giữ chỗ TASK.md
TASK.md
ls: cannot access 'deny.toml': No such file or directory
.github/workflows/TASK.md
crates/TASK.md
deny.toml.TASK.md
docs/errors/TASK.md
lib/core/TASK.md
tests/fuzz/TASK.md
tests/golden/TASK.md
== corpus .gt
tests/pass: 409 tệp .gt
tests/fail: 451 tệp .gt
tests/warn: 13 tệp .gt
tests/lexer: 8 tệp .gt
== tài liệu lỗi
docs/errors/*.md: 67
SUMMARY: workspace 22 crate; 1477 tệp theo dõi
```
</details>


### B01_fmt — cargo fmt --all --check

- **Trạng thái:** PASS (mã thoát 0, 1s, bắt đầu 2026-10-20T11:17:03Z)

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
```
</details>


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


### B03_audit — cargo audit (lỗ hổng phụ thuộc)

- **Trạng thái:** PASS (mã thoát 0, 129s, bắt đầu 2026-10-20T11:17:06Z)

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling rustls-platform-verifier v0.7.0
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.13.3
   Compiling gix-transport v0.57.1
   Compiling tame-index v0.26.3
   Compiling gix-protocol v0.62.0
   Compiling gix v0.84.0
   Compiling rustsec v0.33.0
   Compiling cargo-audit v0.22.2
    Finished `release` profile [optimized] target(s) in 2m 04s
  Installing /root/.cargo/bin/cargo-audit
   Installed package `cargo-audit v0.22.2` (executable `cargo-audit`)
    Fetching advisory database from `https://github.com/RustSec/advisory-db.git`
      Loaded 1345 security advisories (from /root/.cargo/advisory-db)
    Updating crates.io index
    Scanning Cargo.lock for vulnerabilities (50 crate dependencies)
```
</details>


### B04_ci_deny — CI (ci.yml) và deny.toml có tồn tại không

- **Trạng thái:** FAIL (mã thoát 1, 0s, bắt đầu 2026-10-20T11:19:15Z)
- dự án CHƯA có CI và CHƯA có deny.toml dù GRIT_PLAN §0.7 yêu cầu

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
THIẾU .github/workflows/ci.yml (chỉ có TASK.md giữ chỗ)
THIẾU deny.toml (chỉ có deny.toml.TASK.md)
-- GRIT_PLAN §0.7 đòi: clippy -D warnings, cargo deny, cargo audit chạy trong CI
SUMMARY: dự án CHƯA có CI và CHƯA có deny.toml dù GRIT_PLAN §0.7 yêu cầu
```
</details>


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


### C_gt-span — cargo test -p gt-span

- **Trạng thái:** PASS (mã thoát 0, 1s, bắt đầu 2026-10-20T11:19:15Z)
- **cargo test:** 9 đạt / 0 lỗi / 0 ignore trong 3 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test file_id_is_ordered_hashable_and_indexes ... ok
test empty_file_and_name ... ok
test line_col_counts_unicode_scalars_not_bytes ... ok
test line_col_lf_and_crlf ... ok
test snippet_and_line_text ... ok
test source_map_assigns_file_ids_by_sorted_name_not_by_input_order ... ok
test source_map_rejects_duplicate_names_and_handles_empty ... ok
test span_basics ... ok
test span_new_rejects_inverted_range - should panic ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

   Doc-tests gt_span

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


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


### C_gt-diag — cargo test -p gt-diag

- **Trạng thái:** PASS (mã thoát 0, 0s, bắt đầu 2026-10-20T11:19:25Z)
- **cargo test:** 40 đạt / 0 lỗi / 0 ignore trong 5 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test json_key_order_is_fixed_and_bytes_are_identical ... ok
test json_is_safe_to_embed_in_scripts ... ok
test labels_and_edits_in_another_file_get_positions_from_their_own_file ... ok
test labels_notes_and_suggestions ... ok
test schema_file_matches_code_version ... ok
test many_diagnostics_and_truncation_in_summary ... ok
test string_escaping_roundtrips ... ok
test unknown_file_gives_null_positions ... ok
test summary_exit_code_and_deny_yellow ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests gt_diag

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


### C_gt-ast — cargo test -p gt-ast

- **Trạng thái:** PASS (mã thoát 0, 0s, bắt đầu 2026-10-20T11:19:25Z)
- **cargo test:** 7 đạt / 0 lỗi / 0 ignore trong 3 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```

running 7 tests
test contracts_effects_generics_and_trait_impl_members ... ok
test doc_comments_and_attributes_are_node_children ... ok
test every_grammar_construct_is_representable ... ok
test hand_built_function_has_unique_dense_ids_and_spans ... ok
test node_id_gen_is_dense_sequential_and_overflow_safe ... ok
test node_ids_of_different_files_never_collide ... ok
test ast_is_independent_of_surface_syntax ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests gt_ast

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


### C_gt-parser — cargo test -p gt-parser

- **Trạng thái:** PASS (mã thoát 0, 12s, bắt đầu 2026-10-20T11:19:26Z)
- **cargo test:** 68 đạt / 0 lỗi / 0 ignore trong 13 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test block_like_statements_and_tail_rule ... ok
test assignment_places_accept_deref_chains_and_self ... ok
test doc_comment_in_wrong_place_is_an_error ... ok
test expr_ns_applies_before_every_block ... ok
test loops ... ok
test match_and_patterns ... ok
test reserved_and_hard_keywords_are_rejected ... ok
test unsafe_blocks ... ok
test simple_statements ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests gt_parser

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


### C_gt-stack — cargo test -p gt-stack

- **Trạng thái:** PASS (mã thoát 0, 0s, bắt đầu 2026-10-20T11:19:38Z)
- **cargo test:** 6 đạt / 0 lỗi / 0 ignore trong 3 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
     Running tests/stack.rs (target/debug/deps/stack-a9a081d1a0a09b1a)

running 6 tests
test nested_calls_each_get_their_own_big_stack ... ok
test deep_recursion_that_would_overflow_a_default_stack_works ... ok
test panic_with_non_string_payload_still_reports ... ok
test panics_become_errors_not_crashes ... ok
test returns_the_value_and_may_borrow_from_the_caller ... ok
test stack_constant_is_large_enough_for_the_depth_contract ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

   Doc-tests gt_stack

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


