//! Dogfood công cụ 3: `examples/sha256` (Grit, dự án Rig). Đối chiếu độc lập: vector NIST viết cứng + `hashlib` của Python (OpenSSL) trên
//! biên đệm, 1 MiB, 32 MiB và 200 tệp ngẫu nhiên. Cần `python` trên PATH (công cụ kiểm thử của dự án, xem docs/ENVIRONMENT.md).
use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("rig-sha-{}-{n}-{name}", std::process::id()));
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

fn build_sha() -> PathBuf {
    let d = tmp("build");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sha256"),
        &d,
    );
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
    d.join("target/sha256.exe")
}

fn lcg(seed: &mut u64) -> u8 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 33) as u8
}

fn tool(exe: &Path, p: &Path) -> String {
    let o = Command::new(exe).arg(p).output().expect("chạy sha256");
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}: {}",
        p.display(),
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// hashlib (Python/OpenSSL) cho một loạt tệp: trả dãy hex theo thứ tự.
fn hashlib(paths: &[PathBuf]) -> Vec<String> {
    let script = "import hashlib,sys\nfor p in sys.argv[1:]:\n    print(hashlib.sha256(open(p,'rb').read()).hexdigest())";
    let o = Command::new("python")
        .args(["-I", "-c", script])
        .args(paths)
        .output()
        .expect("cần `python` trên PATH để làm bản tham chiếu độc lập (hashlib)");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

fn check_all(exe: &Path, files: &[(String, Vec<u8>)], dir: &Path) {
    let paths: Vec<PathBuf> = files
        .iter()
        .map(|(n, b)| {
            let p = dir.join(n);
            std::fs::write(&p, b).expect("ghi");
            p
        })
        .collect();
    let want = hashlib(&paths);
    for ((name, b), (p, w)) in files.iter().zip(paths.iter().zip(want)) {
        assert_eq!(
            tool(exe, p),
            format!("{w} {}\n", p.display()),
            "{name} ({} byte) ≠ hashlib",
            b.len()
        );
    }
}

#[test]
fn nist_vectors_hard_coded() {
    let exe = build_sha();
    let d = tmp("nist");
    for (name, data, hex) in [
        ("empty", b"".to_vec(), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
        ("abc", b"abc".to_vec(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
        (
            "448bit",
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".to_vec(),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        ),
        (
            "896bit",
            b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu".to_vec(),
            "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
        ),
        ("million_a", vec![b'a'; 1_000_000], "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"),
    ] {
        let p = d.join(format!("{name}.bin"));
        std::fs::write(&p, &data).expect("ghi");
        assert_eq!(tool(&exe, &p), format!("{hex} {}\n", p.display()), "{name}");
    }
}

#[test]
fn padding_boundaries_match_hashlib() {
    let exe = build_sha();
    let d = tmp("pad");
    let mut seed = 5u64;
    let files: Vec<(String, Vec<u8>)> = [
        0usize, 1, 2, 54, 55, 56, 57, 63, 64, 65, 119, 120, 121, 127, 128, 129, 191, 192, 255, 256,
    ]
    .iter()
    .map(|&n| {
        (
            format!("len{n}.bin"),
            (0..n).map(|_| lcg(&mut seed)).collect(),
        )
    })
    .collect();
    check_all(&exe, &files, &d);
}

#[test]
fn random_content_200_files_match_hashlib() {
    let exe = build_sha();
    let d = tmp("rand");
    let mut seed = 20261013u64;
    let files: Vec<(String, Vec<u8>)> = (0..200)
        .map(|i| {
            let len = match i % 4 {
                0 => (lcg(&mut seed) as usize) % 64,
                1 => (lcg(&mut seed) as usize) * 3,
                2 => 1000 + (lcg(&mut seed) as usize) * 37,
                _ => (lcg(&mut seed) as usize) % 300,
            };
            (
                format!("r{i}.bin"),
                (0..len).map(|_| lcg(&mut seed)).collect(),
            )
        })
        .collect();
    check_all(&exe, &files, &d);
}

#[test]
fn large_files_match_hashlib_and_report_throughput() {
    let exe = build_sha();
    let d = tmp("big");
    let mut seed = 77u64;
    for (name, mib) in [("1MiB", 1usize), ("32MiB", 32)] {
        let data: Vec<u8> = (0..mib * 1024 * 1024).map(|_| lcg(&mut seed)).collect();
        let p = d.join(format!("{name}.bin"));
        std::fs::write(&p, &data).expect("ghi");
        let t = std::time::Instant::now();
        let got = tool(&exe, &p);
        let dt = t.elapsed();
        let want = hashlib(std::slice::from_ref(&p));
        assert_eq!(got, format!("{} {}\n", want[0], p.display()), "{name}");
        println!(
            "sha256.gt (LLVM -O0, mọi check giữ) {name}: {:.2} s ⇒ {:.1} MiB/s",
            dt.as_secs_f64(),
            mib as f64 / dt.as_secs_f64()
        );
    }
}

#[test]
fn sha256_error_paths() {
    let exe = build_sha();
    let d = tmp("err");
    let o = Command::new(&exe)
        .arg(d.join("khong_co.bin"))
        .output()
        .expect("chạy");
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stderr).ends_with(": no such file\n"));
    assert!(o.stdout.is_empty());
    let o = Command::new(&exe).output().expect("chạy");
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&o.stderr), "usage: sha256 <file>\n");
}
