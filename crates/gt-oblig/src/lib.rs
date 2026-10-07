#![forbid(unsafe_code)]
// `add/sub/mul/neg/not` là phép toán TRÊN IR/miền (không phải `std::ops`): tên cố ý trùng ký hiệu toán học.
#![allow(clippy::should_implement_trait)]
//! gt-oblig (M7, ADR-025): Obligation IR, bộ giải P1 (khoảng + zone) và bộ giải vét cạn thứ hai.
pub mod int;
pub mod interval;

pub use int::I256;
pub use interval::{B, Interval, Tri, Ty};
pub mod analysis;
pub mod driver;
pub mod env;
pub mod ir;
pub mod prover;
pub mod zone;
