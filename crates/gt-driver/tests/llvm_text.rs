//! Hồi quy FB-4 (2026-10-14, gặp khi dựng `examples/wordfreq`): `u8 as char` KHÔNG hạ được ở codegen
//! (`backend llvm chưa hỗ trợ: ép kiểu Int(U8) → Char`) dù thông dịch chạy được. Mọi `u8` là một giá trị vô hướng hợp lệ ⇒ nới không dấu.
#![cfg(feature = "llvm")]
use std::path::PathBuf;
use std::process::Command;

fn run(src: &str, backend: &str) -> (i32, String) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d: PathBuf = std::env::temp_dir().join(format!("gt-text-test-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo");
    let f = d.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args(["run", "--json", &format!("--backend={backend}")])
        .arg(&f)
        .output()
        .expect("chạy gritc");
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let line = text.lines().rev().find(|l| l.starts_with('{')).unwrap_or("");
    let v: serde_json::Value = serde_json::from_str(line)
        .unwrap_or_else(|e| panic!("JSON ({e}): {text}\n{}", String::from_utf8_lossy(&o.stderr)));
    let raw = v["run"]["value"].as_str().unwrap_or("").to_string();
    let value = raw
        .strip_prefix("Int(")
        .and_then(|s| s.split(',').next())
        .map_or(raw.clone(), str::to_string);
    let _ = std::fs::remove_dir_all(&d);
    (o.status.code().unwrap_or(-1), value)
}

fn both(src: &str, expect: &str) {
    let l = run(src, "llvm");
    assert_eq!(l, (0, expect.to_string()), "llvm\n{src}");
    assert_eq!(run(src, "interpreter"), l, "thông dịch ≠ llvm\n{src}");
}

/// `u8 as char` rồi đẩy vào Text: các byte 0x41 ('A'), 0x7f, 0xe9 ('é' U+00E9 = 2 byte UTF-8), 0xff ('ÿ' = 2 byte): độ dài 1 + 1 + 2 + 2 = 6
#[test]
fn u8_as_char_lowers_and_encodes_latin1_scalars() {
    both(
        "ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  μt ≔ Text::new(&!ar);
  t.push_char(65u8 as char);
  t.push_char(127u8 as char);
  t.push_char(233u8 as char);
  t.push_char(255u8 as char);
  b ≔ t.as_bytes();
  (t.len() as u64) *% 1000u64 +% (b[2] as u64)
}
",
        "6195",
    );
}

/// `char as u32` quay lại đúng giá trị (không đổi dấu)
#[test]
fn u8_char_round_trip() {
    both(
        "ƒ main() → u32 ! ∅ { c ≔ 200u8 as char; c as u32 }\n",
        "200",
    );
}
