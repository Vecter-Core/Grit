//! Giá trị region = tập điểm CFG + các `end(U)` của region phổ quát; giải outlives tới điểm cố định (ADR-023 §1).
use crate::regions::Gen;
use gt_dataflow::{BitSet, JoinSemiLattice, Liveness};
use gt_mir::*;

/// Chỉ số điểm: mỗi `Location` (khối, câu lệnh; chỉ số `len` là terminator) một số.
pub(crate) struct Points {
    prefix: Vec<usize>,
    pub total: usize,
}

impl Points {
    pub fn new(body: &Body) -> Points {
        let mut prefix = Vec::with_capacity(body.blocks.len());
        let mut t = 0;
        for b in &body.blocks {
            prefix.push(t);
            t += b.statements.len() + 1;
        }
        Points { prefix, total: t }
    }

    pub fn idx(&self, l: Location) -> usize {
        self.prefix[l.block.index()] + l.statement
    }
}

pub(crate) struct Values {
    pub v: Vec<BitSet>,
    pub npoints: usize,
}

impl Values {
    pub fn contains_point(&self, region: usize, p: usize) -> bool {
        self.v[region].contains(p)
    }

    /// `end(j)` có trong giá trị của `region` không (j là chỉ số trong `Gen.univ`).
    pub fn has_end(&self, region: usize, j: usize) -> bool {
        self.v[region].contains(self.npoints + j)
    }
}

/// Liveness theo từng điểm: các local sống NGAY TRƯỚC điểm đó (live-in).
pub(crate) fn live_before(body: &Body, pts: &Points) -> Result<Vec<BitSet>, String> {
    let r = gt_dataflow::solve(&Liveness, body)?;
    let mut out = vec![BitSet::empty(body.locals.len()); pts.total];
    for (bi, bb) in body.blocks.iter().enumerate() {
        let b = BasicBlock(bi as u32);
        let mut st = r.state[bi].clone();
        let tloc = Location {
            block: b,
            statement: bb.statements.len(),
        };
        if let Some(t) = &bb.terminator {
            gt_dataflow::Analysis::terminator_effect(&Liveness, &mut st, t, tloc);
        }
        out[pts.idx(tloc)] = st.clone();
        for (j, s) in bb.statements.iter().enumerate().rev() {
            let loc = Location {
                block: b,
                statement: j,
            };
            gt_dataflow::Analysis::statement_effect(&Liveness, &mut st, s, loc);
            out[pts.idx(loc)] = st.clone();
        }
    }
    Ok(out)
}

pub(crate) fn solve(g: &Gen<'_>, pts: &Points, live: &[BitSet]) -> Values {
    let body = g.body;
    let nuniv = g.univ.len();
    let width = pts.total + nuniv;
    let mut v: Vec<BitSet> = (0..g.n).map(|_| BitSet::empty(width)).collect();
    // điểm sống: local x sống tại P ⇒ mọi region trong kiểu của x chứa P
    for (p, lv) in live.iter().enumerate() {
        for x in lv.iter() {
            for r in &body.locals[x].regions {
                v[r.0 as usize].insert(p);
            }
        }
    }
    // region phổ quát: mọi điểm + end(chính nó); 'static: mọi end
    for (j, (var, _)) in g.univ.iter().enumerate() {
        for p in 0..pts.total {
            v[*var].insert(p);
        }
        v[*var].insert(pts.total + j);
        if j == 0 {
            for k in 0..nuniv {
                v[*var].insert(pts.total + k);
            }
        }
    }
    // lan truyền: a : b ⇒ giá trị(a) ⊇ giá trị(b)
    let mut succ: Vec<Vec<usize>> = vec![Vec::new(); g.n];
    for c in &g.cons {
        succ[c.b].push(c.a);
    }
    let mut work: std::collections::BTreeSet<usize> = (0..g.n).collect();
    while let Some(&b) = work.iter().next() {
        work.remove(&b);
        let vb = v[b].clone();
        for &a in &succ[b] {
            if v[a].join(&vb) {
                work.insert(a);
            }
        }
    }
    Values {
        v,
        npoints: pts.total,
    }
}
