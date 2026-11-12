//! FB-C20 / Seed 1.0 S7 (đọc mã đối kháng nhóm C): kiểm biên của `s[a..b]`, `s[i] ← …`, `s[i].m(…)` dùng ĐỘ DÀI/CON TRỎ cũ khi biểu thức đứng sau
//! (khối `{ s ← &khác[..]; … }`) gán lại `s` trước lúc dùng chỗ ⇒ đọc/ghi NGOÀI mảng thật ở LLVM (và "MIR không hợp lệ" ở thông dịch).
//! Đúng: kiểm biên theo trạng thái TẠI LÚC DÙNG ⇒ abort (mã 70) ở mọi backend/hồ sơ.
#![cfg(feature = "llvm")]
use std::process::Command;

fn exit_of(name: &str, src: &str, backend: &str, profile: &str) -> (Option<i32>, String) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("gt-stale-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    let f = d.join(format!("{name}.gt"));
    std::fs::write(&f, src).expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args([
            "run",
            &format!("--backend={backend}"),
            &format!("--profile={profile}"),
        ])
        .arg(&f)
        .output()
        .expect("chạy gritc");
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    (o.status.code(), out)
}

const CASES: [(&str, &str); 5] = [
    // đầu mút `to` của lát gán lại `s` SAU khi độ dài đã được đọc (đọc lát)
    (
        "range_read",
        "ƒ main() → u64 ! ∅ {\n  big ≔ [7u8; 4096];\n  small ≔ [1u8, 2u8];\n  μs ≔ &big[0usize..4096usize];\n  t ≔ &s[0usize..{ s ← &small[0usize..2usize]; 2000usize }];\n  μacc ≔ 0u64; μi ≔ 0usize;\n  ⟳ i < 2000usize ↓ 2000usize - i { acc ← acc + t[i] as u64; i ← i + 1usize; }\n  t.len() as u64 * 1000000000u64 + acc\n}\n",
    ),
    // cùng lỗi nhưng GHI qua lát `&!`
    (
        "range_write",
        "ƒ main() → u64 ! ∅ {\n  μbig ≔ [7u8; 64];\n  μsmall ≔ [1u8, 2u8];\n  μvictim ≔ [5u8; 32];\n  μs ≔ &!big[0usize..64usize];\n  μt ≔ &!s[0usize..{ s ← &!small[0usize..2usize]; 60usize }];\n  μi ≔ 0usize;\n  ⟳ i < 60usize ↓ 60usize - i { t[i] ← 200u8; i ← i + 1usize; }\n  victim[0usize] as u64 + small[0usize] as u64\n}\n",
    ),
    // gán phần tử: chỗ đích dựng trước, giá trị phải sau đổi `s`
    (
        "assign_elem",
        "ƒ main() → u64 ! ∅ {\n  μbig ≔ [7u8; 64];\n  μsmall ≔ [1u8, 2u8];\n  μs ≔ &!big[0usize..64usize];\n  s[50usize] ← { s ← &!small[0usize..2usize]; 9u8 };\n  small[0usize] as u64\n}\n",
    ),
    // người nhận phương thức dựng trước, đối số đổi `s`, khoản mượn người nhận tạo SAU
    (
        "method_receiver",
        "▢ P { v: u64 }\n⊐ P {\n  ƒ get(&self, k: u64) → u64 ! ∅ { self.v + k }\n}\nƒ main() → u64 ! ∅ {\n  big ≔ [P { v: 7u64 }; 64];\n  small ≔ [P { v: 1u64 }, P { v: 2u64 }];\n  μs ≔ &big[0usize..64usize];\n  s[50usize].get({ s ← &small[0usize..2usize]; 1u64 })\n}\n",
    ),
    // chỉ số được tính SAU khi `s` đổi (đã đúng từ trước; giữ làm đối chứng không hồi quy)
    (
        "index_expr",
        "ƒ main() → u64 ! ∅ {\n  big ≔ [7u8; 64];\n  small ≔ [1u8, 2u8];\n  μs ≔ &big[0usize..64usize];\n  x ≔ s[{ s ← &small[0usize..2usize]; 50usize }];\n  x as u64\n}\n",
    ),
];

#[test]
fn bounds_are_checked_against_the_state_at_the_point_of_use_fb_c20() {
    for (name, src) in CASES {
        for (backend, profile) in [
            ("llvm", "dev"),
            ("llvm", "release"),
            ("llvm", "release-o2"),
            ("interpreter", "dev"),
        ] {
            let (code, out) = exit_of(name, src, backend, profile);
            assert_eq!(
                code,
                Some(70),
                "{name} [{backend}/{profile}]: phải abort (mã 70), thực tế {code:?}\n{out}"
            );
            assert!(
                out.contains("index_out_of_bounds") || out.contains("slice_range"),
                "{name} [{backend}/{profile}]: thiếu lý do abort\n{out}"
            );
        }
    }
}
