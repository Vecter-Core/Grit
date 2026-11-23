//! Chuỗi pass của `gritc check` (ADR-028 §1): M4 kiểu → M5 MIR → M6 mượn/sở hữu → M7 nghĩa vụ → M8 dừng/hiệu ứng/đa luồng → M9 secret.
//! Mỗi pass chạy trong luồng stack lớn bắt panic (`gt_stack`): panic ⇒ ICE `E9901`, KHÔNG BAO GIỜ coi là "sạch".
use crate::render::{ChecksReport, checks_report};
use crate::sha256::sha256_hex;
use crate::verdict::{Verdict, decide};
use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_mir::MirOutput;
use gt_oblig::driver::{Checks, OblOutput, analyze_program_with, build_prog_info};
use gt_oblig::prover::ZoneProver;
use gt_resolve::{Prelude, SourceInput};
use gt_secrets::Allowed;
use gt_span::Span;
use gt_types::TypesOutput;
use std::sync::OnceLock;

/// Prelude của gritc (giả lập `lib/core`; M12 thay bằng thư viện lõi thật). Nguồn: `lib/core/prelude.gt`.
const PRELUDE: &str = include_str!("../../../lib/core/prelude.gt");

#[must_use]
pub fn prelude_text() -> String {
    PRELUDE.replace("\r\n", "\n")
}

/// Pass BẮT BUỘC (ADR-028 §5; GRIT_PLAN §7 điểm 11): thiếu một pass trong danh sách mà báo cáo "sạch" ⇒ ICE, không output, không chạy.
/// `m5.return_paths` (E0307) và `m7.const_bounds` (E0601) là hai điều kiện cứng.
pub const REQUIRED_PASSES: [&str; 11] = [
    "selftest",
    "m4.types",
    "m5.mir",
    "m5.return_paths",
    "m6.borrowck",
    "m7.obligations",
    "m7.const_bounds",
    "m8.termination",
    "m8.effects",
    "m8.conc",
    "m9.secrets",
];

const AFTER_M4: [&str; 9] = [
    "m5.mir",
    "m5.return_paths",
    "m6.borrowck",
    "m7.obligations",
    "m7.const_bounds",
    "m8.termination",
    "m8.effects",
    "m8.conc",
    "m9.secrets",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassStatus {
    Done,
    /// không chạy (vd tầng trước đã ĐỎ)
    Skipped,
    /// panic / lỗi nội bộ
    Failed,
}

impl PassStatus {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            PassStatus::Done => "done",
            PassStatus::Skipped => "skipped",
            PassStatus::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassRecord {
    pub name: &'static str,
    pub status: PassStatus,
}

/// Chèn lỗi để KIỂM cổng phán quyết (chỉ làm gritc TỪ CHỐI thêm: panic ⇒ ICE, bỏ pass ⇒ thiếu pass bắt buộc ⇒ ICE). Không có chiều ngược lại.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Panic(String),
    Skip(String),
}

/// Chế độ check mặc định của gritc. Chỉ đổi sang `Proven` khi điều kiện đột biến "xoá thêm check" giải quyết VÀ Giang duyệt (ADR-028 §2).
pub const DEFAULT_CHECKS: Checks = Checks::All;

#[derive(Clone, Debug)]
pub struct Options {
    pub checks: Checks,
    pub deny_yellow: bool,
    pub fault: Option<Fault>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            checks: DEFAULT_CHECKS,
            deny_yellow: false,
            fault: None,
        }
    }
}

/// Sản phẩm các pass, chỉ lấy được qua [`CheckResult::open_gate`] / [`CheckResult::take_gated`] (XANH/VÀNG).
pub struct Artifacts {
    pub types: TypesOutput,
    pub mir: MirOutput,
    pub obl: OblOutput,
}

pub struct CheckResult {
    pub report: Report,
    pub passes: Vec<PassRecord>,
    pub verdict: Verdict,
    pub sources: Vec<(String, String)>,
    pub checks: Checks,
    /// báo cáo "check còn lại" (chỉ là DỮ LIỆU báo cáo, không phải sản phẩm để ghi/chạy)
    pub checks_report: Option<ChecksReport>,
    /// các `#allow` đã áp dụng (dấu vết kiểm toán)
    pub allowed: Vec<Allowed>,
    artifacts: Option<Artifacts>,
    /// backend dùng cho đầu ra này (`interpreter` | `llvm`); cli đặt trước khi in
    pub backend: &'static str,
}

impl CheckResult {
    /// IR Guard từ chối IR đã sinh (E1101): ghi chẩn đoán ĐỎ và đóng cổng (không còn sản phẩm nào lấy được).
    pub fn reject_ir(&mut self, violations: &[String]) {
        for v in violations {
            self.report.diagnostics.push(gritc_diag(
                "E1101",
                format!("IR Guard từ chối IR do backend sinh: {v}"),
            ));
        }
        self.verdict = Verdict::Red;
        self.artifacts = None;
    }

    /// CỔNG: chỉ trả sản phẩm khi phán quyết cho phép (XANH/VÀNG). ĐỎ/ICE ⇒ `None`: không có gì để ghi hay chạy.
    #[must_use]
    pub fn open_gate(&self) -> Option<&Artifacts> {
        if self.verdict.allows_output() {
            self.artifacts.as_ref()
        } else {
            None
        }
    }

    /// Lấy sở hữu sản phẩm (cho `run` cần di chuyển MIR). Cũng chỉ qua cổng; ĐỎ/ICE ⇒ `None` và sản phẩm KHÔNG bị lấy.
    #[must_use]
    pub fn take_gated(&mut self) -> Option<Artifacts> {
        if self.verdict.allows_output() {
            self.artifacts.take()
        } else {
            None
        }
    }
}

fn gritc_diag(code: &str, msg: String) -> Diagnostic {
    let c = Code::parse(code).unwrap_or_else(|_| Code::parse("E9901").expect("E9901 hợp lệ"));
    Diagnostic::new(c, "<gritc>", Span::new(0, 0), msg)
}

fn merge(bag: &mut Diagnostics, r: &Report) {
    for d in &r.diagnostics {
        bag.push(d.clone());
    }
}

struct Run<'a> {
    opts: &'a Options,
    passes: Vec<PassRecord>,
    bag: Diagnostics,
}

impl Run<'_> {
    fn record(&mut self, name: &'static str, status: PassStatus) {
        self.passes.push(PassRecord { name, status });
    }

    /// Chạy một pass trong luồng stack lớn; panic/lỗi tạo luồng ⇒ `Failed` + ICE.
    fn stage<T: Send>(&mut self, name: &'static str, f: impl FnOnce() -> T + Send) -> Option<T> {
        let fault = self.opts.fault.clone();
        if fault == Some(Fault::Skip(name.to_string())) {
            self.record(name, PassStatus::Skipped);
            return None;
        }
        let inject = fault == Some(Fault::Panic(name.to_string()));
        let r = gt_stack::run_with_big_stack(move || {
            assert!(!inject, "lỗi chèn thử trong pass {name}");
            f()
        });
        match r {
            Ok(v) => {
                self.record(name, PassStatus::Done);
                Some(v)
            }
            Err(e) => {
                self.record(name, PassStatus::Failed);
                self.bag.push(gritc_diag(
                    "E9901",
                    format!("lỗi nội bộ trong pass `{name}`: {e}"),
                ));
                None
            }
        }
    }

    /// Nhãn phụ của một pass lớn (E0307 do `lower_program`, E0601 do `analyze_program`): ghi `Done` nếu pass cha xong; vẫn chịu chèn lỗi
    /// (Panic ⇒ ICE, Skip ⇒ thiếu pass bắt buộc) để kiểm cổng cho cả hai điều kiện cứng.
    fn alias(&mut self, name: &'static str) {
        match self.opts.fault.clone() {
            Some(Fault::Skip(n)) if n == name => self.record(name, PassStatus::Skipped),
            Some(Fault::Panic(n)) if n == name => {
                self.record(name, PassStatus::Failed);
                self.bag.push(gritc_diag(
                    "E9901",
                    format!("lỗi nội bộ trong pass `{name}`: lỗi chèn thử"),
                ));
            }
            _ => self.record(name, PassStatus::Done),
        }
    }

    fn skip(&mut self, names: &[&'static str]) {
        for n in names {
            self.record(n, PassStatus::Skipped);
        }
    }
}

/// Hai chương trình CANARY: các điều kiện cứng phải còn hoạt động thật (E0601 literal ngoài phạm vi, E0307 thiếu đường trả về). Nếu
/// gritc bị cấu hình/biên dịch sai làm hai kiểm này mất tác dụng, `selftest` hỏng ⇒ ICE ⇒ không output, không chạy.
const CANARY_E0601: &str = "ƒ f() → u8 ! ∅ { x: u8 ≔ 300; x }";
const CANARY_E0307: &str = "ƒ f() → u32 ! ∅ { }";

fn selftest() -> Result<(), String> {
    static CACHE: OnceLock<Result<(), String>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let opts = Options::default();
            for (src, want) in [(CANARY_E0601, "E0601"), (CANARY_E0307, "E0307")] {
                let inputs = [SourceInput {
                    name: "canary.gt".to_string(),
                    text: src.to_string(),
                }];
                match run_passes(&inputs, &opts, false) {
                    Ok(res)
                        if res
                            .report
                            .diagnostics
                            .iter()
                            .any(|d| d.code.as_str() == want) => {}
                    Ok(res) => {
                        return Err(format!(
                            "canary {want} không còn bị bắt (điều kiện cứng hỏng); báo cáo: {:?}",
                            res.report
                                .diagnostics
                                .iter()
                                .map(|d| d.code.as_str().to_string())
                                .collect::<Vec<_>>()
                        ));
                    }
                    Err(e) => return Err(format!("canary {want}: {e}")),
                }
            }
            Ok(())
        })
        .clone()
}

/// Chạy toàn bộ chuỗi pass và phán quyết. `Err` = đầu vào không hợp lệ (lỗi dùng, mã thoát 3), KHÔNG phải phán quyết về chương trình.
pub fn check(inputs: &[SourceInput], opts: &Options) -> Result<CheckResult, String> {
    run_passes(inputs, opts, true)
}

fn run_passes(
    inputs: &[SourceInput],
    opts: &Options,
    with_selftest: bool,
) -> Result<CheckResult, String> {
    let mut run = Run {
        opts,
        passes: Vec::new(),
        bag: Diagnostics::new(5000),
    };
    let sources: Vec<(String, String)> = inputs
        .iter()
        .map(|i| (i.name.clone(), i.text.clone()))
        .collect();
    if opts.checks == Checks::Proven {
        // chế độ THỬ NGHIỆM xoá check đã chứng minh: nhắc MỖI lần chạy (lệnh Giang 2026-10-14, mục 5)
        run.bag.push(gritc_diag(
            "N0606",
            "`--checks=proven` là chế độ THỬ NGHIỆM: xoá các check đã chứng minh (mặc định an toàn là `--checks=all`; còn 69 đột biến nguy hiểm của bộ giải chưa giải quyết — KNOWN_LIMITATIONS)".to_string(),
        ));
    }
    if with_selftest {
        if let Some(Err(why)) = run.stage("selftest", selftest) {
            // pass chạy xong nhưng kết luận HỎNG: đánh dấu Failed + ICE
            if let Some(p) = run.passes.iter_mut().find(|p| p.name == "selftest") {
                p.status = PassStatus::Failed;
            }
            run.bag
                .push(gritc_diag("E9901", format!("selftest thất bại: {why}")));
        }
    } else {
        run.record("selftest", PassStatus::Done);
    }
    let prelude = match Prelude::from_source(&prelude_text()) {
        Ok(p) => p,
        Err(e) => {
            run.bag
                .push(gritc_diag("E9901", format!("prelude hỏng: {e:?}")));
            run.record("m4.types", PassStatus::Failed);
            run.skip(&AFTER_M4);
            return Ok(finish(run, sources, None));
        }
    };
    // ---- M4
    let t = match run.stage("m4.types", || gt_types::check_program(inputs, &prelude)) {
        Some(Ok(t)) => t,
        Some(Err(e)) => return Err(format!("đầu vào không hợp lệ: {e:?}")),
        None => {
            run.skip(&AFTER_M4);
            return Ok(finish(run, sources, None));
        }
    };
    merge(&mut run.bag, &t.report);
    if t.report.errors() > 0 {
        run.skip(&AFTER_M4);
        return Ok(finish(run, sources, None));
    }
    // ---- M5
    // `GRITC_FAULT=mir:pat-mode`: hạ MIR suy chế độ khớp qua tham chiếu sai ⇒ phải ra ICE E9901 (ADR-040)
    gt_mir::set_pat_mode_fault_for_test(
        std::env::var("GRITC_FAULT").as_deref() == Ok("mir:pat-mode"),
    );
    // `GRITC_FAULT=mir:pat-field`: biến bind trỏ nhầm trường (chỉ để kiểm bộ so sánh codegen/oracle tự bắt)
    gt_mir::set_pat_field_fault_for_test(
        std::env::var("GRITC_FAULT").as_deref() == Ok("mir:pat-field"),
    );
    // `GRITC_FAULT=mir:pat-field-swap`: hoán đổi hai bind cùng kiểu (lệch tinh vi mà M6 vẫn chấp nhận)
    gt_mir::set_pat_swap_fault_for_test(
        std::env::var("GRITC_FAULT").as_deref() == Ok("mir:pat-field-swap"),
    );
    // hạ MIR rồi elaboration drop (ADR-C18: cờ drop, drop mở) TRƯỚC M6 để M6 thấy đúng drop nào còn là một lần dùng
    let Some((m, drops)) = run.stage("m5.mir", || {
        let mut m = gt_mir::lower_program(&t);
        let drops = if m.report.errors() == 0 {
            gt_dataflow::elaborate_program(&t, &mut m)
        } else {
            Diagnostics::new(1).finish()
        };
        (m, drops)
    }) else {
        run.skip(&AFTER_M4[1..]);
        return Ok(finish(run, sources, None));
    };
    // E0307 do chính `lower_program` báo; canary kiểm nó còn hoạt động
    run.alias("m5.return_paths");
    merge(&mut run.bag, &m.report);
    merge(&mut run.bag, &drops);
    if drops.errors() > 0 {
        run.skip(&AFTER_M4[2..]);
        return Ok(finish(run, sources, None));
    }
    if m.report.errors() > 0 {
        run.skip(&AFTER_M4[2..]);
        return Ok(finish(run, sources, None));
    }
    // ---- M6
    if let Some(r) = run.stage("m6.borrowck", || gt_borrowck::check_program(&t, &m)) {
        merge(&mut run.bag, &r);
    }
    // ---- M7
    let checks = opts.checks;
    let obl = run.stage("m7.obligations", || {
        analyze_program_with(&t, &m, &mut ZoneProver::new(24), checks)
    });
    if let Some(o) = &obl {
        // E0601 (literal ngoài phạm vi) do chính `analyze_program` báo
        run.alias("m7.const_bounds");
        merge(&mut run.bag, &o.report);
    } else {
        run.skip(&["m7.const_bounds"]);
    }
    // ---- M8
    if let Some(r) = run.stage("m8.termination", || {
        let info = build_prog_info(&t, &m);
        gt_term::check_termination(&t, &m, &info, &mut ZoneProver::new(24)).report
    }) {
        merge(&mut run.bag, &r);
    }
    if let Some(o) = run.stage("m8.effects", || gt_effects::check_effects(&t, &m)) {
        merge(&mut run.bag, &o.report);
    }
    if let Some(r) = run.stage("m8.conc", || {
        let mut bag = Diagnostics::new(500);
        gt_conc::check_orderings(&t, &m, &mut bag);
        bag.finish()
    }) {
        merge(&mut run.bag, &r);
    }
    // ---- M9
    let allowed = match run.stage("m9.secrets", || gt_secrets::check(&t, &m, &sources)) {
        Some(s) => {
            merge(&mut run.bag, &s.report);
            s.allowed
        }
        None => Vec::new(),
    };
    let cr = obl.as_ref().map(|o| checks_report(o, checks, &m.keys));
    let arts = obl.map(|o| Artifacts {
        types: t,
        mir: m,
        obl: o,
    });
    let mut res = finish(run, sources, arts);
    res.checks_report = cr;
    res.allowed = allowed;
    Ok(res)
}

fn finish(
    run: Run<'_>,
    sources: Vec<(String, String)>,
    artifacts: Option<Artifacts>,
) -> CheckResult {
    let report = run.bag.finish();
    let verdict = decide(&report, &run.passes, run.opts.deny_yellow);
    CheckResult {
        report,
        passes: run.passes,
        verdict,
        sources,
        checks: run.opts.checks,
        checks_report: None,
        allowed: Vec::new(),
        artifacts,
        backend: crate::render::BACKEND,
    }
}

/// Văn bản MIR sau M7 + BẢNG INSTANCE (ADR-032 phụ lục A): attestation băm cả hai, nên đổi tập instance ⇒ đổi attestation.
#[must_use]
pub fn mir_dump(art: &Artifacts) -> String {
    let mut names: Vec<String> = art
        .types
        .instances
        .iter()
        .map(|i| match art.types.fn_keys.get(&i.node) {
            Some(key) => gt_types::instance_name(key, &i.args, &art.types.resolve.defs).0,
            // instance thiếu khoá KHÔNG được rơi khỏi băm (attestation phủ ít hơn): để lại dấu riêng làm băm đổi (FAIL_OPEN_AUDIT D1)
            None => format!("?missing:{:?}", i.node),
        })
        .collect();
    names.sort();
    format!(
        "{:#?}\ninstances {}\n",
        art.obl.bodies,
        sha256_hex(names.join("\n").as_bytes())
    )
}

/// Bản ghi attestation (ADR-028 §5): băm nguồn + phiên bản gritc + chế độ check + danh sách pass + MIR sau M7.
#[must_use]
pub fn attestation(res: &CheckResult, mir_dump: &str) -> String {
    let mut s = String::from("GTMIR1\n");
    for (n, t) in &res.sources {
        s.push_str(&format!("src {n} {}\n", sha256_hex(t.as_bytes())));
    }
    s.push_str(&format!("gritc {}\n", env!("CARGO_PKG_VERSION")));
    s.push_str(&format!(
        "checks {}\n",
        if res.checks == Checks::All {
            "all"
        } else {
            "proven"
        }
    ));
    for p in &res.passes {
        s.push_str(&format!("pass {} {}\n", p.name, p.status.name()));
    }
    s.push_str(&format!("mir {}\n", sha256_hex(mir_dump.as_bytes())));
    sha256_hex(s.as_bytes())
}
