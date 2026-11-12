//! R2b (lệnh Giang 2026-10-17): hợp đồng `⊳`/`⊲` khi hàm được dùng làm GIÁ TRỊ rồi gọi qua con trỏ. Nỗi lo: bộ giải M7 xoá check trong thân hàm dựa trên `⊳` mà đường gián tiếp
//! không kiểm `⊳` ⇒ check bị xoá nhầm (false negative). ĐO (không suy ra): với hàm bị dùng làm giá trị, `⊳` KHÔNG được dùng làm giả thiết xoá check (KNOWN_LIMITATIONS) ⇒ kiểm tra trong thân
//! còn nguyên và chương trình abort ĐÚNG loại cả ở thông dịch lẫn exe LLVM, kể cả khi hàm VỪA được gọi trực tiếp VỪA bị lấy làm giá trị. Gọi TRỰC TIẾP với đối số vi phạm: E0901 tĩnh.
//! Test khoá hành vi để mọi thay đổi bộ giải làm ⊳ "rò" sang đường gián tiếp bị bắt.
#![cfg(feature = "llvm")]
use gt_driver::native::{build_exe_profile, run_exe_timeout};
use gt_driver::pipeline::{attestation, mir_dump};
use gt_driver::runner::run_main;
use gt_driver::{Options, RunEnd, check};
use gt_oblig::driver::Checks;
use gt_resolve::SourceInput;
use std::time::Duration;

const GET: &str = "ƒ get(i: usize) → i32 ! ∅ ⊳ i < 4 {\n  a ≔ [10, 20, 30, 40];\n  a[i]\n}\n";
const GET_NO_PRE: &str = "ƒ get(i: usize) → i32 ! ∅ {\n  a ≔ [10, 20, 30, 40];\n  a[i]\n}\n";
const DV: &str = "ƒ dv(a: u32, b: u32) → u32 ! ∅ ⊳ b ≠ 0u32 { a / b }\n";
const SHL: &str = "ƒ f(n: u32) → u32 ! ∅ ⊳ n < 32u32 { shl(1u32, n) }\n";

#[derive(Debug, PartialEq)]
enum Out {
    Red(&'static str),
    Abort(&'static str),
    Value(&'static str),
}

fn run_both(
    src: &str,
    opt: gt_codegen_llvm::Opt,
    checks: Checks,
) -> (Option<String>, RunEnd, Option<String>, RunEnd, Vec<String>) {
    let inputs = [SourceInput {
        name: "main.gt".to_string(),
        text: src.to_string(),
    }];
    let mut res = check(
        &inputs,
        &Options {
            checks,
            ..Options::default()
        },
    )
    .expect("check");
    let codes: Vec<String> = res
        .report
        .diagnostics
        .iter()
        .filter(|d| d.level == gt_diag::Level::Error)
        .map(|d| d.code.as_str().to_string())
        .collect();
    if !res.verdict.allows_output() {
        return (None, RunEnd::Normal, None, RunEnd::Normal, codes);
    }
    let dir = std::env::temp_dir().join(format!("gt-cvv-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo");
    let exe = dir.join(format!("p{}.exe", src.len()));
    {
        let art = res.open_gate().expect("cổng");
        let att = attestation(&res, &mir_dump(art));
        build_exe_profile(art, &exe, true, &att, opt).expect("dựng exe");
    }
    let llvm = run_exe_timeout(&exe, Some(Duration::from_secs(20)));
    let _ = std::fs::remove_file(&exe);
    let interp = run_main(res.take_gated().expect("sản phẩm"));
    let norm = |v: &Option<String>| {
        v.as_deref().map(|s| {
            s.strip_prefix("Int(")
                .and_then(|i| i.split(',').next())
                .unwrap_or(s)
                .to_string()
        })
    };
    (norm(&interp.value), interp.end, llvm.value, llvm.end, codes)
}

fn expect(name: &str, src: &str, want: &Out) {
    // `--checks=proven` XOÁ check đã chứng minh (thí nghiệm): đây mới là chế độ mà `⊳` làm giả thiết có thể xoá nhầm check ở đường gián tiếp; hồ sơ release từ chối proven.
    let modes = [
        (gt_codegen_llvm::Opt::Dev, Checks::All),
        (gt_codegen_llvm::Opt::ReleaseO2, Checks::All),
        (gt_codegen_llvm::Opt::Dev, Checks::Proven),
    ];
    for (opt, checks) in modes {
        let (iv, ie, lv, le, codes) = run_both(src, opt, checks);
        match want {
            Out::Red(c) => {
                assert!(
                    codes.iter().any(|x| x == c) && ie == RunEnd::Normal,
                    "{name}: kỳ vọng ĐỎ {c}, có {codes:?}"
                );
                return;
            }
            Out::Abort(k) => {
                assert!(
                    codes.is_empty(),
                    "{name} {opt:?}/{checks:?}: kỳ vọng xanh, có {codes:?}"
                );
                assert_eq!(ie, RunEnd::Aborted((*k).to_string()), "{name}: thông dịch");
                assert_eq!(
                    le,
                    RunEnd::Aborted((*k).to_string()),
                    "{name}: LLVM {opt:?}/{checks:?}"
                );
            }
            Out::Value(v) => {
                assert!(codes.is_empty(), "{name}: kỳ vọng xanh, có {codes:?}");
                assert_eq!(
                    (ie, iv.as_deref()),
                    (RunEnd::Normal, Some(*v)),
                    "{name}: thông dịch"
                );
                assert_eq!(
                    (le, lv.as_deref()),
                    (RunEnd::Normal, Some(*v)),
                    "{name}: LLVM {opt:?}/{checks:?}"
                );
            }
        }
    }
}

#[test]
fn preconditioned_functions_are_rejected_when_used_as_values_fb_c6() {
    // Trực tiếp, đối số vi phạm: E0901 tĩnh.
    expect(
        "direct",
        &format!("{GET}ƒ main() → i32 ! ∅ {{ get(9usize) }}"),
        &Out::Red("E0901"),
    );
    // FB-C6 fail-closed: hàm có `⊳` không thoát qua biến hàm, tham số hay khi vừa gọi trực tiếp vừa lấy làm giá trị.
    let escaped = [
        (
            "local",
            format!("{GET}ƒ main() → i32 ! ∅ {{ f ≔ get; f(9usize) }}"),
        ),
        (
            "mixed",
            format!("{GET}ƒ main() → i32 ! ∅ {{ x ≔ get(1usize); f ≔ get; x +% f(9usize) }}"),
        ),
        (
            "parameter",
            format!(
                "{GET}ƒ call_it(f: ƒ(usize) → i32 ! ∅, i: usize) → i32 ! ∅ {{ f(i) }}\nƒ main() → i32 ! ∅ {{ call_it(get, 9usize) }}"
            ),
        ),
        (
            "division",
            format!("{DV}ƒ main() → u32 ! ∅ {{ f ≔ dv; f(1u32, 0u32) }}"),
        ),
        (
            "shift",
            format!("{SHL}ƒ main() → u32 ! ∅ {{ g ≔ f; g(40u32) }}"),
        ),
        (
            "valid_argument_still_rejected",
            format!("{GET}ƒ main() → i32 ! ∅ {{ f ≔ get; f(2usize) }}"),
        ),
    ];
    for (name, src) in escaped {
        expect(name, &src, &Out::Red("E0901"));
    }
}

#[test]
fn bounds_checks_remain_on_precondition_free_indirect_calls() {
    // Dù không có `⊳` để từ chối thoát giá trị, kiểm biên trong thân vẫn phải còn ở M7.
    expect(
        "value",
        &format!("{GET_NO_PRE}ƒ main() → i32 ! ∅ {{ f ≔ get; f(9usize) }}"),
        &Out::Abort("index_out_of_bounds"),
    );
    expect(
        "valid-value",
        &format!("{GET_NO_PRE}ƒ main() → i32 ! ∅ {{ f ≔ get; f(2usize) }}"),
        &Out::Value("30"),
    );
}

#[test]
fn postcondition_that_can_never_hold_is_still_rejected_through_a_value() {
    let bad = "ƒ bad(x: u32) → u32 ! ∅ ⊲ result > x { x }\n";
    expect(
        "post-direct",
        &format!("{bad}ƒ main() → u32 ! ∅ {{ bad(5u32) }}"),
        &Out::Red("E0901"),
    );
    expect(
        "post-value",
        &format!("{bad}ƒ main() → u32 ! ∅ {{\n  f ≔ bad;\n  f(5u32)\n}}"),
        &Out::Red("E0901"),
    );
}
