# Đột biến kernel chứng chỉ Farkas (GĐ2 A3, ADR-P2-04) — 2026-11-23

Tệp (TCB): `crates/gt-kernel/src/check.rs`, `crates/gt-kernel/src/linear.rs`. Công cụ: `tools/mut.py` (TẤT CẢ tự động, 76 đột biến).
Bộ test của đột biến: `cargo test -p gt-kernel` (test độc hại, oracle vét cạn 3000 nghĩa vụ, test cô lập từng luật, test đầy đủ mọi dạng công thức).
Chi tiết: `docs/MUT_KERNEL_P2.jsonl` (`first_status`, `status` cuối, `round`, `note` = lập luận cho con còn sống).

| Lượt | Thay đổi test trước lượt | Giết | Sống |
|---|---|---|---|
| 1 | — | 52 | 24 |
| 2 | test cô lập TỪNG luật kernel trên một khẳng định SAI (hệ số âm, tổ hợp còn biến, nguyên tử một chiều); test mọi dạng công thức kernel hiểu (completeness) | +18 | |
| 3 | chứng chỉ nhiều ca có ca nguyên tử ở GIỮA (giải mã); mục tiêu `∧` | +2 | |
| **Cuối** | | **72** | **4 (đều tương đương / chỉ chặt hơn / chỉ hiệu năng)** |

Phát hiện quan trọng ở lượt 1: **3 đột biến KHÔNG SOUND sống** — `check.rs:85` (bỏ kiểm hệ số âm), `check.rs:93` (bỏ kiểm "tổ hợp còn biến"),
`check.rs:126` (chỉ đòi một vế của `a ∧ ¬a`). Nguyên nhân: mọi chứng chỉ giả trong test độc hại cũ đều sai ở NHIỀU luật cùng lúc, nên tắt một
luật vẫn bị luật khác chặn. Bài học: test độc hại phải có chứng chỉ chỉ vi phạm ĐÚNG MỘT luật. Cả ba đã bị giết từ lượt 2.

`mut.py` sập ở lượt 1 khi một đột biến làm test treo (`taskkill` không có trên Linux); đã sửa (nhóm tiến trình + `os.killpg` trên POSIX) và chạy lại
đột biến đó bằng `--resume` (bị giết).

4 con sống:
- `check.rs:63` (`if !c.is_empty()` → `true`): TƯƠNG ĐƯƠNG — ca Farkas rỗng không bao giờ hợp lệ (tổng rỗng là `0 ≤ 0`), cả hai bản đều từ chối.
- `check.rs:85` (`< 0` → `<= 0`): CHỈ CHẶT HƠN — từ chối thêm hệ số 0; không bao giờ chấp nhận thêm.
- `linear.rs:95`, `linear.rs:173` (bỏ giới hạn `MAX_CASES`): CHỈ HIỆU NĂNG — DNF vẫn đúng ngữ nghĩa.
