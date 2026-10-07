//! Điều phối mức chương trình (ADR-025 §5, §8): chạy phân tích từng thân, áp chính sách một nghĩa vụ, dựng MIR đã tối ưu
//! (xoá `Assert` đã chứng minh), phát chẩn đoán, và báo cáo "số check còn lại" dạng JSON.
//!
//! Chính sách (D24/D25): `Discharged` → xoá `Assert`; `DefinitelyFalse` → ĐỎ E060x; `Lowered` → giữ `Assert` + GHI CHÚ N06xx;
//! `Rejected` (không kiểm được lúc chạy mà chưa giải) → ĐỎ E0902.
use crate::analysis::{BodyReport, Status, analyze_body};
use crate::ir::{ObKind, Prover, esc};
use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_mir::{Body, MirOutput, TerminatorKind};
use gt_span::Span;
use gt_types::TypesOutput;

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
        _ => "kiểm tra giữ lại lúc chạy",
    }
}

/// Phân tích mọi thân. `m` phải sạch (không chẩn đoán); nếu không trả về rỗng (fail-closed).
pub fn analyze_program(
    t: &TypesOutput,
    m: &MirOutput,
    prover: &mut (dyn Prover + Send),
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
    let mut bodies = Vec::with_capacity(m.bodies.len());
    let mut per_body = Vec::with_capacity(m.bodies.len());
    for b in &m.bodies {
        let rep = gt_stack::run_with_big_stack(|| analyze_body(b, &m.keys, &mut *prover));
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
        for blk in &rep.discharged_blocks {
            if let Some(term) = ob.blocks[*blk as usize].terminator.as_mut()
                && let TerminatorKind::Assert { target, .. } = &term.kind
            {
                term.kind = TerminatorKind::Goto(*target);
            }
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
