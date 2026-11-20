#![forbid(unsafe_code)]
//! Điều kiện dừng (M8 nửa 1, ADR-026 §2; semantics §21): E0701 (vòng lặp thiếu/không suy ra được biến thể, vòng vô hạn thiếu `div`),
//! E0702 (`↓` không giảm nghiêm ngặt hoặc không bị chặn dưới), E0703 (đệ quy thiếu `↓`/`div`). KHÔNG có đường hạ runtime (D25):
//! không chứng minh được thì ĐỎ.
//!
//! Cách chứng minh: mỗi vòng `while`/`⟳ { }` có ứng viên biến thể `v` (người dùng viết `↓ v`, hoặc tự suy từ một vế `a < b`/… của điều
//! kiện). Tại đầu vòng, phân tích chụp "biến bóng" `g_x := x` cho mọi biến của `v` (quan hệ zone `x − g_x` được dịch qua thân vòng);
//! tại MỖI cạnh lùi phải chứng minh `v(x) < v(g)` và `0 ≤ v(g)` bằng chính bộ giải của M7 (`Prover`, cùng bộ vét cạn kiểm chứng).
//! `⟳ x in a..b` / `⟳ x in &xs` hữu hạn và tự giảm nên không cần biến thể. Đệ quy: SCC của đồ thị gọi; mỗi hàm trong SCC đệ quy cần
//! `↓` (hoặc `div`) và mỗi lời gọi trong SCC phải thoả `v_callee(đối số) < v_caller(tham số)` và `0 ≤ v_callee(đối số)`.
use gt_ast::{
    BinaryOp, ContractKind, Expr, ExprKind, FieldName, FnItem, Item, ItemKind, LitKind, NodeId,
    UnaryOp,
};
use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_mir::{Body, BodyKind, Callee, Local, LocalKind, LoopShape, MirOutput, TerminatorKind};
use gt_oblig::analysis::{BodyReport, LEN_BASE, LoopSpec, ProgInfo, Status, analyze_body};
use gt_oblig::ir::{ObKind, Prover, Term, VarId};
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

/// Đồ thị gọi ở mức NÚT HÀM sau khi gộp instance: `recursive[i]` nút `i` có instance nằm trong chu trình; `comp[i]` số hiệu thành phần
/// (các nút cùng một SCC đệ quy chung số hiệu); `bad_indirect[i]` có cạnh gián tiếp (giá trị hàm/phương thức trait) nằm trong SCC của nó.
struct NodeGraph {
    comp: Vec<usize>,
    recursive: Vec<bool>,
    bad_indirect: Vec<bool>,
}

type Inst = (NodeId, Vec<gt_types::Ty>);

/// số lần phải lùi về đồ thị theo nút bảo thủ từ đầu tiến trình (đo đạc cho test: bộ sinh `term_graph` đòi 0)
static CONSERVATIVE_FALLBACKS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[must_use]
pub fn conservative_fallbacks() -> usize {
    CONSERVATIVE_FALLBACKS.load(std::sync::atomic::Ordering::SeqCst)
}

static DROP_TRAIT_EDGES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Chốt kiểm thử độ nhạy (ADR-033 phụ lục): bỏ MỌI cạnh phương thức trait của đồ thị instance (toàn tiến trình: M8 chạy ở luồng khác luồng
/// gọi; FAIL-OPEN cố ý, để chứng minh bộ sinh `term_graph` tự bắt được). KHÔNG dùng ngoài test.
pub fn set_drop_trait_edges_for_test(on: bool) {
    DROP_TRAIT_EDGES.store(on, std::sync::atomic::Ordering::SeqCst);
}

/// Chốt phòng thủ: quá số nút này ⇒ không dựng đồ thị instance (lùi về bảo thủ).
const MAX_GRAPH_NODES: usize = 2 * gt_types::MAX_PROGRAM_INSTANCES;

static GRAPH_LIMIT_OVERRIDE: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
static FORCE_CONSERVATIVE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Chốt kiểm thử: đặt cận số nút của đồ thị instance (0 = mặc định). KHÔNG dùng ngoài test.
pub fn set_graph_limit_for_test(n: usize) {
    GRAPH_LIMIT_OVERRIDE.store(n, std::sync::atomic::Ordering::SeqCst);
}

/// Chốt kiểm thử: buộc lùi về đồ thị theo nút bảo thủ (để kiểm đường dự phòng). KHÔNG dùng ngoài test.
pub fn set_force_conservative_for_test(on: bool) {
    FORCE_CONSERVATIVE.store(on, std::sync::atomic::Ordering::SeqCst);
}

fn graph_limit() -> usize {
    match GRAPH_LIMIT_OVERRIDE.load(std::sync::atomic::Ordering::SeqCst) {
        0 => MAX_GRAPH_NODES,
        n => n,
    }
}

/// Đồ thị gọi theo INSTANCE. `None` ⇒ không dựng được trọn vẹn (kiểu chưa cụ thể, không tìm được impl, quá lớn): người gọi dùng đồ thị
/// bảo thủ. KHÔNG BAO GIỜ bỏ cạnh vì thiếu thông tin.
fn instance_graph(
    t: &TypesOutput,
    bodies: &[&Body],
    by_node: &BTreeMap<NodeId, usize>,
    escaped: &BTreeSet<NodeId>,
    contracts: &BTreeMap<NodeId, Vec<&Body>>,
) -> Option<NodeGraph> {
    use gt_types::Ty;
    if FORCE_CONSERVATIVE.load(std::sync::atomic::Ordering::SeqCst) {
        return None;
    }
    let mut idx: BTreeMap<Inst, usize> = BTreeMap::new();
    let mut list: Vec<Inst> = Vec::new();
    let add = |i: Inst, idx: &mut BTreeMap<Inst, usize>, list: &mut Vec<Inst>| -> Option<usize> {
        if let Some(&k) = idx.get(&i) {
            return Some(k);
        }
        if list.len() >= graph_limit() {
            return None;
        }
        let k = list.len();
        idx.insert(i.clone(), k);
        list.push(i);
        Some(k)
    };
    for b in bodies {
        if t.db.fn_params(b.node).is_empty() {
            add((b.node, Vec::new()), &mut idx, &mut list)?;
        }
    }
    for i in &t.instances {
        if by_node.contains_key(&i.node) {
            add((i.node, i.args.clone()), &mut idx, &mut list)?;
        }
    }
    // cạnh: (đích, gián tiếp?); `value_call[k]`: instance k có lời gọi giá trị hàm
    let mut edges: Vec<Vec<(usize, bool)>> = Vec::new();
    let mut value_call: Vec<bool> = Vec::new();
    let mut done = 0usize;
    while done < list.len() {
        let k = done;
        done += 1;
        let (node, args) = list[k].clone();
        while edges.len() <= k {
            edges.push(Vec::new());
            value_call.push(false);
        }
        let Some(&bi) = by_node.get(&node) else {
            continue;
        };
        // GÓI 11 FB-C35: kiểm tra hợp đồng `⊳`/`⊲` chạy lúc chạy (chèn sau M8) và gọi hàm người dùng trong thân hợp đồng; gộp cạnh gọi của thân hợp đồng
        // vào nút của hàm sở hữu (mọi chu trình thật qua hợp đồng đều là chu trình trong đồ thị gộp).
        let mut srcs: Vec<&Body> = vec![bodies[bi]];
        if let Some(cs) = contracts.get(&node) {
            srcs.extend(cs.iter().copied());
        }
        let params = t.db.fn_params(node);
        let sub = |ty: &Ty| -> Ty {
            ty.subst(&|p| {
                params
                    .iter()
                    .position(|q| *q == p)
                    .and_then(|i| args.get(i).cloned())
            })
        };
        for body in srcs {
            for bb in &body.blocks {
                // hàm dùng như giá trị: chỉ PHÁT HIỆN instance (không có cạnh)
                let mut ops: Vec<&gt_mir::Operand> = Vec::new();
                for st in &bb.statements {
                    if let gt_mir::StatementKind::Assign(_, rv) = &st.kind {
                        match rv {
                            gt_mir::Rvalue::Use(o)
                            | gt_mir::Rvalue::Cast(o, _)
                            | gt_mir::Rvalue::Repeat(o, _) => {
                                ops.push(o);
                            }
                            gt_mir::Rvalue::Aggregate(_, v) => ops.extend(v.iter()),
                            _ => {}
                        }
                    }
                }
                let Some(term) = bb.terminator.as_ref() else {
                    for o in ops {
                        discover(o, &sub, by_node, &mut |i| add(i, &mut idx, &mut list))?;
                    }
                    continue;
                };
                let TerminatorKind::Call {
                    callee,
                    args: cargs,
                    ..
                } = &term.kind
                else {
                    for o in ops {
                        discover(o, &sub, by_node, &mut |i| add(i, &mut idx, &mut list))?;
                    }
                    continue;
                };
                ops.extend(cargs.iter());
                for o in ops {
                    discover(o, &sub, by_node, &mut |i| add(i, &mut idx, &mut list))?;
                }
                match callee {
                    Callee::Fn { node: n2, generics } => {
                        if by_node.contains_key(n2) {
                            let a2: Vec<Ty> = generics.iter().map(&sub).collect();
                            let j = add((*n2, a2), &mut idx, &mut list)?;
                            edges[k].push((j, false));
                        } else {
                            // GÓI 7 FB-C33: callee KHÔNG có thân (intrinsic như `spawn`, `Mutex::with`) có thể gọi hàm được truyền làm đối số
                            // (`spawn(f, n)` chạy `f` ở luồng mới): cạnh GIÁN TIẾP tới mọi hàm `FnItem` trong đối số. Thiếu cạnh này, đệ quy qua spawn
                            // không bị kiểm biến thể (không dừng được chấp nhận).
                            for o in cargs {
                                if let gt_mir::Operand::Const(gt_mir::Const::FnItem {
                                    node: fnode,
                                    generics: fg,
                                    ..
                                }) = o
                                    && by_node.contains_key(fnode)
                                {
                                    let a3: Vec<Ty> = fg.iter().map(&sub).collect();
                                    let j = add((*fnode, a3), &mut idx, &mut list)?;
                                    edges[k].push((j, true));
                                }
                            }
                            // GÓI 8 S1b (FB-C34): đối số KHÔNG phải hằng hàm (vd kết quả `id(f)` qua generic) ⇒ không biết callee chạy hàm nào:
                            // lùi về đồ thị bảo thủ (cạnh gián tiếp tới mọi hàm bị thoát). Không bao giờ bỏ cạnh vì thiếu thông tin.
                            if dyn_fn_arg(&body.locals, cargs) && takes_fn_param(t, *n2) {
                                return None;
                            }
                        }
                    }
                    Callee::TraitMethod {
                        trait_def,
                        member,
                        self_ty,
                        generics,
                    } => {
                        let st = sub(self_ty);
                        if st.has_param() || st.has_infer() {
                            return None;
                        }
                        let ga: Vec<Ty> = generics.iter().map(&sub).collect();
                        let (n2, a2) = t.db.resolve_trait_method(*trait_def, *member, &st, &ga)?;
                        if by_node.contains_key(&n2) {
                            let j = add((n2, a2), &mut idx, &mut list)?;
                            if !DROP_TRAIT_EDGES.load(std::sync::atomic::Ordering::SeqCst) {
                                edges[k].push((j, true));
                            }
                        }
                    }
                    Callee::Value(_) => value_call[k] = true,
                }
            }
        }
    }
    let n = list.len();
    while edges.len() < n {
        edges.push(Vec::new());
        value_call.push(false);
    }
    // lời gọi giá trị hàm ⇒ cạnh gián tiếp tới mọi instance của hàm bị thoát
    for k in 0..n {
        if value_call[k] {
            for (j, (nj, _)) in list.iter().enumerate() {
                if escaped.contains(nj) {
                    edges[k].push((j, true));
                }
            }
        }
    }
    let adj: Vec<Vec<usize>> = edges
        .iter()
        .map(|e| {
            let mut v: Vec<usize> = e.iter().map(|(j, _)| *j).collect();
            v.sort_unstable();
            v.dedup();
            v
        })
        .collect();
    let comp = sccs(n, &adj);
    let mut size: BTreeMap<usize, usize> = BTreeMap::new();
    for c in &comp {
        *size.entry(*c).or_default() += 1;
    }
    let inst_rec: Vec<bool> = (0..n)
        .map(|k| size[&comp[k]] > 1 || adj[k].contains(&k))
        .collect();
    // gộp theo nút
    let nb = bodies.len();
    let mut parent: Vec<usize> = (0..nb).collect();
    fn find(p: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            r = p[r];
        }
        let mut y = x;
        while p[y] != r {
            let nx = p[y];
            p[y] = r;
            y = nx;
        }
        r
    }
    let mut recursive = vec![false; nb];
    let mut bad = vec![false; nb];
    let mut first_of_comp: BTreeMap<usize, usize> = BTreeMap::new();
    for k in 0..n {
        let Some(&bi) = by_node.get(&list[k].0) else {
            continue;
        };
        if !inst_rec[k] {
            continue;
        }
        recursive[bi] = true;
        if edges[k].iter().any(|(j, ind)| *ind && comp[*j] == comp[k]) {
            bad[bi] = true;
        }
        match first_of_comp.get(&comp[k]) {
            Some(&f) => {
                let (ra, rb) = (find(&mut parent, f), find(&mut parent, bi));
                if ra != rb {
                    parent[rb] = ra;
                }
            }
            None => {
                first_of_comp.insert(comp[k], bi);
            }
        }
    }
    let comps: Vec<usize> = (0..nb).map(|i| find(&mut parent, i)).collect();
    Some(NodeGraph {
        comp: comps,
        recursive,
        bad_indirect: bad,
    })
}

/// nếu `o` là hàm dùng như giá trị thì phát hiện instance của nó (đối số đã thế)
fn discover(
    o: &gt_mir::Operand,
    sub: &dyn Fn(&gt_types::Ty) -> gt_types::Ty,
    by_node: &BTreeMap<NodeId, usize>,
    add: &mut dyn FnMut(Inst) -> Option<usize>,
) -> Option<()> {
    if let gt_mir::Operand::Const(gt_mir::Const::FnItem { node, generics, .. }) = o
        && by_node.contains_key(node)
    {
        add((*node, generics.iter().map(sub).collect()))?;
    }
    Some(())
}

/// GÓI 8 S1b (FB-C34): kiểu có THỂ mang một giá trị hàm: hàm, tham số kiểu chưa biết, suy kiểu chưa xong (và tham chiếu/con trỏ tới chúng).
/// Hằng hàm (`Const`) xử lý riêng (cạnh trực tiếp). Place có phép chiếu (trường/chỉ số) ⇒ coi là có thể (bảo thủ).
/// Giới hạn đã biết: intrinsic không thân nhận hàm nằm TRONG cấu trúc (tuple/struct) không được phát hiện qua chữ ký.
fn may_hold_fn(ty: &gt_types::Ty) -> bool {
    use gt_types::Ty;
    // KHÔNG tính `Param`/`Infer`: giá trị kiểu tham số không thể là đối số `spawn` mà chưa khớp kiểu hàm; tính chúng làm mã generic của `std`
    // (vd `Ord::cmp` của `Option<T>`) bị E0703 oan (cổng GÓI 8 lần 1: std_lib, std2_lib, hashmap_collide, release_policy, rig).
    if let Ty::Ref(_, inner) | Ty::RawPtr(_, inner) = ty {
        return may_hold_fn(inner);
    }
    matches!(ty, Ty::Fn(..))
}

fn dyn_fn_arg(locals: &[gt_mir::LocalDecl], args: &[gt_mir::Operand]) -> bool {
    args.iter().any(|o| match o {
        gt_mir::Operand::Const(_) => false,
        gt_mir::Operand::Copy(p) | gt_mir::Operand::Move(p) => {
            !p.projection.is_empty()
                || match locals.get(p.local.index()) {
                    Some(d) => may_hold_fn(&d.ty),
                    None => true,
                }
        }
    })
}

/// GÓI 8 S1b (FB-C34): hàm (không có thân trong chương trình) có tham số kiểu hàm trong chữ ký: có thể chạy một hàm do người gọi đưa vào
/// (vd `spawn`). Không tìm được chữ ký ⇒ `true` (bảo thủ).
fn takes_fn_param(t: &TypesOutput, node: NodeId) -> bool {
    let sig =
        t.db.fns
            .values()
            .map(|(sig, _)| sig)
            .chain(t.db.impls.iter().flat_map(|imp| imp.members.iter()))
            .find(|sig| sig.node == node);
    match sig {
        Some(sig) => sig.params.iter().any(may_hold_fn),
        None => true,
    }
}

/// Đồ thị theo NÚT bảo thủ (hành vi cũ): mọi `TraitMethod` nối tới mọi thành viên của mọi impl của trait; mọi lời gọi giá trị nối tới
/// mọi hàm bị thoát.
fn conservative_graph(
    t: &TypesOutput,
    bodies: &[&Body],
    by_node: &BTreeMap<NodeId, usize>,
    escaped: &BTreeSet<NodeId>,
    contracts: &BTreeMap<NodeId, Vec<&Body>>,
) -> NodeGraph {
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
        // GÓI 11 FB-C35: thân hợp đồng của hàm này là một phần của nút (xem `instance_graph`)
        let mut srcs: Vec<&Body> = vec![*b];
        if let Some(cs) = contracts.get(&b.node) {
            srcs.extend(cs.iter().copied());
        }
        for b in srcs {
            for bb in &b.blocks {
                let Some(TerminatorKind::Call { callee, args, .. }) =
                    bb.terminator.as_ref().map(|t| &t.kind)
                else {
                    continue;
                };
                match callee {
                    Callee::Fn { node, .. } => {
                        if let Some(&j) = by_node.get(node) {
                            adj[i].push(j);
                        } else {
                            // GÓI 7 FB-C33 + GÓI 8 S1b (FB-C34): callee không có thân nhận đối số không phải hằng hàm ⇒ gọi được mọi hàm bị thoát
                            let dyn_arg = dyn_fn_arg(&b.locals, args);
                            if dyn_arg && takes_fn_param(t, *node) {
                                for n in escaped {
                                    if let Some(&j) = by_node.get(n) {
                                        adj[i].push(j);
                                        indirect[i].push(j);
                                    }
                                }
                            }
                        }
                    }
                    Callee::Value(_) => {
                        for n in escaped {
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
    let recursive: Vec<bool> = (0..bodies.len())
        .map(|i| size[&comp[i]] > 1 || adj[i].contains(&i))
        .collect();
    let bad: Vec<bool> = (0..bodies.len())
        .map(|i| indirect[i].iter().any(|j| comp[*j] == comp[i]))
        .collect();
    NodeGraph {
        comp,
        recursive,
        bad_indirect: bad,
    }
}

/// Có đường đi từ đầu vòng quay lại chính nó không (cạnh lùi). Không có ⇒ vòng lặp lặp tối đa một lần.
fn has_back_edge(b: &Body, head: gt_mir::BasicBlock) -> bool {
    let mut seen = vec![false; b.blocks.len()];
    // khối không có terminator ⇒ MIR hỏng ⇒ KHÔNG biết có cạnh lùi hay không ⇒ coi như CÓ (bảo thủ: vòng lặp phải được chứng minh dừng)
    let Some(ht) = &b.blocks[head.index()].terminator else {
        return true;
    };
    let mut stack: Vec<gt_mir::BasicBlock> = ht.kind.successors();
    while let Some(x) = stack.pop() {
        if x == head {
            return true;
        }
        if std::mem::replace(&mut seen[x.index()], true) {
            continue;
        }
        match &b.blocks[x.index()].terminator {
            Some(t) => stack.extend(t.kind.successors()),
            None => return true,
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
                            gt_mir::Rvalue::Use(o)
                            | gt_mir::Rvalue::Cast(o, _)
                            | gt_mir::Rvalue::Repeat(o, _) => {
                                ops.push(o);
                            }
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
    // Đồ thị gọi theo INSTANCE (ADR-033 phụ lục); không dựng được ⇒ lùi về đồ thị theo nút bảo thủ (mọi `TraitMethod` nối tới mọi impl).
    let mut contracts: BTreeMap<NodeId, Vec<&Body>> = BTreeMap::new();
    for b in &m.bodies {
        if matches!(b.kind, BodyKind::Pre | BodyKind::Post) {
            contracts.entry(b.sig_node).or_default().push(b);
        }
    }
    let graph = instance_graph(t, &bodies, &by_node, &escaped, &contracts).unwrap_or_else(|| {
        CONSERVATIVE_FALLBACKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        conservative_graph(t, &bodies, &by_node, &escaped, &contracts)
    });
    for (i, b) in bodies.iter().enumerate() {
        if !graph.recursive[i] {
            continue;
        }
        info.scc.insert(b.node, graph.comp[i]);
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
        } else if graph.bad_indirect[i] {
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
        if !fns.contains_key(&b.node) {
            // thân KHÔNG có chữ ký hàm nhưng có vòng lặp ⇒ KHÔNG kiểm được dừng ⇒ ICE (trước đây bị bỏ qua trong im lặng)
            if !b.loops.is_empty() {
                bag.push(mk(
                    t,
                    "E9901",
                    b.node,
                    b.span,
                    format!(
                        "thân `{}` có vòng lặp nhưng không có chữ ký hàm: không kiểm được dừng",
                        b.key
                    ),
                ));
            }
            continue;
        }
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
        // Tập nghĩa vụ RỖNG KHÔNG tự nó là bằng chứng dừng (lệnh Giang 2026-10-15, B.3). Chỉ chấp nhận khi M7 KHẲNG ĐỊNH TƯỜNG MINH rằng KHÔNG có cạnh lùi khả thi
        // tới đầu vòng (`BodyReport::no_back_edge_heads`: vòng lặp chạy tối đa một lượt, ví dụ thân không bao giờ vào được); nghĩa vụ bị mất/cắt/không sinh ⇒ từ chối.
        let ok = if judged.is_empty() {
            rep.is_some_and(|r| r.no_back_edge_heads.contains(head))
        } else if *inferred {
            let mut cands: BTreeMap<u32, bool> = BTreeMap::new();
            for j in &judged {
                let (_, c) = j.tag.unwrap_or((0, 0));
                let e = cands.entry(c).or_insert(true);
                *e &= matches!(j.status, Status::Discharged { .. });
            }
            cands.values().any(|v| *v)
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
            // GÓI 6 S2/S3: hàm vượt giới hạn của bộ chứng minh (biến biến-thể-vòng-lặp hoặc biến theo dõi > 24) bỏ quan hệ giữa các biến nên vòng ĐÚNG cũng bị từ chối (fail-closed):
            // nói rõ nguyên nhân thay vì để người dùng tưởng chương trình sai.
            let need: usize = info
                .loops
                .get(key)
                .map_or(0, |v| v.iter().map(|sp| sp.vars.len()).sum());
            // chỉ để quyết định lời nhắc (không ảnh hưởng phán quyết): thiếu báo cáo thân ⇒ không nhắc
            let dropped = match rep {
                Some(r) => !r.zone_dropped.is_empty(),
                None => false,
            };
            let mut msg =
                "biến thể `↓` không giảm nghiêm ngặt qua mỗi vòng hoặc không bị chặn dưới (≥ 0)"
                    .to_string();
            if need > gt_oblig::analysis::GHOST_MAX_VARS || dropped {
                msg.push_str(&format!(
                    "; lưu ý: hàm này vượt giới hạn của bộ chứng minh (tối đa {} biến theo dõi và {} biến biến-thể vòng lặp mỗi hàm) nên quan hệ giữa các biến bị bỏ và vòng đúng cũng có thể bị từ chối — tách các vòng lặp/phần thân thành hàm nhỏ hơn (docs/LIMITS_AUDIT.md)",
                    gt_oblig::analysis::ZONE_MAX_VARS,
                    gt_oblig::analysis::GHOST_MAX_VARS
                ));
            }
            bag.push(mk(t, "E0702", *node, Span::new(span.start, span.end), msg));
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
