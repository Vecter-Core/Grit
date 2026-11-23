//! Biểu diễn MIR (ADR-020 §2, §9). Chỉ mục là newtype trên `u32`; mọi bảng là `Vec` — không `HashMap` (tất định).
//! Không có cạnh unwind, không có khối cleanup (D3, panic = abort).
use gt_ast::{Expr, NodeId};
use gt_resolve::{DefId, LocalId};
use gt_span::Span;
use gt_types::{FloatTy, IntTy, Ty};

macro_rules! idx {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u32);
        impl $name {
            #[must_use]
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

idx!(
    /// Biến cục bộ. `_0` là chỗ trả về, `_1.._n` là tham số.
    Local
);
idx!(
    /// Khối cơ bản; `bb0` là entry.
    BasicBlock
);
idx!(
    /// Định danh khoản mượn (mỗi `Rvalue::Ref` một cái).
    BorrowId
);
idx!(
    /// Biến region (M6 suy luận; MIR chỉ cấp chỗ).
    RegionVid
);
idx!(
    /// Phạm vi nguồn (cho `⚠`).
    ScopeId
);

pub const RETURN_PLACE: Local = Local(0);
pub const ENTRY: BasicBlock = BasicBlock(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyKind {
    Fn,
    Const,
    Static,
    /// thân của một hợp đồng `⊳` (tiền điều kiện): hàm thuần `(tham số) → bool`
    Pre,
    /// thân của một hợp đồng `⊲`: `(tham số, result) → bool`
    Post,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body {
    /// Đường dẫn đủ tên (vd `main::f`, `main::S::get`, `main::<S as T>::m`).
    pub key: String,
    pub node: NodeId,
    /// Nút của hàm/thành viên mà chữ ký (và `RegionSig`) của thân này lấy từ đó: = `node` với hàm/const/static; với thân hợp đồng
    /// `⊳`/`⊲` là hàm chứa hợp đồng (FB-1: tra `RegionSig` theo nút hợp đồng làm borrowck báo ICE giả).
    pub sig_node: NodeId,
    pub kind: BodyKind,
    pub arg_count: usize,
    pub locals: Vec<LocalDecl>,
    pub blocks: Vec<BlockData>,
    pub borrows: Vec<BorrowData>,
    pub scopes: Vec<SourceScope>,
    pub region_count: u32,
    /// Hợp đồng `⊳ ⊲ ↓ ⋈` và chứng cứ của `⚠`: chỉ ghi nhận, hạ ở M7 (ADR-020 §2, Q29).
    pub contracts: Vec<NodeId>,
    /// Lối "rơi khỏi cuối hàm" khi kiểu trả về ≠ `()` (để kiểm E0307): khối và vị trí cuối thân hàm.
    pub fall_off: Option<(BasicBlock, Span)>,
    /// Vị trí thân hàm (cho chẩn đoán).
    pub span: Span,
    /// Literal nguyên ngoài phạm vi kiểu của nó (ADR-025 Q39): M5 cắt bit nên thông tin này sẽ mất; M7 báo E0601 từ đây, KHÔNG phụ
    /// thuộc đường tới được. Sắp theo vị trí, không trùng nút.
    pub bad_literals: Vec<BadLiteral>,
    /// nghĩa vụ trong hợp đồng mà M5 không hạ được thành kiểm lúc chạy (vd lượng từ không biên): M7 báo E0902 (D25)
    pub uncheckable: Vec<Uncheckable>,
    /// Mọi vòng lặp của thân (M8, ADR-026): đầu vòng (đích của cạnh lùi), hình dạng, biến thể `↓` và điều kiện (bản sao AST) để M8 kiểm dừng.
    pub loops: Vec<LoopInfo>,
    /// Ánh xạ định danh biến của resolver → local MIR (tham số và `≔`), để dịch biểu thức biến thể từ AST.
    pub var_map: Vec<(LocalId, Local)>,
    /// Đã qua elaboration drop (ADR-C18: bỏ drop của chỗ chắc chắn đã move, cờ drop cho chỗ có thể đã move, drop mở theo trường khi move
    /// một phần). Backend CHỈ chạy drop glue có `drop` người dùng trên thân đã elaboration — thân chưa elaboration thì lỗi nội bộ, không đoán.
    pub drops_elaborated: bool,
}

/// Hình dạng vòng lặp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopShape {
    Infinite,
    While,
    /// `⟳ x in a..b` (số phần tử còn lại hữu hạn, tự giảm)
    ForRange,
    /// `⟳ x in &xs` (hữu hạn, tự giảm)
    ForSlice,
}

/// Một vòng lặp trong thân.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopInfo {
    /// khối đầu vòng: đích của MỌI cạnh lùi (và của cạnh vào lần đầu)
    pub head: BasicBlock,
    pub shape: LoopShape,
    pub node: NodeId,
    pub span: Span,
    pub variant: Option<Expr>,
    pub cond: Option<Expr>,
}

/// Một thành phần hợp đồng không kiểm được lúc chạy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Uncheckable {
    pub node: NodeId,
    pub span: Span,
    pub why: String,
}

/// Một literal nguyên không vừa kiểu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BadLiteral {
    pub node: NodeId,
    pub span: Span,
    pub ty: gt_types::IntTy,
    pub negative: bool,
    /// độ lớn (không dấu) của literal
    pub magnitude: u128,
}

impl Body {
    #[must_use]
    pub fn local(&self, l: Local) -> &LocalDecl {
        &self.locals[l.index()]
    }

    #[must_use]
    pub fn block(&self, b: BasicBlock) -> &BlockData {
        &self.blocks[b.index()]
    }

    #[must_use]
    pub fn return_ty(&self) -> &Ty {
        &self.locals[0].ty
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalDecl {
    pub ty: Ty,
    /// Một biến region cho MỖI vị trí region của `ty` (thứ tự duyệt trước, quy ước của `RegionSig`).
    pub regions: Vec<RegionVid>,
    pub kind: LocalKind,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalKind {
    Return,
    Arg(Option<String>),
    User { name: String, id: LocalId },
    Temp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceScope {
    pub parent: Option<ScopeId>,
    /// Bên trong `⚠` (M7 dùng cho E1001).
    pub unsafe_ctx: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceInfo {
    pub span: Span,
    pub node: NodeId,
    pub scope: ScopeId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockData {
    pub statements: Vec<Statement>,
    /// `None` chỉ trong lúc dựng; validator đòi `Some` ở mọi khối.
    pub terminator: Option<Terminator>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement {
    pub kind: StatementKind,
    pub source: SourceInfo,
}

// `Assign` lớn hơn các biến thể khác; giữ không đóng hộp để khớp mẫu (`match`) ở mọi pass đọc MIR đơn giản — chi phí chỉ là
// bộ nhớ của `Vec<Statement>`, không ảnh hưởng tính đúng.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementKind {
    Assign(Place, Rvalue),
    StorageLive(Local),
    StorageDead(Local),
    /// "Đọc giả": giữ chỗ được coi là đã dùng ở đây (scrutinee của `⌥`, vế khởi tạo `≔`) cho M6.
    FakeRead(FakeReadCause, Place),
    /// Xoá (ghi 0, không đọc) toàn bộ byte của chỗ nếu kiểu SAU KHI THẾ chứa `Secret` (ADR-P2-03); không có thì không làm gì. Elaboration chèn
    /// ngay sau mỗi move ra khỏi chỗ có thể chứa Secret; chỗ đã chưa khởi tạo nên không phải một lần "dùng"/"ghi" đối với M6/M7.
    Wipe(Place),
    Nop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FakeReadCause {
    MatchScrutinee,
    LetInit,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Place {
    pub local: Local,
    pub projection: Vec<PlaceElem>,
}

impl Place {
    #[must_use]
    pub fn local(l: Local) -> Place {
        Place {
            local: l,
            projection: Vec::new(),
        }
    }

    #[must_use]
    pub fn project(&self, e: PlaceElem) -> Place {
        let mut p = self.clone();
        p.projection.push(e);
        p
    }

    /// Có đi qua `Deref` không (ghi qua tham chiếu/con trỏ)?
    #[must_use]
    pub fn has_deref(&self) -> bool {
        self.projection
            .iter()
            .any(|e| matches!(e, PlaceElem::Deref))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlaceElem {
    Deref,
    /// Trường thứ `i` (struct/tuple, hoặc của biến thể sau `Downcast`) và kiểu của nó.
    Field(u32, Ty),
    /// Phần tử theo chỉ số (chỉ số là một local `usize`; kiểm biên bằng `Assert` trước đó).
    Index(Local),
    /// Xem enum như biến thể thứ `i`.
    Downcast(u32),
    /// Lát con `[from..to]` (`to = None`: tới hết) của một slice/mảng; kiểm biên bằng `Assert` trước đó.
    Subslice {
        from: Local,
        to: Option<Local>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    Copy(Place),
    Move(Place),
    Const(Const),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Const {
    /// Mẫu bit (bù hai, đã cắt theo độ rộng của `ty`).
    Int {
        bits: u128,
        ty: IntTy,
    },
    /// Văn bản literal nguồn (tránh so sánh số thực).
    Float {
        text: String,
        ty: FloatTy,
    },
    Bool(bool),
    Char(u32),
    /// Literal chuỗi (đã giải escape), kiểu `&'static str`.
    Str(String),
    Unit,
    /// Hàm dùng như giá trị (đã thế tham số kiểu).
    FnItem {
        node: NodeId,
        generics: Vec<Ty>,
        ty: Ty,
    },
    /// Giá trị của `□`/`■` (thân MIR riêng của nó).
    Item {
        node: NodeId,
        ty: Ty,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BorrowKind {
    Shared,
    Unique,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BorrowData {
    pub place: Place,
    pub kind: BorrowKind,
    pub region: RegionVid,
    pub source: SourceInfo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BinOp {
    /// `+ - * / %` — có nghĩa vụ (D2): phép thường đi sau `Assert` (dạng `Checked*` cho tràn).
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
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// `∧` không ngắn mạch giữa hai bool đã tính (chỉ bước hạ MIR dùng, khi hai vế không có tác dụng phụ).
    BitAnd,
}

impl BinOp {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            BinOp::Add => "Add",
            BinOp::Sub => "Sub",
            BinOp::Mul => "Mul",
            BinOp::Div => "Div",
            BinOp::Rem => "Rem",
            BinOp::AddWrap => "AddWrap",
            BinOp::SubWrap => "SubWrap",
            BinOp::MulWrap => "MulWrap",
            BinOp::AddSat => "AddSat",
            BinOp::SubSat => "SubSat",
            BinOp::MulSat => "MulSat",
            BinOp::Eq => "Eq",
            BinOp::Ne => "Ne",
            BinOp::Lt => "Lt",
            BinOp::Le => "Le",
            BinOp::Gt => "Gt",
            BinOp::Ge => "Ge",
            BinOp::BitAnd => "BitAnd",
        }
    }

    #[must_use]
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Not,
    /// Phủ định số học (với số nguyên có dấu: đi sau `Assert(x ≠ MIN)`).
    Neg,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AggregateKind {
    Tuple,
    Array(Ty),
    /// ADT: định nghĩa, chỉ số biến thể (0 cho struct), đối số kiểu.
    Adt(DefId, u32, Vec<Ty>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rvalue {
    Use(Operand),
    Repeat(Operand, u64),
    Ref(BorrowId, BorrowKind, Place),
    /// Độ dài của mảng/slice tại chỗ này (`usize`).
    Len(Place),
    Cast(Operand, Ty),
    /// `true` nếu ép `operand as ty` không mất thông tin (điều kiện của `Assert` loại `CastLossy`).
    CastFits(Operand, Ty),
    BinaryOp(BinOp, Operand, Operand),
    /// `(kết quả bao quanh, có tràn)` — điều kiện của `Assert` loại `Overflow`.
    CheckedBinaryOp(BinOp, Operand, Operand),
    UnaryOp(UnOp, Operand),
    /// Chỉ số biến thể của enum (`u32`... biểu diễn bằng `usize`).
    Discriminant(Place),
    Aggregate(AggregateKind, Vec<Operand>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssertKind {
    Overflow(BinOp, Operand, Operand),
    OverflowNeg(Operand),
    DivisionByZero(Operand),
    RemainderByZero(Operand),
    IndexOutOfBounds {
        len: Operand,
        index: Operand,
    },
    /// Lát con `[from..to]` phải có `from ≤ to ≤ len`.
    SliceRange {
        from: Operand,
        to: Operand,
        len: Operand,
    },
    CastLossy {
        value: Operand,
        to: Ty,
    },
    /// kiểm `⊳` tại điểm gọi (M7 chèn khi không chứng minh được; M5 không phát)
    Precondition,
    /// kiểm `⊲` ở điểm thoát (M7 chèn)
    Postcondition,
    /// `⋈ P` ở đầu vòng lặp (M5 phát; chỉ runtime ở Phase 1, M7 xoá nếu chứng minh được)
    Invariant,
    /// `core::fuel_left(n)`: nhiên liệu của vòng lặp kiểu "nhiên liệu" phải còn `n > 0` khi thoát vòng; cạn ⇒ abort `fuel_exhausted`
    Fuel,
}

impl AssertKind {
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            AssertKind::Overflow(..) => "overflow",
            AssertKind::OverflowNeg(_) => "overflow_neg",
            AssertKind::DivisionByZero(_) => "div_by_zero",
            AssertKind::RemainderByZero(_) => "rem_by_zero",
            AssertKind::IndexOutOfBounds { .. } => "index_out_of_bounds",
            AssertKind::SliceRange { .. } => "slice_range",
            AssertKind::CastLossy { .. } => "cast_lossy",
            AssertKind::Precondition => "precondition",
            AssertKind::Postcondition => "postcondition",
            AssertKind::Invariant => "invariant",
            AssertKind::Fuel => "fuel_exhausted",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Callee {
    Fn {
        node: NodeId,
        generics: Vec<Ty>,
    },
    TraitMethod {
        trait_def: DefId,
        member: usize,
        self_ty: Ty,
        generics: Vec<Ty>,
    },
    Value(Operand),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Terminator {
    pub kind: TerminatorKind,
    pub source: SourceInfo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminatorKind {
    Goto(BasicBlock),
    SwitchInt {
        discr: Operand,
        targets: Vec<(u128, BasicBlock)>,
        otherwise: BasicBlock,
    },
    Return,
    /// Không tới được DO CẤU TẠO. M10 PHẢI hạ thành abort (trap), không phải `unreachable` của LLVM (ADR-020 §9).
    Unreachable,
    Drop {
        place: Place,
        target: BasicBlock,
    },
    /// `target = None`: callee trả `!` (không có khối sau, không có cạnh unwind).
    Call {
        callee: Callee,
        args: Vec<Operand>,
        arg_spans: Vec<Span>,
        dest: Place,
        target: Option<BasicBlock>,
    },
    /// `cond != expected` ⇒ abort tức thì (D3).
    Assert {
        cond: Operand,
        expected: bool,
        kind: AssertKind,
        target: BasicBlock,
    },
}

impl TerminatorKind {
    /// Các khối kế tiếp, theo thứ tự cố định.
    #[must_use]
    pub fn successors(&self) -> Vec<BasicBlock> {
        match self {
            TerminatorKind::Goto(b) => vec![*b],
            TerminatorKind::SwitchInt {
                targets, otherwise, ..
            } => {
                let mut v: Vec<BasicBlock> = targets.iter().map(|(_, b)| *b).collect();
                v.push(*otherwise);
                v
            }
            TerminatorKind::Return | TerminatorKind::Unreachable => Vec::new(),
            TerminatorKind::Drop { target, .. } | TerminatorKind::Assert { target, .. } => {
                vec![*target]
            }
            TerminatorKind::Call { target, .. } => target.iter().copied().collect(),
        }
    }
}

/// Vị trí trong thân: (khối, chỉ số câu lệnh; `== statements.len()` là terminator).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location {
    pub block: BasicBlock,
    pub statement: usize,
}
