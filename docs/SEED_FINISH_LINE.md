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

## Cập nhật GÓI 7 (2026-11-19)
* **Fuzz trên CI Linux** (điều kiện #1; việc chặn Seed lớn nhất): workflow `.github/workflows/fuzz.yml` (CHỈ bấm tay). Cách bấm (cần Actions mở): (a) thử đường đi: `gh workflow run fuzz.yml --ref windows-work -f minutes=10`; (b) chạy thật: `gh workflow run fuzz.yml --ref windows-work -f minutes=330` — mỗi lần ≈ 5,5 giờ cho CẢ HAI mục tiêu song song; cần ≈ 5 lần (không chạy chồng; `concurrency` không huỷ) để đủ 24 giờ mỗi cái; (c) sau mỗi lần: `gh run download <run-id> -D /tmp/fz` rồi `python tools/fuzz_ingest.py /tmp/fz` (chống ghi trùng) rồi `python tools/fuzz_total.py` (in giờ đồng hồ treo tường theo nguồn trên băm hiện tại). Trước đó sửa lexer/parser làm băm đổi ⇒ mất giờ cũ: đừng sửa gt-lexer/gt-parser.
* **Hàng Billing và Actions: CHƯA bỏ** — Actions vẫn bị khoá (HTTP 422); kho nay công khai (phút miễn phí) nên Billing không còn là rào, còn Actions bị khoá thì chưa có CI.
* **Mới (GÓI 7):** cổng đầy đủ rc5 liền mạch (Linux và Windows) CHƯA có: cần máy rảnh bộ nhớ ảo ≥ 4 GB.
* **Mới:** kho công khai ⇒ Giang xử lý mục KHẨN trong `docs/PENDING_GIANG.md` (fixture token, giấy phép, cài đặt GitHub).

## Cập nhật GÓI 8 (2026-11-20)
* **Cổng đầy đủ rc6:** Windows XANH (`16c37f0`); Linux CHƯA: Giang chạy phiên đám mây theo `docs/CLOUD_GATE_PROMPT.md` (≈ 46–74 phút + dựng) và dán vào `docs/CLOUD_GATE_RESULT.md`; cần CẢ HAI trên cùng cây mã mới đẩy `main`.
* **Phiên tấn công (hộp đen):** `bash tools/make_attack_kit.sh <thư_mục>` tạo kit (không mã nguồn) + `docs/E_PROMPT_ATTACK.md` (mỗi phiên MỘT chủ đề); chạy trên thư mục riêng, KHÔNG ghi vào kho; kết quả nhập bằng `tools/import_attack.py`.
* **Fuzz qua đêm:** cần đóng WSL (`wsl --shutdown`) và trình duyệt để bộ nhớ ảo ≥ 4 GB (preflight); chạy trong clone riêng nếu phiên khác ghi vào kho.
* **Giấy phép / kiểm fixture bí mật:** `docs/PENDING_GIANG.md` (mục KIỂM FIXTURE BÍ MẬT ở đầu tệp; giấy phép; email `gamil`).
* **Actions:** vẫn khoá (HTTP 422, thử 1 lần trong gói).
* **Quyết định kiến trúc chờ Giang:** ADR-C12 (cho phép `{detach}` ở mã người dùng rồi siết, hoặc đồng bộ prelude/TCB theo hành vi hiện tại), ADR-C13 (declassify không Cap).

## Cập nhật GÓI 9 (2026-11-20, chỉ tài liệu)
* Giai đoạn 2 (#14) vẫn "cấm khi chưa có lệnh". Đã có kế hoạch ĐỀ XUẤT `docs/PHASE2_PLAN.md` (điều kiện vào ở mục 1) và đối chiếu mã `docs/PHASE2_FACTCHECK.md`.
* **Đợt đo ở `PHASE2_FACTCHECK.md` mục 10 chạy SAU cổng rc6** (một Cargo tại một thời điểm; preflight PASS trước; không chạy khi cổng đang chạy).

## Cập nhật GÓI 10 (2026-11-20; chỉ tài liệu/công cụ/test)
* **CHẶN bởi GitHub:** CI mọi job; `main`; fuzz trên CI Linux; cổng Linux qua đám mây (`CLOUD_GATE_PROMPT.md`).
* **Làm được CỤC BỘ (chưa làm xong):** cổng Linux bằng WSL2 Ubuntu (CẦN Giang cài WSL Ubuntu, xem PENDING_GIANG; và bộ nhớ ảo ≥ 4 GB); fuzz (`tools/fuzz_*`); phiên Opus đọc mã (R11c) trong clone riêng; hai phiên capstone sạch; thử VS Code; điều kiện #6 (Giang đọc); giấy phép (GPL-3.0-only đã đặt ở kho mới `Vecter-Core/Grit`).
* Bản sao ngoài máy: `D:\Grit_export` (Giang chép ra ngoài ổ). Đo Giai đoạn 2: CHƯA ĐO (preflight đỏ).
