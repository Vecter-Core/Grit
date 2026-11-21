# GIANG_DOC6_READING — danh sách đọc rút gọn cho điều kiện #6 (GÓI 3 S4.d, 2026-11-18)

**Điều kiện #6 (`GRIT_PLAN.md` §7):** "`KNOWN_LIMITATIONS.md` và `HISTORY.md` được xem xét bởi Giang". Hai tệp dài (≈ 100 KB / 166 KB). **Đây là danh sách ĐỀ MỤC để tìm** (theo tên đề mục, không theo số dòng vì sẽ trôi: dùng Ctrl+F). Mỗi mục: 3 dòng tóm tắt nói điều bạn đang chấp nhận. Mục lục đầy đủ hơn: `docs/KNOWN_LIMITATIONS.md`. Đọc khoảng 60–90 phút. **#6 vẫn CHỜ GIANG** (tài liệu này chưa đánh dấu gì).

## A. `docs/KNOWN_LIMITATIONS.md` — các mục RỦI RO (đọc kỹ)
1. **"ĐIỀU KIỆN CỨNG: lỗ hổng tạm thời của M4/M5"** — Trước khi có M7, số vượt phạm vi (`x: u8 ≔ 300`) lọt qua M4. Điều kiện: `gritc` không sinh binary nếu E0601 (M7) và E0307 (M5) chưa chạy. Cả hai đã chạy và qua test (READINESS #11).
2. **"Đột biến chưa giải quyết, coi như false negative tiềm ẩn (M6 …)"** — 3 đột biến (361/591/554) của `regions.rs` sống sót; chấp nhận bằng lập luận đơn điệu + đo 24 143 chương trình (0 vi phạm). *Bạn đang chấp nhận:* bằng chứng là đo + đọc mã, không phải chứng minh hình thức (xem B1).
3. **"ĐIỀU KIỆN CỨNG: mutability bên trong"** và **"Điều kiện cứng mutability bên trong: TRẠNG THÁI"** — `Cell`/`Mutex`/`Atomic` đưa vào ngôn ngữ sau khi mở lại các đột biến; đột biến 238 đã giết, kịch bản `interior_mut` trong bộ sinh.
4. **"Những gì oracle không quan sát được (M7 …)"** — Oracle M7 mù với một số loại sai (liệt kê ngay đầu mục). *Bạn đang chấp nhận:* những chỗ đó chỉ được bảo vệ bằng đọc mã và đột biến.
5. **"Đột biến chưa giải quyết của gt-oblig, coi như false negative tiềm ẩn (M7 …)"** — 191 đột biến nhóm nguy hiểm ("xoá thêm check"): 30 giết bằng test, 30 chỉ có lập luận. Chế độ mặc định giữ mọi check nên rủi ro chỉ khi gõ `--checks=proven` (xem B2).
6. **"M8 'ĐẠT CÓ ĐIỀU KIỆN' …"** — Quy tắc bất biến cho Mutex/Atomic chưa chứng minh là *cần* bằng kiểm "tắt bản sửa ⇒ 0 âm tính giả". Không có ThreadSanitizer/loom (xem `TSAN_FEASIBILITY.md`, B4).
7. **"Lỗ hổng âm tính giả tìm thấy MUỘN"** — Danh sách lỗi false-negative phát hiện sau khi mốc đã "đạt". Cho thấy khả năng còn lỗ chưa tìm là có thật.
8. **"Kiểm động Arena `arena_overlap`"** — Chỉ hồ sơ `dev` có kiểm chồng lấn Arena (bằng chứng, không phải chứng minh); release/release-o2 không kiểm.
9. **"FAIL_OPEN_AUDIT — giới hạn còn lại sau R3"** — Công cụ quét mẫu "im lặng chấp nhận": mọi chỗ khớp đã được đọc và phân loại; không phải chứng minh vắng fail-open.
10. **"Sự cố ghi NUL khi phiên bị ngắt — nguyên nhân CHƯA rõ"** — Tệp bị ghi byte NUL khi phiên công cụ bị ngắt; có kiểm NUL trong cổng và preflight, nguyên nhân gốc chưa biết.
11. **"Nền tảng và tài nguyên"** và **"Seed 1.0 — giới hạn của bề mặt ngôn ngữ/thư viện"** — Nền tảng đã kiểm, chưa kiểm; chỗ bề mặt ngôn ngữ còn thiếu và cách né.
12. **"Ghi nhận từ đọc mã đối kháng (Seed 1.0 S7 …)"** và **"GÓI 3 (2026-11-18): `spawn` và tài nguyên, stack, Guard, `unsafe`"** — các giới hạn mới nhất: 256 MiB mỗi luồng, GUARD-02, kiểm kê `unsafe`.

## B. `docs/HISTORY.md` — chỉ đọc dòng tiêu đề và trạng thái của từng mốc
Tìm các tiêu đề bắt đầu `## M0`, `## M1` … `## M13`, `## M6-close`, `## M7-close`, `## M8 đóng`, `## M10`, `## M11` và đọc dòng trạng thái ngay sau (HOÀN THÀNH / ĐẠT CÓ ĐIỀU KIỆN / chờ). Mốc "đạt có điều kiện": M6, M7, M8, M10. Không cần đọc số liệu chi tiết.

## C. Hai tài liệu nói thẳng với người dùng
`docs/SAFETY_CLAIMS.md` (lời hứa an toàn của Grit) và `docs/SEED_LIMITATIONS.md` (không làm được gì). Đây là cái người ngoài sẽ đọc; nếu bạn thấy lời nào hứa quá tay, báo.

---
# DỰ THẢO ADR — "điều kiện #6 hiểu là Giang đọc các mục rủi ro trong `GIANG_DOC6_READING.md`"
**Trạng thái: DỰ THẢO, CHƯA CÓ HIỆU LỰC.** Chỉ có hiệu lực khi Giang xác nhận (một tin nhắn) rằng **đã đọc** các mục A.1–A.12 và các tiêu đề ở B, C. Agent KHÔNG tự đánh dấu #6.
- **Bối cảnh.** `GRIT_PLAN.md` §7 điểm 6 đòi Giang "xem xét" hai tệp ≈ 266 KB. Mục đích của điều kiện là chủ dự án biết rủi ro nào đang được chấp nhận, không phải đọc từng dòng.
- **Quyết định (đề xuất).** Điều kiện #6 được coi là ĐẠT khi Giang xác nhận đã đọc: (A) 12 mục rủi ro ở trên, (B) trạng thái từng mốc ở `HISTORY.md`, (C) `SAFETY_CLAIMS.md` và `SEED_LIMITATIONS.md`. Đây là ĐỊNH NGHĨA LẠI phạm vi đọc của điều kiện do chính Giang quyết, không phải nới tiêu chuẩn của người khác.
- **Hệ quả.** Mục nào của `KNOWN_LIMITATIONS.md` thêm sau ngày xác nhận không được tính là đã đọc; khi Giang thêm hoặc thấy rủi ro mới, danh sách này được cập nhật và #6 mở lại cho phần mới.
- **Khi hiệu lực:** ghi vào `docs/STATUS.md` dòng #6 "đạt (Giang xác nhận <ngày>, theo ADR-C10)" và đánh số ADR vào `docs/DECISIONS.md`.
