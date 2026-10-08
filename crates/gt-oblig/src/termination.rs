//! Điều kiện dừng (M8 nửa 1, ADR-026 §2; semantics §21): E0701 (vòng lặp thiếu/không suy ra được biến thể, vòng vô hạn thiếu `div`),
//! E0702 (`↓` không giảm nghiêm ngặt hoặc không bị chặn dưới), E0703 (đệ quy thiếu `↓`/`div`). KHÔNG có đường hạ runtime (D25):
//! không chứng minh được thì ĐỎ.
//!
//! Cách chứng minh: mỗi vòng `while`/`⟳ { }` có ứng viên biến thể `v` (người dùng viết `↓ v`, hoặc tự suy từ một vế `a < b`/… của điều
//! kiện). Tại đầu vòng, phân tích chụp "biến bóng" `g_x := x` cho mọi biến của `v` (quan hệ zone `x − g_x` được dịch qua thân vòng);
//! tại MỖI cạnh lùi phải chứng minh `v(x) < v(g)` và `0 ≤ v(g)` bằng chính bộ giải của M7 (`Prover`, cùng bộ vét cạn kiểm chứng).
//! `⟳ x in a..b` / `⟳ x in &xs` hữu hạn và tự giảm nên không cần biến thể. Đệ quy: SCC của đồ thị gọi; mỗi hàm trong SCC đệ quy cần
//! `↓` (hoặc `div`) và mỗi lời gọi trong SCC phải thoả `v_callee(đối số) < v_caller(tham số)` và `0 ≤ v_callee(đối số)`.
use crate::analysis::{BodyReport, LEN_BASE, LoopSpec, ProgInfo, Status, analyze_body};
use crate::ir::{ObKind, Prover, Term, VarId};
use gt_ast::{
    BinaryOp, ContractKind, Expr, ExprKind, FieldName, FnItem, Item, ItemKind, LitKind, NodeId,
    UnaryOp,
};
use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_mir::{Body, BodyKind, Callee, Local, LocalKind, LoopShape, MirOutput, TerminatorKind};
use gt_resolve::Res;
use gt_span::Span;
use gt_types::TypesOutput;
use std::collections::{BTreeMap, BTreeSet};

const LIMIT: usize = 200;

#[derive(Debug)]
pub struct TermOutput {
    pub report: Report,
    pub per_body: Vec<BodyReport>,
    /// số vòng lặp cần chứng minh / đã chứng minh (để báo cáo)
    pub loops_checked: usize,
    pub loops_proved: usize,
}

fn index_fns<'a>(items: &'a [Item], out: &mut BTreeMap<NodeId, &'a FnItem>) {
    for it in items {
        match &it.kind {
            ItemKind::Fn(f) => {
                out.insert(it.id, f);
            }
            ItemKind::Impl(i) => {
                for m in &i.members {
                    out.insert(m.id, &m.func);
                }
            }
            ItemKind::Mod(m) => index_fns(&m.items, out),
            _ => {}
        }
    }
}

struct Tx<'a> {
    t: &'a TypesOutput,
    body: &'a Body,
    /// tên tham số → local (cho biến thể của hàm, nơi tên không qua `LocalId` của thân)
    params: BTreeMap<String, Local>,
}

impl Tx<'_> {
    fn local_of(&self, e: &Expr) -> Option<Local> {
        let ExprKind::Path(p) = &e.kind else {
            return None;
        };
        match self.t.resolve.uses.get(&p.id) {
            Some(Res::Local(id)) => self
                .body
                .var_map
                .iter()
                .find(|(i, _)| i == id)
                .map(|(_, l)| *l),
            _ => {
                // tham số của hàm (biến thể ở chữ ký): theo tên đoạn cuối của đường dẫn
                if let [seg] = p.segments.as_slice()
                    && let gt_ast::SegmentKind::Name(name) = &seg.kind
                {
                    return self.params.get(name).copied();
                }
                None
            }
        }
    }

    fn is_int_local(&self, l: Local) -> bool {
        matches!(self.body.locals[l.index()].ty, gt_types::Ty::Int(_))
            && matches!(
                self.body.locals[l.index()].kind,
                LocalKind::Arg(_) | LocalKind::User { .. }
            )
    }

    /// Dịch biểu thức nguyên tuyến tính thành `Term` theo `Var(local)` / `Var(LEN_BASE + local)`.
    fn term(&self, e: &Expr) -> Option<Term> {
        match &e.kind {
            ExprKind::Lit(l) => match &l.kind {
                LitKind::Int(s) => {
                    let (v, _) = gt_types::int_literal_value(s)?;
                    let v = i128::try_from(v).ok()?;
                    Some(Term::int(if l.negative { -v } else { v }))
                }
                _ => None,
            },
            ExprKind::Path(_) => {
                let l = self.local_of(e)?;
                self.is_int_local(l).then(|| Term::var(l.index() as VarId))
            }
            ExprKind::Unary {
                op: UnaryOp::Neg,
                operand,
            } => Some(Term::Neg(Box::new(self.term(operand)?))),
            ExprKind::Binary { op, lhs, rhs } => {
                let (a, b) = (self.term(lhs)?, self.term(rhs)?);
                match op {
                    BinaryOp::Add => Some(Term::add(a, b)),
                    BinaryOp::Sub => Some(Term::sub(a, b)),
                    BinaryOp::Mul if matches!(a, Term::Int(_)) || matches!(b, Term::Int(_)) => {
                        Some(Term::mul(a, b))
                    }
                    _ => None,
                }
            }
            // `xs.len()` với `xs` là tham chiếu lát
            ExprKind::Call { callee, args } if args.is_empty() => {
                let ExprKind::Field {
                    base,
                    name: FieldName::Named(n),
                } = &callee.kind
                else {
                    return None;
                };
                if n.name != "len" {
                    return None;
                }
                let l = self.local_of(base)?;
                let ty = &self.body.locals[l.index()].ty;
                matches!(ty, gt_types::Ty::Ref(_, inner) if matches!(**inner, gt_types::Ty::Slice(_)))
                    .then(|| Term::var(LEN_BASE + l.index() as VarId))
            }
            _ => None,
        }
    }
}

fn conjuncts<'a>(e: &'a Expr, out: &mut Vec<&'a Expr>) {
    if let ExprKind::Binary {
        op: BinaryOp::And,
        lhs,
        rhs,
    } = &e.kind
    {
        conjuncts(lhs, out);
        conjuncts(rhs, out);
    } else {
        out.push(e);
    }
}

/// Các ứng viên biến thể tự suy từ điều kiện `while`: một vế `a < b`, `a <= b`, `a > b`, `a >= b`.
fn inferred_candidates(tx: &Tx<'_>, cond: &Expr) -> Vec<Term> {
    let mut cs = Vec::new();
    conjuncts(cond, &mut cs);
    let mut out = Vec::new();
    for c in cs {
        let ExprKind::Binary { op, lhs, rhs } = &c.kind else {
            continue;
        };
        let (Some(a), Some(b)) = (tx.term(lhs), tx.term(rhs)) else {
            continue;
        };
        let m = match op {
            BinaryOp::Lt => Term::sub(b, a),
            BinaryOp::Le => Term::add(Term::sub(b, a), Term::int(1)),
            BinaryOp::Gt => Term::sub(a, b),
            BinaryOp::Ge => Term::add(Term::sub(a, b), Term::int(1)),
            _ => continue,
        };
        out.push(m);
    }
    out
}

fn mk(t: &TypesOutput, code: &str, node: NodeId, span: Span, msg: String) -> Diagnostic {
    let c = Code::parse(code).unwrap_or_else(|_| Code::parse("E9901").expect("E9901 hợp lệ"));
    Diagnostic::new(c, &t.file_name(node), span, msg)
}

/// Tarjan lặp: trả về số hiệu thành phần liên thông mạnh của từng đỉnh.
fn sccs(n: usize, adj: &[Vec<usize>]) -> Vec<usize> {
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut on = vec![false; n];
    let mut comp = vec![usize::MAX; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut next = 0usize;
    let mut ncomp = 0usize;
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut work: Vec<(usize, usize)> = vec![(root, 0)];
        index[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        on[root] = true;
        while let Some((v, i)) = work.pop() {
            if i < adj[v].len() {
                work.push((v, i + 1));
                let w = adj[v][i];
                if index[w] == usize::MAX {
                    index[w] = next;
                    low[w] = next;
                    next += 1;
                    stack.push(w);
                    on[w] = true;
                    work.push((w, 0));
                } else if on[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                if low[v] == index[v] {
                    while let Some(w) = stack.pop() {
                        on[w] = false;
                        comp[w] = ncomp;
                        if w == v {
                            break;
                        }
                    }
                    ncomp += 1;
                }
                if let Some(&(p, _)) = work.last() {
                    low[p] = low[p].min(low[v]);
                }
            }
        }
    }
    comp
}

/// Có đường đi từ đầu vòng quay lại chính nó không (cạnh lùi). Không có ⇒ vòng lặp lặp tối đa một lần.
fn has_back_edge(b: &Body, head: gt_mir::BasicBlock) -> bool {
    let mut seen = vec![false; b.blocks.len()];
    let mut stack: Vec<gt_mir::BasicBlock> = b.blocks[head.index()]
        .terminator
        .as_ref()
        .map(|t| t.kind.successors())
        .unwrap_or_default();
    while let Some(x) = stack.pop() {
        if x == head {
            return true;
        }
        if std::mem::replace(&mut seen[x.index()], true) {
            continue;
        }
        if let Some(t) = &b.blocks[x.index()].terminator {
            stack.extend(t.kind.successors());
        }
    }
    false
}

/// Kiểm điều kiện dừng toàn chương trình. `base` từ `driver::build_prog_info` (hợp đồng, vị từ…).
pub fn check_termination(
    t: &TypesOutput,
    m: &MirOutput,
    base: &ProgInfo,
    prover: &mut (dyn Prover + Send),
) -> TermOutput {
    let mut bag = Diagnostics::new(LIMIT);
    let mut fns: BTreeMap<NodeId, &FnItem> = BTreeMap::new();
    for p in &t.resolve.programs {
        index_fns(&p.items, &mut fns);
    }
    let bodies: Vec<&Body> = m.bodies.iter().filter(|b| b.kind == BodyKind::Fn).collect();
    let by_node: BTreeMap<NodeId, usize> = bodies
        .iter()
        .enumerate()
        .map(|(i, b)| (b.node, i))
        .collect();
    let has_div = |b: &Body| {
        t.db.sig_by_node(b.node)
            .is_some_and(|s| s.effects.iter().any(|e| e == "div"))
    };
    let mut info = base.clone();
    info.loops.clear();
    info.fn_variant.clear();
    info.scc.clear();

    // ---- biến thể của hàm (đệ quy) ----
    for b in &bodies {
        let Some(f) = fns.get(&b.node) else { continue };
        let Some(c) = f
            .sig
            .contracts
            .iter()
            .find(|c| c.kind == ContractKind::Variant)
        else {
            continue;
        };
        let mut params = BTreeMap::new();
        for i in 1..=b.arg_count {
            if let LocalKind::Arg(Some(n)) = &b.locals[i].kind {
                params.insert(n.clone(), Local(i as u32));
            }
        }
        let tx = Tx { t, body: b, params };
        match tx.term(&c.expr) {
            Some(term) => {
                info.fn_variant.insert(b.node, term);
            }
            None => bag.push(mk(
                t,
                "E0702",
                c.id,
                c.expr.span,
                format!("biểu thức `↓` của `{}` không thuộc dạng tuyến tính theo tham số nguyên (và `xs.len()`) mà bộ kiểm hỗ trợ", b.key),
            )),
        }
    }

    // ---- đồ thị gọi và SCC ----
    let escaped: BTreeSet<NodeId> = {
        let mut s = BTreeSet::new();
        for b in &m.bodies {
            for bb in &b.blocks {
                for st in &bb.statements {
                    if let gt_mir::StatementKind::Assign(_, rv) = &st.kind {
                        let mut ops: Vec<&gt_mir::Operand> = Vec::new();
                        match rv {
                            gt_mir::Rvalue::Use(o) | gt_mir::Rvalue::Cast(o, _) => ops.push(o),
                            gt_mir::Rvalue::Aggregate(_, v) => ops.extend(v.iter()),
                            _ => {}
                        }
                        for o in ops {
                            if let gt_mir::Operand::Const(gt_mir::Const::FnItem { node, .. }) = o {
                                s.insert(*node);
                            }
                        }
                    }
                }
                if let Some(TerminatorKind::Call { args, .. }) =
                    bb.terminator.as_ref().map(|t| &t.kind)
                {
                    for o in args {
                        if let gt_mir::Operand::Const(gt_mir::Const::FnItem { node, .. }) = o {
                            s.insert(*node);
                        }
                    }
                }
            }
        }
        s
    };
    let trait_impl_members = |trait_def: gt_resolve::DefId| -> Vec<NodeId> {
        t.db.impls
            .iter()
            .filter(|i| {
                i.trait_ref
                    .as_ref()
                    .is_some_and(|tr| tr.trait_def == trait_def)
            })
            .flat_map(|i| i.members.iter().map(|mm| mm.node))
            .collect()
    };
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); bodies.len()];
    let mut indirect: Vec<Vec<usize>> = vec![Vec::new(); bodies.len()];
    for (i, b) in bodies.iter().enumerate() {
        for bb in &b.blocks {
            let Some(TerminatorKind::Call { callee, .. }) = bb.terminator.as_ref().map(|t| &t.kind)
            else {
                continue;
            };
            match callee {
                Callee::Fn { node, .. } => {
                    if let Some(&j) = by_node.get(node) {
                        adj[i].push(j);
                    }
                }
                Callee::Value(_) => {
                    for n in &escaped {
                        if let Some(&j) = by_node.get(n) {
                            adj[i].push(j);
                            indirect[i].push(j);
                        }
                    }
                }
                Callee::TraitMethod { trait_def, .. } => {
                    for n in trait_impl_members(*trait_def) {
                        if let Some(&j) = by_node.get(&n) {
                            adj[i].push(j);
                            indirect[i].push(j);
                        }
                    }
                }
            }
        }
        adj[i].sort_unstable();
        adj[i].dedup();
        indirect[i].sort_unstable();
        indirect[i].dedup();
    }
    let comp = sccs(bodies.len(), &adj);
    let mut size: BTreeMap<usize, usize> = BTreeMap::new();
    for c in &comp {
        *size.entry(*c).or_default() += 1;
    }
    for (i, b) in bodies.iter().enumerate() {
        let recursive = size[&comp[i]] > 1 || adj[i].contains(&i);
        if !recursive {
            continue;
        }
        info.scc.insert(b.node, comp[i]);
        if has_div(b) {
            continue;
        }
        if !info.fn_variant.contains_key(&b.node) {
            bag.push(mk(
                t,
                "E0703",
                b.node,
                b.span,
                format!(
                    "`{}` thuộc một chu trình đệ quy mà thiếu biến thể `↓` (hoặc hiệu ứng `div`)",
                    b.key
                ),
            ));
        } else if indirect[i].iter().any(|j| comp[*j] == comp[i]) {
            bag.push(mk(
                t,
                "E0703",
                b.node,
                b.span,
                format!("`{}` đệ quy qua giá trị hàm hoặc phương thức trait: không kiểm được biến thể giảm; khai báo hiệu ứng `div` nếu cố ý", b.key),
            ));
        }
    }

    // ---- vòng lặp ----
    let mut loop_nodes: Vec<(String, u32, NodeId, Span, bool)> = Vec::new(); // (thân, đầu, nút, span, tự suy)
    for b in &bodies {
        let Some(f) = fns.get(&b.node) else { continue };
        let _ = f;
        let mut params = BTreeMap::new();
        for i in 1..=b.arg_count {
            if let LocalKind::Arg(Some(n)) = &b.locals[i].kind {
                params.insert(n.clone(), Local(i as u32));
            }
        }
        let tx = Tx { t, body: b, params };
        let mut specs: Vec<LoopSpec> = Vec::new();
        for li in &b.loops {
            let head = li.head.index() as u32;
            let span = (li.span.start, li.span.end);
            let mut push = |term: Term, cand: u32, given: bool| {
                let mut vars = Vec::new();
                term.vars(&mut vars);
                vars.sort_unstable();
                vars.dedup();
                specs.push(LoopSpec {
                    head,
                    cand,
                    term,
                    vars,
                    given,
                    node: li.node,
                    span,
                });
            };
            // vòng for-in hữu hạn và tự giảm: `↓` người dùng ghi (nếu có) là thừa; vòng không có cạnh lùi (thân luôn thoát) lặp tối đa một lần
            if matches!(li.shape, LoopShape::ForRange | LoopShape::ForSlice)
                || !has_back_edge(b, li.head)
            {
                continue;
            }
            match (li.shape, &li.variant) {
                (LoopShape::ForRange | LoopShape::ForSlice, _) => continue,
                (_, Some(v)) => match tx.term(v) {
                    Some(term) => {
                        push(term, 0, true);
                        loop_nodes.push((b.key.clone(), head, li.node, li.span, false));
                    }
                    None => bag.push(mk(
                        t,
                        "E0702",
                        li.node,
                        v.span,
                        "biểu thức `↓` không thuộc dạng tuyến tính theo biến nguyên (và `xs.len()`) mà bộ kiểm hỗ trợ".to_string(),
                    )),
                },
                (LoopShape::Infinite, None) => {
                    if !has_div(b) {
                        bag.push(mk(
                            t,
                            "E0701",
                            li.node,
                            li.span,
                            "vòng lặp vô hạn `⟳ { … }` cần hiệu ứng `div` (hàm khai báo thiếu `div`)".to_string(),
                        ));
                    }
                }
                (LoopShape::While, None) => {
                    let cands = li.cond.as_ref().map(|c| inferred_candidates(&tx, c)).unwrap_or_default();
                    if cands.is_empty() {
                        bag.push(mk(
                            t,
                            "E0701",
                            li.node,
                            li.span,
                            "vòng lặp không có `↓` và không suy ra được biến thể từ điều kiện (cần một vế `a < b`, `a <= b`, `a > b` hoặc `a >= b` theo biến nguyên)".to_string(),
                        ));
                    } else {
                        for (k, c) in cands.into_iter().enumerate() {
                            push(c, k as u32, false);
                        }
                        loop_nodes.push((b.key.clone(), head, li.node, li.span, true));
                    }
                }
            }
            if li.shape == LoopShape::Infinite && li.variant.is_some() && !has_div(b) {
                bag.push(mk(
                    t,
                    "E0701",
                    li.node,
                    li.span,
                    "vòng lặp vô hạn `⟳ { … }` cần hiệu ứng `div` (hàm khai báo thiếu `div`)"
                        .to_string(),
                ));
            }
        }
        if !specs.is_empty() {
            info.loops.insert(b.key.clone(), specs);
        }
    }

    // ---- phân tích + gom kết quả ----
    let mut per_body = Vec::new();
    for b in &bodies {
        per_body.push(analyze_body(b, &info, prover));
    }
    let mut proved = 0usize;
    for (key, head, node, span, inferred) in &loop_nodes {
        let rep = per_body.iter().find(|r| &r.key == key);
        let judged: Vec<_> = rep
            .map(|r| {
                r.judged
                    .iter()
                    .filter(|j| j.tag.is_some_and(|(h, _)| h == *head))
                    .collect()
            })
            .unwrap_or_default();
        let ok = if *inferred {
            let mut cands: BTreeMap<u32, bool> = BTreeMap::new();
            for j in &judged {
                let (_, c) = j.tag.unwrap_or((0, 0));
                let e = cands.entry(c).or_insert(true);
                *e &= matches!(j.status, Status::Discharged { .. });
            }
            cands.is_empty() || cands.values().any(|v| *v)
        } else {
            judged
                .iter()
                .all(|j| matches!(j.status, Status::Discharged { .. }))
        };
        if ok {
            proved += 1;
        } else if *inferred {
            bag.push(mk(
                t,
                "E0701",
                *node,
                Span::new(span.start, span.end),
                "không chứng minh được biến thể tự suy từ điều kiện giảm nghiêm ngặt qua mỗi vòng (thêm `↓` hoặc đổi thân vòng lặp)".to_string(),
            ));
        } else {
            bag.push(mk(
                t,
                "E0702",
                *node,
                Span::new(span.start, span.end),
                "biến thể `↓` không giảm nghiêm ngặt qua mỗi vòng hoặc không bị chặn dưới (≥ 0)"
                    .to_string(),
            ));
        }
    }
    // lời gọi đệ quy
    for r in &per_body {
        for j in &r.judged {
            if j.tag.is_none()
                && j.ob.kind == ObKind::Variant
                && !matches!(j.status, Status::Discharged { .. })
            {
                bag.push(mk(
                    t,
                    "E0702",
                    j.node,
                    Span::new(j.ob.span.0, j.ob.span.1.max(j.ob.span.0)),
                    "lời gọi đệ quy không làm biến thể `↓` giảm nghiêm ngặt (hoặc không bị chặn dưới ≥ 0)".to_string(),
                ));
            }
        }
    }
    TermOutput {
        report: bag.finish(),
        per_body,
        loops_checked: loop_nodes.len(),
        loops_proved: proved,
    }
}
