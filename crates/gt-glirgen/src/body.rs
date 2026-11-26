//! Hạ MỘT thân MIR (`BodyKind::Fn`, hoặc thân hợp đồng `Pre`/`Post` được gọi tới; tập con vô hướng) sang hàm GLIR (ADR-P2-02 C3; thiết kế theo cố vấn Opus 2026-11-25).
//! Mỗi local MIR là một slot `$lN` có kiểu; mọi lần dùng là `load`, mọi lần ghi là `store` (khớp `alloca` của codegen LLVM; mem2reg để sau).
//! `Assert` ⇒ `condbr` tới khối đích hoặc một khối riêng chỉ chứa `abort <loại>` (một khối cho MỖI Assert). Mọi thứ ngoài tập con ⇒ `Unsupported(lý do)`.
//! Tập con: kiểu `Int`(≤ 64 bit)/`Bool`/`()`/tuple `(Int, Bool)`; `Use`, `BinaryOp`(số học bao quanh, so sánh, `BitAnd`, `Div/Rem`), `CheckedBinaryOp`(Add/Sub/Mul),
//! `UnaryOp`, `Cast` số nguyên/bool→số nguyên; `Goto`/`SwitchInt`/`Return`/`Assert`/`Unreachable`.
//! Thân generic hạ theo INSTANCE (một hàm GLIR cho mỗi `Instance{node,args}`; thân không generic = instance `args=[]`): `Lw::sub` thế tham số ở mọi
//! chỗ đọc `Ty` của MIR (KHÔNG sao chép thân); chỗ sót vẫn fail-closed vì `lower_ty` từ chối `Ty::Param`.
use crate::lower_ty;
use gt_glir::{BinOp as GB, Block, CastOp, CmpPred, Func, Inst, Opd, Term, Ty as G};
use gt_mir::{
    AssertKind, BinOp, Body, BodyKind, Const, Operand, Place, PlaceElem, Rvalue, StatementKind,
    TerminatorKind, UnOp,
};
use gt_types::{Instance, IntTy, Ty, TypeDb};
use std::cell::RefCell;
use std::collections::BTreeMap;

/// Lý do (ngắn, ổn định) một thân chưa hạ được — dùng để đếm độ phủ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unsupported(pub String);

type R<T> = Result<T, Unsupported>;

fn un<T>(why: impl Into<String>) -> R<T> {
    Err(Unsupported(why.into()))
}

/// instance → (tên GLIR, kiểu trả về Grit ĐÃ THẾ) nếu chữ ký (đã thế) nằm trong tập con, `None` nếu không — bộ nhớ đệm, điền dần khi gặp lời gọi.
pub type Sigs = BTreeMap<Instance, Option<(String, Ty)>>;

/// Ngữ cảnh chung khi hạ cả tập thân.
struct Ctx<'a> {
    db: &'a TypeDb,
    keys: &'a BTreeMap<gt_ast::NodeId, String>,
    defs: &'a [gt_resolve::Def],
    /// thân `Fn` theo nút
    bodies: BTreeMap<gt_ast::NodeId, &'a Body>,
    sigs: RefCell<Sigs>,
}

impl Ctx<'_> {
    /// chữ ký của instance nếu có thân MIR và chữ ký nằm trong tập con
    fn sig(&self, inst: &Instance) -> Option<(String, Ty)> {
        if let Some(s) = self.sigs.borrow().get(inst) {
            return s.clone();
        }
        let r = self.bodies.get(&inst.node).and_then(|b| {
            let ps = self.db.fn_params(b.sig_node);
            if ps.len() != inst.args.len()
                || inst.args.iter().any(|a| a.has_param() || a.has_infer())
            {
                return None;
            }
            let sub = |t: &Ty| subst_inst(t, &ps, &inst.args);
            (0..=b.arg_count)
                .all(|i| in_subset(self.db, &sub(&b.locals[i].ty)))
                .then(|| {
                    (
                        inst_name(&b.key, &inst.args, self.defs),
                        sub(&b.locals[0].ty),
                    )
                })
        });
        self.sigs.borrow_mut().insert(inst.clone(), r.clone());
        r
    }
}

/// Thế tham số kiểu của instance (`params[i]` ↦ `args[i]`) vào `ty` — đúng `subst_inst` của codegen LLVM.
fn subst_inst(ty: &Ty, params: &[gt_types::ParamId], args: &[Ty]) -> Ty {
    ty.subst(&|p| {
        params
            .iter()
            .position(|q| *q == p)
            .and_then(|i| args.get(i).cloned())
    })
}

struct Lw<'a> {
    cx: &'a Ctx<'a>,
    db: &'a TypeDb,
    body: &'a Body,
    /// tham số kiểu của thân (impl rồi hàm) và đối số của instance đang hạ
    params: Vec<gt_types::ParamId>,
    args: Vec<Ty>,
    /// kiểu ĐÃ THẾ của mọi local
    local_tys: Vec<Ty>,
    slots: BTreeMap<usize, G>,
    blocks: Vec<Block>,
    cur: Vec<Inst>,
    cur_name: String,
    tmp: u32,
    extra: u32,
    /// slot phụ (dựng con trỏ béo của chuỗi hằng…)
    aux_slots: Vec<(String, G)>,
    /// global (chuỗi hằng) mà thân này dùng
    globals: Vec<gt_glir::Global>,
    /// instance mà thân này gọi trực tiếp
    calls: Vec<Instance>,
}

fn is_int_or_bool(t: &Ty) -> bool {
    match t {
        Ty::Int(it) => it.bits() <= 64,
        Ty::Bool | Ty::Char | Ty::Never => true,
        _ => false,
    }
}

fn plain_struct(db: &TypeDb, def: &gt_resolve::DefId) -> bool {
    db.adts
        .get(def)
        .is_some_and(|a| matches!(a.kind, gt_types::AdtKind::Struct(_)) && a.lang.is_none())
}

/// `Arena` (lang, `{x: i32}` = tay cầm runtime): chỉ được tạo/dùng qua lời gọi runtime (`Arena::new/alloc/reset`)
fn is_arena(db: &TypeDb, t: &Ty) -> bool {
    matches!(t, Ty::Adt(def, args) if args.is_empty()
        && db.adts.get(def).is_some_and(|a| a.lang == Some(gt_types::Lang::Arena)))
}

/// `Cap<T>` (lang): giá trị chỉ là bằng chứng quyền (D29); `root_*` tạo giá trị ZERO (như `Intr::RootCap` của codegen)
fn is_cap(db: &TypeDb, t: &Ty) -> bool {
    matches!(t, Ty::Adt(def, _) if db.adts.get(def).is_some_and(|a| a.lang == Some(gt_types::Lang::Cap)))
}

/// `Cell<T>` (lang, `{v: T}`): chỉ đọc/ghi qua `Cell::new/get/set` (như `Intr::Cell*` của codegen: chèn/đọc/ghi trường 0)
fn is_cell(db: &TypeDb, t: &Ty) -> bool {
    matches!(t, Ty::Adt(def, _) if db.adts.get(def).is_some_and(|a| a.lang == Some(gt_types::Lang::Cell)))
}

/// `Vec<'a, T>` (lang, `{arena: &Arena, ptr: *μ T, len: usize, cap: usize}`): kiểu phần tử `T` (đọc từ trường `ptr` — không phụ thuộc cách
/// đối số lifetime nằm trong `args`); chỉ được tạo/dùng qua intrinsic `Vec::*` (như `Intr::Vec*` của codegen).
fn vec_elem(db: &TypeDb, t: &Ty) -> Option<Ty> {
    let Ty::Adt(def, args) = t else { return None };
    if db
        .adts
        .get(def)
        .is_none_or(|a| a.lang != Some(gt_types::Lang::Vec))
    {
        return None;
    }
    match crate::variant_fields(db, *def, args, 0).ok()?.as_slice() {
        [_, Ty::RawPtr(_, e), _, _] => Some((**e).clone()),
        _ => None,
    }
}

/// `Text<'a>` (lang, `{bytes: Vec<'a, u8>}`): kiểu trường `bytes` (chỉ tạo/dùng qua intrinsic `Text::*`, như `Intr::Text*` của codegen)
fn text_vec(db: &TypeDb, t: &Ty) -> Option<Ty> {
    let Ty::Adt(def, args) = t else { return None };
    if db
        .adts
        .get(def)
        .is_none_or(|a| a.lang != Some(gt_types::Lang::Text))
    {
        return None;
    }
    match crate::variant_fields(db, *def, args, 0).ok()?.as_slice() {
        [v] if vec_elem(db, v) == Some(Ty::Int(IntTy::U8)) => Some(v.clone()),
        _ => None,
    }
}

/// `Owned<T>` (lang, `{ptr: *μ T}`): kiểu `T` (chỉ tạo/dùng qua intrinsic `Owned::*` và drop glue `Free`, như codegen)
fn owned_elem(db: &TypeDb, t: &Ty) -> Option<Ty> {
    let Ty::Adt(def, args) = t else { return None };
    if db
        .adts
        .get(def)
        .is_none_or(|a| a.lang != Some(gt_types::Lang::Owned))
    {
        return None;
    }
    match crate::variant_fields(db, *def, args, 0).ok()?.as_slice() {
        [Ty::RawPtr(_, e)] => Some((**e).clone()),
        _ => None,
    }
}

/// Khai báo của hàm runtime mà GLIR được phép gọi: tên và chữ ký ĐÚNG như codegen LLVM phát (`gt-codegen-llvm` `Intr::Arena*`; bảng `RUNTIME`
/// của gt-guard). Tên ngoài bảng ⇒ `None` (không hạ).
#[must_use]
pub fn runtime_decl(name: &str) -> Option<gt_glir::Decl> {
    let (params, ret) = match name {
        "gritrt_arena_new" => (Vec::new(), G::I32),
        "gritrt_arena_alloc" => (vec![G::I32, G::I64, G::I64], G::Ptr),
        "gritrt_arena_reset" => (vec![G::I32], G::Void),
        "gritrt_utf8_valid" => (vec![G::Ptr, G::I64], G::I32),
        "gritrt_heap_alloc" => (vec![G::I64, G::I64], G::Ptr),
        "gritrt_heap_free" => (vec![G::Ptr, G::I64, G::I64], G::Void),
        "gritrt_secret_wipe" => (vec![G::Ptr, G::I64], G::Void),
        _ => return None,
    };
    Some(gt_glir::Decl {
        name: name.to_string(),
        params,
        ret,
    })
}

/// Trần độ sâu đệ quy của `in_sub` (bảo vệ ngăn xếp): ≥ `MAX_INSTANCE_DEPTH` của đơn hình hoá cộng phần lồng của trường ADT — instance hợp lệ
/// sâu tới 64 mức (mẫu `e0317_*`: tuple lồng 12 mức) không bị từ chối chỉ vì độ sâu.
const SUBSET_MAX_DEPTH: u32 = 2 * gt_types::MAX_INSTANCE_DEPTH;
/// Ngân sách số nút kiểu mà MỘT lần kiểm `in_subset` được duyệt: chặn bùng nổ hàm mũ (ADT rộng lồng nhau `S{a: S', b: S'}`…) và chu trình qua
/// tham chiếu (`▢ N { next: &N }` — duyệt vô hạn ⇒ hết ngân sách ⇒ ngoài tập con, fail-closed như trần độ sâu 8 cũ).
const SUBSET_MAX_NODES: u32 = 4096;

fn in_subset(db: &TypeDb, t: &Ty) -> bool {
    let mut budget = SUBSET_MAX_NODES;
    in_sub(db, t, 0, &mut budget)
}

fn in_sub(db: &TypeDb, t: &Ty, depth: u32, budget: &mut u32) -> bool {
    if depth > SUBSET_MAX_DEPTH || *budget == 0 {
        return false;
    }
    *budget -= 1;
    let mut rec = |x: &Ty| in_sub(db, x, depth + 1, budget);
    match t {
        Ty::Tuple(v) => v.iter().all(rec),
        Ty::Ref(_, inner) => match &**inner {
            Ty::Str => true,
            Ty::Slice(e) => rec(e),
            i => rec(i),
        },
        Ty::Array(e, n) => *n <= 4096 && rec(e),
        Ty::Adt(def, args) => {
            if is_arena(db, t) {
                return true;
            }
            if let Some(e) = vec_elem(db, t) {
                return rec(&e);
            }
            if let Some(v) = text_vec(db, t) {
                return rec(&v);
            }
            if let Some(e) = owned_elem(db, t) {
                return rec(&e);
            }
            if is_cap(db, t) || is_cell(db, t) || plain_struct(db, def) {
                return crate::variant_fields(db, *def, args, 0).is_ok_and(|fs| fs.iter().all(rec));
            }
            match db.adts.get(def).map(|a| &a.kind) {
                Some(gt_types::AdtKind::Enum(vs)) => (0..vs.len()).all(|v| {
                    crate::variant_fields(db, *def, args, v).is_ok_and(|fs| fs.iter().all(&mut rec))
                }),
                _ => false,
            }
        }
        t => is_int_or_bool(t),
    }
}

fn gint(it: IntTy) -> G {
    match it.bits() {
        8 => G::I8,
        16 => G::I16,
        32 => G::I32,
        _ => G::I64,
    }
}

fn gty(db: &TypeDb, t: &Ty) -> R<G> {
    if !in_subset(db, t) {
        return un(format!("kiểu {}", kind_name(t)));
    }
    lower_ty(db, t).map_err(Unsupported)
}

fn kind_name(t: &Ty) -> &'static str {
    match t {
        Ty::Int(_) => "int128",
        Ty::Float(_) => "float",
        Ty::Ref(..) | Ty::RawPtr(..) => "con trỏ",
        Ty::Adt(..) => "adt",
        Ty::Array(..) | Ty::Slice(_) | Ty::Str => "mảng/slice",
        Ty::Tuple(_) => "tuple",
        Ty::Fn(..) => "hàm",
        _ => "khác",
    }
}

fn is_unit(t: &Ty) -> bool {
    matches!(t, Ty::Tuple(v) if v.is_empty()) || matches!(t, Ty::Never)
}

impl Lw<'_> {
    /// thế tham số của instance vào một kiểu đọc từ MIR — PHẢI gọi ở mọi chỗ đọc `Ty` của thân (chỗ sót vẫn fail-closed: `lower_ty` từ chối `Ty::Param`)
    fn sub(&self, ty: &Ty) -> Ty {
        subst_inst(ty, &self.params, &self.args)
    }
    fn t(&mut self) -> String {
        self.tmp += 1;
        format!("t{}", self.tmp)
    }
    fn push(&mut self, i: Inst) {
        self.cur.push(i);
    }
    fn finish(&mut self, term: Term, next: Option<String>) {
        let name = next.unwrap_or_default();
        let insts = std::mem::take(&mut self.cur);
        let done = std::mem::replace(&mut self.cur_name, name);
        self.blocks.push(Block {
            name: done,
            insts,
            term,
        });
    }
    fn slot(&mut self, l: usize) -> R<String> {
        if !self.slots.contains_key(&l) {
            let ty = gty(self.db, &self.local_tys[l])?;
            self.slots.insert(l, ty);
        }
        Ok(format!("l{l}"))
    }
    /// con trỏ tới chỗ (slot, rồi chuỗi trường tuple/struct) + kiểu Grit của chỗ
    fn place(&mut self, p: &Place) -> R<(Opd, Ty)> {
        let (ptr, ty, meta) = self.place_m(p)?;
        if meta.is_some() {
            return un("chỗ không kích thước dùng như chỗ có kích thước");
        }
        Ok((ptr, ty))
    }
    /// như `place`, thêm `meta` (độ dài, i64) khi chỗ là slice/`str` (đứng sau `Deref` của con trỏ béo hoặc `Subslice`)
    fn place_m(&mut self, p: &Place) -> R<(Opd, Ty, Option<Opd>)> {
        let mut meta: Option<Opd> = None;
        let l = p.local.index();
        let mut cur_ty = self.local_tys[l].clone();
        let mut ptr = Opd::Slot(self.slot(l)?);
        let mut downcast: Option<u32> = None;
        for e in &p.projection {
            match e {
                PlaceElem::Field(i, fty) => {
                    let gt = if let Some(v) = downcast.take() {
                        // trường của biến thể `v`: nhìn vùng payload `[k × i64]` như struct của biến thể (đúng cách codegen LLVM làm)
                        let Ty::Adt(def, args) = &cur_ty else {
                            return un("Downcast+Field trên kiểu không phải enum");
                        };
                        let fs = crate::variant_fields(self.db, *def, args, v as usize)
                            .map_err(Unsupported)?;
                        let gfs = fs.iter().map(|t| gty(self.db, t)).collect::<R<Vec<_>>>()?;
                        G::Struct(gfs)
                    } else {
                        if !matches!(cur_ty, Ty::Tuple(_) | Ty::Adt(..)) {
                            return un("trường của kiểu không phải tuple/struct");
                        }
                        if let Ty::Adt(def, _) = &cur_ty
                            && !plain_struct(self.db, def)
                        {
                            return un("trường của enum không qua Downcast");
                        }
                        gty(self.db, &cur_ty)?
                    };
                    let d = self.t();
                    self.push(Inst::FieldPtr {
                        d: d.clone(),
                        ty: gt,
                        ptr,
                        idx: *i,
                    });
                    ptr = Opd::Val(d);
                    cur_ty = self.sub(fty);
                }
                PlaceElem::Deref => {
                    let Ty::Ref(_, inner) = &cur_ty else {
                        return un("Deref của kiểu không phải tham chiếu");
                    };
                    let inner = (**inner).clone();
                    if matches!(inner, Ty::Str | Ty::Slice(_)) {
                        // con trỏ béo `{ptr, i64}`: dữ liệu ở trường 0, độ dài ở trường 1
                        let fat = gty(self.db, &cur_ty)?;
                        let (p0, p1) = (self.t(), self.t());
                        self.push(Inst::FieldPtr {
                            d: p0.clone(),
                            ty: fat.clone(),
                            ptr: ptr.clone(),
                            idx: 0,
                        });
                        self.push(Inst::FieldPtr {
                            d: p1.clone(),
                            ty: fat,
                            ptr,
                            idx: 1,
                        });
                        let (dp, ln) = (self.t(), self.t());
                        self.push(Inst::Load {
                            d: dp.clone(),
                            ty: G::Ptr,
                            ptr: Opd::Val(p0),
                        });
                        self.push(Inst::Load {
                            d: ln.clone(),
                            ty: G::I64,
                            ptr: Opd::Val(p1),
                        });
                        ptr = Opd::Val(dp);
                        meta = Some(Opd::Val(ln));
                        cur_ty = inner;
                        continue;
                    }
                    gty(self.db, &inner)?;
                    let d = self.t();
                    self.push(Inst::Load {
                        d: d.clone(),
                        ty: G::Ptr,
                        ptr,
                    });
                    ptr = Opd::Val(d);
                    cur_ty = inner;
                }
                PlaceElem::Index(il) => {
                    let elem = match &cur_ty {
                        Ty::Array(e, _) | Ty::Slice(e) => (**e).clone(),
                        _ => return un("Index của kiểu không phải mảng/slice"),
                    };
                    let ge = gty(self.db, &elem)?;
                    let (iv, _) = self.load(&Place::local(*il))?;
                    let d = self.t();
                    self.push(Inst::IndexPtr {
                        d: d.clone(),
                        elem: ge,
                        ptr,
                        idx: iv,
                    });
                    ptr = Opd::Val(d);
                    cur_ty = elem;
                    meta = None;
                }
                PlaceElem::Subslice { from, to } => {
                    let (elem, total) = match &cur_ty {
                        Ty::Array(e, n) => ((**e).clone(), Opd::Int(G::I64, i128::from(*n))),
                        Ty::Slice(e) => (
                            (**e).clone(),
                            meta.clone()
                                .ok_or(Unsupported("Subslice của slice không có độ dài".into()))?,
                        ),
                        _ => return un("Subslice của kiểu không phải mảng/slice"),
                    };
                    let ge = gty(self.db, &elem)?;
                    let (fv, _) = self.load(&Place::local(*from))?;
                    let end = match to {
                        Some(t) => self.load(&Place::local(*t))?.0,
                        None => total,
                    };
                    let d = self.t();
                    self.push(Inst::IndexPtr {
                        d: d.clone(),
                        elem: ge,
                        ptr,
                        idx: fv.clone(),
                    });
                    let len = self.bin(GB::Sub, &G::I64, end, fv);
                    ptr = Opd::Val(d);
                    meta = Some(len);
                    cur_ty = Ty::Slice(Box::new(elem));
                }
                PlaceElem::Downcast(v) => {
                    let Ty::Adt(..) = &cur_ty else {
                        return un("Downcast của kiểu không phải enum");
                    };
                    let ge = gty(self.db, &cur_ty)?;
                    let d = self.t();
                    self.push(Inst::FieldPtr {
                        d: d.clone(),
                        ty: ge,
                        ptr,
                        idx: 1,
                    });
                    ptr = Opd::Val(d);
                    downcast = Some(*v);
                }
            }
        }
        if downcast.is_some() {
            return un("Downcast không theo sau bởi Field");
        }
        Ok((ptr, cur_ty, meta))
    }
    fn load(&mut self, p: &Place) -> R<(Opd, Ty)> {
        let (ptr, ty) = self.place(p)?;
        let g = gty(self.db, &ty)?;
        let d = self.t();
        self.push(Inst::Load {
            d: d.clone(),
            ty: g,
            ptr,
        });
        Ok((Opd::Val(d), ty))
    }
    fn operand(&mut self, o: &Operand) -> R<(Opd, Ty)> {
        match o {
            Operand::Copy(p) | Operand::Move(p) => self.load(p),
            Operand::Const(Const::Int { bits, ty }) => {
                if ty.bits() > 64 {
                    return un("hằng 128 bit");
                }
                let w = ty.bits();
                let v = if ty.signed() && (bits >> (w - 1)) & 1 == 1 {
                    (*bits as i128) - (1i128 << w)
                } else {
                    *bits as i128
                };
                Ok((Opd::Int(gint(*ty), v), Ty::Int(*ty)))
            }
            Operand::Const(Const::Bool(b)) => Ok((Opd::Int(G::I1, i128::from(*b)), Ty::Bool)),
            Operand::Const(Const::Char(c)) => Ok((Opd::Int(G::I32, i128::from(*c)), Ty::Char)),
            Operand::Const(Const::Str(text)) => {
                // chuỗi hằng ⇒ global bất biến + con trỏ béo `{@global, len}` dựng trong slot phụ
                let bytes = text.as_bytes().to_vec();
                let name = format!("s.{:016x}", gt_types::fnv1a64(&bytes));
                if !self.globals.iter().any(|g| g.name == name) {
                    self.globals.push(gt_glir::Global {
                        name: name.clone(),
                        bytes: bytes.clone(),
                    });
                }
                let fat = G::Struct(vec![G::Ptr, G::I64]);
                self.extra += 1;
                let sname = format!("str{}", self.extra);
                self.aux_slots.push((sname.clone(), fat.clone()));
                let (p0, p1) = (self.t(), self.t());
                self.push(Inst::FieldPtr {
                    d: p0.clone(),
                    ty: fat.clone(),
                    ptr: Opd::Slot(sname.clone()),
                    idx: 0,
                });
                self.push(Inst::Store {
                    ty: G::Ptr,
                    val: Opd::Global(name),
                    ptr: Opd::Val(p0),
                });
                self.push(Inst::FieldPtr {
                    d: p1.clone(),
                    ty: fat.clone(),
                    ptr: Opd::Slot(sname.clone()),
                    idx: 1,
                });
                self.push(Inst::Store {
                    ty: G::I64,
                    val: Opd::Int(G::I64, bytes.len() as i128),
                    ptr: Opd::Val(p1),
                });
                let d = self.t();
                self.push(Inst::Load {
                    d: d.clone(),
                    ty: fat,
                    ptr: Opd::Slot(sname),
                });
                Ok((Opd::Val(d), Ty::Ref(false, Box::new(Ty::Str))))
            }
            // `()` không có biểu diễn: mọi chỗ ghi/truyền kiểu `()` đều bị bỏ qua (xem `store`, `Call`, `Aggregate`)
            Operand::Const(Const::Unit) => Ok((Opd::Int(G::I1, 0), Ty::Tuple(Vec::new()))),
            Operand::Const(_) => un("hằng không phải số nguyên/bool/()"),
        }
    }
    fn store(&mut self, dest: &Place, v: Opd, ty: &Ty) -> R<()> {
        if is_unit(ty) {
            return Ok(());
        }
        let (ptr, dty) = self.place(dest)?;
        if is_unit(&dty) {
            return Ok(());
        }
        let g = gty(self.db, &dty)?;
        self.push(Inst::Store { ty: g, val: v, ptr });
        Ok(())
    }
    fn bin(&mut self, op: GB, ty: &G, a: Opd, b: Opd) -> Opd {
        let d = self.t();
        self.push(Inst::Bin {
            d: d.clone(),
            op,
            ty: ty.clone(),
            a,
            b,
        });
        Opd::Val(d)
    }
    fn cmp(&mut self, pred: CmpPred, ty: &G, a: Opd, b: Opd) -> Opd {
        let d = self.t();
        self.push(Inst::ICmp {
            d: d.clone(),
            pred,
            ty: ty.clone(),
            a,
            b,
        });
        Opd::Val(d)
    }
    fn cast(&mut self, op: CastOp, from: &G, v: Opd, to: &G) -> Opd {
        let d = self.t();
        self.push(Inst::Cast {
            d: d.clone(),
            op,
            from: from.clone(),
            v,
            to: to.clone(),
        });
        Opd::Val(d)
    }
    /// tay cầm `i32` của Arena mà `p` (kiểu `&Arena`/`&!Arena`) trỏ tới
    fn arena_handle(&mut self, p: Opd) -> Opd {
        let f = self.t();
        self.push(Inst::FieldPtr {
            d: f.clone(),
            ty: G::Struct(vec![G::I32]),
            ptr: p,
            idx: 0,
        });
        let h = self.t();
        self.push(Inst::Load {
            d: h.clone(),
            ty: G::I32,
            ptr: Opd::Val(f),
        });
        Opd::Val(h)
    }

    /// Lời gọi intrinsic runtime (callee KHÔNG có thân MIR) ⇒ `call @gritrt_*` tới một `declare` (đúng như codegen LLVM). `Ok(false)`: không phải intrinsic
    /// được hỗ trợ. Đối số được tính THEO THỨ TỰ trước khi gọi (như `vals` của codegen).
    fn runtime_call(
        &mut self,
        key: &str,
        generics: &[Ty],
        args: &[Operand],
        dest: &Place,
    ) -> R<bool> {
        match key {
            "$prelude::Arena::new" | "$prelude::Cap<Alloc>::arena" => {
                // `Cap<Alloc>::arena(&self)` = `Arena::new()` (codegen `Intr::ArenaNew`): người nhận chỉ được tính (bằng chứng quyền), không dùng
                let want = usize::from(key != "$prelude::Arena::new");
                if args.len() != want || !generics.is_empty() {
                    return un("Arena::new: sai số đối số");
                }
                if let Some(c) = args.first() {
                    let (_, tc) = self.operand(c)?;
                    if !matches!(&tc, Ty::Ref(_, i) if is_cap(self.db, i)) {
                        return un("Cap<Alloc>::arena: người nhận không phải &Cap");
                    }
                }
                let h = self.t();
                self.push(Inst::Call {
                    d: Some(h.clone()),
                    ret: G::I32,
                    callee: "gritrt_arena_new".into(),
                    args: Vec::new(),
                });
                let (ptr, dty) = self.place(dest)?;
                if !is_arena(self.db, &dty) {
                    return un("Arena::new: đích không phải Arena");
                }
                let f = self.t();
                self.push(Inst::FieldPtr {
                    d: f.clone(),
                    ty: G::Struct(vec![G::I32]),
                    ptr,
                    idx: 0,
                });
                self.push(Inst::Store {
                    ty: G::I32,
                    val: Opd::Val(h),
                    ptr: Opd::Val(f),
                });
            }
            "$prelude::Arena::alloc" => {
                let ([a, v], [t]) = (args, generics) else {
                    return un("Arena::alloc: sai số đối số");
                };
                let (pa, ta) = self.operand(a)?;
                if !matches!(&ta, Ty::Ref(_, i) if is_arena(self.db, i)) {
                    return un("Arena::alloc: người nhận không phải &Arena");
                }
                let (pv, _) = self.operand(v)?;
                let g = gty(self.db, t)?;
                let l = gt_glir::layout::layout(&g);
                let h = self.arena_handle(pa);
                let p = self.t();
                self.push(Inst::Call {
                    d: Some(p.clone()),
                    ret: G::Ptr,
                    callee: "gritrt_arena_alloc".into(),
                    args: vec![
                        h,
                        Opd::Int(G::I64, i128::from(l.size)),
                        Opd::Int(G::I64, i128::from(l.align.max(1))),
                    ],
                });
                if !is_unit(t) {
                    self.push(Inst::Store {
                        ty: g,
                        val: pv,
                        ptr: Opd::Val(p.clone()),
                    });
                }
                self.store(dest, Opd::Val(p), &Ty::Ref(false, Box::new(t.clone())))?;
            }
            "$prelude::Arena::reset" => {
                let [a] = args else {
                    return un("Arena::reset: sai số đối số");
                };
                let (pa, ta) = self.operand(a)?;
                if !matches!(&ta, Ty::Ref(true, i) if is_arena(self.db, i)) {
                    return un("Arena::reset: người nhận không phải &!Arena");
                }
                let h = self.arena_handle(pa);
                self.push(Inst::Call {
                    d: None,
                    ret: G::Void,
                    callee: "gritrt_arena_reset".into(),
                    args: vec![h],
                });
            }
            k if k.starts_with("$prelude::root_") => {
                if !args.is_empty() || !generics.is_empty() {
                    return un("root_*: sai số đối số");
                }
                let (ptr, dty) = self.place(dest)?;
                if !is_cap(self.db, &dty) {
                    return un("root_*: đích không phải Cap");
                }
                let g = gty(self.db, &dty)?;
                self.store_zero(ptr, &g)?;
            }
            k if matches!(
                k.strip_prefix("$prelude::"),
                Some("band" | "bor" | "bxor" | "bnot" | "shl" | "shr" | "rotl" | "rotr")
            ) =>
            {
                let name = &k["$prelude::".len()..];
                let mut vs = Vec::new();
                for a in args {
                    let (v, t) = self.operand(a)?;
                    if t != Ty::Int(IntTy::U32) {
                        return un("phép bit prelude trên kiểu không phải u32");
                    }
                    vs.push(v);
                }
                if !generics.is_empty() || vs.len() != if name == "bnot" { 1 } else { 2 } {
                    return un("phép bit prelude: sai số đối số");
                }
                let a = vs[0].clone();
                let g = G::I32;
                let r = match name {
                    "bnot" => self.bin(GB::Xor, &g, a, Opd::Int(G::I32, -1)),
                    "band" => self.bin(GB::And, &g, a, vs[1].clone()),
                    "bor" => self.bin(GB::Or, &g, a, vs[1].clone()),
                    "bxor" => self.bin(GB::Xor, &g, a, vs[1].clone()),
                    _ => {
                        // chốt riêng của codegen (`bit_op`: `n < 32` ⇒ abort `precondition`), ngoài Assert ⊳ ở chỗ gọi
                        let n = vs[1].clone();
                        let ok = self.cmp(CmpPred::Ult, &g, n.clone(), Opd::Int(G::I32, 32));
                        self.extra += 1;
                        let (good, bad) = (
                            format!("rt{}.ok", self.extra),
                            format!("rt{}.abort", self.extra),
                        );
                        self.finish(Term::CondBr(ok, good.clone(), bad.clone()), Some(bad));
                        self.finish(Term::Abort("precondition".into()), Some(good));
                        match name {
                            "shl" => self.bin(GB::Shl, &g, a, n),
                            "shr" => self.bin(GB::LShr, &g, a, n),
                            _ => {
                                // xoay = (a ≪ n) | (a ≫ ((32 − n) & 31)) (rotl), đối xứng cho rotr; n = 0 ⇒ a | a = a (không dịch ≥ 32)
                                let m = self.bin(GB::Sub, &g, Opd::Int(G::I32, 32), n.clone());
                                let m = self.bin(GB::And, &g, m, Opd::Int(G::I32, 31));
                                let (x, y) = if name == "rotl" {
                                    (
                                        self.bin(GB::Shl, &g, a.clone(), n),
                                        self.bin(GB::LShr, &g, a, m),
                                    )
                                } else {
                                    (
                                        self.bin(GB::LShr, &g, a.clone(), n),
                                        self.bin(GB::Shl, &g, a, m),
                                    )
                                };
                                self.bin(GB::Or, &g, x, y)
                            }
                        }
                    }
                };
                self.store(dest, r, &Ty::Int(IntTy::U32))?;
            }
            k if k.starts_with("$prelude::Cell<") => {
                let m = k.rsplit("::").next().unwrap_or_default();
                let mut vs = Vec::new();
                for a in args {
                    vs.push(self.operand(a)?);
                }
                match (m, vs.as_slice()) {
                    ("new", [(v, tv)]) => {
                        let (ptr, dty) = self.place(dest)?;
                        if !is_cell(self.db, &dty) {
                            return un("Cell::new: đích không phải Cell");
                        }
                        let gc = gty(self.db, &dty)?;
                        if gc != G::Struct(vec![gty(self.db, tv)?]) {
                            return un("Cell::new: kiểu giá trị khác trường của Cell");
                        }
                        if !is_unit(tv) {
                            let f = self.t();
                            self.push(Inst::FieldPtr {
                                d: f.clone(),
                                ty: gc,
                                ptr,
                                idx: 0,
                            });
                            let gv = gty(self.db, tv)?;
                            self.push(Inst::Store {
                                ty: gv,
                                val: v.clone(),
                                ptr: Opd::Val(f),
                            });
                        }
                    }
                    ("get" | "set", [(p, Ty::Ref(_, inner)), rest @ ..])
                        if is_cell(self.db, inner) =>
                    {
                        let Ty::Adt(def, cargs) = &**inner else {
                            return un("Cell: người nhận lạ");
                        };
                        let [tv] = crate::variant_fields(self.db, *def, cargs, 0)
                            .map_err(Unsupported)?
                            .try_into()
                            .map_err(|_| Unsupported("Cell phải có đúng một trường".into()))?;
                        let gc = gty(self.db, inner)?;
                        let gv = gty(self.db, &tv)?;
                        let f = self.t();
                        self.push(Inst::FieldPtr {
                            d: f.clone(),
                            ty: gc,
                            ptr: p.clone(),
                            idx: 0,
                        });
                        match (m, rest) {
                            ("get", []) => {
                                if is_unit(&tv) {
                                    return Ok(true);
                                }
                                let d = self.t();
                                self.push(Inst::Load {
                                    d: d.clone(),
                                    ty: gv,
                                    ptr: Opd::Val(f),
                                });
                                self.store(dest, Opd::Val(d), &tv)?;
                            }
                            ("set", [(v, t2)]) if gty(self.db, t2)? == gv => {
                                if !is_unit(&tv) {
                                    self.push(Inst::Store {
                                        ty: gv,
                                        val: v.clone(),
                                        ptr: Opd::Val(f),
                                    });
                                }
                            }
                            _ => return un("Cell::get/set: sai đối số"),
                        }
                    }
                    _ => return un("Cell: thao tác/đối số lạ"),
                }
            }
            k if k.starts_with("$prelude::Text::")
                || matches!(k, "$prelude::str::slice" | "$prelude::str::as_bytes") =>
            {
                let m = k.rsplit("::").next().unwrap_or_default();
                let mut vs = Vec::new();
                for a in args {
                    vs.push(self.operand(a)?);
                }
                if !generics.is_empty() {
                    return un("Text/str: đối số kiểu lạ");
                }
                if k.starts_with("$prelude::str::") {
                    self.str_op(m, &vs, dest)?;
                } else {
                    self.text_op(m, &vs, dest)?;
                }
            }
            k if k.starts_with("$prelude::Owned<") => {
                let m = k.rsplit("::").next().unwrap_or_default();
                let mut vs = Vec::new();
                for a in args {
                    vs.push(self.operand(a)?);
                }
                let [t] = generics else {
                    return un("Owned: sai số đối số kiểu");
                };
                self.owned_op(m, t, &vs, dest)?;
            }
            k if k.starts_with("$prelude::Vec<") => {
                let m = k.rsplit("::").next().unwrap_or_default();
                let mut vs = Vec::new();
                for a in args {
                    vs.push(self.operand(a)?);
                }
                if generics.len() != 1 {
                    return un("Vec: sai số đối số kiểu");
                }
                self.vec_op(m, &vs, dest)?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// trường `idx` của `Vec` mà `p` trỏ tới (kiểu GLIR `gv` của Vec)
    fn vec_field(&mut self, gv: &G, p: &Opd, idx: u32) -> Opd {
        let d = self.t();
        self.push(Inst::FieldPtr {
            d: d.clone(),
            ty: gv.clone(),
            ptr: p.clone(),
            idx,
        });
        Opd::Val(d)
    }

    fn load_val(&mut self, ty: G, ptr: Opd) -> Opd {
        let d = self.t();
        self.push(Inst::Load {
            d: d.clone(),
            ty,
            ptr,
        });
        Opd::Val(d)
    }

    /// `ok` sai ⇒ khối riêng `abort <kind>` (chốt của codegen ngoài `Assert` của MIR, vd `guard(…, OVERFLOW)` của `Vec::push`)
    fn guard(&mut self, ok: Opd, kind: &str) {
        self.extra += 1;
        let (good, bad) = (
            format!("rt{}.ok", self.extra),
            format!("rt{}.abort", self.extra),
        );
        self.finish(Term::CondBr(ok, good.clone(), bad.clone()), Some(bad));
        self.finish(Term::Abort(kind.into()), Some(good));
    }

    /// `(a·b mod 2^64, tràn u64)` bằng nhân 128 bit (thay cho `llvm.umul.with.overflow.i64` của codegen)
    fn umul_ovf(&mut self, a: Opd, b: Opd) -> (Opd, Opd) {
        let wa = self.cast(CastOp::ZExt, &G::I64, a, &G::I128);
        let wb = self.cast(CastOp::ZExt, &G::I64, b, &G::I128);
        let wm = self.bin(GB::Mul, &G::I128, wa, wb);
        let r = self.cast(CastOp::Trunc, &G::I128, wm.clone(), &G::I64);
        let back = self.cast(CastOp::ZExt, &G::I64, r.clone(), &G::I128);
        let o = self.cmp(CmpPred::Ne, &G::I128, back, wm);
        (r, o)
    }

    /// `vec_grow_to` của codegen: cấp khối MỚI `ncap·size` byte trong Arena của Vec (tràn ⇒ abort `overflow`), sao chép `len` phần tử cũ
    /// (VÒNG LẶP từng phần tử thay cho `memcpy`), rồi `ptr ← mới`, `cap ← ncap`.
    fn vec_grow_to(&mut self, gv: &G, base: &Opd, ncap: Opd, ge: &G, unit: bool) {
        let l = gt_glir::layout::layout(ge);
        let lp = self.vec_field(gv, base, 2);
        let len = self.load_val(G::I64, lp);
        let (bytes, o) = self.umul_ovf(ncap.clone(), Opd::Int(G::I64, i128::from(l.size)));
        let ok = self.bin(GB::Xor, &G::I1, o, Opd::Int(G::I1, 1));
        self.guard(ok, "overflow");
        let ap = self.vec_field(gv, base, 0);
        let ar = self.load_val(G::Ptr, ap);
        let h = self.arena_handle(ar);
        let np = self.t();
        self.push(Inst::Call {
            d: Some(np.clone()),
            ret: G::Ptr,
            callee: "gritrt_arena_alloc".into(),
            args: vec![h, bytes, Opd::Int(G::I64, i128::from(l.align.max(1)))],
        });
        let np = Opd::Val(np);
        let pp = self.vec_field(gv, base, 1);
        let old = self.load_val(G::Ptr, pp.clone());
        if !unit {
            // for i in 0..len { np[i] ← old[i] }
            self.extra += 1;
            let n = self.extra;
            let ctr = format!("vi{n}");
            self.aux_slots.push((ctr.clone(), G::I64));
            self.push(Inst::Store {
                ty: G::I64,
                val: Opd::Int(G::I64, 0),
                ptr: Opd::Slot(ctr.clone()),
            });
            let (head, body, exit) = (
                format!("rt{n}.cp"),
                format!("rt{n}.cpb"),
                format!("rt{n}.cpx"),
            );
            self.finish(Term::Br(head.clone()), Some(head.clone()));
            let i = self.load_val(G::I64, Opd::Slot(ctr.clone()));
            let c = self.cmp(CmpPred::Ult, &G::I64, i.clone(), len);
            self.finish(
                Term::CondBr(c, body, exit.clone()),
                Some(format!("rt{n}.cpb")),
            );
            let src = self.t();
            self.push(Inst::IndexPtr {
                d: src.clone(),
                elem: ge.clone(),
                ptr: old,
                idx: i.clone(),
            });
            let v = self.load_val(ge.clone(), Opd::Val(src));
            let dst = self.t();
            self.push(Inst::IndexPtr {
                d: dst.clone(),
                elem: ge.clone(),
                ptr: np.clone(),
                idx: i.clone(),
            });
            self.push(Inst::Store {
                ty: ge.clone(),
                val: v,
                ptr: Opd::Val(dst),
            });
            let i1 = self.bin(GB::Add, &G::I64, i, Opd::Int(G::I64, 1));
            self.push(Inst::Store {
                ty: G::I64,
                val: i1,
                ptr: Opd::Slot(ctr),
            });
            self.finish(Term::Br(head), Some(exit));
        }
        self.push(Inst::Store {
            ty: G::Ptr,
            val: np,
            ptr: pp,
        });
        let cp = self.vec_field(gv, base, 3);
        self.push(Inst::Store {
            ty: G::I64,
            val: ncap,
            ptr: cp,
        });
    }

    /// ghi `{arena, null, 0, 0}` vào Vec tại `ptr` (`Intr::VecNew`/`TextNew` của codegen: không cấp phát tới lần thêm đầu)
    fn vec_init(&mut self, gv: &G, ptr: &Opd, arena: Opd) {
        for (i, v) in [
            (0, arena),
            (1, Opd::Null),
            (2, Opd::Int(G::I64, 0)),
            (3, Opd::Int(G::I64, 0)),
        ] {
            let f = self.vec_field(gv, ptr, i);
            self.push(Inst::Store {
                ty: if i < 2 { G::Ptr } else { G::I64 },
                val: v,
                ptr: f,
            });
        }
    }

    /// `vec_extend` của codegen: thêm `n` byte từ `src` vào cuối `Vec<u8>` tại `base`: `len + n` tràn ⇒ abort `overflow`; `len + n > cap` ⇒ cấp khối
    /// mới `max(16, max(cap·2 (tràn ⇒ overflow), len + n))`; sao chép `n` byte (VÒNG LẶP thay cho `memcpy`); `len ← len + n`.
    fn vec_extend(&mut self, gv: &G, base: &Opd, src: Opd, n: Opd) {
        let lp = self.vec_field(gv, base, 2);
        let len = self.load_val(G::I64, lp.clone());
        let cp = self.vec_field(gv, base, 3);
        let cap = self.load_val(G::I64, cp);
        let sum = self.bin(GB::Add, &G::I64, len.clone(), n.clone());
        let ok = self.cmp(CmpPred::Uge, &G::I64, sum.clone(), len.clone());
        self.guard(ok, "overflow");
        let need = self.cmp(CmpPred::Ugt, &G::I64, sum.clone(), cap.clone());
        self.extra += 1;
        let k = self.extra;
        let (grow, copy) = (format!("rt{k}.ext"), format!("rt{k}.extc"));
        self.finish(Term::CondBr(need, grow.clone(), copy.clone()), Some(grow));
        let (dbl, o) = self.umul_ovf(cap, Opd::Int(G::I64, 2));
        let ok = self.bin(GB::Xor, &G::I1, o, Opd::Int(G::I1, 1));
        self.guard(ok, "overflow");
        let lt = self.cmp(CmpPred::Ult, &G::I64, dbl.clone(), sum.clone());
        let m1 = self.t();
        self.push(Inst::Select {
            d: m1.clone(),
            ty: G::I64,
            c: lt,
            a: sum.clone(),
            b: dbl,
        });
        let lt16 = self.cmp(
            CmpPred::Ult,
            &G::I64,
            Opd::Val(m1.clone()),
            Opd::Int(G::I64, 16),
        );
        let nc = self.t();
        self.push(Inst::Select {
            d: nc.clone(),
            ty: G::I64,
            c: lt16,
            a: Opd::Int(G::I64, 16),
            b: Opd::Val(m1),
        });
        self.vec_grow_to(gv, base, Opd::Val(nc), &G::I8, false);
        self.finish(Term::Br(copy.clone()), Some(copy));
        let pp = self.vec_field(gv, base, 1);
        let cur = self.load_val(G::Ptr, pp);
        // for i in 0..n { cur[len + i] ← src[i] }
        let ctr = format!("vx{k}");
        self.aux_slots.push((ctr.clone(), G::I64));
        self.push(Inst::Store {
            ty: G::I64,
            val: Opd::Int(G::I64, 0),
            ptr: Opd::Slot(ctr.clone()),
        });
        let (head, body, exit) = (
            format!("rt{k}.xh"),
            format!("rt{k}.xb"),
            format!("rt{k}.xx"),
        );
        self.finish(Term::Br(head.clone()), Some(head.clone()));
        let i = self.load_val(G::I64, Opd::Slot(ctr.clone()));
        let c = self.cmp(CmpPred::Ult, &G::I64, i.clone(), n);
        self.finish(Term::CondBr(c, body.clone(), exit.clone()), Some(body));
        let s = self.t();
        self.push(Inst::IndexPtr {
            d: s.clone(),
            elem: G::I8,
            ptr: src,
            idx: i.clone(),
        });
        let v = self.load_val(G::I8, Opd::Val(s));
        let at = self.bin(GB::Add, &G::I64, len, i.clone());
        let d = self.t();
        self.push(Inst::IndexPtr {
            d: d.clone(),
            elem: G::I8,
            ptr: cur,
            idx: at,
        });
        self.push(Inst::Store {
            ty: G::I8,
            val: v,
            ptr: Opd::Val(d),
        });
        let i1 = self.bin(GB::Add, &G::I64, i, Opd::Int(G::I64, 1));
        self.push(Inst::Store {
            ty: G::I64,
            val: i1,
            ptr: Opd::Slot(ctr),
        });
        self.finish(Term::Br(head), Some(exit));
        self.push(Inst::Store {
            ty: G::I64,
            val: sum,
            ptr: lp,
        });
    }

    /// `(ptr, len)` của con trỏ béo `v` (giá trị `{ptr, i64}`), qua một slot phụ
    fn split_fat(&mut self, v: Opd) -> (Opd, Opd) {
        let fat = G::Struct(vec![G::Ptr, G::I64]);
        self.extra += 1;
        let sname = format!("fat{}", self.extra);
        self.aux_slots.push((sname.clone(), fat.clone()));
        self.push(Inst::Store {
            ty: fat.clone(),
            val: v,
            ptr: Opd::Slot(sname.clone()),
        });
        let f0 = self.vec_field(&fat, &Opd::Slot(sname.clone()), 0);
        let p = self.load_val(G::Ptr, f0);
        let f1 = self.vec_field(&fat, &Opd::Slot(sname), 1);
        let n = self.load_val(G::I64, f1);
        (p, n)
    }

    /// ghi con trỏ béo `{p, n}` vào chỗ `dest` (kiểu `&str`/`&[u8]`)
    fn store_fat(&mut self, dest: &Place, p: Opd, n: Opd) -> R<()> {
        let (dptr, dty) = self.place(dest)?;
        if !matches!(&dty, Ty::Ref(_, s) if matches!(**s, Ty::Str | Ty::Slice(_))) {
            return un("đích không phải con trỏ béo");
        }
        let fat = gty(self.db, &dty)?;
        let f0 = self.vec_field(&fat, &dptr, 0);
        self.push(Inst::Store {
            ty: G::Ptr,
            val: p,
            ptr: f0,
        });
        let f1 = self.vec_field(&fat, &dptr, 1);
        self.push(Inst::Store {
            ty: G::I64,
            val: n,
            ptr: f1,
        });
        Ok(())
    }

    /// `utf8_valid` của codegen: `gritrt_utf8_valid(p, n) ≠ 0` (i1)
    fn utf8_valid(&mut self, p: Opd, n: Opd) -> Opd {
        let r = self.t();
        self.push(Inst::Call {
            d: Some(r.clone()),
            ret: G::I32,
            callee: "gritrt_utf8_valid".into(),
            args: vec![p, n],
        });
        self.cmp(CmpPred::Ne, &G::I32, Opd::Val(r), Opd::Int(G::I32, 0))
    }

    /// `Text::*` như `Intr::Text*` của codegen: Text = `{Vec<u8>}`; `new` ⇒ Vec rỗng; `push_str`/`push_utf8` (hợp lệ) ⇒ `vec_extend`;
    /// `push_char` ⇒ mã hoá UTF-8 1–4 byte vào bộ đệm `[4 × i8]` rồi `vec_extend`; `from_utf8` ⇒ `Some(Text{…})`/`None` theo `gritrt_utf8_valid`;
    /// `len`/`as_str`/`as_bytes` đọc trường của Vec.
    fn text_op(&mut self, m: &str, vs: &[(Opd, Ty)], dest: &Place) -> R<()> {
        match (m, vs) {
            ("new", [(a, ta)]) => {
                if !matches!(ta, Ty::Ref(_, i) if is_arena(self.db, i)) {
                    return un("Text::new: đối số không phải &Arena");
                }
                let (ptr, dty) = self.place(dest)?;
                let Some(vt) = text_vec(self.db, &dty) else {
                    return un("Text::new: đích không phải Text");
                };
                let (gt, gv) = (gty(self.db, &dty)?, gty(self.db, &vt)?);
                let vp = self.vec_field(&gt, &ptr, 0);
                self.vec_init(&gv, &vp, a.clone());
            }
            ("from_utf8", [(a, ta), (b, Ty::Ref(_, bs))])
                if **bs == Ty::Slice(Box::new(Ty::Int(IntTy::U8))) =>
            {
                if !matches!(ta, Ty::Ref(_, i) if is_arena(self.db, i)) {
                    return un("Text::from_utf8: đối số không phải &Arena");
                }
                let (ptr, dty) = self.place(dest)?;
                // đích `Option<Text>`: biến thể 1 = `Some(Text)`
                let Ty::Adt(od, oargs) = &dty else {
                    return un("Text::from_utf8: đích không phải Option");
                };
                let some = crate::variant_fields(self.db, *od, oargs, 1).map_err(Unsupported)?;
                let [tt] = some.as_slice() else {
                    return un("Text::from_utf8: Some phải có đúng một trường");
                };
                let Some(vt) = text_vec(self.db, tt) else {
                    return un("Text::from_utf8: Some không chứa Text");
                };
                let (ge, gt, gv) = (gty(self.db, &dty)?, gty(self.db, tt)?, gty(self.db, &vt)?);
                let (p, n) = self.split_fat(b.clone());
                let valid = self.utf8_valid(p.clone(), n.clone());
                self.extra += 1;
                let k = self.extra;
                let (yes, no, end) = (
                    format!("rt{k}.some"),
                    format!("rt{k}.none"),
                    format!("rt{k}.end"),
                );
                self.finish(Term::CondBr(valid, yes.clone(), no.clone()), Some(yes));
                let tag = self.vec_field(&ge, &ptr, 0);
                self.push(Inst::Store {
                    ty: G::I64,
                    val: Opd::Int(G::I64, 1),
                    ptr: tag,
                });
                let pay = self.vec_field(&ge, &ptr, 1);
                let tp = self.vec_field(&G::Struct(vec![gt.clone()]), &pay, 0);
                let vp = self.vec_field(&gt, &tp, 0);
                self.vec_init(&gv, &vp, a.clone());
                self.vec_extend(&gv, &vp, p, n);
                self.finish(Term::Br(end.clone()), Some(no));
                let tag = self.vec_field(&ge, &ptr, 0);
                self.push(Inst::Store {
                    ty: G::I64,
                    val: Opd::Int(G::I64, 0),
                    ptr: tag,
                });
                self.finish(Term::Br(end.clone()), Some(end));
            }
            (_, [(base, Ty::Ref(_, inner)), rest @ ..]) => {
                let Some(vt) = text_vec(self.db, inner) else {
                    return un("Text: người nhận không phải Text");
                };
                let (gt, gv) = (gty(self.db, inner)?, gty(self.db, &vt)?);
                let vp = self.vec_field(&gt, base, 0);
                match (m, rest) {
                    ("len", []) => {
                        let lp = self.vec_field(&gv, &vp, 2);
                        let len = self.load_val(G::I64, lp);
                        self.store(dest, len, &Ty::Int(IntTy::Usize))?;
                    }
                    ("as_str" | "as_bytes", []) => {
                        let pp = self.vec_field(&gv, &vp, 1);
                        let p = self.load_val(G::Ptr, pp);
                        let lp = self.vec_field(&gv, &vp, 2);
                        let len = self.load_val(G::I64, lp);
                        self.store_fat(dest, p, len)?;
                    }
                    ("push_str", [(s, Ty::Ref(_, st))]) if **st == Ty::Str => {
                        let (p, n) = self.split_fat(s.clone());
                        self.vec_extend(&gv, &vp, p, n);
                    }
                    ("push_utf8", [(s, Ty::Ref(_, st))])
                        if **st == Ty::Slice(Box::new(Ty::Int(IntTy::U8))) =>
                    {
                        let (p, n) = self.split_fat(s.clone());
                        let valid = self.utf8_valid(p.clone(), n.clone());
                        self.extra += 1;
                        let k = self.extra;
                        let (yes, done) = (format!("rt{k}.u8y"), format!("rt{k}.u8d"));
                        self.finish(
                            Term::CondBr(valid.clone(), yes.clone(), done.clone()),
                            Some(yes),
                        );
                        self.vec_extend(&gv, &vp, p, n);
                        self.finish(Term::Br(done.clone()), Some(done));
                        self.store(dest, valid, &Ty::Bool)?;
                    }
                    ("push_char", [(c, Ty::Char)]) => {
                        let (buf, n) = self.utf8_encode(c.clone());
                        self.vec_extend(&gv, &vp, buf, n);
                    }
                    _ => return un(format!("Text::{m}: thao tác/đối số lạ")),
                }
            }
            _ => return un(format!("Text::{m}: thao tác/đối số lạ")),
        }
        Ok(())
    }

    /// `text_push_char` của codegen: mã hoá `c` (i32, giá trị vô hướng hợp lệ) thành 1–4 byte UTF-8 trong slot phụ `[4 × i8]` bằng `select`
    /// (không nhánh); trả `(con trỏ bộ đệm, số byte: i64)`.
    fn utf8_encode(&mut self, c: Opd) -> (Opd, Opd) {
        let g = G::I32;
        let k = |v: i128| Opd::Int(G::I32, v);
        let l1 = self.cmp(CmpPred::Ult, &g, c.clone(), k(0x80));
        let l2 = self.cmp(CmpPred::Ult, &g, c.clone(), k(0x800));
        let l3 = self.cmp(CmpPred::Ult, &g, c.clone(), k(0x1_0000));
        let sel = |lw: &mut Self, cnd: Opd, a: Opd, b: Opd| -> Opd {
            let d = lw.t();
            lw.push(Inst::Select {
                d: d.clone(),
                ty: G::I32,
                c: cnd,
                a,
                b,
            });
            Opd::Val(d)
        };
        let n4 = sel(self, l3.clone(), k(3), k(4));
        let n3 = sel(self, l2.clone(), k(2), n4);
        let nn = sel(self, l1.clone(), k(1), n3);
        let hdr = |lw: &mut Self, s: i128, m: i128| -> Opd {
            let x = lw.bin(GB::LShr, &G::I32, c.clone(), k(s));
            lw.bin(GB::Or, &G::I32, x, k(m))
        };
        let h2 = hdr(self, 6, 0xC0);
        let h3 = hdr(self, 12, 0xE0);
        let h4 = hdr(self, 18, 0xF0);
        let t34 = sel(self, l3, h3, h4);
        let t = sel(self, l2, h2, t34);
        let b0 = sel(self, l1, c.clone(), t);
        let cont = |lw: &mut Self, s: i128| -> Opd {
            let x = if s == 0 {
                c.clone()
            } else {
                lw.bin(GB::LShr, &G::I32, c.clone(), k(s))
            };
            let x = lw.bin(GB::And, &G::I32, x, k(0x3F));
            lw.bin(GB::Or, &G::I32, x, k(0x80))
        };
        let (c0, c6, c12) = (cont(self, 0), cont(self, 6), cont(self, 12));
        let e2 = self.cmp(CmpPred::Eq, &g, nn.clone(), k(2));
        let e3 = self.cmp(CmpPred::Eq, &g, nn.clone(), k(3));
        let b1b = sel(self, e3.clone(), c6.clone(), c12);
        let b1 = sel(self, e2, c0.clone(), b1b);
        let b2 = sel(self, e3, c0.clone(), c6);
        let buf_ty = G::Array(Box::new(G::I8), 4);
        self.extra += 1;
        let buf = format!("u8buf{}", self.extra);
        self.aux_slots.push((buf.clone(), buf_ty));
        for (i, v) in [b0, b1, b2, c0].into_iter().enumerate() {
            let p = self.t();
            self.push(Inst::IndexPtr {
                d: p.clone(),
                elem: G::I8,
                ptr: Opd::Slot(buf.clone()),
                idx: Opd::Int(G::I64, i as i128),
            });
            let tb = self.cast(CastOp::Trunc, &G::I32, v, &G::I8);
            self.push(Inst::Store {
                ty: G::I8,
                val: tb,
                ptr: Opd::Val(p),
            });
        }
        let n64 = self.cast(CastOp::ZExt, &G::I32, nn, &G::I64);
        (Opd::Slot(buf), n64)
    }

    /// `str::slice` (`str_slice` của codegen): `from ≤ to ≤ len` và hai đầu ở BIÊN KÝ TỰ (`idx = len` hoặc byte tại `idx` không phải byte nối
    /// `10xxxxxx`); sai ⇒ abort `slice_range` (4 chốt như codegen). `str::as_bytes` ⇒ cùng con trỏ béo.
    fn str_op(&mut self, m: &str, vs: &[(Opd, Ty)], dest: &Place) -> R<()> {
        match (m, vs) {
            ("as_bytes", [(s, Ty::Ref(_, st))]) if **st == Ty::Str => {
                let (p, n) = self.split_fat(s.clone());
                self.store_fat(dest, p, n)
            }
            (
                "slice",
                [
                    (s, Ty::Ref(_, st)),
                    (from, Ty::Int(IntTy::Usize)),
                    (to, Ty::Int(IntTy::Usize)),
                ],
            ) if **st == Ty::Str => {
                let (p, n) = self.split_fat(s.clone());
                let c1 = self.cmp(CmpPred::Ule, &G::I64, from.clone(), to.clone());
                self.guard(c1, "slice_range");
                let c2 = self.cmp(CmpPred::Ule, &G::I64, to.clone(), n.clone());
                self.guard(c2, "slice_range");
                for idx in [from, to] {
                    let at_end = self.cmp(CmpPred::Eq, &G::I64, idx.clone(), n.clone());
                    self.extra += 1;
                    let k = self.extra;
                    let (chk, join, bad) = (
                        format!("rt{k}.bchk"),
                        format!("rt{k}.bok"),
                        format!("rt{k}.abort"),
                    );
                    self.finish(Term::CondBr(at_end, join.clone(), chk.clone()), Some(chk));
                    let bp = self.t();
                    self.push(Inst::IndexPtr {
                        d: bp.clone(),
                        elem: G::I8,
                        ptr: p.clone(),
                        idx: idx.clone(),
                    });
                    let byte = self.load_val(G::I8, Opd::Val(bp));
                    let masked = self.bin(GB::And, &G::I8, byte, Opd::Int(G::I8, 0xC0));
                    let ok = self.cmp(CmpPred::Ne, &G::I8, masked, Opd::Int(G::I8, 0x80));
                    self.finish(Term::CondBr(ok, join.clone(), bad.clone()), Some(bad));
                    self.finish(Term::Abort("slice_range".into()), Some(join));
                }
                let np = self.t();
                self.push(Inst::IndexPtr {
                    d: np.clone(),
                    elem: G::I8,
                    ptr: p,
                    idx: from.clone(),
                });
                let nl = self.bin(GB::Sub, &G::I64, to.clone(), from.clone());
                self.store_fat(dest, Opd::Val(np), nl)
            }
            _ => un(format!("str::{m}: thao tác/đối số lạ")),
        }
    }

    /// `gritrt_heap_alloc/free(…, size_of T, align_of T)` (bố cục GLIR của `T`, như `heap_alloc`/`heap_free` của codegen)
    fn heap_call(&mut self, name: &str, p: Option<Opd>, t: &Ty) -> R<Option<Opd>> {
        let l = gt_glir::layout::layout(&gty(self.db, t)?);
        let mut args: Vec<Opd> = p.into_iter().collect();
        args.extend([
            Opd::Int(G::I64, i128::from(l.size)),
            Opd::Int(G::I64, i128::from(l.align.max(1))),
        ]);
        if name == "gritrt_heap_alloc" {
            let d = self.t();
            self.push(Inst::Call {
                d: Some(d.clone()),
                ret: G::Ptr,
                callee: name.into(),
                args,
            });
            return Ok(Some(Opd::Val(d)));
        }
        self.push(Inst::Call {
            d: None,
            ret: G::Void,
            callee: name.into(),
            args,
        });
        Ok(None)
    }

    /// `gritrt_secret_wipe(p, size_of ty)` (như `secret_wipe` của codegen; ADR-P2-03)
    fn secret_wipe(&mut self, p: Opd, ty: &Ty) -> R<()> {
        let l = gt_glir::layout::layout(&gty(self.db, ty)?);
        self.push(Inst::Call {
            d: None,
            ret: G::Void,
            callee: "gritrt_secret_wipe".into(),
            args: vec![p, Opd::Int(G::I64, i128::from(l.size))],
        });
        Ok(())
    }

    /// `Owned<T>::*` như `Intr::Owned*` của codegen: `new(cap, v)` ⇒ `p = gritrt_heap_alloc(size, align)`, `*p ← v`, `Owned{p}`; `get`/`get_mut` ⇒
    /// trường `ptr`; `into_inner(self)` ⇒ nạp `*p`, (T chứa Secret ⇒ `gritrt_secret_wipe(p, size)`), `gritrt_heap_free(p, size, align)`.
    fn owned_op(&mut self, m: &str, t: &Ty, vs: &[(Opd, Ty)], dest: &Place) -> R<()> {
        let gt = gty(self.db, t)?;
        match (m, vs) {
            ("new", [(_, tc), (v, tv)]) => {
                if !matches!(tc, Ty::Ref(_, i) if is_cap(self.db, i)) || tv != t {
                    return un("Owned::new: đối số lạ");
                }
                let Some(p) = self.heap_call("gritrt_heap_alloc", None, t)? else {
                    return un("Owned::new: heap_alloc");
                };
                if !is_unit(t) {
                    self.push(Inst::Store {
                        ty: gt,
                        val: v.clone(),
                        ptr: p.clone(),
                    });
                }
                let (ptr, dty) = self.place(dest)?;
                if owned_elem(self.db, &dty).as_ref() != Some(t) {
                    return un("Owned::new: đích không phải Owned<T>");
                }
                let go = gty(self.db, &dty)?;
                let f = self.vec_field(&go, &ptr, 0);
                self.push(Inst::Store {
                    ty: G::Ptr,
                    val: p,
                    ptr: f,
                });
            }
            ("get" | "get_mut", [(p, Ty::Ref(_, inner))])
                if owned_elem(self.db, inner).as_ref() == Some(t) =>
            {
                let go = gty(self.db, inner)?;
                let f = self.vec_field(&go, p, 0);
                let hp = self.load_val(G::Ptr, f);
                self.store(dest, hp, &Ty::Ref(m == "get_mut", Box::new(t.clone())))?;
            }
            ("into_inner", [(o, to)]) if owned_elem(self.db, to).as_ref() == Some(t) => {
                let go = gty(self.db, to)?;
                self.extra += 1;
                let sname = format!("own{}", self.extra);
                self.aux_slots.push((sname.clone(), go.clone()));
                self.push(Inst::Store {
                    ty: go.clone(),
                    val: o.clone(),
                    ptr: Opd::Slot(sname.clone()),
                });
                let f = self.vec_field(&go, &Opd::Slot(sname), 0);
                let hp = self.load_val(G::Ptr, f);
                let v = if is_unit(t) {
                    None
                } else {
                    Some(self.load_val(gt, hp.clone()))
                };
                if self.db.secret_used && self.db.contains_secret(t) {
                    self.secret_wipe(hp.clone(), t)?;
                }
                self.heap_call("gritrt_heap_free", Some(hp), t)?;
                if let Some(v) = v {
                    self.store(dest, v, t)?;
                }
            }
            _ => return un(format!("Owned::{m}: thao tác/đối số lạ")),
        }
        Ok(())
    }

    /// Drop glue tại `ptr` (kiểu `ty`) — đúng `drop_glue_at` của codegen: `Call` ⇒ gọi `drop` người dùng (instance không đối số) rồi glue bên trong;
    /// `Free` ⇒ glue của `T` tại khối heap rồi `gritrt_heap_free`; `Wipe` ⇒ glue của trường `v` rồi `gritrt_secret_wipe` cả Secret; `Fields`/`Variants`
    /// (theo tag)/`Array` (vòng lặp) ⇒ đệ quy.
    fn drop_glue_at(&mut self, ptr: Opd, ty: &Ty, g: &gt_types::Glue) -> R<()> {
        use gt_types::Glue;
        match g {
            Glue::Nothing => Ok(()),
            Glue::Call(node, inner) => {
                let inst = Instance {
                    node: *node,
                    args: Vec::new(),
                };
                let Some((name, rty)) = self.cx.sig(&inst) else {
                    return un("drop: `drop` người dùng ngoài tập con");
                };
                if !is_unit(&rty) {
                    return un("drop: `drop` trả giá trị");
                }
                self.calls.push(inst);
                self.push(Inst::Call {
                    d: None,
                    ret: G::Void,
                    callee: name,
                    args: vec![ptr.clone()],
                });
                self.drop_glue_at(ptr, ty, inner)
            }
            Glue::Free(t, inner) => {
                if owned_elem(self.db, ty).as_ref() != Some(t) {
                    return un("drop glue: Free trên kiểu không phải Owned<T>");
                }
                let go = gty(self.db, ty)?;
                let f = self.vec_field(&go, &ptr, 0);
                let hp = self.load_val(G::Ptr, f);
                self.drop_glue_at(hp.clone(), t, inner)?;
                self.heap_call("gritrt_heap_free", Some(hp), t)?;
                Ok(())
            }
            Glue::Wipe(_) => un("drop glue: Secret (xoá) chưa hạ"),
            Glue::Fields(fs) => {
                let gs = gty(self.db, ty)?;
                for (i, fty, fg) in fs {
                    let fp = self.vec_field(&gs, &ptr, *i);
                    self.drop_glue_at(fp, fty, fg)?;
                }
                Ok(())
            }
            Glue::Variants(vs) => {
                let Ty::Adt(def, args) = ty else {
                    return un("drop glue: biến thể trên kiểu không phải enum");
                };
                let ge = gty(self.db, ty)?;
                let tp = self.vec_field(&ge, &ptr, 0);
                let tag = self.load_val(G::I64, tp);
                self.extra += 1;
                let k = self.extra;
                let cont = format!("rt{k}.dcont");
                for (n, (vi, fs)) in vs.iter().enumerate() {
                    let (arm, next) = (format!("rt{k}.dv{n}"), format!("rt{k}.dn{n}"));
                    let c = self.cmp(
                        CmpPred::Eq,
                        &G::I64,
                        tag.clone(),
                        Opd::Int(G::I64, i128::from(*vi)),
                    );
                    self.finish(Term::CondBr(c, arm.clone(), next.clone()), Some(arm));
                    let pay = self.vec_field(&ge, &ptr, 1);
                    let vfs = crate::variant_fields(self.db, *def, args, *vi as usize)
                        .map_err(Unsupported)?;
                    let vst =
                        G::Struct(vfs.iter().map(|t| gty(self.db, t)).collect::<R<Vec<_>>>()?);
                    for (i, fty, fg) in fs {
                        let fp = self.vec_field(&vst, &pay, *i);
                        self.drop_glue_at(fp, fty, fg)?;
                    }
                    self.finish(Term::Br(cont.clone()), Some(next));
                }
                self.finish(Term::Br(cont.clone()), Some(cont));
                Ok(())
            }
            Glue::Array(n, ety, eg) => {
                let ge = gty(self.db, ety)?;
                self.extra += 1;
                let k = self.extra;
                let ctr = format!("di{k}");
                self.aux_slots.push((ctr.clone(), G::I64));
                self.push(Inst::Store {
                    ty: G::I64,
                    val: Opd::Int(G::I64, 0),
                    ptr: Opd::Slot(ctr.clone()),
                });
                let (head, body, exit) = (
                    format!("rt{k}.dah"),
                    format!("rt{k}.dab"),
                    format!("rt{k}.dax"),
                );
                self.finish(Term::Br(head.clone()), Some(head.clone()));
                let i = self.load_val(G::I64, Opd::Slot(ctr.clone()));
                let c = self.cmp(
                    CmpPred::Ult,
                    &G::I64,
                    i.clone(),
                    Opd::Int(G::I64, i128::from(*n)),
                );
                self.finish(Term::CondBr(c, body.clone(), exit.clone()), Some(body));
                let ep = self.t();
                self.push(Inst::IndexPtr {
                    d: ep.clone(),
                    elem: ge,
                    ptr,
                    idx: i.clone(),
                });
                self.drop_glue_at(Opd::Val(ep), ety, eg)?;
                let i1 = self.bin(GB::Add, &G::I64, i, Opd::Int(G::I64, 1));
                self.push(Inst::Store {
                    ty: G::I64,
                    val: i1,
                    ptr: Opd::Slot(ctr),
                });
                self.finish(Term::Br(head), Some(exit));
                Ok(())
            }
        }
    }

    /// `Vec<T>::*` như `Intr::Vec*` của codegen: `new` ⇒ `{arena, null, 0, 0}`; `len`/`as_slice`/`as_mut_slice` đọc trường; `truncate` ⇒
    /// `len ← min(n, len)`; `reserve(n)` ⇒ `n > cap` thì cấp đúng `n`; `push` ⇒ `len = cap` thì `cap·2` (tràn ⇒ abort `overflow`, tối thiểu 4) rồi ghi.
    fn vec_op(&mut self, m: &str, vs: &[(Opd, Ty)], dest: &Place) -> R<()> {
        if let ("new", [(a, ta)]) = (m, vs) {
            if !matches!(ta, Ty::Ref(_, i) if is_arena(self.db, i)) {
                return un("Vec::new: đối số không phải &Arena");
            }
            let (ptr, dty) = self.place(dest)?;
            if vec_elem(self.db, &dty).is_none() {
                return un("Vec::new: đích không phải Vec");
            }
            let gv = gty(self.db, &dty)?;
            self.vec_init(&gv, &ptr, a.clone());
            return Ok(());
        }
        let [(base, Ty::Ref(_, inner)), rest @ ..] = vs else {
            return un("Vec: người nhận không phải tham chiếu");
        };
        let Some(elem) = vec_elem(self.db, inner) else {
            return un("Vec: người nhận không phải Vec");
        };
        let gv = gty(self.db, inner)?;
        let ge = gty(self.db, &elem)?;
        let unit = is_unit(&elem);
        match (m, rest) {
            ("len", []) => {
                let lp = self.vec_field(&gv, base, 2);
                let len = self.load_val(G::I64, lp);
                self.store(dest, len, &Ty::Int(IntTy::Usize))?;
            }
            ("as_slice" | "as_mut_slice", []) => {
                let pp = self.vec_field(&gv, base, 1);
                let p = self.load_val(G::Ptr, pp);
                let lp = self.vec_field(&gv, base, 2);
                let len = self.load_val(G::I64, lp);
                let (dptr, dty) = self.place(dest)?;
                if !matches!(&dty, Ty::Ref(_, s) if **s == Ty::Slice(Box::new(elem.clone()))) {
                    return un("Vec::as_slice: đích không phải &[T]");
                }
                let fat = gty(self.db, &dty)?;
                for (i, (t, v)) in [(G::Ptr, p), (G::I64, len)].into_iter().enumerate() {
                    let f = self.vec_field(&fat, &dptr, i as u32);
                    self.push(Inst::Store {
                        ty: t,
                        val: v,
                        ptr: f,
                    });
                }
            }
            ("truncate", [(n, Ty::Int(IntTy::Usize))]) => {
                let lp = self.vec_field(&gv, base, 2);
                let len = self.load_val(G::I64, lp.clone());
                let c = self.cmp(CmpPred::Ult, &G::I64, n.clone(), len.clone());
                let d = self.t();
                self.push(Inst::Select {
                    d: d.clone(),
                    ty: G::I64,
                    c,
                    a: n.clone(),
                    b: len,
                });
                self.push(Inst::Store {
                    ty: G::I64,
                    val: Opd::Val(d),
                    ptr: lp,
                });
            }
            ("reserve", [(n, Ty::Int(IntTy::Usize))]) => {
                let cp = self.vec_field(&gv, base, 3);
                let cap = self.load_val(G::I64, cp);
                let need = self.cmp(CmpPred::Ugt, &G::I64, n.clone(), cap);
                self.extra += 1;
                let (grow, done) = (
                    format!("rt{}.rsv", self.extra),
                    format!("rt{}.rsvd", self.extra),
                );
                self.finish(Term::CondBr(need, grow.clone(), done.clone()), Some(grow));
                self.vec_grow_to(&gv, base, n.clone(), &ge, unit);
                self.finish(Term::Br(done.clone()), Some(done));
            }
            ("push", [(x, tx)]) if gty(self.db, tx)? == ge => {
                let lp = self.vec_field(&gv, base, 2);
                let len = self.load_val(G::I64, lp.clone());
                let cp = self.vec_field(&gv, base, 3);
                let cap = self.load_val(G::I64, cp);
                let full = self.cmp(CmpPred::Eq, &G::I64, len.clone(), cap.clone());
                self.extra += 1;
                let (grow, store) = (
                    format!("rt{}.grow", self.extra),
                    format!("rt{}.store", self.extra),
                );
                self.finish(Term::CondBr(full, grow.clone(), store.clone()), Some(grow));
                let (dbl, o) = self.umul_ovf(cap, Opd::Int(G::I64, 2));
                let ok = self.bin(GB::Xor, &G::I1, o, Opd::Int(G::I1, 1));
                self.guard(ok, "overflow");
                let small = self.cmp(CmpPred::Ult, &G::I64, dbl.clone(), Opd::Int(G::I64, 4));
                let nc = self.t();
                self.push(Inst::Select {
                    d: nc.clone(),
                    ty: G::I64,
                    c: small,
                    a: Opd::Int(G::I64, 4),
                    b: dbl,
                });
                self.vec_grow_to(&gv, base, Opd::Val(nc), &ge, unit);
                self.finish(Term::Br(store.clone()), Some(store));
                if !unit {
                    let pp = self.vec_field(&gv, base, 1);
                    let cur = self.load_val(G::Ptr, pp);
                    let slot = self.t();
                    self.push(Inst::IndexPtr {
                        d: slot.clone(),
                        elem: ge.clone(),
                        ptr: cur,
                        idx: len.clone(),
                    });
                    self.push(Inst::Store {
                        ty: ge,
                        val: x.clone(),
                        ptr: Opd::Val(slot),
                    });
                }
                let nl = self.bin(GB::Add, &G::I64, len, Opd::Int(G::I64, 1));
                self.push(Inst::Store {
                    ty: G::I64,
                    val: nl,
                    ptr: lp,
                });
            }
            _ => return un(format!("Vec::{m}: thao tác/đối số lạ")),
        }
        Ok(())
    }

    /// ghi 0 vào mọi lá số nguyên của `g` tại `ptr` (giá trị `Cap` zero); kiểu khác ⇒ không hạ
    fn store_zero(&mut self, ptr: Opd, g: &G) -> R<()> {
        match g {
            G::Struct(fs) => {
                for (i, f) in fs.iter().enumerate() {
                    let d = self.t();
                    self.push(Inst::FieldPtr {
                        d: d.clone(),
                        ty: g.clone(),
                        ptr: ptr.clone(),
                        idx: i as u32,
                    });
                    self.store_zero(Opd::Val(d), f)?;
                }
                Ok(())
            }
            t if t.is_int() => {
                self.push(Inst::Store {
                    ty: t.clone(),
                    val: Opd::Int(t.clone(), 0),
                    ptr,
                });
                Ok(())
            }
            _ => un("giá trị zero của kiểu không phải struct số nguyên"),
        }
    }

    fn block_name(b: usize) -> String {
        format!("bb{b}")
    }

    fn rvalue(&mut self, dest: &Place, rv: &Rvalue) -> R<()> {
        match rv {
            Rvalue::Use(o) => {
                let (v, ty) = self.operand(o)?;
                self.store(dest, v, &ty)
            }
            Rvalue::BinaryOp(op, a, b) => {
                let (va, ta) = self.operand(a)?;
                let (vb, _) = self.operand(b)?;
                let (r, rty) = self.binop(*op, &ta, va, vb)?;
                self.store(dest, r, &rty)
            }
            Rvalue::CheckedBinaryOp(op, a, b) => {
                let (va, ta) = self.operand(a)?;
                let (vb, _) = self.operand(b)?;
                let Ty::Int(it) = ta else {
                    return un("checked trên kiểu không nguyên");
                };
                let (r, ovf) = self.checked(*op, it, va, vb)?;
                let (ptr, _) = self.place(dest)?;
                let tup = G::Struct(vec![gint(it), G::I1]);
                let f0 = self.t();
                self.push(Inst::FieldPtr {
                    d: f0.clone(),
                    ty: tup.clone(),
                    ptr: ptr.clone(),
                    idx: 0,
                });
                self.push(Inst::Store {
                    ty: gint(it),
                    val: r,
                    ptr: Opd::Val(f0),
                });
                let f1 = self.t();
                self.push(Inst::FieldPtr {
                    d: f1.clone(),
                    ty: tup,
                    ptr,
                    idx: 1,
                });
                self.push(Inst::Store {
                    ty: G::I1,
                    val: ovf,
                    ptr: Opd::Val(f1),
                });
                Ok(())
            }
            Rvalue::UnaryOp(op, o) => {
                let (v, ty) = self.operand(o)?;
                let g = gty(self.db, &ty)?;
                let r = match (op, &ty) {
                    (UnOp::Not, Ty::Bool) => self.bin(GB::Xor, &G::I1, v, Opd::Int(G::I1, 1)),
                    (UnOp::Not, Ty::Int(_)) => self.bin(GB::Xor, &g, v, Opd::Int(g.clone(), -1)),
                    (UnOp::Neg, Ty::Int(_)) => self.bin(GB::Sub, &g, Opd::Int(g.clone(), 0), v),
                    _ => return un("UnaryOp trên kiểu lạ"),
                };
                self.store(dest, r, &ty)
            }
            Rvalue::Cast(o, to) => {
                let to = &self.sub(to);
                let (v, from) = self.operand(o)?;
                // `u8 as char` (như `cast` của codegen): mọi u8 là giá trị vô hướng hợp lệ ⇒ nới không dấu lên i32
                if !matches!(
                    (&from, to),
                    (Ty::Int(_) | Ty::Bool | Ty::Char, Ty::Int(_)) | (Ty::Int(IntTy::U8), Ty::Char)
                ) {
                    return un("Cast ngoài int/bool→int, u8→char");
                }
                if let Ty::Int(it) = to
                    && it.bits() > 64
                {
                    return un("Cast tới 128 bit");
                }
                let (gf, gt) = (gty(self.db, &from)?, gty(self.db, to)?);
                let (bf, bt) = (gf.bits().unwrap_or(0), gt.bits().unwrap_or(0));
                let signed_from = matches!(from, Ty::Int(it) if it.signed());
                let r = if bt == bf {
                    v
                } else if bt < bf {
                    self.cast(CastOp::Trunc, &gf, v, &gt)
                } else {
                    self.cast(
                        if signed_from {
                            CastOp::SExt
                        } else {
                            CastOp::ZExt
                        },
                        &gf,
                        v,
                        &gt,
                    )
                };
                self.store(dest, r, to)
            }
            Rvalue::CastFits(o, to) => {
                let to = &self.sub(to);
                // `ép x as T` không mất thông tin ⇔ (đi-về qua T cho lại x) VÀ (cùng dấu giá trị khi hai kiểu khác dấu); xem `cast_fits` của codegen
                let (v, from) = self.operand(o)?;
                let one = Opd::Int(G::I1, 1);
                if matches!(from, Ty::Bool) || matches!((&from, to), (Ty::Int(IntTy::U8), Ty::Char))
                {
                    return self.store(dest, one, &Ty::Bool);
                }
                let (Ty::Int(fi), Ty::Int(ti)) = (&from, to) else {
                    return un("CastFits ngoài int→int");
                };
                if fi.bits() > 64 || ti.bits() > 64 {
                    return un("CastFits 128 bit");
                }
                let (gf, gt) = (gint(*fi), gint(*ti));
                let ext = |s: bool| if s { CastOp::SExt } else { CastOp::ZExt };
                let conv = |lw: &mut Self, v: Opd, a: &G, sa: bool, b: &G| -> Opd {
                    let (ba, bb) = (a.bits().unwrap_or(0), b.bits().unwrap_or(0));
                    if ba == bb {
                        v
                    } else if bb < ba {
                        lw.cast(CastOp::Trunc, a, v, b)
                    } else {
                        lw.cast(ext(sa), a, v, b)
                    }
                };
                let y = conv(self, v.clone(), &gf, fi.signed(), &gt);
                let z = conv(self, y.clone(), &gt, ti.signed(), &gf);
                let mut ok = self.cmp(CmpPred::Eq, &gf, v.clone(), z);
                if fi.signed() != ti.signed() {
                    let negx = if fi.signed() {
                        self.cmp(CmpPred::Slt, &gf, v, Opd::Int(gf.clone(), 0))
                    } else {
                        Opd::Int(G::I1, 0)
                    };
                    let negy = if ti.signed() {
                        self.cmp(CmpPred::Slt, &gt, y, Opd::Int(gt.clone(), 0))
                    } else {
                        Opd::Int(G::I1, 0)
                    };
                    let same = self.cmp(CmpPred::Eq, &G::I1, negx, negy);
                    ok = self.bin(GB::And, &G::I1, ok, same);
                }
                self.store(dest, ok, &Ty::Bool)
            }
            Rvalue::Aggregate(gt_mir::AggregateKind::Array(_), ops) => {
                let (ptr, dty) = self.place(dest)?;
                let Ty::Array(elem, _) = &dty else {
                    return un("Aggregate mảng vào kiểu không phải mảng");
                };
                let elem = (**elem).clone();
                let ge = gty(self.db, &elem)?;
                for (i, o) in ops.iter().enumerate() {
                    let (v, _) = self.operand(o)?;
                    if is_unit(&elem) {
                        continue;
                    }
                    let p = self.t();
                    self.push(Inst::IndexPtr {
                        d: p.clone(),
                        elem: ge.clone(),
                        ptr: ptr.clone(),
                        idx: Opd::Int(G::I64, i as i128),
                    });
                    self.push(Inst::Store {
                        ty: ge.clone(),
                        val: v,
                        ptr: Opd::Val(p),
                    });
                }
                Ok(())
            }
            Rvalue::Aggregate(kind, ops) => {
                let kind = &match kind {
                    gt_mir::AggregateKind::Adt(def, v, args) => gt_mir::AggregateKind::Adt(
                        *def,
                        *v,
                        args.iter().map(|a| self.sub(a)).collect(),
                    ),
                    k => k.clone(),
                };
                let (ptr, dty) = self.place(dest)?;
                let (field_tys, tag): (Vec<Ty>, Option<u32>) = match (kind, &dty) {
                    (gt_mir::AggregateKind::Tuple, Ty::Tuple(v)) => (v.clone(), None),
                    (gt_mir::AggregateKind::Adt(def, 0, args), Ty::Adt(..))
                        if plain_struct(self.db, def) =>
                    {
                        (
                            crate::variant_fields(self.db, *def, args, 0).map_err(Unsupported)?,
                            None,
                        )
                    }
                    (gt_mir::AggregateKind::Adt(def, v, args), Ty::Adt(..))
                        if !plain_struct(self.db, def) =>
                    {
                        (
                            crate::variant_fields(self.db, *def, args, *v as usize)
                                .map_err(Unsupported)?,
                            Some(*v),
                        )
                    }
                    _ => return un("Aggregate ngoài tuple/struct/enum"),
                };
                if field_tys.len() != ops.len() {
                    return un("Aggregate sai số trường");
                }
                // enum: ghi tag `i64` rồi nhìn vùng payload như struct của biến thể; struct/tuple: chính chỗ đích
                let (cptr, cty) = if let Some(v) = tag {
                    let ge = gty(self.db, &dty)?;
                    let tp = self.t();
                    self.push(Inst::FieldPtr {
                        d: tp.clone(),
                        ty: ge.clone(),
                        ptr: ptr.clone(),
                        idx: 0,
                    });
                    self.push(Inst::Store {
                        ty: G::I64,
                        val: Opd::Int(G::I64, i128::from(v)),
                        ptr: Opd::Val(tp),
                    });
                    let pp = self.t();
                    self.push(Inst::FieldPtr {
                        d: pp.clone(),
                        ty: ge,
                        ptr,
                        idx: 1,
                    });
                    let gfs = field_tys
                        .iter()
                        .map(|t| gty(self.db, t))
                        .collect::<R<Vec<_>>>()?;
                    (Opd::Val(pp), G::Struct(gfs))
                } else {
                    (ptr, gty(self.db, &dty)?)
                };
                for (i, (o, ft)) in ops.iter().zip(&field_tys).enumerate() {
                    if is_unit(ft) {
                        continue;
                    }
                    let (v, _) = self.operand(o)?;
                    let fp = self.t();
                    self.push(Inst::FieldPtr {
                        d: fp.clone(),
                        ty: cty.clone(),
                        ptr: cptr.clone(),
                        idx: i as u32,
                    });
                    let g = gty(self.db, ft)?;
                    self.push(Inst::Store {
                        ty: g,
                        val: v,
                        ptr: Opd::Val(fp),
                    });
                }
                Ok(())
            }
            Rvalue::Ref(_, _, place) => {
                let (ptr, pty, meta) = self.place_m(place)?;
                let (dptr, dty) = self.place(dest)?;
                gty(self.db, &dty)?;
                if matches!(pty, Ty::Str | Ty::Slice(_)) {
                    // tham chiếu tới chỗ không kích thước ⇒ con trỏ béo `{ptr, len}`
                    let Some(len) = meta else {
                        return un("Ref tới slice không có độ dài");
                    };
                    let fat = gty(self.db, &dty)?;
                    let (p0, p1) = (self.t(), self.t());
                    self.push(Inst::FieldPtr {
                        d: p0.clone(),
                        ty: fat.clone(),
                        ptr: dptr.clone(),
                        idx: 0,
                    });
                    self.push(Inst::Store {
                        ty: G::Ptr,
                        val: ptr,
                        ptr: Opd::Val(p0),
                    });
                    self.push(Inst::FieldPtr {
                        d: p1.clone(),
                        ty: fat,
                        ptr: dptr,
                        idx: 1,
                    });
                    self.push(Inst::Store {
                        ty: G::I64,
                        val: len,
                        ptr: Opd::Val(p1),
                    });
                    return Ok(());
                }
                // địa chỉ của chỗ chính là con trỏ: ghi vào chỗ đích kiểu `&T`
                self.push(Inst::Store {
                    ty: G::Ptr,
                    val: ptr,
                    ptr: dptr,
                });
                Ok(())
            }
            Rvalue::Discriminant(place) => {
                let (ptr, ty) = self.place(place)?;
                let Ty::Adt(..) = &ty else {
                    return un("Discriminant của kiểu không phải enum");
                };
                let ge = gty(self.db, &ty)?;
                let f = self.t();
                self.push(Inst::FieldPtr {
                    d: f.clone(),
                    ty: ge,
                    ptr,
                    idx: 0,
                });
                let d = self.t();
                self.push(Inst::Load {
                    d: d.clone(),
                    ty: G::I64,
                    ptr: Opd::Val(f),
                });
                self.store(dest, Opd::Val(d), &Ty::Int(IntTy::Usize))
            }
            Rvalue::Len(place) => {
                let (_, ty, meta) = self.place_m(place)?;
                let len = match (ty, meta) {
                    (Ty::Array(_, n), _) => Opd::Int(G::I64, i128::from(n)),
                    (Ty::Slice(_) | Ty::Str, Some(m)) => m,
                    _ => return un("Len của kiểu không phải mảng/slice"),
                };
                self.store(dest, len, &Ty::Int(IntTy::Usize))
            }
            Rvalue::Repeat(o, n) => {
                let (ptr, dty) = self.place(dest)?;
                let Ty::Array(elem, _) = &dty else {
                    return un("Repeat vào kiểu không phải mảng");
                };
                if *n > 64 {
                    return un("Repeat lớn");
                }
                let elem = (**elem).clone();
                let (v, _) = self.operand(o)?;
                if is_unit(&elem) {
                    return Ok(());
                }
                let ge = gty(self.db, &elem)?;
                for i in 0..*n {
                    let p = self.t();
                    self.push(Inst::IndexPtr {
                        d: p.clone(),
                        elem: ge.clone(),
                        ptr: ptr.clone(),
                        idx: Opd::Int(G::I64, i128::from(i)),
                    });
                    self.push(Inst::Store {
                        ty: ge.clone(),
                        val: v.clone(),
                        ptr: Opd::Val(p),
                    });
                }
                Ok(())
            }
        }
    }

    fn binop(&mut self, op: BinOp, ta: &Ty, a: Opd, b: Opd) -> R<(Opd, Ty)> {
        let (g, signed) = match ta {
            Ty::Int(it) => (gint(*it), it.signed()),
            Ty::Bool => (G::I1, false),
            Ty::Char => (G::I32, false),
            _ => return un("BinaryOp trên kiểu không nguyên"),
        };
        let pick = |s: CmpPred, u: CmpPred| if signed { s } else { u };
        let (r, rty) = match op {
            BinOp::Add | BinOp::AddWrap => (self.bin(GB::Add, &g, a, b), ta.clone()),
            BinOp::Sub | BinOp::SubWrap => (self.bin(GB::Sub, &g, a, b), ta.clone()),
            BinOp::Mul | BinOp::MulWrap => (self.bin(GB::Mul, &g, a, b), ta.clone()),
            BinOp::Div => (
                self.bin(if signed { GB::SDiv } else { GB::UDiv }, &g, a, b),
                ta.clone(),
            ),
            BinOp::Rem => (
                self.bin(if signed { GB::SRem } else { GB::URem }, &g, a, b),
                ta.clone(),
            ),
            BinOp::BitAnd => (self.bin(GB::And, &g, a, b), ta.clone()),
            BinOp::Eq => (self.cmp(CmpPred::Eq, &g, a, b), Ty::Bool),
            BinOp::Ne => (self.cmp(CmpPred::Ne, &g, a, b), Ty::Bool),
            BinOp::Lt => (
                self.cmp(pick(CmpPred::Slt, CmpPred::Ult), &g, a, b),
                Ty::Bool,
            ),
            BinOp::Le => (
                self.cmp(pick(CmpPred::Sle, CmpPred::Ule), &g, a, b),
                Ty::Bool,
            ),
            BinOp::Gt => (
                self.cmp(pick(CmpPred::Sgt, CmpPred::Ugt), &g, a, b),
                Ty::Bool,
            ),
            BinOp::Ge => (
                self.cmp(pick(CmpPred::Sge, CmpPred::Uge), &g, a, b),
                Ty::Bool,
            ),
            BinOp::AddSat | BinOp::SubSat | BinOp::MulSat => return un("phép bão hoà"),
        };
        Ok((r, rty))
    }

    /// `(kết quả bao quanh, có tràn: i1)` bằng công thức tường minh (không intrinsic).
    fn checked(&mut self, op: BinOp, it: IntTy, a: Opd, b: Opd) -> R<(Opd, Opd)> {
        let g = gint(it);
        let signed = it.signed();
        let zero = Opd::Int(g.clone(), 0);
        match op {
            BinOp::Add => {
                let r = self.bin(GB::Add, &g, a.clone(), b.clone());
                let o = if signed {
                    let x = self.bin(GB::Xor, &g, a, r.clone());
                    let y = self.bin(GB::Xor, &g, b, r.clone());
                    let z = self.bin(GB::And, &g, x, y);
                    self.cmp(CmpPred::Slt, &g, z, zero)
                } else {
                    self.cmp(CmpPred::Ult, &g, r.clone(), a)
                };
                Ok((r, o))
            }
            BinOp::Sub => {
                let r = self.bin(GB::Sub, &g, a.clone(), b.clone());
                let o = if signed {
                    let x = self.bin(GB::Xor, &g, a.clone(), b);
                    let y = self.bin(GB::Xor, &g, a, r.clone());
                    let z = self.bin(GB::And, &g, x, y);
                    self.cmp(CmpPred::Slt, &g, z, zero)
                } else {
                    self.cmp(CmpPred::Ult, &g, a, b)
                };
                Ok((r, o))
            }
            BinOp::Mul => {
                let wide = match it.bits() {
                    8 => G::I16,
                    16 => G::I32,
                    32 => G::I64,
                    _ => return un("nhân có kiểm 64 bit"),
                };
                let ext = if signed { CastOp::SExt } else { CastOp::ZExt };
                let wa = self.cast(ext, &g, a.clone(), &wide);
                let wb = self.cast(ext, &g, b.clone(), &wide);
                let wm = self.bin(GB::Mul, &wide, wa, wb);
                let r = self.cast(CastOp::Trunc, &wide, wm.clone(), &g);
                let back = self.cast(ext, &g, r.clone(), &wide);
                let o = self.cmp(CmpPred::Ne, &wide, back, wm);
                Ok((r, o))
            }
            _ => un("CheckedBinaryOp ngoài Add/Sub/Mul"),
        }
    }

    fn terminator(&mut self, bi: usize, t: &TerminatorKind) -> R<()> {
        match t {
            TerminatorKind::Goto(b) => self.finish(Term::Br(Self::block_name(b.index())), None),
            TerminatorKind::Return => {
                let rty = self.local_tys[0].clone();
                if is_unit(&rty) {
                    self.finish(Term::Ret(None), None);
                } else {
                    let (v, ty) = self.load(&Place::local(gt_mir::Local(0)))?;
                    let g = gty(self.db, &ty)?;
                    self.finish(Term::Ret(Some((g, v))), None);
                }
            }
            TerminatorKind::Unreachable => self.finish(Term::Abort("unreachable".into()), None),
            TerminatorKind::SwitchInt {
                discr,
                targets,
                otherwise,
            } => {
                let (d, ty) = self.operand(discr)?;
                let g = gty(self.db, &ty)?;
                for (k, (val, tgt)) in targets.iter().enumerate() {
                    let c = self.cmp(
                        CmpPred::Eq,
                        &g,
                        d.clone(),
                        Opd::Int(g.clone(), *val as i128),
                    );
                    let next = format!("bb{bi}.sw{k}");
                    self.finish(
                        Term::CondBr(c, Self::block_name(tgt.index()), next.clone()),
                        Some(next),
                    );
                }
                self.finish(Term::Br(Self::block_name(otherwise.index())), None);
            }
            TerminatorKind::Assert {
                cond,
                expected,
                kind,
                target,
            } => {
                let (c, _) = self.operand(cond)?;
                if let AssertKind::CastLossy { .. } = kind {
                    // vẫn hạ được: chỉ cần `cond`
                }
                self.extra += 1;
                let bad = format!("bb{bi}.abort{}", self.extra);
                let tgt = Self::block_name(target.index());
                let term = if *expected {
                    Term::CondBr(c, tgt, bad.clone())
                } else {
                    Term::CondBr(c, bad.clone(), tgt)
                };
                self.finish(term, Some(bad));
                self.finish(Term::Abort(kind.name().to_string()), None);
            }
            TerminatorKind::Call {
                callee,
                args,
                dest,
                target,
                ..
            } => {
                // intrinsic `len` của slice/`str`: đọc trường độ dài của con trỏ béo (không có thân MIR)
                if let gt_mir::Callee::Fn { node, .. } = callee
                    && matches!(
                        self.cx.keys.get(node).map(String::as_str),
                        Some("$prelude::[T0]::len" | "$prelude::str::len")
                    )
                    && args.len() == 1
                    && let Some(tg) = target
                {
                    let (v, _) = self.operand(&args[0])?;
                    let fat = G::Struct(vec![G::Ptr, G::I64]);
                    self.extra += 1;
                    let sname = format!("len{}", self.extra);
                    self.aux_slots.push((sname.clone(), fat.clone()));
                    self.push(Inst::Store {
                        ty: fat.clone(),
                        val: v,
                        ptr: Opd::Slot(sname.clone()),
                    });
                    let f1 = self.t();
                    self.push(Inst::FieldPtr {
                        d: f1.clone(),
                        ty: fat,
                        ptr: Opd::Slot(sname),
                        idx: 1,
                    });
                    let l = self.t();
                    self.push(Inst::Load {
                        d: l.clone(),
                        ty: G::I64,
                        ptr: Opd::Val(f1),
                    });
                    self.store(dest, Opd::Val(l), &Ty::Int(IntTy::Usize))?;
                    self.finish(Term::Br(Self::block_name(tg.index())), None);
                    return Ok(());
                }
                if let gt_mir::Callee::Fn { node, generics } = callee
                    && !self.cx.bodies.contains_key(node)
                    && let Some(key) = self.cx.keys.get(node)
                {
                    let Some(tg) = target else {
                        return un("Call không trở về");
                    };
                    let gs: Vec<Ty> = generics.iter().map(|g| self.sub(g)).collect();
                    if self.runtime_call(key, &gs, args, dest)? {
                        self.finish(Term::Br(Self::block_name(tg.index())), None);
                        return Ok(());
                    }
                }
                let inst = match callee {
                    gt_mir::Callee::Fn { node, generics } => Instance {
                        node: *node,
                        args: generics.iter().map(|g| self.sub(g)).collect(),
                    },
                    gt_mir::Callee::TraitMethod {
                        trait_def,
                        member,
                        self_ty,
                        generics,
                    } => {
                        // đơn hình hoá: thế tham số của instance đang hạ rồi phân giải NGAY bằng bảng impl (một nguồn với codegen/thông dịch MIR)
                        let st = self.sub(self_ty);
                        let ga: Vec<Ty> = generics.iter().map(|g| self.sub(g)).collect();
                        if st.has_param() || st.has_infer() {
                            return un("TraitMethod trên kiểu chưa thế");
                        }
                        let Some((node, all)) =
                            self.db.resolve_trait_method(*trait_def, *member, &st, &ga)
                        else {
                            return un("TraitMethod không phân giải được");
                        };
                        Instance { node, args: all }
                    }
                    gt_mir::Callee::Value(_) => return un("Call không phải hàm trực tiếp"),
                };
                let Some(tg) = target else {
                    return un("Call không trở về");
                };
                let Some((name, rty)) = self.cx.sig(&inst) else {
                    return un(format!(
                        "Call tới hàm ngoài tập con: {}",
                        self.cx.keys.get(&inst.node).map_or("?", |k| k.as_str())
                    ));
                };
                self.calls.push(inst);
                let mut vals = Vec::new();
                for a in args {
                    let (v, t) = self.operand(a)?;
                    if !is_unit(&t) {
                        vals.push(v);
                    }
                }
                if is_unit(&rty) {
                    self.push(Inst::Call {
                        d: None,
                        ret: G::Void,
                        callee: name,
                        args: vals,
                    });
                } else {
                    let d = self.t();
                    let g = gty(self.db, &rty)?;
                    self.push(Inst::Call {
                        d: Some(d.clone()),
                        ret: g,
                        callee: name,
                        args: vals,
                    });
                    self.store(dest, Opd::Val(d), &rty)?;
                }
                self.finish(Term::Br(Self::block_name(tg.index())), None);
            }
            TerminatorKind::Drop { place, target } => {
                // drop glue: MỘT nguồn sự thật `TypeDb::drop_glue` (như codegen LLVM và thông dịch MIR); glue rỗng (vd `T` thế bằng kiểu số) ⇒ chỉ nhảy
                let Some(pty) = gt_mir::types::place_ty(self.body, place) else {
                    return un("Drop: không biết kiểu chỗ");
                };
                let ty = self.sub(&pty);
                if ty.has_param() || ty.has_infer() {
                    return un("Drop trên kiểu chưa thế");
                }
                let g = self
                    .db
                    .drop_glue(&ty)
                    .map_err(|e| Unsupported(format!("drop glue: {e}")))?;
                if !g.is_nothing() {
                    // như codegen: glue có việc chỉ trên thân ĐÃ elaboration (cờ drop nằm trong MIR)
                    if !self.body.drops_elaborated {
                        return un("Drop có glue trên thân chưa elaboration");
                    }
                    let (ptr, pty2) = self.place(place)?;
                    if pty2 != ty {
                        return un("Drop: kiểu chỗ lệch");
                    }
                    self.drop_glue_at(ptr, &ty, &g)?;
                }
                self.finish(Term::Br(Self::block_name(target.index())), None)
            }
        }
        Ok(())
    }
}

/// Hạ một thân `Fn` không generic. Trả hàm GLIR (chưa verify) hoặc lý do chưa hạ được.
pub fn lower_body(db: &TypeDb, body: &Body) -> R<Func> {
    let keys = BTreeMap::new();
    let cx = Ctx {
        db,
        keys: &keys,
        defs: &[],
        bodies: BTreeMap::new(),
        sigs: RefCell::new(Sigs::new()),
    };
    let inst = Instance {
        node: body.node,
        args: Vec::new(),
    };
    let (f, globals, _) = lower_inst(&cx, body, &inst)?;
    if !globals.is_empty() {
        return un("thân dùng chuỗi hằng: cần lower_module");
    }
    Ok(f)
}

/// Tên hàm GLIR từ khoá thân MIR: ký tự ngoài `[A-Za-z0-9_.]` đổi thành `_`; nếu có đổi thì thêm hậu tố băm khoá để KHÔNG trùng tên.
#[must_use]
pub fn glir_name(key: &str) -> String {
    let clean: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if clean == key {
        clean
    } else {
        format!(
            "{clean}.{:06x}",
            gt_types::fnv1a64(key.as_bytes()) & 0xff_ffff
        )
    }
}

/// Tên hàm GLIR của một instance: không đối số kiểu ⇒ `glir_name(key)`; có ⇒ `glir_name(key)` + `.` + băm 64 bit của khoá CHUẨN
/// (đúng phần băm của `gt_types::instance_name`, nên hai instance khác nhau không trùng tên).
#[must_use]
pub fn inst_name(key: &str, args: &[Ty], defs: &[gt_resolve::Def]) -> String {
    if args.is_empty() {
        return glir_name(key);
    }
    let (sym, _) = gt_types::instance_name(key, args, defs);
    let hash = sym.rsplit('.').next().unwrap_or_default();
    format!("{}.{hash}", glir_name(key))
}

/// Kết quả hạ cả tập thân.
pub struct Lowered {
    pub module: gt_glir::Module,
    /// tên hàm GLIR → instance, cho MỌI hàm trong `module`
    pub funcs: BTreeMap<String, Instance>,
    /// instance không hạ được: (khoá thân, instance, lý do)
    pub bad: Vec<(String, Instance, Unsupported)>,
    /// khoá các thân `Fn` generic không có instance nào (không hạ)
    pub no_instance: Vec<String>,
}

/// Hạ cả tập thân: hàm nào gọi (trực tiếp) hàm cùng tập con thì được hạ kèm lời gọi; hàm gọi tới hàm không hạ được bị loại (lặp tới điểm cố định).
#[must_use]
pub fn lower_module(t: &gt_types::TypesOutput, mir: &gt_mir::MirOutput) -> Lowered {
    let db = &t.db;
    let fns: Vec<&Body> = mir
        .bodies
        .iter()
        .filter(|b| b.kind == BodyKind::Fn)
        .collect();
    let cx = Ctx {
        db,
        keys: &mir.keys,
        defs: &t.resolve.defs,
        // thân hợp đồng `⊳`/`⊲` (M7 chèn lời gọi tới chúng tại điểm gọi — FB-P2-03): hạ theo nhu cầu như hàm thường (codegen LLVM cũng vậy)
        bodies: mir
            .bodies
            .iter()
            .filter(|b| matches!(b.kind, BodyKind::Fn | BodyKind::Pre | BodyKind::Post))
            .map(|b| (b.node, b))
            .collect(),
        sigs: RefCell::new(Sigs::new()),
    };
    let mut ok: BTreeMap<String, (Instance, Func)> = BTreeMap::new();
    let mut bad: Vec<(String, Instance, Unsupported)> = Vec::new();
    let mut globals: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    // FB-G2-04: tên GLIR trùng giữa hai instance khác nhau (va chạm băm 24 bit của `glir_name`) ⇒ loại CẢ HAI (fail-closed; lời gọi theo tên
    // sẽ trỏ nhầm thân), không ghi đè im lặng
    let mut clash: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // gốc: thân không generic (instance args=[]) và mọi instance của bảng M4; thêm theo nhu cầu các instance được gọi (vd hàm generic của
    // prelude — không có trong bảng M4), như hàng đợi `pending` của codegen LLVM
    let mut seen: std::collections::BTreeSet<Instance> = std::collections::BTreeSet::new();
    let mut queue: std::collections::VecDeque<Instance> = std::collections::VecDeque::new();
    for b in &fns {
        if db.fn_params(b.sig_node).is_empty() {
            queue.push_back(Instance {
                node: b.node,
                args: Vec::new(),
            });
        }
    }
    queue.extend(
        t.instances
            .iter()
            .filter(|i| !i.args.is_empty() && cx.bodies.contains_key(&i.node))
            .cloned(),
    );
    while let Some(inst) = queue.pop_front() {
        if !seen.insert(inst.clone()) {
            continue;
        }
        let Some(b) = cx.bodies.get(&inst.node).copied() else {
            continue;
        };
        if seen.len() > 2 * gt_types::MAX_PROGRAM_INSTANCES {
            bad.push((
                b.key.clone(),
                inst,
                Unsupported("quá nhiều instance".into()),
            ));
            continue;
        }
        if inst
            .args
            .iter()
            .any(|a| a.depth() > gt_types::MAX_INSTANCE_DEPTH)
        {
            bad.push((
                b.key.clone(),
                inst,
                Unsupported("instance lồng quá sâu".into()),
            ));
            continue;
        }
        match lower_inst(&cx, b, &inst) {
            Ok((f, gs, calls)) => {
                queue.extend(calls.into_iter().filter(|c| !seen.contains(c)));
                let conflict = gs
                    .iter()
                    .any(|g| globals.get(&g.name).is_some_and(|x| *x != g.bytes));
                if conflict {
                    bad.push((
                        b.key.clone(),
                        inst,
                        Unsupported("global trùng tên khác nội dung".into()),
                    ));
                    continue;
                }
                if clash.contains(&f.name) || ok.contains_key(&f.name) {
                    if let Some((pi, _)) = ok.remove(&f.name) {
                        let key = cx
                            .bodies
                            .get(&pi.node)
                            .map_or(f.name.clone(), |b| b.key.clone());
                        bad.push((key, pi, Unsupported("tên GLIR trùng".into())));
                    }
                    clash.insert(f.name.clone());
                    bad.push((b.key.clone(), inst, Unsupported("tên GLIR trùng".into())));
                    continue;
                }
                for g in gs {
                    globals.insert(g.name, g.bytes);
                }
                ok.insert(f.name.clone(), (inst, f));
            }
            Err(u) => bad.push((b.key.clone(), inst, u)),
        }
    }
    loop {
        let names: std::collections::BTreeSet<String> = ok.keys().cloned().collect();
        let drop: Vec<String> = ok
            .values()
            .filter(|(_, f)| {
                f.blocks.iter().flat_map(|b| &b.insts).any(|i| {
                    matches!(i, Inst::Call { callee, .. }
                            if !names.contains(callee) && runtime_decl(callee).is_none())
                })
            })
            .map(|(_, f)| f.name.clone())
            .collect();
        if drop.is_empty() {
            break;
        }
        for n in drop {
            if let Some((inst, _)) = ok.remove(&n) {
                let key = cx.bodies.get(&inst.node).map_or(n, |b| b.key.clone());
                bad.push((key, inst, Unsupported("callee không hạ được".into())));
            }
        }
    }
    // `declare` cho MỌI hàm runtime mà các hàm còn lại gọi (theo tên, tất định)
    let decls: Vec<gt_glir::Decl> = ok
        .values()
        .flat_map(|(_, f)| f.blocks.iter().flat_map(|b| &b.insts))
        .filter_map(|i| match i {
            Inst::Call { callee, .. } if !ok.contains_key(callee) => Some(callee.clone()),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>()
        .iter()
        .filter_map(|n| runtime_decl(n))
        .collect();
    let seen_nodes: std::collections::BTreeSet<gt_ast::NodeId> =
        seen.iter().map(|i| i.node).collect();
    let funcs = ok
        .iter()
        .map(|(n, (i, _))| (n.clone(), i.clone()))
        .collect();
    Lowered {
        module: gt_glir::Module {
            globals: globals
                .into_iter()
                .map(|(name, bytes)| gt_glir::Global { name, bytes })
                .collect(),
            decls,
            funcs: ok.into_values().map(|(_, f)| f).collect(),
        },
        funcs,
        bad,
        no_instance: fns
            .iter()
            .filter(|b| !seen_nodes.contains(&b.node))
            .map(|b| b.key.clone())
            .collect(),
    }
}

/// Hạ thân `body` theo instance `inst`: (hàm, global dùng tới, instance được gọi trực tiếp).
fn lower_inst(
    cx: &Ctx<'_>,
    body: &Body,
    inst: &Instance,
) -> R<(Func, Vec<gt_glir::Global>, Vec<Instance>)> {
    let db = cx.db;
    if !matches!(body.kind, BodyKind::Fn | BodyKind::Pre | BodyKind::Post) {
        return un("không phải thân Fn/hợp đồng");
    }
    let params = db.fn_params(body.sig_node);
    if params.len() != inst.args.len() {
        return un("số đối số kiểu của instance không khớp số tham số");
    }
    let local_tys: Vec<Ty> = body
        .locals
        .iter()
        .map(|l| subst_inst(&l.ty, &params, &inst.args))
        .collect();
    let mut lw = Lw {
        cx,
        db,
        body,
        params,
        args: inst.args.clone(),
        local_tys,
        slots: BTreeMap::new(),
        blocks: Vec::new(),
        cur: Vec::new(),
        cur_name: "entry".into(),
        tmp: 0,
        extra: 0,
        aux_slots: Vec::new(),
        globals: Vec::new(),
        calls: Vec::new(),
    };
    // tham số: store vào slot ở khối vào
    let mut params = Vec::new();
    for i in 1..=body.arg_count {
        let ty = lw.local_tys[i].clone();
        if is_unit(&ty) {
            continue;
        }
        let g = gty(db, &ty)?;
        params.push((format!("p{i}"), g.clone()));
        let s = lw.slot(i)?;
        lw.push(Inst::Store {
            ty: g,
            val: Opd::Val(format!("p{i}")),
            ptr: Opd::Slot(s),
        });
    }
    let ret = if is_unit(&lw.local_tys[0]) {
        G::Void
    } else {
        gty(db, &lw.local_tys[0])?
    };
    lw.finish(Term::Br("bb0".into()), Some("bb0".into()));
    for (bi, bd) in body.blocks.iter().enumerate() {
        if bi > 0 {
            lw.cur_name = Lw::block_name(bi);
        }
        for s in &bd.statements {
            match &s.kind {
                StatementKind::Assign(p, rv) => lw.rvalue(p, rv)?,
                StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::FakeRead(..)
                | StatementKind::Nop => {}
                StatementKind::Wipe(_) => return un("Wipe"),
            }
        }
        let Some(t) = &bd.terminator else {
            return un("khối thiếu terminator");
        };
        lw.terminator(bi, &t.kind)?;
    }
    let mut slots: Vec<(String, G)> = lw
        .slots
        .iter()
        .map(|(l, t)| (format!("l{l}"), t.clone()))
        .collect();
    slots.sort_by(|a, b| a.0.cmp(&b.0));
    slots.extend(lw.aux_slots.iter().cloned());
    slots.sort_by(|a, b| a.0.cmp(&b.0));
    Ok((
        Func {
            name: inst_name(&body.key, &inst.args, cx.defs),
            params,
            ret,
            slots,
            blocks: lw.blocks,
        },
        lw.globals,
        lw.calls,
    ))
}

/// Kiểm (d) của ADR-P2-02 (cố vấn Opus): với mỗi loại abort `k`, số khối `Abort(k)` TỚI ĐƯỢC trong GLIR ≥ số `Assert` loại `k` (cộng `Unreachable`
/// ⇒ `unreachable`) TỚI ĐƯỢC trong MIR. Đóng giới hạn Guard "không đếm được Assert đã bị xoá khi hạ". Trả lỗi dạng `G0501: …`.
#[must_use]
pub fn check_abort_cover(body: &Body, f: &Func) -> Vec<String> {
    let mut mir: BTreeMap<String, usize> = BTreeMap::new();
    let n = body.blocks.len();
    let mut seen = vec![false; n];
    let mut stack = vec![0usize];
    while let Some(i) = stack.pop() {
        if i >= n || std::mem::replace(&mut seen[i], true) {
            continue;
        }
        let Some(t) = &body.blocks[i].terminator else {
            continue;
        };
        match &t.kind {
            TerminatorKind::Assert { kind, .. } => {
                *mir.entry(kind.name().to_string()).or_default() += 1
            }
            TerminatorKind::Unreachable => *mir.entry("unreachable".to_string()).or_default() += 1,
            _ => {}
        }
        stack.extend(t.kind.successors().iter().map(|b| b.index()));
    }
    let idx: BTreeMap<&str, usize> = f
        .blocks
        .iter()
        .enumerate()
        .map(|(i, b)| (b.name.as_str(), i))
        .collect();
    let mut glir: BTreeMap<String, usize> = BTreeMap::new();
    let mut seen = vec![false; f.blocks.len()];
    let mut stack = vec![0usize];
    while let Some(i) = stack.pop() {
        if i >= f.blocks.len() || std::mem::replace(&mut seen[i], true) {
            continue;
        }
        match &f.blocks[i].term {
            Term::Abort(k) => *glir.entry(k.clone()).or_default() += 1,
            Term::Br(t) => stack.extend(idx.get(t.as_str()).copied()),
            // FB-G2-04: điều kiện HẰNG chỉ đi một nhánh (một `Assert` hạ thành `condbr i1 1, ok, abort` không được tính là còn chốt)
            Term::CondBr(Opd::Int(_, v), a, b) => {
                stack.extend(idx.get(if v & 1 == 1 { a } else { b }.as_str()).copied())
            }
            Term::CondBr(_, a, b) => {
                stack.extend([a, b].iter().filter_map(|t| idx.get(t.as_str()).copied()))
            }
            Term::Ret(_) => {}
        }
    }
    mir.iter()
        .filter(|(k, c)| glir.get(*k).copied().unwrap_or(0) < **c)
        .map(|(k, c)| {
            format!(
                "G0501: @{}: loại abort `{k}`: MIR có {c} Assert tới được, GLIR chỉ {} khối Abort",
                f.name,
                glir.get(k).copied().unwrap_or(0)
            )
        })
        .collect()
}
