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
use gt_types::{IntTy, Ty, TypeDb};
use std::collections::BTreeMap;

/// Lý do (ngắn, ổn định) một thân chưa hạ được — dùng để đếm độ phủ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unsupported(pub String);

type R<T> = Result<T, Unsupported>;

fn un<T>(why: impl Into<String>) -> R<T> {
    Err(Unsupported(why.into()))
}

/// callee (nút) → (tên GLIR, kiểu trả về Grit) của các thân có chữ ký trong tập con (có thể gọi trực tiếp)
pub type Sigs = BTreeMap<gt_ast::NodeId, (String, Ty)>;

struct Lw<'a> {
    sigs: &'a Sigs,
    db: &'a TypeDb,
    body: &'a Body,
    slots: BTreeMap<usize, G>,
    blocks: Vec<Block>,
    cur: Vec<Inst>,
    cur_name: String,
    tmp: u32,
    extra: u32,
}

fn is_int_or_bool(t: &Ty) -> bool {
    match t {
        Ty::Int(it) => it.bits() <= 64,
        Ty::Bool => true,
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
        Ty::Adt(def, args) => {
            plain_struct(db, def)
                && crate::variant_fields(db, *def, args, 0)
                    .is_ok_and(|fs| fs.iter().all(|x| in_sub(db, x, depth + 1)))
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
    matches!(t, Ty::Tuple(v) if v.is_empty())
}

impl Lw<'_> {
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
            let ty = gty(self.db, &self.body.locals[l].ty)?;
            self.slots.insert(l, ty);
        }
        Ok(format!("l{l}"))
    }
    /// con trỏ tới chỗ (slot, rồi chuỗi trường tuple/struct) + kiểu Grit của chỗ
    fn place(&mut self, p: &Place) -> R<(Opd, Ty)> {
        let l = p.local.index();
        let mut cur_ty = self.body.locals[l].ty.clone();
        let mut ptr = Opd::Slot(self.slot(l)?);
        for e in &p.projection {
            let PlaceElem::Field(i, fty) = e else {
                return un("chỗ có phép chiếu ngoài trường tuple/struct");
            };
            if !matches!(cur_ty, Ty::Tuple(_) | Ty::Adt(..)) {
                return un("trường của kiểu không phải tuple/struct");
            }
            let gt = gty(self.db, &cur_ty)?;
            let d = self.t();
            self.push(Inst::FieldPtr {
                d: d.clone(),
                ty: gt,
                ptr,
                idx: *i,
            });
            ptr = Opd::Val(d);
            cur_ty = fty.clone();
        }
        Ok((ptr, cur_ty))
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
                let (v, from) = self.operand(o)?;
                let (Ty::Int(_) | Ty::Bool, Ty::Int(_)) = (&from, to) else {
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
            Rvalue::CastFits(..) => un("CastFits"),
            Rvalue::Aggregate(kind, ops) => {
                let (ptr, dty) = self.place(dest)?;
                let field_tys: Vec<Ty> = match (kind, &dty) {
                    (gt_mir::AggregateKind::Tuple, Ty::Tuple(v)) => v.clone(),
                    (gt_mir::AggregateKind::Adt(def, 0, args), Ty::Adt(..))
                        if plain_struct(self.db, def) =>
                    {
                        crate::variant_fields(self.db, *def, args, 0).map_err(Unsupported)?
                    }
                    _ => return un("Aggregate ngoài tuple/struct"),
                };
                if field_tys.len() != ops.len() {
                    return un("Aggregate sai số trường");
                }
                let gd = gty(self.db, &dty)?;
                for (i, (o, ft)) in ops.iter().zip(&field_tys).enumerate() {
                    if is_unit(ft) {
                        continue;
                    }
                    let (v, _) = self.operand(o)?;
                    let fp = self.t();
                    self.push(Inst::FieldPtr {
                        d: fp.clone(),
                        ty: gd.clone(),
                        ptr: ptr.clone(),
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
            Rvalue::Repeat(..) | Rvalue::Ref(..) | Rvalue::Len(_) | Rvalue::Discriminant(_) => {
                un("Rvalue ngoài tập con")
            }
        }
    }

    fn binop(&mut self, op: BinOp, ta: &Ty, a: Opd, b: Opd) -> R<(Opd, Ty)> {
        let (g, signed) = match ta {
            Ty::Int(it) => (gint(*it), it.signed()),
            Ty::Bool => (G::I1, false),
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
                let rty = self.body.locals[0].ty.clone();
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
                let gt_mir::Callee::Fn { node, generics } = callee else {
                    return un("Call không phải hàm trực tiếp");
                };
                if !generics.is_empty() {
                    return un("Call hàm generic");
                }
                let Some(tg) = target else {
                    return un("Call không trở về");
                };
                let Some((name, rty)) = self.sigs.get(node) else {
                    return un("Call tới hàm ngoài tập con");
                };
                let (name, rty) = (name.clone(), rty.clone());
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
            TerminatorKind::Drop { .. } => return un("Drop"),
        }
        Ok(())
    }
}

/// Hạ một thân `Fn` không generic. Trả hàm GLIR (chưa verify) hoặc lý do chưa hạ được.
pub fn lower_body(db: &TypeDb, body: &Body) -> R<Func> {
    lower_body_with(db, body, &Sigs::new())
}

fn glir_name(key: &str) -> String {
    key.replace(['<', '>', ' ', ':'], "_")
}

/// Hạ cả tập thân: hàm nào gọi (trực tiếp) hàm cùng tập con thì được hạ kèm lời gọi; hàm gọi tới hàm không hạ được bị loại (lặp tới điểm cố định).
#[must_use]
pub fn lower_module(db: &TypeDb, bodies: &[Body]) -> (gt_glir::Module, Vec<(String, Unsupported)>) {
    let mut sigs = Sigs::new();
    for b in bodies.iter().filter(|b| b.kind == BodyKind::Fn) {
        let ok = (0..=b.arg_count).all(|i| in_subset(db, &b.locals[i].ty));
        if ok {
            sigs.insert(b.node, (glir_name(&b.key), b.locals[0].ty.clone()));
        }
    }
    let mut ok: BTreeMap<String, Func> = BTreeMap::new();
    let mut bad: Vec<(String, Unsupported)> = Vec::new();
    for b in bodies.iter().filter(|b| b.kind == BodyKind::Fn) {
        match lower_body_with(db, b, &sigs) {
            Ok(f) => {
                ok.insert(f.name.clone(), f);
            }
            Err(u) => bad.push((b.key.clone(), u)),
        }
    }
    loop {
        let names: std::collections::BTreeSet<String> = ok.keys().cloned().collect();
        let drop: Vec<String> = ok
            .values()
            .filter(|f| {
                f.blocks
                    .iter()
                    .flat_map(|b| &b.insts)
                    .any(|i| matches!(i, Inst::Call { callee, .. } if !names.contains(callee)))
            })
            .map(|f| f.name.clone())
            .collect();
        if drop.is_empty() {
            break;
        }
        for n in drop {
            ok.remove(&n);
            bad.push((n, Unsupported("callee không hạ được".into())));
        }
    }
    (
        gt_glir::Module {
            decls: Vec::new(),
            funcs: ok.into_values().collect(),
        },
        bad,
    )
}

fn lower_body_with(db: &TypeDb, body: &Body, sigs: &Sigs) -> R<Func> {
    if body.kind != BodyKind::Fn {
        return un("không phải thân Fn");
    }
    let mut lw = Lw {
        sigs,
        db,
        body,
        slots: BTreeMap::new(),
        blocks: Vec::new(),
        cur: Vec::new(),
        cur_name: "entry".into(),
        tmp: 0,
        extra: 0,
    };
    // tham số: store vào slot ở khối vào
    let mut params = Vec::new();
    for i in 1..=body.arg_count {
        let ty = body.locals[i].ty.clone();
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
    let ret = if is_unit(&body.locals[0].ty) {
        G::Void
    } else {
        gty(db, &body.locals[0].ty)?
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
    Ok(Func {
        name: glir_name(&body.key),
        params,
        ret,
        slots,
        blocks: lw.blocks,
    })
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
