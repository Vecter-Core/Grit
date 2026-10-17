//! FAIL_OPEN_AUDIT D1: `mir_dump` băm bảng instance vào attestation; instance KHÔNG có khoá hàm từng bị `filter_map` bỏ khỏi băm (attestation phủ ít hơn).
//! Bất biến: mọi instance sinh ra từ chương trình hợp lệ đều có khoá (nên nhánh `?missing` không bao giờ chạm tới), và `mir_dump` có băm bảng instance.
use gt_driver::pipeline::mir_dump;
use gt_driver::{Options, check};
use gt_resolve::SourceInput;

const GEN: &str = "ƒ nhân<T: Copy>(x: T) → T ! ∅ { x }\nƒ main() → u32 ! ∅ { a ≔ nhân(3u32); b ≔ nhân(4u8); a +% (b as u32) }\n";

#[test]
fn every_instance_has_a_function_key_and_the_dump_hashes_the_instance_table() {
    let inputs = [SourceInput {
        name: "main.gt".to_string(),
        text: GEN.to_string(),
    }];
    let mut res = check(&inputs, &Options::default()).expect("check");
    assert!(res.verdict.allows_output(), "{:?}", res.report.diagnostics);
    let art = res.open_gate().expect("cổng");
    assert!(art.types.instances.len() >= 2, "cần ≥ 2 instance sinh ra");
    for i in &art.types.instances {
        assert!(
            art.types.fn_keys.contains_key(&i.node),
            "instance thiếu khoá hàm: {:?}",
            i.node
        );
    }
    let dump = mir_dump(art);
    assert!(dump.contains("\ninstances "), "{dump}");
}
