#![forbid(unsafe_code)]
//! gt-kernel (Giai đoạn 2, A2/A3 bước 1 — ADR-P2-04): KERNEL nhỏ kiểm chứng chỉ Farkas cho nghĩa vụ số học tuyến tính nguyên.
//! TCB: `linear` (dịch nghĩa vụ ⇒ DNF ràng buộc) và `check` (kiểm chứng chỉ). NGOÀI TCB: `search` (tìm chứng chỉ) và `CertifiedProver`.
pub mod check;
pub mod linear;
pub mod search;

pub use check::{CaseCert, FarkasCert, check};
use gt_oblig::ir::{Budget, Cert, Obligation, Prover, Verdict};

/// Tìm chứng chỉ cho cả nghĩa vụ (mọi ca); `None` nếu không tìm được trong ngân sách.
#[must_use]
pub fn find_cert(ob: &Obligation, steps: &mut u64) -> Option<FarkasCert> {
    let d = linear::refutation_dnf(ob)?;
    let mut cases = Vec::with_capacity(d.len());
    for case in &d {
        // mâu thuẫn nguyên tử trước (rẻ), rồi Farkas
        let conflict = case
            .atoms
            .iter()
            .find(|(a, p)| *p && case.atoms.contains(&(*a, false)))
            .map(|(a, _)| *a);
        cases.push(match conflict {
            Some(a) => CaseCert::Atom(a),
            None => CaseCert::Farkas(search::refute_case(&case.cons, steps)?),
        });
    }
    Some(FarkasCert { cases })
}

/// Số liệu của `CertifiedProver` (để đo độ phủ kernel).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KernelStats {
    /// bộ chứng minh bên trong (Phase 1) chứng minh được
    pub inner_proved: u64,
    /// ... và kernel cũng xác nhận bằng chứng chỉ (chỉ đếm khi bật đo độ phủ)
    pub inner_kernel_confirmed: u64,
    /// bộ bên trong "Unknown" nhưng chứng chỉ Farkas được kernel chấp nhận
    pub farkas_only: u64,
    /// bộ tìm đưa chứng chỉ mà kernel TỪ CHỐI (lỗi của bộ tìm; không bao giờ thành `Proved`)
    pub rejected: u64,
    /// chế độ `require_kernel`: bộ bên trong chứng minh nhưng KHÔNG có chứng chỉ kernel ⇒ hạ xuống "Unknown"
    pub inner_unconfirmed: u64,
}

/// Bọc một bộ chứng minh Phase 1: giữ nguyên kết quả của nó; khi nó "Unknown", thử chứng chỉ Farkas và CHỈ trả `Proved` khi kernel chấp nhận.
pub struct CertifiedProver<P: Prover> {
    pub inner: P,
    pub stats: KernelStats,
    /// đo độ phủ: thử chứng chỉ cả cho những nghĩa vụ bộ bên trong đã chứng minh (tốn thời gian; tắt mặc định)
    pub measure: bool,
    /// ngân sách bước của bộ tìm cho mỗi nghĩa vụ
    pub search_steps: u64,
    /// CHỈ TIN KERNEL: chứng minh của bộ bên trong chỉ được dùng khi kernel xác nhận được chứng chỉ (bộ bên trong ra khỏi TCB cho quyết định
    /// "đã chứng minh"); không xác nhận được ⇒ "Unknown" (fail-closed: chỉ làm gritc từ chối/giữ kiểm nhiều hơn)
    pub require_kernel: bool,
}

impl<P: Prover> CertifiedProver<P> {
    pub fn new(inner: P) -> CertifiedProver<P> {
        CertifiedProver {
            inner,
            stats: KernelStats::default(),
            measure: false,
            search_steps: 20_000,
            require_kernel: false,
        }
    }
}

impl<P: Prover> Prover for CertifiedProver<P> {
    fn name(&self) -> &'static str {
        "zone+farkas-kernel"
    }

    fn prove(&mut self, ob: &Obligation, budget: &mut Budget) -> Verdict {
        let v = self.inner.prove(ob, budget);
        match v {
            Verdict::Proved(_) => {
                self.stats.inner_proved += 1;
                if self.require_kernel {
                    let mut steps = self.search_steps;
                    return match find_cert(ob, &mut steps) {
                        Some(c) if check(ob, &c).is_ok() => {
                            self.stats.inner_kernel_confirmed += 1;
                            Verdict::Proved(Cert(c.encode()))
                        }
                        _ => {
                            self.stats.inner_unconfirmed += 1;
                            Verdict::Unknown
                        }
                    };
                }
                if self.measure {
                    let mut steps = self.search_steps;
                    if find_cert(ob, &mut steps).is_some_and(|c| check(ob, &c).is_ok()) {
                        self.stats.inner_kernel_confirmed += 1;
                    }
                }
                v
            }
            Verdict::Unknown => {
                let mut steps = self.search_steps.min(budget.steps);
                let before = steps;
                let found = find_cert(ob, &mut steps);
                let _ = budget.spend(before - steps);
                match found {
                    Some(c) => match check(ob, &c) {
                        Ok(()) => {
                            self.stats.farkas_only += 1;
                            Verdict::Proved(Cert(c.encode()))
                        }
                        Err(_) => {
                            self.stats.rejected += 1;
                            Verdict::Unknown
                        }
                    },
                    None => Verdict::Unknown,
                }
            }
            r @ Verdict::Refuted { .. } => r,
        }
    }
}
