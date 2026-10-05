# DECISIONS (ADR ngắn: bối cảnh – quyết định – hệ quả)

## ADR-001 — Cấu trúc workspace & toolchain (M0)
- **Bối cảnh:** plan §6.1 yêu cầu workspace nhiều crate; M0 cấm viết compiler.
- **Quyết định:** `Cargo.toml` workspace gốc, `resolver = "3"`, edition 2024, `unsafe_code = forbid` và clippy `all = deny` ở mức workspace; `members = []` cho tới khi từng crate được tạo ở milestone của nó. Toolchain hiện tại: rustc/cargo 1.98.1.
- **Hệ quả:** không có crate "rỗng giả vờ chạy". Ngoại lệ FFI LLVM (M10) cần ADR riêng.

## ADR-002 — Bảng ký hiệu & đo token (M0) — trạng thái: CHƯA đóng băng chính thức
- **Bối cảnh:** plan §2.3/§6.2 yêu cầu đo token trước khi chốt. Môi trường không có tokenizer của Claude; dùng `tiktoken` (cl100k, o200k) làm **proxy** (`tools/measure_tokens.py`, 18 chương trình `tests/pass`). Giang chấp nhận proxy làm tham khảo (Q2).
- **Số đo (proxy, KHÔNG phải Claude):** mỗi ký hiệu canonical tốn 2–4 token. Trên toàn bộ mẫu ASCII alias tốn **nhiều hơn** canonical: +4,0% (cl100k), +6,1% (o200k). Nhưng 8/37 (cl100k) và 14/37 (o200k) ký hiệu riêng lẻ đắt hơn alias (chủ yếu ký hiệu hiếm `⎇ ⌥ ↦ ⏎ ⊏ ⊐ ⊠ ⊡ ⊔ ⊢ ⊳ ⊲`: 4 token so với 3).
- **Quyết định (Giang, Q2):** bỏ `⊤ ⊥`; hằng boolean là từ khoá ASCII `true`/`false` (không có dạng canonical riêng).
- **Quyết định (Giang, Q3):** quy trình duyệt ký hiệu M0★ từng ký hiệu một; bảng `docs/spec/symbols.md` **chưa đóng băng** cho tới khi Giang duyệt xong từng dòng M0★.
- **Quyết định (Giang, Q5):** `μ` là ký hiệu dành riêng — luôn là token `MUT`, không bao giờ gộp vào định danh (`μs` → `MUT`,`IDENT(s)`); không định danh nào chứa `μ`. Test: `tests/lexer/mu_split.*`, `docs/spec/semantics.md` §12.
- **Hệ quả:** nếu số đo Claude khác, chỉ đổi bảng ánh xạ lexer (alias↔canonical); AST không đổi vì thiết kế 1–1. Tên biến chứa `μ` (kể cả giữa chữ) không dùng được.

## ADR-003 — Khởi động lại repo từ M0 (M0)
- **Bối cảnh:** repo từng có seed cũ (P8k, cú pháp `▶ Δ ↻`, codegen ELF trực tiếp) mâu thuẫn plan.
- **Quyết định (Giang, Q1):** xoá sạch, không giữ tag/lịch sử. Lịch sử git đã được reset: hiện chỉ có commit `4c9b489 init: clean workspace for Grit`. Dự án bắt đầu hoàn toàn mới từ M0.
- **Hệ quả:** `break/continue` `⊠ ⊡` giờ chỉ là đề xuất M0★ độc lập, không còn "kế thừa" từ đâu.

## ADR-004 — Mã lỗi E0305 và E0310 (M0)
- **Quyết định (Giang, Q4):** `match` không đầy đủ = **E0305**; thiếu capability = **E0310**, đúng như `docs/spec/error_codes.md`.

## ADR-005 — Công việc hoãn sang AI khác (M0)
- **Quyết định (Giang, Q6):** hoãn `.github/workflows/TASK.md` (CI), `deny.toml.TASK.md`, `docs/errors/TASK.md`, `crates/TASK.md` (và các TASK.md thư mục rỗng). `tests/TASK.md` **không** hoãn: đã làm thật (`tools/check_expectations.py`, `tests/README.md`) và xoá.
- **Hệ quả:** M0 chỉ coi là xong khi Giang xác nhận; các việc hoãn được liệt kê trung thực ở `PROGRESS.md`.

## ADR-006 — Duyệt 13 ký hiệu M0★ và bổ sung ký hiệu (M0, Giang 2026-10-05)
- **Quyết định:** duyệt cả 13 ký hiệu M0★. `*` (deref/nhân), `self`, `as` giữ ASCII; `self` là từ khoá cứng (E0101 khi dùng làm tên). Bổ sung: `≡`/`\type` (type alias), `□`/`\const`, `■`/`\static` (bất biến ở Phase 1 — đề xuất của agent, chờ xác nhận), `\extern` dành riêng (token `EXTERN`, E0101 nếu dùng). Lifetime dùng `'a` (ASCII-only), thay `@r` của bản nháp trước; lexer phân biệt `'a'` (char) và `'a` (lifetime) theo semantics §15.
- **Không dùng mã lỗi mới:** vi phạm từ khoá cứng/ký hiệu dành riêng gộp vào E0101.
- **Còn mở:** Q7 (where), Q8 (dấu kết thúc), Q9 (toán tử bit). Bảng ký hiệu chưa đóng băng.

## ADR-007 — Q7, Q8, Q9 và comment (M0, Giang 2026-10-05)
- **Q7:** ràng buộc trait inline `<T: A + B>`; dành riêng `\where` (token `KEYWORD_WHERE`, Phase 2); `for` từ khoá cứng ASCII; không dùng `|`.
- **Q8:** `;` tường minh kiểu Rust; hàm trả khác `()` mà khối kết thúc bằng `;` không có `⏎` → E0307; `block_stmt` không cần `;`.
- **Q9:** toán tử bit là hàm `band/bor/bxor/bnot/shl/shr` trong `lib/core` (M12), không có ký hiệu riêng.
- **Comment:** `//` bỏ qua (`LINE_COMMENT`); `///` giữ lại (`DOC_COMMENT`, gắn node kế tiếp, dùng ở Rig 8.F); `/* */` cấm ở Phase 1 (E0101).
- **Bổ sung vòng cuối (giữ mở, chờ duyệt):** Q10 for-in, Q11 while, Q12 never/kiểu hàm, Q13 con trỏ thô, Q14 điểm agent tự chọn. Từ khoá cứng nay gồm `self Self as for true false`.
- **Hệ quả:** lexer cần luật nhìn trước cho `'a'`/`'a`, sau `.` chỉ đọc chữ số, `/*` luôn là comment khối. Trên Windows tên file không phân biệt hoa/thường nên `tools/check_expectations.py` kiểm va chạm tên.

## ADR-008 — Q10–Q14 (M0, Giang 2026-10-05)
- **Q10:** for-in là `⟳ x in xs { }`; `in` là từ khoá cứng ASCII (khác `∈`/`\in`). Termination trên range/slice hữu hạn được suy tự động.
- **Q11:** `⟳ cond { }` là while dùng chung `⟳`; không cond = vô hạn (bắt buộc `div`); termination suy từ cond, `↓` tuỳ chọn và được xác minh nếu có.
- **Q12:** `#noreturn` chỉ cho intrinsic trusted trong `lib/core`; kiểu hàm tĩnh `ƒ(A) → R ! E`; kiểu never `!` thêm vào `prim`.
- **Q13:** con trỏ thô `*□ T` / `*μ T` (ASCII `*\const T` / `*\mut T`); dùng ngoài `⚠` → E1001 (không thêm E1002).
- **Q14:** duyệt mọi điểm agent tự chọn, ngoại trừ: range mở `a..`, `..b`, `..` có ở Phase 1 (slice index + pattern); đứng độc lập chưa có ngữ nghĩa (chờ iterator Phase 2).
- **Cách hiểu của agent (Giang xem lại nếu khác ý):**
  1. "Termination của while suy từ cond, không bắt buộc ↓" được hiểu là *bộ kiểm suy ra biến thể từ dạng mẫu (cond so sánh biến với cận + bước hằng)*; không suy ra được và không có `↓` → vẫn E0701.
     Lý do: D12/P3 (fail-closed). Hệ quả: test `E0701_loop_no_variant` được viết lại (vòng cũ `⟳ i < 10 { i ← i +% 1 }` nay suy ra được nên hợp lệ).
  2. `#noreturn` (khẳng định, chỉ intrinsic) và `→ !` (kiểu) cùng tồn tại: hàm Grit thường có thể trả `!` nếu compiler chứng minh mọi đường phân kỳ.
  3. Yêu cầu "semantics §7 / §Q12 / §Q13" được đặt ở §21 / §22 / §23 (vì §7 là Secret).
  4. Range mở độc lập bị từ chối bằng E0301 (mã chọn vì "sai kiểu / chưa có ngữ nghĩa"); không thêm mã mới.
- **Hệ quả cho lexer:** `!` phát BANG cho cả hiệu ứng lẫn kiểu never (parser phân biệt theo vị trí); `in` chỉ khớp nguyên từ (`index` vẫn là IDENT).

## ADR-009 — Phụ thuộc ngoài, fuzz harness và cách hiểu D15 của lexer (M1)
- **Phụ thuộc ngoài (TCB mở rộng, plan §3.3):** `unicode-ident` (XID_Start/XID_Continue, UAX #31), `unicode-normalization` (kiểm NFC; kéo theo `tinyvec`), `unicode-script` (script từng ký tự cho E1203).
  Cả ba đều là thư viện Unicode phổ biến, giấy phép MIT/Apache-2.0 (tinyvec: Zlib/Apache-2.0/MIT), chỉ dùng trong `gt-lexer`; `cargo audit` sạch (1290 advisory, 6 crate). Tự viết lại bảng Unicode là rủi ro lớn hơn dùng lại.
- **Cách hiểu D15 (Giang xem lại nếu khác ý):** (1) bidi (U+202A–202E, U+2066–2069, cộng LRM/RLM/ALM U+200E/200F/061C) cấm ở **mọi nơi** → E1201; (2) vô hình/zero-width/điều khiển/khoảng trắng lạ → E1202 ở **mọi nơi, kể cả trong chuỗi và comment** — plan nói "ngoài chuỗi/comment có chú giải" nhưng chưa định nghĩa "chú giải", nên chọn phương án chặt (fail-closed; muốn ký tự đó trong chuỗi thì dùng `\u{…}`);
  (3) BOM đầu file là E1202; (4) trộn bảng chữ (không kể Common/Inherited) trong một định danh/lifetime → E1203; (5) `µ` (U+00B5, MICRO SIGN) bị chặn E1203 vì trông giống `μ` dành riêng; (6) nguồn không NFC → E1204.
- **CRLF:** `\r\n` là xuống dòng; `\r` lẻ là E1202 (repo checkout trên Windows có thể là CRLF; thêm `.gitattributes` ép LF cho corpus).
- **Fuzz harness (`crates/gt-lexer/fuzz`):** crate riêng ngoài workspace, cần nightly + `cargo-fuzz`; macro `fuzz_target!` sinh mã `unsafe` nên crate này **không** `forbid(unsafe_code)` — ngoại lệ có chủ đích, không phát hành. Trên Windows cần thêm thư mục chứa `clang_rt.asan_dynamic-x86_64.dll` (MSVC) vào PATH.
- **Tên token** vẫn là tên tạm của tests/lexer/*.tokens; M1 đã dùng đúng các tên đó (xem `crates/gt-lexer/src/token.rs`), chờ Giang chốt.

## ADR-010 — Tên token chính thức (M1, Giang 2026-10-05)
- **Quyết định:** từ khoá dùng tiền tố `KW_` (`KW_FN KW_MUT KW_IF KW_ELSE KW_MATCH KW_BREAK KW_CONT KW_RET KW_STRUCT KW_ENUM KW_TRAIT KW_IMPL KW_MOD KW_USE KW_PUB KW_TYPE KW_CONST KW_STATIC KW_UNSAFE KW_EXTERN KW_WHERE KW_SELF_KW KW_SELF_TY KW_AS KW_FOR KW_IN KW_TRUE KW_FALSE`);
  ký hiệu: `ARROW FAT_ARROW WALRUS ASSIGN LOOP_SYM DEC PRE POST INV PROOF_SYM EMPTY_SET IMPLIES IN_SET FORALL EXISTS LEQ GEQ NEQ LAND LOR LNOT BORROW BORROW_MUT`;
  dấu câu: `SEMI COLON COMMA DOT DOTDOT DOTDOTEQ LBRACE RBRACE LPAREN RPAREN LBRACKET RBRACKET COLONCOLON HASH STAR SLASH PERCENT PLUS MINUS LT GT EQ_EQ`;
  đặc biệt: `LIFETIME DOC_COMMENT INT_LIT FLOAT_LIT STR_LIT CHAR_LIT IDENT EOF UNKNOWN`. Áp dụng ngay cho `gt-lexer`, `tests/lexer/*.tokens`, semantics, symbols, grammar.
- **Cách hiểu của agent (Giang xem lại nếu khác ý):**
  1. Danh sách có cả `KW_LOOP`/`KW_PROOF` lẫn `LOOP_SYM`/`PROOF_SYM` cho cùng `⟳`/`⊢`. Alias ASCII và canonical phải ra CÙNG một token (round-trip) nên chỉ dùng **một** tên cho mỗi khái niệm: giữ `LOOP_SYM`, `PROOF_SYM` (là cái có ghi ký hiệu); `KW_LOOP`, `KW_PROOF` **không được phát** (test `token_names.rs` ghi rõ).
  2. `KW_IN` xuất hiện hai lần trong danh sách — coi là một tên.
  3. Tên ngoài danh sách nhưng lexer cần, đặt theo cùng quy ước: `BANG` (`!`), `UNDERSCORE` (`_`), `PLUS_WRAP MINUS_WRAP STAR_WRAP PLUS_SAT MINUS_SAT STAR_SAT PLUS_CHK MINUS_CHK STAR_CHK` (toán tử số học `+% … *?`).
  4. `UNKNOWN` được phát cho mọi đoạn nguồn không hợp lệ ở tầng lexer (xem semantics §24); trước đây các trường hợp này chỉ có lỗi, không có token.
  5. Tên biến thể Rust (`TokenKind::KwMut`, `LoopSym`, `Walrus`…) đổi theo cho khớp.
- **Hệ quả:** ADR-002…ADR-009 còn nhắc tên tạm cũ (`MUT`, `KEYWORD_IN`…) — giữ nguyên làm lịch sử; tên đúng nằm ở đây và ở `token.rs`.
- **Xác nhận (Giang):** giữ `LOOP_SYM` và `PROOF_SYM`, bỏ `KW_LOOP`/`KW_PROOF` trùng. Nguyên tắc chung: **canonical Unicode và alias ASCII của cùng một ký hiệu luôn ra đúng một token duy nhất; không có token riêng cho từng dạng bề mặt.**

## ADR-011 — gt-diag: thiết kế diagnostics và schema JSON v1 (M2) — ĐÃ DUYỆT (Giang, 2026-10-05)
- **Quyết định:** `Diagnostic { code, level, message, file, span, span_label, labels, notes, suggestions }`; `Level` = `Error`(ĐỎ)/`Warn`(VÀNG)/`Note`(GHI CHÚ) **suy ra từ tiền tố mã** (D16) nên không thể mâu thuẫn; `Suggestion { message, applicability, edits }` với edit áp dụng nguyên tử; bộ gom `Diagnostics` sắp xếp tất định, bỏ trùng, giới hạn số lượng (ICE không bao giờ bị cắt), phán quyết exit 0/1/2 (D17) và `--deny-yellow`. JSON viết tay (khoá cố định, tất định), kiểm bằng `serde_json` chỉ ở **dev-dependency**; schema ở `docs/spec/diag_schema.json`.
- **Lệch khỏi đặc tả ban đầu của Giang (lý do kỹ thuật):**
  1. `suggestions: Vec<Edit>` → `Vec<Suggestion>` (mỗi gợi ý = message + nhiều edit nguyên tử + `applicability`). Một bản sửa thường cần nhiều edit đi cùng nhau, và AI/LSP cần biết gợi ý nào được phép tự áp dụng (fail-closed: `maybe-incorrect` không bao giờ tự áp dụng).
  2. Thêm `labels` (nhãn phụ) và `notes`. Borrow checker (E04xx) cần "đã move ở đây / dùng ở đây" — nếu thiếu, thêm sau là đổi schema.
  3. `level` không phải trường độc lập mà suy ra từ code (vẫn có trong JSON cho bên tiêu thụ).
  4. "Màu" chỉ là tuỳ chọn `RenderOptions.color` (mặc định tắt) vì output tất định (D9); bật màu chỉ thêm mã ANSI, bỏ ANSI ra thì y hệt bản thuần (có test).
- **Về "schema ổn định ngay từ đầu":** chưa có AST/parser nên chưa thể đảm bảo tuyệt đối. Quy tắc tương thích ghi trong schema: cùng `schema_version` chỉ được THÊM trường (bên tiêu thụ bỏ qua trường lạ); đổi tên/xoá/đổi kiểu = tăng version. Các điểm còn có thể đổi trước khi đóng băng: nhãn/edit nhiều tệp (M3), cách biểu diễn vị trí cho LSP (hiện: offset byte + dòng/cột theo Unicode scalar; LSP đổi sang UTF-16 từ nguồn).
- **Không làm (YAGNI):** tiêu đề/giải thích mã lỗi trong registry (để `gritc explain` ở M-sau), chuyển `LexError` → `Diagnostic` (thuộc `gt-parser`, nơi lexer và diag gặp nhau).
- **QUY TẮC TƯƠNG THÍCH SCHEMA (đã duyệt, nguyên văn):** cùng `schema_version` chỉ được **thêm** trường; đổi tên/xoá/đổi kiểu một trường thì **tăng version**. Bên tiêu thụ (LSP M13, giao thức AI 8.E) phải bỏ qua trường lạ. Giang xác nhận "ổn định tuyệt đối" là hứa quá; quy tắc trên thay thế.
- **Đã duyệt cả bốn điểm lệch:** `Suggestion` có `applicability`, `labels`/`notes`, `level` suy từ tiền tố mã, màu tắt mặc định.
- **Adapter `LexError → Diagnostic`:** đặt ở `gt-parser`. Khi có consumer thứ hai cần lỗi lexer mà không qua parser thì chuyển adapter ra chỗ dùng chung, kèm ADR.
- **Thứ tự tất định (D19):** sắp theo (tệp, offset bắt đầu, offset kết thúc, mã, thông điệp) — chặt hơn yêu cầu (tệp, offset bắt đầu, mã) để các lỗi cùng vị trí/mã cũng có thứ tự cố định; có test hoán vị mọi thứ tự đầu vào.
