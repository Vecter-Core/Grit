//! ORACLE của điều kiện dừng (M8, ADR-026 §3): chương trình ngẫu nhiên có vòng lặp/đệ quy; sự thật nền = chạy trong thông dịch MIR với nhiên
//! liệu. Ràng buộc cứng: chương trình được CHẤP NHẬN (không E07xx) mà một đầu vào làm nó cạn nhiên liệu/đệ quy quá sâu = bộ kiểm dừng sai
//! (âm tính giả — lọt vòng lặp vô hạn). Chương trình bị bác mà thật sự phân kỳ là bác ĐÚNG; bị bác mà dừng là mất chính xác (chấp nhận).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
#[path = "support/loopgen.rs"]
mod loopgen;
#[path = "../../gt-mir/tests/support/progen.rs"]
mod progen;
use gt_mir::interp::{Interp, Outcome, V};
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::check_termination;
use std::collections::BTreeMap;

fn ity_of(name: &str) -> gt_types::IntTy {
    match name {
        "i32" => gt_types::IntTy::I32,
        "u32" => gt_types::IntTy::U32,
        "i64" => gt_types::IntTy::I64,
        "u8" => gt_types::IntTy::U8,
        _ => gt_types::IntTy::I16,
    }
}

fn programs() -> u64 {
    std::env::var("GT_TERM_PROGRAMS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1500)
}

#[test]
fn accepted_programs_never_run_out_of_fuel() {
    // GT_TERM_DROP=1: MÔ PHỎNG nghĩa vụ biến thể bị mất (B.3): gt-term không được chấp nhận chương trình phân kỳ nào (kiểm "tắt bản sửa": với logic cũ test này đỏ)
    let drop_variants = std::env::var("GT_TERM_DROP").is_ok_and(|v| v == "1");
    gt_oblig::analysis::set_drop_variant_obligations_for_test(drop_variants);
    let n = programs();
    let mut tally: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut codes: BTreeMap<String, u64> = BTreeMap::new();
    for seed in 1..=n {
        let mut g = loopgen::LGen::new(seed);
        let p = g.program();
        let t = common::run(&[("main.gt", p.src.as_str())]);
        assert!(
            t.report.diagnostics.is_empty(),
            "#{seed}: không qua M4: {:?}\n{}",
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
        let info = build_prog_info(&t, &m);
        let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
        assert!(
            !o.report
                .diagnostics
                .iter()
                .any(|d| d.code.as_str() == "E9901"),
            "#{seed}: ICE {:?}\n{}",
            o.report
                .diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>(),
            p.src
        );
        let accepted = o.report.diagnostics.is_empty();
        for d in &o.report.diagnostics {
            *codes.entry(d.code.as_str().to_string()).or_default() += 1;
        }
        let ty = ity_of(p.ty);
        let mut rng = progen::Rng(seed ^ 0x1357_9BDF_2468_ACE1 | 1);
        let mut diverged = false;
        for _ in 0..8 {
            let args: Vec<V> = (0..p.entry_params)
                .map(|_| V::Int(rng.below(26) as i128, ty))
                .collect();
            let mut it = Interp::new(&m, 300_000);
            match it.call_key("main::entry", args.clone()) {
                Outcome::OutOfFuel => diverged = true,
                Outcome::Bug(b) if b.contains("đệ quy quá sâu") => diverged = true,
                Outcome::Bug(b) => panic!("#{seed}: Bug {b}\n{}", p.src),
                _ => {}
            }
        }
        if accepted && diverged {
            panic!(
                "#{seed}: ĐƯỢC CHẤP NHẬN nhưng không dừng (âm tính giả)\n{}",
                p.src
            );
        }
        let key = match (accepted, diverged, p.has_loop || p.has_rec) {
            (true, false, _) => "chấp nhận, dừng",
            (false, true, _) => "bác, phân kỳ thật",
            (false, false, _) => "bác, nhưng dừng (mất chính xác)",
            (true, true, _) => unreachable!(),
        };
        *tally.entry(key).or_default() += 1;
    }
    eprintln!("{tally:?}\nmã: {codes:?}");
    if n < 1500 || drop_variants {
        return;
    }
    let get = |k: &str| tally.get(k).copied().unwrap_or(0);
    assert!(
        get("chấp nhận, dừng") >= 300,
        "chỉ {} chương trình chấp nhận",
        get("chấp nhận, dừng")
    );
    assert!(
        get("bác, phân kỳ thật") >= 200,
        "chỉ {} chương trình phân kỳ bị bác đúng",
        get("bác, phân kỳ thật")
    );
    for c in ["E0701", "E0702", "E0703"] {
        assert!(
            codes.get(c).copied().unwrap_or(0) >= 50,
            "mã {c} chỉ {} lần",
            codes.get(c).copied().unwrap_or(0)
        );
    }
}
