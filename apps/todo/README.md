# todo — việc cần làm trên tệp (app thử đường dùng thật, GÓI 6 S3)

**Viết bởi phiên KHÔNG sạch (Claude Sonnet 5.5, sau khi đọc toàn bộ repo); dùng làm hồi quy và để thử đường `docs/START_CODING.md`, KHÔNG thay capstone.**

`todo <tệp> add <văn bản> | done <n> | remove <n> | list`. Lưu bằng `Cap<Fs>::write` (ghi nguyên tệp).

## Đặc tả (bản tham chiếu độc lập: `reference.py`)
* Tệp dữ liệu: mỗi mục một dòng LF `[ ] nội dung` (chưa xong) hoặc `[x] nội dung` (xong); nội dung KHÔNG rỗng, không chứa LF. Tệp chưa có = danh sách rỗng. Tệp không đúng định dạng (dòng không bắt đầu bằng `[ ] `/`[x] ` + ≥ 1 byte, dòng rỗng, thiếu LF cuối) ⇒ thoát 4, không đổi tệp.
* `list`: in `<số thứ tự từ 1> <dòng>` mỗi mục. Danh sách rỗng ⇒ không in gì. `add <văn bản>`: thêm `[ ] văn bản`, in `added <số mục mới>`; văn bản rỗng hoặc chứa LF ⇒ thoát 2. `done <n>`: đổi `[ ]` thành `[x]` (làm lại được), in `done <n>` (n in nguyên văn đối số). `remove <n>`: xoá mục n, các mục sau đổi số, in `removed <n>`.
* `n` là số thập phân không dấu (`u64`); sai cú pháp/tràn u64 ⇒ thoát 2; `0` hoặc lớn hơn số mục ⇒ thoát 3.
* Sai cách dùng (thiếu/thừa đối số, lệnh lạ) ⇒ thoát 2. Không đọc/ghi được tệp ⇒ thoát 2. Lệnh lỗi không đổi tệp.
