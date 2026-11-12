//! Hạ biểu thức và câu lệnh. Thứ tự đánh giá trái sang phải (semantics §3a); `Assert` cho mọi nghĩa vụ (ADR-020 §9).
use super::{Builder, R, ice};
use crate::mir::*;
use crate::types::{place_ty, variant_fields};
use gt_ast::{
    BinaryOp, Block, Expr, ExprKind, FieldName, Lit, LitKind, Loop, LoopKind, Stmt, StmtKind,
    UnaryOp,
};
use gt_resolve::{DefKind, Res};
use gt_types::{AdtKind, CallTarget, FloatTy, IntTy, Lang, Ty};

/// Giải escape của literal chuỗi.
pub(crate) fn decode_str(s: &str) -> Option<String> {
    // AST giữ nội dung KHÔNG gồm dấu `"` (escape còn nguyên)
    let inner = s
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .unwrap_or(s);
    let mut out = String::new();
    let mut it = inner.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next()? {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            '0' => out.push('\0'),
            '\\' => out.push('\\'),
            '\'' => out.push('\''),
            '"' => out.push('"'),
            'u' => {
                if it.next()? != '{' {
                    return None;
                }
                let mut hex = String::new();
                loop {
                    let h = it.next()?;
                    if h == '}' {
                        break;
                    }
                    hex.push(h);
                }
                out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
            }
            _ => return None,
        }
    }
    Some(out)
}

fn mask(ty: IntTy) -> u128 {
    if ty.bits() == 128 {
        u128::MAX
    } else {
        (1u128 << ty.bits()) - 1
    }
}

/// Hằng số nguyên dạng mẫu bit của kiểu `ty`.
pub(crate) fn int_const(v: i128, ty: IntTy) -> Const {
    Const::Int {
        bits: (v as u128) & mask(ty),
        ty,
    }
}

/// `MIN` của kiểu có dấu.
fn min_const(ty: IntTy) -> Const {
    int_const(ty.min_val(), ty)
}

/// Ép `from as to` có thể mất thông tin (nghĩa vụ D1)?
pub(crate) fn cast_may_lose(from: &Ty, to: &Ty) -> bool {
    match (from, to) {
        (a, b) if a == b => false,
        (Ty::Int(f), Ty::Int(t)) => {
            let widen_same = f.signed() == t.signed() && t.bits() >= f.bits();
            let u_to_wider_s = !f.signed() && t.signed() && t.bits() > f.bits();
            !(widen_same || u_to_wider_s)
        }
        (Ty::Bool, Ty::Int(_)) => false,
        (Ty::Char, Ty::Int(t)) => t.bits() < 32,
        (Ty::Int(IntTy::U8), Ty::Char) => false,
        (Ty::Int(f), Ty::Float(t)) => {
            let mant = if *t == FloatTy::F32 { 24 } else { 53 };
            f.bits() > mant || (f.signed() && f.bits() > mant)
        }
        (Ty::Float(FloatTy::F32), Ty::Float(FloatTy::F64)) => false,
        _ => true,
    }
}

impl Builder<'_> {
    // ---------------- tiện ích ----------------

    pub(crate) fn operand_of(&self, p: Place, ty: &Ty) -> Operand {
        if self.is_copy(ty) {
            Operand::Copy(p)
        } else {
            Operand::Move(p)
        }
    }

    /// `Assert(cond == expected)` rồi đi tiếp ở khối mới.
    pub(crate) fn assert(&mut self, cond: Operand, expected: bool, kind: AssertKind) -> R<()> {
        let next = self.new_block();
        self.terminate(TerminatorKind::Assert {
            cond,
            expected,
            kind,
            target: next,
        })?;
        self.cur = next;
        Ok(())
    }

    /// Rẽ nhánh theo một rvalue (bool/số nguyên): giá trị nằm trong một biến tạm chết NGAY ở đầu mọi nhánh, để không có
    /// biến tạm nào chỉ sống trên một đường (giữ `StorageLive/Dead` nhất quán ở điểm hợp).
    pub(crate) fn switch_rv(
        &mut self,
        rv: Rvalue,
        ty: Ty,
        targets: Vec<(u128, BasicBlock)>,
        otherwise: BasicBlock,
    ) -> R<()> {
        let t = self.decl_temp_unregistered(ty);
        self.push(StatementKind::StorageLive(t));
        self.assign(Place::local(t), rv);
        self.switch_local(t, targets, otherwise)
    }

    /// Rẽ nhánh theo local `t` (đã sống): `t` chết NGAY ở đầu mọi nhánh.
    pub(crate) fn switch_local(
        &mut self,
        t: Local,
        targets: Vec<(u128, BasicBlock)>,
        otherwise: BasicBlock,
    ) -> R<()> {
        let mut tramp = Vec::new();
        for (v, b) in &targets {
            let x = self.new_block();
            tramp.push((*v, x, *b));
        }
        let ow = self.new_block();
        self.terminate(TerminatorKind::SwitchInt {
            discr: Operand::Copy(Place::local(t)),
            targets: tramp.iter().map(|(v, x, _)| (*v, *x)).collect(),
            otherwise: ow,
        })?;
        for (_, x, b) in tramp {
            self.cur = x;
            self.push(StatementKind::StorageDead(t));
            self.terminate(TerminatorKind::Goto(b))?;
        }
        self.cur = ow;
        self.push(StatementKind::StorageDead(t));
        self.terminate(TerminatorKind::Goto(otherwise))?;
        Ok(())
    }

    fn decl_temp_unregistered(&mut self, ty: Ty) -> Local {
        let n = crate::types::region_slots(self.db, &ty);
        let regions = (0..n)
            .map(|_| {
                let r = RegionVid(self.body.region_count);
                self.body.region_count += 1;
                r
            })
            .collect();
        self.body.locals.push(LocalDecl {
            ty,
            regions,
            kind: LocalKind::Temp,
            mutable: true,
            span: self.at.0,
        });
        Local(u32::try_from(self.body.locals.len() - 1).unwrap_or(u32::MAX))
    }

    /// Điều kiện ghép (`∧ ∨ ⇒ ¬`) cần hạ thẳng thành nhánh.
    pub(crate) fn is_compound_cond(e: &Expr) -> bool {
        matches!(
            &e.kind,
            ExprKind::Binary {
                op: BinaryOp::And | BinaryOp::Or | BinaryOp::Implies,
                ..
            } | ExprKind::Unary {
                op: UnaryOp::Not,
                ..
            }
        )
    }

    /// Hạ điều kiện bool THẲNG thành nhánh điều khiển (lệnh Giang 2026-10-08, M5-fix): đúng → `t`, sai → `f`, KHÔNG cụ thể hoá
    /// `∧ ∨ ⇒ ¬` thành biến tạm bool rồi mới rẽ. Thứ tự đánh giá và ngắn mạch như semantics §3a: vế trái trước, vế phải chỉ khi cần.
    /// `scoped`: đang ở khối chỉ một đường tới (vế phải, hoặc đầu mỗi vòng lặp) nên biến tạm của lá phải chết TRƯỚC khi rẽ.
    pub(crate) fn cond_branch(
        &mut self,
        e: &Expr,
        t: BasicBlock,
        f: BasicBlock,
        scoped: bool,
    ) -> R<()> {
        match &e.kind {
            ExprKind::Lit(Lit {
                kind: LitKind::Bool(v),
                ..
            }) => self.with_at(e, |b| {
                b.terminate(TerminatorKind::Goto(if *v { t } else { f }))
            }),
            ExprKind::Unary {
                op: UnaryOp::Not,
                operand,
            } => self.cond_branch(operand, f, t, scoped),
            ExprKind::Binary {
                op: BinaryOp::And,
                lhs,
                rhs,
            } => {
                let mid = self.new_block();
                self.cond_branch(lhs, mid, f, scoped)?;
                self.cur = mid;
                self.cond_branch(rhs, t, f, true)
            }
            ExprKind::Binary {
                op: BinaryOp::Or,
                lhs,
                rhs,
            } => {
                let mid = self.new_block();
                self.cond_branch(lhs, t, mid, scoped)?;
                self.cur = mid;
                self.cond_branch(rhs, t, f, true)
            }
            ExprKind::Binary {
                op: BinaryOp::Implies,
                lhs,
                rhs,
            } => {
                // a ⇒ b  ≡  ¬a ∨ b
                let mid = self.new_block();
                self.cond_branch(lhs, mid, t, scoped)?;
                self.cur = mid;
                self.cond_branch(rhs, t, f, true)
            }
            _ => self.with_at(e, |b| {
                if scoped {
                    let cv = b.decl_temp_unregistered(Ty::Bool);
                    b.push(StatementKind::StorageLive(cv));
                    b.in_stmt(|b| {
                        let o = b.as_operand(e)?;
                        b.assign(Place::local(cv), Rvalue::Use(o));
                        Ok(())
                    })?;
                    b.switch_local(cv, vec![(0, f)], t)
                } else {
                    let c = b.as_operand(e)?;
                    b.terminate(TerminatorKind::SwitchInt {
                        discr: c,
                        targets: vec![(0, f)],
                        otherwise: t,
                    })
                }
            }),
        }
    }

    /// Nhảy theo bool: `false` → `f`, khác → `t`.
    fn branch_bool(&mut self, rv: Rvalue, t: BasicBlock, f: BasicBlock) -> R<()> {
        self.switch_rv(rv, Ty::Bool, vec![(0, f)], t)
    }

    fn place_ty_of(&self, p: &Place) -> R<Ty> {
        place_ty(&self.body, p).map_or_else(|| ice("kiểu của chỗ không xác định"), Ok)
    }

    fn with_at<T>(&mut self, e: &Expr, f: impl FnOnce(&mut Self) -> R<T>) -> R<T> {
        let old = self.at;
        self.at = (e.span, e.id);
        let r = f(self);
        self.at = old;
        r
    }

    // ---------------- câu lệnh ----------------

    pub(crate) fn lower_stmt(&mut self, s: &Stmt) -> R<()> {
        self.at = (s.span, s.id);
        match &s.kind {
            StmtKind::Let {
                mutable,
                name,
                init,
                ty: ty_ann,
            } => self.in_stmt(|b| {
                let Some(&id) = b.t.resolve.decls.get(&name.id) else {
                    return ice(format!("biến `{}` không có định danh", name.name));
                };
                let l = b.user_var(id, *mutable, name.span)?;
                let dest = Place::local(l);
                // luật kéo dài tạm (ADR-020 §9): `x ≔ &V` / `&!V`, V không phải chỗ ⇒ tạm sống tới cuối khối
                match &init.kind {
                    ExprKind::Unary {
                        op: op @ (UnaryOp::Borrow | UnaryOp::BorrowMut),
                        operand,
                    } if !b.is_place_expr(operand) => b.with_at(init, |b| {
                        let ty = b.ty_of(operand)?;
                        let t = b.extended_temp(ty);
                        b.expr_into(&Place::local(t), operand)?;
                        let k = if *op == UnaryOp::Borrow {
                            BorrowKind::Shared
                        } else {
                            BorrowKind::Unique
                        };
                        let rv = b.new_borrow(k, Place::local(t));
                        b.assign(dest.clone(), rv);
                        Ok(())
                    })?,
                    // reborrow ngầm (ADR-023 §0.1): `y: &!T ≔ r;` với `r` là CHỖ kiểu `&!T`
                    _ if ty_ann.is_some() && b.is_unique_ref_place(init)? => {
                        let op = b.as_coerced_operand(init)?;
                        b.assign(dest.clone(), Rvalue::Use(op));
                    }
                    _ => b.expr_into(&dest, init)?,
                }
                b.push(StatementKind::FakeRead(FakeReadCause::LetInit, dest));
                Ok(())
            }),
            StmtKind::Assign { target, value } => self.in_stmt(|b| {
                // chỗ trước (kể cả kiểm chỉ số), rồi giá trị, rồi drop giá trị cũ, rồi ghi (semantics §3a)
                let p = b.as_place(target)?;
                let rv = b.as_rvalue(value)?;
                if !Self::is_read_only(value) {
                    // giá trị có thể làm đổi gốc của chỗ đích (`s[i] ← { s ← …; x }`): kiểm biên lại theo trạng thái lúc GHI (FB-C20)
                    b.recheck_place(&p)?;
                }
                let ty = b.place_ty_of(&p)?;
                if b.needs_drop(&ty) {
                    // giữ giá trị mới trong một tạm trong lúc drop giá trị cũ
                    let t = b.temp(ty);
                    b.assign(Place::local(t), rv);
                    let next = b.new_block();
                    b.terminate(TerminatorKind::Drop {
                        place: p.clone(),
                        target: next,
                    })?;
                    b.cur = next;
                    b.assign(p, Rvalue::Use(Operand::Move(Place::local(t))));
                } else {
                    b.assign(p, rv);
                }
                Ok(())
            }),
            StmtKind::Expr { expr, .. } => self.in_stmt(|b| b.discard(expr)),
            StmtKind::Return(e) => self.in_stmt(|b| b.ret(e.as_ref())),
            StmtKind::Break => self.break_or_continue(true),
            StmtKind::Continue => self.break_or_continue(false),
        }
    }

    /// Đánh giá để lấy tác dụng phụ; giá trị nằm trong một tạm của câu lệnh.
    fn discard(&mut self, e: &Expr) -> R<()> {
        let ty = self.ty_of(e)?;
        let t = self.temp(ty);
        self.expr_into(&Place::local(t), e)
    }

    // ---------------- chỗ ----------------

    /// Biểu thức chỉ một chỗ (không tạo tạm)?
    pub(crate) fn is_place_expr(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Path(p) => matches!(self.res_of(p.id), Some(Res::Local(_) | Res::SelfValue)),
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => self.is_place_expr(base),
            ExprKind::Unary {
                op: UnaryOp::Deref, ..
            } => true,
            _ => false,
        }
    }

    pub(crate) fn as_place(&mut self, e: &Expr) -> R<Place> {
        self.with_at(e, |b| match &e.kind {
            ExprKind::Path(p) => match b.res_of(p.id) {
                Some(Res::Local(id)) => match b.vars.get(&id) {
                    Some(l) => Ok(Place::local(*l)),
                    None => ice(format!("biến {id:?} chưa được khai báo trong MIR")),
                },
                Some(Res::SelfValue) => Ok(Place::local(Local(1))),
                _ => b.temp_place(e),
            },
            ExprKind::Field { base, name } => b.field_place(base, name),
            ExprKind::Index { base, index } => b.index_place(base, index),
            ExprKind::Unary {
                op: UnaryOp::Deref,
                operand,
            } => {
                let p = if b.is_place_expr(operand) {
                    b.as_place(operand)?
                } else {
                    b.temp_place(operand)?
                };
                Ok(p.project(PlaceElem::Deref))
            }
            _ => b.temp_place(e),
        })
    }

    fn temp_place(&mut self, e: &Expr) -> R<Place> {
        let ty = self.ty_of(e)?;
        let t = self.temp(ty);
        self.expr_into(&Place::local(t), e)?;
        Ok(Place::local(t))
    }

    /// Chỗ của `base` sau khi bỏ mọi lớp tham chiếu (autoderef), cùng kiểu cuối.
    fn autoderef_place(&mut self, base: &Expr) -> R<(Place, Ty)> {
        let mut p = if self.is_place_expr(base) {
            self.as_place(base)?
        } else {
            self.temp_place(base)?
        };
        let mut ty = self.ty_of(base)?;
        while let Ty::Ref(_, inner) = ty {
            p = p.project(PlaceElem::Deref);
            ty = *inner;
        }
        Ok((p, ty))
    }

    fn field_place(&mut self, base: &Expr, name: &FieldName) -> R<Place> {
        let (p, ty) = self.autoderef_place(base)?;
        match (&ty, name) {
            (Ty::Tuple(v), FieldName::Index(i)) => {
                let k = i.value as usize;
                let Some(ft) = v.get(k) else {
                    return ice("chỉ số tuple vượt phạm vi sau M4");
                };
                Ok(p.project(PlaceElem::Field(i.value, ft.clone())))
            }
            (Ty::Adt(d, args), FieldName::Named(n)) => {
                let Some(adt) = self.db.adts.get(d) else {
                    return ice("thiếu ADT");
                };
                let AdtKind::Struct(fs) = &adt.kind else {
                    return ice("truy cập trường trên enum sau M4");
                };
                let Some(k) = fs.iter().position(|f| f.name == n.name) else {
                    return ice(format!("không có trường `{}` sau M4", n.name));
                };
                let fts = variant_fields(self.db, *d, 0, args);
                Ok(p.project(PlaceElem::Field(k as u32, fts[k].clone())))
            }
            _ => ice(format!("truy cập trường trên kiểu {ty:?}")),
        }
    }

    fn usize_temp(&mut self, op: Operand) -> Local {
        let t = self.temp(Ty::Int(IntTy::Usize));
        self.assign(Place::local(t), Rvalue::Use(op));
        t
    }

    /// Kiểm lại biên của MỌI `Index`/`Subslice` trong `p` theo trạng thái HIỆN TẠI (FB-C20): chỗ `(*s)[i]` đọc `s` ở lúc DÙNG, nên nếu giữa lúc dựng chỗ
    /// (kèm kiểm biên) và lúc dùng có biểu thức làm đổi `s` (`s ← &khác[..]`) thì kiểm biên cũ không còn nói gì về ô nhớ thật.
    /// Dùng lại các tạm chỉ số đã tính (không đánh giá lại biểu thức ⇒ không lặp tác dụng phụ).
    pub(crate) fn recheck_place(&mut self, p: &Place) -> R<()> {
        let usz = Ty::Int(IntTy::Usize);
        let mut pre = Place::local(p.local);
        for e in &p.projection {
            match e {
                PlaceElem::Index(i) => {
                    let len = self.temp(usz.clone());
                    self.assign(Place::local(len), Rvalue::Len(pre.clone()));
                    let ok = self.temp(Ty::Bool);
                    self.assign(
                        Place::local(ok),
                        Rvalue::BinaryOp(
                            BinOp::Lt,
                            Operand::Copy(Place::local(*i)),
                            Operand::Copy(Place::local(len)),
                        ),
                    );
                    self.assert(
                        Operand::Copy(Place::local(ok)),
                        true,
                        AssertKind::IndexOutOfBounds {
                            len: Operand::Copy(Place::local(len)),
                            index: Operand::Copy(Place::local(*i)),
                        },
                    )?;
                }
                PlaceElem::Subslice { from, to } => {
                    let len = self.temp(usz.clone());
                    self.assign(Place::local(len), Rvalue::Len(pre.clone()));
                    let to_op = match to {
                        Some(t) => Operand::Copy(Place::local(*t)),
                        None => Operand::Copy(Place::local(len)),
                    };
                    self.assert_range(*from, to_op, len)?;
                }
                _ => {}
            }
            pre = pre.project(e.clone());
        }
        Ok(())
    }

    /// `from ≤ to ≤ len` (abort `slice_range`)
    fn assert_range(&mut self, from: Local, to_op: Operand, len: Local) -> R<()> {
        let a = self.temp(Ty::Bool);
        self.assign(
            Place::local(a),
            Rvalue::BinaryOp(BinOp::Le, Operand::Copy(Place::local(from)), to_op.clone()),
        );
        let c = self.temp(Ty::Bool);
        self.assign(
            Place::local(c),
            Rvalue::BinaryOp(BinOp::Le, to_op.clone(), Operand::Copy(Place::local(len))),
        );
        let both = self.temp(Ty::Bool);
        self.assign(
            Place::local(both),
            Rvalue::BinaryOp(
                BinOp::BitAnd,
                Operand::Copy(Place::local(a)),
                Operand::Copy(Place::local(c)),
            ),
        );
        self.assert(
            Operand::Copy(Place::local(both)),
            true,
            AssertKind::SliceRange {
                from: Operand::Copy(Place::local(from)),
                to: to_op,
                len: Operand::Copy(Place::local(len)),
            },
        )
    }

    fn index_place(&mut self, base: &Expr, index: &Expr) -> R<Place> {
        let (p, ty) = self.autoderef_place(base)?;
        if !matches!(ty, Ty::Array(..) | Ty::Slice(_)) {
            return ice(format!("đánh chỉ số kiểu {ty:?}"));
        }
        let usz = Ty::Int(IntTy::Usize);
        let len = self.temp(usz.clone());
        if let ExprKind::Range {
            start,
            end,
            inclusive,
        } = &index.kind
        {
            // lát con: from ≤ to ≤ len. Đánh giá `from`, `to` TRƯỚC rồi mới đọc độ dài (FB-C20: `&s[0..{ s ← …; n }]`)
            let from = match start {
                Some(s) => {
                    let o = self.as_operand(s)?;
                    self.usize_temp(o)
                }
                None => self.usize_temp(Operand::Const(int_const(0, IntTy::Usize))),
            };
            let to = match end {
                Some(e) => {
                    let o = self.as_operand(e)?;
                    Some(self.usize_temp(o))
                }
                None => None,
            };
            let may_mutate = start.as_ref().is_some_and(|s| !Self::is_read_only(s))
                || end.as_ref().is_some_and(|e| !Self::is_read_only(e));
            if may_mutate {
                // các chỉ số có thể đã làm đổi gốc của `p` (kiểm biên của phần tiền tố được dựng trước đó thành cũ)
                self.recheck_place(&p)?;
            }
            self.assign(Place::local(len), Rvalue::Len(p.clone()));
            if let (true, Some(t)) = (*inclusive, to) {
                // `..=b`: b < len trước, nên b + 1 không tràn
                let ok = self.temp(Ty::Bool);
                self.assign(
                    Place::local(ok),
                    Rvalue::BinaryOp(
                        BinOp::Lt,
                        Operand::Copy(Place::local(t)),
                        Operand::Copy(Place::local(len)),
                    ),
                );
                self.assert(
                    Operand::Copy(Place::local(ok)),
                    true,
                    AssertKind::IndexOutOfBounds {
                        len: Operand::Copy(Place::local(len)),
                        index: Operand::Copy(Place::local(t)),
                    },
                )?;
                self.assign(
                    Place::local(t),
                    Rvalue::BinaryOp(
                        BinOp::AddWrap,
                        Operand::Copy(Place::local(t)),
                        Operand::Const(int_const(1, IntTy::Usize)),
                    ),
                );
            }
            let to_op = match to {
                Some(t) => Operand::Copy(Place::local(t)),
                None => Operand::Copy(Place::local(len)),
            };
            self.assert_range(from, to_op, len)?;
            return Ok(p.project(PlaceElem::Subslice { from, to }));
        }
        let o = self.as_operand(index)?;
        let i = self.usize_temp(o);
        if !Self::is_read_only(index) {
            self.recheck_place(&p)?;
        }
        self.assign(Place::local(len), Rvalue::Len(p.clone()));
        let ok = self.temp(Ty::Bool);
        self.assign(
            Place::local(ok),
            Rvalue::BinaryOp(
                BinOp::Lt,
                Operand::Copy(Place::local(i)),
                Operand::Copy(Place::local(len)),
            ),
        );
        self.assert(
            Operand::Copy(Place::local(ok)),
            true,
            AssertKind::IndexOutOfBounds {
                len: Operand::Copy(Place::local(len)),
                index: Operand::Copy(Place::local(i)),
            },
        )?;
        Ok(p.project(PlaceElem::Index(i)))
    }

    // ---------------- toán hạng / rvalue ----------------

    pub(crate) fn lit_const(&self, l: &Lit, ty: &Ty) -> R<Const> {
        Ok(match (&l.kind, ty) {
            (LitKind::Int(s), Ty::Int(it)) => {
                let Some((v, _)) = gt_types::int_literal_value(s) else {
                    return ice("literal nguyên hỏng sau M4");
                };
                let bits = if l.negative { 0u128.wrapping_sub(v) } else { v };
                // ngoài phạm vi kiểu: ghi lại (E0601 ở M7) trước khi cắt bit làm mất thông tin
                let half = if it.bits() == 128 {
                    1u128 << 127
                } else {
                    1u128 << (it.bits() - 1)
                };
                let fits = match (it.signed(), l.negative) {
                    (true, true) => v <= half,
                    (true, false) => v < half,
                    (false, true) => v == 0,
                    (false, false) => it.bits() == 128 || v <= mask(*it),
                };
                if !fits {
                    self.bad_lits.borrow_mut().push(BadLiteral {
                        node: l.id,
                        span: l.span,
                        ty: *it,
                        negative: l.negative,
                        magnitude: v,
                    });
                }
                Const::Int {
                    bits: bits & mask(*it),
                    ty: *it,
                }
            }
            (LitKind::Float(s), Ty::Float(ft)) => Const::Float {
                text: if l.negative {
                    format!("-{s}")
                } else {
                    s.clone()
                },
                ty: *ft,
            },
            (LitKind::Bool(b), _) => Const::Bool(*b),
            (LitKind::Char(c), _) => match gt_types::char_literal_value(c) {
                Some(cp) => Const::Char(cp),
                None => return ice("literal ký tự hỏng"),
            },
            (LitKind::Str(s), _) => match decode_str(s) {
                Some(x) => Const::Str(x),
                None => return ice("literal chuỗi hỏng"),
            },
            _ => return ice(format!("literal với kiểu {ty:?}")),
        })
    }

    /// `e` là một CHỖ có kiểu `&!T`?
    pub(crate) fn is_unique_ref_place(&self, e: &Expr) -> R<bool> {
        Ok(self.is_place_expr(e) && matches!(self.ty_of(e)?, Ty::Ref(true, _)))
    }

    /// Biểu thức không thể ghi/move/mượn gì (chỉ đọc, tối đa abort): đứng sau một toán hạng chỗ thì không cần chụp lại.
    fn is_read_only(e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Lit(_) | ExprKind::Path(_) => true,
            ExprKind::Unary { op, operand } => {
                !matches!(op, UnaryOp::Borrow | UnaryOp::BorrowMut) && Self::is_read_only(operand)
            }
            ExprKind::Binary { lhs, rhs, .. } => Self::is_read_only(lhs) && Self::is_read_only(rhs),
            ExprKind::Cast { expr, .. } => Self::is_read_only(expr),
            ExprKind::Field { base, .. } => Self::is_read_only(base),
            ExprKind::Index { base, index } => {
                Self::is_read_only(base) && Self::is_read_only(index)
            }
            ExprKind::Tuple(v) | ExprKind::Array(v) => v.iter().all(Self::is_read_only),
            ExprKind::ArrayRepeat { value, .. } => Self::is_read_only(value),
            ExprKind::StructLit { fields, .. } => {
                fields.iter().all(|f| Self::is_read_only(&f.value))
            }
            _ => false,
        }
    }

    /// Thứ tự đánh giá trái sang phải (semantics §3a): toán hạng là CHỖ (biến/trường/`*r`) bị đọc/move ở lúc dùng,
    /// nên nếu một biểu thức đứng sau có thể ghi/move/mượn thì phải chụp giá trị vào biến tạm NGAY bây giờ.
    fn stabilize(&mut self, op: Operand, ty: &Ty, later: &[&Expr]) -> Operand {
        if later.iter().all(|l| Self::is_read_only(l)) {
            return op;
        }
        let p = match &op {
            Operand::Copy(p) | Operand::Move(p) => p.clone(),
            Operand::Const(_) => return op,
        };
        let decl = &self.body.locals[p.local.index()];
        let is_temp = p.projection.is_empty() && matches!(decl.kind, LocalKind::Temp);
        // chỗ chép (Copy) của biến bất biến, không qua tham chiếu: không ai ghi được
        let frozen = matches!(op, Operand::Copy(_)) && p.projection.is_empty() && !decl.mutable;
        if is_temp || frozen {
            return op;
        }
        let t = self.temp(ty.clone());
        self.assign(Place::local(t), Rvalue::Use(op));
        self.operand_of(Place::local(t), ty)
    }

    /// Các toán hạng theo thứ tự trái sang phải, mỗi cái được chụp lại nếu biểu thức sau nó có thể làm đổi nó.
    pub(crate) fn seq_operands(&mut self, es: &[&Expr], coerced: bool) -> R<Vec<Operand>> {
        let mut out = Vec::with_capacity(es.len());
        for (i, e) in es.iter().enumerate() {
            let o = if coerced {
                self.as_coerced_operand(e)?
            } else {
                self.as_operand(e)?
            };
            let ty = self.ty_of(e)?;
            out.push(self.stabilize(o, &ty, &es[i + 1..]));
        }
        Ok(out)
    }

    /// Toán hạng tại điểm ÉP KIỂU (đối số, `≔` có chú thích, `⏎`): một chỗ kiểu `&!T` được reborrow ngầm
    /// (`&!*r`) thay vì move, như Rust — nếu không, `f(r); f(r)` là dùng-sau-move (ADR-023 §0.1, Q30).
    pub(crate) fn as_coerced_operand(&mut self, e: &Expr) -> R<Operand> {
        if self.is_unique_ref_place(e)? {
            return self.with_at(e, |b| {
                let ty = b.ty_of(e)?;
                let p = b.as_place(e)?;
                let t = b.temp(ty.clone());
                let rv = b.new_borrow(BorrowKind::Unique, p.project(PlaceElem::Deref));
                b.assign(Place::local(t), rv);
                Ok(b.operand_of(Place::local(t), &ty))
            });
        }
        self.as_operand(e)
    }

    pub(crate) fn as_operand(&mut self, e: &Expr) -> R<Operand> {
        self.with_at(e, |b| {
            let ty = b.ty_of(e)?;
            match &e.kind {
                ExprKind::Lit(l) => return Ok(Operand::Const(b.lit_const(l, &ty)?)),
                ExprKind::Path(p) => match b.res_of(p.id) {
                    Some(Res::Local(_) | Res::SelfValue) => {
                        let pl = b.as_place(e)?;
                        return Ok(b.operand_of(pl, &ty));
                    }
                    Some(Res::Def(d)) => {
                        if let Some(c) = b.def_const(e, d, &ty)? {
                            return Ok(Operand::Const(c));
                        }
                    }
                    // `Kiểu::hàm` (mục kết hợp) dùng làm giá trị: chỗ gọi do M4 ghi quyết định đích (FB-13)
                    Some(Res::Assoc { .. }) => return Ok(Operand::Const(b.fn_item_const(e, &ty)?)),
                    _ => {}
                },
                _ if b.is_place_expr(e) => {
                    let pl = b.as_place(e)?;
                    return Ok(b.operand_of(pl, &ty));
                }
                _ => {}
            }
            // `as_rvalue` của một đường dẫn lại gọi `as_operand`: đường dẫn KHÔNG hạ được thành toán hạng ở trên mà rơi xuống đây sẽ đệ quy vô hạn (FB-13) ⇒ ICE
            if let ExprKind::Path(p) = &e.kind
                && !matches!(b.res_of(p.id), Some(Res::Def(d)) if b.t.defs()[d.index()].kind == DefKind::Variant)
            {
                return ice("đường dẫn không hạ được thành toán hạng (không phải biến, hàm, hằng, biến thể)");
            }
            let t = b.temp(ty.clone());
            b.expr_into(&Place::local(t), e)?;
            Ok(b.operand_of(Place::local(t), &ty))
        })
    }

    /// `Const::FnItem` của hàm mà chỗ gọi `e` đã phân giải (M4 ghi `sites`). Hàm qua trait dùng như giá trị chưa hỗ trợ ⇒ ICE.
    fn fn_item_const(&self, e: &Expr, ty: &Ty) -> R<Const> {
        let Some(site) = self.t.sites.get(&e.id) else {
            return ice("thiếu chỗ gọi của hàm");
        };
        let CallTarget::Fn(node) = site.target else {
            return ice("hàm qua trait dùng như giá trị");
        };
        Ok(Const::FnItem {
            node,
            generics: site.generics.clone(),
            ty: ty.clone(),
        })
    }

    /// Hằng của một định nghĩa dùng như giá trị: hàm, `□`/`■`. `None` nếu không phải hằng (vd biến thể).
    fn def_const(&self, e: &Expr, d: gt_resolve::DefId, ty: &Ty) -> R<Option<Const>> {
        let def = &self.t.defs()[d.index()];
        Ok(match def.kind {
            DefKind::Fn => Some(self.fn_item_const(e, ty)?),
            DefKind::Const | DefKind::Static => Some(Const::Item {
                node: def.node,
                ty: ty.clone(),
            }),
            _ => None,
        })
    }

    /// (enum, chỉ số biến thể) của một định nghĩa biến thể.
    pub(crate) fn variant_of(&self, vd: gt_resolve::DefId) -> R<(gt_resolve::DefId, u32)> {
        let Some(parent) = self.t.defs()[vd.index()].parent else {
            return ice("biến thể không có enum cha");
        };
        let Some(adt) = self.db.adts.get(&parent) else {
            return ice("thiếu enum");
        };
        let AdtKind::Enum(vs) = &adt.kind else {
            return ice("biến thể của kiểu không phải enum");
        };
        match vs.iter().position(|v| v.def == vd) {
            Some(i) => Ok((parent, i as u32)),
            None => ice("không tìm thấy biến thể"),
        }
    }

    fn adt_args(ty: &Ty) -> Vec<Ty> {
        match ty {
            Ty::Adt(_, a) => a.clone(),
            _ => Vec::new(),
        }
    }

    pub(crate) fn as_rvalue(&mut self, e: &Expr) -> R<Rvalue> {
        self.with_at(e, |b| {
            let ty = b.ty_of(e)?;
            match &e.kind {
                ExprKind::Lit(_) => Ok(Rvalue::Use(b.as_operand(e)?)),
                ExprKind::Path(p) => match b.res_of(p.id) {
                    Some(Res::Def(d)) if b.t.defs()[d.index()].kind == DefKind::Variant => {
                        let (_, vi) = b.variant_of(d)?;
                        let Ty::Adt(ed, args) = &ty else {
                            return ice("biến thể dùng như hàm (chưa hỗ trợ ở M5)");
                        };
                        Ok(Rvalue::Aggregate(
                            AggregateKind::Adt(*ed, vi, args.clone()),
                            Vec::new(),
                        ))
                    }
                    _ => Ok(Rvalue::Use(b.as_operand(e)?)),
                },
                ExprKind::Field { .. } | ExprKind::Index { .. } => {
                    let p = b.as_place(e)?;
                    Ok(Rvalue::Use(b.operand_of(p, &ty)))
                }
                ExprKind::Unary { op, operand } => b.unary(*op, operand, &ty, e),
                ExprKind::Binary { op, lhs, rhs } => b.binary(*op, lhs, rhs, &ty, e),
                ExprKind::Cast { expr, .. } => {
                    let from = b.ty_of(expr)?;
                    let a = b.as_operand(expr)?;
                    if cast_may_lose(&from, &ty) {
                        let f = b.temp(Ty::Bool);
                        b.assign(Place::local(f), Rvalue::CastFits(a.clone(), ty.clone()));
                        b.assert(
                            Operand::Copy(Place::local(f)),
                            true,
                            AssertKind::CastLossy {
                                value: a.clone(),
                                to: ty.clone(),
                            },
                        )?;
                    }
                    Ok(Rvalue::Cast(a, ty))
                }
                ExprKind::Tuple(v) => {
                    let es: Vec<&Expr> = v.iter().collect();
                    let ops = b.seq_operands(&es, false)?;
                    Ok(Rvalue::Aggregate(AggregateKind::Tuple, ops))
                }
                ExprKind::Array(v) => {
                    let Ty::Array(el, _) = &ty else {
                        return ice("literal mảng không có kiểu mảng");
                    };
                    let es: Vec<&Expr> = v.iter().collect();
                    let ops = b.seq_operands(&es, false)?;
                    Ok(Rvalue::Aggregate(AggregateKind::Array((**el).clone()), ops))
                }
                ExprKind::ArrayRepeat { value, .. } => {
                    let Ty::Array(_, n) = &ty else {
                        return ice("mảng lặp không có kiểu mảng");
                    };
                    let o = b.as_operand(value)?;
                    Ok(Rvalue::Repeat(o, *n))
                }
                ExprKind::StructLit { fields, .. } => {
                    let Ty::Adt(d, args) = &ty else {
                        return ice("struct literal không có kiểu ADT");
                    };
                    let Some(AdtKind::Struct(defs)) = b.db.adts.get(d).map(|a| a.kind.clone())
                    else {
                        return ice("struct literal của kiểu không phải struct");
                    };
                    // đánh giá theo thứ tự VIẾT, rồi xếp theo thứ tự khai báo
                    let es: Vec<&Expr> = fields.iter().map(|f| &f.value).collect();
                    let os = b.seq_operands(&es, false)?;
                    let got: Vec<(String, Operand)> =
                        fields.iter().map(|f| f.name.name.clone()).zip(os).collect();
                    let mut got = got;
                    let mut ops = Vec::new();
                    for df in &defs {
                        match got.iter().position(|(n, _)| *n == df.name) {
                            Some(i) => ops.push(got.remove(i).1),
                            None => return ice(format!("thiếu trường `{}` sau M4", df.name)),
                        }
                    }
                    Ok(Rvalue::Aggregate(
                        AggregateKind::Adt(*d, 0, args.clone()),
                        ops,
                    ))
                }
                ExprKind::Range {
                    start: Some(s),
                    end: Some(en),
                    ..
                } => {
                    let Ty::Adt(d, args) = &ty else {
                        return ice("range không có kiểu Range");
                    };
                    let mut ac = b.seq_operands(&[&**s, &**en], false)?;
                    let c = ac.remove(1);
                    let a = ac.remove(0);
                    let order = b.range_field_order(*d)?;
                    let ops = if order { vec![a, c] } else { vec![c, a] };
                    Ok(Rvalue::Aggregate(
                        AggregateKind::Adt(*d, 0, args.clone()),
                        ops,
                    ))
                }
                ExprKind::Call { callee, args } => {
                    if let ExprKind::Path(p) = &callee.kind
                        && let Some(Res::Def(d)) = b.res_of(p.id)
                        && b.t.defs()[d.index()].kind == DefKind::Variant
                    {
                        let (ed, vi) = b.variant_of(d)?;
                        let es: Vec<&Expr> = args.iter().collect();
                        let ops = b.seq_operands(&es, false)?;
                        return Ok(Rvalue::Aggregate(
                            AggregateKind::Adt(ed, vi, Self::adt_args(&ty)),
                            ops,
                        ));
                    }
                    b.via_temp(e, &ty)
                }
                _ => b.via_temp(e, &ty),
            }
        })
    }

    /// Trường `start` có đứng trước `end` trong khai báo của Range không (để dựng Aggregate đúng thứ tự).
    fn range_field_order(&self, d: gt_resolve::DefId) -> R<bool> {
        match self.db.adts.get(&d).map(|a| &a.kind) {
            Some(AdtKind::Struct(fs)) => {
                let s = fs.iter().position(|f| f.name == "start");
                let e = fs.iter().position(|f| f.name == "end");
                match (s, e) {
                    (Some(s), Some(e)) if fs.len() == 2 => Ok(s < e),
                    _ => ice("lang item range cần đúng hai trường `start`, `end`"),
                }
            }
            _ => ice("lang item range phải là struct"),
        }
    }

    fn via_temp(&mut self, e: &Expr, ty: &Ty) -> R<Rvalue> {
        let t = self.temp(ty.clone());
        self.expr_into(&Place::local(t), e)?;
        Ok(Rvalue::Use(self.operand_of(Place::local(t), ty)))
    }

    fn unary(&mut self, op: UnaryOp, operand: &Expr, ty: &Ty, e: &Expr) -> R<Rvalue> {
        match op {
            UnaryOp::Not => Ok(Rvalue::UnaryOp(UnOp::Not, self.as_operand(operand)?)),
            UnaryOp::Neg => {
                let a = self.as_operand(operand)?;
                if let Ty::Int(it) = ty
                    && it.signed()
                {
                    let z = self.temp(Ty::Bool);
                    self.assign(
                        Place::local(z),
                        Rvalue::BinaryOp(BinOp::Eq, a.clone(), Operand::Const(min_const(*it))),
                    );
                    self.assert(
                        Operand::Copy(Place::local(z)),
                        false,
                        AssertKind::OverflowNeg(a.clone()),
                    )?;
                }
                Ok(Rvalue::UnaryOp(UnOp::Neg, a))
            }
            UnaryOp::Deref => {
                let p = self.as_place(e)?;
                Ok(Rvalue::Use(self.operand_of(p, ty)))
            }
            UnaryOp::Borrow | UnaryOp::BorrowMut => {
                let p = if self.is_place_expr(operand) {
                    self.as_place(operand)?
                } else {
                    self.temp_place(operand)?
                };
                let k = if op == UnaryOp::Borrow {
                    BorrowKind::Shared
                } else {
                    BorrowKind::Unique
                };
                Ok(self.new_borrow(k, p))
            }
        }
    }

    fn binary(&mut self, op: BinaryOp, lhs: &Expr, rhs: &Expr, ty: &Ty, e: &Expr) -> R<Rvalue> {
        use BinaryOp as B;
        let cmp = |o: B| match o {
            B::Eq => Some(BinOp::Eq),
            B::Ne => Some(BinOp::Ne),
            B::Lt => Some(BinOp::Lt),
            B::Le => Some(BinOp::Le),
            B::Gt => Some(BinOp::Gt),
            B::Ge => Some(BinOp::Ge),
            _ => None,
        };
        if let Some(c) = cmp(op) {
            let mut ab = self.seq_operands(&[lhs, rhs], false)?;
            let b2 = ab.remove(1);
            let a = ab.remove(0);
            return Ok(Rvalue::BinaryOp(c, a, b2));
        }
        match op {
            B::And | B::Or | B::Implies | B::AddChecked | B::SubChecked | B::MulChecked => {
                self.via_temp(e, ty)
            }
            B::InSet => self.in_set(lhs, rhs),
            B::AddWrap | B::SubWrap | B::MulWrap | B::AddSat | B::SubSat | B::MulSat => {
                let bo = match op {
                    B::AddWrap => BinOp::AddWrap,
                    B::SubWrap => BinOp::SubWrap,
                    B::MulWrap => BinOp::MulWrap,
                    B::AddSat => BinOp::AddSat,
                    B::SubSat => BinOp::SubSat,
                    _ => BinOp::MulSat,
                };
                let mut ac = self.seq_operands(&[lhs, rhs], false)?;
                let c = ac.remove(1);
                let a = ac.remove(0);
                Ok(Rvalue::BinaryOp(bo, a, c))
            }
            B::Add | B::Sub | B::Mul | B::Div | B::Rem => {
                let bo = match op {
                    B::Add => BinOp::Add,
                    B::Sub => BinOp::Sub,
                    B::Mul => BinOp::Mul,
                    B::Div => BinOp::Div,
                    _ => BinOp::Rem,
                };
                let mut ac = self.seq_operands(&[lhs, rhs], false)?;
                let c = ac.remove(1);
                let a = ac.remove(0);
                let Ty::Int(it) = ty else {
                    // số thực: không có nghĩa vụ
                    return Ok(Rvalue::BinaryOp(bo, a, c));
                };
                match bo {
                    BinOp::Add | BinOp::Sub | BinOp::Mul => {
                        let t = self.temp(Ty::Tuple(vec![ty.clone(), Ty::Bool]));
                        self.assign(
                            Place::local(t),
                            Rvalue::CheckedBinaryOp(bo, a.clone(), c.clone()),
                        );
                        self.assert(
                            Operand::Copy(Place::local(t).project(PlaceElem::Field(1, Ty::Bool))),
                            false,
                            AssertKind::Overflow(bo, a, c),
                        )?;
                        Ok(Rvalue::Use(Operand::Copy(
                            Place::local(t).project(PlaceElem::Field(0, ty.clone())),
                        )))
                    }
                    _ => {
                        let z = self.temp(Ty::Bool);
                        self.assign(
                            Place::local(z),
                            Rvalue::BinaryOp(
                                BinOp::Ne,
                                c.clone(),
                                Operand::Const(int_const(0, *it)),
                            ),
                        );
                        let k = if bo == BinOp::Div {
                            AssertKind::DivisionByZero(a.clone())
                        } else {
                            AssertKind::RemainderByZero(a.clone())
                        };
                        self.assert(Operand::Copy(Place::local(z)), true, k)?;
                        if it.signed() {
                            // MIN / -1 tràn
                            let m1 = self.temp(Ty::Bool);
                            self.assign(
                                Place::local(m1),
                                Rvalue::BinaryOp(
                                    BinOp::Eq,
                                    a.clone(),
                                    Operand::Const(min_const(*it)),
                                ),
                            );
                            let m2 = self.temp(Ty::Bool);
                            self.assign(
                                Place::local(m2),
                                Rvalue::BinaryOp(
                                    BinOp::Eq,
                                    c.clone(),
                                    Operand::Const(int_const(-1, *it)),
                                ),
                            );
                            let both = self.temp(Ty::Bool);
                            self.assign(
                                Place::local(both),
                                Rvalue::BinaryOp(
                                    BinOp::BitAnd,
                                    Operand::Copy(Place::local(m1)),
                                    Operand::Copy(Place::local(m2)),
                                ),
                            );
                            self.assert(
                                Operand::Copy(Place::local(both)),
                                false,
                                AssertKind::Overflow(bo, a.clone(), c.clone()),
                            )?;
                        }
                        Ok(Rvalue::BinaryOp(bo, a, c))
                    }
                }
            }
            _ => ice("toán tử nhị phân lạ"),
        }
    }

    /// `x ∈ r`: `r.start ≤ x` và `x < r.end` (hoặc `≤` với `a..=b`).
    fn in_set(&mut self, lhs: &Expr, rhs: &Expr) -> R<Rvalue> {
        let (lo, hi, incl, x) = match &rhs.kind {
            ExprKind::Range {
                start: Some(s),
                end: Some(e),
                inclusive,
            } => {
                let mut v = self.seq_operands(&[lhs, &**s, &**e], false)?;
                let hi = v.remove(2);
                let lo = v.remove(1);
                (lo, hi, *inclusive, v.remove(0))
            }
            _ => {
                let x = {
                    let o = self.as_operand(lhs)?;
                    let lty = self.ty_of(lhs)?;
                    self.stabilize(o, &lty, &[rhs])
                };
                let rty = self.ty_of(rhs)?;
                let Ty::Adt(d, args) = &rty else {
                    return ice("∈ trên kiểu không phải range");
                };
                let el = args.first().cloned().unwrap_or(Ty::Err);
                let incl = self.db.lang.get(&Lang::RangeInclusive) == Some(d);
                let order = self.range_field_order(*d)?;
                let (si, ei) = if order { (0, 1) } else { (1, 0) };
                let p = if self.is_place_expr(rhs) {
                    self.as_place(rhs)?
                } else {
                    self.temp_place(rhs)?
                };
                (
                    Operand::Copy(p.project(PlaceElem::Field(si, el.clone()))),
                    Operand::Copy(p.project(PlaceElem::Field(ei, el))),
                    incl,
                    x,
                )
            }
        };
        let c1 = self.temp(Ty::Bool);
        self.assign(Place::local(c1), Rvalue::BinaryOp(BinOp::Ge, x.clone(), lo));
        let c2 = self.temp(Ty::Bool);
        let op = if incl { BinOp::Le } else { BinOp::Lt };
        self.assign(Place::local(c2), Rvalue::BinaryOp(op, x, hi));
        Ok(Rvalue::BinaryOp(
            BinOp::BitAnd,
            Operand::Copy(Place::local(c1)),
            Operand::Copy(Place::local(c2)),
        ))
    }

    // ---------------- ghi vào đích ----------------

    pub(crate) fn expr_into(&mut self, dest: &Place, e: &Expr) -> R<()> {
        self.with_at(e, |b| match &e.kind {
            ExprKind::Block(blk) => b.lower_block(dest, blk),
            ExprKind::If {
                cond,
                then_block,
                else_branch,
            } => b.if_into(dest, cond, then_block, else_branch.as_deref()),
            ExprKind::Match { scrutinee, arms } => b.match_into(dest, scrutinee, arms),
            ExprKind::Loop(l) => {
                b.loop_into(l)?;
                b.assign_unit_if_unit(dest)
            }
            ExprKind::Unsafe { proofs, body } => {
                for p in proofs {
                    b.body.contracts.push(p.id);
                }
                let old = b.enter_unsafe();
                let r = b.lower_block(dest, body);
                b.leave_scope(old);
                r
            }
            ExprKind::Call { callee, args } => b.call_into(dest, e, callee, args),
            ExprKind::Binary {
                op: op @ (BinaryOp::And | BinaryOp::Or | BinaryOp::Implies),
                lhs,
                rhs,
            } => b.short_circuit(dest, *op, lhs, rhs),
            ExprKind::Binary {
                op: op @ (BinaryOp::AddChecked | BinaryOp::SubChecked | BinaryOp::MulChecked),
                lhs,
                rhs,
            } => b.checked_option(dest, *op, lhs, rhs, e),
            ExprKind::Quantifier {
                kind,
                var,
                domain,
                body,
            } => b.quantifier_into(dest, *kind, var, domain, body),
            _ => {
                let rv = b.as_rvalue(e)?;
                b.assign(dest.clone(), rv);
                Ok(())
            }
        })
    }

    pub(crate) fn assign_unit_if_unit(&mut self, dest: &Place) -> R<()> {
        if self.place_ty_of(dest)?.is_unit() {
            self.assign(dest.clone(), Rvalue::Use(Operand::Const(Const::Unit)));
        }
        Ok(())
    }

    fn if_into(&mut self, dest: &Place, cond: &Expr, then_b: &Block, els: Option<&Expr>) -> R<()> {
        // gấp hằng nghĩa đen (ADR-020 §4b): KHÔNG const-prop
        if let ExprKind::Lit(Lit {
            kind: LitKind::Bool(v),
            ..
        }) = &cond.kind
        {
            return if *v {
                self.lower_block(dest, then_b)
            } else {
                match els {
                    Some(x) => self.in_stmt(|b| b.expr_into(dest, x)),
                    None => self.assign_unit_if_unit(dest),
                }
            };
        }
        if Self::is_compound_cond(cond) {
            let then_bb = self.new_block();
            let else_bb = self.new_block();
            let join = self.new_block();
            self.cond_branch(cond, then_bb, else_bb, false)?;
            return self.if_arms(dest, then_b, els, then_bb, else_bb, join);
        }
        let c = self.as_operand(cond)?;
        let then_bb = self.new_block();
        let else_bb = self.new_block();
        let join = self.new_block();
        self.terminate(TerminatorKind::SwitchInt {
            discr: c,
            targets: vec![(0, else_bb)],
            otherwise: then_bb,
        })?;
        self.if_arms(dest, then_b, els, then_bb, else_bb, join)
    }

    fn if_arms(
        &mut self,
        dest: &Place,
        then_b: &Block,
        els: Option<&Expr>,
        then_bb: BasicBlock,
        else_bb: BasicBlock,
        join: BasicBlock,
    ) -> R<()> {
        self.cur = then_bb;
        self.lower_block(dest, then_b)?;
        self.terminate(TerminatorKind::Goto(join))?;
        self.cur = else_bb;
        match els {
            Some(x) => self.in_stmt(|b| b.expr_into(dest, x))?,
            None => self.assign_unit_if_unit(dest)?,
        }
        self.terminate(TerminatorKind::Goto(join))?;
        self.cur = join;
        Ok(())
    }

    fn short_circuit(&mut self, dest: &Place, op: BinaryOp, lhs: &Expr, rhs: &Expr) -> R<()> {
        let a = self.as_operand(lhs)?;
        let rhs_bb = self.new_block();
        let short_bb = self.new_block();
        let join = self.new_block();
        // And: a sai ⇒ false; Or: a đúng ⇒ true; Implies: a sai ⇒ true
        let (targets, otherwise, short_val) = match op {
            BinaryOp::And => (vec![(0, short_bb)], rhs_bb, false),
            BinaryOp::Or => (vec![(0, rhs_bb)], short_bb, true),
            _ => (vec![(0, short_bb)], rhs_bb, true),
        };
        self.terminate(TerminatorKind::SwitchInt {
            discr: a,
            targets,
            otherwise,
        })?;
        self.cur = rhs_bb;
        self.in_stmt(|b| {
            let r = b.as_operand(rhs)?;
            b.assign(dest.clone(), Rvalue::Use(r));
            Ok(())
        })?;
        self.terminate(TerminatorKind::Goto(join))?;
        self.cur = short_bb;
        self.assign(
            dest.clone(),
            Rvalue::Use(Operand::Const(Const::Bool(short_val))),
        );
        self.terminate(TerminatorKind::Goto(join))?;
        self.cur = join;
        Ok(())
    }

    /// `a +? b`: `None` nếu tràn, ngược lại `Some(a + b)` (không có `Assert`).
    fn checked_option(
        &mut self,
        dest: &Place,
        op: BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        e: &Expr,
    ) -> R<()> {
        let ty = self.ty_of(e)?;
        let Ty::Adt(opt, args) = &ty else {
            return ice("`+?` không trả Option");
        };
        let el = args.first().cloned().unwrap_or(Ty::Err);
        let bo = match op {
            BinaryOp::AddChecked => BinOp::Add,
            BinaryOp::SubChecked => BinOp::Sub,
            _ => BinOp::Mul,
        };
        let (none_i, some_i) = self.option_variants(*opt)?;
        let mut ac = self.seq_operands(&[lhs, rhs], false)?;
        let c = ac.remove(1);
        let a = ac.remove(0);
        let t = self.temp(Ty::Tuple(vec![el.clone(), Ty::Bool]));
        self.assign(Place::local(t), Rvalue::CheckedBinaryOp(bo, a, c));
        let ov = self.new_block();
        let ok = self.new_block();
        let join = self.new_block();
        self.terminate(TerminatorKind::SwitchInt {
            discr: Operand::Copy(Place::local(t).project(PlaceElem::Field(1, Ty::Bool))),
            targets: vec![(0, ok)],
            otherwise: ov,
        })?;
        self.cur = ov;
        self.assign(
            dest.clone(),
            Rvalue::Aggregate(AggregateKind::Adt(*opt, none_i, args.clone()), Vec::new()),
        );
        self.terminate(TerminatorKind::Goto(join))?;
        self.cur = ok;
        self.assign(
            dest.clone(),
            Rvalue::Aggregate(
                AggregateKind::Adt(*opt, some_i, args.clone()),
                vec![Operand::Copy(
                    Place::local(t).project(PlaceElem::Field(0, el)),
                )],
            ),
        );
        self.terminate(TerminatorKind::Goto(join))?;
        self.cur = join;
        Ok(())
    }

    fn option_variants(&self, d: gt_resolve::DefId) -> R<(u32, u32)> {
        match self.db.adts.get(&d).map(|a| &a.kind) {
            Some(AdtKind::Enum(vs)) => {
                let n = vs.iter().position(|v| v.name == "None");
                let s = vs.iter().position(|v| v.name == "Some");
                match (n, s) {
                    (Some(n), Some(s)) => Ok((n as u32, s as u32)),
                    _ => ice("lang item option cần biến thể `None` và `Some`"),
                }
            }
            _ => ice("lang item option phải là enum"),
        }
    }

    // ---------------- lời gọi ----------------

    fn call_into(&mut self, dest: &Place, e: &Expr, callee: &Expr, args: &[Expr]) -> R<()> {
        let ret_ty = self.ty_of(e)?;
        // dạng phương thức `x.m(…)`
        if let ExprKind::Field { base, .. } = &callee.kind
            && let Some(site) = self.t.sites.get(&e.id).cloned()
            && site.method_syntax
        {
            let mut recv = if self.is_place_expr(base) {
                self.as_place(base)?
            } else {
                self.temp_place(base)?
            };
            for _ in 0..site.derefs {
                recv = recv.project(PlaceElem::Deref);
            }
            // đối số trước, người nhận SAU: khoản mượn (`&self`/`&!self`, autoref muộn, Q28) và cả người nhận theo giá trị
            // (ADR-022 §5: `x.consume(x.len())` hợp lệ — đối số đánh giá trước khi người nhận bị tiêu thụ)
            let es: Vec<&Expr> = args.iter().collect();
            let ops = self.seq_operands(&es, true)?;
            if !es.iter().all(|a| Self::is_read_only(a)) {
                // đối số có thể đã đổi gốc của người nhận (FB-C20): kiểm biên lại ngay trước khi mượn/dùng người nhận
                self.recheck_place(&recv)?;
            }
            let mut spans = vec![base.span];
            spans.extend(args.iter().map(|a| a.span));
            let rty = self.place_ty_of(&recv)?;
            let recv_op = match site.self_kind {
                Some(gt_ast::SelfKind::Ref) | Some(gt_ast::SelfKind::RefMut) => {
                    let k = if site.self_kind == Some(gt_ast::SelfKind::RefMut) {
                        BorrowKind::Unique
                    } else {
                        BorrowKind::Shared
                    };
                    let rt = Ty::Ref(k == BorrowKind::Unique, Box::new(rty));
                    let t = self.temp(rt.clone());
                    let rv = self.new_borrow(k, recv);
                    self.assign(Place::local(t), rv);
                    self.operand_of(Place::local(t), &rt)
                }
                _ => self.operand_of(recv, &rty),
            };
            let mut ops = ops;
            ops.insert(0, recv_op);
            let c = Self::callee_of(&site);
            return self.emit_call(dest, c, ops, spans, &ret_ty);
        }
        // biến thể có trường: dựng giá trị
        if let ExprKind::Path(p) = &callee.kind
            && let Some(Res::Def(d)) = self.res_of(p.id)
            && self.t.defs()[d.index()].kind == DefKind::Variant
        {
            let rv = self.as_rvalue(e)?;
            self.assign(dest.clone(), rv);
            return Ok(());
        }
        // `core::fuel_left(n)` (lang của prelude): kiểm `n > 0` như một `Assert` loại Fuel (abort `fuel_exhausted`; M7 có thể xoá nếu chứng minh được)
        if let ExprKind::Path(p) = &callee.kind
            && let Some(Res::Def(d)) = self.res_of(p.id)
            && self.t.defs()[d.index()].path == "$prelude::core::fuel_left"
            && args.len() == 1
        {
            let es: Vec<&Expr> = args.iter().collect();
            let ops = self.seq_operands(&es, true)?;
            let t = self.temp(Ty::Bool);
            self.assign(
                Place::local(t),
                Rvalue::BinaryOp(
                    BinOp::Gt,
                    ops[0].clone(),
                    Operand::Const(Const::Int {
                        bits: 0,
                        ty: gt_types::IntTy::U64,
                    }),
                ),
            );
            self.assert(Operand::Copy(Place::local(t)), true, AssertKind::Fuel)?;
            self.assign_unit_if_unit(dest)?;
            return Ok(());
        }
        // hàm/hàm kết hợp gọi trực tiếp, hoặc giá trị kiểu hàm
        let c = match (&callee.kind, self.t.sites.get(&callee.id).cloned()) {
            (ExprKind::Path(_), Some(site)) => Self::callee_of(&site),
            _ => {
                let o = self.as_operand(callee)?;
                let cty = self.ty_of(callee)?;
                let es: Vec<&Expr> = args.iter().collect();
                Callee::Value(self.stabilize(o, &cty, &es))
            }
        };
        let es: Vec<&Expr> = args.iter().collect();
        let ops = self.seq_operands(&es, true)?;
        let spans = args.iter().map(|a| a.span).collect();
        self.emit_call(dest, c, ops, spans, &ret_ty)
    }

    fn callee_of(site: &gt_types::CallSite) -> Callee {
        match &site.target {
            CallTarget::Fn(n) => Callee::Fn {
                node: *n,
                generics: site.generics.clone(),
            },
            CallTarget::TraitMethod {
                trait_def,
                member,
                self_ty,
            } => Callee::TraitMethod {
                trait_def: *trait_def,
                member: *member,
                self_ty: self_ty.clone(),
                generics: site.generics.clone(),
            },
        }
    }

    fn emit_call(
        &mut self,
        dest: &Place,
        callee: Callee,
        args: Vec<Operand>,
        arg_spans: Vec<gt_span::Span>,
        ret_ty: &Ty,
    ) -> R<()> {
        if *ret_ty == Ty::Never {
            // hàm `→ !`: không có khối sau, không có cạnh unwind (ADR-020 §9)
            let t = self.temp(Ty::Never);
            self.terminate(TerminatorKind::Call {
                callee,
                args,
                arg_spans,
                dest: Place::local(t),
                target: None,
            })?;
            self.diverge();
            return Ok(());
        }
        let next = self.new_block();
        self.terminate(TerminatorKind::Call {
            callee,
            args,
            arg_spans,
            dest: dest.clone(),
            target: Some(next),
        })?;
        self.cur = next;
        Ok(())
    }

    // ---------------- vòng lặp ----------------

    /// Ghi nhận khối đầu vòng lặp đang hạ (M8): đích của mọi cạnh lùi.
    fn register_head(&mut self, head: BasicBlock, shape: LoopShape) {
        if let Some((span, node, variant, cond)) = self.pending_loop.take() {
            self.body.loops.push(LoopInfo {
                head,
                shape,
                node,
                span,
                variant,
                cond,
            });
        }
    }

    fn loop_into(&mut self, l: &Loop) -> R<()> {
        let cond_clone = match &l.kind {
            LoopKind::While(c) => Some((**c).clone()),
            _ => None,
        };
        self.pending_loop = Some((
            self.at.0,
            self.at.1,
            l.variant.as_deref().cloned(),
            cond_clone,
        ));
        if let Some(v) = &l.variant {
            self.body.contracts.push(v.id);
        }
        for i in &l.invariants {
            self.body.contracts.push(i.id);
        }
        let scratch = self.temp(Ty::unit());
        let scr = Place::local(scratch);
        match &l.kind {
            LoopKind::Infinite => {
                let head = self.new_block();
                let exit = self.new_block();
                self.goto(head)?;
                self.register_head(head, LoopShape::Infinite);
                self.invariants_at_head(&l.invariants)?;
                self.enter_loop(exit, head);
                self.lower_block(&scr, &l.body)?;
                self.terminate(TerminatorKind::Goto(head))?;
                self.leave_loop();
                self.cur = exit;
                Ok(())
            }
            LoopKind::While(c) => {
                let head = self.new_block();
                let body_bb = self.new_block();
                let exit = self.new_block();
                let lit = match &c.kind {
                    ExprKind::Lit(Lit {
                        kind: LitKind::Bool(v),
                        ..
                    }) => Some(*v),
                    _ => None,
                };
                let compound = Self::is_compound_cond(c);
                let cv = if compound {
                    None
                } else {
                    Some(self.temp(Ty::Bool))
                };
                self.goto(head)?;
                self.register_head(head, LoopShape::While);
                self.invariants_at_head(&l.invariants)?;
                match lit {
                    Some(true) => self.terminate(TerminatorKind::Goto(body_bb))?,
                    Some(false) => self.terminate(TerminatorKind::Goto(exit))?,
                    None if compound => self.cond_branch(c, body_bb, exit, true)?,
                    None => {
                        let cv = cv.expect("điều kiện đơn có biến tạm");
                        self.in_stmt(|b| {
                            let o = b.as_operand(c)?;
                            b.assign(Place::local(cv), Rvalue::Use(o));
                            Ok(())
                        })?;
                        self.terminate(TerminatorKind::SwitchInt {
                            discr: Operand::Copy(Place::local(cv)),
                            targets: vec![(0, exit)],
                            otherwise: body_bb,
                        })?;
                    }
                }
                self.cur = body_bb;
                self.enter_loop(exit, head);
                self.lower_block(&scr, &l.body)?;
                self.terminate(TerminatorKind::Goto(head))?;
                self.leave_loop();
                self.cur = exit;
                Ok(())
            }
            LoopKind::ForIn { var, iter } => self.for_in(var, iter, &l.body, &scr, &l.invariants),
        }
    }

    /// `⋈ P` (ADR-025 §6): kiểm `P` mỗi khi điều khiển tới đầu vòng lặp (kể cả lần thoát). Chỉ là `Assert`: M7 có thể xoá nếu chứng minh được.
    fn invariants_at_head(&mut self, invs: &[Expr]) -> R<()> {
        for inv in invs {
            let t = self.decl_temp_unregistered(Ty::Bool);
            self.push(StatementKind::StorageLive(t));
            self.in_contract += 1;
            self.in_stmt(|b| {
                let o = b.as_operand(inv)?;
                b.assign(Place::local(t), Rvalue::Use(o));
                Ok(())
            })?;
            self.in_contract -= 1;
            self.assert(Operand::Copy(Place::local(t)), true, AssertKind::Invariant)?;
            self.push(StatementKind::StorageDead(t));
        }
        Ok(())
    }

    fn for_in(
        &mut self,
        var: &gt_ast::Ident,
        iter: &Expr,
        body: &Block,
        scr: &Place,
        invs: &[Expr],
    ) -> R<()> {
        let Some(&vid) = self.t.resolve.decls.get(&var.id) else {
            return ice("biến lặp không có định danh");
        };
        let ity = self.ty_of(iter)?;
        match &ity {
            Ty::Ref(m, inner) if matches!(**inner, Ty::Slice(_) | Ty::Array(..)) => {
                let m = *m;
                let el = match &**inner {
                    Ty::Slice(t) | Ty::Array(t, _) => (**t).clone(),
                    _ => Ty::Err,
                };
                let s = if self.is_place_expr(iter) {
                    self.as_place(iter)?
                } else {
                    self.temp_place(iter)?
                };
                let base = s.project(PlaceElem::Deref);
                let usz = Ty::Int(IntTy::Usize);
                let len = self.temp(usz.clone());
                self.assign(Place::local(len), Rvalue::Len(base.clone()));
                let i = self.temp(usz.clone());
                self.assign(
                    Place::local(i),
                    Rvalue::Use(Operand::Const(int_const(0, IntTy::Usize))),
                );
                let head = self.new_block();
                let body_bb = self.new_block();
                let step = self.new_block();
                let exit = self.new_block();
                self.goto(head)?;
                self.register_head(head, LoopShape::ForSlice);
                self.invariants_at_head(invs)?;
                self.branch_bool(
                    Rvalue::BinaryOp(
                        BinOp::Lt,
                        Operand::Copy(Place::local(i)),
                        Operand::Copy(Place::local(len)),
                    ),
                    body_bb,
                    exit,
                )?;
                self.cur = body_bb;
                self.enter_loop(exit, step);
                self.in_block(|b| {
                    let x = b.user_var(vid, false, var.span)?;
                    let k = if m {
                        BorrowKind::Unique
                    } else {
                        BorrowKind::Shared
                    };
                    let rv = b.new_borrow(k, base.project(PlaceElem::Index(i)));
                    b.assign(Place::local(x), rv);
                    b.lower_block(scr, body)
                })?;
                self.terminate(TerminatorKind::Goto(step))?;
                self.leave_loop();
                self.cur = step;
                self.assign(
                    Place::local(i),
                    Rvalue::BinaryOp(
                        BinOp::AddWrap,
                        Operand::Copy(Place::local(i)),
                        Operand::Const(int_const(1, IntTy::Usize)),
                    ),
                );
                self.terminate(TerminatorKind::Goto(head))?;
                self.cur = exit;
                let _ = el;
                Ok(())
            }
            Ty::Adt(d, args) => {
                let el = args.first().cloned().unwrap_or(Ty::Err);
                let Ty::Int(eit) = el else {
                    return ice("lặp trên range không phải số nguyên chưa hỗ trợ ở M5");
                };
                let incl = self.db.lang.get(&Lang::RangeInclusive) == Some(d);
                let (lo, hi) = match &iter.kind {
                    ExprKind::Range {
                        start: Some(s),
                        end: Some(e),
                        ..
                    } => (self.as_operand(s)?, self.as_operand(e)?),
                    _ => {
                        let order = self.range_field_order(*d)?;
                        let (si, ei) = if order { (0, 1) } else { (1, 0) };
                        let p = if self.is_place_expr(iter) {
                            self.as_place(iter)?
                        } else {
                            self.temp_place(iter)?
                        };
                        (
                            Operand::Copy(p.project(PlaceElem::Field(si, el.clone()))),
                            Operand::Copy(p.project(PlaceElem::Field(ei, el.clone()))),
                        )
                    }
                };
                let hi_t = self.temp(el.clone());
                self.assign(Place::local(hi_t), Rvalue::Use(hi));
                let i = self.temp(el.clone());
                self.assign(Place::local(i), Rvalue::Use(lo));
                let head = self.new_block();
                let body_bb = self.new_block();
                let step = self.new_block();
                let exit = self.new_block();
                if incl {
                    // rỗng nếu lo > hi; i == hi ở cuối vòng thì thoát (không tăng quá MAX)
                    let pre = self.new_block();
                    self.branch_bool(
                        Rvalue::BinaryOp(
                            BinOp::Le,
                            Operand::Copy(Place::local(i)),
                            Operand::Copy(Place::local(hi_t)),
                        ),
                        pre,
                        exit,
                    )?;
                    self.cur = pre;
                    self.goto(head)?;
                    self.register_head(head, LoopShape::ForRange);
                    self.invariants_at_head(invs)?;
                    self.terminate(TerminatorKind::Goto(body_bb))?;
                } else {
                    self.goto(head)?;
                    self.register_head(head, LoopShape::ForRange);
                    self.invariants_at_head(invs)?;
                    self.branch_bool(
                        Rvalue::BinaryOp(
                            BinOp::Lt,
                            Operand::Copy(Place::local(i)),
                            Operand::Copy(Place::local(hi_t)),
                        ),
                        body_bb,
                        exit,
                    )?;
                }
                self.cur = body_bb;
                self.enter_loop(exit, step);
                self.in_block(|b| {
                    let x = b.user_var(vid, false, var.span)?;
                    b.assign(Place::local(x), Rvalue::Use(Operand::Copy(Place::local(i))));
                    b.lower_block(scr, body)
                })?;
                self.terminate(TerminatorKind::Goto(step))?;
                self.leave_loop();
                self.cur = step;
                if incl {
                    let inc = self.new_block();
                    self.branch_bool(
                        Rvalue::BinaryOp(
                            BinOp::Eq,
                            Operand::Copy(Place::local(i)),
                            Operand::Copy(Place::local(hi_t)),
                        ),
                        exit,
                        inc,
                    )?;
                    self.cur = inc;
                }
                self.assign(
                    Place::local(i),
                    Rvalue::BinaryOp(
                        BinOp::AddWrap,
                        Operand::Copy(Place::local(i)),
                        Operand::Const(int_const(1, eit)),
                    ),
                );
                self.terminate(TerminatorKind::Goto(head))?;
                self.cur = exit;
                Ok(())
            }
            _ => ice(format!("⟳ in trên kiểu {ity:?}")),
        }
    }

    /// `∀/∃ x ∈ a..b : P` có biên: vòng lặp thuần (ADR-025 §6). Miền không phải khoảng nguyên ⇒ KHÔNG kiểm được lúc chạy:
    /// ghi `Body.uncheckable` (M7 báo E0902) và dùng hằng thay thế (fail-closed ở M7, không phải ở đây).
    fn quantifier_into(
        &mut self,
        dest: &Place,
        qk: gt_ast::QuantifierKind,
        var: &gt_ast::Ident,
        domain: &Expr,
        body: &Expr,
    ) -> R<()> {
        // chốt phòng thủ: M4 (E0316) đã chặn mọi lượng từ ngoài hợp đồng, nên nhánh này không với tới được
        if self.in_contract == 0 {
            return ice("lượng từ ∀/∃ ngoài hợp đồng (đáng lẽ M4 đã báo E0316)");
        }
        let Some(&vid) = self.t.resolve.decls.get(&var.id) else {
            return ice("biến lượng từ không có định danh");
        };
        let forall = qk == gt_ast::QuantifierKind::ForAll;
        let (lo_e, hi_e, incl, eit) = match (&domain.kind, self.ty_of(domain)?) {
            (
                ExprKind::Range {
                    start: Some(s),
                    end: Some(e),
                    inclusive,
                },
                Ty::Adt(_, args),
            ) if matches!(args.first(), Some(Ty::Int(_))) => {
                let Some(Ty::Int(it)) = args.first().cloned() else {
                    return ice("miền lượng từ");
                };
                (s, e, *inclusive, it)
            }
            _ => {
                self.body.uncheckable.push(Uncheckable {
                    node: domain.id,
                    span: domain.span,
                    why: "lượng từ không có biên số nguyên `a..b`".to_string(),
                });
                self.assign(
                    dest.clone(),
                    Rvalue::Use(Operand::Const(Const::Bool(forall))),
                );
                return Ok(());
            }
        };
        let el = Ty::Int(eit);
        let lo = self.as_operand(lo_e)?;
        let hi = self.as_operand(hi_e)?;
        self.assign(
            dest.clone(),
            Rvalue::Use(Operand::Const(Const::Bool(forall))),
        );
        let hi_t = self.temp(el.clone());
        self.assign(Place::local(hi_t), Rvalue::Use(hi));
        let i = self.temp(el.clone());
        self.assign(Place::local(i), Rvalue::Use(lo));
        let v = self.temp(Ty::Bool);
        let head = self.new_block();
        let body_bb = self.new_block();
        let step = self.new_block();
        let exit = self.new_block();
        let cmp = if incl { BinOp::Le } else { BinOp::Lt };
        self.goto(head)?;
        self.branch_bool(
            Rvalue::BinaryOp(
                cmp,
                Operand::Copy(Place::local(i)),
                Operand::Copy(Place::local(hi_t)),
            ),
            body_bb,
            exit,
        )?;
        self.cur = body_bb;
        self.in_block(|b| {
            let x = b.user_var(vid, false, var.span)?;
            b.assign(Place::local(x), Rvalue::Use(Operand::Copy(Place::local(i))));
            b.in_stmt(|b| {
                let o = b.as_operand(body)?;
                b.assign(Place::local(v), Rvalue::Use(o));
                Ok(())
            })
        })?;
        // ∀: v sai ⇒ kết quả sai, thoát; ∃: v đúng ⇒ kết quả đúng, thoát
        let decided = self.new_block();
        let (to_decided, to_step) = if forall { (0, step) } else { (1, step) };
        let _ = to_step;
        if forall {
            self.terminate(TerminatorKind::SwitchInt {
                discr: Operand::Copy(Place::local(v)),
                targets: vec![(0, decided)],
                otherwise: step,
            })?;
        } else {
            self.terminate(TerminatorKind::SwitchInt {
                discr: Operand::Copy(Place::local(v)),
                targets: vec![(0, step)],
                otherwise: decided,
            })?;
        }
        let _ = to_decided;
        self.cur = decided;
        self.assign(
            dest.clone(),
            Rvalue::Use(Operand::Const(Const::Bool(!forall))),
        );
        self.terminate(TerminatorKind::Goto(exit))?;
        self.cur = step;
        if incl {
            let inc = self.new_block();
            self.branch_bool(
                Rvalue::BinaryOp(
                    BinOp::Eq,
                    Operand::Copy(Place::local(i)),
                    Operand::Copy(Place::local(hi_t)),
                ),
                exit,
                inc,
            )?;
            self.cur = inc;
        }
        self.assign(
            Place::local(i),
            Rvalue::BinaryOp(
                BinOp::AddWrap,
                Operand::Copy(Place::local(i)),
                Operand::Const(int_const(1, eit)),
            ),
        );
        self.terminate(TerminatorKind::Goto(head))?;
        self.cur = exit;
        Ok(())
    }
}
