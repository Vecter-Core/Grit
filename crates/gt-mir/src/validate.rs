//! Validator MIR (ADR-020 §7, §9a). KHÔNG thuộc TCB: chỉ có thể từ chối (lỗi nội bộ), không làm chương trình sai được chấp nhận.
//! Kiểm: (1) cấu trúc — terminator, chỉ mục, khoản mượn một-một, biến region, kiểu của phép gán;
//! (2) trên các khối TỚI ĐƯỢC — `StorageLive/Dead` cân và nhất quán trên mọi đường (cùng một tập "đang sống" ở mọi
//! cạnh vào một khối), mọi lần dùng một local là lúc nó đang sống, và ở `Return` không còn biến nào sống.
use crate::mir::*;
use crate::types::{operand_ty, place_ty, region_slots, rvalue_ty};
use gt_types::{Ty, TypeDb};
use std::collections::BTreeSet;

/// Kiểm một thân; trả mọi vi phạm (thứ tự tất định).
pub fn validate(body: &Body, db: &TypeDb) -> Result<(), Vec<String>> {
    let mut v = V {
        body,
        db,
        errs: Vec::new(),
    };
    v.unresolved_types();
    if v.errs.is_empty() {
        v.structure();
    }
    if v.errs.is_empty() {
        v.storage();
    }
    if v.errs.is_empty() {
        Ok(())
    } else {
        Err(v.errs)
    }
}

struct V<'a> {
    body: &'a Body,
    db: &'a TypeDb,
    errs: Vec<String>,
}

/// Mọi local được nhắc tới trong một chỗ (gốc và các chỉ số).
fn place_locals(p: &Place, out: &mut Vec<Local>) {
    out.push(p.local);
    for e in &p.projection {
        match e {
            PlaceElem::Index(l) => out.push(*l),
            PlaceElem::Subslice { from, to } => {
                out.push(*from);
                if let Some(t) = to {
                    out.push(*t);
                }
            }
            _ => {}
        }
    }
}

fn operand_locals(o: &Operand, out: &mut Vec<Local>) {
    if let Operand::Copy(p) | Operand::Move(p) = o {
        place_locals(p, out);
    }
}

fn rvalue_operands(rv: &Rvalue) -> (Vec<&Operand>, Vec<&Place>) {
    match rv {
        Rvalue::Use(o) | Rvalue::Repeat(o, _) | Rvalue::Cast(o, _) | Rvalue::CastFits(o, _) => {
            (vec![o], vec![])
        }
        Rvalue::UnaryOp(_, o) => (vec![o], vec![]),
        Rvalue::BinaryOp(_, a, b) | Rvalue::CheckedBinaryOp(_, a, b) => (vec![a, b], vec![]),
        Rvalue::Ref(_, _, p) | Rvalue::Len(p) | Rvalue::Discriminant(p) => (vec![], vec![p]),
        Rvalue::Aggregate(_, v) => (v.iter().collect(), vec![]),
    }
}

fn assert_operands(k: &AssertKind) -> Vec<&Operand> {
    match k {
        AssertKind::Overflow(_, a, b) => vec![a, b],
        AssertKind::OverflowNeg(a)
        | AssertKind::DivisionByZero(a)
        | AssertKind::RemainderByZero(a) => {
            vec![a]
        }
        AssertKind::IndexOutOfBounds { len, index } => vec![len, index],
        AssertKind::SliceRange { from, to, len } => vec![from, to, len],
        AssertKind::CastLossy { value, .. } => vec![value],
        AssertKind::Precondition
        | AssertKind::Postcondition
        | AssertKind::Invariant
        | AssertKind::Fuel => vec![],
    }
}

/// Các local mà một câu lệnh dùng (đọc hoặc ghi), KHÔNG tính `StorageLive/Dead`.
fn statement_uses(s: &Statement) -> Vec<Local> {
    let mut out = Vec::new();
    match &s.kind {
        StatementKind::Assign(p, rv) => {
            place_locals(p, &mut out);
            let (os, ps) = rvalue_operands(rv);
            for o in os {
                operand_locals(o, &mut out);
            }
            for p in ps {
                place_locals(p, &mut out);
            }
        }
        StatementKind::FakeRead(_, p) | StatementKind::Wipe(p) => place_locals(p, &mut out),
        StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop => {}
    }
    out
}

fn terminator_uses(t: &TerminatorKind) -> Vec<Local> {
    let mut out = Vec::new();
    match t {
        TerminatorKind::SwitchInt { discr, .. } => operand_locals(discr, &mut out),
        TerminatorKind::Drop { place, .. } => place_locals(place, &mut out),
        TerminatorKind::Call {
            callee, args, dest, ..
        } => {
            if let Callee::Value(o) = callee {
                operand_locals(o, &mut out);
            }
            for a in args {
                operand_locals(a, &mut out);
            }
            place_locals(dest, &mut out);
        }
        TerminatorKind::Assert { cond, kind, .. } => {
            operand_locals(cond, &mut out);
            for o in assert_operands(kind) {
                operand_locals(o, &mut out);
            }
        }
        TerminatorKind::Goto(_) | TerminatorKind::Return | TerminatorKind::Unreachable => {}
    }
    out
}

/// Các khối tới được từ entry (theo thứ tự chỉ số).
#[must_use]
pub fn reachable(body: &Body) -> Vec<bool> {
    let mut seen = vec![false; body.blocks.len()];
    let mut stack = vec![ENTRY];
    while let Some(b) = stack.pop() {
        if b.index() >= seen.len() || seen[b.index()] {
            continue;
        }
        seen[b.index()] = true;
        if let Some(t) = &body.blocks[b.index()].terminator {
            for s in t.kind.successors().into_iter().rev() {
                stack.push(s);
            }
        }
    }
    seen
}

/// `Ty::Err`/`Ty::Infer` ở BẤT KỲ độ sâu nào (M5 chỉ chạy khi M4 sạch ⇒ không bao giờ được xuất hiện trong MIR; FAIL_OPEN_AUDIT S-D).
fn unresolved(t: &Ty) -> bool {
    match t {
        Ty::Err | Ty::Infer(_) => true,
        Ty::Tuple(v) | Ty::Adt(_, v) => v.iter().any(unresolved),
        Ty::Array(t, _) | Ty::Slice(t) | Ty::Ref(_, t) | Ty::RawPtr(_, t) => unresolved(t),
        Ty::Fn(p, r, _) => p.iter().any(unresolved) || unresolved(r),
        Ty::Int(_) | Ty::Float(_) | Ty::Bool | Ty::Char | Ty::Str | Ty::Never | Ty::Param(_) => {
            false
        }
    }
}

fn place_tys<'p>(p: &'p Place, out: &mut Vec<&'p Ty>) {
    for e in &p.projection {
        if let PlaceElem::Field(_, t) = e {
            out.push(t);
        }
    }
}

fn operand_tys<'p>(o: &'p Operand, out: &mut Vec<&'p Ty>) {
    match o {
        Operand::Copy(p) | Operand::Move(p) => place_tys(p, out),
        Operand::Const(Const::FnItem { generics, ty, .. }) => {
            out.push(ty);
            out.extend(generics.iter());
        }
        Operand::Const(Const::Item { ty, .. }) => out.push(ty),
        Operand::Const(_) => {}
    }
}

impl V<'_> {
    fn err(&mut self, m: String) {
        self.errs.push(m);
    }

    /// Mọi `Ty` đến được từ thân không được chứa `Err`/`Infer`.
    fn unresolved_types(&mut self) {
        let b = self.body;
        let mut tys: Vec<&Ty> = b.locals.iter().map(|l| &l.ty).collect();
        for bb in &b.blocks {
            for s in &bb.statements {
                match &s.kind {
                    StatementKind::Assign(p, rv) => {
                        place_tys(p, &mut tys);
                        let (os, ps) = rvalue_operands(rv);
                        for o in os {
                            operand_tys(o, &mut tys);
                        }
                        for p in ps {
                            place_tys(p, &mut tys);
                        }
                        match rv {
                            Rvalue::Cast(_, t) | Rvalue::CastFits(_, t) => tys.push(t),
                            Rvalue::Aggregate(AggregateKind::Array(t), _) => tys.push(t),
                            Rvalue::Aggregate(AggregateKind::Adt(_, _, args), _) => {
                                tys.extend(args.iter());
                            }
                            _ => {}
                        }
                    }
                    StatementKind::FakeRead(_, p) | StatementKind::Wipe(p) => {
                        place_tys(p, &mut tys)
                    }
                    StatementKind::StorageLive(_)
                    | StatementKind::StorageDead(_)
                    | StatementKind::Nop => {}
                }
            }
            if let Some(t) = &bb.terminator {
                match &t.kind {
                    TerminatorKind::SwitchInt { discr, .. } => operand_tys(discr, &mut tys),
                    TerminatorKind::Drop { place, .. } => place_tys(place, &mut tys),
                    TerminatorKind::Call {
                        callee, args, dest, ..
                    } => {
                        match callee {
                            Callee::Fn { generics, .. } => tys.extend(generics.iter()),
                            Callee::TraitMethod {
                                self_ty, generics, ..
                            } => {
                                tys.push(self_ty);
                                tys.extend(generics.iter());
                            }
                            Callee::Value(o) => operand_tys(o, &mut tys),
                        }
                        for a in args {
                            operand_tys(a, &mut tys);
                        }
                        place_tys(dest, &mut tys);
                    }
                    TerminatorKind::Assert { cond, kind, .. } => {
                        operand_tys(cond, &mut tys);
                        for o in assert_operands(kind) {
                            operand_tys(o, &mut tys);
                        }
                        if let AssertKind::CastLossy { to, .. } = kind {
                            tys.push(to);
                        }
                    }
                    TerminatorKind::Goto(_)
                    | TerminatorKind::Return
                    | TerminatorKind::Unreachable => {}
                }
            }
        }
        if tys.iter().any(|t| unresolved(t)) {
            self.err("kiểu chưa giải quyết (Err/Infer) trong MIR (M4 phải chặn trước)".to_string());
        }
    }

    fn structure(&mut self) {
        let b = self.body;
        let nl = b.locals.len();
        let nb = b.blocks.len();
        if nb == 0 {
            self.err("thân không có khối nào".to_string());
            return;
        }
        if nl < b.arg_count + 1 {
            self.err("thiếu local cho chỗ trả về/tham số".to_string());
            return;
        }
        if !matches!(b.locals[0].kind, LocalKind::Return) {
            self.err("_0 phải là chỗ trả về".to_string());
        }
        for i in 1..=b.arg_count {
            if !matches!(b.locals[i].kind, LocalKind::Arg(_)) {
                self.err(format!("_{i} phải là tham số"));
            }
        }
        // biến region: đúng số vị trí, trong phạm vi, không dùng lại giữa hai vị trí
        let mut seen_r = BTreeSet::new();
        for (i, l) in b.locals.iter().enumerate() {
            let want = region_slots(self.db, &l.ty);
            if l.regions.len() != want {
                self.err(format!(
                    "_{i}: {} biến region, kiểu cần {want}",
                    l.regions.len()
                ));
            }
            for r in &l.regions {
                if r.0 >= b.region_count || !seen_r.insert(*r) {
                    self.err(format!("_{i}: biến region '#{} sai hoặc dùng lại", r.0));
                }
            }
        }
        let mut borrow_seen = vec![0usize; b.borrows.len()];
        for (bi, bb) in b.blocks.iter().enumerate() {
            for (si, s) in bb.statements.iter().enumerate() {
                if s.source.scope.index() >= b.scopes.len() {
                    self.err(format!("bb{bi}[{si}]: phạm vi nguồn sai"));
                }
                for l in statement_uses(s) {
                    if l.index() >= nl {
                        self.err(format!("bb{bi}[{si}]: local _{} không tồn tại", l.0));
                        return;
                    }
                }
                match &s.kind {
                    StatementKind::StorageLive(l) | StatementKind::StorageDead(l) => {
                        if l.index() >= nl {
                            self.err(format!("bb{bi}[{si}]: local _{} không tồn tại", l.0));
                            return;
                        }
                        if l.index() <= b.arg_count {
                            self.err(format!(
                                "bb{bi}[{si}]: Storage* trên chỗ trả về/tham số _{}",
                                l.0
                            ));
                        }
                    }
                    StatementKind::Assign(p, rv) => {
                        if let Rvalue::Ref(id, k, rp) = rv {
                            match b.borrows.get(id.index()) {
                                Some(bd) => {
                                    borrow_seen[id.index()] += 1;
                                    if bd.place != *rp || bd.kind != *k {
                                        self.err(format!(
                                            "bb{bi}[{si}]: Ref không khớp BorrowData b{}",
                                            id.0
                                        ));
                                    }
                                    if bd.region.0 >= b.region_count {
                                        self.err(format!("b{}: biến region sai", id.0));
                                    }
                                }
                                None => self.err(format!(
                                    "bb{bi}[{si}]: khoản mượn b{} không tồn tại",
                                    id.0
                                )),
                            }
                        }
                        let pt = place_ty(b, p);
                        let rt = rvalue_ty(b, self.db, rv);
                        match (pt, rt) {
                            (Some(a), Some(c)) if a == c => {}
                            (a, c) => self.err(format!(
                                "bb{bi}[{si}]: kiểu phép gán không khớp: chỗ {a:?}, giá trị {c:?}"
                            )),
                        }
                    }
                    StatementKind::FakeRead(_, p) | StatementKind::Wipe(p) => {
                        if place_ty(b, p).is_none() {
                            self.err(format!("bb{bi}[{si}]: chỗ không hợp lệ"));
                        }
                    }
                    StatementKind::Nop => {}
                }
            }
            let Some(t) = &bb.terminator else {
                self.err(format!("bb{bi}: thiếu terminator"));
                continue;
            };
            if t.source.scope.index() >= b.scopes.len() {
                self.err(format!("bb{bi}: phạm vi nguồn của terminator sai"));
            }
            for s in t.kind.successors() {
                if s.index() >= nb {
                    self.err(format!("bb{bi}: nhảy tới bb{} không tồn tại", s.0));
                }
            }
            for l in terminator_uses(&t.kind) {
                if l.index() >= nl {
                    self.err(format!("bb{bi}: local _{} không tồn tại", l.0));
                    return;
                }
            }
            match &t.kind {
                TerminatorKind::SwitchInt { discr, .. } => match operand_ty(b, discr) {
                    Some(Ty::Int(_) | Ty::Bool | Ty::Char) => {}
                    other => self.err(format!("bb{bi}: switchInt trên kiểu {other:?}")),
                },
                TerminatorKind::Assert { cond, .. } => {
                    if operand_ty(b, cond) != Some(Ty::Bool) {
                        self.err(format!("bb{bi}: điều kiện assert không phải bool"));
                    }
                }
                TerminatorKind::Call { dest, args, .. } => {
                    if place_ty(b, dest).is_none() {
                        self.err(format!("bb{bi}: đích lời gọi không hợp lệ"));
                    }
                    for a in args {
                        if operand_ty(b, a).is_none() {
                            self.err(format!("bb{bi}: đối số không hợp lệ"));
                        }
                    }
                }
                TerminatorKind::Drop { place, .. } if place_ty(b, place).is_none() => {
                    self.err(format!("bb{bi}: drop chỗ không hợp lệ"));
                }
                _ => {}
            }
        }
        for (i, n) in borrow_seen.iter().enumerate() {
            if *n != 1 {
                self.err(format!("khoản mượn b{i} xuất hiện {n} lần (cần đúng 1)"));
            }
        }
    }

    /// Phân tích tiến "tập local đang có storage" trên các khối tới được.
    fn storage(&mut self) {
        let b = self.body;
        let always = |l: Local| l.index() <= b.arg_count;
        let mut state: Vec<Option<BTreeSet<Local>>> = vec![None; b.blocks.len()];
        state[0] = Some(BTreeSet::new());
        let mut work: BTreeSet<usize> = BTreeSet::new();
        work.insert(0);
        let mut steps = 0usize;
        while let Some(&bi) = work.iter().next() {
            work.remove(&bi);
            steps += 1;
            if steps > 4 * b.blocks.len() + 16 {
                self.err("phân tích storage không hội tụ".to_string());
                return;
            }
            let Some(mut live) = state[bi].clone() else {
                continue;
            };
            let bb = &b.blocks[bi];
            for (si, s) in bb.statements.iter().enumerate() {
                match &s.kind {
                    StatementKind::StorageLive(l) => {
                        if !live.insert(*l) {
                            self.err(format!("bb{bi}[{si}]: StorageLive(_{}) khi đang sống", l.0));
                        }
                    }
                    StatementKind::StorageDead(l) => {
                        if !live.remove(l) {
                            self.err(format!(
                                "bb{bi}[{si}]: StorageDead(_{}) khi không sống",
                                l.0
                            ));
                        }
                    }
                    _ => {
                        for l in statement_uses(s) {
                            if !always(l) && !live.contains(&l) {
                                self.err(format!("bb{bi}[{si}]: dùng _{} ngoài storage", l.0));
                            }
                        }
                    }
                }
            }
            let Some(t) = &bb.terminator else { continue };
            for l in terminator_uses(&t.kind) {
                if !always(l) && !live.contains(&l) {
                    self.err(format!("bb{bi}: terminator dùng _{} ngoài storage", l.0));
                }
            }
            if matches!(t.kind, TerminatorKind::Return) && !live.is_empty() {
                let v: Vec<String> = live.iter().map(|l| format!("_{}", l.0)).collect();
                self.err(format!(
                    "bb{bi}: return khi còn storage sống: {}",
                    v.join(", ")
                ));
            }
            for s in t.kind.successors() {
                match &state[s.index()] {
                    None => {
                        state[s.index()] = Some(live.clone());
                        work.insert(s.index());
                    }
                    Some(prev) if *prev == live => {}
                    Some(prev) => {
                        let a: Vec<String> = prev.iter().map(|l| format!("_{}", l.0)).collect();
                        let c: Vec<String> = live.iter().map(|l| format!("_{}", l.0)).collect();
                        self.err(format!(
                            "bb{}: storage không nhất quán giữa các đường vào: [{}] khác [{}] (từ bb{bi})",
                            s.0,
                            a.join(", "),
                            c.join(", ")
                        ));
                        return;
                    }
                }
            }
        }
    }
}
