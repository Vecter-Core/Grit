//! Điều phối mức chương trình (ADR-025 §5, §8): chạy phân tích từng thân, áp chính sách một nghĩa vụ, dựng MIR đã tối ưu
//! (xoá `Assert` đã chứng minh), phát chẩn đoán, và báo cáo "số check còn lại" dạng JSON.
//!
//! Chính sách (D24/D25): `Discharged` → xoá `Assert`; `DefinitelyFalse` → ĐỎ E060x; `Lowered` → giữ `Assert` + GHI CHÚ N06xx;
//! `Rejected` (không kiểm được lúc chạy mà chưa giải) → ĐỎ E0902.
use crate::analysis::{
    BodyReport, ContractProp, Insert, ProgInfo, Status, analyze_body, extract_contract,
};
use crate::ir::{ObKind, Prover, esc};
use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_mir::{
    AssertKind, BasicBlock, BlockData, Body, BodyKind, Callee, Const, Local, LocalDecl, LocalKind,
    MirOutput, Operand, Place, Rvalue, Statement, StatementKind, Terminator, TerminatorKind,
};
use gt_span::Span;
use gt_types::TypesOutput;
use std::collections::{BTreeMap, BTreeSet};

const LIMIT: usize = 200;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub obligations: usize,
    pub discharged: usize,
    pub lowered: usize,
    pub definitely_false: usize,
    pub rejected: usize,
    /// số `Assert` trong MIR của M5 / sau M7
    pub asserts_before: usize,
    pub asserts_after: usize,
    pub literal_errors: usize,
    /// số E1001 (`⚠` thiếu bằng chứng, con trỏ thô ngoài `⚠`)
    pub unsafe_errors: usize,
}

/// Chế độ xử lý check (ADR-028 §2): `All` GIỮ mọi check (không xoá check nào, kể cả đã chứng minh; chèn cả kiểm hợp đồng đã chứng minh mà
/// kiểm được lúc chạy) — MẶC ĐỊNH của `gritc` cho tới khi điều kiện đột biến "xoá thêm check" giải quyết; `Proven` xoá check đã chứng minh.
/// Nghĩa vụ không chứng minh được và không kiểm được lúc chạy vẫn ĐỎ ở MỌI chế độ (D25).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Checks {
    All,
    Proven,
}

#[derive(Debug)]
pub struct OblOutput {
    /// E060x, E0901/E0902, N06xx/N09xx, E9901
    pub report: Report,
    /// thân đã xoá `Assert` chứng minh được (cùng thứ tự `MirOutput.bodies`)
    pub bodies: Vec<Body>,
    pub per_body: Vec<BodyReport>,
    pub stats: Stats,
}

#[must_use]
pub fn count_asserts(b: &Body) -> usize {
    b.blocks
        .iter()
        .filter(|bb| {
            matches!(
                bb.terminator.as_ref().map(|t| &t.kind),
                Some(TerminatorKind::Assert { .. })
            )
        })
        .count()
}

fn mk(t: &TypesOutput, code: &str, node: gt_ast::NodeId, span: Span, msg: String) -> Diagnostic {
    // mã được liệt kê cố định ở đây; `Code::parse` chỉ thất bại với chuỗi sai định dạng
    let c = Code::parse(code).unwrap_or_else(|_| Code::parse("E9901").expect("E9901 hợp lệ"));
    Diagnostic::new(c, &t.file_name(node), span, msg)
}

fn definite_code(k: &ObKind) -> &'static str {
    match k {
        ObKind::Overflow | ObKind::NegMin | ObKind::DivOverflow | ObKind::LiteralRange => "E0601",
        ObKind::DivZero | ObKind::RemZero => "E0602",
        ObKind::Index | ObKind::SliceRange => "E0603",
        ObKind::CastLossy => "E0604",
        ObKind::Variant => "E0702",
        ObKind::LoopBound => "E0701",
        _ => "E0901",
    }
}

fn definite_msg(k: &ObKind) -> &'static str {
    match k {
        ObKind::Overflow => "phép toán tràn kiểu trên mọi đường tới được điểm này",
        ObKind::NegMin => "phép đảo dấu luôn gặp giá trị nhỏ nhất của kiểu có dấu",
        ObKind::DivOverflow => "phép chia/% luôn gặp `MIN / -1`",
        ObKind::DivZero => "chia cho 0 trên mọi đường tới được điểm này",
        ObKind::RemZero => "phần dư khi chia cho 0 trên mọi đường tới được điểm này",
        ObKind::Index => "chỉ số luôn ngoài biên",
        ObKind::SliceRange => "khoảng cắt lát luôn ngoài biên",
        ObKind::CastLossy => "phép ép kiểu luôn làm mất thông tin",
        ObKind::LiteralRange => "literal ngoài phạm vi kiểu",
        ObKind::Pre => "tiền điều kiện luôn sai tại điểm gọi",
        ObKind::Post => "hậu điều kiện luôn sai",
        ObKind::Invariant => "bất biến vòng lặp luôn sai khi tới đầu vòng",
        ObKind::Fuel => "nhiên liệu luôn cạn khi thoát vòng",
        ObKind::Variant => "biến thể `↓` luôn không giảm",
        ObKind::LoopBound => "biến thể tự suy luôn không giảm",
    }
}

fn note_msg(code: &str) -> &'static str {
    match code {
        "N0601" => "không chứng minh được không tràn: giữ kiểm tra lúc chạy",
        "N0602" => "không chứng minh được chỉ số/khoảng trong biên: giữ kiểm tra lúc chạy",
        "N0603" => {
            "không chứng minh được mẫu số khác 0 (và không `MIN / -1`): giữ kiểm tra lúc chạy"
        }
        "N0604" => "không chứng minh được phép ép kiểu không mất thông tin: giữ kiểm tra lúc chạy",
        "N0901" => "không chứng minh được tiền điều kiện: kiểm tra lúc chạy tại điểm gọi",
        "N0902" => "không chứng minh được hậu điều kiện: kiểm tra lúc chạy ở mọi điểm thoát",
        "N0903" => "lượng từ có biên được kiểm lúc chạy bằng vòng lặp",
        "N0904" => "không chứng minh được bất biến vòng lặp: kiểm tra lúc chạy ở đầu mỗi vòng",
        "N0905" => {
            "không chứng minh được nhiên liệu còn sau vòng lặp: kiểm tra lúc chạy (abort `fuel_exhausted` nếu cạn)"
        }
        _ => "kiểm tra giữ lại lúc chạy",
    }
}

/// Phân tích mọi thân. `m` phải sạch (không chẩn đoán); nếu không trả về rỗng (fail-closed).
pub fn analyze_program(
    t: &TypesOutput,
    m: &MirOutput,
    prover: &mut (dyn Prover + Send),
) -> OblOutput {
    analyze_program_with(t, m, prover, Checks::Proven)
}

/// Như `analyze_program` nhưng chọn chế độ check (`Checks::All` giữ mọi check).
#[must_use]
pub fn analyze_program_with(
    t: &TypesOutput,
    m: &MirOutput,
    prover: &mut (dyn Prover + Send),
    mode: Checks,
) -> OblOutput {
    let mut bag = Diagnostics::new(LIMIT);
    let mut stats = Stats::default();
    if !m.report.diagnostics.is_empty() {
        return OblOutput {
            report: bag.finish(),
            bodies: Vec::new(),
            per_body: Vec::new(),
            stats,
        };
    }
    // E1001: `⚠` thiếu bằng chứng và con trỏ thô ngoài `⚠` (D21, Q13) — độc lập với phân tích số
    for d in crate::unsafe_check::check_unsafe(t, m) {
        stats.unsafe_errors += 1;
        bag.push(d);
    }
    let info = build_prog_info(t, m);
    let mut bodies = Vec::with_capacity(m.bodies.len());
    let mut per_body = Vec::with_capacity(m.bodies.len());
    for b in &m.bodies {
        let rep = gt_stack::run_with_big_stack(|| analyze_body(b, &info, &mut *prover));
        let rep = match rep {
            Ok(r) => r,
            Err(_) => {
                bag.push(mk(
                    t,
                    "E9901",
                    b.node,
                    b.span,
                    format!("lỗi nội bộ: phân tích `{}` bị ngắt", b.key),
                ));
                bodies.push(b.clone());
                per_body.push(BodyReport {
                    key: b.key.clone(),
                    ice: Some("ngắt".into()),
                    ..BodyReport::default()
                });
                continue;
            }
        };
        stats.asserts_before += count_asserts(b);
        // literal ngoài phạm vi kiểu: KHÔNG phụ thuộc đường tới
        for bl in &b.bad_literals {
            stats.literal_errors += 1;
            let sign = if bl.negative { "-" } else { "" };
            bag.push(mk(
                t,
                "E0601",
                bl.node,
                bl.span,
                format!(
                    "literal `{sign}{}` ngoài phạm vi của `{}`",
                    bl.magnitude,
                    bl.ty.name()
                ),
            ));
        }
        for u in &b.uncheckable {
            stats.rejected += 1;
            bag.push(mk(
                t,
                "E0902",
                u.node,
                u.span,
                format!(
                    "nghĩa vụ không kiểm được lúc chạy mà chưa chứng minh được: {}",
                    u.why
                ),
            ));
        }
        if let Some(e) = &rep.ice {
            bag.push(mk(
                t,
                "E9901",
                b.node,
                b.span,
                format!("lỗi nội bộ trong `{}`: {e}", b.key),
            ));
        }
        if !rep.zone_dropped.is_empty() {
            bag.push(mk(
                t,
                "N0605",
                b.node,
                b.span,
                format!(
                    "quá {} biến cho miền quan hệ trong `{}`: bỏ theo dõi quan hệ của {}",
                    crate::analysis::ZONE_MAX_VARS,
                    b.key,
                    rep.zone_dropped.join(", ")
                ),
            ));
        }
        if rep.budget_exhausted {
            bag.push(mk(
                t,
                "N0605",
                b.node,
                b.span,
                format!(
                    "hết ngân sách bước phân tích `{}`: bỏ mọi quan hệ, chỉ còn khoảng",
                    b.key
                ),
            ));
        }
        let mut ob = b.clone();
        for j in &rep.judged {
            stats.obligations += 1;
            let span = Span::new(j.ob.span.0, j.ob.span.1.max(j.ob.span.0));
            match &j.status {
                Status::Discharged { .. } => stats.discharged += 1,
                Status::DefinitelyFalse => {
                    stats.definitely_false += 1;
                    let k = &j.ob.kind;
                    bag.push(mk(
                        t,
                        definite_code(k),
                        j.node,
                        span,
                        definite_msg(k).to_string(),
                    ));
                }
                Status::Lowered { note } => {
                    stats.lowered += 1;
                    bag.push(mk(t, note, j.node, span, note_msg(note).to_string()));
                }
                Status::Rejected { why } => {
                    stats.rejected += 1;
                    bag.push(mk(
                        t,
                        "E0902",
                        j.node,
                        span,
                        format!("nghĩa vụ không kiểm được lúc chạy mà chưa chứng minh được: {why}"),
                    ));
                }
            }
        }
        // chèn kiểm hợp đồng lúc chạy (`⊳` trước lời gọi, `⊲` trước `Return`)
        let mut tails: BTreeMap<u32, usize> = BTreeMap::new();
        for j in &rep.judged {
            if let Some(ins) = &j.insert {
                apply_insert(t, &mut ob, ins, &mut tails);
            }
        }
        if mode == Checks::All {
            // giữ mọi check: chèn thêm kiểm hợp đồng đã chứng minh mà kiểm được lúc chạy, KHÔNG xoá `Assert` nào
            for j in &rep.judged {
                if let Some(ins) = &j.proven_insert {
                    apply_insert(t, &mut ob, ins, &mut tails);
                }
            }
        } else {
            for blk in &rep.discharged_blocks {
                if let Some(term) = ob.blocks[*blk as usize].terminator.as_mut()
                    && let TerminatorKind::Assert { target, .. } = &term.kind
                {
                    term.kind = TerminatorKind::Goto(*target);
                }
            }
        }
        if let Err(errs) = gt_mir::validate::validate(&ob, &t.db) {
            bag.push(mk(
                t,
                "E9901",
                b.node,
                b.span,
                format!(
                    "lỗi nội bộ: MIR sau M7 của `{}` không hợp lệ: {}",
                    b.key,
                    errs.join("; ")
                ),
            ));
        }
        stats.asserts_after += count_asserts(&ob);
        bodies.push(ob);
        per_body.push(rep);
    }
    OblOutput {
        report: bag.finish(),
        bodies,
        per_body,
        stats,
    }
}

/// Báo cáo "số check còn lại" (ADR-025 §8, Phase 2 đọc được): JSON tất định, có version.
#[must_use]
pub fn remaining_checks_json(out: &OblOutput) -> String {
    let st = &out.stats;
    let mut s = format!(
        "{{\"schema\":{},\"totals\":{{\"obligations\":{},\"discharged\":{},\"remaining_runtime_checks\":{},\"rejected\":{},\"definitely_false\":{},\"literal_errors\":{},\"asserts_before\":{},\"asserts_after\":{}}},\"bodies\":[",
        crate::ir::SCHEMA_VERSION,
        st.obligations,
        st.discharged,
        st.lowered,
        st.rejected,
        st.definitely_false,
        st.literal_errors,
        st.asserts_before,
        st.asserts_after
    );
    let mut first = true;
    for r in &out.per_body {
        // thân hợp đồng của prelude không có nghĩa vụ nào: không đưa vào báo cáo
        if r.key.starts_with("$prelude") && r.judged.is_empty() {
            continue;
        }
        if !first {
            s.push(',');
        }
        first = false;
        s.push_str(&format!("{{\"key\":{},\"obligations\":[", esc(&r.key)));
        let mut f2 = true;
        for j in &r.judged {
            if !f2 {
                s.push(',');
            }
            f2 = false;
            let (status, extra) = match &j.status {
                Status::Discharged { prover, cert } => (
                    "discharged",
                    format!(",\"prover\":{},\"cert\":{}", esc(prover), esc(cert)),
                ),
                Status::DefinitelyFalse => ("definitely_false", String::new()),
                Status::Lowered { note } => ("runtime_check", format!(",\"note\":{}", esc(note))),
                Status::Rejected { why } => ("rejected", format!(",\"why\":{}", esc(why))),
            };
            s.push_str(&format!(
                "{{\"status\":{}{},\"ob\":{}}}",
                esc(status),
                extra,
                j.ob.to_json()
            ));
        }
        s.push_str("]}");
    }
    s.push_str("]}");
    s
}

// ====================================================================== thông tin hợp đồng toàn chương trình

fn escaped_fns(m: &MirOutput) -> BTreeSet<gt_ast::NodeId> {
    let mut out = BTreeSet::new();
    let mut visit = |o: &Operand| {
        if let Operand::Const(Const::FnItem { node, .. }) = o {
            out.insert(*node);
        }
    };
    for b in &m.bodies {
        for bb in &b.blocks {
            for st in &bb.statements {
                if let StatementKind::Assign(_, rv) = &st.kind {
                    match rv {
                        Rvalue::Use(o)
                        | Rvalue::Repeat(o, _)
                        | Rvalue::Cast(o, _)
                        | Rvalue::CastFits(o, _)
                        | Rvalue::UnaryOp(_, o) => visit(o),
                        Rvalue::BinaryOp(_, a, c) | Rvalue::CheckedBinaryOp(_, a, c) => {
                            visit(a);
                            visit(c);
                        }
                        Rvalue::Aggregate(_, v) => v.iter().for_each(&mut visit),
                        _ => {}
                    }
                }
            }
            match bb.terminator.as_ref().map(|t| &t.kind) {
                Some(TerminatorKind::Call { callee, args, .. }) => {
                    args.iter().for_each(&mut visit);
                    if let Callee::Value(o) = callee {
                        visit(o);
                    }
                }
                Some(TerminatorKind::SwitchInt { discr, .. }) => visit(discr),
                Some(TerminatorKind::Assert { cond, .. }) => visit(cond),
                _ => {}
            }
        }
    }
    out
}

/// Gom hợp đồng: trích mệnh đề từng thân `⊳`/`⊲`, chọn hàm có `⊳` được phép làm giả thiết (ADR-025 §6 + §12).
#[must_use]
pub fn build_prog_info(t: &TypesOutput, m: &MirOutput) -> ProgInfo {
    let mut info = ProgInfo {
        keys: m.keys.clone(),
        ..ProgInfo::default()
    };
    // khoá → NodeId của MỌI hàm (kể cả prelude, vốn chỉ có hợp đồng được hạ)
    let fn_by_key: BTreeMap<&str, gt_ast::NodeId> =
        m.keys.iter().map(|(n, k)| (k.as_str(), *n)).collect();
    // vị từ thuần một đối số trả `bool`, hiệu ứng `∅`, không nhận `&!`: mô hình hoá như nguyên tử không diễn giải
    for node in m.keys.keys() {
        if let Some(sig) = t.db.sig_by_node(*node) {
            let n_args = sig.params.len() + usize::from(sig.self_kind.is_some());
            let mut_arg = matches!(sig.self_kind, Some(gt_ast::SelfKind::RefMut))
                || sig
                    .params
                    .iter()
                    .any(|p| matches!(p, gt_types::Ty::Ref(true, _)));
            // kiểu của `self` (thành viên impl) không nằm trong `sig.params`: lấy từ impl chứa thành viên
            let imp =
                t.db.impls
                    .iter()
                    .find(|i| i.members.iter().any(|m| m.node == *node));
            let gens: Vec<&gt_types::GParam> = sig
                .generics
                .iter()
                .chain(imp.iter().flat_map(|i| i.generics.iter()))
                .collect();
            // điều kiện cứng mutability bên trong (Giang 2026-10-07): vị từ trên kiểu có thể chứa ô ghi được qua `&` không phải sự kiện ổn định
            let interior = sig
                .params
                .iter()
                .any(|p| t.db.may_have_interior_mut(p, &gens))
                || imp.is_some_and(|i| t.db.may_have_interior_mut(&i.self_ty, &gens));
            if n_args == 1
                && sig.ret == gt_types::Ty::Bool
                && sig.effects.is_empty()
                && !mut_arg
                && !interior
            {
                let idx = info.pure_preds.len() as u32;
                info.pure_preds.insert(*node, idx);
            }
        }
    }
    for b in &m.bodies {
        let scope_node = match b.key.rsplit_once(['⊳', '⊲']) {
            Some((owner, _)) => fn_by_key.get(owner).copied().unwrap_or(b.node),
            None => b.node,
        };
        let scope = t.scope_of(scope_node);
        info.local_copy.insert(
            b.key.clone(),
            b.locals
                .iter()
                .map(|d| t.db.is_copy_resolved(&d.ty, &scope))
                .collect(),
        );
    }
    let mut counter = 0u32;
    let base = ProgInfo {
        keys: info.keys.clone(),
        pure_preds: info.pure_preds.clone(),
        ..ProgInfo::default()
    };
    for b in &m.bodies {
        if !matches!(b.kind, BodyKind::Pre | BodyKind::Post) {
            continue;
        }
        let mark = if b.kind == BodyKind::Pre {
            '⊳'
        } else {
            '⊲'
        };
        let Some((owner_key, _)) = b.key.rsplit_once(mark) else {
            continue;
        };
        let Some(&fnode) = fn_by_key.get(owner_key) else {
            continue;
        };
        let unchecked = b.uncheckable.first().map(|u| u.why.clone());
        let prop = if unchecked.is_some() {
            None
        } else {
            extract_contract(b, &base, &mut counter)
        };
        let cp = ContractProp {
            node: b.node,
            key: b.key.clone(),
            prop,
            arg_count: b.arg_count,
            arg_copy: (1..=b.arg_count)
                .map(|i| info.local_copy.get(&b.key).is_some_and(|v| v[i]))
                .collect(),
            unchecked,
            has_loop: has_cycle(b),
        };
        if b.kind == BodyKind::Pre {
            info.pre.entry(fnode).or_default().push(cp);
        } else {
            info.post.entry(fnode).or_default().push(cp);
        }
    }
    info.contract_atoms = counter;
    // `⊳` chỉ làm giả thiết cho hàm CHỈ bị gọi trực tiếp: không thoát qua `FnItem`, không là thành viên của `⊐ Trait for`
    let escaped = escaped_fns(m);
    let trait_member: BTreeSet<gt_ast::NodeId> = t
        .bodies
        .iter()
        .filter(|br| {
            br.impl_node.is_some_and(|n| {
                t.db.impls
                    .iter()
                    .any(|i| i.node == n && i.trait_ref.is_some())
            })
        })
        .map(|br| br.node)
        .collect();
    for f in info.pre.keys() {
        if !escaped.contains(f) && !trait_member.contains(f) {
            info.assume_pre.insert(*f);
        }
    }
    info
}

// ====================================================================== chèn kiểm lúc chạy

fn identity_generics(t: &TypesOutput, node: gt_ast::NodeId) -> Vec<gt_types::Ty> {
    t.db.sig_by_node(node)
        .map(|s| {
            s.generics
                .iter()
                .map(|g| gt_types::Ty::Param(g.id))
                .collect()
        })
        .unwrap_or_default()
}

fn apply_insert(t: &TypesOutput, ob: &mut Body, ins: &Insert, tails: &mut BTreeMap<u32, usize>) {
    let (block, contract, generics, args, kind) = match ins {
        Insert::Pre {
            block,
            contract,
            generics,
            args,
        } => (
            *block,
            *contract,
            generics.clone(),
            args.clone(),
            AssertKind::Precondition,
        ),
        Insert::Post {
            block,
            contract,
            params,
        } => {
            let mut a: Vec<Operand> = (1..*params)
                .map(|i| Operand::Copy(Place::local(Local(i as u32))))
                .collect();
            a.push(Operand::Copy(Place::local(Local(0))));
            (
                *block,
                *contract,
                identity_generics(t, ob.node),
                a,
                AssertKind::Postcondition,
            )
        }
    };
    let cur = *tails.get(&block).unwrap_or(&(block as usize));
    // invariant: `cur` là khối đang giữ terminator gốc (`tails` theo dõi khối đuôi sau mỗi lần tách). Mất terminator ⇒ KHÔNG được bỏ qua lặng lẽ việc chèn kiểm tra
    // lúc chạy (fail-open): panic ⇒ ICE E9901 ở tầng pass (R3, FAIL_OPEN_AUDIT).
    let Some(orig) = ob.blocks[cur].terminator.take() else {
        panic!(
            "apply_insert: khối {cur} của `{}` không còn terminator — không chèn được kiểm tra {kind:?}",
            ob.key
        );
    };
    let src = orig.source;
    let span = src.span;
    // bản sao các đối số (Move của kiểu Copy ⇒ Copy)
    let args: Vec<Operand> = args
        .into_iter()
        .map(|o| match o {
            Operand::Move(p) => Operand::Copy(p),
            o => o,
        })
        .collect();
    let tmp = Local(ob.locals.len() as u32);
    ob.locals.push(LocalDecl {
        ty: gt_types::Ty::Bool,
        regions: Vec::new(),
        kind: LocalKind::Temp,
        mutable: false,
        span,
    });
    let mid = BasicBlock(ob.blocks.len() as u32);
    let tail = BasicBlock(ob.blocks.len() as u32 + 1);
    ob.blocks[cur].statements.push(Statement {
        kind: StatementKind::StorageLive(tmp),
        source: src,
    });
    ob.blocks[cur].terminator = Some(Terminator {
        kind: TerminatorKind::Call {
            callee: Callee::Fn {
                node: contract,
                generics,
            },
            arg_spans: vec![span; args.len()],
            args,
            dest: Place::local(tmp),
            target: Some(mid),
        },
        source: src,
    });
    ob.blocks.push(BlockData {
        statements: Vec::new(),
        terminator: Some(Terminator {
            kind: TerminatorKind::Assert {
                cond: Operand::Copy(Place::local(tmp)),
                expected: true,
                kind,
                target: tail,
            },
            source: src,
        }),
    });
    ob.blocks.push(BlockData {
        statements: vec![Statement {
            kind: StatementKind::StorageDead(tmp),
            source: src,
        }],
        terminator: Some(orig),
    });
    tails.insert(block, tail.index());
}

/// Đồ thị khối có chu trình (thân hợp đồng có lượng từ)?
fn has_cycle(b: &Body) -> bool {
    let n = b.blocks.len();
    let mut state = vec![0u8; n]; // 0 chưa thăm, 1 đang thăm, 2 xong
    let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
    state[0] = 1;
    while let Some((bb, i)) = stack.pop() {
        let succ = b.blocks[bb]
            .terminator
            .as_ref()
            .map_or_else(Vec::new, |t| t.kind.successors());
        if i < succ.len() {
            stack.push((bb, i + 1));
            let s = succ[i].index();
            match state[s] {
                1 => return true,
                0 => {
                    state[s] = 1;
                    stack.push((s, 0));
                }
                _ => {}
            }
        } else {
            state[bb] = 2;
        }
    }
    false
}

#[cfg(test)]
#[path = "../../gt-types/tests/common/mod.rs"]
mod tc;

#[cfg(test)]
mod insert_tests {
    use super::{BTreeMap, Insert, apply_insert, tc};

    /// FAIL_OPEN_AUDIT R3 (driver.rs `apply_insert`): khối không còn terminator ⇒ từng `return` LẶNG LẼ ⇒ kiểm tra lúc chạy `⊳`/`⊲` KHÔNG được chèn (fail-open).
    /// Invariant bị phá phải là lỗi NỘI BỘ (panic ⇒ ICE E9901 ở tầng pass), không bao giờ bỏ qua việc chèn.
    #[test]
    #[should_panic(expected = "terminator")]
    fn apply_insert_on_a_block_without_terminator_is_an_internal_error_not_a_silent_skip() {
        let t = tc::run(&[("main.gt", "ƒ f() → u32 ! ∅ { 1 }")]);
        let m = gt_mir::lower_program(&t);
        let mut ob = m.bodies[0].clone();
        ob.blocks[0].terminator = None;
        let ins = Insert::Pre {
            block: 0,
            contract: ob.node,
            generics: Vec::new(),
            args: Vec::new(),
        };
        apply_insert(&t, &mut ob, &ins, &mut BTreeMap::new());
    }
}
