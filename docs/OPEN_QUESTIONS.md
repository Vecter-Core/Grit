# OPEN QUESTIONS — cần Giang trả lời (agent dừng ở các điểm này)

- **Q1 — Seed cũ trong git.** Working tree đã xoá toàn bộ seed P8k (examples/, crates/gritc/…), chỉ còn GRIT_PLAN.md. Xác nhận: (a) bỏ hẳn và khởi động lại theo plan (agent đang giả định thế), hay (b) giữ/di chuyển seed cũ vào `legacy/`? Có commit việc xoá không?
- **Q2 — Đo token bằng tokenizer thật.** Chỉ có proxy tiktoken (ADR-002). Cần số đo từ tokenizer của Claude (hoặc chấp nhận proxy) trước khi đóng băng bảng ký hiệu. Đề xuất cụ thể: bỏ `⊤ ⊥` (dùng `true/false`).
- **Q3 — Duyệt ký hiệu M0★** (xem `docs/spec/symbols.md`): `⎇ ⇝ ⌥ ↦ ⏎ ▢ ⊔ ▦ ⇲ ◈ ⊏ ⊐`, `@r` cho region, `as` cho ép kiểu, `*` giải tham chiếu, `self_param`.
- **Q4 — Gán mã lỗi.** Plan để mơ hồ vị trí "match đầy đủ" (E01xx/E03xx/E06xx) và "thiếu capability" (E03xx/E08xx). Đề xuất E0305 và E0310 (`docs/spec/error_codes.md`).
- **Q5 — `μ` là chữ Hy Lạp nằm trong XID.** `μs` có thể bị lexer đọc là định danh. Đề xuất: `μ` luôn là token riêng khi đứng ngay trước định danh ở vị trí khai báo; định danh người dùng KHÔNG được bắt đầu bằng `μ`. Cần duyệt vì nó hạn chế tên biến.
- **Q6 — Việc giao cho "AI rẻ hơn".** Theo yêu cầu giảm token, phần đơn giản được để dưới dạng file `TASK.md` thay vì làm (liệt kê trong PROGRESS.md). CLAUDE.md cấm `// TODO`/stub; các file này là mô tả việc, không phải code giả vờ chạy. Xác nhận cách này chấp nhận được, và rằng M0 chỉ "xong" sau khi các việc đó được làm.
