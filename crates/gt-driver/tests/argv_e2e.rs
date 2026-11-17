//! H10 đầu-cuối (R11b nghi vấn "argv", GÓI 3 S2.a): đối số dòng lệnh đi từ HĐH qua `gt-rt` tới `Cap<Env>::arg` đúng từng byte UTF-8 ở dev và release-o2
//! (rỗng, Unicode, 32 000 ký tự, 200 đối số); trên Windows, surrogate LẺ (WTF-16 không phải UTF-8) bị thay bằng U+FFFD — giới hạn CÓ CHỦ Ý, luôn là `&str` hợp lệ,
//! `arg_count` và độ dài nhất quán. Bằng chứng thực nghiệm trên các ca đã thử, không phải chứng minh.
#![cfg(feature = "llvm")]
use std::path::PathBuf;
use std::process::Command;

const PROG: &str = "ƒ main() ! {io} {\n\
  env ≔ root_env();\n  out ≔ root_out();\n  out.write_u64(env.arg_count() as u64);\n  out.write_str(\"\\n\");\n\
  n ≔ env.arg_count();\n  μi ≔ 1usize;\n\
  ⟳ i < n ↓ n - i {\n    s ≔ env.arg(i);\n    out.write_u64(s.len() as u64);\n    out.write_str(\":\");\n    out.write_str(s);\n    out.write_str(\"\\n\");\n    i ← i + 1usize;\n  }\n}\n";

fn build(profile: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("grit_argv_{profile}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo");
    let src = d.join("argv.gt");
    std::fs::write(&src, PROG).expect("ghi");
    let exe = d.join("argv.exe");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args(["build", "--backend=llvm", &format!("--profile={profile}"), "-o"])
        .arg(&exe)
        .arg(&src)
        .output()
        .expect("chạy gritc");
    assert!(o.status.success(), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    std::thread::sleep(std::time::Duration::from_secs(3)); // Defender quét exe mới (WinError 5)
    exe
}

fn expected(args: &[String]) -> Vec<u8> {
    let mut out = format!("{}\n", args.len() + 1).into_bytes();
    for a in args {
        out.extend_from_slice(format!("{}:", a.len()).as_bytes());
        out.extend_from_slice(a.as_bytes());
        out.push(b'\n');
    }
    out
}

#[test]
fn arguments_reach_the_program_byte_for_byte_in_dev_and_release_o2() {
    let cases: Vec<(&str, Vec<String>)> = vec![
        ("ascii", vec!["abc".into(), "x y".into()]),
        ("empty", vec![String::new(), "z".into()]),
        ("unicode", vec!["héllo日本語😀".into(), "ñ".into()]),
        ("long", vec!["a".repeat(32_000)]),
        ("many", (0..200).map(|i| i.to_string()).collect()),
    ];
    for profile in ["dev", "release-o2"] {
        let exe = build(profile);
        for (name, args) in &cases {
            let o = Command::new(&exe).args(args).output().expect("chạy exe");
            assert_eq!(o.status.code(), Some(0), "{profile}/{name}");
            assert_eq!(o.stdout, expected(args), "{profile}/{name}: byte đối số khác");
        }
        #[cfg(windows)]
        {
            use std::ffi::OsString;
            use std::os::windows::ffi::OsStringExt;
            let lone: OsString = OsString::from_wide(&[0x61, 0xD800, 0x62]);
            let o = Command::new(&exe).arg(&lone).output().expect("chạy exe");
            assert_eq!(o.status.code(), Some(0), "{profile}/surrogate");
            // 'a' + U+FFFD (EF BF BD) + 'b' = 5 byte; luôn UTF-8 hợp lệ
            assert_eq!(o.stdout, b"2\n5:a\xEF\xBF\xBDb\n".to_vec(), "{profile}/surrogate lẻ");
            assert!(std::str::from_utf8(&o.stdout).is_ok());
        }
    }
}
