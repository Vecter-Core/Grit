#![no_main]
//! Fuzz chuỗi kiểm đầy đủ: với MỌI chuỗi UTF-8 làm một tệp nguồn, `gt_driver::pipeline::check` (backend thông dịch, `--checks=all`) phải
//! (1) không panic ra ngoài, (2) không trả chẩn đoán ICE (E9901: lỗi nội bộ = lỗi của gritc dù đã fail-closed), (3) kết thúc, (4) tất định
//! (hai lần chạy cho cùng danh sách mã chẩn đoán và cùng phán quyết). Hạt giống: `tests/{pass,fail,warn}/*.gt`.
use gt_driver::pipeline::{Options, check};
use gt_resolve::SourceInput;
use libfuzzer_sys::fuzz_target;

fn codes(src: &str, edition: u32) -> Vec<String> {
    let inputs = [SourceInput {
        name: "f.gt".to_string(),
        text: src.to_string(),
    }];
    match check(
        &inputs,
        &Options {
            edition,
            ..Options::default()
        },
    ) {
        Ok(res) => {
            let mut v: Vec<String> = res
                .report
                .diagnostics
                .iter()
                .map(|d| d.code.as_str().to_string())
                .collect();
            v.push(format!("{:?}", res.verdict));
            v
        }
        Err(e) => vec![format!("ERR:{e}")],
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else {
        return;
    };
    // FB-C49 (Gemini G-09a): fuzz CẢ edition 2
    for edition in [1u32, 2] {
        let a = codes(src, edition);
        assert!(
            !a.iter().any(|c| c == "E9901"),
            "ICE (E9901) trên đầu vào hợp lệ UTF-8 (edition {edition}): {a:?}"
        );
        assert_eq!(a, codes(src, edition), "không tất định");
    }
});
