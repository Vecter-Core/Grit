//! Hiệu ứng (M8 nửa 1, ADR-026 §1; semantics §4): "khai báo ⊇ suy ra".
//!
//! Suy ra cho mỗi thân hàm = hợp của (a) hiệu ứng KHAI BÁO của mọi hàm được gọi — gọi trực tiếp (`Callee::Fn`), qua giá trị hàm
//! (hiệu ứng nằm trong kiểu `Ty::Fn`), hoặc qua phương thức trait (hiệu ứng khai báo trên thành viên trait) — và (b) KHÔNG suy `div` từ vòng vô hạn: điều kiện dừng (`termination`, E0701) sở hữu việc đó để không báo trùng. Vì mỗi hàm đã bị kiểm "khai báo ⊇ suy ra" nên dùng hiệu ứng KHAI BÁO của callee là đủ và truyền tới mọi tầng,
//! kể cả đệ quy tương hỗ (không cần điểm bất động).
//!
//! E0801: hiệu ứng suy ra ⊄ khai báo (kể cả hàm `∅`). E0802: chữ ký không tương thích về hiệu ứng — thành viên impl khai nhiều hiệu ứng
//! hơn thành viên trait, (hiệu ứng trong KIỂU hàm khớp chính xác và do bộ kiểm kiểu M4 bác bằng E0301 nên không cần kiểm lại ở đây). Không có đường lúc chạy: không chứng minh được ⇒ ĐỎ.
use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_mir::{BodyKind, Callee, MirOutput, TerminatorKind};
use gt_span::Span;
use gt_types::{Ty, TypesOutput};
use std::collections::{BTreeMap, BTreeSet};

const LIMIT: usize = 200;

#[derive(Debug)]
pub struct EffectsOutput {
    pub report: Report,
    /// hiệu ứng suy ra của từng thân hàm (khoá `Body::key`), đã sắp
    pub inferred: BTreeMap<String, Vec<String>>,
}

fn mk(t: &TypesOutput, code: &str, node: gt_ast::NodeId, span: Span, msg: String) -> Diagnostic {
    let c = Code::parse(code).unwrap_or_else(|_| Code::parse("E9901").expect("E9901 hợp lệ"));
    Diagnostic::new(c, &t.file_name(node), span, msg)
}

fn fmt_set(s: &BTreeSet<String>) -> String {
    if s.is_empty() {
        "∅".to_string()
    } else {
        format!("{{{}}}", s.iter().cloned().collect::<Vec<_>>().join(", "))
    }
}

/// `declared` có chứa `e` không. `lock(L)` chỉ được che bởi đúng `lock(L)` (so khớp chuỗi).
fn covers(declared: &BTreeSet<String>, e: &str) -> bool {
    declared.contains(e)
}

pub fn check_effects(t: &TypesOutput, m: &MirOutput) -> EffectsOutput {
    let mut bag = Diagnostics::new(LIMIT);
    let mut inferred_out: BTreeMap<String, Vec<String>> = BTreeMap::new();

    // ---- hiệu ứng suy ra từ thân hàm ----
    for b in m.bodies.iter().filter(|b| b.kind == BodyKind::Fn) {
        let Some(sig) = t.db.sig_by_node(b.node) else {
            continue;
        };
        let declared: BTreeSet<String> = sig.effects.iter().cloned().collect();
        // (hiệu ứng → nơi đầu tiên sinh ra nó, mô tả nguồn)
        let mut found: BTreeMap<String, (Span, String)> = BTreeMap::new();
        let mut add = |e: &str, span: Span, why: String| {
            found.entry(e.to_string()).or_insert((span, why));
        };
        for bb in &b.blocks {
            let Some(term) = &bb.terminator else { continue };
            let TerminatorKind::Call { callee, .. } = &term.kind else {
                continue;
            };
            let span = term.source.span;
            match callee {
                Callee::Fn { node, .. } => {
                    let Some(cs) = t.db.sig_by_node(*node) else {
                        bag.push(mk(
                            t,
                            "E9901",
                            b.node,
                            span,
                            format!("không tìm thấy chữ ký của hàm được gọi trong `{}`", b.key),
                        ));
                        continue;
                    };
                    for e in &cs.effects {
                        add(e, span, format!("lời gọi `{}`", cs.name));
                    }
                }
                Callee::TraitMethod {
                    trait_def, member, ..
                } => {
                    let Some(cs) = t
                        .db
                        .traits
                        .get(trait_def)
                        .and_then(|td| td.members.get(*member))
                    else {
                        bag.push(mk(
                            t,
                            "E9901",
                            b.node,
                            span,
                            format!("không tìm thấy thành viên trait được gọi trong `{}`", b.key),
                        ));
                        continue;
                    };
                    for e in &cs.effects {
                        add(e, span, format!("phương thức trait `{}`", cs.name));
                    }
                }
                Callee::Value(op) => match gt_mir::types::operand_ty(b, op) {
                    Some(Ty::Fn(_, _, effs)) => {
                        for e in &effs {
                            add(e, span, "lời gọi qua giá trị hàm".to_string());
                        }
                    }
                    other => bag.push(mk(
                        t,
                        "E9901",
                        b.node,
                        span,
                        format!(
                            "callee là giá trị không có kiểu hàm ({other:?}) trong `{}`: không suy được hiệu ứng",
                            b.key
                        ),
                    )),
                },
            }
        }
        inferred_out.insert(b.key.clone(), found.keys().cloned().collect());
        for (e, (span, why)) in &found {
            if !covers(&declared, e) {
                bag.push(mk(
                    t,
                    "E0801",
                    b.node,
                    *span,
                    format!(
                        "`{}` khai báo hiệu ứng {} nhưng suy ra `{e}` ({why})",
                        b.key,
                        fmt_set(&declared)
                    ),
                ));
            }
        }
    }

    // ---- chữ ký: thành viên impl ⊆ thành viên trait ----
    for imp in &t.db.impls {
        let Some(tr) = &imp.trait_ref else { continue };
        let Some(td) = t.db.traits.get(&tr.trait_def) else {
            continue;
        };
        for mem in &imp.members {
            let Some(tm) = td.members.iter().find(|x| x.name == mem.name) else {
                continue;
            };
            let allowed: BTreeSet<String> = tm.effects.iter().cloned().collect();
            for e in &mem.effects {
                if !covers(&allowed, e) {
                    bag.push(mk(
                        t,
                        "E0802",
                        mem.node,
                        mem.span,
                        format!(
                            "thành viên impl `{}` khai hiệu ứng `{e}` mà thành viên trait chỉ cho phép {}",
                            mem.name,
                            fmt_set(&allowed)
                        ),
                    ));
                }
            }
        }
    }

    EffectsOutput {
        report: bag.finish(),
        inferred: inferred_out,
    }
}
