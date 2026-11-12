# editors/vscode — khung tiện ích VS Code cho Grit (KHUNG SƠ KHAI)

Gồm: tô màu cú pháp TextMate (`syntaxes/grit.tmLanguage.json`), cấu hình ngôn ngữ, và khách LSP tối thiểu chạy `gritc lsp` (chẩn đoán khi mở/sửa/lưu, định dạng tài liệu).
**Chưa đóng gói, chưa thử trong VS Code thật** (môi trường soạn thảo là container không có VS Code). Máy chủ LSP (`gritc lsp`) CÓ test tự động (`crates/gt-driver/tests/lsp.rs`).
Dùng thử: `npm install` trong thư mục này, mở thư mục bằng VS Code, F5 (Extension Development Host), đặt `grit.gritcPath` nếu `gritc` không có trong PATH.
