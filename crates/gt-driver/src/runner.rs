//! Giai đoạn SAU cổng: ghi gói `*.gtmir` (build) và chạy bằng thông dịch MIR tuần tự (run). Chỉ nhận [`Artifacts`] — thứ chỉ lấy được qua
//! [`crate::pipeline::CheckResult::take_gated`]/`open_gate` (XANH/VÀNG) — nên ĐỎ/ICE không thể tới đây (ADR-028 §3).
use crate::pipeline::{Artifacts, CheckResult, attestation};
use crate::verdict::RunEnd;
use gt_mir::Body;
use gt_mir::interp::{Interp, Outcome};

/// Nhiên liệu tối đa của bộ chạy tham chiếu (bước MIR). Hết ⇒ [`RunEnd::Limit`] (mã thoát 71), không phải lỗi của chương trình.
pub const FUEL: u64 = 50_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunReport {
    pub end: RunEnd,
    /// giá trị `main` trả về (dạng Debug) khi chạy xong
    pub value: Option<String>,
}

/// Tìm hàm `main`: thân hạng nhất `<tệp>::main` không tham số. Lỗi (thông báo) nếu không có hoặc có nhiều.
pub fn find_main(bodies: &[Body]) -> Result<&Body, String> {
    let cands: Vec<&Body> = bodies
        .iter()
        .filter(|b| {
            b.key.ends_with("::main") && b.key.matches("::").count() == 1 && b.arg_count == 0
        })
        .collect();
    match cands.as_slice() {
        [one] => Ok(one),
        [] => Err("không có hàm `main` không tham số ở cấp tệp".to_string()),
        many => Err(format!(
            "có {} hàm `main` ({}): đặt đúng một",
            many.len(),
            many.iter()
                .map(|b| b.key.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// Chạy `main` bằng thông dịch tham chiếu trên MIR SAU M7 (đã áp chế độ check).
#[must_use]
pub fn run_main(art: Artifacts) -> RunReport {
    run_main_with(art, FUEL)
}

/// Như [`run_main`] nhưng chọn nhiên liệu (cho test giới hạn bộ chạy).
#[must_use]
pub fn run_main_with(art: Artifacts, fuel: u64) -> RunReport {
    let Artifacts { mir, obl, .. } = art;
    let mut mir = mir;
    mir.bodies = obl.bodies;
    let key = match find_main(&mir.bodies) {
        Ok(b) => b.key.clone(),
        Err(e) => {
            return RunReport {
                end: RunEnd::Internal(e),
                value: None,
            };
        }
    };
    let mut it = Interp::new(&mir, fuel);
    match it.call_key(&key, Vec::new()) {
        Outcome::Value(v) => RunReport {
            end: RunEnd::Normal,
            value: Some(format!("{v:?}")),
        },
        Outcome::Abort(k) => RunReport {
            end: RunEnd::Aborted(k),
            value: None,
        },
        Outcome::OutOfFuel => RunReport {
            end: RunEnd::Limit,
            value: None,
        },
        Outcome::Bug(m) if m.contains("đệ quy quá sâu") => RunReport {
            end: RunEnd::Limit,
            value: None,
        },
        Outcome::Bug(m) => RunReport {
            end: RunEnd::Internal(format!("bộ chạy phát hiện MIR không hợp lệ: {m}")),
            value: None,
        },
    }
}

/// Văn bản gói: tiêu đề + attestation + MIR sau M7 (Debug tất định).
#[must_use]
pub fn package_text(res: &CheckResult, art: &Artifacts) -> (String, String) {
    let dump = format!("{:#?}", art.obl.bodies);
    let att = attestation(res, &dump);
    let mut s = String::new();
    s.push_str("GTMIR1\n");
    s.push_str(&format!("attestation: {att}\n"));
    s.push_str(&format!("gritc: {}\n", env!("CARGO_PKG_VERSION")));
    s.push_str(&format!("backend: {}\n", crate::render::BACKEND));
    s.push_str(&format!(
        "checks: {}\n",
        if res.checks == gt_oblig::driver::Checks::All {
            "all"
        } else {
            "proven"
        }
    ));
    s.push_str(&format!(
        "passes: {}\n",
        res.passes
            .iter()
            .map(|p| format!("{}={}", p.name, p.status.name()))
            .collect::<Vec<_>>()
            .join(",")
    ));
    s.push_str("---\n");
    s.push_str(&dump);
    s.push('\n');
    (s, att)
}

/// Ghi gói NGUYÊN TỬ (tệp tạm rồi đổi tên); lỗi ⇒ không để lại tệp tạm. Trả attestation.
pub fn write_package(
    path: &std::path::Path,
    res: &CheckResult,
    art: &Artifacts,
) -> Result<String, String> {
    let (text, att) = package_text(res, art);
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    if let Err(e) = std::fs::write(&tmp, text) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("không ghi được {}: {e}", tmp.display()));
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("không đổi tên thành {}: {e}", path.display()));
    }
    Ok(att)
}
