# SEED_PLAN — kế hoạch tới SEED 1.0 (lập 2026-10-21; nền tảng làm việc: Linux x86_64 container; Windows: chưa kiểm)

Nhãn **SEED 1.0** chỉ khi ĐỦ sáu mục (A1 dùng được, A2 an toàn, A3 cổng xanh liền mạch, A4 tái lập & di động, A5 tài liệu kiểm được, A6 đóng băng). Thiếu mục nào ⇒ nhãn RC kèm danh sách mục thiếu. Ước lượng: S = nhỏ (≤ 1 giờ), M = vừa (vài giờ), L = lớn (nhiều phiên), XL = rất lớn.

| Mục | Việc | Tiêu chí XONG (đo được) | Rủi ro | Ước lượng |
|---|---|---|---|---|
| S0 | HANDOFF + bảng CÓ/THIẾU; `preflight.sh`, giai đoạn `nul`, `INTRINSIC_AS_VALUE.md` (ĐO), RUNLIST, MERGE_NOTES, SEED_PLAN; cổng nền | các tệp có; unittest `nul_scan` xanh; cổng nền chạy xong và ghi số THẬT | cổng nền ~45 phút CPU, phải chạy theo đơn vị | M |
| S1 | FB-C5 (hiệu ứng hàm kết hợp làm giá trị), FB-C6 (`⊳` qua giá trị), M6 qua giá trị hàm (+ bộ sinh `fn_value_ref` ≥ 300+300, tắt bản sửa thì tự bắt), `ARENA_CHECK` chỉ bật thêm, bảng F-01…F-08 đỏ-trước | mỗi lỗi: test đỏ trước (bằng chứng hoàn nguyên), commit riêng, FB-C; corpus ≥ 3 fail + ≥ 3 pass mỗi hành vi mới | M6 qua kiểu hàm không mang region ⇒ có thể phải từ chối kiểu hàm có tham chiếu (giảm khả năng biểu đạt) | L |
| S2 | K1–K4 + bản tham chiếu Rust + bộ tệp đầu vào (`docs/CAPSTONE.md`); bảng khoảng trống; `?` và phân rã tuple (desugar, ADR-C), `Result/IoError` cho I/O, `Cap<In>`/stdin, ghi tệp, đối số dòng lệnh, API Vec/Text/str/HashMap, tên tệp ≠ tên module, Arena chia sẻ (R6) / HashMap (R7) | mỗi hàm std mới có bộ kiểm riêng như std hiện tại (≥ 30 chương trình ba nguồn); intrinsic mới ≤ 6, mỗi cái hợp đồng TCB + Guard + đột biến lô nhỏ; tính năng ngôn ngữ mới: ADR-C, ASCII 1-1, gt-fmt, round-trip, corpus, bộ sinh | RẤT LỚN: `?`, HashMap, Result I/O chạm parser/types/MIR/codegen/TCB; Arena chia sẻ chạm ADR-038 (đổi ngữ nghĩa mượn là ĐIỀU KIỆN DỪNG (e)) | XL |
| S3 | QUICKSTART, GRIT_BOOK, LANGUAGE_REFERENCE, STD_REFERENCE (sinh bằng `tools/gen_docs.py`, test khoá từng byte), AI_GUIDE (≤ 12 000 token proxy), SAFETY_CLAIMS, `tools/doc_test.py` vào `gate.sh` | mọi khối ```grit trong docs biên dịch/bị từ chối đúng chú thích; gate đỏ nếu sai | khối mã trong tài liệu hiện có (nếu có) chưa theo chú thích | L |
| S4 | 4 subagent MỚI viết K1–K4 chỉ từ AI_GUIDE + STD_REFERENCE + đề bài; `CAPSTONE_LOG.md`; sửa tài liệu/chẩn đoán/compiler/thư viện theo log; lặp tới khi đúng với tham chiếu ở dev và release-o2 | 4/4 đầu ra ĐÚNG trên toàn bộ tệp đầu vào, hai hồ sơ; không nới kiểm tra | subagent cần quyền chạy gritc; nếu không có công cụ thì tự làm trong ngữ cảnh sạch và ghi giới hạn | L |
| S5 | `setup_cloud.sh`, kiểm clone mới (đo thời gian từng bước, ≤ 25 phút), `QUICKSTART_CLOUD.md`, `BUILDING.md`, `install.sh/.ps1`, CI Linux+Windows (chỉ viết), `gritc doctor`; cổng xanh liền mạch trên mã cuối; bundle; push; TAG `seed-1.0-rc1` | cổng `GATE_RESULT=GREEN` trên đúng commit gắn tag; clone mới chạy được `fib` và bắt chương trình sai | tải LLVM 20.1.8 trong container có proxy; Windows chưa kiểm | L |
| S6 | `gritc lsp`, `editors/vscode/`, schema JSON chẩn đoán khoá bằng test | kịch bản stdio test; LSP không đổi phán quyết | M13 trước đây KHÔNG làm: nay có lệnh | L |
| S7 | R3 fail-open scan, R5 flake/std release, R10 69 đột biến M7, R9 fuzz tích luỹ ≤ 2 giờ, R11 đọc mã đối kháng bằng subagent | từng mục theo RUNLIST; không đánh dấu XONG khi chưa đạt | thời gian; đột biến chậm | XL |
| S8 | FREEZE.md, SEED_LIMITATIONS.md, CHANGELOG.md, `--version` = `1.0.0-seed` hoặc `-rcN`, READINESS bảng, cổng xanh, build hai lần cùng SHA-256, bundle, tag LOCAL→push tag | sáu mục A đều đạt ⇒ `1.0.0-seed`; còn thiếu ⇒ `-rcN` + danh sách | A2 có mục "chưa" vì thời gian (fuzz 24 h, đọc mã bằng model khác) — phải ghi trung thực | M |

## Thứ tự
S0 → S1 → S2 → S3 → S4 → S5 → [TAG seed-1.0-rc1] → S6 → S7 → S8. Push sau mỗi mục và ≥ mỗi 45 phút. Nếu cạn thời gian thì dừng ở mốc gần nhất; rc1 ưu tiên cao.

## Điểm dừng cứng (lệnh): (a) push lỗi hoàn toàn; (b) cổng đỏ sau 2 lần sửa; (c) preflight báo kho hỏng/ổ thiếu; (d) lỗ hổng điều kiện cứng #9 hoặc #12 không sửa được ngay; (e) tính năng cần thiết đòi đổi ngữ nghĩa mượn hoặc thêm tính năng ngoài danh sách S2.
## Rủi ro lớn nhất
S2 (một mình nó có thể ăn nhiều phiên) và S4 (lặp theo log). Mọi quyết định dồn lại phải để cổng liền mạch xanh trên mã cuối — không "đánh dấu cho đủ".
