# Đột biến `let` phân rã (GĐ2 C2b, ADR-P2-10) — 2026-11-24

Vùng: nhìn trước + `parse_let_pat` (`gt-parser/src/exprs.rs`), nhánh `StmtKind::LetPat` (M4, `gt-types/src/check/mod.rs`), `let_pat_into` (M5, `gt-mir/src/build/matching.rs`). Công cụ: `tools/mut.py` (22 đột biến, TẤT CẢ tự động).
Bộ test: `corpus_e2e` (có `tests/ed2/`) + `let_pat_oracle` + `qm_oracle`, qua script bọc dựng runtime (xem `docs/MUT_QM_P2.md`). Chi tiết: `docs/MUT_LP_P2.jsonl`.

| | Số lượng |
|---|---|
| Lượt 1: giết / sống | 14 / 8 |
| Mã chết đã XOÁ (5 con: `mod.rs:902`×4, `:910`) | 6 |
| Giết sau mẫu mới `let_lookahead_long_expression_statement` (`exprs.rs:827`, biên 512 token của nhìn trước) | 1 |
| Còn sống có lập luận | 2 (`exprs.rs:823`: chỉ khác lời thông báo; `matching.rs:152`: chỉ làm yếu M7) |

Lỗ thật lấp được: biên 512 token của nhìn trước (một câu lệnh biểu thức dài hơn 512 token không có `≔` phải vẫn là biểu thức); và mã chết trong M4 (phân rã không bao giờ cần ép unsize/`!`) bị xoá.
