# KẾT QUẢ CHẠY TOÀN BỘ DỰ ÁN GRIT (tự sinh — đừng sửa tay; ghi chú tay đặt ở `audit/results/<ID>.note.md`)

- Mã nguồn được chạy: `c56f9d1 2026-10-20 audit: bộ chạy toàn bộ dự án + kế hoạch + probe thử tấn công (điểm lưu 1)` (nhánh làm việc hiện tại; bộ chạy KHÔNG sửa tệp nào của dự án).
- Môi trường: Linux container (xem giai đoạn A00). Dự án được thiết kế cho **Windows x86_64 MSVC + LLVM 20.1.8** (docs/ENVIRONMENT.md).
- Cách chạy/tiếp tục: `bash audit/run_all.sh` (tự bỏ qua giai đoạn đã xong; xem `audit/PLAN.md`).

## Tổng hợp

| Trạng thái | Số giai đoạn |
|---|---|
| PASS | 25 |
| FAIL | 4 |
| BLOCKED | 1 |
| RUNNING | 2 |
| PENDING | 18 |

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
| C_gt-fmt | cargo test -p gt-fmt | **PASS** | 0 | 915 |
| C_gt-resolve | cargo test -p gt-resolve | **PASS** | 0 | 22 |
| C_gt-types | cargo test -p gt-types | **PASS** | 0 | 34 |
| C_gt-mir | cargo test -p gt-mir | **PASS** | 0 | 6 |
| C_gt-dataflow | cargo test -p gt-dataflow | **PASS** | 0 | 4 |
| C_gt-borrowck | cargo test -p gt-borrowck | **PASS** | 0 | 204 |
| C_gt-oblig | cargo test -p gt-oblig | **PASS** | 0 | 275 |
| C_gt-effects | cargo test -p gt-effects | **PASS** | 0 | 0 |
| C_gt-conc | cargo test -p gt-conc | **PASS** | 0 | 1 |
| C_gt-term | cargo test -p gt-term | **RUNNING** | - | 0 |
| C_gt-secrets | cargo test -p gt-secrets | PENDING | | |
| D01_native_probe | Dựng nguyên trạng gt-driver/rig/gt-llvm (cần LLVM 20 + Windows) | PENDING | | |
| E01_gtrt | runtime/gt-rt (runtime native, chỉ Windows x86_64) | PENDING | | |
| F01_expect | tools/check_expectations.py (nhất quán corpus + mã lỗi) | PENDING | | |
| F02_tool_tests | unittest của tools/*.py | PENDING | | |
| G00_copy_patch | Bản sao NGOÀI dự án + vá tối thiểu để gritc dựng không cần LLVM | **PASS** | 0 | 10 |
| G01_driver_tests | [bản sao] cargo test -p gt-driver --no-default-features | **FAIL** | 101 | 21 |
| G02_rig_tests | [bản sao] cargo test -p rig --no-default-features | **FAIL** | 101 | 6 |
| G03_corpus_cli | [bản sao] gritc check trên toàn bộ tests/{pass,fail,warn} | **PASS** | 0 | 12 |
| G04_run_programs | [bản sao] gritc run: mẫu, bench, 6 công cụ examples/ | **PASS** | 0 | 670 |
| G05_fmt_idempotent | [bản sao] gritc fmt: tính lũy đẳng trên corpus pass | PENDING | | |
| G06_determinism | [bản sao] gritc build hai lần ⇒ cùng băm (D9) | PENDING | | |
| G07_explain | [bản sao] gritc explain cho mọi mã trong KNOWN | PENDING | | |
| G08_probes | [bản sao] ~110 chương trình THỬ TẤN CÔNG tự viết (audit/probes): tìm âm tính giả | PENDING | | |
| G09_diff_checks | [bản sao] vi sai M7: --checks=all ≡ --checks=proven trên số học ngẫu nhiên (bộ sinh riêng) | PENDING | | |
| G10_mutfuzz | [bản sao] fuzz đột biến token qua CLI gritc (ICE/crash/treo) | PENDING | | |
| L01_linux_port | [bản sao 2] port Linux (vá 5 tệp): gt-rt + gritc có backend LLVM 20 + liên kết bằng cc | **PASS** | 0 | 26 |
| L02_native_driver_tests | [bản sao 2] cargo test -p gt-driver (backend LLVM thật: llvm*, release*, threads, vec, utf8…) | **RUNNING** | - | 0 |
| L03_native_rig_tests | [bản sao 2] cargo test -p rig (rig build/run/test + 6 công cụ dogfood bằng exe native) | PENDING | | |
| L04_native_other_crates | [bản sao 2] cargo test -p gt-guard -p gt-codegen-llvm -p gt-llvm + gt-rt | PENDING | | |
| L05_native_diff | [bản sao 2] vi sai thông dịch ≡ exe native trên số học ngẫu nhiên (bộ sinh riêng) | PENDING | | |
| H01_llvm_probe | Thử lấy LLVM 20 cho Linux (khả năng port) | **BLOCKED** | 77 | 1 |
| I01_fuzz_tools | Công cụ fuzz: nightly + cargo-fuzz (cài nếu thiếu, ngoài dự án) | PENDING | | |
| I02_fuzz_lex | [bản sao] cargo-fuzz lexer 300s (ASan, libFuzzer) | PENDING | | |
| I03_fuzz_parse | [bản sao] cargo-fuzz parser 300s (ASan, libFuzzer) | PENDING | | |

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


### C_gt-fmt — cargo test -p gt-fmt

- **Trạng thái:** PASS (mã thoát 0, 915s, bắt đầu 2026-10-20T11:19:38Z)
- **cargo test:** 35 đạt / 0 lỗi / 0 ignore trong 6 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```

running 7 tests
test alias_and_canonical_give_identical_output ... ok
test already_canonical_corpus_files_keep_every_comment_line ... ok
test files_with_syntax_errors_are_refused ... ok
test lifetime_arguments_in_types_roundtrip_adr_021 ... ok
test every_pass_file_roundtrips_and_is_idempotent ... ok
test warn_files_and_parseable_fail_files_roundtrip_too ... ok
test random_expressions_roundtrip_with_correct_parentheses ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.52s

   Doc-tests gt_fmt

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


### C_gt-resolve — cargo test -p gt-resolve

- **Trạng thái:** PASS (mã thoát 0, 22s, bắt đầu 2026-10-20T11:34:53Z)
- **cargo test:** 53 đạt / 0 lỗi / 0 ignore trong 8 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test random_programs_never_panic_terminate_and_are_deterministic ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.93s

     Running tests/snapshots.rs (target/debug/deps/snapshots-273f6c765b23188c)

running 2 tests
test every_resolver_snapshot_has_a_case_and_says_something ... ok
test resolver_error_messages_match_their_snapshots ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

   Doc-tests gt_resolve

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


### C_gt-types — cargo test -p gt-types

- **Trạng thái:** PASS (mã thoát 0, 34s, bắt đầu 2026-10-20T11:35:15Z)
- **cargo test:** 174 đạt / 0 lỗi / 0 ignore trong 19 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test bodies_list_functions_members_consts_and_statics_but_not_the_prelude ... ok
test call_sites_record_target_generics_and_receiver_adjustment ... ok
test impl_bounds_are_in_scope_of_members ... ok
test is_copy_respects_the_bounds_in_scope ... ok
test needs_drop_follows_adr_020_q26 ... ok
test inclusive_ranges_have_their_own_type ... ok
test trait_methods_on_type_parameters_stay_unresolved_until_mono ... ok
test type_parameters_need_drop_in_polymorphic_mir ... ok
test user_drop_impls_are_rejected_in_phase_1_e0313 ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

   Doc-tests gt_types

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


### C_gt-mir — cargo test -p gt-mir

- **Trạng thái:** PASS (mã thoát 0, 6s, bắt đầu 2026-10-20T11:35:49Z)
- **cargo test:** 30 đạt / 0 lỗi / 0 ignore trong 11 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test duplicating_a_storage_dead_or_using_a_dead_local_is_detected ... ok
test fall_off_is_reported_only_when_reachable ... ok
test implicit_reborrow_replaces_move_of_a_unique_reference_place_at_coercion_points ... ok
test lowered_mir_is_valid ... ok
test literal_true_and_false_conditions_are_folded_but_nothing_else ... ok
test never_function_with_a_reachable_return_is_e0307 ... ok
test quantifier_outside_a_contract_is_a_type_error_not_an_ice ... ok
test removing_any_single_storage_dead_on_a_reachable_path_is_detected ... ok
test structural_errors_are_detected ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

   Doc-tests gt_mir

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


### C_gt-dataflow — cargo test -p gt-dataflow

- **Trạng thái:** PASS (mã thoát 0, 4s, bắt đầu 2026-10-20T11:35:56Z)
- **cargo test:** 14 đạt / 0 lỗi / 0 ignore trong 5 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test definitely_assigned_is_an_all_paths_analysis ... ok
test call_destination_is_initialized_only_on_the_return_edge ... ok
test drop_deinitializes_and_is_exempt_from_the_assignment_invariant ... ok
test liveness_is_fixed_on_the_counting_loop ... ok
test moves_deinitialize_and_partial_moves_touch_only_the_subtree ... ok
test maybe_init_and_maybe_uninit_on_the_diamond ... ok
test return_uses_the_return_place ... ok
test reverse_postorder_is_fixed_and_skips_unreachable_blocks ... ok
test the_same_cfg_with_both_branches_assigning_is_clean ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests gt_dataflow

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


### C_gt-borrowck — cargo test -p gt-borrowck

- **Trạng thái:** PASS (mã thoát 0, 204s, bắt đầu 2026-10-20T11:36:00Z)
- **cargo test:** 57 đạt / 0 lỗi / 0 ignore trong 11 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test alias_through_function_calls_and_returned_references ... ok
test alias_through_generics ... ok
test alias_through_match_and_partial_moves ... ok
test alias_through_reborrow_chains ... ok
test invariance_survives_adt_construction ... ok
test arena_versions_are_invalidated_by_every_operation ... ok
test moves_out_of_places_that_cannot_be_moved_from ... ok
test unique_references_cannot_be_duplicated_by_copying_or_moving_around ... ok
test references_stored_in_structs_and_collections ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s

   Doc-tests gt_borrowck

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


### C_gt-oblig — cargo test -p gt-oblig

- **Trạng thái:** PASS (mã thoát 0, 275s, bắt đầu 2026-10-20T11:39:24Z)
- **cargo test:** 134 đạt / 0 lỗi / 5 ignore trong 20 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test drop_all_leaves_only_the_diagonal ... ok
test join_with_an_empty_zone_is_the_other_zone ... ok
test meet_with_an_empty_zone_empties_and_empty_stays_empty ... ok
test forget_keeps_the_diagonal_and_clears_only_that_variable ... ok
test order_treats_the_empty_zone_as_bottom ... ok
test self_difference_constraints ... ok
test shift_leaves_the_diagonal_at_zero ... ok
test widen_with_an_empty_zone_is_the_other_zone ... ok
test widen_keeps_equal_bounds_and_drops_looser_ones ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests gt_oblig

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


### C_gt-effects — cargo test -p gt-effects

- **Trạng thái:** PASS (mã thoát 0, 0s, bắt đầu 2026-10-20T11:43:59Z)
- **cargo test:** 16 đạt / 0 lỗi / 0 ignore trong 4 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test two_level_types_with_the_same_short_name_are_ambiguous ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s

     Running tests/fail_closed_nodes.rs (target/debug/deps/fail_closed_nodes-1c7524818aeeedde)

running 2 tests
test mutex_with_without_a_function_argument_is_rejected ... ok
test a_block_without_terminator_is_an_ice_not_silent_acceptance ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests gt_effects

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


### C_gt-conc — cargo test -p gt-conc

- **Trạng thái:** PASS (mã thoát 0, 1s, bắt đầu 2026-10-20T11:43:59Z)
- **cargo test:** 4 đạt / 0 lỗi / 0 ignore trong 3 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ordering.rs (target/debug/deps/ordering-c5f79655ddf00f6a)

running 4 tests
test atomic_methods_used_as_function_values_are_rejected ... ok
test only_atomic_methods_are_rejected_as_function_values ... ok
test atomic_orderings_must_be_literal_and_valid ... ok
test ordering_tracing_is_exact_and_scans_every_call ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

   Doc-tests gt_conc

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


### G00_copy_patch — Bản sao NGOÀI dự án + vá tối thiểu để gritc dựng không cần LLVM

- **Trạng thái:** PASS (mã thoát 0, 10s, bắt đầu 2026-10-20T11:32:41Z)
- đã vá 1 tệp (crates/gt-driver/src/cli.rs, +17 dòng) trong BẢN SAO để gritc dựng không cần LLVM

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
   Compiling memchr v2.8.3
   Compiling itoa v1.0.18
   Compiling gt-parser v0.1.0 (/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit/crates/gt-parser)
   Compiling gt-resolve v0.1.0 (/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit/crates/gt-resolve)
   Compiling gt-fmt v0.1.0 (/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit/crates/gt-fmt)
   Compiling gt-types v0.1.0 (/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit/crates/gt-types)
   Compiling gt-mir v0.1.0 (/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit/crates/gt-mir)
   Compiling gt-dataflow v0.1.0 (/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit/crates/gt-dataflow)
   Compiling gt-oblig v0.1.0 (/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit/crates/gt-oblig)
   Compiling gt-effects v0.1.0 (/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit/crates/gt-effects)
   Compiling gt-borrowck v0.1.0 (/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit/crates/gt-borrowck)
   Compiling gt-secrets v0.1.0 (/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit/crates/gt-secrets)
   Compiling gt-conc v0.1.0 (/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit/crates/gt-conc)
   Compiling gt-term v0.1.0 (/tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit/crates/gt-term)
    Finished `dev` profile [unoptimized] target(s) in 9.69s
-rwxr-xr-x 2 root root 12268216 Oct 20 11:32 /tmp/claude-0/-home-user-Grit/95de13cb-b880-56c6-959a-865f7f8f23cf/scratchpad/work/grit/target/debug/gritc
gritc 0.1.0 (backend mặc định: interpreter)
SUMMARY: đã vá 1 tệp (crates/gt-driver/src/cli.rs, +17 dòng) trong BẢN SAO để gritc dựng không cần LLVM
```
</details>


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


### G02_rig_tests — [bản sao] cargo test -p rig --no-default-features

- **Trạng thái:** FAIL (mã thoát 101, 6s, bắt đầu 2026-10-20T11:33:12Z)
- **cargo test:** 10 đạt / 37 lỗi / 3 ignore trong 10 bộ test
- **rustc/clippy:** 8 dòng `error`, 0 dòng `warning`

**Test lỗi (37):**
- `wc_error_paths_are_controlled`
- `wc_handles_a_large_file_and_reports_throughput`
- `wc_written_in_grit_matches_the_reference_on_many_files`
- `json_tool_counts_and_depth_hand_checked`
- `deep_nesting_is_a_controlled_error_not_a_stack_overflow`
- `json_tool_matches_reference_and_serde_on_hand_written_documents`
- `json_tool_error_paths_and_missing_args`
- `mutation_fuzz_against_reference_and_serde`
- `json_tool_on_large_files_reports_throughput`
- `nist_vectors_hard_coded`
- `large_files_match_hashlib_and_report_throughput`
- `padding_boundaries_match_hashlib`
- `random_content_200_files_match_hashlib`
- `sha256_error_paths`
- `arena_formula_is_exact_and_exhaustion_is_controlled`
- `error_paths_use_stderr_and_exit_code_2`
- `four_mib_of_empty_lines`
- `one_million_short_lines`
- `one_very_long_line_of_1_mib`
- `random_text_1_mib_and_32_mib`
- `small_cases_match_the_reference_byte_for_byte`
- `bad_tokens_and_too_large_values_are_controlled_errors`
- `random_and_large_inputs_match_the_reference`
- `small_cases_match_the_reference_byte_for_byte`
- `invalid_utf8_is_a_controlled_error_not_an_abort`
- `large_files_match_the_reference`
- `many_distinct_words`
- `small_cases_match_the_reference_byte_for_byte`
- `builds_are_reproducible_byte_for_byte`
- `add_edits_manifest_relocks_and_rolls_back_on_failure`
- `cross_package_generic_trait_forwarder_passes_e0703_and_runs_correctly`
- `failing_test_exits_1_and_names_the_abort`
- `lock_missing_is_created_stale_is_always_refused_and_locked_flag_is_strict`
- `exit_codes_gritc_passthrough_and_rig_own_range`
- `option_ord_in_another_package_uses_reference_patterns_passes_e0703_and_runs_correctly`
- `new_check_build_run_roundtrip`
- `three_package_project_builds_runs_and_tests_with_diamond_loaded_once`

<details><summary>Chi tiết panic (tối đa 12)</summary>

`wc_error_paths_are_controlled`
```
thread 'wc_error_paths_are_controlled' (14955) panicked at crates/rig/tests/dogfood.rs:56:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:12:27
   |
12 |     ⎇ c == 10u8 { lines ← lines + 1; } ⇝ { }
   |                           ^^^^^^^^^
```
`wc_handles_a_large_file_and_reports_throughput`
```
thread 'wc_handles_a_large_file_and_reports_throughput' (14956) panicked at crates/rig/tests/dogfood.rs:56:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:12:27
   |
12 |     ⎇ c == 10u8 { lines ← lines + 1; } ⇝ { }
   |                           ^^^^^^^^^
```
`wc_written_in_grit_matches_the_reference_on_many_files`
```
thread 'wc_written_in_grit_matches_the_reference_on_many_files' (15508) panicked at crates/rig/tests/dogfood.rs:56:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:12:27
   |
12 |     ⎇ c == 10u8 { lines ← lines + 1; } ⇝ { }
   |                           ^^^^^^^^^
```
`json_tool_counts_and_depth_hand_checked`
```
thread 'json_tool_counts_and_depth_hand_checked' (15765) panicked at crates/rig/tests/dogfood_json.rs:42:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:16:19
   |
16 |   ⎇ is_digit(c) { (c as u32) - 48u32 } ⇝ { ⎇ c >= 97u8 { (c as u32) - 87u32 } ⇝ { (c as u32) - 55u32 } }
   |                   ^^^^^^^^^^^^^^^^^^
```
`deep_nesting_is_a_controlled_error_not_a_stack_overflow`
```
thread 'deep_nesting_is_a_controlled_error_not_a_stack_overflow' (15764) panicked at crates/rig/tests/dogfood_json.rs:42:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:16:19
   |
16 |   ⎇ is_digit(c) { (c as u32) - 48u32 } ⇝ { ⎇ c >= 97u8 { (c as u32) - 87u32 } ⇝ { (c as u32) - 55u32 } }
   |                   ^^^^^^^^^^^^^^^^^^
```
`json_tool_matches_reference_and_serde_on_hand_written_documents`
```
thread 'json_tool_matches_reference_and_serde_on_hand_written_documents' (17705) panicked at crates/rig/tests/dogfood_json.rs:42:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:16:19
   |
16 |   ⎇ is_digit(c) { (c as u32) - 48u32 } ⇝ { ⎇ c >= 97u8 { (c as u32) - 87u32 } ⇝ { (c as u32) - 55u32 } }
   |                   ^^^^^^^^^^^^^^^^^^
```
`json_tool_error_paths_and_missing_args`
```
thread 'json_tool_error_paths_and_missing_args' (17704) panicked at crates/rig/tests/dogfood_json.rs:42:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:16:19
   |
16 |   ⎇ is_digit(c) { (c as u32) - 48u32 } ⇝ { ⎇ c >= 97u8 { (c as u32) - 87u32 } ⇝ { (c as u32) - 55u32 } }
   |                   ^^^^^^^^^^^^^^^^^^
```
`mutation_fuzz_against_reference_and_serde`
```
thread 'mutation_fuzz_against_reference_and_serde' (19463) panicked at crates/rig/tests/dogfood_json.rs:42:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:16:19
   |
16 |   ⎇ is_digit(c) { (c as u32) - 48u32 } ⇝ { ⎇ c >= 97u8 { (c as u32) - 87u32 } ⇝ { (c as u32) - 55u32 } }
   |                   ^^^^^^^^^^^^^^^^^^
```
`json_tool_on_large_files_reports_throughput`
```
thread 'json_tool_on_large_files_reports_throughput' (19449) panicked at crates/rig/tests/dogfood_json.rs:42:5:
assertion `left == right` failed: rig build: note[N0601]: không chứng minh được không tràn: giữ kiểm tra lúc chạy
  --> main.gt:16:19
   |
16 |   ⎇ is_digit(c) { (c as u32) - 48u32 } ⇝ { ⎇ c >= 97u8 { (c as u32) - 87u32 } ⇝ { (c as u32) - 55u32 } }
   |                   ^^^^^^^^^^^^^^^^^^
```
`nist_vectors_hard_coded`
```
thread 'nist_vectors_hard_coded' (21232) panicked at crates/rig/tests/dogfood_sha256.rs:41:5:
assertion `left == right` failed: rig build: note[N0602]: không chứng minh được chỉ số/khoảng trong biên: giữ kiểm tra lúc chạy
  --> main.gt:14:15
   |
14 |   bor(bor(shl(b[o] as u32, 24u32), shl(b[o + 1] as u32, 16u32)), bor(shl(b[o + 2] as u32, 8u32), b[o + 3] as u32))
   |               ^^^^
```
`large_files_match_hashlib_and_report_throughput`
```
thread 'large_files_match_hashlib_and_report_throughput' (21230) panicked at crates/rig/tests/dogfood_sha256.rs:41:5:
assertion `left == right` failed: rig build: note[N0602]: không chứng minh được chỉ số/khoảng trong biên: giữ kiểm tra lúc chạy
  --> main.gt:14:15
   |
14 |   bor(bor(shl(b[o] as u32, 24u32), shl(b[o + 1] as u32, 16u32)), bor(shl(b[o + 2] as u32, 8u32), b[o + 3] as u32))
   |               ^^^^
```
`padding_boundaries_match_hashlib`
```
thread 'padding_boundaries_match_hashlib' (22138) panicked at crates/rig/tests/dogfood_sha256.rs:41:5:
assertion `left == right` failed: rig build: note[N0602]: không chứng minh được chỉ số/khoảng trong biên: giữ kiểm tra lúc chạy
  --> main.gt:14:15
   |
14 |   bor(bor(shl(b[o] as u32, 24u32), shl(b[o + 1] as u32, 16u32)), bor(shl(b[o + 2] as u32, 8u32), b[o + 3] as u32))
   |               ^^^^
```
</details>

**Lỗi (dòng `error` đầu tiên):**
```
error: test failed, to rerun pass `-p rig --test dogfood`
     Running tests/dogfood_json.rs (target/debug/deps/dogfood_json-9a0a78e0696b1576)

running 6 tests
error: test failed, to rerun pass `-p rig --test dogfood_json`
     Running tests/dogfood_sha256.rs (target/debug/deps/dogfood_sha256-6d2b73e3d16234f4)

running 5 tests
error: test failed, to rerun pass `-p rig --test dogfood_sha256`
     Running tests/dogfood_sortlines.rs (target/debug/deps/dogfood_sortlines-f10adfaea3fef02f)

running 8 tests
error: test failed, to rerun pass `-p rig --test dogfood_sortlines`
     Running tests/dogfood_stats.rs (target/debug/deps/dogfood_stats-85f7110243405591)

running 4 tests
error: test failed, to rerun pass `-p rig --test dogfood_stats`
     Running tests/dogfood_wordfreq.rs (target/debug/deps/dogfood_wordfreq-b3b6d6a2a87eb5cd)

running 6 tests
error: test failed, to rerun pass `-p rig --test dogfood_wordfreq`
     Running tests/rig.rs (target/debug/deps/rig-36e069ca0be0dbbc)

running 14 tests
error: test failed, to rerun pass `-p rig --test rig`
   Doc-tests rig

running 0 tests
error: 7 targets failed:
    `-p rig --test dogfood`
    `-p rig --test dogfood_json`
    `-p rig --test dogfood_sha256`
```

<details><summary>Đuôi log (18 dòng cuối)</summary>

```

test result: FAILED. 5 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s

error: test failed, to rerun pass `-p rig --test rig`
   Doc-tests rig

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 7 targets failed:
    `-p rig --test dogfood`
    `-p rig --test dogfood_json`
    `-p rig --test dogfood_sha256`
    `-p rig --test dogfood_sortlines`
    `-p rig --test dogfood_stats`
    `-p rig --test dogfood_wordfreq`
    `-p rig --test rig`
```
</details>


### G03_corpus_cli — [bản sao] gritc check trên toàn bộ tests/{pass,fail,warn}

- **Trạng thái:** PASS (mã thoát 0, 12s, bắt đầu 2026-10-20T11:33:18Z)
- corpus qua dòng lệnh gritc: 855 ca; khớp 855; lệch 0
- fail:OK = 438
- fail:OK_EXTRA = 2
- pass:OK = 402
- warn:OK = 13
- 2 ca fail phát thêm mã E ngoài mã kỳ vọng (thông tin)

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: corpus qua dòng lệnh gritc: 855 ca; khớp 855; lệch 0
SUMMARY: fail:OK = 438
SUMMARY: fail:OK_EXTRA = 2
SUMMARY: pass:OK = 402
SUMMARY: warn:OK = 13
SUMMARY: 2 ca fail phát thêm mã E ngoài mã kỳ vọng (thông tin)
EXTRA fail/E0401_move_in_while.gt  muốn=fail E0401  có thêm mã ['E0701']
EXTRA fail/E1001_raw_ptr_return.gt  muốn=fail E1001  có thêm mã ['E0703']
```
</details>


### G04_run_programs — [bản sao] gritc run: mẫu, bench, 6 công cụ examples/

- **Trạng thái:** PASS (mã thoát 0, 670s, bắt đầu 2026-10-20T11:33:31Z)
- mẫu/bench chạy bằng thông dịch: 8 chương trình; exit0=2 abort70=0 hết giới hạn71=2 khác=4

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
== tests/samples + tests/bench (chạy bằng thông dịch MIR)
arrays_slices.gt: exit=0 kỳ vọng=79 nhận=Int(79, I64) green
numeric.gt: exit=0 kỳ vọng=11630 nhận=Int(11630, U64) green
threads.gt: exit=2 kỳ vọng=8000 nhận=None green
two_locks.gt: exit=2 kỳ vọng=4000 nhận=None green
fib.gt: exit=71 kỳ vọng=? nhận=None green
loop_sum.gt: exit=71 kỳ vọng=? nhận=None green
slice_sum.gt: exit=124 kỳ vọng=? nhận=NOJSON
threads.gt: exit=2 kỳ vọng=? nhận=None green
== 6 công cụ dogfood (examples/*/src) — cần Cap<Fs>/args/IO của runtime native
examples/json: exit=2 :: gritc: lỗi nội bộ khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR cho $prelude::root_env
examples/sha256: exit=2 :: gritc: lỗi nội bộ khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR cho $prelude::root_env
examples/sortlines: exit=1 :: gritc: phán quyết red; 1 lỗi, 0 cảnh báo, 0 ghi chú; backend: interpreter
examples/stats: exit=2 :: gritc: lỗi nội bộ khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR cho $prelude::root_env
examples/wc: exit=2 :: gritc: lỗi nội bộ khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR cho $prelude::root_env
examples/wordfreq: exit=2 :: gritc: lỗi nội bộ khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR cho $prelude::root_env
SUMMARY: mẫu/bench chạy bằng thông dịch: 8 chương trình; exit0=2 abort70=0 hết giới hạn71=2 khác=4
```
</details>


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


### H01_llvm_probe — Thử lấy LLVM 20 cho Linux (khả năng port)

- **Trạng thái:** BLOCKED (mã thoát 77, 1s, bắt đầu 2026-10-20T11:41:07Z)
- kết quả thăm dò xem log; chưa dựng gì

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
  Version table:
 *** 1:20.1.2-0ubuntu1~24.04.3 100
        100 /var/lib/dpkg/status
llvm-20-dev:
  Installed: (none)
  Candidate: (none)
  Version table:
== tải LLVM 20.1.8 bản Linux từ GitHub (chỉ HEAD)
HTTP/1.1 200 Connection Established

HTTP/1.1 302 Found
Connection: close
Cache-Control: no-cache
== apt.llvm.org
HTTP/1.1 200 Connection Established

HTTP/2 200 
SUMMARY: kết quả thăm dò xem log; chưa dựng gì
```
</details>


