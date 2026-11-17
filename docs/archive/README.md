# docs/archive — tài liệu của việc ĐÃ XONG (dọn dẹp 2026-11-17)

Mọi thứ ở đây được **chuyển bằng `git mv`** (lịch sử git còn nguyên, không xoá nội dung, không động tới mã). Không tệp nào ở đây được cổng/test/công cụ đọc theo đường dẫn cũ (đã kiểm bằng `git grep` trước khi chuyển). Cần xem lại việc cũ thì đọc ở đây; muốn biết trạng thái HIỆN TẠI thì đọc `docs/PROJECT_STATUS.md`.

| Tệp / thư mục | Là gì | Vì sao đưa vào lưu trữ |
|---|---|---|
| `MERGE_NOTES.md` | Phân tích từng commit của nhánh Claude (Linux) trước khi hợp nhất vào nhánh Windows | Hợp nhất đã xong ở commit `c3ba7de` (26 xung đột đã giải, cổng hậu merge GREEN) |
| `SEED_PLAN.md` | Kế hoạch S0–S8 tới "Seed 1.0" (lập 2026-10-21) | Các bước S0–S8 đã chạy xong (ra bản `1.0.0-rc2`); trạng thái còn lại nằm ở `docs/RUNLIST.md` và `docs/PROJECT_STATUS.md`. Vẫn là nơi định nghĩa SÁU điều kiện A1–A6 của nhãn `1.0.0-seed` |
| `E_PROMPT.md` | Lời nhắc đọc-mã-đối-kháng bản đầu (subagent CÙNG họ model) | Việc S7 đã xong; bản mới còn hiệu lực là `docs/E_PROMPT_CLAUDE.md` (dành cho Claude/model khác họ — vẫn CHỜ) |
| `adversarial/` | Báo cáo + chương trình tái hiện của 5 nhóm đọc-mã-đối-kháng (A–E) và `PROMPT.txt` | Việc S7 đã xong; là BẰNG CHỨNG được `docs/FAIL_OPEN_AUDIT.md` §9 và `CHANGELOG.md` dẫn (đã sửa đường dẫn) |
| `scratch/scratch_edit2.py` | Script sửa-tay một lần cho `gt-codegen-llvm/src/lower.rs` (2026-10-13) | Tệp tạm lỡ vào git ở thư mục gốc; không công cụ nào dùng |

Cũng trong đợt dọn dẹp này:
- `docs/HANDOFF.md`: phần lịch sử tháng 10 (mục 0, 3, 5, 6.1–8, "kiểm toán ngoài") được chuyển **nguyên văn** xuống cuối `docs/HANDOFF_ARCHIVE.md` (đề mục "Chuyển từ `docs/HANDOFF.md` ngày 2026-11-17").
- Tệp rỗng `{}` ở thư mục gốc đã bị xoá (có dòng ở `docs/DELETIONS.md`).

**Chưa động tới (cố ý):** `audit/` (274 tệp: bộ kiểm toán ngoài đã chạy xong, nhưng có script chạy được và 181 chương trình thử tấn công — là mã/dữ liệu, không phải hướng dẫn); các sổ `*.jsonl` (bằng chứng đo đạc, công cụ ghi/đọc theo đường dẫn); `docs/DECISIONS.md`, `PROGRESS.md`, `FIELD_BUGS.md`, `KNOWN_LIMITATIONS.md` (nhật ký lõi, rất dài nhưng được nhiều công cụ/tài liệu dẫn).
