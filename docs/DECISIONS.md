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

## ADR-012 — Comment `//` là trivia, không bị lexer bỏ (M1 sửa, M2)
- **Bối cảnh:** plan nói lexer "bỏ qua" `//` và không đưa vào AST. Nhưng gt-fmt (thư viện, M2) in lại từ AST sẽ **xoá mọi comment của người dùng**. Kiểm tra gt-lexer M1: `//` bị bỏ hẳn (`self.pos = line_end`, không phát gì) — đúng như Giang lo.
- **Quyết định:** lexer giữ comment trong **bảng riêng** `LexOutput.trivia: Vec<Trivia { kind: LineComment, text (nguyên văn gồm `//`, không CR/LF), span, own_line }>`, theo thứ tự nguồn. Doc comment `///` vẫn là token `DOC_COMMENT` (gắn vào node AST).
- **Lệch nhẹ so với đề nghị ("phát LINE_COMMENT trong luồng token"):** dùng bảng riêng thay vì token nội tuyến. Lý do: (1) parser không phải lọc comment ở mọi vị trí (một comment chen giữa biểu thức sẽ làm mọi production phải biết bỏ qua); (2) `tests/lexer/*.tokens` và mọi test token không đổi; (3) thông tin vẫn đầy đủ vì có span, gt-fmt ghép lại theo vị trí. Nếu sau này cần token nội tuyến (vd LSP semantic tokens) thì dựng từ hai nguồn là được.
- **`own_line`:** lexer tính sẵn "đứng một mình trên dòng" (chỉ có khoảng trắng phía trước) vs "cuối dòng", vì đó là thông tin gt-fmt cần và lexer có sẵn rẻ.
- **Quy tắc gắn comment của gt-fmt (chi tiết ở KNOWN_LIMITATIONS):** comment `own_line` gắn vào item/câu lệnh KẾ TIẾP; comment cuối dòng gắn vào câu lệnh/item đứng TRƯỚC trên cùng dòng. **Nguyên tắc chặn cứng: nếu gt-fmt không gắn được một comment thì nó TỪ CHỐI định dạng (báo lỗi), không bao giờ xoá comment.**
- **Test:** `crates/gt-lexer/tests/trivia.rs` (viết trước, đỏ, rồi xanh): nguyên văn, `own_line`, `////`, CRLF, cuối tệp không xuống dòng, `//` trong chuỗi không phải comment, `///` không phải trivia, thứ tự/không chồng token, và **mọi dòng có `//` trong corpus cho đúng một mục (trivia hoặc DOC_COMMENT)**. Fuzz target và test độ bền kiểm cả trivia.

## ADR-013 — gt-ast: cây Box + NodeId dày đặc (M2)
- **Bối cảnh:** AST chỉ định nghĩa kiểu, mỗi node cần `Span` và `NodeId` ổn định để các pass sau (types, borrowck, effects) gắn bảng kết quả theo `NodeId`.
- **Phương án:** (A) cây `Box` + `NodeId` là trường thường; (B) arena `Vec<Node>` với `NodeId` là chỉ số.
  - (A): khớp mẫu và viết parser đệ quy/Pratt tự nhiên; `Debug`/`Eq`/`Clone` derive cấu trúc, so sánh hai AST (round-trip alias↔canonical) là `==`; không có handle treo/arena sai. Nhược: con trỏ rải rác, không có liên kết cha sẵn (pass nào cần thì dựng bảng cha theo NodeId).
  - (B): bộ nhớ liền, bảng cha/phụ rẻ. Nhược: mọi truy cập đi qua arena (`ast.expr(id)`), khớp mẫu dài dòng, rủi ro dùng NodeId của arena khác; lợi ích không đáng vì M5 hạ xuống HIR/MIR (CFG, place) — đó mới là nơi cần cấu trúc phẳng.
- **Chọn (A).** `NodeId(u32)` cấp **tuần tự từ 0, không lỗ** bởi `NodeIdGen` của parser, theo thứ tự dựng node tất định (D9): cùng nguồn → cùng cây → cùng NodeId. Bảng kết quả của pass sau dùng `Vec<T>` chỉ mục `NodeId::index()`. `NodeIdGen::next` trả `Option` (hết u32 → `None`, không panic, không tràn lặng lẽ).
- **Giới hạn đã biết:** NodeId duy nhất **trong một tệp** và ổn định cho cùng nguồn, **không** ổn định qua chỉnh sửa (LSP tăng dần M13 phải parse lại và ánh xạ lại; cần `(FileId, NodeId)` từ M3 khi nhiều tệp).
- **AST không biết dạng bề mặt:** `gt-ast` chỉ phụ thuộc `gt-span` (không có `gt-lexer`), không có kiểu token/alias; test khoá điều này (đọc `Cargo.toml` và mã nguồn). Số literal giữ văn bản nguyên văn (suffix nằm trong đó).
- **Quy ước "node":** struct có `id` + `span` (Item, Type, Expr, Stmt, Pattern, Block, Param, Generics, …). Doc comment và attribute là node con của `Item`/`FieldDef`/`Variant`/thành viên trait-impl. Enum nhỏ không phải node (`Visibility`, `BinaryOp`, `PrimType`…).
- **Không có `Paren`:** dấu ngoặc nhóm không vào AST (độc lập bề mặt; gt-fmt dựng lại ngoặc theo độ ưu tiên). Tuple 1 phần tử `(a,)` phân biệt bằng `ExprKind::Tuple` có đúng một phần tử.

## ADR-014 — gt-parser: quyết định thiết kế và các điều chỉnh khi viết parser thật (M2)
- **Literal âm (Giang yêu cầu ghi ADR):** `gt_ast::Lit` có trường `negative: bool` (đã bỏ `PatLit`; pattern dùng `Lit` trực tiếp). Parser gấp `-` vào literal **chỉ khi** `-` đứng ngay trước token literal số **và** literal không được theo sau bởi `.`, `(`, `[` (postfix chặt hơn unary: `-2.abs()` = `-(2.abs())`). `-(128i8)` KHÔNG gấp (ngoặc không vào AST nhưng parser quyết định ở mức token) nên M7 báo tràn đúng; `-128i8` gấp nên hợp lệ. `Lit.span` phủ cả dấu `-`. Biểu diễn bằng cờ chứ không bằng tiền tố `-` trong văn bản để pass sau không phải phân tích chuỗi.
- **Độ sâu 256 và stack (phát hiện khi test):** mỗi mức lồng đi qua ~9 frame đệ quy; ở bản debug 256 mức vẫn **tràn stack 2 MB** của luồng test (`STATUS_STACK_OVERFLOW`). Đếm độ sâu một mình không bảo đảm gì nếu stack không đủ. Quyết định: `parse()` chạy trong **luồng riêng có stack cố định 64 MiB** (`PARSER_STACK_BYTES`; địa chỉ ảo, cấp dần), vì đo theo địa chỉ stack sẽ làm kết quả phụ thuộc chế độ build (vi phạm D9). Mọi **pass đệ quy trên AST (types, borrowck, …) cũng phải chạy với stack tương đương** — ghi trong KNOWN_LIMITATIONS.
- **Panic nội bộ → ICE:** `parse()` bắt panic của luồng parser và trả `E9901` (exit 2, không chạy) thay vì sập tiến trình. Hệ quả cho kiểm thử: test độ bền và fuzz target **phải khẳng định không có `E9901`**, nếu không panic bị che.
- **Độ sâu tính cả chuỗi phẳng (Q15e):** độ sâu cây = độ lồng đệ quy + độ dài chuỗi toán tử/postfix/cast; vượt 256 → ĐỎ `E0102` và **dừng toàn bộ phân tích** (mọi token sau đó coi như hết tệp) để không phát lỗi rác.
- **Khôi phục lỗi:** không có node lỗi trong AST; câu lệnh/item/nhánh match hỏng bị bỏ và parser đồng bộ lại ở `;` (tiêu thụ) hoặc trước `}` (câu lệnh), `,`/`}` (nhánh match), đóng `}` của item hoặc từ khoá mở item kế tiếp (item). Mỗi vòng có chốt "pos phải tăng, nếu không ép tiêu thụ một token"; test ngẫu nhiên 30 000 chuỗi token kiểm công việc ≤ 300 × (độ dài + 1). Lỗi ở token `UNKNOWN` được báo im lặng (lexer đã báo) để không có hai chẩn đoán cho cùng một chỗ.
- **Mã lỗi:** thêm `E0102` (lồng quá sâu, ĐỎ, không phải ICE) vào `error_codes.md` + `gt-diag`, kèm `tests/fail/E0102_too_deep.gt`. `E0307` KHÔNG thuộc parser (Q15f).
- **Phần tôi tự thêm để parse được corpus/mẫu, chờ Giang duyệt (OPEN_QUESTIONS):** Q15a (`expr_ns` cho hợp đồng/`↓`/`⋈`/`⊢`), Q15b (`self` làm biểu thức), Q15c (`∈`, `⇒`), Q15h (`↓` trong chữ ký hàm cho đệ quy — `ContractKind::Variant`), Q15i (đích gán rộng hơn grammar).

## ADR-015 — `gt-stack` và `gt_ast::MAX_DEPTH`: luật an toàn stack (M2)
- **Quyết định (Giang yêu cầu viết thành luật, không chỉ trường hợp):**
  1. **`MAX_DEPTH` = 256 nằm đúng một chỗ:** `gt_ast::MAX_DEPTH` (vì đó là bất biến của cây AST). Test `max_depth_constant_lives_in_one_place` quét mọi `crates/*/src` và fail nếu có định nghĩa thứ hai.
  2. **Helper chạy-trong-luồng-stack-lớn ở đúng một chỗ:** crate riêng `gt-stack` (không phụ thuộc gì) với `run_with_big_stack` và `BIG_STACK_BYTES` = 64 MiB; panic → `StackError::Panic`, không tạo được luồng → `StackError::Spawn`; người dùng helper map cả hai thành ICE `E9901`. Chọn crate riêng (không để trong `gt-ast`/`gt-diag`) vì mọi pass sau (types, borrowck, effects…) đều cần mà không nên kéo phụ thuộc không liên quan; chi phí là một crate nhỏ.
  3. **Mọi pass đệ quy trên AST dùng helper này.** Parser đã dùng; M4+ phải dùng (ghi KNOWN_LIMITATIONS).
- **Test là luật, không phải ví dụ:** `depth_law` kiểm 35 loại lồng (biểu thức, khối, kiểu, pattern, mod, chuỗi phẳng, trộn lẫn); với mỗi loại tìm độ sâu cực đại parse được **ở bản debug** — kết quả: mọi loại chạm đúng độ sâu 256 (bộ đếm `max_depth` = 256), không ICE, không tràn stack — và cực đại + 1 cho đúng một chẩn đoán `E0102`. **Độ nhạy đã kiểm:** hạ `BIG_STACK_BYTES` xuống 2 MiB thì test sập (`STATUS_STACK_OVERFLOW`); khôi phục 64 MiB. Một khúc xạ đáng biết: số mức lồng tối đa theo từng loại khác nhau vì chi phí độ sâu mỗi mức khác nhau (khối `{}`/`⎇`/`⟳`/`⚠` tốn 2 mỗi mức nên tối đa 127 mức; biểu thức/kiểu/pattern 253–255).
- **E0102** nay luôn kèm ghi chú gợi ý "tách biểu thức con ra biến phụ bằng `≔`".
- **Giới hạn:** 64 MiB là địa chỉ ảo; trên hệ thống giới hạn bộ nhớ ảo chặt, tạo luồng có thể thất bại → ICE `E9901` (fail-closed, không chạy).

## ADR-016 — gt-fmt: quy tắc in, ngoặc, comment (M2) — **E1301 chờ Giang duyệt**
- **Phạm vi:** thư viện `gt_fmt::format(file, source) -> Result<String, FmtError>`; không binary (xem crates/TASK.md, `gritc fmt` là M9). Chạy trong luồng `gt-stack` (ADR-015); panic → ICE `E9901`.
- **In từ AST, một dạng chuẩn duy nhất** (canonical Unicode). Alias ASCII và canonical cho **cùng đầu ra** (test). Fail-closed: nguồn có chẩn đoán của parser → từ chối với đúng chẩn đoán đó.
- **Ngoặc do printer tự thêm/bỏ**, không nhớ ngoặc nguồn (AST không có `Paren`, ADR-013): bọc khi độ ưu tiên < `min` của vị trí (bảng ưu tiên/kết hợp của parser), hoặc khi là struct literal ở vị trí `expr_ns` (điều kiện `⎇ ⟳`, `⌥`, `↓ ⋈ ⊢`, hợp đồng); `ns` được đặt lại trong ngoặc/`[]`/đối số. Ba ca đặc biệt để đọc lại đúng AST: `-` trước literal in `-(lit)`; chỉ số tuple sau literal `(2).0`; vế trái của `<` kết thúc bằng `as T` (`(x as T) < y`). Tuple một phần tử luôn có dấu phẩy `(a,)`.
- **Bố cục:** thụt 2 khoảng trắng; contract mỗi cái một dòng thụt, thân `{` xuống dòng riêng; khối chỉ có tail ngắn (≤ `INLINE_MAX`=60 ký tự, một dòng) in `{ e }`; `μx` (không cách) cho tham số/let mut; **một dòng trống giữa hai item liền kề (Giang duyệt)**, không dòng trống thừa.
- **Comment — cơ chế token (v2, Q16b, Giang duyệt 2026-10-06; thay quy tắc theo từng vị trí của v1):**
  1. Printer in văn bản CƠ SỞ không có comment. Nguồn và văn bản cơ sở được lex lại và **căn token** (hai luồng chỉ được khác nhau ở `(` `)` `,` `;` do printer thêm/bỏ; khác khác = ICE `E9901`).
  2. Mỗi comment gắn vào token gần nhất: **dòng riêng → token kế tiếp; cuối dòng → token liền trước.** Token nguồn đã bị bỏ thì dịch sang token cơ sở khớp gần nhất theo cùng hướng; dấu `,` `;` do printer thêm ngay sau token neo thì comment đứng sau chúng; `(` do printer thêm ngay trước token neo thì comment đứng trước chúng.
  3. Phát: comment dòng riêng trên dòng riêng ngay trước token (trước `}` `)` `]` thì thụt sâu hơn token đóng 2 khoảng); comment cuối dòng cách token 2 khoảng trắng và **ép xuống dòng** sau nó (dòng tiếp theo thụt = thụt dòng hiện tại + 2; token đóng thì bằng thụt dòng hiện tại). Duy nhất một quyết định bố cục có biết comment: khối/danh sách CHỨA comment không bao giờ in gọn một dòng.
  4. Xác minh: lex đầu ra phải cho đúng luồng token cơ sở, không lỗi lexer, và đúng dãy comment nguồn; không thì **E1301 (phương án cuối)**. Không có luật ngầm nào khác. Ca nào còn không đặt được phải được ghi lại (hiện: không có ca hợp lệ nào — bài đo chèn comment sau từng token của corpus cho 0 từ chối; ca `t.` + xuống dòng + `0.1` biến chính nguồn thành lỗi cú pháp nên không phải đầu vào của fmt).
  5. Bất biến kiểm bằng test thường trực: mọi ca chèn → số/nội dung/thứ tự comment giữ nguyên, `fmt(fmt(x)) == fmt(x)`, `AST(fmt(x)) == AST(x)`.
- **Thay đổi cách trình bày chuẩn (trước → sau):**
  - Comment sau `{`: `ƒ g() ! ∅ { // c` → `ƒ g() ! ∅ {  // c` (hai khoảng trắng, như mọi comment cuối dòng).
  - Sau dòng contract (trước: E1301): `ƒ f(x: i32) ! ∅` / `  ⊳ x > 0 // pre` → `  ⊳ x > 0  // pre`.
  - Giữa các contract / sau tham số (trước: E1301): `ƒ f(` `⏎  a: i32, // a` `⏎  b: i32,` `⏎) ! ∅ { }` → `ƒ f(a: i32,  // a` / `  b: i32) ! ∅ { }` (danh sách một dòng bị ngắt tại comment, không thành mỗi phần tử một dòng).
  - Giữa biểu thức (trước: E1301): `1 + // c` `⏎ 2` → `  1 +  // c` / `    2`; nhiều lần liên tiếp tạo bậc thang +2.
  - Comment cuối dòng đứng TRƯỚC dấu phẩy nguồn (`x: i32 // c` `⏎ ,`) giữ nguyên vị trí đó: `x: i32  // c` / `    ,`.
  - Comment dòng riêng trước `⇝` / giữa doc comment và item: nay được giữ (`  ⎇ c { 1 }` / `    // else` / `    ⇝ { 2 }`).
- **`E1301`: Giang duyệt (Q16a); nhóm `E13xx: định dạng (gt-fmt)` thêm vào D16.** Nay chỉ là phương án cuối (xác minh thất bại), kiểm bằng unit test, không có mẫu `.gt` (hai mẫu cũ chuyển thành mẫu `pass` 32/33).

## ADR-017 — M3 giải tên: FileId, test nhiều tệp, prelude, quy tắc phân giải — **Giang duyệt 2026-10-06 (Q17, Q18, Q19)**

### (a) Định danh node xuyên tệp
- **Bối cảnh:** `NodeId` chỉ duy nhất trong một tệp (ADR-013). Nhiều tệp cần khoá toàn cục cho mọi bảng kết quả (giải tên, kiểu, mượn…).
- **Phương án:** (A) `NodeId = (FileId, u32)`; (B) bộ đếm toàn cục; (C) giữ `NodeId` u32 và dùng cặp `(FileId, NodeId)` ở các pass sau.
  - (B) loại: id phụ thuộc THỨ TỰ parse các tệp → đổi theo thứ tự tệp (phá tính chất bắt buộc "kết quả không phụ thuộc thứ tự"), chặn parse song song, LSP tăng dần không thể giữ id.
  - (C) loại: mỗi bảng phải nhớ khoá bằng cặp; quên `file` là lỗi lặng lẽ (hai node khác tệp trùng khoá).
  - (A) chọn: một kiểu khoá duy nhất cho mọi pass; không thể quên tệp. Chi phí: +4 byte/node, đổi `NodeId`/`NodeIdGen` và API parse.
- **Quyết định:** `FileId(u32)` ở `gt-span`; `NodeId { file: FileId, index: u32 }` ở `gt-ast`, `NodeIdGen::new(file)`; `index` vẫn dày từ 0 trong từng tệp. **`FileId` = hạng của tên tệp trong danh sách đã SẮP XẾP** (`SourceMap`), nên không phụ thuộc thứ tự đưa tệp vào. `gt_parser::parse(name, src)` giữ nguyên (FileId 0, tiện cho test một tệp); thêm `parse_file(file_id, name, src)`.
- **Schema JSON:** `label` và `edit` ĐÃ có trường bắt buộc `file` từ v1 → không cần đổi, **không tăng `schema_version`** (đúng quy tắc chỉ-thêm-trường). Phần còn thiếu là kiểm chứng: test JSON cho nhãn/edit ở tệp khác (line/col tính theo tệp của nhãn) và renderer người-đọc vẽ nhãn ở tệp khác thành khối riêng (trước đây chỉ liệt kê dạng văn bản).

### (b) Test nhiều tệp
- **Quyết định:** một ca = **một thư mục** `tests/{pass,fail,warn}/<ca>/` chứa ≥2 tệp `*.gt` và một tệp tên `expect` có đúng một dòng: `expect: pass` | `expect: fail Exxxx` | `expect: warn Sxxx` (dòng `note:` tuỳ chọn). Tên thư mục theo quy ước tên tệp đơn (`Exxxx_mô_tả`, `NN_mô_tả`). Tệp `.gt` trong thư mục KHÔNG có dòng `// expect:` (kỳ vọng thuộc về cả ca). **Tên tệp (không đuôi) là tên module** (phải là ident hợp lệ, duy nhất). Ghi vào `tests/README.md`; `tools/check_expectations.py` kiểm: có `expect`, đúng định dạng, thư mục đúng loại, mã có trong bảng, thư mục được liệt kê ở cột "Test mẫu" như tệp đơn, Unicode sạch cho mọi `.gt`.

### (c) Prelude
- **Quyết định:** resolver nhận prelude là **đầu vào tường minh** (`Prelude`), KHÔNG có tên nào viết cứng trong resolver. `Prelude` dựng từ mã nguồn Grit (`Prelude::from_source`): các item cấp cao của nó thấy được ở mọi module bằng tên trần (bất kể `◈`); **biến thể của mọi `enum` cấp cao trong prelude cũng thấy được bằng tên trần** (`Some`, `None`, `Ok`, `Err`); module con của prelude (vd `core`) truy cập qua `core::abort` và các thành viên cần `◈`. Test dùng fixture prelude trong mã test; M12 thay bằng `lib/core` thật (cùng đường vào, không đổi resolver).

### (d) Quy tắc phân giải (đề xuất)
1. **Chương trình = tập tệp.** Cây module: gốc ẩn danh (không chứa item) → mỗi tệp là một module tên = tên tệp bỏ đuôi → `▦` lồng bên trong. Không đổi grammar (không có `mod x;`). Thứ tự tệp và thứ tự khai báo item không có ý nghĩa.
2. **Một không gian tên cho mỗi module** (không tách kiểu/giá trị): item, `▦` con, tên nhập (`⇲`) và **biến thể của mọi enum trong module** (Q19: corpus M0 dùng biến thể trần `Circle(r)`, nhất quán với `Some`/`None` của prelude) cùng một bảng; biến thể mang khả kiến của enum chứa nó; trùng tên → **E0202** (MỘT chẩn đoán cho mỗi (module, tên), nêu số lần và gắn nhãn mọi vị trí; thông điệp không nói "thứ hai" để độc lập thứ tự). Cũng E0202: biến thể/trường trùng trong một enum/struct, tham số hàm trùng, tham số generics trùng, thành viên trùng trong một khối `⊏`/`⊐`.
3. **Phân giải đường dẫn** (biểu thức, kiểu, pattern): đoạn đầu tìm theo thứ tự: biến cục bộ (chỉ đường dẫn một đoạn) → tham số generics → bảng tên của module hiện tại → **tên module-tệp (mọi tệp thấy tên mọi tệp khác, vd `b::g()` không cần `⇲`)** → prelude. **Không kế thừa tên từ module cha** (module `▦` lồng không thấy tên của module chứa nó; dùng đường dẫn đầy đủ từ tên tệp, vd `⇲ a::n::g;`). Các đoạn sau: thành viên của module hoặc biến thể của enum. `Kiểu::tên` với `tên` không phải biến thể là mục kết hợp (impl/trait) — M4 kiểm; M3 ghi `Res::Assoc` (không báo lỗi, không bịa). Không tìm thấy → **E0201** (một lỗi cho mỗi vị trí dùng; không lan truyền).
4. **`⇲ đường_dẫn;`** phân giải **từ gốc** (đoạn đầu = tên module-tệp hoặc tên trong prelude; đường dẫn tuyệt đối gồm cả tên tệp). Dạng nhóm `⇲ a::b::{c, d};` = nhập `a::b::c` và `a::b::d`. Tên nhập = đoạn cuối (không đổi tên). Nhập được item, module, biến thể. Nhập là private trừ khi `◈ ⇲`.
5. **Khả kiến `◈`:** item `◈` thấy ở mọi nơi; item private thấy ở module định nghĩa và MỌI module con của nó. Vượt khả kiến → **E0203**. (Truy cập trường private là M4 vì cần kiểu.)
6. **E0204 — vòng phụ thuộc module:** cạnh S→T khi S chứa tham chiếu đã phân giải tới item của module T ≠ S mà T không phải hậu duệ của S (tham chiếu tới chính mình/hậu duệ không tạo cạnh). Mỗi thành phần liên thông mạnh có >1 module bị báo MỘT lần (liệt kê module theo thứ tự tên, nhãn tại các vị trí tham chiếu tạo vòng).
7. **Biến cục bộ và D7 — E0205:** tên mới (tham số, `≔`, biến `⟳ x in`, binding trong pattern) không được trùng (i) tên cục bộ/tham số đang trong phạm vi trong cùng hàm, (ii) tên generics đang trong phạm vi, (iii) tên thấy được ở cấp module (item, biến thể, nhập, tên module-tệp, prelude; riêng pattern chỉ có ident phân giải tới biến thể thì là pattern biến thể, không phải binding). Khối anh em được dùng lại tên. Dùng tên trước khi khai báo trong cùng khối → E0201. (E0205 chuyển từ M4 sang M3 trong `error_codes.md`.)
8. **Pattern chỉ có ident `x`:** nếu `x` phân giải (module/prelude) tới một **biến thể enum** → pattern biến thể (kiểm số tham số là M4); ngược lại là binding mới (và chịu luật mục 7). Rủi ro đã biết: gõ nhầm tên biến thể (`Nonee`) lặng lẽ thành binding — match-vét-cạn (M5) sẽ bắt phần lớn.
9a. **`result` trong `⊲`:** đặc tả hiện KHÔNG nói hậu điều kiện gọi giá trị trả về là gì (OPEN_QUESTIONS Q18). Đề xuất agent đã cài: `result` là biến cục bộ ngầm CHỈ trong biểu thức `⊲` (chịu luật E0205 nếu trùng tên tham số). Chưa duyệt.
9. **`Self`** chỉ hợp lệ trong thân/chữ ký của `⊏`/`⊐` (ngoài đó → E0201, thông điệp riêng); **`self`** (giá trị) chỉ trong hàm có tham số `self`.
10. **Không phân giải ở M3:** trường, phương thức, mục kết hợp, tên hiệu ứng, attribute, lifetime `'a` (M4/M4.5/M6/M8).
- **Kết quả:** `ResolveOutput { resolutions, report }`. Mỗi vị trí dùng → `Res::{Def, Local, Variant, Assoc, Err}`; DefId đặt theo đường dẫn đủ tên (không theo thứ tự khai báo). Dạng chuẩn `canonical()` (khoá = đường dẫn item bao + số thứ tự trong item) dùng để so sánh bất biến thứ tự.
- **Tính chất bắt buộc (plan) — phạm vi chính xác:** xáo thứ tự TỆP → kết quả và danh sách lỗi (sắp D19) **giống từng byte** (vị trí trong tệp không đổi). Xáo thứ tự ITEM trong tệp → `canonical()` giống hệt và tập lỗi (mã, thông điệp, vị trí logic = đường dẫn item bao) giống hệt; **vị trí byte và do đó thứ tự D19 theo offset KHÔNG thể giống** (D19 sắp theo offset, mà offset đổi khi xáo). Đó là hệ quả của D19, không phải lỗi; ADR ghi để không hứa điều không thể.

## ADR-018 — M4 hệ thống kiểu: đơn hình hoá, biểu diễn impl, kiểu `!` — **Giang duyệt 2026-10-06 (điểm 1, 2, 3; Q20, Q21, Q22; `#lang`)**
Viết TRƯỚC khi có dòng code M4 nào. Mỗi điểm: bối cảnh → phương án → **đề xuất** → hệ quả (kể cả chi phí) → cái cần Giang chốt.

### 1. Đơn hình hoá (monomorphization): eager hay lazy?
- **Bối cảnh.** Generics kiểu `ƒ id<T>(x: T)`, ràng buộc trait inline `T: A + B`, chỉ dispatch tĩnh (không `dyn`). Codegen M10 (LLVM) chỉ sinh mã cho kiểu cụ thể. Dù chọn cách nào, thân hàm generic LUÔN được kiểm kiểu ĐÚNG MỘT LẦN với tham số kiểu là kiểu mờ chỉ biết qua ràng buộc (lỗi báo ở nơi định nghĩa, không phải ở nơi dùng — khác template C++). Câu hỏi chỉ là: lúc nào và ở đâu sinh các bản đã thay thế `T := kiểu cụ thể`.
- **Phương án EAGER:** cuối M4, với mỗi (hàm generic × bộ tham số kiểu) cụ thể sinh một BẢN SAO thân hàm đã thay kiểu, rồi mọi pass sau (M5 MIR, M6 mượn, M7, M8) và M10 chỉ thấy mã đơn hình. Ưu: M10 không biết gì về generics. Nhược: số bản = số instance; MIR/dataflow/borrowck/hiệu ứng chạy lại trên từng bản (chi phí nhân lên), hoặc phải chạy trên bản gốc VÀ bản sao (thừa); lỗi trên bản sao lặp lại cho mỗi instance.
- **Phương án LAZY (đề xuất):** sau kiểm kiểu chạy một **bộ thu thập instance** theo hàng đợi từ các gốc (`main`/mục `◈` xuất ra), mỗi instance là `(FnRef, Vec<Ty>)`; kết quả là bảng instance đã SẮP XẾP tất định, KHÔNG sao chép thân hàm. M5–M8 chạy MỘT lần trên thân generic (borrow/hiệu ứng không phụ thuộc kiểu cụ thể, chỉ phụ thuộc ràng buộc như `T: Copy`). Phép thay thế `T := …` chỉ xảy ra ở M10 khi hạ từng instance (với Phase 1 — không có kiểu liên kết, không GAT, không specialization — phép thay thế là ánh xạ tham số→kiểu, nhỏ).
- **Hệ quả của đề xuất.** (i) Thời gian biên dịch tỉ lệ với MÃ, không với số instance; generic không ai dùng không tốn gì ngoài kiểm kiểu một lần. (ii) M10 phải cài thay thế kiểu (việc nhỏ, dồn về một chỗ). (iii) Cần giới hạn đệ quy đa hình (`ƒ f<T>() { f<Vec<T>>() }` sinh vô hạn instance): bộ thu thập có ngưỡng độ sâu cố định (ĐỀ XUẤT 64) → lỗi ĐỎ (mã mới, xem Q20), không treo. (iv) bảng instance là đầu vào của M10 và của kiểm "mọi instance của hàm có hiệu ứng/biên thoả" nếu M7/M8 cần. (v) Thứ tự instance tất định (sắp theo `(đường dẫn item, chuỗi kiểu)`) để biên dịch tái lập (D9).
- **Cần Giang chốt:** LAZY (đề xuất) hay EAGER; ngưỡng đệ quy đa hình (64?).

### 2. Dispatch trait và biểu diễn `⊐ Trait for Type` trong bảng kiểu
- **Bối cảnh.** Phase 1 chỉ dispatch tĩnh. M6 (mượn) cần: một kiểu có `Copy` không, chữ ký phương thức (self theo giá trị/`&`/`&!`). M10 cần: gọi `x.m()` cụ thể trỏ tới hàm nào với tham số kiểu nào. Grammar chỉ cho trait chứa CHỮ KÝ HÀM (không kiểu liên kết, không hằng liên kết) và `⊐` chứa hàm.
- **Đề xuất — bảng impl (`ImplTable`):** mỗi `⊐` là một `ImplDef { id, generics, trait_ref: Option<TraitRef{trait: DefId, args}>, self_ty: Ty, members: tên → FnRef }`; `⊐` thường (không trait) cùng bảng với `trait_ref = None` (inherent). `FnRef` = `NodeId` của hàm/thành viên (khoá toàn cục nhờ ADR-017a), vì thành viên impl/trait không có `DefId` ở resolver. Sắp xếp theo khoá chính tắc (văn bản `trait + kiểu`) cho tất định.
- **Phân giải gọi.** `x.m(..)`: thử theo thứ tự nhận theo giá trị → `&` → `&!`; ứng viên inherent trước, rồi trait CÓ TRONG PHẠM VI (tên trait phải thấy được qua resolver, không phép màu). Kết quả ghi lại trên node lời gọi: `Method::Impl{impl, member, substs}` khi biết impl, hoặc `Method::TraitMethod{trait, member, self_ty}` khi gọi qua ràng buộc generic (`T: Tr`) — chọn impl cụ thể được hoãn tới bộ thu thập instance (điểm 1). Thoả ràng buộc `T: Tr` tại điểm gọi = tìm impl khớp đầu đề bằng unify (tham số impl là biến suy luận) rồi kiểm các ràng buộc inline của impl.
- **Luật kết dính (coherence):** với mỗi `(trait, kiểu)` tối đa MỘT impl khớp; hai đầu đề unify được ⇒ lỗi chồng lấn. Thành viên impl phải khớp đúng bộ và chữ ký với trait (thiếu/thừa/khác chữ ký ⇒ lỗi). Trait đánh dấu lang (`Copy`) KHÔNG cho người dùng impl cho kiểu có cờ cấm (xem Q21).
- **`Copy` cho M6:** `is_copy(ty)`: nguyên thuỷ số/bool/char, `&T`, tuple/mảng toàn phần tử `Copy`, `!`; struct/enum chỉ khi có `⊐ Copy for T` hợp lệ (mọi trường `Copy`) ; `&!T`, `Secret<_>`, `Cap<_>` không bao giờ. Đây là TRUY VẤN của gt-types mà M6 gọi, không phải bảng riêng.
- **Hệ quả.** Không có dyn/vtable nên không cần biểu diễn "đối tượng trait". Hạn chế đã biết (ghi KNOWN_LIMITATIONS): không kiểu liên kết, không trait generic nhiều tham số ngoài mức đơn giản, không specialization, không impl chồng lấn.
- **Cần Giang chốt:** có chấp nhận tập mã lỗi mới cho impl/ràng buộc (Q20); luật "trait phải thấy được trong phạm vi mới gọi được phương thức"; `Copy` chỉ do `⊐ Copy for T` hoặc builtin.

### 3. Kiểu `!` (never): lan truyền
- **Hiện trạng đã duyệt (Q12, `semantics.md` §22):** `!` là kiểu đáy, gán được cho mọi kiểu; `⏎`, `⊠`, `⊡` và vòng vô hạn không thoát có kiểu `!`; nhánh `!` hợp nhất với kiểu các nhánh còn lại. Vậy câu hỏi "unify với mọi kiểu hay phải khai báo" ĐÃ có đáp án: giống Rust, không cần khai báo. Phần còn thiếu là CHI TIẾT lan truyền, đề xuất:
  1. **Là ép kiểu (coercion) tại các điểm ép, KHÔNG phải quan hệ con tổng quát:** vế phải của `≔`/`←`, đối số, giá trị trả về/`⏎`, trường literal, nhánh `⎇`/`⌥`, đuôi khối. Ở các điểm này `!` ép được sang kiểu mong đợi. Ngoài điểm ép (vd toán hạng của `+`, so sánh) `!` KHÔNG tự thành `i32` — lỗi E0301.
  2. **Hợp nhất nhánh:** kiểu của `⎇`/`⌥` = kiểu chung của các nhánh KHÔNG phải `!`; nếu mọi nhánh là `!` thì cả biểu thức là `!`.
  3. **Khối:** khối có tail → kiểu của tail. Khối KHÔNG tail mà có câu lệnh biểu thức kiểu `!` (`⏎ x;`, `abort(..);`) → kiểu khối là `!` (mã sau đó là mã chết — M5 báo GHI CHÚ, không phải lỗi). Khối không tail, không phân kỳ → `()`.
  4. **`≔` với vế phải `!`:** biến nhận kiểu `!` (mã sau là mã chết); không có luật riêng.
  5. **Hàm `→ !`:** M4 chỉ yêu cầu thân có kiểu `!` (theo luật 2–3); việc "mọi đường đều phân kỳ" (kể cả vòng lặp) do M5 chứng minh (E0307) — M4 không tin lời khai.
  6. **Vị trí kiểu:** `!` hợp lệ làm kiểu trả về và (do ép kiểu) làm kiểu nhánh; `Option<!>`, `[!; N]`, tham số kiểu `!`, trường kiểu `!`: ĐỀ XUẤT từ chối ở M4 (kiểu không có giá trị làm trường/phần tử là vô nghĩa và làm phức tạp layout M10) — mã lỗi mới (Q20).
- **Hệ quả.** Chi tiết 1 giữ hệ suy luận đơn giản (một chỗ ép kiểu, không có subtyping tràn lan); chi tiết 6 là hạn chế chủ ý, ghi KNOWN_LIMITATIONS.
- **Cần Giang chốt:** 1–6 như đề xuất; riêng 6 (cấm `!` làm tham số/trường) có chấp nhận không.

### 4. Phản biện kỹ thuật với chính lệnh M4 (cần Giang đọc trước khi duyệt ADR)
- **Mã lỗi trong lệnh mâu thuẫn bảng đã duyệt.** Lệnh gọi "E0305 (thiếu/thừa trường)" và "E0306 (match đầy đủ)", nhưng `error_codes.md` (đã duyệt) ghi E0305 = `⌥` không đầy đủ (D6) và E0306 = biến chưa khởi tạo (M5); mẫu `E0305_nonexhaustive` đã có. Không đổi mã khi chưa có ADR+duyệt. **Đề xuất:** giữ E0305 cho match không đầy đủ (chuyển milestone M5→M4 theo lệnh), E0306 giữ nguyên (M5); dùng mã MỚI chưa dùng cho trường struct (Q20).
- **"M4.5 gộp vào M4" không khớp plan.** M4.5 của plan là vùng nhớ (`Arena`/region) và kiểm capability (E0310, ≥20+20 ca), KHÔNG phải kiểm trường struct/enum (thuộc M4 ngay từ plan). Đề xuất: M4 làm đúng 6 mục lệnh (trong đó phần capability chỉ là BIẾT kiểu `Secret<T>`/`Cap<T>`); M4.5 giữ nguyên phạm vi cũ (region + E0310). Cần Giang xác nhận (Q21).
- **`Secret<T>`/`Cap<T>` "không Copy, không Default" ngay từ M4:** làm được phần KIỂU (`is_copy` = false; cấm `⊐ Copy for` chúng; cấm tạo bằng struct literal ngoài `lib/core` vì chúng phải không giả mạo được — D29). Phần dùng-sau-move (affine thật sự) là M6, không phải M4: M4 chỉ đảm bảo kiểu bị đánh dấu không-Copy. Cách đánh dấu "không giả mạo/lang item" cần một thuộc tính CHỈ lib/core được dùng (như `#noreturn`, §22) — thuộc tính mới ⇒ cần duyệt (Q21). Không có "Default" trong Phase 1 (chưa có trait Default): cấm đồng nghĩa với không cung cấp.
- **Cắt đột biến ở M4 là rủi ro có chủ đích:** nhưng M4 chứa chính các luật an toàn D1 (không ép kiểu ngầm), D4 (`==` float), D5 (bất biến), và phép ép `!`. Ở M3, đột biến tìm ra 4 lỗi thật (1 lỗi fail-open mà 50 test đã viết vẫn để lọt). Tôi sẽ làm theo lệnh (không đột biến M4) và bù bằng: TDD, test thuộc tính (mọi biểu thức ngẫu nhiên có kiểu hoặc lỗi, tất định), test "mọi vị trí con của AST được duyệt", corpus ≥3+≥3 mỗi mã. **Nếu Giang muốn một lưới rẻ hơn:** đột biến CHỈ cho 4 kiểm tra an toàn nói trên (khoảng 40 đột biến, ~1 giờ) — tôi đề xuất nhưng không tự làm.
- **Corpus M4 cần prelude phong phú hơn M3.** Mẫu `pass` (vd `16_cap_fs.gt`: `fs.read(path)` rồi `f.len()`) cần `Cap<Fs>` có phương thức `read` trả kiểu có `len() → usize`. Fixture prelude của test phải khai báo các kiểu/impl đó (ghi vào fixture, không hard-code trong gt-types; lib/core thật ở M12).

### 5. Chốt bổ sung (Giang, 2026-10-06)
- **Duyệt nguyên văn đề xuất:** (1) đơn hình hoá LAZY, ngưỡng đệ quy đa hình 64; (2) `ImplTable` + luật kết dính + `is_copy`; (3) `!` như đề xuất 1–6 (kể cả cấm `!` làm tham số kiểu/trường/phần tử).
- **Match trên kiểu không liệt kê hết được (số nguyên, `char`, `str`, số thực):** BẮT BUỘC có nhánh `_` hoặc nhánh gán tên (pattern chỉ có ident, vd `n ↦ …`) — Phase 1 chưa có phủ theo khoảng (range pattern) để chứng minh vét cạn; thiếu → E0305. Literal và range trong pattern vẫn được dùng nhưng KHÔNG đóng góp vào vét cạn. `bool`: phải có cả `true` và `false` (hoặc `_`/tên). Enum: phủ mọi biến thể (kể cả qua `_`/tên). Tuple/struct pattern: vét cạn theo từng thành phần (thuật toán ma trận pattern chuẩn). Mỗi nhánh sau một nhánh bắt-mọi-thứ (`_`/tên) là mã chết — ghi chú N (M5), không phải lỗi M4.
- **`#lang`:** thuộc tính chỉ hợp lệ trong `lib/core` hoặc fixture prelude (đầu vào tường minh của resolver/type checker). Dùng ở tệp chương trình thường → lỗi mã **E1002** (họ E10xx; E1001 đã dành cho nghĩa vụ `⚠`/con trỏ thô/`#noreturn`). Có tài liệu `docs/errors/E1002.md`, ≥3 test fail và ≥3 test pass.
- **Bảng mã lỗi mới (đã kiểm không đụng mã có sẵn trong `error_codes.md`):** E0304 trường struct/biến thể/pattern (thiếu, thừa, không tồn tại, trùng); E0309 ràng buộc trait không thoả / không có phương thức / mơ hồ; E0311 `⊐` không hợp lệ (thiếu/thừa/khác chữ ký thành viên, chồng lấn); E0312 đệ quy đa hình vượt ngưỡng instance (64); E0313 kiểu lang dùng sai (tạo `Secret`/`Cap` bằng literal, `⊐ Copy for` chúng); E0314 `!` ở vị trí không hợp lệ; E1002 `#lang` ngoài lib/core. E0305 (match không đầy đủ) chuyển milestone M5→M4; E0306/E0307 giữ M5. Sai số đối số/tham số của lời gọi gộp vào E0301 (không thêm mã).
- **Phạm vi M4 (thứ tự làm):** (a) nguyên thuỷ + literal; (b) struct/enum/tuple + trường (E0304); (c) kiểu hàm, suy luận bidirectional cục bộ, generic theo điểm 1–2; (d) `Secret<T>`/`Cap<T>` mức kiểu (`is_copy = false`, cấm `⊐ Copy for`, cấm tạo bằng literal ngoài lib/core; "dùng sau move" là M6); (e) match đầy đủ (E0305); (f) fixture prelude phong phú; (g) docs lỗi cho mọi E03xx; (h) đột biến có mục tiêu. Sau đó M4.5 (Arena/region, E0310, ≥20 ca fail + ≥20 ca pass), rồi M5.
- **Chính sách đột biến (sửa lệnh 2026-10-06 trước đó):** vẫn KHÔNG đột biến toàn bộ gt-types/gt-hir/gt-mir/gt-dataflow; NGOẠI LỆ có mục tiêu cho 4 kiểm tra an toàn trong gt-types — D1 (không ép kiểu ngầm), D4 (cấm `==` trên số thực), D5 (bất biến mặc định), phép ép kiểu `!` — khoảng 40 đột biến ở CUỐI M4; mọi đột biến sống sót phải có test mới (hoặc được chứng minh tương đương). Giữ nguyên: M6, M7, M8 có đột biến đầy đủ.

### 6. Triển khai M4 — các chỗ khác với lệnh/§5 (tôi nói thẳng; Giang xác nhận ở Q23)
- **Match vét cạn trên số nguyên/`char`: dùng KHOẢNG, không chỉ `_`/tên.** Lệnh §5 ghi "Phase 1 chưa có range pattern" — SAI về mặt sự kiện: range pattern đã được duyệt ở Q14 và nằm trong corpus đã duyệt (`tests/pass/29_range_open_patterns.gt`: `..0 ↦ …, 0..=9 ↦ …, 10.. ↦ …` trên `i32`, không có `_`). Áp đúng lệnh (literal/range KHÔNG đóng góp vào vét cạn) thì mẫu 29 — đã duyệt — bị E0305, tức mâu thuẫn corpus. Cách làm: chia khoảng (interval splitting) cho số nguyên và `char`; `str`, số thực, kiểu mờ vẫn BẮT BUỘC `_`/tên; literal đơn lẻ là khoảng một điểm. Hệ quả: `..0, 0..=9, 10..` phủ đủ `i32`; hở một giá trị (thiếu `0`) vẫn là E0305. Test: `match_exhaustive.rs` (`integers_need_a_catch_all_unless_ranges_cover_the_whole_type`, `the_approved_range_pattern_sample_is_exhaustive`). **Nếu Giang vẫn muốn luật chặt hơn, phải sửa/loại mẫu 29 và Q14.**
- **Literal ngoài phạm vi kiểu KHÔNG phải E0301.** `semantics.md` §1 và ADR-012 (mục literal âm) quy định đây là lỗi biên chắc chắn **E0601 (M7)**; mẫu `E0601_literal_out_of_range` đã có. Bản đầu của gt-types báo E0301 (sai so với spec); đã gỡ. Hệ quả: tới khi M7 xong, `x: u8 ≔ 300` QUA M4 (không phải lỗ hổng của M4 mà là việc M7 chặn); ghi KNOWN_LIMITATIONS. Literal vượt cả `u128` thì không kiểu nào chứa được nên vẫn là E0301 tại M4.
- **Ranh giới E0307 (M5):** thân hàm trả kiểu khác `()` mà khối không có tail (hoặc tail là `⎇` không `⇝`) KHÔNG bị M4 báo E0301; M4 im lặng và M5 báo E0307. Mẫu `E0307_missing_return`/`E0307_trailing_semicolon` đòi đúng như vậy.
- **Hai mẫu của milestone sau không đúng kiểu và đã sửa:** `E0402_double_unique`, `E0403_unique_vs_shared` gán `a ← 2` vào biến kiểu `&!i32` (đích gán phải là `*a`; gán vào tham chiếu là sai kiểu theo D5/Q15i). Đổi thành `*a ← 2`; mã mong đợi (E0402/E0403 của M6) giữ nguyên.
- **Hạn chế đã biết của bản M4 (ghi KNOWN_LIMITATIONS):** không bắt buộc "trait phải thấy được trong phạm vi" (mọi impl đều thấy — luật này còn để ngỏ ở §2); generics của trait chưa dùng được; thân hàm trong prelude không được kiểm (tin chữ ký); `∈` yêu cầu miền là range có ngoặc (`x ∈ (0..10)`); không có pattern `..=b` (chỉ `a..=b`, `a..`, `..b`).
- **Gợi ý sửa dạng edit (tiêu chí M4):** E0302 (`as T`), E0304 (thêm trường thiếu), E0308 (thêm `μ` vào khai báo nếu nguyên nhân là khai báo; KHÔNG gợi ý khi nguyên nhân là tham chiếu `&T` hay binding trong pattern), E0305 (thêm nhánh `_ ↦ …` sau nhánh cuối). Test áp gợi ý rồi kiểm lại chương trình sạch: `suggestions.rs`.
