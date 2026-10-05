# Danh sách crate (plan §6.1) — chỉ tạo khi đến milestone, KHÔNG tạo crate rỗng
M1 gt-span gt-lexer · M2 gt-diag gt-ast gt-parser gt-fmt · M3 gt-resolve · M4 gt-types (+M4.5)
M5 gt-hir gt-mir gt-dataflow · M6 gt-borrowck · M7 gt-oblig gt-edge · M8 gt-effects gt-conc gt-term
M9 gt-secrets gt-driver · M10 gt-codegen-llvm gt-guard runtime/gt-rt · M11 rig · M12 lib/core/*.gt
Mỗi crate: `#![forbid(unsafe_code)]`, `[lints] workspace = true`, thêm vào `members`; ADR nếu có phụ thuộc ngoài.
