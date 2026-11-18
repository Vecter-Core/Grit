# csv2json — CSV → JSON (app de thử đường dùng thật, GÓI 6 S3)

**Viết bởi phiên KHÔNG sạch (Claude Sonnet 5.5, sau khi đọc toàn bộ repo); dùng làm hồi quy và để thử đường `docs/START_CODING.md`, KHÔNG thay capstone.**

`csv2json <tệp.csv>` in một mảng JSON các đối tượng ra stdout (một đối tượng mỗi dòng; khoá = hàng đầu).

## Đặc tả (bản tham chiếu độc lập: `reference.py`)
* Bản ghi kết thúc bằng LF hoặc CRLF ngoài nháy; CR đứng lẻ là dữ liệu. Bản ghi cuối có thể thiếu xuống dòng. **Dòng rỗng (không byte) bị bỏ qua.**
* Trường bắt đầu bằng `"` là trường có nháy: nội dung tới nháy đóng, `""` là một nháy. Sau nháy đóng chỉ được là `,`, hết bản ghi hoặc hết tệp; không thì lỗi.
* Trường không nháy chạy tới `,` hoặc hết bản ghi; có `"` trong trường không nháy ⇒ lỗi. Nháy không đóng ⇒ lỗi.
* Hàng đầu (sau khi bỏ dòng rỗng) là tiêu đề. Mỗi bản ghi dữ liệu phải có ĐÚNG số trường của tiêu đề, không thì lỗi. Không có bản ghi dữ liệu ⇒ in `[]`.
* Thoát JSON: `"`→`\"`, `\`→`\`, LF→`\n`, CR→`\r`, TAB→`\t`, các byte < 0x20 khác → `\u00xx` (hex thường). Mọi thứ khác (kể cả 0x7F và UTF-8 nhiều byte) giữ nguyên.
* Đầu vào PHẢI là UTF-8 hợp lệ (kiểm cả tệp trước khi in).
* Mã thoát: 0 xong; 2 sai cách dùng / không đọc được tệp / không phải UTF-8; 3 lỗi CSV (không in gì ra stdout).
* Định dạng: `[` LF, các đối tượng `{"k":"v",...}` cách nhau `,` LF, rồi LF `]` LF (đúng từng byte; xem `reference.py`).
