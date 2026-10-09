//! CỔNG PHÁN QUYẾT (D17; GRIT_PLAN mục 5; ADR-028 §3). MỘT nơi duy nhất quyết định "có được sinh output / chạy hay không".
//!
//! **Mã thoát của `gritc` (phân vùng, ADR-028 §3 sửa theo Giang 2026-10-09):**
//!
//! | Mã | Nghĩa | Giai đoạn (`phase` trong JSON) |
//! |---|---|---|
//! | 0 | qua (có thể kèm VÀNG) / chương trình chạy xong bình thường | check / run |
//! | 1 | ĐỎ (hoặc VÀNG khi `--deny-yellow`) — không output, không chạy | check |
//! | 2 | ICE (lỗi nội bộ, kể cả thiếu pass bắt buộc, pass panic, bộ chạy phát hiện MIR sai) — không output, không chạy | check / run |
//! | 3 | lỗi dùng / I/O (đối số sai, không đọc/ghi được tệp, mã `explain` lạ, không có `main`) | check |
//! | 70 | chương trình **abort** (D3: tràn, chỉ số, chia 0, …) | run |
//! | 71 | bộ chạy tham chiếu hết giới hạn (nhiên liệu / độ sâu): chương trình KHÔNG kết thúc trong giới hạn của backend | run |
//! | 100+s | trạng thái thoát s (1..=100) do chính chương trình báo — dành riêng, Phase 1 chưa có cách tạo | run |
//!
//! Các vùng KHÔNG chồng nhau nên "chương trình trả 1" không bao giờ lẫn với "gritc báo ĐỎ": mã 1 chỉ do gritc ở giai đoạn `check`; mã
//! của chương trình luôn ở 0/70/71/100+. Mã thoát tuỳ ý của chương trình còn đi qua JSON (`run.exit_status`).
use crate::pipeline::{PassRecord, PassStatus, REQUIRED_PASSES};
use gt_diag::Report;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Green,
    Yellow,
    Red,
    Ice,
}

impl Verdict {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Verdict::Green => "green",
            Verdict::Yellow => "yellow",
            Verdict::Red => "red",
            Verdict::Ice => "ice",
        }
    }

    /// Chỉ XANH và VÀNG mở cổng (sinh output / chạy). ĐỎ và ICE thì KHÔNG BAO GIỜ.
    #[must_use]
    pub fn allows_output(self) -> bool {
        matches!(self, Verdict::Green | Verdict::Yellow)
    }
}

/// Phán quyết từ báo cáo và danh sách pass. Thứ tự ưu tiên (fail-closed): ICE > ĐỎ > (thiếu pass bắt buộc ⇒ ICE) > VÀNG/`--deny-yellow` > XANH.
#[must_use]
pub fn decide(report: &Report, passes: &[PassRecord], deny_yellow: bool) -> Verdict {
    if report.diagnostics.iter().any(|d| d.code.is_ice())
        || passes.iter().any(|p| p.status == PassStatus::Failed)
    {
        return Verdict::Ice;
    }
    if report.errors() > 0 {
        return Verdict::Red;
    }
    // KHÔNG có lỗi nào được báo: "sạch" chỉ đáng tin nếu MỌI pass bắt buộc đã chạy xong (điều kiện cứng E0601/E0307 nằm trong danh sách này)
    let done = |name: &str| {
        passes
            .iter()
            .any(|p| p.name == name && p.status == PassStatus::Done)
    };
    if !REQUIRED_PASSES.iter().all(|n| done(n)) {
        return Verdict::Ice;
    }
    if report.warnings() > 0 {
        if deny_yellow {
            Verdict::Red
        } else {
            Verdict::Yellow
        }
    } else {
        Verdict::Green
    }
}

/// Mã thoát của giai đoạn `check` (0/1/2).
#[must_use]
pub fn check_exit_code(v: Verdict) -> i32 {
    match v {
        Verdict::Green | Verdict::Yellow => 0,
        Verdict::Red => 1,
        Verdict::Ice => 2,
    }
}

/// Kết thúc của giai đoạn `run`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunEnd {
    /// chạy xong bình thường
    Normal,
    /// abort của chương trình (D3), kèm loại
    Aborted(String),
    /// hết nhiên liệu / độ sâu của bộ chạy tham chiếu
    Limit,
    /// chương trình báo trạng thái thoát `s` (dành riêng: Phase 1 chưa có cách tạo)
    ProgramExit(u8),
    /// bộ chạy phát hiện MIR không hợp lệ: lỗi nội bộ của gritc
    Internal(String),
}

/// Mã thoát của giai đoạn `run`. `ProgramExit(0)` = bình thường; s ∈ 1..=100 ↦ 100+s; ngoài khoảng ⇒ kẹp về 200 (vẫn trong vùng chương trình).
#[must_use]
pub fn run_exit_code(e: &RunEnd) -> i32 {
    match e {
        RunEnd::Normal | RunEnd::ProgramExit(0) => 0,
        RunEnd::Aborted(_) => 70,
        RunEnd::Limit => 71,
        RunEnd::ProgramExit(s) => 100 + i32::from((*s).min(100)),
        RunEnd::Internal(_) => 2,
    }
}
