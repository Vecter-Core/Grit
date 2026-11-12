//! ADR-036 §6c (Q77): bẫy abort cho MỌI loại abort mà chương trình Grit chạm tới được, và bẫy số thực (FMA, gập hằng, đổi thứ tự cộng, NaN, ±0, dưới chuẩn).
//! Mỗi chương trình chạy ở dev VÀ release (qua tiến trình `gritc run --profile=…`); đòi cùng mã thoát/loại abort/giá trị, và đúng KỲ VỌNG ĐỘC LẬP
//! (loại abort cố định; số thực do Rust tính theo IEEE từng phép, KHÔNG hợp nhất).
#![cfg(feature = "llvm")]
use std::process::Command;

/// hồ sơ release dưới kiểm: `GT_PROFILE` (`release` hoặc `release-o2`; mặc định `release`)
fn rel() -> String {
    std::env::var("GT_PROFILE").unwrap_or_else(|_| "release".to_string())
}

fn run(src: &str, profile: &str, env: &[(&str, &str)]) -> (i32, bool, String, String) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("gt-rtrap-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("tạo");
    let f = dir.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let mut c = Command::new(env!("CARGO_BIN_EXE_gritc"));
    for (k, v) in env {
        c.env(k, v);
    }
    let o = c
        .args([
            "run",
            "--json",
            "--backend=llvm",
            &format!("--profile={profile}"),
        ])
        .arg(&f)
        .output()
        .expect("chạy gritc");
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let line = text
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .unwrap_or("");
    let v: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|e| {
        panic!(
            "JSON hỏng ({e}): {text}\nstderr: {}",
            String::from_utf8_lossy(&o.stderr)
        )
    });
    let _ = std::fs::remove_dir_all(&dir);
    let r = &v["run"];
    (
        o.status.code().unwrap_or(-1),
        r["aborted"].as_bool().unwrap_or(false),
        r["abort_kind"].as_str().unwrap_or("").to_string(),
        r["value"].as_str().unwrap_or("").to_string(),
    )
}

/// (tên loại abort, chương trình, môi trường)
type Case = (
    &'static str,
    &'static str,
    Vec<(&'static str, &'static str)>,
);

fn abort_cases() -> Vec<Case> {
    vec![
        (
            "overflow",
            "ƒ f(a: u8) → u8 ! ∅ { a + 100u8 }\nƒ main() → u8 ! ∅ { f(200u8) }\n",
            vec![],
        ),
        (
            "overflow",
            "ƒ f(a: i64, b: i64) → i64 ! ∅ { a * b }\nƒ main() → i64 ! ∅ { f(4611686018427387904i64, 4i64) }\n",
            vec![],
        ),
        (
            "overflow",
            "ƒ f(a: u32) → u32 ! ∅ { a - 1u32 }\nƒ main() → u32 ! ∅ { f(0u32) }\n",
            vec![],
        ),
        (
            "overflow_neg",
            "ƒ f(a: i8) → i8 ! ∅ { -a }\nƒ main() → i8 ! ∅ { f(-128i8) }\n",
            vec![],
        ),
        (
            "div_by_zero",
            "ƒ f(a: i32, b: i32) → i32 ! ∅ { a / b }\nƒ main() → i32 ! ∅ { f(7, 0) }\n",
            vec![],
        ),
        (
            "rem_by_zero",
            "ƒ f(a: u32, b: u32) → u32 ! ∅ { a % b }\nƒ main() → u32 ! ∅ { f(7u32, 0u32) }\n",
            vec![],
        ),
        (
            "index_out_of_bounds",
            "ƒ f(a: [u8; 3], i: usize) → u8 ! ∅ { a[i] }\nƒ main() → u8 ! ∅ { f([1u8, 2u8, 3u8], 5usize) }\n",
            vec![],
        ),
        (
            "slice_range",
            "ƒ f(n: usize) → usize ! ∅ { a ≔ [1i64, 2, 3]; s ≔ &a[0..n]; s.len() }\nƒ main() → usize ! ∅ { f(9) }\n",
            vec![],
        ),
        (
            "cast_lossy",
            "ƒ f(x: i32) → u8 ! ∅ { x as u8 }\nƒ main() → u8 ! ∅ { f(300) }\n",
            vec![],
        ),
        (
            "cast_lossy",
            "ƒ f(x: i64) → f64 ! ∅ { x as f64 }\nƒ g(x: i64) → u64 ! ∅ { ⌥ f(x).to_i64() { Some(v) ↦ v as u64, None ↦ 0u64 } }\nƒ main() → u64 ! ∅ { g(9007199254740993i64) }\n",
            vec![],
        ),
        (
            "precondition",
            "ƒ h(p: u8) → u8 ! ∅ ⊳ p < 5u8 { p }\nƒ g(a: u8) → u8 ! ∅ { h(a) }\nƒ main() → u8 ! ∅ { g(9u8) }\n",
            vec![],
        ),
        (
            "postcondition",
            "ƒ h(p: u8) → u8 ! ∅ ⊲ result == 1u8 { p }\nƒ main() → u8 ! ∅ { h(9u8) }\n",
            vec![],
        ),
        (
            "invariant",
            "ƒ main() → u64 ! ∅ { μi ≔ 0u64; ⟳ i < 5u64 ↓ 5u64 - i ⋈ i < 3u64 { i ← i + 1u64; } i }\n",
            vec![],
        ),
        (
            "fuel_exhausted",
            "ƒ main() → usize ! ∅ { μi ≔ 0usize; μfuel ≔ 3u64; ⟳ fuel > 0u64 ∧ i < 100usize ↓ fuel { fuel ← fuel - 1u64; i ← i + 1; } core::fuel_left(fuel); i }\n",
            vec![],
        ),
        (
            "abort",
            "ƒ main() → u8 ! {div} { core::abort(\"x\") }\n",
            vec![],
        ),
        (
            "arg_out_of_range",
            "ƒ main() ! {io} { env ≔ root_env(); a ≔ env.arg(5usize); out ≔ root_out(); out.write_str(a); }\n",
            vec![],
        ),
        (
            "arena_exhausted",
            "ƒ main() → u64 ! {mem} {\n  μar ≔ Arena::new();\n  μv ≔ Vec::new(&ar);\n  μi ≔ 0u64;\n  ⟳ i < 1000000u64 ↓ 1000000u64 - i { v.push(i); i ← i + 1u64; }\n  v.len() as u64\n}\n",
            vec![("GRITRT_ARENA_LIMIT", "2097152")],
        ),
    ]
}

#[test]
fn every_reachable_abort_kind_is_the_same_in_dev_and_release() {
    for (kind, src, env) in abort_cases() {
        let d = run(src, "dev", &env);
        let r = run(src, &rel(), &env);
        assert_eq!(
            (d.0, d.1, d.2.as_str()),
            (70, true, kind),
            "dev: loại abort sai\n{src}\n{d:?}"
        );
        assert_eq!(
            (r.0, r.1, r.2.as_str()),
            (70, true, kind),
            "RELEASE: loại abort sai (hoặc bị tối ưu xoá)\n{src}\n{r:?}"
        );
    }
}

fn lcg(s: &mut u64) -> u64 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *s >> 11
}

/// bẫy FMA / gập hằng / đổi thứ tự: kỳ vọng do Rust tính từng phép IEEE (không hợp nhất, không đổi thứ tự); release PHẢI khớp từng bit
#[test]
fn float_traps_match_ieee_per_operation_in_release() {
    let mut seed = 20261014u64;
    let mut fma_differs = 0;
    let mut reorder_differs = 0;
    for i in 0..24 {
        // lấy ba số "đẹp" có mẫu bit đầy đủ (mantissa ngẫu nhiên, mũ vừa phải) để FMA khác nhau thường xuyên
        let mut mk = || {
            let m = lcg(&mut seed) & 0xF_FFFF_FFFF_FFFF;
            let e = 1023 - 8 + (lcg(&mut seed) % 17);
            f64::from_bits((e << 52) | m)
        };
        let (a, b, c0) = (mk(), mk(), mk());
        // nửa số ca: c ≈ −a·b (triệt tiêu) để FMA lộ sai số làm tròn của phép nhân
        let c = if i % 2 == 0 { -(a * b) } else { c0 };
        let fused = a.mul_add(b, c);
        let plain = a * b + c;
        if fused.to_bits() != plain.to_bits() {
            fma_differs += 1;
        }
        let left = (a + b) + c;
        let right = a + (b + c);
        if left.to_bits() != right.to_bits() {
            reorder_differs += 1;
        }
        let src = format!(
            "ƒ main() → u64 ! ∅ {{\n  a ≔ core::f64_from_bits({}u64);\n  b ≔ core::f64_from_bits({}u64);\n  c ≔ core::f64_from_bits({}u64);\n  r1 ≔ (a * b + c).to_bits();\n  r2 ≔ ((a + b) + c).to_bits();\n  r3 ≔ (a + (b + c)).to_bits();\n  r1 +% (r2 *% 3u64) +% (r3 *% 5u64)\n}}\n",
            a.to_bits(),
            b.to_bits(),
            c.to_bits()
        );
        let want = plain
            .to_bits()
            .wrapping_add(left.to_bits().wrapping_mul(3))
            .wrapping_add(right.to_bits().wrapping_mul(5));
        for p in ["dev", rel().as_str()] {
            let got = run(&src, p, &[]);
            assert_eq!(
                (got.0, got.3.as_str()),
                (0, want.to_string().as_str()),
                "{p}: số thực lệch IEEE\n{src}\n{got:?}"
            );
        }
    }
    assert!(
        fma_differs >= 5 && reorder_differs >= 3,
        "bộ bẫy mù: FMA khác ở {fma_differs}/24, đổi thứ tự khác ở {reorder_differs}/24"
    );
}

/// hằng số được LLVM gập: kết quả vẫn đúng IEEE (nhân, cộng, chia, sqrt trên literal)
#[test]
fn constant_folded_floats_are_ieee_exact_in_release() {
    let cases: [(&str, f64); 5] = [
        ("(0.1f64 + 0.2f64)", 0.1f64 + 0.2f64),
        ("(1.0f64 / 3.0f64)", 1.0f64 / 3.0f64),
        ("(2.0f64).sqrt()", 2.0f64.sqrt()),
        ("(0.1f64 * 3.0f64 + 0.3f64)", 0.1f64 * 3.0f64 + 0.3f64),
        ("(1e308f64 * 10.0f64)", 1e308f64 * 10.0f64),
    ];
    for (e, want) in cases {
        let src = format!("ƒ main() → u64 ! ∅ {{ ({e}).to_bits() }}\n");
        for p in ["dev", rel().as_str()] {
            let got = run(&src, p, &[]);
            assert_eq!(
                (got.0, got.3.as_str()),
                (0, want.to_bits().to_string().as_str()),
                "{p}: {e}\n{got:?}"
            );
        }
    }
}

/// NaN, ±0, ±∞, số dưới chuẩn: so sánh có thứ tự, `is_nan`, bit của ±0, nhân dưới chuẩn
#[test]
#[allow(clippy::neg_multiply)] // cố ý: kiểm `0.0 * -1.0` cho bit của −0
fn nan_signed_zero_infinity_subnormal_behave_per_ieee_in_release() {
    let nan = "core::f64_from_bits(9221120237041090560u64)"; // 0x7FF8000000000000
    // (biểu thức u64, kỳ vọng)
    let progs: Vec<(String, u64)> = vec![
        (format!("⎇ {nan} < 1.0f64 {{ 1u64 }} ⇝ {{ 0u64 }}"), 0),
        (format!("⎇ {nan} > 1.0f64 {{ 1u64 }} ⇝ {{ 0u64 }}"), 0),
        (format!("⎇ {nan} <= {nan} {{ 1u64 }} ⇝ {{ 0u64 }}"), 0),
        (format!("⎇ {nan}.is_nan() {{ 1u64 }} ⇝ {{ 0u64 }}"), 1),
        (
            "((0.0f64) * (-1.0f64)).to_bits()".to_string(),
            ((0.0f64) * (-1.0f64)).to_bits(),
        ),
        ("(0.0f64 - 0.0f64).to_bits()".to_string(), 0),
        (
            "(-0.0f64 + 0.0f64).to_bits()".to_string(),
            (-0.0f64 + 0.0f64).to_bits(),
        ),
        (
            "(1.0f64 / 0.0f64).to_bits()".to_string(),
            (1.0f64 / 0.0f64).to_bits(),
        ),
        (
            "(-1.0f64 / 0.0f64).to_bits()".to_string(),
            (-1.0f64 / 0.0f64).to_bits(),
        ),
        (
            "(core::f64_from_bits(1u64) * 0.5f64).to_bits()".to_string(),
            (f64::from_bits(1) * 0.5).to_bits(),
        ),
        (
            "(core::f64_from_bits(1u64) + core::f64_from_bits(1u64)).to_bits()".to_string(),
            (f64::from_bits(1) + f64::from_bits(1)).to_bits(),
        ),
        (
            "(core::f64_from_bits(4503599627370496u64) * 0.5f64).to_bits()".to_string(),
            (f64::from_bits(4503599627370496) * 0.5).to_bits(),
        ),
        (
            "(core::f64_from_bits(1u64)).sqrt().to_bits()".to_string(),
            f64::from_bits(1).sqrt().to_bits(),
        ),
    ];
    for (e, want) in progs {
        let src = format!("ƒ main() → u64 ! ∅ {{ {e} }}\n");
        for p in ["dev", rel().as_str()] {
            let got = run(&src, p, &[]);
            assert_eq!(
                (got.0, got.3.as_str()),
                (0, want.to_string().as_str()),
                "{p}: {e}\n{got:?}"
            );
        }
    }
}
