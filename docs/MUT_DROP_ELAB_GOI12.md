# Đột biến elaboration drop (GÓI 12 phần 4, ADR-C18) — 2026-11-23

Tệp: `crates/gt-dataflow/src/elaborate.rs`. Công cụ: `tools/mut.py` (TẤT CẢ tự động, 107 đột biến chạy được trong 110 dòng `list`).
Bộ test của đột biến: `cargo test --no-default-features -p gt-driver --test user_drop --test corpus_e2e` (thông dịch; exe LLVM của cùng
chương trình chạy trong `user_drop` khi bản dựng có LLVM). Chi tiết từng đột biến: `docs/MUT_DROP_ELAB_GOI12.jsonl` (`first_status`,
`status` cuối, `round`, `note` = lập luận tương đương).

| Lượt | Thay đổi test trước lượt | Giết | Sống | Không biên dịch |
|---|---|---|---|---|
| 1 | — | 49 | 46 | 12 |
| 2 | bộ sinh thêm literal struct (lệnh `Assign`), move có điều kiện ra khỏi `Option`, gán đè trường của kiểu có `drop`, tham số move có điều kiện, trường không-glue đứng trước | +24 | | |
| 3 | điều kiện của bộ sinh đi qua `opq(true/false)`: bước hạ MIR CẮT nhánh `⎇ true/false` literal nên ở lượt 1–2 hầu như KHÔNG có move có điều kiện nào — cờ drop chỉ được thử qua vòng `i == m` và một ca viết tay | +30 | | |
| 4 | test hợp đồng giữ bản sao đối số; mẫu `E0407_contract_moves_drop_arg_into_tuple` + mẫu pass đối chứng | +23 (gồm cả đột biến của các lượt trước được chạy lại) | | |
| **Cuối** | | **70** | **25 (đều TƯƠNG ĐƯƠNG, có lập luận)** | **12** |

Bài học (ghi cho người viết oracle): một bộ sinh "trông đủ" (có move có điều kiện) có thể rỗng về ngữ nghĩa nếu compiler gập điều kiện
literal. Chỉ đột biến mới lộ ra (đột biến bỏ hẳn drop mở của enum vẫn SỐNG ở lượt 2).

25 con sống, theo nhóm (lập luận đầy đủ trong `note` của tệp jsonl):
- **Chỉ đổi hình dạng/hiệu năng:** 59 (cờ thay drop vô điều kiện), 117/121 (nhánh rỗng), 141 (drop trường không glue), 278/279/360 (thân không có
  drop có glue / không có cờ), 342/354 (`mutable` của local tạm không được đọc sau M5).
- **Tắt cờ dư thừa:** 376, 433, 461×3, 464×2 — mọi drop do bước hạ phát luôn theo sau bởi `StorageDead` hoặc gán đè chính chỗ đó trước lần đọc cờ kế.
- **Nhánh qua `*` / giá trị hàm / kiểu không glue không bao giờ có cờ:** 316, 371, 398, 400, 402, 408, 410.
- **Phòng thủ chiều sâu ở hợp đồng:** 259, 260 — M7 không bao giờ chạy thân hợp đồng có đối số không-Copy (chứng minh tĩnh hoặc E0902; đã thử).
