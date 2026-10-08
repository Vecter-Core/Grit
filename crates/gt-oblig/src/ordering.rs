//! Memory ordering của atomics (M8 nửa 2, ADR-027 §4): E0503.
//!
//! Mọi thao tác của `Atomic<T>` nhận `Ordering` TƯỜNG MINH (không tham số mặc định ⇒ thiếu là lỗi arity thường). Ở đây kiểm:
//! (1) đối số ordering phải truy ngược được về MỘT biến thể viết thẳng (`Ordering::Acquire`, hoặc biến cục bộ gán đúng một lần từ nó);
//! không truy được ⇒ E0503 (fail-closed: không đoán); (2) tổ hợp hợp lệ: `load` ∈ {Relaxed, Acquire, SeqCst}; `store` ∈ {Relaxed, Release,
//! SeqCst}; `compare_exchange`: thứ tự thất bại ∈ {Relaxed, Acquire, SeqCst} và không mạnh hơn thứ tự thành công. Kiểm ở MIR để mọi cú pháp
//! gọi (`a.load(o)`, `Atomic::load(&a, o)`) cùng đi qua một chỗ.
use gt_diag::{Code, Diagnostic, Diagnostics};
use gt_mir::{
    AggregateKind, Body, BodyKind, Callee, Local, MirOutput, Operand, Rvalue, StatementKind,
    TerminatorKind,
};
use gt_span::Span;
use gt_types::{Lang, Ty, TypesOutput};

/// Chỉ số biến thể theo khai báo: Relaxed, Acquire, Release, AcqRel, SeqCst.
const NAMES: [&str; 5] = ["Relaxed", "Acquire", "Release", "AcqRel", "SeqCst"];

fn mk(t: &TypesOutput, node: gt_ast::NodeId, span: Span, msg: String) -> Diagnostic {
    let c = Code::parse("E0503").unwrap_or_else(|_| Code::parse("E9901").expect("E9901 hợp lệ"));
    Diagnostic::new(c, &t.file_name(node), span, msg)
}

/// Truy ngược một toán hạng về chỉ số biến thể `Ordering` (qua tối đa vài lần sao chép, mỗi local chỉ gán MỘT lần).
fn variant_of(b: &Body, op: &Operand, ord_def: gt_resolve::DefId, depth: u32) -> Option<u32> {
    if depth > 8 {
        return None;
    }
    let (Operand::Copy(p) | Operand::Move(p)) = op else {
        return None;
    };
    if !p.projection.is_empty() {
        return None;
    }
    let l: Local = p.local;
    let mut found: Option<&Rvalue> = None;
    for bb in &b.blocks {
        for s in &bb.statements {
            if let StatementKind::Assign(dst, rv) = &s.kind
                && dst.local == l
            {
                if !dst.projection.is_empty() || found.is_some() {
                    return None;
                }
                found = Some(rv);
            }
        }
        // gán qua đích của lời gọi cũng là một định nghĩa khác
        if let Some(TerminatorKind::Call { dest, .. }) = bb.terminator.as_ref().map(|t| &t.kind)
            && dest.local == l
        {
            return None;
        }
    }
    match found? {
        Rvalue::Aggregate(AggregateKind::Adt(d, idx, _), ops)
            if *d == ord_def && ops.is_empty() =>
        {
            Some(*idx)
        }
        Rvalue::Use(o) => variant_of(b, o, ord_def, depth + 1),
        _ => None,
    }
}

pub fn check_orderings(t: &TypesOutput, m: &MirOutput, bag: &mut Diagnostics) {
    let Some(&ord_def) = t.db.lang.get(&Lang::Ordering) else {
        return;
    };
    for b in m.bodies.iter().filter(|b| b.kind == BodyKind::Fn) {
        for bb in &b.blocks {
            let Some(term) = &bb.terminator else { continue };
            let TerminatorKind::Call {
                callee: Callee::Fn { node, .. },
                args,
                ..
            } = &term.kind
            else {
                continue;
            };
            let span = term.source.span;
            let Some(imp) =
                t.db.impls
                    .iter()
                    .find(|i| i.members.iter().any(|mm| mm.node == *node))
            else {
                continue;
            };
            let is_atomic = matches!(&imp.self_ty, Ty::Adt(d, _) if t.db.adts.get(d).and_then(|a| a.lang) == Some(Lang::Atomic));
            if !is_atomic {
                continue;
            }
            let Some(sig) = t.db.sig_by_node(*node) else {
                continue;
            };
            let off = usize::from(sig.self_kind.is_some());
            // các tham số kiểu `Ordering` (vị trí trong danh sách đối số)
            let mut ords: Vec<(usize, Option<u32>)> = Vec::new();
            for (i, p) in sig.params.iter().enumerate() {
                if matches!(p, Ty::Adt(d, _) if *d == ord_def) {
                    let v = args.get(i + off).and_then(|a| variant_of(b, a, ord_def, 0));
                    ords.push((i, v));
                }
            }
            let mut bad = false;
            for (_, v) in &ords {
                if v.is_none() {
                    bad = true;
                    bag.push(mk(
                        t,
                        b.node,
                        span,
                        format!(
                            "đối số memory ordering của `{}` phải viết thẳng là một biến thể (`Ordering::Acquire`…), không tính toán được",
                            sig.name
                        ),
                    ));
                }
            }
            if bad {
                continue;
            }
            let name = |v: u32| NAMES.get(v as usize).copied().unwrap_or("?");
            let o = |k: usize| ords.get(k).and_then(|x| x.1).unwrap_or(4);
            let problem: Option<String> = match sig.name.as_str() {
                "load" if matches!(o(0), 2 | 3) => Some(format!(
                    "`load` không nhận ordering `{}` (chỉ Relaxed, Acquire, SeqCst)",
                    name(o(0))
                )),
                "store" if matches!(o(0), 1 | 3) => Some(format!(
                    "`store` không nhận ordering `{}` (chỉ Relaxed, Release, SeqCst)",
                    name(o(0))
                )),
                "compare_exchange" => {
                    let (s, f) = (o(0), o(1));
                    if matches!(f, 2 | 3) {
                        Some(format!(
                            "thứ tự thất bại của `compare_exchange` không được là `{}` (chỉ Relaxed, Acquire, SeqCst)",
                            name(f)
                        ))
                    } else if f == 4 && s != 4 {
                        Some("thứ tự thất bại `SeqCst` đòi thứ tự thành công `SeqCst`".to_string())
                    } else if f == 1 && !matches!(s, 1 | 3 | 4) {
                        Some(format!(
                            "thứ tự thất bại `Acquire` mạnh hơn thứ tự thành công `{}`",
                            name(s)
                        ))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(msg) = problem {
                bag.push(mk(t, b.node, span, msg));
            }
        }
    }
}
