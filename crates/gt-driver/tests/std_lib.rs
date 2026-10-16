//! Bộ kiểm của gói `std` (lib/std, Q84/Q86 bước (i)): mọi impl/hàm của std được gọi từ chương trình sinh ngẫu nhiên QUA CÙNG CỔNG của gritc như mã người dùng
//! (`check` đầy đủ: M4–M9, Guard), rồi so BA NGUỒN: mô hình Rust độc lập (`Ord`/`Option`/`Result` của Rust) ≡ thông dịch MIR ≡ exe LLVM.
//! Mỗi chương trình gọi MỌI hàm std (mỗi ca = một hàm Grit, nhiều lần gọi); mặc định 32 chương trình ngẫu nhiên + 3 chương trình chéo biên (`edge`).
//! Hồ sơ: `GT_PROFILE=dev|release|release-o2` (mặc định dev). Kết quả: `STD_OUT=<đường dẫn tuyệt đối .jsonl>` ghi nối một dòng bảng bộ kiểm.
#![cfg(feature = "llvm")]
#[path = "support/stdgen.rs"]
mod stdgen;

use gt_driver::native::{build_exe_profile, run_exe_timeout};
use gt_driver::pipeline::{attestation, mir_dump};
use gt_driver::runner::run_main;
use gt_driver::{Options, RunEnd, check};
use gt_resolve::SourceInput;
use std::collections::BTreeMap;
use std::io::Write;
use std::time::Duration;

const STD: &str = include_str!("../../../lib/std/src/std.gt");
const STD_ORD: &str = include_str!("../../../lib/std/src/std_ord.gt");
const STD_ORD_INT: &str = include_str!("../../../lib/std/src/std_ord_int.gt");

fn profile() -> (gt_codegen_llvm::Opt, &'static str) {
    match std::env::var("GT_PROFILE").as_deref() {
        Ok("release") => (gt_codegen_llvm::Opt::Release, "release"),
        Ok("release-o2") => (gt_codegen_llvm::Opt::ReleaseO2, "release-o2"),
        _ => (gt_codegen_llvm::Opt::Dev, "dev"),
    }
}

fn inputs(main: &str) -> Vec<SourceInput> {
    [
        ("std.gt", STD),
        ("std_ord.gt", STD_ORD),
        ("std_ord_int.gt", STD_ORD_INT),
        ("main.gt", main),
    ]
    .iter()
    .map(|(n, t)| SourceInput {
        name: (*n).to_string(),
        text: (*t).to_string(),
    })
    .collect()
}

fn norm(v: &Option<String>) -> Option<String> {
    let s = v.as_deref()?;
    if let Some(inner) = s.strip_prefix("Int(") {
        return inner.split(',').next().map(str::to_string);
    }
    Some(s.to_string())
}

/// chạy một chương trình qua cổng + exe + thông dịch; trả (giá trị thông dịch, giá trị llvm) hoặc lỗi cổng
fn run3(
    main: &str,
    tag: &str,
    interp_too: bool,
) -> Result<(Option<String>, Option<String>), String> {
    let mut res = check(&inputs(main), &Options::default()).map_err(|e| e.to_string())?;
    if !res.verdict.allows_output() {
        let d: Vec<String> = res
            .report
            .diagnostics
            .iter()
            .take(6)
            .map(|d| format!("{} {} @{}", d.code.as_str(), d.message, d.span.start))
            .collect();
        return Err(format!("cổng ĐỎ/ICE: {d:?}"));
    }
    let att = {
        let art = res.open_gate().expect("cổng mở");
        attestation(&res, &mir_dump(art))
    };
    let dir = std::env::temp_dir().join(format!("gt-stdlib-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo thư mục");
    let exe = dir.join(format!("{tag}.exe"));
    let art = res.take_gated().expect("cổng");
    build_exe_profile(&art, &exe, true, &att, profile().0)
        .map_err(|e| format!("backend lỗi (mã {}): {e}", e.exit_code()))?;
    let llvm = run_exe_timeout(&exe, Some(Duration::from_secs(30)));
    let _ = std::fs::remove_file(&exe);
    if !interp_too {
        if llvm.end != RunEnd::Normal {
            return Err(format!("exe kết thúc bất thường: {:?}", llvm.end));
        }
        return Ok((llvm.value.clone(), llvm.value));
    }
    let interp = run_main(art);
    if interp.end != RunEnd::Normal || llvm.end != RunEnd::Normal {
        return Err(format!(
            "kết thúc bất thường: thông dịch {:?}, llvm {:?}",
            interp.end, llvm.end
        ));
    }
    Ok((norm(&interp.value), llvm.value))
}

/// khi một chương trình lệch mô hình: chạy RIÊNG từng ca để chỉ ra hàm std nào sai
fn diagnose(p: &stdgen::Prog, seed: u64) -> String {
    let mut out = String::new();
    for (i, (name, func, model, needs_alloc)) in p.funcs.iter().enumerate() {
        let mut src = p.helpers.clone();
        src.push_str(func);
        if *needs_alloc {
            src.push_str(&format!(
                "ƒ main() → u64 ! {{mem}} {{ alloc ≔ root_alloc(); c{i}(&alloc) }}\n"
            ));
        } else {
            src.push_str(&format!("ƒ main() → u64 ! ∅ {{ c{i}() }}\n"));
        }
        match run3(&src, &format!("d{seed}_{i}"), !*needs_alloc) {
            Ok((a, b)) => {
                let w = Some(model.to_string());
                if a != w || b != w {
                    out.push_str(&format!(
                        "  LỆCH {name}: mô hình {model}, thông dịch {a:?}, llvm {b:?}\n"
                    ));
                }
            }
            Err(e) => out.push_str(&format!("  LỖI {name}: {e}\n")),
        }
    }
    out
}

#[test]
fn std_ord_int_file_matches_generator() {
    // sửa tay lib/std/src/std_ord_int.gt ⇒ ĐỎ: tệp phải bằng ĐÚNG đầu ra của tools/gen_std.py (tất định)
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let o = std::process::Command::new("python")
        .arg(root.join("tools/gen_std.py"))
        .arg("--stdout")
        .output()
        .expect("chạy python tools/gen_std.py");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let again = std::process::Command::new("python")
        .arg(root.join("tools/gen_std.py"))
        .arg("--stdout")
        .output()
        .expect("chạy lần hai");
    assert_eq!(o.stdout, again.stdout, "bộ sinh không tất định");
    assert_eq!(
        String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n"),
        STD_ORD_INT.replace("\r\n", "\n"),
        "lib/std/src/std_ord_int.gt khác đầu ra của tools/gen_std.py: sửa script rồi `python tools/gen_std.py`"
    );
}

#[test]
fn every_std_function_matches_the_independent_rust_model_in_three_sources() {
    let random: u64 = std::env::var("STD_RANDOM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(32);
    let edge_n: u64 = 3;
    let mut table: BTreeMap<&'static str, (u64, u64, u64)> = BTreeMap::new(); // tên → (số chương trình, số lần gọi, số lần gọi biên)
    let mut fails: Vec<String> = Vec::new();
    for k in 0..random + edge_n {
        let edge = k >= random;
        let seed = if edge { 9000 + k } else { 1 + k };
        // chương trình Arena (Text) chỉ so mô hình ≡ exe: thông dịch không mô phỏng Arena
        for mem_only in [false, true] {
            let p = stdgen::program(seed, edge, mem_only);
            match run3(&p.src, &format!("p{seed}{mem_only}"), !mem_only) {
                Ok((a, b)) => {
                    let w = Some(p.expect.to_string());
                    if a == w && b == w {
                        for (name, calls, ec) in &p.cases {
                            let e = table.entry(name).or_default();
                            e.0 += 1;
                            e.1 += calls;
                            e.2 += ec;
                        }
                    } else {
                        fails.push(format!(
                            "#{seed}{}: mô hình {} ≠ thông dịch {a:?} / llvm {b:?}\n{}",
                            if edge { " (edge)" } else { "" },
                            p.expect,
                            diagnose(&p, seed)
                        ));
                    }
                }
                Err(e) => fails.push(format!("#{seed}: {e}\n{}", p.src)),
            }
        }
    }
    println!(
        "{:<34} {:>9} {:>8} {:>8}",
        "hàm/impl của std", "chương trình", "lần gọi", "lần biên"
    );
    for (n, (pr, c, e)) in &table {
        println!("{n:<34} {pr:>9} {c:>8} {e:>8}");
    }
    if let Some(out) = std::env::var_os("STD_OUT") {
        let rows: Vec<String> = table
            .iter()
            .map(|(n, (pr, c, e))| format!("[\"{n}\",{pr},{c},{e}]"))
            .collect();
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(out)
            .expect("mở tệp kết quả");
        writeln!(
            f,
            "{{\"suite\":\"std\",\"profile\":\"{}\",\"random\":{random},\"edge\":{edge_n},\"fails\":{},\"rows\":[{}]}}",
            profile().1,
            fails.len(),
            rows.join(",")
        )
        .expect("ghi");
    }
    assert!(fails.is_empty(), "{}", fails.join("\n=====\n"));
    for (n, (pr, _, _)) in &table {
        assert!(*pr >= 30, "{n}: chỉ {pr} chương trình (< 30)");
    }
}
