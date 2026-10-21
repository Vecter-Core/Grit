# BÁO CÁO CHẠY (nối thêm sau mỗi mục; mới nhất ở cuối)

Quy ước: nền tảng ghi ở MỌI báo cáo cổng. Chỗ lệnh Seed 1.0 sai kỹ thuật thì ghi ở "Sai kỹ thuật của lệnh" và chọn phương án fail-closed.

## Sai kỹ thuật của lệnh (ghi nhận, đã chọn phương án fail-closed)
1. "Lô nặng ≤ 10 phút + không tiến trình nền nào sống qua một lệnh" mâu thuẫn với test dài nhất của dự án (`gt-fmt` ~915 s một bộ, `gt-borrowck` oracle ~210 s, `gt-oblig`/`gt-term` hàng trăm giây). Phương án: cổng chạy THEO ĐƠN VỊ có điểm lưu (`GATE_BUDGET_SEC`), đơn vị dài chạy bằng cơ chế nền của công cụ (có timeout, chờ xong ngay trong cùng lượt, không để tiến trình nào sống sau khi báo cáo mục).
2. "Đổi sang `proven`" bị cấm, nhưng một số đo (G09/G14 của kiểm toán) so `all` với `proven` để TÌM khác biệt: chỉ đọc, không đổi mặc định.
3. Lệnh mong `hàm io truyền vào tham số thuần ⇒ E0801`; thực tế gritc từ chối ở bước kiểu với E0301 (hiệu ứng nằm trong kiểu hàm, bất biến). ĐỎ như nhau; KHÔNG đổi mã (đổi mã = nới/đánh số lại). Mẫu corpus khoá hành vi hiện tại.
