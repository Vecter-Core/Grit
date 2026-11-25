//! GLIR (ADR-P2-02, thiết kế dạng LAI theo cố vấn Opus 2026-11-24): giá trị tạm là SSA `%v`, bộ nhớ là slot có kiểu `$s` (tương ứng `alloca`).
//! C1: kiểu dữ liệu + in + đọc + verifier. CHƯA có đường hạ MIR→GLIR và GLIR→LLVM. Không có `Unreachable`/`Assert` trong tập lệnh:
//! mọi thất bại là `Abort(loại)` (terminator) ⇒ bất biến "không còn Assert/unreachable" đúng DO CẤU TẠO.
pub mod interp;
pub mod layout;
pub mod parse;
pub mod print;
pub mod verify;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ty {
    I1,
    I8,
    I16,
    I32,
    I64,
    I128,
    F64,
    Ptr,
    Void,
    Struct(Vec<Ty>),
    Array(Box<Ty>, u64),
}

impl Ty {
    #[must_use]
    pub fn is_int(&self) -> bool {
        matches!(
            self,
            Ty::I1 | Ty::I8 | Ty::I16 | Ty::I32 | Ty::I64 | Ty::I128
        )
    }
    #[must_use]
    pub fn bits(&self) -> Option<u32> {
        Some(match self {
            Ty::I1 => 1,
            Ty::I8 => 8,
            Ty::I16 => 16,
            Ty::I32 => 32,
            Ty::I64 => 64,
            Ty::I128 => 128,
            _ => return None,
        })
    }
}

/// Toán hạng: giá trị SSA, slot (là một `ptr`), hoặc hằng nguyên có kiểu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opd {
    Val(String),
    Slot(String),
    Int(Ty, i128),
    /// địa chỉ (ptr) của một global bất biến (chuỗi hằng…)
    Global(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    UDiv,
    SDiv,
    URem,
    SRem,
    And,
    Or,
    Xor,
    Shl,
    LShr,
    AShr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpPred {
    Eq,
    Ne,
    Ult,
    Ule,
    Ugt,
    Uge,
    Slt,
    Sle,
    Sgt,
    Sge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastOp {
    ZExt,
    SExt,
    Trunc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inst {
    Bin {
        d: String,
        op: BinOp,
        ty: Ty,
        a: Opd,
        b: Opd,
    },
    ICmp {
        d: String,
        pred: CmpPred,
        ty: Ty,
        a: Opd,
        b: Opd,
    },
    Select {
        d: String,
        ty: Ty,
        c: Opd,
        a: Opd,
        b: Opd,
    },
    Cast {
        d: String,
        op: CastOp,
        from: Ty,
        v: Opd,
        to: Ty,
    },
    Load {
        d: String,
        ty: Ty,
        ptr: Opd,
    },
    Store {
        ty: Ty,
        val: Opd,
        ptr: Opd,
    },
    FieldPtr {
        d: String,
        ty: Ty,
        ptr: Opd,
        idx: u32,
    },
    /// `ptr + idx × cỡ(elem)` (idx: i64); kiểm biên do `Assert` của MIR đảm nhận (tương ứng `inbounds gep`)
    IndexPtr {
        d: String,
        elem: Ty,
        ptr: Opd,
        idx: Opd,
    },
    Call {
        d: Option<String>,
        ret: Ty,
        callee: String,
        args: Vec<Opd>,
    },
    Phi {
        d: String,
        ty: Ty,
        ins: Vec<(Opd, String)>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    Br(String),
    CondBr(Opd, String, String),
    Ret(Option<(Ty, Opd)>),
    /// loại abort (`bounds`, `overflow`, …): thay cho cặp `call gritrt_abort` + `unreachable`
    Abort(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub name: String,
    pub insts: Vec<Inst>,
    pub term: Term,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Func {
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    pub slots: Vec<(String, Ty)>,
    pub blocks: Vec<Block>,
}

/// Dữ liệu bất biến trong module (chuỗi hằng): `bytes` đặt ở một vùng chỉ-đọc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Global {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Khai báo hàm ngoài (runtime) mà GLIR được phép gọi.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decl {
    pub name: String,
    pub params: Vec<Ty>,
    pub ret: Ty,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Module {
    pub globals: Vec<Global>,
    pub decls: Vec<Decl>,
    pub funcs: Vec<Func>,
}

pub use parse::parse;
pub use print::print;
pub use verify::verify;
