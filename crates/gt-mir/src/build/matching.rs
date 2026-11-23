//! Hạ `⌥`: kiểm từng nhánh tuần tự; biến của pattern chỉ được gán SAU khi cả pattern khớp (không move trước khi biết khớp).
//! Vét cạn đã được M4 chứng minh (E0305) nên lối "không nhánh nào khớp" là `Unreachable` (M10 hạ thành abort).
use super::{Builder, R, ice};
use crate::mir::*;
use crate::types::variant_fields;
use gt_ast::{Arm, Expr, Pattern, PatternKind};
use gt_resolve::{DefKind, LocalId, Res};
use gt_types::{AdtKind, IntTy, Lang, Ty};

static PAT_MODE_FAULT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Chốt kiểm thử (`GRITC_FAULT=mir:pat-mode`): làm hạ MIR suy chế độ khớp qua tham chiếu SAI để chứng minh đường ICE E9901. KHÔNG dùng ngoài test.
pub fn set_pat_mode_fault_for_test(on: bool) {
    PAT_MODE_FAULT.store(on, std::sync::atomic::Ordering::SeqCst);
}

static PAT_FIELD_FAULT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Chốt kiểm thử (`GRITC_FAULT=mir:pat-field`): biến bind ở chế độ tham chiếu trỏ NHẦM trường (trường k ≥ 1 ⇒ k − 1) để chứng minh bộ sinh so sánh tự bắt. KHÔNG dùng ngoài test.
pub fn set_pat_field_fault_for_test(on: bool) {
    PAT_FIELD_FAULT.store(on, std::sync::atomic::Ordering::SeqCst);
}

static PAT_SWAP_FAULT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Chốt kiểm thử (`GRITC_FAULT=mir:pat-field-swap`): hai biến bind ở chế độ tham chiếu cùng cha, cùng kiểu, khác trường HOÁN ĐỔI chỗ nguồn (M6 vẫn chấp nhận vì không chồng lấn;
/// chỉ so sánh với mô hình độc lập mới thấy). KHÔNG dùng ngoài test.
pub fn set_pat_swap_fault_for_test(on: bool) {
    PAT_SWAP_FAULT.store(on, std::sync::atomic::Ordering::SeqCst);
}

fn swap_same_typed_binds(binds: &mut Binds) {
    let mut done = vec![false; binds.len()];
    for i in 0..binds.len() {
        if done[i] || binds[i].4.is_none() {
            continue;
        }
        for j in i + 1..binds.len() {
            if done[j] || binds[j].4.is_none() || binds[i].2 != binds[j].2 {
                continue;
            }
            let (a, b) = (&binds[i].1, &binds[j].1);
            let n = a.projection.len();
            if a.local == b.local
                && n == b.projection.len()
                && n > 0
                && a.projection[..n - 1] == b.projection[..n - 1]
                && matches!(
                    (&a.projection[n - 1], &b.projection[n - 1]),
                    (PlaceElem::Field(x, _), PlaceElem::Field(y, _)) if x != y
                )
            {
                let pb = binds[j].1.clone();
                let pa = std::mem::replace(&mut binds[i].1, pb);
                binds[j].1 = pa;
                done[i] = true;
                done[j] = true;
                break;
            }
        }
    }
}

fn pat_mode_fault() -> bool {
    PAT_MODE_FAULT.load(std::sync::atomic::Ordering::SeqCst)
}

/// Biến của pattern chờ gán: (định danh, chỗ nguồn, kiểu).
/// `Some(loại)` = bind ở CHẾ ĐỘ THAM CHIẾU (ADR-040): biến là tham chiếu tới chỗ nguồn (khoản vay mới), không phải giá trị của nó.
type Binds = Vec<(LocalId, Place, Ty, gt_span::Span, Option<BorrowKind>)>;

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
            self.test_pat(&arm.pattern, &sp, &sty, None, fail, &mut binds)?;
            if PAT_SWAP_FAULT.load(std::sync::atomic::Ordering::SeqCst) {
                swap_same_typed_binds(&mut binds);
            }
            self.in_block(|b| {
                for (id, place, ty, span, mode) in binds {
                    let l = b.user_var(id, false, span)?;
                    match mode {
                        Some(k) => {
                            let mut place = place;
                            if PAT_FIELD_FAULT.load(std::sync::atomic::Ordering::SeqCst)
                                && let Some(PlaceElem::Field(i, t)) =
                                    place.projection.last().cloned()
                                && i >= 1
                            {
                                place.projection.pop();
                                place.projection.push(PlaceElem::Field(i - 1, t));
                            }
                            let rv = b.new_borrow(k, place);
                            b.assign(Place::local(l), rv);
                        }
                        None => {
                            let o = b.operand_of(place, &ty);
                            b.assign(Place::local(l), Rvalue::Use(o));
                        }
                    }
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

    /// `e?` (ADR-P2-09): tương đương `⌥ e { Some(x) ↦ x, None ↦ ⏎ None }` / `⌥ e { Ok(x) ↦ x, Err(er) ↦ ⏎ Err(er) }`, dựng bằng CHÍNH các nguyên thủy của `⌥`/`⏎`
    /// (`Discriminant` + `SwitchInt`, `Downcast`/`Field`, `exit_to(0)` + `Return`) — không thêm loại câu lệnh/terminator MIR. M4 đã bảo đảm hàm bao trả cùng họ (và cùng `E`).
    pub(crate) fn try_into(&mut self, dest: &Place, inner: &Expr) -> R<()> {
        let ity = self.ty_of(inner)?;
        let Ty::Adt(d, targs) = &ity else {
            return ice("`?` trên kiểu không phải ADT sau M4");
        };
        let d = *d;
        let is_opt = self.db.lang.get(&Lang::Option) == Some(&d);
        if !is_opt && self.db.lang.get(&Lang::Result) != Some(&d) {
            return ice("`?` trên kiểu không phải Option/Result sau M4");
        }
        let (ok_name, bad_name) = if is_opt {
            ("Some", "None")
        } else {
            ("Ok", "Err")
        };
        let vi = |name: &str| -> R<u32> {
            match self.db.adts.get(&d).map(|a| &a.kind) {
                Some(AdtKind::Enum(vs)) => match vs.iter().position(|v| v.name == name) {
                    Some(i) => Ok(i as u32),
                    None => ice(format!("thiếu biến thể `{name}`")),
                },
                _ => ice("Option/Result không phải enum"),
            }
        };
        let (ok_vi, bad_vi) = (vi(ok_name)?, vi(bad_name)?);
        // kiểu trả về của hàm bao: cùng họ (M4)
        let rty = self.body.locals[RETURN_PLACE.index()].ty.clone();
        let Ty::Adt(rd, rargs) = &rty else {
            return ice("`?` trong hàm trả kiểu không phải ADT sau M4");
        };
        if *rd != d {
            return ice("`?`: hàm bao trả khác họ sau M4");
        }
        let rargs = rargs.clone();
        let t = self.temp(ity.clone());
        let tp = Place::local(t);
        self.expr_into(&tp, inner)?;
        let ok_blk = self.new_block();
        let bad_blk = self.new_block();
        self.switch_rv(
            Rvalue::Discriminant(tp.clone()),
            Ty::Int(IntTy::Usize),
            vec![(u128::from(ok_vi), ok_blk)],
            bad_blk,
        )?;
        // nhánh lan truyền: dựng giá trị trả về từ `None` / `Err(er)` rồi thoát như `⏎`
        self.cur = bad_blk;
        let ret_rv = if is_opt {
            Rvalue::Aggregate(AggregateKind::Adt(d, bad_vi, rargs.clone()), vec![])
        } else {
            let fts = variant_fields(self.db, d, bad_vi, targs);
            let Some(ety) = fts.first() else {
                return ice("`Err` không có trường");
            };
            let src = tp
                .project(PlaceElem::Downcast(bad_vi))
                .project(PlaceElem::Field(0, ety.clone()));
            let op = self.operand_of(src, ety);
            Rvalue::Aggregate(AggregateKind::Adt(d, bad_vi, rargs.clone()), vec![op])
        };
        self.assign(Place::local(RETURN_PLACE), ret_rv);
        self.exit_to(0)?;
        self.terminate(TerminatorKind::Return)?;
        self.diverge();
        // nhánh tiếp tục: giá trị bên trong `Some(x)` / `Ok(x)`
        self.cur = ok_blk;
        let fts = variant_fields(self.db, d, ok_vi, targs);
        let Some(vty) = fts.first() else {
            return ice("`Some`/`Ok` không có trường");
        };
        let src = tp
            .project(PlaceElem::Downcast(ok_vi))
            .project(PlaceElem::Field(0, vty.clone()));
        let op = self.operand_of(src, vty);
        self.assign(dest.clone(), Rvalue::Use(op));
        Ok(())
    }

    /// Pattern phá cấu trúc (khớp luật của gt-types `destructures`).
    fn destructures(&self, p: &Pattern) -> bool {
        match &p.kind {
            PatternKind::Variant { .. } | PatternKind::Struct { .. } | PatternKind::Tuple(_) => {
                true
            }
            PatternKind::Binding(ident) => matches!(
                self.res_of(ident.id),
                Some(Res::Def(vd)) if self.t.defs()[vd.index()].kind == DefKind::Variant
            ),
            _ => false,
        }
    }

    /// Phát kiểm tra pattern `p` trên chỗ `place` (kiểu `ty`): không khớp thì nhảy `fail`; khớp thì đi tiếp ở `self.cur`.
    /// `mode`: chế độ tham chiếu (ADR-040) — `Some(duy nhất?)` khi đã đi qua một tham chiếu; pattern phá cấu trúc trên chỗ kiểu `&U`/`&!U` (khi chưa có chế độ)
    /// khớp `U` tại `*place` và bật chế độ.
    fn test_pat(
        &mut self,
        p: &Pattern,
        place: &Place,
        ty: &Ty,
        mode: Option<bool>,
        fail: BasicBlock,
        binds: &mut Binds,
    ) -> R<()> {
        if self.destructures(p)
            && let Ty::Ref(m, inner) = ty
        {
            if mode.is_some() {
                return ice(
                    "pattern phá cấu trúc trên tham chiếu lồng trong chế độ tham chiếu (M4 phải chặn E0301)",
                );
            }
            // ADR-040: chế độ MIR suy ra PHẢI bằng chế độ gt-types đã ghi (`pat_modes`); lệch ⇒ ICE, không hạ lặng lẽ theo chế độ khác
            let derived = if pat_mode_fault() { !*m } else { *m };
            if self.t.pat_modes.get(&p.id) != Some(&derived) {
                return ice(format!(
                    "chế độ khớp qua tham chiếu của MIR ({derived}) khác gt-types ({:?})",
                    self.t.pat_modes.get(&p.id)
                ));
            }
            let deref = place.project(PlaceElem::Deref);
            return self.test_pat_in(p, &deref, inner, Some(derived), fail, binds);
        }
        if self.destructures(p) && mode.is_none() && self.t.pat_modes.contains_key(&p.id) {
            return ice(
                "gt-types ghi chế độ tham chiếu cho một pattern mà MIR không thấy tham chiếu",
            );
        }
        self.test_pat_in(p, place, ty, mode, fail, binds)
    }

    fn bind_kind(mode: Option<bool>) -> Option<BorrowKind> {
        mode.map(|m| {
            if m {
                BorrowKind::Unique
            } else {
                BorrowKind::Shared
            }
        })
    }

    fn test_pat_in(
        &mut self,
        p: &Pattern,
        place: &Place,
        ty: &Ty,
        mode: Option<bool>,
        fail: BasicBlock,
        binds: &mut Binds,
    ) -> R<()> {
        match &p.kind {
            PatternKind::Wildcard | PatternKind::Rest => Ok(()),
            PatternKind::Binding(ident) => match self.res_of(ident.id) {
                Some(Res::Def(vd)) if self.t.defs()[vd.index()].kind == DefKind::Variant => {
                    self.variant_test(vd, &[], place, ty, mode, fail, binds)
                }
                _ => {
                    let Some(&id) = self.t.resolve.decls.get(&ident.id) else {
                        return ice(format!("binding `{}` không có định danh", ident.name));
                    };
                    binds.push((
                        id,
                        place.clone(),
                        ty.clone(),
                        ident.span,
                        Self::bind_kind(mode),
                    ));
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
                    self.variant_test(vd, args, place, ty, mode, fail, binds)
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
                        Some(sp) => self.test_pat(sp, &sub, &fts[k], mode, fail, binds)?,
                        None => {
                            // `P { x }` = `P { x: x }`
                            let Some(&id) = self.t.resolve.decls.get(&fp.name.id) else {
                                return ice("trường viết tắt không có định danh");
                            };
                            binds.push((
                                id,
                                sub,
                                fts[k].clone(),
                                fp.name.span,
                                Self::bind_kind(mode),
                            ));
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
                    mode,
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
        mode: Option<bool>,
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
            self.test_pat(p, &pl, t, mode, fail, binds)?;
        }
        Ok(())
    }

    // 8 tham số: (cổng clippy) `mode` (ADR-040) là một giá trị độc lập thêm vào cùng nhóm với `fail`/`binds`; gom vào struct chỉ để lách lint sẽ làm mờ luồng chế độ
    #[allow(clippy::too_many_arguments)]
    fn variant_test(
        &mut self,
        vd: gt_resolve::DefId,
        args: &[Pattern],
        place: &Place,
        ty: &Ty,
        mode: Option<bool>,
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
            mode,
            fail,
            binds,
        )
    }
}
