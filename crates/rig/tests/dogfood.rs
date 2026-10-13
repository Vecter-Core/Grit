//! Dogfood (docs/DOGFOOD.md): `examples/wc` — công cụ đếm dòng/từ/byte viết BẰNG GRIT trong một dự án Rig, dựng bằng `rig build`,
//! so với bản tham chiếu viết bằng Rust trên nhiều tệp (kể cả tệp rỗng, nhị phân, lớn) và các đường lỗi (abort có mã 70).
use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("rig-dogfood-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("tạo");
    for e in std::fs::read_dir(from).expect("đọc").flatten() {
        let p = e.path();
        if e.file_name() == "target" {
            continue;
        }
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).expect("chép");
        }
    }
}

/// Tham chiếu: cùng định nghĩa với wc.gt (từ = dãy byte không phải ' ' '\n' '\t' '\r').
fn reference(b: &[u8]) -> (u64, u64, u64) {
    let (mut lines, mut words, mut in_word) = (0u64, 0u64, false);
    for &c in b {
        if c == b'\n' {
            lines += 1;
        }
        if matches!(c, b' ' | b'\n' | b'\t' | b'\r') {
            in_word = false;
        } else if !in_word {
            words += 1;
            in_word = true;
        }
    }
    (lines, words, b.len() as u64)
}

fn build_wc() -> PathBuf {
    let d = tmp("build");
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/wc");
    copy_dir(&src, &d);
    let o = Command::new(env!("CARGO_BIN_EXE_rig"))
        .current_dir(&d)
        .args(["build", "--locked"])
        .output()
        .expect("chạy rig");
    assert_eq!(
        o.status.code(),
        Some(0),
        "rig build: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    d.join("target/wc.exe")
}

fn lcg(seed: &mut u64) -> u8 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 33) as u8
}

#[test]
fn wc_written_in_grit_matches_the_reference_on_many_files() {
    let exe = build_wc();
    let d = tmp("files");
    let mut seed = 7u64;
    let words = ["alpha", "b", "gamma_delta", "é", "日本語", "x1"];
    let mut cases: Vec<(String, Vec<u8>)> = vec![
        ("empty".into(), vec![]),
        ("one_newline".into(), b"\n".to_vec()),
        ("no_trailing_newline".into(), b"one two\nthree".to_vec()),
        ("crlf".into(), b"a b\r\nc d\r\n".to_vec()),
        ("spaces_only".into(), b"   \t \n\n  ".to_vec()),
        ("all_bytes".into(), (0u8..=255).collect()),
        ("nul_and_high".into(), vec![0, 0xff, b'a', 0, b' ', 0x80]),
    ];
    let mut text = Vec::new();
    for i in 0..5000 {
        text.extend_from_slice(words[i % words.len()].as_bytes());
        text.push(if i % 7 == 0 { b'\n' } else { b' ' });
    }
    cases.push(("text".into(), text));
    let random: Vec<u8> = (0..100_000).map(|_| lcg(&mut seed)).collect();
    cases.push(("random_100k".into(), random));
    cases.push(("path with spaces".into(), b"x y\n".to_vec()));
    for (name, bytes) in &cases {
        let p = d.join(format!("{name}.txt"));
        std::fs::write(&p, bytes).expect("ghi");
        let o = Command::new(&exe).arg(&p).output().expect("chạy wc");
        assert_eq!(
            o.status.code(),
            Some(0),
            "{name}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        let (l, w, b) = reference(bytes);
        let want = format!("{l} {w} {b} {}\n", p.display());
        assert_eq!(String::from_utf8_lossy(&o.stdout), want, "{name}");
    }
}

#[test]
fn wc_handles_a_large_file_and_reports_throughput() {
    let exe = build_wc();
    let d = tmp("big");
    let mut seed = 99u64;
    let big: Vec<u8> = (0..32 * 1024 * 1024)
        .map(|_| match lcg(&mut seed) % 8 {
            0 => b'\n',
            1 | 2 => b' ',
            _ => b'a' + lcg(&mut seed) % 26,
        })
        .collect();
    let p = d.join("big.txt");
    std::fs::write(&p, &big).expect("ghi");
    let t = std::time::Instant::now();
    let o = Command::new(&exe).arg(&p).output().expect("chạy wc");
    let dt = t.elapsed();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let (l, w, b) = reference(&big);
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        format!("{l} {w} {b} {}\n", p.display())
    );
    println!(
        "wc.gt (LLVM -O0, mọi check giữ) trên {} MiB: {:.2} s ⇒ {:.0} MiB/s",
        b / 1024 / 1024,
        dt.as_secs_f64(),
        (b as f64 / 1048576.0) / dt.as_secs_f64()
    );
}

#[test]
fn wc_error_paths_abort_with_code_70_and_a_named_kind() {
    let exe = build_wc();
    let d = tmp("err");
    let o = Command::new(&exe)
        .arg(d.join("khong_co.txt"))
        .output()
        .expect("chạy");
    assert_eq!(o.status.code(), Some(70));
    let e = String::from_utf8_lossy(&o.stderr).into_owned();
    assert!(
        e.contains("io_error") && e.contains("abort: io_error"),
        "{e}"
    );
    assert!(o.stdout.is_empty(), "không in gì khi lỗi");
    let o = Command::new(&exe).output().expect("chạy");
    assert_eq!(o.status.code(), Some(70));
    assert!(String::from_utf8_lossy(&o.stderr).contains("abort: arg_out_of_range"));
}
