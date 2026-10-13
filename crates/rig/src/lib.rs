#![forbid(unsafe_code)]
//! rig (M11): trình quản lý gói v0 của Grit. Mô hình: một gói = `rig.toml` + `src/*.gt` (mỗi tệp một module, phẳng); phụ thuộc theo
//! đường dẫn cục bộ; `rig.lock` ghi băm nội dung từng gói. Mọi lệnh biên dịch đi qua CÙNG cổng phán quyết của gritc (`gt_driver::check`).
pub mod cli;
pub mod lock;
pub mod project;
pub mod toml;
