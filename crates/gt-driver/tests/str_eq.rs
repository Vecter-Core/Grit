//! FB-C13 / Seed 1.0 S2: `==`/`≠` trên `&str` được M4 chấp nhận (và thông dịch chạy đúng) nhưng backend LLVM từ chối
//! ("phép hai ngôi Eq trên Ref(false, Str)") ⇒ chương trình dùng thật (so tên lệnh, cờ) không dựng được. Cần chạy đúng ở cả hai backend,
//! 3 hồ sơ (`GT_PROFILE`), ≥ 30 cặp chuỗi (rỗng, tiền tố, khác độ dài, khác một byte, nhiều byte, trùng), so với mô hình Rust.
#![cfg(feature = "llvm")]
use std::process::Command;

fn run_value(src: &str, backend: &str) -> String {
    let profile = std::env::var("GT_PROFILE").unwrap_or_else(|_| "dev".to_string());
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("gt-streq-{}-{n}-{backend}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    let f = d.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args([
            "run",
            "--json",
            &format!("--backend={backend}"),
            &format!("--profile={profile}"),
        ])
        .arg(&f)
        .output()
        .expect("chạy gritc");
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    assert_eq!(o.status.code(), Some(0), "{backend}: {text}\n{err}\n{src}");
    let line = text
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .unwrap_or("");
    let v: serde_json::Value = serde_json::from_str(line).expect("JSON");
    let raw = v["run"]["value"].as_str().unwrap_or("").to_string();
    let _ = std::fs::remove_dir_all(&d);
    raw.strip_prefix("Int(")
        .and_then(|s| s.split(',').next())
        .map_or(raw.clone(), str::to_string)
}

const POOL: [&str; 12] = [
    "",
    "a",
    "ab",
    "abc",
    "abd",
    "b",
    "-c",
    "-n",
    "--",
    "é",
    "aé",
    "zzzzzzzzzzzzzzzzzzzz",
];

#[test]
fn str_equality_agrees_with_rust_in_both_backends_fb_c13() {
    // 12 × 12 = 144 cặp, chia thành 6 chương trình (mỗi cái 24 cặp) để giữ thời gian dựng ngắn
    let mut pairs: Vec<(&str, &str)> = Vec::new();
    for a in POOL {
        for b in POOL {
            pairs.push((a, b));
        }
    }
    for (n, chunk) in pairs.chunks(24).enumerate() {
        let mut src = String::from(
            "ƒ bn(b: bool) → u64 ! ∅ { ⎇ b { 1u64 } ⇝ { 0u64 } }\nƒ main() → u64 ! ∅ {\n  μacc ≔ 0u64;\n",
        );
        let mut want = 0u64;
        for (a, b) in chunk {
            src.push_str(&format!(
                "  acc ← acc *% 3u64 +% bn(\"{a}\" == \"{b}\") +% 2u64 *% bn(\"{a}\" != \"{b}\");\n"
            ));
            want = want
                .wrapping_mul(3)
                .wrapping_add((a == b) as u64)
                .wrapping_add(2 * (a != b) as u64);
        }
        src.push_str("  acc\n}\n");
        for backend in ["interpreter", "llvm"] {
            assert_eq!(
                run_value(&src, backend),
                want.to_string(),
                "chunk {n}, {backend}\n{src}"
            );
        }
    }
}

/// So sánh trên giá trị KHÔNG phải hằng (đối số chạy thật qua hàm) để chắc chắn đi qua codegen chứ không bị gấp hằng.
#[test]
fn str_equality_on_runtime_values_fb_c13() {
    let src = "ƒ same(a: &str, b: &str) → bool ! ∅ { a == b }
ƒ differ(a: &str, b: &str) → bool ! ∅ { a != b }
ƒ pick(k: u8) → &str ! ∅ { ⎇ k == 0u8 { \"ab\" } ⇝ { ⎇ k == 1u8 { \"abc\" } ⇝ { \"\" } } }
ƒ main() → u64 ! ∅ {
  μr ≔ 0u64;
  ⎇ same(pick(0u8), \"ab\") { r ← r + 1u64; } ⇝ { }
  ⎇ same(pick(0u8), pick(1u8)) { r ← r + 10u64; } ⇝ { }
  ⎇ differ(pick(1u8), pick(2u8)) { r ← r + 100u64; } ⇝ { }
  ⎇ same(pick(2u8), \"\") { r ← r + 1000u64; } ⇝ { }
  r
}";
    for backend in ["interpreter", "llvm"] {
        assert_eq!(run_value(src, backend), "1101", "{backend}");
    }
}
