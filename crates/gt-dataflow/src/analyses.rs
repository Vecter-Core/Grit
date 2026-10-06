//! Phân tích cài sẵn (ADR-020 §3): move path, MaybeInit, MaybeUninit, Liveness, và bất biến "đã gán trước khi đọc".
use crate::framework::{Analysis, BitSet, Direction, Dual};
use gt_mir::*;

// ======================= move path =======================

/// Cây move path: gốc là local, con theo `Field`/`Downcast` (không qua `Deref`/`Index`/`Subslice` — không move ra được).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovePaths {
    /// (cha, phép chiếu tới nút này từ cha); gốc có cha `None`.
    pub nodes: Vec<(Option<usize>, Option<PlaceElem>, Local)>,
    /// Nút gốc của mỗi local.
    pub roots: Vec<usize>,
}

fn movable_prefix(p: &Place) -> &[PlaceElem] {
    let n = p
        .projection
        .iter()
        .position(|e| !matches!(e, PlaceElem::Field(..) | PlaceElem::Downcast(_)))
        .unwrap_or(p.projection.len());
    &p.projection[..n]
}

impl MovePaths {
    #[must_use]
    pub fn build(body: &Body) -> MovePaths {
        let mut mp = MovePaths {
            nodes: Vec::new(),
            roots: Vec::new(),
        };
        for i in 0..body.locals.len() {
            mp.roots.push(mp.nodes.len());
            mp.nodes.push((None, None, Local(i as u32)));
        }
        let add = |p: &Place, mp: &mut MovePaths| {
            let mut cur = mp.roots[p.local.index()];
            for e in movable_prefix(p) {
                let found = mp
                    .nodes
                    .iter()
                    .position(|(par, el, _)| *par == Some(cur) && el.as_ref() == Some(e));
                cur = match found {
                    Some(c) => c,
                    None => {
                        mp.nodes.push((Some(cur), Some(e.clone()), p.local));
                        mp.nodes.len() - 1
                    }
                };
            }
        };
        for bb in &body.blocks {
            for s in &bb.statements {
                if let StatementKind::Assign(p, rv) = &s.kind {
                    add(p, &mut mp);
                    for o in rvalue_moves(rv) {
                        add(o, &mut mp);
                    }
                }
            }
            if let Some(t) = &bb.terminator {
                match &t.kind {
                    TerminatorKind::Call {
                        args, dest, callee, ..
                    } => {
                        for a in args {
                            if let Operand::Move(p) = a {
                                add(p, &mut mp);
                            }
                        }
                        if let Callee::Value(Operand::Move(p)) = callee {
                            add(p, &mut mp);
                        }
                        add(dest, &mut mp);
                    }
                    TerminatorKind::Drop { place, .. } => add(place, &mut mp),
                    _ => {}
                }
            }
        }
        mp
    }

    /// Nút ứng với chỗ (tiền tố movable dài nhất có trong cây). `None` nếu chỗ đi qua `Deref` ngay từ gốc.
    #[must_use]
    pub fn find(&self, p: &Place) -> usize {
        let mut cur = self.roots[p.local.index()];
        for e in movable_prefix(p) {
            match self
                .nodes
                .iter()
                .position(|(par, el, _)| *par == Some(cur) && el.as_ref() == Some(e))
            {
                Some(c) => cur = c,
                None => break,
            }
        }
        cur
    }

    /// Nút và mọi hậu duệ.
    #[must_use]
    pub fn subtree(&self, n: usize) -> Vec<usize> {
        let mut out = vec![n];
        let mut i = 0;
        while i < out.len() {
            let x = out[i];
            for (k, (par, _, _)) in self.nodes.iter().enumerate() {
                if *par == Some(x) {
                    out.push(k);
                }
            }
            i += 1;
        }
        out
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Các chỗ bị move bởi một rvalue.
fn rvalue_moves(rv: &Rvalue) -> Vec<&Place> {
    let ops: Vec<&Operand> = match rv {
        Rvalue::Use(o)
        | Rvalue::Repeat(o, _)
        | Rvalue::Cast(o, _)
        | Rvalue::CastFits(o, _)
        | Rvalue::UnaryOp(_, o) => {
            vec![o]
        }
        Rvalue::BinaryOp(_, a, b) | Rvalue::CheckedBinaryOp(_, a, b) => vec![a, b],
        Rvalue::Aggregate(_, v) => v.iter().collect(),
        Rvalue::Ref(..) | Rvalue::Len(_) | Rvalue::Discriminant(_) => vec![],
    };
    ops.into_iter()
        .filter_map(|o| match o {
            Operand::Move(p) if !p.has_deref() => Some(p),
            _ => None,
        })
        .collect()
}

// ======================= MaybeInit / MaybeUninit =======================

/// "Có thể đã khởi tạo" trên move path (tiến, hợp). Move và `Drop` làm chỗ mất khởi tạo; gán làm có khởi tạo.
pub struct MaybeInit<'a> {
    pub paths: &'a MovePaths,
}

/// "Có thể CHƯA khởi tạo" (tiến, hợp). Khởi tạo chắc chắn của `p` = `p ∉ MaybeUninit`. M6 dùng cho E0401.
pub struct MaybeUninit<'a> {
    pub paths: &'a MovePaths,
}

fn set_tree(s: &mut BitSet, mp: &MovePaths, n: usize, on: bool) {
    for k in mp.subtree(n) {
        if on {
            s.insert(k);
        } else {
            s.remove(k);
        }
    }
}

fn init_effects(s: &mut BitSet, mp: &MovePaths, stmt: &Statement, init_bit: bool) {
    match &stmt.kind {
        StatementKind::Assign(p, rv) => {
            for m in rvalue_moves(rv) {
                set_tree(s, mp, mp.find(m), !init_bit);
            }
            if !p.has_deref() {
                set_tree(s, mp, mp.find(p), init_bit);
            }
        }
        StatementKind::StorageLive(l) | StatementKind::StorageDead(l) => {
            set_tree(s, mp, mp.roots[l.index()], !init_bit);
        }
        StatementKind::FakeRead(..) | StatementKind::Nop => {}
    }
}

fn init_term(s: &mut BitSet, mp: &MovePaths, t: &Terminator, init_bit: bool) {
    match &t.kind {
        TerminatorKind::Call { args, callee, .. } => {
            for a in args {
                if let Operand::Move(p) = a
                    && !p.has_deref()
                {
                    set_tree(s, mp, mp.find(p), !init_bit);
                }
            }
            if let Callee::Value(Operand::Move(p)) = callee
                && !p.has_deref()
            {
                set_tree(s, mp, mp.find(p), !init_bit);
            }
        }
        TerminatorKind::Drop { place, .. } if !place.has_deref() => {
            set_tree(s, mp, mp.find(place), !init_bit);
        }
        _ => {}
    }
}

impl Analysis for MaybeInit<'_> {
    type Domain = BitSet;
    const DIRECTION: Direction = Direction::Forward;
    fn bottom(&self, _: &Body) -> BitSet {
        BitSet::empty(self.paths.len())
    }
    fn initialize_start(&self, body: &Body, s: &mut BitSet) {
        for i in 1..=body.arg_count {
            set_tree(s, self.paths, self.paths.roots[i], true);
        }
    }
    fn statement_effect(&self, s: &mut BitSet, st: &Statement, _: Location) {
        init_effects(s, self.paths, st, true);
    }
    fn terminator_effect(&self, s: &mut BitSet, t: &Terminator, _: Location) {
        init_term(s, self.paths, t, true);
    }
    fn call_return_effect(&self, s: &mut BitSet, dest: &Place) {
        if !dest.has_deref() {
            set_tree(s, self.paths, self.paths.find(dest), true);
        }
    }
}

impl Analysis for MaybeUninit<'_> {
    type Domain = BitSet;
    const DIRECTION: Direction = Direction::Forward;
    fn bottom(&self, _: &Body) -> BitSet {
        BitSet::empty(self.paths.len())
    }
    fn initialize_start(&self, body: &Body, s: &mut BitSet) {
        // mọi thứ chưa khởi tạo trừ tham số
        for (k, (_, _, l)) in self.paths.nodes.iter().enumerate() {
            if l.index() == 0 || l.index() > body.arg_count {
                s.insert(k);
            }
        }
    }
    fn statement_effect(&self, s: &mut BitSet, st: &Statement, _: Location) {
        init_effects(s, self.paths, st, false);
    }
    fn terminator_effect(&self, s: &mut BitSet, t: &Terminator, _: Location) {
        init_term(s, self.paths, t, false);
    }
    fn call_return_effect(&self, s: &mut BitSet, dest: &Place) {
        if !dest.has_deref() {
            set_tree(s, self.paths, self.paths.find(dest), false);
        }
    }
}

// ======================= Liveness =======================

/// Liveness theo local (lùi, hợp): local "sống" nếu giá trị hiện tại có thể được đọc sau này.
/// Gán toàn phần / `StorageDead` / `StorageLive` / đích `Call` là "def"; đọc toán hạng, chỗ đi qua `Deref`, chỉ số,
/// `Ref`, `Len`, `Discriminant`, `FakeRead`, `Drop` (drop-use), và `_0` ở `Return` là "use".
pub struct Liveness;

fn place_uses(p: &Place, s: &mut BitSet, as_target: bool) {
    // chỉ số luôn là đọc
    for e in &p.projection {
        match e {
            PlaceElem::Index(l) => {
                s.insert(l.index());
            }
            PlaceElem::Subslice { from, to } => {
                s.insert(from.index());
                if let Some(t) = to {
                    s.insert(t.index());
                }
            }
            _ => {}
        }
    }
    if !as_target || p.has_deref() {
        // đọc chỗ, hoặc ghi qua tham chiếu (đọc chính tham chiếu)
        s.insert(p.local.index());
    }
}

fn operand_use(o: &Operand, s: &mut BitSet) {
    if let Operand::Copy(p) | Operand::Move(p) = o {
        place_uses(p, s, false);
    }
}

fn rvalue_uses(rv: &Rvalue, s: &mut BitSet) {
    match rv {
        Rvalue::Use(o)
        | Rvalue::Repeat(o, _)
        | Rvalue::Cast(o, _)
        | Rvalue::CastFits(o, _)
        | Rvalue::UnaryOp(_, o) => {
            operand_use(o, s);
        }
        Rvalue::BinaryOp(_, a, b) | Rvalue::CheckedBinaryOp(_, a, b) => {
            operand_use(a, s);
            operand_use(b, s);
        }
        Rvalue::Aggregate(_, v) => {
            for o in v {
                operand_use(o, s);
            }
        }
        Rvalue::Ref(_, _, p) | Rvalue::Len(p) | Rvalue::Discriminant(p) => place_uses(p, s, false),
    }
}

impl Analysis for Liveness {
    type Domain = BitSet;
    const DIRECTION: Direction = Direction::Backward;
    fn bottom(&self, body: &Body) -> BitSet {
        BitSet::empty(body.locals.len())
    }
    fn initialize_start(&self, _: &Body, _: &mut BitSet) {}
    fn statement_effect(&self, s: &mut BitSet, st: &Statement, _: Location) {
        match &st.kind {
            StatementKind::Assign(p, rv) => {
                if p.projection.is_empty() {
                    s.remove(p.local.index());
                }
                place_uses(p, s, true);
                rvalue_uses(rv, s);
            }
            StatementKind::StorageLive(l) | StatementKind::StorageDead(l) => {
                s.remove(l.index());
            }
            StatementKind::FakeRead(_, p) => place_uses(p, s, false),
            StatementKind::Nop => {}
        }
    }
    fn terminator_effect(&self, s: &mut BitSet, t: &Terminator, _: Location) {
        match &t.kind {
            TerminatorKind::Return => {
                s.insert(RETURN_PLACE.index());
            }
            TerminatorKind::SwitchInt { discr, .. } => operand_use(discr, s),
            TerminatorKind::Drop { place, .. } => place_uses(place, s, false),
            TerminatorKind::Call {
                callee, args, dest, ..
            } => {
                if dest.projection.is_empty() {
                    s.remove(dest.local.index());
                }
                place_uses(dest, s, true);
                if let Callee::Value(o) = callee {
                    operand_use(o, s);
                }
                for a in args {
                    operand_use(a, s);
                }
            }
            TerminatorKind::Assert { cond, .. } => operand_use(cond, s),
            TerminatorKind::Goto(_) | TerminatorKind::Unreachable => {}
        }
    }
}

// ======================= bất biến khởi tạo (ICE) =======================

/// "Chắc chắn ĐÃ GÁN" theo local (tiến, giao), BỎ QUA move (ADR-020 §9): một lần đọc mà local chưa chắc chắn được gán
/// là lỗi của bước hạ MIR (ICE), không phải lỗi người dùng (E0306 đã nghỉ). Move làm chỗ "chưa khởi tạo" là chuyện của
/// M6 (E0401), không phải ở đây.
pub struct DefinitelyAssigned;

impl Analysis for DefinitelyAssigned {
    type Domain = Dual;
    const DIRECTION: Direction = Direction::Forward;
    fn bottom(&self, body: &Body) -> Dual {
        Dual(BitSet::full(body.locals.len()))
    }
    fn initialize_start(&self, body: &Body, s: &mut Dual) {
        let mut b = BitSet::empty(body.locals.len());
        for i in 1..=body.arg_count {
            b.insert(i);
        }
        *s = Dual(b);
    }
    fn statement_effect(&self, s: &mut Dual, st: &Statement, _: Location) {
        match &st.kind {
            StatementKind::Assign(p, _) if p.projection.is_empty() => {
                s.0.insert(p.local.index());
            }
            StatementKind::StorageLive(l) | StatementKind::StorageDead(l) => {
                s.0.remove(l.index());
            }
            _ => {}
        }
    }
    fn terminator_effect(&self, _: &mut Dual, _: &Terminator, _: Location) {}
    fn call_return_effect(&self, s: &mut Dual, dest: &Place) {
        if dest.projection.is_empty() {
            s.0.insert(dest.local.index());
        }
    }
}

fn read_locals_of_place(p: &Place, out: &mut Vec<Local>) {
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

fn read_locals_operand(o: &Operand, out: &mut Vec<Local>) {
    if let Operand::Copy(p) | Operand::Move(p) = o {
        read_locals_of_place(p, out);
    }
}

/// Kiểm bất biến: mọi lần ĐỌC một local (toán hạng, chỗ được mượn/đo/so, chỉ số, ghi qua `*`, `_0` ở `Return`) xảy ra khi
/// local đã chắc chắn được gán. `Drop` được miễn (drop một chỗ có thể chưa khởi tạo là hợp lệ; elaboration xử lý ở M10).
pub fn check_definitely_assigned(body: &Body) -> Result<(), Vec<String>> {
    let a = DefinitelyAssigned;
    let r = crate::framework::solve(&a, body).map_err(|e| vec![e])?;
    let mut errs = Vec::new();
    for (bi, bb) in body.blocks.iter().enumerate() {
        let mut st = r.state[bi].clone();
        let check = |st: &Dual, reads: Vec<Local>, at: String, errs: &mut Vec<String>| {
            for l in reads {
                if !st.0.contains(l.index()) {
                    errs.push(format!("{at}: đọc _{} khi chưa chắc chắn được gán", l.0));
                }
            }
        };
        for (si, s) in bb.statements.iter().enumerate() {
            let mut reads = Vec::new();
            match &s.kind {
                StatementKind::Assign(p, rv) => {
                    match rv {
                        Rvalue::Use(o)
                        | Rvalue::Repeat(o, _)
                        | Rvalue::Cast(o, _)
                        | Rvalue::CastFits(o, _)
                        | Rvalue::UnaryOp(_, o) => {
                            read_locals_operand(o, &mut reads);
                        }
                        Rvalue::BinaryOp(_, x, y) | Rvalue::CheckedBinaryOp(_, x, y) => {
                            read_locals_operand(x, &mut reads);
                            read_locals_operand(y, &mut reads);
                        }
                        Rvalue::Aggregate(_, v) => {
                            for o in v {
                                read_locals_operand(o, &mut reads);
                            }
                        }
                        Rvalue::Ref(_, _, q) | Rvalue::Len(q) | Rvalue::Discriminant(q) => {
                            read_locals_of_place(q, &mut reads);
                        }
                    }
                    // ghi vào chỗ có phép chiếu: gốc phải đã có giá trị (ghi một phần / qua tham chiếu)
                    if !p.projection.is_empty() {
                        read_locals_of_place(p, &mut reads);
                    }
                }
                StatementKind::FakeRead(_, p) => read_locals_of_place(p, &mut reads),
                _ => {}
            }
            check(&st, reads, format!("bb{bi}[{si}]"), &mut errs);
            a.statement_effect(
                &mut st,
                s,
                Location {
                    block: BasicBlock(bi as u32),
                    statement: si,
                },
            );
        }
        if let Some(t) = &bb.terminator {
            let mut reads = Vec::new();
            match &t.kind {
                TerminatorKind::Return => reads.push(RETURN_PLACE),
                TerminatorKind::SwitchInt { discr, .. } => read_locals_operand(discr, &mut reads),
                TerminatorKind::Assert { cond, .. } => read_locals_operand(cond, &mut reads),
                TerminatorKind::Call {
                    callee, args, dest, ..
                } => {
                    if let Callee::Value(o) = callee {
                        read_locals_operand(o, &mut reads);
                    }
                    for x in args {
                        read_locals_operand(x, &mut reads);
                    }
                    if !dest.projection.is_empty() {
                        read_locals_of_place(dest, &mut reads);
                    }
                }
                TerminatorKind::Drop { .. }
                | TerminatorKind::Goto(_)
                | TerminatorKind::Unreachable => {}
            }
            check(&st, reads, format!("bb{bi}"), &mut errs);
        }
    }
    if errs.is_empty() { Ok(()) } else { Err(errs) }
}
