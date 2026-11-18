# START_CODING — bắt đầu viết chương trình Grit thật (GÓI 5 S4)

Dành cho Giang. Tự chứa, ≤ 2 trang. Trạng thái: `1.0.0-rc3` (chưa phải Seed). Mức tin cậy: đọc `docs/SAFETY_CLAIMS.md` và `docs/SEED_LIMITATIONS.md` trước khi tin kết quả.

## 1. Mở phiên Claude Code đúng chỗ
1. Mở Claude Code trên thư mục repo, nhánh `windows-work` (làm việc) — KHÔNG làm trên `main` (`main` chỉ nhận fast-forward sau cổng xanh).
2. Dựng công cụ.
   * Linux/đám mây: `bash tools/setup_cloud.sh` (idempotent; thoát 0 = xanh; đã chạy thật trên Ubuntu 24.04 ở CI).
   * Windows (Git Bash `C:\Program Files\Git\bin\bash.exe`; LLVM 20.1.8 ở `D:\Tools\llvm-20.1.8`, xem `docs/ENVIRONMENT.md`):
     ```sh
     export GRIT_LLVM_DIR='D:\Tools\llvm-20.1.8' PATH="/d/Tools/llvm-20.1.8/bin:$PATH"
     cargo build --release --manifest-path runtime/gt-rt/Cargo.toml
     cargo build --release -p gt-driver -p rig
     ```
3. Tạo gói trong `apps/` (xem `apps/README.md`): `cd apps && ../target/release/rig new ten_goi` (hoặc `rig` trên PATH).

## 2. Câu lệnh mẫu bảo Claude viết Grit
> Đọc `docs/AI_GUIDE.md` và `docs/STD_REFERENCE.md`. Viết bằng Grit chương trình **<mô tả: đầu vào, đầu ra, mã thoát>** trong `apps/<tên>/`. Chạy `gritc check` và `gritc run`, sửa theo chẩn đoán tới khi đúng. **KHÔNG sửa compiler, KHÔNG tắt hay nới kiểm tra.** Nếu `gritc` từ chối chương trình mà bạn tin là đúng, hoặc chấp nhận chương trình bạn tin là sai: **DỪNG** và ghi vào `docs/USER_BUGS.md` (mẫu trong tệp đó). Cuối cùng so kết quả với bản Rust hoặc Python tương đương trên cùng đầu vào và báo số lần biên dịch và các lỗi gặp.

Luật cho phiên viết ứng dụng ở `AGENTS.md`: chỉ sửa trong `apps/` (và ghi `docs/USER_BUGS.md`); không sửa `crates/`, `runtime/`, `tests/`, `docs/spec/`.

## 3. Ba chương trình đầu nên thử (dễ → khó) — ĐÃ có bản mẫu trong `apps/`
Đếm từ và máy tính đã nằm trong bốn capstone nên ba chương trình mẫu có hình dạng khác. Mỗi cái đã được viết, so từng byte với bản tham chiếu Python ở `dev` và `release-o2` (`docs/APPS_LOG.md`: số lần biên dịch, thông báo khó hiểu, khoảng trống). Viết bởi phiên KHÔNG sạch: dùng làm hồi quy và ví dụ, không thay capstone.
| # | chương trình | đối chiếu |
|---|---|---|
| 1 | `apps/todo` — `todo <tệp> add|done|remove|list` lưu bằng `Cap<Fs>::write` | `python apps/check_app.py todo` (110/110 ca) |
| 2 | `apps/csv2json` — CSV (có nháy, CRLF, UTF-8) → JSON mảng đối tượng | `python apps/check_app.py csv2json` (148/148) |
| 3 | `apps/difflite` — so hai tệp theo dòng bằng quy hoạch động hai chiều (giới hạn cỡ nghiêm ngặt) | `python apps/check_app.py difflite` (186/186) |
Muốn thử đường dùng với chương trình MỚI: bảo Claude viết bản riêng (đừng chép) trong `apps/<tên_mới>/` kèm `README.md` đặc tả, `reference.py` độc lập và `cases.py` (xem ba mẫu), rồi `python apps/check_app.py <tên_mới>` (cần thư mục `apps/<tên>/src/*.gt`; `rig new` tạo sẵn gói với phụ thuộc `std`).
Gặp lỗi: đọc `docs/AI_GUIDE.md` §10 và bảng lỗi (E0702: hàm quá lớn → tách hàm; E0902: hợp đồng trên lát không kiểm được lúc chạy).

## 4. Grit CHƯA làm được (trích `docs/SEED_LIMITATIONS.md`, đọc đủ ở tệp đó)
* Không mạng (kể cả `Cap<Net>::connect` chưa có lowering), không đọc stdin theo dòng (đọc toàn bộ rồi tách), không chuỗi định dạng, không in/phân tích số thực.
* Không closure, macro, `dyn`, `async`, `?`, phân rã tuple, `f32`; đa luồng chỉ mức `spawn` + `Mutex`/`Atomic` tĩnh.
* `Vec`/`HashMap` chỉ phần tử `Copy`/không cần drop; `sort` không ổn định; thứ tự duyệt `HashMap` không xác định.
* stdout không đệm; ca lớn chậm hơn Rust ~×5 ở `dev`; `spawn`/`main` dùng ngăn xếp 256 MiB.
* `check` xanh nhưng `build/run` có thể đỏ (mã 3) cho vài hàm prelude chưa có lowering.

## 5. Khi gặp lỗi
`docs/USER_BUGS.md`: "chấp nhận sai" (nghiêm trọng, nghi false negative) hay "từ chối oan" (phiền, an toàn). Đếm: `python tools/triage_user_bugs.py`.

## Kiểm các lệnh trong tệp này (ghi thật, GÓI 5)
* `bash tools/setup_cloud.sh` — ĐÃ CHẠY THẬT trên Linux CI (Ubuntu 24.04), thoát 0 (các job `linux-gate`/`tsan`).
* Windows: `cargo build --release --manifest-path runtime/gt-rt/Cargo.toml` và `cargo build --release -p gt-driver -p rig` — ĐÃ CHẠY THẬT ở các gói trước (cổng `89438fd`); tại gói này chỉ dùng bản debug.
* `rig new` / `rig lock` / `rig check` / `rig run` trong `apps/` (bản debug, Windows, 2026-11-19): ĐÃ CHẠY, đều thoát 0 (`rig check`: green, 0 lỗi); gói thử đã xoá.
* Ba chương trình mẫu `todo`, `csv2json`, `difflite` và so với bản tham chiếu Python: ĐÃ CHẠY (`python apps/check_app.py <tên>` ở Windows, dev + release-o2: 110/110, 148/148, 186/186 ca khớp).
* `gritc check` / `gritc run` trực tiếp trên một tệp: ĐÃ CHẠY (`gritc run --backend=llvm tests/tsan/narrow_droplock.gt` ⇒ "main trả về 2", thoát 0).
* Mở Claude Code trên repo và dán câu lệnh mẫu: CHƯA THỬ (cần Giang).
