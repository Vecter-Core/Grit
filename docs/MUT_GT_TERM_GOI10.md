# MUT_GT_TERM_GOI10 — đột biến hàm quyết định của gt-term (GÓI 10, S3.b; 2026-11-20)

Phạm vi: `crates/gt-term/src/lib.rs` dòng 293–690 (`instance_graph`, `discover`, `conservative_graph`, `dyn_fn_arg`, `takes_fn_param`, `has_back_edge`: mã sửa ở FB-C32/33/34). Công cụ: `tools/mut.py` qua bọc `tools/mut_range.py` (lọc theo khoảng dòng). 1 worker; bản sao làm việc `D:/Grit_mut/w0`.
Bộ test dùng: `cargo test -p gt-term --test spawn_recursion --test instance_graph --test conservative_fallback --test termination --test fail_closed_loops -- --test-threads=2` (baseline xanh). `term_oracle` KHÔNG dùng được làm baseline (thoát mã 143 ở worker; CHƯA điều tra). Timeout mỗi đột biến 150 s (một đột biến `!= → ==` ở union-find treo ⇒ tính là bị bắt).

## Số liệu (từ `docs/MUT_GT_TERM_GOI10.jsonl`)
- 110 đột biến, **nguồn: 110 tự động, 0 tay**. Bị bắt 80, không hợp lệ (lỗi biên dịch) 3, **sống sót 27**.
- Chạy lại ca sống sót với bộ test rộng hơn (`m8_corpus`, `ghost_wide_oracle`, `limit_hint`, `ghost_limit_soundness`): DỪNG sớm vì chậm (≈ 80 s mỗi ca); chỉ có 6 kết quả (`docs/MUT_GT_TERM_GOI10_R2.jsonl`): các dòng 300/309/318 — 300 `→false`, 309 `>= → >` và `→false` vẫn sống sót; 300 `→true`, 309 `→true`, 318 `→true` bị bắt. Phần còn lại CHƯA chạy lại.

## 27 ca sống sót, phân loại THEO ĐỌC MÃ (CHƯA kiểm bằng chạy)
Hướng "bảo thủ hơn" (chương trình bị từ chối nhiều hơn; không âm tính giả): 309 `>= → >` (lệch một ở giới hạn đồ thị), 451 `→true`, 554 `false→true`, 615 (`&&→||`, `→true`), 686 `false→true`. Còn lại là hướng có thể làm gt-term CHẤP NHẬN thêm hoặc chưa rõ: 318, 323 (×3), 340, 361, 372, 385, 426 (×2), 431, 433, 443, 445, 495, 507, 559, 576, 671, 675, 683.
Nhánh phòng thủ (`None => true` ở 559, 576; `return true` ở 671, 683: MIR hỏng/không có chữ ký) có vẻ KHÔNG tới được từ chương trình hợp lệ, nhưng điều này CHƯA được chứng minh: đặt vào `docs/KNOWN_LIMITATIONS.md`.
**Kết luận trung thực:** theo luật gói ("hướng làm CHẤP NHẬN thêm phải bị test bắt hoặc có lập luận tương đương cụ thể kèm đo") thì ~22 ca CHƯA đạt: chưa có test bắt, chưa có lập luận tương đương. Không sửa mã (đóng băng); việc thêm test để giết các ca này nằm ở `docs/NEXT_PRODUCT_BATCH.md` mục 6.
