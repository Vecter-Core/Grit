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

### ADR-017 phụ lục (2026-10-08, M3-fix): phạm vi tên của mã prelude
- Mã trong prelude (chữ ký, thân, hợp đồng) chỉ thấy `[module prelude, prelude]`; KHÔNG thấy gốc người dùng. Lý do: nếu thấy thì tên tệp/module của người dùng bị luật D7 (cấm shadowing) so với tham số của hàm lib → E0205 oan, và tên trong lib có thể phân giải nhầm sang item cùng tên của người dùng. Hướng ngược lại giữ nguyên: mã người dùng thấy prelude và D7 áp dụng đầy đủ. Cài ở `gt-resolve/src/walk.rs::scope_containers`; test `gt-resolve/tests/prelude_isolation.rs`.

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

## ADR-019 — M4.5: capability (E0310), region ở chữ ký (E0315), `Arena` (2026-10-06)
- **Bối cảnh.** M4.5 của plan: kiểu capability + region/Arena trong hệ kiểu, kiểm tĩnh D28, bản nháp ngữ nghĩa region cho M6. Giang giao làm ngay (không chờ duyệt); các lựa chọn dưới đây là của tôi, nêu để Giang xác nhận ở Q24.
- **Capability (E0310) — luật nguồn gốc, không phải phân tích dòng chảy.** Dữ liệu chảy theo kiểu: muốn gọi thao tác cần capability phải có giá trị `Cap<K>`, và giá trị đó chỉ vào hàm qua tham số (chữ ký, D28) hoặc từ một NGUỒN GỐC. Nguồn gốc = hàm/phương thức/hàm kết hợp có kiểu trả về chứa `Cap<…>` mà không có tham số (kể cả `self`) chứa `Cap<…>` — định nghĩa theo kiểu ĐÃ GIẢI sau suy luận nên bắt cả `ƒ conjure<T>() → T`. Chỉ `main` cấp cao của một tệp (đường dẫn đúng hai đoạn `tệp::main`, không phải thành viên impl/trait, không lồng `▦`) được gọi hoặc nhắc tới nguồn gốc (D29); nơi khác, kể cả `□`/`■` (không có capability toàn cục), là E0310. *Đánh đổi:* đơn giản, tất định, không cần dataflow ở M4; không bắt "dùng capability sai loại" (đó là E0309: `Cap<Net>` không có phương thức của `Cap<Fs>`). *Không làm:* kiểm khai báo nguồn gốc trong chương trình người dùng (vô hại vì không có thân nào tạo được giá trị `Cap`, E0313), kiểm hiệu ứng `{io}` của thao tác (M5).
- **Danh sách loại capability do lib/core quyết, compiler không biết:** chỉ biết `Cap<_>` là lang item (`#lang(cap)`). `Fs/Net/Env/Clock/Alloc` là kiểu thường của prelude. Tên `Cap<Secret>` của plan va với `Secret<T>` (struct generic): marker cho "đọc Secret" do lib/core M12 đặt tên.
- **Region (E0315) — kiểm CHỮ KÝ, bảng riêng, không đưa vào `Ty`.** Đưa region vào `Ty::Ref` làm mọi kiểu so sánh theo region và buộc unify xử lý region trước khi M6 tồn tại; M6 vẫn phải suy region trên MIR. Thay vào đó: `RegionSig` theo hàm (`regions.md` §3.5) + kiểm khai báo/elision tĩnh. Mã mới **E0315** (một mã cho: lifetime chưa khai báo, khai báo trùng/che, trường thiếu lifetime, elision mơ hồ) — chọn mã mới thay vì E0201/E0202 để không lẫn với resolver. *Điểm cần Giang chốt (Q24):* (a) luật elision chặt đúng `semantics.md` (không có "`&self` thắng" như Rust) — dễ nới sau, khó siết; (b) không vị trí tham chiếu vào ⇒ `'static` thay vì lỗi (cần vì mẫu `warn/S101_key_in_source` đã duyệt: `ƒ key() → &str`); (c) mã E0315.
- **`Arena` là lang item (`#lang(arena)`, `Lang::Arena`).** Để M6 phân biệt E0405 (thoát Arena) với E0404 (thoát local) chỉ bằng kiểu của vật bị mượn, không dựa vào tên. Là mở rộng danh sách `#lang` đã duyệt ở Q21 (option, copy, range, secret, cap); vẫn chỉ hợp lệ trong prelude (E1002 ngoài đó, có test). *Cần Giang chấp nhận (Q24).*
- **Hạn chế ngữ pháp phát hiện (không tự sửa — đổi grammar cần duyệt):** không có đối số lifetime trong đường dẫn kiểu (`Holder<'a>`), không có `'a: 'b`. M6 làm được corpus yêu cầu (use-after-move, dangling, aliasing, trả tham chiếu local, thoát Arena) với chữ ký hiện có; struct giữ tham chiếu bị ràng buộc region ẩn. Nếu corpus M6 cần ràng buộc rõ, cần quyết định grammar (Q24).
- **Quyết định (Giang, 2026-10-06, Q24):** duyệt E0315, `#lang(arena)` (thêm vào danh sách lang item trong TCB, GRIT_PLAN §3.3), elision chặt hơn Rust (không có luật `&self` thắng), và "không tham chiếu vào ⇒ trả về `'static`" — với điều kiện M6 kiểm thân hàm thật sự chỉ trả dữ liệu `'static` (trả tham chiếu tới biến cục bộ vẫn bị chặn). Đối số lifetime trong kiểu (`Holder<'a>`) thêm ngay (ADR-021); `'a: 'b` hoãn. Q24f (entry = `main` cấp cao) chưa được trả lời.
- **Hệ quả cho mẫu cũ:** `E0310_missing_cap` (có sẵn từ M1) đúng; `tests/pass/42_cap_pass_through` (viết ở M4) dùng `main` nên vẫn qua. Test cũ `lang_types` đổi `ƒ f() { root_fs() }` thành `ƒ main()`.

## ADR-021 — Đối số lifetime trong kiểu: `Holder<'a>` (Giang duyệt 2026-10-06, Q24e)
- **Bối cảnh.** M6 cần nói "struct này mượn với region nào" ở chữ ký (`ƒ get<'a>(h: Holder<'a>) → &'a i32`). Trước đây ngữ pháp chỉ có lifetime sau `&`.
- **Quyết định.** `type_args = "<" (region {"," region} ["," type {"," type}] | type {"," type}) ">"` — đối số lifetime đứng TRƯỚC đối số kiểu; lifetime sau một đối số kiểu là lỗi cú pháp E0101. AST: `TypeKind::Path { path, lifetimes, args }`. gt-fmt in `Holder<'a, T>` (lifetime trước). Resolver không có không gian tên lifetime (bỏ qua); gt-types kiểm (E0315): lifetime đã khai báo; số đối số lifetime đúng bằng số tham số lifetime của struct/enum/alias (kiểu tham số/nguyên thuỷ nhận đối số lifetime cũng là E0315); bỏ trống ở trường/alias là E0315; bỏ trống ở chữ ký hàm thì mỗi tham số lifetime là một vị trí ẩn, tính vào luật elision (ADR-019) và vào `RegionSig`.
- **Hoãn:** ràng buộc `'a: 'b` — một số chữ ký hợp lệ chưa viết được; chỉ thêm khi chương trình thật đòi (KNOWN_LIMITATIONS).
- **Test:** parser (đúng/sai thứ tự), gt-fmt `parse(fmt(x)) == parse(x)` và idempotent trên các dạng mới, gt-types (khai báo, số lượng, elision, `RegionSig`), mẫu corpus.

## ADR-020 — M5: MIR, khung dataflow, kiểm đầy đủ (Giang DUYỆT có sửa đổi 2026-10-06 — xem mục 9)
**Phạm vi (đúng lệnh):** MIR + khung dataflow + kiểm đầy đủ (E0306/E0307, `→ !`). KHÔNG borrow check (M6), KHÔNG interval/Obligation (M7). Không đột biến gt-mir/gt-dataflow (chính sách); bù ở mục 7.

### 0. Tóm tắt và những chỗ lệnh cần nói thẳng
1. **E0306 không có nguồn kích hoạt trong ngữ pháp hiện tại.** `≔` luôn có vế khởi tạo (`parse_let`: `tên [: kiểu] ≔ biểu_thức ;`), không có khai báo trơn `x: T;`, và tham số/biến vòng lặp/binding pattern luôn được gán. Vậy KHÔNG chương trình nguồn nào đạt E0306 → không thể có ≥3 mẫu `fail` (plan §0.5). "Dùng sau khi move" là E0401 (M6), không phải E0306. Hệ quả nếu cứ làm theo lệnh: mã chết, không test được. → Q25 (3 phương án, mục 4d).
2. **"Drop elaboration" không phải việc của MIR cho M6, và Phase 1 chưa có khái niệm `Drop`.** M6 cần MIR *chưa* elaborate (có `Drop` tường minh ở cuối phạm vi, làm "dùng" cho liveness) — rustc cũng chạy borrowck trước elaboration. Elaboration (cờ drop) chỉ cần cho codegen (M10). Và spec/plan không định nghĩa trait `Drop` (không có trong danh sách `#lang` đã duyệt; `Box/Vec` của lib/core M12 cần nó). → Q26. M5 làm: `needs_drop(ty)`, `Drop` tường minh, phân tích MaybeInit/MaybeUninit; KHÔNG làm cờ drop (để M10).
3. **"Mỗi borrow có region" phải hiểu là *biến* region (suy luận ở M6), không phải region cố định.** Và `Ty` của gt-types không chứa region (ADR-019, chủ ý). Vậy MIR phải mang bảng riêng "vị trí region của từng local" (mục 2). Nếu không, M6 phải sửa MIR — đúng điều lệnh muốn tránh.
4. **Không tạo crate `gt-hir`** (plan §3.1/crates/TASK.md liệt kê nó). "HIR có kiểu" = AST + các bảng của `TypesOutput` (kiểu theo NodeId, phân giải phương thức, kiểu biến cục bộ). Một crate HIR sao chép AST + gắn kiểu là ~2× mã cho một lớp mà hạ-xuống-MIR đọc một lần rồi bỏ. → Q27 (lệch plan, cần duyệt).
5. **Bù cho "không đột biến" bằng bộ kiểm cấu trúc + kiểm tra ngữ nghĩa (mục 7)**: validator MIR chạy trên mọi ca, và một bộ thông dịch MIR cỡ nhỏ để so kết quả với giá trị mong đợi của chương trình sinh ngẫu nhiên. Nếu chỉ golden test như lệnh, lỗi hạ-xuống *nhất quán* (vd sai thứ tự đánh giá) vẫn qua, vì golden khoá chính cái sai đó.

### 1. Giao diện với gt-types (điều kiện tiên quyết, việc đầu tiên của M5)
`TypesOutput` hiện chỉ có bảng kết quả; hạ-xuống-MIR còn cần truy vấn: trường/biến thể theo chỉ số, chữ ký hàm, bảng impl, lang item, `is_copy(ty, phạm_vi_ràng_buộc)`. **Đề xuất:** gt-types xuất `TypeDb` (ảnh chụp chỉ-đọc của `Env`: ADT, trait, chữ ký, impl, lang) + `is_copy`/`needs_drop` có nhận ràng buộc của hàm đang xét. Là thay đổi giao diện của gt-types (thêm, không đổi hành vi M4); có test.

### 2. (a)+(b) Biểu diễn MIR
- **Một thân (`Body`)** cho mỗi hàm/thành viên/`□`/`■`; thân generic hạ **đúng một lần** (đa hình, chứa `Ty::Param`), mọi quyết định phụ thuộc kiểu dùng ràng buộc của hàm: `Copy`/`Move` theo `T: Copy` trong phạm vi; lời gọi trên `T: Trait` là `Callee::TraitMethod` (không giải sớm); layout/instance là việc của M10 qua bảng instance (ADR-018). *Đánh đổi:* codegen phải thế kiểu; đổi lại `use-after-move` của `ƒ f<T>(x: T) { (x, x) }` được kiểm một lần và đúng cho mọi `T` (không phụ thuộc instance).
- **Cấu trúc** (chỉ mục là newtype, mảng `Vec` — KHÔNG `HashMap` để tất định):
  `Body { name, node, arg_count, locals: Vec<LocalDecl>, blocks: Vec<BlockData>, borrows: Vec<BorrowData>, scopes: Vec<SourceScope>, region_count }`.
  `LocalDecl { ty: Ty, regions: Vec<RegionVid>, kind: Return|Arg|User{name, id}|Temp, mutable, span }` — `regions` là một biến cho MỖI vị trí tham chiếu của `ty` theo thứ tự duyệt trước (cùng quy ước `RegionSig` của ADR-019); `'static` là hằng riêng.
  `BlockData { statements, terminator }` (không có khối cleanup: panic = abort tức thì, plan §3.1).
  `Statement { kind, source: SourceInfo{span, node, scope} }`; `StatementKind = Assign(Place, Rvalue) | StorageLive(Local) | StorageDead(Local) | FakeRead(Cause, Place) | Nop`.
  `Place { local, projection: Vec<Elem> }`, `Elem = Deref | Field(idx, Ty) | Index(Local) | Downcast(variant)`.
  `Operand = Copy(Place) | Move(Place) | Const(Const)`.
  `Rvalue = Use | Repeat | Ref(BorrowId, Place) | Len(Place) | Cast(kind, Operand, Ty) | BinaryOp(BinOp, Operand, Operand) | UnaryOp | Discriminant(Place) | Aggregate(Tuple | Array(Ty) | Adt(def, variant, args), Vec<Operand>)`; `BinOp` giữ RIÊNG phép cộng thường / bao quanh (`+%`) / bão hoà (`+^`) / kiểm tra (`+?`) để M7 gắn Obligation đúng phép.
  `Terminator = Goto | SwitchInt{discr, targets, otherwise} | Return | Unreachable | Drop{place, target} | Call{callee, args, arg_spans, dest, target: Option<BB>}` (`target = None` khi callee trả `!`). `Callee = Fn(node, args) | TraitMethod{trait, member, self_ty, args} | Value(Operand)`.
  `BorrowData { place, kind: Shared|Unique, region: RegionVid, source }` — mỗi `Rvalue::Ref` mang `BorrowId` duy nhất.
- **Hạ xuống:** `∧ ∨ ⇒` thành nhánh CFG (giữ đánh giá ngắn mạch); `⎇`/`⌥` thành `SwitchInt` tuần tự theo nhánh (vét cạn đã được M4 chứng minh nên nhánh cuối không rơi tới `Unreachable` trừ khi kiểm sai → ICE); `⟳` cả ba dạng; `⟳ x ∈ a..b` và `∈ slice` thành vòng chỉ số tường minh (`i` tăng bằng `+%`, so sánh `i < b` TRƯỚC khi tăng nên không tràn ở `b = MAX`; `a..=b` dùng cờ); `⏎/⊠/⊡` thoát phạm vi: phát `StorageDead`/`Drop` theo thứ tự ngược cho mọi biến của phạm vi bị thoát; `⚠ ⊢ …` hạ thân bình thường, `SourceScope.unsafe_ctx = true` (M7 dùng cho E1001).
- **Phạm vi biến tạm và `StorageLive/Dead`:** biến người dùng sống từ khai báo tới hết khối; biến tạm hết ở cuối câu lệnh, NGOẠI LỆ "kéo dài" khi vế khởi tạo của `≔` là `&e`/`&!e` với `e` là rvalue: tạm sống tới hết khối (như Rust). Không có ngoại lệ này `r ≔ &mk();` sẽ luôn bị M6 báo dangling. → Q28.
- **Autoref phương thức (`v.push(v.len())`):** ba cách — (A) mượn hai pha như Rust (phức tạp, M6 phải biết "đặt chỗ"), (B) **autoref muộn**: đánh giá đối số trước, tạo khoản mượn ngay trước `Call` (đơn giản, an toàn bộ nhớ; chương trình mà đối số làm thay đổi người nhận được chấp nhận với nghĩa "đối số đánh giá trước", Rust từ chối), (C) cấm. **Đề xuất B** (người nhận là *chỗ* nên đánh giá nó không có tác dụng phụ). → Q28.
- **Hợp đồng `⊳ ⊲ ↓` và lượng từ:** M5 KHÔNG hạ chúng thành CFG; `Body.contracts` chỉ ghi `NodeId` biểu thức + ánh xạ tên tham số → `Local`. Hạ thật (kể cả `∀ ∃` có biên thành vòng) làm ở M7 (VC) / M10 (kiểm lúc chạy). *Đánh đổi:* M6 không thấy "đọc" trong hợp đồng — vô hại cho aliasing vì hợp đồng chỉ đọc giá trị tham số không giữ khoản mượn. → Q29.
- **`□`/`■`:** thân không tham số (`kind = Const/Static`), cùng cấu trúc, để M7 (hằng) và M10 dùng.

**Danh sách M6 cần (và nếu thiếu sẽ buộc sửa MIR):** (1) `BorrowId` + `BorrowData` cho mọi `Ref`; (2) `LocalDecl.regions` cho mọi local/tạm và `RegionSig` (ADR-019) cho callee; (3) `StorageLive/Dead` đủ trên mọi đường thoát (kể cả `⏎/⊠/⊡`); (4) `Drop` tường minh, `Arena` luôn `needs_drop` (để khoản mượn Arena xung đột với drop → E0405); (5) `FakeRead` cho scrutinee của `⌥` và vế khởi tạo `≔` (để khoản mượn đang sống bị coi là đã dùng); (6) `Deref` tường minh cho mọi autoderef (reborrow `&*p` là `Ref` của `Place{p, Deref}`); (7) `Move` vs `Copy` đã quyết bằng `is_copy` có ràng buộc; (8) `SourceInfo` mỗi lệnh/terminator + `arg_spans` mỗi `Call` (nhãn "giá trị bị move ở đây" cần vị trí từng đối số); (9) `Assign` mọi vế trái qua `Deref/Field/Index` (cho E0406); (10) move path theo trường (mục 3); (11) quyết định autoref muộn (trên) — nếu đổi sang hai pha thì MIR phải thêm `Rvalue::Ref` hai pha.

### 3. (c) Khung dataflow (`gt-dataflow`)
- `trait Lattice: Clone + Eq { fn join(&mut self, other: &Self) -> bool }` (trả `true` nếu đổi). Miền cài sẵn: `BitSet<I>` (union) và `Dual<BitSet<I>>` (giao, cho phân tích "chắc chắn").
- `trait Analysis { type Domain: Lattice; const DIRECTION: Forward|Backward; fn bottom(&self, &Body) -> Domain; fn initial(&self, &Body) -> Domain; fn statement_effect(...); fn terminator_effect(...); fn call_return_effect(...) }` — `call_return_effect` riêng vì `dest` của `Call` chỉ được gán trên cạnh quay về.
- **Bộ giải:** hàng đợi ưu tiên theo thứ tự duyệt sau-đảo (RPO; ngược lại cho backward), lặp đến điểm cố định; thứ tự cố định nên kết quả tất định; trần số vòng lặp (khối × chiều cao dàn) vượt thì ICE chứ không treo. `Results { entry_state[block] }` + `ResultsCursor` để đọc trạng thái tại một `Location` bằng cách phát lại.
- **Cài sẵn:** `MaybeInit`, `MaybeUninit` (trên *move path*: cây theo `Field/Downcast`, không qua `Deref/Index`; di chuyển một phần được MIR biểu diễn nhưng chính sách dùng sau di chuyển một phần là của M6), `Liveness` (backward, theo `Local`; `StorageDead` và gán toàn phần là "def").
- Tính chất kiểm bằng test thuộc tính (PRNG tự viết như `order_independence`, KHÔNG thêm `proptest`/`insta` — ADR-009): đơn điệu (join không làm giảm), kết quả không phụ thuộc thứ tự khởi tạo hàng đợi, độc lập thứ tự đánh số khối.

### 4. (d) Kiểm đầy đủ ở M5
- **a. E0307 (thiếu đường trả về):** khi thân không có `tail` (hoặc đuôi là `⎇` thiếu `⇝`) và kiểu trả về ≠ `()`, lối "rơi khỏi cuối hàm" được hạ thành `Unreachable` có đánh dấu `FallOff`; nếu khối đó **tới được từ entry** → E0307 tại cuối thân hàm. `⏎` ở mọi nhánh làm khối cuối không tới được → không lỗi. Mẫu `E0307_missing_return`, `E0307_trailing_semicolon` đã có.
- **b. Hàm `→ !` (ADR-018 điểm 3):** chứng minh *không có terminator `Return` nào tới được* (cả `⏎` tường minh lẫn rơi cuối). Lời gọi hàm `→ !` là `Call{target: None}`; vòng `⟳ { }` không có `⊠` không có cạnh ra. Có `Return` tới được → E0307 (cùng mã, thông điệp riêng). Chỉ gấp hằng nghĩa đen `true`/`false` ở điều kiện `⟳ cond`/`⎇` (không const-prop, không interval — đó là M7): `⟳ true { }` coi như vô hạn; `⟳ cond { }` với `cond` bất kỳ khác thì có cạnh ra, và `→ !` kèm vòng đó bị từ chối (đóng — fail-closed).
- **c. Match đầy đủ:** ĐÃ làm ở M4 (E0305), không lặp lại (plan §M5 liệt kê trước khi quyết định đó).
- **d. E0306 — cần quyết (Q25):** (A) **giữ ngữ pháp, nghỉ hưu E0306** ở Phase 1 (ghi "dành riêng, không có nguồn kích hoạt"), dataflow `MaybeUninit` vẫn dựng vì M6 cần cho E0401; test E0306 bằng CFG dựng tay ở mức thư viện — *không* thoả quy tắc "mỗi mã ≥3 mẫu fail" nên cần ngoại lệ được ghi. (B) **thêm khai báo trơn** `μx: T;` (gán sau): E0306 thành mã thật, có ≥3 fail/≥3 pass; phải sửa ngữ pháp + gt-ast + parser + fmt + resolver + types (đổi ngôn ngữ — cần duyệt). (C) bỏ mã E0306 khỏi bảng. **Đề xuất A:** không mở rộng ngữ pháp chỉ để có một mã; lợi ích khai báo trơn thấp khi `⎇` là biểu thức.

### 5. (e) MIR dạng text ổn định
Dạng một-dòng-một-lệnh, đánh số cục bộ/khối theo THỨ TỰ TẠO của bộ hạ-xuống (tất định), in khối theo chỉ số, không in địa chỉ/băm/vị trí tuyệt đối:
```
fn main::f(_1: i32, _2: &'#0 [i32]) -> i32 {
    let _0: i32;                 // kết quả
    let _3: i32;                 // user `x`
    bb0: {
        StorageLive(_3);
        _3 = Add(copy _1, const 1_i32);
        _4 = Len((*_2));
        switchInt(move _5) -> [0: bb2, otherwise: bb1];
    }
}
```
`'#n` là biến region; `copy/move/const` tường minh; tên người dùng chỉ ở chú thích. Golden dùng cơ chế snapshot sẵn có của repo (`GRIT_UPDATE_SNAPSHOTS=1`, đọc `git diff`), không thêm `insta` (ADR-009 — plan nhắc `insta` nhưng repo đã chọn tự viết). Mọi thay đổi định dạng là thay đổi có chủ đích của snapshot.

### 6. (f) Cắt phạm vi và thứ tự làm (TDD)
Không làm: borrow check, region inference, interval/Obligation, hạ hợp đồng, cờ drop (elaboration), tối ưu MIR, codegen. Thứ tự: (1) `TypeDb` + `is_copy/needs_drop` ở gt-types; (2) crate `gt-mir`: kiểu + printer + validator + test; (3) hạ xuống theo nhóm cấu trúc (biểu thức → khối/biến → rẽ nhánh/vòng → match → gọi/phương thức → thoát phạm vi/drop) mỗi nhóm có golden; (4) `gt-dataflow` + test đặc tả trên CFG dựng tay (kết quả cố định, plan M5); (5) E0307/`→ !`; (6) corpus: mọi mẫu `pass` hạ được và qua validator; mọi `fail` E0307 đúng mã; docs/errors cho E0307 (≥3+≥3). Fail-closed: MIR chỉ dựng khi M4 sạch; lỗi hạ-xuống bất thường là ICE (E9901), không bao giờ bỏ qua hàm.

### 7. Bù cho việc không đột biến (đề xuất)
1. **Validator MIR** (hàm thuần, chạy sau mỗi lần hạ trong test và ở chế độ debug): mọi khối kết thúc bằng terminator; mọi `Local/BB/BorrowId` hợp lệ; `StorageLive/Dead` cân trên mọi đường (mỗi đường entry→Return); mỗi biến dùng đã `StorageLive`; kiểu `Assign` khớp; `Ref` ↔ `BorrowData` một-một; vị trí nguồn có mặt. Chạy trên TOÀN BỘ corpus pass + ca golden.
2. **Test ngữ nghĩa:** thông dịch MIR cỡ nhỏ (số nguyên, bool, tuple, struct/enum, `⎇ ⌥ ⟳`, gọi hàm) + bộ sinh chương trình ngẫu nhiên tất định tự tính kết quả mong đợi bằng Rust; hạ → thông dịch → so sánh (đúng ý "bảo toàn ngữ nghĩa khi hạ MIR" của plan §9). Bắt lỗi nhất quán mà golden không bắt.
3. Golden CFG + test thuộc tính dataflow (mục 3).
Ước lượng: thông dịch ~1 ngày, validator ~0,5 ngày — rẻ hơn rất nhiều so với một lỗi MIR lọt xuống M6.

### 8. Cần Giang chốt
Q25 (E0306: A/B/C — đề xuất A), Q26 (có trait `Drop`? đề xuất `#lang(drop)`, `⊐ Drop for T` chỉ trong lib/core; `Arena` luôn `needs_drop`; không cờ drop ở M5), Q27 (không crate `gt-hir`), Q28 (autoref muộn + kéo dài tạm của `≔ &e`), Q29 (hợp đồng chưa hạ ở M5), và `TypeDb` ở gt-types (mục 1).

### 9. Quyết định của Giang (2026-10-06) và bổ sung bắt buộc trước khi code — mục này THẮNG các mục 0–8 nếu khác
- **Q25 → A.** KHÔNG thêm khai báo trơn (D3 cấm biến chưa khởi tạo). **E0306 đã nghỉ, không tái sử dụng số**; rút khỏi `error_codes.md` và `gt-diag`. Khởi tạo chắc chắn là **bất biến của MIR**: vi phạm là bug compiler → ICE (E9901), không phải lỗi người dùng; test bằng CFG dựng tay. Mở lại nếu Phase 2 thêm khởi tạo trì hoãn.
  *Chỗ cần nói rõ (của tôi):* bất biến này phải bỏ qua **move**. Sau `y ≔ x` (move) thì `x` "chưa khởi tạo" theo MaybeUninit, và dùng lại `x` là lỗi NGƯỜI DÙNG E0401 (M6), không phải ICE. Vậy kiểm bất biến ở M5 = "mọi lần đọc một local đều có một lần GÁN (không tính move) trên mọi đường kể từ `StorageLive`"; phân tích MaybeUninit có tính move được dựng sẵn cho M6 nhưng KHÔNG sinh lỗi ở M5. Nếu gộp hai cái, mọi use-after-move sẽ thành ICE ở M5.
- **Q26.** `#lang(drop)` (thêm vào danh sách lang item của TCB). MIR giữ `Drop` tường minh, KHÔNG elaborate (elaboration chỉ cho codegen M10). Phase 1 không có `⊐ Drop for T` do người dùng viết: ngoài lib/core là **E0313** (kiểu lang dùng sai — cùng họ với `⊐ Copy for Cap`; mở rộng nghĩa của E0313, ghi vào bảng mã). `needs_drop(ty)`: ADT có impl `Drop` (lib/core), `Arena` (luôn), tuple/mảng/ADT chứa thành phần needs_drop; tham số kiểu `T` → **coi là needs_drop** (MIR đa hình phát `Drop(T)`, instance không cần drop thì codegen bỏ). `Copy` và `&T`/`&!T`/nguyên thuỷ không bao giờ needs_drop.
- **Region:** duyệt "biến region do M6 suy luận" + bảng vị trí region theo từng local (mục 2).
- **Q27.** Không tạo `gt-hir` — **hoãn, không phải bỏ**. Bước hạ MIR là nơi DUY NHẤT đọc AST + bảng của `TypesOutput`; mọi pass sau (M6–M10) chỉ đọc MIR (+ `TypeDb`). GRIT_PLAN §6.1 và crates/TASK.md đã cập nhật.
- **Validator + thông dịch MIR (duyệt có điều kiện):** (a) là **công cụ test, không nằm trong TCB**: thông dịch và bộ sinh chương trình chỉ ở `crates/gt-mir/tests/support/` (không biên dịch vào thư viện); validator là hàm thư viện chỉ có thể TỪ CHỐI (ICE), không thể làm một chương trình sai được chấp nhận, và cũng không được tin. (b) **Đối chứng độc lập:** bộ sinh chương trình ngẫu nhiên TỰ TÍNH kết quả kỳ vọng bằng Rust trong lúc sinh (không dùng chính bước hạ MIR làm chuẩn). (c) Phạm vi: số nguyên, bool, rẽ nhánh, vòng lặp, gọi hàm, struct/tuple/enum. (d) Thông dịch hiểu `Assert`: điều kiện sai ⇒ **abort** (kết quả "abort(loại)"), bộ sinh cũng tính được khi nào chương trình abort (tràn, chia 0, chỉ số) để so.
- **Q28 (duyệt).** Autoref muộn. **Kéo dài tạm — luật cú pháp:** trong câu lệnh `tên [: T] ≔ E ;`, nếu E (bỏ ngoặc) có dạng `&V` hoặc `&!V` và V **không phải biểu thức chỗ** (biến, `self`, `.trường`, `[chỉ_số]`, `*e`), thì biến tạm giữ giá trị V sống tới **cuối khối chứa câu lệnh** (StorageDead ở cuối khối, theo thứ tự ngược khai báo cùng các biến khác). Mọi trường hợp khác (kể cả `&V` lồng trong tuple/struct literal/đối số) tạm chết ở cuối câu lệnh. Chặt hơn Rust (Rust kéo dài cả trong `(&V, …)`, `S { f: &V }`); nới sau được, siết thì khó.
- **Q29 (duyệt).** Hợp đồng/lượng từ hoãn sang M7. Không dùng `insta`/`proptest` (duyệt).
- **Bổ sung 1 — không unwind, không cleanup (D3, panic = abort).** MIR không có cạnh unwind và không có khối cleanup; `Call` chỉ có MỘT đích (`target`), không có đích "khi panic". *Chỗ tôi sửa trong lệnh:* điểm phân kỳ là lời gọi tới hàm `→ !` — biểu diễn bằng `Call { target: None }` (không có khối sau), KHÔNG cần khối `Unreachable` theo sau. `Unreachable` chỉ dùng cho chỗ **không tới được do cấu tạo** (nhánh rơi của `⌥` đã được M4 chứng minh vét cạn; lối rơi cuối hàm `→ !`/hàm có E0307 đã báo). Và M10 **phải hạ `Unreachable` thành abort (trap), KHÔNG phải `unreachable` của LLVM** (UB): nếu chứng minh của compiler sai thì chương trình dừng, không chạy tiếp vào hành vi không xác định — đúng tinh thần fail-closed. Thông dịch MIR: tới `Unreachable` = lỗi test (bug compiler).
- **Bổ sung 2 — `Assert` có từ M5.** `Terminator::Assert { cond: Operand, expected: bool, kind: AssertKind, target }`; sai ⇒ abort. `AssertKind = Overflow(BinOp, a, b) | OverflowNeg(a) | DivisionByZero(a) | RemainderByZero(a) | IndexOutOfBounds { len, index } | CastLossy { from: Ty, to: Ty }` (mỗi loại mang toán hạng để M7 dựng Obligation). Bước hạ MIR phát `Assert` cho MỌI chỗ có nghĩa vụ theo semantics §2/D1/D2: `+ - *` nguyên thường (`CheckedBinaryOp` cho `(giá trị, tràn)` rồi `Assert(!tràn)`), `/ %` (chia 0, và `MIN / -1` với kiểu có dấu), `-x` có dấu, chỉ số mảng/slice (`Len` + so sánh), `as` có thể mất thông tin (thu hẹp, đổi dấu, thực→nguyên). `+% +^ +?` và số thực không có `Assert`. M7 quyết giữ (runtime check + N0601) hay xoá (đã chứng minh); MIR không đổi.
- **Bổ sung 3 — thứ tự đánh giá: trái sang phải** (ghi vào semantics §3a): toán hạng trái rồi phải (`∧ ∨ ⇒` ngắn mạch); lời gọi: callee rồi đối số theo thứ tự; phương thức: các toán hạng của chỗ người nhận (vd chỉ số) rồi đối số, rồi mới tạo khoản mượn autoref (Q28); tuple/mảng/struct literal: theo thứ tự VIẾT trong nguồn (không theo thứ tự khai báo trường); `chỗ ← giá_trị`: các biểu thức con của CHỖ (chỉ số, deref) trước, rồi giá trị, rồi ghi — **khác Rust** (Rust đánh giá vế phải trước với `=`); hệ quả: `a[i] ← f()` với `f` đổi `i` ghi vào vị trí theo `i` CŨ; `a[i] ← …` kiểm chỉ số (`Assert`) trước khi đánh giá vế phải.
- **Golden bắt buộc:** `⊠`/`⊡`/`⏎` xuyên nhiều phạm vi lồng nhau — `StorageDead`/`Drop` theo thứ tự NGƯỢC khai báo, không thiếu, không thừa, trên MỌI đường thoát (validator kiểm cân bằng trên mọi đường; golden khoá thứ tự).

## ADR-022 — Các quyết định phát sinh khi cài M5 (Giang DUYỆT 2026-10-06, Q34; kèm Q24f và `TypeDb`)
1. **Lỗi ngữ nghĩa có sẵn từ M4: `a..=b` làm GIÁ TRỊ mất "gồm b".** `m ≔ 1..=2` (mẫu pass 23 đã duyệt) có kiểu `Range<i32>` giống hệt `1..2`, nên `⟳ i in m` sẽ lặp thiếu phần tử cuối. Test cũ của M4 còn KHOÁ đúng cái sai đó (`1u8..=9` → `Range<u8>`). Sửa: lang item mới **`#lang(range_inclusive)`** (`RangeInclusive<T> { start, end }`); `a..=b` có kiểu này, lặp/`∈` hiểu cả hai. *Cần duyệt:* thêm vào danh sách lang item của TCB (GRIT_PLAN §3.3) — tôi đã cài vì không có cách hạ MIR đúng nếu thiếu; nếu bác thì phải cấm `a..=b` làm giá trị (đổi luật ngôn ngữ, mẫu 23 sai).
2. **Giao diện gt-types → MIR (ADR-020 §1):** `TypesOutput` thêm `db: TypeDb` (bảng ADT/trait/chữ ký/impl/lang + `is_copy_resolved`, `needs_drop`, `sig_by_node`), `sites` (mỗi chỗ dùng hàm: đích, đối số kiểu đã giải, `self`, số lần autoderef), `bodies`, `def_by_node`, `file_names`, `scope_of`. Không đổi hành vi M4 (test cũ giữ nguyên, trừ mục 1).
3. **Khoá hàm impl ổn định:** `module::Kiểu::hàm` và `module::<Kiểu as Trait>::hàm` thay cho `impl#<NodeId>::hàm` (cũ phụ thuộc NodeId: golden vỡ khi prelude đổi).
4. **Biến tạm của biểu thức ĐUÔI một khối chết ở cuối khối** (không phải cuối câu lệnh bao ngoài như Rust): `x ≔ { &mk() };` sẽ bị M6 coi là dangling. Chặt hơn Rust; nới sau được.
5. **Người nhận theo giá trị bị move SAU đối số** (cùng nguyên tắc autoref muộn, Q28): `x.consume(x.len())` hợp lệ (Rust từ chối). Hệ quả ngữ nghĩa nhất quán với "đối số đánh giá trước khi người nhận bị tiêu thụ".
6. **MIR M6 nhận chỉ gồm khối TỚI ĐƯỢC** (bước hạ bỏ mã chết sau `⏎ ⊠ ⊡`/lời gọi `→ !` và đánh số lại khối + khoản mượn). `Drop` phát vô điều kiện cho mọi biến cần drop ở cuối phạm vi, kể cả khi giá trị đã bị move (`_0 = move _1; drop(_1)`): elaboration (M10) bỏ đi; M6 dùng MaybeUninit để biết drop đó là no-op.
7. **Bất biến "đã gán trước khi đọc" bỏ qua move và `Drop`** (đã nói ở ADR-020 §9): drop một chỗ có thể chưa khởi tạo là hợp lệ; dùng sau move là E0401 của M6.
8. **Chưa hạ ở M5 → ICE E9901 (không bao giờ lặng lẽ bỏ hàm):** lượng từ ngoài hợp đồng (M7), hàm tạo biến thể có trường dùng như giá trị (`f ≔ Some;`), phương thức trait dùng như giá trị, `⟳ c in 'a'..='z'` (range ký tự). Không mẫu nào của corpus dùng các dạng này (test kiểm).
9. **`→ !` "thật sự phân kỳ":** kiểm ở MIR (không `Return` nào tới được) — nhưng M4 đã buộc thân có kiểu `!` nên KHÔNG có chương trình nguồn hợp lệ nào kích hoạt nhánh này; nó là lưới an toàn, test bằng CFG dựng tay.

## ADR-023 — M6: borrow checker (Giang DUYỆT 2026-10-06 — xem mục 10, 11 bổ sung)
**Phạm vi:** crate `gt-borrowck` đọc MIR của M5 (`gt-mir`) + `gt-dataflow` + `RegionSig`/`TypeDb` của gt-types; không đọc AST. Mã lỗi E0401–E0406 (+ đề xuất ở §8). Đột biến ĐẦY ĐỦ (chính sách: M6 có đột biến toàn crate).

### 0. Những chỗ cần nói thẳng trước khi duyệt
1. **MIR của M5 còn thiếu MỘT thứ M6 cần: reborrow ngầm khi truyền `&!T`.** Hiện `f(r)` với `r: &!T` sinh `move _r` — sau lời gọi `r` dùng lại sẽ là E0401, khác hẳn Rust (Rust tự chèn `&mut *r`). Đề xuất: bước hạ MIR chèn `Ref(Unique, (*r))` ở mọi điểm ép kiểu (đối số, `≔` có chú thích kiểu, `⏎`) khi giá trị là một CHỖ kiểu `&!T`; tương tự cho `&T` không cần (Copy). Đây là sửa MIR, làm ĐẦU TIÊN của M6, có golden/validator. (Đúng điều lệnh M5 muốn tránh — tôi nêu vì phát hiện khi thiết kế M6, không phải khi thiết kế MIR.)
2. **Không thể cấm move một phần ở Phase 1** dù mới nghe có vẻ đơn giản hơn: bước hạ `⌥` gán biến của pattern bằng cách move TRƯỜNG của scrutinee (`p ≔ move (_1 as Some).0`). Cấm move một phần ⇒ `⌥ o { Some(p) ↦ dùng(p), None ↦ … }` với `p` không Copy bị từ chối. Xem §3.
3. **"Quy tắc phiên bản của Arena"** — tôi hiểu là: mọi thao tác làm Arena sang "phiên bản" mới (reset/clear, move, drop) VÔ HIỆU mọi tham chiếu đã cấp từ phiên bản cũ. Nếu ý Giang khác (vd đếm thế hệ lúc chạy), cần nói lại; §5 viết theo cách hiểu tĩnh này.
4. **Bảng mã E04xx hiện có không đủ phủ hai lỗi phổ biến:** "giá trị bị mượn không sống đủ lâu" khi KHÔNG phải trả về (Rust E0597: `r` ở phạm vi ngoài mượn `x` ở phạm vi trong) và "move ra khỏi tham chiếu/chỉ số" (Rust E0507). Đề xuất ở §8 (Q31).

### 1. Thuật toán: NLL theo liveness + ràng buộc region, tập khoản vay
- **Biến region:** mọi vị trí region trong `LocalDecl.regions` + region của mỗi `BorrowData` (M5 đã cấp). **Region phổ quát** (của chữ ký: `Named`, `Anon(n)` của đầu vào, `'static`) chứa MỌI điểm của thân + điểm kết thúc của riêng nó (`end('a)`).
- **Giá trị region** = tập điểm CFG (`Location`) + tập `end(..)`. Giải bằng lan truyền tới điểm cố định (khung gt-dataflow cho liveness; lan truyền ràng buộc bằng worklist tất định).
- **Ràng buộc** (NLL Rust 2018, không phụ thuộc vị trí cho `outlives`):
  - *Liveness:* nếu local `x` sống tại điểm P (gt-dataflow `Liveness`, có tính drop-use) thì mọi biến region trong kiểu của `x` chứa P.
  - *Gán:* `dst = rvalue` sinh `'src_i : 'dst_i` theo từng vị trí region tương ứng (hiệp biến cho `&`; với `&!T` bất biến theo `T` ⇒ ràng buộc hai chiều cho region bên trong `T`).
  - *Mượn:* `dst = &'b place` sinh `'b : 'dst_0`; nếu `place` đi qua `*p` với `p: &'p T` thì `'p : 'b` (reborrow không sống lâu hơn tham chiếu gốc).
  - *Lời gọi:* khởi tạo `RegionSig` của callee với biến mới cho từng tên/vị trí ẩn (`Static` → `'static`); `'arg : 'param` cho mỗi vị trí của mỗi đối số; `'ret : 'dst` cho đích.
  - *Trả về:* `'_0 : 'out_i` với region đầu ra phổ quát ⇒ mọi khoản vay chảy vào `_0` phải chứa `end('out)`.
- **Khoản vay (loan):** mỗi `BorrowData` = (chỗ, Shared|Unique, region). Khoản vay **còn hiệu lực** tại P nếu P ∈ giá trị region của nó và chưa bị *kill* (gán đè toàn bộ local gốc của chỗ, hoặc `StorageDead` của local đó). Tập khoản vay hiệu lực tính bằng gt-dataflow (tiến, hợp: gen ở `Ref`, kill như trên, lọc theo giá trị region).
- **Kiểm truy cập:** ở mỗi điểm, mỗi truy cập vào chỗ X (đọc nông/sâu, ghi, move, `Drop`, `StorageDead`) đối chiếu các khoản vay hiệu lực *chồng lấn* X (tiền tố/hậu tố theo phép chiếu; `Index` coi như chồng lấn mọi phần tử):
  - tạo `&!` khi có khoản vay khác chồng lấn còn hiệu lực ⇒ **E0402** (khoản kia `&!`) / **E0403** (khoản kia `&`); tạo `&` khi có `&!` chồng lấn ⇒ **E0403**;
  - ghi / move ra khi có khoản vay chồng lấn ⇒ **E0406**;
  - `StorageDead`/`Drop` của local khi có khoản vay hiệu lực mà region chứa điểm sau đó (khoản vay sống quá biến) ⇒ **E0404** (nếu region phổ quát/chảy vào `_0`: trả tham chiếu tới local) hoặc **E0405** nếu local có kiểu `Arena` (`Lang::Arena`); trường hợp không trả về nhưng sống quá ⇒ mã đề xuất ở §8.
  - đọc một move path có thể chưa khởi tạo (gt-dataflow `MaybeUninit`) ⇒ **E0401**; `Drop` của chỗ có thể chưa khởi tạo KHÔNG lỗi (no-op sau elaboration).
- **Đánh đổi:** NLL 2018 có dương tính giả đã biết ("problem case #3": trả tham chiếu có điều kiện qua nhiều nhánh) mà Polonius sửa được. Polonius chính xác hơn nhưng phức tạp gấp nhiều lần và chưa ổn định ở chính Rust. **Đề xuất NLL 2018**, ghi KNOWN_LIMITATIONS các dương tính giả; mọi dương tính giả là chặt hơn (an toàn), không phải lỗ hổng.

### 2. Reborrow
- Tường minh `&*p`, `&!*p`: một khoản vay mới trên `(*p)` với `'p : 'b`. Mượn duy nhất qua `p: &T` (`&!*p`) là E0308 của M4 (đã có).
- Ngầm (đề xuất §0.1): đối số, `≔` có chú thích kiểu `&!T`, `⏎`. Autoref của phương thức đã tường minh trong MIR (autoref muộn, Q28) và tự là reborrow khi người nhận đi qua `*`.
- Khoản vay qua reborrow chặn truy cập vào `p` (gốc) như khoản vay trực tiếp, cho tới khi reborrow hết hiệu lực (NLL: lần dùng cuối).

### 3. Move một phần (struct/tuple)
- **Đề xuất: CHO PHÉP** move ra khỏi trường của một local (struct/tuple, và trường của biến thể sau `Downcast` — chính là gán biến trong `⌥`), theo move path của gt-dataflow (đã có). Dùng lại trường đã move hoặc cả local ⇒ E0401; dùng trường KHÁC chưa move ⇒ hợp lệ (như Rust).
- **Cấm** (như Rust): move ra khỏi chỗ đi qua `*` (tham chiếu, con trỏ thô) hoặc `[i]`/lát; move một phần khỏi giá trị có `⊐ Drop` (lib/core) vì drop của nó cần cả giá trị. Mã: đề xuất §8 (E0407).
- **Vì sao không cấm hẳn:** xem §0.2 — match trên dữ liệu không Copy sẽ không viết được.

### 4. Mượn qua lời gọi hàm theo `RegionSig`
- Ở mỗi `Call`, `RegionSig` của callee (M4.5, kể cả `Holder<'a>` của ADR-021) được khởi tạo bằng biến region MỚI: cùng tên ⇒ cùng biến (vd `ƒ pick<'a>(x: &'a T, y: &'a T) → &'a T` buộc kết quả sống không lâu hơn GIAO của hai đầu vào); `Anon(n)` ⇒ mỗi vị trí một biến; `Static` ⇒ `'static` (M6 kiểm thân hàm loại này thật sự chỉ trả dữ liệu `'static` — Q24d).
- Callee qua trait (`Callee::TraitMethod`) dùng `RegionSig` của thành viên TRAIT (đã kiểm khớp với impl ở M4 về kiểu; về region: thành viên impl phải có cùng `RegionSig` sau đổi tên — kiểm ở M6, lỗi là E0311 mở rộng hoặc mã mới, xem Q31).
- Khoản vay của đối số chưa kết thúc khi hàm trả về nếu region của nó chảy vào đầu ra (vd `first(&v)` trả `&T` ⇒ `v` còn bị mượn tới lần dùng cuối của kết quả).

### 5. Arena và "phiên bản" (theo cách hiểu ở §0.3)
- `a.alloc(x)` = khoản vay **chung** trên `a` (chữ ký `alloc(&'r self, …) → &'r T`); mọi tham chiếu cấp ra mang region của khoản vay đó.
- Thao tác tạo "phiên bản mới" của Arena cần `&!a` hoặc move/drop `a`: `reset(&!self)`/`clear(&!self)` (lib/core M12), gán đè `a`, move `a`, `Drop`/`StorageDead` của `a`. Mỗi thao tác này xung đột với mọi khoản vay chung còn hiệu lực trên `a` ⇒ tham chiếu từ phiên bản cũ còn được dùng sau đó là lỗi: **E0405** (khi xung đột với drop/StorageDead/move — tham chiếu thoát khỏi Arena) và **E0403** (khi xung đột với `&!a` của reset/clear — chọn E0403 để không thêm mã; hoặc dùng E0405 cho mọi trường hợp có `Lang::Arena`, Q32).
- Arena là tham số `a: &'r Arena`: trả `&'r T` hợp lệ (region phổ quát `'r`); Arena cục bộ: trả tham chiếu vào nó ⇒ E0405.

### 6. Nới hay chặt hơn Rust (và hệ quả)
| Điểm | So với Rust | Hệ quả |
|---|---|---|
| Autoref muộn (Q28) thay mượn hai pha | **Nới hơn** | `v.push(v.len())`, `x.consume(x.len())` hợp lệ; nghĩa xác định: đối số trước. Không có chương trình mất an toàn bộ nhớ. |
| Phép gán: chỗ trước, giá trị sau (semantics §3a) | Khác | `a[i] ← f()` ghi theo `i` cũ; borrowck xét theo đúng thứ tự MIR. |
| Elision chặt, không `'a: 'b` (ADR-019/021) | **Chặt hơn** | Một số chữ ký hợp lệ chưa viết được (KNOWN_LIMITATIONS). |
| Tạm của đuôi khối chết ở cuối khối (ADR-022 §4) | **Chặt hơn** | `x ≔ { &mk() };` lỗi (Rust chấp nhận). |
| NLL 2018, không Polonius | Bằng Rust 2018 (chặt hơn Polonius) | Dương tính giả "problem case #3". |
| Reborrow ngầm chỉ ở đối số/`≔ : &!T`/`⏎` | Hẹp hơn Rust một chút | Một số chỗ cần viết `&!*r` tường minh. |
| Move một phần | Bằng Rust | — |
| Arena: mọi thao tác "phiên bản" xung đột tham chiếu cũ | Tương đương lifetime của Rust với `&mut self` | Không có kiểm lúc chạy. |

### 7. Corpus (tiêu chí hoàn thành plan M6): ≥150 ca XẤU chặn đúng mã, ≥150 ca ĐÚNG không chặn nhầm
Mỗi nhóm có cả biến thể trong nhánh `⎇`, trong `⌥`, trong vòng lặp (qua cạnh lùi), qua lời gọi, qua phương thức, và qua nhiều phạm vi lồng nhau. Số tối thiểu:
| Nhóm | Xấu (mã) | Đúng tương ứng |
|---|---|---|
| Dùng sau move (cả local, một phần, trong vòng lặp, sau lời gọi, sau `⌥` move trường) | 30 (E0401) | 30 (move rồi gán lại; dùng trường khác; Copy) |
| Hai `&!` chồng nhau (trực tiếp, qua trường/chỉ số, qua reborrow, qua phương thức `&!self`) | 20 (E0402) | 20 (NLL: lần dùng cuối trước khi mượn lại) |
| `&!` chồng `&` | 20 (E0403) | 20 |
| Trả tham chiếu tới local/tạm/tham số theo giá trị | 15 (E0404) | 15 (trả tham chiếu từ tham số; `'static`) |
| Tham chiếu thoát Arena (trả về, sống quá phạm vi, sau reset/move/drop Arena) | 20 (E0405) | 20 (Arena là tham số; dùng trong phạm vi) |
| Gán/move khi đang bị mượn | 20 (E0406) | 20 |
| Sống không đủ lâu (không trả về), move ra khỏi tham chiếu | 15 (mã §8) | 15 |
| Mượn qua lời gọi theo `RegionSig` (`pick<'a>`, `Holder<'a>`, elision) | 15 (các mã trên) | 15 |
| **Tổng** | **≥155** | **≥155** |
Mọi dương tính giả/âm tính giả phát hiện về sau thêm vào corpus (plan). Thêm: test thuộc tính "thêm một lần dùng sau cùng vào chương trình đúng không làm mất lỗi cũ" và đột biến đầy đủ crate gt-borrowck.

### 8. Mã lỗi cần chốt (Q31)
- **Mở rộng nghĩa E0404:** "tham chiếu sống lâu hơn giá trị bị mượn" (gồm trả về tham chiếu tới local, và biến ở phạm vi ngoài giữ tham chiếu tới biến phạm vi trong). Hoặc thêm **E0408** riêng cho trường hợp không trả về. Đề xuất: **mở rộng E0404** (ít mã, cùng bản chất).
- **Mã mới E0407:** move ra khỏi chỗ đi qua `*`/`[i]`/lát, hoặc move một phần khỏi giá trị có `⊐ Drop`.
- Không cần mã mới cho Arena (E0405) hay reborrow.

### 9. Thứ tự làm (TDD) và kiểm chứng
(1) reborrow ngầm trong bước hạ MIR + golden (§0.1); (2) crate gt-borrowck: biến region, ràng buộc, giải; (3) khoản vay hiệu lực (dataflow); (4) kiểm truy cập → E0402/E0403/E0406; (5) E0401 từ MaybeUninit; (6) E0404/E0405 từ StorageDead/Drop; (7) lời gọi theo `RegionSig`; (8) corpus ≥155+≥155, docs/errors cho mọi mã E04xx (≥3+≥3); (9) đột biến toàn crate gt-borrowck, mọi đột biến sống sót có test mới. Thông báo lỗi chỉ ra: chỗ mượn, chỗ dùng xung đột, và lần dùng sau khiến khoản vay còn sống (NLL), với vị trí từ `SourceInfo`/`arg_spans` của MIR.

### 10. Duyệt của Giang (2026-10-06) và bổ sung — mục này THẮNG các mục 0–9 nếu khác
Giang duyệt theo bản tóm tắt; tôi đã đối chiếu từng điểm với mục 0–9: **không có điểm nào mâu thuẫn**. Hai chỗ làm rõ:
- **"Kiểu có Drop (hiện chỉ Arena)"** (ràng buộc E0407): ở §3 tôi viết "giá trị có `⊐ Drop`" — nay đúng là **kiểu có `⊐ Drop for` HOẶC là `Arena`** (`Lang::Arena`, cần drop dựng sẵn). Move một TRƯỜNG ra khỏi giá trị thuộc loại đó là **E0407**. Với kiểu KHÔNG có Drop, move một phần được phép; **cờ drop theo từng trường là việc của codegen M10** (elaboration, ADR-020 §9/ADR-022 §6): MIR của M5/M6 giữ `Drop` vô điều kiện cho cả local, M6 dùng MaybeUninit để biết phần nào đã move.
- **Q32 (mã khi xung đột với Arena):** tôi chọn **E0405 cho MỌI thao tác làm Arena sang "phiên bản" mới** — move, `Drop`/`StorageDead`, gán đè, và tạo `&!a` (reset/clear) — khi còn khoản vay tới tham chiếu cấp từ phiên bản cũ. Một quy tắc, một mã, đúng D27; E0402/E0403 không dùng cho Arena. (Giang duyệt "Arena theo nghĩa tĩnh" nhưng chưa chọn mã; nếu muốn E0403 cho `reset` thì nói.)

Quyết định đã duyệt: **NLL 2018** ("sound nhưng không complete, P5; Polonius xét ở Phase 2" — KNOWN_LIMITATIONS); **move một phần** + ràng buộc trên; **Arena tĩnh**; **reborrow ngầm sửa MIR ĐẦU M6, commit riêng "M6-mir: implicit reborrow"**; **mã lỗi: mở rộng E0404, thêm E0407** (đã đối chiếu `error_codes.md`: E0407/E0408 chưa dùng); bảng nới/chặt §6 (bắt buộc đầy đủ — thêm các dòng ở §11).

Làm thêm (Giang):
- **(a) Đột biến toàn crate gt-borrowck**, mọi đột biến sống sót có test mới.
- **(b) Oracle động** (công cụ test, NGOÀI TCB, độc lập với borrowck — không dùng lại mã phân tích của nó): thông dịch MIR có theo dõi khoản vay/move/Arena; bộ sinh chương trình ngẫu nhiên. Chương trình borrowck CHẤP NHẬN mà thông dịch phát hiện vi phạm = false negative thật: vào corpus vĩnh viễn + sửa borrowck. Chiều ngược lại (chương trình bị từ chối mà chạy sạch trên mọi nhánh) chỉ ĐO và báo cáo.
- **(c) Soundness hunting:** chương trình tấn công (alias qua reborrow/generic/match/vòng lặp, move một phần rồi dùng, mượn rồi reset Arena); mỗi cái thành test vĩnh viễn.
- **(d) Thống kê false positive** trên corpus đúng và trên các mẫu `pass` cũ, ghi PROGRESS.md.

### 11. Chi tiết thiết kế bổ sung khi cài (cần ghi để review)
- **Phạm vi oracle động (nói thẳng giới hạn):** nó chỉ thấy vi phạm trên các ĐƯỜNG ĐÃ CHẠY (bộ sinh dùng tham số `c: bool` và chạy cả hai giá trị để phủ cả hai nhánh; vòng lặp có cận cố định) nên "oracle sạch" là bằng chứng KIỂM THỬ, không phải chứng minh. Vi phạm được định nghĩa bằng PHÂN TÍCH VẾT sau khi chạy (một khoản vay bị truy cập xung đột rồi còn được DÙNG về sau), không bằng liveness tĩnh.
- **Region trong ADT:** không có ánh xạ trường → tham số lifetime (Ty không chứa region). Đọc/ghi trường của ADT dùng "bó" (mọi region của ADT outlive/được outlive bởi giá trị trường — chặt hơn, sound); gán nguyên giá trị ADT so khớp theo từng vị trí; biến thiên của ADT: hiệp biến nếu ADT không chứa `&!`/con trỏ thô/kiểu hàm (kể cả lồng), ngược lại bất biến. Hệ quả: một số chương trình hợp lệ có ADT giữ `&!` bị từ chối (KNOWN_LIMITATIONS).
- **Bound ngầm (implied bounds) KHÔNG dùng:** `ƒ f<'a,'b>(x: &'a &'b T)` không ngầm có `'b: 'a`. Chặt hơn Rust.
- **Không "static promotion":** `&1`, `&CONST` trong thân hàm không tự thành `'static`; `ƒ f() → &i32 { &1 }` là E0404. (Chốt điểm mở của `regions.md` §5.) Literal chuỗi là `&'static str`.
- **Lỗi vi phạm region phổ quát** (vd `ƒ f<'a,'b>(x: &'a T, y: &'b T) → &'a T { y }`) báo **E0404** (tham chiếu không sống đủ lâu theo chữ ký).
- **Phép gán `chỗ ← v` qua `*p` không xung đột với khoản vay `p` đang giữ** (truy cập đi QUA chính khoản vay đó); thứ tự đánh giá theo MIR.
- **Bảng nới/chặt bổ sung (§6):** ADT bó/bất biến — chặt hơn; không implied bounds — chặt hơn; không static promotion — chặt hơn; `Drop` của ADT có tham chiếu giữ region sống tới điểm drop (không `may_dangle`) — chặt hơn; Arena gộp một mã E0405 — khác Rust (không có Arena); move một phần khỏi Arena/kiểu có Drop bị cấm — bằng Rust (E0509).


## ADR-024 — M6: oracle động, độ phủ bộ sinh, tấn công hai phía (theo lệnh của Giang 2026-10-06; ADR-023 §10b)
**Phạm vi:** công cụ TEST ngoài TCB, trong `crates/gt-borrowck/tests/` (`support/oracle.rs`, `support/progen.rs`, `oracle.rs`). Độc lập với mã phân tích của borrowck: không dùng `gt-borrowck`, `gt-dataflow`, liveness/tập khoản vay tĩnh; chỉ đọc cấu trúc MIR của `gt-mir`.

### 1. Định nghĩa vi phạm (phân tích vết, không phải liveness tĩnh)
Thông dịch MIR trên mô hình bộ nhớ có theo dõi. Mỗi `Rvalue::Ref` tạo một **khoản vay** (chỗ cụ thể, `&`/`&!`, cha = khoản vay của tham chiếu mà chỗ đi qua). Mọi **truy cập** (đọc, ghi, move, `StorageDead`/`Drop`/hết khung, tạo `&`/`&!`) vô hiệu mọi khoản vay chồng lấn mà truy cập không đi qua chính nó hay tổ tiên của nó (hậu duệ KHÔNG được miễn). Quy tắc xung đột: khoản vay `&!` xung đột với mọi truy cập; khoản vay `&` chỉ với ghi/move/chết/`&!` mới. **Vi phạm = giải tham chiếu mang một khoản vay (hoặc hậu duệ của nó) đã bị vô hiệu** (dùng tham chiếu đồng thời dùng mọi tổ tiên của nó). Thêm: đọc/mượn/ghi-trường giá trị đã move hoặc chưa gán; truy cập chỗ đã chết; move ra khỏi chỗ đi qua `*`/`[i]`; tham chiếu bị vô hiệu nằm trong GIÁ TRỊ TRẢ VỀ của một hàm (người gọi sẽ dùng nó). Arena: `alloc` trả ô mới với khoản vay là CON của khoản vay chung trên Arena; `reset`/move/drop/gán đè Arena vô hiệu khoản vay chung ⇒ E0405 (Q32).
Mỗi lần chạy tạo khoản vay MỚI theo từng lần thực thi `Ref` (vòng lặp không lẫn các lần lặp).

### 2. Bộ sinh và độ phủ (điều kiện: loại cấu trúc dưới ngưỡng thì CHƯA được tính là đã kiểm)
Chương trình = 1–3 "kịch bản" nối tiếp, mỗi kịch bản trong nhánh `⎇`/vòng lặp (≥2 lần lặp) hay trần; thứ tự hành động trong kịch bản NGẪU NHIÊN (nên nhiều chương trình vi phạm — cố ý). Hàm `entry(c, d, e: bool)` chạy cả 8 tổ hợp; mỗi tầng `⎇` lồng dùng điều kiện riêng (dùng chung một điều kiện làm nhánh `else` của `⎇ c` chứa `⎇ c` không bao giờ chạy — lỗi của bộ sinh đã gặp và sửa). Chín loại cấu trúc đo: `move`, `reborrow`, `borrow_in_loop`, `borrow_via_match`, `partial_move`, `arena_reset`, `call_regionsig`, `nested_struct`, `ref_flow` (tham chiếu chảy qua tuple/mảng/`Option`/`Hs`/`Hm` giữ `&!`/`&&`/hàm generic; thêm sau khi đột biến cho thấy ràng buộc region qua kiểu chưa được phủ) — mỗi chương trình mang tập thẻ do bộ sinh gắn khi dựng. Bộ sinh còn có hành động "gán/truyền tham chiếu sang biến khác" (thêm sau khi săn soundness tìm ra lỗ hổng mà 20 000 chương trình trước đó không phủ).
**Ngưỡng (kiểm bằng test, chỉ ở bản đầy đủ 20 000 chương trình):** mỗi loại phải có **≥ 300 chương trình mà borrowck CHẤP NHẬN và oracle chạy SẠCH** (nơi âm tính giả sẽ lộ ra) **VÀ ≥ 300 chương trình oracle thấy vi phạm** (độ nhạy của oracle trên loại đó). *Lý do:* "quy tắc ba" — với n mẫu độc lập, một lỗi có tỉ lệ ≥ p bị bỏ sót với xác suất (1−p)ⁿ; n = 300 và p = 1% cho < 5%. Con số 300 là lựa chọn của tôi, không phải suy ra từ lỗi đã biết. Chương trình sinh ra phải qua M4/M5 (nếu không là lỗi bộ sinh, test đỏ). Biến `GT_ORACLE_PROGRAMS=n` chạy bản nhanh (đột biến mã nguồn dùng); bản nhanh KHÔNG kiểm ngưỡng.

### 3. Tấn công hai phía
Lấy chương trình HỢP LỆ ở cả hai phía (borrowck chấp nhận ∧ oracle sạch ở 8 tổ hợp), áp MỘT phép đột biến ngữ nghĩa, kiểm cả hai phía: (a) chèn dùng sau move ngay sau một lần move cả biến (`E0401`); (b) chèn khoản vay thứ hai/truy cập xung đột giữa lúc tạo tham chiếu và lần dùng sau trong CÙNG khối (`E0402`/`E0403`/`E0406`, tuỳ loại khoản vay); (c) chèn `reset`/move/gán đè Arena giữa `alloc` và lần dùng sau (`E0405`). Vị trí chèn luôn nằm giữa hai lệnh cùng khối nên tới được khi khối chạy. **Mong đợi:** borrowck báo mã mong đợi VÀ oracle thấy vi phạm; mỗi loại tấn công cần ≥ 300 ca hai phía cùng bắt ở bản đầy đủ. Bất đồng = lỗi ở một trong hai phía (hoặc ở điều kiện tới-được của phép chèn) và làm test đỏ.

### 4. Hiệu chuẩn oracle trên corpus
Chạy `f` của mọi mẫu `bc` có tham số toàn `bool` (≤2): mẫu ĐÚNG phải 0 vi phạm; mẫu XẤU oracle phải thấy ≥ 80%. Phần oracle không thấy được liệt kê (đều có lý do: nhánh không chạy, chỉ số cụ thể, tham chiếu không được dùng, quy tắc tĩnh Drop).

### 5. Chỗ "tỉ lệ dương tính giả" cần đọc cẩn thận (nói thẳng, vì lệnh dùng nó như một con số)
"borrowck từ chối ∧ oracle sạch" **không** đồng nghĩa dương tính giả của borrowck: oracle chỉ thấy các đường đã chạy và dùng chỉ số/thân hàm cụ thể, còn borrowck là phân tích tĩnh, mô-đun hoá theo `RegionSig`, và NLL 2018 chủ ý chặt hơn (ADR-023 §1, §6, §11). Vì vậy báo cáo con số cùng PHÂN LOẠI theo nguyên nhân không-đầy-đủ đã biết: `RegionSig` (kết quả của `pick(&x,&y)` buộc với cả hai đối số dù thân chỉ trả `x`), chỉ số mảng (borrowck coi mọi chỉ số chồng lấn), vòng lặp có cận (mất khi lần lặp ≥ 2), và "KHÁC (cần xem tay)" — nhóm cuối phải bằng 0 hoặc từng ca được giải thích; mọi ca "KHÁC" là ứng viên lỗi và được đọc tay.
Ngược lại, "oracle sạch" chỉ là bằng chứng KIỂM THỬ cho chiều âm tính giả (không phải chứng minh): oracle thấy vi phạm ở n chương trình mà borrowck từ chối cho thấy độ nhạy, còn số âm tính giả = số chương trình borrowck chấp nhận mà oracle thấy vi phạm, và PHẢI bằng 0.

### 6. Giới hạn đã biết của oracle
Chỉ thấy vi phạm trên đường đã chạy; chương trình bộ sinh không có tham số tham chiếu ở `entry`, không số thực, không lời gọi qua trait; quy tắc tĩnh "move một trường khỏi kiểu có `⊐ Drop`" (E0407) không được mô hình hoá (thân hàm của prelude không chạy được — trusted intrinsic, ngoại trừ Arena được cài sẵn); chỉ số mảng chính xác theo giá trị cụ thể.

### 7. Bổ sung sau đột biến và săn soundness (ghi để review)
- **Lỗ hổng #1 (E0401):** ghi qua con trỏ/mảng đã move không bị báo (`check_uses` chỉ kiểm đọc). Sửa + `soundness.rs` + 3 mẫu corpus + hành động bộ sinh; tắt bản sửa thì oracle tự bắt 14/5 000.
- **Lỗ hổng #2 (E0404/E0403):** DỰNG một ADT bất biến (`Sl<'a> { s: &!'a &'a i32 }`) từ `&!p` chỉ ràng buộc hiệp biến ⇒ thu hẹp được region trong của `p` ⇒ ghi một tham chiếu ngắn qua ADT làm `p` treo, borrowck chấp nhận. Sửa: `Aggregate(Adt)` của ADT bất biến ràng buộc hai chiều với bó region của ADT. **Chi phí độ chính xác (sound, chặt hơn):** khoản vay `&!p` bị kéo dài bằng vòng đời của vùng tham chiếu bên trong (`use_i(*p)` sau khi dựng `Sl` là E0403 — Rust cũng từ chối `&'a mut &'a i32`). Do đột biến `adt_invariant` sống sót tìm ra; oracle không bắt được lúc đó vì bộ sinh chưa có ADT như vậy (bài học: oracle chỉ phủ cấu trúc bộ sinh sinh ra).
- **Bộ đếm phòng thủ (và fail-closed, M6-close):** mỗi chốt chặn phòng thủ trong `call_sig`/`walk`/`relate`/`setup_universals` khi bị chạm nay ghi ICE (`Gen::defend` ⇒ `check_body` trả E9901, nguyên tắc P3) — KHÔNG còn âm thầm rơi về đường bảo thủ hay bỏ qua quan hệ. `gt_borrowck::defensive_counters()` đếm số lần chạm; mọi test (corpus xấu/đúng, 20 000 chương trình ngẫu nhiên) khẳng định bằng (0, 0). **Bằng chứng kiểm thử, không phải chứng minh** rằng các chốt không với tới được bằng chương trình qua M4.
- **Luật bộ sinh (grit-contract §3.3, M6-close):** lỗ hổng #2 giờ có kịch bản riêng `adt_invariant` (loại thứ 10 của bảng độ phủ, ngưỡng ≥ 300 mỗi cột); tắt bản sửa ⇒ bộ sinh tự phát hiện 151 âm tính giả/5 000 chương trình. Dựng ADT dùng `ty_invariant` của kiểu ĐÃ THẾ (vd `Id1<Sl<'a>>` bất biến dù `Id1` hiệp biến).


## ADR-025 — M7: Obligation IR và bộ giải P1 (DỰ THẢO viết TRƯỚC khi code, 2026-10-07; CHỜ GIANG DUYỆT — chưa có dòng mã M7 nào)
**Phạm vi:** crate mới `gt-bounds` (đề xuất tên) đọc MIR của M5 (`Assert`, `Body.contracts`, literal) + dataflow; sinh **Obligation**, chạy bộ giải P1, xoá check chứng minh được, giữ check còn lại kèm ghi chú N06xx/N09xx hoặc báo ĐỎ E06xx/E09xx. Đây là **giao diện ổn định** sang Phase 2 (điều kiện 7 của cổng chuyển giai đoạn, GRIT_PLAN §7). Mọi đoạn "Đề xuất" bên dưới là ý kiến của tôi, KHÔNG phải quyết định.

### 0. Những chỗ nói thẳng trước khi duyệt (plan sai/thiếu/mâu thuẫn)
1. **Ví dụ `sum` ở GRIT_PLAN §4.3 (`xs[i]` được chứng minh từ `i < xs.len()`) KHÔNG làm được bằng miền khoảng thuần**, đúng như lệnh nhận xét: `xs.len()` không phải hằng, nên từ `i ∈ [0, MAX−1]` và `len ∈ [0, MAX]` không suy ra `i < len`. Cần **quan hệ giữa hai biến**. Đề xuất §3: miền khoảng + **zone** (ràng buộc hiệu `x − y ≤ c`). Với zone, ví dụ làm được đúng như plan viết (xoá check `xs[i]`; `i + 1` cũng xoá được vì `i < len ≤ MAX`; `s + xs[i]` còn check N0601). Hệ quả: plan §6.2 M7 ghi "miền khoảng" cần sửa thành "miền khoảng + zone (ràng buộc hiệu)"; ví dụ §4.3 giữ nguyên. (Cần Giang duyệt vì plan là nguồn sự thật.)
2. **"shift" và "unwrap trên None" của plan M7 chưa có cú pháp/lõi để sinh Obligation:** `docs/spec/symbols.md` (Q9) bỏ toán tử bit/dịch bit, chỉ có hàm `shl shr band …` trong `lib/core` (M12); `unwrap` cũng là hàm của `lib/core`. Hai nghĩa vụ này vì vậy là **tiền điều kiện `⊳` của hàm thư viện** (`⊳ n < bits`, `⊳ self là Some`) và đi qua cơ chế hợp đồng (§7). M7 cài cơ chế và test bằng hàm người dùng có `⊳ n < 32`; nối vào `shl/shr/unwrap` thật xảy ra ở M12. Plan M7 không sai nhưng dễ bị hiểu là M7 tự sinh chúng.
3. **Mâu thuẫn nhỏ semantics §8 vs D25:** §8 nói `⊲` "kiểm lúc chạy ở `dev`"; D25/fail-closed đòi nghĩa vụ chưa chứng minh mà kiểm được phải GIỮ ở mọi hồ sơ (nếu release bỏ thì lời hứa chưa chứng minh được tin cậy). Đề xuất: giữ ở mọi hồ sơ; hồ sơ chỉ đổi sanitizer. Cần xác nhận (Q36).
4. **Chưa có định nghĩa chính thức** của làm tròn `/` `%` trên số nguyên có dấu (thông dịch M5 dùng chia cắt về 0, dấu của `%` theo số bị chia). Đề xuất ghi vào semantics §2: cắt về 0, `%` cùng dấu số bị chia, `MIN / -1` và `MIN % -1` là tràn (Obligation `x ≠ MIN ∨ y ≠ −1`). Cần xác nhận (Q37).
5. **Không dùng tiên đề thêm về độ dài:** chỉ `len ≤ usize::MAX` (hiển nhiên theo kiểu). Không giả định `len ≤ isize::MAX` dù Rust có: đó là tiên đề về runtime phải nằm trong TCB; chưa cần cho `sum` (xem 1).
6. **Cổng cứng mục 7 điểm 9:** E0601 (literal ngoài phạm vi kiểu) là việc của M7 và chặn M9/`gritc build`; xem §6.

### 1. Cấu trúc Obligation (a)
```
Obligation {
  id:       ObId,          // ổn định: (khoá thân hàm, thứ tự sinh trong thân theo RPO + chỉ số trong câu lệnh); không phụ thuộc băm
  kind:     ObKind,        // Overflow{op,ty} | NegMin{ty} | DivZero | RemZero | DivOverflow{ty} (MIN/−1)
                           // | Index{len_atom} | SliceRange | CastLossy{from,to} | LiteralRange{ty}
                           // | Pre{callee} | Post | Invariant | QuantBounded | Custom(lib) ; mở rộng được, `#[non_exhaustive]`
  span, node, body,        // vị trí nguồn + NodeId + khoá thân (chẩn đoán, `--json`)
  at:       Location,      // điểm MIR (khối, chỉ số) — để Phase 2 đối chiếu đúng chỗ
  goal:     Prop,          // công thức cần chứng minh (xem dưới)
  hyps:     Vec<Prop>,     // GIẢ THIẾT tại điểm đó (điều kiện nhánh/vòng lặp, ⊳ của hàm này, bất biến suy ra), cùng ngôn ngữ Prop
  runtime:  Checkability,  // Checkable{via: AssertKind} | NotCheckable{why} (D25)
  status:   Status,        // Open | Discharged{by: Prover, cert: Cert} | DefinitelyFalse{witness} | Lowered{note: NCode}
}
Term = Int(ty, I256) | Var(VarId) | Len(Term)  // `len` của slice/mảng bất biến là nguyên tử thuần
     | Add|Sub|Mul|Div|Rem|Neg(Term..) | Cast(ty, Term) | Ite(Prop, Term, Term)
Prop = Le|Lt|Eq|Ne(Term, Term) | And|Or|Not|Implies | ForallB(var, lo, hi, Prop) | ExistsB(..)  // lượng từ CÓ BIÊN
```
`Var` là biến SSA-hoá theo điểm MIR (không phải `Local` thô) để `hyps` và `goal` cùng nghĩa; công thức KHÔNG chứa đối tượng của miền trừu tượng nên **bất kỳ bộ giải nào cũng đọc được** (SMT-LIB hoá trực tiếp: `Term` → bit-vector/integer, `Prop` → boolean). Đề xuất: số nguyên toán học (không modular) trong `Prop`; phép quấn vòng `+% -% *%` biểu diễn bằng `Cast`/`Rem` tường minh, không có ký hiệu riêng.
**Ranh giới Phase 1 / Phase 2:** Phase 1 sở hữu (i) sinh Obligation từ MIR, (ii) trích `hyps` từ dataflow, (iii) `trait Prover { fn prove(&mut self, ob: &Obligation, budget: Budget) -> Verdict }` với `Verdict = Proved(Cert) | Refuted(Witness) | Unknown`, (iv) bộ giải P1 là MỘT cài đặt của `Prover`, (v) chính sách hạ/xoá/báo. Phase 2 chỉ thay/chồng `Prover` (SMT, kernel chứng minh) và thêm giả thiết (bất biến vòng lặp `⋈`, hợp đồng), KHÔNG đổi `Obligation`/`Verdict`/chính sách. `Cert` ở Phase 1 là bản ghi nhỏ ("khoảng của x là [a,b] tại điểm P", "zone: x−y ≤ c") để một bộ kiểm độc lập kiểm lại; ở Phase 2 là chứng cứ SMT/kernel. `budget` đo bằng BƯỚC (D20), không phải giây.
**Đánh đổi:** công thức trung lập (+) chặn phụ thuộc vào miền, dễ nối SMT; (−) lặp lại một phần thông tin mà miền đã có, và phải dịch miền → `hyps` (làm một lần, kiểm bằng test "hyps ⊢ goal" ngược lại với bộ giải). Phương án thay thế "chuyển trực tiếp trạng thái miền sang Phase 2" gắn chặt giao diện vào zone — bị loại.

### 2. Mức mạnh của bộ giải P1 (b) — sẽ và KHÔNG chứng minh
**Đề xuất:** mỗi biến nguyên có (i) khoảng `[lo, hi]` bằng `I256` (§5), (ii) tập **zone** `x − y ≤ c` giữa biến nguyên và nguyên tử `len(·)`, đóng bao Floyd–Warshall (zone/DBM, Miné 2001), (iii) sự kiện nhánh/vòng lặp được cộng vào cả hai; (iv) `⊳` của hàm hiện tại là giả thiết.
**Sẽ chứng minh:** các Obligation `x < len`, `x + c < len`, `x ≤ y + c` khi suy ra được từ điều kiện nhánh/vòng lặp bằng bắc cầu và cộng hằng (ví dụ `sum`; hai biến chỉ số `i < j < len`; `n == len ∧ n > 0 ⇒ n−1 < len`); tràn của `i + 1` khi `i < len ≤ MAX`; phép toán trên hằng và khoảng nhỏ; `x − y` không âm khi `y ≤ x`; chỉ số của vòng đếm tới hằng `0..N` với mảng `[T; N]`.
**KHÔNG chứng minh (⇒ check còn lại + N06xx):** phụ thuộc dữ liệu (`s + xs[i]`); phi tuyến (`i*j < n`, `i*i`); quan hệ qua bộ nhớ/tham chiếu thay đổi được (`*p` đổi giữa hai điểm: nguyên tử coi là không biết); chỉ số tính từ nhiều biến ngoài zone (`xs[2*i + 1]`); đa thức/modulo (`i % 2`) ngoài khoảng; mọi thứ phụ thuộc bất biến vòng lặp phải ĐOÁN (cần SMT/`⋈`, 8.A). Với mỗi loại nghĩa vụ ADR liệt kê ví dụ "xoá được/không xoá được" và chúng trở thành test hai chiều (§9).
**Đánh đổi:** chỉ khoảng: rẻ, nhưng không làm được ví dụ chủ đạo của plan. Khoảng + sự kiện nhánh khớp cú pháp (so `goal` với điều kiện): làm được `sum` nhưng dễ vỡ (`i < len` viết `len > i`, hay qua biến phụ). Zone: bắc cầu chuẩn, độ phức tạp `O(n³)` mỗi đóng bao (n = số biến trong hàm; chặn bằng ngân sách bước, quá ngân sách ⇒ bỏ hết ràng buộc quan hệ, sound). Octagon/đa diện: mạnh hơn, đắt hơn, để Phase 2. **Đề xuất: zone.**

### 3. Phân tích vòng lặp: dừng và sound (c)
**Đề xuất:** lặp điểm cố định theo RPO; **widening** tại đầu vòng lặp (đích của cạnh lùi): khoảng nhảy tới ngưỡng tiếp theo trong tập {0, ±1, hằng xuất hiện trong hàm, MIN/MAX của kiểu} rồi ±∞∩kiểu; zone dùng widening chuẩn (bỏ ràng buộc không ổn định); sau đó **hai lượt narrowing** dùng lại điều kiện nhánh. **Dừng:** dàn có chiều cao hữu hạn sau widening (mọi cận lấy từ tập ngưỡng hữu hạn + kiểu; ràng buộc zone dùng hằng từ tập hữu hạn), cộng thêm **ngân sách bước** (D20) — hết ngân sách ⇒ trạng thái của hàm = ⊤ cho mọi Obligation chưa chứng minh (sound, nhiều check hơn).
**Sound:** (1) widening ⊒ join nên kết quả là hậu-điểm-cố-định; (2) narrowing chỉ áp dụng khi kết quả vẫn ⊒ lfp: mỗi bước `S' = F(S) ⊓ S`, giữ `F(S') ⊑ S'`; (3) **bộ kiểm hậu-điểm-cố-định độc lập** (cùng tinh thần validator của M5): sau khi chốt, kiểm lại `F(S_b) ⊑ S_b` tại MỌI khối bằng một hàm viết riêng, không dùng đường widening; kiểm hỏng ⇒ coi cả hàm là ⊤ và báo ICE E99xx (không âm thầm tin kết quả). Chứng minh bằng test: §9.
**Đánh đổi:** chỉ widening (nhanh, rẻ, mất chính xác ở vòng đếm); thêm narrowing + ngưỡng (đủ cho `0..n`, đếm lên/xuống); unrolling/phân tích đệ quy (để Phase 2). Đề xuất như trên; số lượt narrowing = 2 là ngưỡng thực nghiệm sẽ đo.

### 4. Ngữ nghĩa chính xác của số học trên miền (d)
**Kiểu:** `i8 i16 i32 i64 i128 isize`, `u8 u16 u32 u64 u128 usize`; `isize/usize` = 64 bit (x86-64, spec §1). Cận khoảng là **`I256`** (số nguyên có dấu 256 bit, cài trong crate, KHÔNG phụ thuộc ngoài): đủ chứa chính xác tích hai giá trị 128 bit để không phải tràn khi tính cận (đề xuất; thay thế `i128` + "+∞" cho `u128` gây mất chính xác ở `u128::MAX` và dễ sai ở biên — loại).
- `+ - *` (kiểm tràn, D2): kết quả toán học `[a,b]`; Obligation `a ≥ MIN ∧ b ≤ MAX` (đúng cho MỌI giá trị trong khoảng). **Sau** phép (nếu check giữ hoặc đã chứng minh), trạng thái tiếp tục với `[a,b] ∩ [MIN,MAX]` (đường tràn đã abort). `*` dùng 4 tích góc (có dấu) hoặc 2 góc (không dấu).
- `/ %`: Obligation `y ≠ 0`; có dấu thêm `¬(x = MIN ∧ y = −1)`. Cắt về 0; `x / y` trên khoảng: tách khoảng của y theo dấu, bỏ 0; `%` có `|kết quả| < |y|` và dấu theo `x`.
- `-x` có dấu: Obligation `x ≠ MIN`; kết quả `[−hi, −lo]`.
- `+% -% *%` (quấn vòng, không Obligation): nếu cả khoảng kết quả toán học nằm trong MỘT chu kỳ `2^bits` ⇒ dịch về đúng khoảng modulo; ngược lại ⇒ toàn kiểu `[MIN, MAX]`.
- `+^ -^ *^` (bão hoà): kẹp cận vào `[MIN,MAX]` chính xác (hàm đơn điệu theo từng cận).
- `+? -? *?` (trả `Option<T>`): miền cặp `{may_none, may_some, khoảng của Some}`: `may_none ⇔ khoảng toán học vượt kiểu`, `may_some ⇔ giao khác rỗng`; khoảng của `Some` = giao với kiểu.
- Dịch bit/AND/OR/XOR: hàm `lib/core` + `⊳ n < bits` (xem §0.2); kết quả khoảng bảo thủ (`shl`: nhân `2^n` nếu không tràn bit, nếu không toàn kiểu; `band` với hằng không âm: `[0, hằng]`).
- Ép kiểu `as` giữa nguyên: Obligation `giá trị ∈ [MIN',MAX']` của kiểu đích (D1); kết quả giao với khoảng đích. `bool→nguyên`: `[0,1]`. `char→nguyên`: `[0, 0x10FFFF] ∖ surrogate`, bảo thủ khoảng liền.
- So sánh nguyên `< <= > >= == ≠`: dùng làm sự kiện nhánh trên cả hai khoảng và zone (cả hai nhánh, nhánh sai dùng phủ định). **Số thực** (`f32/f64`): KHÔNG theo dõi (miền ⊤ cho giá trị); so sánh số thực (`<`, … và hàm `total_cmp`) cho `bool` không biết ⇒ không sinh sự kiện nhánh (sound; NaN làm mọi so sánh trừ `≠` sai); `==` đã bị cấm (D4).
- Phép chia/số học không sinh Obligation cho số thực (semantics §2); ép số thực→nguyên là một Obligation mất thông tin (bão hoà/NaN) — **chưa định nghĩa trong semantics**, đề xuất cấm `as` thực→nguyên ở Phase 1 (dùng hàm `lib/core` có `⊳`), cần Giang xác nhận (Q38).

### 5. Hạ xuống runtime check (e)
- **Có sẵn từ M5:** mỗi nghĩa vụ số nguyên đã là một terminator `Assert{kind}` (Overflow, OverflowNeg, DivisionByZero, RemainderByZero, IndexOutOfBounds, SliceRange, CastLossy). M7 KHÔNG tạo check mới: nó **xoá** `Assert` chứng minh được (thay bằng `Goto`, ghi `Discharged` + chứng cứ) hoặc **giữ** và ghi `Lowered`.
- **Chính sách một nghĩa vụ:** `Discharged` (xoá) | `DefinitelyFalse` ⇒ ĐỎ | `Open` & `Checkable` ⇒ giữ `Assert` + GHI CHÚ N06xx/N09xx (D24) | `Open` & `NotCheckable` ⇒ ĐỎ E0902 (D25). Trong M7 chỉ có hai nguồn "không kiểm được": lượng từ không biên (§7) và mệnh đề tham chiếu bộ nhớ không thuần.
- **"Chắc chắn sai" (E060x) nghĩa là:** nghĩa vụ sai trên MỌI thực thi tới điểm đó **và điểm đó tới được** (trạng thái ≠ ⊥). Mã chết (⊥) không báo gì. Phân loại: E0601 tràn chắc chắn (cả `+ - *`, `-MIN`, ngoài kiểu), E0602 chia/% 0, E0603 chỉ số/lát ngoài biên, E0604 ép kiểu mất thông tin, E0901 tiền điều kiện chắc chắn sai. (Đề xuất giữ đúng bảng hiện có của `error_codes.md`.)
- **E0601 cho literal ngoài phạm vi kiểu (`x: u8 ≔ 300`) — cổng cứng mục 7.9:** khác các nghĩa vụ trên, nó là nghĩa vụ HẰNG của chính literal (`LiteralRange{ty}`), kiểm trước dataflow, **không phụ thuộc đường tới được** (dù trong mã chết vẫn ĐỎ — vì chương trình đã sai kiểu). Điểm gắn: sau M4 suy luận kiểu cho literal; M7 duyệt mọi literal đã có kiểu cuối (đã có `Const::Int{bits,ty}` trong MIR nhưng M5 cắt bit theo kiểu và làm MẤT thông tin ngoài phạm vi — **M7 phải đọc literal từ AST/`TypesOutput`, không từ `Const` của MIR**; đây là một sửa nhỏ ở M5 cần xác nhận, Q39).
- **Mã GHI CHÚ đề xuất (cần duyệt vì sửa bảng mã):** N0601 tràn số (gồm `-MIN`), N0602 chỉ số/lát, N0603 chia/% 0 (và `MIN/−1`), N0604 ép kiểu, N0901 `⊳` hạ runtime (đã có ở semantics §8), N0902 `⊲`, N0903 lượng từ có biên chạy lúc chạy, N0904 bất biến `⋈`. (Bảng hiện chỉ có N0601, N0602.)
- Hồ sơ: check giữ ở MỌI hồ sơ (§0.3). Chi phí hiển thị bằng N-mã theo D24.

### 6. Hợp đồng `⊳` `⊲` ở Phase 1 (f)
- **`⊳` trong thân hàm đó:** giả thiết (`hyps`) cho mọi Obligation trong thân. **`⊳` của callee tại điểm gọi:** Obligation `Pre{callee}` với `goal = ⊳[đối số]`: giải bằng P1 → xoá; không giải được và biểu thức thuần/kiểm được → hạ runtime tại điểm gọi + N0901; chắc chắn sai → E0901. **`⊲`:** kiểm runtime ở MỌI điểm thoát hàm (`⏎`, cuối thân) trừ khi P1 chứng minh trên mọi đường thoát (khi đó xoá + chứng cứ); tại điểm gọi, `⊲` của callee là giả thiết cho đoạn sau (sound vì `⊲` của callee hoặc đã chứng minh hoặc được kiểm lúc chạy — vi phạm thì abort trước khi tới đoạn sau).
- **Lượng từ có biên** `∀ i ∈ a..b : P`, `∃`: thực thi như vòng lặp thuần có chặn trên (`b − a` bước) khi hạ runtime, hoặc P1 giải trường hợp tầm thường (miền rỗng, `P` đúng với mọi `i` trong khoảng kiểm bằng khoảng). **Không biên** mà chưa giải ⇒ ĐỎ E0902 (D25).
- **`⋈` bất biến vòng lặp:** Phase 1 chỉ kiểm lúc chạy (semantics §3, không dùng để chứng minh); Obligation `Invariant` mang cờ `Checkable`.
- **Yêu cầu hạ tầng MIR:** `Body.contracts` hiện chỉ ghi NodeId (M5, Q29): M7 cần hạ `⊳ ⊲ ⋈` thành các biểu thức MIR thuần ("thân hợp đồng") gọi được ở điểm gọi/thoát; bước đầu tiên của M7 là mở rộng bước hạ MIR (có golden). Hợp đồng chứa hàm có hiệu ứng ⇒ ĐỎ (hợp đồng phải thuần, `! ∅`).
- **Đánh đổi:** hạ `⊲` thành runtime ở thoát hàm làm chậm hàm nóng nếu không chứng minh được (hiển thị N0902); phương án "chỉ kiểm ở dev" nhẹ hơn nhưng mâu thuẫn fail-closed — đề xuất giữ kiểm.

### 7. Biến thể `↓` (E0701) thuộc M7 hay M8 (g)
**Xác nhận M8** (đúng như plan): phân tích dừng cần đọc khoảng/zone của biến lặp để kiểm "giảm nghiêm ngặt" và dạng mẫu `i ← i + c`. **Ranh giới:** M7 chỉ (i) **ghi nhận** biểu thức `↓` và `⋈` của mỗi vòng lặp/hàm và công bố một API `DomainQuery` (khoảng/zone của một biểu thức tại một điểm MIR) để M8 dùng; (ii) KHÔNG sinh E07xx; (iii) không dùng `↓` làm giả thiết cho Obligation số học (nó không nói gì về giá trị). Không đề xuất đổi plan. Rủi ro: M8 phải dựa vào độ chính xác M7; mitigation: test `DomainQuery` cho các mẫu `0..n`, đếm lên/xuống.

### 8. Chiến lược test soundness (h) — đề xuất
1. **Soundness của hàm chuyển trên miền (đơn vị, ngẫu nhiên + biên):** mọi phép toán × mọi kiểu × khoảng ngẫu nhiên/cận biên: lấy các giá trị cụ thể trong khoảng nguồn, tính bằng **bộ đánh giá tham chiếu độc lập** (số học chính xác riêng của test: cặp `i128` + dấu, `u128` với `checked_*`/mở rộng; KHÔNG dùng `I256` của bộ giải), kiểm kết quả cụ thể ∈ khoảng kết quả và Obligation sai ⇒ có giá trị cụ thể vi phạm. Với zone: tập ràng buộc ngẫu nhiên + gán thoả ⇒ đóng bao/join/widening/narrowing vẫn thoả. Có test CHIỀU NGƯỢC (precision): các khoảng hẹp phải chứng minh được.
2. **Mức chương trình (oracle độc lập, như M6):** thông dịch MIR của M5 làm oracle; bộ sinh chương trình (mở rộng bộ sinh M5: mảng/lát, vòng lặp có cận, so sánh, hợp đồng) cho chương trình có `Assert`; với MỌI `Assert` mà bộ giải XOÁ, chạy thân GỐC (còn đủ `Assert`) trên đầu vào ngẫu nhiên + biên (MIN, MAX, ±1, 0, độ dài 0/1/…): `Assert` đó KHÔNG BAO GIỜ được thất bại (đây là âm tính giả). Ca ngược: Assert chắc chắn sai (E060x) phải thất bại ở mọi đầu vào tới được; Assert giữ lại không có ràng buộc.
3. **Độ phủ bắt buộc** (cùng luật M6, ghi vào ADR khi duyệt): bảng loại phép toán (`+ - * / % -x +% +^ +? as`) × loại nghĩa vụ × độ rộng kiểu (8/16/32/64/128, có/không dấu) × kết cục (xoá / giữ / chắc chắn sai); ngưỡng đề xuất ≥ 300 chương trình ở mỗi ô **mà bộ giải XOÁ** (nơi âm tính giả lộ ra) và ≥ 300 ở ô "giữ"; ô dưới ngưỡng thì CHƯA được tính là đã kiểm. Quy tắc mới (grit-contract §3.3): mỗi lỗ hổng tìm bằng cách khác với bộ sinh phải mở rộng bộ sinh rồi tắt bản sửa để xem tự phát hiện.
4. **Đột biến bộ giải:** ĐẦY ĐỦ cho M7 (chính sách đã cho phép, `tools/mut.py`), mọi đột biến sống sót có test hoặc lập luận tương đương cụ thể; bộ đếm phòng thủ fail-closed (ICE) như M6.
5. **Bộ kiểm hậu-điểm-cố-định** (§3) và chứng cứ (`Cert`) có bộ kiểm riêng: test đột biến ép bộ giải trả chứng cứ sai phải bị bộ kiểm bắt.
6. **Fuzz:** `Prover` với khoảng/zone/công thức ngẫu nhiên: không panic, không treo (ngân sách), kết quả tất định.

### 9. Đo "số check còn lại" (i)
Trên mỗi chương trình mẫu (`tests/pass`, golden riêng cho M7): bảng theo hàm và theo loại — `tổng nghĩa vụ`, `xoá được`, `giữ lại (N-mã)`, `ĐỎ`; số `Assert` trong MIR trước (M5) và sau (M7). Nguồn: `Obligation.status` + so sánh số terminator `Assert`. Khoá bằng golden `tests/golden/obligations/*.txt` (đọc tay một lần như M5) và test cả hai chiều: `sum` ⇒ `xs[i]` và `i + 1` xoá, `s + xs[i]` còn N0601; một họ chương trình nhỏ (đếm tới hằng, hai chỉ số, chia cho biến đã `≠ 0`, ép kiểu có khoảng hẹp, hợp đồng) với số kỳ vọng cố định. Chỉ số kết quả báo trong PROGRESS: tỉ lệ nghĩa vụ xoá được trên corpus mẫu và số check còn lại trên 3 chương trình thật của M12.

### 10. Thứ tự làm (nếu duyệt)
(1) `I256` + test tham chiếu; (2) miền khoảng + hàm chuyển + test soundness; (3) zone + đóng bao + widening/narrowing; (4) Obligation IR + `Prover` + hyps; (5) sinh Obligation từ `Assert` của MIR, xoá/giữ, N-mã, E060x (kể cả LiteralRange); (6) hạ hợp đồng xuống MIR + `Pre/Post`; (7) oracle động + bộ sinh + độ phủ; (8) đột biến đầy đủ; (9) golden "số check còn lại". Dừng chờ duyệt sau (3), (6), (9).

### 11. Câu hỏi cần Giang quyết (đề xuất kèm lý do ở trên)
- **Q35** Miền P1 = khoảng + **zone** (không chỉ khoảng); sửa plan §6.2 M7 (đề xuất: ĐỒNG Ý, vì ví dụ §4.3 đòi hỏi).
- **Q36** `⊲`/mọi check chưa chứng minh được giữ ở MỌI hồ sơ (đề xuất: giữ; sửa semantics §8 "kiểm ở dev").
- **Q37** Định nghĩa `/ %` có dấu (cắt về 0; `MIN/−1` là tràn) viết vào semantics §2.
- **Q38** Cấm `as` số thực→nguyên ở Phase 1 (hay Obligation riêng).
- **Q39** E0601 literal đọc từ AST/`TypesOutput`, không từ `Const` của MIR (cần một sửa nhỏ ở M5 để không cắt bit mất thông tin, hoặc hạ literal ngoài phạm vi thành ICE ở M5).
- **Q40** Thêm mã GHI CHÚ N0603, N0604, N0902–N0904 (bảng mã) và xác nhận E0601–E0604, E0901, E0902 giữ nghĩa như `error_codes.md`.
- **Q41** `I256` tự cài trong crate (đề xuất) hay phụ thuộc crate ngoài (cần `cargo audit/deny`).
- **Q42** Ngưỡng độ phủ ≥ 300/ô và 2 lượt narrowing (số tôi chọn, không suy ra từ lỗi đã biết).

### 12. Duyệt của Giang (2026-10-07) và bổ sung — mục này THẮNG các mục 0–11 nếu khác
Giang chỉ có bản tóm tắt ADR-025; tôi đối chiếu từng điểm với ADR: **không có điểm nào mâu thuẫn**; ba chỗ là mở rộng hoặc tinh chỉnh (ghi rõ để review):
- **Tên crate:** `gt-oblig` (lệnh của Giang), thay `gt-bounds` (tên "đề xuất" trong §intro). Giao diện bộ giải là một trait (`Prover`, §1) — khớp.
- **Bộ giải thứ hai (BẮT BUỘC ở Phase 1, mục 10 của plan "mock SMT"):** ADR §8 mới chỉ nêu "bộ đánh giá tham chiếu độc lập" trong test; nay nâng thành CÀI ĐẶT THỨ HAI của `Prover` ngay trong `gt-oblig`: bộ giải **liệt kê vét cạn** trên độ rộng bit nhỏ, dùng làm oracle và chứng minh giao diện cắm được. Miền được tham số hoá theo độ rộng bit; vét cạn ở 2..7 bit (mọi cặp toán hạng, mọi miền con, qua bảng cực trị 2-D nên mỗi cặp hộp kiểm O(1)); 8 bit phân tầng (mọi hộp có một đầu mút ở tập {MIN, MIN+1, −1, 0, 1, MAX−1, MAX} + ngẫu nhiên có hạt giống) vì vét cạn mọi cặp hộp 8 bit tốn ~10⁹ ô/phép; tin công thức cho 16..128. **Lệch nhỏ so với lệnh ("2..8 vét cạn"):** 8 bit KHÔNG vét cạn hoàn toàn mọi cặp hộp; nói thẳng để review. Hai bộ giải bất đồng ⇒ bug ở một trong hai; bộ giải chính không được "chứng minh" nghĩa vụ mà vét cạn tìm ra phản ví dụ.
- **Zone (§2) — ba điều kiện của Giang:** (a) **quấn vòng/bão hoà:** `+% -% *%`, `+^ -^ *^` giữ quan hệ zone của đích CHỈ khi phép toán được chứng minh không tràn (khi đó bằng phép toán toán học); ngược lại quan hệ của đích bị huỷ (chỉ còn khoảng). (b) **ghi qua bí danh:** quan hệ liên quan một chỗ bị ghi — trực tiếp, qua `&!`, hoặc qua lời gọi nhận `&!` — bị huỷ; cài đặt bảo thủ: **mọi local có khoản mượn `&!` ở bất kỳ đâu trong thân (bảng `Body.borrows` của M5, `BorrowKind::Unique`) KHÔNG được theo dõi** (luôn ⊤, không có biến zone); nguồn thông tin alias = bảng khoản mượn của MIR (M5) cộng bảo đảm của M6 rằng chỉ có một `&!` sống và `&` không ghi được; không dùng region của M6. Trường/đích qua `*p` không theo dõi. (c) **Cận chi phí:** hằng có tên `ZONE_MAX_VARS = 24` biến mỗi hàm cho zone; vượt thì giữ 24 biến đầu theo thứ tự chỉ số `Local` tăng dần (tất định) và phát **GHI CHÚ N0605** liệt kê biến bị bỏ — KHÔNG bỏ qua trong im lặng. Hết ngân sách bước (D20) ⇒ bỏ MỌI quan hệ zone của hàm (còn khoảng) + N0605. Kết quả tất định (D9, D20).
- **Số học của chính bộ phân tích:** `I256` có kiểm tràn cho mọi phép: tràn ⇒ cận nhảy lên đỉnh (top), không bao giờ quấn vòng. **Biểu diễn cận u128:** `I256` (cận lấy chính xác `u128::MAX`); hằng zone dùng `i128` với "không ràng buộc" khi vượt ±2¹²⁶ (không quấn vòng).
- **Obligation IR (§1):** thêm yêu cầu **tuần tự hoá** (JSON, khoá cố định và thứ tự xác định), `ObId` tất định, giới hạn tài nguyên theo BƯỚC (D20); báo cáo "số check còn lại" xuất JSON từ đầu (schema có version).
- **shift/unwrap (§0.2):** theo lệnh, prelude fixture khai báo `shl`, `shr`, `unwrap` kèm `⊳` ngay ở M7 và test pass/fail dùng chúng (sửa lại phần "test bằng hàm người dùng" của ADR). `unwrap` dùng vị từ thuần `is_some` như NGUYÊN TỬ không diễn giải (giả thiết nhánh `is_some(o)` khớp mục tiêu `Pre(unwrap)` khi `o` không đổi).
- **Câu hỏi:** Q35 (zone), Q36 (mọi hồ sơ), Q37–Q42 theo ADR đã duyệt; Q40: thêm N0605 cho cận zone.

### 13. Ghi nhận triển khai M7 (2026-10-08) — số thật, độ lệch so với §0–§12, ngưỡng đã đặt
**Trạng thái: M7 "ĐẠT CÓ ĐIỀU KIỆN"** (như M6): còn 69 đột biến theo hướng nguy hiểm chưa giải quyết (KNOWN_LIMITATIONS, mục gt-oblig), xem điều kiện cứng mới ở GRIT_PLAN §7 điểm 11.

**Cái đã làm đúng như ADR:** IR nghĩa vụ tuần tự hoá JSON (schema 1, id tất định, cờ `runtime`), trait `Prover` với hai cài đặt (`ZoneProver` = khoảng + zone; `ExhaustiveProver` = vét cạn độc lập), miền tham số theo độ rộng, widening có ngưỡng + hai lượt narrowing + kiểm hậu-điểm-cố-định, `ZONE_MAX_VARS = 24` (N0605), trần bước (D20), chính sách xoá/giữ/ĐỎ/E0902, E0601 độc lập đường tới (M5 ghi `Body.bad_literals`), hợp đồng `⊳ ⊲` (thân hợp đồng MIR + trích mệnh đề), bất biến `⋈` (Assert ở đầu vòng), lượng từ có biên (vòng lặp thuần), `shl/shr/unwrap` trong prelude fixture, báo cáo JSON "check còn lại".

**Độ lệch / quyết định phát sinh (để Giang biết):**
1. **Hợp đồng ở MIR:** thêm `BodyKind::{Pre, Post}`, `AssertKind::{Precondition, Postcondition, Invariant}`, `Body.uncheckable`, `Body.bad_literals`. `⊳` chỉ là giả thiết của thân hàm khi hàm CHỈ bị gọi trực tiếp (không thoát qua `FnItem`, không thuộc `⊐ Trait for`); `⊲` được dùng làm giả thiết sau lời gọi trực tiếp; `⊲` nhắc tham số bị gán lại/mượn `&!` ⇒ E0902 (và KHÔNG được chứng minh bằng giá trị hiện tại).
2. **Vị từ thuần** (một đối số, trả `bool`, hiệu ứng `∅`, không `&!`) được mô hình như nguyên tử không diễn giải khoá theo local (cần cho `unwrap ⊳ self.is_some()`); ràng buộc bổ sung vào điều kiện cứng mutability bên trong.
3. **E0316 (mới, M4):** lượng từ ngoài hợp đồng là lỗi ĐỎ (lệnh Giang 2026-10-08); ICE ở M5 chỉ là chốt phòng thủ.
4. **`E0901`** dùng cho cả `⊳`, `⊲` và `⋈` chắc chắn sai. **Mã mới trong bảng:** N0603, N0604, N0605, N0902–N0904, E0316.
5. **Định nghĩa biến tạm (`defs`)** chỉ cho biến tạm, giá trị trả về và biến người dùng BẤT BIẾN; bị xoá ngay khi một local nó nhắc tới bị gán lại (bỏ cơ chế nhãn phiên bản — nhãn MERGED làm mất định nghĩa qua vòng lặp).
6. **Vét cạn:** 2..7 bit đầy đủ mọi cặp hộp, 8 bit phân tầng; 8 bit đầy đủ (`GT_FULL8=1`) CHƯA chạy (chạy một lần sau cổng cuối, không chặn commit — lệnh Giang). Thực đo: cộng/trừ/nhân **493 564 176** cặp hộp, chia/dư **329 042 784** cặp hộp, mỗi cặp kiểm mọi giá trị cụ thể; ngoài ra 154 016 nghĩa vụ ngẫu nhiên so bộ chính với bộ vét cạn (0 chứng minh sai, 0 'chắc chắn sai' sai).
7. **Điều kiện ghép `∧ ∨`** chưa tinh chỉnh khoảng (M5 hạ thành luồng điều khiển rồi hợp): câu hỏi mở, KHÔNG làm trong M7.

**Lỗ hổng soundness tìm thấy khi làm M7 (cả hai đều đã sửa và có test):**
- **#3** chuỗi bí danh của tham số lát đi qua tham số có gán ngầm lúc vào ⇒ `ys ≔ xs; xs ← zs; ⎇ i < ys.len() { xs[i] }` xoá nhầm check chỉ số. Tìm bằng `tests/decisions.rs` (KHÔNG phải bộ sinh); theo luật §3.3 bộ sinh đã được mở rộng (tham số lát `μxs, zs`, bí danh, gán lại, gọi mọi hàm phụ ở đầu `entry`) và kiểm "tắt bản sửa": oracle bắt được ở chương trình #141.
- **#4** `⊲` nhắc tham số bị gán lại được chứng minh bằng giá trị HIỆN TẠI (sai nghĩa "giá trị lúc vào"); sửa: goal là nguyên tử không giải được ⇒ E0902. Oracle không thể quan sát (chương trình bị E0902 từ chối) nên chỉ `decisions2.rs` giết được; ghi nhận ở đây theo luật §3.3.

**Ngưỡng độ phủ của oracle mức chương trình (đặt trong `tests/oracle.rs`, 1 500 chương trình × 24 đầu vào):** mỗi ô (loại.phép × {xoá, giữ}) ≥ 100 nghĩa vụ và ≥ 100 lần chạm, với loại.phép ∈ {overflow.Add/Sub/Mul/Div/Rem, neg_min, div_zero, rem_zero, index, cast_lossy, invariant}; `slice_range` ≥ 40; mỗi ô 'đỏ' ≥ 8 nghĩa vụ và ≥ 8 lần chạm; `pre`/`post` ≥ 100 (xoá|giữ) và ≥ 8 (đỏ); mỗi loại abort (precondition, postcondition, invariant, overflow, div_by_zero, rem_by_zero, index_out_of_bounds, cast_lossy, overflow_neg) ≥ 100; ≥ 30 000 lần so sánh. Điểm yếu đã biết: ~97% lần chạy abort sớm (độ sâu thực thi mỏng).

**Quy trình đột biến (lệnh Giang 2026-10-08):** 1 worker, opt-level 0 cho `gt-oblig` khi chạy bộ test nhỏ (`GRIT_MUT_NO_OPT=1`), `GRIT_MUT_LINES` để chạy lại theo dòng, kết quả ghi nối sau từng mẫu.

**Bổ sung §13 (2026-10-08) — ngưỡng "chạy tới cuối hàm":** oracle đo thêm, theo từng loại nghĩa vụ, SỐ CHECK BỊ XOÁ mà ít nhất một lần chạy kết thúc bình thường sau khi chạy qua nó (đoạn code sau check đã chạy tới cuối hàm). **Ngưỡng ≥ 100 mỗi loại**; lý do: một check bị xoá nhầm chỉ lộ ra khi đoạn code sau nó chạy (nếu lần chạy abort sớm ở chỗ khác thì xoá nhầm không quan sát được) và 100 là cỡ đủ để loại bỏ trường hợp một-hai chương trình may mắn, vẫn đạt được với bộ sinh hiện có. Loại nào dưới ngưỡng thì CHƯA được tính là đã kiểm (test đỏ). `FULL_PROGRAMS` nâng 1 500 → 2 000. Tỉ lệ lần chạy kết thúc bình thường: 8,4% → 29,9% sau khi thêm chế độ "nghiêm" của bộ sinh (số liệu PROGRESS).

## ADR-026 — M8: hiệu ứng, điều kiện dừng, đa luồng (DỰ THẢO viết TRƯỚC khi code, 2026-10-08; CHỜ GIANG DUYỆT — chưa có dòng mã M8 nào)

### 0. Điều kiện xuất phát (nói thẳng)
- **M7 chỉ "đạt có điều kiện"** (69 đột biến nguy hiểm của `analysis.rs`/`driver.rs` chưa giải quyết). M8 phụ thuộc vào độ chính xác VÀ độ đúng của `analysis.rs` (kiểm giảm nghiêm ngặt dùng khoảng/zone). Rủi ro: một chứng minh "giảm" sai ⇒ E07xx bị bỏ sót ⇒ vòng lặp không dừng lọt qua. Giảm thiểu: M8 KHÔNG dùng "xoá check" mà dùng truy vấn miền chỉ-đọc, và mọi kết luận "giảm" cũng phải qua bộ giải kép (vét cạn ở độ rộng nhỏ) như nghĩa vụ M7.
- **Món nợ của M7 với M8:** ADR-025 §7 hứa M7 công bố `DomainQuery` (khoảng/zone của một biểu thức tại một điểm MIR). **M7 CHƯA làm** (không có trong lệnh (i)–(x)). Đề xuất: bước 1 của M8 là thêm API này (đọc `states[bb]` đã có trong `analyze_body`; ~100 dòng) kèm test cho `0..n`, đếm lên/xuống.
- **Khoá cứng (GRIT_PLAN §7 điểm 9–10):** phần `Mutex`/atomics của M8 KHÔNG bắt đầu cho tới khi bốn đột biến 238/361/591/554 của `gt-borrowck/src/regions.rs` được mở lại (giết bằng chương trình tấn công `interior_mut`, hoặc lập luận tương đương mới) VÀ vị từ trên các kiểu có mutability bên trong bị loại khỏi `pure_preds` của M7. Mọi thứ khác của M8 (hiệu ứng, dừng, `spawn` thuần) KHÔNG bị chặn.

### 1. Phạm vi và thứ tự đề xuất
(a) `DomainQuery` + test; (b) **hiệu ứng** (E0801, E0802): suy luận trên MIR theo đồ thị gọi, so với khai báo `! {…}`; SCC cho đệ quy tương hỗ; (c) **dừng** (E0701–E0703): `↓` trên kiểu well-founded (số nguyên không âm/có cận dưới), kiểm "giảm nghiêm ngặt qua mỗi vòng" bằng `DomainQuery` + dạng mẫu `i ← i ± c` hướng về cận; `div` cho phân kỳ chủ ý; đệ quy tương hỗ theo SCC (mọi cạnh trong SCC phải giảm một biến thể chung); (d) **`spawn(hàm, đối_số)`** (chưa có closure): điều kiện kiểu `Send`-like tự suy, chia sẻ chỉ qua kiểu đồng bộ — phần không cần mutability bên trong; (e) **BỊ KHOÁ:** `Mutex<T, L>` + thứ tự khoá tăng nghiêm ngặt xuyên hàm (E0502, E0501) và atomics có ordering bắt buộc.

### 2. Thiết kế chính
- **Hiệu ứng:** tập hiệu ứng của thân = hợp hiệu ứng của mọi `Call`/`Callee` (+ hiệu ứng nguyên thuỷ của capability/`div`); hàm `∅` gọi hàm có hiệu ứng ⇒ E0802; suy ra ⊄ khai báo ⇒ E0801. `Callee::Value`/`TraitMethod` dùng hiệu ứng trong KIỂU hàm/trait (đã có ở M4) — không suy ngược. Đệ quy: điểm cố định trên SCC (tập hữu hạn ⇒ dừng, tất định D9/D20).
- **Dừng:** mỗi cạnh lùi của CFG phải mang biến thể `v` (từ `↓`) với `v` giảm nghiêm ngặt và bị chặn dưới trên MỌI đường quanh vòng: Obligation `Variant{decrease}` giải bằng bộ giải M7 (cùng `Prover`, cùng bộ vét cạn kiểm chứng). Không có `↓` ⇒ E0701; `↓` không giảm/không chặn ⇒ E0702; đệ quy thiếu `↓` ⇒ E0703. `⟳` vô hạn có `⊠` vẫn cần `↓` hoặc `div` (D12).
- **Đa luồng (phần mở được):** `spawn` nhận hàm tự do và đối số có thể gửi; một kiểu là `Send`-like nếu mọi trường là `Send`-like và không chứa tham chiếu `&!`/con trỏ thô/`Rc`-like; chia sẻ tham chiếu `&T` sang luồng khác chỉ khi `T` đồng bộ (`Sync`-like). Vi phạm ⇒ E0501.
- **Khoá (khi mở khoá cứng):** hạng khoá `L` (số nguyên cấp thời gian biên dịch) gắn vào `Mutex<T, L>`; hiệu ứng `lock(L)` truyền xuyên hàm; thứ tự khoá phải tăng nghiêm ngặt trên mọi đường ⇒ chặn deadlock theo thứ tự (E0502).

### 3. Chiến lược test (rút kinh nghiệm M6–M7)
1. **Corpus** (plan M8): data race / deadlock / vòng lặp không biến thể đều bị chặn đúng mã; ≥3 fail + ≥3 pass mỗi mã; chương trình đa luồng đúng chạy qua (trong phạm vi thông dịch).
2. **Oracle động:** bộ sinh vòng lặp/đệ quy ngẫu nhiên có/không `↓` hợp lệ; thông dịch MIR với nhiên liệu — chương trình được M8 chấp nhận KHÔNG ĐƯỢC hết nhiên liệu trên đầu vào thử (âm tính giả = lọt vòng lặp vô hạn); chương trình bị bác ĐỎ không cần chạy. Hiệu ứng: oracle so tập hiệu ứng thực thi (vết lời gọi) với tập suy ra (suy ra phải ⊇ thực thi).
3. **Luật bộ sinh (grit-contract §3.3):** mọi lỗ hổng tìm bằng cách khác ⇒ mở rộng bộ sinh + kiểm "tắt bản sửa". Bộ sinh M7 chỉ phủ ~3% lần chạy không abort — M8 phải cải thiện độ sâu thực thi.
4. **Đột biến:** đầy đủ cho M8 (chính sách), 1 worker, lô nhỏ, quy tắc môi trường 2026-10-08; hàm quyết định "giảm nghiêm ngặt" và "suy hiệu ứng" được nhóm theo hàm, có golden riêng.
5. **Khoá cứng test:** `interior_mut` kịch bản trong bộ sinh borrowck + M7 trước khi `Mutex`/atomics vào fixture.

### 4. Rủi ro / đánh đổi
- Độ chính xác của "giảm nghiêm ngặt" bị chặn bởi M7 (chưa tinh chỉnh `∧ ∨`, hằng khởi tạo không tạo quan hệ zone): nhiều vòng lặp hợp lệ sẽ bị E0702 nhầm — fail-closed nhưng gây khó chịu; bù bằng dạng mẫu `i ← i ± c`. Đề xuất (chờ Giang): sửa M5 hạ điều kiện `⎇ a ∧ b` thành nhánh lồng (ảnh hưởng golden M5, M6) — hỏi trước khi làm.
- Hiệu ứng suy ngược qua `Callee::Value` quá bảo thủ ⇒ phải khai báo; chấp nhận.
- Khoá thứ tự: toàn bộ chứng minh phụ thuộc "tóm tắt hiệu ứng `lock`" đúng ⇒ nhóm đột biến riêng, và vì bị khoá cứng nên là phần cuối.

### 5. Câu hỏi cho Giang (đề xuất kèm lý do ở trên)
- **Q43** Thêm `DomainQuery` như bước 1 của M8 (món nợ của ADR-025 §7). Đề xuất: có.
- **Q44** Thứ tự (a)→(b)→(c)→(d), (e) chặn bởi khoá cứng. Đề xuất: có; dừng chờ duyệt sau (a)+(b), (c), (d).
- **Q45** Biến thể `↓` bắt buộc kiểu số nguyên không âm hay cho tuple lexicographic? Đề xuất Phase 1: chỉ một số nguyên (D12), tuple để Phase 2.
- **Q46** Hạ `⎇ a ∧ b` thành nhánh lồng ở M5 để M7/M8 chính xác hơn (đụng golden M5/M6). Đề xuất: làm như một bước riêng, có golden, TRƯỚC (c).
- **Q47** Mã lỗi: dùng E0501 cho vi phạm `Send/Sync`-like của `spawn`, E0502 cho thứ tự khoá (đúng bảng hiện có); thêm ghi chú N08xx cho hiệu ứng suy ra rộng hơn khai báo? Đề xuất: chưa thêm.

### 6. Quyết định khi cài M8 nửa 1 (2026-10-08) — ĐỀ XUẤT, chờ Giang duyệt
- **Q48 (đề xuất)** Nghĩa của E0802: bảng cũ ghi "hàm `∅` gọi hàm có hiệu ứng", nhưng mẫu đã duyệt `E0801_effect_undeclared` chính là trường hợp đó. Cài đặt: E0801 = suy ra ⊄ khai báo (kể cả hàm `∅`); E0802 = thành viên impl khai hiệu ứng ngoài tập thành viên trait cho phép. Hiệu ứng trong KIỂU hàm khớp chính xác nên đưa hàm khác hiệu ứng vào chỗ kiểu hàm đã bị M4 bác bằng E0301 (không có nhánh E0802 cho trường hợp này; đã thử bỏ mã chết không kiểm thử được).
- **Q49 (SỬA 2026-10-09, theo đối chiếu với GRIT_PLAN §6.1 và crates/TASK.md)** Bản đầu đặt `effects`/`termination` trong gt-oblig — LỆCH kế hoạch. Đã tách đúng bố cục: **`gt-effects`** (hiệu ứng, `lock(L)`, thứ tự khoá E0502), **`gt-conc`** (memory ordering E0503; `Send`/`Sync` tự động ở lại `gt-types` vì là nghĩa vụ trait), **`gt-term`** (điều kiện dừng, dùng bộ giải của `gt-oblig`). `gt-edge` (M7) vẫn gộp trong `gt-oblig` (lệch kế hoạch, chưa tách). Vòng vô hạn thiếu `div` do E0701 (gt-term) sở hữu, gt-effects không suy `div` từ vòng lặp.
- **Q50 (đề xuất)** Vòng for-in (`a..b`, `&xs`) hữu hạn và tự giảm: `↓` ghi thêm ở đó bị BỎ QUA (không kiểm). Vòng không có cạnh lùi (thân luôn thoát) lặp tối đa một lần nên không cần biến thể và không bị kiểm. Cả hai đều chỉ nới lỏng chỗ vốn đã chắc chắn dừng.
- Chọn cài đặt (không cần duyệt): biến của biến thể vòng được xếp TRƯỚC trong ngân sách 24 biến zone (hàm nhiều tham số không mất biến cần chứng minh; chỉ ảnh hưởng thân có vòng cần biến thể). Đệ quy qua giá trị hàm hoặc phương thức trait luôn bị E0703 dù có `↓` (không kiểm được giảm), khai `div` nếu cố ý.

### 7. M8 nửa 2 — chỉ thị của Giang (2026-10-08) và `spawn` thuần
- **Chỉ thị đã duyệt:** (1) `Mutex`/`Atomic*` là kiểu DỰNG SẴN qua `#lang` (lib/core, prelude); KHÔNG cho ép `&!` sang con trỏ thô trong `⚠` (trái D21, D22 và mở lại đường ép kiểu mà M4 đã đóng bằng E0301). (2) Làm `spawn` thuần TRƯỚC, commit, rồi mới sang Mutex. (3) Điều kiện kèm theo: (a) `Mutex`/`Atomic*` vào danh sách lang item trong TCB (GRIT_PLAN mục 3.3), mỗi cái có hợp đồng ghi rõ bất biến, quyền truy cập, memory ordering tường minh; (b) giữ điều kiện cứng mở lại 238/361/591/554 với kịch bản `interior_mut` (tắt bản sửa để kiểm) TRƯỚC khi chúng vào fixture/lib, hoặc có lập luận tương đương mới tính cả mutability bên trong, và loại vị từ trên các kiểu này khỏi `pure_preds`; (c) thông dịch MIR cài các lang item này chỉ là công cụ test, NGOÀI TCB — **M10 phải cài bằng atomics LLVM thật; ngữ nghĩa của thông dịch không được coi là bằng chứng cho ngữ nghĩa codegen.**
- **`spawn` thuần (đã cài):** `Send` là lang trait TỰ ĐỘNG (suy theo cấu trúc, `gt-types/src/send.rs`), cấm `⊐ Send for` (E0313); `ƒ spawn<Arg: Send>(f: ƒ(Arg) ! ∅, arg: Arg) ! ∅` trong prelude; ràng buộc `Send` thất bại báo **E0501** (thay cho E0309). Thông dịch chạy `f(arg)` tuần tự tại chỗ (công cụ test, ngoài TCB; M10 phải dùng luồng thật).
- **Q51 (đề xuất, chờ duyệt) — Phase 1 bảo thủ:** MỌI tham chiếu (`&T`, `&!T`), con trỏ thô và `Arena` đều KHÔNG `Send`. Lý do: chưa có `'static`/luồng có phạm vi nên `&T` vào ngăn xếp người gọi sẽ treo khi luồng sống lâu hơn; ADR-026 viết "`&T` sang luồng khác nếu `T` đồng bộ" cần thêm ràng buộc thời gian sống mà ngôn ngữ chưa biểu diễn được. Hệ quả: chia sẻ dữ liệu chỉ qua Mutex/atomics ở nửa sau.
- **Q52 (đề xuất, chờ duyệt) — hiệu ứng của worker:** `f` trong `spawn` phải có kiểu `ƒ(Arg) ! ∅` (hiệu ứng trong kiểu hàm khớp chính xác, M4 bác khác bằng E0301). Worker có `io`/`lock(L)` chưa truyền được; cần đa hình hiệu ứng hoặc quy tắc riêng cho `spawn` khi làm Mutex (lock(L) phải đi xuyên luồng để kiểm thứ tự khoá).

## ADR-027 — M8 nửa 2: Mutex, Atomic*, Sync, thứ tự khoá (2026-10-08; theo chỉ thị Giang cùng ngày; các quyết định cài đặt nội bộ do tôi chọn theo D11/D21/D22 + fail-closed)

### 0. Điều kiện cứng mutability bên trong: ĐÃ ĐÓNG TRƯỚC khi Mutex/Atomic* vào fixture/lib
- Thêm lang item `cell` (`Cell<T>`, ghi được qua `&self`) làm kiểu thử: gt-borrowck coi MỌI vị trí region bên trong ADT lang `cell` là BẤT BIẾN, kể cả khi chỉ có `&` tới nó (`regions.rs::adt_invariant`). Nhóm "mutability bên trong" (Cell, Mutex, Atomic) dùng chung quy tắc.
- **238 (`ty_invariant`: `&T` hiệp biến) — ĐÃ GIẾT** bằng `gt-borrowck/tests/interior_mut.rs` (chương trình tấn công: trực tiếp qua `&Cell`, ADT giữ `&Cell`, hàm generic, tuple, `Option`, mảng, `&&Cell`, ghi đè muộn; đối chứng hợp lệ) và bằng KỊCH BẢN `interior_mut` của bộ sinh (oracle động hiểu `Cell::new/set/get`; 20 000 chương trình: 0 âm tính giả, 1 711 chương trình thuộc kịch bản, 1 051 oracle thấy vi phạm). **Kiểm "tắt bản sửa":** vô hiệu quy tắc `Cell` bất biến ⇒ test tấn công đỏ và oracle bắt **53 âm tính giả** trong 3 000 chương trình.
- **361, 591, 554 — HƯỚNG của từng đột biến (đọc trên mã, `gt-borrowck/src/regions.rs`):**

| # | Vị trí hiện tại | Bản gốc | Bản đột biến | Hướng |
|---|---|---|---|---|
| 361 | `Rvalue::Repeat` (dòng 372; cùng kiểu: nhánh `Aggregate(Array)` dòng 414) | `relate(el, &s, &dslots, false)` | `…, true` | **CHẶT hơn**: với `inv = true`, `relate` thêm ràng buộc ngược `dst ⊒ src` cho mọi vị trí region và coi phần bên trong là bất biến. Không bỏ ràng buộc nào. |
| 591 | kết quả lời gọi (dòng 602) | `relate(&ret_inst, &rl, dslots, false)` | `…, true` | **CHẶT hơn**: cùng cơ chế (thêm chiều ngược cho vị trí region của kết quả). Không bỏ ràng buộc nào. |
| 554 | kiểu khai báo của `&self` (dòng 565) | `SelfKind::Ref ⇒ Ty::Ref(false, Self)` | `Ty::Ref(true, Self)` | **CHẶT hơn**: `relate` xử lý `Ref(m, inner)` bằng `relate(inner, …, inv ‖ m)`, nên pointee bị coi bất biến; `walk` chỉ khớp vị trí region theo cấu trúc (cùng số vị trí với `Ref(false)`) nên không đổi gì khác. Không bỏ ràng buộc nào. |

  Cả ba đều CHỈ THÊM ràng buộc outlives, không bỏ/đổi ràng buộc nào nên **không có đột biến nào LỎNG hơn bản gốc** ⇒ không cần chứng minh "ràng buộc dư thừa về soundness". Lập luận đơn điệu: tập ràng buộc tăng ⇒ region (nhỏ nhất thoả mãn) chỉ lớn lên ⇒ khoản vay sống thêm ⇒ số xung đột/lỗi "không sống đủ lâu" chỉ tăng; mọi phán quyết của borrowck đơn điệu theo tập ràng buộc nên chúng chỉ có thể TỪ CHỐI THÊM, không bao giờ chấp nhận thêm; lập luận không dùng giả thiết gì về tính khả biến nên đúng cả với kiểu có mutability bên trong (ở đó bất biến được cài bằng chính các ràng buộc hai chiều này). Bằng chứng đo (6 000 chương trình mỗi lần, bộ sinh có `interior_mut` với Cell/Mutex/Atomic): bản gốc và cả ba bản đột biến đều ra **0 âm tính giả và 127 dương tính giả** — oracle động KHÔNG phân biệt được chúng (tức là không quan sát được khác biệt trên chương trình sinh ra), nên hướng dựa vào đọc mã chứ không dựa vào đo. Ngược lại, đột biến theo hướng LỎNG (bỏ chiều ngược của vị trí bất biến) bị bắt: tắt quy tắc `Cell` bất biến ⇒ 24 âm tính giả/3 000 chương trình. Rủi ro còn lại của ba đột biến là dương tính giả (chặn nhầm), không phải an toàn. **Chờ Giang chấp nhận lập luận.**
- **Kịch bản `interior_mut` với Mutex/Atomic THẬT (đã có):** ba biến thể — `Cell` (20%: trực tiếp, ADT giữ `&Cell`, hàm generic, tuple, `Option`, `putc`), `Mutex<&i32, L>` (40%: gán khoá chứa tham chiếu tới thứ chết ngắn hơn rồi đọc qua `with`), `Atomic<u32>` (40%: mượn `&Atomic` tới thứ chết rồi `fetch_add`). Oracle động hiểu `Cell/Mutex/Atomic::new/get/set/load/store/swap/fetch_add/fetch_sub/with` (`with` tạo khoản vay `&!` con của khoản vay trên khoá). Độ phủ (20 000 chương trình, tổng / chấp nhận+sạch / oracle thấy vi phạm): `interior_mut` 3 700 / 1 194 / 2 415; `interior_mut_mutex` 1 561 / 510 / 1 033; `interior_mut_atomic` 1 523 / 474 / 1 031; ngưỡng ≥ 300 mỗi loại đạt; **0 âm tính giả**.
- **Kiểm "tắt bản sửa" (3 000 chương trình):** tắt quy tắc bất biến của CẢ NHÓM (Cell+Mutex+Atomic) ⇒ **24 âm tính giả** (oracle bắt); chỉ tắt Mutex/Atomic, giữ Cell ⇒ **0** âm tính giả. **Nói thẳng:** với Mutex/Atomic kiểm này KHÔNG chứng minh được quy tắc bất biến là cần thiết, vì hiện không có đường nhét tham chiếu sống ngắn vào chúng — đối số của `with` phải `Send` (cấm mọi tham chiếu) và `Atomic` chỉ chứa số — nên chương trình tấn công bị chặn bởi quy tắc `Send` và quy tắc outlives thường, không bởi quy tắc bất biến. Quy tắc bất biến cho Mutex/Atomic là PHÒNG THỦ CHIỀU SÂU, chỉ thể hiện qua Cell. Khi 8.D cho phép tham chiếu trong `Arg` hoặc luồng có phạm vi, PHẢI thêm tấn công riêng cho Mutex vào bộ sinh và chạy lại kiểm tắt bản sửa. Ghi vào KNOWN_LIMITATIONS; **M8 là "ĐẠT CÓ ĐIỀU KIỆN"** vì lý do này (cộng các đột biến M7 chưa giải quyết).
- **Loại vị từ khỏi `pure_preds` (M7):** vị từ có tham số/`self` thuộc kiểu có thể chứa ô (lang `cell`, rồi Mutex/Atomic*) bị loại; tham số kiểu CÓ ràng buộc trait cũng bị loại (impl của trait có thể đọc ô), tham số kiểu không ràng buộc thì không (tính tham số: thân không quan sát được nội dung). `TypeDb::may_have_interior_mut`, test `gt-oblig/tests/pure_preds_interior_mut.rs`.

### 1. Danh sách lang item mới (vào TCB, GRIT_PLAN mục 3.3 — mỗi cái kèm hợp đồng)
Tất cả khai báo CHỈ trong lib/core/prelude (E1002 nếu ở nơi khác); thân của chúng là intrinsic trusted không được kiểm (D22); hợp đồng dưới đây là thứ ta buộc phải tin và là thứ test kiểm:
1. **`cell` — `Cell<T>`** (kiểu thử cho điều kiện cứng). Bất biến: giá trị chỉ đổi qua `set`. Quyền truy cập: `get`/`set` qua `&self`; KHÔNG `Sync`, `Send` iff `T: Send`. Region: mọi vị trí bên trong bất biến.
2. **`mutex` — `Mutex<T, L>`**, `L` là kiểu đánh dấu cấp khoá (§3). Bất biến: tại mọi thời điểm có ≤ 1 luồng chạy bên trong `with`. Quyền truy cập: chỉ qua `Mutex::with(&self, f, arg)` (API có phạm vi, KHÔNG có guard/`unlock` rời) — khoá giữ đúng trong lời gọi `f`. `Send`/`Sync` iff `T: Send`. Mọi vị trí region bên trong bất biến.
3. **`atomic` — `Atomic<T>`** với `T: AtomicInt` (trait lang do prelude impl cho `bool, i32, u32, i64, u64, usize`). Bất biến: mọi thao tác nguyên tử, thứ tự theo memory ordering ĐƯỢC TRUYỀN. Quyền truy cập: qua `&self`. `Send`/`Sync` luôn. Mọi thao tác nhận `Ordering` BẮT BUỘC (không tham số mặc định).
4. **`ordering` — `enum Ordering { Relaxed, Acquire, Release, AcqRel, SeqCst }`** (kiểu thường, không phải từ khoá).
5. **`sync` — trait tự động `Sync`**, tương tự `Send` (§2).
6. **`effect_poly`** — đánh dấu hàm lang có tham số kiểu hàm với hiệu ứng đa hình (§3).
7. **Thuộc tính `#level("n")`** trên kiểu đánh dấu khoá — gán cấp (§3).
Mỗi lang item có test hợp đồng riêng (corpus + thuộc tính). Thông dịch MIR cài chúng (tuần tự, tất định) chỉ là CÔNG CỤ TEST, NGOÀI TCB: **M10 phải cài bằng atomics và khoá LLVM/hệ điều hành thật; ngữ nghĩa của thông dịch KHÔNG được coi là bằng chứng cho ngữ nghĩa codegen** (thông dịch không thể hiện đua dữ liệu hay hành vi memory ordering).

### 2. (a)+(b) Chia sẻ giữa luồng và Sync-like tự suy
- **Đường chia sẻ duy nhất ở Phase 1: `■` (static) có kiểu `Sync`** (D11 đã cho phép "biến toàn cục khả biến qua kiểu đồng bộ"). Static sống mãi nên không treo; luồng truy cập bằng TÊN, không truyền tham chiếu (Q51 giữ: tham chiếu không `Send`). `■ X: T ≔ …` đòi `T: Sync` (E0501, thông báo "không Sync"). Kiểu sở hữu chung (`Arc`-like) để 8.D.
- **`Sync` suy theo cấu trúc** (như `Send`): nguyên thuỷ/hàm `Sync`; `&T`, `&!T`: `Sync` iff `T: Sync`; con trỏ thô, `Arena`, `Cell<T>`: KHÔNG; `Mutex<T,L>`: iff `T: Send`; `Atomic<T>`: có; ADT/tuple/mảng: mọi thành phần `Sync`; tham số kiểu cần ràng buộc `Sync`. `Send` cập nhật: `Mutex<T,L>` iff `T: Send`, `Atomic` có, `Cell<T>` iff `T: Send`.

### 3. (c) Cấp khoá, thứ tự tăng nghiêm ngặt, tóm tắt hiệu ứng `lock(...)`
- **Cấp khoá = kiểu đánh dấu mang thuộc tính cấp**: `#level("10") ▢ CfgLock { }` rồi `Mutex<Config, CfgLock>`. Cú pháp thuộc tính hiện có nhận ident hoặc chuỗi nên số viết dạng chuỗi. KHÔNG thêm cú pháp mới của ngôn ngữ (không const generic).
- **Hiệu ứng `lock(L)`** (cú pháp hiệu ứng đã có) ghi tên kiểu đánh dấu. `Mutex::with` có hiệu ứng `lock(L)` (L lấy từ đối số kiểu thứ hai của lời gọi) hợp với hiệu ứng của hàm `f` truyền vào. Tóm tắt hiệu ứng truyền xuyên hàm qua cơ chế E0801 sẵn có (khai báo ⊇ suy ra).
- **Quy tắc thứ tự — kiểm tại mỗi lời gọi `Mutex::with(m, f, a)`**: mọi `lock(L')` trong hiệu ứng của `f` (đã là bao đóng bắc cầu nhờ E0801) phải có cấp(L') > cấp(L) TUYỆT ĐỐI; vi phạm (kể cả L' = L: tự khoá lại) ⇒ **E0502**. Vì khoá giữ đúng trong lời gọi `f`, mọi khoá có thể lấy khi đang giữ L đều nằm trong tóm tắt của `f`; nên kiểm không cần luồng dữ liệu về "tập khoá đang giữ" và đầy đủ (không sót đường lồng). Thứ tự toàn cục tăng nghiêm ngặt ⇒ không chu trình chờ ⇒ không deadlock.
- **Hiệu ứng đa hình theo hàm truyền vào (giải Q52)**: hàm lang `spawn` và `Mutex::with` nhận tham số kiểu hàm mà hiệu ứng KHÔNG cố định: M4 chấp nhận mọi tập hiệu ứng ở tham số hàm của hàm được đánh dấu `#lang(effect_poly)`, và hiệu ứng của lời gọi = hiệu ứng khai báo ∪ hiệu ứng kiểu của đối số hàm. Với `spawn`: hiệu ứng của worker truyền vào người gọi TRỪ `lock(*)` (worker không chạy lồng trong khoá người gọi đang giữ nên không tạo cạnh thứ tự; worker tự bị kiểm thứ tự trong chính nó). Tập hiệu ứng worker được phép: **`lock(L)`, `mem`, `io`, `st`, `div`**; `io` được phép (hiệu ứng thuộc `Cap<…>` đã bị chặn bởi D28/D29 nên worker không tự phát capability; người gọi phải khai `io`); `ffi` bị cấm Phase 1 (D14). Cập nhật E0501/Q52 cho khớp.
- **Kết quả `with` không chứa tham chiếu** (`R: Send`, `Arg: Send` — Send cấm mọi tham chiếu): ngăn `&!T` trong khoá rò ra ngoài lời gọi.

### 4. (d) Atomics bắt buộc memory ordering tường minh
- Mọi thao tác (`load/store/swap/fetch_add/fetch_sub/compare_exchange`) có tham số `Ordering` KHÔNG mặc định ⇒ thiếu đối số là lỗi arity thường (E0301). Đối số ordering phải viết THẲNG là biến thể (`Ordering::Acquire`); biểu thức khác ⇒ **E0503**. Tổ hợp không hợp lệ ⇒ **E0503**: `load` với `Release`/`AcqRel`; `store` với `Acquire`/`AcqRel`; `compare_exchange` với thứ tự thất bại `Release`/`AcqRel` hoặc mạnh hơn thứ tự thành công. (E0503 là mã mới: bảng mã lỗi + docs/errors ≥3 fail + ≥3 pass.)

### 5. Rủi ro / đánh đổi
- API khoá có phạm vi (`with`) không biểu diễn được giữ-khoá-qua-nhiều-hàm bằng guard; đổi lại kiểm thứ tự đầy đủ mà không cần phân tích sống của guard (an toàn hơn). Ghi vào KNOWN_LIMITATIONS.
- Không có oracle động cho đua dữ liệu/deadlock (thông dịch tuần tự): độ tin cậy dựa corpus + test cấu trúc + đột biến; M10 cần kiểm thử luồng thật (ThreadSanitizer/loom-like) trước khi tuyên bố đa luồng.

## ADR-028 — M9: `gritc`, Cổng phán quyết, secret scanner — DỰ THẢO viết TRƯỚC khi code (2026-10-08; CHỜ GIANG DUYỆT — chưa có dòng mã M9 nào)

### 0. Điều kiện xuất phát (nói thẳng)
- M8 xong về mã (nửa 1: dừng, hiệu ứng, DomainQuery; nửa 2: `spawn`, `Send`/`Sync`, `Mutex`/`Atomic`, thứ tự khoá, ordering). Cổng đầy đủ xanh (634 test). **Chưa có codegen (M10)**: ở M9 `gritc build` KHÔNG sinh binary thật và `gritc run` KHÔNG chạy mã máy; phần "chạy" của M9 dùng thông dịch MIR tuần tự (§1). Mọi tuyên bố "ĐỎ ⇒ không chạy" ở M9 kiểm bằng thông dịch; M10 phải lặp lại kiểm bằng binary thật.
- **Hai điều kiện cứng đang áp dụng cho M9:** (a) GRIT_PLAN §7 điểm 10: không còn đột biến "xoá thêm check" chưa giải quyết ở `gt-oblig/src/analysis.rs` (69 đột biến nguy hiểm: `analysis.rs` 62 + `driver.rs` 7, xem KNOWN_LIMITATIONS "gt-oblig") và có chế độ **giữ mọi check** làm MẶC ĐỊNH; (b) điểm 11: `gritc` không được sinh binary nếu E0601 (M7) hoặc E0307 (M5) chưa triển khai và qua test. E0601 và E0307 ĐÃ triển khai và có corpus; M9 biến chúng thành cổng chặn kiểm được ở mức CLI (§5).

### 1. Lệnh `gritc` và cấu trúc crate
- **Crate:** `gt-driver` (thư viện: chạy chuỗi pass, bộ tổng hợp phán quyết, bằng chứng/attestation, báo cáo JSON), `gt-secrets` (thư viện: S1xx/S2xx), `gritc` (binary mỏng: đọc đối số, gọi `gt-driver`, ánh xạ exit code). Toàn bộ logic kiểm tra nằm ở thư viện đã có; `gritc` KHÔNG tự suy phán quyết (một nguồn sự thật duy nhất, D17).
- `gritc check <tệp|thư mục> [--json] [--deny-yellow] [--checks=all|proven]`: chạy lexer → parser → resolve → types → MIR → borrowck → oblig (nghĩa vụ) → dừng → hiệu ứng/đa luồng → secret scanner; in chẩn đoán (người đọc được hoặc JSON schema v1 có version); KHÔNG ghi tệp nào ngoài báo cáo yêu cầu.
- `gritc build <tệp> [-o ra] [--checks=…]` (M9): `check` + ghi **gói trung gian** `*.gtmir` (MIR đã hạ + danh sách check giữ/xoá + attestation băm). Ghi nguyên tử (tệp tạm rồi đổi tên) và CHỈ khi phán quyết ≤ VÀNG cho phép. M10 thay gói bằng binary LLVM.
- `gritc run <tệp> [--checks=…]` (M9): `check` rồi chạy gói bằng **thông dịch MIR tuần tự** (công cụ tham chiếu, NGOÀI TCB: không phải bằng chứng cho codegen). In rõ "chạy bằng thông dịch tham chiếu". M10 chạy binary thật.
- `gritc explain <mã>`: in `docs/errors/<mã>.md` (nhúng lúc biên dịch bằng `include_str!`; mã lạ ⇒ exit 3). `gritc fmt [--check]`: dùng `gt-fmt` (chuẩn hoá về dạng canonical; `--check` exit 1 nếu khác).
- Không có đối số ẩn, không có cấu hình ngầm; cùng đầu vào + cùng phiên bản ⇒ cùng đầu ra (D9/D20).

### 2. Chế độ giữ mọi check — MẶC ĐỊNH (trả lời điều kiện cứng điểm 10)
- `--checks=all` (**mặc định**): mọi `Assert` do M5 sinh ra ĐƯỢC GIỮ, kể cả check đã chứng minh. Bộ giải nghĩa vụ vẫn chạy và vẫn báo ĐỎ (E0601…) và GHI CHÚ N06xx; chỉ khác là KHÔNG xoá. `--checks=proven`: xoá check đã chứng minh (đường M7 hiện có) — chỉ dùng cho đo hiệu năng và thử nghiệm, phải gõ tường minh.
- **Điều kiện đổi mặc định sang `proven`:** (1) mọi đột biến "xoá thêm check" của `analysis.rs`/`driver.rs` đã bị giết hoặc có lập luận tương đương được Giang chấp nhận; (2) báo cáo đột biến mới trên `analysis.rs` đạt cùng ngưỡng như `regions.rs`; (3) Giang duyệt bằng văn bản. Đến lúc đó cờ mặc định là hằng số duy nhất trong `gt-driver`, có test khoá giá trị.
- Test bắt buộc: chương trình mẫu chạy qua `gritc run` ở `all` có số `Assert` thực thi ≥ ở `proven` và KHÔNG có khác biệt quan sát được ngoài các abort thêm vào chỗ chương trình sai; mutation: đột biến cờ mặc định phải bị bắt.

### 3. Từ chối chạy khi ĐỎ hoặc ICE; exit code
- **Một cổng duy nhất:** `gt-driver::verdict(pass_results) -> Verdict { Green, Yellow, Red, Ice }`. `build` và `run` nhận ĐÚNG đối tượng `Verdict` do `check` trả về trong cùng tiến trình (không gọi lại, không tái hiện logic). Chỉ `Green`/`Yellow` (và `Yellow` khi KHÔNG `--deny-yellow`) mở cổng; `Red`/`Ice` ⇒ không ghi tệp đầu ra, không chạy.
- **ICE:** mọi pass chạy qua `gt_stack::run_with_big_stack`; panic hay lỗi tạo luồng ⇒ `E9901` ⇒ `Ice`. Bộ tổng hợp tự phòng thủ: pass nào thiếu kết quả (chưa chạy, trả lỗi không rõ) ⇒ `Ice`, không bao giờ coi "không có chẩn đoán" là sạch nếu pass không báo đã hoàn tất (danh sách pass bắt buộc ở §5).
- **Exit code (D17 giữ nguyên):** `0` = qua (có thể kèm VÀNG), `1` = có ĐỎ (hoặc có VÀNG khi `--deny-yellow`), `2` = ICE, **`3` = lỗi dùng/IO** (đối số sai, không đọc được tệp, mã `explain` lạ). Sau khi cổng mở, `gritc run` trả: `0` nếu chương trình kết thúc bình thường; `70` nếu chương trình **abort** (D3: tràn, chỉ số, chia 0… — mọi `Assert` thất bại). Mã thoát tuỳ ý của chương trình người dùng đi qua JSON (`run.exit_status`), KHÔNG qua exit code của `gritc`, để 1/2/3/70 không bao giờ mơ hồ. Mọi exit code ở M9 có test ở mức tiến trình.
- Test bắt buộc ("ĐỎ ⇒ không output, không chạy"): với mẫu ĐỎ mọi lệnh `build`/`run` trả 1, KHÔNG tạo tệp `-o`, KHÔNG gọi thông dịch (đếm bằng móc đo lường test); với mẫu ICE (chèn panic bằng hook test-only) trả 2 như vậy; với mẫu VÀNG: chạy và in cảnh báo, `--deny-yellow` ⇒ 1 và không chạy.

### 4. VÀNG, `#allow` có lý do, `--deny-yellow`
- VÀNG = mã `S…` (secret scanner, S1xx/S2xx theo mục 5 của kế hoạch). `N…` là GHI CHÚ, không phải VÀNG; ĐỎ `E…` không có cách tắt.
- `#allow(S1xx, "lý do")` đặt TRƯỚC item; phạm vi = item đó. **Lý do bắt buộc, không rỗng** (thiếu/rỗng ⇒ ĐỎ, mã mới **E1301**-họ: sẽ chọn số khi cài, đưa vào bảng mã + docs + ≥3 fail/≥3 pass); `#allow` mã ĐỎ hoặc mã không tồn tại ⇒ ĐỎ; `#allow` không dùng tới ⇒ GHI CHÚ (không im lặng nuốt). Mọi `#allow` đã áp dụng xuất hiện trong JSON `allowed:[{code, reason, span}]` làm dấu vết kiểm toán.
- `--deny-yellow`: mọi VÀNG còn lại sau `#allow` ⇒ exit 1 và không chạy. Mặc định: VÀNG ⇒ chạy + in cảnh báo.

### 5. Cổng chặn cứng E0601 và E0307 (điểm 11)
- `gt-driver` giữ **danh sách pass bắt buộc** (`REQUIRED_PASSES`): tên các pass phải báo "đã hoàn tất" trong attestation, trong đó có `m5.return_paths` (E0307) và `m7.const_bounds` (E0601). Thiếu một pass bắt buộc (cấu hình sai/biên dịch thiếu) ⇒ `Ice`, KHÔNG sinh gói, KHÔNG chạy.
- Test CLI: các mẫu `tests/fail/E0601_*` và `E0307_*` qua `gritc build`/`run` PHẢI ra exit 1 và không có tệp đầu ra (chạy mọi mẫu của hai mã). Test "gỡ pass": dựng `gt-driver` với danh sách pass thiếu `m7.const_bounds` ⇒ `gritc build` exit 2.
- Attestation (gói `*.gtmir`): băm nguồn + phiên bản gritc + danh sách pass đã chạy + kết quả + chế độ `--checks`; M10 IR Guard đối chiếu.

### 6. Báo cáo JSON "check còn lại" (remaining checks)
- Thêm vào JSON `gritc check --json` (schema v1, **chỉ thêm trường** theo quy tắc tương thích; tăng `schema_version` chỉ khi đổi/xoá): `checks: { mode: "all"|"proven", kept: [...], deleted: [...], would_delete: [...] }`. Mỗi phần tử: `{ id, fn, kind (overflow.Add, index, div_zero…), span, status: "kept_not_proven" | "kept_mode_all" | "deleted_proven" , solver: "interval+zone"|"exhaustive"|null }`.
- Ở `--checks=all`: `deleted` rỗng; `would_delete` liệt kê những check mà `proven` sẽ xoá — đây là nguồn dữ liệu để người/AI đánh giá độ tin cậy trước khi bật `proven`. `kept` gồm cả check giữ vì chưa chứng minh được (`kept_not_proven`) và vì chế độ (`kept_mode_all`).
- Thứ tự xác định (theo hàm, vị trí). Test golden trên vài chương trình mẫu; kiểm: tổng `kept + deleted` = số `Assert` của MIR.

### 7. Secret scanner (S1xx/S2xx)
- Theo mục 5 của kế hoạch: mẫu khoá API/token/khoá riêng/URL kèm thông tin xác thực trong literal chuỗi và comment; mỗi mẫu một mã `S…`, mức VÀNG, có test pass/fail (≥3 mỗi loại) và tài liệu giới hạn (không phải bộ quét đầy đủ; có dương tính giả/âm tính giả). Chạy trên nguồn đã qua lexer (biết chuỗi/comment), không trên MIR.

### 8. Rủi ro / đánh đổi
- Thông dịch MIR là "bộ chạy tham chiếu" mang đi trong sản phẩm: sai ngữ nghĩa của nó KHÔNG phá bảo đảm an toàn của binary M10 nhưng có thể gây hiểu nhầm; nhãn rõ trong đầu ra và tài liệu, không dùng làm bằng chứng codegen.
- `--checks=all` mặc định làm chương trình chậm hơn; chấp nhận cho tới khi điều kiện §2 đạt.
- Phạm vi M9 không gồm: codegen, LSP, `rig`, `prove`/`recheck`.

### 9. Câu hỏi cho Giang
- **Q53** Duyệt chia crate `gt-driver`/`gt-secrets`/`gritc` và việc `build` ở M9 sinh gói `*.gtmir` (chưa phải binary)?
- **Q54** Duyệt mặc định `--checks=all` và ba điều kiện đổi mặc định ở §2?
- **Q55** Duyệt exit code thêm `3` (lỗi dùng/IO) và `70` (chương trình abort) cho `run`, mã thoát tuỳ ý của chương trình qua JSON?
- **Q56** Duyệt `#allow` bắt buộc lý do không rỗng, lỗi thiếu lý do là ĐỎ (mã mới họ E13xx), và `#allow` không dùng tới là GHI CHÚ?
- **Q57** Duyệt `REQUIRED_PASSES` làm cổng chặn cứng E0601/E0307 ở mức CLI (§5)?
- **Q58** Duyệt trường JSON `checks.{kept,deleted,would_delete}` (§6) là thêm trường vào schema v1?
