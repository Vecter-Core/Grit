//! Kiểm thử codegen bằng SO SÁNH (M10 sau duyệt, bước 1): chương trình số nguyên ngẫu nhiên của bộ sinh oracle M7 (`obgen`) được hạ ra EXE
//! THẬT và chạy; kết quả + loại abort phải khớp thông dịch MIR (sau M7, `--checks=all`). Chương trình/đầu vào chứa thứ chưa hạ được
//! (exit 3) bị bỏ khỏi so sánh VÀ ĐẾM; ĐỎ/ICE cũng đếm riêng. Bất đồng nào là bug (codegen hoặc thông dịch) ⇒ thêm seed vào `PERMANENT`.
//!
//! Chạy theo lô (kết quả ghi nối vào tệp sau mỗi lô):
//!   `GT_DIFF_START=1000 GT_DIFF_COUNT=25 GT_DIFF_OUT=target/tmp/diff.jsonl cargo test -p gt-driver --test llvm_diff -- --ignored batch`
#![cfg(feature = "llvm")]
#[path = "../../gt-oblig/tests/support/obgen.rs"]
mod obgen;
#[path = "../../gt-mir/tests/support/progen.rs"]
mod progen;

use gt_driver::native::{build_exe, run_exe_timeout};
use gt_driver::pipeline::attestation;
use gt_driver::runner::run_main;
use gt_driver::{Options, RunEnd, check};
use gt_resolve::SourceInput;
use std::io::Write;
use std::time::Duration;

/// Seed đã từng lộ bất đồng thật (thêm vào đây khi sửa bug; chạy mãi mãi).
const PERMANENT: &[u64] = &[];

/// Số đầu vào cho mỗi chương trình (mỗi đầu vào = một exe riêng vì tham số nằm trong `main`).
const INPUTS_PER_PROGRAM: usize = 4;

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

/// Literal nguyên của kiểu `ty` (MIN của kiểu có dấu không viết trực tiếp được: `-128i8` là `-(128i8)` ngoài phạm vi).
fn lit(ty: obgen::Ity, v: i128) -> String {
    if ty.signed && v == ty.min() {
        format!("(-{}{} -% 1{})", ty.max(), ty.name, ty.name)
    } else {
        ty.lit(v)
    }
}

#[derive(Default, Debug, Clone)]
struct Tally {
    programs: u64,
    runs: u64,
    equal: u64,
    mismatch: u64,
    interp_limit: u64,
    llvm_timeout: u64,
    unsupported: u64,
    red_or_ice: u64,
    /// chương trình bị cổng ĐỎ/ICE ngay ở đầu vào đầu tiên ⇒ bỏ cả chương trình
    red_programs: u64,
    normal: u64,
    aborts: std::collections::BTreeMap<String, u64>,
    /// lý do bỏ khỏi so sánh: mã chẩn đoán đầu tiên (ĐỎ/ICE) hoặc thông báo `Unsupported`
    skipped: std::collections::BTreeMap<String, u64>,
    detail: Vec<String>,
}

fn norm_interp(v: &Option<String>) -> Option<String> {
    let s = v.as_deref()?;
    if s == "Unit" {
        return None;
    }
    if let Some(inner) = s.strip_prefix("Int(") {
        return inner.split(',').next().map(str::to_string);
    }
    Some(s.to_string())
}

/// `true` nếu chương trình qua cổng (được so sánh), `false` nếu bị ĐỎ/ICE ngay từ đầu vào đầu tiên.
fn one(seed: u64, t: &mut Tally, dir: &std::path::Path) -> bool {
    let mut g = obgen::OGen::new(seed);
    let p = g.program();
    t.programs += 1;
    let mut rng = progen::Rng(seed ^ 0x5151_7777_ABCD_0123 | 1);
    for (k, input) in inputs(&mut rng, &p.entry_ranges, INPUTS_PER_PROGRAM)
        .into_iter()
        .enumerate()
    {
        t.runs += 1;
        let args: Vec<String> = input.iter().map(|v| lit(p.ty, *v)).collect();
        let src = format!(
            "{}ƒ main() → {} ! ∅ {{ entry({}) }}\n",
            p.src,
            p.ty.name,
            args.join(", ")
        );
        let inputs = [SourceInput {
            name: "main.gt".to_string(),
            text: src.clone(),
        }];
        let mut res = match check(&inputs, &Options::default()) {
            Ok(r) => r,
            Err(e) => {
                t.red_or_ice += 1;
                t.detail
                    .push(format!("#{seed} {input:?}: đầu vào không hợp lệ: {e}"));
                continue;
            }
        };
        if !res.verdict.allows_output() {
            t.red_or_ice += 1;
            let code = res
                .report
                .diagnostics
                .iter()
                .find(|d| d.level == gt_diag::Level::Error)
                .map_or("ICE-không-mã".to_string(), |d| {
                    d.code.as_str().to_string()
                });
            *t.skipped.entry(format!("ĐỎ {code}")).or_default() += 1;
            if code == "E9901" && t.detail.len() < 3 {
                let msg = res
                    .report
                    .diagnostics
                    .iter()
                    .find(|d| d.code.as_str() == "E9901")
                    .map(|d| d.message.clone())
                    .unwrap_or_default();
                t.detail.push(format!(
                    "#{seed} ICE: {msg}
{src}"
                ));
            }
            if k == 0 {
                t.red_programs += 1;
                if code == "E9901" {
                    // ICE là bug của gritc, KHÔNG phải "chương trình không hợp lệ"
                    t.mismatch += 1;
                    t.detail.push(format!(
                        "#{seed}: gritc báo ICE (E9901) trên chương trình hợp lệ
{src}"
                    ));
                }
                return false;
            }
            continue;
        }
        let exe = dir.join(format!("p{seed}.exe"));
        let att = {
            let art = res.open_gate().expect("cổng mở");
            let att = attestation(&res, &format!("{:#?}", art.obl.bodies));
            match build_exe(art, &exe, true, &att) {
                Ok(()) => {}
                Err(e) => {
                    if e.exit_code() == 3 {
                        t.unsupported += 1;
                        let msg: String = e.to_string().chars().take(70).collect();
                        *t.skipped.entry(format!("chưa hỗ trợ: {msg}")).or_default() += 1;
                    } else {
                        t.mismatch += 1;
                        t.detail.push(format!(
                            "#{seed} {input:?}: backend llvm lỗi (mã {}): {e}\n{src}",
                            e.exit_code()
                        ));
                    }
                    continue;
                }
            }
            att
        };
        let _ = att;
        let llvm = run_exe_timeout(&exe, Some(Duration::from_secs(20)));
        let _ = std::fs::remove_file(&exe);
        let interp = run_main(res.take_gated().expect("sản phẩm"));
        if interp.end == RunEnd::Limit {
            t.interp_limit += 1;
            continue;
        }
        if llvm.end == RunEnd::Limit {
            t.llvm_timeout += 1;
            t.detail.push(format!(
                "#{seed} {input:?}: exe quá hạn 20 s, thông dịch xong: {:?}",
                interp.end
            ));
            continue;
        }
        let same = match (&interp.end, &llvm.end) {
            (RunEnd::Normal, RunEnd::Normal) => norm_interp(&interp.value) == llvm.value,
            (RunEnd::Aborted(a), RunEnd::Aborted(b)) => a == b,
            _ => false,
        };
        if same {
            t.equal += 1;
            match &llvm.end {
                RunEnd::Aborted(k) => *t.aborts.entry(k.clone()).or_default() += 1,
                _ => t.normal += 1,
            }
        } else {
            t.mismatch += 1;
            t.detail.push(format!(
                "#{seed} {input:?}: thông dịch {:?}/{:?} ≠ llvm {:?}/{:?}\n{src}",
                interp.end, interp.value, llvm.end, llvm.value
            ));
        }
    }
    true
}

fn run_seeds(seeds: &[u64]) -> Tally {
    let dir = std::env::temp_dir().join(format!("gt-diff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo thư mục tạm");
    let mut t = Tally::default();
    for s in seeds {
        one(*s, &mut t, &dir);
    }
    let _ = std::fs::remove_dir_all(&dir);
    t
}

/// Lấy seed từ `start` cho tới khi có `want` chương trình QUA CỔNG (so sánh được). Trả (kết quả, seed kế tiếp, số seed đã xét).
fn run_until(start: u64, want: u64) -> (Tally, u64) {
    let dir = std::env::temp_dir().join(format!("gt-diff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo thư mục tạm");
    let mut t = Tally::default();
    let (mut seed, mut green) = (start, 0);
    while green < want && seed < start + want * 200 {
        if one(seed, &mut t, &dir) {
            green += 1;
        }
        seed += 1;
    }
    let _ = std::fs::remove_dir_all(&dir);
    (t, seed)
}

fn summary(t: &Tally) -> String {
    format!(
        "{{\"programs\":{},\"runs\":{},\"equal\":{},\"mismatch\":{},\"normal\":{},\"aborts\":{:?},\"interp_limit\":{},\"llvm_timeout\":{},\"unsupported\":{},\"red_or_ice\":{},\"red_programs\":{},\"skipped\":{:?}}}",
        t.programs,
        t.runs,
        t.equal,
        t.mismatch,
        t.normal,
        t.aborts,
        t.interp_limit,
        t.llvm_timeout,
        t.unsupported,
        t.red_or_ice,
        t.red_programs,
        t.skipped
    )
}

/// Lô ngẫu nhiên (chạy tay, ghi nối kết quả ra tệp). Mặc định bỏ qua.
#[test]
#[ignore = "chạy theo lô bằng GT_DIFF_START/COUNT/OUT"]
fn batch() {
    let start: u64 = std::env::var("GT_DIFF_START")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    let count: u64 = std::env::var("GT_DIFF_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(25);
    let (t, next) = run_until(start, count);
    let line = format!(
        "{{\"start\":{start},\"green_wanted\":{count},\"next_seed\":{next},\"result\":{}}}",
        summary(&t)
    );
    println!("{line}");
    if let Some(out) = std::env::var_os("GT_DIFF_OUT") {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(out)
            .expect("mở tệp kết quả");
        writeln!(f, "{line}").expect("ghi");
        for d in &t.detail {
            writeln!(f, "  DETAIL {d}").expect("ghi");
        }
    }
    if std::env::var_os("GT_DIFF_ALLOW_DETAIL").is_none() {
        assert_eq!(t.mismatch, 0, "bất đồng: {:#?}", t.detail);
    }
}

/// Seed đã lộ bất đồng thật + một lô nhỏ cố định: chạy trong cổng đầy đủ (rẻ).
#[test]
fn permanent_and_small_fixed_batch() {
    let mut seeds: Vec<u64> = PERMANENT.to_vec();
    seeds.extend(1..=8);
    let t = run_seeds(&seeds);
    println!("{}", summary(&t));
    assert_eq!(t.mismatch, 0, "bất đồng: {:#?}", t.detail);
    assert!(
        t.equal >= 10,
        "quá ít phép so sánh có nghĩa ({}): {}",
        t.equal,
        summary(&t)
    );
}
