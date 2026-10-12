//! gt-llvm: lớp mỏng quanh LLVM-C cho `gt-codegen-llvm` (sinh IR) và `gt-guard` (kiểm IR). Hai crate đó KHÔNG phụ thuộc nhau;
//! cùng dùng `inkwell` qua đây để chỉ có MỘT chỗ liên kết LLVM và MỘT shim FFI.
mod shim;

pub use inkwell;
