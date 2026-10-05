//! E0204 — vòng phụ thuộc module: thành phần liên thông mạnh có >1 module trên đồ thị cạnh S → T (ADR-017 mục 6).
use crate::model::DefId;
use crate::tables::Tables;
use gt_span::{FileId, Span};
use std::collections::{BTreeMap, BTreeSet};

struct Tarjan<'g> {
    adj: &'g BTreeMap<DefId, Vec<DefId>>,
    index: BTreeMap<DefId, usize>,
    low: BTreeMap<DefId, usize>,
    on_stack: BTreeSet<DefId>,
    stack: Vec<DefId>,
    next: usize,
    sccs: Vec<Vec<DefId>>,
}

impl Tarjan<'_> {
    fn visit(&mut self, v: DefId) {
        self.index.insert(v, self.next);
        self.low.insert(v, self.next);
        self.next += 1;
        self.stack.push(v);
        self.on_stack.insert(v);
        let succ: Vec<DefId> = self.adj.get(&v).cloned().unwrap_or_default();
        for w in succ {
            if !self.index.contains_key(&w) {
                self.visit(w);
                let lw = self.low[&w];
                if lw < self.low[&v] {
                    self.low.insert(v, lw);
                }
            } else if self.on_stack.contains(&w) {
                let iw = self.index[&w];
                if iw < self.low[&v] {
                    self.low.insert(v, iw);
                }
            }
        }
        if self.low[&v] == self.index[&v] {
            let mut comp = Vec::new();
            while let Some(w) = self.stack.pop() {
                self.on_stack.remove(&w);
                comp.push(w);
                if w == v {
                    break;
                }
            }
            comp.sort();
            self.sccs.push(comp);
        }
    }
}

pub(crate) fn report_cycles(t: &mut Tables) {
    let mut adj: BTreeMap<DefId, Vec<DefId>> = BTreeMap::new();
    let mut nodes: BTreeSet<DefId> = BTreeSet::new();
    for &(s, d) in t.edges.keys() {
        adj.entry(s).or_default().push(d);
        nodes.insert(s);
        nodes.insert(d);
    }
    for v in adj.values_mut() {
        v.sort();
    }
    let mut tj = Tarjan {
        adj: &adj,
        index: BTreeMap::new(),
        low: BTreeMap::new(),
        on_stack: BTreeSet::new(),
        stack: Vec::new(),
        next: 0,
        sccs: Vec::new(),
    };
    for n in nodes {
        if !tj.index.contains_key(&n) {
            tj.visit(n);
        }
    }
    let mut sccs: Vec<Vec<DefId>> = tj.sccs.into_iter().filter(|c| c.len() > 1).collect();
    sccs.sort();
    for comp in sccs {
        let members: BTreeSet<DefId> = comp.iter().copied().collect();
        // vị trí nhỏ nhất của mỗi cạnh trong thành phần
        let mut sites: Vec<(String, FileId, Span, DefId, DefId)> = Vec::new();
        for (&(s, d), v) in &t.edges {
            if members.contains(&s) && members.contains(&d) {
                let best = v
                    .iter()
                    .map(|&(f, sp)| (t.fname(f), f, sp))
                    .min_by(|a, b| (&a.0, a.2.start, a.2.end).cmp(&(&b.0, b.2.start, b.2.end)));
                if let Some((n, f, sp)) = best {
                    sites.push((n, f, sp, s, d));
                }
            }
        }
        sites.sort_by(|a, b| (&a.0, a.2.start, a.2.end).cmp(&(&b.0, b.2.start, b.2.end)));
        let names: Vec<String> = comp
            .iter()
            .map(|d| format!("`{}`", t.defs[d.index()].path))
            .collect();
        let msg = format!(
            "các module {} phụ thuộc lẫn nhau (vòng phụ thuộc module)",
            names.join(", ")
        );
        let Some((_, f0, s0, _, _)) = sites.first().cloned() else {
            continue;
        };
        let Some(mut d) = t.mk("E0204", f0, s0, msg) else {
            continue;
        };
        d = d.with_span_label("tham chiếu tạo vòng");
        for (n, _, sp, s, dst) in sites.iter().skip(1) {
            let m = format!(
                "`{}` phụ thuộc `{}`",
                t.defs[s.index()].path,
                t.defs[dst.index()].path
            );
            d = d.with_label(n, *sp, m);
        }
        let site = t.defs[comp[0].index()].path.clone();
        t.push("E0204", d, &site);
    }
}
