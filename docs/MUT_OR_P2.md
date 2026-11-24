# Đột biến or-pattern `p | q` (GĐ2 C2c, ADR-P2-11) — 2026-11-24

Vùng: `parse_alts` (`gt-parser/src/pats.rs`), nhánh `Or` + `pat_binds` (`gt-types/src/check/pat.rs`), khai triển `Or` ở `missing` (`gt-types/src/exhaust.rs`), hạ `Or` (`gt-mir/src/build/matching.rs`). Công cụ: `tools/mut.py` (17 đột biến, TẤT CẢ tự động).
Bộ test: `corpus_e2e` (có `tests/ed2/`) + `or_pattern_oracle` qua script bọc dựng runtime. Chi tiết: `docs/MUT_OR_P2.jsonl`.

| Lượt | Giết | Sống |
|---|---|---|
| 1 | 14 | 3 |
| Sau 3 mẫu fail mới (`E0322_or_tuple_binds_in_one_component`, `E0322_or_multi_field_variant_binds_one`, `E0322_or_multi_field_struct_binds_one`) | **17** | **0** |

Lỗ thật lấp được: `pat_binds` (phát hiện alternative có bind) bị đột biến `any → all` mà không test nào thấy — mọi mẫu E0322 cũ chỉ có alternative mà MỌI thành phần đều bind hoặc chỉ một thành phần. Cần mẫu có vài thành phần không bind và đúng MỘT thành phần bind
(tuple, biến thể nhiều trường, struct nhiều trường): nếu không, `(0, v) | (1, 2)` có thể lọt qua E0322 và M5 sẽ hạ một alternative có bind mà không gán biến.
