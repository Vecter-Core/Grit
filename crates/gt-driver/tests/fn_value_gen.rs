//! BỘ SINH HÀM LÀM GIÁ TRỊ (luật bộ sinh, lệnh Giang 2026-10-17 mục 3): S-E, FB-13 và FB-15 đều tìm bằng ĐỌC MÃ, không do bộ sinh ngẫu nhiên nào. Bộ sinh này sinh chương trình
//! dùng hàm làm giá trị theo 8 loại — hàm thường, gọi qua chuỗi biến, hàm kết hợp `&self`, hàm kết hợp `self`, hàm generic, truyền qua tham số kiểu hàm, intrinsic (E0313),
//! phương thức Atomic (E0503) — với hiệu ứng ngẫu nhiên của callee/caller. Oracle ĐỘC LẬP (mô hình Rust, không đọc kết quả của gritc): quy tắc "hiệu ứng callee ⊆ hiệu ứng caller" và
//! "intrinsic không là giá trị" ⇒ mã chẩn đoán kỳ vọng CHÍNH XÁC; chương trình xanh ⇒ giá trị mô hình == thông dịch == exe LLVM.
//! Tắt bản sửa FB-13 / FB-15 / S-E / E0313 thì bộ sinh phải ĐỎ (kiểm bằng hoàn nguyên, ghi ở PROGRESS.md).
//! `GT_FNGEN_N` = số chương trình mỗi loại (mặc định 32). Bảng độ phủ in ra bằng `--nocapture`.
#![cfg(feature = "llvm")]
use gt_driver::native::{build_exe_profile, run_exe_timeout};
use gt_driver::pipeline::{attestation, mir_dump};
use gt_driver::runner::run_main;
use gt_driver::{Options, RunEnd, check};
use gt_resolve::SourceInput;
use std::collections::BTreeMap;
use std::time::Duration;

fn profile() -> (gt_codegen_llvm::Opt, &'static str) {
    match std::env::var("GT_PROFILE").as_deref() {
        Ok("release") => (gt_codegen_llvm::Opt::Release, "release"),
        Ok("release-o2") => (gt_codegen_llvm::Opt::ReleaseO2, "release-o2"),
        _ => (gt_codegen_llvm::Opt::Dev, "dev"),
    }
}

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
    fn u32(&mut self) -> u32 {
        let edges = [0u32, 1, 2, u32::MAX, u32::MAX - 1, 1 << 31];
        if self.below(3) == 0 {
            edges[self.below(edges.len() as u64) as usize]
        } else {
            self.next() as u32
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
enum Kind {
    Plain,
    VarChain,
    AssocRef,
    AssocVal,
    Generic,
    ViaParam,
    Intrinsic,
    Atomic,
}
const KINDS: [Kind; 8] = [
    Kind::Plain,
    Kind::VarChain,
    Kind::AssocRef,
    Kind::AssocVal,
    Kind::Generic,
    Kind::ViaParam,
    Kind::Intrinsic,
    Kind::Atomic,
];

#[derive(Debug)]
enum Expect {
    Green(u32),
    Red(&'static str),
}

fn eff(e: bool) -> &'static str {
    if e { "{io}" } else { "∅" }
}

/// Chương trình + kết quả mô hình: callee có io mà caller không khai báo ⇒ E0801; ngược lại giá trị.
fn gen_program(kind: Kind, r: &mut Rng) -> (String, Expect) {
    let (ce, me) = (r.below(2) == 0, r.below(2) == 0);
    let (a, c1) = (r.u32(), r.u32());
    let covered = !ce || me;
    let val = |v: u32| {
        if covered {
            Expect::Green(v)
        } else {
            Expect::Red("E0801")
        }
    };
    let (ces, mes) = (eff(ce), eff(me));
    match kind {
        Kind::Plain => (
            format!(
                "ƒ f1(x: u32) → u32 ! {ces} {{ x +% {c1}u32 }}\nƒ main() → u32 ! {mes} {{\n  g ≔ f1;\n  g({a}u32)\n}}\n"
            ),
            val(a.wrapping_add(c1)),
        ),
        Kind::VarChain => (
            format!(
                "ƒ f1(x: u32) → u32 ! {ces} {{ x +% {c1}u32 }}\nƒ main() → u32 ! {mes} {{\n  g ≔ f1;\n  h ≔ g;\n  k ≔ h;\n  k({a}u32)\n}}\n"
            ),
            val(a.wrapping_add(c1)),
        ),
        Kind::AssocRef => (
            format!(
                "▢ B {{ v: u32 }}\n⊐ B {{\n  ƒ get(&self) → u32 ! {ces} {{ self.v +% {c1}u32 }}\n}}\nƒ main() → u32 ! {mes} {{\n  b ≔ B {{ v: {a}u32 }};\n  g ≔ B::get;\n  g(&b)\n}}\n"
            ),
            val(a.wrapping_add(c1)),
        ),
        Kind::AssocVal => (
            format!(
                "▢ B {{ v: u32 }}\n⊐ B {{\n  ƒ take(self) → u32 ! {ces} {{ self.v +% {c1}u32 }}\n}}\nƒ main() → u32 ! {mes} {{\n  b ≔ B {{ v: {a}u32 }};\n  g ≔ B::take;\n  g(b)\n}}\n"
            ),
            val(a.wrapping_add(c1)),
        ),
        Kind::Generic => (
            format!(
                "ƒ idg<T>(x: T) → T ! {ces} {{ x }}\nƒ main() → u32 ! {mes} {{\n  g: ƒ(u32) → u32 ! {ces} ≔ idg;\n  g({a}u32)\n}}\n"
            ),
            val(a),
        ),
        Kind::ViaParam => (
            format!(
                "ƒ f1(x: u32) → u32 ! {ces} {{ x +% {c1}u32 }}\nƒ apply(f: ƒ(u32) → u32 ! {ces}, x: u32) → u32 ! {ces} {{ f(x) }}\nƒ main() → u32 ! {mes} {{ apply(f1, {a}u32) }}\n"
            ),
            val(a.wrapping_add(c1)),
        ),
        Kind::Intrinsic => {
            let two = ["shl", "shr", "band", "bor", "bxor", "rotr", "rotl"];
            if r.below(8) == 0 {
                (
                    format!(
                        "ƒ main() → u32 ! ∅ {{\n  g: ƒ(u32) → u32 ! ∅ ≔ bnot;\n  g({a}u32)\n}}\n"
                    ),
                    Expect::Red("E0313"),
                )
            } else {
                let n = two[r.below(two.len() as u64) as usize];
                (
                    format!(
                        "ƒ main() → u32 ! ∅ {{\n  g: ƒ(u32, u32) → u32 ! ∅ ≔ {n};\n  g({a}u32, {}u32)\n}}\n",
                        r.below(40)
                    ),
                    Expect::Red("E0313"),
                )
            }
        }
        Kind::Atomic => {
            let ords = ["Relaxed", "Acquire", "Release", "AcqRel", "SeqCst"];
            let o = ords[r.below(5) as usize];
            (
                format!(
                    "ƒ main() → u32 ! ∅ {{\n  a ≔ Atomic::new({a}u32);\n  g: ƒ(&Atomic<u32>, AtomicOrdering) → u32 ! ∅ ≔ Atomic::load;\n  g(&a, AtomicOrdering::{o})\n}}\n"
                ),
                Expect::Red("E0503"),
            )
        }
    }
}

fn norm_interp(v: &Option<String>) -> Option<String> {
    let s = v.as_deref()?;
    if let Some(inner) = s.strip_prefix("Int(") {
        return inner.split(',').next().map(str::to_string);
    }
    Some(s.to_string())
}

#[derive(Default, Debug)]
struct Cov {
    programs: usize,
    green: usize,
    red_expected: usize,
}

#[test]
fn function_values_agree_with_the_independent_model() {
    let n: usize = std::env::var("GT_FNGEN_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(32);
    let dir = std::env::temp_dir().join(format!("gt-fngen-{}", std::process::id()));
    let (opt, profile_name) = profile();
    std::fs::create_dir_all(&dir).expect("tạo");
    let mut cov: BTreeMap<Kind, Cov> = BTreeMap::new();
    let mut bad: Vec<String> = Vec::new();
    for (ki, kind) in KINDS.iter().enumerate() {
        let mut r = Rng(0x9E37_79B9_7F4A_7C15 ^ ((ki as u64 + 1) << 40) | 1);
        for i in 0..n {
            let (src, exp) = gen_program(*kind, &mut r);
            let c = cov.entry(*kind).or_default();
            c.programs += 1;
            let inputs = [SourceInput {
                name: "main.gt".to_string(),
                text: src.clone(),
            }];
            let mut res = check(&inputs, &Options::default()).expect("check");
            let codes: Vec<String> = res
                .report
                .diagnostics
                .iter()
                .filter(|d| d.level == gt_diag::Level::Error)
                .map(|d| d.code.as_str().to_string())
                .collect();
            match exp {
                Expect::Red(code) => {
                    c.red_expected += 1;
                    if res.verdict.allows_output()
                        || codes.is_empty()
                        || codes.iter().any(|x| x != code)
                    {
                        bad.push(format!(
                            "{kind:?}#{i}: kỳ vọng chỉ {code}, có {codes:?} (cổng mở: {})\n{src}",
                            res.verdict.allows_output()
                        ));
                    }
                }
                Expect::Green(want) => {
                    c.green += 1;
                    if !res.verdict.allows_output() {
                        bad.push(format!(
                            "{kind:?}#{i}: kỳ vọng XANH {want}, có {codes:?}\n{src}"
                        ));
                        continue;
                    }
                    let exe = dir.join(format!("p{ki}_{i}.exe"));
                    {
                        let art = res.open_gate().expect("cổng");
                        let att = attestation(&res, &mir_dump(art));
                        if let Err(e) = build_exe_profile(art, &exe, true, &att, opt) {
                            bad.push(format!("{kind:?}#{i}: LLVM từ chối: {e}\n{src}"));
                            continue;
                        }
                    }
                    let llvm = run_exe_timeout(&exe, Some(Duration::from_secs(20)));
                    let _ = std::fs::remove_file(&exe);
                    let interp = run_main(res.take_gated().expect("sản phẩm"));
                    let want_s = Some(want.to_string());
                    if interp.end != RunEnd::Normal || norm_interp(&interp.value) != want_s {
                        bad.push(format!(
                            "{kind:?}#{i}: thông dịch {:?}/{:?} ≠ mô hình {want}\n{src}",
                            interp.end, interp.value
                        ));
                    }
                    if llvm.end != RunEnd::Normal || llvm.value != want_s {
                        bad.push(format!(
                            "{kind:?}#{i}: LLVM {:?}/{:?} ≠ mô hình {want}\n{src}",
                            llvm.end, llvm.value
                        ));
                    }
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    println!("fn_value_gen: profile={profile_name} n/loại={n}");
    for (k, c) in &cov {
        println!(
            "  {k:?}: {} chương trình = {} xanh (mô hình==thông dịch==LLVM) + {} đỏ kỳ vọng (mã đúng)",
            c.programs, c.green, c.red_expected
        );
    }
    for k in KINDS {
        assert!(cov[&k].programs >= 30.min(n), "độ phủ loại {k:?}");
    }
    assert!(
        bad.is_empty(),
        "{} bất đồng:\n{}",
        bad.len(),
        bad.iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n---\n")
    );
}
