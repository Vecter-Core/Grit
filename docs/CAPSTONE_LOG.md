# CAPSTONE_LOG — bốn chương trình do AI MỚI viết chỉ từ tài liệu (S4, 2026-10-22)

**Phương pháp.** Với mỗi K1–K4 một subagent MỚI (cùng họ model với tác giả compiler — đây KHÔNG phải người lập trình độc lập) chỉ được đọc `docs/AI_GUIDE.md`, `docs/STD_REFERENCE.md` và đề bài (`docs/CAPSTONE.md`) và chạy `gritc`/`rig`/`capstone/check.py`; cấm đọc mã compiler, `tests/`, `docs/spec`, mã tham chiếu, ví dụ. Giới hạn nói thẳng: cấm chỉ bằng chỉ dẫn (không có sandbox kỹ thuật); nhật ký gốc nằm ở thư mục tạm của phiên (không lưu vào repo), tóm tắt dưới đây; lời giải cuối ở `capstone/solutions/`. Thí điểm của tác giả (K3, K4 viết tay trước đó để tìm khoảng trống) KHÔNG được cho subagent xem.
**Kết quả.** Bốn chương trình khớp bản tham chiếu Rust (mã thoát, stdout, stderr, tệp ghi ra) trên toàn bộ bộ ca + ca lớn, ở `dev` và `release-o2`: `python3 capstone/verify.py` ⇒ `CAPSTONE_VERIFY XANH` (8/8). Nền: Linux x86_64 (Windows: chưa kiểm).

| chương trình | số lần chạy trình biên dịch tới XANH / tới khớp hết | mã lỗi gặp | ca khớp (check.py --large) | dựng | ca lớn nhất (Grit vs Rust) |
|---|---|---|---|---|---|
| K1 csvstat | 1 (mã thật; +1 lần lãng phí do lỗi công cụ ghi tệp) / 3 | không | 29/29 | ~1,3 s | 1 triệu dòng, 23,6 MB: 0,50 s vs 0,09 s (×5,6) |
| K2 calc | 2 / 3 | E0403 (`f(st, st.pos+1)` với `st: &!St`) | 21/21 | ~1,0 s | 9 MB, 300 000 dòng: 0,50 s vs 0,06 s |
| K3 kvstore | 1 / 3 (4 tổng) | không | 20/20 | ~1,3 s | 99 000 lệnh: 0,63 s vs 0,13 s |
| K4 grep_lite | 2 / 3 | E0702 (thoát vòng bằng cờ bool) | 21/21 | ~0,8 s | 200 000 dòng 8,7 MB: 0,09 s (`-c`); in 199 817 dòng 1,7 s (stdout không đệm) |
Mục tiêu tham khảo ≤ 15 lần biên dịch: đạt (tối đa 4). Không subagent nào báo lỗi compiler (không có chương trình đúng bị từ chối, không có khác biệt thông dịch/exe).

## Phát hiện và xử lý
| # | phát hiện | xử lý |
|---|---|---|
| 1 | `rig check/build` in hàng chục `note[N06xx]` (toàn từ `std.gt`) làm chìm kết quả | văn bản cho người ẨN ghi chú `N…` (trừ N0606), `GRIT_NOTES=1`/`--json` để xem; dòng tóm tắt vẫn đếm |
| 2 | dòng `backend: interpreter` của `check`/`rig build` gây tưởng exe là thông dịch | AI_GUIDE §12 giải thích; (nhãn giữ nguyên vì JSON/test dùng) |
| 3 | E0702 khi thoát vòng bằng cờ bool; E0403 với `f(st, st.field)` | AI_GUIDE §12 (mẫu cờ đối số bằng `⊠;`, mẫu `&!Struct`, bẫy mượn hai pha); KNOWN_LIMITATIONS |
| 4 | thiếu mẫu: ghi qua lát khả biến, `i64::MIN`, `+?` làm scrutinee, `core::exit` ở vị trí biểu thức, byte→`&str` (`Text::from_utf8`), `⊐ Ord` cho struct, `Bytes: Copy+Ord+Hash`, API stderr hẹp hơn stdout, hiệu năng I/O không đệm, `main` lớn ⇒ N0605 | AI_GUIDE §12 |
| 5 | STD_REFERENCE KHÔNG có mục cho `Arena`, `Host`, `Cap<Clock/Net/Alloc>` (bộ sinh bỏ sót impl viết gọn một dòng) — lỗi THẬT của công cụ sinh tài liệu | sửa `tools/gen_docs.py` + test mới `tools/test_gen_docs.py` (đếm độc lập số hàm nguồn = số mục) |
| 6 | đề bài K3 mơ hồ ở các biên (đối số "có mặt" iff có dấu cách, `get `…) | "Làm rõ K3" trong CAPSTONE.md (bản tham chiếu là nguồn sự thật; hành vi tham chiếu KHÔNG đổi nên không làm lại bộ so) |
| 7 | I/O chậm hơn tham chiếu ~×5 ở tệp lớn (stdout không đệm; −O0 ở dev) | ghi nhận; hướng dẫn gom vào `Vec<u8>`; KHÔNG đổi runtime trong Seed (đổi hợp đồng TCB `write_*` "flush ngay") |
| 8 | thông báo E0702 trỏ cả khối vòng lặp, không chỉ đường không giảm | ghi nhận (cải thiện chẩn đoán để sau Seed; AI_GUIDE §5/§12 nêu nguyên nhân thường gặp) |
| 9 | `?` và phân rã tuple (G8/G9) | KHÔNG cần: cả bốn viết xong không có chúng ⇒ không làm trong Seed 1.0 (quyết định, không ADR) |

## Đo thêm của tác giả
Thí điểm K3/K4 viết tay (trước S4) đã tìm FB-C10/C12/C13 và khoảng trống std; sau S4 không phát hiện lỗi compiler mới từ bốn subagent.
