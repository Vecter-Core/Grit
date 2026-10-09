#![forbid(unsafe_code)]
//! gt-mir (M5, ADR-020): MIR — biểu diễn, bước hạ từ AST + bảng của gt-types (nơi DUY NHẤT đọc AST sau M4), in text ổn
//! định, validator, và kiểm đầy đủ E0307 (thiếu đường trả về / hàm `→ !` có thể trả về).
//!
//! Fail-closed: chỉ hạ khi kiểm kiểu sạch; lỗi bất thường khi hạ là ICE (E9901), không bao giờ bỏ qua một thân.
mod build;
pub mod mir;
pub mod print;
pub mod types;
pub mod validate;

pub use mir::*;
pub mod interp;

use gt_ast::{Item, ItemKind, NodeId};
use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_types::{BodyKind as TBodyKind, Ty, TypesOutput};
use std::collections::BTreeMap;

const LIMIT: usize = 100;

/// Kết quả hạ MIR của cả chương trình.
#[derive(Debug)]
pub struct MirOutput {
    /// E0307 và ICE (nếu có). Rỗng = mọi thân hạ được và qua kiểm đầy đủ.
    pub report: Report,
    /// Mọi thân theo thứ tự `TypesOutput.bodies` (NodeId).
    pub bodies: Vec<Body>,
    /// Khoá (đường dẫn) của mọi hàm/item theo NodeId — cho printer.
    pub keys: BTreeMap<NodeId, String>,
}

impl MirOutput {
    #[must_use]
    pub fn body(&self, key: &str) -> Option<&Body> {
        self.bodies.iter().find(|b| b.key == key)
    }
}

enum Src<'a> {
    Fn(&'a gt_ast::FnItem),
    Const(&'a gt_ast::Expr),
}

fn index_items<'a>(items: &'a [Item], out: &mut BTreeMap<NodeId, Src<'a>>) {
    for it in items {
        match &it.kind {
            ItemKind::Fn(f) => {
                out.insert(it.id, Src::Fn(f));
            }
            ItemKind::Impl(i) => {
                for m in &i.members {
                    out.insert(m.id, Src::Fn(&m.func));
                }
            }
            ItemKind::Const(c) => {
                out.insert(it.id, Src::Const(&c.value));
            }
            ItemKind::Static(s) => {
                out.insert(it.id, Src::Const(&s.value));
            }
            ItemKind::Mod(m) => index_items(&m.items, out),
            _ => {}
        }
    }
}

fn diag(
    t: &TypesOutput,
    code: &str,
    node: NodeId,
    span: gt_span::Span,
    msg: String,
) -> Option<Diagnostic> {
    let c = Code::parse(code).ok()?;
    Some(Diagnostic::new(c, &t.file_name(node), span, msg))
}

/// Hạ mọi thân của chương trình. Gọi khi `t.report` sạch; nếu không, trả về rỗng (fail-closed: không hạ chương trình sai).
#[must_use]
pub fn lower_program(t: &TypesOutput) -> MirOutput {
    let mut keys: BTreeMap<NodeId, String> = t.fn_keys.clone();
    for b in &t.bodies {
        keys.insert(b.node, b.key.clone());
    }
    let mut bag = Diagnostics::new(LIMIT);
    if !t.report.diagnostics.is_empty() {
        return MirOutput {
            report: bag.finish(),
            bodies: Vec::new(),
            keys,
        };
    }
    let result = gt_stack::run_with_big_stack(|| lower_all(t));
    let (bodies, diags) = match result {
        Ok(x) => x,
        Err(e) => (
            Vec::new(),
            diag(
                t,
                "E9901",
                NodeId::new(gt_span::FileId::new(0), 0),
                gt_span::Span::new(0, 0),
                format!("lỗi nội bộ của compiler (ICE) khi hạ MIR: {e}"),
            )
            .into_iter()
            .collect(),
        ),
    };
    for b in &bodies {
        if matches!(b.kind, BodyKind::Pre | BodyKind::Post) {
            keys.insert(b.node, b.key.clone());
        }
    }
    for d in diags {
        bag.push(d);
    }
    MirOutput {
        report: bag.finish(),
        bodies,
        keys,
    }
}

fn lower_all(t: &TypesOutput) -> (Vec<Body>, Vec<Diagnostic>) {
    let mut srcs: BTreeMap<NodeId, Src<'_>> = BTreeMap::new();
    for p in &t.resolve.programs {
        index_items(&p.items, &mut srcs);
    }
    if let Some(pp) = &t.resolve.prelude_program {
        index_items(&pp.items, &mut srcs);
    }
    let mut bodies = Vec::new();
    let mut diags = Vec::new();
    for br in &t.prelude_contracts {
        if let Some(Src::Fn(f)) = srcs.get(&br.node) {
            lower_contracts(t, br, f, &mut bodies, &mut diags);
        }
    }
    for br in &t.bodies {
        let built = match (srcs.get(&br.node), br.kind) {
            (Some(Src::Fn(f)), TBodyKind::Fn) => {
                let self_ty = br.impl_node.and_then(|n| {
                    t.db.impls
                        .iter()
                        .find(|i| i.node == n)
                        .map(|i| i.self_ty.clone())
                });
                build::build_fn(t, &br.key, br.node, f, self_ty)
            }
            (Some(Src::Const(v)), k @ (TBodyKind::Const | TBodyKind::Static)) => {
                let ty = t
                    .def_by_node
                    .get(&br.node)
                    .and_then(|d| t.db.consts.get(d))
                    .cloned()
                    .unwrap_or(Ty::Err);
                let kind = if k == TBodyKind::Const {
                    BodyKind::Const
                } else {
                    BodyKind::Static
                };
                build::build_const(t, &br.key, br.node, kind, ty, v)
            }
            _ => Err(build::BuildError(format!(
                "không tìm thấy AST của {}",
                br.key
            ))),
        };
        match built {
            Ok(b) => {
                if let Err(errs) = validate::validate(&b, &t.db) {
                    diags.extend(diag(
                        t,
                        "E9901",
                        br.node,
                        b.span,
                        format!(
                            "lỗi nội bộ của compiler (ICE): MIR của `{}` không hợp lệ: {}",
                            br.key,
                            errs.join("; ")
                        ),
                    ));
                }
                diags.extend(completeness(t, &b));
                bodies.push(b);
                if let (Some(Src::Fn(f)), TBodyKind::Fn) = (srcs.get(&br.node), br.kind) {
                    lower_contracts(t, br, f, &mut bodies, &mut diags);
                }
            }
            Err(build::BuildError(m)) => {
                diags.extend(diag(
                    t,
                    "E9901",
                    br.node,
                    gt_span::Span::new(0, 0),
                    format!("lỗi nội bộ của compiler (ICE) khi hạ `{}`: {m}", br.key),
                ));
            }
        }
    }
    (bodies, diags)
}

/// Hạ các hợp đồng `⊳`/`⊲` của một hàm thành thân thuần (ADR-025 §6). Khoá: `<khoá hàm>⊳<i>` / `<khoá hàm>⊲<i>`.
fn lower_contracts(
    t: &TypesOutput,
    br: &gt_types::BodyRef,
    f: &gt_ast::FnItem,
    bodies: &mut Vec<Body>,
    diags: &mut Vec<Diagnostic>,
) {
    let self_ty = br.impl_node.and_then(|n| {
        t.db.impls
            .iter()
            .find(|i| i.node == n)
            .map(|i| i.self_ty.clone())
    });
    for (i, c) in f.sig.contracts.iter().enumerate() {
        let mark = match c.kind {
            gt_ast::ContractKind::Pre => "⊳",
            gt_ast::ContractKind::Post => "⊲",
            gt_ast::ContractKind::Variant => continue,
        };
        let key = format!("{}{mark}{i}", br.key);
        match build::build_contract(t, &key, br.node, f, self_ty.clone(), c) {
            Ok(b) => {
                if let Err(errs) = validate::validate(&b, &t.db) {
                    diags.extend(diag(
                        t,
                        "E9901",
                        c.id,
                        b.span,
                        format!(
                            "lỗi nội bộ của compiler (ICE): MIR của hợp đồng `{key}` không hợp lệ: {}",
                            errs.join("; ")
                        ),
                    ));
                }
                bodies.push(b);
            }
            Err(build::BuildError(m)) => {
                diags.extend(diag(
                    t,
                    "E9901",
                    c.id,
                    c.span,
                    format!("lỗi nội bộ của compiler (ICE) khi hạ hợp đồng `{key}`: {m}"),
                ));
            }
        }
    }
}

/// Kiểm đầy đủ (ADR-020 §4): E0307 khi lối rơi cuối hàm tới được; hàm `→ !` có `Return` tới được.
#[must_use]
pub fn completeness(t: &TypesOutput, b: &Body) -> Vec<Diagnostic> {
    let reach = validate::reachable(b);
    let mut out = Vec::new();
    if let Some((bb, span)) = b.fall_off
        && reach[bb.index()]
    {
        let rt = b.return_ty().render(t.defs());
        out.extend(diag(
            t,
            "E0307",
            b.node,
            span,
            format!("thiếu đường trả về: có đường chạy tới cuối thân hàm mà không `⏎` một giá trị kiểu `{rt}` (khối kết thúc bằng `;` không trả giá trị)"),
        ));
    }
    if *b.return_ty() == Ty::Never {
        for (i, bb) in b.blocks.iter().enumerate() {
            if reach[i]
                && let Some(term) = &bb.terminator
                && term.kind == TerminatorKind::Return
            {
                out.extend(diag(
                    t,
                    "E0307",
                    b.node,
                    term.source.span,
                    "hàm khai báo `→ !` nhưng có đường trả về: mọi đường phải phân kỳ (gọi hàm `→ !`, vòng lặp vô hạn)".to_string(),
                ));
                break;
            }
        }
    }
    out
}
