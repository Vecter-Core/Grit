//! Elaboration drop (ADR-C18 bước 1): biến mỗi `Drop` "vô điều kiện ở cuối phạm vi" mà bước hạ MIR phát thành drop ĐÚNG — chỉ chạy khi
//! chỗ còn khởi tạo — trên cùng phân tích move path / MaybeInit / MaybeUninit mà M6 dùng cho E0401 (một nguồn sự thật về "đã move chưa").
//!
//! Với mỗi `Drop` mà kiểu CÓ THỂ có drop glue (`TypeDb::may_have_glue`: Drop người dùng ở đâu đó bên trong, hoặc tham số kiểu):
//! * chỗ chắc chắn CHƯA khởi tạo (đã move trên mọi đường) ⇒ bỏ drop;
//! * chắc chắn ĐÃ khởi tạo ⇒ giữ drop vô điều kiện;
//! * tuỳ đường ⇒ CỜ DROP: một local `bool` phản ánh đúng bit MaybeInit của move path đó lúc chạy, drop chỉ khi cờ bật;
//! * chỗ bị move MỘT PHẦN (trường của struct/tuple, trường của biến thể enum) ⇒ drop MỞ: drop từng trường còn lại theo thứ tự khai báo
//!   (enum: đọc `Discriminant` rồi chỉ đi vào biến thể đang có). Kiểu có `drop` riêng không bao giờ mở (E0407 cấm move trường ra khỏi nó).
//!
//! Cờ được cập nhật NGAY tại mọi điểm mà phân tích MaybeInit đổi bit của nó (gán, move, `StorageLive/Dead`, đối số/đích của `Call`, `Drop`),
//! theo đúng thứ tự hiệu ứng của phân tích ⇒ giá trị cờ lúc chạy = bit của phân tích trên đường thật sự đi qua. Không có unwind (D3) nên
//! đặt cờ của đích `Call` TRƯỚC lời gọi là tương đương (lời gọi không trở về thì không có drop nào sau nó).
//!
//! Thân hợp đồng `⊳`/`⊲`: tham số là BẢN SAO bit của giá trị thật (M7 truyền `Copy`) ⇒ KHÔNG BAO GIỜ drop tham số; move một tham số có thể
//! có glue ra khỏi hợp đồng (vd truyền theo giá trị cho hàm khác) sẽ drop bản sao ⇒ từ chối (E0407).
use crate::analyses::{MaybeInit, MaybeUninit, MovePaths};
use crate::framework::{BitSet, solve, state_at};
use gt_mir::types::{place_ty, variant_fields};
use gt_mir::*;
use gt_span::Span;
use gt_types::{AdtKind, IntTy, Ty, TypeDb};
use std::collections::{BTreeMap, BTreeSet};

/// Chẩn đoán NGƯỜI DÙNG của elaboration (mã, vị trí, thông báo). Lỗi nội bộ đi qua `Err` (ICE).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropDiag {
    pub code: &'static str,
    pub span: Span,
    pub msg: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cond {
    Always,
    Flag(usize),
}

#[derive(Clone, Debug)]
enum Action {
    Plain(Place, Cond),
    /// enum bị move một phần: điều kiện của cả giá trị, rồi theo biến thể
    Switch(Place, Cond, Vec<(u32, Vec<Action>)>),
}

struct Cx<'a> {
    db: &'a TypeDb,
    mp: &'a MovePaths,
    init: BitSet,
    uninit: BitSet,
    flags: &'a mut BTreeSet<usize>,
}

impl Cx<'_> {
    fn leaf(&mut self, n: usize) -> Option<Cond> {
        if !self.init.contains(n) {
            return None;
        }
        if !self.uninit.contains(n) {
            return Some(Cond::Always);
        }
        self.flags.insert(n);
        Some(Cond::Flag(n))
    }

    fn kids(&self, n: usize) -> Vec<(usize, PlaceElem)> {
        self.mp
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(k, (par, e, _))| match (par, e) {
                (Some(p), Some(e)) if *p == n => Some((k, e.clone())),
                _ => None,
            })
            .collect()
    }

    fn open(
        &mut self,
        n: usize,
        place: Place,
        ty: &Ty,
        out: &mut Vec<Action>,
    ) -> Result<(), String> {
        let kids = self.kids(n);
        let own = matches!(ty, Ty::Adt(d, _) if self.db.user_drops.contains_key(d));
        if kids.is_empty() || own || !matches!(ty, Ty::Tuple(_) | Ty::Adt(..)) {
            if let Some(c) = self.leaf(n) {
                out.push(Action::Plain(place, c));
            }
            return Ok(());
        }
        match ty {
            Ty::Tuple(v) => self.fields(n, &kids, &place, v, out),
            Ty::Adt(d, args) => match self.db.adts.get(d).map(|a| &a.kind) {
                Some(AdtKind::Struct(_)) => {
                    let fs = variant_fields(self.db, *d, 0, args);
                    self.fields(n, &kids, &place, &fs, out)
                }
                Some(AdtKind::Enum(vs)) => {
                    let Some(c) = self.leaf(n) else {
                        return Ok(());
                    };
                    let mut arms = Vec::new();
                    for vi in 0..vs.len() {
                        let vi = u32::try_from(vi).map_err(|_| "quá nhiều biến thể".to_string())?;
                        let fs = variant_fields(self.db, *d, vi, args);
                        let vplace = place.project(PlaceElem::Downcast(vi));
                        let mut acts = Vec::new();
                        match kids.iter().find(|(_, e)| *e == PlaceElem::Downcast(vi)) {
                            Some((dn, _)) => {
                                let dk = self.kids(*dn);
                                self.fields(*dn, &dk, &vplace, &fs, &mut acts)?;
                            }
                            None => self.fields(n, &[], &vplace, &fs, &mut acts)?,
                        }
                        if !acts.is_empty() {
                            arms.push((vi, acts));
                        }
                    }
                    if !arms.is_empty() {
                        out.push(Action::Switch(place, c, arms));
                    }
                    Ok(())
                }
                None => Err(format!("ADT {d:?} không có trong cơ sở kiểu (drop mở)")),
            },
            _ => Err("drop mở trên kiểu không phải tuple/ADT".to_string()),
        }
    }

    fn fields(
        &mut self,
        n: usize,
        kids: &[(usize, PlaceElem)],
        place: &Place,
        fs: &[Ty],
        out: &mut Vec<Action>,
    ) -> Result<(), String> {
        for (i, fty) in fs.iter().enumerate() {
            if !self.db.may_have_glue(fty) {
                continue;
            }
            let i = u32::try_from(i).map_err(|_| "quá nhiều trường".to_string())?;
            let fp = place.project(PlaceElem::Field(i, fty.clone()));
            match kids
                .iter()
                .find(|(_, e)| matches!(e, PlaceElem::Field(j, _) if *j == i))
            {
                Some((k, _)) => self.open(*k, fp, fty, out)?,
                None => {
                    if let Some(c) = self.leaf(n) {
                        out.push(Action::Plain(fp, c));
                    }
                }
            }
        }
        Ok(())
    }
}

fn movable(p: &Place) -> bool {
    p.projection
        .iter()
        .all(|e| matches!(e, PlaceElem::Field(..) | PlaceElem::Downcast(_)))
}

fn moved_places_of_rvalue(rv: &Rvalue) -> Vec<&Place> {
    let ops: Vec<&Operand> = match rv {
        Rvalue::Use(o)
        | Rvalue::Repeat(o, _)
        | Rvalue::Cast(o, _)
        | Rvalue::CastFits(o, _)
        | Rvalue::UnaryOp(_, o) => vec![o],
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

/// Cập nhật cờ theo một thay đổi bit của phân tích: (nút, bật/tắt) cho cả cây con, giữ thứ tự.
fn flag_updates(
    mp: &MovePaths,
    flags: &BTreeMap<usize, Local>,
    node: usize,
    on: bool,
    out: &mut Vec<(Local, bool)>,
) {
    for k in mp.subtree(node) {
        if let Some(l) = flags.get(&k) {
            out.push((*l, on));
        }
    }
}

fn assign_flag(l: Local, on: bool, source: SourceInfo) -> Statement {
    Statement {
        kind: StatementKind::Assign(
            Place::local(l),
            Rvalue::Use(Operand::Const(Const::Bool(on))),
        ),
        source,
    }
}

/// Elaboration drop của một thân. Trả các chẩn đoán người dùng; `Err` = lỗi nội bộ (ICE).
pub fn elaborate_drops(body: &mut Body, db: &TypeDb) -> Result<Vec<DropDiag>, String> {
    let mut diags = Vec::new();
    let glue_ty = |b: &Body, p: &Place| -> Result<bool, String> {
        let ty = place_ty(b, p).ok_or_else(|| format!("kiểu của chỗ drop {p:?} không xác định"))?;
        Ok(db.may_have_glue(&ty))
    };
    // ---- thân hợp đồng: tham số là bản sao bit — không drop, không move ra
    if matches!(body.kind, BodyKind::Pre | BodyKind::Post) {
        let is_arg = |l: Local| l.index() >= 1 && l.index() <= body.arg_count;
        let mut bad: Vec<Span> = Vec::new();
        for bd in &body.blocks {
            for s in &bd.statements {
                if let StatementKind::Assign(_, rv) = &s.kind {
                    for p in moved_places_of_rvalue(rv) {
                        if is_arg(p.local) && glue_ty(body, p)? {
                            bad.push(s.source.span);
                        }
                    }
                }
            }
            if let Some(Terminator {
                kind: TerminatorKind::Call { args, .. },
                source,
            }) = &bd.terminator
            {
                for a in args {
                    if let Operand::Move(p) = a
                        && is_arg(p.local)
                        && glue_ty(body, p)?
                    {
                        bad.push(source.span);
                    }
                }
            }
        }
        for span in bad {
            diags.push(DropDiag {
                code: "E0407",
                span,
                msg: "hợp đồng không được move giá trị có thể có `Drop` ra khỏi tham số (hợp đồng chỉ nhìn bản sao của giá trị thật; ADR-C18)".to_string(),
            });
        }
        let arg_count = body.arg_count;
        for bd in &mut body.blocks {
            if let Some(t) = &mut bd.terminator
                && let TerminatorKind::Drop { place, target } = &t.kind
                && place.local.index() >= 1
                && place.local.index() <= arg_count
            {
                t.kind = TerminatorKind::Goto(*target);
            }
        }
    }
    // ---- có drop nào cần elaboration không?
    let mut todo: Vec<usize> = Vec::new();
    for (bi, bd) in body.blocks.iter().enumerate() {
        if let Some(Terminator {
            kind: TerminatorKind::Drop { place, .. },
            ..
        }) = &bd.terminator
            && glue_ty(body, place)?
        {
            todo.push(bi);
        }
    }
    if todo.is_empty() {
        body.drops_elaborated = true;
        return Ok(diags);
    }
    let mp = MovePaths::build(body);
    let mi_a = MaybeInit { paths: &mp };
    let mu_a = MaybeUninit { paths: &mp };
    let mi = solve(&mi_a, body)?;
    let mu = solve(&mu_a, body)?;
    // ---- quyết định cho từng drop
    let mut flag_nodes: BTreeSet<usize> = BTreeSet::new();
    let mut plans: Vec<(usize, Vec<Action>, Place, BasicBlock, SourceInfo)> = Vec::new();
    for &bi in &todo {
        let bd = &body.blocks[bi];
        let Some(Terminator {
            kind: TerminatorKind::Drop { place, target },
            source,
        }) = &bd.terminator
        else {
            return Err("mất terminator Drop".to_string());
        };
        let loc = Location {
            block: BasicBlock(u32::try_from(bi).map_err(|_| "quá nhiều khối")?),
            statement: bd.statements.len(),
        };
        let mut acts = Vec::new();
        if let Some(ty) = place_ty(body, place)
            && !ty.has_param()
            && !ty.has_infer()
            && let Err(why) = db.drop_glue(&ty)
        {
            // kiểu cụ thể mà glue không đi vào được (kiểu lang đục chứa Drop người dùng): từ chối, không âm thầm bỏ drop
            diags.push(DropDiag {
                code: "E0318",
                span: source.span,
                msg: format!("giá trị này không drop được: {why}"),
            });
        }
        if movable(place) {
            let ty = place_ty(body, place).ok_or("kiểu chỗ drop")?;
            let mut cx = Cx {
                db,
                mp: &mp,
                init: state_at(&mi_a, body, &mi, loc),
                uninit: state_at(&mu_a, body, &mu, loc),
                flags: &mut flag_nodes,
            };
            let n = mp.find(place);
            cx.open(n, place.clone(), &ty, &mut acts)?;
        } else {
            // qua `*`/chỉ số: không move ra được ⇒ luôn khởi tạo
            acts.push(Action::Plain(place.clone(), Cond::Always));
        }
        plans.push((bi, acts, place.clone(), *target, *source));
    }
    // ---- cấp local cờ
    let mut flags: BTreeMap<usize, Local> = BTreeMap::new();
    let span0 = body.span;
    for &n in &flag_nodes {
        let l = Local(u32::try_from(body.locals.len()).map_err(|_| "quá nhiều local")?);
        body.locals.push(LocalDecl {
            ty: Ty::Bool,
            regions: Vec::new(),
            kind: LocalKind::Temp,
            mutable: true,
            span: span0,
        });
        flags.insert(n, l);
    }
    let mut new_blocks: Vec<BlockData> = Vec::new();
    let base = body.blocks.len();
    let fresh_local = |locals: &mut Vec<LocalDecl>, ty: Ty| -> Local {
        locals.push(LocalDecl {
            ty,
            regions: Vec::new(),
            kind: LocalKind::Temp,
            mutable: true,
            span: span0,
        });
        Local(u32::try_from(locals.len() - 1).unwrap_or(u32::MAX))
    };
    // ---- cập nhật cờ tại mọi điểm đổi bit (trên các khối GỐC, trước khi viết lại drop)
    if !flags.is_empty() {
        let elaborated: BTreeSet<usize> = plans.iter().map(|p| p.0).collect();
        for (bi, bd) in body.blocks.iter_mut().enumerate() {
            let mut stmts: Vec<Statement> = Vec::with_capacity(bd.statements.len());
            for s in std::mem::take(&mut bd.statements) {
                let mut ups: Vec<(Local, bool)> = Vec::new();
                match &s.kind {
                    StatementKind::Assign(p, rv) => {
                        for m in moved_places_of_rvalue(rv) {
                            flag_updates(&mp, &flags, mp.find(m), false, &mut ups);
                        }
                        if !p.has_deref() {
                            flag_updates(&mp, &flags, mp.find(p), true, &mut ups);
                        }
                    }
                    StatementKind::StorageLive(l) | StatementKind::StorageDead(l) => {
                        flag_updates(&mp, &flags, mp.roots[l.index()], false, &mut ups);
                    }
                    StatementKind::FakeRead(..) | StatementKind::Nop => {}
                }
                let src = s.source;
                stmts.push(s);
                stmts.extend(ups.into_iter().map(|(l, on)| assign_flag(l, on, src)));
            }
            if let Some(t) = &bd.terminator {
                let mut ups: Vec<(Local, bool)> = Vec::new();
                match &t.kind {
                    TerminatorKind::Call {
                        args, callee, dest, ..
                    } => {
                        for a in args {
                            if let Operand::Move(p) = a
                                && !p.has_deref()
                            {
                                flag_updates(&mp, &flags, mp.find(p), false, &mut ups);
                            }
                        }
                        if let Callee::Value(Operand::Move(p)) = callee
                            && !p.has_deref()
                        {
                            flag_updates(&mp, &flags, mp.find(p), false, &mut ups);
                        }
                        if !dest.has_deref() {
                            flag_updates(&mp, &flags, mp.find(dest), true, &mut ups);
                        }
                    }
                    // drop không elaboration (kiểu không có glue): bit tắt như phân tích, không có gì chạy nên đặt trước là tương đương
                    TerminatorKind::Drop { place, .. }
                        if !place.has_deref() && !elaborated.contains(&bi) =>
                    {
                        flag_updates(&mp, &flags, mp.find(place), false, &mut ups);
                    }
                    _ => {}
                }
                let src = t.source;
                stmts.extend(ups.into_iter().map(|(l, on)| assign_flag(l, on, src)));
            }
            bd.statements = stmts;
        }
        // giá trị đầu: như `initialize_start` của MaybeInit (tham số khởi tạo, còn lại chưa)
        let src0 = body.blocks[0]
            .statements
            .first()
            .map(|s| s.source)
            .or_else(|| body.blocks[0].terminator.as_ref().map(|t| t.source))
            .ok_or("bb0 rỗng")?;
        let mut init: Vec<Statement> = Vec::new();
        for (n, l) in &flags {
            let local = mp.nodes[*n].2.index();
            init.push(Statement {
                kind: StatementKind::StorageLive(*l),
                source: src0,
            });
            init.push(assign_flag(*l, local >= 1 && local <= body.arg_count, src0));
        }
        body.blocks[0].statements.splice(0..0, init);
        // cờ sống suốt thân: chết ngay trước mỗi `Return` (validator đòi không còn storage sống khi trả về)
        for bd in &mut body.blocks {
            if let Some(t) = &bd.terminator
                && matches!(t.kind, TerminatorKind::Return)
            {
                let src = t.source;
                for l in flags.values() {
                    bd.statements.push(Statement {
                        kind: StatementKind::StorageDead(*l),
                        source: src,
                    });
                }
            }
        }
    }
    // ---- viết lại từng drop thành chuỗi khối
    let mut next_id = base;
    let mut alloc = |blocks: &mut Vec<BlockData>, bd: BlockData| -> BasicBlock {
        blocks.push(bd);
        next_id += 1;
        BasicBlock(u32::try_from(next_id - 1).unwrap_or(u32::MAX))
    };
    for (bi, acts, place, target, source) in plans {
        // khối cuối: tắt cờ của cây con chỗ vừa drop (hiệu ứng `Drop` của phân tích), rồi về đích cũ
        let mut tail = target;
        if movable(&place) {
            let mut ups = Vec::new();
            flag_updates(&mp, &flags, mp.find(&place), false, &mut ups);
            if !ups.is_empty() {
                tail = alloc(
                    &mut new_blocks,
                    BlockData {
                        statements: ups
                            .into_iter()
                            .map(|(l, on)| assign_flag(l, on, source))
                            .collect(),
                        terminator: Some(Terminator {
                            kind: TerminatorKind::Goto(target),
                            source,
                        }),
                    },
                );
            }
        }
        let entry = emit(
            &acts,
            tail,
            source,
            &flags,
            &mut |bd| alloc(&mut new_blocks, bd),
            &mut |ty| fresh_local(&mut body.locals, ty),
        )?;
        if let Some(t) = &mut body.blocks[bi].terminator {
            t.kind = TerminatorKind::Goto(entry);
        }
    }
    body.blocks.extend(new_blocks);
    body.drops_elaborated = true;
    Ok(diags)
}

fn term(kind: TerminatorKind, source: SourceInfo) -> Option<Terminator> {
    Some(Terminator { kind, source })
}

/// Phát chuỗi hành động kết thúc ở `next`; trả khối vào.
fn emit(
    acts: &[Action],
    next: BasicBlock,
    source: SourceInfo,
    flags: &BTreeMap<usize, Local>,
    alloc: &mut dyn FnMut(BlockData) -> BasicBlock,
    local: &mut dyn FnMut(Ty) -> Local,
) -> Result<BasicBlock, String> {
    let mut next = next;
    for a in acts.iter().rev() {
        let (body_entry, cond) = match a {
            Action::Plain(p, c) => (
                alloc(BlockData {
                    statements: Vec::new(),
                    terminator: term(
                        TerminatorKind::Drop {
                            place: p.clone(),
                            target: next,
                        },
                        source,
                    ),
                }),
                *c,
            ),
            Action::Switch(p, c, arms) => {
                let d = local(Ty::Int(IntTy::Usize));
                // mỗi cạnh ra khỏi phép chọn đi qua một khối kết thúc storage của biến thẻ
                let dead = |to: BasicBlock| BlockData {
                    statements: vec![Statement {
                        kind: StatementKind::StorageDead(d),
                        source,
                    }],
                    terminator: term(TerminatorKind::Goto(to), source),
                };
                let mut targets = Vec::new();
                for (vi, acts) in arms {
                    let e = emit(acts, next, source, flags, alloc, local)?;
                    targets.push((u128::from(*vi), alloc(dead(e))));
                }
                let otherwise = alloc(dead(next));
                let sw = alloc(BlockData {
                    statements: vec![
                        Statement {
                            kind: StatementKind::StorageLive(d),
                            source,
                        },
                        Statement {
                            kind: StatementKind::Assign(
                                Place::local(d),
                                Rvalue::Discriminant(p.clone()),
                            ),
                            source,
                        },
                    ],
                    terminator: term(
                        TerminatorKind::SwitchInt {
                            discr: Operand::Copy(Place::local(d)),
                            targets,
                            otherwise,
                        },
                        source,
                    ),
                });
                (sw, *c)
            }
        };
        next = match cond {
            Cond::Always => body_entry,
            Cond::Flag(n) => {
                let l = *flags.get(&n).ok_or("cờ drop chưa được cấp")?;
                alloc(BlockData {
                    statements: Vec::new(),
                    terminator: term(
                        TerminatorKind::SwitchInt {
                            discr: Operand::Copy(Place::local(l)),
                            targets: vec![(0, next)],
                            otherwise: body_entry,
                        },
                        source,
                    ),
                })
            }
        };
    }
    Ok(next)
}
