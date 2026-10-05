//! Kiểu.
use crate::expr::Expr;
use crate::item::{Effects, Lifetime, Path};
use crate::node::{NodeId, impl_node};
use gt_span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimType {
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    F32,
    F64,
    Bool,
    Char,
    Str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Type {
    pub id: NodeId,
    pub span: Span,
    pub kind: TypeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeKind {
    Prim(PrimType),
    SelfType,
    /// `Option<T>`, `Cap<Fs>`, `Vec`… (`args` rỗng nếu không có tham số kiểu).
    Path {
        path: Path,
        args: Vec<Type>,
    },
    Ref {
        lifetime: Option<Lifetime>,
        mutable: bool,
        inner: Box<Type>,
    },
    /// `[T; N]`
    Array {
        elem: Box<Type>,
        len: Box<Expr>,
    },
    /// `[T]` (chỉ hợp lệ sau tham chiếu — kiểm ở tầng kiểu)
    Slice(Box<Type>),
    /// `()` là tuple rỗng; `(A,)` là tuple một phần tử.
    Tuple(Vec<Type>),
    /// `*□ T` / `*μ T` — chỉ hợp lệ trong `⚠` (kiểm ở tầng sau).
    RawPtr {
        mutable: bool,
        inner: Box<Type>,
    },
    /// Kiểu hàm tĩnh `ƒ(A) → R ! E`.
    Fn {
        params: Vec<Type>,
        ret: Option<Box<Type>>,
        effects: Effects,
    },
    /// Kiểu never `!`.
    Never,
}

impl_node!(Type);
