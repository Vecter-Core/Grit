//! ORACLE mức chương trình của M7 (ADR-025 §8): chương trình ngẫu nhiên có nhiều loại nghĩa vụ; THAM CHIẾU = MIR giữ MỌI check
//! (bộ giải "không chứng minh gì": mọi `Assert` còn, mọi hợp đồng chèn kiểm), ỨNG VIÊN = MIR sau M7 với bộ giải chính. Chạy cả hai
//! trên cùng đầu vào (biên + ngẫu nhiên): kết quả PHẢI giống hệt — nếu ứng viên xoá một check đáng lẽ thất bại, tham chiếu abort mà
//! ứng viên không (âm tính giả). Ngược lại, mọi nghĩa vụ "chắc chắn sai" mà tham chiếu CHẠM TỚI phải THẤT BẠI ở đó.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
#[path = "../../gt-mir/tests/support/interp.rs"]
mod interp;
#[path = "support/obgen.rs"]
mod obgen;
#[path = "../../gt-mir/tests/support/progen.rs"]
mod progen;
use gt_mir::{AssertKind, MirOutput, TerminatorKind};
use gt_oblig::analysis::Status;
use gt_oblig::driver::{OblOutput, analyze_program};
use gt_oblig::ir::{Budget, ObKind, Obligation, Prover, Verdict};
use gt_oblig::prover::ZoneProver;
use interp::{Interp, Outcome, V};
use std::collections::BTreeMap;

struct ZeroProver;
impl Prover for ZeroProver {
    fn name(&self) -> &'static str {
        "zero"
    }
    fn prove(&mut self, _: &Obligation, _: &mut Budget) -> Verdict {
        Verdict::Unknown
    }
}

fn mir_of(m: &MirOutput, o: &OblOutput) -> MirOutput {
    MirOutput {
        report: gt_diag::Diagnostics::new(1).finish(),
        bodies: o.bodies.clone(),
        keys: m.keys.clone(),
    }
}

/// Tên "loại.phép" của một nghĩa vụ (từ `Assert` gốc).
fn op_name(m: &MirOutput, key: &str, block: u32, kind: &ObKind) -> String {
    if matches!(kind, ObKind::Pre | ObKind::Post | ObKind::Invariant) {
        return kind.name().to_string();
    }
    let b = m.body(key).expect("thân");
    match b.blocks[block as usize]
        .terminator
        .as_ref()
        .map(|t| &t.kind)
    {
        Some(TerminatorKind::Assert {
            kind: AssertKind::Overflow(op, ..),
            ..
        }) => format!("overflow.{}", op.name()),
        _ => kind.name().to_string(),
    }
}

#[derive(Default)]
struct Cov {
    /// (loại.phép, trạng thái) → (số nghĩa vụ, số lần chạm tới trong chạy tham chiếu)
    cells: BTreeMap<(String, &'static str), (u64, u64)>,
    runs: u64,
    compared: u64,
    skipped: u64,
    aborts: u64,
    abort_kinds: BTreeMap<String, u64>,
}

fn status_name(s: &Status) -> &'static str {
    match s {
        Status::Discharged { .. } => "xoa",
        Status::DefinitelyFalse => "do",
        Status::Lowered { .. } => "giu",
        Status::Rejected { .. } => "e0902",
    }
}

fn inputs(rng: &mut progen::Rng, ty: obgen::Ity, n: usize, count: usize) -> Vec<Vec<i128>> {
    let b = ty.boundary();
    (0..count)
        .map(|_| {
            (0..n)
                .map(|_| {
                    if rng.chance(75) {
                        b[rng.below(b.len() as u64) as usize]
                    } else {
                        let span = ((ty.max() - ty.min()).min(1 << 30)) as u64;
                        (ty.min().max(-(1 << 29)) + rng.below(span + 1) as i128)
                            .clamp(ty.min(), ty.max())
                    }
                })
                .collect()
        })
        .collect()
}

fn ity_of(name: &str) -> gt_types::IntTy {
    match name {
        "i8" => gt_types::IntTy::I8,
        "u8" => gt_types::IntTy::U8,
        "i16" => gt_types::IntTy::I16,
        "u16" => gt_types::IntTy::U16,
        "i32" => gt_types::IntTy::I32,
        "u32" => gt_types::IntTy::U32,
        "i64" => gt_types::IntTy::I64,
        "u64" => gt_types::IntTy::U64,
        _ => gt_types::IntTy::Usize,
    }
}

fn one_program(seed: u64, cov: &mut Cov, stats: &mut obgen::GenStats) {
    let mut g = obgen::OGen::new(seed);
    let p = g.program();
    let s = &g.stats;
    stats.programs += s.programs;
    stats.loops += s.loops;
    stats.invariants += s.invariants;
    stats.arrays += s.arrays;
    stats.slice_calls += s.slice_calls;
    stats.pre += s.pre;
    stats.post += s.post;
    stats.casts += s.casts;
    stats.divs += s.divs;
    stats.negs += s.negs;
    let t = common::run(&[("main.gt", p.src.as_str())]);
    assert!(
        t.report.diagnostics.is_empty(),
        "#{seed}: chương trình sinh ra không qua M4: {:?}\n{}",
        common::show(&t),
        p.src
    );
    let m = gt_mir::lower_program(&t);
    assert!(
        m.report.diagnostics.is_empty(),
        "#{seed}: {:?}\n{}",
        common::mir_codes(&m),
        p.src
    );
    let o_ref = analyze_program(&t, &m, &mut ZeroProver);
    let o_new = analyze_program(&t, &m, &mut ZoneProver::new(24));
    assert!(
        !o_new
            .report
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "E9901"),
        "#{seed}: ICE {:?}\n{}",
        o_new
            .report
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>(),
        p.src
    );
    let m_ref = mir_of(&m, &o_ref);
    let m_new = mir_of(&m, &o_new);
    // bảng nghĩa vụ của ứng viên: (khoá thân, khối) → (loại.phép, trạng thái)
    let mut table: BTreeMap<(String, u32), (String, &'static str)> = BTreeMap::new();
    for r in &o_new.per_body {
        for j in &r.judged {
            let k = op_name(&m, &r.key, j.ob.block, &j.ob.kind);
            let st = status_name(&j.status);
            cov.cells.entry((k.clone(), st)).or_default().0 += 1;
            table.insert((r.key.clone(), j.ob.block), (k, st));
        }
    }
    let ty = ity_of(p.ty.name);
    let mut rng = progen::Rng(seed ^ 0xABCD_EF01_2345_6789 | 1);
    let ins = inputs(&mut rng, p.ty, p.entry_params, 24);
    for input in ins {
        cov.runs += 1;
        let args: Vec<V> = input.iter().map(|v| V::Int(*v, ty)).collect();
        let mut ir = Interp::new(&m_ref, 200_000);
        let out_r = ir.call_key("main::entry", args.clone());
        // vết của tham chiếu: nghĩa vụ chạm tới
        for (k, b, ok) in &ir.asserts {
            if let Some((kind, st)) = table.get(&(k.clone(), *b)) {
                cov.cells.entry((kind.clone(), st)).or_default().1 += 1;
                if *st == "do" {
                    assert!(
                        !*ok,
                        "#{seed}: nghĩa vụ 'chắc chắn sai' ({k} bb{b}) mà tham chiếu chạm tới lại ĐẠT, đầu vào {input:?}\n{}",
                        p.src
                    );
                }
            }
        }
        let mut inew = Interp::new(&m_new, 200_000);
        let out_n = inew.call_key("main::entry", args);
        match (&out_r, &out_n) {
            (Outcome::Bug(_) | Outcome::OutOfFuel, _)
            | (_, Outcome::Bug(_) | Outcome::OutOfFuel) => {
                cov.skipped += 1;
                if let Outcome::Bug(b) = &out_r {
                    panic!("#{seed}: Bug trong tham chiếu: {b}\n{}", p.src);
                }
                if let Outcome::Bug(b) = &out_n {
                    panic!("#{seed}: Bug trong ứng viên: {b}\n{}", p.src);
                }
            }
            (a, b) => {
                cov.compared += 1;
                if let Outcome::Abort(k) = a {
                    cov.aborts += 1;
                    *cov.abort_kinds.entry(k.clone()).or_default() += 1;
                }
                assert_eq!(
                    a, b,
                    "#{seed}: ứng viên khác tham chiếu với đầu vào {input:?} (một check bị xoá sai?)\n{}",
                    p.src
                );
            }
        }
    }
}

const FULL_PROGRAMS: u64 = 1500;

fn programs() -> u64 {
    std::env::var("GT_OBLIG_PROGRAMS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(FULL_PROGRAMS)
}

#[test]
fn pruned_programs_behave_exactly_like_fully_checked_ones() {
    let mut cov = Cov::default();
    let mut stats = obgen::GenStats::default();
    let n = programs();
    for seed in 1..=n {
        one_program(seed, &mut cov, &mut stats);
    }
    eprintln!("sinh: {stats:?}");
    eprintln!(
        "chạy {} (so sánh {}, bỏ {}, abort {})",
        cov.runs, cov.compared, cov.skipped, cov.aborts
    );
    for ((k, st), (n, hit)) in &cov.cells {
        eprintln!("  {k:<22} {st:<6} nghĩa vụ {n:>5}  chạm {hit:>6}");
    }
    eprintln!("  loại abort (tham chiếu): {:?}", cov.abort_kinds);
    if n < FULL_PROGRAMS {
        return; // bản nhanh (đột biến giai đoạn 1): chỉ kiểm tính đúng, không kiểm ngưỡng độ phủ
    }
    // ---- ngưỡng độ phủ (ghi trong ADR-025 §12): mỗi ô (loại.phép × xoá|giữ) ≥ 100 nghĩa vụ và ≥ 100 lần chạm;
    //      ô 'đỏ' ≥ 8 và ≥ 8; slice_range ≥ 40; hợp đồng: số nghĩa vụ ≥ 100 (xoá|giữ) và mỗi loại abort ≥ 100.
    let need = |k: &str, st: &'static str, min_n: u64, min_hit: u64| {
        let (n, hit) = cov
            .cells
            .get(&(k.to_string(), st))
            .copied()
            .unwrap_or((0, 0));
        assert!(
            n >= min_n && hit >= min_hit,
            "độ phủ ô ({k}, {st}) = ({n}, {hit}) < ({min_n}, {min_hit})"
        );
    };
    for k in [
        "overflow.Add",
        "overflow.Sub",
        "overflow.Mul",
        "overflow.Div",
        "overflow.Rem",
        "neg_min",
        "div_zero",
        "rem_zero",
        "index",
        "cast_lossy",
        "invariant",
    ] {
        need(k, "xoa", 100, 100);
        need(k, "giu", 100, 100);
    }
    need("slice_range", "xoa", 40, 40);
    need("slice_range", "giu", 40, 40);
    for k in [
        "overflow.Add",
        "overflow.Sub",
        "overflow.Mul",
        "overflow.Div",
        "neg_min",
        "div_zero",
        "rem_zero",
        "index",
        "cast_lossy",
        "slice_range",
        "invariant",
    ] {
        need(k, "do", 8, 8);
    }
    for k in ["pre", "post"] {
        need(k, "xoa", 100, 0);
        need(k, "giu", 100, 0);
        need(k, "do", 8, 0);
    }
    for k in [
        "precondition",
        "postcondition",
        "invariant",
        "overflow",
        "div_by_zero",
        "rem_by_zero",
        "index_out_of_bounds",
        "cast_lossy",
        "overflow_neg",
    ] {
        let n = cov.abort_kinds.get(k).copied().unwrap_or(0);
        assert!(n >= 100, "abort '{k}' chỉ {n} < 100");
    }
    assert!(
        cov.compared >= 30_000,
        "chỉ so sánh {} lần chạy",
        cov.compared
    );
}
