//! E1001 (M7, D21/D22, Q12/Q13; cài ở M9 khi gritc thành cổng thật): `unsafe` thiếu bằng chứng.
//!
//! * **`⚠` trong mã người dùng ⇒ ĐỎ.** Mỗi nghĩa vụ của `⚠` (hợp lệ con trỏ, căn lề, không alias, độ sống, không data race…) phải được GIẢI bởi bộ
//!   giải (D21: "không có cơ chế tin tôi đi"). Phase 1 chưa có nghĩa vụ nào của `⚠` giải được (mệnh đề `⊢` chưa có ngữ nghĩa chứng minh) nên
//!   MỌI `⚠` là chưa giải ⇒ E1001, kể cả khối rỗng. Chỉ `lib/core` có `intrinsic trusted` (D22) và không bị kiểm ở đây.
//! * **Kiểu con trỏ thô `*□ T` / `*μ T` chỉ hợp lệ BÊN TRONG `⚠`** (Q13): có trong chữ ký hàm, trường ADT, kiểu hằng/static, tham số, hoặc
//!   một biến gán ngoài `⚠` ⇒ E1001.
//!
//! Fail-closed: dùng cờ `unsafe_ctx` do M5 ghi sẵn trong từng phạm vi MIR; thiếu thông tin thì coi như NGOÀI `⚠`.
use gt_diag::{Code, Diagnostic};
use gt_mir::{Body, BodyKind, MirOutput, StatementKind, TerminatorKind};
use gt_span::Span;
use gt_types::{AdtKind, Ty, TypesOutput};
use std::collections::BTreeSet;

fn has_raw_ptr(ty: &Ty, depth: usize) -> bool {
    // GÓI 8 S0g(ii): giới hạn 32 làm tuple lồng 33 tầng (không có con trỏ thô) bị E1001 oan. Kiểu là cây hữu hạn; giới hạn chỉ bảo vệ ngăn xếp.
    // Vượt 256 tầng vẫn coi là "có" (fail-closed; mã E1001 khi đó KHÔNG phải mã đúng: xem KNOWN_LIMITATIONS).
    if depth > 256 {
        return true;
    }
    match ty {
        Ty::RawPtr(..) => true,
        Ty::Ref(_, t) | Ty::Array(t, _) | Ty::Slice(t) => has_raw_ptr(t, depth + 1),
        Ty::Tuple(v) => v.iter().any(|t| has_raw_ptr(t, depth + 1)),
        Ty::Adt(_, args) => args.iter().any(|t| has_raw_ptr(t, depth + 1)),
        Ty::Fn(ps, r, _) => {
            ps.iter().any(|t| has_raw_ptr(t, depth + 1)) || has_raw_ptr(r, depth + 1)
        }
        _ => false,
    }
}

fn diag(t: &TypesOutput, node: gt_ast::NodeId, span: Span, msg: String) -> Diagnostic {
    let c = Code::parse("E1001").unwrap_or_else(|_| Code::parse("E9901").expect("E9901 hợp lệ"));
    Diagnostic::new(c, &t.file_name(node), span, msg)
}

fn is_prelude(t: &TypesOutput, node: gt_ast::NodeId) -> bool {
    t.file_name(node) == "<prelude>"
}

/// Thân hàm: `⚠` và con trỏ thô gán ngoài `⚠`.
fn check_body(t: &TypesOutput, b: &Body, out: &mut Vec<Diagnostic>) {
    // phạm vi ⚠: báo một lần cho mỗi phạm vi `unsafe_ctx` mà cha của nó KHÔNG phải ⚠ (khối ⚠ ngoài cùng)
    let outermost: BTreeSet<usize> = b
        .scopes
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            s.unsafe_ctx
                && !s
                    .parent
                    .is_some_and(|p| b.scopes.get(p.index()).is_some_and(|ps| ps.unsafe_ctx))
        })
        .map(|(i, _)| i)
        .collect();
    let in_unsafe = |scope: usize| b.scopes.get(scope).is_some_and(|s| s.unsafe_ctx);
    let mut first_span: std::collections::BTreeMap<usize, Span> = std::collections::BTreeMap::new();
    let mut raw_assign: Option<Span> = None;
    for bb in &b.blocks {
        for s in &bb.statements {
            let sc = s.source.scope.index();
            if in_unsafe(sc) {
                // gốc của khối ⚠ chứa câu lệnh này
                let mut cur = sc;
                while let Some(p) = b.scopes[cur].parent {
                    if b.scopes[p.index()].unsafe_ctx {
                        cur = p.index();
                    } else {
                        break;
                    }
                }
                first_span
                    .entry(cur)
                    .and_modify(|v| {
                        if s.source.span.start < v.start {
                            *v = s.source.span;
                        }
                    })
                    .or_insert(s.source.span);
            } else if let StatementKind::Assign(dst, _) = &s.kind
                && b.locals
                    .get(dst.local.index())
                    .is_some_and(|d| has_raw_ptr(&d.ty, 0))
            {
                raw_assign.get_or_insert(s.source.span);
            }
        }
        if let Some(term) = &bb.terminator {
            let sc = term.source.scope.index();
            if in_unsafe(sc) {
                let mut cur = sc;
                while let Some(p) = b.scopes[cur].parent {
                    if b.scopes[p.index()].unsafe_ctx {
                        cur = p.index();
                    } else {
                        break;
                    }
                }
                first_span
                    .entry(cur)
                    .and_modify(|v| {
                        if term.source.span.start < v.start {
                            *v = term.source.span;
                        }
                    })
                    .or_insert(term.source.span);
            } else if let TerminatorKind::Call { dest, .. } = &term.kind
                && b.locals
                    .get(dst_local(dest))
                    .is_some_and(|d| has_raw_ptr(&d.ty, 0))
            {
                raw_assign.get_or_insert(term.source.span);
            }
        }
    }
    for sc in &outermost {
        let span = first_span.get(sc).copied().unwrap_or(b.span);
        out.push(diag(
            t,
            b.node,
            span,
            format!(
                "`⚠` trong `{}`: nghĩa vụ unsafe chưa giải được (D21) — Phase 1 chưa có cơ chế chứng minh nghĩa vụ `⊢`, nên mọi `⚠` trong mã người dùng bị từ chối",
                b.key
            ),
        ));
    }
    // tham số / kết quả chứa con trỏ thô: nằm ngoài mọi ⚠
    let mut sig_flagged = false;
    for (i, d) in b.locals.iter().enumerate() {
        let is_arg = (1..=b.arg_count).contains(&i);
        if (is_arg || i == 0) && has_raw_ptr(&d.ty, 0) {
            out.push(diag(
                t,
                b.node,
                b.span,
                format!(
                    "kiểu con trỏ thô trong chữ ký của `{}`: `*□ T`/`*μ T` chỉ hợp lệ bên trong `⚠` (Q13)",
                    b.key
                ),
            ));
            sig_flagged = true;
            break;
        }
    }
    // đã báo ở chữ ký thì không báo thêm "tạo ngoài ⚠" cho cùng hàm (tránh nhiễu)
    if let Some(span) = raw_assign.filter(|_| !sig_flagged) {
        out.push(diag(
            t,
            b.node,
            span,
            format!(
                "giá trị kiểu con trỏ thô được tạo ngoài `⚠` trong `{}` (Q13)",
                b.key
            ),
        ));
    }
}

fn dst_local(p: &gt_mir::Place) -> usize {
    p.local.index()
}

/// E1001 cho cả chương trình (mã người dùng; prelude/lib/core được miễn theo D22).
#[must_use]
pub fn check_unsafe(t: &TypesOutput, m: &MirOutput) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for b in m.bodies.iter().filter(|b| b.kind == BodyKind::Fn) {
        if !is_prelude(t, b.node) {
            check_body(t, b, &mut out);
        }
    }
    // thành viên trait: chữ ký KHÔNG có thân MIR (mọi hàm/phương thức có thân đã được `check_body` kiểm cả chữ ký)
    for td in t.db.traits.values() {
        for mem in &td.members {
            if !is_prelude(t, mem.node)
                && (mem.params.iter().any(|p| has_raw_ptr(p, 0)) || has_raw_ptr(&mem.ret, 0))
            {
                out.push(diag(
                    t,
                    mem.node,
                    mem.span,
                    format!(
                        "kiểu con trỏ thô trong chữ ký của `{}`: chỉ hợp lệ bên trong `⚠` (Q13)",
                        mem.name
                    ),
                ));
            }
        }
    }
    // trường ADT
    for (d, adt) in &t.db.adts {
        let def = &t.defs()[d.index()];
        if is_prelude(t, def.node) {
            continue;
        }
        let tys: Vec<&Ty> = match &adt.kind {
            AdtKind::Struct(fs) => fs.iter().map(|f| &f.ty).collect(),
            AdtKind::Enum(vs) => vs.iter().flat_map(|v| v.fields.iter()).collect(),
        };
        if tys.iter().any(|ty| has_raw_ptr(ty, 0)) {
            out.push(diag(
                t,
                def.node,
                def.name_span,
                format!(
                    "trường kiểu con trỏ thô trong `{}`: chỉ hợp lệ bên trong `⚠` (Q13)",
                    def.path
                ),
            ));
        }
    }
    // hằng / static
    for (d, ty) in &t.db.consts {
        let def = &t.defs()[d.index()];
        if !is_prelude(t, def.node) && has_raw_ptr(ty, 0) {
            out.push(diag(
                t,
                def.node,
                def.name_span,
                format!(
                    "kiểu con trỏ thô của `{}`: chỉ hợp lệ bên trong `⚠` (Q13)",
                    def.path
                ),
            ));
        }
    }
    out.sort_by(|a, b| {
        (&a.file, a.span.start, a.span.end, &a.message).cmp(&(
            &b.file,
            b.span.start,
            b.span.end,
            &b.message,
        ))
    });
    out.dedup_by(|a, b| a.file == b.file && a.span == b.span && a.message == b.message);
    out
}
