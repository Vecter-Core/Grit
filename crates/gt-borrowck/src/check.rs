//! Khoản vay hiệu lực, kiểm truy cập (E0402/E0403/E0404/E0405/E0406), dùng sau move và move bất hợp lệ (E0401/E0407).
use crate::solve::{Points, Values};
use gt_dataflow::{Analysis, BitSet, Direction, MaybeUninit, MovePaths, solve};
use gt_diag::{Code, Diagnostic};
use gt_mir::types::place_ty;
use gt_mir::*;
use gt_span::Span;
use gt_types::{Lang, Ty, TypeDb, TypesOutput};
use std::collections::BTreeSet;

// ======================= khoản vay trong phạm vi =======================

/// Tập khoản vay còn hiệu lực (tiến, hợp): sinh ở `Ref`; chết khi điểm hiện tại ngoài region của nó, khi gán đè local gốc
/// (các khoản vay đi QUA `*` của local đó), hoặc `StorageDead` của local gốc.
pub(crate) struct LoanScope<'a> {
    pub body: &'a Body,
    pub pts: &'a Points,
    pub values: &'a Values,
}

impl LoanScope<'_> {
    fn filter(&self, s: &mut BitSet, loc: Location) {
        let p = self.pts.idx(loc);
        let dead: Vec<usize> = s
            .iter()
            .filter(|b| {
                !self
                    .values
                    .contains_point(self.body.borrows[*b].region.0 as usize, p)
            })
            .collect();
        for b in dead {
            s.remove(b);
        }
    }

    fn kill_through_deref(&self, s: &mut BitSet, l: Local) {
        let dead: Vec<usize> = s
            .iter()
            .filter(|b| {
                let bp = &self.body.borrows[*b].place;
                bp.local == l && bp.has_deref()
            })
            .collect();
        for b in dead {
            s.remove(b);
        }
    }

    fn kill_all(&self, s: &mut BitSet, l: Local) {
        let dead: Vec<usize> = s
            .iter()
            .filter(|b| self.body.borrows[*b].place.local == l)
            .collect();
        for b in dead {
            s.remove(b);
        }
    }
}

impl Analysis for LoanScope<'_> {
    type Domain = BitSet;
    const DIRECTION: Direction = Direction::Forward;
    fn bottom(&self, body: &Body) -> BitSet {
        BitSet::empty(body.borrows.len())
    }
    fn initialize_start(&self, _: &Body, _: &mut BitSet) {}
    fn statement_effect(&self, s: &mut BitSet, st: &Statement, loc: Location) {
        self.filter(s, loc);
        match &st.kind {
            StatementKind::Assign(dst, rv) => {
                if dst.projection.is_empty() {
                    self.kill_through_deref(s, dst.local);
                }
                if let Rvalue::Ref(b, ..) = rv {
                    s.insert(b.index());
                }
            }
            StatementKind::StorageDead(l) => self.kill_all(s, *l),
            _ => {}
        }
    }
    fn terminator_effect(&self, s: &mut BitSet, _: &Terminator, loc: Location) {
        self.filter(s, loc);
    }
    fn call_return_effect(&self, s: &mut BitSet, dest: &Place) {
        if dest.projection.is_empty() {
            self.kill_through_deref(s, dest.local);
        }
    }
}

// ======================= truy cập =======================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Read,
    Move,
    Write,
    SharedBorrow,
    UniqueBorrow,
    StorageDead,
    Drop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Depth {
    Deep,
    Shallow,
}

#[derive(Clone, Debug)]
struct Access {
    place: Place,
    kind: Kind,
    depth: Depth,
}

fn op_access(o: &Operand, out: &mut Vec<Access>) {
    match o {
        Operand::Copy(p) => out.push(Access {
            place: p.clone(),
            kind: Kind::Read,
            depth: Depth::Deep,
        }),
        Operand::Move(p) => out.push(Access {
            place: p.clone(),
            kind: Kind::Move,
            depth: Depth::Deep,
        }),
        Operand::Const(_) => {}
    }
}

fn stmt_accesses(body: &Body, s: &Statement) -> Vec<Access> {
    let mut v = Vec::new();
    match &s.kind {
        StatementKind::Assign(dst, rv) => {
            match rv {
                Rvalue::Use(o)
                | Rvalue::Repeat(o, _)
                | Rvalue::Cast(o, _)
                | Rvalue::CastFits(o, _)
                | Rvalue::UnaryOp(_, o) => op_access(o, &mut v),
                Rvalue::BinaryOp(_, a, b) | Rvalue::CheckedBinaryOp(_, a, b) => {
                    op_access(a, &mut v);
                    op_access(b, &mut v);
                }
                Rvalue::Aggregate(_, ops) => {
                    for o in ops {
                        op_access(o, &mut v);
                    }
                }
                Rvalue::Ref(_, k, p) => v.push(Access {
                    place: p.clone(),
                    kind: if *k == BorrowKind::Unique {
                        Kind::UniqueBorrow
                    } else {
                        Kind::SharedBorrow
                    },
                    depth: Depth::Deep,
                }),
                // độ dài mảng cố định là hằng theo kiểu: không đọc chỗ (như Rust)
                Rvalue::Len(p) if matches!(place_ty(body, p), Some(Ty::Array(..))) => {}
                Rvalue::Len(p) | Rvalue::Discriminant(p) => v.push(Access {
                    place: p.clone(),
                    kind: Kind::Read,
                    depth: Depth::Deep,
                }),
            }
            v.push(Access {
                place: dst.clone(),
                kind: Kind::Write,
                depth: Depth::Shallow,
            });
        }
        StatementKind::StorageDead(l) => v.push(Access {
            place: Place::local(*l),
            kind: Kind::StorageDead,
            depth: Depth::Shallow,
        }),
        StatementKind::FakeRead(_, p) => v.push(Access {
            place: p.clone(),
            kind: Kind::Read,
            depth: Depth::Deep,
        }),
        StatementKind::StorageLive(_) | StatementKind::Nop => {}
    }
    v
}

fn term_accesses(t: &Terminator) -> Vec<Access> {
    let mut v = Vec::new();
    match &t.kind {
        TerminatorKind::SwitchInt { discr, .. } => op_access(discr, &mut v),
        TerminatorKind::Assert { cond, .. } => op_access(cond, &mut v),
        TerminatorKind::Call {
            callee, args, dest, ..
        } => {
            if let Callee::Value(o) = callee {
                op_access(o, &mut v);
            }
            for a in args {
                op_access(a, &mut v);
            }
            v.push(Access {
                place: dest.clone(),
                kind: Kind::Write,
                depth: Depth::Shallow,
            });
        }
        TerminatorKind::Drop { place, .. } => v.push(Access {
            place: place.clone(),
            kind: Kind::Drop,
            depth: Depth::Shallow,
        }),
        TerminatorKind::Goto(_) | TerminatorKind::Return | TerminatorKind::Unreachable => {}
    }
    v
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rel {
    Disjoint,
    Equal,
    /// chỗ truy cập là tiền tố (ngắn hơn) của chỗ bị mượn: khoản vay nằm TRONG chỗ truy cập
    LoanInside,
    /// chỗ bị mượn là tiền tố của chỗ truy cập: truy cập nằm TRONG khoản vay
    AccessInside,
}

fn rel(acc: &Place, loan: &Place) -> Rel {
    if acc.local != loan.local {
        return Rel::Disjoint;
    }
    let n = acc.projection.len().min(loan.projection.len());
    for i in 0..n {
        match (&acc.projection[i], &loan.projection[i]) {
            (PlaceElem::Field(x, _), PlaceElem::Field(y, _)) if x != y => return Rel::Disjoint,
            (PlaceElem::Downcast(x), PlaceElem::Downcast(y)) if x != y => return Rel::Disjoint,
            _ => {}
        }
    }
    match acc.projection.len().cmp(&loan.projection.len()) {
        std::cmp::Ordering::Equal => Rel::Equal,
        std::cmp::Ordering::Less => Rel::LoanInside,
        std::cmp::Ordering::Greater => Rel::AccessInside,
    }
}

fn conflicts(acc: &Access, loan: &BorrowData) -> bool {
    let r = rel(&acc.place, &loan.place);
    if r == Rel::Disjoint {
        return false;
    }
    let weak = matches!(acc.kind, Kind::Read | Kind::SharedBorrow);
    if weak && loan.kind != BorrowKind::Unique {
        return false;
    }
    if acc.depth == Depth::Shallow && r == Rel::LoanInside {
        // truy cập nông vào `p` không đụng dữ liệu SAU một `*` của `p`
        let extra = &loan.place.projection[acc.place.projection.len()..];
        if extra.iter().any(|e| matches!(e, PlaceElem::Deref)) {
            return false;
        }
    }
    true
}

// ======================= bộ kiểm =======================

/// Mã chẩn đoán từ chuỗi. Chuỗi sai định dạng KHÔNG được làm chẩn đoán biến mất (fail-open): lùi về `E9901` (lỗi nội bộ ⇒ cổng từ chối). FAIL_OPEN_AUDIT B1.
fn diag_code(code: &str) -> Code {
    Code::parse(code).unwrap_or_else(|_| Code::parse("E9901").expect("E9901 hợp lệ"))
}

pub(crate) struct Checker<'a> {
    pub body: &'a Body,
    pub db: &'a TypeDb,
    pub file: String,
    pub diags: Vec<Diagnostic>,
    seen: BTreeSet<(String, usize, u32)>,
}

impl<'a> Checker<'a> {
    pub fn new(t: &'a TypesOutput, body: &'a Body) -> Self {
        Checker {
            body,
            db: &t.db,
            file: t.file_name(body.node),
            diags: Vec::new(),
            seen: BTreeSet::new(),
        }
    }

    pub fn push(
        &mut self,
        code: &str,
        span: Span,
        msg: String,
        key: (usize, u32),
    ) -> Option<usize> {
        if !self.seen.insert((code.to_string(), key.0, key.1)) {
            return None;
        }
        let c = diag_code(code);
        self.diags.push(Diagnostic::new(c, &self.file, span, msg));
        Some(self.diags.len() - 1)
    }

    pub fn label(&mut self, idx: Option<usize>, span: Span, text: &str) {
        if let Some(i) = idx {
            let f = self.file.clone();
            let d = self.diags.remove(i);
            self.diags.insert(i, d.with_label(&f, span, text));
        }
    }

    fn lname(&self, l: Local) -> String {
        match &self.body.locals[l.index()].kind {
            LocalKind::User { name, .. } => format!("`{name}`"),
            LocalKind::Arg(Some(n)) => format!("`{n}`"),
            LocalKind::Arg(None) => "`self`".to_string(),
            LocalKind::Temp => "một giá trị tạm".to_string(),
            LocalKind::Return => "giá trị trả về".to_string(),
        }
    }

    fn is_arena(&self, p: &Place) -> bool {
        let Some(&ad) = self.db.lang.get(&Lang::Arena) else {
            return false;
        };
        matches!(place_ty(self.body, p), Some(Ty::Adt(d, _)) if d == ad)
    }

    fn has_own_drop(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Adt(d, _) => {
                self.db.adts.get(d).and_then(|a| a.lang) == Some(Lang::Arena)
                    || self.db.has_drop_impl(ty)
            }
            _ => false,
        }
    }

    // ---------------- E0401 / E0407 ----------------

    /// (nút move path, có khớp đúng chỗ?) — `exact` nếu chỗ không sâu hơn nút tìm được.
    fn resolve_path(mp: &MovePaths, p: &Place) -> (usize, bool) {
        let mut cur = mp.roots[p.local.index()];
        let mut used = 0;
        for e in &p.projection {
            if !matches!(e, PlaceElem::Field(..) | PlaceElem::Downcast(_)) {
                break;
            }
            match mp
                .nodes
                .iter()
                .position(|(par, el, _)| *par == Some(cur) && el.as_ref() == Some(e))
            {
                Some(c) => {
                    cur = c;
                    used += 1;
                }
                None => break,
            }
        }
        (cur, used == p.projection.len())
    }

    fn maybe_uninit(mp: &MovePaths, us: &BitSet, p: &Place) -> bool {
        let (node, exact) = Self::resolve_path(mp, p);
        if exact {
            mp.subtree(node).iter().any(|n| us.contains(*n))
        } else {
            us.contains(node)
        }
    }

    fn check_uses(&mut self, mp: &MovePaths, us: &BitSet, accs: &[Access], src: SourceInfo) {
        for a in accs {
            let is_use = !matches!(a.kind, Kind::StorageDead | Kind::Drop);
            if !is_use {
                continue;
            }
            if a.kind == Kind::Write {
                // ghi QUA một tham chiếu/chỉ số: bản thân con trỏ/mảng phải còn nguyên (`*a ← v` sau `b ≔ a` là dùng-sau-move;
                // lỗ hổng tìm thấy khi săn soundness: hai `&!` cùng ghi được vào một chỗ)
                let last_indirection = a.place.projection.iter().rposition(|e| {
                    matches!(
                        e,
                        PlaceElem::Deref | PlaceElem::Index(_) | PlaceElem::Subslice { .. }
                    )
                });
                if let Some(k) = last_indirection {
                    let mut base = a.place.clone();
                    base.projection.truncate(k);
                    if Self::maybe_uninit(mp, us, &base) {
                        self.report_use_after_move(&a.place, src, "ghi qua");
                        continue;
                    }
                }
                // ghi vào trường: phần cha phải còn nguyên (không bị move cả khối)
                if let Some(PlaceElem::Field(..) | PlaceElem::Downcast(_)) =
                    a.place.projection.last()
                    && !a.place.has_deref()
                {
                    let mut parent = a.place.clone();
                    parent.projection.pop();
                    let (node, exact) = Self::resolve_path(mp, &parent);
                    if exact && us.contains(node) {
                        self.report_use_after_move(&a.place, src, "ghi vào một phần của");
                    }
                }
                continue;
            }
            if Self::maybe_uninit(mp, us, &a.place) {
                self.report_use_after_move(&a.place, src, "dùng");
            }
        }
    }

    fn report_use_after_move(&mut self, p: &Place, src: SourceInfo, what: &str) {
        let name = self.lname(p.local);
        let site = self.find_move_site(p.local);
        let idx = self.push(
            "E0401",
            src.span,
            format!(
                "{what} {name} sau khi giá trị đã bị move (có thể chưa khởi tạo trên một số đường)"
            ),
            (p.local.index(), src.span.start),
        );
        if let Some(s) = site {
            self.label(idx, s, "giá trị bị move ở đây");
        }
    }

    /// Chỗ move đầu tiên (theo thứ tự khối) của local `l`, để đặt nhãn.
    fn find_move_site(&self, l: Local) -> Option<Span> {
        for bb in &self.body.blocks {
            for s in &bb.statements {
                let mut v = Vec::new();
                if let StatementKind::Assign(_, rv) = &s.kind {
                    match rv {
                        Rvalue::Use(o) | Rvalue::Cast(o, _) | Rvalue::UnaryOp(_, o) => {
                            op_access(o, &mut v);
                        }
                        Rvalue::Aggregate(_, ops) => ops.iter().for_each(|o| op_access(o, &mut v)),
                        _ => {}
                    }
                }
                if v.iter().any(|a| a.kind == Kind::Move && a.place.local == l) {
                    return Some(s.source.span);
                }
            }
            if let Some(Terminator {
                kind:
                    TerminatorKind::Call {
                        args, arg_spans, ..
                    },
                source,
            }) = &bb.terminator
            {
                for (i, a) in args.iter().enumerate() {
                    if matches!(a, Operand::Move(p) if p.local == l) {
                        return Some(arg_spans.get(i).copied().unwrap_or(source.span));
                    }
                }
            }
        }
        None
    }

    fn check_moves_out(&mut self, accs: &[Access], src: SourceInfo) {
        for a in accs {
            if a.kind != Kind::Move || a.place.projection.is_empty() {
                continue;
            }
            let name = self.lname(a.place.local);
            // qua `*`, chỉ số, lát
            if a.place.projection.iter().any(|e| {
                matches!(
                    e,
                    PlaceElem::Deref | PlaceElem::Index(_) | PlaceElem::Subslice { .. }
                )
            }) {
                self.push(
                    "E0407",
                    src.span,
                    format!("không thể move ra khỏi {name}: chỗ này nằm sau một tham chiếu/chỉ số/lát (chỉ được copy hoặc mượn)"),
                    (a.place.local.index(), src.span.start),
                );
                continue;
            }
            // trường của giá trị có Drop / Arena
            for k in 0..a.place.projection.len() {
                let prefix = Place {
                    local: a.place.local,
                    projection: a.place.projection[..k].to_vec(),
                };
                if let Some(ty) = place_ty(self.body, &prefix)
                    && self.has_own_drop(&ty)
                {
                    self.push(
                        "E0407",
                        src.span,
                        format!("không thể move một trường ra khỏi {name}: kiểu của nó có `Drop` (hoặc là `Arena`), cần drop nguyên giá trị"),
                        (a.place.local.index(), src.span.start),
                    );
                    break;
                }
            }
        }
    }

    // ---------------- xung đột với khoản vay ----------------

    fn check_conflicts(&mut self, ls: &BitSet, accs: &[Access], src: SourceInfo) {
        for a in accs {
            for bi in ls.iter() {
                let loan = &self.body.borrows[bi];
                if !conflicts(a, loan) {
                    continue;
                }
                self.report_conflict(a, bi, src);
            }
        }
    }

    fn report_conflict(&mut self, a: &Access, bi: usize, src: SourceInfo) {
        let loan = self.body.borrows[bi].clone();
        let arena = self.is_arena(&loan.place);
        let lname = self.lname(loan.place.local);
        let lk = if loan.kind == BorrowKind::Unique {
            "mượn duy nhất `&!`"
        } else {
            "mượn chung `&`"
        };
        let (code, msg): (&str, String) = match a.kind {
            Kind::UniqueBorrow if arena && loan.kind == BorrowKind::Shared => (
                "E0405",
                format!(
                    "tạo `&!` trên Arena {lname} (reset/clear) trong khi còn tham chiếu cấp từ phiên bản hiện tại: các tham chiếu đó sẽ bị vô hiệu"
                ),
            ),
            Kind::UniqueBorrow if loan.kind == BorrowKind::Unique => (
                "E0402",
                format!(
                    "hai khoản mượn duy nhất `&!` chồng nhau trên {lname}: khoản mượn trước còn được dùng về sau"
                ),
            ),
            Kind::UniqueBorrow => (
                "E0403",
                format!(
                    "mượn duy nhất `&!` {lname} trong khi còn khoản mượn chung `&` đang được dùng về sau"
                ),
            ),
            Kind::SharedBorrow | Kind::Read => (
                "E0403",
                format!("dùng/mượn chung {lname} trong khi còn khoản {lk} đang được dùng về sau"),
            ),
            Kind::Write | Kind::Move if arena => (
                "E0405",
                format!(
                    "gán/move Arena {lname} làm vô hiệu các tham chiếu đã cấp từ nó, mà chúng còn được dùng về sau"
                ),
            ),
            Kind::Write => (
                "E0406",
                format!(
                    "gán vào {lname} trong khi nó đang bị mượn ({lk}) và khoản mượn còn được dùng về sau"
                ),
            ),
            Kind::Move => (
                "E0406",
                format!(
                    "move {lname} ra trong khi nó đang bị mượn ({lk}) và khoản mượn còn được dùng về sau"
                ),
            ),
            Kind::StorageDead | Kind::Drop => {
                if arena {
                    (
                        "E0405",
                        format!(
                            "tham chiếu thoát khỏi Arena {lname}: Arena bị drop/hết phạm vi trong khi tham chiếu cấp từ nó còn được dùng"
                        ),
                    )
                } else {
                    (
                        "E0404",
                        format!(
                            "{lname} không sống đủ lâu: bị drop/hết phạm vi trong khi còn khoản {lk} được dùng về sau"
                        ),
                    )
                }
            }
        };
        let loan_span = loan.source.span;
        let key = if matches!(a.kind, Kind::StorageDead | Kind::Drop) {
            (bi, 0)
        } else {
            (bi, src.span.start)
        };
        let idx = self.push(code, src.span, msg, key);
        self.label(idx, loan_span, "khoản mượn bắt đầu ở đây");
    }

    /// `Return`: khoản vay còn hiệu lực trên một chỗ THUỘC KHUNG (không qua `*`) sống lâu hơn khung.
    fn check_return(&mut self, ls: &BitSet, src: SourceInfo) {
        for bi in ls.iter() {
            let loan = self.body.borrows[bi].clone();
            if loan.place.has_deref() || loan.place.local == RETURN_PLACE {
                continue;
            }
            let arena = self.is_arena(&loan.place);
            let name = self.lname(loan.place.local);
            let (code, msg) = if arena {
                (
                    "E0405",
                    format!(
                        "tham chiếu thoát khỏi Arena {name}: được trả về/lưu ra ngoài nhưng Arena chết cùng khung hàm"
                    ),
                )
            } else {
                (
                    "E0404",
                    format!(
                        "trả về (hoặc lưu ra ngoài) tham chiếu tới {name}, thuộc khung hàm và chết khi hàm kết thúc"
                    ),
                )
            };
            let idx = self.push(code, src.span, msg, (bi, 0));
            self.label(idx, loan.source.span, "khoản mượn bắt đầu ở đây");
        }
    }

    // ---------------- vòng chính ----------------

    pub fn run(&mut self, pts: &Points, values: &Values) -> Result<(), String> {
        let body = self.body;
        let loans = LoanScope { body, pts, values };
        let lres = solve(&loans, body)?;
        let mp = MovePaths::build(body);
        let un = MaybeUninit { paths: &mp };
        let ures = solve(&un, body)?;
        for (bi, bb) in body.blocks.iter().enumerate() {
            let b = BasicBlock(bi as u32);
            let mut ls = lres.state[bi].clone();
            let mut us = ures.state[bi].clone();
            for (si, s) in bb.statements.iter().enumerate() {
                let loc = Location {
                    block: b,
                    statement: si,
                };
                // lọc theo điểm hiện tại trước khi kiểm
                let p = pts.idx(loc);
                let dead: Vec<usize> = ls
                    .iter()
                    .filter(|x| !values.contains_point(body.borrows[*x].region.0 as usize, p))
                    .collect();
                for x in dead {
                    ls.remove(x);
                }
                let accs = stmt_accesses(body, s);
                self.check_uses(&mp, &us, &accs, s.source);
                self.check_moves_out(&accs, s.source);
                self.check_conflicts(&ls, &accs, s.source);
                loans.statement_effect(&mut ls, s, loc);
                un.statement_effect(&mut us, s, loc);
            }
            let Some(t) = &bb.terminator else { continue };
            let loc = Location {
                block: b,
                statement: bb.statements.len(),
            };
            let p = pts.idx(loc);
            let dead: Vec<usize> = ls
                .iter()
                .filter(|x| !values.contains_point(body.borrows[*x].region.0 as usize, p))
                .collect();
            for x in dead {
                ls.remove(x);
            }
            let accs = term_accesses(t);
            self.check_uses(&mp, &us, &accs, t.source);
            self.check_moves_out(&accs, t.source);
            self.check_conflicts(&ls, &accs, t.source);
            if t.kind == TerminatorKind::Return {
                self.check_return(&ls, t.source);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(dead_code, clippy::unwrap_used)]
#[path = "../../gt-types/tests/common/mod.rs"]
mod tc;

#[cfg(test)]
mod tests {
    use super::{Checker, diag_code, tc};

    #[test]
    fn malformed_diagnostic_code_falls_back_to_the_internal_error_instead_of_vanishing() {
        assert_eq!(diag_code("E0401").as_str(), "E0401");
        for bad in ["", "E04", "X0401", "E04O1", "e0401", "E04011"] {
            assert_eq!(diag_code(bad).as_str(), "E9901", "{bad:?}");
        }
    }

    /// Đường thật `Checker::push`: mã sai định dạng PHẢI để lại một chẩn đoán (E9901), không được biến mất (FAIL_OPEN_AUDIT B1).
    #[test]
    fn push_with_a_malformed_code_still_records_a_diagnostic() {
        let t = tc::run(&[("main.gt", "ƒ f() → u32 ! ∅ { 1 }")]);
        let m = gt_mir::lower_program(&t);
        let body = &m.bodies[0];
        let mut ck = Checker::new(&t, body);
        let idx = ck.push("E04", body.span, "thử".to_string(), (0, 0));
        assert!(idx.is_some(), "push trả None ⇒ chẩn đoán bị nuốt");
        assert_eq!(ck.diags.len(), 1);
        assert_eq!(ck.diags[0].code.as_str(), "E9901");
    }
}
