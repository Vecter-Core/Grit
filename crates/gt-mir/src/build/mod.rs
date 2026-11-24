//! Bước hạ MIR (ADR-020 §2, §9): nơi DUY NHẤT đọc AST + các bảng của `TypesOutput`. Thân generic hạ đúng một lần.
//!
//! Phạm vi (frame): mỗi khối là một frame "khối" (biến người dùng), mỗi câu lệnh là một frame "lệnh" (biến tạm).
//! Ra khỏi frame — bình thường hay qua `⊠ ⊡ ⏎` — phát `Drop` (nếu cần) rồi `StorageDead` cho từng biến theo thứ tự NGƯỢC
//! khai báo. Lỗi bất thường (cấu trúc chưa hỗ trợ, bảng kiểu thiếu) là ICE: không bao giờ bỏ qua hàm.
mod expr;
mod matching;
pub use matching::{
    set_pat_field_fault_for_test, set_pat_mode_fault_for_test, set_pat_swap_fault_for_test,
};

use crate::mir::*;
use crate::types::region_slots;
use gt_ast::{Block, Expr, ExprKind, FnItem, NodeId, ParamKind, SelfKind};
use gt_resolve::{LocalId, Res};
use gt_span::Span;
use gt_types::{Bound, Ty, TypeDb, TypesOutput};
use std::collections::BTreeMap;

/// Lỗi nội bộ khi hạ (thành ICE E9901).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildError(pub String);

pub(crate) type R<T> = Result<T, BuildError>;

pub(crate) fn ice<T>(msg: impl Into<String>) -> R<T> {
    Err(BuildError(msg.into()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FrameKind {
    /// Tham số của hàm: không có `StorageLive/Dead`, chỉ `Drop` khi trả về.
    Args,
    Block,
    Stmt,
}

pub(super) struct Frame {
    kind: FrameKind,
    /// (local, cần drop) theo thứ tự khai báo.
    locals: Vec<(Local, bool)>,
}

struct LoopCx {
    /// Số frame đang mở khi vào vòng lặp (thoát tới mức này khi `⊠`/`⊡`).
    depth: usize,
    break_bb: BasicBlock,
    continue_bb: BasicBlock,
}

pub(crate) struct Builder<'t> {
    pub(crate) t: &'t TypesOutput,
    pub(crate) db: &'t TypeDb,
    pub(crate) body: Body,
    pub(crate) cur: BasicBlock,
    scope: ScopeId,
    frames: Vec<Frame>,
    loops: Vec<LoopCx>,
    pub(crate) vars: BTreeMap<LocalId, Local>,
    bounds: Vec<Bound>,
    /// literal nguyên ngoài phạm vi kiểu gặp khi hạ (ghi qua `&self`)
    pub(crate) bad_lits: std::cell::RefCell<Vec<BadLiteral>>,
    /// đang hạ một biểu thức hợp đồng (thân `⊳`/`⊲` hay `⋈`): `∀/∃` chỉ hạ được ở đây
    pub(crate) in_contract: u32,
    /// dữ liệu vòng lặp đang hạ (`register_head` lấy ra khi biết khối đầu vòng)
    pub(crate) pending_loop: Option<(Span, NodeId, Option<Expr>, Option<Expr>)>,
    /// Nút AST đang hạ (cho `SourceInfo`).
    pub(crate) at: (Span, NodeId),
    /// nút đang được ép unsize (chặn đệ quy: phần hạ bên trong của chính nút đó không ép lại)
    pub(crate) unsize_busy: std::collections::BTreeSet<NodeId>,
}

impl<'t> Builder<'t> {
    fn new(t: &'t TypesOutput, key: String, node: NodeId, kind: BodyKind, span: Span) -> Self {
        let body = Body {
            key,
            node,
            sig_node: node,
            kind,
            arg_count: 0,
            locals: Vec::new(),
            blocks: vec![BlockData {
                statements: Vec::new(),
                terminator: None,
            }],
            borrows: Vec::new(),
            scopes: vec![SourceScope {
                parent: None,
                unsafe_ctx: false,
            }],
            region_count: 0,
            contracts: Vec::new(),
            fall_off: None,
            span,
            bad_literals: Vec::new(),
            uncheckable: Vec::new(),
            loops: Vec::new(),
            var_map: Vec::new(),
            drops_elaborated: false,
        };
        Builder {
            t,
            db: &t.db,
            body,
            cur: ENTRY,
            scope: ScopeId(0),
            frames: Vec::new(),
            loops: Vec::new(),
            vars: BTreeMap::new(),
            bounds: t.scope_of(node),
            bad_lits: std::cell::RefCell::new(Vec::new()),
            in_contract: 0,
            pending_loop: None,
            at: (span, node),
            unsize_busy: std::collections::BTreeSet::new(),
        }
    }

    // ---------------- tiện ích dựng ----------------

    pub(crate) fn src(&self) -> SourceInfo {
        SourceInfo {
            span: self.at.0,
            node: self.at.1,
            scope: self.scope,
        }
    }

    pub(crate) fn new_block(&mut self) -> BasicBlock {
        self.body.blocks.push(BlockData {
            statements: Vec::new(),
            terminator: None,
        });
        BasicBlock(u32::try_from(self.body.blocks.len() - 1).unwrap_or(u32::MAX))
    }

    pub(crate) fn push(&mut self, kind: StatementKind) {
        let s = Statement {
            kind,
            source: self.src(),
        };
        self.body.blocks[self.cur.index()].statements.push(s);
    }

    pub(crate) fn assign(&mut self, p: Place, rv: Rvalue) {
        self.push(StatementKind::Assign(p, rv));
    }

    /// Kết thúc khối hiện tại. Khối đã có terminator (vd mã sau `⏎` trong cùng khối) là lỗi nội bộ.
    pub(crate) fn terminate(&mut self, kind: TerminatorKind) -> R<()> {
        let src = self.src();
        let b = &mut self.body.blocks[self.cur.index()];
        if b.terminator.is_some() {
            return ice(format!("khối bb{} đã có terminator", self.cur.0));
        }
        b.terminator = Some(Terminator { kind, source: src });
        Ok(())
    }

    pub(crate) fn goto(&mut self, target: BasicBlock) -> R<()> {
        self.terminate(TerminatorKind::Goto(target))?;
        self.cur = target;
        Ok(())
    }

    /// Sau một điểm phân kỳ: tiếp tục ở một khối mới không có cạnh vào (mã chết).
    pub(crate) fn diverge(&mut self) {
        self.cur = self.new_block();
    }

    fn fresh_region(&mut self) -> RegionVid {
        let r = RegionVid(self.body.region_count);
        self.body.region_count += 1;
        r
    }

    fn decl(&mut self, ty: Ty, kind: LocalKind, mutable: bool, span: Span) -> Local {
        let n = region_slots(self.db, &ty);
        let regions = (0..n).map(|_| self.fresh_region()).collect();
        self.body.locals.push(LocalDecl {
            ty,
            regions,
            kind,
            mutable,
            span,
        });
        Local(u32::try_from(self.body.locals.len() - 1).unwrap_or(u32::MAX))
    }

    pub(crate) fn needs_drop(&self, ty: &Ty) -> bool {
        self.db.needs_drop(ty)
    }

    pub(crate) fn is_copy(&self, ty: &Ty) -> bool {
        self.db.is_copy_resolved(ty, &self.bounds)
    }

    /// Biến tạm trong frame lệnh hiện tại (sống tới cuối câu lệnh).
    pub(crate) fn temp(&mut self, ty: Ty) -> Local {
        self.temp_in(ty, FrameKind::Stmt)
    }

    /// Biến tạm sống tới cuối KHỐI chứa câu lệnh (luật kéo dài tạm của `≔ &V`, ADR-020 §9).
    pub(crate) fn extended_temp(&mut self, ty: Ty) -> Local {
        self.temp_in(ty, FrameKind::Block)
    }

    fn temp_in(&mut self, ty: Ty, want: FrameKind) -> Local {
        let nd = self.needs_drop(&ty);
        let l = self.decl(ty, LocalKind::Temp, true, self.at.0);
        self.push(StatementKind::StorageLive(l));
        self.register(l, nd, want);
        l
    }

    fn register(&mut self, l: Local, nd: bool, want: FrameKind) {
        // frame gần nhất có loại `want`; không có (vd tạm ở mức hàm) thì frame trên cùng
        let idx = self
            .frames
            .iter()
            .rposition(|f| f.kind == want)
            .unwrap_or(self.frames.len().saturating_sub(1));
        if let Some(f) = self.frames.get_mut(idx) {
            f.locals.push((l, nd));
        }
    }

    /// Biến người dùng (từ `≔`, pattern, biến vòng lặp): StorageLive + đăng ký trong frame khối gần nhất.
    pub(crate) fn user_var(&mut self, id: LocalId, mutable: bool, span: Span) -> R<Local> {
        let Some((_, name, ty)) = self.t.locals.get(&id).cloned() else {
            return ice(format!("thiếu kiểu của biến cục bộ {id:?}"));
        };
        let nd = self.needs_drop(&ty);
        let l = self.decl(ty, LocalKind::User { name, id }, mutable, span);
        self.push(StatementKind::StorageLive(l));
        self.register(l, nd, FrameKind::Block);
        self.vars.insert(id, l);
        Ok(l)
    }

    pub(crate) fn new_borrow(&mut self, kind: BorrowKind, place: Place) -> Rvalue {
        let region = self.fresh_region();
        let id = BorrowId(u32::try_from(self.body.borrows.len()).unwrap_or(u32::MAX));
        self.body.borrows.push(BorrowData {
            place: place.clone(),
            kind,
            region,
            source: self.src(),
        });
        Rvalue::Ref(id, kind, place)
    }

    // ---------------- frame ----------------

    fn push_frame(&mut self, kind: FrameKind) {
        self.frames.push(Frame {
            kind,
            locals: Vec::new(),
        });
    }

    /// Phát drop + StorageDead cho các frame từ trên cùng xuống tới (không gồm) mức `depth`, KHÔNG gỡ frame.
    fn exit_to(&mut self, depth: usize) -> R<()> {
        let mut todo: Vec<(Local, bool, bool)> = Vec::new();
        for f in self.frames[depth..].iter().rev() {
            let storage = f.kind != FrameKind::Args;
            for (l, nd) in f.locals.iter().rev() {
                todo.push((*l, *nd, storage));
            }
        }
        for (l, nd, storage) in todo {
            if nd {
                let next = self.new_block();
                self.terminate(TerminatorKind::Drop {
                    place: Place::local(l),
                    target: next,
                })?;
                self.cur = next;
            }
            if storage {
                self.push(StatementKind::StorageDead(l));
            }
        }
        Ok(())
    }

    fn pop_frame(&mut self) -> R<()> {
        let d = self.frames.len().saturating_sub(1);
        self.exit_to(d)?;
        self.frames.pop();
        Ok(())
    }

    /// Chạy `f` trong một frame lệnh (biến tạm chết khi xong).
    pub(crate) fn in_stmt<T>(&mut self, f: impl FnOnce(&mut Self) -> R<T>) -> R<T> {
        self.push_frame(FrameKind::Stmt);
        let r = f(self)?;
        self.pop_frame()?;
        Ok(r)
    }

    pub(crate) fn in_block<T>(&mut self, f: impl FnOnce(&mut Self) -> R<T>) -> R<T> {
        self.push_frame(FrameKind::Block);
        let r = f(self)?;
        self.pop_frame()?;
        Ok(r)
    }

    pub(crate) fn enter_unsafe(&mut self) -> ScopeId {
        let old = self.scope;
        self.body.scopes.push(SourceScope {
            parent: Some(old),
            unsafe_ctx: true,
        });
        self.scope = ScopeId(u32::try_from(self.body.scopes.len() - 1).unwrap_or(u32::MAX));
        old
    }

    pub(crate) fn leave_scope(&mut self, old: ScopeId) {
        self.scope = old;
    }

    // ---------------- điều khiển ----------------

    pub(crate) fn enter_loop(&mut self, break_bb: BasicBlock, continue_bb: BasicBlock) {
        self.loops.push(LoopCx {
            depth: self.frames.len(),
            break_bb,
            continue_bb,
        });
    }

    pub(crate) fn leave_loop(&mut self) {
        self.loops.pop();
    }

    pub(crate) fn break_or_continue(&mut self, is_break: bool) -> R<()> {
        let Some(l) = self.loops.last() else {
            return ice("⊠/⊡ ngoài vòng lặp sau M4");
        };
        let (depth, target) = (l.depth, if is_break { l.break_bb } else { l.continue_bb });
        self.exit_to(depth)?;
        self.terminate(TerminatorKind::Goto(target))?;
        self.diverge();
        Ok(())
    }

    /// `⏎ e`: ghi `_0`, thoát MỌI frame (kể cả drop tham số), `return`.
    pub(crate) fn ret(&mut self, e: Option<&Expr>) -> R<()> {
        match e {
            Some(e) if self.is_unique_ref_place(e)? => {
                let op = self.as_coerced_operand(e)?;
                self.assign(Place::local(RETURN_PLACE), Rvalue::Use(op));
            }
            Some(e) => self.expr_into(&Place::local(RETURN_PLACE), e)?,
            None => self.assign(
                Place::local(RETURN_PLACE),
                Rvalue::Use(Operand::Const(Const::Unit)),
            ),
        }
        self.exit_to(0)?;
        self.terminate(TerminatorKind::Return)?;
        self.diverge();
        Ok(())
    }

    pub(crate) fn ty_of(&self, e: &Expr) -> R<Ty> {
        match self.t.types.get(&e.id) {
            Some(t) => Ok(t.clone()),
            None => ice(format!("thiếu kiểu của biểu thức {:?}", e.id)),
        }
    }

    pub(crate) fn res_of(&self, id: NodeId) -> Option<Res> {
        self.t.resolve.uses.get(&id).cloned()
    }

    // ---------------- kết thúc ----------------

    /// Đóng các khối còn mở bằng `Unreachable` — chỉ hợp lệ nếu chúng KHÔNG tới được (nếu tới được: lỗi nội bộ).
    fn finish(mut self) -> R<Body> {
        let mut bad = std::mem::take(&mut *self.bad_lits.borrow_mut());
        bad.sort_by_key(|b| (b.span.start, b.span.end, b.node));
        bad.dedup_by_key(|b| b.node);
        self.body.bad_literals = bad;
        self.body.var_map = self.vars.iter().map(|(k, v)| (*k, *v)).collect();
        let open: Vec<usize> = self
            .body
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.terminator.is_none())
            .map(|(i, _)| i)
            .collect();
        let src = SourceInfo {
            span: self.body.span,
            node: self.body.node,
            scope: ScopeId(0),
        };
        for i in &open {
            self.body.blocks[*i].terminator = Some(Terminator {
                kind: TerminatorKind::Unreachable,
                source: src,
            });
        }
        let reach = crate::validate::reachable(&self.body);
        for i in open {
            if reach[i] && self.body.fall_off.map(|(b, _)| b.index()) != Some(i) {
                return ice(format!("khối bb{i} tới được nhưng chưa có terminator"));
            }
        }
        prune_unreachable(&mut self.body, &reach);
        Ok(self.body)
    }
}

/// Hạ một hàm (tự do hoặc thành viên impl).
// 8 tham số: mỗi cái là một dữ kiện độc lập của thân cần dựng (hàm hay hợp đồng)
#[allow(clippy::too_many_arguments)]
fn start_fn<'t>(
    t: &'t TypesOutput,
    key: &str,
    node: NodeId,
    f: &FnItem,
    self_ty: Option<Ty>,
    sig_node: NodeId,
    kind: BodyKind,
    ret_ty: Option<Ty>,
) -> R<(Builder<'t>, Ty)> {
    let Some(sig) = t.db.sig_by_node(sig_node).cloned() else {
        return ice(format!("thiếu chữ ký của {key}"));
    };
    let mut b = Builder::new(t, key.to_string(), node, kind, f.body.span);
    b.decl(
        ret_ty.unwrap_or_else(|| sig.ret.clone()),
        LocalKind::Return,
        true,
        f.body.span,
    );
    // tham số: `self` trước (nếu có), rồi các tham số có tên theo thứ tự
    b.push_frame(FrameKind::Args);
    let mut named = sig.params.iter();
    for p in &f.sig.params {
        match &p.kind {
            ParamKind::SelfParam(k) => {
                let Some(st) = self_ty.clone() else {
                    return ice("`self` ngoài impl");
                };
                let ty = match k {
                    SelfKind::Value => st,
                    SelfKind::Ref => Ty::Ref(false, Box::new(st)),
                    SelfKind::RefMut => Ty::Ref(true, Box::new(st)),
                };
                let nd = b.needs_drop(&ty);
                let l = b.decl(ty, LocalKind::Arg(None), false, p.span);
                b.register(l, nd, FrameKind::Args);
            }
            ParamKind::Named { mutable, name, .. } => {
                let Some(ty) = named.next().cloned() else {
                    return ice("số tham số không khớp chữ ký");
                };
                let nd = b.needs_drop(&ty);
                let l = b.decl(
                    ty,
                    LocalKind::Arg(Some(name.name.clone())),
                    *mutable,
                    p.span,
                );
                b.register(l, nd, FrameKind::Args);
                if let Some(&id) = t.resolve.decls.get(&name.id) {
                    b.vars.insert(id, l);
                }
            }
        }
    }
    Ok((b, sig.ret.clone()))
}

/// Hạ một hàm (tự do hoặc thành viên impl).
pub(crate) fn build_fn(
    t: &TypesOutput,
    key: &str,
    node: NodeId,
    f: &FnItem,
    self_ty: Option<Ty>,
) -> R<Body> {
    let (mut b, ret) = start_fn(t, key, node, f, self_ty, node, BodyKind::Fn, None)?;
    b.body.arg_count = b.body.locals.len() - 1;
    for c in &f.sig.contracts {
        b.body.contracts.push(c.id);
    }
    let body_block = &f.body;
    // thân không có đuôi (hoặc đuôi là `⎇` thiếu `⇝`) mà kiểu trả về ≠ `()`: lối rơi cuối là E0307 nếu tới được
    let open_end = body_block.tail.as_ref().is_none_or(|t| {
        matches!(
            t.kind,
            ExprKind::If {
                else_branch: None,
                ..
            }
        )
    });
    if open_end && !ret.is_unit() {
        // giá trị `()` của thân nằm trong một tạm của frame khối riêng (StorageDead trên MỌI đường thoát)
        b.in_block(|b| {
            let scratch = b.extended_temp(Ty::unit());
            b.lower_block(&Place::local(scratch), body_block)
        })?;
        b.exit_to(0)?;
        let end = Span::new(body_block.span.end.saturating_sub(1), body_block.span.end);
        b.body.fall_off = Some((b.cur, end));
        b.terminate(TerminatorKind::Unreachable)?;
        b.diverge();
    } else {
        b.lower_block(&Place::local(RETURN_PLACE), body_block)?;
        b.exit_to(0)?;
        b.terminate(TerminatorKind::Return)?;
        b.diverge();
    }
    b.finish()
}

/// Hạ một hợp đồng `⊳`/`⊲` thành thân hàm thuần trả `bool` (ADR-025 §6): tham số = tham số của hàm (cùng chỉ số local),
/// riêng `⊲` có thêm tham số cuối `result` (giá trị trả về). Thân là chính biểu thức hợp đồng.
pub(crate) fn build_contract(
    t: &TypesOutput,
    key: &str,
    fn_node: NodeId,
    f: &FnItem,
    self_ty: Option<Ty>,
    c: &gt_ast::Contract,
) -> R<Body> {
    let kind = if c.kind == gt_ast::ContractKind::Post {
        BodyKind::Post
    } else {
        BodyKind::Pre
    };
    let (mut b, ret) = start_fn(t, key, c.id, f, self_ty, fn_node, kind, Some(Ty::Bool))?;
    b.body.sig_node = fn_node;
    if c.kind == gt_ast::ContractKind::Post {
        let nd = b.needs_drop(&ret);
        let l = b.decl(ret, LocalKind::Arg(Some("result".into())), false, c.span);
        b.register(l, nd, FrameKind::Args);
        if let Some(&id) = t.resolve.decls.get(&c.id) {
            b.vars.insert(id, l);
        }
    }
    b.body.arg_count = b.body.locals.len() - 1;
    b.in_contract += 1;
    b.in_stmt(|b| b.expr_into(&Place::local(RETURN_PLACE), &c.expr))?;
    b.in_contract -= 1;
    b.exit_to(0)?;
    b.terminate(TerminatorKind::Return)?;
    b.diverge();
    b.finish()
}

/// Hạ `□`/`■`: thân không tham số trả giá trị khởi tạo.
pub(crate) fn build_const(
    t: &TypesOutput,
    key: &str,
    node: NodeId,
    kind: BodyKind,
    ty: Ty,
    value: &Expr,
) -> R<Body> {
    let mut b = Builder::new(t, key.to_string(), node, kind, value.span);
    b.decl(ty, LocalKind::Return, true, value.span);
    b.push_frame(FrameKind::Args);
    b.in_stmt(|b| b.expr_into(&Place::local(RETURN_PLACE), value))?;
    b.exit_to(0)?;
    b.terminate(TerminatorKind::Return)?;
    b.diverge();
    b.finish()
}

impl Builder<'_> {
    /// Hạ một khối vào `dest` (frame khối riêng).
    pub(crate) fn lower_block(&mut self, dest: &Place, blk: &Block) -> R<()> {
        self.in_block(|b| {
            for s in &blk.stmts {
                b.lower_stmt(s)?;
            }
            match &blk.tail {
                Some(t) => b.in_stmt(|b| b.expr_into(dest, t)),
                None => {
                    // khối không đuôi có kiểu `()` (hoặc `!` — khi đó đích không phải `()` và khối hiện tại là mã chết)
                    b.at = (blk.span, blk.id);
                    b.assign_unit_if_unit(dest)
                }
            }
        })
    }
}

/// Bỏ các khối không tới được (mã sau `⏎ ⊠ ⊡`, sau lời gọi `→ !`) và đánh số lại khối + khoản mượn theo thứ tự cũ,
/// để các pass sau (M6) chỉ thấy CFG tới được. Lối rơi cuối hàm không tới được thì bỏ luôn (không có E0307).
fn prune_unreachable(body: &mut Body, reach: &[bool]) {
    let mut map: Vec<Option<BasicBlock>> = vec![None; body.blocks.len()];
    let mut n = 0u32;
    for (i, r) in reach.iter().enumerate() {
        if *r {
            map[i] = Some(BasicBlock(n));
            n += 1;
        }
    }
    let m = |b: BasicBlock| map[b.index()].unwrap_or(b);
    let old = std::mem::take(&mut body.blocks);
    for (i, mut bb) in old.into_iter().enumerate() {
        if !reach[i] {
            continue;
        }
        if let Some(t) = &mut bb.terminator {
            match &mut t.kind {
                TerminatorKind::Goto(x) => *x = m(*x),
                TerminatorKind::SwitchInt {
                    targets, otherwise, ..
                } => {
                    for (_, x) in targets.iter_mut() {
                        *x = m(*x);
                    }
                    *otherwise = m(*otherwise);
                }
                TerminatorKind::Drop { target, .. } | TerminatorKind::Assert { target, .. } => {
                    *target = m(*target);
                }
                TerminatorKind::Call { target, .. } => {
                    if let Some(x) = target {
                        *x = m(*x);
                    }
                }
                TerminatorKind::Return | TerminatorKind::Unreachable => {}
            }
        }
        body.blocks.push(bb);
    }
    body.fall_off = body
        .fall_off
        .and_then(|(b, sp)| map[b.index()].map(|nb| (nb, sp)));
    body.loops = std::mem::take(&mut body.loops)
        .into_iter()
        .filter_map(|mut li| {
            map[li.head.index()].map(|h| {
                li.head = h;
                li
            })
        })
        .collect();
    // khoản mượn: giữ những cái còn được tham chiếu, đánh số lại theo thứ tự cũ
    let mut used = vec![false; body.borrows.len()];
    for bb in &body.blocks {
        for st in &bb.statements {
            if let StatementKind::Assign(_, Rvalue::Ref(id, ..)) = &st.kind {
                used[id.index()] = true;
            }
        }
    }
    let mut bmap: Vec<Option<BorrowId>> = vec![None; body.borrows.len()];
    let mut k = 0u32;
    for (i, u) in used.iter().enumerate() {
        if *u {
            bmap[i] = Some(BorrowId(k));
            k += 1;
        }
    }
    let oldb = std::mem::take(&mut body.borrows);
    body.borrows = oldb
        .into_iter()
        .enumerate()
        .filter(|(i, _)| used[*i])
        .map(|(_, b)| b)
        .collect();
    for bb in &mut body.blocks {
        for st in &mut bb.statements {
            if let StatementKind::Assign(_, Rvalue::Ref(id, ..)) = &mut st.kind
                && let Some(nid) = bmap[id.index()]
            {
                *id = nid;
            }
        }
    }
}
