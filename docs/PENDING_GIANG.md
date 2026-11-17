# PENDING_GIANG — việc CHỜ GIANG (lập 2026-11-17; agent KHÔNG tự đánh dấu thay Giang)

Mỗi mục: đề xuất của kiến trúc sư/agent và **hành vi mặc định trong lúc chờ**. Agent làm tiếp mọi việc không phụ thuộc mục này.

| # | CHỜ GIANG | Đề xuất | Mặc định trong lúc chờ |
|---|---|---|---|
| 1 | Hợp nhất `integration` vào `main` | chờ P1–P3 xong và cổng xanh trên đầu cuối, rồi Giang tự hợp nhất (hoặc ra lệnh rõ). `main` đang ở `cc23c24`, cách `integration` khoảng 130 commit | KHÔNG làm; không push `main` |
| 2 | Quyền ghi `D:\Grit_backup` (ACL chỉ cho đọc) | cấp quyền ghi cho tài khoản chạy agent | bundle ở `D:\Grit\.backup` (có `git bundle verify`) + push `origin/windows-work` |
| 3 | Đọc `docs/KNOWN_LIMITATIONS.md` và `docs/PROGRESS.md` (điều kiện READINESS #6) | đọc `docs/KNOWN_LIMITATIONS_DIGEST.md` (mục lục, ≤150 dòng) trước, rồi bản đầy đủ | #6 giữ "CHƯA" |
| 4 | Xác nhận Q23 (match số nguyên bằng khoảng, E0601) và Q35–Q42 (thiết kế bộ giải M7, ADR-025) — đã cài, `OPEN_QUESTIONS.md` vẫn ghi chờ | xác nhận (đã có corpus/đột biến); nếu muốn đổi thì nêu | coi là "đã cài, chờ xác nhận"; không đổi mã |
| 5 | Điều kiện #9 (3 đột biến M6 chấp nhận bằng lập luận đơn điệu + đo) và #10 (30 đột biến M7 chỉ có lập luận tương đương) | xác nhận chấp nhận có điều kiện lần cuối, hoặc đòi thêm bằng chứng | giữ "đạt có điều kiện" |
| 6 | Chấp nhận rủi ro H7 `Atomic`: không có ThreadSanitizer/loom (không oracle đua dữ liệu) | chấp nhận cho Seed, ghi vào SAFETY_CLAIMS; xem lại ở Phase 2 | giữ nguyên là rủi ro chấp nhận có ý thức, KHÔNG gọi là đã duyệt |
| 7 | Có làm Giai đoạn 2 / M13 đầy đủ (hover… mở rộng) không | sau khi nhãn `1.0.0-seed` đạt | KHÔNG làm Giai đoạn 2; M13 chỉ tới mức P3.b |
| 8 | Phiên Claude/model khác họ đọc mã TCB do Codex viết (R11b), nếu agent hiện tại không phải Claude | mở phiên Claude theo `docs/E_PROMPT_CLAUDE.md` | RUNLIST "R11b CHỜ PHIÊN CLAUDE" |
| 9 | Phiên sạch viết capstone K1–K4 trên Windows | mở phiên mới chỉ đọc `AI_GUIDE` + `STD_REFERENCE` + đề (`docs/CAPSTONE_PROMPTS.md`) | RUNLIST "CAPSTONE WINDOWS: CHỜ PHIÊN SẠCH" |
| 10 | Chạy `tools/fuzz_loop.sh <giờ>` qua đêm để đạt 24 giờ/mục tiêu (điều kiện #2) | lệnh ghi ở HANDOFF sau P3.a | #2 ghi "ĐANG TÍCH LUỸ" kèm số thật |
