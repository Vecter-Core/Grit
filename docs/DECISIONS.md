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
