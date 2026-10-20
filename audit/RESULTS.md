# KẾT QUẢ CHẠY TOÀN BỘ DỰ ÁN GRIT (tự sinh — đừng sửa tay; ghi chú tay đặt ở `audit/results/<ID>.note.md`)

- Mã nguồn được chạy: `8f3f344 2026-10-20 audit: G14 result + ANALYSIS row (checkpoint 5)` (nhánh làm việc hiện tại; bộ chạy KHÔNG sửa tệp nào của dự án).
- Môi trường: Linux container (xem giai đoạn A00). Dự án được thiết kế cho **Windows x86_64 MSVC + LLVM 20.1.8** (docs/ENVIRONMENT.md).
- Cách chạy/tiếp tục: `bash audit/run_all.sh` (tự bỏ qua giai đoạn đã xong; xem `audit/PLAN.md`).

## Tổng hợp

| Trạng thái | Số giai đoạn |
|---|---|
| PASS | 43 |
| FAIL | 12 |
| BLOCKED | 3 |

PASS = chạy xong và đạt · FAIL = chạy được nhưng lỗi/thiếu · BLOCKED = KHÔNG chạy được trong môi trường này (có bằng chứng) · TIMEOUT = quá hạn · SKIP = bỏ qua có chủ ý · PENDING = chưa tới lượt.

## Bảng giai đoạn

| ID | Tên | Trạng thái | Thoát | Giây |
|---|---|---|---|---|
| A00_env | Môi trường: công cụ, phiên bản, tài nguyên | **PASS** | 0 | 0 |
| A01_fetch | Tải phụ thuộc theo Cargo.lock (--locked) | **PASS** | 0 | 0 |
| A02_inventory | Kiểm kê dự án: crate, tệp, LOC, tệp nhị phân bị theo dõi, CI/deny | **PASS** | 0 | 1 |
| B01_fmt | cargo fmt --all --check | **PASS** | 0 | 1 |
| B02_clippy | clippy -D warnings (17 crate thuần) | **FAIL** | 101 | 13 |
| B03_audit | cargo audit (lỗ hổng phụ thuộc) | **PASS** | 0 | 129 |
| B04_ci_deny | CI (ci.yml) và deny.toml có tồn tại không | **FAIL** | 1 | 0 |
| B05_codes | Mọi mã chẩn đoán đã biết có tài liệu + ≥3 mẫu fail + ≥3 mẫu pass (GRIT_PLAN §0.5) | **FAIL** | 1 | 0 |
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
| C_gt-term | cargo test -p gt-term | **PASS** | 0 | 359 |
| C_gt-secrets | cargo test -p gt-secrets | **PASS** | 0 | 1 |
| D01_native_probe | Dựng nguyên trạng gt-driver/rig/gt-llvm (cần LLVM 20 + Windows) | **BLOCKED** | 77 | 4 |
| E01_gtrt | runtime/gt-rt (runtime native, chỉ Windows x86_64) | **BLOCKED** | 77 | 0 |
| F01_expect | tools/check_expectations.py (nhất quán corpus + mã lỗi) | **PASS** | 0 | 1 |
| F02_tool_tests | unittest của tools/*.py | **PASS** | 0 | 8 |
| G00_copy_patch | Bản sao NGOÀI dự án + vá tối thiểu để gritc dựng không cần LLVM | **PASS** | 0 | 10 |
| G01_driver_tests | [bản sao] cargo test -p gt-driver --no-default-features | **FAIL** | 101 | 21 |
| G02_rig_tests | [bản sao] cargo test -p rig --no-default-features | **FAIL** | 101 | 6 |
| G03_corpus_cli | [bản sao] gritc check trên toàn bộ tests/{pass,fail,warn} | **PASS** | 0 | 12 |
| G04_run_programs | [bản sao] gritc run: mẫu, bench, 6 công cụ examples/ | **PASS** | 0 | 670 |
| G05_fmt_idempotent | [bản sao] gritc fmt: tính lũy đẳng trên corpus pass | **PASS** | 0 | 7 |
| G06_determinism | [bản sao] gritc build hai lần ⇒ cùng băm (D9) | **PASS** | 0 | 4 |
| G07_explain | [bản sao] gritc explain cho mọi mã trong KNOWN | **FAIL** | 1 | 0 |
| G08_probes | [bản sao] ~180 chương trình THỬ TẤN CÔNG tự viết (audit/probes): tìm âm tính giả | **FAIL** | 1 | 4 |
| G09_diff_checks | [bản sao] vi sai M7: --checks=all ≡ --checks=proven trên số học ngẫu nhiên (bộ sinh riêng) | **PASS** | 0 | 407 |
| G10_mutfuzz | [bản sao] fuzz đột biến token qua CLI gritc (ICE/crash/treo) | **FAIL** | 1 | 122 |
| L01_linux_port | [bản sao 2] port Linux (vá 5 tệp): gt-rt + gritc có backend LLVM 20 + liên kết bằng cc | **PASS** | 0 | 26 |
| L02_native_driver_tests | [bản sao 2] cargo test -p gt-driver (backend LLVM thật: llvm*, release*, threads, vec, utf8…) | **FAIL** | 101 | 237 |
| L03_native_rig_tests | [bản sao 2] cargo test -p rig (rig build/run/test + 6 công cụ dogfood bằng exe native) | **PASS** | 0 | 32 |
| L04_native_other_crates | [bản sao 2] cargo test -p gt-guard -p gt-codegen-llvm -p gt-llvm + gt-rt | **PASS** | 0 | 1 |
| L05_native_diff | [bản sao 2] vi sai thông dịch ≡ exe native trên số học ngẫu nhiên (bộ sinh riêng) | **PASS** | 0 | 229 |
| L06_native_unsound_demo | [bản sao 2] chạy THẬT chương trình XANH nhưng dùng-sau-reset Arena (nhân đôi tay cầm): có đọc nhầm không | **FAIL** | 1 | 0 |
| L07_native_stack_limits | [bản sao 2] chương trình HỢP LỆ gây tràn ngăn xếp native: có abort có kiểm soát (mã 70) không | **FAIL** | 1 | 1 |
| L08_native_compiler_crash | [bản sao 2] đầu vào được M4–M9 cho qua nhưng làm CHÍNH gritc sập (kiểu kích thước vô hạn…) | **FAIL** | 1 | 0 |
| G11_fmt_meta | [bản sao] metamorphic: gritc fmt không đổi phán quyết (841 mẫu) | **PASS** | 0 | 65 |
| G12_alias_meta | [bản sao] metamorphic: alias ASCII ≡ canonical không đổi phán quyết (816 mẫu) | **PASS** | 0 | 56 |
| G13_order_meta | [bản sao] metamorphic: xáo thứ tự khai báo không đổi phán quyết (429 mẫu × 2) | **PASS** | 0 | 47 |
| G14_diff_arrays | [bản sao] vi sai M7 all≡proven trên chỉ số mảng/lát (nơi M7 xoá kiểm tra biên) | **PASS** | 0 | 206 |
| P01_proposed_fix_f01 | [bản sao 3] ĐỀ XUẤT sửa F-01 (9 dòng ở gt-borrowck/check.rs): 14 probe có bị chặn không, corpus/test có hồi quy không | **PASS** | 0 | 240 |
| H01_llvm_probe | Thử lấy LLVM 20 cho Linux (khả năng port) | **BLOCKED** | 77 | 1 |
| I01_fuzz_tools | Công cụ fuzz: nightly + cargo-fuzz (cài nếu thiếu, ngoài dự án) | **PASS** | 0 | 29 |
| I02_fuzz_lex | [bản sao] cargo-fuzz lexer 300s (ASan, libFuzzer) | **PASS** | 0 | 317 |
| I03_fuzz_parse | [bản sao] cargo-fuzz parser 300s (ASan, libFuzzer) | **PASS** | 0 | 315 |

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


**Ghi chú của người phân tích:**

- Lần chạy đầu dừng ở lỗi đầu tiên (không có `--keep-going`). Chạy lại tay với `--keep-going` (thư mục build riêng) cho **đúng 2 lỗi, cả hai ở tệp test, không ở mã sản phẩm**: (1) `crates/gt-lexer/tests/leak_check.rs:6` `unused import: std::process::Command` — `Command` chỉ dùng ở nhánh `cfg(windows)`/`cfg(macos)` nên trên Linux là import thừa; (2) `crates/gt-types/tests/match_oracle.rs:5` lint `doc_lazy_continuation` (clippy 1.97 trên máy này; dự án dùng 1.98.1).
- Bốn crate native (`gt-llvm`, `gt-codegen-llvm`, `gt-guard`, `gt-driver`) và `rig` không clippy được ở môi trường này (cần LLVM).


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


### B05_codes — Mọi mã chẩn đoán đã biết có tài liệu + ≥3 mẫu fail + ≥3 mẫu pass (GRIT_PLAN §0.5)

- **Trạng thái:** FAIL (mã thoát 1, 0s, bắt đầu 2026-10-20T11:50:15Z)
- 72 mã trong KNOWN; 8 mã chưa đủ tài liệu/mẫu theo GRIT_PLAN §0.5

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: 72 mã trong KNOWN; 8 mã chưa đủ tài liệu/mẫu theo GRIT_PLAN §0.5
THIẾU E1101: tài liệu có 0 mẫu fail/warn và 0 mẫu pass (cần ≥3 và ≥3)
THIẾU E1201: không có docs/errors/E1201.md
THIẾU E1202: không có docs/errors/E1202.md
THIẾU E1203: không có docs/errors/E1203.md
THIẾU E1204: không có docs/errors/E1204.md
THIẾU E1301: không có docs/errors/E1301.md
THIẾU E9901: không có docs/errors/E9901.md
THIẾU N0606: tài liệu có 0 mẫu fail/warn và 0 mẫu pass (cần ≥3 và ≥3)
```
</details>


**Ghi chú của người phân tích:**

- Kiểm toàn bộ 72 mã trong `KNOWN` (không chỉ danh sách `DOC_REQUIRED` mà `tools/check_expectations.py` dùng). 8 mã thiếu: 6 không có tài liệu (E1201–E1204, E1301, E9901), 2 có tài liệu nhưng 0 mẫu liệt kê (E1101 — test nằm ở gt-guard; N0606 — ghi chú của `--checks=proven`). Một số mã này có test trong crate (lexer/guard) nên là thiếu *tài liệu + mẫu corpus*, không nhất thiết thiếu *test*; nhưng GRIT_PLAN §0.5 đòi cả hai.


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


### C_gt-term — cargo test -p gt-term

- **Trạng thái:** PASS (mã thoát 0, 359s, bắt đầu 2026-10-20T11:44:00Z)
- **cargo test:** 29 đạt / 0 lỗi / 1 ignore trong 9 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test dump_term ... ignored
test given_variants_must_decrease_and_be_bounded ... ok
test indirect_recursion_is_rejected_even_with_a_variant ... ok
test infinite_loop_with_variant_still_needs_div ... ok
test counting_loops_are_inferred_or_verified ... ok
test methods_multiplication_and_leading_conjuncts ... ok
test loops_without_provable_progress_are_red ... ok
test recursive_variant_never_uses_a_modified_parameter ... ok
test recursion_needs_a_decreasing_variant_or_div ... ok

test result: ok. 10 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.24s

   Doc-tests gt_term

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


### C_gt-secrets — cargo test -p gt-secrets

- **Trạng thái:** PASS (mã thoát 0, 1s, bắt đầu 2026-10-20T11:49:59Z)
- **cargo test:** 15 đạt / 0 lỗi / 0 ignore trong 3 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test s101_in_comment_and_non_matches ... ok
test s101_known_patterns_in_string_literals ... ok
test s102_high_entropy_literal_assigned_to_secret_like_name ... ok
test s102_non_matches ... ok
test s201_declassify_and_s202_flow_to_io_sinks ... ok
test scanning_continues_past_non_matching_names_and_invalid_allows ... ok
test pattern_boundaries_are_exact ... ok
test taint_needs_only_one_tainted_local_among_many_operands ... ok
test taint_flows_through_many_shapes_and_only_real_declassify_counts ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s

   Doc-tests gt_secrets

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


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


**Ghi chú của người phân tích:**

- Bằng chứng: (1) `-p gt-driver` và `-p gt-llvm` ⇒ `failed to run custom build command for gt-llvm` (build.rs đòi `GRIT_LLVM_DIR/lib/LLVM-C.lib`; `.cargo/config.toml` đặt mặc định `D:/Tools/llvm-20.1.8`); (2) `-p gt-driver --no-default-features` và `-p rig --no-default-features` ⇒ **E0433 ×5** `cannot find module or crate gt_codegen_llvm` ở `cli.rs:25,60,71–73`. Tức là **kể cả cách "dựng không LLVM" mà tài liệu mô tả cũng hỏng**. ANALYSIS F-02.


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


**Ghi chú của người phân tích:**

- `runtime/gt-rt/src/lib.rs:10`: `compile_error!` cho mọi target ngoài `windows`+`x86_64`. Bản port Linux ở giai đoạn L01 thay `SRWLOCK` bằng khoá quay và bỏ cổng này: gt-rt dựng staticlib 22 MB và 6 test đạt.


### F01_expect — tools/check_expectations.py (nhất quán corpus + mã lỗi)

- **Trạng thái:** PASS (mã thoát 0, 1s, bắt đầu 2026-10-20T11:50:04Z)

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
Đã kiểm 881 file .gt: fail=440, warn=13, pass=402, lexer=8
Bảng mã lỗi: 72 mã, 246 tham chiếu test
OK: corpus nhất quán với error_codes.md
```
</details>


### F02_tool_tests — unittest của tools/*.py

- **Trạng thái:** PASS (mã thoát 0, 8s, bắt đầu 2026-10-20T11:50:05Z)
- **unittest:** Ran 9 tests in 1.087s
- **unittest:** OK
- **unittest:** Ran 4 tests in 0.508s
- **unittest:** OK
- **unittest:** Ran 3 tests in 4.029s
- **unittest:** OK
- **unittest:** Ran 18 tests in 2.820s
- **unittest:** OK

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
================ unittest tools/test_mut_guard.py
....
----------------------------------------------------------------------
Ran 4 tests in 0.508s

OK
================ unittest tools/test_mut_resume.py
...
----------------------------------------------------------------------
Ran 3 tests in 4.029s

OK
================ unittest tools/test_gate.py
..................
----------------------------------------------------------------------
Ran 18 tests in 2.820s

OK
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


**Ghi chú của người phân tích:**

- **Cả 9 test lỗi đều do bản dựng KHÔNG có backend LLVM, không phải lỗi logic**: 8 test nằm trong `release_guard.rs`/`release_traps.rs` (đòi `--profile=release`) và 1 test `cli.rs::directory_input_and_version` (đòi `--version` in "backend mặc định: llvm"). Các tệp này không được `#![cfg(feature = "llvm")]` bao lại như 18 tệp `llvm*.rs` khác ⇒ với `--no-default-features` chúng đỏ thay vì bị loại — **khuyết điểm cấu hình test** (xem ANALYSIS F-02).
- Phần chạy được và XANH: `gate.rs` (19 test: cổng phán quyết ĐỎ/ICE ⇒ không output, canary E0601/E0307, pass bắt buộc, chèn lỗi), `cli.rs` 9/10, `release_policy.rs` 6/6, `instance_keys.rs` 2/2, `ref_pattern_fault.rs` 1/1.
- Giai đoạn này chạy trên **bản sao có vá 17 dòng** (`audit/patches/driver_nollvm.patch`) vì nguyên trạng `gt-driver --no-default-features` không biên dịch (D01).


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


**Ghi chú của người phân tích:**

- 11 test đạt (4 unit + 5 `rig.rs` + 1 `dogfood_wordfreq` + …), phần còn lại đỏ vì `rig build/run/test` và 6 công cụ dogfood cần backend LLVM + liên kết exe (bản sao không có). Xem giai đoạn L03 (bản sao có port Linux) cho kết quả với backend native thật.


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


**Ghi chú của người phân tích:**

- Giai đoạn này **chỉ thông tin** (luôn trả 0). Đọc kết quả đúng cách: thông dịch MIR chỉ chạy được chương trình thuần số học/mảng (`arrays_slices` = 79, `numeric` = 11630 đúng kỳ vọng); `threads`/`two_locks` (static + spawn) và cả 6 công cụ `examples/*` (cần `root_env`/`root_fs`/I/O của runtime native) thoát mã 2 với thông báo "lỗi nội bộ: không có thân MIR cho $prelude::…" — thông dịch không có thân cho intrinsic của prelude. Đây là giới hạn đã ghi (DOGFOOD, BENCHMARK) nhưng thông báo "lỗi nội bộ" dễ gây hiểu nhầm là bug của gritc.
- `fib.gt`/`loop_sum.gt` thoát 71 (hết nhiên liệu 50 triệu bước), `slice_sum.gt` quá 600 s (thông dịch sao chép mảng mỗi lần `Use`): bench thiết kế cho backend native.
- Giới hạn đệ quy của thông dịch = **200 khung** (`gt-mir/src/interp.rs:125`): chương trình hợp lệ, dừng, đệ quy sâu hơn 200 (vd `rec(100000)` có `↓ n`) thoát 71.


### G05_fmt_idempotent — [bản sao] gritc fmt: tính lũy đẳng trên corpus pass

- **Trạng thái:** PASS (mã thoát 0, 7s, bắt đầu 2026-10-20T11:44:41Z)
- gritc fmt trên 398 mẫu pass: bị từ chối 0; không lũy đẳng 0; (mẫu gốc chưa ở dạng chuẩn: 338)

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: gritc fmt trên 398 mẫu pass: bị từ chối 0; không lũy đẳng 0; (mẫu gốc chưa ở dạng chuẩn: 338)
```
</details>


### G06_determinism — [bản sao] gritc build hai lần ⇒ cùng băm (D9)

- **Trạng thái:** PASS (mã thoát 0, 4s, bắt đầu 2026-10-20T11:44:48Z)
- tính tất định (D9): 50 mẫu dựng hai lần; khác nhau 0; không dựng được 33

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: tính tất định (D9): 50 mẫu dựng hai lần; khác nhau 0; không dựng được 33
```
</details>


### G07_explain — [bản sao] gritc explain cho mọi mã trong KNOWN

- **Trạng thái:** FAIL (mã thoát 1, 0s, bắt đầu 2026-10-20T11:44:52Z)
- gritc explain: 72 mã trong KNOWN (code.rs); lỗi 6

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
explain E1201 LỖI
explain E1202 LỖI
explain E1203 LỖI
explain E1204 LỖI
explain E1301 LỖI
explain E9901 LỖI
SUMMARY: gritc explain: 72 mã trong KNOWN (code.rs); lỗi 6
```
</details>


**Ghi chú của người phân tích:**

- 66/72 mã có `gritc explain`; 6 mã lỗi vì **thiếu `docs/errors/<mã>.md`** (bảng `EXPLAIN` được nhúng từ thư mục đó lúc biên dịch): `E1201–E1204` (Unicode), `E1301` (fmt) và **`E9901` (ICE — mã người dùng cần nhất khi gritc hỏng)**. Liên quan B05, ANALYSIS F-08.


### G08_probes — [bản sao] ~110 chương trình THỬ TẤN CÔNG tự viết (audit/probes): tìm âm tính giả

- **Trạng thái:** FAIL (mã thoát 1, 4s, bắt đầu 2026-10-20T12:10:15Z)
- 181 probe tự viết; FALSE_NEGATIVE=23; KNOWN_GAP=3; OK=155

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
FALSE_NEGATIVE  m6_same_stmt_call_args_double_move            kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_cap_duplicated                   kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_generic_dup                      kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_method_self_and_arg              kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_struct_lit_double_move           kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_tuple_double_move                kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_tuple_use_after_move             kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_unique_ref_duplicated            kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_use_after_move_in_args           kỳ vọng=reject E0401                 verdict=green E=[] S=[]
KNOWN_GAP       secret_evasion_split_consts                   kỳ vọng=warn S101                    verdict=green E=[] S=[]
KNOWN_GAP       secret_evasion_unicode_escape                 kỳ vọng=warn S101                    verdict=green E=[] S=[]
FALSE_NEGATIVE  ty_alias_cycle                                kỳ vọng=reject                       verdict=green E=[] S=[]
FALSE_NEGATIVE  type_recursive_enum                           kỳ vọng=reject                       verdict=green E=[] S=[]
FALSE_NEGATIVE  type_recursive_struct                         kỳ vọng=reject                       verdict=green E=[] S=[]
FALSE_NEGATIVE  type_recursive_struct_used                    kỳ vọng=reject                       verdict=green E=[] S=[]
FALSE_NEGATIVE  type_slice_field_by_value                     kỳ vọng=reject E0301                 verdict=green E=[] S=[]
FALSE_NEGATIVE  type_slice_param_by_value                     kỳ vọng=reject E0301                 verdict=green E=[] S=[]
FALSE_NEGATIVE  type_str_param_by_value                       kỳ vọng=reject E0301                 verdict=green E=[] S=[]
```
</details>


**Ghi chú của người phân tích:**

- **FAIL ở đây nghĩa là tìm thấy lỗ hổng**, không phải chạy hỏng. Số `FALSE_NEGATIVE` (≈23) gom theo nguyên nhân: nhóm `m6_same_stmt_*`/`m6_dup_*` (14 tệp) = **F-01**; `type_recursive_*` (3) = F-03; `type_slice_*`/`type_str_*` (3) = F-04; `cap_fnptr_launder` (1) = F-05; `d4_float_in_match_literal` (1) = F-09/D4; `ty_alias_cycle` (1) = vô hại. 3 `KNOWN_GAP` là giới hạn dự án đã tự ghi (Cyrillic thuần, 2 né quét secret). Phần còn lại (~150) khớp kỳ vọng — danh sách ở ANALYSIS §2 cuối.
- Để tái hiện riêng từng lỗi: `gritc check audit/probes/<tên>.gt` trên bản sao (`$AUDIT_WORK/grit/target/debug/gritc`).


### G09_diff_checks — [bản sao] vi sai M7: --checks=all ≡ --checks=proven trên số học ngẫu nhiên (bộ sinh riêng)

- **Trạng thái:** PASS (mã thoát 0, 407s, bắt đầu 2026-10-20T11:50:15Z)
- vi sai [--backend=interpreter --checks=all] ≡ [--backend=interpreter --checks=proven]: programs=3000; rejected=1228; runs=10632; aborts=4017; diffs=0; ice=0; seed=5

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: vi sai [--backend=interpreter --checks=all] ≡ [--backend=interpreter --checks=proven]: programs=3000; rejected=1228; runs=10632; aborts=4017; diffs=0; ice=0; seed=5
```
</details>


### G10_mutfuzz — [bản sao] fuzz đột biến token qua CLI gritc (ICE/crash/treo)

- **Trạng thái:** FAIL (mã thoát 1, 122s, bắt đầu 2026-10-20T11:57:02Z)
- fuzz đột biến token qua CLI gritc: 8000 mẫu, seed 11; ICE=1; OK_GREEN=2068; OK_RED=5931
- ICE x1: lỗi nội bộ của compiler (ICE): MIR của `m_NaNfNfaN::pass_through` không hợp lệ:

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: fuzz đột biến token qua CLI gritc: 8000 mẫu, seed 11; ICE=1; OK_GREEN=2068; OK_RED=5931
SUMMARY: ICE x1: lỗi nội bộ của compiler (ICE): MIR của `m_NaNfNfaN::pass_through` không hợp lệ: 
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


**Ghi chú của người phân tích:**

- **Port Linux THỬ NGHIỆM thành công trên bản sao (5 tệp, +62/−26 dòng; `audit/patches/linux_port.patch`)**: `gt-llvm/build.rs` liên kết `libLLVM-20.so` của hệ thống (gói `libllvm20` 20.1.2 — KHÔNG phải 20.1.8 mà dự án ghim), `gt-driver/build.rs`+`rig/build.rs` bỏ cờ `/DELAYLOAD`, `gt-driver/src/native.rs` liên kết bằng `cc -no-pie` thay `link.exe`, `gt-rt` bỏ `compile_error!` và thay `SRWLOCK` bằng khoá quay trên ô 8 byte. Không đổi dòng nào của logic kiểm tra/codegen.
- Hệ quả: `gritc build/run` sinh **exe ELF thật** (chương trình mẫu trả 45) — tức là toàn bộ backend LLVM + IR Guard + gt-rt chạy được ở đây qua bản sao, nên các giai đoạn L02–L07 có bằng chứng native thật (khác BLOCKED của D01/E01 trên mã nguyên trạng).


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


**Ghi chú của người phân tích:**

- **135 test đạt, 5 lỗi — cả 5 là lỗi của chính cổng Windows, không phải lỗi codegen**: `built_exe_does_not_depend_on_llvm_dll` (gọi `dumpbin.exe` của MSVC), `check_runs_without_llvm_on_path` (kiểm `LLVM-C.dll` trên PATH — khái niệm Windows), và 3 test `golden_ir_*`.
- **Bằng chứng mạnh về tính tất định/di động của codegen**: chạy lại 3 test golden với `GT_UPDATE_GOLDEN=1` trong bản sao rồi `git diff tests/golden_ll` ⇒ IR sinh trên Linux (libLLVM 20.1.2) **giống từng byte** golden sinh trên Windows (LLVM 20.1.8), CHỈ khác đúng hai dòng `target datalayout` và `target triple` ở mỗi tệp (6 dòng đổi cho 3 tệp). Cả băm attestation trong IR cũng khớp.
- Gồm các bộ khó nhất của dự án chạy với exe native thật: `llvm_diff*` (so thông dịch ≡ exe), `release_*` (dev ≡ release, Guard-post), `threads_repeat`, `vec_overflow`, `utf8_diff`, `arena_dynamic`, `fs_read_io_error`, `std_lib`, `release_policy` (tái lập bản dựng).


### L03_native_rig_tests — [bản sao 2] cargo test -p rig (rig build/run/test + 6 công cụ dogfood bằng exe native)

- **Trạng thái:** PASS (mã thoát 0, 32s, bắt đầu 2026-10-20T11:48:35Z)
- **cargo test:** 47 đạt / 0 lỗi / 3 ignore trong 10 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
test failing_test_exits_1_and_names_the_abort ... ok
test fmt_rewrites_and_check_flags_unformatted_sources ... ok
test lock_missing_is_created_stale_is_always_refused_and_locked_flag_is_strict ... ok
test manifest_errors_are_fail_closed_with_clear_messages ... ok
test module_name_collisions_and_prefix_rule_are_loud_errors ... ok
test new_check_build_run_roundtrip ... ok
test builds_are_reproducible_byte_for_byte ... ok
test three_package_project_builds_runs_and_tests_with_diamond_loaded_once ... ok
test option_ord_in_another_package_uses_reference_patterns_passes_e0703_and_runs_correctly ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.84s

   Doc-tests rig

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```
</details>


**Ghi chú của người phân tích:**

- **47 test đạt, 0 lỗi** với backend native thật (bản sao có port Linux): `rig new/check/build/run/test/add/lock/fmt/clean`, dự án 3 gói, tái lập byte-for-byte, và cả **6 công cụ dogfood** (`wc`, `json`, `sha256`, `sortlines`, `wordfreq`, `stats` viết bằng Grit, dựng bằng `rig build`, chạy exe ELF) khớp bản tham chiếu độc lập (NIST vector + `hashlib` cho sha256, `serde_json` cho json, mô hình Rust cho phần còn lại). 3 test `ignored` là bench hiệu năng (`--ignored`).
- Lưu ý: kết quả này là của **bản port** (LLVM 20.1.2, Linux, liên kết `cc`), không phải môi trường mà dự án nhắm tới; nó xác nhận LOGIC của backend và công cụ, không xác nhận các tính chất riêng Windows (MSVC, `LLVM-C.dll`, quét virus).


### L04_native_other_crates — [bản sao 2] cargo test -p gt-guard -p gt-codegen-llvm -p gt-llvm + gt-rt

- **Trạng thái:** PASS (mã thoát 0, 1s, bắt đầu 2026-10-20T11:49:07Z)
- **cargo test:** 39 đạt / 0 lỗi / 0 ignore trong 9 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests gt_guard

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests gt_llvm

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
</details>


**Ghi chú của người phân tích:**

- `gt-guard` (IR Guard), `gt-codegen-llvm`, `gt-llvm` và `gt-rt` (6 test) đều đạt trên bản port — gồm các test IR dựng tay của Guard và các lỗi chèn.


### L05_native_diff — [bản sao 2] vi sai thông dịch ≡ exe native trên số học ngẫu nhiên (bộ sinh riêng)

- **Trạng thái:** PASS (mã thoát 0, 229s, bắt đầu 2026-10-20T11:49:08Z)
- vi sai [--backend=interpreter] ≡ [--backend=llvm]: programs=600; rejected=252; runs=2088; aborts=767; diffs=0; ice=0; seed=3

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: vi sai [--backend=interpreter] ≡ [--backend=llvm]: programs=600; rejected=252; runs=2088; aborts=767; diffs=0; ice=0; seed=3
```
</details>


### L06_native_unsound_demo — [bản sao 2] chạy THẬT chương trình XANH nhưng dùng-sau-reset Arena (nhân đôi tay cầm): có đọc nhầm không

- **Trạng thái:** FAIL (mã thoát 1, 0s, bắt đầu 2026-10-20T11:45:33Z)
- kỳ vọng đúng ngữ nghĩa: main trả 1234 (giá trị đã cấp); nhận giá trị khác = bộ nhớ đã bị tái dùng sau reset (UB) trong chương trình được gritc cho qua

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
gritc check: exit=0 (gritc: phán quyết green; 0 lỗi, 0 cảnh báo, 0 ghi chú; backend: interpreter)
profile=dev: chạy xong (backend: llvm); main trả về 99
profile=release: chạy xong (backend: llvm); main trả về 99
profile=release-o2: chạy xong (backend: llvm); main trả về 99
SUMMARY: kỳ vọng đúng ngữ nghĩa: main trả 1234 (giá trị đã cấp); nhận giá trị khác = bộ nhớ đã bị tái dùng sau reset (UB) trong chương trình được gritc cho qua
```
</details>


### L07_native_stack_limits — [bản sao 2] chương trình HỢP LỆ gây tràn ngăn xếp native: có abort có kiểm soát (mã 70) không

- **Trạng thái:** FAIL (mã thoát 1, 1s, bắt đầu 2026-10-20T11:47:48Z)
- D3 đòi abort có kiểm soát (70) khi lỗi không khôi phục; nhận mã khác 0/70/71 = tiến trình bị HĐH giết (tràn ngăn xếp), gt-rt không có bảo vệ

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
gritc: phán quyết green; 0 lỗi, 0 cảnh báo, 0 ghi chú; backend: interpreter
stack_array: exit=2 :: gritc: lỗi nội bộ khi chạy: chương trình bị hệ điều hành dừng
gritc: phán quyết green; 0 lỗi, 0 cảnh báo, 1 ghi chú; backend: interpreter
stack_recursion: exit=2 :: gritc: lỗi nội bộ khi chạy: chương trình bị hệ điều hành dừng
SUMMARY: D3 đòi abort có kiểm soát (70) khi lỗi không khôi phục; nhận mã khác 0/70/71 = tiến trình bị HĐH giết (tràn ngăn xếp), gt-rt không có bảo vệ
```
</details>


### L08_native_compiler_crash — [bản sao 2] đầu vào được M4–M9 cho qua nhưng làm CHÍNH gritc sập (kiểu kích thước vô hạn…)

- **Trạng thái:** FAIL (mã thoát 1, 0s, bắt đầu 2026-10-20T11:49:57Z)
- mã thoát hợp lệ của gritc là 0/1/2/3 (D17); 134/139 = tiến trình gritc tự sập (SIGABRT/SIGSEGV)

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
/home/user/Grit/audit/run_all.sh: line 397: 19095 Aborted                 timeout 120 "$g" run "$f" > "$WORK/crash.out" 2>&1
crash_recursive_type.gt: exit=134 :: thread 'main' (19101) has overflowed its stack fatal runtime error: stack overflow, aborting 
SUMMARY: mã thoát hợp lệ của gritc là 0/1/2/3 (D17); 134/139 = tiến trình gritc tự sập (SIGABRT/SIGSEGV)
```
</details>


### G11_fmt_meta — [bản sao] metamorphic: gritc fmt không đổi phán quyết (841 mẫu)

- **Trạng thái:** PASS (mã thoát 0, 65s, bắt đầu 2026-10-20T11:57:41Z)
- fmt metamorphic: 841 mẫu; phán quyết không đổi 816; fmt từ chối 25; ĐỔI 0

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: fmt metamorphic: 841 mẫu; phán quyết không đổi 816; fmt từ chối 25; ĐỔI 0
```
</details>


### G12_alias_meta — [bản sao] metamorphic: alias ASCII ≡ canonical không đổi phán quyết (816 mẫu)

- **Trạng thái:** PASS (mã thoát 0, 56s, bắt đầu 2026-10-20T11:58:46Z)
- alias ASCII ≡ canonical: 816 mẫu; giống 816; KHÁC 0; bỏ qua 25 (lỗi cú pháp/Unicode cố ý)

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: alias ASCII ≡ canonical: 816 mẫu; giống 816; KHÁC 0; bỏ qua 25 (lỗi cú pháp/Unicode cố ý)
```
</details>


### G13_order_meta — [bản sao] metamorphic: xáo thứ tự khai báo không đổi phán quyết (429 mẫu × 2)

- **Trạng thái:** PASS (mã thoát 0, 47s, bắt đầu 2026-10-20T12:01:20Z)
- order metamorphic: 429 mẫu (2 hoán vị mỗi mẫu); giống 429; KHÁC 0; bỏ qua 412

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: order metamorphic: 429 mẫu (2 hoán vị mỗi mẫu); giống 429; KHÁC 0; bỏ qua 412
```
</details>


### G14_diff_arrays — [bản sao] vi sai M7 all≡proven trên chỉ số mảng/lát (nơi M7 xoá kiểm tra biên)

- **Trạng thái:** PASS (mã thoát 0, 206s, bắt đầu 2026-10-20T12:02:07Z)
- vi sai [--backend=interpreter --checks=all] ≡ [--backend=interpreter --checks=proven]: programs=2500; rejected=1495; runs=6030; aborts=2481; diffs=0; ice=0; seed=9

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: vi sai [--backend=interpreter --checks=all] ≡ [--backend=interpreter --checks=proven]: programs=2500; rejected=1495; runs=6030; aborts=2481; diffs=0; ice=0; seed=9
```
</details>


### P01_proposed_fix_f01 — [bản sao 3] ĐỀ XUẤT sửa F-01 (9 dòng ở gt-borrowck/check.rs): 14 probe có bị chặn không, corpus/test có hồi quy không

- **Trạng thái:** PASS (mã thoát 0, 240s, bắt đầu 2026-10-20T12:05:34Z)
- 181 probe tự viết; FALSE_NEGATIVE=9; KNOWN_GAP=3; OK=169
- corpus qua dòng lệnh gritc: 855 ca; khớp 855; lệch 0
- fail:OK = 438
- fail:OK_EXTRA = 2
- đề xuất sửa F-01: ĐẠT — 14 probe bị chặn, corpus 855/855, gt-borrowck xanh
- **cargo test:** 57 đạt / 0 lỗi / 0 ignore trong 11 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: 181 probe tự viết; FALSE_NEGATIVE=9; KNOWN_GAP=3; OK=169
== corpus qua CLI (kỳ vọng 855/855)
SUMMARY: corpus qua dòng lệnh gritc: 855 ca; khớp 855; lệch 0
SUMMARY: fail:OK = 438
SUMMARY: fail:OK_EXTRA = 2
== cargo test -p gt-borrowck (kỳ vọng không hồi quy)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.98s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.72s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 205.82s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
SUMMARY: đề xuất sửa F-01: ĐẠT — 14 probe bị chặn, corpus 855/855, gt-borrowck xanh
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


### I01_fuzz_tools — Công cụ fuzz: nightly + cargo-fuzz (cài nếu thiếu, ngoài dự án)

- **Trạng thái:** PASS (mã thoát 0, 29s, bắt đầu 2026-10-20T11:50:37Z)
- nightly + cargo-fuzz sẵn sàng

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
   Compiling fastrand v2.4.1
   Compiling once_cell v1.21.4
   Compiling toml_writer v1.1.1+spec-1.1.0
   Compiling either v1.16.0
   Compiling rayon v1.12.0
   Compiling toml v1.1.2+spec-1.1.0
   Compiling tempfile v3.27.0
   Compiling clap v4.6.1
   Compiling cargo_metadata v0.23.1
   Compiling rustc_version v0.4.1
   Compiling num_cpus v1.17.0
   Compiling cargo-fuzz v0.13.2
    Finished `release` profile [optimized] target(s) in 28.51s
  Installing /root/.cargo/bin/cargo-fuzz
   Installed package `cargo-fuzz v0.13.2` (executable `cargo-fuzz`)
cargo 1.101.0-nightly (3960fa69b 2026-10-12)
cargo-fuzz 0.13.2
SUMMARY: nightly + cargo-fuzz sẵn sàng
```
</details>


### I02_fuzz_lex — [bản sao] cargo-fuzz lexer 300s (ASan, libFuzzer)

- **Trạng thái:** PASS (mã thoát 0, 317s, bắt đầu 2026-10-20T11:51:06Z)
- fuzz lex: 300s, thoát 0, 0 tệp artifact (crash/timeout/oom) — chép vào audit/results/fuzz_artifacts/lex nếu có

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
"\000\000\000\001" # Uses: 50
".+" # Uses: 57
"\000\021\000\000" # Uses: 52
"\022\000\000\000\000\000\000\000" # Uses: 30
"\007\000\000\000\000\000\000\000" # Uses: 31
"X\000\000\000" # Uses: 21
"e\000\000\000" # Uses: 26
"\000\000\000<" # Uses: 23
"\005\375" # Uses: 24
"ze\314\256" # Uses: 15
"\000&" # Uses: 10
"\001w" # Uses: 7
"\000\000\000\000\000\000\000f" # Uses: 2
"\377\377\377\377\377\377\377\012" # Uses: 4
"pu3" # Uses: 0
###### End of recommended dictionary. ######
Done 2667202 runs in 301 second(s)
SUMMARY: fuzz lex: 300s, thoát 0, 0 tệp artifact (crash/timeout/oom) — chép vào audit/results/fuzz_artifacts/lex nếu có
```
</details>


### I03_fuzz_parse — [bản sao] cargo-fuzz parser 300s (ASan, libFuzzer)

- **Trạng thái:** PASS (mã thoát 0, 315s, bắt đầu 2026-10-20T11:56:23Z)
- fuzz parse: 300s, thoát 0, 0 tệp artifact (crash/timeout/oom) — chép vào audit/results/fuzz_artifacts/parse nếu có

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
"vo2v" # Uses: 47
"\020\000\000\000\000\000\000\000" # Uses: 28
".\022=" # Uses: 43
"\007\000\000\000\000\000\000\000" # Uses: 33
"\000\000\000\000\000\000\000\025" # Uses: 43
"24A" # Uses: 42
"mo" # Uses: 29
".1#" # Uses: 31
"\027\000" # Uses: 28
".\000\000\000\000\000\000\000" # Uses: 13
"4\000\000\000\000\000\000\000" # Uses: 13
"./" # Uses: 20
"mn" # Uses: 12
"(\000\000\000" # Uses: 7
"\010\000\000\000" # Uses: 5
###### End of recommended dictionary. ######
Done 544794 runs in 301 second(s)
SUMMARY: fuzz parse: 300s, thoát 0, 0 tệp artifact (crash/timeout/oom) — chép vào audit/results/fuzz_artifacts/parse nếu có
```
</details>


