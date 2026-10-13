# sortlines

`sortlines <tệp>` đọc một tệp, tách thành dòng, sắp xếp theo thứ tự BYTE (ổn định) và in ra stdout. Viết bằng Grit (dự án Rig), dogfood công cụ thứ tư.

## Quy ước (bản tham chiếu Rust trong `crates/rig/tests/dogfood_sortlines.rs` dùng y hệt)
- Chỉ tách theo byte `\n`. `\r` là MỘT byte của dòng (CRLF không được xử lý đặc biệt: `a\r` và `a` là hai dòng khác nhau).
- Mỗi dòng ra được in kèm `\n`, kể cả dòng cuối của tệp KHÔNG có `\n` cuối.
- Tệp rỗng ra rỗng; tệp chỉ có `\n` ra một dòng rỗng (`\n`).
- So sánh từ điển theo byte (không theo locale; byte cao ≥ 0x80 lớn hơn ASCII); tiền tố ngắn hơn đứng trước.
- Sắp xếp trộn ổn định (không quan sát được khi so toàn dòng).

## Lỗi
Thiếu đối số hoặc không mở được tệp: một dòng lên stderr (`usage: sortlines <file>` hoặc `sortlines: <đường-dẫn>: <lý do>`), thoát mã 2.

## Bộ nhớ
Hai `Vec<Line>` (danh sách dòng và bộ đệm trộn) trên hai Arena, mỗi Vec `reserve(số dòng)` một lần: mỗi Arena cần đúng một khối `max(1 MiB, số_dòng·16 + 8)` byte; số dòng ≤ độ_dài_tệp + 1. Tệp toàn dòng rỗng là xấu nhất: ≈ 2 · 16 · độ_dài_tệp byte (32 MiB toàn dòng rỗng ≈ 1 GiB). Phép nhân kích thước có kiểm tràn (abort `overflow`); quá 64 GiB mỗi Vec thì từ chối bằng thông báo (mã 2).

## Dựng và chạy
```
rig build --locked
target\sortlines.exe duong-dan\tep.txt
```
