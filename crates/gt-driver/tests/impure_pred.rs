//! FB-C39 (Gemini G-02, kiểm chéo khác họ): hàm `! ∅` đọc/ghi `■ Atomic` KHÔNG thuần ⇒ không được làm nguyên tử vị từ (`pure_preds`).
//! Trước sửa: kernel coi `is_ok(x)` là sự kiện ổn định ⇒ `--checks=proven` xoá nhầm `⊳ is_ok(x)` (chạy exit 0 dù `⊳` thực tế sai).
//! Test động: dưới `--checks=proven` chương trình PHẢI vẫn abort ở `⊳` (lần gọi thứ hai `is_ok` trả `false`).
#![cfg(feature = "llvm")]
use gt_driver::native::{build_exe_profile, run_exe_timeout};
use gt_driver::pipeline::{attestation, mir_dump};
use gt_driver::{Options, RunEnd, check};
use gt_oblig::driver::Checks;
use gt_resolve::SourceInput;
use std::time::Duration;

const SRC: &str = "\
■ A: Atomic<u32> ≔ Atomic::new(0u32);
ƒ is_ok(x: u32) → bool ! ∅ { A.fetch_add(1u32, AtomicOrdering::SeqCst) == 0u32 }
ƒ need_ok(x: u32) → u32 ! ∅ ⊳ is_ok(x) { x }
ƒ main() → u32 ! ∅ { x: u32 ≔ 1u32; ⎇ is_ok(x) { need_ok(x) } ⇝ { 0u32 } }
";

#[test]
fn a_predicate_that_touches_an_atomic_static_is_not_removed_under_checks_proven() {
    for checks in [Checks::All, Checks::Proven] {
        let mut res = check(
            &[SourceInput {
                name: "main.gt".to_string(),
                text: SRC.to_string(),
            }],
            &Options {
                checks,
                ..Options::default()
            },
        )
        .expect("check");
        assert!(res.verdict.allows_output(), "{checks:?}: phải xanh/vàng");
        let dir = std::env::temp_dir().join(format!("gt-impure-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tạo");
        let exe = dir.join(format!("p{checks:?}.exe"));
        {
            let art = res.open_gate().expect("cổng");
            let att = attestation(&res, &mir_dump(art));
            build_exe_profile(art, &exe, true, &att, gt_codegen_llvm::Opt::Dev).expect("dựng exe");
        }
        let r = run_exe_timeout(&exe, Some(Duration::from_secs(20)));
        let _ = std::fs::remove_file(&exe);
        assert!(
            matches!(r.end, RunEnd::Aborted(_)),
            "{checks:?}: `⊳ is_ok(x)` sai ở lần gọi thứ hai, phải abort, nhận {:?}",
            r.end
        );
    }
}
