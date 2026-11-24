//! Or-pattern (edition 2, ADR-P2-11) — oracle động. Chương trình ngẫu nhiên: `⌥ (x, e, o)` với `x: u32 ∈ 0..6`, `e` thuộc enum 4 biến thể, `o: Option<u32>` (None hoặc Some(0..4)); mỗi nhánh có pattern
//! ba thành phần, mỗi thành phần là `_`, literal/biến thể đơn, hoặc `Or` của 2–3 alternative (kể cả lồng trong `Some(…)`); nhánh cuối `_`. Mô hình Rust ĐỘC LẬP chọn nhánh khớp đầu tiên
//! trên MỌI tổ hợp (6 × 4 × 5 = 120 đầu vào); giá trị trả về hash mọi kết quả nhánh nên sai MỘT lần chọn là lệch. Thông dịch MIR VÀ exe LLVM (dev, release, release-o2) phải khớp.
use gt_driver::{Options, RunEnd, check};
use gt_resolve::SourceInput;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// thành phần của pattern: tập giá trị khớp + văn bản nguồn
struct Part {
    text: String,
    /// miền giá trị khớp (chỉ số trong miền của cột)
    set: Vec<u32>,
}

const ENUM: [&str; 4] = ["A", "B", "C", "D"];

fn int_part(r: &mut Rng) -> Part {
    match r.below(4) {
        0 => Part {
            text: "_".into(),
            set: (0..6).collect(),
        },
        1 => {
            let v = r.below(6) as u32;
            Part {
                text: format!("{v}"),
                set: vec![v],
            }
        }
        2 => {
            // Or của 2–3 literal
            let n = 2 + r.below(2) as usize;
            let vs: Vec<u32> = (0..n).map(|_| r.below(6) as u32).collect();
            Part {
                text: vs
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(" | "),
                set: vs,
            }
        }
        _ => {
            // Or có khoảng: lo..=hi | literal
            let lo = r.below(4) as u32;
            let hi = lo + r.below(3) as u32;
            let v = r.below(6) as u32;
            let mut set: Vec<u32> = (lo..=hi.min(5)).collect();
            set.push(v);
            Part {
                text: format!("{lo}..={hi} | {v}"),
                set,
            }
        }
    }
}

fn enum_part(r: &mut Rng) -> Part {
    match r.below(3) {
        0 => Part {
            text: "_".into(),
            set: (0..4).collect(),
        },
        1 => {
            let v = r.below(4) as usize;
            Part {
                text: ENUM[v].into(),
                set: vec![v as u32],
            }
        }
        _ => {
            let n = 2 + r.below(2) as usize;
            let vs: Vec<usize> = (0..n).map(|_| r.below(4) as usize).collect();
            Part {
                text: vs.iter().map(|v| ENUM[*v]).collect::<Vec<_>>().join(" | "),
                set: vs.iter().map(|v| *v as u32).collect(),
            }
        }
    }
}

/// miền của `o`: 0..=3 = Some(i), 4 = None
fn opt_part(r: &mut Rng) -> Part {
    match r.below(5) {
        0 => Part {
            text: "_".into(),
            set: (0..5).collect(),
        },
        1 => Part {
            text: "None".into(),
            set: vec![4],
        },
        2 => Part {
            text: "Some(_)".into(),
            set: (0..4).collect(),
        },
        3 => {
            // Some(a | b): Or LỒNG trong biến thể
            let (a, b) = (r.below(4) as u32, r.below(4) as u32);
            Part {
                text: format!("Some({a} | {b})"),
                set: vec![a, b],
            }
        }
        _ => {
            // None | Some(v): Or ở mức cao với alternative thuộc hai biến thể khác nhau
            let v = r.below(4) as u32;
            Part {
                text: format!("None | Some({v})"),
                set: vec![4, v],
            }
        }
    }
}

struct Arm {
    parts: [Part; 3],
    result: u64,
}

fn program(seed: u64) -> (String, u64) {
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let n = 2 + r.below(5) as usize;
    let arms: Vec<Arm> = (0..n)
        .map(|k| Arm {
            parts: [int_part(&mut r), enum_part(&mut r), opt_part(&mut r)],
            result: 10 + k as u64 * 7,
        })
        .collect();
    let mut src = String::from(
        "⊔ E { A, B, C, D }\nƒ mk_e(i: u32) → E ! ∅ { ⌥ i { 0 | 1 ↦ ⎇ i == 0 { A } ⇝ { B }, 2 ↦ C, _ ↦ D } }\nƒ mk_o(i: u32) → Option<u32> ! ∅ { ⎇ i == 4 { None } ⇝ { Some(i) } }\n",
    );
    src.push_str("ƒ f(x: u32, e: E, o: Option<u32>) → u64 ! ∅ {\n  ⌥ (x, e, o) {\n");
    for a in &arms {
        src.push_str(&format!(
            "    ({}, {}, {}) ↦ {}u64,\n",
            a.parts[0].text, a.parts[1].text, a.parts[2].text, a.result
        ));
    }
    src.push_str("    _ ↦ 1u64,\n  }\n}\n");
    src.push_str("ƒ main() → u64 ! ∅ {\n  μacc ≔ 0u64;\n  μx ≔ 0u32;\n  ⟳ x < 6u32 ↓ 6u32 - x {\n    μe ≔ 0u32;\n    ⟳ e < 4u32 ↓ 4u32 - e {\n      μo ≔ 0u32;\n      ⟳ o < 5u32 ↓ 5u32 - o {\n        acc ← acc *% 31u64 +% f(x, mk_e(e), mk_o(o));\n        o ← o + 1u32;\n      }\n      e ← e + 1u32;\n    }\n    x ← x + 1u32;\n  }\n  acc\n}\n");
    // mô hình
    let mut acc = 0u64;
    for x in 0..6u32 {
        for e in 0..4u32 {
            for o in 0..5u32 {
                let res = arms
                    .iter()
                    .find(|a| {
                        a.parts[0].set.contains(&x)
                            && a.parts[1].set.contains(&e)
                            && a.parts[2].set.contains(&o)
                    })
                    .map_or(1, |a| a.result);
                acc = acc.wrapping_mul(31).wrapping_add(res);
            }
        }
    }
    (src, acc)
}

fn parse_u64(v: Option<&str>) -> Option<u64> {
    let v = v?;
    let v = v.strip_prefix("Int(").unwrap_or(v);
    v.split(',').next()?.trim().parse().ok()
}

fn run(src: &str, tag: &str) -> Vec<(String, Option<u64>)> {
    let opts = Options {
        edition: 2,
        ..Options::default()
    };
    let mut res = check(
        &[SourceInput {
            name: "main.gt".to_string(),
            text: src.to_string(),
        }],
        &opts,
    )
    .expect("đầu vào hợp lệ");
    let errs: Vec<String> = res
        .report
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str().starts_with('E'))
        .map(|d| format!("{}: {}", d.code.as_str(), d.message))
        .collect();
    assert!(errs.is_empty(), "{tag}: gritc từ chối: {errs:?}\n{src}");
    #[allow(unused_mut)]
    let mut out = Vec::new();
    #[cfg(feature = "llvm")]
    {
        use gt_driver::pipeline::{attestation, mir_dump};
        let att = {
            let art = res.open_gate().expect("cổng mở");
            attestation(&res, &mir_dump(art))
        };
        let dir = std::env::temp_dir().join(format!("gt-orpat-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tạo thư mục");
        for (pname, prof) in [
            ("dev", gt_codegen_llvm::Opt::Dev),
            ("release", gt_codegen_llvm::Opt::Release),
            ("release-o2", gt_codegen_llvm::Opt::ReleaseO2),
        ] {
            let exe = dir.join(format!("{tag}-{pname}.exe"));
            gt_driver::native::build_exe_profile(
                res.open_gate().expect("cổng"),
                &exe,
                true,
                &att,
                prof,
            )
            .unwrap_or_else(|e| panic!("{tag}/{pname}: backend lỗi: {e}\n{src}"));
            let r =
                gt_driver::native::run_exe_timeout(&exe, Some(std::time::Duration::from_secs(30)));
            let _ = std::fs::remove_file(&exe);
            assert_eq!(
                r.end,
                RunEnd::Normal,
                "{tag}/{pname}: exe {:?}\n{src}",
                r.end
            );
            out.push((format!("llvm-{pname}"), parse_u64(r.value.as_deref())));
        }
    }
    let art = res.take_gated().expect("cổng mở");
    let r = gt_driver::runner::run_main(art);
    assert_eq!(
        r.end,
        RunEnd::Normal,
        "{tag}: thông dịch {:?}\n{src}",
        r.end
    );
    out.push(("interp".to_string(), parse_u64(r.value.as_deref())));
    out
}

#[test]
fn or_patterns_pick_the_same_arm_as_an_independent_model_on_every_input_and_backend() {
    let mut with_or = 0usize;
    for seed in 1..=40u64 {
        let (src, want) = program(seed);
        with_or += src.matches(" | ").count();
        for (backend, got) in run(&src, &format!("or{seed}")) {
            assert_eq!(
                got,
                Some(want),
                "#{seed} ({backend}): chọn nhánh sai, kỳ vọng {want}\n{src}"
            );
        }
    }
    // bộ sinh không rỗng: có nhiều or-pattern (gồm lồng trong `Some(…)`)
    assert!(with_or >= 150, "quá ít or-pattern: {with_or}");
}
