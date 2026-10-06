#![forbid(unsafe_code)]
//! gt-dataflow (M5, ADR-020 §3): khung dataflow tổng quát + phân tích cài sẵn (khởi tạo chắc chắn, liveness) và kiểm
//! bất biến khởi tạo của MIR (vi phạm = ICE; E0306 đã nghỉ — ADR-020 §9).
pub mod analyses;
pub mod framework;

pub use analyses::{
    DefinitelyAssigned, Liveness, MaybeInit, MaybeUninit, MovePaths, check_definitely_assigned,
};
pub use framework::{
    Analysis, BitSet, Direction, Dual, JoinSemiLattice, Results, predecessors, reverse_postorder,
    solve, state_at,
};

use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_mir::MirOutput;
use gt_types::TypesOutput;

/// Kiểm bất biến dataflow của mọi thân MIR; vi phạm là ICE (E9901).
#[must_use]
pub fn check_invariants(t: &TypesOutput, m: &MirOutput) -> Report {
    let mut bag = Diagnostics::new(100);
    for b in &m.bodies {
        if let Err(errs) = check_definitely_assigned(b)
            && let Ok(c) = Code::parse("E9901")
        {
            bag.push(Diagnostic::new(
                c,
                &t.file_name(b.node),
                b.span,
                format!(
                    "lỗi nội bộ của compiler (ICE): MIR của `{}` đọc biến chưa được gán: {}",
                    b.key,
                    errs.join("; ")
                ),
            ));
        }
    }
    bag.finish()
}
