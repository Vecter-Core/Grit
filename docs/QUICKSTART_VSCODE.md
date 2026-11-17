# QUICKSTART_VSCODE — tự thử tiện ích Grit trong VS Code thật (5 bước)

**Trạng thái trung thực:** tiện ích `editors/vscode` mới được kiểm BẰNG MÁY (bản kê khai hợp lệ, tệp tồn tại — test `vscode_manifest_is_valid_and_references_existing_files`) và máy chủ `gritc lsp` có test kịch bản stdio. **CHƯA THỬ trong VS Code thật.** Năm bước dưới đây để Giang tự thử.

1. **Dựng gritc:** `cargo build --release -p gt-driver` (Git Bash; LLVM trong PATH như `docs/ENVIRONMENT.md`). Ghi lại đường dẫn `target/release/gritc.exe`.
2. **Cài phụ thuộc tiện ích:** `cd editors/vscode && npm install` (cần Node.js; tải `vscode-languageclient`).
3. **Mở tiện ích:** mở thư mục `editors/vscode` trong VS Code, bấm **F5** (Run Extension) — một cửa sổ "Extension Development Host" mở ra.
4. **Trỏ tới gritc:** trong cửa sổ mới: Settings → `grit.gritcPath` = đường dẫn bước 1 (hoặc để `gritc` nếu đã có trong PATH). Mở một tệp `.gt` (vd `tests/pass/40_copy_impl.gt`).
5. **Kiểm 4 việc:** (a) tô màu cú pháp; (b) sửa cố ý thành sai kiểu ⇒ gạch đỏ kèm mã `E…`; (c) **di chuột lên chỗ gạch đỏ** ⇒ hiện thông điệp + tài liệu `gritc explain`; (d) **F12 / Ctrl+Click** trên tên hàm ⇒ nhảy tới định nghĩa; Outline hiện danh sách item; `Shift+Alt+F` định dạng.

Nếu có gì không chạy: ghi lại thông báo trong "Output → Grit" và gửi cho agent. Giới hạn: định nghĩa chỉ cấp item (không biến cục bộ); không hoàn thành mã.
