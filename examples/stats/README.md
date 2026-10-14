# stats

`stats <tệp>` đọc một dãy số nguyên không âm và in số lượng, trung bình, độ lệch chuẩn (tổng thể), làm tròn thành số nguyên. Viết bằng Grit (dự án Rig), công cụ thứ sáu, dogfood `f64`.

## Quy ước (bản tham chiếu Rust trong `crates/rig/tests/dogfood_stats.rs` dùng y hệt)
- Tệp là các số THẬP PHÂN không âm (chỉ chữ số) cách nhau bởi khoảng trắng ASCII (` `, tab, `\n`, `\r`). Mỗi số ≤ 2^53 (đổi sang f64 chính xác).
- Token có ký tự lạ (dấu `-`, dấu chấm, chữ…) hoặc giá trị > 2^53 ⇒ lỗi CÓ KIỂM SOÁT: `stats: <đường-dẫn>: <lý do>` ở stderr, stdout rỗng, mã 2 (không abort).
- Thứ tự tính cố định bằng f64 IEEE-754: `tổng` = cộng lần lượt; `m = tổng / n`; `v = Σ (xᵢ − m)² / n` (cộng lần lượt); `s = √v`. Hai bên (Grit và Rust) thực hiện CÙNG các phép theo CÙNG thứ tự nên cho cùng mẫu bit.
- In đúng 3 dòng: `count <n>`, `mean <m>`, `stddev <s>`; `m`, `s` làm tròn nửa lên = `⌊x + 0,5⌋` bằng `f64::to_i64` (không cần `write_f64`). `n = 0` ⇒ `mean 0`, `stddev 0`.

## Lỗi
Thiếu đối số hoặc không mở được tệp: stderr (`usage: stats <file>` hoặc `stats: <đường-dẫn>: <lý do>`), thoát mã 2.

## Dựng và chạy
```
rig build --locked
target\stats.exe duong-dan\tep.txt
```
