//! D.1 (TCB `Cap<Fs>::read`): đọc KHÔNG đủ phải abort `io_error` (kind 14, mã thoát 70), KHÔNG trả nội dung cụt như thể đầy đủ.
//! Exe thật: tệp thường đọc đúng; thư mục, tệp không tồn tại, tệp BỊ KHOÁ (Windows: chia sẻ 0) ⇒ abort `io_error`. Phần "cụt/tăng/lỗi giữa chừng" kiểm
//! TẤT ĐỊNH ở unit test của gt-rt (`read_checked_never_returns_partial_content`, bộ đọc giả): tệp bị cắt GIỮA lúc đọc không dựng lại được ổn định ở mức exe.
#![cfg(feature = "llvm")]
use std::path::PathBuf;
use std::process::Command;

const SRC: &str = "ƒ main() ! {io} {
  fs ≔ root_fs();
  env ≔ root_env();
  out ≔ root_out();
  f ≔ fs.read(env.arg(1usize));
  out.write_u64(f.len() as u64);
}
";

fn scratch(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("gt-fsread-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo");
    d
}

fn build(d: &std::path::Path, profile: &str) -> PathBuf {
    let src = d.join("prog.gt");
    std::fs::write(&src, SRC).expect("ghi");
    let exe = d.join("prog.exe");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args(["build", &format!("--profile={profile}"), "-o"])
        .arg(&exe)
        .arg(&src)
        .output()
        .expect("chạy gritc");
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    exe
}

fn run(exe: &std::path::Path, arg: &std::path::Path) -> (i32, String, String) {
    let o = Command::new(exe).arg(arg).output().expect("chạy exe");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn regular_files_are_read_completely_and_every_other_kind_aborts_io_error() {
    for profile in ["dev", "release-o2"] {
        let d = scratch(profile);
        let exe = build(&d, profile);
        // tệp thường: rỗng, nhỏ, 3 MiB
        for (name, data) in [
            ("empty", Vec::new()),
            ("small", b"hello".to_vec()),
            ("big", vec![7u8; 3 << 20]),
        ] {
            let f = d.join(name);
            std::fs::write(&f, &data).expect("ghi");
            let (code, out, err) = run(&exe, &f);
            assert_eq!(code, 0, "{profile}/{name}: {err}");
            assert_eq!(out, data.len().to_string(), "{profile}/{name}");
        }
        // thư mục
        let (code, _, err) = run(&exe, &d);
        assert_eq!(code, 70, "{profile}/thư mục: {err}");
        assert!(err.contains("abort: io_error"), "{profile}/thư mục: {err}");
        // không tồn tại
        let (code, _, err) = run(&exe, &d.join("khong_co"));
        assert_eq!(code, 70, "{profile}/không tồn tại: {err}");
        assert!(err.contains("abort: io_error"), "{err}");
        let _ = std::fs::remove_dir_all(&d);
    }
}

/// tệp bị khoá độc quyền bởi tiến trình khác (Windows: `share_mode(0)`) ⇒ abort `io_error`, không đọc nửa chừng
#[cfg(windows)]
#[test]
fn a_locked_file_aborts_io_error() {
    use std::os::windows::fs::OpenOptionsExt;
    let d = scratch("locked");
    let exe = build(&d, "dev");
    let f = d.join("locked.txt");
    std::fs::write(&f, b"noi dung").expect("ghi");
    let _hold = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&f)
        .expect("khoá");
    let (code, _, err) = run(&exe, &f);
    assert_eq!(code, 70, "{err}");
    assert!(err.contains("abort: io_error"), "{err}");
    drop(_hold);
    let (code, out, err) = run(&exe, &f);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "8");
    let _ = std::fs::remove_dir_all(&d);
}
