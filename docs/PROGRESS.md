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

## M2 — Parser + AST + Diagnostics — **HOÀN THÀNH** (commits `599452e`, `9505469`, `aad181d`, `1d8e79a` M2d, `d370890` M2e, `745c258` M2f; Giang duyệt Q15, Q16a/b/c)
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

### gt-fmt (có thật; M2d `1d8e79a` + M2e `d370890`) — thư viện, KHÔNG phải lệnh `gritc fmt` (M9, crates/TASK.md)
- `gt_fmt::format(file, source)`: in lại từ AST, một dạng chuẩn canonical, tự thêm/bỏ ngoặc theo độ ưu tiên + `expr_ns`. **Comment đặt theo cơ chế TOKEN tổng quát (Q16b, ADR-016 v2):** in văn bản cơ sở không comment, căn token (chỉ `( ) , ;` được khác), comment dòng riêng → token kế tiếp, cuối dòng → token liền trước, ép xuống dòng sau comment cuối dòng, rồi xác minh (token + dãy comment) — `E1301` chỉ còn là phương án cuối.
- **Nghiệm thu Q16b (test thường trực):** chèn `// zz` sau MỖI token của 33 file corpus ở hai dạng (cuối dòng / dòng riêng): **hơn 3 000 ca, 0 ca bị từ chối**; số/nội dung/thứ tự comment giữ nguyên, `fmt` idempotent, AST giữ nguyên. Thay test `known_limitation_…` cũ.
- **Đột biến:** printer (ngoặc/ưu tiên/`ns`/bố cục) 65 đột biến → 62 bị bắt + 3 tương đương (mã chết đã xoá); bộ máy comment cũ 22 đột biến + `attach.rs` mới 28 đột biến → mọi đột biến không-tương-đương bị bắt (sống sót thật đều có test mới: biên khoảng, `finish`, token đóng `)` `]`, `(` khớp). Đột biến nào tương đương hành vi hoặc mã chết đã bị xoá khỏi mã.
- Thay đổi bố cục chuẩn (trước/sau) ghi ở ADR-016: comment sau `{` cách hai khoảng trắng; danh sách bị ngắt tại comment cuối dòng (bậc thang +2).

### M2f (`745c258`): E13xx, `x as T < y` (Q16a, Q16c)
- D16 có "E13xx định dạng (gt-fmt)". `x as T < y` là quy tắc ngôn ngữ (`semantics.md` §25b): parser phát một E0101 tại `<` kèm gợi ý `machine-applicable` bọc `(x as T)` (chỉ khi phần sau `<` parse được thành biểu thức, nên gợi ý không bao giờ sai); test fail, test hình dạng gợi ý, test áp dụng gợi ý, test các dạng hợp lệ.

### Cổng hoàn thành M2 (plan §6.2) — kết quả thật
- **Parse đúng toàn bộ mẫu hợp lệ:** 37/37 file `pass` (33 đơn + 4 ca nhiều tệp) parse không lỗi (`gt-parser` test `corpus`); mọi mẫu `fail` của lexer/parser ra đúng mã.
- **`fmt(parse(x))` idempotent:** `roundtrip` trên cả corpus + 6000 biểu thức ngẫu nhiên; mọi ca chèn comment (3 000+) idempotent; `parens_oracle` (parser làm thước đo).
- **Fuzz parser sạch:** 6 phiên × 10 phút SAU thay đổi Q16c (parser có gợi ý bọc ngoặc), máy rảnh: **0/6 phiên lỗi, 0 artifact crash, tổng 877 830 lượt** (157 828 / 151 945 / 142 769 / 140 682 / 141 838 / 142 768). Log: `crates/gt-parser/fuzz/logs/summary.log`. Các lần chạy ngắn trước đó (3×5 phút) có 2 phiên chết vì HẾT COMMIT BỘ NHỚ hệ thống (ASan `error code: 1455`, trùng lúc build song song); artifact chạy lại dưới đúng harness đều sạch và được giữ làm test hồi quy `crates/gt-parser/tests/fuzz_regressions/`; log fuzz nằm cục bộ (thư mục `fuzz/logs` bị `.gitignore`).
- **Snapshot test thông báo lỗi (tiêu chí plan mà trước đây CHƯA có — đã thêm):** `crates/gt-parser/tests/snapshots.rs` khoá từng byte (người-đọc + JSON) của 22 file fail lexer/parser (`tests/snapshots/*.snap`); `crates/gt-resolve/tests/snapshots.rs` làm tương tự cho 25 ca E02xx nhiều tệp (`tests/snapshots_resolve/`). Không tự cập nhật ngầm (`GRIT_UPDATE_SNAPSHOTS=1` có chủ đích).
- `check_expectations.py` OK (138 file `.gt`, 47 mã, 76 tham chiếu; kiểm cả ca nhiều tệp và `docs/errors/`); `cargo clippy --workspace --all-targets -D warnings`, `cargo fmt --check`, `cargo audit` sạch; **233 test xanh**.
- **Hạn chế còn lại của M2:** xem `KNOWN_LIMITATIONS.md` (gt-parser, gt-fmt, thông báo lỗi parser còn thô ở vài ca, dòng nguồn dài vẽ nguyên dòng).

## M3 — Giải tên & module — **XONG phần code/test; chờ Giang duyệt ADR-017 (Q17, Q18, Q19)**
Cập nhật: 2026-10-06.

### Quyết định (ADR-017, viết TRƯỚC khi code)
- (a) `NodeId = (FileId, u32)`; `FileId` = hạng của tên tệp trong danh sách đã sắp (độc lập thứ tự nhập). Đã sửa `gt-span` (`FileId`, `SourceMap`), `gt-ast` (`NodeId{file,index}`, `NodeIdGen::new(file)`), `gt-parser` (`parse_file`, `parse` mặc định FileId 0). **Schema JSON không cần đổi** (`label`/`edit` đã có `file` từ v1) → không tăng `schema_version`; thay vào đó đã KIỂM: test JSON nhãn/edit sang tệp khác (line/col theo tệp của nhãn; tệp thiếu → null, không crash) và `render_human_files` vẽ nhãn ở tệp khác thành khối `:::` riêng.
- (b) Test nhiều tệp: một thư mục + tệp `expect` một dòng; ghi ở `tests/README.md`; `check_expectations.py` mở rộng (kiểm `expect`, tên tệp = ident duy nhất, không có `// expect:` trong tệp, tham chiếu thư mục trong bảng mã lỗi). 14 ca mới (10 fail, 4 pass).
- (c) Prelude: đầu vào tường minh (`Prelude::from_source`), không có tên nào viết cứng trong resolver; fixture trong test; M12 thay bằng lib/core thật.
- Quy tắc phân giải: module = tệp (không đổi grammar), một không gian tên mỗi module, `⇲` từ gốc (kể cả nhóm), `◈`, E0204 theo định nghĩa cạnh, D7 → E0205 (chuyển milestone M4→M3), pattern ident, `Self`/`self`, `result` trong `⊲`.

### gt-resolve (có thật)
- Crate `gt-resolve`: `resolve_program(inputs, prelude)` → `ResolveOutput {report, defs, uses, decls, canonical(), canonical_diagnostics()}`; chạy trong luồng stack lớn; chỉ chạy khi mọi tệp parse sạch (fail-closed). DefId theo đường dẫn đủ tên.
- **Test (TDD, viết trước):** 50 test: `names.rs` 37 (E0201/2/3/4/5 mọi vị trí, thứ tự phạm vi, biến thể, prelude, nhập nhóm/vòng nhập, mọi vị trí con của AST), `corpus.rs` 4 (31+ pass sạch; fail E02xx ra đúng mã và chỉ mã đó; mẫu của milestone sau không bị báo thay; ca nhiều tệp), `order_independence.rs` 3, `robustness.rs` 4, `snapshots.rs` 2.
- **Tính chất bắt buộc — kết quả không phụ thuộc thứ tự:** 400 chương trình ngẫu nhiên nhiều tệp × 3 lần xáo, hai chiều: xáo TỆP → `canonical()`, tập chẩn đoán và cả danh sách D19 **giống từng byte**; xáo ITEM → `canonical()` và tập chẩn đoán logic giống hệt; thêm kiểm 24/24 hoán vị item cho cùng phán quyết khả kiến. (Phạm vi chính xác ghi ở ADR-017: offset, nên thứ tự D19 theo offset, đổi khi xáo item — D19 sắp theo offset.)
- **Độ bền (thay fuzz, không bắt buộc ở M3):** 1500 chương trình ngẫu nhiên (kể cả bị phá cú pháp) × 2 lần: không panic/ICE, tất định; cây sâu tối đa hợp lệ không tràn stack.
- **Lỗi thật phát hiện nhờ test/đột biến:** (1) mẫu `pass` đã duyệt dùng biến thể trần (`Circle(r)`) → quy tắc biến thể-là-tên-module (Q19); (2) **fail-open**: vòng nhập `a::x ↔ b::x` qua im lặng không báo gì → nay E0201 một lần, chọn thành viên nhỏ nhất (độc lập thứ tự); (3) nhánh `Hidden` của enum bị nuốt im lặng → nay báo E0203; (4) mẫu corpus cần hàm `root_fs`, `raw_read` của thư viện chuẩn → thêm vào fixture prelude (không hard-code).
- **Đột biến gt-resolve (tay, 98 đột biến trên tables/walk/graph/lib):** vòng 1: 70 bị bắt, 27 sống sót, 1 treo; sau test mới + sửa mã, chạy lại các đột biến sống sót: 20/21 bị bắt (kể cả treo tính là bị bắt) và 1 tương đương (đã xoá mã thừa); các đột biến tương đương khác nêu bên dưới. Tương đương/mã chết đã xoá khỏi mã: cạnh tới prelude/gốc (hợp nhất một kiểm), `dedup` kề, kiểm `lexed.errors` thừa, `truncate` generics thừa. Một đột biến cho vòng lặp parent vô hạn → `mut.py` nay có timeout (treo = bị bắt) và in tức thì.
- **Tài liệu mã lỗi (plan §0.5):** `docs/errors/{E0101,E0102,E0201,E0202,E0203,E0204,E0205}.md`, mỗi mã ≥3 `- fail:` và ≥3 `- pass:` tham chiếu test thật; `check_expectations.py` kiểm tồn tại và đúng kỳ vọng. (E12xx, E1301 chưa làm — ngoài phạm vi nhóm E01xx/E02xx đã giao.)
- **Cổng:** clippy `-D warnings`, fmt, audit, `check_expectations` sạch; 233 test xanh.

### Chưa làm / chờ duyệt
- Giang duyệt ADR-017: Q17 (module=tệp, một không gian tên, `⇲` từ gốc, E0204, E0205 che cả item/prelude), Q18 (`result` trong `⊲`), Q19 (biến thể là tên module). Commit M3 chờ lệnh. M4 chưa bắt đầu.
- Rủi ro: M4 PHẢI kiểm mục kết hợp/trường/phương thức mà M3 chỉ ghi `Res::Assoc`; pattern ident gõ nhầm thành binding (M5 bắt); môi trường Windows hết commit bộ nhớ làm build/fuzz hỏng ngẫu nhiên (KNOWN_LIMITATIONS).

### Kết quả cổng chạy LẠI ngay trước commit M2-close/M3 (2026-10-06; không dùng kết quả cũ)
- `cargo fmt --all --check`: OK.
- `cargo clippy --workspace --all-targets -- -D warnings` (sau khi `touch` mọi `lib.rs` để buộc kiểm lại): 0 lỗi, 0 cảnh báo.
- `cargo audit`: mã thoát 0 (1290 advisory, 22 crate phụ thuộc).
- `python tools/check_expectations.py`: OK — 138 file `.gt` (fail=74, warn=1, pass=37, lexer=8), 47 mã, 76 tham chiếu test; kiểm cả ca nhiều tệp và `docs/errors/`.
- `cargo test --workspace --no-fail-fast`: **233 test pass, 0 fail**.
- Duyệt của Giang 2026-10-06: Q17, Q18, Q19 (ADR-017) — đã quyết.

## M4 — Hệ thống kiểu — **HOÀN THÀNH** (Giang duyệt Q23 và 4 điểm sai khác, 2026-10-06)
Cập nhật: 2026-10-06. ADR-018 đã duyệt (mono lazy, `ImplTable`, `!`); mục 6 của ADR ghi các chỗ bản M4 khác lệnh (Q23).

### gt-types (có thật)
- API: `check_program(&[SourceInput], &Prelude) -> TypesOutput`. Chỉ chạy khi giải tên sạch (fail-closed). Mô-đun: `ty`, `infer` (biến suy luận Int/Float, mặc định i32/f64), `collect` (hạ kiểu, bảng ADT/trait/impl), `traits` (tra phương thức, autoderef, ràng buộc), `exhaust` (ma trận Maranget + chia khoảng), `validate` (hợp lệ của `⊐`, coherence), `mono` (instance lazy, ngưỡng 64), `check/{expr,call,pat,mod}`.
- Làm đủ (a)–(g) theo thứ tự lệnh: nguyên thuỷ + literal; struct/enum/tuple/trường (E0304); kiểu hàm + suy luận cục bộ + generics; `Secret<T>`/`Cap<T>` mức kiểu (không Copy, cấm `⊐ Copy for`, cấm literal ngoài prelude — E0313; `#lang` ngoài prelude — E1002); `⌥` vét cạn (E0305); fixture prelude phong phú (`Cap<Fs>::read → File`, `File::len`, `Arena::new/alloc`, `Copy`…); tài liệu `docs/errors/` cho E0301–E0305, E0308, E0309, E0311–E0314, E1002 (mỗi mã ≥3 fail + ≥3 pass; `check_expectations.py` kiểm bắt buộc).
- Gợi ý sửa dạng edit: E0302, E0304, E0305, E0308 (test áp gợi ý rồi kiểm lại sạch).
- Corpus: `tests/fail` 116 mẫu (42 mới của M4), `tests/pass` 49 mẫu (12 mới); mọi mẫu pass kiểm kiểu sạch, mọi mẫu fail của M4 ra ĐÚNG mã (mẫu M4 có lỗi cú pháp thì test đỏ, không bỏ qua).
- Test gt-types: 110 test (~365 ca ok/err) + 91 mẫu corpus ⇒ vượt tiêu chí ≥200 ca.

### Đột biến có mục tiêu (ngoại lệ chính sách 2026-10-06) — kết quả thật
- **49 đột biến** (48 ban đầu + 1 cho chỗ vừa sửa) trên D1 (ép số ngầm), D4 (`==` số thực), D5 (bất biến) và phép ép `!`. Công cụ: `mut.py` (mỗi đột biến cần ≥1 test ĐỎ; luôn hoàn nguyên, đã kiểm).
- Lượt 1: **39 bị bắt, 9 sống sót.** Cả 9 đều có test mới (`tests/mutation_survivors.rs`): `&!self` trên giá trị tạm; E0314 không kéo lỗi dây chuyền; literal nguyên + literal thực không hợp nhất; thông điệp toán hạng `!`; nhánh `!` dẫn đầu không quyết định kiểu join; `==` trên tuple/mảng chứa float bị cấm; `x ≔ !` / câu lệnh `!` làm khối phân kỳ; deref kiểu không phải con trỏ.
- Lượt 2 (chỉ 9 đột biến sống sót + 1 mới): **10/10 bị bắt, 0 sống sót.**
- Một sống sót là LỖI THẬT nhỏ: `*x ← 1` với `x: i32` sinh thêm E0308 dây chuyền ngoài E0301; đã sửa trong code (không phải đột biến tương đương).
- Không có đột biến nào được coi là "tương đương" để bỏ qua.

### Cổng chạy lại cuối M4 (2026-10-06, kết quả thật)
- `cargo fmt --all -- --check`: sạch. `cargo clippy --workspace --all-targets -- -D warnings`: sạch. `cargo audit`: exit 0. `tools/check_expectations.py`: OK (192 file .gt; 54 mã; 118 tham chiếu). `cargo test --workspace`: **343 test, 0 hỏng**.

### Sai khác so với lệnh / spec (chi tiết ADR-018 mục 6, Q23 — cần Giang xác nhận)
1. Match số nguyên/`char` vét cạn theo KHOẢNG (lệnh nói "chưa có range pattern" là sai: Q14 + mẫu pass 29 đã duyệt).
2. Literal ngoài phạm vi kiểu là E0601/M7 theo spec, không phải E0301 (bản đầu của tôi sai; đã gỡ).
3. Thiếu đường trả về (E0307) để M5; M4 im lặng.
4. Hai mẫu M6 (`E0402_double_unique`, `E0403_unique_vs_shared`) gán sai kiểu `a ← 2`; đã sửa `*a ← 2` (mã mong đợi giữ nguyên). Fixture prelude của gt-resolve thêm `⊏ Copy { }`.

### Chưa làm / hạn chế (KNOWN_LIMITATIONS)
- Chưa bắt "trait phải thấy trong phạm vi"; generics của trait chưa dùng được; thân prelude không kiểm; hiệu ứng ghi vào chữ ký nhưng chưa kiểm gọi (M5); literal ngoài phạm vi chờ M7.
- **Điều kiện cứng (Giang):** M9 không xong và `gritc` không sinh binary nếu E0601 (M7) / E0307 (M5) chưa có — GRIT_PLAN.md mục 7 điểm 9, KNOWN_LIMITATIONS.md.
- Cổng chạy LẠI ngay trước commit: fmt sạch; clippy workspace `-D warnings` sạch; audit exit 0; check_expectations OK (54 mã/118 tham chiếu); `cargo test --workspace` 343 passed, 0 failed.
- Tiếp theo: M4.5 (Arena/region, E0310, ≥20 fail + ≥20 pass), rồi M5.

## M4.5 — Kiểu vùng nhớ & kiểu capability — **HOÀN THÀNH** (Giang duyệt Q24 a–e, 2026-10-06; Q24f chưa trả lời)
Cập nhật: 2026-10-06. Không đột biến (đúng chính sách).

### Đã làm (có thật, có test)
- **E0310 (D28/D29):** luật NGUỒN GỐC theo kiểu đã giải — hàm trả kiểu chứa `Cap<…>` mà không nhận `Cap<…>` chỉ `main` cấp cao được gọi/nhắc tới; mọi nơi khác (hàm, phương thức, trait impl, generic, nhánh/vòng lặp, `□`/`■`) là E0310. Hook ở 4 chỗ (tham chiếu hàm, hàm kết hợp trait, hàm kết hợp impl, lời gọi phương thức), kiểm ở `finish` sau suy luận.
- **E0315 (mã MỚI):** lifetime ở chữ ký: chưa khai báo / khai báo trùng hoặc che / trường struct-enum không ghi lifetime / elision mơ hồ ở kiểu trả về (đúng một vị trí tham chiếu vào, hoặc không có ⇒ `'static`).
- **`RegionSig`** (`TypesOutput.regions`) cho mọi chữ ký (kể cả prelude); **`Lang::Arena`** (`#lang(arena)`; ngoài prelude là E1002).
- Fixture prelude: thêm `Env/Clock/Alloc/Host`, `Cap<Fs>::sub`, `Cap<Net|Env|Clock|Alloc>` + thao tác, `root_net/env/clock/alloc`, `Host::fs()` (cả fixture của gt-resolve).
- Tài liệu: `docs/errors/E0310.md`, `E0315.md`; `docs/spec/regions.md` (mô hình region + hợp đồng cho M6); ADR-019; semantics §5/§6; error_codes (E0315 mới); KNOWN_LIMITATIONS; Q24.

### Tiêu chí hoàn thành (plan)
- ≥20 ca thiếu capability bị chặn đúng mã: **25 mẫu fail E0310** (1 có sẵn + 24 mới) + 11 test biên trong `capabilities.rs`.
- ≥20 ca có capability hợp lệ qua: **26 mẫu pass mới** (+ 16_cap_fs, 17_secret_type, 42_cap_pass_through, 15_arena_region cũ).
- E0315: **9 mẫu fail**, 7 mẫu pass được tham chiếu; 14 test trong `regions.rs` (mọi vị trí kiểu lồng nhau được duyệt).
- Bản nháp ngữ nghĩa region sẵn sàng cho M6: `docs/spec/regions.md`.

### Cổng chạy lại (2026-10-06, kết quả thật)
- `cargo fmt --check` sạch; `cargo clippy --workspace --all-targets -- -D warnings` sạch; `cargo audit` exit 0; `check_expectations.py` OK (251 file .gt: fail=149, pass=75; 55 mã, 152 tham chiếu); `cargo test --workspace`: **368 passed, 0 failed**.
- Dung lượng sau dọn: `target/` 394 MB (đã xoá `debug/incremental` 84 MB, không dùng — build đặt `CARGO_INCREMENTAL=0`), `.git/` 1.1 MB; xoá `crates/gt-lexer/fuzz/target` (101 MB) và `crates/gt-parser/fuzz/target` (111 MB) vì chỉ là build ASan cũ, sinh lại được; giữ corpus fuzz (25 MB + 15 MB, ngoài git).

- **Chạy LẠI ngay trước commit M4.5** (sau khi ghi quyết định Q24): fmt sạch; clippy workspace `-D warnings` sạch; audit exit 0; check_expectations OK (251 file .gt; 55 mã; 152 tham chiếu); `cargo test --workspace` 368 passed, 0 failed.

### Hạn chế / chờ xác nhận
- Thân hàm chưa kiểm gì về region (trả tham chiếu local, Arena thoát: qua cho tới M6). Không có `Holder<'a>`/`'a: 'b` trong ngữ pháp (Q24e). Chi tiết KNOWN_LIMITATIONS.

## spec: đối số lifetime trong kiểu (ADR-021, Giang duyệt Q24e, 2026-10-06)
- `Holder<'a>`, `Pair<'a, T>` (lifetime trước kiểu): grammar.ebnf (`type_args`), gt-ast (`TypeKind::Path.lifetimes`), parser (lifetime sau kiểu → E0101), gt-fmt (in được), gt-types (E0315: chưa khai báo / sai số lượng / bỏ trống ở trường; bỏ trống ở chữ ký là vị trí ẩn trong `RegionSig`). `'a: 'b` hoãn (KNOWN_LIMITATIONS).
- Test: `gt-parser/tests/lifetime_args.rs` (4), gt-fmt `parse(fmt(x)) == parse(x)` + idempotent trên 8 dạng mới, gt-types `regions.rs` (+2), mẫu `pass/76_lifetime_args_in_types`, `fail/E0315_lifetime_arg_count`, `E0315_lifetime_arg_undeclared`, `E0315_field_elided_lifetime_arg`, `E0101_lifetime_after_type_arg` (+ snapshot).
- Cổng chạy lại ngay trước commit: fmt sạch; clippy workspace `-D warnings` sạch; audit exit 0; check_expectations OK (256 file .gt; 55 mã; 156 tham chiếu); `cargo test --workspace` 375 passed, 0 failed.

## M5 — MIR + dataflow — **HOÀN THÀNH** (Giang duyệt ADR-022/Q34, Q24f, `TypeDb`, 2026-10-06)
Theo thứ tự lệnh: định nghĩa MIR + in text → bước hạ → validator → dataflow (khởi tạo chắc chắn, liveness) → E0307 và `→ !` → thông dịch MIR + bộ sinh ngẫu nhiên. Không đột biến (chính sách).

### Đã làm (có thật, có test)
- **gt-types (bước 1, ADR-020 §1):** `TypeDb` công khai (`is_copy_resolved`, `needs_drop`, `sig_by_node`), `sites`, `bodies`, `scope_of`, `file_names`; `Lang::Drop` (`⊐ Drop for` ngoài lib/core → E0313); `Lang::RangeInclusive` (ADR-022 §1 — sửa lỗi M4); khoá impl ổn định (ADR-022 §3). Test: `typedb.rs` (9).
- **gt-mir:** biểu diễn (`mir.rs`: place gồm deref/field/index/downcast/subslice, rvalue, terminator gồm `Assert` với loại nghĩa vụ, `Call{target: None}` cho `→ !`, không unwind/cleanup); bảng biến region theo từng local + `BorrowData` cho mọi `Ref`; in text ổn định (`print.rs`); validator (`validate.rs`, cân bằng `StorageLive/Dead` trên mọi đường); bước hạ (`build/`: biểu thức, câu lệnh, `⎇ ⌥ ⟳` 3 dạng + `⟳ in` range/range gồm/slice, phương thức với autoref muộn, kéo dài tạm của `≔ &V`, thoát phạm vi với `Drop`/`StorageDead` ngược thứ tự trên mọi đường `⊠ ⊡ ⏎`, `Assert` cho tràn/chia 0/MIN÷-1/-MIN/chỉ số/lát/ép mất thông tin, bỏ mã chết); E0307 và kiểm `→ !` (`completeness`).
- **gt-dataflow:** dàn tổng quát (`BitSet`, `Dual`), bộ giải tiến/lùi theo RPO với trần số bước; `MovePaths`, `MaybeInit`, `MaybeUninit`, `Liveness`, `DefinitelyAssigned` + `check_definitely_assigned`/`check_invariants` (vi phạm = ICE; E0306 đã nghỉ).
- **E0307:** 5 mẫu fail (3 mới) + 4 pass được tham chiếu; `docs/errors/E0307.md`; `DOC_REQUIRED` có E0307.

### Tiêu chí hoàn thành (plan M5) và bù cho việc không đột biến
- **MIR text ổn định:** 9 ca golden `tests/golden/mir/*.gt → *.mir` (gồm ca BẮT BUỘC thoát phạm vi lồng nhau `scope_exit_nested`, `scope_exit_loops`), đọc kiểm tay; in hai lần giống từng byte.
- **Test đặc tả dataflow trên CFG mẫu, kết quả cố định:** `spec.rs` (9).
- **Test thuộc tính dataflow:** 300 CFG ngẫu nhiên × 4 phân tích — trùng điểm cố định với một bộ giải ngây thơ viết độc lập; bất biến khi đánh số lại khối; tất định; dàn đơn điệu. Đã kiểm độ nhạy: làm hỏng hiệu ứng cạnh `Call` → test đỏ.
- **Validator:** chạy trên MỌI thân của corpus; test bỏ từng `StorageDead` trên đường tới được (≥10 vị trí) → đều bị bắt; trùng `StorageDead`, dùng biến đã chết, khoản mượn trùng id, sai số biến region, sai kiểu gán, nhảy sai khối → đều bị bắt.
- **Bảo toàn ngữ nghĩa:** 400 chương trình ngẫu nhiên (bộ sinh có bộ đánh giá AST riêng) → kết quả MIR (thông dịch) trùng kỳ vọng, kể cả LOẠI abort; mọi loại `Assert` đều được kích hoạt và khớp; thứ tự đánh giá trái-sang-phải (kể cả struct literal theo thứ tự viết) có test riêng. Đã kiểm độ nhạy: `+%` hạ sai thành `+^` → đỏ; struct literal đánh giá theo thứ tự khai báo → đỏ.
- **Corpus:** 77 mẫu pass hạ được, qua validator, qua bất biến khởi tạo, các phân tích hội tụ; mẫu M6+ (E04xx) hạ được không lỗi.

### Cổng chạy lại cuối M5 (2026-10-06, kết quả thật, sau `cargo clean`)
- `cargo fmt --check` sạch; `cargo clippy --workspace --all-targets -- -D warnings` sạch; `cargo audit` exit 0; `check_expectations.py` OK (260 file .gt: fail=156, pass=77; 54 mã; 159 tham chiếu); `cargo test --workspace`: **414 passed, 0 failed**.
- Dung lượng: `cargo clean` xoá 1620 tệp / 508 MiB (binary test cũ tích tụ); sau build lại `target/` 425 MB, `.git/` 1,8 MB. Không còn fuzz target cũ (đã xoá ở M4.5); chỉ dùng profile `dev`.

### Chờ xác nhận / hạn chế
- ADR-022 (9 điểm), quan trọng nhất: `#lang(range_inclusive)` (sửa lỗi M4) và biến tạm của đuôi khối. Chi tiết KNOWN_LIMITATIONS (gt-mir / gt-dataflow).
- Cổng chạy LẠI ngay trước commit M5 (2026-10-06): fmt sạch; clippy workspace `-D warnings` sạch; audit exit 0; check_expectations OK (260 file .gt; 54 mã; 159 tham chiếu); `cargo test --workspace` **414 passed, 0 failed**.

## M6 — Borrow checker — **ĐẠT CÓ ĐIỀU KIỆN** (ADR-023/024; còn 4 đột biến chưa giải quyết hoàn toàn — xem "M6-close" ở cuối mục và KNOWN_LIMITATIONS)
- ADR-023 trả lời: thuật toán (NLL 2018: liveness + ràng buộc region + tập khoản vay hiệu lực), move một phần (cho phép — bắt buộc vì `⌥`), reborrow (tường minh + ngầm ở đối số, cần sửa MIR), mượn qua lời gọi theo `RegionSig`, quy tắc "phiên bản" của Arena, bảng nới/chặt so với Rust, corpus ≥155 xấu + ≥155 đúng theo nhóm, mã lỗi cần chốt (Q30–Q33).

## M6-mir: reborrow ngầm (ADR-023 §0.1, Q30) — commit riêng
- Bước hạ MIR chèn `Ref(Unique, (*r))` khi một CHỖ kiểu `&!T` đứng ở đối số (kể cả đối số phương thức, trừ người nhận đã autoref), `≔` có chú thích kiểu, hoặc `⏎`; tail của hàm vẫn move. Golden `tests/golden/mir/reborrow.{gt,mir}` + test cấu trúc (3 reborrow, 1 move). Không đổi golden cũ.
- Cổng chạy lại: fmt sạch; clippy workspace `-D warnings` sạch; `cargo test --workspace` 415 passed, 0 failed.


## M6 — corpus 169 xấu / 187 đúng, tài liệu E0401–E0407 (commit "M6: corpus …")
Checker `gt-borrowck` (NLL 2018: liveness + ràng buộc region + khoản vay hiệu lực) đã chạy được; commit này đưa vào cả mã checker vì corpus không chạy được nếu thiếu nó.

### Số mẫu theo nhóm của bảng ADR-023 §7 (mỗi mẫu `bc` mang dòng `// group: Gn`; test `every_group_of_adr_023_table_meets_its_minimum`)
| Nhóm | Tối thiểu (xấu/đúng) | Xấu | Đúng |
|---|---|---|---|
| G1 dùng sau move | 30 / 30 | 31 (E0401) | 32 |
| G2 hai `&!` chồng | 20 / 20 | 21 (E0402) | 20 |
| G3 `&!` chồng `&` | 20 / 20 | 21 (E0403) | 23 |
| G4 trả tham chiếu tới local/tạm/tham số theo giá trị | 15 / 15 | 16 (E0404) | 16 |
| G5 Arena thoát | 20 / 20 | 21 (E0405) | 22 |
| G6 gán/move khi đang mượn | 20 / 20 | 21 (E0406) | 22 |
| G7 sống không đủ lâu (không trả về) + move ra khỏi tham chiếu | 15 / 15 | 21 (E0404 ×5 + E0407 ×16) | 17 |
| G8 mượn qua lời gọi theo `RegionSig` | 15 / 15 | 17 (mã E0402–E0406) | 35 |
| **Tổng** | **155 / 155** | **169** | **187** |
Mỗi mẫu thuộc đúng một nhóm. Cộng 5 mẫu xấu cũ (E0401–E0405 từ M0) và 77 mẫu đúng cũ: `tests/fail` có 325 tệp, `tests/pass` 264. Mọi mẫu xấu bị chặn ĐÚNG mã và CHỈ mã đó (`every_bad_sample_is_rejected_with_exactly_its_code`); mọi mẫu đúng (kể cả 77 mẫu cũ) qua sạch ⇒ **0 dương tính giả trên corpus đúng** (187 + 77 chương trình). `docs/errors/E0401..E0407.md` liệt kê toàn bộ mẫu fail của từng mã và ≥5 mẫu pass.

### Lỗi thật của MIR M5 tìm thấy khi viết corpus (đã sửa ở commit riêng `M5-fix`)
Toán hạng là một CHỖ được đọc/move lúc dùng chứ không phải lúc đánh giá: `g(x, { x ← 2; 3 })` đọc `x = 2`; `both(b, &b)` bị báo E0406 thay vì E0401. **400 chương trình ngẫu nhiên của M5 không phủ trường hợp này** vì bộ sinh cũ không có biểu thức có tác dụng phụ. Nay bộ sinh có `{ x ← e; e' }` và sinh "chỗ rồi tác-dụng-phụ"; tắt bản sửa thì bộ sinh TỰ phát hiện (chương trình #41), không cần test viết tay. Trong lúc sửa tôi suýt làm hỏng ADR-022 §5 (người nhận theo giá trị bị tiêu thụ SAU đối số): đã hoàn nguyên, có test ghim (`by_value_receiver_is_consumed_after_the_arguments`) và mẫu `call_by_value_receiver_arg_uses_receiver`.

### Mẫu ĐÃ DUYỆT bị sửa để checker qua — danh sách đầy đủ (điều kiện 3 của Giang)
1. **`tests/pass/39_impl_generic.gt`**: `⊐ <T> W<T> { ƒ get(&self) → T … { self.v } }` → `⊐ <T: Copy> W<T> { … }`.
   - *Lý do:* thân hàm move `self.v` (kiểu `T` không ràng buộc, không `Copy`) ra khỏi `&self` — "move ra khỏi chỗ đi qua `*`" bị cấm bởi ADR-023 §3 (đã duyệt) và là **E0407**. M4 không kiểm move nên mẫu đã qua ở các milestone trước.
   - *Vì sao không phải dương tính/âm tính giả:* đây không phải sở thích của checker mà là luật ngôn ngữ đã duyệt (tương đương E0507 của Rust). Ý đồ của mẫu (impl generic + hàm kết hợp) giữ nguyên. **Văn bản gốc được giữ vĩnh viễn làm mẫu xấu** `fail/E0407_generic_field_through_ref` (nhóm G7) để hành vi trên bản gốc luôn được kiểm. Tôi chắc về lý do ngôn ngữ nên không đưa vào OPEN_QUESTIONS; nếu Giang muốn `T` không ràng buộc vẫn move được qua `&self` thì đó là thay đổi ADR-023 §3.
2. Không mẫu đã duyệt nào khác bị sửa (kiểm bằng `git status`: chỉ 39 ở trạng thái `M` trong `tests/`; mọi tệp còn lại là mới).

### Thay đổi fixture dùng chung (không phải mẫu đã duyệt)
- Prelude của gt-types (`tests/common/mod.rs`): thêm `Guard { ◈ inner: Buf, ◈ n: i32 }` + `⊐ Drop for Guard` + `mk_guard()` (mẫu E0407 cho kiểu có Drop) và `Arena::reset(&!self)` (mẫu E0405 reset). Prelude của gt-resolve: thêm `Guard`, `mk_guard` (corpus pass phải qua M3). Hai thay đổi này chỉ THÊM tên; `Buf.n` KHÔNG đổi (tôi từng làm nó công khai rồi hoàn nguyên: sửa mẫu thay vì sửa fixture).

### Checker: `Len` của mảng
`Len` của MẢNG không còn là "đọc chỗ"; `Len` của lát vẫn là một lần đọc. Ghi chú kỹ thuật: ở Grit mọi mảng đều có độ dài biết lúc biên dịch (`Ty::Array(_, u64)`), nên "mảng kích thước biết lúc biên dịch" = mọi mảng; điều kiện của Giang được thoả bằng cách phân biệt theo `Ty::Array` vs `Ty::Slice`. Test hai chiều `tests/len_reads.rs`: mảng chỉ báo E0402, lát báo E0402+E0403; đọc `s.len()` khi reborrow `&!*s` còn dùng là E0403; hợp lệ khi reborrow đã hết.

### Công cụ
- `tools/check_expectations.py`: mẫu fail không bị coi là mồ côi nếu nằm trong `docs/errors/<mã>.md`; kiểm ngược (mẫu được liệt kê phải tồn tại, header khớp mã, mỗi mã ≥3 fail + ≥3 pass) đã có từ trước và giữ nguyên. **`tools/test_check_expectations.py` (9 test)**: sao chép corpus vào thư mục tạm, áp một đột biến, script PHẢI thất bại — thêm mẫu mồ côi, đổi mã trong header, pass→fail, xoá mẫu được liệt kê, dòng docs trỏ vào hư không, <3 mẫu fail, <3 mẫu pass, liệt kê sai tài liệu mã; cộng baseline sạch.
- `tools/gate.sh` (cổng đầy đủ), `t.sh` (cargo có thử lại, TEMP→D:). Môi trường: xem KNOWN_LIMITATIONS "Môi trường build trên Windows".

### Cổng chạy lại ngay trước commit (2026-10-06, kết quả thật, `tools/gate.sh`)
- `cargo fmt --all --check`: sạch.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0 (lần đầu gt-borrowck có 3 lỗi — import thừa, `or_insert_with(Vec::new)`, tham số chỉ dùng trong đệ quy — đã sửa).
- `cargo audit`: exit 0.
- `cargo test --workspace`: **424 passed, 0 failed**.
- `python tools/check_expectations.py`: OK (616 tệp .gt: fail=325, warn=1, pass=264, lexer=8; 55 mã; 159 tham chiếu).
- `python -m unittest tools/test_check_expectations.py`: 9 test, OK.
- Cổng của commit `M5-fix` chạy riêng trên cây sạch (HEAD + 3 tệp): fmt sạch, clippy exit 0, audit exit 0, 417 passed 0 failed, check_expectations OK.

## M6 — oracle động (ADR-024, commit "M6: dynamic oracle …")
Thông dịch MIR có theo dõi khoản vay/move/Arena, độc lập với mã phân tích của borrowck; bộ sinh chương trình ngẫu nhiên có ĐỘ PHỦ theo loại cấu trúc; tấn công hai phía; hiệu chuẩn trên corpus. Mã: `crates/gt-borrowck/tests/{oracle.rs,support/}`.

### Kết quả lần chạy đầy đủ (2026-10-06; `cargo test -p gt-borrowck --test oracle -- --nocapture`, ~2 phút)
- **20 000 chương trình ngẫu nhiên** (mỗi chương trình chạy 8 tổ hợp `(c, d, e)`), **455 966 khoản vay được theo dõi**; mọi chương trình qua M4/M5; oracle không gặp MIR ngoài phạm vi (0 `Bug`).
  - borrowck chấp nhận ∧ oracle sạch: **10 002**.
  - borrowck chấp nhận ∧ oracle thấy vi phạm (**âm tính giả thật**): **0**.
  - borrowck từ chối ∧ oracle thấy vi phạm: **9 979**; trong đó mã oracle ≠ mã borrowck: 0.
  - borrowck từ chối ∧ oracle sạch (ứng viên dương tính giả): **19 = 0,19%** của 10 021 chương trình oracle sạch; **cả 19 đều thuộc một nguyên nhân đã biết: chữ ký `RegionSig`** (`pick(&x, &y)` buộc kết quả với cả hai đối số dù thân chỉ trả `x`), cùng mã E0406; nhóm "KHÁC (cần xem tay)" = 0 ca.
- **Độ phủ bộ sinh theo loại cấu trúc** (ngưỡng ADR-024 §2: ≥ 300 chấp nhận+sạch VÀ ≥ 300 oracle thấy vi phạm; mọi loại đạt):
  | loại | tổng | chấp nhận+sạch | oracle thấy vi phạm |
  |---|---|---|---|
  | move | 6 725 | 2 404 | 4 319 |
  | reborrow | 4 751 | 1 693 | 3 055 |
  | borrow_in_loop | 4 224 | 1 848 | 2 374 |
  | borrow_via_match | 4 003 | 1 920 | 2 080 |
  | partial_move | 1 783 | 446 | 1 336 |
  | arena_reset | 1 708 | 500 | 1 208 |
  | call_regionsig | 4 923 | 2 609 | 2 295 |
  | nested_struct | 6 153 | 2 674 | 3 476 |
- **Tấn công hai phía:** 1 577 chương trình cơ sở hợp lệ ở cả hai phía; hai phía cùng bắt **UseAfterMove 400, SecondBorrow 753, ArenaReset 424**; **0 bất đồng**.
- **Hiệu chuẩn trên corpus:** 114 mẫu đúng chạy được → 0 vi phạm; 133 mẫu xấu chạy được → oracle thấy **127 (95,5%)** (E0401 26/28, E0402 19/21, E0403 20/20, E0404 21/21, E0405 21/21, E0406 20/21, E0407 0/1). 6 mẫu oracle không thấy, đều có lý do: `E0401_move_in_condition` (điều kiện luôn đúng nên nhánh dùng-sau-move không chạy), `E0401_move_in_conditional_loop_branch` (vòng chỉ 2 lần, move ở lần cuối), `E0402_array_elements_conservative` và `E0406_assign_element_while_other_borrowed` (borrowck coi mọi chỉ số chồng lấn; oracle chính xác theo chỉ số), `E0402_method_and_arg_unique` (hai `&!` truyền vào hàm có thân rỗng nên không bao giờ được dùng), `E0407_drop_type_field_let` (quy tắc tĩnh Drop, thân prelude không chạy).

### Lỗi/bài học trong lúc làm (đều của oracle/bộ sinh, không phải của borrowck)
1. Bộ sinh dùng chung một điều kiện `c` cho `⎇` lồng nhau ⇒ nhánh chèn không bao giờ chạy ⇒ lần đầu 11 "bất đồng" giả; sửa bằng điều kiện riêng mỗi tầng (`c`, `d`, `e`).
2. Vòng lặp `0..1` làm 180 chương trình thành "ứng viên dương tính giả" (borrowck đúng khi thấy cạnh lùi); sửa bằng ≥ 2 lần lặp ⇒ ứng viên còn 19 (chữ ký).
3. Oracle thiếu: mượn chỗ đã move cả struct, tham chiếu treo nằm trong giá trị trả về, hằng `□` — đã bổ sung (đều tìm ra nhờ phân loại ứng viên dương tính giả và hiệu chuẩn trên corpus).
4. Thiếu độ nhạy ban đầu: hiệu chuẩn chỉ thấy 102/133 mẫu xấu ⇒ sửa oracle ⇒ 127/133.

## M6 — soundness hunting: LỖ HỔNG THẬT tìm thấy và sửa (commit "M6-fix: …")
- **Lỗ hổng (âm tính giả, E0401):** `μx ≔ 1; a ≔ &!x; b ≔ a; *b ← 1; *a ← 2;` được borrowck CHẤP NHẬN. `b ≔ a` MOVE `a` (`&!` không Copy), rồi `*a ← 2` ghi qua con trỏ đã move ⇒ hai `&!` cùng ghi vào một chỗ (vi phạm tính duy nhất; Rust: E0382). Nguyên nhân: `check_uses` chỉ kiểm "đã move" cho truy cập ĐỌC; truy cập GHI chỉ kiểm trường hợp ghi vào trường của cha đã move, không kiểm con trỏ/mảng đứng trước `Deref`/`Index`. Cùng lỗ hổng cho `b.n ← 1` qua `&!W` đã move và `a[0] ← v` của mảng đã move.
- **Cách tìm:** soundness hunting viết tay (test `tests/soundness.rs`, 9 test, ~70 chương trình tấn công + hợp lệ) tìm ra; sau đó thêm hành động "gán tham chiếu sang biến khác" vào BỘ SINH và xác nhận oracle TỰ phát hiện: tắt bản sửa ⇒ **14 âm tính giả trong 5 000 chương trình ngẫu nhiên**. (20 000 chương trình trước đó không phát hiện vì bộ sinh chưa có hành động này — bộ sinh phải sinh được cấu trúc thì mới "phủ".)
- **Sửa:** ghi qua `Deref`/`Index`/`Subslice` đòi con trỏ/mảng (phần chỗ đứng trước phép chiếu gián tiếp cuối) còn khởi tạo. **Test vĩnh viễn:** `soundness.rs::unique_references_cannot_be_duplicated_by_copying_or_moving_around`, 3 mẫu corpus mới `E0401_write_through_moved_unique_ref`, `E0401_write_field_through_moved_ref`, `E0401_write_index_of_moved_array` (nhóm G1: 34 mẫu xấu), và hành động bộ sinh. Oracle cũng được sửa: giải tham chiếu một tham chiếu đã move là vi phạm `UseAfterMove` (trước đó là `Bug`).
- Các hướng tấn công khác đã thử và borrowck chặn đúng: alias qua chuỗi reborrow, hàm trả lại tham chiếu/generic/struct generic, match move payload/mượn payload, move một phần (struct, tuple), vòng lặp (cạnh lùi), mọi thao tác "đổi phiên bản" Arena (reset, move, gán đè, thoát phạm vi, trả về, trong nhánh/vòng, tham số `&!Arena`), tham chiếu giữ trong struct/tuple/Option/mảng, move ra khỏi `*`/`[i]`/Drop. Hai chiều (chặn đúng + chấp nhận mẫu hợp lệ song sinh).
- Cổng chạy lại ngay trước commit sửa lỗ hổng (kết quả thật): fmt sạch; clippy `-D warnings` exit 0; `cargo audit` exit 0; `cargo test --workspace` **438 passed, 0 failed**; check_expectations OK (619 tệp .gt: fail=328, pass=264); test script 9/9.

## M6 — đột biến gt-borrowck (ADR-023 §10a) và số liệu đóng M6 (commit "M6: mutation testing …")
**Công cụ:** `tools/mut.py` (đột biến theo dòng: so sánh, `&&/||`, `true/false`, bỏ phủ định, `any/all`, Kind/Depth/BorrowKind, `return`, `continue`, `±1`, `condition → true/false/phủ định`, xoá câu lệnh có tác dụng phụ; mỗi đột biến chạy `cargo test -p gt-borrowck` trên bản sao riêng, 3 worker; bị bắt = có test ĐỎ thật, treo = bị bắt, lỗi biên dịch = không tính). Hai lỗi của chính công cụ đã sửa: baseline chưa được kiểm xanh (một vòng "mọi đột biến bị bắt" giả) và artifact biên dịch cũ sau đột biến treo (nay `touch` + giết cả cây tiến trình + kiểm baseline xanh trước khi chạy).
**Kết quả lần chạy đầy đủ cuối (trên mã nguồn cuối, bộ test cuối; 347 đột biến):** bị bắt **271**, **sống sót 63**, không biên dịch 13. (Vòng đầu 340 đột biến: bị bắt 195, sống sót 133; sau test mới và sửa mã: 270/64; sau thêm một test cho ADT bất biến: 271/63.)
**Sống sót đều đã phân loại (không có ca nào bỏ qua):**
| Loại | Số | Ghi chú |
|---|---|---|
| Tương đương/không quan sát được | 22 | `kill_through_deref` cho đích LỜI GỌI (MIR luôn gán kết quả vào biến tạm rồi `Assign`, nên nhánh `call_return_effect` không bao giờ có khoản vay để giết); `Deep→Shallow` ở `Copy`/`Len`/`Discriminant`/`FakeRead`/`Drop`/ghi đích lời gọi (khác nhau chỉ khi truy cập nông vào con trỏ trong lúc khoản vay đi QUA `*` của nó: cần con trỏ Copy có `&!` hay kiểu Drop giữ tham chiếu — ngôn ngữ chưa có); `Downcast` rời nhau (không có mẫu `ref`); khử trùng chẩn đoán (`Report` đã bỏ trùng); `is_arena` khi thiếu `Lang::Arena` (prelude luôn có); `continue→break` trong vòng lặp mà phần tử sau không có gì để kiểm; ràng buộc tự thân `a:a`; `bundle.is_empty()` (thêm `'static : x` vô hại); `while v.len() < n → <=` (thừa một biến, bị `take(n)`); `check_program` thoát sớm khi M4/M5 đã lỗi (M5 vốn fail-closed) |
| Nhánh phòng thủ (`return false`, kiểm độ dài, chốt chặn) | 25 | `call_sig`/`walk`/`relate`: bộ đếm `defensive_counters()` = (0, 0) trên toàn corpus + 20 000 chương trình ngẫu nhiên ⇒ không với tới được bằng chương trình qua M4 (bằng chứng, không phải chứng minh) |
| Bộ đếm đo đạc (chính dòng đếm) | 4 | |
| Chỉ làm đổi VỊ TRÍ chẩn đoán | 2 | `cause_span` (BFS định vị lỗi region phổ quát) |
| **Còn lại: chưa giết, chưa chứng minh tương đương** | **10** | `regions.rs`: 224 (giá trị chốt chặn vòng của ADT đệ quy), 238 (`ty_invariant` qua `&`/mảng/lát), 240 ×2 (`ty_invariant` của ADT: `||→&&`, `any→all` ở đối số kiểu), 273 (`relate` con trỏ thô), 279 (`if invariant` trong `relate` ADT), 341 (`Repeat` truyền `inv=true`), 529 (`&self` khai báo `&!`), 566 (quan hệ kết quả của lời gọi với `inv=true`); `solve.rs:90` (`'static` chứa mọi `end`: chỉ đổi SỐ chẩn đoán phụ của một lỗi vẫn bị báo). Phần lớn là hướng "chặt hơn" (thêm ràng buộc ⇒ chỉ từ chối thừa); 224/238/240/273 là hướng LỎNG hơn nhưng cần ADT/kiểu mà tôi chưa viết được chương trình phân biệt — KHÔNG đủ để khẳng định tương đương |
Đột biến TÌM RA lỗ hổng thật #2 (ADR-024 §7): dựng ADT bất biến chỉ ràng buộc hiệp biến ⇒ `Sl { s: &!p }` cho phép thu hẹp region của `p`; đã sửa và có test (`soundness.rs`, `kills.rs`). **Test mới do đột biến: 20 test trong `kills.rs` + 1 trong `soundness.rs`.**

### Số liệu đóng M6 (kết quả thật, 2026-10-07)
- **Mẫu theo nhóm (ADR-023 §7), xấu / đúng:** G1 move 34/32 · G2 hai `&!` 21/20 · G3 `&!`∧`&` 21/23 · G4 trả tham chiếu 16/16 · G5 Arena 21/22 · G6 gán/move khi mượn 21/22 · G7 sống không đủ lâu + move ra khỏi tham chiếu 21/17 · G8 qua lời gọi 17/35; **tổng 172 xấu / 187 đúng** (cộng 5 xấu + 77 đúng cũ ⇒ `tests/fail` 328, `tests/pass` 264). Ngưỡng 30/20/20/15/20/20/15/15 đều đạt.
- **Chương trình ngẫu nhiên qua oracle:** 20 000 (× 8 tổ hợp nhánh), 416 581 khoản vay. **Vi phạm tìm được:** 10 355 chương trình bị borrowck từ chối và oracle xác nhận (mã trùng 100%); **âm tính giả: 0**; chấp nhận ∧ sạch: 9 556. Ngoài ra săn soundness tìm ra 2 lỗ hổng thật (đã sửa); tắt bản sửa #1 thì oracle tự thấy 14/5 000.
- **Tấn công hai phía:** 1 820 cơ sở hợp lệ ở cả hai phía; hai phía cùng bắt UseAfterMove **448**, SecondBorrow **972**, ArenaReset **400**; **0 bất đồng**.
- **Độ phủ bộ sinh (chấp nhận+sạch / oracle thấy vi phạm; ngưỡng ≥ 300 mỗi cột, mọi loại đạt):** move 2 296 / 4 350 · reborrow 1 640 / 3 126 · borrow_in_loop 1 631 / 2 552 · borrow_via_match 1 924 / 2 109 · partial_move 415 / 1 371 · arena_reset 394 / 933 · call_regionsig 2 173 / 2 076 · nested_struct 2 223 / 3 277 · ref_flow 1 400 / 1 434 (tổng chương trình mang loại đó: 6 663 / 4 777 / 4 204 / 4 044 / 1 789 / 1 331 / 4 277 / 5 515 / 2 910).
- **Tỉ lệ dương tính giả:** trên corpus đúng (187 + 77 mẫu cũ): **0**. Trên chương trình ngẫu nhiên: **89 / 9 645 oracle-sạch = 0,92%**, đều có nguyên nhân không-đầy-đủ đã biết: 22 do chữ ký `RegionSig`, 67 do liveness theo biến (Rust cũng vậy); **0 ca "KHÁC (cần xem tay)"** (test khẳng định `== 0`). Đây không phải tỉ lệ dương tính giả theo nghĩa lỗi (ADR-024 §5).
- **Hiệu chuẩn oracle:** 114 mẫu đúng chạy được → 0 vi phạm; 136 mẫu xấu chạy được → oracle thấy 129 (94,9%).
- **Đột biến:** 347, bị bắt 271, sống sót 63 (22 tương đương, 25 phòng thủ, 4 đo đạc, 2 vị trí chẩn đoán, 10 chưa giải quyết), 13 không biên dịch.
- **Dung lượng:** `target/` 1,1 GB, `.git/` 1,3 MB (thư mục làm việc đột biến `D:/Grit_mut` ngoài repo đã xoá).
- **Cổng chạy lại ngay trước commit (kết quả thật, `tools/gate.sh`):** fmt sạch; clippy `-D warnings` exit 0; `cargo audit` exit 0; `cargo test --workspace` **459 passed, 0 failed**; check_expectations OK (619 tệp .gt: fail=328, pass=264); test script 9/9.

## M6-close (2026-10-07) — xử lý đột biến sống sót, nhánh phòng thủ fail-closed, luật bộ sinh
**Trạng thái: M6 ĐẠT CÓ ĐIỀU KIỆN.** Điều kiện: 4 đột biến còn sống (`regions.rs` 254, 361, 554, 591) chưa được giết; 3 trong số đó có lập luận tương đương (nêu ở KNOWN_LIMITATIONS) nhưng KHÔNG phải chứng minh, nên coi như false negative tiềm ẩn cho tới khi Giang chấp nhận lập luận.
1. **Đột biến (lần chạy đầy đủ cuối, 347 đột biến trên mã nguồn cuối):** bị bắt **284**, sống sót **50**, không biên dịch 13. Sống sót đã phân loại: **21** tương đương/không quan sát được (lý do cụ thể ở bảng PROGRESS trước), **2** chỉ đổi vị trí chẩn đoán (`cause_span`), **9** là sổ sách ICE/bộ đếm (chính `defend` ghi ICE lần đầu, đếm), **14** là điều kiện của các chốt chặn phòng thủ (nay mỗi chốt chạm tới ⇒ ICE E9901, bộ đếm = 0 trên mọi test), **4** chưa giải quyết.
   - Hai mục ưu tiên "làm checker lỏng hơn" đã **giết bằng chương trình tấn công**: **224** (chốt chặn vòng của ADT đệ quy: `recursive_adts_stay_covariant`), **240** `any→all`/`||→&&` (`a_covariant_wrapper_of_an_invariant_adt_is_invariant`, `an_adt_with_one_invariant_field…`), **273** (con trỏ thô: `raw_pointers_are_invariant_in_what_they_point_to`), **230**, **279**, **563** (`call_arguments_flow_one_way_into_the_signature`), `solve.rs:90` (`a_static_requirement_reports_every_region…`), `646` (`function_typed_parameters_are_mapped`). Còn 238 (nay dòng 254): lập luận tương đương — không có đường GHI qua tham chiếu chung `&`, nên không thể khai thác độ lỏng của `ty_invariant(&…)`.
   - **Còn lại 4:** `regions.rs` 254 (như trên), 361 (`Repeat` quan hệ hai chiều: một nguồn duy nhất nên chiều ngược không thêm gì), 591 (kết quả lời gọi quan hệ hai chiều: biến region của callee chỉ có kết quả làm cận dưới), 554 (`&self` khai báo như `&!`: hướng CHẶT hơn, tôi chưa viết được chương trình hợp lệ bị từ chối nhầm).
2. **Nhánh phòng thủ fail-closed (nguyên tắc P3):** các `return false`/kiểm độ dài của `call_sig`/`walk`/`relate` và các nhánh "không khớp RegionSig" nay gọi `Gen::defend`, ghi ICE và `check_body` trả **E9901** (trước đây âm thầm rơi về đường bảo thủ hoặc bỏ qua quan hệ). `defensive_counters()` giữ nguyên và là TEST: bằng (0, 0) trên toàn corpus xấu/đúng và trên 20 000 chương trình ngẫu nhiên (ADR-024 §7: bằng chứng kiểm thử, không phải chứng minh).
3. **Luật bộ sinh mới (`.claude/skills/grit-contract.md` §3.3):** mỗi lỗ hổng soundness tìm bằng cách khác với bộ sinh phải mở rộng bộ sinh để tự tìm được, kiểm bằng cách tắt bản sửa. **Lỗ hổng #2 thực hiện ngay:** kịch bản `adt_invariant` (`SlT<'a> { s: &!'a &'a i32 }`, có biến thể bọc `Id1<T>`); **tắt bản sửa (dựng ADT bất biến chỉ hiệp biến) ⇒ bộ sinh tự phát hiện 151 âm tính giả trong 5 000 chương trình**; bảng độ phủ có loại thứ 10. Cũng nhờ lượt này: dựng ADT dùng `ty_invariant` của kiểu ĐÃ THẾ (`Id1<Sl<'a>>`), không chỉ định nghĩa ADT.
4. **Snapshot nhãn/vị trí chẩn đoán E0401–E0407:** `tests/diag_snapshot.rs` + `tests/golden/borrowck/diagnostics.txt` (187 chẩn đoán trên 172 mẫu xấu: mã, văn bản được gạch chân, nhãn @ văn bản, thông điệp), đã đọc tay một lần; kiểm thêm: vị trí không rỗng, nhãn có mặt trừ E0401/E0407/lỗi region phổ quát.
5. **KNOWN_LIMITATIONS:** trạng thái bằng chứng của borrowck (chưa chứng minh hình thức; bằng chứng; hai lỗ hổng đều KHÔNG do bộ sinh tìm ra lần đầu) và mục "đột biến chưa giải quyết, coi như false negative tiềm ẩn".
6. **M7:** ADR-025 đã viết (DỰ THẢO, chưa có mã M7), chờ Giang duyệt; Q35–Q42 ở OPEN_QUESTIONS.

### Số liệu sau M6-close (kết quả thật)
- Mẫu: 172 xấu / 187 đúng (nhóm như trên); `tests/fail` 328, `tests/pass` 264.
- Oracle: 20 000 chương trình × 8 tổ hợp, 447 798 khoản vay; **âm tính giả 0**; chấp nhận ∧ sạch 9 068; từ chối ∧ oracle thấy vi phạm 10 514 (mã trùng 100%). Ứng viên dương tính giả 418/9 486 = 4,4%, TOÀN BỘ có nguyên nhân đã biết: 337 do ADT bất biến `Sl` ghim `p` suốt `'a` (Rust cũng từ chối), 34 do `RegionSig`, 47 do liveness theo biến; 0 ca chưa giải thích (tỉ lệ cao hơn trước vì kịch bản `adt_invariant` mới cố ý sinh nhiều chương trình loại này).
- Tấn công hai phía: 1 609 cơ sở hợp lệ; UseAfterMove 445, SecondBorrow 764, ArenaReset 400 ca hai phía cùng bắt; 0 bất đồng.
- Độ phủ (chấp nhận+sạch / oracle thấy vi phạm; ngưỡng ≥ 300 mỗi cột): move 2 233 / 4 388 · reborrow 1 564 / 3 223 · borrow_in_loop 1 377 / 2 347 · borrow_via_match 1 835 / 2 132 · partial_move 400 / 1 354 · arena_reset 362 / 954 · call_regionsig 2 053 / 2 054 · nested_struct 1 868 / 2 906 · ref_flow 1 005 / 1 067 · **adt_invariant 847 / 1 717**.
- Hiệu chuẩn: 114 mẫu đúng → 0 vi phạm; 129/136 mẫu xấu được oracle thấy.
- Dung lượng: `target/` 1,1 GB, `.git/` 1,5 MB.
- **Cổng chạy lại ngay trước commit (kết quả thật, `tools/gate.sh`):** fmt sạch; clippy `-D warnings` exit 0; `cargo audit` exit 0; `cargo test --workspace` **465 passed, 0 failed**; check_expectations OK (619 tệp .gt); test script 9/9.

## M7 (2026-10-07) — tiến độ (i)–(iv): IR nghĩa vụ, hai bộ giải, phân tích khoảng+zone, sinh/xoá/hạ nghĩa vụ
- **(i)** `gt-oblig`: `I256`, `Interval` (độ rộng tham số hoá), `Zone` (DBM, hằng `i128` kiểm tràn), `Env`, IR `Term/Prop/Obligation` tuần tự hoá JSON (schema v1, id tất định), trait `Prover` với **hai** cài đặt: `ZoneProver` (khoảng+zone) và `ExhaustiveProver` (vét cạn, độc lập, không dùng `Env`).
- **(ii)** Soundness hàm chuyển: `tests/exhaustive.rs` (mọi cặp hộp ở độ rộng 2..7, 8 lấy mẫu phân tầng; 8 đầy đủ với `GT_FULL8=1`), `tests/provers.rs` (67 016 + 81 000 + 6 000 nghĩa vụ so chính vs vét cạn: 0 chứng minh sai; 0 'chắc chắn sai' sai; chính xác tuyệt đối với giả thiết dạng hộp; 3 ca giả thiết mâu thuẫn chưa bị phát hiện → chỉ mất chính xác).
- **(iii)(iv)** `analysis.rs` + `driver.rs`: phân tích luồng trên MIR, widening/narrowing, kiểm hậu-điểm-cố-định (narrowing bị bỏ nếu không qua), định nghĩa biến tạm (`defs`) bị xoá ngay khi biến nó tham chiếu bị gán lại, local bị mượn `&!` không theo dõi, `ZONE_MAX_VARS=24` + N0605, gốc tái mượn của `&[T]` tĩnh cho nguyên tử độ dài, `<[T]>::len` mô hình hoá; chính sách xoá/giữ/ĐỎ; literal ngoài phạm vi → E0601 độc lập đường tới (M5: `Body.bad_literals`); báo cáo JSON "check còn lại".
- Mã mới trong bảng: N0603, N0604, N0605, N0902, N0903, N0904.
- Corpus: 264 ca pass → 237 nghĩa vụ: 24 xoá, 213 giữ (N-mã), 0 lỗi E; E0601/E0602/E0603 (4 mẫu fail) ra đúng mã. `sum`: `xs[i]` và `i+1` xoá, `s + xs[i]` giữ N0601.
- **Cổng chạy lại ngay trước commit (kết quả thật, `tools/gate.sh`, 37 phút do `CARGO_BUILD_JOBS=1`):** fmt sạch; clippy `-D warnings` exit 0; `cargo audit` exit 0; `cargo test --workspace` **495 passed, 0 failed**; check_expectations OK.

## M7-close (2026-10-08) — hợp đồng, oracle, đột biến, cổng cuối. **Trạng thái: M7 ĐẠT CÓ ĐIỀU KIỆN**
Điều kiện: 69 đột biến của `analysis.rs`/`driver.rs` theo hướng nguy hiểm (có thể xoá thêm check) chưa giải quyết; xem KNOWN_LIMITATIONS (mục gt-oblig) và GRIT_PLAN §7 điểm 11 (M9 phải có chế độ giữ mọi check).

### Số liệu (kết quả thật)
- **Corpus:** `tests/pass` 315 (+51 so với M6), `tests/fail` 355 (+27: E0601–E0604, E0901, E0902, E0316); 29 mẫu fail E06xx/E09xx ra ĐÚNG mã; 315 mẫu pass không có lỗi E; 27 mẫu ghi chú `pass/N0xxx_*` phát đúng ghi chú. Mỗi mã mới có docs + ≥3 fail + ≥3 pass (ghi chú: ≥3 pass).
- **Vét cạn / bộ giải kép:** cộng/trừ/nhân 493 564 176 cặp hộp, chia/dư 329 042 784 cặp hộp (2..7 bit đầy đủ, 8 bit phân tầng); 154 016 nghĩa vụ ngẫu nhiên so bộ chính với bộ vét cạn: 0 chứng minh sai, 0 'chắc chắn sai' sai. **GT_FULL8=1 (8 bit đầy đủ): CHƯA chạy** (theo lệnh: chạy một lần, một worker, không chặn commit).
- **Oracle mức chương trình:** 1 500 chương trình, 36 000 lần chạy, ~35 600 lần so sánh MIR đủ check với MIR đã tỉa: **0 bất đồng**; độ phủ đạt mọi ngưỡng ở ADR-025 §13; kiểm "tắt bản sửa" #3: oracle bắt ở chương trình #141. Điểm yếu: ~97% lần chạy abort sớm.
- **Lỗ hổng soundness tìm được:** #3 (chuỗi bí danh qua tham số lát) và #4 (`⊲` chứng minh bằng giá trị hiện tại của tham số bị gán lại) — đã sửa, có test (`tests/decisions*.rs`).
- **Đột biến gt-oblig:** 1 148 đột biến (7 tệp): **826 bị bắt, 232 sống sót, 90 không biên dịch**. Sống sót theo nhóm: 69 nguy hiểm chưa giải quyết (`analysis.rs` 62, `driver.rs` 7), 7 chèn kiểm, 134 tương đương/an toàn theo lập luận, 22 ở lõi (tương đương). Chi tiết theo hàm và lập luận: KNOWN_LIMITATIONS.
- **Số check còn lại trên mẫu golden** (`tests/golden/obligations`): `sum` 3 nghĩa vụ → xoá 2, giữ 1 (N0601); `count_to_const` 3 → xoá 3; `two_indices` 9 → xoá 6, giữ 3; `division_guard` 6 → xoá 5, giữ 1; `cast_narrow` 3 → xoá 2, giữ 1; corpus pass: 294 nghĩa vụ, 54 xoá, 239 giữ. Chưa có 3 chương trình thật của M12.
- **Dung lượng:** `target/` 1,2 GB, `.git/` 3,0 MB.
- **Cổng chạy MỘT lần ở cuối (kết quả thật, `tools/gate.sh`, ~19 phút với `CARGO_BUILD_JOBS=2 --test-threads=2`):** fmt sạch; clippy `-D warnings` exit 0 (lần chạy đầu đỏ vì 4 lỗi clippy — đã sửa rồi chạy lại); `cargo audit` exit 0; `cargo test --workspace` **576 passed, 0 failed**; check_expectations OK; test script OK.
- **Lịch sử trung thực:** 6 commit giữa chừng ghi "chưa chạy cổng đầy đủ"; commit này là lần đầu cổng đầy đủ chạy sau chúng.

## M7-sau-duyệt (2026-10-08) — M5-fix điều kiện ghép, đo tỉ lệ chạy tới cuối hàm của oracle, điểm mù của oracle
### 1. M5-fix: lower boolean conditions directly to branches (commit riêng `8b2d020`)
- Điều kiện của `⎇` và `⟳` có `∧ ∨ ⇒ ¬` (lồng nhau) hạ THẲNG thành nhánh; điều kiện đơn và điều kiện dùng làm giá trị hạ như cũ. Thứ tự đánh giá/ngắn mạch giữ nguyên (semantics §3a): vế trái đầy đủ trước, vế phải chỉ khi cần.
- **Golden cũ KHÔNG đổi** (không golden nào có điều kiện ghép trong `⎇`/`⟳`); golden mới: `tests/golden/mir/conditions` (and/or/not-and/lồng/`⇒`/`⟳ ∧`/dùng làm giá trị/có tác dụng phụ ở vế trái), đọc tay từng nhánh. Golden obligations mới: `compound_conditions`, `conditions`.
- Kiểm (chỉ các test đó, không cổng đầy đủ, không đột biến): `gt-mir` (corpus, golden, semantics — thông dịch MIR vs bộ đánh giá AST độc lập trên 400 chương trình ngẫu nhiên CÓ điều kiện ghép), `gt-borrowck` (corpus, soundness, diag_snapshot, oracle động M6), `gt-dataflow`: 79 passed. Bơm lỗi (đảo nhánh "đúng" của `∨`) thì `semantics.rs` tự bắt. Bộ sinh M7 (`obgen`) nay sinh cả `⟳ i < lim ∧ <cond>` (555/2000 chương trình có vòng điều kiện ghép).
- Kết quả cho bộ giải (`tests/decisions.rs`): sau `⎇ x >= 10 ∧ x < 20` biết cả hai vế; nhánh sai của `∨` biết `¬a ∧ ¬b`; `⎇ i < xs.len() ∧ xs[i] > 3` xoá check chỉ số ở vế phải; `⟳ i < xs.len() ∧ xs[i] > 0` xoá cả chỉ số và `i + 1`. Điều kiện dùng làm GIÁ TRỊ vẫn chưa refine (đã ghi KNOWN_LIMITATIONS).
### 2. Oracle mức chương trình: tỉ lệ chạy tới cuối hàm (2 000 chương trình × 24 đầu vào = 48 000 lần chạy)
- **Trước khi chỉnh bộ sinh:** 3 034/36 000 = **8,4%** lần chạy kết thúc bình thường (91,6% abort sớm; ví dụ trước đó 12 114/14 217 ở bản đầu). Lý do: ép kiểu hẹp, phép số học, chỉ số, tiền điều kiện của hàm phụ gần như luôn abort sớm.
- **Sau khi chỉnh bộ sinh** (chế độ "nghiêm" 37% chương trình: mọi phép có nghĩa vụ được bọc trong bảo vệ vừa chứng minh được vừa ĐÚNG lúc chạy; vòng ngắn; `entry` có `⊳` với đầu vào luôn thoả nó kể cả các biên; đối số hàm phụ thoả `⊳` của nó 75%; mẫu "trả nguyên tham số" với `⊲ result ⋈ p0`): **14 367/48 000 = 29,9%** kết thúc bình thường; so sánh 47 539 lần, 0 bất đồng; bỏ 461 (hết nhiên liệu).
- **Theo từng loại nghĩa vụ** (lần thực thi trong lần chạy bình thường : trong lần chạy abort; số check BỊ XOÁ mà đoạn sau chạy tới cuối hàm):
| loại | bình thường | abort | check xoá chạy tới cuối hàm |
|---|---|---|---|
| overflow.Add | 959 134 | 19 705 016 | 1 005 |
| overflow.Sub | 456 120 | 1 733 440 | 703 |
| overflow.Mul | 22 097 | 1 357 052 | 583 |
| overflow.Div | 6 045 | 10 210 | 147 |
| overflow.Rem | 6 285 | 466 843 | 163 |
| neg_min | 9 107 | 10 987 | 238 |
| div_zero | 28 670 | 413 023 | 580 |
| rem_zero | 22 521 | 1 006 977 | 580 |
| index | 13 767 | 65 048 | 401 |
| cast_lossy | 67 757 | 3 428 200 | 2 016 |
| slice_range | 30 582 | 43 074 | 1 495 |
| invariant | 468 764 | 11 133 123 | 203 |
| pre, post | (không có vết Assert) | | ≥ 100 (đếm theo chương trình có ≥ 1 lần chạy bình thường) |
- **Ngưỡng:** mỗi loại ≥ 100 check bị xoá mà đoạn sau chạy tới cuối hàm (hằng `END_THRESHOLD` trong `oracle.rs`); dưới ngưỡng thì test ĐỎ ("chưa được tính là đã kiểm"). Lúc đầu 7/13 loại dưới ngưỡng (invariant 17, neg_min 46, Add 87, Div 41, pre 65, post 33…) — là lý do phải chỉnh bộ sinh.
- Kiểm "tắt bản sửa" #3 sau khi đổi bộ sinh: oracle vẫn bắt ở chương trình #346 (Bug: đọc ngoài biên không có assert).
### 3. Điểm mù của oracle
Ghi vào `.claude/skills/grit-contract.md` §3.3 (luật) và `docs/KNOWN_LIMITATIONS.md` (mục "những gì oracle không quan sát được"). Lỗ hổng #4 là ví dụ: oracle không thể quan sát (chương trình bị E0902), nên chỉ `tests/decisions2.rs` giết được.

## M8 nửa 1 (2026-10-08) — DomainQuery, hiệu ứng E08xx, điều kiện dừng E07xx. **Chưa commit tại thời điểm viết**
- **DomainQuery** (`gt_oblig::analysis::DomainQuery`, ADR-025 §7): khoảng của biểu thức tại điểm MIR, kèm cận zone (`Env::eval_term_zone`). 4 test (`tests/domain_query.rs`): đếm lên (nhánh vào thân `i − n ≤ −1`, nhánh thoát `≥ 0`), đếm xuống, `0..n`, điểm không tới được.
- **Điều kiện dừng** (`termination.rs`): biến bóng + quan hệ zone, chứng minh ở mỗi cạnh lùi `v' < v ∧ 0 ≤ v` bằng bộ giải M7; tự suy từ `∧` của điều kiện; SCC cho đệ quy; `div`. Dữ liệu vòng lặp mới ở MIR (`Body.loops`, `var_map`; `tests/loops_info.rs`). Sửa 2 chỗ khi chạy corpus: biến vòng xếp trước trong ngân sách zone (mẫu N0605 30 tham số); `↓` trên for-in bị bỏ qua, vòng không cạnh lùi không cần biến thể.
- **Hiệu ứng** (`effects.rs`): suy từ hiệu ứng khai báo của callee (trực tiếp, qua giá trị hàm, qua phương thức trait); E0801 suy ra ⊄ khai báo, E0802 impl ⊄ trait. 7 test.
- **Oracle** `term_oracle` (1500 chương trình, không âm tính giả; ngưỡng ≥300 chấp nhận, ≥200 bác đúng, mỗi mã ≥50 đều đạt: 318/627, E0701 608, E0702 1109, E0703 112). Chèn lỗi vào chứng minh biến thể: bắt ở #78.
- **Tài liệu/mẫu:** `docs/errors/E0701..E0703, E0801, E0802` (mỗi mã ≥3 fail + ≥3 pass, 30 mẫu mới), `DOC_REQUIRED` cập nhật, corpus kiểm mã M8 (`fail_cases_of_m8_give_exactly_their_code`, `every_pass_case_has_no_termination_or_effect_errors`).
- **Hồi quy:** gt-mir, gt-types, gt-borrowck, gt-oblig (gồm oracle M7 2000 chương trình) xanh sau các thay đổi `env.rs`/`analysis.rs`. Cổng đầy đủ chưa chạy (theo lệnh: sau nửa 2).
- **Đột biến:** xem KNOWN_LIMITATIONS "gt-oblig M8 nửa 1". Quyết định Q48–Q50 chờ duyệt (DECISIONS §6).

## M8 nửa 2 — bước 1: `spawn` thuần (2026-10-08)
- Trait `Send` tự động (`send.rs`), `spawn` trong prelude fixture, E0501 (5 fail + 4 pass, `docs/errors/E0501.md`), `⊐ Send for` bị E0313, thông dịch chạy `spawn` tuần tự (công cụ test). 8 test `gt-types/tests/spawn.rs`, 1 test `gt-mir` (worker abort chứng tỏ chạy với đối số).
- Đột biến `send.rs`: 22, lần chạy lại còn 1 tương đương. Hồi quy gt-types, gt-mir xanh; corpus gt-borrowck, gt-oblig xanh.
- Phát hiện E0205 oan khi thêm `spawn<A>(f, a: A)`: ĐÃ truy gốc và sửa ở commit `M3-fix` (xem dưới); không còn né bằng đổi tên.
- Chưa chạy cổng đầy đủ; chờ xong nửa 2.

## M3-fix: mã prelude chỉ thấy prelude (2026-10-08)
- **Gốc (không phải chữ hoa/thường, cũng không phải generics rò):** `module_scope_site`/`module_scope_lookup` (gt-resolve/src/walk.rs) luôn tra `[module hiện tại, gốc người dùng, prelude]`, kể cả khi đang duyệt MÃ PRELUDE. Hệ quả: (1) tên tệp/module của người dùng (vd `a.gt`) bị D7 so với tham số trong chữ ký prelude → E0205 oan trên `<prelude>`; mọi tệp tên trùng tham số của lib (`x`, `n`, `p`, `a`, `f`…) đều dính; (2) tên trong prelude có thể phân giải nhầm sang item cùng tên của người dùng.
- **Tái hiện tối thiểu:** prelude `ƒ idv(a: u32) → u32 ! ∅ { a }` + tệp `a.gt` (E0205), `b.gt`/`A.gt` sạch.
- **Sửa:** `scope_containers()` — mã prelude chỉ thấy `[module prelude, prelude]`. Test hồi quy `gt-resolve/tests/prelude_isolation.rs` (6 ca pass: tệp/module tên `x`/`n`/`p`; 4 ca fail: D7 vẫn áp dụng cho mã người dùng) và `gt-types/tests/adt.rs` (tệp `a.gt` với prelude `spawn<A>(…, a: A)`).
- Sai sót của tôi cùng đợt: commit `4a436e2` làm hỏng corpus gt-resolve (fixture prelude riêng chưa có `spawn`/`Send`) vì tôi không chạy gt-resolve; đã bổ sung fixture.

## M8 nửa 2 — bước 2: đóng điều kiện cứng mutability bên trong (2026-10-08)
- Lang item `cell` (`Cell<T>`, fixture gt-types), `adt_invariant` coi nó bất biến; `gt-borrowck/tests/interior_mut.rs` (chương trình tấn công + đối chứng); kịch bản `interior_mut` (KINDS 10 → 11) + oracle động hiểu `Cell::new/set/get`. 20 000 chương trình: 0 âm tính giả; phân loại thêm 1 nguyên nhân dương tính giả đã biết.
- Kiểm tắt bản sửa: 53 âm tính giả/3 000 chương trình. Đột biến 238 giết; 361/591/554 còn sống, có lập luận đơn điệu (ADR-027 §0), chờ Giang.
- `pure_preds` loại kiểu có thể chứa ô (`TypeDb::may_have_interior_mut`, 3 test); sửa một hồi quy do chính tôi gây ra trong đợt này (`Option<T>::is_some` bị loại oan vì coi mọi tham số kiểu là có thể chứa ô → chỉ tham số có ràng buộc trait mới bị loại).
- ADR-027 viết xong (lang item, Sync, `with` có phạm vi, cấp khoá `#level`, E0502/E0503, hiệu ứng đa hình `effect_poly`). Chưa cài Mutex/Atomic*.

## M8 nửa 2 — bước 3: Mutex, Atomic, Sync, thứ tự khoá (2026-10-08, ADR-027)
- **Lang item mới** (prelude fixture, hợp đồng ở ADR-027 §1): `send`/`sync` (tự động), `cell`, `mutex`, `atomic`, `atomic_int`, `ordering`; marker hiệu ứng `poly`/`detach` (chỉ lib/core, E1002 nếu ở nơi khác); `#level("n")`. `Mutex::with` có phạm vi; `spawn` dùng `detach`.
- **Kiểm:** `Sync`/`Send` tự động (send.rs, E0501; `■` static phải `Sync`); `lock(L)` suy từ `Mutex::with` + lan truyền xuyên hàm; **E0502** thứ tự khoá tăng nghiêm ngặt tại mỗi `with` (đầy đủ nhờ phạm vi); **E0503** memory ordering viết thẳng và tổ hợp hợp lệ (mức MIR); borrowck/M7 coi Mutex/Atomic cùng nhóm với Cell (bất biến, loại khỏi `pure_preds`).
- **Thông dịch MIR** (tuần tự, công cụ test, ngoài TCB) cài `Cell`/`Mutex::with`/`Atomic`/`spawn`; 1 test ngữ nghĩa (kết quả 45 tính tay).
- **Corpus:** E0501 mở rộng (4 fail + 4 pass mới), E0502 (4 fail + 4 pass), E0503 (4 fail + 4 pass); `docs/errors/E0502.md`, `E0503.md`; chương trình đa luồng đúng (static `Mutex`/`Atomic` + `spawn`) trong pass.
- **Đột biến** (1 worker, lô nhỏ): send.rs 21, ordering.rs 63, effects.rs phần khoá 23 — chi tiết và phần chưa giải quyết ở KNOWN_LIMITATIONS.
- Hồi quy: gt-resolve, gt-types, gt-mir, gt-borrowck (oracle 20 000), gt-oblig xanh. Cổng đầy đủ chưa chạy (một lần, ngay sau bước này).

## M8 đóng (2026-10-08): cổng đầy đủ + ADR M9
- **Cổng đầy đủ (chạy một lần sau nửa 2, `tools/gate.sh`): XANH** — fmt, clippy `-D warnings`, audit, **634 test pass / 0 lỗi** (M7-close: 576), check_expectations (757 tệp .gt, 63 mã lỗi, 184 tham chiếu test) + unittest script. Lần chạy đầu đỏ vì sổ mã `gt_diag::known_codes()` thiếu E0503; đã thêm và chạy lại.
- Kích thước: `target/` 1,8 GB, `.git/` 2,1 MB.
- ADR-028 (M9) viết xong, DỰ THẢO chờ duyệt (Q53–Q58).
- **GT_FULL8=1 (vét cạn 8 bit HOÀN TOÀN, chạy một lần khi máy rảnh): XANH** — `gt-oblig/tests/exhaustive.rs` 4 test pass, 665,9 s (~11 phút).

## M8 bổ sung (2026-10-09): kịch bản interior_mut với Mutex/Atomic thật
- Oracle động của borrowck hiểu `Mutex::new/with` và `Atomic::*`; bộ sinh thêm `interior_mut_mutex` (40%) và `interior_mut_atomic` (40%) trong kịch bản; KINDS 11 → 13; tỉ trọng kịch bản cân lại (`moves` 16% → 8%, `interior_mut` 4% → 9%, `slot` 5% → 6%) để mọi loại vượt ngưỡng 300.
- Độ phủ 20 000 chương trình: interior_mut 3 700/1 194/2 415; mutex 1 561/510/1 033; atomic 1 523/474/1 031 (tổng/chấp nhận+sạch/vi phạm); 0 âm tính giả; dương tính giả 4,41%.
- Tắt bản sửa: cả nhóm ⇒ 24 âm tính giả (bắt được); chỉ Mutex/Atomic ⇒ 0 (nói thẳng ở ADR-027 §0). Đo hướng 361/591/554: giống hệt bản gốc (0 / 127); đều CHẶT hơn theo mã.

## M9 (2026-10-09): gritc, cổng phán quyết, bộ quét secret — ĐÃ cài, chờ Giang duyệt
- **Crate mới:** `gt-driver` (thư viện + binary `gritc`), `gt-secrets`; M8 tách đúng bố cục kế hoạch: `gt-effects`, `gt-conc`, `gt-term` (commit `M8-fix`). Thông dịch MIR chuyển thành `gt_mir::interp` (bộ chạy tham chiếu, ngoài TCB) dùng chung cho `gritc run` và test.
- **Lệnh:** `gritc check|build|run|explain|fmt` (+ `--json`, `--deny-yellow`, `--checks=all|proven`, `-o`); `--version`. `build` ghi gói `*.gtmir` nguyên tử (MIR + attestation SHA-256 tự cài, có vector chuẩn); `run` chạy bằng thông dịch; mọi đầu ra ghi `backend: interpreter`.
- **Cổng phán quyết** (`verdict.rs`, `pipeline.rs`): ĐỎ/ICE ⇒ không có sản phẩm để ghi hay chạy (kiểu: `open_gate`/`take_gated` chỉ trả `Artifacts` khi XANH/VÀNG); pass bắt buộc gồm `m5.return_paths` (E0307) và `m7.const_bounds` (E0601); `selftest` canary kiểm E0601/E0307 còn bị bắt trước mọi phán quyết sạch; panic mỗi pass ⇒ ICE (E9901). **Mã thoát phân vùng** 0/1/2/3 (gritc) và 0/70/71/100+s (chương trình), ghi vào D17 và test cả hai phía ở mức tiến trình.
- **`--checks=all` mặc định** (hằng `DEFAULT_CHECKS`, có test khoá): không xoá Assert nào, chèn cả kiểm hợp đồng đã chứng minh; mọi mẫu ĐỎ M7 giữ nguyên mã ở cả hai chế độ (D25). Báo cáo JSON `checks.{mode,kept,deleted,would_delete}`.
- **Bộ quét secret:** S101 (mẫu), S102 (entropy), S201 (declassify), S202 (taint tới sink io); `#allow(Sxxx, "lý do")` bắt buộc lý do (E1401), `#allow` thừa ⇒ N1401, dấu vết `allowed` trong JSON; `--deny-yellow` nâng VÀNG thành ĐỎ.
- **Sơ hở M7 phát hiện và vá:** E1001 (`⚠` thiếu bằng chứng, con trỏ thô ngoài `⚠`) có trong bảng mã và mẫu nhưng chưa từng được cài — gritc từng chấp nhận chương trình đó. Cài `gt-oblig/src/unsafe_check.rs`; mutation phát hiện thêm lỗ hổng chữ ký trait (đã vá). Một mẫu `pass` cũ dùng mã `#allow(S103)` không tồn tại đã đổi sang mã thật (Q56).
- **Kiểm thử:** `gt-driver` 19 test trong tiến trình (corpus fail/pass/warn của mọi milestone qua driver THẬT: ≥ 300 mẫu fail đều ĐỎ đúng mã, mọi mẫu pass được chấp nhận, mọi mẫu E0601/E0307 bị từ chối) + 10 test mức tiến trình; `gt-secrets` 14; `gt-oblig` `keep_all` 4 + `unsafe_check` 5; schema JSON mở rộng (chỉ thêm trường) có test khoá.
- **Mutation:** xem KNOWN_LIMITATIONS "Đột biến M9" (phần quyết định: verdict 25→0 còn sống; pipeline 1 tương đương; cli 7 hiển thị; keep-all 0; unsafe_check và gt-secrets: số liệu cuối chưa đủ, nói thẳng ở đó).
- **Cổng đầy đủ (chạy lại 3 lần vì đỏ giữa chừng: sổ mã E0503 đã sửa ở M8; test schema của gt-diag; vector SHA-256 tôi chép sai — cài đặt đúng, khớp `hashlib`): XANH — fmt, clippy `-D warnings`, audit, 687 test pass / 0 lỗi, check_expectations (788 tệp .gt, 67 mã lỗi, 201 tham chiếu test). `target/` 5,8 GB, `.git/` 3,1 MB.**
- **ADR-029 (M10) đã viết, chưa có dòng mã M10 nào; dừng chờ duyệt.** Môi trường đo thật và LLVM 20.1.8 đã trích tại `D:\Tools\llvm-20.1.8`; xem `docs/ENVIRONMENT.md`.

## Handoff (2026-10-12)
- Viết `docs/HANDOFF.md` (dự án trong 5 dòng, tệp bắt buộc đọc, luật làm việc, trạng thái, việc tiếp theo theo thứ tự, môi trường, điều kiện cứng/đột biến chưa giải quyết, câu hỏi chờ). Từ đây mỗi bước nhỏ: commit + cập nhật mục "Trạng thái" và "Việc tiếp theo" của HANDOFF.
- Giang duyệt ADR-029 (LLVM 20.1.8 dựng sẵn, inkwell 0.10.0, liên kết động `LLVM-C`) kèm điều kiện: ghi SHA-256 gói LLVM; gritc cần `LLVM-C.dll` trên PATH khi chạy; binary do gritc sinh KHÔNG phụ thuộc `LLVM-C.dll`; `libxml2s.lib` rỗng chỉ là workaround. Q64: Giang tự chuyển paging file sang D: và restart.

## M10 (2026-10-12 → 2026-10-13): backend LLVM, IR Guard, gt-rt — ĐÃ cài, chờ cổng đầy đủ + Giang duyệt
- **Bước 0 spike** đạt: inkwell 0.10.0 + LLVM-C.dll liên kết động, nạp trễ; phát hiện `no-llvm-linking` bỏ shim C ⇒ tự định nghĩa 10 hàm `LLVM_Initialize*` (ADR-029 "kết quả spike").
- **Crate mới:** `gt-llvm` (liên kết + shim + re-export inkwell), `gt-codegen-llvm` (MIR→IR→object), `gt-guard` (IR Guard, E1101), `runtime/gt-rt` (workspace riêng, staticlib panic=abort). `gritc` có `--backend=llvm|interpreter`, mặc định llvm khi có feature `llvm`.
- **Hạ được:** số nguyên/bool/char, `Assert`→kiểm+abort (mã 70), `Unreachable`→abort, hàm/đệ quy/đơn hình hoá, enum/Option, tuple/mảng/struct, tham chiếu thường + béo (`&[T]`, `&str`), `static` (bí danh, khởi tạo trước main), `Atomic` (atomicrmw/cmpxchg/load/store nguyên tử với ordering hằng), `Mutex::with` (SRWLOCK qua gt-rt), `Cell`, `spawn` (luồng HĐH thật).
- **Test:** gt-driver/tests/llvm.rs 26 test chạy exe thật (đối chiếu thông dịch + giá trị cố định tính tay; abort 70 từng loại; ĐỎ ⇒ không exe; `dumpbin /dependents` không có LLVM; `check` chạy khi PATH không có LLVM; IR Guard mức tiến trình; golden IR 3 chương trình; 3 mẫu thật `tests/samples/*.gt`); gt-guard/tests/guard.rs 10 test IR dựng tay; gt-rt 3 test. Đa luồng: 4×1000 chạy 68/68 vòng đúng (TSan không có trên MSVC: KNOWN_LIMITATIONS).
- **Benchmark nền** (số thật): `docs/BENCHMARK.md` (exe -O0, mọi check giữ; ~20× chậm hơn Rust -O ở fib, ngang Rust -O0 ở vòng lặp/slice; thông dịch ~2000× chậm hơn exe).
- **Chưa làm / ghi thẳng:** hồ sơ `release` (-O2); đột biến `gt-guard`/`gt-codegen-llvm` (ADR đòi lô nhỏ 1 worker — chưa chạy vì tín dụng/thời gian; việc đầu tiên nếu Giang muốn); số thực, trait method, 128-bit mul/div (xem KNOWN_LIMITATIONS "M10").
- **Cổng đầy đủ M10 (2026-10-13, một lần):** fmt OK; clippy 0; audit 0; 723 test pass / 0 lỗi; check_expectations OK (788 .gt, 67 mã); gt-rt 3 test. Dừng chờ Giang duyệt.
- **M10 ĐƯỢC DUYỆT "đạt có điều kiện" (Giang, 2026-10-13):** điều kiện = chưa đột biến `gt-guard` và `gt-codegen-llvm`; chỉ có hồ sơ `dev` (-O0). Việc bù: (1) so sánh codegen ≡ thông dịch trên chương trình ngẫu nhiên; (2) đột biến chỉ `gt-guard`; (3) `release` cần ADR riêng (bộ tối ưu LLVM nằm trong TCB).

## M11 (2026-10-13): Rig v0 — XONG, cổng đầy đủ xanh (742 test)
- `crates/rig`: new/check/build/run/test/add/lock/fmt/clean, `--locked`, parser TOML con fail-closed, `rig.lock` băm SHA-256 từng gói, mã thoát riêng 10–13 (mã gritc chuyển nguyên), phụ thuộc path (vòng/trùng tên/diamond), luật tiền tố tên tệp (ADR-030). 11 test mức tiến trình + 4 unit; dự án 3 gói (hình thoi) build/run/test; tái lập SHA-256 đạt (dựng lại, sau clean, thư mục khác sau >1 s; bỏ `/Brepro` thì test ĐỎ). FB-1 luật bộ sinh; lô so sánh abort: 847 phép chạy khớp.

### M11 — số liệu đo thật (2026-10-13, lệnh yêu cầu ghi số)
- **Tái lập (SHA-256 của `app.exe`, dự án 3 gói base ← mid ← app, app phụ thuộc cả base và mid):** dựng lần 1 `512f18186d4b7921364e34488373d1fc95906483b661e5d600cdf51529a758d5`; dựng lần 2 (liền sau, cùng thư mục) `512f1818…a758d5`; sau `rig clean` rồi dựng `512f1818…a758d5`; chép dự án sang thư mục khác, chờ 2 s rồi dựng `512f1818…a758d5` ⇒ **4/4 giống hệt**. Độ nhạy: bỏ `/Brepro` thì ca "thư mục khác sau >1 s" ĐỎ (test `builds_are_reproducible_byte_for_byte`). Phạm vi tuyên bố: cùng máy, cùng bản LLVM 20.1.8, cùng MSVC link.exe 14.44, cùng `gt_rt.lib`; KHÔNG tuyên bố tái lập giữa máy/bản công cụ khác (chưa đo).
- **Dự án 3 gói:** `rig lock` ⇒ 3 `[[package]]` (base chỉ MỘT lần dù đi hai đường); `rig build --locked` xanh; `rig run` thoát 0; `rig test` "t_check ... ok", "1 qua, 0 hỏng", thoát 0; test có kết quả sai ⇒ "FAILED (abort: div_by_zero)" thoát 1.
- **Mã thoát của rig (đo bằng test mức tiến trình, `crates/rig/tests/rig.rs`):** gritc ĐỎ ⇒ 1 (chuyển nguyên, không có exe); đầu vào gritc từ chối (tên module không hợp lệ) ⇒ 3 (chuyển nguyên); `rig.lock` lệch (mọi lệnh build/check/run/test, có/không `--locked`) hoặc thiếu kèm `--locked` ⇒ 10; manifest/dự án sai (14 ca TOML, vòng, trùng tên/module, thiếu main) ⇒ 11; dùng sai lệnh/cờ ⇒ 12; IO của rig ⇒ 13.
- Số test: crates/rig 11 mức tiến trình + 4 unit; dogfood 3 (`crates/rig/tests/dogfood.rs`).

## Cổng đầy đủ sau dogfood/IR Guard I/O/Arena/JSON (2026-10-13) — dán số thật
Bốn lần chạy `tools/gate.sh`; ba lần đầu ĐỎ thật, mỗi lần sửa rồi chạy lại:
1. clippy `doc_lazy_continuation` ở gt-guard + 3 golden IR (prelude đổi ⇒ attestation đổi) — sửa.
2. clippy xong, nhưng test `llvm_diff::permanent_and_small_fixed_batch` lộ **FB-2** (IR Guard từ chối nhầm hàm hợp đồng tên Unicode) — sửa bằng lấy tên qua API; test hồi quy mới.
3. `gt-resolve` corpus ĐỎ: bản fixture prelude THỨ BA (gt-resolve/tests/common) thiếu `Out`/`root_out` — sửa.
4. **Lần cuối: fmt OK; clippy 0; audit 0; 756 test pass / 0 lỗi; check_expectations OK (793 file `.gt`: fail 392, warn 13, pass 362; 67 mã).**
Rút kinh nghiệm: prelude có BA bản (lib/core, fixture gt-types, fixture gt-resolve) — chỉ một cặp được test khoá bằng nhau (docs/KNOWN_LIMITATIONS: nên khoá cả gt-resolve).

## Bổ sung số liệu theo lệnh 2026-10-13: luật bộ sinh FB-1 và lô abort ít phủ
- **FB-1, luật bộ sinh:** oracle M7 (`gt-oblig/tests/oracle.rs`) nay chạy thêm `gt_borrowck::check_program` trên MỌI chương trình sinh ra (bộ sinh `obgen` đã sinh sẵn hàm có `⊳`/`⊲` cùng tham số lát/tham chiếu) và cấm E9901. KIỂM BẰNG TẮT BẢN SỬA: đổi `regions.rs` về `regions.get(&body.node)` ⇒ oracle ĐỎ ngay ở **seed #1** (thông báo "ICE trong borrowck `main::h1⊳0` … tham số không khớp RegionSig", 0.01 s); bật lại ⇒ XANH (198.59 s cả bộ). Lý do lọt lưới cũ ghi ở `docs/FIELD_BUGS.md` (FB-1): oracle M7 không chạy borrowck, oracle M6 không sinh hợp đồng, corpus không có tổ hợp.
- **Lô so sánh abort ít phủ** (`gt-driver/tests/llvm_diff_abort.rs`, lô 25 chương trình, ghi vào `docs/CODEGEN_DIFF_RESULTS.jsonl`, dòng `suite":"abort-targeted"`): 6 lô (index, divrem ×4, pre) = 150 chương trình, **847 phép chạy exe thật, 847 khớp thông dịch, 0 bất đồng**. Số lần abort đã so khớp: `index_out_of_bounds` **90**, `rem_by_zero` **64**, `div_by_zero` **38**, `precondition` **74** (mục tiêu ≥ 30 mỗi loại: đạt). Còn lại bình thường: chạy thường không abort.
- Cột "người duyệt" của `docs/TCB_INTRINSICS.md` giữ nguyên `CHỜ GIANG`.

## Công cụ 3 `sha256` (examples/sha256, 2026-10-13) — số liệu thật
- **Đúng:** 5 test (`crates/rig/tests/dogfood_sha256.rs`) ĐẠT; đối chiếu 227 tệp (5 vector NIST viết cứng + 20 độ dài biên đệm + 200 tệp ngẫu nhiên + 1 MiB + 32 MiB) với `hashlib` (Python/OpenSSL): 227/227 khớp từng ký tự. Lỗi: tệp thiếu ⇒ mã 2 + `…: no such file` ở stderr, stdout rỗng; thiếu đối số ⇒ mã 2 + `usage: sha256 <file>`.
- **Tốc độ** (LLVM -O0, mọi check giữ): 32 MiB trong 1.46 s ≈ 21.9 MiB/s.
- **Phép bit u32:** so sánh 3 nguồn (thông dịch, exe LLVM, đánh giá Rust của cây biểu thức), 4 lô × 25 chương trình ⇒ 352 lần chạy, 0 bất đồng (abort khớp: overflow 116, precondition 51; 185 chạy thường); độ nhạy: đổi `rotr`↔`rotl` ở codegen ⇒ lô ĐỎ, đã hoàn nguyên. Ghi ở `docs/CODEGEN_DIFF_RESULTS.jsonl`.
- **Check còn lại** (`gritc check examples/sha256/src/main.gt --json`): `--checks=all` **106** (overflow 34, index 42, `pre` 16, slice_range 7, cast_lossy 4, div_zero 2, rem_zero 1; 66 "sẽ bị xoá nếu proven"); `--checks=proven` **24** (index 18, overflow 5, cast_lossy 1).
- **Riêng phép dịch/xoay (`n < 32`):** có 16 vị trí (15 lượng dịch HẰNG + 1 lượng dịch KHÔNG hằng `((7 - j) * 4) as u32` ở `report`). **Cả 16 được M7 CHỨNG MINH** (solver interval+zone). Ở `--checks=proven`: **0 check `pre` còn lại** (đúng kỳ vọng: lượng dịch hằng ⇒ 0, và cả lượng dịch biến cũng 0). Ở `--checks=all` cả 16 vẫn được CHÈN theo chính sách (trạng thái `inserted_mode_all`, không phải `kept_not_proven`); đây là chính sách của `all`, KHÔNG phải thiếu chứng minh. Ở codegen, lượng dịch hằng làm IRBuilder gấp `n < 32` thành `br i1 true` (không có lệnh so sánh lúc chạy).
- **Khoảng trống/bug** (S1–S5, #18–#20) ở `docs/DOGFOOD.md`. Chưa kiểm: phép bit u64/u8 (chưa có), `release` (-O2).

## Cổng đầy đủ (lệnh 2026-10-13, mục 0d) — kết quả thật
- Lần chạy 1 (sau bước 1–3 + sha256, `tools/gate.sh`): gt-rt (workspace riêng) 3 test ĐẠT; fmt OK; clippy `-D warnings` EXIT 0; audit EXIT 0 (1309 advisory, 50 crate); **test: 167 đạt, 3 ĐỎ** — `golden_ir_checked_add_and_abort_blocks`, `golden_ir_slice_bounds`, `golden_ir_static_atomic_and_mutex` (llvm.rs:659); check_expectations OK (797 tệp .gt, 68 mã, 201 tham chiếu), unittest script 9 OK.
- Nguyên nhân: prelude lớn thêm (phép bit, `write_bytes`, `ErrOut`, `arg_count`) làm đổi băm attestation; golden chưa cập nhật. `GT_UPDATE_GOLDEN=1` rồi xem diff: CHỈ 6 dòng đổi, toàn bộ là `@gt.attestation` và `!0` (3 golden × 2); không dòng IR nào khác đổi.
- Sau sửa: chỉ chạy lại `cargo test -p gt-driver --test llvm`: 29/29 ĐẠT. **Chưa chạy lại toàn cổng** (45–60 phút); các phần khác của lần 1 đã xanh và mã không đổi ngoài 3 golden. Cổng đầy đủ tiếp theo: sau (v).

## Bước (i) lệnh ADR-032 — đơn hình hoá + gọi qua trait + tên symbol (2026-10-13)
- **Gọi qua ràng buộc generic `x.m()` (T: Tr) nay hạ được** ở codegen và chạy được ở thông dịch. MỘT nguồn chọn impl: `TypeDb::{select_impl, resolve_trait_method, fn_params}` (gt-types); mono, codegen, thông dịch (`Interp::with_types`) cùng dùng. Không có impl khớp ⇒ ICE E9901 (test chèn lỗi `GRITC_FAULT=types:select-impl`: gritc thoát 2, không exe; thông dịch cũng từ chối).
- **Số liệu test mới** (`gt-driver/tests/llvm_generics.rs`, 7 test, cả hai backend, số tính tay): 2258 (hai impl + impl generic + hàm generic lồng), 61; symbol ASCII + khác nhau + ánh xạ ngược Unicode trong `!gt.instances`; build hai lần ⇒ IR và exe giống từng byte; Guard từ chối hàm lạ; ICE khi `select_impl` hỏng. **Độ nhạy:** đổi `select_impl` thành "chọn impl đầu của trait" ⇒ 2 test ĐỎ (đã hoàn nguyên).
- **E0317** (`MAX_PROGRAM_INSTANCES` = 10 000, tách khỏi E0312): 3 fail + 3 pass (sát dưới cận: 8 191 và 9 841 instance, ~0,4 s kiểm); `check_expectations`: 803 tệp, 69 mã, 204 tham chiếu OK.
- **IR Guard luật thứ 6 (instance):** mọi tên hàm toàn ASCII `[A-Za-z0-9_.]`; mọi hàm định nghĩa (trừ `main`, `gt_entry`, `gt.tramp.<số>`) thuộc tập tính độc lập từ bảng instance của M4 (`expected_instances`; thêm `□`/`■` và hợp đồng prelude không generic, vì không nằm trong bảng); `!gt.instances` liệt kê ĐÚNG tập đã định nghĩa; phải có ≥1 instance. Test gt-guard: 22 (7 mới). Attestation nay băm cả tập instance (`mir_dump`: dòng `instances <sha256>`).
- **Đột biến (1 worker) trên hàm quyết định MỚI của gt-guard** (`support_fn`, `ascii_symbol`, `instance_violations`, dòng 41–47 và 108–159): **41/41 giết, 0 sống, 0 không hợp lệ** (kết quả ở `docs/MUTATION_GT_GUARD.jsonl`). Sau đó clippy buộc đổi `iter().any(==)` thành `contains` (một dòng, hành vi giữ nguyên; chưa chạy lại đột biến cho dòng đó — chỉ chạy lại test).
- **Cổng đầy đủ CHƯA chạy lại** sau bước này (chỉ clippy toàn workspace sạch, test gt-guard/gt-types/gt-mir/gt-driver/rig xanh). Chạy một lần sau (v) theo lệnh.
- **Khoảng trống phát hiện:** gọi trait trong thân impl generic bị M8 từ chối E0703 (KNOWN_LIMITATIONS, test khoá hành vi). Chưa kiểm: thời gian codegen ở cận 10 000 instance; trình gỡ lỗi với tên symbol mới.

## Bước (ii) lệnh ADR-032 — `Vec<'a, T>` trên Arena (2026-10-13/14)
- **Kiểm đầu tiên ĐẠT:** trường `&!'a Arena` trong ADT biểu diễn được; M6 xử lý đúng: hai Vec cùng Arena ⇒ E0402; reset/alloc Arena khi Vec còn dùng về sau ⇒ E0402/E0403; lát còn sống qua `push` ⇒ E0403; Arena dùng lại được sau khi Vec chết; hai Vec cần hai Arena. Không dùng `&Arena`, không mutability bên trong. 4 fail + 3 pass M6, snapshot chẩn đoán M6 thêm đúng 4 dòng.
- **Cài đặt:** prelude `Vec<'a,T>` + `new/push/len/as_slice/as_mut_slice` (lang `vec`, TCB §B3); codegen (`vec_push` gấp đôi tối thiểu 4, kiểm tràn `2·cap` và `cap·size`, `llvm.memcpy`; không hàm runtime mới); thông dịch (`vec_op`, Arena `new/reset`); abort `arena_exhausted` (kind 17, runtime + `GRITRT_ARENA_LIMIT` để kiểm thử); `Arena::alloc` dùng chung đường abort mới.
- **E0318** (`Vec<T>` với `T` cần drop): lỗi biên dịch, 4 fail + 4 pass, docs/errors/E0318.md; E0313 cho `⊐ Copy for Vec` (+1 fail) và literal Vec ngoài prelude.
- **Test mới `llvm_vec.rs` (10, hai backend khi so được):** tổng 1 499 500; struct u8/u64/u16 (3996); ZST (10); `Vec<&[u8]>` (24504); ghi qua `as_mut_slice` (44); index ngoài biên qua lát abort khớp; **10⁶ phần tử** trên exe (500 000 500 000); `arena_exhausted` với giới hạn 2 MiB ⇒ mã 70, bỏ giới hạn ⇒ chạy bình thường; `Arena::alloc` cùng kind; báo cáo check có mục `arena_exhausted`/`intrinsic_runtime` ở cả `all` và `proven`. **Độ nhạy:** bỏ `memcpy` khi tăng trưởng ⇒ 4 test ĐỎ (đã hoàn nguyên).
- **Số check:** mỗi `push`/`Arena::alloc` thêm đúng 1 mục `arena_exhausted` (không xoá được). `s ≔ v.as_slice()` rồi lặp cục bộ: `s[j]` KHÔNG được chứng minh; truyền lát cho hàm nhận `&[T]` thì chứng minh được (đo ở ADR-032 phụ lục B).
- **Chưa kiểm:** đường tràn `2·cap`/`cap·size`; thông dịch không mô phỏng hết Arena (chỉ kiểm exe); cổng đầy đủ chạy MỘT lần sau sortlines.
- Test các crate đổi: gt-types, gt-resolve, gt-borrowck, gt-mir, gt-oblig, gt-guard, gt-conc, gt-diag, gt-term, gt-effects, gt-secrets, gt-dataflow, gt-driver, rig, gt-codegen-llvm: xanh (3 golden IR + 1 snapshot M6 cập nhật có kiểm diff). Clippy workspace + gt-rt sạch.

## Bước (iii) lệnh ADR-032 — `Ord`/`Ordering` + sắp xếp trộn bằng Grit (2026-10-14)
- **Prelude:** `Ordering { Less, Equal, Greater }`, trait `Ord` (không impl sẵn). **`examples/sortlines/src/msort.gt`:** `sort<T: Ord + Copy>` trộn bottom-up ổn định + `cmp_bytes` (so sánh byte), Grit thuần, gọi qua trait (bước (i)) và `Vec` trên hai Arena (bước (ii)).
- **Đúng (`gt-driver/tests/llvm_sort.rs`, 4 test, ĐẠT; ~99 s):** so với `sort_by_key` ổn định của Rust (hash dãy (khoá, chỉ số gốc) ⇒ kiểm cả ổn định): biên n = 0,1,2,3 và 4…65 (luỹ thừa 2 và ±1; tăng dần/giảm dần/toàn bằng nhau) ⇒ 14×3 + 5 ca; 60 mảng ngẫu nhiên n ≤ 200 nhiều khoá trùng (20 ca đầu cũng chạy ở thông dịch ⇒ khớp exe); 20 000 phần tử trên exe; `cmp_bytes` 12 cặp (rỗng, tiền tố, byte cao 0x7f/0x80/0xff, UTF-8) ở cả hai backend. **Độ nhạy:** đổi `le` thành so sánh chặt (mất ổn định) ⇒ test ngẫu nhiên ĐỎ (đã hoàn nguyên).
- **Dừng (M8):** vòng trộn chứng minh được; hai vòng bước tiến không chặn dưới dùng nhiên liệu (đo: `lo ← lo + w + w`, `width ← width + width` ⇒ E0702 khi không có nhiên liệu).
- **Check còn lại** (mẫu 1 phần tử): `all` 33, `proven` 20 (`merge_runs` index 6, `pass` overflow 6, `sort` index 2 + overflow 1, `main` index 1, `arena_exhausted` 2, `fuel` 2).
- Test các crate liên quan (gt-types, gt-resolve, gt-mir, gt-borrowck, gt-oblig, gt-guard, gt-term, gt-effects, gt-secrets, gt-conc, gt-driver, rig) không-fail-fast: 0 đỏ (3 golden IR đổi attestation). Clippy gt-driver sạch.
- **Chưa kiểm:** phần tử to hơn 16 byte; thời gian ở 10⁶ dòng (đo ở sortlines); `T` không `Copy` (sort chỉ cho `Copy`).

## Bước (iv) lệnh ADR-032 — so sánh generic/trait/Vec với kỳ vọng ĐỘC LẬP (ADR-034), 2026-10-14
- **Bộ sinh** `gt-driver/tests/llvm_diff_gtv.rs`: 3 nguồn (mô hình Rust của bộ sinh — bảng impl riêng, bộ đánh giá hàm generic theo cấu trúc, không gọi `select_impl`; thông dịch; exe LLVM). Loại: generic không ràng buộc (`idt`, `swp`), gọi qua ràng buộc (`x.m()` và `Tr1::m(x)`), impl generic (`⊐ <T> Tr1 for W<T>`, `⊐ <T: Tr1> Tr2 for W<T>`, W lồng 2 tầng), nhiều trait (cùng tên `m` ở hai trait, `T: Tr1 + Tr2`, hàm gọi hàm nhiều tầng), `Vec` (đẩy/ghi qua `as_mut_slice`/đọc/độ dài; phần tử u8,u16,u32,u64,i32,i64,bool,struct `{u8,u64,u16}`, tuple `(u8,u32)`; 1–70 phần tử ⇒ nhiều lần tăng trưởng), `Vec` chỉ số ngoài biên (abort `index_out_of_bounds`, ba nguồn), `Vec` cạn Arena (exe-only, `GRITRT_ARENA_LIMIT` 2 MiB; kỳ vọng theo vùng không mơ hồ).
- **9 lô × 25 = 225 chương trình** (ghi ở `docs/CODEGEN_DIFF_RESULTS.jsonl`, suite `generic-trait-vec`): đã so sánh được bound_call 33, generic_unbounded 32, impl_generic 32, multi_trait 32, vec_ops 32, vec_index_oob 32, vec_arena_exhausted 32 — **mọi loại ≥ 30 (ngưỡng ADR-034)**; **bị bỏ: 0** (không có ĐỎ, không chưa-hạ-được) ⇒ tỉ lệ bỏ 0% mọi loại; **bất đồng: 0**.
- **Độ nhạy (luật bộ sinh, bắt buộc):** `select_impl` đổi thành "chọn impl đầu của trait" (`GT_GTV_FIRST=1`, lô 1–25) ⇒ **10/25 bất đồng** (bound_call, impl_generic, multi_trait; ghi dưới suite `generic-trait-vec-SENSITIVITY-select-first`); bộ sinh tự phát hiện. Đã hoàn nguyên (cờ chỉ bật trong test, thread-local).
- **Số tính tay vẫn giữ** ở `llvm_generics.rs` (2258, 61, 19) để giảm rủi ro hiểu nhầm ngữ nghĩa chung giữa mô hình và bộ sinh (cùng một người viết).
- **Chưa kiểm:** gọi trait trong thân impl generic (M8 từ chối, ADR-033); phần tử Vec kiểu mảng/enum/Option; hai Vec cùng chương trình với khoá khác kiểu; thông dịch không mô phỏng hết Arena (kịch bản cạn chỉ exe).

## Mục 1 lệnh sortlines — kiểu phần tử của sortlines (2026-10-14)
- **4 lô × 25 = 100 chương trình** (suite `generic-trait-vec-elem`, `docs/CODEGEN_DIFF_RESULTS.jsonl`), ba nguồn (mô hình Rust, thông dịch, exe): **`vec_big_sort` 50/50 so sánh được, `vec_line_sort` 49/50**; bị bỏ 1 (`vec_line_sort`: ĐỎ E0301 — chương trình 0 dòng nên kiểu phần tử của `Vec` không suy ra được; tỉ lệ bỏ 2%); **bất đồng 0**. Ngưỡng ≥ 30 đạt cho cả hai loại.
- `vec_line_sort`: `Vec<Line<'a>>` (16 byte, `⊐ Ord` so byte), push/đọc/sắp xếp bằng `sort`; kỳ vọng `Vec<Vec<u8>>::sort`. `vec_big_sort`: struct 24 và 32 byte căn lề hỗn hợp (4 bố cục), push/ghi/sắp xếp, kỳ vọng `sort_by_key` ổn định.
- **1d:** thêm `wide_elements_24_and_32_bytes_match_stable_sort_at_scale` (`llvm_sort.rs`): 4 bố cục × 50 000 phần tử trên exe, khoá trong 0..200 (rất nhiều trùng), khớp `sort_by_key` ổn định ở cả 4 (5,8 s).
- **Gặp khi làm:** FB-3 — `o: &Line<'a>` trong `⊐ <'a> Ord for Line<'a>` bị E0311 oan (SAI TÍCH CỰC; né bằng `&Self`); hồ sơ ở FIELD_BUGS, chưa sửa.
- **Chưa kiểm:** phần tử có enum/Option/mảng; kích thước thật `size_of` (không có trong Grit — suy từ bố cục LLVM tự nhiên).

## Bước (v) lệnh ADR-032 — công cụ thứ tư `sortlines` (2026-10-14)
- **Công cụ:** `examples/sortlines` (Grit: `main.gt` + `msort.gt`, dự án Rig; README ghi quy ước). Thêm `Vec::reserve(n)` (intrinsic TCB §B3, một lần cấp khối, cùng đường kiểm tràn/hết Arena với `push`) để có công thức cỡ Arena: mỗi Arena đúng một khối `max(1 MiB, số_dòng·16 + 8)`.
- **Đúng (7 test `rig/tests/dogfood_sortlines.rs` ĐẠT, khớp TỪNG BYTE với tham chiếu Rust `sort_by` theo byte):** 15 ca nhỏ (rỗng, `\n`, một dòng, dòng rỗng xen kẽ, CRLF, không `\n` cuối, UTF-8/byte cao/NUL, trùng, tiền tố, `\r` đơn); một dòng dài 1 MiB ×3; 1 MiB và 32 MiB văn bản ngẫu nhiên; 10⁶ dòng ngắn; 4 MiB toàn dòng rỗng; đường lỗi (mã 2, stderr); công thức Arena KIỂM CHẶT (giới hạn = khối ⇒ chạy đúng; ít hơn 1 byte ⇒ abort có loại `arena_exhausted`, mã 70). Thêm 2 test `llvm_vec`: `reserve` giữ nội dung/hai backend khớp; nhu cầu Arena đúng một khối.
- **Ổn định:** KHÔNG quan sát được ở sortlines (so toàn dòng) ⇒ không tuyên bố; đã kiểm ở (iii).
- **Hiệu năng (-O0, mọi check giữ, ống pipe; không phải cổng):** 1 MiB 0,31 s (×7,2 so Rust `sort_by`); 32 MiB 7,24 s (×3,6; đỉnh 86 MiB); 10⁶ dòng ngắn 3,63 s (×6,8; 39 MiB); 4 MiB toàn dòng rỗng 7,20 s (×27; 136 MiB). Chi tiết và phương sai chưa đo ở DOGFOOD.
- **Khoảng trống/bug:** DOGFOOD T1–T8; FB-3 (E0311 oan, né `&Self`); ghi stdout tới tệp chậm (flush mỗi lời gọi) — KNOWN_LIMITATIONS.
- **Chưa kiểm:** tràn thật của `nl·16`/`n·size`; 32 MiB toàn dòng rỗng (cố ý KHÔNG chạy, công thức bộ nhớ ở KNOWN_LIMITATIONS); đo lặp nhiều lần; bộ nhớ Rust chỉ ước tính.
- Test các crate liên quan (gt-types, gt-resolve, gt-mir, gt-borrowck, gt-oblig, gt-guard, gt-term, gt-effects, gt-secrets, gt-conc, gt-driver, rig) không-fail-fast: 0 đỏ. Clippy workspace sạch.

## CỔNG ĐẦY ĐỦ sau bước (v) và E0703 (2026-10-14) — kết quả thật (`bash tools/gate.sh /d/Grit`)
- **Lần chạy HỢP LỆ:** gt-rt (workspace riêng) OK; `fmt` OK; `clippy -D warnings` EXIT 0; `audit` EXIT 0; **`test --workspace`: 803 đạt, 0 đỏ, 149 bộ test (SUITES=149)**; `check_expectations`: 822 tệp .gt (fail 407, warn 13, pass 376, lexer 8), 70 mã, 215 tham chiếu test, OK; unittest script 9 OK.
- **Sự cố của chính cổng (đã sửa):** lần chạy đầu giai đoạn `test` KHÔNG biên dịch được (`link.exe: Access is denied`, lỗi thoáng qua của hệ điều hành) nhưng `gate.sh` vẫn in `TEST_PASSED= FAILED=` (rỗng) và các giai đoạn khác xanh ⇒ trông như xanh. Sửa `tools/gate.sh`: in thêm `SUITES=<số bộ test>` và `TEST_STAGE_INVALID` khi không có dòng `test result`. Lần đầu bị bỏ, lần hai là bản ghi ở trên. (Bài học: cổng không được xanh khi không có kết quả test.)
- Không có test đỏ nào phải sửa trong lần này (3 golden IR và 1 snapshot M6 đã cập nhật có kiểm diff ở các bước trước).

## Xác nhận và dọn theo lệnh 2026-10-14 (mục 0)
- **0a — số liệu lô phần tử và sortlines (đã có ở các mục "Mục 1 lệnh sortlines" và "Bước (v)" phía trên; xác nhận + cập nhật):** lô phần tử: 4 lô × 25 = 100 chương trình, `vec_big_sort` (struct 24/32 byte) 50/50 và `vec_line_sort` (`Line<'a>`, lát `&[u8]`, 16 byte) 49/50 so sánh được, bị bỏ 1 (ĐỎ E0301, chương trình 0 dòng), 0 bất đồng; sắp xếp rộng 4 bố cục × 50 000 phần tử khớp. sortlines: 7 test khớp TỪNG BYTE với tham chiếu Rust (15 ca nhỏ, dòng dài 1 MiB, 1 MiB và 32 MiB, 10⁶ dòng, 4 MiB toàn dòng rỗng, lỗi, công thức Arena). **Tính ổn định KHÔNG quan sát được ở sortlines** (so toàn dòng ⇒ hai dòng bằng nhau về byte không phân biệt được); không tuyên bố đã kiểm ổn định ở sortlines; ổn định của `sort` kiểm riêng ở `llvm_sort.rs` (hash theo chỉ số gốc). **Hiệu năng ĐO LẠI sau 0d (ghi một lần):** 1 MiB 0,09 s (×2,4 so Rust `sort_by`), 32 MiB 2,43 s (×1,2; đỉnh 118 MiB), 10⁶ dòng 0,75 s (×1,4; 42 MiB), 4 MiB toàn dòng rỗng 0,99 s (×3,8; 139 MiB); bảng cũ (×3,6–×27) bị thay vì phần lớn là syscall ghi từng dòng.
- **0b — `tools/gate.sh` thoát ≠ 0:** TRƯỚC đây script KHÔNG có mã thoát (chỉ in); nay mọi giai đoạn đỏ hoặc không hợp lệ ⇒ `exit 1` và dòng cuối `GATE_RESULT=RED` (xanh ⇒ `GATE_RESULT=GREEN`, mã 0). Test cho chính cổng: `tools/test_gate.py` (5 test, chạy được không cần cargo bằng `GATE_STAGES=test` + `GATE_TEST_CMD` giả lập): giai đoạn test không biên dịch được (mô phỏng `link.exe: Access is denied`) ⇒ RED + `TEST_STAGE_INVALID`; không có dòng `test result` dù lệnh thoát 0 ⇒ RED; có test đỏ ⇒ RED; kết quả hợp lệ không đỏ ⇒ GREEN. Cổng tự chạy `test_gate.py` ở giai đoạn `expect`.
- **0c — FB-3:** FAIL-CLOSED (từ chối thứ đúng, không chấp nhận thứ sai) ⇒ không phải sửa trước; ghi HANDOFF.
- **0d — sortlines gom đầu ra:** `Vec<u8>` trên Arena thứ ba + MỘT `write_bytes`; kèm sửa `gritrt_write_str/ewrite_str` an toàn với lát rỗng null (xem ADR-031). 7 test đạt.

## E0703 / ADR-033 — đồ thị gọi theo INSTANCE cho M8 (2026-10-14, mục 1 lệnh)
- **Thiết kế (ADR-033 phụ lục):** nút = instance `(hàm, đối số kiểu)`; cạnh `TraitMethod` nối đúng MỘT impl nhờ `TypeDb::resolve_trait_method`; chu trình instance thật ⇒ E0703 như trước; không chu trình ⇒ không đòi `↓`. Lùi về đồ thị theo nút bảo thủ nếu không dựng được (đo: 0 lần lùi trong mọi lô). Điểm mù chung với thông dịch/codegen (`select_impl`) nêu trong ADR; giảm bằng bộ sinh độc lập.
- **Mẫu corpus (đổi hành vi M8):** pass 4 (`e0703_forwarder_trait_call_in_generic_impl`, `..._free_fn_...`, `e0703_nested_wrappers_w_w_s`, `e0703_two_traits_alternating_no_cycle`); fail 3 mới + giữ nguyên fixture cũ (`E0703_trait_cycle_through_generic_impl`, `_two_traits_`, `_three_levels_`): vòng THẬT vẫn bị từ chối. Test: `llvm_generics` (forwarder chạy đúng hai backend: 31 và 3; vòng thật bị từ chối), `gt-term/tests/instance_graph.rs` (4), `conservative_fallback.rs` (cận số nút chính xác, đường bảo thủ không bao giờ chấp nhận thứ đường chính từ chối).
- **Bộ sinh `term_graph` (độc lập `select_impl`; mô hình = bảng impl của bộ sinh):** 7 lô ⇒ **175 chương trình: 61 không chu trình (XANH + giá trị khớp ở thông dịch và exe LLVM) và 114 có chu trình (ĐỎ, mọi lỗi là E0703); 0 bất đồng; 0 lần lùi về đồ thị bảo thủ.** Độ nhạy (40 chương trình mỗi chế độ): đổi `select_impl` thành "chọn impl đầu" ⇒ **9 bất đồng**; bỏ mọi cạnh trait (fail-open cố ý, chốt `set_drop_trait_edges_for_test`) ⇒ **9 bất đồng** (chu trình không qua cạnh trait vẫn bị bắt nên không phải mọi ca cyclic lệch). Ghi ở `docs/CODEGEN_DIFF_RESULTS.jsonl` (suite `generic-trait-vec-term*`). Lô nhỏ cố định 30 chương trình vào cổng (`tiny_term_graph_batch`).
- **Đột biến `gt-term` mã mới:** 91 đột biến (1 worker): **73 giết, 15 tương đương có lập luận (KNOWN_LIMITATIONS), 2 ở mã cũ ngoài phạm vi, 1 không biên dịch**. Hai test được THÊM vì đột biến sống sót bộc lộ lỗ hổng thật (đều theo hướng fail-open): SCC liên hàm dùng chung số hiệu (đột biến `ra != rb` không bị bắt ⇒ đệ quy tương hỗ không giảm có thể lọt) ⇒ `mutual_recursion_variants_are_checked_across_the_whole_scc`; hàm `div` đứng trước hàm đệ quy thường (`continue→break`) ⇒ `a_div_function_does_not_hide_later_recursive_functions`; cạnh trait cạnh cạnh trực tiếp trong SCC (`any→all`) ⇒ `a_trait_edge_in_the_scc_is_flagged_even_next_to_a_checked_direct_edge` và bản bảo thủ (đòi đúng một chẩn đoán nêu `f`). Công cụ: `tools/mut.py` nay chép cả `examples/` (cần cho test dùng `msort.gt`); dùng `GRIT_RT_LIB` cho worker.
- **Chưa kiểm:** đồ thị instance với giá trị hàm (`Callee::Value`) và hàm generic bị thoát (chỉ phủ bằng test bảo thủ/đơn vị); cổng đầy đủ chạy sau (ii) theo lệnh.

## Bước (i) lệnh sau Q72 — `Text<'a>` (String Phase 1), 2026-10-14
- **Cài đặt:** prelude `Text<'a>` (bộ đệm UTF-8 trên MỘT Arena: `new`, `push_str`, `push_char`, `push_utf8`, `from_utf8 → Option`, `len`, `as_str`, `as_bytes`) và `str::{len, slice}`; codegen (`vec_extend`, mã hoá UTF-8 1–4 byte, `str_slice` kiểm biên ký tự), thông dịch (`V::Str`, `Deref` của `V::Str` giữ chỗ chứa), runtime `gritrt_utf8_valid` (= `std::str::from_utf8` của Rust, nguồn kiểm RFC 3629 ĐỘC LẬP với bộ sinh); IR Guard bảng `RUNTIME` 22 → 23. Hợp đồng TCB §B4; ADR-035 phụ lục (đóng băng bằng mượn, đường tạo hợp lệ duy nhất, nghĩa vụ cắt biên ký tự, danh sách SẼ KHÔNG chứng minh được).
- **M6 / mã lỗi:** 4 mẫu fail mới (E0403 ×3: khung nhìn `as_str`/`as_bytes` còn sống qua `push_str`/`push_char`/`push_utf8`; E0402 ×1: hai Text một Arena), 1 fail E0313 (`⊐ Copy for Text`), 3 pass (dựng-rồi-đóng-băng, khung nhìn hết sống trước khi dựng tiếp, `from_utf8` + `slice`). Không cần mã lỗi mới (E0313/E0402/E0403 đủ); snapshot chẩn đoán M6 thêm đúng 4 dòng. Báo cáo "check còn lại": `Text::{push_str,push_char,push_utf8,from_utf8}` ⇒ `arena_exhausted`, `str::slice` ⇒ `str_slice` (trạng thái `intrinsic_runtime`).
- **Bộ sinh ba nguồn (mô hình Rust của bộ sinh / thông dịch / exe), `GT_GTV_SET=text`, 4 lô × 25 = 100 chương trình, `docs/CODEGEN_DIFF_RESULTS.jsonl` (suite `generic-trait-vec-text`):** `text_build` 33/33 (dựng ngẫu nhiên từ mảnh chuỗi 1–4 byte và ký tự U+0061…U+10FFFF), `text_utf8` 34/34 (`from_utf8` + `push_utf8` trên byte hợp lệ/bị làm hỏng: cắt giữa ký tự, 0xC0/0xC1/0xF5/0xFF, surrogate ED A0 80, > U+10FFFF, overlong, byte thô; kỳ vọng = `std::str::from_utf8` của Rust), `text_slice` 33/33 (cắt tại biên hợp lệ, giữa ký tự, ngoài biên, `from > to`: abort `slice_range` ở cả hai backend); **bị bỏ 0; bất đồng 0; mọi loại ≥ 30**. Độ nhạy: đổi byte đầu 3-byte `0xE0 → 0xE1` ở `push_char` ⇒ 1 bất đồng/25; đảo điều kiện biên ký tự ở `str_slice` ⇒ 4 bất đồng/25 (đã hoàn nguyên).
- **Test các crate liên quan** (gt-types, gt-resolve, gt-mir, gt-oblig, gt-guard, gt-term, gt-effects, gt-driver, rig): xanh sau khi cập nhật snapshot M6; clippy workspace + gt-rt sạch; `check_expectations` 835 tệp OK.
- **Chưa kiểm:** `Text` rất lớn (≥ 10⁶ byte) và `push_char` hàng triệu lần trên exe; `from_utf8` trên tệp lớn (kiểm ở công cụ thứ năm); không có `str::as_bytes`; hết Arena của `Text` chỉ dùng chung đường `vec_grow_to` đã kiểm ở `Vec` (không có test riêng cho Text); cổng đầy đủ chạy sau (ii).

## Bước (ii) — công cụ thứ năm `wordfreq` (2026-10-14)
- **Công cụ:** `examples/wordfreq` (Grit, dùng `Text::from_utf8` kiểm UTF-8, `msort`, `Vec`, `Text` dựng đầu ra rồi ghi MỘT lần); README ghi quy ước. UTF-8 sai ⇒ lỗi CÓ KIỂM SOÁT (stderr, mã 2), không abort.
- **Đúng (5 test `dogfood_wordfreq.rs` ĐẠT, khớp TỪNG BYTE với tham chiếu Rust HashMap+sắp xếp):** 14 ca nhỏ; 9 ca UTF-8 sai; 1 MiB và 32 MiB; từ 1 MiB; 10⁶ lần `a`; 200 000 từ phân biệt; `msort.gt` của hai công cụ giống hệt (test khoá).
- **Hiệu năng (-O0, mọi check giữ; không phải cổng):** 1 MiB ×4,2 (đỉnh 12 MiB), 32 MiB ×4,8 (7,23 s; đỉnh 234 MiB ≈ 7× tệp), 4 MiB/200 000 từ ×4,0 (đỉnh 33 MiB).
- **Bug thật gặp:** FB-4 (`u8 as char` không hạ được ở codegen): test hồi quy TRƯỚC (đỏ khi bỏ sửa), commit riêng 49e7bc1. Khoảng trống W1–W7 ở DOGFOOD (không có HashMap theo lệnh; không có `Display`; `Text::from_utf8` sao chép; …).
- **Chưa kiểm:** tệp UTF-8 > 100 MiB; số từ phân biệt > 10⁶; bộ nhớ của tham chiếu Rust không đo; phương sai của phép đo; cổng đầy đủ chạy sau bước này.
