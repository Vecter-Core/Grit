//! FB-C23 / Seed 1.0 S7 (đọc mã đối kháng nhóm B): `alloca` đặt TẠI CHỖ GỌI (`io.res`, `cas.res`, `rep.i`) cấp thêm ngăn xếp ở MỖI vòng lặp và không bao giờ
//! giải phóng trước khi hàm trả ⇒ vòng lặp hợp lệ, có chứng minh dừng, chết bằng SIGSEGV thay vì chạy hết. Đúng: mọi `alloca` ở khối ENTRY.
#![cfg(feature = "llvm")]
use std::process::Command;

const SRC: &str = "■ A: Atomic<u32> ≔ Atomic::new(0u32);
ƒ main() ! {io, div} {
  fs ≔ root_fs();
  out ≔ root_out();
  μi ≔ 0u64;
  μbad ≔ 0u64;
  ⟳ i < 3u64 ↓ 3u64 - i {
    ⌥ fs.open(\"/nonexistent_zzz\") { Ok(_) ↦ { bad ← bad + 1u64; }, Err(_) ↦ { } }
    r ≔ A.compare_exchange(0u32, 1u32, AtomicOrdering::SeqCst, AtomicOrdering::SeqCst);
    ⌥ r { Ok(_) ↦ { bad ← bad + 0u64; }, Err(_) ↦ { } }
    z ≔ [0u8; 16];
    bad ← bad + z[3usize] as u64;
    i ← i + 1u64;
  }
  out.write_u64(bad);
}
";

#[test]
fn every_alloca_is_in_the_entry_block_fb_c23() {
    for profile in ["dev", "release-o2"] {
        let d = std::env::temp_dir().join(format!("gt-alloca-{}-{profile}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("tạo");
        let src = d.join("prog.gt");
        std::fs::write(&src, SRC).expect("ghi");
        let ir_path = d.join("prog.ll");
        let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
            .env("GRITC_DUMP_IR", &ir_path)
            .args(["build", &format!("--profile={profile}"), "-o"])
            .arg(d.join("prog.exe"))
            .arg(&src)
            .output()
            .expect("chạy gritc");
        assert_eq!(
            o.status.code(),
            Some(0),
            "{profile}: {}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        let ir = std::fs::read_to_string(&ir_path).expect("IR");
        let mut func = String::new();
        let mut blocks = 0usize;
        let mut total = 0usize;
        let mut bad = Vec::new();
        for line in ir.lines() {
            if line.starts_with("define ") {
                func = line.to_string();
                blocks = 0; // nhãn đầu tiên (hoặc không nhãn) là khối entry
                continue;
            }
            // nhãn khối: `tên:` ở cột 0 (có thể kèm `; preds = …`)
            if !line.starts_with(' ')
                && line.split(':').next().is_some_and(|h| {
                    line.contains(':')
                        && !h.is_empty()
                        && h.chars()
                            .all(|c| c.is_alphanumeric() || "_.$-\"".contains(c))
                })
            {
                blocks += 1;
                continue;
            }
            if line.contains("= alloca ") {
                total += 1;
                if blocks > 1 {
                    bad.push(format!("{func}\n  {}", line.trim()));
                }
            }
        }
        assert!(
            total > 0,
            "{profile}: không thấy alloca nào (parser IR hỏng?)"
        );
        assert!(
            bad.is_empty(),
            "{profile}: alloca ngoài khối entry:\n{}",
            bad.join("\n")
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
