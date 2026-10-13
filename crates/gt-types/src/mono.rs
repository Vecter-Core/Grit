//! Bộ thu thập instance (đơn hình hoá LAZY, ADR-018 §1): hàng đợi từ các gốc không generic; KHÔNG sao chép thân hàm.
use crate::check::CallRec;
use crate::collect::{Env, FnBody};
use crate::ty::{ParamId, Ty};
use gt_ast::NodeId;
use std::collections::{BTreeMap, BTreeSet};

/// Ngưỡng độ lồng kiểu của tham số instance (đệ quy đa hình → E0312).
pub const MAX_INSTANCE_DEPTH: u32 = 64;
/// Cận TỔNG số instance của một chương trình (khác ngưỡng độ sâu: chặn cả chương trình KHÔNG đệ quy nhưng lồng tham số kiểu để nổ
/// số instance). Vượt ⇒ ĐỎ E0317.
pub const MAX_PROGRAM_INSTANCES: usize = 10_000;

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
        let mut capped = false;
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
                        let Some(r) = self.db.resolve_trait_method(*td, *mi, &st, &args) else {
                            continue;
                        };
                        r
                    }
                    _ => continue,
                };
                if seen.len() > MAX_PROGRAM_INSTANCES {
                    let at = bodies
                        .iter()
                        .find(|b| b.node == callee)
                        .map_or(gt_span::Span::new(0, 0), |b| b.sig.span);
                    self.err(
                        "E0317",
                        callee,
                        at,
                        format!("chương trình sinh quá {MAX_PROGRAM_INSTANCES} instance sau đơn hình hoá (bùng nổ tổ hợp kiểu)"),
                    );
                    capped = true;
                    break;
                }
                if args.iter().any(|a| a.depth() > MAX_INSTANCE_DEPTH) {
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
            if capped {
                break;
            }
        }
        self.out_calls = calls;
        seen.into_iter().collect()
    }
}

/// Tên instance cho IR: `symbol` TOÀN ASCII (`[A-Za-z0-9_.]`) = phần đọc được đã làm sạch + `.` + băm FNV-1a 64 bit (16 hex) của khoá
/// CHUẨN; `readable` là dạng đọc được (có thể chứa Unicode) — nằm trong metadata `!gt.instances` (ADR-032 phụ lục A, Q69).
#[must_use]
pub fn instance_name(key: &str, args: &[Ty], defs: &[gt_resolve::Def]) -> (String, String) {
    let list = |f: &dyn Fn(&Ty) -> String| args.iter().map(f).collect::<Vec<_>>().join(", ");
    let readable = if args.is_empty() {
        key.to_string()
    } else {
        format!("{key}<{}>", list(&|t| t.render(defs)))
    };
    let canon = format!("{key}<{}>", list(&|t| t.canon(defs)));
    let mut clean: String = readable
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    clean.truncate(80);
    (
        format!("{clean}.{:016x}", fnv1a64(canon.as_bytes())),
        readable,
    )
}

/// FNV-1a 64 bit (tự cài: `DefaultHasher` không bảo đảm ổn định giữa các bản Rust).
#[must_use]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}
