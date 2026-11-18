# difflite — so hai tệp theo dòng (app thử đường dùng thật, GÓI 6 S3)

**Viết bởi phiên KHÔNG sạch (Claude Sonnet 5.5, sau khi đọc toàn bộ repo); dùng làm hồi quy và để thử đường `docs/START_CODING.md`, KHÔNG thay capstone.**

`difflite <tệp_a> <tệp_b>`: quy hoạch động hai chiều (LCS) trên các dòng, in kịch bản sửa.

## Đặc tả (bản tham chiếu độc lập: `reference.py`)
* Dòng tách bằng LF; dòng cuối không có LF vẫn là một dòng; tệp rỗng = 0 dòng. CR là dữ liệu. Hai dòng bằng nhau khi bằng từng byte (không xét LF cuối).
* Bảng `L[i][j]` = độ dài LCS của `a[i..]` và `b[j..]`. Đi từ (0, 0): nếu `a[i] == b[j]` in `"  " + dòng`, tiến cả hai; nếu hết `b` hoặc `L[i+1][j] >= L[i][j+1]` in `"- " + a[i]`, tiến `i`; ngược lại in `"+ " + b[j]`, tiến `j`. Mỗi dòng in kết thúc bằng LF.
* Hai tệp giống nhau (không có dòng `-`/`+`) ⇒ không in gì, thoát 0. Khác nhau ⇒ in TOÀN BỘ kịch bản (cả dòng chung), thoát 1.
* Giới hạn nghiêm ngặt: mỗi tệp ≤ 2000 dòng và `(na+1)*(nb+1) ≤ 4 000 000` ô, không thì thoát 3 (không in gì ra stdout).
* Sai cách dùng / không đọc được tệp ⇒ thoát 2.
