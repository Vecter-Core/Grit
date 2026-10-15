#![forbid(unsafe_code)]
//! gt-borrowck (M6, ADR-023): sở hữu & mượn trên MIR theo NLL 2018 — sound nhưng không complete (P5).
//!
//! Đọc MIR của gt-mir (+ `RegionSig`/`TypeDb` của gt-types); KHÔNG đọc AST. Không có state ngoài đầu vào: tất định.
//! Chỉ chạy khi kiểm kiểu và hạ MIR sạch (fail-closed): `check_program` trả rỗng nếu một trong hai có chẩn đoán.
mod check;
mod regions;
mod solve;

use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_mir::{Body, MirOutput};
use gt_span::Span;
use gt_types::{RegionRef, TypesOutput};
use regions::Gen;
#[cfg(feature = "mutant-probe")]
pub use regions::Mutant;
pub use regions::defensive_counters;

/// `true` chỉ khi crate được dựng với feature `mutant-probe` (CHỈ trong test). Bản phát hành của `gritc` PHẢI có `false`
/// (test `gt-driver/tests/release_policy.rs::release_has_no_mutant_probe`).
pub const MUTANT_PROBE: bool = cfg!(feature = "mutant-probe");
use std::collections::BTreeSet;

const LIMIT: usize = 100;

fn ice(t: &TypesOutput, b: &Body, msg: &str) -> Diagnostic {
    let c = Code::parse("E9901").unwrap_or_else(|_| unreachable_code());
    Diagnostic::new(
        c,
        &t.file_name(b.node),
        b.span,
        format!(
            "lỗi nội bộ của compiler (ICE) trong borrowck `{}`: {msg}",
            b.key
        ),
    )
}

fn unreachable_code() -> Code {
    // E9901 luôn hợp lệ (bảng mã); nhánh này không thể tới
    Code::parse("E9901").unwrap_or_else(|_| unreachable_code())
}

fn name_of(r: &RegionRef) -> String {
    match r {
        RegionRef::Named(n) => format!("'{n}"),
        RegionRef::Static => "'static".to_string(),
        RegionRef::Anon(k) => format!("(vị trí tham chiếu ẩn #{k})"),
    }
}

/// Kiểm một thân; trả chẩn đoán (chưa sắp xếp).
#[must_use]
pub fn check_body(t: &TypesOutput, body: &Body) -> Vec<Diagnostic> {
    #[cfg(feature = "mutant-probe")]
    {
        check_body_impl(t, body, Mutant::None)
    }
    #[cfg(not(feature = "mutant-probe"))]
    {
        check_body_impl(t, body)
    }
}

fn check_body_impl(
    t: &TypesOutput,
    body: &Body,
    #[cfg(feature = "mutant-probe")] mutant: Mutant,
) -> Vec<Diagnostic> {
    let pts = solve::Points::new(body);
    let live = match solve::live_before(body, &pts) {
        Ok(l) => l,
        Err(e) => return vec![ice(t, body, &e)],
    };
    let mut g = Gen::new(t, body);
    #[cfg(feature = "mutant-probe")]
    {
        g.mutant = mutant;
    }
    g.generate();
    if let Some(e) = g.ice.take() {
        return vec![ice(t, body, &e)];
    }
    let values = solve::solve(&g, &pts, &live);
    let mut ck = check::Checker::new(t, body);
    if let Err(e) = ck.run(&pts, &values) {
        return vec![ice(t, body, &e)];
    }
    // vi phạm region phổ quát: region `U` phải outlive `V` mà chữ ký không hứa
    let mut seen = BTreeSet::new();
    for j in 1..g.univ.len() {
        for k in 0..g.univ.len() {
            if j == k || !values.has_end(g.univ[j].0, k) {
                continue;
            }
            if !seen.insert((j, k)) {
                continue;
            }
            let span = cause_span(&g, g.univ[k].0, g.univ[j].0).unwrap_or(body.span);
            let msg = format!(
                "tham chiếu có region {} bị dùng ở nơi cần {}: chữ ký không hứa {} sống lâu hơn {} (thêm `'r` tường minh hoặc đổi dữ liệu trả về)",
                name_of(&g.univ[j].1),
                name_of(&g.univ[k].1),
                name_of(&g.univ[j].1),
                name_of(&g.univ[k].1)
            );
            ck.push("E0404", span, msg, (1_000_000 + j * 1000 + k, 0));
        }
    }
    ck.diags
}

/// Cạnh ràng buộc cuối cùng trên đường từ `from` tới `to` (để định vị lỗi region phổ quát).
fn cause_span(g: &Gen<'_>, from: usize, to: usize) -> Option<Span> {
    // cons: a : b  ⇒ giá trị đi từ b sang a. BFS từ `from` theo cạnh b→a.
    let mut prev: Vec<Option<usize>> = vec![None; g.n];
    let mut seen = vec![false; g.n];
    let mut q = std::collections::VecDeque::new();
    seen[from] = true;
    q.push_back(from);
    while let Some(x) = q.pop_front() {
        if x == to {
            break;
        }
        for (ci, c) in g.cons.iter().enumerate() {
            if c.b == x && !seen[c.a] {
                seen[c.a] = true;
                prev[c.a] = Some(ci);
                q.push_back(c.a);
            }
        }
    }
    let mut cur = to;
    let mut best = None;
    while let Some(ci) = prev[cur] {
        let c = &g.cons[ci];
        if c.at.is_some() {
            best = Some(c.span);
        }
        cur = c.b;
    }
    best
}

/// Kiểm mọi thân. Fail-closed: nếu kiểm kiểu hoặc hạ MIR đã có chẩn đoán thì không chạy (trả rỗng).
#[must_use]
pub fn check_program(t: &TypesOutput, m: &MirOutput) -> Report {
    #[cfg(feature = "mutant-probe")]
    {
        check_program_impl(t, m, Mutant::None)
    }
    #[cfg(not(feature = "mutant-probe"))]
    {
        check_program_impl(t, m)
    }
}

/// CHỈ TEST (feature `mutant-probe`): như `check_program` nhưng chọn đột biến của `regions.rs` lúc chạy.
#[cfg(feature = "mutant-probe")]
#[must_use]
pub fn check_program_probe(t: &TypesOutput, m: &MirOutput, mutant: Mutant) -> Report {
    check_program_impl(t, m, mutant)
}

/// CHỈ TEST (feature `mutant-probe`): tập CẠNH ràng buộc outlives `(a, b)` ("a : b") mà `Gen` sinh cho mọi thân, theo đột biến đã chọn. Dùng để kiểm TIỀN ĐỀ
/// của lập luận đơn điệu: tập cạnh của bản gốc PHẢI là tập con của tập cạnh của mỗi đột biến CHẶT hơn (361/591/554). Không chạy borrowck.
#[cfg(feature = "mutant-probe")]
#[must_use]
pub fn probe_constraint_edges(
    t: &TypesOutput,
    m: &MirOutput,
    mutant: Mutant,
) -> Vec<(String, BTreeSet<(usize, usize)>)> {
    let mut out = Vec::new();
    for b in &m.bodies {
        let mut g = Gen::new(t, b);
        g.mutant = mutant;
        g.generate();
        out.push((b.key.clone(), g.cons.iter().map(|c| (c.a, c.b)).collect()));
    }
    out
}

fn check_program_impl(
    t: &TypesOutput,
    m: &MirOutput,
    #[cfg(feature = "mutant-probe")] mutant: Mutant,
) -> Report {
    let mut bag = Diagnostics::new(LIMIT);
    if !t.report.diagnostics.is_empty() || !m.report.diagnostics.is_empty() {
        return bag.finish();
    }
    let result = gt_stack::run_with_big_stack(|| {
        let mut all = Vec::new();
        for b in &m.bodies {
            #[cfg(feature = "mutant-probe")]
            all.extend(check_body_impl(t, b, mutant));
            #[cfg(not(feature = "mutant-probe"))]
            all.extend(check_body_impl(t, b));
        }
        all
    });
    match result {
        Ok(ds) => {
            for d in ds {
                bag.push(d);
            }
        }
        Err(e) => {
            if let Ok(c) = Code::parse("E9901") {
                bag.push(Diagnostic::new(
                    c,
                    "<borrowck>",
                    Span::new(0, 0),
                    format!("lỗi nội bộ của compiler (ICE) trong borrowck: {e}"),
                ));
            }
        }
    }
    bag.finish()
}
