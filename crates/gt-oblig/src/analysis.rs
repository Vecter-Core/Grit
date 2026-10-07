//! Phân tích luồng trên MIR (ADR-025 §2–§5): khoảng + zone, widening/narrowing tại đầu vòng lặp, sinh Obligation tại mỗi `Assert`
//! rồi nhờ `Prover` giải. KHÔNG tạo check mới: chỉ quyết định XOÁ hay GIỮ `Assert` của M5.
//!
//! Soundness (tóm tắt, chi tiết ở ADR-025 §3 và §12):
//! * mọi local từng bị mượn `&!` (bảng `Body.borrows` của M5) KHÔNG được theo dõi — ghi qua bí danh không thể làm sai quan hệ;
//! * một `Def` (định nghĩa của biến tạm) chỉ được dùng khi mọi biến nó tham chiếu chưa bị gán lại (nhãn phiên bản theo VỊ TRÍ);
//! * quan hệ zone chỉ được tạo/giữ qua phép gán tuyến tính đơn vị mà kết quả toán học vừa kiểu (đường tràn đã abort);
//! * sau một `Assert` thành công, trạng thái được thu hẹp bằng chính mục tiêu (đường thất bại abort tức thì, D3).
use crate::env::Env;
use crate::int::I256;
use crate::interval::{B, Interval, Tri, Ty};
use crate::ir::{
    Budget, Checkability, ObKind, Obligation, Prop, Prover, Term, VarDecl, VarId, Verdict,
};
use gt_mir::{
    AssertKind, BasicBlock, BinOp, Body, BorrowKind, Const, Local, LocalKind, Operand, PlaceElem,
    Rvalue, Statement, StatementKind, Terminator, TerminatorKind, UnOp,
};
use gt_types::Ty as GTy;
use std::collections::{BTreeMap, BTreeSet};

/// Số biến tối đa được cấp zone mỗi hàm (ADR-025 §12): vượt thì giữ các biến đầu theo thứ tự cố định và phát GHI CHÚ N0605.
pub const ZONE_MAX_VARS: usize = 24;
/// Trần số lần xử lý khối (D20, đo bằng BƯỚC): vượt ⇒ bỏ mọi quan hệ và chạy lại; vẫn vượt ⇒ mọi nghĩa vụ là `Unknown`.
pub const MAX_BLOCK_STEPS: u64 = 400_000;

/// Kết cục của một nghĩa vụ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Discharged {
        prover: &'static str,
        cert: String,
    },
    /// chắc chắn sai trên mọi đường tới được
    DefinitelyFalse,
    /// giữ `Assert` (kiểm được lúc chạy) + ghi chú
    Lowered {
        note: &'static str,
    },
    /// không kiểm được lúc chạy mà chưa giải (D25)
    Rejected {
        why: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Judged {
    pub node: gt_ast::NodeId,
    pub ob: Obligation,
    pub status: Status,
}

#[derive(Clone, Debug, Default)]
pub struct BodyReport {
    pub key: String,
    pub judged: Vec<Judged>,
    /// khối có `Assert` được xoá (chứng minh được)
    pub discharged_blocks: BTreeSet<u32>,
    /// biến zone bị bỏ do vượt `ZONE_MAX_VARS` (N0605)
    pub zone_dropped: Vec<String>,
    /// hết ngân sách bước: quan hệ bị bỏ (N0605)
    pub budget_exhausted: bool,
    /// ICE: bộ kiểm hậu-điểm-cố-định thất bại
    pub ice: Option<String>,
    /// narrowing bị bỏ vì không qua kiểm hậu-điểm-cố-định
    pub narrowing_rejected: bool,
}

fn ity(t: &GTy) -> Option<Ty> {
    match t {
        GTy::Int(it) => Some(Ty::new(it.bits(), it.signed())),
        _ => None,
    }
}

fn i256_of_bits(bits: u128, ty: gt_types::IntTy) -> I256 {
    // `Const::Int.bits` là mẫu bit bù hai đã cắt theo độ rộng
    if ty.signed() {
        let w = ty.bits();
        if w == 128 {
            I256::from_i128(bits as i128)
        } else if (bits >> (w - 1)) & 1 == 1 {
            I256::from_i128(bits as i128 - (1i128 << w))
        } else {
            I256::from_u128(bits)
        }
    } else {
        I256::from_u128(bits)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Def {
    Int { t: Term },
    Bool { p: Prop },
    Chk { res: Term, flag: Prop },
}

impl Def {
    /// local mà định nghĩa tham chiếu (qua biến, nguyên tử độ dài hay nguyên tử bool)
    fn locals(&self, cx: &Ctx<'_>) -> Vec<usize> {
        let mut vs = Vec::new();
        let mut at = Vec::new();
        match self {
            Def::Int { t } => t.vars(&mut vs),
            Def::Bool { p } => p.vars(&mut vs, &mut at),
            Def::Chk { res, flag } => {
                res.vars(&mut vs);
                flag.vars(&mut vs, &mut at);
            }
        }
        vs.extend(at);
        vs.iter().filter_map(|v| cx.stamp_local(*v)).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AState {
    env: Env,
    /// `defs[l]` = biểu thức toán học bằng giá trị HIỆN TẠI của `l`; bị xoá ngay khi một local nó tham chiếu bị gán lại
    defs: BTreeMap<usize, Def>,
}

impl AState {
    fn merge_meta(&self, o: &AState, env: Env) -> AState {
        // một định nghĩa đúng ở CẢ HAI trạng thái thì đúng ở trạng thái hợp
        let defs = self
            .defs
            .iter()
            .filter(|(k, d)| o.defs.get(k) == Some(d))
            .map(|(k, d)| (*k, d.clone()))
            .collect();
        AState { env, defs }
    }

    fn join(&self, o: &AState) -> AState {
        if self.env.is_dead() {
            return o.clone();
        }
        if o.env.is_dead() {
            return self.clone();
        }
        self.merge_meta(o, self.env.join(&o.env))
    }

    fn widen(&self, new: &AState, th: &[I256]) -> AState {
        if self.env.is_dead() {
            return new.clone();
        }
        if new.env.is_dead() {
            return self.clone();
        }
        self.merge_meta(new, self.env.widen(&new.env, th))
    }
}

struct Ctx<'a> {
    body: &'a Body,
    nl: usize,
    itys: Vec<Option<Ty>>,
    /// bị mượn `&!` ở đâu đó ⇒ không theo dõi
    aliased: Vec<bool>,
    zvars: Vec<VarId>,
    zone_dropped: Vec<String>,
    slice_atom: BTreeMap<usize, VarId>,
    array_len: BTreeMap<usize, u64>,
    thresholds: Vec<I256>,
    rpo: Vec<BasicBlock>,
    rpo_pos: Vec<usize>,
    len_callee: BTreeSet<gt_ast::NodeId>,
}

fn loc_id(block: usize, stmt: usize) -> u32 {
    (block as u32)
        .saturating_mul(2048)
        .saturating_add(stmt as u32 + 1)
        .min(u32::MAX - 1)
}

impl<'a> Ctx<'a> {
    fn new(body: &'a Body, keys: &BTreeMap<gt_ast::NodeId, String>, use_zone: bool) -> Ctx<'a> {
        let nl = body.locals.len();
        let itys: Vec<Option<Ty>> = body.locals.iter().map(|d| ity(&d.ty)).collect();
        let mut aliased = vec![false; nl];
        for b in &body.borrows {
            if b.kind == BorrowKind::Unique {
                aliased[b.place.local.index()] = true;
            }
        }
        // địa chỉ của local có thể lộ ra qua con trỏ thô/hàm; MIR M5 chỉ lộ qua `Ref`: mọi `Ref` Unique đã ở trên
        let mut slice_atom = BTreeMap::new();
        let mut array_len = BTreeMap::new();
        // số lần gán tĩnh của mỗi local (Assign vào local trần + đích của Call); tham số = 0 lần gán thêm
        let mut assigns = vec![0u32; nl];
        let mut reborrow: BTreeMap<usize, usize> = BTreeMap::new();
        for bb in &body.blocks {
            for st in &bb.statements {
                if let StatementKind::Assign(p, rv) = &st.kind {
                    assigns[p.local.index()] += 1;
                    if p.projection.is_empty() {
                        match rv {
                            Rvalue::Ref(_, _, q)
                                if matches!(q.projection.as_slice(), [PlaceElem::Deref]) =>
                            {
                                reborrow.insert(p.local.index(), q.local.index());
                            }
                            Rvalue::Use(Operand::Copy(q) | Operand::Move(q))
                                if q.projection.is_empty() =>
                            {
                                reborrow.insert(p.local.index(), q.local.index());
                            }
                            _ => {}
                        }
                    }
                }
            }
            if let Some(Terminator {
                kind: TerminatorKind::Call { dest, .. },
                ..
            }) = &bb.terminator
            {
                assigns[dest.local.index()] += 1;
            }
        }
        let is_slice_ref = |i: usize| matches!(&body.locals[i].ty, GTy::Ref(_, inner) if matches!(**inner, GTy::Slice(_)));
        let is_arg = |i: usize| matches!(body.locals[i].kind, LocalKind::Arg(_));
        // gốc của một tham chiếu lát: đi theo chuỗi tái mượn chỉ khi mỗi mắt xích được gán đúng một lần và gốc là tham số KHÔNG BAO GIỜ bị gán lại
        let root_of = |i: usize| -> usize {
            let mut cur = i;
            for _ in 0..nl {
                match reborrow.get(&cur) {
                    Some(&r) if assigns[cur] == 1 && is_slice_ref(r) => cur = r,
                    _ => break,
                }
            }
            if cur != i && !(is_arg(cur) && assigns[cur] == 0) {
                i
            } else {
                cur
            }
        };
        for (i, d) in body.locals.iter().enumerate() {
            match &d.ty {
                GTy::Ref(_, inner) if matches!(**inner, GTy::Slice(_)) => {
                    slice_atom.insert(i, (nl + root_of(i)) as VarId);
                }
                GTy::Ref(_, inner) => {
                    if let GTy::Array(_, n) = &**inner {
                        array_len.insert(i, *n);
                    }
                }
                GTy::Array(_, n) => {
                    array_len.insert(i, *n);
                }
                _ => {}
            }
        }
        // chọn biến zone: tham số, biến người dùng (int, theo dõi được), rồi nguyên tử độ dài; theo thứ tự chỉ số
        let mut cand: Vec<(VarId, String)> = Vec::new();
        for (i, d) in body.locals.iter().enumerate() {
            let is_named = matches!(d.kind, LocalKind::Arg(_) | LocalKind::User { .. });
            if is_named && itys[i].is_some() && !aliased[i] {
                cand.push((i as VarId, Self::lname(body, i)));
            }
        }
        let mut seen_atoms = BTreeSet::new();
        for (l, a) in &slice_atom {
            if seen_atoms.insert(*a) {
                cand.push((*a, format!("len({})", Self::lname(body, *a as usize - nl))));
            }
            let _ = l;
        }
        let mut zvars = Vec::new();
        let mut zone_dropped = Vec::new();
        for (v, n) in cand {
            if use_zone && zvars.len() < ZONE_MAX_VARS {
                zvars.push(v);
            } else if use_zone {
                zone_dropped.push(n);
            }
        }
        // ngưỡng widening: hằng xuất hiện trong thân + 0, ±1 + cận kiểu
        let mut th: BTreeSet<I256> = BTreeSet::new();
        for z in [-1i128, 0, 1] {
            th.insert(I256::from_i128(z));
        }
        for t in itys.iter().flatten().copied() {
            th.insert(t.min());
            th.insert(t.max());
        }
        for bb in &body.blocks {
            let mut ops: Vec<&Operand> = Vec::new();
            for s in &bb.statements {
                if let StatementKind::Assign(_, rv) = &s.kind {
                    match rv {
                        Rvalue::Use(o) => ops.push(o),
                        Rvalue::BinaryOp(_, a, b) | Rvalue::CheckedBinaryOp(_, a, b) => {
                            ops.push(a);
                            ops.push(b);
                        }
                        _ => {}
                    }
                }
            }
            for o in ops {
                if let Operand::Const(Const::Int { bits, ty }) = o {
                    let c = i256_of_bits(*bits, *ty);
                    th.insert(c);
                    if let Some(p) = c.checked_add(I256::from_i128(1)) {
                        th.insert(p);
                    }
                    if let Some(m) = c.checked_sub(I256::from_i128(1)) {
                        th.insert(m);
                    }
                }
            }
        }
        let rpo = gt_dataflow_rpo(body);
        let mut rpo_pos = vec![usize::MAX; body.blocks.len()];
        for (i, b) in rpo.iter().enumerate() {
            rpo_pos[b.index()] = i;
        }
        Ctx {
            body,
            nl,
            itys,
            aliased,
            zvars,
            zone_dropped,
            slice_atom,
            array_len,
            thresholds: th.into_iter().collect(),
            rpo,
            rpo_pos,
            len_callee: keys
                .iter()
                .filter(|(_, k)| k.as_str() == "$prelude::[T0]::len")
                .map(|(n, _)| *n)
                .collect(),
        }
    }

    fn lname(body: &Body, i: usize) -> String {
        match &body.locals[i].kind {
            LocalKind::User { name, .. } => name.clone(),
            LocalKind::Arg(Some(n)) => n.clone(),
            LocalKind::Arg(None) => "self".into(),
            LocalKind::Temp => format!("_{i}"),
            LocalKind::Return => "_0".into(),
        }
    }

    fn var_name(&self, v: VarId) -> String {
        let v = v as usize;
        if v < self.nl {
            Self::lname(self.body, v)
        } else if v < 2 * self.nl {
            format!("len({})", Self::lname(self.body, v - self.nl))
        } else {
            format!("a{v}")
        }
    }

    fn tracked(&self, l: usize) -> bool {
        self.itys[l].is_some() && !self.aliased[l]
    }

    fn bool_atom(&self, l: usize) -> VarId {
        (2 * self.nl + l) as VarId
    }

    /// local mà một biến (hoặc nguyên tử độ dài) phụ thuộc, để đóng dấu phiên bản.
    fn stamp_local(&self, v: VarId) -> Option<usize> {
        let v = v as usize;
        if v < self.nl {
            Some(v)
        } else if v < 2 * self.nl {
            Some(v - self.nl)
        } else if v < 3 * self.nl {
            Some(v - 2 * self.nl)
        } else {
            None
        }
    }

    /// xoá mọi định nghĩa tham chiếu tới `l` (vừa bị gán lại / hết sống)
    fn purge(&self, st: &mut AState, l: usize) {
        let dead: Vec<usize> = st
            .defs
            .iter()
            .filter(|(_, d)| d.locals(self).contains(&l))
            .map(|(k, _)| *k)
            .collect();
        for k in dead {
            st.defs.remove(&k);
        }
    }

    fn init(&self) -> AState {
        let mut env = Env::new(&self.zvars);
        for (i, t) in self.itys.iter().enumerate() {
            if let Some(t) = t {
                env.declare(i as VarId, *t);
            }
        }
        for a in self.slice_atom.values() {
            env.declare(*a, Ty::new(64, false));
        }
        AState {
            env,
            defs: BTreeMap::new(),
        }
    }

    // ------------------------------------------------------------------ toán hạng

    fn const_int(c: &Const) -> Option<I256> {
        match c {
            Const::Int { bits, ty } => Some(i256_of_bits(*bits, *ty)),
            _ => None,
        }
    }

    /// Biểu thức TOÁN HỌC của một toán hạng nguyên tại trạng thái `st` (đã thay biến tạm bằng định nghĩa còn hợp lệ).
    fn term_of(&self, st: &AState, op: &Operand) -> Option<Term> {
        match op {
            Operand::Const(c) => Self::const_int(c).map(Term::Int),
            Operand::Copy(p) | Operand::Move(p) => {
                let l = p.local.index();
                match p.projection.as_slice() {
                    [] => {
                        if !self.tracked(l) {
                            return None;
                        }
                        if let Some(Def::Int { t }) = st.defs.get(&l) {
                            return Some(t.clone());
                        }
                        Some(Term::Var(l as VarId))
                    }
                    [PlaceElem::Field(0, _)] => match st.defs.get(&l) {
                        Some(Def::Chk { res, .. }) => Some(res.clone()),
                        _ => None,
                    },
                    _ => None,
                }
            }
        }
    }

    fn prop_of(&self, st: &AState, op: &Operand) -> Option<Prop> {
        match op {
            Operand::Const(Const::Bool(b)) => Some(if *b { Prop::True } else { Prop::False }),
            Operand::Const(_) => None,
            Operand::Copy(p) | Operand::Move(p) => {
                let l = p.local.index();
                match p.projection.as_slice() {
                    [] => {
                        if !matches!(self.body.locals[l].ty, GTy::Bool) {
                            return None;
                        }
                        if let Some(Def::Bool { p: pr }) = st.defs.get(&l) {
                            return Some(pr.clone());
                        }
                        Some(Prop::Atom(self.bool_atom(l)))
                    }
                    [PlaceElem::Field(1, _)] => match st.defs.get(&l) {
                        Some(Def::Chk { flag, .. }) => Some(flag.clone()),
                        _ => None,
                    },
                    _ => None,
                }
            }
        }
    }

    fn len_term(&self, st: &AState, place: &gt_mir::Place) -> Option<Term> {
        let l = place.local.index();
        let _ = st;
        match place.projection.as_slice() {
            // `*s` với `s: &[T]`/`&![T]`
            [PlaceElem::Deref] => {
                if let Some(a) = self.slice_atom.get(&l) {
                    return Some(Term::Var(*a));
                }
                self.array_len
                    .get(&l)
                    .map(|n| Term::Int(I256::from_u128(u128::from(*n))))
            }
            [] => self
                .array_len
                .get(&l)
                .map(|n| Term::Int(I256::from_u128(u128::from(*n)))),
            _ => None,
        }
    }

    fn range_prop(ty: Ty, t: &Term) -> Prop {
        Prop::And(vec![
            Prop::Le(Term::Int(ty.min()), t.clone()),
            Prop::Le(t.clone(), Term::Int(ty.max())),
        ])
    }

    fn arith_term(op: BinOp, a: Term, b: Term) -> Option<Term> {
        match op {
            BinOp::Add | BinOp::AddWrap | BinOp::AddSat => Some(Term::add(a, b)),
            BinOp::Sub | BinOp::SubWrap | BinOp::SubSat => Some(Term::sub(a, b)),
            BinOp::Mul | BinOp::MulWrap | BinOp::MulSat => Some(Term::mul(a, b)),
            _ => None,
        }
    }

    // ------------------------------------------------------------------ gán

    fn forget_local(&self, st: &mut AState, l: usize, _loc: u32) {
        self.purge(st, l);
        st.defs.remove(&l);
        if self.itys[l].is_some() {
            st.env.forget(l as VarId);
        }
        st.env.forget_atom(self.bool_atom(l));
        if let Some(a) = self.slice_atom.get(&l)
            && *a as usize == self.nl + l
        {
            st.env.forget(*a);
        }
    }

    fn assign_local(&self, st: &mut AState, dst: usize, rv: &Rvalue, _loc: u32) {
        enum Val {
            Int(Term),
            Interval(Interval),
            Bool(Prop),
            Chk(Term, Prop),
            Unknown,
        }
        let dty = self.itys[dst];
        let is_bool = matches!(self.body.locals[dst].ty, GTy::Bool);
        let tuple_int = match &self.body.locals[dst].ty {
            GTy::Tuple(ts) if ts.len() == 2 => ity(&ts[0]).filter(|_| matches!(ts[1], GTy::Bool)),
            _ => None,
        };
        let val: Val = match rv {
            Rvalue::Use(op) => {
                if dty.is_some() {
                    self.term_of(st, op).map_or(Val::Unknown, Val::Int)
                } else if is_bool {
                    self.prop_of(st, op).map_or(Val::Unknown, Val::Bool)
                } else {
                    Val::Unknown
                }
            }
            Rvalue::BinaryOp(op, a, b) => {
                if op.is_comparison() {
                    match (self.term_of(st, a), self.term_of(st, b)) {
                        (Some(ta), Some(tb)) => Val::Bool(match op {
                            BinOp::Eq => Prop::Eq(ta, tb),
                            BinOp::Ne => Prop::Ne(ta, tb),
                            BinOp::Lt => Prop::Lt(ta, tb),
                            BinOp::Le => Prop::Le(ta, tb),
                            BinOp::Gt => Prop::Lt(tb, ta),
                            _ => Prop::Le(tb, ta),
                        }),
                        // so sánh hai bool: không diễn giải
                        _ => Val::Unknown,
                    }
                } else if *op == BinOp::BitAnd {
                    match (self.prop_of(st, a), self.prop_of(st, b)) {
                        (Some(pa), Some(pb)) => Val::Bool(Prop::and(vec![pa, pb])),
                        _ => Val::Unknown,
                    }
                } else if let Some(ty) = dty {
                    let (ta, tb) = (self.term_of(st, a), self.term_of(st, b));
                    match (op, ta, tb) {
                        (BinOp::Div, ..) | (BinOp::Rem, ..) => {
                            let (ia, ib) =
                                (self.eval_operand(st, a, ty), self.eval_operand(st, b, ty));
                            let r = if *op == BinOp::Div {
                                ia.div_trunc_nz(ib)
                            } else {
                                ia.rem_trunc_nz(ib)
                            };
                            r.map_or(Val::Unknown, Val::Interval)
                        }
                        (_, Some(ta), Some(tb)) => match Self::arith_term(*op, ta, tb) {
                            Some(t) => {
                                let iv = st.env.eval_term(&t);
                                if iv.fits(ty) == Tri::Yes {
                                    Val::Int(t)
                                } else {
                                    match op {
                                        BinOp::AddSat | BinOp::SubSat | BinOp::MulSat => {
                                            Val::Interval(iv.saturate(ty))
                                        }
                                        _ => Val::Interval(iv.wrap(ty)),
                                    }
                                }
                            }
                            None => Val::Unknown,
                        },
                        _ => Val::Unknown,
                    }
                } else {
                    Val::Unknown
                }
            }
            Rvalue::UnaryOp(UnOp::Not, op) => self
                .prop_of(st, op)
                .map_or(Val::Unknown, |p| Val::Bool(p.not())),
            Rvalue::UnaryOp(UnOp::Neg, op) => match (dty, self.term_of(st, op)) {
                (Some(ty), Some(t)) => {
                    let n = Term::Neg(Box::new(t));
                    let iv = st.env.eval_term(&n);
                    if iv.fits(ty) == Tri::Yes {
                        Val::Int(n)
                    } else {
                        Val::Interval(iv.wrap(ty))
                    }
                }
                _ => Val::Unknown,
            },
            Rvalue::Cast(op, to) => match (to, dty) {
                (GTy::Int(_), Some(_)) => {
                    if let Some(t) = self.term_of(st, op) {
                        Val::Int(t)
                    } else if self.prop_of(st, op).is_some() {
                        Val::Interval(Interval::new(B::from_i128(0), B::from_i128(1)))
                    } else {
                        Val::Unknown
                    }
                }
                _ => Val::Unknown,
            },
            Rvalue::CastFits(op, to) => match (self.term_of(st, op), ity(to)) {
                (Some(t), Some(ty)) => Val::Bool(Self::range_prop(ty, &t)),
                _ => Val::Unknown,
            },
            Rvalue::CheckedBinaryOp(op, a, b) => {
                match (tuple_int, self.term_of(st, a), self.term_of(st, b)) {
                    (Some(ty), Some(ta), Some(tb)) => match Self::arith_term(*op, ta, tb) {
                        Some(res) => {
                            let flag = Self::range_prop(ty, &res).not();
                            Val::Chk(res, flag)
                        }
                        None => Val::Unknown,
                    },
                    _ => Val::Unknown,
                }
            }
            Rvalue::Len(place) => match self.len_term(st, place) {
                Some(t) => Val::Int(t),
                None => Val::Unknown,
            },
            _ => Val::Unknown,
        };
        let new_def: Option<Def> = match val {
            Val::Int(ref t) => Some(Def::Int { t: t.clone() }),
            Val::Bool(ref p) => Some(Def::Bool { p: p.clone() }),
            Val::Chk(ref res, ref flag) => Some(Def::Chk {
                res: res.clone(),
                flag: flag.clone(),
            }),
            _ => None,
        };
        // định nghĩa mới KHÔNG được tham chiếu chính `dst` (sau gán `Var(dst)` là giá trị mới)
        // chỉ biến TẠM mới mang định nghĩa (biến người dùng sống trong `env`); định nghĩa không được tham chiếu chính `dst`
        let new_def = new_def.filter(|d| {
            matches!(self.body.locals[dst].kind, LocalKind::Temp) && !d.locals(self).contains(&dst)
        });
        // cập nhật môi trường
        let tracked = self.tracked(dst);
        match &val {
            Val::Int(t) if tracked => st.env.assign(dst as VarId, t),
            Val::Interval(iv) if tracked => st.env.assign_interval(dst as VarId, *iv),
            _ => {
                if self.itys[dst].is_some() {
                    st.env.forget(dst as VarId);
                }
            }
        }
        self.purge(st, dst);
        st.defs.remove(&dst);
        st.env.forget_atom(self.bool_atom(dst));
        if let Some(a) = self.slice_atom.get(&dst)
            && *a as usize == self.nl + dst
        {
            st.env.forget(*a);
        }
        if let Some(d) = new_def {
            st.defs.insert(dst, d);
        }
    }

    fn eval_operand(&self, st: &AState, op: &Operand, ty: Ty) -> Interval {
        match self.term_of(st, op) {
            Some(t) => st.env.eval_term(&t),
            None => ty.range(),
        }
    }

    fn step_stmt(&self, st: &mut AState, s: &Statement, loc: u32) {
        if st.env.is_dead() {
            return;
        }
        match &s.kind {
            StatementKind::Assign(p, rv) => {
                if p.projection.is_empty() {
                    self.assign_local(st, p.local.index(), rv, loc);
                }
                // ghi qua trường/deref: các local được theo dõi KHÔNG bị mượn `&!` nên không bị ảnh hưởng
            }
            StatementKind::StorageLive(l) | StatementKind::StorageDead(l) => {
                self.forget_local(st, l.index(), loc)
            }
            StatementKind::FakeRead(..) | StatementKind::Nop => {}
        }
    }

    // ------------------------------------------------------------------ nghĩa vụ

    fn hyps_for(
        &self,
        st: &AState,
        goal: &Prop,
    ) -> (Vec<VarDecl>, Vec<(VarId, String)>, Vec<Prop>) {
        let mut vars = Vec::new();
        let mut atoms = Vec::new();
        goal.vars(&mut vars, &mut atoms);
        vars.sort_unstable();
        vars.dedup();
        atoms.sort_unstable();
        atoms.dedup();
        let mut decls = Vec::new();
        let mut hyps = Vec::new();
        for v in &vars {
            let Some(ty) = st.env.ty(*v) else { continue };
            decls.push(VarDecl {
                id: *v,
                name: self.var_name(*v),
                ty,
            });
            let iv = st.env.interval(*v);
            if let B::Fin(lo) = iv.lo
                && lo > ty.min()
            {
                hyps.push(Prop::Le(Term::Int(lo), Term::Var(*v)));
            }
            if let B::Fin(hi) = iv.hi
                && hi < ty.max()
            {
                hyps.push(Prop::Le(Term::Var(*v), Term::Int(hi)));
            }
        }
        for (i, x) in vars.iter().enumerate() {
            for (j, y) in vars.iter().enumerate() {
                if i == j {
                    continue;
                }
                if let Some(c) = st.env.zone_bound(*x, *y) {
                    hyps.push(Prop::Le(
                        Term::sub(Term::Var(*x), Term::Var(*y)),
                        Term::int(c),
                    ));
                }
            }
        }
        let mut atom_names = Vec::new();
        for a in &atoms {
            atom_names.push((*a, format!("a{a}")));
            match st.env.atom(*a) {
                Some(true) => hyps.push(Prop::Atom(*a)),
                Some(false) => hyps.push(Prop::Atom(*a).not()),
                None => {}
            }
        }
        (decls, atom_names, hyps)
    }

    fn kind_of(k: &AssertKind) -> ObKind {
        match k {
            AssertKind::Overflow(op, ..) => match op {
                BinOp::Div | BinOp::Rem => ObKind::DivOverflow,
                _ => ObKind::Overflow,
            },
            AssertKind::OverflowNeg(_) => ObKind::NegMin,
            AssertKind::DivisionByZero(_) => ObKind::DivZero,
            AssertKind::RemainderByZero(_) => ObKind::RemZero,
            AssertKind::IndexOutOfBounds { .. } => ObKind::Index,
            AssertKind::SliceRange { .. } => ObKind::SliceRange,
            AssertKind::CastLossy { .. } => ObKind::CastLossy,
        }
    }

    // ------------------------------------------------------------------ cạnh

    /// Trạng thái ở đầu mỗi khối kế tiếp (đã áp hiệu ứng terminator và thu hẹp theo cạnh). `judge`: sinh/giải nghĩa vụ tại `Assert`.
    fn edges(
        &self,
        st: &AState,
        bb: usize,
        term: &Terminator,
        mut judge: Option<&mut Judge<'_>>,
    ) -> Vec<(BasicBlock, AState)> {
        let mut out = Vec::new();
        if st.env.is_dead() {
            return out;
        }
        let loc = loc_id(bb, self.body.blocks[bb].statements.len());
        match &term.kind {
            TerminatorKind::Goto(t) => out.push((*t, st.clone())),
            TerminatorKind::Return | TerminatorKind::Unreachable => {}
            TerminatorKind::Drop { target, .. } => out.push((*target, st.clone())),
            TerminatorKind::SwitchInt {
                discr,
                targets,
                otherwise,
            } => {
                let prop = self.prop_of(st, discr);
                let int_term = self.term_of(st, discr);
                for (val, tgt) in targets {
                    let mut s = st.clone();
                    if let Some(p) = &prop
                        && matches!(discr, Operand::Copy(_) | Operand::Move(_))
                    {
                        s.env.assume(p, *val != 0);
                    } else if let Some(t) = &int_term {
                        let k = Term::Int(I256::from_u128(*val));
                        s.env.assume(&Prop::Eq(t.clone(), k), true);
                    }
                    out.push((*tgt, s));
                }
                let mut s = st.clone();
                if let Some(p) = &prop
                    && targets.len() == 1
                    && targets[0].0 == 0
                {
                    s.env.assume(p, true);
                } else if let Some(p) = &prop
                    && targets.len() == 1
                    && targets[0].0 == 1
                {
                    s.env.assume(p, false);
                } else if let Some(t) = &int_term {
                    for (val, _) in targets {
                        let k = Term::Int(I256::from_u128(*val));
                        s.env.assume(&Prop::Ne(t.clone(), k), true);
                    }
                }
                out.push((*otherwise, s));
            }
            TerminatorKind::Assert {
                cond,
                expected,
                kind,
                target,
            } => {
                let mut s = st.clone();
                let goal = self
                    .prop_of(st, cond)
                    .map(|p| if *expected { p } else { p.not() });
                if let (Some(j), Some(g)) = (judge.as_deref_mut(), goal.as_ref()) {
                    j.judge(self, st, bb, term, kind, g.clone());
                } else if let Some(j) = judge {
                    // không có định nghĩa của điều kiện: nghĩa vụ không giải được (nguyên tử mới)
                    j.judge(
                        self,
                        st,
                        bb,
                        term,
                        kind,
                        Prop::Atom((3 * self.nl + bb) as VarId),
                    );
                }
                // đường thất bại abort tức thì: sau Assert điều kiện đúng
                if let Some(g) = goal {
                    s.env.assume(&g, true);
                }
                out.push((*target, s));
            }
            TerminatorKind::Call {
                callee,
                args,
                dest,
                target,
                ..
            } => {
                if let Some(t) = target {
                    let mut s = st.clone();
                    if dest.projection.is_empty() {
                        let d = dest.local.index();
                        self.forget_local(&mut s, d, loc);
                        // `<[T]>::len(&s)` = nguyên tử độ dài của gốc `s`
                        if let (
                            gt_mir::Callee::Fn { node, .. },
                            [Operand::Copy(a) | Operand::Move(a)],
                        ) = (callee, args.as_slice())
                            && self.len_callee.contains(node)
                            && a.projection.is_empty()
                            && let Some(atom) = self.slice_atom.get(&a.local.index())
                            && self.tracked(d)
                        {
                            let t = Term::Var(*atom);
                            s.env.assign(d as VarId, &t);
                            s.defs.insert(d, Def::Int { t });
                        }
                    }
                    out.push((*t, s));
                }
            }
        }
        out
    }

    /// Hiệu ứng của cả khối lên trạng thái đầu, rồi các cạnh ra.
    fn block_out(
        &self,
        st: &AState,
        bb: usize,
        judge: Option<&mut Judge<'_>>,
    ) -> Vec<(BasicBlock, AState)> {
        let mut s = st.clone();
        let b = &self.body.blocks[bb];
        for (i, stmt) in b.statements.iter().enumerate() {
            self.step_stmt(&mut s, stmt, loc_id(bb, i));
        }
        match &b.terminator {
            Some(t) => self.edges(&s, bb, t, judge),
            None => vec![],
        }
    }
}

/// Bộ gom/giải nghĩa vụ.
pub struct Judge<'p> {
    pub prover: &'p mut dyn Prover,
    pub budget_per_ob: u64,
    pub out: Vec<Judged>,
    pub discharged: BTreeSet<u32>,
    pub key: String,
    pub ordinal: usize,
}

impl Judge<'_> {
    fn judge(
        &mut self,
        cx: &Ctx<'_>,
        st: &AState,
        bb: usize,
        term: &Terminator,
        kind: &AssertKind,
        goal: Prop,
    ) {
        let (vars, atoms, hyps) = cx.hyps_for(st, &goal);
        let ob = Obligation {
            id: format!("{}#{}", self.key, self.ordinal),
            kind: Ctx::kind_of(kind),
            body: self.key.clone(),
            block: bb as u32,
            stmt: cx.body.blocks[bb].statements.len() as u32,
            span: (term.source.span.start, term.source.span.end),
            vars,
            atoms,
            goal,
            hyps,
            runtime: Checkability::Runtime(format!("assert:{}", kind.name())),
        };
        self.ordinal += 1;
        let mut b = Budget::new(self.budget_per_ob);
        let status = match self.prover.prove(&ob, &mut b) {
            Verdict::Proved(c) => {
                self.discharged.insert(bb as u32);
                Status::Discharged {
                    prover: self.prover.name(),
                    cert: c.0,
                }
            }
            Verdict::Refuted {
                everywhere: true, ..
            } => Status::DefinitelyFalse,
            _ => match &ob.runtime {
                Checkability::Runtime(_) => Status::Lowered {
                    note: match ob.kind {
                        ObKind::Overflow | ObKind::NegMin => "N0601",
                        ObKind::Index | ObKind::SliceRange => "N0602",
                        ObKind::DivZero | ObKind::RemZero | ObKind::DivOverflow => "N0603",
                        _ => "N0604",
                    },
                },
                Checkability::NotCheckable(w) => Status::Rejected { why: w.clone() },
            },
        };
        self.out.push(Judged {
            node: term.source.node,
            ob,
            status,
        });
    }
}

fn gt_dataflow_rpo(body: &Body) -> Vec<BasicBlock> {
    // RPO tất định theo thứ tự successors của terminator (cùng quy ước của gt-dataflow)
    let n = body.blocks.len();
    let mut seen = vec![false; n];
    let mut post = Vec::new();
    let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
    seen[0] = true;
    while let Some((b, i)) = stack.pop() {
        let succ: Vec<BasicBlock> = body.blocks[b]
            .terminator
            .as_ref()
            .map_or(vec![], |t| t.kind.successors());
        if i < succ.len() {
            stack.push((b, i + 1));
            let s = succ[i].index();
            if !seen[s] {
                seen[s] = true;
                stack.push((s, 0));
            }
        } else {
            post.push(BasicBlock(b as u32));
        }
    }
    post.reverse();
    post
}

/// Phân tích một thân; `prover` giải từng nghĩa vụ.
pub fn analyze_body(
    body: &Body,
    keys: &BTreeMap<gt_ast::NodeId, String>,
    prover: &mut dyn Prover,
) -> BodyReport {
    let mut rep = BodyReport {
        key: body.key.clone(),
        ..BodyReport::default()
    };
    let mut cx = Ctx::new(body, keys, true);
    rep.zone_dropped = cx.zone_dropped.clone();
    let mut steps = 0u64;
    let mut states = match fixpoint(&cx, &mut steps) {
        Some(s) => s,
        None => {
            // hết ngân sách: bỏ quan hệ zone và chạy lại
            rep.budget_exhausted = true;
            cx = Ctx::new(body, keys, false);
            steps = 0;
            match fixpoint(&cx, &mut steps) {
                Some(s) => s,
                None => {
                    // vẫn vượt: mọi nghĩa vụ là Unknown (giữ mọi check)
                    let mut judged_none = NoProver;
                    let mut judge = Judge {
                        prover: &mut judged_none,
                        budget_per_ob: 0,
                        out: vec![],
                        discharged: BTreeSet::new(),
                        key: body.key.clone(),
                        ordinal: 0,
                    };
                    collect_unknown(&cx, &mut judge);
                    rep.judged = judge.out;
                    return rep;
                }
            }
        }
    };
    let pre = states.clone();
    narrow(&cx, &mut states);
    if check_post_fixpoint(&cx, &states).is_err() {
        // narrowing không đơn điệu trên siêu dữ liệu: bỏ nó, dùng điểm cố định thuần widening (vẫn phải qua kiểm)
        states = pre;
        rep.narrowing_rejected = true;
    }
    if let Err(e) = check_post_fixpoint(&cx, &states) {
        rep.ice = Some(e);
        return rep;
    }
    let mut judge = Judge {
        prover,
        budget_per_ob: 1 << 16,
        out: vec![],
        discharged: BTreeSet::new(),
        key: body.key.clone(),
        ordinal: 0,
    };
    for b in &cx.rpo {
        let bb = b.index();
        if let Some(s) = &states[bb] {
            let _ = cx.block_out(s, bb, Some(&mut judge));
        }
    }
    rep.judged = judge.out;
    rep.discharged_blocks = judge.discharged;
    rep
}

struct NoProver;
impl Prover for NoProver {
    fn name(&self) -> &'static str {
        "none"
    }
    fn prove(&mut self, _: &Obligation, _: &mut Budget) -> Verdict {
        Verdict::Unknown
    }
}

fn collect_unknown(cx: &Ctx<'_>, judge: &mut Judge<'_>) {
    let init = cx.init();
    for b in &cx.rpo {
        let bb = b.index();
        let _ = cx.block_out(&init, bb, Some(judge));
    }
}

/// Lặp tới điểm cố định (RPO) với widening tại đầu vòng lặp; `None` nếu vượt ngân sách bước.
fn fixpoint(cx: &Ctx<'_>, steps: &mut u64) -> Option<Vec<Option<AState>>> {
    let n = cx.body.blocks.len();
    let mut states: Vec<Option<AState>> = vec![None; n];
    states[0] = Some(cx.init());
    let mut visits = vec![0u32; n];
    loop {
        let mut changed = false;
        for b in &cx.rpo {
            let bb = b.index();
            let Some(s) = states[bb].clone() else {
                continue;
            };
            *steps += 1;
            if *steps > MAX_BLOCK_STEPS {
                return None;
            }
            for (tgt, out) in cx.block_out(&s, bb, None) {
                let t = tgt.index();
                let back_edge = cx.rpo_pos[t] <= cx.rpo_pos[bb];
                let new = match &states[t] {
                    None => out,
                    Some(old) => {
                        let joined = old.join(&out);
                        if back_edge && visits[t] >= 1 {
                            old.widen(&joined, &cx.thresholds)
                        } else {
                            joined
                        }
                    }
                };
                if states[t].as_ref() != Some(&new) {
                    states[t] = Some(new);
                    visits[t] += 1;
                    changed = true;
                }
            }
        }
        if !changed {
            return Some(states);
        }
    }
}

/// Hai lượt narrowing: thay trạng thái bằng giá trị tính lại từ các cạnh vào nếu mạnh hơn (⊑).
fn narrow(cx: &Ctx<'_>, states: &mut [Option<AState>]) {
    let n = cx.body.blocks.len();
    for _ in 0..2 {
        let mut incoming: Vec<Option<AState>> = vec![None; n];
        incoming[0] = Some(cx.init());
        for b in &cx.rpo {
            let bb = b.index();
            let Some(s) = states[bb].clone() else {
                continue;
            };
            for (tgt, out) in cx.block_out(&s, bb, None) {
                let t = tgt.index();
                incoming[t] = Some(match incoming[t].take() {
                    None => out,
                    Some(o) => o.join(&out),
                });
            }
        }
        for (bb, inc) in incoming.into_iter().enumerate() {
            if let (Some(i), Some(old)) = (inc, states[bb].as_ref())
                && i.env.leq(&old.env)
                && bb != 0
            {
                states[bb] = Some(i);
            }
        }
    }
}

/// Kiểm HẬU-ĐIỂM-CỐ-ĐỊNH độc lập với đường widening/narrowing: mọi cạnh `b → s` phải thoả `out(b → s).env ⊑ state(s).env`.
fn check_post_fixpoint(cx: &Ctx<'_>, states: &[Option<AState>]) -> Result<(), String> {
    for b in &cx.rpo {
        let bb = b.index();
        let Some(s) = &states[bb] else { continue };
        for (tgt, out) in cx.block_out(s, bb, None) {
            if out.env.is_dead() {
                continue;
            }
            match &states[tgt.index()] {
                Some(t) if out.env.leq(&t.env) => {}
                _ => {
                    return Err(format!(
                        "trạng thái cuối không phải hậu-điểm-cố-định tại cạnh bb{bb} → bb{}",
                        tgt.index()
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Báo cáo ngắn của một thân (cho test/CLI).
#[must_use]
pub fn summarize(r: &BodyReport) -> (usize, usize, usize, usize) {
    let mut d = 0;
    let mut l = 0;
    let mut f = 0;
    let mut x = 0;
    for j in &r.judged {
        match j.status {
            Status::Discharged { .. } => d += 1,
            Status::Lowered { .. } => l += 1,
            Status::DefinitelyFalse => f += 1,
            Status::Rejected { .. } => x += 1,
        }
    }
    (d, l, f, x)
}

#[allow(dead_code)]
fn _use(_: Local) {}
