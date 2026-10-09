#![forbid(unsafe_code)]
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
    let levels = Levels::new(t, &mut bag);

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
            let TerminatorKind::Call { callee, args, .. } = &term.kind else {
                continue;
            };
            let span = term.source.span;
            match callee {
                Callee::Fn { node, generics } => {
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
                    let cx = CallCx {
                        t,
                        b,
                        levels: &levels,
                        node: *node,
                        generics,
                        args,
                        span,
                    };
                    for e in call_effects(&cx, cs, &mut bag) {
                        add(&e, span, format!("lời gọi `{}`", cs.name));
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

/// Cấp khoá của các kiểu đánh dấu (`#level("n")`), khoá theo TÊN NGẮN (chuỗi trong hiệu ứng `lock(Tên)` do người dùng viết).
struct Levels {
    by_name: BTreeMap<String, u64>,
}

impl Levels {
    fn new(t: &TypesOutput, bag: &mut Diagnostics) -> Levels {
        let mut by_name: BTreeMap<String, u64> = BTreeMap::new();
        for (d, adt) in &t.db.adts {
            let Some(l) = adt.level else { continue };
            let def = &t.defs()[d.index()];
            let short = def.path.rsplit("::").next().unwrap_or("").to_string();
            if let Some(old) = by_name.insert(short.clone(), l)
                && old != l
            {
                bag.push(mk(
                    t,
                    "E0502",
                    def.node,
                    def.name_span,
                    format!("hai kiểu cấp khoá cùng tên `{short}` với cấp khác nhau ({old} và {l}): hiệu ứng `lock({short})` mơ hồ"),
                ));
            }
        }
        Levels { by_name }
    }
}

struct CallCx<'a> {
    t: &'a TypesOutput,
    b: &'a gt_mir::Body,
    levels: &'a Levels,
    node: gt_ast::NodeId,
    generics: &'a [Ty],
    args: &'a [gt_mir::Operand],
    span: Span,
}

/// Cấp tên ngắn của một hiệu ứng `lock(Tên)`.
fn lock_name(e: &str) -> Option<&str> {
    e.strip_prefix("lock(")?.strip_suffix(')')
}

/// Hiệu ứng của một lời gọi hàm (ADR-027 §3): hiệu ứng khai báo của callee, với
/// - marker `poly`: thêm hiệu ứng của mọi đối số kiểu hàm; `detach` (spawn): như `poly` nhưng BỎ `lock(*)` (worker không chạy lồng trong khoá);
/// - `lock(L)` của thành viên `Mutex`: thay `L` bằng tên kiểu cấp khoá của lời gọi; thứ tự khoá (E0502) được kiểm tại `Mutex::with`.
fn call_effects(cx: &CallCx<'_>, cs: &gt_types::FnSigInfo, bag: &mut Diagnostics) -> Vec<String> {
    let t = cx.t;
    let imp =
        t.db.impls
            .iter()
            .find(|i| i.members.iter().any(|m| m.node == cx.node));
    let mutex_lock = imp.and_then(|i| match &i.self_ty {
        Ty::Adt(d, a) if t.db.adts.get(d).and_then(|x| x.lang) == Some(gt_types::Lang::Mutex) => {
            // tham số cấp khoá = đối số kiểu thứ hai của `Mutex<T, L>`
            let Some(Ty::Param(pl)) = a.get(1) else {
                return None;
            };
            let pos = i.generics.iter().position(|g| g.id == *pl)?;
            cx.generics.get(pos).cloned()
        }
        _ => None,
    });
    let mut out: Vec<String> = Vec::new();
    for e in &cs.effects {
        match e.as_str() {
            "poly" | "detach" => {
                for a in cx.args {
                    if let Some(Ty::Fn(_, _, effs)) = gt_mir::types::operand_ty(cx.b, a) {
                        for x in effs {
                            if e == "detach" && lock_name(&x).is_some() {
                                continue;
                            }
                            out.push(x);
                        }
                    }
                }
            }
            x if lock_name(x).is_some() && mutex_lock.is_some() => {
                let lvl = match &mutex_lock {
                    Some(Ty::Adt(d, _)) => {
                        let short = t.defs()[d.index()]
                            .path
                            .rsplit("::")
                            .next()
                            .unwrap_or("")
                            .to_string();
                        Some(short)
                    }
                    _ => None,
                };
                match lvl {
                    Some(short) => {
                        out.push(format!("lock({short})"));
                        if cs.name == "with" {
                            check_lock_order(cx, &short, bag);
                        }
                    }
                    None => bag.push(mk(
                        t,
                        "E0502",
                        cx.b.node,
                        cx.span,
                        "cấp khoá của `Mutex` phải là một kiểu cụ thể có `#level` (không phải tham số kiểu)".to_string(),
                    )),
                }
            }
            x => out.push(x.to_string()),
        }
    }
    out
}

/// Thứ tự khoá tăng nghiêm ngặt (ADR-027 §3): mọi `lock(L')` trong hiệu ứng của hàm `f` truyền vào `Mutex::with` phải có cấp > cấp(L).
fn check_lock_order(cx: &CallCx<'_>, held: &str, bag: &mut Diagnostics) {
    let t = cx.t;
    let Some(&hl) = cx.levels.by_name.get(held) else {
        bag.push(mk(
            t,
            "E0502",
            cx.b.node,
            cx.span,
            format!("kiểu cấp khoá `{held}` thiếu thuộc tính `#level(\"n\")`"),
        ));
        return;
    };
    // tham số hàm `f` là đối số hàm đầu tiên sau người nhận
    let Some(Ty::Fn(_, _, effs)) =
        cx.args
            .iter()
            .skip(1)
            .find_map(|a| match gt_mir::types::operand_ty(cx.b, a) {
                Some(f @ Ty::Fn(..)) => Some(f),
                _ => None,
            })
    else {
        return;
    };
    for e in &effs {
        let Some(inner) = lock_name(e) else { continue };
        match cx.levels.by_name.get(inner) {
            Some(&il) if il > hl => {}
            Some(&il) => bag.push(mk(
                t,
                "E0502",
                cx.b.node,
                cx.span,
                format!("thứ tự khoá: đang giữ `{held}` (cấp {hl}) mà hàm bên trong lấy `{inner}` (cấp {il}); phải lấy khoá theo cấp TĂNG NGHIÊM NGẶT"),
            )),
            None => bag.push(mk(
                t,
                "E0502",
                cx.b.node,
                cx.span,
                format!("kiểu cấp khoá `{inner}` thiếu thuộc tính `#level(\"n\")`"),
            )),
        }
    }
}
