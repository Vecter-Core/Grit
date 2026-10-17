//! FB-9: hồ sơ release/release-o2 từng từ chối MỌI chương trình có `static Mutex` (Guard-post thấy `nuw` trên biểu thức hằng GEP). Test hồi quy RẺ
//! (chỉ dựng, không chạy lặp): hai mẫu có Mutex tĩnh phải DỰNG ĐƯỢC ở cả hai hồ sơ release; gt-guard vẫn từ chối `nuw` trên số học (test ở gt-guard).
#![cfg(feature = "llvm")]
use gt_driver::native::build_exe_profile;
use gt_driver::pipeline::{attestation, mir_dump};
use gt_driver::{Options, check};
use gt_resolve::SourceInput;

const THREADS: &str = include_str!("../../../tests/samples/threads.gt");
const TWO_LOCKS: &str = include_str!("../../../tests/samples/two_locks.gt");

fn build(src: &str, opt: gt_codegen_llvm::Opt, tag: &str) -> Result<(), String> {
    let inputs = [SourceInput {
        name: "main.gt".to_string(),
        text: src.to_string(),
    }];
    let mut res = check(&inputs, &Options::default()).expect("check");
    assert!(res.verdict.allows_output());
    let att = {
        let art = res.open_gate().expect("cổng");
        attestation(&res, &mir_dump(art))
    };
    let art = res.take_gated().expect("cổng");
    let dir = std::env::temp_dir().join(format!("gt-fb9-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo");
    let exe = dir.join(format!("{tag}.exe"));
    build_exe_profile(&art, &exe, true, &att, opt)
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

#[test]
fn static_mutex_programs_build_in_both_release_profiles() {
    for (name, src) in [("threads", THREADS), ("two_locks", TWO_LOCKS)] {
        for (pn, opt) in [
            ("release", gt_codegen_llvm::Opt::Release),
            ("release_o2", gt_codegen_llvm::Opt::ReleaseO2),
        ] {
            build(src, opt, &format!("{name}_{pn}"))
                .unwrap_or_else(|e| panic!("{name}/{pn} bị từ chối: {e}"));
        }
    }
}
