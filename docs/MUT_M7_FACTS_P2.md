# Đột biến phần M7 "sự kiện tuyến tính" (GĐ2 A2 bước 2, ADR-P2-05) — 2026-11-23

Tệp: `crates/gt-oblig/src/analysis.rs`, chỉ các dòng mã mới (`AState.facts`, `fact_locals_opt`, `add_fact`, `purge`, `hyps_for`, các điểm gọi `add_fact`) qua `GRIT_MUT_LINES`.
Công cụ: `tools/mut.py` (49/52 kết quả TỰ ĐỘNG; 3 con ghi `tay`: áp đột biến bằng `sed` rồi chạy test mới — không có trong bộ tự động vì test viết SAU lượt chạy). Bộ test của đột biến:
`cargo test -p gt-driver --test linear_facts --test kernel_coverage --test corpus_e2e`. Chi tiết từng con: `docs/MUT_M7_FACTS_P2.jsonl` (`first_status`, `status` cuối, `source`, `note`).

| | Giết | Sống | Không biên dịch |
|---|---|---|---|
| Lượt 1 (bộ test lúc đầu) | 26 | 26 | 0 |
| Cuối (sau test mới) | **33** | **18 (đều có lập luận)** | 1 |

**Nguyên tắc phân loại:** bỏ bớt sự kiện luôn AN TOÀN (bộ chứng minh chỉ yếu đi); chỉ đột biến THÊM sự kiện SAI hoặc GIỮ sự kiện CŨ làm mất soundness. Lượt 1 lộ ba ca loại sau, đã thêm test (`crates/gt-driver/tests/linear_facts.rs`) và kiểm lại:
- `635` (cho phép sự kiện chứa NGUYÊN TỬ logic): **KHÔNG SOUND thật** — sự kiện `c ∧ x + y ≤ 60` còn sống sau `c ← d` rồi chứng minh `⊳ c ∧ …` của lời gọi sau đó. Đã bị bắt bởi ca "nguyên tử bị gán lại" (lần đầu viết ca bằng `∧` trong thân KHÔNG bắt được: `∧` hạ thành hai nhánh, không tạo mệnh đề trộn; mệnh đề trộn chỉ đến từ HỢP ĐỒNG).
- `719` (đảo dấu `⊳` ban đầu của hàm): bị bắt bởi ca "⊳ không bị đảo" + "⊳ của chính hàm".
- `1430`, `1495` (đảo dấu sự kiện sau lời gọi: `⊳` của callee / `⊲`): bị bắt bởi `facts_assumed_after_a_call_are_the_callee_contract_not_its_negation` (cặp dương/âm tách riêng để hai lỗi không triệt tiêu).
Lưu ý quy trình: một lô đột biến ghi cảnh báo thiếu bộ nhớ (kernel giết một tiến trình) nên các kết quả "bị giết" của lô đó được chạy lại với 1 worker. Các con bị giết ở lượt 1–2 (19) KHÔNG được chạy lại.

**18 con sống (lập luận trong `note`):** `640`, `642`×2 (tương đương: id không tới được); `647`×3 (phòng thủ chiều sâu: local bị mượn `&!` không có term); `663`×2 (trần số sự kiện / môi trường chết — chỉ hiệu năng, sự kiện thêm vẫn đúng); `672`×3 (chỉ thêm sự kiện đúng, thừa);
`1150`×3 (chứng minh hai bước: sau khi `prune` sửa gốc, ngay cả `hashmap_collide` xanh với `if false` ⇒ KHÔNG test nào phân biệt ⇒ chỉ là phòng thủ chiều sâu, KHÔNG được tính là "có test bảo vệ"); `1156`, `1162` (chỉ làm yếu: đóng bắc cầu — **CHƯA có test chuỗi ≥ 3 sự kiện**); `1160` (thêm sự kiện đúng không liên quan);
`1814` (mã chết hiện tại: nhánh `targets[0].0 == 1` được chạm 0 lần trên corpus pass+warn, `std`, 3 dự án mẫu).
