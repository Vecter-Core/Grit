# PROGRESS

## M0 — Đặc tả & nền móng — **HOÀN THÀNH (đặc tả FINAL, Q1–Q14 đóng)**; commit "M0: finalize spec — close Q7-Q14"
Cập nhật: 2026-10-05 (sau khi Giang trả lời Q1–Q6).

### Đã làm (có thật, đã kiểm)
- **Workspace:** `Cargo.toml` (`resolver = "3"`, edition 2024). Kiểm bằng cargo thật: `cargo 1.98.1 (797e8a9bc 2026-08-05)`, `rustc 1.98.1 (48a229cea 2026-09-01)`; `cargo metadata` chạy được; đối chứng `resolver = "9"` bị cargo từ chối với "valid options are "1", "2" or "3"".
- **Đặc tả:** `docs/spec/{grammar.ebnf, symbols.md, semantics.md, error_codes.md}` (45 mã). Đã cập nhật theo Q2 (bỏ `⊤ ⊥`) và Q5 (`μ` = `MUT`, semantics §12).
- **Sổ quyết định:** `DECISIONS.md` (ADR-001..005), `OPEN_QUESTIONS.md` (Q1–Q6 đánh dấu "đã quyết", Q3 có điều kiện), `KNOWN_LIMITATIONS.md`.
- **Đo token:** `tools/measure_tokens.py` (proxy tiktoken); số liệu trong ADR-002.
- **Corpus ban đầu 50 file .gt (nay 59, xem bên dưới).** Mới: `tests/lexer/mu_split.{gt,tokens}` (`μs` → `MUT`,`IDENT(s)`), `tests/fail/E0101_mu_as_ident.gt`.
- **`tools/check_expectations.py` + `tests/README.md`** (viết thật, `tests/TASK.md` đã xoá). Kiểm: đúng 1 dòng `// expect:` ở dòng đầu; thư mục khớp loại; tên file khớp mã; mã có trong `error_codes.md`; mỗi mẫu fail/warn được liệt kê trong bảng và mọi tham chiếu trong bảng trỏ tới file có thật; Unicode (không bidi/zero-width/non-NFC ngoài 4 mẫu E1201–E1204, và 4 mẫu đó phải thật sự chứa ký tự mục tiêu); `.tokens` tồn tại. Kết quả chạy thật: 50 file lúc đó, 0 lỗi, exit 0 (hiện 59 file, 0 lỗi). Đã kiểm ngược bằng 7 đột biến (bidi lẻn vào mẫu pass, sai mã, mã không tồn tại, file mồ côi, thiếu .tokens, homoglyph mất ký tự mục tiêu, sai thư mục): cả 7 bị bắt. Trong lúc làm, script bắt được 1 lỗi thật của tài liệu (ô E0204 trong bảng).
- **Git:** repo đã được Giang reset về 1 commit `4c9b489 init: clean workspace for Grit` (seed cũ không còn). Thay đổi sau commit đó chưa commit.

### Vòng bổ sung sau khi Giang duyệt M0★ (2026-10-05)
- Duyệt 13 ký hiệu M0★ (ADR-006); `*`, `self`, `as` giữ ASCII.
- **Đã thêm** vào `symbols.md` + `grammar.ebnf`: `≡`/`\type`, `□`/`\const`, `■`/`\static`, `\extern` (dành riêng), lifetime `'a` (ASCII-only, thay `@r`), từ khoá cứng `self as true false for`, quy tắc `block_stmt`.
- **semantics.md** §13 (từ khoá cứng), §14 (`*` prefix vs infix), §15 (`'a` vs `'a'`), §16 (`≡ □ ■`).
- **Mẫu mới (+9 file, tổng 59):** pass `19_deref_vs_mul`, `20_const_static_type`, `21_lifetime_named` (và `15_arena_region` đổi sang `'r`); lexer `star_ctx`, `quote_ctx` (+`.tokens`); fail `E0101_star_dangling`, `E0101_self_as_var`, `E0101_self_as_var_ascii`, `E0101_extern_reserved`. `check_expectations.py`: 59 file, 0 lỗi.
- (Q7, Q8, Q9 sau đó đã được duyệt — xem "Vòng duyệt cuối".)
- Trong lúc làm phát hiện & sửa 2 lỗi do shell nuốt backslash (`\type` thành TAB trong `measure_tokens.py`; `'\n'` thành xuống dòng trong semantics; cả `DECISIONS.md` và chính dòng này) — đã sửa và quét toàn bộ docs/tests/tools không còn ký tự điều khiển lạc.

### Vòng duyệt cuối (2026-10-05): Q7–Q9, comment, 16 mục bổ sung
- **Đóng:** Q7 (inline `<T: A + B>`, dành riêng `\where`), Q8 (`;` kiểu Rust + `block_stmt`), Q9 (toán tử bit = hàm `lib/core`), `■` bất biến, tên token tạm (ADR-007).
- **Grammar viết lại đầy đủ** (`grammar.ebnf`): literal số (hex/octal/bin/`_`/hậu tố, i128/u128), char vs lifetime, escape, `//` `///` (DOC_COMMENT) và cấm `/* */`,
  thuộc tính `#`, tuple/unit/1-tuple, array/slice, range `..` `..=`, struct literal + field/tuple index, pattern (enum/struct/tuple/wildcard),
  `Self` từ khoá cứng, cast tách khỏi unary, `expr_ns`, kiểu hàm và con trỏ thô (hai thứ này đánh dấu chờ duyệt).
- **semantics.md:** §1 (primitive chính thức), §3 (khối/`;`, struct trong điều kiện, vòng lặp), §13 (từ khoá/`\where`), §15 (char vs lifetime, thuật toán + ví dụ),
  §17 (`abort`), §18 (`spawn`, atomic ordering là enum), §19 (lexer literal/comment), §20 (range/tuple/struct/pattern/con trỏ thô).
- **KNOWN_LIMITATIONS.md:** thêm mục "Hạn chế chủ ý" (bit-op, không `+=`, không `/* */`, closure/variadic/cfg/…, con trỏ thô, range, pattern).
- **Mở (không tự chốt):** Q10 for-in (đề xuất `in` ASCII), Q11 while (đề xuất giữ `⟳ cond`), Q12 never + kiểu hàm, Q13 con trỏ thô (`*□ T` vs `*const T`), Q14 (14 điểm agent tự chọn).
- **Mẫu mới (+16 file, tổng 75):** pass `22_comments`, `23_literals`, `24_tuple_array_range`, `25_struct_pattern`; lexer `comments`, `num_literals`, `lifetime_vs_char`, `tuple_index`;
  fail `E0101_{block_comment,where_reserved,compound_assign,char_multi,doc_comment_dangling,selftype_as_var}`, `E0307_trailing_semicolon`, `E0601_literal_out_of_range`.
- **`check_expectations.py`:** 75 file, 0 lỗi, exit 0. Thêm kiểm va chạm tên không phân biệt hoa/thường (phát hiện từ sự cố thật: `E0101_Self_as_var.gt` đã ghi đè `E0101_self_as_var.gt` trên Windows; đã khôi phục, đổi tên thành `E0101_selftype_as_var.gt`). Kiểm này chưa được thử bằng đột biến (Windows không tạo được hai tên chỉ khác hoa/thường).
- **Không làm theo chỉ thị trước:** commit "M0: finalize spec…" và M1 — tin nhắn mới nhất yêu cầu dừng, chưa commit, chưa sang M1. Danh sách "#18–#21, #24–#25" không có trong tin nhắn nào tôi nhận được (Q14.12).

### Vòng cuối: Q10–Q14 (2026-10-05)
- **Đóng Q10–Q14** (ADR-008): for-in `⟳ x in xs`, while `⟳ cond`, never `!` + `#noreturn` + kiểu hàm, con trỏ thô `*□ T`/`*μ T` (ngoài `⚠` → E1001), range mở `a.. ..b ..`.
- **grammar.ebnf:** nhánh for-in, `in` từ khoá cứng, `!` trong prim, range mở, pattern range/rest. **symbols.md:** `in`, `!`, con trỏ thô, `#noreturn`, range mở. **semantics.md:** §21 termination, §22 never, §23 con trỏ thô.
- **Cách hiểu của agent cần Giang để mắt (ADR-008):** "while suy từ cond" = suy biến thể từ dạng mẫu, không suy ra được vẫn E0701 (nên viết lại `E0701_loop_no_variant`); `#noreturn` ↔ `→ !`; §7 → §21.
- **Mẫu mới:** pass `26_for_in_range`, `27_for_in_slice`, `28_while_cond`, `29_range_open_patterns`, `30_never_abort` (+`24` có slice mở); fail `E0101_in_as_var`, `E1001_raw_ptr_outside_unsafe`, `E0301_open_range_standalone`, `E0701_infinite_no_div` (+ viết lại `E0701_loop_no_variant`); lexer `for_in_tokens`.
- **check_expectations.py: 85 file (pass 30, fail 46, warn 1, lexer 8), 0 lỗi, exit 0.**

### Hoãn sang AI khác (đã được duyệt, Q6) — CHƯA làm
- `.github/workflows/TASK.md` (CI) · `deny.toml.TASK.md` · `docs/errors/TASK.md` (45 file `Exxxx.md`) · `crates/TASK.md`, `runtime/gt-rt/TASK.md`, `lib/core/TASK.md`, `tests/{golden,fuzz,bench}/TASK.md` (chỉ ghi chú, đúng phạm vi M0).
- Lưu ý: plan §0.5 đòi tài liệu cho mỗi mã lỗi — việc hoãn `docs/errors` phải hoàn tất trước khi mã tương ứng được triển khai.

### Còn thiếu / rủi ro
- Không còn câu hỏi mở của M0. Tiêu chí plan "Giang duyệt đặc tả" **chưa đạt** cho tới đó.
- Token chỉ là proxy tiktoken.
- Corpus chưa được parse bởi parser nào (M2); tên token trong `.tokens` là tạm (M1 chốt).
- Hệ quả Q5 cần Giang để ý: mọi `μ` là `MUT` (kể cả giữa chữ, `aμb`) — mở rộng hợp lý của "không gộp vào định danh", chặt hơn chữ "cấm bắt đầu bằng μ".
- Không chạy clippy/fmt/test: chưa có crate Rust nào (đúng phạm vi M0).

## M1 — Lexer (hai bề mặt) — **HOÀN THÀNH các cổng đã thoả thuận; chờ Giang xác nhận** (tên token đã chốt, ADR-010)
Cập nhật: 2026-10-05.

### Đã làm (có thật)
- **Crate:** `gt-span` (Span, SourceFile, LineCol — dòng/cột theo Unicode scalar, CRLF) rồi `gt-lexer` (Token/TokenKind, ErrorCode, lex, render_tokens), thêm vào workspace; `#![forbid(unsafe_code)]`, `[lints] workspace = true`.
- **TDD thật:** viết test trước, chạy đỏ (`unresolved imports gt_span::…` / `gt_lexer::{ErrorCode, lex, render_tokens}`), rồi mới cài đặt. Bốn test yêu cầu — `round_trip_alias_canonical`, `mu_split`, `lifetime_vs_char`, `for_in_tokens` (`crates/gt-lexer/tests/m1_first.rs`) — **xanh**. Bảng alias trong test là oracle độc lập với bảng trong lexer; round-trip còn so cả file thật `tests/pass/01_add.gt` với `11_ascii_alias.gt`.
- **Test bổ sung (cả workspace: 21 test xanh = 15 của gt-lexer + 6 của gt-span):** `details.rs` (số, chuỗi, comment, toán tử khớp dài nhất, CRLF/khoảng trắng, Unicode nguy hiểm, span cắt lại đúng nguồn, 40 000 chuỗi giả ngẫu nhiên không panic) và `corpus.rs` (chạy lexer trên **toàn bộ** `tests/`: 8 file `.tokens` khớp; mọi file `pass`/`warn` lex sạch; mọi file `fail` E1201–E1204 bị chặn đúng mã, 3 file E0101 mức token bị chặn, các file lỗi tầng sau không bị lexer báo nhầm). `gt-span`: 6 test.
- **Kiểm tra test không "xanh rỗng":** 6 đột biến thủ công trong lexer (char→lifetime, alias `\fn`, chỉ số tuple, bidi, trộn bảng chữ, `/*`) đều làm test đỏ; hoàn nguyên đã kiểm. (Lần đầu 3 đột biến "sống sót" vì 4 test đầu không phủ chúng → thêm `details`/`corpus`.)
- **Chất lượng:** `cargo fmt --check` sạch, `cargo clippy --workspace --all-targets -- -D warnings` sạch, `cargo audit` sạch (6 crate). `cargo deny` chưa chạy (chưa có `deny.toml` — việc đã hoãn cho AI khác).
- **Fuzz:** `cargo-fuzz` 0.13.2 + nightly, target `lex` (không panic, span hợp lệ/đơn điệu, kết thúc `Eof`, xác định). Phiên 1 giờ liên tục bị ASan hết bộ nhớ nội bộ (OOM công cụ, artifact rỗng, không tái hiện). Chạy lại bằng `tools/fuzz_lexer.sh` (quarantine ASan 8 MB, corpus giữ giữa phiên): **6 phiên × 10 phút, 0 crash**, 606–784 nghìn lượt mỗi phiên (tổng ~4,08 triệu lượt, ~60 phút).
- **leak_check:** 1 triệu lần lex, bộ nhớ riêng tăng 36 KB (ngưỡng 10 MB); đã kiểm độ nhạy bằng rò rỉ cố ý (+751 MB → đỏ) và test riêng chứng minh phép đo thấy mức tăng 64 MB.
- **Tên token chính thức (ADR-010)** áp dụng; thêm token `UNKNOWN`; test `token_names.rs` khoá bộ tên. **Kết quả cuối:** 26 test xanh (gt-lexer 20, gt-span 6), fmt/clippy/audit sạch, `check_expectations.py` exit 0 (85 file).
- **Tài liệu:** semantics §24 (xuống dòng/ký tự điều khiển/alias), ADR-009, KNOWN_LIMITATIONS "Lexer (M1)", sửa §19 (thiếu `\\` trong danh sách escape), `.gitattributes` (LF).

### Còn thiếu cho "Hoàn thành khi" của M1 (plan §6.2)
- **Fuzz ≥1 giờ: ĐẠT dưới dạng 6×10 phút** (không phải 1 phiên liên tục vì giới hạn ASan trên Windows). Cổng plan §7 còn đòi ≥24 giờ tích luỹ trước Seed 1.0 — chưa đạt, ngoài phạm vi M1.
- **"Mọi ký tự cấm có test fail":** đã có E1201–E1204; danh sách vô hình là hữu hạn (xem KNOWN_LIMITATIONS).
- **Chờ Giang:** chốt tên token; xác nhận cách hiểu D15 trong ADR-009 (zero-width cấm cả trong chuỗi/comment); duyệt trước khi sang phần tiếp theo của M1.
- **Rủi ro mới:** homoglyph thuần một bảng chữ chưa phát hiện; `cargo-fuzz` Windows cần PATH tới ASan DLL (đã ghi ADR-009).
