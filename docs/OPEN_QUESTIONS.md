# OPEN QUESTIONS (lịch sử — giữ làm bản ghi quyết định, không xoá)

## Đã quyết (Giang, 2026-10-05) — chi tiết ở DECISIONS.md
- **Q1 — Seed cũ trong git. [ĐÃ QUYẾT]** Xoá sạch, không giữ tag/lịch sử; bắt đầu mới từ M0. → ADR-003.
- **Q2 — Đo token. [ĐÃ QUYẾT]** Chấp nhận số đo tiktoken (proxy) làm tham khảo. Bỏ `⊤ ⊥`, dùng `true/false`. → ADR-002.
- **Q3 — Ký hiệu M0★. [ĐÃ QUYẾT]** Duyệt cả 13 ký hiệu M0★. `*`, `self`, `as` giữ ASCII (`self` là từ khoá cứng). Bổ sung `≡ □ ■`, `\extern` (dành riêng), lifetime `'a` (ASCII-only, thay `@r`). → ADR-006.
- **Q4 — Mã lỗi. [ĐÃ QUYẾT]** E0305 (match đầy đủ), E0310 (capability). → ADR-004.
- **Q5 — `μ` vs XID. [ĐÃ QUYẾT]** `μ` là token riêng `MUT`, không gộp vào định danh; có test lexer. → ADR-002.
- **Q6 — Việc giao AI khác. [ĐÃ QUYẾT]** Hoãn CI/deny/errors/crates; làm ngay `tests/TASK.md`. → ADR-005.
- **Q7 — Ràng buộc trait. [ĐÃ QUYẾT]** Phương án (b): inline `<T: A + B>`, không dùng `|`. Dành riêng `\where` (token `KEYWORD_WHERE`, chưa có production — Phase 2). `for` là từ khoá cứng ASCII. → ADR-007.
- **Q8 — Dấu kết thúc lệnh. [ĐÃ QUYẾT]** `;` tường minh, quy tắc kiểu Rust (biểu thức cuối không `;` = giá trị khối; có `;` = trả `()`; hàm trả khác `()` mà khối kết thúc bằng `;` không có `⏎` → E0307). `block_stmt` (`⎇ ⌥ ⟳ ⚠` kết thúc tại `}`) không cần `;`. → ADR-007.
- **Q9 — Toán tử bit. [ĐÃ QUYẾT]** Không thêm ký hiệu; dùng hàm `band/bor/bxor/bnot/shl/shr` trong `lib/core` (M12). Ghi KNOWN_LIMITATIONS; không chặn M1–M11. → ADR-007.
- **`■` static. [ĐÃ XÁC NHẬN]** Chỉ bất biến ở Phase 1 (D11). **Tên token tạm (`MUT`, `STAR`, `LIFETIME`…). [CHẤP NHẬN]** M1 chốt chính thức.

- **Q10 — for…in. [ĐÃ QUYẾT]** `⟳ x in xs { }` với `in` từ khoá cứng ASCII; termination trên range/slice hữu hạn tự suy (semantics §21). → ADR-008.
- **Q11 — while. [ĐÃ QUYẾT]** `⟳ cond { }` làm while dùng chung `⟳`; không cond = vô hạn (bắt buộc `div`); termination suy từ cond, `↓` tuỳ chọn và được xác minh nếu có. → ADR-008.
- **Q12 — never + kiểu hàm. [ĐÃ QUYẾT]** `#noreturn` chỉ cho intrinsic trusted; kiểu hàm tĩnh `ƒ(A) → R ! E`; kiểu never `!` thêm vào prim. → ADR-008.
- **Q13 — con trỏ thô. [ĐÃ QUYẾT]** `*□ T` / `*μ T` (ASCII `*\const T` / `*\mut T`); dùng ngoài `⚠` → E1001 (không thêm E1002). → ADR-008.
- **Q14 — điểm agent tự chọn. [ĐÃ QUYẾT]** Duyệt tất cả, ngoại trừ: range mở `a..`, `..b`, `..` PHẢI có ở Phase 1 (slice index + pattern); độc lập chưa có ngữ nghĩa (chờ iterator Phase 2). → ADR-008.

## Đang mở
(không có)
