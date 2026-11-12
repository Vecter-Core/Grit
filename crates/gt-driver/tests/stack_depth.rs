//! Chương trình HỢP LỆ và đã được M8 chứng minh dừng không được chết vì ngăn xếp mặc định của tiến trình (1 MiB Windows / 8 MiB Linux):
//! `gt-rt` chạy `main` và worker trên luồng 256 MiB. Trước đó đệ quy sâu có `↓` và mảng lớn cục bộ bị HĐH giết và bị báo là lỗi nội bộ (mã 2).
#![cfg(feature = "llvm")]
use std::process::Command;

fn run(name: &str, src: &str) -> (Option<i32>, String) {
    let dir = std::env::temp_dir().join(format!("grit_stack_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("tạo thư mục");
    let f = dir.join("main.gt");
    std::fs::write(&f, src).expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args(["run", "--backend=llvm"])
        .arg(&f)
        .output()
        .expect("chạy gritc");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    (o.status.code(), text)
}

#[test]
fn deep_terminating_recursion_runs_natively() {
    let (code, out) = run(
        "rec",
        "ƒ rec(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { 1 + rec(n - 1) } }\nƒ main() → u32 ! ∅ { rec(2000000) }\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("main trả về 2000000"), "{out}");
}

#[test]
fn large_local_array_runs_natively() {
    let (code, out) = run(
        "arr",
        "ƒ main() → u8 ! ∅ { a ≔ [7u8; 20000000]; a[19999999] }\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("main trả về 7"), "{out}");
}

#[test]
fn spawned_worker_large_local_array_runs_natively() {
    let (code, out) = run(
        "spawn-arr",
        "■ DONE: Atomic<u32> ≔ Atomic::new(0u32);\n\
         ƒ big(n: u32) ! ∅ {\n\
           μa ≔ [0u8; 3000000];\n\
           a[2999999usize] ← 1u8;\n\
           DONE.fetch_add(a[2999999usize] as u32, AtomicOrdering::SeqCst);\n\
         }\n\
         ƒ main() → u32 ! ∅ {\n\
           spawn(big, 0u32);\n\
           μspin ≔ 0u64;\n\
           ⟳ spin < 3000000000u64 ∧ DONE.load(AtomicOrdering::SeqCst) < 1u32 ↓ 3000000000u64 - spin { spin ← spin + 1u64; }\n\
           DONE.load(AtomicOrdering::SeqCst)\n\
         }\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("main trả về 1"), "{out}");
}
