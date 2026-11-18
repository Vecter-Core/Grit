//! ORACLE mức chương trình của M7 (ADR-025 §8): chương trình ngẫu nhiên có nhiều loại nghĩa vụ; THAM CHIẾU = MIR giữ MỌI check
//! (bộ giải "không chứng minh gì": mọi `Assert` còn, mọi hợp đồng chèn kiểm), ỨNG VIÊN = MIR sau M7 với bộ giải chính. Chạy cả hai
//! trên cùng đầu vào (biên + ngẫu nhiên): kết quả PHẢI giống hệt — nếu ứng viên xoá một check đáng lẽ thất bại, tham chiếu abort mà
//! ứng viên không (âm tính giả). Ngược lại, mọi nghĩa vụ "chắc chắn sai" mà tham chiếu CHẠM TỚI phải THẤT BẠI ở đó.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
#[path = "support/obgen.rs"]
mod obgen;
#[path = "../../gt-mir/tests/support/progen.rs"]
mod progen;
use gt_mir::interp::{Interp, Outcome, V};
use gt_mir::{AssertKind, MirOutput, TerminatorKind};
use gt_oblig::analysis::Status;
use gt_oblig::driver::{OblOutput, analyze_program};
use gt_oblig::ir::{Budget, ObKind, Obligation, Prover, Verdict};
use gt_oblig::prover::ZoneProver;
use std::collections::{BTreeMap, BTreeSet};

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
    /// loại → các check BỊ XOÁ (chương trình, thân, khối) mà ít nhất một lần chạy kết thúc BÌNH THƯỜNG sau khi chạy qua nó
    /// (đoạn code sau check chạy tới cuối hàm — lệnh Giang 2026-10-08, mục 2)
    end_checks: BTreeMap<String, BTreeSet<(u64, String, u32)>>,
    /// loại → (số lần check được thực thi trong lần chạy kết thúc bình thường, … trong lần chạy abort)
    reach: BTreeMap<String, (u64, u64)>,
    normal_runs: u64,
}

fn status_name(s: &Status) -> &'static str {
    match s {
        Status::Discharged { .. } => "xoa",
        Status::DefinitelyFalse => "do",
        Status::Lowered { .. } => "giu",
        Status::Rejected { .. } => "e0902",
    }
}

/// Đầu vào của `entry` THOẢ tiền điều kiện `⊳` (không ai kiểm `⊳` của `entry` nên bộ chạy phải tôn trọng nó), thiên về biên của khoảng.
fn inputs(rng: &mut progen::Rng, ranges: &[(i128, i128)], count: usize) -> Vec<Vec<i128>> {
    (0..count)
        .map(|_| {
            ranges
                .iter()
                .map(|&(lo, hi)| {
                    let edges = [lo, lo + 1, hi - 1, hi, (lo + hi) / 2, 0, 1, -1];
                    if rng.chance(70) {
                        let e = edges[rng.below(edges.len() as u64) as usize];
                        if e >= lo && e <= hi {
                            return e;
                        }
                    }
                    let span = ((hi - lo).min(1 << 30)) as u64;
                    lo + rng.below(span + 1) as i128
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
    one_program_with(obgen::OGen::new(seed), seed, cov, stats);
}

fn one_program_with(mut g: obgen::OGen, seed: u64, cov: &mut Cov, stats: &mut obgen::GenStats) {
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
    stats.compound_loops += s.compound_loops;
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
    // LƯỚI THÊM (FB-1, 2026-10-13): chương trình sinh ra có `⊳`/`⊲` cùng tham số tham chiếu/lát; borrowck KHÔNG được báo ICE (E9901) trên chúng.
    // (Chương trình sinh ngẫu nhiên có thể bị borrowck từ chối bằng E04xx hợp lệ — vd gán lát sang lát khác region — nên chỉ cấm ICE.)
    let bck = gt_borrowck::check_program(&t, &m);
    assert!(
        !bck.diagnostics.iter().any(|d| d.code.as_str() == "E9901"),
        "#{seed}: ICE trong borrowck {:?}
{}",
        bck.diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>(),
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
    let ins = inputs(&mut rng, &p.entry_ranges, 24);
    let mut normal_in_prog = false;
    for input in ins {
        cov.runs += 1;
        let args: Vec<V> = input.iter().map(|v| V::Int(*v, ty)).collect();
        let mut ir = Interp::new(&m_ref, 200_000);
        let out_r = ir.call_key("main::entry", args.clone());
        // vết của tham chiếu: nghĩa vụ chạm tới
        let ended = matches!(out_r, Outcome::Value(_));
        if ended {
            cov.normal_runs += 1;
            normal_in_prog = true;
        }
        for (k, b, ok) in &ir.asserts {
            if let Some((kind, st)) = table.get(&(k.clone(), *b)) {
                cov.cells.entry((kind.clone(), st)).or_default().1 += 1;
                let r = cov.reach.entry(kind.clone()).or_default();
                if ended {
                    r.0 += 1;
                } else {
                    r.1 += 1;
                }
                if ended && *st == "xoa" {
                    cov.end_checks
                        .entry(kind.clone())
                        .or_default()
                        .insert((seed, k.clone(), *b));
                }
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
    // hợp đồng không có vết `Assert`: tính theo chương trình có >= 1 lần chạy kết thúc bình thường
    if normal_in_prog {
        for ((key, blk), (kind, st)) in &table {
            if *st == "xoa" && (kind == "pre" || kind == "post") {
                cov.end_checks
                    .entry(kind.clone())
                    .or_default()
                    .insert((seed, key.clone(), *blk));
            }
        }
    }
}

const FULL_PROGRAMS: u64 = 2000;

/// ngưỡng "đã kiểm": số check BỊ XOÁ mà đoạn sau nó chạy tới cuối hàm, mỗi loại (ADR-025 §13)
const END_THRESHOLD: usize = 100;

fn programs() -> u64 {
    std::env::var("GT_OBLIG_PROGRAMS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(FULL_PROGRAMS)
}

/// R2a (lệnh Giang 2026-10-17): CÙNG bộ sinh nhưng mọi lời gọi hàm phụ đi qua GIÁ TRỊ HÀM (`hv_hK ≔ hK; hv_hK(...)`): bản đã xoá check phải hành xử Y HỆT bản kiểm đầy đủ. Hàm bị lấy
/// làm giá trị KHÔNG được dùng `⊳` làm giả thiết xoá check (`escaped_fns`); nếu bộ giải rò `⊳` sang đường gián tiếp, oracle thấy hai bản khác nhau.
#[test]
fn pruned_programs_with_function_value_calls_behave_exactly_like_fully_checked_ones() {
    let mut cov = Cov::default();
    let mut stats = obgen::GenStats::default();
    let n = programs().min(3000);
    for seed in 1..=n {
        one_program_with(obgen::OGen::with_values(seed), seed, &mut cov, &mut stats);
    }
    eprintln!("fn_value: sinh: {stats:?}");
    eprintln!(
        "fn_value: chạy {} (so sánh {}, bỏ {}, abort {}), kết thúc bình thường {}",
        cov.runs, cov.compared, cov.skipped, cov.aborts, cov.normal_runs
    );
    assert!(
        cov.compared > 0 && cov.normal_runs > 0,
        "bộ sinh không ra lần chạy so sánh được"
    );
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
    eprintln!(
        "  lần chạy kết thúc bình thường: {}/{}",
        cov.normal_runs, cov.runs
    );
    for (k, (ok, ab)) in &cov.reach {
        eprintln!(
            "  {k:<22} thực thi trong lần chạy bình thường {ok:>8}, trong lần chạy abort {ab:>8}; check XOÁ chạy tới cuối hàm: {}",
            cov.end_checks.get(k).map_or(0, BTreeSet::len)
        );
    }
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
        "pre",
        "post",
    ] {
        let n = cov.end_checks.get(k).map_or(0, BTreeSet::len);
        assert!(
            n >= END_THRESHOLD,
            "loại '{k}': chỉ {n} check bị xoá mà đoạn sau chạy tới cuối hàm (< {END_THRESHOLD}) — chưa được tính là đã kiểm"
        );
    }
    assert!(
        cov.compared >= 30_000,
        "chỉ so sánh {} lần chạy",
        cov.compared
    );
}

#[test]
#[ignore = "thăm dò thủ công kịch bản alias, không khẳng định gì"]
fn probe_alias_scenario() {
    let mut shown = 0;
    let mut with = 0;
    for seed in 1..=300u64 {
        let mut g = obgen::OGen::new(seed);
        let p = g.program();
        if !p.src.contains("al0 ≔ xs") {
            continue;
        }
        with += 1;
        if shown < 2 {
            shown += 1;
            eprintln!("--- seed {seed}\n{}", p.src);
            let t = common::run(&[("main.gt", p.src.as_str())]);
            let m = gt_mir::lower_program(&t);
            let o = analyze_program(&t, &m, &mut ZoneProver::new(24));
            for r in &o.per_body {
                for j in &r.judged {
                    eprintln!(
                        "  {} {} {}",
                        j.ob.id,
                        j.ob.kind.name(),
                        status_name(&j.status)
                    );
                }
            }
        }
    }
    eprintln!("chương trình có kịch bản: {with}/300");
}
