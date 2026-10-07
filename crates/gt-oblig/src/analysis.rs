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
/// Miền id biến độc lập với số local của thân (để mệnh đề hợp đồng dùng chung giữa hàm và thân hợp đồng của nó):
/// `[0, LEN_BASE)` giá trị nguyên của local; `LEN_BASE + l` độ dài lát của gốc `l`; `BOOL_BASE + l` nguyên tử bool của local `l`;
/// `OPAQUE_BASE + k` nguyên tử không diễn giải được.
pub const LEN_BASE: u32 = 1 << 20;
pub const BOOL_BASE: u32 = 2 << 20;
pub const OPAQUE_BASE: u32 = 3 << 20;
/// nguyên tử không diễn giải được của MỘT hợp đồng (đánh số toàn chương trình, tăng dần)
pub const CONTRACT_OPAQUE_BASE: u32 = 4 << 20;
/// nguyên tử của hợp đồng sau khi thế vào một điểm gọi/thoát (duy nhất theo (khối, nguyên tử), tất định)
pub const SITE_BASE: u32 = 5 << 20;
/// nghĩa vụ hợp đồng không biểu diễn được: một nguyên tử riêng cho mỗi (khối, chỉ số hợp đồng)
pub const UNREP_BASE: u32 = 6 << 20;
/// vị từ thuần một đối số trên một local (vd `is_some(&x)`): `PRED_BASE + chỉ_số_vị_từ·1024 + local`
pub const PRED_BASE: u32 = 7 << 20;

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
    /// chỗ phải chèn lệnh gọi hợp đồng + `Assert` khi `status` là `Lowered` (chỉ `Pre`/`Post`)
    pub insert: Option<Insert>,
}

/// Chèn kiểm hợp đồng lúc chạy (driver áp lên MIR đã tối ưu).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Insert {
    /// trước `Call` ở cuối khối `block`: gọi thân `⊳` với bản sao các đối số rồi `Assert`
    Pre {
        block: u32,
        contract: gt_ast::NodeId,
        generics: Vec<GTy>,
        args: Vec<Operand>,
    },
    /// trước `Return` ở cuối khối `block`: gọi thân `⊲` với `(_1.._n, _0)` rồi `Assert`
    Post {
        block: u32,
        contract: gt_ast::NodeId,
        params: usize,
    },
}

/// Một hợp đồng đã trích thành mệnh đề trên tham số (cùng chỉ số local với hàm; `⊲` có thêm `result` ở chỉ số cuối).
#[derive(Clone, Debug)]
pub struct ContractProp {
    /// NodeId của thân hợp đồng (= id hợp đồng trong AST)
    pub node: gt_ast::NodeId,
    pub key: String,
    /// `None`: không biểu diễn được (vòng lặp, lời gọi, biến tạm không định nghĩa…)
    pub prop: Option<Prop>,
    pub arg_count: usize,
    /// tham số `i` (1-based, chỉ số `i-1`) sao chép được (kiểu Copy) — cần để gọi thân hợp đồng lúc chạy
    pub arg_copy: Vec<bool>,
    /// thân hợp đồng có thành phần không kiểm được lúc chạy (lượng từ không biên)
    pub unchecked: Option<String>,
    /// thân hợp đồng có vòng lặp (lượng từ có biên): ghi chú N0903 thay cho N0901/N0902
    pub has_loop: bool,
}

/// Thông tin toàn chương trình cho phân tích một thân.
#[derive(Clone, Debug, Default)]
pub struct ProgInfo {
    pub keys: BTreeMap<gt_ast::NodeId, String>,
    /// theo NodeId của HÀM
    pub pre: BTreeMap<gt_ast::NodeId, Vec<ContractProp>>,
    pub post: BTreeMap<gt_ast::NodeId, Vec<ContractProp>>,
    /// hàm có `⊳` được ĐẢM BẢO kiểm ở mọi lời gọi (chỉ gọi trực tiếp: không thoát qua `FnItem`, không phải thành viên impl của trait)
    pub assume_pre: BTreeSet<gt_ast::NodeId>,
    /// tổng số nguyên tử hợp đồng đã cấp (để đánh số `SITE_BASE`)
    pub contract_atoms: u32,
    /// hàm được mô hình hoá như vị từ không diễn giải (thuần, một đối số, trả `bool`) → chỉ số vị từ
    pub pure_preds: BTreeMap<gt_ast::NodeId, u32>,
    /// theo khoá thân: từng local có kiểu Copy (theo ràng buộc đa hình của hàm)
    pub local_copy: BTreeMap<String, Vec<bool>>,
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
    Int {
        t: Term,
    },
    Bool {
        p: Prop,
    },
    Chk {
        res: Term,
        flag: Prop,
    },
    /// biến tạm là `&x` (tham chiếu chia sẻ tới local trần `x`)
    RefOf(usize),
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
            Def::RefOf(l) => return vec![*l],
        }
        vs.extend(at);
        vs.iter().filter_map(|v| cx.stamp_local(*v)).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AState {
    env: Env,
    /// điều kiện đường đi (chỉ ghi khi `Ctx.record_pc`, dùng để trích mệnh đề hợp đồng)
    pc: Vec<Prop>,
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
        AState {
            env,
            pc: Vec::new(),
            defs,
        }
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
    info: &'a ProgInfo,
    record_pc: bool,
    /// số lần gán tĩnh của mỗi local (không tính ràng buộc tham số)
    assigns: Vec<u32>,
}

fn loc_id(block: usize, stmt: usize) -> u32 {
    (block as u32)
        .saturating_mul(2048)
        .saturating_add(stmt as u32 + 1)
        .min(u32::MAX - 1)
}

impl<'a> Ctx<'a> {
    fn new(body: &'a Body, info: &'a ProgInfo, use_zone: bool) -> Ctx<'a> {
        let keys = &info.keys;
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
                    slice_atom.insert(i, LEN_BASE + root_of(i) as VarId);
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
                cand.push((
                    *a,
                    format!("len({})", Self::lname(body, (*a - LEN_BASE) as usize)),
                ));
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
            info,
            record_pc: false,
            assigns,
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
        if v < LEN_BASE {
            Self::lname(self.body, v as usize)
        } else if v < BOOL_BASE {
            format!("len({})", Self::lname(self.body, (v - LEN_BASE) as usize))
        } else {
            format!("a{v}")
        }
    }

    fn tracked(&self, l: usize) -> bool {
        self.itys[l].is_some() && !self.aliased[l]
    }

    fn bool_atom(&self, l: usize) -> VarId {
        BOOL_BASE + l as VarId
    }

    /// local mà một biến (hoặc nguyên tử độ dài) phụ thuộc, để đóng dấu phiên bản.
    fn stamp_local(&self, v: VarId) -> Option<usize> {
        if v < LEN_BASE {
            Some(v as usize)
        } else if v < BOOL_BASE {
            Some((v - LEN_BASE) as usize)
        } else if v < OPAQUE_BASE {
            Some((v - BOOL_BASE) as usize)
        } else if v >= PRED_BASE {
            Some(((v - PRED_BASE) % 1024) as usize)
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
        // `⊳` được đảm bảo kiểm ở mọi lời gọi ⇒ giả thiết cho cả thân
        if self.body.kind == gt_mir::BodyKind::Fn && self.info.assume_pre.contains(&self.body.node)
        {
            for cp in self.info.pre.get(&self.body.node).into_iter().flatten() {
                if let Some(p) = &cp.prop {
                    env.assume(p, true);
                }
            }
        }
        AState {
            env,
            pc: Vec::new(),
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
        st.env
            .retain_atoms(|a| !(a >= PRED_BASE && ((a - PRED_BASE) % 1024) as usize == l));
        if let Some(a) = self.slice_atom.get(&l)
            && *a == LEN_BASE + l as VarId
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
            RefOf(usize),
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
            Rvalue::Ref(_, BorrowKind::Shared, p)
                if p.projection.is_empty() && !self.aliased[p.local.index()] =>
            {
                Val::RefOf(p.local.index())
            }
            _ => Val::Unknown,
        };
        let new_def: Option<Def> = match val {
            Val::Int(ref t) => Some(Def::Int { t: t.clone() }),
            Val::Bool(ref p) => Some(Def::Bool { p: p.clone() }),
            Val::Chk(ref res, ref flag) => Some(Def::Chk {
                res: res.clone(),
                flag: flag.clone(),
            }),
            Val::RefOf(l) => Some(Def::RefOf(l)),
            _ => None,
        };
        // định nghĩa mới KHÔNG được tham chiếu chính `dst` (sau gán `Var(dst)` là giá trị mới)
        // chỉ biến TẠM mới mang định nghĩa (biến người dùng sống trong `env`); định nghĩa không được tham chiếu chính `dst`
        let new_def = new_def.filter(|d| {
            matches!(
                self.body.locals[dst].kind,
                LocalKind::Temp | LocalKind::Return
            ) && !d.locals(self).contains(&dst)
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
        st.env
            .retain_atoms(|a| !(a >= PRED_BASE && ((a - PRED_BASE) % 1024) as usize == dst));
        if let Some(a) = self.slice_atom.get(&dst)
            && *a == LEN_BASE + dst as VarId
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
            AssertKind::Precondition => ObKind::Pre,
            AssertKind::Postcondition => ObKind::Post,
            AssertKind::Invariant => ObKind::Invariant,
        }
    }

    // ------------------------------------------------------------------ hợp đồng

    /// local mà một đối số của vị từ thuần nhắc tới: `x` (theo giá trị) hoặc `&x` qua biến tạm `RefOf`
    fn pred_key(&self, st: &AState, a: &Operand) -> Option<usize> {
        let (Operand::Copy(p) | Operand::Move(p)) = a else {
            return None;
        };
        if !p.projection.is_empty() {
            return None;
        }
        let l = p.local.index();
        let key = match st.defs.get(&l) {
            Some(Def::RefOf(x)) => *x,
            _ => l,
        };
        (key < 1024 && !self.aliased[key]).then_some(key)
    }

    fn push_pc(&self, s: &mut AState, p: Prop) {
        if self.record_pc {
            s.pc.push(p);
        }
    }

    fn site_atom(&self, bb: usize, a: VarId) -> VarId {
        SITE_BASE + (bb as u32) * self.info.contract_atoms.max(1) + (a - CONTRACT_OPAQUE_BASE)
    }

    fn unrep_atom(&self, bb: usize, idx: usize) -> VarId {
        UNREP_BASE + (bb as u32) * 1024 + idx.min(1023) as u32
    }

    fn subst_term(t: &Term, ints: &BTreeMap<VarId, Term>) -> Option<Term> {
        Some(match t {
            Term::Int(_) => t.clone(),
            Term::Var(v) => ints.get(v)?.clone(),
            Term::Add(a, b) => Term::add(Self::subst_term(a, ints)?, Self::subst_term(b, ints)?),
            Term::Sub(a, b) => Term::sub(Self::subst_term(a, ints)?, Self::subst_term(b, ints)?),
            Term::Mul(a, b) => Term::mul(Self::subst_term(a, ints)?, Self::subst_term(b, ints)?),
            Term::Neg(a) => Term::Neg(Box::new(Self::subst_term(a, ints)?)),
        })
    }

    fn subst_prop_k(
        &self,
        p: &Prop,
        ints: &BTreeMap<VarId, Term>,
        props: &BTreeMap<VarId, Prop>,
        keys: &BTreeMap<u32, u32>,
        bb: usize,
    ) -> Option<Prop> {
        let t = |x: &Term| Self::subst_term(x, ints);
        Some(match p {
            Prop::True | Prop::False => p.clone(),
            Prop::Le(a, b) => Prop::Le(t(a)?, t(b)?),
            Prop::Lt(a, b) => Prop::Lt(t(a)?, t(b)?),
            Prop::Eq(a, b) => Prop::Eq(t(a)?, t(b)?),
            Prop::Ne(a, b) => Prop::Ne(t(a)?, t(b)?),
            Prop::And(v) => Prop::And(
                v.iter()
                    .map(|q| self.subst_prop_k(q, ints, props, keys, bb))
                    .collect::<Option<_>>()?,
            ),
            Prop::Or(v) => Prop::Or(
                v.iter()
                    .map(|q| self.subst_prop_k(q, ints, props, keys, bb))
                    .collect::<Option<_>>()?,
            ),
            Prop::Not(q) => Prop::Not(Box::new(self.subst_prop_k(q, ints, props, keys, bb)?)),
            Prop::Atom(a) => match props.get(a) {
                Some(q) => q.clone(),
                None if *a >= PRED_BASE => {
                    let off = *a - PRED_BASE;
                    let l = keys.get(&(off % 1024))?;
                    Prop::Atom(PRED_BASE + (off / 1024) * 1024 + *l)
                }
                None if *a >= CONTRACT_OPAQUE_BASE && *a < SITE_BASE => {
                    Prop::Atom(self.site_atom(bb, *a))
                }
                None => return None,
            },
        })
    }

    /// Ánh xạ tham số hợp đồng ← đối số của một lời gọi (chỉ int, độ dài lát, bool).
    fn arg_maps(
        &self,
        st: &AState,
        args: &[Operand],
    ) -> (
        BTreeMap<VarId, Term>,
        BTreeMap<VarId, Prop>,
        BTreeMap<u32, u32>,
    ) {
        let mut ints = BTreeMap::new();
        let mut props = BTreeMap::new();
        let mut keys = BTreeMap::new();
        for (j, a) in args.iter().enumerate() {
            let i = (j + 1) as VarId;
            if let Some(t) = self.term_of(st, a) {
                ints.insert(i, t);
            }
            if let Some(p) = self.prop_of(st, a) {
                props.insert(BOOL_BASE + i, p);
            }
            if let Some(k) = self.pred_key(st, a) {
                keys.insert(i, k as u32);
            }
            if let Operand::Copy(pl) | Operand::Move(pl) = a
                && pl.projection.is_empty()
                && let Some(atom) = self.slice_atom.get(&pl.local.index())
            {
                ints.insert(LEN_BASE + i, Term::Var(*atom));
            }
        }
        (ints, props, keys)
    }

    /// `⊳` tại một lời gọi trực tiếp: nghĩa vụ `Pre` cho từng hợp đồng; sau điểm gọi `⊳` đúng (sai thì đã abort).
    #[allow(clippy::too_many_arguments)]
    fn pres_common(
        &self,
        s: &mut AState,
        st: &AState,
        bb: usize,
        term: Option<&Terminator>,
        node: gt_ast::NodeId,
        generics: &[GTy],
        args: &[Operand],
        mut judge: Option<&mut Judge<'_>>,
    ) {
        let Some(cps) = self.info.pre.get(&node) else {
            return;
        };
        let (ints, props, keys) = self.arg_maps(st, args);
        for (idx, cp) in cps.iter().enumerate() {
            let goal = cp
                .prop
                .as_ref()
                .and_then(|p| self.subst_prop_k(p, &ints, &props, &keys, bb));
            if let (Some(j), Some(term)) = (judge.as_deref_mut(), term) {
                let g = goal.clone().unwrap_or(Prop::Atom(self.unrep_atom(bb, idx)));
                let why = if let Some(u) = &cp.unchecked {
                    Some(format!("hợp đồng chứa phần không kiểm được lúc chạy: {u}"))
                } else if !args.iter().all(|a| self.operand_is_copy(a)) {
                    Some(
                        "đối số không sao chép được (không Copy) nên không gọi được hợp đồng lúc chạy"
                            .to_string(),
                    )
                } else {
                    None
                };
                let runtime = match why {
                    None => Checkability::Runtime("contract:pre".to_string()),
                    Some(w) => Checkability::NotCheckable(w),
                };
                let ins = Insert::Pre {
                    block: bb as u32,
                    contract: cp.node,
                    generics: generics.to_vec(),
                    args: args.to_vec(),
                };
                j.judge_ob(
                    self,
                    st,
                    bb,
                    term,
                    ObKind::Pre,
                    g,
                    runtime,
                    Some(ins),
                    false,
                );
                j.relabel_loop(cp.has_loop);
            }
            if let Some(g) = goal {
                s.env.assume(&g, true);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn judge_pres(
        &self,
        s: &mut AState,
        st: &AState,
        bb: usize,
        term: &Terminator,
        node: gt_ast::NodeId,
        generics: &[GTy],
        args: &[Operand],
        j: &mut Judge<'_>,
    ) {
        self.pres_common(s, st, bb, Some(term), node, generics, args, Some(j));
    }

    fn assume_pres(
        &self,
        s: &mut AState,
        st: &AState,
        bb: usize,
        node: gt_ast::NodeId,
        args: &[Operand],
    ) {
        self.pres_common(s, st, bb, None, node, &[], args, None);
    }

    /// Sau lời gọi trực tiếp: `⊲` của callee đúng với `result := dest` (callee đã chứng minh hoặc kiểm lúc chạy ở thoát).
    fn assume_posts(
        &self,
        s: &mut AState,
        st: &AState,
        bb: usize,
        node: gt_ast::NodeId,
        args: &[Operand],
        dest: usize,
    ) {
        let Some(cps) = self.info.post.get(&node) else {
            return;
        };
        let (mut ints, mut props, keys) = self.arg_maps(st, args);
        // đối số có nhắc `dest` (x = f(x)): giá trị cũ của `dest` đã bị quên ⇒ không thế được
        let mentions_dest = ints.values().any(|t| {
            let mut v = Vec::new();
            t.vars(&mut v);
            v.iter().any(|x| self.stamp_local(*x) == Some(dest))
        });
        if mentions_dest {
            return;
        }
        for cp in cps {
            let Some(p) = &cp.prop else { continue };
            let r = cp.arg_count as VarId;
            if self.tracked(dest) {
                ints.insert(r, Term::Var(dest as VarId));
            }
            if matches!(self.body.locals[dest].ty, GTy::Bool) {
                props.insert(BOOL_BASE + r, Prop::Atom(BOOL_BASE + dest as VarId));
            }
            if let Some(g) = self.subst_prop_k(p, &ints, &props, &keys, bb) {
                s.env.assume(&g, true);
            }
        }
    }

    fn ty_is_copy(t: &GTy) -> bool {
        match t {
            GTy::Int(_) | GTy::Float(_) | GTy::Bool | GTy::Char | GTy::Never => true,
            GTy::Ref(false, _) => true,
            GTy::Tuple(v) => v.iter().all(Self::ty_is_copy),
            GTy::Array(e, _) => Self::ty_is_copy(e),
            _ => false,
        }
    }

    fn local_is_copy(&self, l: usize) -> bool {
        match self.info.local_copy.get(&self.body.key) {
            Some(v) => v.get(l).copied().unwrap_or(false),
            None => Self::ty_is_copy(&self.body.locals[l].ty),
        }
    }

    fn operand_is_copy(&self, op: &Operand) -> bool {
        match op {
            Operand::Const(_) => true,
            Operand::Copy(p) | Operand::Move(p) => {
                p.projection.is_empty() && self.local_is_copy(p.local.index())
            }
        }
    }

    /// Nghĩa vụ `Post` ở một điểm thoát.
    fn judge_posts(&self, st: &AState, bb: usize, term: &Terminator, j: &mut Judge<'_>) {
        if self.body.kind != gt_mir::BodyKind::Fn {
            return;
        }
        let Some(cps) = self.info.post.get(&self.body.node) else {
            return;
        };
        let ret_op = Operand::Copy(gt_mir::Place::local(Local(0)));
        for (idx, cp) in cps.iter().enumerate() {
            let r = cp.arg_count as VarId;
            let mut ints: BTreeMap<VarId, Term> = BTreeMap::new();
            let mut props: BTreeMap<VarId, Prop> = BTreeMap::new();
            let mut keys: BTreeMap<u32, u32> = BTreeMap::new();
            let mut bad_param: Option<String> = None;
            let mut vs = Vec::new();
            let mut at = Vec::new();
            if let Some(p) = &cp.prop {
                p.vars(&mut vs, &mut at);
            }
            for i in 1..cp.arg_count.min(self.nl) {
                let used = vs
                    .iter()
                    .any(|v| *v == i as VarId || *v == LEN_BASE + i as VarId)
                    || at.iter().any(|a| {
                        *a == BOOL_BASE + i as VarId
                            || (*a >= PRED_BASE && ((*a - PRED_BASE) % 1024) as usize == i)
                    });
                if !used {
                    continue;
                }
                if self.assigns[i] > 0 || self.aliased[i] {
                    bad_param = Some(format!(
                        "hậu điều kiện nhắc tham số `{}` đã bị gán lại hoặc mượn `&!`",
                        Self::lname(self.body, i)
                    ));
                }
                ints.insert(i as VarId, Term::Var(i as VarId));
                keys.insert(i as u32, i as u32);
                props.insert(BOOL_BASE + i as VarId, Prop::Atom(BOOL_BASE + i as VarId));
                if let Some(a) = self.slice_atom.get(&i) {
                    ints.insert(LEN_BASE + i as VarId, Term::Var(*a));
                }
            }
            if let Some(t) = self.term_of(st, &ret_op) {
                ints.insert(r, t);
            }
            if let Some(p) = self.prop_of(st, &ret_op) {
                props.insert(BOOL_BASE + r, p);
            }
            let goal = cp
                .prop
                .as_ref()
                .and_then(|p| self.subst_prop_k(p, &ints, &props, &keys, bb));
            let g = goal.unwrap_or(Prop::Atom(self.unrep_atom(bb, idx)));
            let params_copy = (1..self.nl.min(cp.arg_count)).all(|i| self.local_is_copy(i));
            let why = if let Some(b) = bad_param {
                Some(b)
            } else if let Some(u) = &cp.unchecked {
                Some(format!("hợp đồng chứa phần không kiểm được lúc chạy: {u}"))
            } else if !params_copy || !self.local_is_copy(0) {
                Some(
                    "tham số hoặc giá trị trả về không sao chép được (không Copy) nên không gọi được hợp đồng lúc chạy"
                        .to_string(),
                )
            } else {
                None
            };
            let runtime = match why {
                None => Checkability::Runtime("contract:post".to_string()),
                Some(w) => Checkability::NotCheckable(w),
            };
            let ins = Insert::Post {
                block: bb as u32,
                contract: cp.node,
                params: cp.arg_count,
            };
            j.judge_ob(
                self,
                st,
                bb,
                term,
                ObKind::Post,
                g,
                runtime,
                Some(ins),
                false,
            );
            j.relabel_loop(cp.has_loop);
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
            TerminatorKind::Unreachable => {}
            TerminatorKind::Return => {
                if let Some(j) = judge {
                    self.judge_posts(st, bb, term, j);
                }
            }
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
                        self.push_pc(
                            &mut s,
                            if *val != 0 {
                                p.clone()
                            } else {
                                p.clone().not()
                            },
                        );
                    } else if let Some(t) = &int_term {
                        let k = Term::Int(I256::from_u128(*val));
                        s.env.assume(&Prop::Eq(t.clone(), k.clone()), true);
                        self.push_pc(&mut s, Prop::Eq(t.clone(), k));
                    } else {
                        self.push_pc(&mut s, Prop::Atom(OPAQUE_BASE + bb as VarId));
                    }
                    out.push((*tgt, s));
                }
                let mut s = st.clone();
                if let Some(p) = &prop
                    && targets.len() == 1
                    && targets[0].0 == 0
                {
                    s.env.assume(p, true);
                    self.push_pc(&mut s, p.clone());
                } else if let Some(p) = &prop
                    && targets.len() == 1
                    && targets[0].0 == 1
                {
                    s.env.assume(p, false);
                    self.push_pc(&mut s, p.clone().not());
                } else if let Some(t) = &int_term {
                    for (val, _) in targets {
                        let k = Term::Int(I256::from_u128(*val));
                        s.env.assume(&Prop::Ne(t.clone(), k.clone()), true);
                        self.push_pc(&mut s, Prop::Ne(t.clone(), k));
                    }
                } else {
                    self.push_pc(&mut s, Prop::Atom(OPAQUE_BASE + bb as VarId).not());
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
                        Prop::Atom(OPAQUE_BASE + bb as VarId),
                    );
                }
                // đường thất bại abort tức thì: sau Assert điều kiện đúng
                if let Some(g) = goal {
                    s.env.assume(&g, true);
                    self.push_pc(&mut s, g);
                } else {
                    self.push_pc(&mut s, Prop::Atom(OPAQUE_BASE + bb as VarId));
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
                let mut s = st.clone();
                if let gt_mir::Callee::Fn { node, generics } = callee {
                    if let Some(j) = judge.as_deref_mut() {
                        self.judge_pres(&mut s, st, bb, term, *node, generics, args, j);
                    } else {
                        self.assume_pres(&mut s, st, bb, *node, args);
                    }
                }
                if let Some(t) = target {
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
                        if let (gt_mir::Callee::Fn { node, .. }, [a]) = (callee, args.as_slice())
                            && let Some(pi) = self.info.pure_preds.get(node)
                            && matches!(self.body.locals[d].ty, GTy::Bool)
                            && let Some(key) = self.pred_key(st, a)
                        {
                            s.defs.insert(
                                d,
                                Def::Bool {
                                    p: Prop::Atom(PRED_BASE + pi * 1024 + key as u32),
                                },
                            );
                        }
                        if let gt_mir::Callee::Fn { node, .. } = callee {
                            self.assume_posts(&mut s, st, bb, *node, args, d);
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
    /// nghĩa vụ vừa thêm là hợp đồng có vòng lặp: đổi ghi chú thành N0903
    fn relabel_loop(&mut self, has_loop: bool) {
        if has_loop
            && let Some(j) = self.out.last_mut()
            && let Status::Lowered { note } = &mut j.status
        {
            *note = "N0903";
        }
    }

    fn judge(
        &mut self,
        cx: &Ctx<'_>,
        st: &AState,
        bb: usize,
        term: &Terminator,
        kind: &AssertKind,
        goal: Prop,
    ) {
        self.judge_ob(
            cx,
            st,
            bb,
            term,
            Ctx::kind_of(kind),
            goal,
            Checkability::Runtime(format!("assert:{}", kind.name())),
            None,
            true,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn judge_ob(
        &mut self,
        cx: &Ctx<'_>,
        st: &AState,
        bb: usize,
        term: &Terminator,
        kind: ObKind,
        goal: Prop,
        runtime: Checkability,
        insert: Option<Insert>,
        is_assert: bool,
    ) {
        let (vars, atoms, hyps) = cx.hyps_for(st, &goal);
        let ob = Obligation {
            id: format!("{}#{}", self.key, self.ordinal),
            kind,
            body: self.key.clone(),
            block: bb as u32,
            stmt: cx.body.blocks[bb].statements.len() as u32,
            span: (term.source.span.start, term.source.span.end),
            vars,
            atoms,
            goal,
            hyps,
            runtime,
        };
        self.ordinal += 1;
        let mut b = Budget::new(self.budget_per_ob);
        let status = match self.prover.prove(&ob, &mut b) {
            Verdict::Proved(c) => {
                if is_assert {
                    self.discharged.insert(bb as u32);
                }
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
                        ObKind::Pre => "N0901",
                        ObKind::Post => "N0902",
                        ObKind::Invariant => "N0904",
                        _ => "N0604",
                    },
                },
                Checkability::NotCheckable(w) => Status::Rejected { why: w.clone() },
            },
        };
        // chương trình có lỗi ĐỎ không được chạy, nhưng MIR vẫn giữ nghĩa abort (oracle so sánh được)
        let insert = if matches!(status, Status::Lowered { .. } | Status::DefinitelyFalse) {
            insert
        } else {
            None
        };
        self.out.push(Judged {
            node: term.source.node,
            ob,
            status,
            insert,
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
pub fn analyze_body(body: &Body, info: &ProgInfo, prover: &mut dyn Prover) -> BodyReport {
    let mut rep = BodyReport {
        key: body.key.clone(),
        ..BodyReport::default()
    };
    let mut cx = Ctx::new(body, info, true);
    rep.zone_dropped = cx.zone_dropped.clone();
    let mut steps = 0u64;
    let mut states = match fixpoint(&cx, &mut steps) {
        Some(s) => s,
        None => {
            // hết ngân sách: bỏ quan hệ zone và chạy lại
            rep.budget_exhausted = true;
            cx = Ctx::new(body, info, false);
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

// ====================================================================== trích mệnh đề hợp đồng

fn walk_paths(
    cx: &Ctx<'_>,
    st: AState,
    bb: usize,
    out: &mut Vec<Prop>,
    visiting: &mut [bool],
    budget: &mut usize,
) -> bool {
    if st.env.is_dead() {
        return true;
    }
    if visiting[bb] || *budget == 0 {
        return false;
    }
    visiting[bb] = true;
    let mut s = st;
    let b = &cx.body.blocks[bb];
    for (i, stmt) in b.statements.iter().enumerate() {
        cx.step_stmt(&mut s, stmt, loc_id(bb, i));
    }
    let ok = match &b.terminator {
        None => false,
        Some(t) => match &t.kind {
            TerminatorKind::Unreachable => true,
            TerminatorKind::Return => {
                let ret = Operand::Copy(gt_mir::Place::local(Local(0)));
                let v = cx.prop_of(&s, &ret).unwrap_or(Prop::Atom(BOOL_BASE));
                let mut conj = s.pc.clone();
                conj.push(v);
                out.push(Prop::and(conj));
                *budget -= 1;
                true
            }
            _ => {
                let mut ok = true;
                for (tg, ns) in cx.edges(&s, bb, t, None) {
                    if !walk_paths(cx, ns, tg.index(), out, visiting, budget) {
                        ok = false;
                        break;
                    }
                }
                ok
            }
        },
    };
    visiting[bb] = false;
    ok
}

fn canon_term(t: &Term, argc: u32) -> Option<Term> {
    Some(match t {
        Term::Int(_) => t.clone(),
        Term::Var(v) => {
            let ok = (1..=argc).contains(v) || (v > &LEN_BASE && *v <= LEN_BASE + argc);
            if !ok {
                return None;
            }
            t.clone()
        }
        Term::Add(a, b) => Term::add(canon_term(a, argc)?, canon_term(b, argc)?),
        Term::Sub(a, b) => Term::sub(canon_term(a, argc)?, canon_term(b, argc)?),
        Term::Mul(a, b) => Term::mul(canon_term(a, argc)?, canon_term(b, argc)?),
        Term::Neg(a) => Term::Neg(Box::new(canon_term(a, argc)?)),
    })
}

fn canon_prop(
    p: &Prop,
    argc: u32,
    map: &mut BTreeMap<VarId, VarId>,
    counter: &mut u32,
) -> Option<Prop> {
    let mut sub = |q: &Prop| canon_prop(q, argc, map, counter);
    Some(match p {
        Prop::True | Prop::False => p.clone(),
        Prop::Le(a, b) => Prop::Le(canon_term(a, argc)?, canon_term(b, argc)?),
        Prop::Lt(a, b) => Prop::Lt(canon_term(a, argc)?, canon_term(b, argc)?),
        Prop::Eq(a, b) => Prop::Eq(canon_term(a, argc)?, canon_term(b, argc)?),
        Prop::Ne(a, b) => Prop::Ne(canon_term(a, argc)?, canon_term(b, argc)?),
        Prop::And(v) => Prop::And(v.iter().map(&mut sub).collect::<Option<_>>()?),
        Prop::Or(v) => Prop::Or(v.iter().map(&mut sub).collect::<Option<_>>()?),
        Prop::Not(q) => Prop::Not(Box::new(sub(q)?)),
        Prop::Atom(a) => {
            // nguyên tử bool của THAM SỐ giữ nguyên (có nghĩa ở hàm); mọi nguyên tử khác thành nguyên tử riêng của hợp đồng
            if (*a > BOOL_BASE && *a <= BOOL_BASE + argc)
                || (*a >= PRED_BASE && (1..=argc).contains(&((*a - PRED_BASE) % 1024)))
            {
                Prop::Atom(*a)
            } else {
                let id = *map.entry(*a).or_insert_with(|| {
                    let n = CONTRACT_OPAQUE_BASE + *counter;
                    *counter += 1;
                    n
                });
                Prop::Atom(id)
            }
        }
    })
}

/// Trích mệnh đề "hợp đồng đánh giá xong (không abort) và trả `true`" từ thân hợp đồng (không vòng lặp, ≤ 512 đường).
/// `None` nếu không biểu diễn được. Mệnh đề chỉ nhắc `Var(1..=n)`, `Var(LEN_BASE+1..)` và nguyên tử riêng.
pub fn extract_contract(body: &Body, info: &ProgInfo, counter: &mut u32) -> Option<Prop> {
    let mut cx = Ctx::new(body, info, false);
    cx.record_pc = true;
    let init = cx.init();
    let n = body.blocks.len();
    let mut paths = Vec::new();
    let mut visiting = vec![false; n];
    let mut budget = 512usize;
    if !walk_paths(&cx, init, 0, &mut paths, &mut visiting, &mut budget) {
        return None;
    }
    let p = match paths.len() {
        0 => Prop::False,
        1 => paths.remove(0),
        _ => Prop::Or(paths),
    };
    let mut map = BTreeMap::new();
    canon_prop(&p, body.arg_count as u32, &mut map, counter)
}
