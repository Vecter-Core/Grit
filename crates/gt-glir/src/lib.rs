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
    /// con trỏ null (`ptr null` của LLVM): giá trị `ptr` không trỏ vào vùng nào — mọi truy cập qua nó là `Ub` trong thông dịch. Dùng cho
    /// `Vec::new` (`ptr` = null tới lần cấp đầu, như codegen `Intr::VecNew`).
    Null,
    /// hằng `f64` theo MẪU BIT IEEE-754 binary64 (in `f64 0x…` 16 chữ số hex: chính xác, không phụ thuộc bộ đọc số thập phân)
    F64(u64),
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
    /// số nguyên có dấu → `f64`, làm tròn về gần nhất, hoà về chẵn (IEEE-754 roundTiesToEven)
    SIToFP,
    /// số nguyên không dấu → `f64`, làm tròn về gần nhất, hoà về chẵn
    UIToFP,
    /// `f64` → số nguyên có dấu, cắt về 0; giá trị (sau cắt) NGOÀI miền đích hoặc NaN/±∞ là hành vi không xác định (như `poison` của LLVM):
    /// bước hạ PHẢI chặn trước (vd `f64::to_i64` so sánh có thứ tự với ±2^63)
    FPToSI,
    /// đổi kiểu giữ nguyên MẪU BIT (`i64` ↔ `f64`)
    BitCast,
}

/// Phép số học `f64` (ADR-P2-02 quyết định 2026-11-26): IEEE-754 binary64, làm tròn về gần nhất hoà về chẵn, KHÔNG co FMA, KHÔNG cờ fast-math,
/// không xả số dưới chuẩn; MẪU BIT của NaN do phép tính sinh ra KHÔNG được bảo đảm (như LLVM) — thông dịch GLIR chuẩn hoá thành `0x7ff8000000000000`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FBinOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Phép một ngôi `f64`: `neg` chỉ lật bit dấu (giữ nguyên mọi bit khác, kể cả NaN); `sqrt` làm tròn đúng (IEEE-754 squareRoot).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FUnOp {
    Neg,
    Sqrt,
}

/// Vị từ so sánh `f64`: `o*` (có thứ tự) sai khi một bên là NaN; `une` đúng khi khác HOẶC có NaN; `uno` đúng ⇔ có NaN.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FCmpPred {
    Oeq,
    Olt,
    Ole,
    Ogt,
    Oge,
    Une,
    Uno,
}

/// Thứ tự bộ nhớ của lệnh nguyên tử (ADR-P2-02 quyết định 2026-11-26), cùng tên/ý nghĩa với LLVM (`AtomicOrdering` của Grit: `Relaxed` = `monotonic`).
/// Thông dịch GLIR chạy TUẦN TỰ (một luồng) nên thứ tự chỉ được GHI LẠI và kiểm hợp lệ (verifier), không ảnh hưởng kết quả; backend phải phát đúng nó.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicOrd {
    Monotonic,
    Acquire,
    Release,
    AcqRel,
    SeqCst,
}

/// Phép đọc-sửa-ghi nguyên tử: `xchg` (ghi giá trị mới), `add`/`sub` (bao quanh); kết quả là giá trị CŨ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RmwOp {
    Xchg,
    Add,
    Sub,
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
    /// `%d = fbin <op> f64 a, b` (xem [`FBinOp`])
    FBin {
        d: String,
        op: FBinOp,
        ty: Ty,
        a: Opd,
        b: Opd,
    },
    /// `%d = funop <op> f64 v` (xem [`FUnOp`])
    FUn {
        d: String,
        op: FUnOp,
        ty: Ty,
        v: Opd,
    },
    /// `%d = fcmp <pred> f64 a, b` ⇒ `i1`
    FCmp {
        d: String,
        pred: FCmpPred,
        ty: Ty,
        a: Opd,
        b: Opd,
    },
    /// `%d = atomicload <ord> <ty> ptr` — đọc nguyên tử (ord ≠ `release`/`acq_rel`)
    AtomicLoad {
        d: String,
        ord: AtomicOrd,
        ty: Ty,
        ptr: Opd,
    },
    /// `atomicstore <ord> <ty> val, ptr` — ghi nguyên tử (ord ≠ `acquire`/`acq_rel`)
    AtomicStore {
        ord: AtomicOrd,
        ty: Ty,
        val: Opd,
        ptr: Opd,
    },
    /// `%d = atomicrmw <op> <ord> <ty> ptr, val` ⇒ giá trị CŨ
    AtomicRmw {
        d: String,
        op: RmwOp,
        ord: AtomicOrd,
        ty: Ty,
        ptr: Opd,
        val: Opd,
    },
    /// `%d = cmpxchg <succ> <fail> <ty> ptr, cmp, new` (MẠNH: không thất bại giả) ⇒ giá trị CŨ; thành công ⇔ cũ = `cmp` (người dùng tự `icmp eq`).
    /// Tương đương `cmpxchg` của LLVM lấy trường 0; `fail` ≠ `release`/`acq_rel`.
    CmpXchg {
        d: String,
        succ: AtomicOrd,
        fail: AtomicOrd,
        ty: Ty,
        ptr: Opd,
        cmp: Opd,
        new: Opd,
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
