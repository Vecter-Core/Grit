#![forbid(unsafe_code)]
//! gt-ast (M2): kiểu node AST — chỉ dữ liệu, không logic (trừ bộ cấp `NodeId`).
//!
//! * Mỗi node có `Span` và `NodeId` tuần tự tất định (ADR-013).
//! * AST **không biết dạng bề mặt** (canonical Unicode hay alias ASCII): crate chỉ phụ thuộc `gt-span`.
//! * Doc comment và attribute là node con của item/trường/variant/thành viên.
//! * Comment `//` KHÔNG ở trong AST (nằm ở bảng trivia của lexer, ADR-012).
mod expr;
mod item;
mod node;
mod pat;
mod ty;

pub use expr::*;
pub use item::*;
pub use node::{Node, NodeId, NodeIdGen};
pub use pat::*;
pub use ty::*;
