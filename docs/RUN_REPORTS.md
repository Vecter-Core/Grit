# BÁO CÁO CHẠY (nối thêm sau mỗi mục; mới nhất ở cuối)

Quy ước: nền tảng ghi ở MỌI báo cáo cổng. Chỗ lệnh Seed 1.0 sai kỹ thuật thì ghi ở "Sai kỹ thuật của lệnh" và chọn phương án fail-closed.

## Sai kỹ thuật của lệnh (ghi nhận, đã chọn phương án fail-closed)
1. "Lô nặng ≤ 10 phút + không tiến trình nền nào sống qua một lệnh" mâu thuẫn với test dài nhất của dự án (`gt-fmt` ~915 s một bộ, `gt-borrowck` oracle ~210 s, `gt-oblig`/`gt-term` hàng trăm giây). Phương án: cổng chạy THEO ĐƠN VỊ có điểm lưu (`GATE_BUDGET_SEC`), đơn vị dài chạy bằng cơ chế nền của công cụ (có timeout, chờ xong ngay trong cùng lượt, không để tiến trình nào sống sau khi báo cáo mục).
2. "Đổi sang `proven`" bị cấm, nhưng một số đo (G09/G14 của kiểm toán) so `all` với `proven` để TÌM khác biệt: chỉ đọc, không đổi mặc định.
3. Lệnh mong `hàm io truyền vào tham số thuần ⇒ E0801`; thực tế gritc từ chối ở bước kiểu với E0301 (hiệu ứng nằm trong kiểu hàm, bất biến). ĐỎ như nhau; KHÔNG đổi mã (đổi mã = nới/đánh số lại). Mẫu corpus khoá hành vi hiện tại.

## S0 — nền và kế hoạch (2026-10-21) — nền tảng: Linux x86_64 (Windows: chưa kiểm)
- Cổng nền `bash tools/gate.sh` trên commit `32cfb96` (đơn vị có điểm lưu): fmt OK · clippy `-D warnings` exit 0 · `cargo audit` exit 0 · **test 939 đạt / 0 lỗi / 22 đơn vị** (gt-span 9, gt-lexer 25, gt-diag 40, gt-ast 7, gt-parser 68, gt-stack 6, gt-fmt 35, gt-resolve 53, gt-types 174, gt-mir 30, gt-dataflow 14, gt-borrowck 60, gt-oblig 134, gt-effects 16, gt-conc 4, gt-term 29, gt-secrets 15, gt-llvm 0, gt-guard 33, gt-codegen-llvm 0, gt-driver 140, rig 47) · nul 0 tệp · deletions OK · track OK.
- Cổng ĐỎ lần đầu vì 2 lỗi của chính cơ chế mới (không phải của compiler): (1) `cargo fmt --all` không phủ `runtime/gt-rt` (workspace riêng) — bản port Linux của tôi để khoá mutex chưa `fmt`; sửa; (2) test `nul_scan` coi thư mục tạm nằm trong kho cha là "kho git" khi `TMPDIR` ở trong dự án; sửa (đòi `--show-toplevel` = gốc). Kiểm lại các giai đoạn bị ảnh hưởng (`gtrt expect nul deletions`): XANH. Snapshot "Initial commit" (needs_drop dở) KHÔNG làm cổng đỏ ⇒ không cần sửa test-trước.
- Ghi nhận thiết kế: khoá điểm lưu = HEAD + diff + tệp mới ⇒ đổi mã là làm lại cả cổng (~50 phút). Vì vậy: kiểm từng đơn vị bằng `GATE_STAGES`, cổng liền mạch chạy ở cuối mỗi mốc.
- Phát hiện mới khi đo `INTRINSIC_AS_VALUE`: FB-C5 (hiệu ứng hàm kết hợp mất khi làm giá trị) và FB-C6 (`⊳` không được kiểm qua giá trị hàm; trên native là UB cho `shl`/`rotr`…) — S1.
