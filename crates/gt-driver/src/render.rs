//! Đầu ra của gritc: văn bản cho người, JSON có version (schema v1, chỉ THÊM trường — `docs/spec/diag_schema.json`) và báo cáo "check còn lại"
//! (ADR-028 §6). Mọi đầu ra ghi rõ `backend` (M9: `"interpreter"`; M10 thêm `"llvm"`).
use crate::pipeline::{CheckResult, PassStatus};
use crate::runner::RunReport;
use crate::verdict::{RunEnd, run_exit_code};
use gt_diag::{RenderOptions, json_string, render_human_files};
use gt_oblig::analysis::Status;
use gt_oblig::driver::{Checks, OblOutput};
use gt_span::SourceFile;
use std::fmt::Write;

/// Backend thực thi của lần chạy này (M9). KHÔNG phải bằng chứng cho ngữ nghĩa codegen (ADR-028 §8).
pub const BACKEND: &str = "interpreter";

/// Một check còn lại / đã xoá.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckItem {
    pub id: String,
    pub func: String,
    pub kind: &'static str,
    pub span: (u32, u32),
    /// `kept_not_proven` | `kept_mode_all` | `inserted_mode_all` | `deleted_proven`
    pub status: &'static str,
    pub solver: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ChecksReport {
    pub mode: &'static str,
    pub kept: Vec<CheckItem>,
    pub deleted: Vec<CheckItem>,
    /// ở `all`: những check `proven` sẽ xoá (tập con của `kept` có `status = kept_mode_all`)
    pub would_delete: Vec<CheckItem>,
}

/// Dựng báo cáo check từ kết quả M7 (tất định: theo thân rồi theo thứ tự nghĩa vụ).
#[must_use]
pub fn checks_report(obl: &OblOutput, mode: Checks) -> ChecksReport {
    let mut r = ChecksReport {
        mode: if mode == Checks::All { "all" } else { "proven" },
        ..ChecksReport::default()
    };
    for rep in &obl.per_body {
        for j in &rep.judged {
            let item = |status: &'static str, solver: Option<&'static str>| CheckItem {
                id: j.ob.id.clone(),
                func: j.ob.body.clone(),
                kind: j.ob.kind.name(),
                span: j.ob.span,
                status,
                solver,
            };
            match &j.status {
                Status::Discharged { prover, .. } => {
                    if rep.discharged_blocks.contains(&j.ob.block) {
                        if mode == Checks::Proven {
                            r.deleted.push(item("deleted_proven", Some(prover)));
                        } else {
                            r.kept.push(item("kept_mode_all", Some(prover)));
                            r.would_delete.push(item("kept_mode_all", Some(prover)));
                        }
                    } else if mode == Checks::All && j.proven_insert.is_some() {
                        r.kept.push(item("inserted_mode_all", Some(prover)));
                    }
                }
                Status::Lowered { .. } => r.kept.push(item("kept_not_proven", None)),
                Status::DefinitelyFalse | Status::Rejected { .. } => {}
            }
        }
    }
    r
}

fn items_json(items: &[CheckItem]) -> String {
    let mut s = String::from("[");
    for (i, c) in items.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let solver = c.solver.map_or_else(|| "null".to_string(), json_string);
        let _ = write!(
            s,
            "{{\"id\":{},\"fn\":{},\"kind\":{},\"span\":[{},{}],\"status\":{},\"solver\":{}}}",
            json_string(&c.id),
            json_string(&c.func),
            json_string(c.kind),
            c.span.0,
            c.span.1,
            json_string(c.status),
            solver
        );
    }
    s.push(']');
    s
}

fn source_files(res: &CheckResult) -> Vec<SourceFile> {
    res.sources
        .iter()
        .filter_map(|(n, t)| SourceFile::new(n.clone(), t.clone()).ok())
        .collect()
}

/// Giai đoạn của đầu ra (quyết định ý nghĩa mã thoát — xem `verdict.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Check,
    Run,
}

impl Phase {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Phase::Check => "check",
            Phase::Run => "run",
        }
    }
}

/// JSON đầy đủ: báo cáo chẩn đoán schema v1 + trường mở rộng (backend, phase, verdict, passes, checks, allowed, attestation, run).
#[must_use]
pub fn json_report(
    res: &CheckResult,
    deny_yellow: bool,
    phase: Phase,
    run: Option<&RunReport>,
    attestation: Option<&str>,
) -> String {
    let files = source_files(res);
    let refs: Vec<&SourceFile> = files.iter().collect();
    let mut extra = String::new();
    let _ = write!(
        extra,
        "\"backend\":{},\"phase\":{},\"verdict\":{},\"passes\":[",
        json_string(res.backend),
        json_string(phase.name()),
        json_string(res.verdict.name())
    );
    for (i, p) in res.passes.iter().enumerate() {
        if i > 0 {
            extra.push(',');
        }
        let _ = write!(
            extra,
            "{{\"name\":{},\"status\":{}}}",
            json_string(p.name),
            json_string(p.status.name())
        );
    }
    extra.push(']');
    let cr = res.checks_report.clone().unwrap_or(ChecksReport {
        mode: if res.checks == Checks::All {
            "all"
        } else {
            "proven"
        },
        ..ChecksReport::default()
    });
    let _ = write!(
        extra,
        ",\"checks\":{{\"mode\":{},\"kept\":{},\"deleted\":{},\"would_delete\":{}}}",
        json_string(cr.mode),
        items_json(&cr.kept),
        items_json(&cr.deleted),
        items_json(&cr.would_delete)
    );
    extra.push_str(",\"allowed\":[");
    for (i, x) in res.allowed.iter().enumerate() {
        if i > 0 {
            extra.push(',');
        }
        let _ = write!(
            extra,
            "{{\"code\":{},\"reason\":{},\"file\":{},\"span\":[{},{}]}}",
            json_string(&x.code),
            json_string(&x.reason),
            json_string(&x.file),
            x.span.start,
            x.span.end
        );
    }
    extra.push(']');
    if let Some(a) = attestation {
        let _ = write!(extra, ",\"attestation\":{}", json_string(a));
    }
    if let Some(r) = run {
        let (aborted, kind) = match &r.end {
            RunEnd::Aborted(k) => (true, k.clone()),
            _ => (false, String::new()),
        };
        let value = r
            .value
            .as_ref()
            .map_or_else(|| "null".to_string(), |v| json_string(v));
        let _ = write!(
            extra,
            ",\"run\":{{\"exit_status\":{},\"aborted\":{},\"abort_kind\":{},\"value\":{}}}",
            run_exit_code(&r.end),
            aborted,
            json_string(&kind),
            value
        );
    }
    res.report.to_json_with(&refs, deny_yellow, &extra)
}

/// Văn bản cho người: chẩn đoán + dòng tóm tắt (luôn ghi backend).
#[must_use]
pub fn human_report(res: &CheckResult, color: bool) -> String {
    let files = source_files(res);
    let refs: Vec<&SourceFile> = files.iter().collect();
    let mut out = String::new();
    for d in &res.report.diagnostics {
        out.push_str(&render_human_files(d, &refs, &RenderOptions { color }));
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    let failed: Vec<&str> = res
        .passes
        .iter()
        .filter(|p| p.status == PassStatus::Failed)
        .map(|p| p.name)
        .collect();
    let _ = write!(
        out,
        "gritc: phán quyết {}; {} lỗi, {} cảnh báo, {} ghi chú; backend: {}",
        res.verdict.name(),
        res.report.errors(),
        res.report.warnings(),
        res.report.notes(),
        res.backend
    );
    if !failed.is_empty() {
        let _ = write!(out, "; pass hỏng: {}", failed.join(", "));
    }
    out.push('\n');
    out
}
