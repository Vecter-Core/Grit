# APPS_LOG — ba chương trình thử đường dùng thật (GÓI 6 S3, 2026-11-19)

**Viết bởi phiên KHÔNG sạch (Claude Sonnet 5.5, đã đọc gần hết repo). Dùng làm hồi quy và để thử đường `docs/START_CODING.md`; KHÔNG thay capstone** (capstone cần phiên sạch, `docs/CAPSTONE_PROMPTS.md`).
Mã: `apps/csv2json`, `apps/todo`, `apps/difflite` (mỗi cái một gói Rig, có `README.md` đặc tả, `reference.py` bản tham chiếu ĐỘC LẬP viết bằng Python từ cùng đặc tả, `cases.py` bộ ca). So sánh: `python apps/check_app.py <tên>` (dựng ở `dev` và `release-o2`, so mã thoát + stdout từng byte, và tệp cuối cho `todo`).

## Kết quả đo (thật)
| ứng dụng | dòng Grit | số lần `rig check` tới xanh | ca (× 2 hồ sơ) | khớp |
|---|---:|---:|---:|---|
| csv2json | ≈ 240 | 7 | 74 | 148/148 |
| todo | ≈ 130 | 2 | 55 | 110/110 |
| difflite | ≈ 170 | 4 | 93 | 186/186 |
Bộ ca: rỗng, biên (giới hạn 2000×2000 của difflite, 3000 dòng CSV, 400 mục todo), hỏng (nháy sai, tệp hỏng, UTF-8 hỏng/cắt giữa, không có tệp, thiếu/thừa đối số), ký tự UTF-8 và byte lạ (NUL, 0x7F, 0xFF), ngẫu nhiên hạt cố định (40 + 25 + 60). KHÔNG có ca nào lệch giữa Grit và bản tham chiếu ở bản cuối; hai lệch đầu tiên là lỗi của chính bản tham chiếu/ca (ghi dưới).

## csv2json
Mã lỗi đã gặp theo thứ tự: (1) E0309 "không có phương thức `push_str`" (tôi tự viết `rig.toml` thiếu phụ thuộc `std`: `Vec<u8>::push_str` là của gói `std`); (2) 5 × E0702 (biến thể vòng lặp: thân dùng cờ bool `bad`/`rec` nên mỗi vòng không chắc giảm); (3) 2 × E0101 "chuỗi không được xuống dòng thô" (tôi gõ `\n` trong chuỗi qua heredoc); (4) E0902 + 2 × E0702 (hàm có `⊲` nhận lát/`Vec` không Copy ⇒ "không gọi được hợp đồng lúc chạy"; vòng ngoài vẫn dùng cờ); (5) E0302 `usize` ≠ `u64` ở `core::fuel_left`; (6) E0702 do một bản sửa của tôi không được áp dụng; (7) xanh.
Cách sửa thật: viết lại thành ba hàm `plain_end`/`quote_end`/`copy_unquoted` mỗi hàm một vòng chỉ tiến, vòng ngoài dùng nhiên liệu `core::fuel_left`; bỏ `⊲` ở hàm nhận lát. Bản đầu viết kiểu "cờ + vòng lồng" như ngôn ngữ khác: bị từ chối 5 lần.
Lệch ở so sánh lần đầu: 10 ca lệch — **cả 10 do bản tham chiếu Python** (heredoc làm mất một nửa dấu `\`, nên bản tham chiếu in LF/CR thô thay vì `\n`/`\r`); sửa bản tham chiếu bằng `bytes([0x5C])`, 148/148.
## todo
2 lần `check`: (1) E0301 "`==`/`≠` chưa hỗ trợ cho kiểu `Option<usize>`" (tôi viết `!= None`; thông báo và `AI_GUIDE.md` có chỉ dẫn: dùng `⌥`); (2) xanh. Ghi tệp bằng `Cap<Fs>::write` (ghi nguyên tệp) chạy đúng ở cả hai hồ sơ; ca dãy lệnh có trạng thái tệp khớp tham chiếu.
## difflite
4 lần: (1) 2 × E0702 (hàm `main` có hơn 24 biến ⇒ quan hệ giữa biến bị bỏ, N0605 ẩn; vòng `⟳ z < total ↓ total - z` đúng nhưng bị từ chối); (2) E0301 cần `&[u32]` gặp `&![u32]` (không tự hạ `&!` xuống `&`: dùng `tv.as_slice()`); (3) E0702 (hàm 13 tham số + 12 biến vượt 24) và E0902 (tiền điều kiện `⊳ as_.len() == ae.len()` với đối số lát); (4) xanh. Sửa: tách `fill_zero`/`build_table`/`walk`, nhiên liệu thay biến thể quan hệ.

## Phát hiện và xử lý
| # | phát hiện | loại | xử lý |
|---|---|---|---|
| 1 | E0702 khi hàm có > 24 biến/vòng lặp (kể cả vòng ĐÚNG); thông báo chỉ nói "biến thể không giảm", N0605 bị ẩn ⇒ dễ tưởng chương trình sai | **thông báo khó hiểu** (từ chối oan, fail-closed; có giới hạn trong `docs/LIMITS_AUDIT.md`) | cải thiện chẩn đoán: thêm lời nhắc nguyên nhân + gợi ý tách hàm (`gt-term`, test ĐỎ trước `crates/gt-term/tests/limit_hint.rs`, commit riêng); AI_GUIDE bảng lỗi E0702 |
| 2 | E0902 "không gọi được hợp đồng lúc chạy" khi hàm có `⊳`/`⊲` nhận lát/`Vec` | tài liệu thiếu (bảng lỗi chỉ nói "tránh `⊳` trên tham số `&!`") | AI_GUIDE bảng lỗi E0902 bổ sung |
| 3 | `rig.toml` tự tạo thiếu `std` ⇒ E0309 không nhắc tới gói std | thông báo khó hiểu nhỏ | AI_GUIDE đã ghi dùng `rig new`; CHƯA đổi chẩn đoán (ghi ở đây) |
| 4 | Cờ bool làm biến thể không giảm; `⊠;` (thoát sớm) và `fuel` có trong AI_GUIDE (§ ~284) nhưng tôi không đọc kỹ — mất 5 lần thử | tài liệu có nhưng khó thấy | bảng lỗi E0702 trỏ tới `⊠;`; **khuyến nghị**: đưa mục này lên đầu §Vòng lặp |
| 5 | `&![T]` không tự hạ thành `&[T]` ở đối số (E0301) | giới hạn ngôn ngữ nhỏ | ghi ở đây; chưa đổi |
| 6 | Hàm lớn: `main` có nhiều biến buộc tách hàm | khoảng trống thư viện/ngôn ngữ (không có struct chứa nhiều Vec dễ truyền) | chỉ ghi |
| — | Lỗi compiler (chấp nhận sai / sinh mã sai) | — | **KHÔNG có** ở ba chương trình: 0 FB-C, 0 USER_BUGS "chấp nhận sai" |

Khoảng trống thư viện gặp (chỉ ghi, không làm trong gói này): không có `HashMap` tiện cho chuỗi (đã có nhưng cần `Bytes`), không có hàm chia chuỗi theo ký tự phân tách, không có `Vec<(usize,usize)>` dễ dùng (dùng hai `Vec<usize>`), không có `break` (dùng `⊠;`/nhiên liệu). Không cái nào chặn cả ba chương trình.
`docs/USER_BUGS.md`: không thêm mục (không có chương trình đúng bị từ chối oan ngoài các trường hợp giới hạn đã biết ở mục 1; đó là thiết kế fail-closed có tài liệu).
