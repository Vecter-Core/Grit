# CAPSTONE_PROMPTS — phiên viết sạch (G2.7)

## Hướng dẫn cho Giang (GÓI 3 S4.a, 2026-11-18)
Mở **hai phiên sạch CỦA CLAUDE, hai MODEL KHÁC NHAU**: một yếu hơn (**Claude Haiku 5.5**) và một mạnh hơn (**Claude Sonnet 5.5**), cùng một K (ví dụ K1 rồi K3; mỗi K một phiên mới). Mỗi phiên **chỉ** được đọc `docs/AI_GUIDE.md`, `docs/STD_REFERENCE.md` và đề (`docs/CAPSTONE.md` + K trong tệp này). Lý do: Grit viết cho AI; model YẾU viết đúng được chỉ từ tài liệu là bằng chứng tốt hơn cho chất lượng tài liệu; nếu chỉ một họ model viết được thì bằng chứng yếu vì tài liệu do cùng họ viết. Cuối mỗi phiên, bắt model điền `docs/CAPSTONE_LOG_TEMPLATE.md` (đặc biệt **DANH SÁCH MỌI TỆP đã mở**: để kiểm phiên có sạch không) rồi bạn chạy `bash tools/capstone_check.sh <src/main.gt> <K>` (đã kiểm lại bằng chương trình fib trên Windows ngày 2026-11-18, xem `RUN_REPORTS`). Phiên nào mở tệp ngoài danh sách cho phép thì kết quả KHÔNG tính là "sạch".

## Dùng trong phiên mới

Chọn đúng một đề K1–K4 từ `docs/CAPSTONE.md`. Chỉ đọc:

1. `docs/AI_GUIDE.md`;
2. `docs/STD_REFERENCE.md`;
3. `docs/CAPSTONE.md` và phần đề được giao bên dưới.

Không đọc mã compiler (`crates/`, `runtime/`), `tests/`, `docs/spec/`, `capstone/reference/`, `capstone/solutions/`, ví dụ, hoặc nhật ký lời giải cũ. Không tìm/chạy lời giải đã có. Ràng buộc này là quy trình phiên sạch, không phải sandbox kỹ thuật.

## Prompt chung (dán vào phiên sạch của một model Claude — Haiku 5.5 hoặc Sonnet 5.5; đề giữ trung lập model)

> Bạn là người viết ứng dụng Grit, không phải người sửa compiler. Chỉ dùng `docs/AI_GUIDE.md`, `docs/STD_REFERENCE.md` và đặc tả K được giao trong `docs/CAPSTONE.md`. Không đọc bất kỳ mã compiler, test, mã tham chiếu Rust, lời giải capstone hiện có, ví dụ, hay tài liệu `docs/spec/`. Viết đúng một gói Rig theo đề. Không nới/né kiểm tra, không đổi đặc tả. Làm theo `rig new`, `rig lock`, `rig check`, `rig run`; sửa lỗi trong chương trình của bạn cho đến khi check và hành vi đạt. Ghi lại mọi mã lỗi, thông báo khó hiểu, thiếu hụt AI_GUIDE và số lần biên dịch. Sau khi có chương trình, chạy `tools/capstone_check.sh <đường-dẫn-src/main.gt> <K1|K2|K3|K4>` để so khớp bản tham chiếu trên toàn bộ ca thường+lớn ở dev và release-o2. Nếu có bất đồng, thu nhỏ ca và sửa chương trình; không sửa bộ so hay reference. Không sửa `capstone/solutions/` trong phiên chuẩn bị hiện tại. Chỉ báo hoàn tất với lệnh và kết quả thật.

### K1 — csvstat

Viết `csvstat [FILE|-]` theo đúng đề mục K1 trong `docs/CAPSTONE.md`. Bao gồm stdin, CSV byte-oriented, thống kê cột số nguyên i64, định dạng trung bình cắt về 0 với 4 chữ số, và mọi mã lỗi/thoát được chỉ định.

### K2 — calc

Viết `calc` theo đúng đề mục K2 trong `docs/CAPSTONE.md`. Bao gồm parser biểu thức ngoặc/độ ưu tiên, số học i64 kiểm tràn, vị trí lỗi theo byte và mã thoát được chỉ định.

### K3 — kvstore

Viết `kvstore` theo đúng đề mục K3 và phần “Làm rõ K3” trong `docs/CAPSTONE.md`. Bao gồm set/get/del/count/dump/save/load, thứ tự dump/save theo byte, và giao dịch load không áp dụng dòng nào nếu có dòng sai.

### K4 — grep-lite

Viết `grep-lite [-c] [-n] PATTERN FILE...` theo đúng đề mục K4 trong `docs/CAPSTONE.md`. Bao gồm thứ tự cờ, `--`, nhiều tệp, dòng CRLF, pattern rỗng, mã thoát và byte đầu ra đúng đề.

## Kiểm tra quy trình trước khi giao capstone

Chương trình mẫu fib đã được dùng để xác minh `gritc check`/`gritc run` độc lập; xem kết quả G2.7 trong `docs/HISTORY.md`. Người chuẩn bị không viết hoặc chỉnh lời giải K1–K4.

> **Giới hạn nói thẳng (GÓI 4 S1.c):** đây là kiểm tra CÙNG HỌ model — tài liệu (`AI_GUIDE`, `STD_REFERENCE`) do Claude viết nên một model Claude đọc có thể hiểu dễ hơn người dùng/AI khác họ. Không dùng kết quả để nói "AI nào cũng viết được Grit".
