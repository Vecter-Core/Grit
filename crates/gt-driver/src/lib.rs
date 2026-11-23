#![forbid(unsafe_code)]
//! gt-driver (M9, ADR-028): `gritc`. Chuỗi pass, CỔNG PHÁN QUYẾT (ĐỎ/ICE ⇒ không output, không chạy), báo cáo JSON có version,
//! `check`/`build`/`run`/`explain`/`fmt`. Backend M9 = thông dịch MIR tuần tự (`"backend":"interpreter"`); M10 thêm LLVM.
/// Edition của ngôn ngữ mà `gritc` hiểu (ADR-P2-01/09): 1 = Seed đóng băng; 2 = cộng toán tử `e?`. Cú pháp edition 2 là TẬP CHA của edition 1 (chỉ thêm).
pub const SUPPORTED_EDITIONS: [u32; 2] = [1, 2];

pub mod cli;
pub mod lsp;
#[cfg(feature = "llvm")]
pub mod native;
pub mod pipeline;
pub mod render;
pub mod runner;
pub mod sha256;
pub mod verdict;

pub use pipeline::{
    Artifacts, CheckResult, DEFAULT_CHECKS, Fault, Options, REQUIRED_PASSES, check,
};
pub use verdict::{RunEnd, Verdict, check_exit_code, decide, run_exit_code};
