//! Hạ MỘT thân MIR (`BodyKind::Fn`, tập con vô hướng) sang hàm GLIR (ADR-P2-02 C3; thiết kế theo cố vấn Opus 2026-11-25).
//! Mỗi local MIR là một slot `$lN` có kiểu; mọi lần dùng là `load`, mọi lần ghi là `store` (khớp `alloca` của codegen LLVM; mem2reg để sau).
//! `Assert` ⇒ `condbr` tới khối đích hoặc một khối riêng chỉ chứa `abort <loại>` (một khối cho MỖI Assert). Mọi thứ ngoài tập con ⇒ `Unsupported(lý do)`.
//! Tập con: kiểu `Int`(≤ 64 bit)/`Bool`/`()`/tuple `(Int, Bool)`; `Use`, `BinaryOp`(số học bao quanh, so sánh, `BitAnd`, `Div/Rem`), `CheckedBinaryOp`(Add/Sub/Mul),
//! `UnaryOp`, `Cast` số nguyên/bool→số nguyên; `Goto`/`SwitchInt`/`Return`/`Assert`/`Unreachable`.
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

fn in_subset(db: &TypeDb, t: &Ty) -> bool {
    in_sub(db, t, 0)
}

fn in_sub(db: &TypeDb, t: &Ty, depth: u32) -> bool {
    if depth > 8 {
        return false;
    }
    match t {
        Ty::Tuple(v) => v.iter().all(|x| in_sub(db, x, depth + 1)),
        Ty::Ref(_, inner) => match &**inner {
            Ty::Str => true,
            Ty::Slice(e) => in_sub(db, e, depth + 1),
            i => in_sub(db, i, depth + 1),
        },
        Ty::Array(e, n) => *n <= 4096 && in_sub(db, e, depth + 1),
        Ty::Adt(def, args) => {
            if plain_struct(db, def) {
                return crate::variant_fields(db, *def, args, 0)
                    .is_ok_and(|fs| fs.iter().all(|x| in_sub(db, x, depth + 1)));
            }
            match db.adts.get(def).map(|a| &a.kind) {
                Some(gt_types::AdtKind::Enum(vs)) => (0..vs.len()).all(|v| {
                    crate::variant_fields(db, *def, args, v)
                        .is_ok_and(|fs| fs.iter().all(|x| in_sub(db, x, depth + 1)))
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
                let (Ty::Int(_) | Ty::Bool | Ty::Char, Ty::Int(_)) = (&from, to) else {
                    return un("Cast ngoài int/bool→int");
                };
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
                match self.db.drop_glue(&ty) {
                    Ok(g) if g.is_nothing() => {
                        self.finish(Term::Br(Self::block_name(target.index())), None)
                    }
                    _ => return un("Drop có glue"),
                }
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
        bodies: fns.iter().map(|b| (b.node, *b)).collect(),
        sigs: RefCell::new(Sigs::new()),
    };
    let mut ok: BTreeMap<String, (Instance, Func)> = BTreeMap::new();
    let mut bad: Vec<(String, Instance, Unsupported)> = Vec::new();
    let mut globals: BTreeMap<String, Vec<u8>> = BTreeMap::new();
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
                f.blocks
                    .iter()
                    .flat_map(|b| &b.insts)
                    .any(|i| matches!(i, Inst::Call { callee, .. } if !names.contains(callee)))
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
            decls: Vec::new(),
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
    if body.kind != BodyKind::Fn {
        return un("không phải thân Fn");
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
