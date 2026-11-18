# SEED_FINISH_LINE — còn gì tới nhãn Seed và tới cổng Giai đoạn 2 (GÓI 6 S4, 2026-11-19)

Cho Giang. Trạng thái thật tại `1.0.0-rc3`; nguồn: `docs/READINESS.md` (§7 và A1–A6), `docs/PENDING_GIANG.md`. **Tôi không đánh dấu thay Giang và không đổi A2 thành đạt.**
Cột **Chặn**: **SEED** = còn thiếu thì KHÔNG được gắn `1.0.0-seed`; **GĐ2** = chỉ chặn Giai đoạn 2 (hiện bị cấm khi chưa có lệnh). "AI" = phiên Claude làm được; "GIANG" = cần người.

| # | điều kiện | trạng thái thật | ai | cách kiểm khi xong | ước tính | chặn |
|---|---|---|---|---|---|---|
| 1 | §7 #2: fuzz lexer + parser (+ typechecker) ≥ 24 giờ MỖI CÁI trên băm hiện tại | **chưa** (lexer 0,05 h, parser 0,02 h trên băm đóng băng; typechecker CHƯA có fuzz) | GIANG chạy máy rảnh; AI viết fuzz typechecker | `python tools/fuzz_total.py` (cột "GIỜ TRÊN BĂM HIỆN TẠI" ≥ 24) | 24–48 giờ máy: `bash tools/fuzz_loop.sh 26 --parallel` (≥ 10 GB bộ nhớ ảo trống; nếu không thì lần lượt ≈ 48 giờ); fuzz typechecker ≈ 1 ngày công AI | **SEED** |
| 2 | R11c: phiên Claude Opus đọc tấn công mã (TCB chưa đọc chéo ≈ 63 000 dòng) | **chờ phiên** | GIANG mở phiên; AI (Opus) đọc | `docs/TCB_CODE_AUDIT_OPUS.md` có tên model, hash, bảng nghi vấn; kiến trúc sư/Giang cập nhật TCB | 1–2 phiên dài | **SEED** (điều kiện TCB/A2) |
| 3 | Hai phiên capstone sạch (Haiku 5.5 và Sonnet 5.5, K1–K4; kiểm CÙNG họ model) | **chờ phiên** (A1 mới "đạt có điều kiện") | GIANG mở; AI viết | `docs/CAPSTONE_PROMPTS.md`; `python capstone/verify.py`; nhật ký `docs/CAPSTONE_LOG.md` | 2 phiên × vài giờ | **SEED** (A1) |
| 4 | §7 #6: Giang đọc `KNOWN_LIMITATIONS.md` và `PROGRESS.md` | **chưa** (không ai làm thay) | GIANG | `docs/GIANG_DOC6_READING.md` (hướng dẫn đọc) và Giang ghi xác nhận | ≈ 1–2 giờ | **SEED** |
| 5 | §7 #8: M13 (LSP) đủ: inlay hint (kiểu/hiệu ứng/nghĩa vụ), hover kiểu, đi tới định nghĩa biến cục bộ; VS Code thật | **một nửa** (schema JSON đã khoá; LSP có chẩn đoán/định dạng/hover thông điệp) | AI viết LSP; GIANG thử VS Code | `docs/QUICKSTART_VSCODE.md` (5 bước), test stdio LSP | LSP ≈ 2–4 ngày công AI; thử VS Code ≈ 30 phút | **SEED** (#8) / phần mở rộng LSP có thể là **GĐ2** nếu kiến trúc sư hạ phạm vi |
| 6 | Billing: số phút Actions còn lại; Spending limit = 0 USD | **chưa kiểm** | GIANG | GitHub → Settings → Billing → Usage | 5 phút | không chặn nhãn (rủi ro vận hành) |
| 7 | Đổi nhánh mặc định GitHub sang `main` | **chưa** (mặc định đang `windows-work`) | GIANG | Settings → Branches | 1 phút | không chặn |
| 8 | Cổng đầy đủ Windows qua CI trước thẻ phát hành | **chưa chạy** (CI Windows là bản rút gọn; cổng đầy đủ Windows xanh gần nhất: máy chính `89438fd`) | GIANG bấm | Actions → ci → Run workflow: `only=all`, `full=true`; mong đợi xanh (≈ 74 phút tính phí thêm) | ≈ 1,5 giờ | **SEED** (A3/A4 tái lập trên máy sạch) |
| 9 | A2 an toàn: đọc chéo khác-họ chỉ ở `gt-rt` + `gt-guard`; TSan chỉ mẫu đã chạy; ATOMIC-01 (bộ nhớ yếu/Windows SRWLOCK) chưa kiểm | **chưa** | AI (loom/TSan mở rộng) + GIANG duyệt | `docs/READINESS.md` A2; `docs/TSAN_RESULTS.md` | nhiều ngày | **SEED** (A2) |
| 10 | Độ phủ nhánh hàm quyết định: nhóm D/B/C còn lại | **đang làm** (`docs/DECISION_COVERAGE.md`) | AI | `python tools/coverage_branches.py cov.json.gz` (số nhánh chưa chạy) | vài lượt CI 55 phút | không chặn nhãn; **GĐ2** nên có |
| 11 | Đột biến M7 "xoá thêm check" còn 69 sống (#10), mutation 361/591/554 (#9) giữ "có điều kiện" | **có điều kiện** | AI (đột biến thêm) + kiến trúc sư | `tools/mut.py` sổ kết quả | nhiều ngày | A2 (SEED nếu muốn bỏ "có điều kiện"); còn lại **GĐ2** |
| 12 | A4: tái lập của chính `gritc`; di động Linux sau thay đổi Windows | **đạt có điều kiện** (exe Grit tái lập; `gritc` chưa đo); Linux CI xanh đã xác nhận di động Linux | AI | `determinism` job CI (dựng hai lần so băm) | 1 ngày công | **SEED** (A4) nếu coi là bắt buộc |
| 13 | Hàm rất lớn: `gritc check` quá chậm/giới hạn | xem `docs/RUN_REPORTS.md` GÓI 6 S2 | AI | `docs/LIMITS_AUDIT.md`, `docs/SCALE_PROBE.jsonl` | đang làm | không chặn nhãn (ghi KNOWN_LIMITATIONS) |
| 14 | Giai đoạn 2 ("Ascend", 9 hướng 8.A–8.I) | **chưa bắt đầu, cấm khi chưa có lệnh** | GIANG ra lệnh | `docs/PROJECT_STATUS.md` Phần B | — | **GĐ2** |

**Tóm tắt:** CHẶN SEED còn: #1 (fuzz), #2 (R11c), #3 (capstone), #4 (Giang đọc #6), #5 (LSP/VS Code), #8 (cổng Windows đầy đủ), #9 (A2), #12 (tái lập `gritc`), có thể #11. Không chặn nhãn: #6, #7, #10, #13. Chặn riêng GĐ2: #14.
Việc ngắn nhất Giang làm được ngay: #4 (đọc), #8 (bấm dispatch), #1 (chạy fuzz qua đêm), #6/#7 (vài phút).
