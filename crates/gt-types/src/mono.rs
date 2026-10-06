//! Bộ thu thập instance (đơn hình hoá LAZY, ADR-018 §1): hàng đợi từ các gốc không generic; KHÔNG sao chép thân hàm.
use crate::check::CallRec;
use crate::collect::{Env, FnBody};
use crate::ty::{ParamId, Ty};
use gt_ast::NodeId;
use std::collections::{BTreeMap, BTreeSet};

/// Ngưỡng độ lồng kiểu của tham số instance (đệ quy đa hình → E0312).
pub const MAX_INSTANCE_DEPTH: u32 = 64;
const MAX_INSTANCES: usize = 100_000;

/// Một instance: hàm/thành viên (NodeId) + tham số kiểu cụ thể (generics của impl rồi của hàm).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instance {
    pub node: NodeId,
    pub args: Vec<Ty>,
}

impl Env<'_> {
    pub(crate) fn collect_instances(&mut self, bodies: &[FnBody<'_>]) -> Vec<Instance> {
        // thứ tự tham số của mỗi hàm
        let mut params: BTreeMap<NodeId, Vec<ParamId>> = BTreeMap::new();
        let mut roots: Vec<NodeId> = Vec::new();
        for f in bodies {
            if f.body.is_none() || self.is_prelude(f.node.file()) {
                continue;
            }
            let mut ps: Vec<ParamId> = Vec::new();
            if let Some(i) = f.impl_idx {
                ps.extend(self.db.impls[i].generics.iter().map(|g| g.id));
            }
            ps.extend(f.sig.generics.iter().map(|g| g.id));
            if ps.is_empty() {
                roots.push(f.node);
            }
            params.insert(f.node, ps);
        }
        let calls = std::mem::take(&mut self.out_calls);
        let mut by_caller: BTreeMap<NodeId, Vec<&CallRec>> = BTreeMap::new();
        for c in &calls {
            by_caller.entry(c.caller).or_default().push(c);
        }
        let mut seen: BTreeSet<Instance> = BTreeSet::new();
        let mut queue: Vec<Instance> = Vec::new();
        for r in roots {
            let i = Instance {
                node: r,
                args: Vec::new(),
            };
            if seen.insert(i.clone()) {
                queue.push(i);
            }
        }
        let mut reported: BTreeSet<NodeId> = BTreeSet::new();
        while let Some(inst) = queue.pop() {
            let Some(ps) = params.get(&inst.node).cloned() else {
                continue;
            };
            let sub = |t: &Ty| -> Ty {
                t.subst(&|p| {
                    ps.iter()
                        .position(|q| *q == p)
                        .and_then(|i| inst.args.get(i).cloned())
                })
            };
            for c in by_caller.get(&inst.node).cloned().unwrap_or_default() {
                let args: Vec<Ty> = c.args.iter().map(sub).collect();
                let (callee, args) = match (&c.callee, &c.trait_method) {
                    (Some(n), _) => (*n, args),
                    (None, Some((td, mi, st))) => {
                        let st = sub(st);
                        if st.has_param() || st.has_infer() {
                            continue;
                        }
                        let Some(tm) = self
                            .db
                            .traits
                            .get(td)
                            .and_then(|t| t.members.get(*mi))
                            .map(|m| m.name.clone())
                        else {
                            continue;
                        };
                        let mut inf = crate::infer::InferCtx::new();
                        let Some((ii, iargs)) = self.db.find_impl(&mut inf, *td, &st, 0) else {
                            continue;
                        };
                        let Some(m) = self.db.impls[ii].members.iter().find(|m| m.name == tm)
                        else {
                            continue;
                        };
                        let mut all: Vec<Ty> = iargs.iter().map(|a| inf.resolve(a)).collect();
                        all.extend(args);
                        (m.node, all)
                    }
                    _ => continue,
                };
                if args.iter().any(|a| a.depth() > MAX_INSTANCE_DEPTH) || seen.len() > MAX_INSTANCES
                {
                    if reported.insert(callee) {
                        let name = bodies
                            .iter()
                            .find(|b| b.node == callee)
                            .map_or("?".to_string(), |b| b.key.clone());
                        self.err(
                            "E0312",
                            callee,
                            bodies.iter().find(|b| b.node == callee).map_or(gt_span::Span::new(0, 0), |b| b.sig.span),
                            format!("đệ quy đa hình: `{name}` sinh instance lồng sâu hơn {MAX_INSTANCE_DEPTH} mức (vô hạn instance)"),
                        );
                    }
                    continue;
                }
                let ni = Instance { node: callee, args };
                if params.contains_key(&callee) && seen.insert(ni.clone()) {
                    queue.push(ni);
                }
            }
        }
        self.out_calls = calls;
        seen.into_iter().collect()
    }
}
