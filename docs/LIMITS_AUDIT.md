# LIMITS_AUDIT — mọi giới hạn số trong bộ kiểm và điều gì xảy ra khi vượt (GÓI 6 S2.b, 2026-11-19)

Quét tay `grep` các hằng/so sánh số trong `gt-dataflow`, `gt-oblig`, `gt-term`, `gt-effects`, `gt-borrowck`, `gt-types`, `gt-mir`, `gt-resolve`, `gt-guard`, `gt-driver`, `gt-stack`. Nguyên tắc cần giữ: vượt giới hạn ⇒ ĐỎ có mã rõ, hoặc ICE (E9901), hoặc kết quả BẢO THỦ (giữ check, không xoá); **KHÔNG BAO GIỜ chấp nhận lặng lẽ hay xoá check**.
Cột "bằng chứng": **TN** = thí nghiệm thật trong gói này (lệnh/tệp kèm theo); **TEST** = test có sẵn; **MÃ** = chỉ đọc mã (CHƯA có thí nghiệm — nói rõ). Không có giới hạn nào cho thấy "vượt mà bị chấp nhận lặng lẽ/xoá check" trong các thí nghiệm đã làm; các mục MÃ vẫn là lập luận.

| # | giới hạn | giá trị | tệp | khi vượt thì | bằng chứng |
|---|---|---|---|---|---|
| 1 | số biến theo dõi quan hệ (zone) mỗi hàm `ZONE_MAX_VARS` | 24 | `gt-oblig/src/analysis.rs` | biến thừa bị bỏ khỏi quan hệ (ghi chú N0605, ẩn mặc định); chỉ giữ khoảng ⇒ chứng minh ít hơn ⇒ check được GIỮ nhiều hơn | TEST `tests/pass/N0605_*` (mỗi mẫu phát đúng N0605); TN `pass/many_locals_beyond_zone_cov` green |
| 2 | số biến "bóng" của biến thể vòng lặp mỗi hàm `GHOST_MAX_VARS` | 24 | `gt-oblig/src/analysis.rs` | vòng thừa không có bóng ⇒ biến thể không chứng minh được ⇒ **E0702 (ĐỎ, từ chối oan, fail-closed)** | TN: 24 vòng (mỗi vòng biến riêng) xanh; 25, 26 vòng bị E0702 (`gt-term/tests/limit_hint.rs`; đo `docs/SCALE_PROBE.jsonl` họ `loop`); thông báo nay nêu nguyên nhân + gợi ý tách hàm |
| 3 | số khối tối đa của một thân `UNREP_MAX_BLOCKS` = 2²⁰/32 | 32 768 | `gt-oblig/src/analysis.rs` | `fixpoint` trả None ⇒ chạy lại không quan hệ ⇒ vẫn None ⇒ MỌI nghĩa vụ Unknown (giữ mọi check); ghi chú N0605 "hết ngân sách bước" | TN: hàm 16 000 `⎇` (≈ 48 000 khối) ⇒ xanh + ghi chú N0605 "hết ngân sách bước phân tích `main`: bỏ mọi quan hệ, chỉ còn khoảng" (đúng nhánh `budget_exhausted`); KHÔNG đo trực tiếp việc giữ check ở `--checks=proven` ⇒ phần "giữ check" là MÃ |
| 4 | số bước tối đa mỗi khối `MAX_BLOCK_STEPS` | 400 000 | `gt-oblig/src/analysis.rs` | như #3 | MÃ (chưa dựng được chương trình chạm riêng giới hạn này) |
| 5 | số hợp đồng tối đa mỗi hàm `UNREP_SLOTS - 1` | 31 | `gt-oblig/src/analysis.rs` | `fixpoint` từ chối thân (như #3) | MÃ |
| 6 | ngân sách bộ chứng minh mỗi nghĩa vụ `budget_per_ob` / `Budget` / `max_points` / `bits > 20` | 2¹⁶ / 2²⁰ / 2²⁴ / 20 bit | `gt-oblig/src/analysis.rs`, `prover.rs` | nghĩa vụ Unknown ⇒ giữ check (E0902 nếu hợp đồng không kiểm lúc chạy được) | MÃ; TEST bộ giải trong `gt-oblig/tests/*` |
| 7 | độ sâu instance đa hình `MAX_INSTANCE_DEPTH` | 64 | `gt-types/src/mono.rs` | **E0312** ĐỎ | TEST `tests/fail/E0312_*`; TN `spawn` kiểu mảng lồng 70 tầng ⇒ ĐỎ (E0312 + E0501) |
| 8 | số instance sau đơn hình hoá `MAX_PROGRAM_INSTANCES` | 10 000 | `gt-types/src/mono.rs` | **E0317** ĐỎ "quá 10000 instance" | TN: 9 000 instance xanh (3,2 s); 10 001 ⇒ E0317 đỏ (0,7 s) |
| 9 | số đỉnh đồ thị lời gọi của gt-term `MAX_GRAPH_NODES` | 20 000 | `gt-term/src/lib.rs` | rơi về đường bảo thủ (mọi lời gọi có thể tới mọi impl) | TEST `gt-term/tests/conservative_fallback.rs` (chốt `set_graph_limit_for_test`) |
| 10 | độ sâu kiểm Send `auto_rec` | 64 | `gt-types/src/send.rs` | `Err("kiểu lồng quá sâu")` ⇒ E0501 ĐỎ | TN (mảng lồng 70 tầng) |
| 11 | độ sâu đánh giá hằng độ dài mảng `const_depth` | 32 lần gọi (mỗi mức hằng ≈ 2) | `gt-types/src/collect.rs` | `None` ⇒ E0301 "độ dài mảng phải là hằng xác định" ĐỎ (thông báo hơi gây hiểu nhầm) | TN: chuỗi 15 hằng lồng xanh, 16 ⇒ E0301 đỏ |
| 12 | độ sâu vòng "chứa chính nó theo giá trị" `DEPTH` | 32 | `gt-types/src/collect.rs` | coi là vòng ⇒ E0301 "kích thước vô hạn" ĐỎ | TEST corpus E0301 (kiểu đệ quy); MÃ cho biên 32 |
| 13 | độ sâu `has_raw_ptr` (kiểm `unsafe`) | 32 | `gt-oblig/src/unsafe_check.rs` | "quá sâu: coi là có" (fail-closed) | MÃ |
| 14 | số chẩn đoán hiển thị mỗi tầng `LIMIT` | 100–200 | các `lib.rs` | cắt hiển thị; phán quyết VẪN ĐỎ | TN: 6 000 lỗi ⇒ hiển thị 100, "phán quyết red; 100 lỗi" |
| 15 | số chẩn đoán toàn pipeline `Diagnostics::new(5000)` | 5 000 | `gt-driver/src/pipeline.rs` | cắt; vẫn ĐỎ | TN gián tiếp (#14); MÃ |
| 16 | độ sâu/nhiên liệu của bộ chạy tham chiếu (`--backend=interpreter`) `FUEL`, `frames > 201` | 50 000 000 / 201 khung | `gt-driver/src/runner.rs`, `gt-mir/src/interp.rs` | mã thoát 71 "hết giới hạn" | TN (bản release, không LLVM): đệ quy 100 000 ⇒ mã 71. **Bản debug của gritc: đệ quy 300 ⇒ gritc tự tràn stack (Rust) và chết (mã 127) TRƯỚC khi chạm giới hạn 201** — chỉ ảnh hưởng backend thử nghiệm của bản debug; fail-closed (không chạy) |
| 17 | stack luồng Grit | 256 MiB | `runtime/gt-rt` | HĐH giết tiến trình (giới hạn tài nguyên chương trình, `KNOWN_LIMITATIONS`) | TN GÓI 4 `docs/STACK_PROBE.md` (đo nhu cầu thật ≤ 4 MiB cho 10 chương trình) |

## Phát hiện
1. **Giới hạn #2 gây từ chối oan có thể gặp thật**: hàm có ≥ 25 vòng lặp (mỗi vòng biến riêng) bị E0702 dù đúng. Đã cải thiện thông báo (test ĐỎ trước `crates/gt-term/tests/limit_hint.rs`, commit riêng) và ghi vào `docs/KNOWN_LIMITATIONS.md`. KHÔNG phải sai âm tính.
2. **Hiệu năng (xem `docs/SCALE_PROBE.jsonl`, `docs/RUN_REPORTS.md`)**: thời gian kiểm tăng ≈ bậc 2 theo số cấu trúc ở các hình dạng hợp lệ (n `⎇`: 3 200 ⇒ 1,4 s; 16 000 ⇒ 37 s ở bản release; n hàm có vòng lặp: 1 600 ⇒ 7 s); riêng họ "n vòng lặp trong MỘT hàm" (vượt #2, vốn bị từ chối) tăng ≈ bậc 3 (100 ⇒ 7,6 s; 200 ⇒ 58 s; 400 ⇒ > 120 s) chủ yếu ở giai đoạn `m8.termination`.
3. **KHÔNG tìm thấy giới hạn nào vượt mà bị chấp nhận lặng lẽ hay xoá check** trong các thí nghiệm. Các mục "MÃ" (#4, #5, #6, #13, #15) CHƯA được chứng minh bằng thí nghiệm; ghi trong `docs/RUN_REPORTS.md` mục CHƯA KIỂM.
