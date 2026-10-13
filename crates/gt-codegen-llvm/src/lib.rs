//! gt-codegen-llvm (M10, ADR-029): hạ MIR SAU M7 thành LLVM IR rồi object COFF x86_64-pc-windows-msvc.
//!
//! Nguyên tắc (ADR-029 §3, Giang 2026-10-12):
//! * `Assert` ⇒ kiểm lúc chạy rồi `gritrt_abort(loại)` (D3); `Unreachable` ⇒ abort, KHÔNG BAO GIỜ làm điều kiện tối ưu.
//!   `unreachable` của LLVM CHỈ xuất hiện ngay sau `call @gritrt_abort` (khối tên `trap.*`) — IR Guard (M10 bước 7) kiểm điều này.
//! * Thứ gì chưa hạ được ⇒ `CodegenError::Unsupported` (fail-closed: KHÔNG sinh nhị phân sai, KHÔNG đoán).
//! * Thông dịch MIR KHÔNG phải bằng chứng cho codegen; codegen có test riêng chạy nhị phân thật.
mod lower;

pub use lower::{Compiled, Inject, Options, compile, expected_instances};

use std::fmt;

/// Lỗi codegen. Cả hai đều làm `gritc` KHÔNG ghi/chạy nhị phân.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    /// Cấu trúc hợp lệ của Grit mà backend LLVM Phase 1 chưa hạ được (nêu rõ cái gì).
    Unsupported(String),
    /// Lỗi nội bộ (LLVM verifier từ chối IR, thiếu thân MIR...): ICE.
    Internal(String),
    /// IR Guard từ chối (E1101, ĐỎ): danh sách vi phạm; KHÔNG có object.
    Guard(Vec<String>),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::Unsupported(m) => write!(f, "backend llvm chưa hỗ trợ: {m}"),
            CodegenError::Internal(m) => write!(f, "lỗi nội bộ codegen: {m}"),
            CodegenError::Guard(v) => write!(f, "IR Guard từ chối: {}", v.join("; ")),
        }
    }
}

impl std::error::Error for CodegenError {}

/// Loại abort — PHẢI khớp `ABORT_KINDS` của `runtime/gt-rt`.
pub mod abort_kind {
    pub const UNKNOWN: u64 = 0;
    pub const OVERFLOW: u64 = 1;
    pub const BOUNDS: u64 = 2;
    pub const DIV_ZERO: u64 = 3;
    pub const ASSERTION: u64 = 4;
    pub const UNREACHABLE: u64 = 5;
    pub const CAST_LOSSY: u64 = 6;
    pub const PRECONDITION: u64 = 7;
    pub const POSTCONDITION: u64 = 8;
    pub const INVARIANT: u64 = 9;
    pub const EXPLICIT: u64 = 10;
    pub const OVERFLOW_NEG: u64 = 11;
    pub const REM_ZERO: u64 = 12;
    pub const SLICE_RANGE: u64 = 13;
    pub const IO_ERROR: u64 = 14;
    pub const ARG_OOR: u64 = 15;
    pub const FUEL: u64 = 16;
    pub const ARENA_EXHAUSTED: u64 = 17;
}
