//! Khung dataflow (ADR-020 §3): dàn nửa-hợp tổng quát, bộ giải lặp tới điểm cố định theo thứ tự CỐ ĐỊNH (RPO cho tiến,
//! ngược lại cho lùi) nên kết quả tất định; trần số bước — vượt là lỗi nội bộ, không treo.
use gt_mir::{BasicBlock, Body, ENTRY, Location, Place, Statement, Terminator, TerminatorKind};
use std::collections::BTreeSet;

/// Dàn nửa-hợp: `join` trả `true` nếu `self` thay đổi.
pub trait JoinSemiLattice: Clone + Eq {
    fn join(&mut self, other: &Self) -> bool;
}

/// Tập bit trên miền `0..len` (join = hợp).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitSet {
    len: usize,
    words: Vec<u64>,
}

impl BitSet {
    #[must_use]
    pub fn empty(len: usize) -> BitSet {
        BitSet {
            len,
            words: vec![0; len.div_ceil(64)],
        }
    }

    #[must_use]
    pub fn full(len: usize) -> BitSet {
        let mut s = BitSet::empty(len);
        for i in 0..len {
            s.insert(i);
        }
        s
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|w| *w == 0)
    }

    pub fn insert(&mut self, i: usize) -> bool {
        if i >= self.len {
            return false;
        }
        let (w, b) = (i / 64, 1u64 << (i % 64));
        let was = self.words[w] & b != 0;
        self.words[w] |= b;
        !was
    }

    pub fn remove(&mut self, i: usize) -> bool {
        if i >= self.len {
            return false;
        }
        let (w, b) = (i / 64, 1u64 << (i % 64));
        let was = self.words[w] & b != 0;
        self.words[w] &= !b;
        was
    }

    #[must_use]
    pub fn contains(&self, i: usize) -> bool {
        i < self.len && self.words[i / 64] & (1u64 << (i % 64)) != 0
    }

    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.len).filter(|i| self.contains(*i))
    }

    /// Giao (cho dàn "chắc chắn").
    pub fn intersect(&mut self, other: &BitSet) -> bool {
        let mut changed = false;
        for (a, b) in self.words.iter_mut().zip(&other.words) {
            let n = *a & *b;
            changed |= n != *a;
            *a = n;
        }
        changed
    }
}

impl JoinSemiLattice for BitSet {
    fn join(&mut self, other: &Self) -> bool {
        let mut changed = false;
        for (a, b) in self.words.iter_mut().zip(&other.words) {
            let n = *a | *b;
            changed |= n != *a;
            *a = n;
        }
        changed
    }
}

/// Dàn đối ngẫu: join = giao (phân tích "trên MỌI đường"); đáy là tập đầy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dual(pub BitSet);

impl JoinSemiLattice for Dual {
    fn join(&mut self, other: &Self) -> bool {
        self.0.intersect(&other.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

/// Một phân tích dataflow.
pub trait Analysis {
    type Domain: JoinSemiLattice;
    const DIRECTION: Direction;
    /// Giá trị đáy (chưa có thông tin) cho mọi khối.
    fn bottom(&self, body: &Body) -> Self::Domain;
    /// Trạng thái ở đầu `bb0` (tiến) — phân tích lùi bỏ qua.
    fn initialize_start(&self, body: &Body, state: &mut Self::Domain);
    fn statement_effect(&self, state: &mut Self::Domain, stmt: &Statement, loc: Location);
    fn terminator_effect(&self, state: &mut Self::Domain, term: &Terminator, loc: Location);
    /// Tiến: hiệu ứng trên cạnh quay về của `Call` (ghi `dest`), chỉ áp lên cạnh tới `target`.
    fn call_return_effect(&self, _state: &mut Self::Domain, _dest: &Place) {}
}

/// Kết quả: với phân tích tiến, `state[b]` là trạng thái ĐẦU khối; với lùi, là trạng thái CUỐI khối (sau terminator).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Results<D> {
    pub state: Vec<D>,
    /// Số lần xử lý một khối (để kiểm hội tụ trong test).
    pub steps: usize,
}

/// Thứ tự duyệt sau-đảo (RPO) của các khối tới được; successors theo thứ tự của terminator.
#[must_use]
pub fn reverse_postorder(body: &Body) -> Vec<BasicBlock> {
    let n = body.blocks.len();
    let mut seen = vec![false; n];
    let mut post = Vec::new();
    // DFS lặp (không đệ quy): (khối, chỉ số successor kế tiếp)
    let mut stack: Vec<(BasicBlock, usize)> = vec![(ENTRY, 0)];
    if n == 0 {
        return post;
    }
    seen[0] = true;
    while let Some((b, i)) = stack.pop() {
        let succ = body.blocks[b.index()]
            .terminator
            .as_ref()
            .map(|t| t.kind.successors())
            .unwrap_or_default();
        if i < succ.len() {
            stack.push((b, i + 1));
            let s = succ[i];
            if s.index() < n && !seen[s.index()] {
                seen[s.index()] = true;
                stack.push((s, 0));
            }
        } else {
            post.push(b);
        }
    }
    post.reverse();
    post
}

/// Danh sách khối liền trước (theo thứ tự chỉ số).
#[must_use]
pub fn predecessors(body: &Body) -> Vec<Vec<BasicBlock>> {
    let mut p: Vec<Vec<BasicBlock>> = vec![Vec::new(); body.blocks.len()];
    for (i, b) in body.blocks.iter().enumerate() {
        if let Some(t) = &b.terminator {
            for s in t.kind.successors() {
                if s.index() < p.len() && !p[s.index()].contains(&BasicBlock(i as u32)) {
                    p[s.index()].push(BasicBlock(i as u32));
                }
            }
        }
    }
    p
}

/// Giải tới điểm cố định. `Err` nếu vượt trần số bước (lỗi nội bộ: dàn không đơn điệu hoặc hiệu ứng sai).
pub fn solve<A: Analysis>(a: &A, body: &Body) -> Result<Results<A::Domain>, String> {
    let n = body.blocks.len();
    let rpo = reverse_postorder(body);
    let mut pos = vec![usize::MAX; n];
    for (i, b) in rpo.iter().enumerate() {
        pos[b.index()] = i;
    }
    let mut state: Vec<A::Domain> = (0..n).map(|_| a.bottom(body)).collect();
    let limit = (n + 1) * 256 + 4096;
    let mut steps = 0usize;
    let mut work: BTreeSet<usize> = BTreeSet::new();
    match A::DIRECTION {
        Direction::Forward => {
            if n > 0 {
                a.initialize_start(body, &mut state[0]);
            }
            for i in 0..rpo.len() {
                work.insert(i);
            }
            while let Some(&i) = work.iter().next() {
                work.remove(&i);
                steps += 1;
                if steps > limit {
                    return Err("dataflow tiến không hội tụ trong trần số bước".to_string());
                }
                let b = rpo[i];
                let bd = &body.blocks[b.index()];
                let mut st = state[b.index()].clone();
                for (k, s) in bd.statements.iter().enumerate() {
                    a.statement_effect(
                        &mut st,
                        s,
                        Location {
                            block: b,
                            statement: k,
                        },
                    );
                }
                let Some(t) = &bd.terminator else { continue };
                a.terminator_effect(
                    &mut st,
                    t,
                    Location {
                        block: b,
                        statement: bd.statements.len(),
                    },
                );
                for s in t.kind.successors() {
                    let mut edge = st.clone();
                    if let TerminatorKind::Call {
                        dest,
                        target: Some(tg),
                        ..
                    } = &t.kind
                        && *tg == s
                    {
                        a.call_return_effect(&mut edge, dest);
                    }
                    if s.index() < n && state[s.index()].join(&edge) && pos[s.index()] != usize::MAX
                    {
                        work.insert(pos[s.index()]);
                    }
                }
            }
        }
        Direction::Backward => {
            let preds = predecessors(body);
            // `before[b]`: trạng thái ở ĐẦU khối b sau khi áp lùi; `state[b]`: cuối khối
            let mut before: Vec<A::Domain> = (0..n).map(|_| a.bottom(body)).collect();
            for i in 0..rpo.len() {
                work.insert(rpo.len() - 1 - i);
            }
            // thứ tự lùi: xử lý theo vị trí RPO GIẢM dần → dùng khoá đảo
            let key = |i: usize| rpo.len() - 1 - i;
            while let Some(&k) = work.iter().next() {
                work.remove(&k);
                steps += 1;
                if steps > limit {
                    return Err("dataflow lùi không hội tụ trong trần số bước".to_string());
                }
                let b = rpo[key(k)];
                let bd = &body.blocks[b.index()];
                let mut st = a.bottom(body);
                if let Some(t) = &bd.terminator {
                    for s in t.kind.successors() {
                        if s.index() < n {
                            st.join(&before[s.index()]);
                        }
                    }
                }
                state[b.index()] = st.clone();
                if let Some(t) = &bd.terminator {
                    a.terminator_effect(
                        &mut st,
                        t,
                        Location {
                            block: b,
                            statement: bd.statements.len(),
                        },
                    );
                }
                for (j, s) in bd.statements.iter().enumerate().rev() {
                    a.statement_effect(
                        &mut st,
                        s,
                        Location {
                            block: b,
                            statement: j,
                        },
                    );
                }
                if before[b.index()] != st {
                    before[b.index()] = st;
                    for p in &preds[b.index()] {
                        if pos[p.index()] != usize::MAX {
                            work.insert(key(pos[p.index()]));
                        }
                    }
                }
            }
        }
    }
    Ok(Results { state, steps })
}

/// Trạng thái NGAY TRƯỚC vị trí `loc` (tiến: sau các câu lệnh trước nó; lùi: trước khi áp lùi câu lệnh tại `loc`,
/// tức trạng thái ngay SAU `loc` theo chiều chương trình).
pub fn state_at<A: Analysis>(
    a: &A,
    body: &Body,
    r: &Results<A::Domain>,
    loc: Location,
) -> A::Domain {
    let bd = &body.blocks[loc.block.index()];
    let mut st = r.state[loc.block.index()].clone();
    match A::DIRECTION {
        Direction::Forward => {
            for (k, s) in bd.statements.iter().enumerate().take(loc.statement) {
                a.statement_effect(
                    &mut st,
                    s,
                    Location {
                        block: loc.block,
                        statement: k,
                    },
                );
            }
        }
        Direction::Backward => {
            if let Some(t) = &bd.terminator
                && loc.statement < bd.statements.len()
            {
                a.terminator_effect(
                    &mut st,
                    t,
                    Location {
                        block: loc.block,
                        statement: bd.statements.len(),
                    },
                );
                for (j, s) in bd.statements.iter().enumerate().rev() {
                    if j <= loc.statement {
                        break;
                    }
                    a.statement_effect(
                        &mut st,
                        s,
                        Location {
                            block: loc.block,
                            statement: j,
                        },
                    );
                }
            }
        }
    }
    st
}
