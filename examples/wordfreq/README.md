# wordfreq

`wordfreq <tệp>` đếm tần suất TỪ trong một tệp UTF-8 và in 10 từ đứng đầu. Viết bằng Grit (dự án Rig), công cụ thứ năm, dogfood `Text`.

## Quy ước (bản tham chiếu Rust trong `crates/rig/tests/dogfood_wordfreq.rs` dùng y hệt)
- Tệp PHẢI là UTF-8 hợp lệ (RFC 3629). Sai ⇒ lỗi CÓ KIỂM SOÁT: `wordfreq: <đường-dẫn>: invalid UTF-8` ở stderr, stdout rỗng, thoát mã 2 (không abort).
- **Từ** = dãy cực đại các byte thuộc `[A-Za-z0-9_]` hoặc ≥ 0x80. Mọi byte ASCII khác (khoảng trắng, dấu câu, `\r`, NUL…) là dấu phân cách. Ký tự không ASCII luôn thuộc từ ⇒ điểm cắt luôn là byte ASCII ⇒ mỗi từ là UTF-8 hợp lệ.
- Phân biệt hoa/thường; so sánh từ theo BYTE (không theo locale).
- In tối đa 10 dòng `<số lần> <từ>`, xếp theo số lần GIẢM dần rồi theo thứ tự byte TĂNG dần. Tệp không có từ ⇒ không in gì.
- Không có bảng băm: sắp xếp các từ rồi đếm đoạn bằng nhau (sắp xếp trộn ổn định `msort.gt`, BẢN SAO của `examples/sortlines/src/msort.gt`; test khoá hai bản giống nhau).

## Lỗi
Thiếu đối số hoặc không mở được tệp: stderr (`usage: wordfreq <file>` hoặc `wordfreq: <đường-dẫn>: <lý do>`), thoát mã 2.

## Bộ nhớ
6 Arena (một Arena một Vec/Text): Text nguồn (≈ độ dài tệp, vì `Text::from_utf8` sao chép tệp đã kiểm), hai Vec từ (16 B/từ), hai Vec cặp (24 B/cặp), Text đầu ra. Đo: tệp 32 MiB ⇒ đỉnh working set ≈ 234 MiB (≈ 7×).

## Dựng và chạy
```
rig build --locked
target\wordfreq.exe duong-dan\tep.txt
```
