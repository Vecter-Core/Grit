# Đột biến toán tử `e?` (GĐ2 C2, ADR-P2-09) — 2026-11-23

Tệp: `crates/gt-types/src/check/expr.rs` (`try_expr`, M4) và `crates/gt-mir/src/build/matching.rs` (`try_into`, M5), qua `GRIT_MUT_LINES`. Công cụ: `tools/mut.py` (TẤT CẢ tự động, 30 đột biến).
Bộ test: `cargo test -p gt-driver --test corpus_e2e --test qm_oracle` (corpus E2E có `tests/ed2/` + oracle ngẫu nhiên × thông dịch + LLVM dev/release/release-o2). Chi tiết: `docs/MUT_QM_P2.jsonl`.

| Lượt | Giết | Sống |
|---|---|---|
| 1 | 26 | 4 |
| Sau 3 mẫu corpus mới (`E0321_operand_is_user_struct`, `E0321_operand_is_look_alike_enum`, `E0309_error_inside_try_operand_is_not_cascaded`) | **28** | **2 (đều tương đương)** |

Hai lỗ test THẬT lộ ra ở lượt 1 và đã lấp: (a) toán hạng là kiểu người dùng (struct, enum giống `Option`) phải báo E0321 chứ không phải E0320 — trước đó mọi mẫu E0321 chỉ dùng kiểu nguyên thuỷ/tuple;
(b) toán hạng ĐÃ lỗi kiểu (`a.nope()?`) chỉ được báo lỗi gốc (E0309), không thêm E0321 dây chuyền.
2 con sống: `matching.rs:137` và `:160` — hai chốt ICE phòng thủ ở M5 (M4 đã bảo đảm họ kiểu); không tới được.

**Quy trình (ghi cho lần sau):** lô đầu bị "BASELINE ĐỎ" vì `qm_oracle` dựng exe LLVM cần `runtime/gt-rt/target/release/libgt_rt.a`, mà `mut.py prepare` chép `runtime/` BỎ `target`. Sửa: lệnh test là một script bọc
(`GRIT_MUT_TEST="bash script"`) tự dựng runtime vào đúng đường dẫn rồi chạy test. `tools/mut.py` in dòng BASELINE; lọc `grep` của tôi đã che nó hai lần — luôn đọc dòng đó.
Phát hiện thêm (không phải lỗi của `?`): sau một lỗi kiểu trong toán hạng nhị phân (`(a + true)?`) vẫn có E0321 dây chuyền, vì phép cộng sai kiểu trả về kiểu `u32` chứ không phải `Err` — thuộc bộ kiểm kiểu chung, ghi vào `docs/KNOWN_LIMITATIONS.md`.
