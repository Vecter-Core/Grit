//! ORACLE ĐỘNG cho borrowck (ADR-024). CÔNG CỤ TEST, NGOÀI TCB, KHÔNG dùng lại mã phân tích của `gt-borrowck` hay
//! `gt-dataflow` (không liveness tĩnh, không tập khoản vay tĩnh): chỉ đọc cấu trúc MIR của `gt-mir` rồi CHẠY nó trên một
//! mô hình bộ nhớ có theo dõi khoản vay/move/Arena.
//!
//! **Định nghĩa vi phạm (phân tích vết, ADR-023 §11):** một khoản vay bị truy cập XUNG ĐỘT (không đi qua chính nó hay hậu duệ
//! của nó) rồi còn được DÙNG về sau (giải tham chiếu mang khoản vay đó, hoặc hậu duệ của nó). Thêm: đọc giá trị đã move/chưa gán,
//! đọc chỗ đã chết (`StorageDead`/hết khung), move ra khỏi chỗ đi qua `*`/`[i]`.
//!
//! **Giới hạn nói thẳng:** chỉ thấy vi phạm trên các đường ĐÃ CHẠY; chính xác theo chỉ số cụ thể (borrowck coi mọi chỉ số chồng
//! lấn nhau) nên "oracle sạch, borrowck từ chối" không tự động là false positive của borrowck; quy tắc tĩnh thuần
//! (move một trường khỏi kiểu có Drop) không được mô hình hoá.
#![allow(dead_code)]
use gt_ast::NodeId;
use gt_mir::*;
use gt_types::{IntTy, Ty};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub enum V {
    Int(i128, IntTy),
    Bool(bool),
    Char(u32),
    Str(String),
    Unit,
    Tuple(Vec<V>),
    Array(Vec<V>),
    Adt(u32, Vec<V>),
    Ref(RefV),
    Arena(u32),
    Fn(NodeId),
    Uninit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RefV {
    pub loc: Loc,
    pub loan: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loc {
    pub fid: usize,
    pub local: usize,
    pub path: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Field(usize),
    Index(usize),
    Sub(usize, usize),
}

/// Loại vi phạm (để thống kê và đối chiếu với mã của borrowck).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VKind {
    /// đọc/ghi trường/move giá trị đã move hoặc chưa gán — E0401
    UseAfterMove,
    /// `&!` mới trong khi `&!` khác còn dùng về sau — E0402
    UniqueVsUnique,
    /// `&!` vs `&`/đọc — E0403
    UniqueVsShared,
    /// ghi/move khi đang bị mượn — E0406
    WriteWhileBorrowed,
    /// chỗ chết (hết phạm vi/khung) khi còn khoản vay dùng về sau — E0404
    Dangling,
    /// như trên nhưng khoản vay xuất phát từ một Arena — E0405
    ArenaEscape,
    /// move ra khỏi `*`/`[i]` — E0407
    MoveOutOfRef,
}

impl VKind {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            VKind::UseAfterMove => "E0401",
            VKind::UniqueVsUnique => "E0402",
            VKind::UniqueVsShared => "E0403",
            VKind::Dangling => "E0404",
            VKind::ArenaEscape => "E0405",
            VKind::WriteWhileBorrowed => "E0406",
            VKind::MoveOutOfRef => "E0407",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Violation {
    pub kind: VKind,
    pub msg: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Value(V),
    Abort(String),
    Violation(Violation),
    /// MIR sai/ngoài phạm vi thông dịch (không phải vi phạm của chương trình)
    Bug(String),
    OutOfFuel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AK {
    Read,
    Write,
    Move,
    Dead,
    BorrowShared,
    BorrowUnique,
}

struct Loan {
    unique: bool,
    loc: Loc,
    parent: Option<usize>,
    from_arena: bool,
    /// (loại vi phạm sẽ báo, mô tả) — đặt khi bị truy cập xung đột
    invalid: Option<(VKind, String)>,
}

struct FrameData {
    locals: Vec<V>,
}

struct Frame {
    fid: usize,
}

/// Thống kê của một lần chạy (cho độ phủ).
#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub loans: usize,
    pub reborrows: usize,
    pub arena_allocs: usize,
    pub arena_resets: usize,
    pub cell_ops: usize,
    pub moves: usize,
    pub calls: usize,
}

pub struct Oracle<'a> {
    pub mir: &'a MirOutput,
    frames: Vec<FrameData>,
    stack: Vec<Frame>,
    loans: Vec<Loan>,
    dead: BTreeSet<(usize, usize)>,
    arenas: u32,
    fuel: u64,
    pub stats: Stats,
}

enum Flow {
    Ret,
    Abort(String),
    Bug(String),
    Viol(Violation),
    Fuel,
}

fn viol<T>(kind: VKind, msg: impl Into<String>) -> Result<T, Flow> {
    Err(Flow::Viol(Violation {
        kind,
        msg: msg.into(),
    }))
}

fn bug<T>(msg: impl Into<String>) -> Result<T, Flow> {
    Err(Flow::Bug(msg.into()))
}

fn overlap(a: &Loc, b: &Loc) -> bool {
    if a.fid != b.fid || a.local != b.local {
        return false;
    }
    for (x, y) in a.path.iter().zip(b.path.iter()) {
        let disjoint = match (x, y) {
            (Step::Field(i), Step::Field(j)) => i != j,
            (Step::Index(i), Step::Index(j)) => i != j,
            (Step::Index(i), Step::Sub(s, e)) | (Step::Sub(s, e), Step::Index(i)) => {
                !(s <= i && i < e)
            }
            (Step::Sub(s1, e1), Step::Sub(s2, e2)) => !(s1 < e2 && s2 < e1),
            _ => false,
        };
        if disjoint {
            return false;
        }
    }
    true
}

impl<'a> Oracle<'a> {
    #[must_use]
    pub fn new(mir: &'a MirOutput, fuel: u64) -> Self {
        Oracle {
            mir,
            frames: Vec::new(),
            stack: Vec::new(),
            loans: Vec::new(),
            dead: BTreeSet::new(),
            arenas: 0,
            fuel,
            stats: Stats::default(),
        }
    }

    pub fn call_key(&mut self, key: &str, args: Vec<V>) -> Outcome {
        let Some(b) = self.mir.body(key) else {
            return Outcome::Bug(format!("không có thân {key}"));
        };
        self.run(b, args)
    }

    fn run(&mut self, b: &'a Body, args: Vec<V>) -> Outcome {
        if self.stack.len() > 100 {
            return Outcome::Bug("đệ quy quá sâu".into());
        }
        let mut locals = vec![V::Uninit; b.locals.len()];
        for (i, a) in args.into_iter().enumerate() {
            locals[i + 1] = a;
        }
        let fid = self.frames.len();
        self.frames.push(FrameData { locals });
        self.stack.push(Frame { fid });
        let r = self.exec(b);
        let ret = self.frames[fid].locals[0].clone();
        let mut end = Ok(());
        // hết khung: mọi biến cục bộ chết (kể cả `_0` sau khi đã sao chép) — khoản vay vào chúng bị vô hiệu
        if matches!(r, Flow::Ret) {
            for l in 0..b.locals.len() {
                if let Err(e) = self.access(
                    &Loc {
                        fid,
                        local: l,
                        path: vec![],
                    },
                    AK::Dead,
                    None,
                ) {
                    end = Err(e);
                    break;
                }
                self.dead.insert((fid, l));
            }
        }
        // giá trị trả về mang tham chiếu: người gọi sẽ dùng chúng — mọi khoản vay trong đó phải còn hợp lệ
        if end.is_ok() && matches!(r, Flow::Ret) {
            end = self.check_returned(&ret);
        }
        self.stack.pop();
        if let Err(e) = end {
            return Self::flow_outcome(e);
        }
        match r {
            Flow::Ret => match ret {
                V::Uninit => Outcome::Bug("trả về `_0` chưa gán".into()),
                v => Outcome::Value(v),
            },
            other => Self::flow_outcome(other),
        }
    }

    /// Mọi tham chiếu trong giá trị trả về phải còn hợp lệ (không treo, không bị truy cập xung đột).
    fn check_returned(&self, v: &V) -> Result<(), Flow> {
        match v {
            V::Ref(r) => self.use_loan(r.loan),
            V::Tuple(x) | V::Array(x) | V::Adt(_, x) => {
                x.iter().try_for_each(|y| self.check_returned(y))
            }
            _ => Ok(()),
        }
    }

    fn flow_outcome(f: Flow) -> Outcome {
        match f {
            Flow::Abort(k) => Outcome::Abort(k),
            Flow::Bug(m) => Outcome::Bug(m),
            Flow::Viol(v) => Outcome::Violation(v),
            Flow::Fuel => Outcome::OutOfFuel,
            Flow::Ret => Outcome::Bug("Ret ngoài ngữ cảnh".into()),
        }
    }

    fn top(&self) -> usize {
        self.stack.last().map_or(0, |f| f.fid)
    }

    // ---------------------------------------------------------------- khoản vay

    fn new_loan(
        &mut self,
        unique: bool,
        loc: Loc,
        parent: Option<usize>,
        from_arena: bool,
    ) -> usize {
        self.stats.loans += 1;
        if parent.is_some() {
            self.stats.reborrows += 1;
        }
        self.loans.push(Loan {
            unique,
            loc,
            parent,
            from_arena,
            invalid: None,
        });
        self.loans.len() - 1
    }

    /// DÙNG khoản vay `x` (giải tham chiếu mang nó): `x` và mọi tổ tiên phải còn hợp lệ.
    fn use_loan(&self, x: usize) -> Result<(), Flow> {
        let mut cur = Some(x);
        while let Some(i) = cur {
            if let Some((k, why)) = &self.loans[i].invalid {
                return viol(*k, why.clone());
            }
            cur = self.loans[i].parent;
        }
        Ok(())
    }

    fn lineage(&self, via: Option<usize>) -> Vec<usize> {
        let mut v = Vec::new();
        let mut cur = via;
        while let Some(i) = cur {
            v.push(i);
            cur = self.loans[i].parent;
        }
        v
    }

    /// Một truy cập vào `loc` (qua khoản vay `via` nếu đi qua một tham chiếu): vô hiệu mọi khoản vay chồng lấn xung đột.
    fn access(&mut self, loc: &Loc, kind: AK, via: Option<usize>) -> Result<(), Flow> {
        if kind != AK::Dead && self.dead.contains(&(loc.fid, loc.local)) {
            return viol(
                VKind::Dangling,
                format!("truy cập chỗ đã chết _{} (khung {})", loc.local, loc.fid),
            );
        }
        let lin = self.lineage(via);
        for i in 0..self.loans.len() {
            if lin.contains(&i) || self.loans[i].invalid.is_some() {
                continue;
            }
            if !overlap(&self.loans[i].loc, loc) {
                continue;
            }
            let l_unique = self.loans[i].unique;
            let write_like = matches!(kind, AK::Write | AK::Move | AK::Dead | AK::BorrowUnique);
            if !(l_unique || write_like) {
                continue;
            }
            let k = match (kind, l_unique) {
                (AK::Dead, _) => {
                    if self.loans[i].from_arena {
                        VKind::ArenaEscape
                    } else {
                        VKind::Dangling
                    }
                }
                (AK::BorrowUnique, true) => VKind::UniqueVsUnique,
                (AK::BorrowUnique, false) => VKind::UniqueVsShared,
                (AK::Write | AK::Move, _) => {
                    if self.loans[i].from_arena && matches!(kind, AK::Move) {
                        VKind::ArenaEscape
                    } else {
                        VKind::WriteWhileBorrowed
                    }
                }
                (AK::Read | AK::BorrowShared, true) => VKind::UniqueVsShared,
                (AK::Read | AK::BorrowShared, false) => VKind::UniqueVsShared,
            };
            // truy cập BorrowUnique vào Arena có khoản vay chung (reset) là E0405 (ADR-023 §10, Q32)
            let k = if self.loans[i].from_arena
                && !l_unique
                && matches!(kind, AK::BorrowUnique | AK::Write)
            {
                VKind::ArenaEscape
            } else {
                k
            };
            self.loans[i].invalid = Some((
                k,
                format!(
                    "khoản vay #{i} ({}) trên _{} bị truy cập {kind:?} xung đột",
                    if l_unique { "&!" } else { "&" },
                    self.loans[i].loc.local
                ),
            ));
        }
        Ok(())
    }

    // ---------------------------------------------------------------- chỗ

    /// Phân giải một chỗ: vị trí cụ thể + khoản vay của tham chiếu cuối cùng được giải tham chiếu.
    fn resolve(&mut self, p: &Place) -> Result<(Loc, Option<usize>), Flow> {
        let fid = self.top();
        let mut loc = Loc {
            fid,
            local: p.local.index(),
            path: Vec::new(),
        };
        let mut via: Option<usize> = None;
        for e in &p.projection {
            match e {
                PlaceElem::Deref => {
                    let v = self.get(&loc)?;
                    match v {
                        V::Ref(r) => {
                            self.use_loan(r.loan)?;
                            via = Some(r.loan);
                            loc = r.loc;
                        }
                        V::Uninit => {
                            return viol(
                                VKind::UseAfterMove,
                                "giải tham chiếu một tham chiếu đã move/chưa gán",
                            );
                        }
                        other => return bug(format!("deref {other:?}")),
                    }
                }
                PlaceElem::Field(i, _) => loc.path.push(Step::Field(*i as usize)),
                PlaceElem::Downcast(_) => {}
                PlaceElem::Index(l) => {
                    let i = self.local_usize(*l)?;
                    match loc.path.last().cloned() {
                        Some(Step::Sub(a, _)) => {
                            loc.path.pop();
                            loc.path.push(Step::Index(a + i));
                        }
                        _ => loc.path.push(Step::Index(i)),
                    }
                }
                PlaceElem::Subslice { from, to } => {
                    let a = self.local_usize(*from)?;
                    let b = match to {
                        Some(t) => self.local_usize(*t)?,
                        None => match self.get(&loc)? {
                            V::Array(v) => v.len(),
                            other => return bug(format!("subslice {other:?}")),
                        },
                    };
                    match loc.path.last().cloned() {
                        Some(Step::Sub(s, _)) => {
                            loc.path.pop();
                            loc.path.push(Step::Sub(s + a, s + b));
                        }
                        _ => loc.path.push(Step::Sub(a, b)),
                    }
                }
            }
        }
        Ok((loc, via))
    }

    fn local_usize(&self, l: Local) -> Result<usize, Flow> {
        let f = self.top();
        match &self.frames[f].locals[l.index()] {
            V::Int(i, _) => usize::try_from(*i).or_else(|_| bug("chỉ số âm")),
            other => bug(format!("chỉ số {other:?}")),
        }
    }

    /// Đọc giá trị tại `loc` (không kiểm khoản vay). `Uninit` ở bất kỳ mức nào trên đường đi = đã move/chưa gán.
    fn get(&self, loc: &Loc) -> Result<V, Flow> {
        let mut v = self.frames[loc.fid].locals[loc.local].clone();
        for st in &loc.path {
            v = match (st, v) {
                (_, V::Uninit) => {
                    return viol(
                        VKind::UseAfterMove,
                        format!("đọc _{} đã move/chưa gán", loc.local),
                    );
                }
                (Step::Field(i), V::Tuple(x) | V::Adt(_, x)) => match x.get(*i) {
                    Some(y) => y.clone(),
                    None => return bug("trường ngoài biên"),
                },
                (Step::Index(i), V::Array(x)) => match x.get(*i) {
                    Some(y) => y.clone(),
                    None => return bug("chỉ số ngoài biên KHÔNG có assert"),
                },
                (Step::Sub(a, b), V::Array(x)) => match x.get(*a..*b) {
                    Some(y) => V::Array(y.to_vec()),
                    None => return bug("lát ngoài biên"),
                },
                (s, o) => return bug(format!("bước {s:?} trên {o:?}")),
            };
        }
        Ok(v)
    }

    fn has_uninit(v: &V) -> bool {
        match v {
            V::Uninit => true,
            V::Tuple(x) | V::Array(x) | V::Adt(_, x) => x.iter().any(Self::has_uninit),
            _ => false,
        }
    }

    fn set(&mut self, loc: &Loc, val: V) -> Result<(), Flow> {
        fn go(slot: &mut V, path: &[Step], val: V) -> Result<(), String> {
            match path.split_first() {
                None => {
                    *slot = val;
                    Ok(())
                }
                Some((_, _)) if matches!(slot, V::Uninit) => Err("UNINIT".into()),
                Some((Step::Field(i), rest)) => match slot {
                    V::Tuple(x) | V::Adt(_, x) => go(x.get_mut(*i).ok_or("trường")?, rest, val),
                    _ => Err("ghi trường vào giá trị không phải aggregate".into()),
                },
                Some((Step::Index(i), rest)) => match slot {
                    V::Array(x) => go(x.get_mut(*i).ok_or("chỉ số")?, rest, val),
                    _ => Err("ghi chỉ số vào giá trị không phải mảng".into()),
                },
                Some((Step::Sub(..), _)) => Err("ghi vào lát".into()),
            }
        }
        match go(&mut self.frames[loc.fid].locals[loc.local], &loc.path, val) {
            Ok(()) => Ok(()),
            Err(m) if m == "UNINIT" => viol(
                VKind::UseAfterMove,
                format!("ghi vào trường của _{} đã move", loc.local),
            ),
            Err(m) => bug(m),
        }
    }

    /// Đặt `Uninit` (move ra) tại `loc`.
    fn take(&mut self, loc: &Loc) -> Result<(), Flow> {
        self.set(loc, V::Uninit)
    }

    fn write_place(&mut self, p: &Place, val: V) -> Result<(), Flow> {
        let (loc, via) = self.resolve(p)?;
        self.access(&loc, AK::Write, via)?;
        // ghi lại cả local khi đang chết? chỉ cho phép sau StorageLive
        self.set(&loc, val)
    }

    fn operand(&mut self, o: &Operand) -> Result<V, Flow> {
        match o {
            Operand::Copy(p) => {
                let (loc, via) = self.resolve(p)?;
                self.access(&loc, AK::Read, via)?;
                let v = self.get(&loc)?;
                if Self::has_uninit(&v) {
                    return viol(VKind::UseAfterMove, "đọc giá trị đã move một phần");
                }
                Ok(v)
            }
            Operand::Move(p) => {
                let (loc, via) = self.resolve(p)?;
                if p.projection.iter().any(|e| {
                    matches!(
                        e,
                        PlaceElem::Deref | PlaceElem::Index(_) | PlaceElem::Subslice { .. }
                    )
                }) {
                    return viol(VKind::MoveOutOfRef, "move ra khỏi chỗ đi qua `*`/`[i]`");
                }
                self.access(&loc, AK::Move, via)?;
                let v = self.get(&loc)?;
                if Self::has_uninit(&v) {
                    return viol(VKind::UseAfterMove, "move giá trị đã move một phần");
                }
                self.stats.moves += 1;
                self.take(&loc)?;
                Ok(v)
            }
            Operand::Const(c) => Ok(match c {
                Const::Int { bits, ty } => V::Int(from_bits(*bits, *ty), *ty),
                Const::Bool(b) => V::Bool(*b),
                Const::Char(c) => V::Char(*c),
                Const::Str(s) => V::Str(s.clone()),
                Const::Unit => V::Unit,
                Const::FnItem { node, .. } => V::Fn(*node),
                Const::Item { node, .. } => {
                    let Some(cb) = self.mir.bodies.iter().find(|x| x.node == *node) else {
                        return bug("thiếu thân của hằng");
                    };
                    match self.run(cb, vec![]) {
                        Outcome::Value(v) => v,
                        Outcome::Abort(k) => return Err(Flow::Abort(k)),
                        Outcome::Violation(v) => return Err(Flow::Viol(v)),
                        Outcome::Bug(m) => return Err(Flow::Bug(m)),
                        Outcome::OutOfFuel => return Err(Flow::Fuel),
                    }
                }
                Const::Float { .. } => return bug("số thực ngoài phạm vi thông dịch"),
            }),
        }
    }

    // ---------------------------------------------------------------- thực thi

    fn exec(&mut self, b: &'a Body) -> Flow {
        let mut cur = ENTRY;
        loop {
            if self.fuel == 0 {
                return Flow::Fuel;
            }
            self.fuel -= 1;
            let bd = &b.blocks[cur.index()];
            for s in &bd.statements {
                if let Err(e) = self.stmt(s) {
                    return e;
                }
            }
            let Some(t) = &bd.terminator else {
                return Flow::Bug("khối thiếu terminator".into());
            };
            match self.terminator(t, &mut cur) {
                Ok(true) => return Flow::Ret,
                Ok(false) => {}
                Err(e) => return e,
            }
        }
    }

    /// `Ok(true)` = trả về.
    fn terminator(&mut self, t: &Terminator, cur: &mut BasicBlock) -> Result<bool, Flow> {
        match &t.kind {
            TerminatorKind::Goto(x) => *cur = *x,
            TerminatorKind::Return => return Ok(true),
            TerminatorKind::Unreachable => return bug(format!("tới `unreachable` ở bb{}", cur.0)),
            TerminatorKind::Drop { place, target } => {
                let (loc, via) = self.resolve(place)?;
                // Drop của chỗ đã move hết là no-op
                if matches!(self.get(&loc), Ok(v) if !matches!(v, V::Uninit)) {
                    self.access(&loc, AK::Dead, via)?;
                }
                *cur = *target;
            }
            TerminatorKind::SwitchInt {
                discr,
                targets,
                otherwise,
            } => {
                let v = self.operand(discr)?;
                let n: u128 = match v {
                    V::Bool(x) => u128::from(x),
                    V::Int(i, t) => {
                        (i as u128)
                            & if t.bits() == 128 {
                                u128::MAX
                            } else {
                                (1u128 << t.bits()) - 1
                            }
                    }
                    V::Char(c) => u128::from(c),
                    other => return bug(format!("switchInt trên {other:?}")),
                };
                *cur = targets
                    .iter()
                    .find(|(k, _)| *k == n)
                    .map_or(*otherwise, |(_, b)| *b);
            }
            TerminatorKind::Assert {
                cond,
                expected,
                kind,
                target,
            } => match self.operand(cond)? {
                V::Bool(x) if x == *expected => *cur = *target,
                V::Bool(_) => return Err(Flow::Abort(kind.name().to_string())),
                o => return bug(format!("assert trên {o:?}")),
            },
            TerminatorKind::Call {
                callee,
                args,
                dest,
                target,
                ..
            } => {
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.operand(a)?);
                }
                self.stats.calls += 1;
                let node = match callee {
                    Callee::Fn { node, .. } => *node,
                    Callee::Value(o) => match self.operand(o)? {
                        V::Fn(n) => n,
                        other => return bug(format!("gọi giá trị {other:?}")),
                    },
                    Callee::TraitMethod { .. } => {
                        return bug("gọi qua trait (ngoài phạm vi thông dịch)");
                    }
                };
                let result = if let Some(cb) = self.mir.bodies.iter().find(|x| x.node == node) {
                    match self.run(cb, vals) {
                        Outcome::Value(v) => v,
                        Outcome::Abort(k) => return Err(Flow::Abort(k)),
                        Outcome::Violation(v) => return Err(Flow::Viol(v)),
                        Outcome::Bug(m) => return Err(Flow::Bug(m)),
                        Outcome::OutOfFuel => return Err(Flow::Fuel),
                    }
                } else {
                    let k = self.mir.keys.get(&node).cloned().unwrap_or_default();
                    self.intrinsic(&k, vals)?
                };
                self.write_place(dest, result)?;
                match target {
                    Some(x) => *cur = *x,
                    None => return bug("hàm `→ !` trả về"),
                }
            }
        }
        Ok(false)
    }

    /// Hàm của prelude (thân KHÔNG được kiểm — trusted intrinsic): chỉ Arena.
    fn intrinsic(&mut self, key: &str, mut args: Vec<V>) -> Result<V, Flow> {
        if key.ends_with("core::abort") {
            return Err(Flow::Abort("abort".into()));
        }
        if key.ends_with("Arena::new") {
            self.arenas += 1;
            return Ok(V::Arena(self.arenas));
        }
        if key.ends_with("Arena::alloc") {
            // alloc(&'r self, v) → &'r T: ô mới, khoản vay là CON của khoản vay chung trên Arena
            let v = args.pop().ok_or(Flow::Bug("alloc thiếu đối số".into()))?;
            let Some(V::Ref(ar)) = args.pop() else {
                return bug("alloc: người nhận không phải tham chiếu");
            };
            self.use_loan(ar.loan)?;
            self.access(&ar.loc, AK::Read, Some(ar.loan))?;
            self.stats.arena_allocs += 1;
            let fid = self.frames.len();
            self.frames.push(FrameData { locals: vec![v] });
            let loc = Loc {
                fid,
                local: 0,
                path: vec![],
            };
            let loan = self.new_loan(false, loc.clone(), Some(ar.loan), true);
            return Ok(V::Ref(RefV { loc, loan }));
        }
        if key.ends_with("Arena::reset") {
            let Some(V::Ref(ar)) = args.pop() else {
                return bug("reset: người nhận không phải tham chiếu");
            };
            self.use_loan(ar.loan)?;
            self.access(&ar.loc, AK::Write, Some(ar.loan))?;
            self.stats.arena_resets += 1;
            return Ok(V::Unit);
        }
        // Lang item MUTABILITY BÊN TRONG (`Cell`, `Mutex`, `Atomic`; công cụ test, NGOÀI TCB, tuần tự): ghi qua `&self` mà KHÔNG làm vô hiệu
        // khoản vay chung khác. Ngữ nghĩa này không phải bằng chứng cho codegen (ADR-027 §1).
        let cell_like = key.contains("Cell") || key.contains("Mutex") || key.contains("Atomic");
        if cell_like && key.ends_with("::new") {
            let v = args.pop().ok_or(Flow::Bug("new thiếu đối số".into()))?;
            return Ok(V::Adt(0, vec![v]));
        }
        let m = key.rsplit("::").next().unwrap_or("");
        if cell_like
            && matches!(
                m,
                "set" | "get" | "load" | "store" | "swap" | "fetch_add" | "fetch_sub" | "with"
            )
        {
            // args = [người nhận, các đối số…]
            let rest: Vec<V> = args.split_off(1);
            let Some(V::Ref(cr)) = args.pop() else {
                return bug("lang item: người nhận không phải tham chiếu");
            };
            self.use_loan(cr.loan)?;
            self.access(&cr.loc, AK::Read, Some(cr.loan))?;
            let mut loc = cr.loc.clone();
            loc.path.push(Step::Field(0));
            self.stats.cell_ops += 1;
            let cur = self.get(&loc)?;
            return match m {
                "get" | "load" => Ok(cur),
                "set" | "store" | "swap" => {
                    let v = rest
                        .first()
                        .cloned()
                        .ok_or(Flow::Bug("thiếu giá trị".into()))?;
                    self.set(&loc, v)?;
                    Ok(if m == "swap" { cur } else { V::Unit })
                }
                "fetch_add" | "fetch_sub" => {
                    let (V::Int(a, t), Some(V::Int(b, _))) = (&cur, rest.first()) else {
                        return bug("fetch_add/sub trên giá trị không nguyên");
                    };
                    let nv = wrap(if m == "fetch_sub" { a - b } else { a + b }, *t);
                    self.set(&loc, V::Int(nv, *t))?;
                    Ok(cur)
                }
                _ => {
                    // `Mutex::with(&self, f, a)`: chạy `f(&!inner, a)` tại chỗ; `&!` mới là CON của khoản vay trên ổ
                    let (Some(V::Fn(fnode)), Some(arg)) = (rest.first(), rest.get(1)) else {
                        return bug("with: thiếu hàm hoặc đối số");
                    };
                    let Some(cb) = self.mir.bodies.iter().find(|x| x.node == *fnode) else {
                        return bug("with: không có thân MIR của hàm");
                    };
                    let loan = self.new_loan(true, loc.clone(), Some(cr.loan), false);
                    match self.run(cb, vec![V::Ref(RefV { loc, loan }), arg.clone()]) {
                        Outcome::Value(v) => Ok(v),
                        Outcome::Abort(k) => Err(Flow::Abort(k)),
                        Outcome::Violation(v) => Err(Flow::Viol(v)),
                        Outcome::Bug(b) => Err(Flow::Bug(b)),
                        Outcome::OutOfFuel => Err(Flow::Fuel),
                    }
                }
            };
        }
        bug(format!("không có thân MIR cho {key}"))
    }

    fn stmt(&mut self, s: &Statement) -> Result<(), Flow> {
        match &s.kind {
            StatementKind::Assign(p, rv) => {
                let v = self.rvalue(rv)?;
                self.write_place(p, v)
            }
            StatementKind::StorageLive(l) => {
                let f = self.top();
                self.dead.remove(&(f, l.index()));
                self.frames[f].locals[l.index()] = V::Uninit;
                Ok(())
            }
            StatementKind::StorageDead(l) => {
                let f = self.top();
                let loc = Loc {
                    fid: f,
                    local: l.index(),
                    path: vec![],
                };
                self.access(&loc, AK::Dead, None)?;
                self.dead.insert((f, l.index()));
                self.frames[f].locals[l.index()] = V::Uninit;
                Ok(())
            }
            StatementKind::FakeRead(_, p) => {
                let (loc, via) = self.resolve(p)?;
                self.access(&loc, AK::Read, via)
            }
            StatementKind::Nop => Ok(()),
        }
    }

    fn rvalue(&mut self, rv: &Rvalue) -> Result<V, Flow> {
        Ok(match rv {
            Rvalue::Use(o) => self.operand(o)?,
            Rvalue::Repeat(o, n) => V::Array(vec![self.operand(o)?; *n as usize]),
            Rvalue::Ref(_, k, p) => {
                let (loc, via) = self.resolve(p)?;
                // mượn một chỗ đã move/chưa gán là dùng-sau-move
                if Self::has_uninit(&self.get(&loc)?) {
                    return viol(VKind::UseAfterMove, "mượn chỗ đã move/chưa gán");
                }
                let unique = *k == BorrowKind::Unique;
                self.access(
                    &loc,
                    if unique {
                        AK::BorrowUnique
                    } else {
                        AK::BorrowShared
                    },
                    via,
                )?;
                // chỗ đang chứa một Arena ⇒ mọi tham chiếu cấp ra mang dấu Arena
                let from_arena = matches!(self.get(&loc), Ok(V::Arena(_)));
                let loan = self.new_loan(unique, loc.clone(), via, from_arena);
                V::Ref(RefV { loc, loan })
            }
            Rvalue::Len(p) => {
                let (loc, via) = self.resolve(p)?;
                // độ dài mảng cố định là hằng; độ dài lát là một lần đọc
                if matches!(loc.path.last(), Some(Step::Sub(..))) {
                    self.access(&loc, AK::Read, via)?;
                }
                match self.get(&loc)? {
                    V::Array(x) => V::Int(x.len() as i128, IntTy::Usize),
                    other => return bug(format!("Len {other:?}")),
                }
            }
            Rvalue::Discriminant(p) => {
                let (loc, via) = self.resolve(p)?;
                self.access(&loc, AK::Read, via)?;
                match self.get(&loc)? {
                    V::Adt(v, _) => V::Int(i128::from(v), IntTy::Usize),
                    other => return bug(format!("discriminant {other:?}")),
                }
            }
            Rvalue::Cast(o, t) => cast(self.operand(o)?, t, false)?,
            Rvalue::CastFits(o, t) => {
                let v = self.operand(o)?;
                V::Bool(cast(v, t, true).is_ok())
            }
            Rvalue::UnaryOp(op, o) => match (op, self.operand(o)?) {
                (UnOp::Not, V::Bool(b)) => V::Bool(!b),
                (UnOp::Neg, V::Int(i, t)) => V::Int(wrap(-i, t), t),
                (_, o) => return bug(format!("unop trên {o:?}")),
            },
            Rvalue::BinaryOp(op, a, b) => {
                let x = self.operand(a)?;
                let y = self.operand(b)?;
                binop(*op, x, y)?
            }
            Rvalue::CheckedBinaryOp(op, a, b) => {
                let x = self.operand(a)?;
                let y = self.operand(b)?;
                match (x, y) {
                    (V::Int(x, t), V::Int(y, _)) => {
                        let exact = match op {
                            BinOp::Add => x.checked_add(y),
                            BinOp::Sub => x.checked_sub(y),
                            BinOp::Mul => x.checked_mul(y),
                            _ => return bug("checked op lạ"),
                        };
                        match exact {
                            Some(r) if in_range(r, t) => {
                                V::Tuple(vec![V::Int(r, t), V::Bool(false)])
                            }
                            Some(r) => V::Tuple(vec![V::Int(wrap(r, t), t), V::Bool(true)]),
                            None => V::Tuple(vec![V::Int(0, t), V::Bool(true)]),
                        }
                    }
                    (a, b) => return bug(format!("checked trên {a:?} {b:?}")),
                }
            }
            Rvalue::Aggregate(k, ops) => {
                let mut vs = Vec::new();
                for o in ops {
                    vs.push(self.operand(o)?);
                }
                match k {
                    AggregateKind::Tuple => {
                        if vs.is_empty() {
                            V::Unit
                        } else {
                            V::Tuple(vs)
                        }
                    }
                    AggregateKind::Array(_) => V::Array(vs),
                    AggregateKind::Adt(_, var, _) => V::Adt(*var, vs),
                }
            }
        })
    }
}

fn wrap(v: i128, t: IntTy) -> i128 {
    let bits = t.bits();
    if bits == 128 {
        return v;
    }
    let m = (1i128 << bits) - 1;
    let u = v & m;
    if t.signed() && (u >> (bits - 1)) & 1 == 1 {
        u - (1i128 << bits)
    } else {
        u
    }
}

fn in_range(v: i128, t: IntTy) -> bool {
    if t.bits() == 128 {
        return true;
    }
    v >= t.min_val() && (v < 0 || (v as u128) <= t.max_val())
}

fn from_bits(bits: u128, t: IntTy) -> i128 {
    if t.signed() {
        let shift = 128 - t.bits();
        ((bits << shift) as i128) >> shift
    } else {
        bits as i128
    }
}

fn binop(op: BinOp, a: V, b: V) -> Result<V, Flow> {
    Ok(match (a, b) {
        (V::Int(x, t), V::Int(y, _)) => match op {
            BinOp::Add => V::Int(wrap(x + y, t), t),
            BinOp::Sub => V::Int(wrap(x - y, t), t),
            BinOp::Mul => V::Int(wrap(x.wrapping_mul(y), t), t),
            BinOp::Div => {
                if y == 0 {
                    return bug("chia 0 KHÔNG có assert");
                }
                V::Int(wrap(x / y, t), t)
            }
            BinOp::Rem => {
                if y == 0 {
                    return bug("% 0 KHÔNG có assert");
                }
                V::Int(wrap(x % y, t), t)
            }
            BinOp::AddWrap => V::Int(wrap(x.wrapping_add(y), t), t),
            BinOp::SubWrap => V::Int(wrap(x.wrapping_sub(y), t), t),
            BinOp::MulWrap => V::Int(wrap(x.wrapping_mul(y), t), t),
            BinOp::AddSat | BinOp::SubSat | BinOp::MulSat => {
                let r = match op {
                    BinOp::AddSat => x.saturating_add(y),
                    BinOp::SubSat => x.saturating_sub(y),
                    _ => x.saturating_mul(y),
                };
                let lo = t.min_val();
                let hi = i128::try_from(t.max_val()).unwrap_or(i128::MAX);
                V::Int(r.clamp(lo, hi), t)
            }
            BinOp::Eq => V::Bool(x == y),
            BinOp::Ne => V::Bool(x != y),
            BinOp::Lt => V::Bool(x < y),
            BinOp::Le => V::Bool(x <= y),
            BinOp::Gt => V::Bool(x > y),
            BinOp::Ge => V::Bool(x >= y),
            BinOp::BitAnd => return bug("BitAnd trên số"),
        },
        (V::Bool(x), V::Bool(y)) => match op {
            BinOp::Eq => V::Bool(x == y),
            BinOp::Ne => V::Bool(x != y),
            BinOp::BitAnd => V::Bool(x && y),
            _ => return bug("phép bool lạ"),
        },
        (a, b) => match op {
            BinOp::Eq => V::Bool(a == b),
            BinOp::Ne => V::Bool(a != b),
            _ => return bug(format!("binop {op:?} trên {a:?} {b:?}")),
        },
    })
}

fn cast(v: V, t: &Ty, strict: bool) -> Result<V, Flow> {
    match (v, t) {
        (V::Int(x, _), Ty::Int(to)) => {
            if strict && !in_range(x, *to) {
                return bug("mất thông tin");
            }
            Ok(V::Int(wrap(x, *to), *to))
        }
        (V::Bool(b), Ty::Int(to)) => Ok(V::Int(i128::from(b), *to)),
        (V::Char(c), Ty::Int(to)) => {
            if strict && !in_range(i128::from(c), *to) {
                return bug("mất thông tin");
            }
            Ok(V::Int(wrap(i128::from(c), *to), *to))
        }
        (V::Int(x, _), Ty::Char) => Ok(V::Char(u32::try_from(x).unwrap_or(0))),
        (v, t) => bug(format!("ép {v:?} sang {t:?}")),
    }
}
