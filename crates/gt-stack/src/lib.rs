#![forbid(unsafe_code)]
//! gt-stack (M2): MỘT chỗ chung để chạy mã đệ quy trên AST trong luồng có stack lớn (ADR-015).
//!
//! `gt_ast::MAX_DEPTH` chỉ bảo đảm cây không sâu hơn giới hạn; nó KHÔNG bảo đảm stack của luồng gọi đủ
//! (ở bản debug mỗi mức lồng đi qua nhiều khung đệ quy). Vì vậy mọi pass đệ quy (parser, types, borrowck, …)
//! phải chạy qua [`run_with_big_stack`]. Stack cố định (không đo theo địa chỉ) để kết quả không phụ thuộc chế độ build (D9).
use std::fmt;

/// Stack của luồng chạy pass: 64 MiB (địa chỉ ảo, cấp dần theo nhu cầu).
pub const BIG_STACK_BYTES: usize = 64 * 1024 * 1024;

/// Lỗi khi chạy pass. Cả hai đều là lỗi nội bộ của compiler (ICE `E9901`), không phải lỗi của người dùng.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StackError {
    /// Không tạo được luồng (hết bộ nhớ?).
    Spawn(String),
    /// Mã trong luồng bị panic; chứa thông điệp panic.
    Panic(String),
}

impl fmt::Display for StackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StackError::Spawn(m) => write!(f, "không tạo được luồng stack lớn: {m}"),
            StackError::Panic(m) => write!(f, "panic nội bộ: {m}"),
        }
    }
}

impl std::error::Error for StackError {}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic không có thông điệp".to_string()
    }
}

/// Chạy `f` trong một luồng riêng có [`BIG_STACK_BYTES`] stack; `f` được phép mượn dữ liệu của người gọi.
/// Panic bị bắt và trả về `StackError::Panic` (không bao giờ làm sập tiến trình).
pub fn run_with_big_stack<T, F>(f: F) -> Result<T, StackError>
where
    T: Send,
    F: FnOnce() -> T + Send,
{
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("gt-big-stack".to_string())
            .stack_size(BIG_STACK_BYTES)
            .spawn_scoped(scope, f)
            .map_err(|e| StackError::Spawn(e.to_string()))?;
        handle
            .join()
            .map_err(|p| StackError::Panic(panic_message(p.as_ref())))
    })
}
