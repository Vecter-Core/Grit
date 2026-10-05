//! Pattern.
use crate::expr::Lit;
use crate::item::{Ident, Path};
use crate::node::{NodeId, impl_node};
use gt_span::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pattern {
    pub id: NodeId,
    pub span: Span,
    pub kind: PatternKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldPat {
    pub id: NodeId,
    pub span: Span,
    pub name: Ident,
    /// `None` nghĩa là viết tắt `x` = `x: x`.
    pub pattern: Option<Pattern>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternKind {
    Wildcard,
    /// `..` đứng trong danh sách pattern (chỉ hợp lệ một lần mỗi danh sách; parser kiểm).
    Rest,
    /// Gán tên (hoặc variant không đối số — giải tên ở M3).
    Binding(Ident),
    /// Literal, có thể âm (`Lit::negative`).
    Lit(Lit),
    /// `a..b`, `a..=b`, `a..`, `..b`
    Range {
        start: Option<Lit>,
        end: Option<Lit>,
        inclusive: bool,
    },
    Variant {
        path: Path,
        args: Vec<Pattern>,
    },
    Struct {
        path: Path,
        fields: Vec<FieldPat>,
        rest: bool,
    },
    Tuple(Vec<Pattern>),
}

impl_node!(Pattern, FieldPat);
