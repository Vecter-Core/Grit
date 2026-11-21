# GIANG_DECISIONS_2026-11-18 — quyết định đã ỦY QUYỀN cho kiến trúc sư/agent

**Nguồn:** phiếu `Giang quyết.md` (kiến trúc sư Claude Opus 5.5, 2026-11-18) + tin nhắn "tiếp tục đi" của Giang (2026-11-18) kèm lệnh GÓI 3.
**Tính chất:** đây là **ỦY QUYỀN** thực hiện các lựa chọn của kiến trúc sư, **KHÔNG phải chữ ký riêng từng mục**. Giang đảo được bất kỳ mục nào bằng một tin nhắn. Agent KHÔNG đánh dấu thay Giang các mục Giang tự làm (C1–C3, #6).

## Loại A — chỉ đổi nhãn (đã tự kiểm bằng chứng tồn tại trong kho, 2026-11-18)
| # | Điểm | Kết quả | Bằng chứng tìm thấy |
|---|---|---|---|
| A1 | Q23 | ĐÃ DUYỆT 2026-10-06 — nhãn sửa | `HISTORY.md:171` "M4 HOÀN THÀNH (Giang duyệt Q23 và 4 điểm sai khác, 2026-10-06)" |
| A2 | Q34 (ADR-022) | ĐÃ DUYỆT 2026-10-06 — nhãn sửa | `DECISIONS.md:341` tiêu đề "Giang DUYỆT 2026-10-06, Q34" |
| A3 | Q35–Q42 (ADR-025) | ĐÃ DUYỆT 2026-10-07 — nhãn sửa | `DECISIONS.md:586` "Duyệt của Giang (2026-10-07)" |
| A4 | Q79–Q82 | ĐÃ DUYỆT 2026-10-14 — nhãn sửa ở READINESS, BENCHMARK | `OPEN_QUESTIONS.md:53–56` |
| A5 | ADR-038 / Q52 | duyệt theo từng bước (Q83–Q86, Q85 hoãn); Q52 cho `spawn` thuần — nhãn sửa | `STATUS.md:93`; `KNOWN_LIMITATIONS.md:190` |
Không có bằng chứng nào bị thiếu ⇒ không mục nào phải giữ "CHỜ GIANG" vì lý do này. Lưu ý: bằng chứng cho Q83–Q86 chỉ là dòng "Đã QUYẾT" trong HANDOFF (không có ADR ghi chi tiết từng Q); Giang nên liếc lại nếu muốn.

## Loại B — quyết định (ỦY QUYỀN)
- **B1 #9 (361/591/554):** chấp nhận có điều kiện, chốt. Mở lại khi thêm luật ĐỎ mới của M6 (cập nhật `MUTANT_ARGUMENTS` A.1 và chạy lại `probe_diff`). `probe_diff` bản ĐỦ (`GT_PROBE_N=20000`) chạy MỘT lần trước nhãn Seed; `GT_PROBE_N=4000` cho các lần đóng milestone khác (số nhỏ giảm khả năng phát hiện).
- **B2 #10 (30 đột biến M7 chỉ có lập luận):** chấp nhận có điều kiện. Ràng buộc cứng: `--checks=proven` giữ THỬ NGHIỆM, bị từ chối cùng `release*`, KHÔNG làm mặc định, tới khi 30 đột biến bị giết hoặc có chứng minh.
- **B3 đọc chéo khác họ:** ghi "đạt có điều kiện cho intrinsic/runtime/Guard" CHỈ sau khi làm S3.a. KHÔNG nâng thành "TCB đã kiểm toán" (checker M6/M7/M8/kiểu chưa được đọc chéo).
- **B4 H7/H8 không có ThreadSanitizer/loom:** chấp nhận rủi ro có ý thức cho Seed; SAFETY_CLAIMS: "an toàn theo kiểu; chưa có kiểm động đua dữ liệu". TSan chỉ ở mức đánh giá khả thi (`TSAN_FEASIBILITY.md`).
- **B5 ADR-C09 (HashMap):** duyệt (a).
- **B6 #6:** hiểu là Giang đọc các mục rủi ro trong danh sách rút gọn (`GIANG_DOC6_READING.md`); vẫn **CHỜ GIANG**.
- **B7 hợp nhất `main` — ĐÓNG: main = e450c69, đẩy fast-forward 1 lần (GÓI 5).** Điều kiện gốc: chỉ khi (1) repo đẩy được, (2) CI GitHub chạy xanh một lần trên Linux và Windows, (3) loại A đã sửa nhãn; qua PR, KHÔNG squash.
- **B8 M13:** nhãn Seed trước; inlay hint chỉ khi Giang thử VS Code thật và thấy cần. KHÔNG sửa điều kiện #8.
- **B9 phiên bản:** đổi lên `1.0.0-rc3` trong MỘT commit riêng ở cuối.
## Loại C — việc của Giang
C1 sao lưu ngoài ổ/đẩy repo (một phần xong: xem PENDING_GIANG); C2 thử VS Code 10 phút; C3 mở phiên sạch viết capstone.

## Ủy quyền GÓI 10 (2026-11-20) — điểm "GIAI ĐOẠN 2" của PENDING_GIANG
Giang đã nhắn "tiếp tục" ⇒ ủy quyền kiến trúc sư quyết 5 điểm sau (THU HỒI ĐƯỢC bất cứ lúc nào; agent KHÔNG ghi thay Giang ở mục khác):
1. Cấu trúc Giai đoạn 2 = bốn dòng việc A (Bảo đảm), B (Độc lập backend), C (Dùng được cho AI), D (Hiệu năng); giai đoạn con 2.0–2.4 (ADR-P2-00).
2. Quy tắc gom: mọi cú pháp mới vào MỘT đợt "edition", sau N8; lexer/parser đóng băng lại; fuzz 24 giờ MỘT lần (ADR-P2-01).
3. Thứ tự: an toàn trước hiệu năng.
4. Hai đường hạ ở L1 (`MIR → LLVM` và `GLIR → LLVM`) tới khi trùng trên cổng liền mạch xanh (ADR-P2-02).
5. Đổi mặc định dev: CHƯA; quyết sau.
**Không có mã Giai đoạn 2** trước khi nhãn Seed đạt hoặc đạt có điều kiện được Giang ký.
Cùng gói: ADR-C12 = phương án (b) cho tài liệu (không sửa prelude); ADR-C13 (declassify) HOÃN; N9 (nhãn đối số) HOÃN tới ngưỡng ≥ 3 lỗi đảo đối số cùng kiểu đã XÁC NHẬN trong ≥ 2 phiên capstone độc lập (Haiku 5.5 và Sonnet 5.5); chưa đạt thì KHÔNG làm.
