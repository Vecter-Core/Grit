# Việc cần làm: CI (đơn giản — giao cho AI khác)
Tạo `ci.yml`: trên push/PR chạy `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `cargo deny check`, `cargo audit`; thêm job build hai lần `gritc` và so sha256 (D9).
Hiện `members = []` nên các lệnh phải không lỗi trên workspace rỗng. Không dùng `continue-on-error`, không tắt bước nào.
