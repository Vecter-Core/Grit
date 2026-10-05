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

## M2 — Parser + AST + Diagnostics — **gt-diag (`599452e`), gt-ast (`9505469`), gt-parser (`aad181d`), gt-fmt xong (chưa commit); dừng chờ Giang duyệt (E1301, Q16), chưa sang M3**
Cập nhật: 2026-10-05. M1 đã commit `4c3c054`.

### Bước 1: gt-diag (có thật)
- Crate `gt-diag` (thêm vào workspace): `Code`/`Level` (D16), `Diagnostic` + `Label` + `Suggestion`/`Edit`/`Applicability`, `apply_edits` (nguyên tử, từ chối chồng/ngoài tệp/giữa ký tự), `render_human` (rustc-style, màu tuỳ chọn), `Diagnostics`/`Report` (sắp xếp tất định, bỏ trùng, giới hạn, ICE không bị cắt, exit 0/1/2, `--deny-yellow`), `Report::to_json` (schema v1).
- **TDD:** test viết trước, đỏ (unresolved imports), rồi cài đặt. 32 test xanh: mã/mức, bộ mã khớp `docs/spec/error_codes.md`, bộ gom, exit code, edit, 12 golden render (span rỗng, EOF, nhiều dòng, span dài bị cắt `...`, CRLF/tab, nhãn phụ, ghi chú, gợi ý, ngoài tệp), JSON (mọi mức, span rỗng/nhiều dòng/ngoài tệp, tệp lạ, escape ký tự điều khiển/U+2028/emoji, cắt bớt, khoá phát ra == `properties` của schema, kiểm theo schema bằng bộ validator tối thiểu).
- **Đột biến:** 10 đột biến thủ công; 2 sống sót ở lần đầu (escape U+2028, span kết thúc đầu dòng) → thêm test, nay cả 10 bị bắt.
- **Cổng:** workspace 58 test xanh; fmt, clippy `-D warnings` (bắt 2 lỗi style thật, đã sửa), audit sạch (17 crate); `check_expectations.py` OK.
- **Schema:** `docs/spec/diag_schema.json` (draft 2020-12), version 1, quy tắc tương thích ghi trong schema; quyết định và 4 điểm lệch so với đặc tả ban đầu ở ADR-011.

### Chưa làm
- gt-ast, gt-parser; chuyển `LexError` → `Diagnostic` (làm ở gt-parser); `gritc explain`.
- `serde_json` chỉ là dev-dependency của gt-diag (không vào TCB sản phẩm).

### Bước 1b: sửa M1 — comment `//` thành trivia (ADR-012)
- **Kiểm tra theo yêu cầu:** gt-lexer M1 **bỏ hẳn** `//` (không phát gì) → gt-fmt (thư viện) sẽ xoá comment. Đã sửa: `LexOutput.trivia` (văn bản nguyên văn, span, `own_line`), doc comment `///` vẫn là token. Test mới `trivia.rs` (viết trước, đỏ; 4 test) gồm bất biến "mọi dòng có `//` trong corpus cho đúng một mục". Fuzz target + test độ bền kiểm cả trivia (92 815 lượt, sạch; **chưa chạy lại 6×10 phút** sau thay đổi này).
- Quy tắc gt-fmt gắn comment và nguyên tắc "không gắn được thì từ chối, không xoá" ghi ở KNOWN_LIMITATIONS; test fmt viết khi tới lượt gt-fmt.

### Bước 2: gt-ast (có thật)
- Crate `gt-ast` (chỉ phụ thuộc `gt-span`): `NodeId` (u32 dày đặc từ 0) + `NodeIdGen` (hết u32 → `None`), trait `Node {id, span}`, kiểu cho Item/Type/Expr/Stmt/Pattern/Block/Param/Generics/Effects/Contract/… theo `grammar.ebnf`; doc comment + attribute là node con; không có `Paren`; literal giữ văn bản nguyên.
- ADR-013 (trade-off Box vs arena, viết **trước** khi code): chọn cây Box + NodeId tuần tự tất định.
- 6 test (viết trước, đỏ): NodeId/overflow, cây dựng tay có id duy nhất+dày, docs/attrs là một phần của node, **mọi biến thể grammar dựng được và khớp mẫu vét cạn** (10 ItemKind, 10 TypeKind, 6 StmtKind, 8 PatternKind, 19 ExprKind, 3 LoopKind), contract/effects/generics/thành viên trait-impl, và test khoá "AST không biết dạng bề mặt" (chỉ phụ thuộc gt-span, mã nguồn không nhắc token/alias).
- **Cổng:** workspace 70 test xanh (gt-ast 6, gt-lexer 24, gt-diag 34, gt-span 6); fmt, clippy `-D warnings`, `check_expectations.py` OK.

### Chưa làm
- gt-parser (khôi phục lỗi ở `;`/`}`; adapter `LexError → Diagnostic` ở đây), gt-fmt, test round-trip alias↔canonical ở mức AST.

### Bước A (đột biến) — xong
- **gt-ast:** cargo-mutants (11 mutant: 7 bị bắt, 4 không biên dịch được) + 6 đột biến tay vào macro `impl_node`/`NodeIdGen`/`NodeId`: **6/6 bị bắt**. (Lần đầu một số kết quả không đáng tin vì lỗi liên kết `LNK1104`; script đột biến nay thử lại và chỉ tính "bị bắt" khi có test thật đỏ.)
- **Trivia (lexer):** 9 đột biến; **1 sống sót lần đầu** (span chứa cả ký tự CR cuối dòng của CRLF) → thêm test `crlf_comment_span_excludes_cr`; nay 9/9 bị bắt.

### Bước D: gt-parser (có thật)
- Crate `gt-parser` (recursive descent + Pratt): items, generics, effects, hợp đồng, kiểu, pattern, khối/câu lệnh, loops (while/for-in/vô hạn), match, unsafe, lượng từ; so sánh **không kết hợp**; `expr_ns`; `-` gấp vào literal (ADR-014); adapter `LexError → Diagnostic`; token `UNKNOWN` báo im lặng (lexer đã báo).
- **Khôi phục lỗi từ đầu:** đồng bộ ở `;`/`}` (câu lệnh), `,`/`}` (nhánh match), `}` đóng/từ khoá item (item); mỗi vòng có chốt tiến-lên. **Giới hạn độ sâu 256** gồm cả chuỗi phẳng, vượt → ĐỎ `E0102` (mã mới + `tests/fail/E0102_too_deep.gt`), không tràn stack.
- **Lỗi thật phát hiện khi test:** 256 mức vẫn tràn stack 2 MB ở bản debug (~9 frame/mức) → parser chạy trong luồng stack 64 MiB + bắt panic thành ICE `E9901` (ADR-014).
- **TDD:** test viết trước (đỏ vì chưa có API). 5 file test, 43 test: biểu thức (độ ưu tiên, kết hợp, âm, range, postfix, struct literal), câu lệnh/loop/match/pattern, item/kiểu, khôi phục lỗi + độ sâu + **30 000 chuỗi token ngẫu nhiên (công việc ≤ 300×độ dài, không ICE)** + cắt cụt chương trình ở mọi byte, và corpus.
- **Corpus bằng parser thật:** **30/30 file `pass` parse không lỗi** (sau khi chấp nhận Q15h cho mẫu 14); `warn` sạch; mọi file `fail` thuộc lexer/parser (E0101, E0102, E12xx — 17 file) ra **đúng mã**; 30 file `fail` thuộc milestone sau **parse được, không báo thay**; `01_add.gt` ≡ `11_ascii_alias.gt` và 3 cặp viết tay cho cùng AST (so sánh sau khi che span).
- **Đột biến parser:** 18 đột biến (độ ưu tiên, kết hợp, so sánh, literal âm, semi, đích gán, for-in, struct literal, cast, self-param, độ sâu chuỗi, lỗi dây chuyền, recovery, pattern, raw ptr…): 16 bị bắt ngay; 1 sống sót thật (token rác sau item hỏng bị nuốt) → thêm test, nay bị bắt; 1 tương đương hành vi (không tiêu thụ `;` khi khôi phục: chốt tiến-lên + im lặng-lỗi-cùng-vị-trí cho cùng kết quả).
- **Fuzz parser:** `crates/gt-parser/fuzz` (không ICE `E9901`, luôn kết thúc nhờ `-timeout`, span chẩn đoán hợp lệ, công việc tuyến tính, tất định; hạt giống từ `tests/`). **6 phiên × 10 phút: 0/6 phiên lỗi, 0 artifact crash, tổng 276 703 lượt** (64 811 / 48 926 / 40 814 / 44 504 / 43 778 / 33 870; số lượt giảm ở các phiên sau vì chạy song song với đột biến gt-fmt). Log: `crates/gt-parser/fuzz/logs/summary.log`.
- **Cổng:** workspace 115 test xanh; fmt, clippy `-D warnings`, `cargo audit` sạch; `check_expectations.py` OK (86 file).

### Chỗ grammar mơ hồ/thiếu phát hiện (OPEN_QUESTIONS Q15a–i) — parser theo đề xuất, chờ Giang duyệt
`expr_ns` thiếu ở hợp đồng/`↓`/`⋈`/`⊢`; `self` không là biểu thức; `∈`/`⇒` chưa có độ ưu tiên/kết hợp; luật khối-biểu-thức cuối khối; độ sâu cho chuỗi phẳng; E0307 không thuộc parser; literal âm; **không có chỗ khai báo biến thể `↓` cho đệ quy**; đích gán hẹp hơn nhu cầu.

### gt-fmt (có thật; chưa commit) — thư viện, KHÔNG phải lệnh `gritc fmt` (M9, crates/TASK.md)
- Crate `gt-fmt`: `gt_fmt::format(file, source)`; in lại từ AST, một dạng chuẩn canonical, tự thêm/bỏ ngoặc theo độ ưu tiên + `expr_ns`; comment lấy từ bảng trivia và gắn lại, **không gắn được → từ chối `E1301`** (không xoá/dời). ADR-016.
- **Test (viết trước):** 37 test gt-fmt: bố cục golden; bảng ngoặc; **thước đo parser** `parens_oracle` (mọi biểu thức con × mọi ngữ cảnh: ngoặc thừa phải bỏ, ngoặc cần phải giữ đúng số lượng); `expr_ns` ở mọi vị trí (⎇ ⌥ ⟳ ↓ ⋈ ⊢, hợp đồng chữ ký và trait) + đặt lại trong ngoặc/`[]`/đối số; `¬(a ∧ b)`, `(a + b) * c`, `(*p).x ← 1`, `(a,)`, `V()`; round-trip `parse(fmt(parse(x))) == parse(x)` + idempotent + alias=canonical + giữ comment trên **toàn corpus pass** (31 file) và 6000 biểu thức ngẫu nhiên; comment: đặt đúng chỗ, từ chối đúng, **chèn comment sau từng token của corpus → không bao giờ mất/đổi comment, chỉ giữ hoặc E1301** (giữ 365 / từ chối 1603).
- **Lỗi thật phát hiện khi test:** `a as i64 < 1` in ra không đọc lại được (`T <` là generic) → bọc ngoặc vế trái kết thúc bằng kiểu.
- **Đột biến gt-fmt (tay, ~90 đột biến; cargo-mutants không dùng cho crate này):** printer ngoặc/ưu tiên/kết hợp/`ns`/bố cục: 65 đột biến → **62 bị bắt, 3 tương đương hành vi** (postfix=11, range=0 trong `prec` — không có vị trí nào đòi `min` ≥ 11 hoặc = 1; nhánh Range của `ends_with_type` là mã chết → đã xoá). Bộ máy comment: 18 đột biến + 4 → **tất cả bị bắt**. **Sống sót thật, mỗi cái đã có test mới:** 22 đột biến ngoặc ở vòng đầu (bảng kiểm tay quá thưa) → test `parens_oracle` + `expr_ns`; `finish()`/chốt chống báo lỗi đôi/biên khoảng của `Comments` (không chạm tới từ API công khai) → 7 unit test trong `comments.rs`; comment dính liền cuối node (`}// c`) → test mới; biên `start >= prev_end` → test mới. (Lần chạy đầu của script bị hỏng vì mã hoá console và backslash trong heredoc; kết quả chỉ tính từ lần chạy hợp lệ.)
- **Cổng cuối:** `cargo test --workspace` **165 test xanh** (gt-fmt 37: 7 unit + 10 comments + 11 golden + 3 oracle + 6 roundtrip); `cargo fmt --check`, `clippy --workspace --all-targets -D warnings`, `cargo audit` sạch; `check_expectations.py` OK (92 file, 47 mã).
- **Phát hiện kỹ thuật cần Giang quyết:** (1) `E1301` là mã mới (Q16a). (2) Quy tắc gắn comment quá hẹp so với cách người viết dùng contract/tham số/đối số (Q16b, KNOWN_LIMITATIONS). (3) `x as T < y` là điểm vụng của grammar (generic args), fmt chỉ giải quanh.
- **Đính chính của Giang:** `gritc fmt` chưa tồn tại ở M2 (gt-driver M9); đã đổi mọi chỗ ở M2 thành "gt-fmt (thư viện)", ghi vào crates/TASK.md. Không tạo binary tạm.

### Chưa làm
- Commit gt-fmt (chờ lệnh). M3 chưa bắt đầu. Chờ Giang: Q16a/Q16b. Còn lại từ trước: `FileId` cho M3, mọi pass đệ quy mới phải dùng `gt-stack`.
