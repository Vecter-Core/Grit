//! Dogfood công cụ 5: `examples/wordfreq` (Grit, dự án Rig, dùng `Text`; docs/DOGFOOD.md). Đối chiếu độc lập với bản tham chiếu Rust (HashMap + sắp
//! xếp) về TỪNG BYTE đầu ra. Quy ước chung (README của dự án): tệp phải là UTF-8 hợp lệ (sai ⇒ stderr + mã 2, không abort); TỪ = dãy cực đại các
//! byte thuộc [A-Za-z0-9_] hoặc ≥ 0x80; phân biệt hoa/thường; in tối đa 10 dòng `<số lần> <từ>` theo số lần GIẢM dần rồi byte TĂNG dần.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("rig-wf-{}-{n}-{name}", std::process::id()));
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

fn build_tool() -> PathBuf {
    let d = tmp("build");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/wordfreq"),
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
    d.join("target/wordfreq.exe")
}

fn is_word_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80
}

/// tham chiếu độc lập: `Err(())` nếu không phải UTF-8
fn reference(data: &[u8]) -> Result<Vec<u8>, ()> {
    if std::str::from_utf8(data).is_err() {
        return Err(());
    }
    let mut counts: HashMap<&[u8], u64> = HashMap::new();
    let mut i = 0;
    while i < data.len() {
        if is_word_byte(data[i]) {
            let s = i;
            while i < data.len() && is_word_byte(data[i]) {
                i += 1;
            }
            *counts.entry(&data[s..i]).or_default() += 1;
        } else {
            i += 1;
        }
    }
    let mut v: Vec<(&[u8], u64)> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let mut out = Vec::new();
    for (w, n) in v.into_iter().take(10) {
        out.extend_from_slice(n.to_string().as_bytes());
        out.push(b' ');
        out.extend_from_slice(w);
        out.push(b'\n');
    }
    Ok(out)
}

fn lcg(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed >> 33
}

fn run_tool(exe: &Path, path: &Path) -> (Option<i32>, Vec<u8>, String) {
    let o = Command::new(exe).arg(path).output().expect("chạy wordfreq");
    (
        o.status.code(),
        o.stdout,
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn check(exe: &Path, dir: &Path, name: &str, data: &[u8]) {
    let p = dir.join(name);
    std::fs::write(&p, data).expect("ghi");
    let (code, out, err) = run_tool(exe, &p);
    match reference(data) {
        Ok(want) => {
            assert_eq!(code, Some(0), "{name}: {err}");
            assert!(
                out == want,
                "{name}: đầu ra khác bản tham chiếu\n--- Grit:\n{}\n--- Rust:\n{}",
                String::from_utf8_lossy(&out),
                String::from_utf8_lossy(&want)
            );
        }
        Err(()) => {
            assert_eq!(
                code,
                Some(2),
                "{name}: UTF-8 sai phải là lỗi có kiểm soát (mã 2): {err}"
            );
            assert!(out.is_empty(), "{name}: stdout phải rỗng");
            assert!(err.ends_with(": invalid UTF-8\n"), "{name}: {err}");
            assert!(err.starts_with("wordfreq: "), "{name}: {err}");
        }
    }
    let _ = std::fs::remove_file(&p);
}

/// `msort.gt` của wordfreq là BẢN SAO của sortlines (một nguồn sự thật): lệch ⇒ test đỏ
#[test]
fn msort_is_the_same_source_as_sortlines() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let a = std::fs::read_to_string(root.join("sortlines/src/msort.gt")).expect("đọc");
    let b = std::fs::read_to_string(root.join("wordfreq/src/msort.gt")).expect("đọc");
    assert_eq!(a.replace("\r\n", "\n"), b.replace("\r\n", "\n"));
}

#[test]
fn small_cases_match_the_reference_byte_for_byte() {
    let exe = build_tool();
    let d = tmp("small");
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", b"".to_vec()),
        ("only_separators", b" \n\t,.;!?-- \r\n".to_vec()),
        ("one_word", b"hello".to_vec()),
        ("one_word_nl", b"hello\n".to_vec()),
        ("ties_by_bytes", b"b a c b a c d".to_vec()),
        ("case_sensitive", b"The the THE the The".to_vec()),
        (
            "punctuation_and_underscore",
            b"dog_1, dog_1; dog-1 dog.1 _x_ _x_".to_vec(),
        ),
        ("crlf", b"a b\r\na b\r\nc\r\n".to_vec()),
        (
            "utf8_words",
            "élan élan Élan 日本語 日本語 😀 😀 😀 naïve café"
                .as_bytes()
                .to_vec(),
        ),
        (
            "utf8_mixed_with_ascii_glue",
            "aé,bé aé;bé".as_bytes().to_vec(),
        ),
        ("nul_is_a_separator", b"ab\0ab\0cd".to_vec()),
        ("digits", b"1 22 333 1 22 1".to_vec()),
        ("twelve_distinct", b"a b c d e f g h i j k l a".to_vec()),
        ("exactly_ten_distinct", b"a b c d e f g h i j".to_vec()),
    ];
    for (n, data) in &cases {
        check(&exe, &d, n, data);
    }
}

#[test]
fn invalid_utf8_is_a_controlled_error_not_an_abort() {
    let exe = build_tool();
    let d = tmp("bad");
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("lone_ff", vec![b'o', b'k', b' ', 0xff, b' ', b'x']),
        ("lone_continuation", vec![0x80]),
        ("truncated_2byte", vec![b'a', 0xc3]),
        ("truncated_4byte", vec![0xf0, 0x9f, 0x98]),
        ("overlong_slash", vec![0xc0, 0xaf]),
        ("overlong_3byte", vec![0xe0, 0x80, 0x80]),
        ("surrogate", vec![0xed, 0xa0, 0x80]),
        ("above_10ffff", vec![0xf4, 0x90, 0x80, 0x80]),
        ("latin1_e_acute", vec![b'c', b'a', b'f', 0xe9]),
    ];
    for (n, data) in &cases {
        assert!(
            reference(data).is_err(),
            "{n}: bản tham chiếu phải coi là không hợp lệ"
        );
        check(&exe, &d, n, data);
    }
    // thiếu tệp / thiếu đối số
    let (code, out, err) = run_tool(&exe, &d.join("khong_co"));
    assert_eq!(code, Some(2));
    assert!(err.ends_with(": no such file\n") && out.is_empty(), "{err}");
    let o = Command::new(&exe).output().expect("chạy");
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "usage: wordfreq <file>\n"
    );
}

fn random_words(seed: &mut u64, bytes: usize, vocab: u64) -> Vec<u8> {
    let alpha = ["a", "b", "c", "é", "ß", "日", "😀", "Z", "9", "_"];
    let mut v = Vec::with_capacity(bytes + 32);
    while v.len() < bytes {
        let id = lcg(seed) % vocab;
        // từ xác định theo id (nhiều từ khác nhau, tần suất lệch ⇒ có hạng rõ)
        let mut x = id;
        let n = 1 + (x % 5) as usize;
        for _ in 0..n {
            v.extend_from_slice(alpha[(x % alpha.len() as u64) as usize].as_bytes());
            x /= 7;
            x += id;
        }
        v.push([b' ', b'\n', b',', b'.'][(lcg(seed) % 4) as usize]);
    }
    v
}

#[test]
fn large_files_match_the_reference() {
    let exe = build_tool();
    let d = tmp("big");
    let mut seed = 20261014u64;
    check(&exe, &d, "1MiB", &random_words(&mut seed, 1 << 20, 300));
    check(&exe, &d, "32MiB", &random_words(&mut seed, 32 << 20, 5000));
    // một từ rất dài (1 MiB) và đếm lớn
    let mut long = vec![b'x'; 1 << 20];
    long.push(b' ');
    long.extend_from_slice(&vec![b'x'; 1 << 20]);
    check(&exe, &d, "long_word_twice", &long);
    let mut many = Vec::new();
    for _ in 0..1_000_000 {
        many.extend_from_slice(b"a ");
    }
    check(&exe, &d, "million_a", &many);
}

#[test]
fn many_distinct_words() {
    let exe = build_tool();
    let d = tmp("distinct");
    // 200 000 từ phân biệt (số thập phân), mỗi từ một lần ⇒ hạng chỉ theo byte; cộng vài từ lặp để có đầu bảng
    let mut data = Vec::new();
    for i in 0..200_000u32 {
        data.extend_from_slice(format!("w{i} ").as_bytes());
    }
    data.extend_from_slice(b"w7 w7 w7 w9 w9 zz");
    check(&exe, &d, "distinct", &data);
}

/// ĐO (không phải cổng; -O0, mọi check giữ): thời gian và bộ nhớ đỉnh so với tham chiếu Rust. `--ignored --nocapture`.
#[test]
#[ignore = "đo hiệu năng: chạy bằng --ignored --nocapture"]
fn perf_report_against_rust() {
    let exe = build_tool();
    let d = tmp("perf");
    let mut seed = 424242u64;
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("1 MiB, 300 từ", random_words(&mut seed, 1 << 20, 300)),
        ("32 MiB, 5000 từ", random_words(&mut seed, 32 << 20, 5000)),
        (
            "4 MiB, 200000 từ",
            random_words(&mut seed, 4 << 20, 200_000),
        ),
    ];
    for (name, data) in &cases {
        let p = d.join("perf.txt");
        std::fs::write(&p, data).expect("ghi");
        let script = format!(
            "$psi = New-Object Diagnostics.ProcessStartInfo; $psi.FileName = '{}'; $psi.Arguments = '{}'; $psi.RedirectStandardOutput = $true; $psi.UseShellExecute = $false; $sw = [Diagnostics.Stopwatch]::StartNew(); $p = [Diagnostics.Process]::Start($psi); $t = $p.StandardOutput.BaseStream.CopyToAsync([IO.Stream]::Null); $m = 0; while (-not $p.HasExited) {{ try {{ $p.Refresh(); if ($p.PeakWorkingSet64 -gt $m) {{ $m = $p.PeakWorkingSet64 }} }} catch {{}}; Start-Sleep -Milliseconds 20 }}; $t.Wait(); $sw.Stop(); \"$($sw.Elapsed.TotalSeconds) $m\"",
            exe.display(),
            p.display()
        );
        let o = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .expect("powershell");
        let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
        let mut it = text.split_whitespace();
        let secs: f64 = it
            .next()
            .and_then(|x| x.replace(',', ".").parse().ok())
            .unwrap_or(f64::NAN);
        let peak: f64 = it.next().and_then(|x| x.parse().ok()).unwrap_or(f64::NAN);
        let t = std::time::Instant::now();
        let want = reference(data).expect("hợp lệ");
        let rust_secs = t.elapsed().as_secs_f64();
        println!(
            "{name}: wordfreq.gt {secs:.2} s, đỉnh working set {:.0} MiB | Rust (HashMap+sort) {rust_secs:.3} s | tỉ lệ thời gian ×{:.1} | (đầu ra {} byte)",
            peak / 1048576.0,
            secs / rust_secs.max(1e-9),
            want.len()
        );
    }
}
