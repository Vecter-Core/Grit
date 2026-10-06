# Danh sách crate (plan §6.1) — chỉ tạo khi đến milestone, KHÔNG tạo crate rỗng
M1 gt-span gt-lexer · M2 gt-diag gt-ast gt-parser gt-fmt · M3 gt-resolve · M4 gt-types (+M4.5)
M5 gt-mir gt-dataflow (gt-hir HOÃN — ADR-020 §9) · M6 gt-borrowck · M7 gt-oblig gt-edge · M8 gt-effects gt-conc gt-term
M9 gt-secrets gt-driver · M10 gt-codegen-llvm gt-guard runtime/gt-rt · M11 rig · M12 lib/core/*.gt
Mỗi crate: `#![forbid(unsafe_code)]`, `[lints] workspace = true`, thêm vào `members`; ADR nếu có phụ thuộc ngoài.
**Lệnh `gritc …` (kể cả `gritc fmt`) CHỈ tồn tại từ M9 (gt-driver, binary `gritc`).** Trước đó mỗi crate là thư viện, test gọi thẳng API Rust (vd. `gt_fmt::format`); không tạo binary tạm. gt-driver là chỗ duy nhất nối gt-fmt thành `gritc fmt`.
