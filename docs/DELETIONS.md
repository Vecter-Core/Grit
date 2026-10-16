# DELETIONS — danh sách cho phép MỌI lần xoá tệp đã theo dõi (lệnh Giang 2026-10-16)

Quy tắc: mọi tệp từng bị xoá trong lịch sử git (`git log --diff-filter=D --name-only`) PHẢI có một dòng bảng ở đây: `| đường dẫn | commit | lý do |`.
Xoá dưới `tests/`, `docs/errors/`, `lib/`, `crates/*/src/` mà lý do RỖNG ⇒ cổng ĐỎ (giai đoạn `deletions` của `tools/gate.sh`, `tools/deletions_check.sh`).
Commit xoá tệp phải cập nhật tệp này CÙNG commit (`tools/commit_check.sh`, hook pre-commit).

| đường dẫn | commit | lý do |
|---|---|---|
| tests/TASK.md | 8dad6c1 | tệp giữ chỗ "việc cho milestone sau"; xoá khi chốt đặc tả M0 (Q7–Q14) |
| runtime/gt-rt/TASK.md | ca0ba0a | tệp giữ chỗ; xoá khi M10 bước 4 tạo `runtime/gt-rt` thật |
| tests/bench/TASK.md | 5f110ab | tệp giữ chỗ; xoá khi M10 bước 8–9 thêm benchmark nền (`tests/bench/*.gt`) |
| crates/gt-fmt/src/comments.rs | d370890 | M2e: luật chú thích theo từng vị trí được thay bằng MỘT cơ chế gắn token (`attach.rs`, ADR-016, Q16b); `comments.rs` bị thay thế |
| tests/fail/E1301_comment_in_expression.gt | d370890 | M2e: cơ chế gắn token chấp nhận chú thích ở vị trí này (ADR-016: 0 từ chối khi chèn `// zz` sau mọi token), nên mẫu "fail E1301" không còn đúng; E1301 chỉ còn là biện pháp cuối cùng (KNOWN_LIMITATIONS) |
| tests/fail/E1301_comment_in_signature.gt | d370890 | như trên (chú thích trong chữ ký) |

## Ghi chú về `lib/core/TASK.md`
Tệp này biến mất khỏi cây làm việc (không qua commit) ngày 2026-10-16 và đã khôi phục bằng `git checkout`; nguyên nhân chưa rõ (xem KNOWN_LIMITATIONS "Tệp bị xoá không rõ nguyên nhân"). Nó vẫn được theo dõi, nên KHÔNG có dòng ở đây (chưa bị xoá trong lịch sử). Nếu sau này chủ ý xoá (giống các `TASK.md` giữ chỗ khác khi M12 đóng), thêm dòng ở đây trong CÙNG commit.
