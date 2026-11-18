//! GÓI 7 S2.a — ORACLE cho hàm có NHIỀU vòng lặp (vượt `GHOST_MAX_VARS` = 24 biến bóng). Bộ sinh `loopgen` chỉ tạo 1–2 vòng nên không bao giờ chạm giới hạn
//! (đó là lý do FB-C "biến bóng cũ làm vòng không dừng được chấp nhận" lọt). Ở đây: hàm 8–32 vòng, biến thể dùng chung `y`, `y` giảm xen kẽ trước các vòng,
//! có vòng đứng yên (không dừng). Sự thật nền = chạy thông dịch MIR có nhiên liệu. CHƯƠNG TRÌNH ĐƯỢC CHẤP NHẬN mà cạn nhiên liệu = âm tính giả = test ĐỎ.
//! Kiểm "tắt bản sửa": bỏ điều kiện `all_ghosted` ở `judge_variant_edge` (gt-oblig/src/analysis.rs) ⇒ test này PHẢI đỏ (ghi ở docs/FIELD_BUGS.md).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
#[path = "../../gt-mir/tests/support/progen.rs"]
mod progen;
use gt_mir::interp::{Interp, Outcome, V};
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::check_termination;
use std::collections::BTreeMap;

fn wide_program(seed: u64) -> String {
    let mut rng = progen::Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let loops = if rng.chance(50) { 8 + rng.below(18) as usize } else { 26 + rng.below(7) as usize };
    // tối đa MỘT vòng đứng yên (không dừng) ở vị trí ngẫu nhiên, 60% chương trình có; còn lại mọi vòng dừng thật
    let bad = if rng.chance(60) { Some(loops - 1 - rng.below(3).min(loops as u64 - 1) as usize) } else { None };
    let mut s = String::from("ƒ entry(a0: u32, a1: u32, a2: u32) → u32 ! ∅ {\n  μy ≔ a0 + 100u32;\n");
    for k in 0..loops {
        // thỉnh thoảng giảm y (có chặn) trước vòng
        if rng.chance(10) {
            s.push_str("  ⎇ y > 0u32 { y ← y - 1u32; }\n");
        }
        let (variant, body) = match if bad == Some(k) { if rng.chance(80) { 6 } else { 7 } } else { rng.below(6) } {
            0 | 1 => (format!("5 - k{k}"), format!("k{k} ← k{k} + 1;")),
            2 | 3 => (format!("y + 5 - k{k}"), format!("k{k} ← k{k} + 1;")),
            4 => (format!("y + 5 - k{k}"), format!("k{k} ← k{k} + 1; ⎇ y > 0u32 {{ y ← y - 1u32; }}")),
            5 => (format!("a1 + 5 - k{k}"), format!("k{k} ← k{k} + 1;")),
            6 => ("y".to_string(), String::new()), // đứng yên: không dừng
            _ => (format!("5 - k{k}"), String::new()), // đứng yên: không dừng
        };
        if bad == Some(k) {
            // y giảm CHẶT ngay trước vòng đứng yên (trong nhánh y > 0): `y < bóng cũ của y` đúng nếu bóng cũ còn trong trạng thái
            s.push_str(&format!("  ⎇ y > 0u32 {{ y ← y - 1u32; μk{k} ≔ 0u32; ⟳ k{k} < 5 ↓ {variant} {{ {body} }} }}\n"));
        } else {
            s.push_str(&format!("  μk{k} ≔ 0u32; ⟳ k{k} < 5 ↓ {variant} {{ {body} }}\n"));
        }
    }
    s.push_str("  y\n}\n");
    s
}

#[test]
fn wide_functions_accepted_never_run_out_of_fuel() {
    let n: u64 = std::env::var("GT_WIDE_PROGRAMS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(150);
    let mut tally: BTreeMap<&'static str, u64> = BTreeMap::new();
    for seed in 1..=n {
        let src = wide_program(seed);
        let t = common::run(&[("main.gt", src.as_str())]);
        assert!(t.report.diagnostics.is_empty(), "#{seed}: không qua M4: {:?}\n{src}", common::show(&t));
        let m = gt_mir::lower_program(&t);
        assert!(m.report.diagnostics.is_empty(), "#{seed}: {:?}\n{src}", common::mir_codes(&m));
        let info = build_prog_info(&t, &m);
        let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
        assert!(!o.report.diagnostics.iter().any(|d| d.code.as_str() == "E9901"), "#{seed}: ICE\n{src}");
        let accepted = o.report.diagnostics.is_empty();
        let mut rng = progen::Rng(seed ^ 0x1357_9BDF_2468_ACE1 | 1);
        let mut diverged = false;
        for _ in 0..2 {
            let args: Vec<V> = (0..3)
                .map(|_| V::Int(rng.below(6) as i128, gt_types::IntTy::U32))
                .collect();
            let mut it = Interp::new(&m, 60_000);
            match it.call_key("main::entry", args) {
                Outcome::OutOfFuel => diverged = true,
                Outcome::Bug(b) => panic!("#{seed}: Bug {b}\n{src}"),
                _ => {}
            }
        }
        if accepted && diverged {
            panic!("#{seed}: ĐƯỢC CHẤP NHẬN nhưng không dừng (âm tính giả)\n{src}");
        }
        let key = match (accepted, diverged) {
            (true, false) => "chấp nhận, dừng",
            (false, true) => "bác, phân kỳ thật",
            (false, false) => "bác, nhưng dừng (mất chính xác)",
            (true, true) => unreachable!(),
        };
        *tally.entry(key).or_default() += 1;
    }
    eprintln!("{tally:?}");
    // không rỗng nghĩa: phải có chương trình bị bác vì phân kỳ thật (kiểm bộ sinh chạm các vòng không dừng)
    if n >= 150 {
        assert!(tally.get("bác, phân kỳ thật").copied().unwrap_or(0) >= 20, "bộ sinh quá yếu: {tally:?}");
        assert!(tally.get("chấp nhận, dừng").copied().unwrap_or(0) >= 5, "bộ sinh không tạo chương trình được chấp nhận: {tally:?}");
    }
}
