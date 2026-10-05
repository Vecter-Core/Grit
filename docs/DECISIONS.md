# DECISIONS (ADR ngắn: bối cảnh – quyết định – hệ quả)

## ADR-001 — Cấu trúc workspace & toolchain (M0)
- **Bối cảnh:** plan §6.1 yêu cầu workspace nhiều crate; M0 cấm viết compiler.
- **Quyết định:** `Cargo.toml` workspace gốc, `resolver = "3"`, edition 2024, `unsafe_code = forbid` và clippy `all = deny` ở mức workspace; `members = []` cho tới khi từng crate được tạo ở milestone của nó. Toolchain hiện tại: rustc/cargo 1.98.1.
- **Hệ quả:** không có crate "rỗng giả vờ chạy". Ngoại lệ FFI LLVM (M10) cần ADR riêng.

## ADR-002 — Bảng ký hiệu: ĐỀ XUẤT, chưa đóng băng (M0)
- **Bối cảnh:** plan §2.3/§6.2 yêu cầu đo token trước khi chốt. Môi trường không có tokenizer của Claude; dùng `tiktoken` (cl100k, o200k) làm **proxy** (`tools/measure_tokens.py`, 18 chương trình `tests/pass`).
- **Số đo (proxy, KHÔNG phải Claude):** mỗi ký hiệu canonical tốn 2–4 token. Trên toàn bộ mẫu, bản ASCII alias tốn **nhiều hơn** canonical: +4,0% (cl100k), +6,1% (o200k). Nhưng nhiều ký hiệu hiếm (`⎇ ⌥ ↦ ⏎ ⊏ ⊐ ⊠ ⊡ ⊔ ⊢ ⊳ ⊲`) tốn 4 token so với 3 của alias; `⊤ ⊥` 4 so với 2 (`true/false`).
- **Quyết định tạm:** giữ canonical làm dạng chuẩn cho ký hiệu trong plan §4.2; **đề xuất** bỏ `⊤ ⊥`, dùng từ khoá `true/false` làm dạng duy nhất; ký hiệu M0★ (không có trong plan) chờ đo bằng tokenizer thật (OPEN_QUESTIONS Q2). Chưa đóng băng.
- **Hệ quả:** nếu số đo Claude khác, chỉ đổi bảng ánh xạ lexer (alias↔canonical); AST không đổi vì thiết kế 1–1.

## ADR-003 — Repo có lịch sử "seed cũ" khác thiết kế (M0)
- **Bối cảnh:** `git log` có commit P8k-1…P8k-5 (compiler `gritc` một crate, cú pháp `▶ Δ ↻`, codegen ELF x86-64 trực tiếp, không kiểm sở hữu/biên). Working tree hiện chỉ còn `GRIT_PLAN.md` (mọi file cũ bị xoá, chưa commit). Cú pháp cũ mâu thuẫn plan.
- **Quyết định:** coi việc xoá là chủ ý khởi động lại theo `GRIT_PLAN.md`; **không** khôi phục, **không** commit việc xoá (chờ Giang). Lịch sử git vẫn giữ mã cũ để tham khảo (vd: `break/continue` dùng `⊠ ⊡`).
- **Hệ quả:** M0 làm từ đầu. Xem OPEN_QUESTIONS Q1.
