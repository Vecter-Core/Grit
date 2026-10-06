//! Hạ `⌥`: kiểm từng nhánh tuần tự; biến của pattern chỉ được gán SAU khi cả pattern khớp (không move trước khi biết khớp).
//! Vét cạn đã được M4 chứng minh (E0305) nên lối "không nhánh nào khớp" là `Unreachable` (M10 hạ thành abort).
use super::{Builder, R, ice};
use crate::mir::*;
use crate::types::variant_fields;
use gt_ast::{Arm, Expr, Pattern, PatternKind};
use gt_resolve::{DefKind, LocalId, Res};
use gt_types::{AdtKind, IntTy, Ty};

/// Biến của pattern chờ gán: (định danh, chỗ nguồn, kiểu).
type Binds = Vec<(LocalId, Place, Ty, gt_span::Span)>;

impl Builder<'_> {
    pub(crate) fn match_into(&mut self, dest: &Place, scrut: &Expr, arms: &[Arm]) -> R<()> {
        let sty = self.ty_of(scrut)?;
        let sp = if self.is_place_expr(scrut) {
            self.as_place(scrut)?
        } else {
            let t = self.temp(sty.clone());
            self.expr_into(&Place::local(t), scrut)?;
            Place::local(t)
        };
        self.push(StatementKind::FakeRead(
            FakeReadCause::MatchScrutinee,
            sp.clone(),
        ));
        let join = self.new_block();
        for arm in arms {
            self.at = (arm.span, arm.id);
            let fail = self.new_block();
            let mut binds: Binds = Vec::new();
            self.test_pat(&arm.pattern, &sp, &sty, fail, &mut binds)?;
            self.in_block(|b| {
                for (id, place, ty, span) in binds {
                    let l = b.user_var(id, false, span)?;
                    let o = b.operand_of(place, &ty);
                    b.assign(Place::local(l), Rvalue::Use(o));
                }
                b.in_stmt(|b| b.expr_into(dest, &arm.body))
            })?;
            self.terminate(TerminatorKind::Goto(join))?;
            self.cur = fail;
        }
        self.terminate(TerminatorKind::Unreachable)?;
        self.cur = join;
        Ok(())
    }

    /// Phát kiểm tra pattern `p` trên chỗ `place` (kiểu `ty`): không khớp thì nhảy `fail`; khớp thì đi tiếp ở `self.cur`.
    fn test_pat(
        &mut self,
        p: &Pattern,
        place: &Place,
        ty: &Ty,
        fail: BasicBlock,
        binds: &mut Binds,
    ) -> R<()> {
        match &p.kind {
            PatternKind::Wildcard | PatternKind::Rest => Ok(()),
            PatternKind::Binding(ident) => match self.res_of(ident.id) {
                Some(Res::Def(vd)) if self.t.defs()[vd.index()].kind == DefKind::Variant => {
                    self.variant_test(vd, &[], place, ty, fail, binds)
                }
                _ => {
                    let Some(&id) = self.t.resolve.decls.get(&ident.id) else {
                        return ice(format!("binding `{}` không có định danh", ident.name));
                    };
                    binds.push((id, place.clone(), ty.clone(), ident.span));
                    Ok(())
                }
            },
            PatternKind::Lit(l) => {
                let c = self.lit_const(l, ty)?;
                let next = self.new_block();
                self.switch_rv(
                    Rvalue::BinaryOp(BinOp::Eq, Operand::Copy(place.clone()), Operand::Const(c)),
                    Ty::Bool,
                    vec![(0, fail)],
                    next,
                )?;
                self.cur = next;
                Ok(())
            }
            PatternKind::Range {
                start,
                end,
                inclusive,
            } => {
                if let Some(s) = start {
                    let c = self.lit_const(s, ty)?;
                    let next = self.new_block();
                    self.switch_rv(
                        Rvalue::BinaryOp(
                            BinOp::Ge,
                            Operand::Copy(place.clone()),
                            Operand::Const(c),
                        ),
                        Ty::Bool,
                        vec![(0, fail)],
                        next,
                    )?;
                    self.cur = next;
                }
                if let Some(e) = end {
                    let c = self.lit_const(e, ty)?;
                    let op = if *inclusive { BinOp::Le } else { BinOp::Lt };
                    let next = self.new_block();
                    self.switch_rv(
                        Rvalue::BinaryOp(op, Operand::Copy(place.clone()), Operand::Const(c)),
                        Ty::Bool,
                        vec![(0, fail)],
                        next,
                    )?;
                    self.cur = next;
                }
                Ok(())
            }
            PatternKind::Variant { path, args } => match self.res_of(path.id) {
                Some(Res::Def(vd)) if self.t.defs()[vd.index()].kind == DefKind::Variant => {
                    self.variant_test(vd, args, place, ty, fail, binds)
                }
                _ => ice("pattern biến thể không phân giải được sau M4"),
            },
            PatternKind::Struct { fields, .. } => {
                let Ty::Adt(d, args) = ty else {
                    return ice("pattern struct trên kiểu không phải ADT");
                };
                let Some(AdtKind::Struct(defs)) = self.db.adts.get(d).map(|a| a.kind.clone())
                else {
                    return ice("pattern struct trên kiểu không phải struct");
                };
                let fts = variant_fields(self.db, *d, 0, args);
                for fp in fields {
                    let Some(k) = defs.iter().position(|f| f.name == fp.name.name) else {
                        return ice(format!("không có trường `{}` sau M4", fp.name.name));
                    };
                    let sub = place.project(PlaceElem::Field(k as u32, fts[k].clone()));
                    match &fp.pattern {
                        Some(sp) => self.test_pat(sp, &sub, &fts[k], fail, binds)?,
                        None => {
                            // `P { x }` = `P { x: x }`
                            let Some(&id) = self.t.resolve.decls.get(&fp.name.id) else {
                                return ice("trường viết tắt không có định danh");
                            };
                            binds.push((id, sub, fts[k].clone(), fp.name.span));
                        }
                    }
                }
                Ok(())
            }
            PatternKind::Tuple(v) => {
                let Ty::Tuple(ts) = ty else {
                    return ice("pattern tuple trên kiểu không phải tuple");
                };
                let ts = ts.clone();
                self.seq_pats(
                    v,
                    &ts,
                    |k, t| place.project(PlaceElem::Field(k as u32, t.clone())),
                    fail,
                    binds,
                )
            }
        }
    }

    /// Dãy pattern con có thể chứa `..` (một lần): căn đầu trước `..`, căn cuối sau `..`.
    fn seq_pats(
        &mut self,
        pats: &[Pattern],
        tys: &[Ty],
        sub: impl Fn(usize, &Ty) -> Place,
        fail: BasicBlock,
        binds: &mut Binds,
    ) -> R<()> {
        let rest = pats
            .iter()
            .position(|p| matches!(p.kind, PatternKind::Rest));
        for (i, p) in pats.iter().enumerate() {
            if matches!(p.kind, PatternKind::Rest) {
                continue;
            }
            let k = match rest {
                Some(r) if i > r => tys.len() - (pats.len() - i),
                _ => i,
            };
            let Some(t) = tys.get(k) else {
                return ice("pattern con vượt số thành phần sau M4");
            };
            let pl = sub(k, t);
            self.test_pat(p, &pl, t, fail, binds)?;
        }
        Ok(())
    }

    fn variant_test(
        &mut self,
        vd: gt_resolve::DefId,
        args: &[Pattern],
        place: &Place,
        ty: &Ty,
        fail: BasicBlock,
        binds: &mut Binds,
    ) -> R<()> {
        let (ed, vi) = self.variant_of(vd)?;
        let targs = match ty {
            Ty::Adt(d, a) if *d == ed => a.clone(),
            _ => return ice("pattern biến thể trên kiểu khác enum của nó"),
        };
        let next = self.new_block();
        self.switch_rv(
            Rvalue::Discriminant(place.clone()),
            Ty::Int(IntTy::Usize),
            vec![(u128::from(vi), next)],
            fail,
        )?;
        self.cur = next;
        let fts = variant_fields(self.db, ed, vi, &targs);
        let down = place.project(PlaceElem::Downcast(vi));
        self.seq_pats(
            args,
            &fts,
            |k, t| down.project(PlaceElem::Field(k as u32, t.clone())),
            fail,
            binds,
        )
    }
}
