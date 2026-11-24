//! Câu lệnh và biểu thức.
use crate::item::{Ident, Path};
use crate::node::{NodeId, impl_node};
use crate::pat::Pattern;
use crate::ty::Type;
use gt_span::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub id: NodeId,
    pub span: Span,
    pub stmts: Vec<Stmt>,
    /// Biểu thức cuối không `;` = giá trị khối.
    pub tail: Option<Box<Expr>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stmt {
    pub id: NodeId,
    pub span: Span,
    pub kind: StmtKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StmtKind {
    Let {
        mutable: bool,
        name: Ident,
        ty: Option<Type>,
        init: Expr,
    },
    /// `pattern [: kiểu] ≔ giá_trị;` — `let` phân rã (CHỈ edition ≥ 2, ADR-P2-10): pattern PHẢI không thể bị từ chối (M4 kiểm vét cạn, E0305); biến bất biến
    LetPat {
        pat: Pattern,
        ty: Option<Type>,
        init: Expr,
    },
    /// `đích ← giá_trị`; đích là biểu thức vị trí (parser đã kiểm dạng).
    Assign {
        target: Expr,
        value: Expr,
    },
    /// `semi = false` cho câu lệnh dạng khối (`⎇ ⌥ ⟳ ⚠`) không cần `;`.
    Expr {
        expr: Expr,
        semi: bool,
    },
    Return(Option<Expr>),
    Break,
    Continue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expr {
    pub id: NodeId,
    pub span: Span,
    pub kind: ExprKind,
}

/// Literal; số/chuỗi/ký tự giữ văn bản nguồn nguyên văn (hậu tố nằm trong đó).
/// `negative`: dấu `-` ngay trước literal số đã được gấp vào (ví dụ `-128i8`); luôn `false` với bool/chuỗi/ký tự.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lit {
    pub id: NodeId,
    pub span: Span,
    pub negative: bool,
    pub kind: LitKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LitKind {
    Int(String),
    Float(String),
    Str(String),
    Char(String),
    Bool(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    /// `¬`
    Not,
    /// `-`
    Neg,
    /// `*` (giải tham chiếu)
    Deref,
    /// `&`
    Borrow,
    /// `&!`
    BorrowMut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    AddWrap,
    SubWrap,
    MulWrap,
    AddSat,
    SubSat,
    MulSat,
    AddChecked,
    SubChecked,
    MulChecked,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Implies,
    InSet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuantifierKind {
    ForAll,
    Exists,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldName {
    Named(Ident),
    /// Chỉ số tuple `t.0`.
    Index(TupleIndex),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TupleIndex {
    pub id: NodeId,
    pub span: Span,
    pub value: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldInit {
    pub id: NodeId,
    pub span: Span,
    pub name: Ident,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arm {
    pub id: NodeId,
    pub span: Span,
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoopKind {
    /// `⟳ { … }` — vô hạn (cần hiệu ứng `div`).
    Infinite,
    /// `⟳ cond { … }`
    While(Box<Expr>),
    /// `⟳ x in iter { … }`
    ForIn { var: Ident, iter: Box<Expr> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loop {
    pub kind: LoopKind,
    /// `↓ biểu_thức`
    pub variant: Option<Box<Expr>>,
    /// `⋈ biểu_thức` (có thể nhiều)
    pub invariants: Vec<Expr>,
    pub body: Block,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    Lit(Lit),
    Path(Path),
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Cast {
        expr: Box<Expr>,
        ty: Type,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Field {
        base: Box<Expr>,
        name: FieldName,
    },
    /// `e?` — toán tử lan truyền lỗi (CHỈ edition ≥ 2, ADR-P2-09): `Option`/`Result` cùng họ với kiểu trả về của hàm bao
    Try(Box<Expr>),
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    /// `a..b`, `a..=b`, `a..`, `..b`, `..` (dạng mở chỉ có nghĩa làm chỉ số slice).
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
    },
    /// `()` là tuple rỗng; `(a,)` là tuple một phần tử. Ngoặc nhóm `(e)` không có node.
    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    /// `[giá_trị; độ_dài]`
    ArrayRepeat {
        value: Box<Expr>,
        len: Box<Expr>,
    },
    StructLit {
        path: Path,
        fields: Vec<FieldInit>,
    },
    Block(Block),
    /// `else_branch`: khối hoặc `If` khác.
    If {
        cond: Box<Expr>,
        then_block: Block,
        else_branch: Option<Box<Expr>>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<Arm>,
    },
    Loop(Loop),
    Unsafe {
        proofs: Vec<Expr>,
        body: Block,
    },
    /// `∀ x ∈ miền : thân` (chỉ trong hợp đồng).
    Quantifier {
        kind: QuantifierKind,
        var: Ident,
        domain: Box<Expr>,
        body: Box<Expr>,
    },
}

impl_node!(Block, Stmt, Expr, Lit, TupleIndex, FieldInit, Arm);
