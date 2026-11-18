# apps/ — chương trình Grit RIÊNG của người dùng

Nơi đặt chương trình bạn (Giang) tự viết hoặc nhờ Claude viết bằng Grit. **Mỗi ứng dụng một gói Rig riêng** (thư mục con có `rig.toml` và `src/main.gt`), tách khỏi `examples/` (ví dụ dogfood kèm kiểm thử) và `capstone/` (bài kiểm tra có đề + bản tham chiếu).

```sh
cd apps
rig new tan_suat_tu          # tạo apps/tan_suat_tu/ (rig.toml, src/main.gt, rig.lock)
cd tan_suat_tu && rig lock && rig check && rig run
```

Luật: ở đây CHỈ có chương trình ứng dụng. Phiên Claude viết ứng dụng KHÔNG sửa compiler, test, spec hay tài liệu ngoài `apps/` và `docs/USER_BUGS.md` (xem `AGENTS.md`, `docs/START_CODING.md`). Gặp lỗi của `gritc` thì ghi `docs/USER_BUGS.md`, không "sửa cho qua".

## Ứng dụng mẫu hiện có (GÓI 6 S3; viết bởi phiên không sạch, làm hồi quy)
`todo/`, `csv2json/`, `difflite/` — mỗi cái: `README.md` (đặc tả), `reference.py` (bản tham chiếu độc lập), `cases.py` (bộ ca), `src/main.gt`. Kiểm: `python apps/check_app.py <tên>`. Nhật ký: `docs/APPS_LOG.md`.
