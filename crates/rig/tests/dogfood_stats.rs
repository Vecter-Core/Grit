//! Dogfood công cụ 6: `examples/stats` (Grit, dự án Rig, dùng `f64`; docs/DOGFOOD.md). Đối chiếu độc lập với bản tham chiếu Rust (CÙNG công thức và CÙNG
//! thứ tự phép tính f64 ⇒ cùng mẫu bit IEEE-754; Rust tính giá trị kỳ vọng) về TỪNG BYTE đầu ra. Quy ước chung (README của dự án): số thập phân không
//! âm cách nhau bởi khoảng trắng ASCII, mỗi số ≤ 2^53; token sai hoặc quá lớn ⇒ stderr + mã 2; in `count`/`mean`/`stddev` làm tròn nửa lên.
use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("rig-st-{}-{n}-{name}", std::process::id()));
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
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/stats"),
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
    d.join("target/stats.exe")
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r')
}

/// tham chiếu độc lập: `Err(())` nếu token sai/quá lớn
fn reference(data: &[u8]) -> Result<String, ()> {
    let mut xs: Vec<u64> = Vec::new();
    for tok in data.split(|b| is_space(*b)).filter(|t| !t.is_empty()) {
        let mut v = 0u64;
        for &c in tok {
            if !c.is_ascii_digit() {
                return Err(());
            }
            v = v * 10 + u64::from(c - b'0');
            if v > 9_007_199_254_740_992 {
                return Err(());
            }
        }
        xs.push(v);
    }
    let n = xs.len();
    let (mut mean, mut sd) = (0.0f64, 0.0f64);
    if n > 0 {
        let mut sum = 0.0f64;
        for &x in &xs {
            sum += x as f64;
        }
        let nf = n as f64;
        mean = sum / nf;
        let mut acc = 0.0f64;
        for &x in &xs {
            let d = x as f64 - mean;
            acc += d * d;
        }
        sd = (acc / nf).sqrt();
    }
    let round = |x: f64| -> u64 { (x + 0.5) as i64 as u64 };
    Ok(format!(
        "count {n}\nmean {}\nstddev {}\n",
        round(mean),
        round(sd)
    ))
}

fn lcg(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed >> 33
}

fn run_tool(exe: &Path, path: &Path) -> (Option<i32>, Vec<u8>, String) {
    let o = Command::new(exe).arg(path).output().expect("chạy stats");
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
            assert_eq!(String::from_utf8_lossy(&out), want, "{name}");
        }
        Err(()) => {
            assert_eq!(
                code,
                Some(2),
                "{name}: token sai phải là lỗi có kiểm soát: {err}"
            );
            assert!(out.is_empty(), "{name}: stdout phải rỗng");
            assert!(err.starts_with("stats: "), "{name}: {err}");
        }
    }
    let _ = std::fs::remove_file(&p);
}

#[test]
fn small_cases_match_the_reference_byte_for_byte() {
    let exe = build_tool();
    let d = tmp("small");
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", b"".to_vec()),
        ("only_whitespace", b" \n\t\r\n".to_vec()),
        ("single_zero", b"0".to_vec()),
        ("single", b"42\n".to_vec()),
        ("constant_stddev_zero", b"7 7 7 7 7".to_vec()),
        ("one_to_six", b"1 2 3 4\n5 6\n".to_vec()),
        ("crlf_and_tabs", b"10\r\n20\t30\r\n40".to_vec()),
        ("leading_zeros", b"007 0010 000".to_vec()),
        ("rounding_half_up", b"1 2".to_vec()),
        ("rounding_below_half", b"0 0 1".to_vec()),
        ("two_pow_53", b"9007199254740992 9007199254740992".to_vec()),
        (
            "near_two_pow_53",
            b"9007199254740991 1 9007199254740992".to_vec(),
        ),
        (
            "many_digits_ok",
            b"4503599627370496 4503599627370497 1".to_vec(),
        ),
        ("unsorted_spread", b"1000000 1 999999 2 500000 3".to_vec()),
    ];
    for (n, data) in &cases {
        check(&exe, &d, n, data);
    }
}

#[test]
fn bad_tokens_and_too_large_values_are_controlled_errors() {
    let exe = build_tool();
    let d = tmp("bad");
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("letter", b"1 2 x 4".to_vec()),
        ("negative", b"-5".to_vec()),
        ("decimal_point", b"1.5".to_vec()),
        ("hex", b"0x10".to_vec()),
        ("plus_sign", b"+3".to_vec()),
        ("utf8_digit", "١٢٣".as_bytes().to_vec()),
        ("comma", b"1,2".to_vec()),
        ("nul_byte", b"1\x002".to_vec()),
        ("too_large_by_one", b"9007199254740993".to_vec()),
        ("way_too_large", b"99999999999999999999999999".to_vec()),
        ("too_large_after_good", b"1 2 3 9007199254740993".to_vec()),
    ];
    for (n, data) in &cases {
        assert!(reference(data).is_err(), "{n}: tham chiếu phải coi là lỗi");
        check(&exe, &d, n, data);
    }
    let (code, out, err) = run_tool(&exe, &d.join("khong_co"));
    assert_eq!(code, Some(2));
    assert!(err.ends_with(": no such file\n") && out.is_empty(), "{err}");
    let o = Command::new(&exe).output().expect("chạy");
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&o.stderr), "usage: stats <file>\n");
}

#[test]
fn random_and_large_inputs_match_the_reference() {
    let exe = build_tool();
    let d = tmp("big");
    let mut seed = 20261014u64;
    // 200 tệp ngẫu nhiên nhỏ (độ dài và độ lớn đa dạng; một số gần 2^53)
    for i in 0..200 {
        let n = (lcg(&mut seed) % 40) as usize;
        let mut data = Vec::new();
        for _ in 0..n {
            let v = match lcg(&mut seed) % 4 {
                0 => lcg(&mut seed) % 10,
                1 => lcg(&mut seed) % 1000,
                2 => lcg(&mut seed) * 1_000_003,
                _ => 9_007_199_254_740_992 - lcg(&mut seed) % 1000,
            };
            data.extend_from_slice(v.to_string().as_bytes());
            data.push(b" \n\t\r"[(lcg(&mut seed) % 4) as usize]);
        }
        check(&exe, &d, &format!("r{i}"), &data);
    }
    // 10^6 số và 32 MiB văn bản
    let mut million = Vec::new();
    for _ in 0..1_000_000 {
        million.extend_from_slice((lcg(&mut seed) % 100_000).to_string().as_bytes());
        million.push(b'\n');
    }
    check(&exe, &d, "million", &million);
    let mut big = Vec::new();
    while big.len() < 32 << 20 {
        big.extend_from_slice(
            (lcg(&mut seed) * 1_000_003 % 9_000_000_000_000_000)
                .to_string()
                .as_bytes(),
        );
        big.push(b' ');
    }
    check(&exe, &d, "32MiB", &big);
}

/// ĐO (không phải cổng; -O0, mọi check giữ): thời gian và bộ nhớ đỉnh so với tham chiếu Rust. `--ignored --nocapture`.
#[test]
#[ignore = "đo hiệu năng: chạy bằng --ignored --nocapture"]
fn perf_report_against_rust() {
    let exe = build_tool();
    let d = tmp("perf");
    let mut seed = 424242u64;
    let mut million = Vec::new();
    for _ in 0..1_000_000 {
        million.extend_from_slice((lcg(&mut seed) % 100_000).to_string().as_bytes());
        million.push(b'\n');
    }
    let mut big = Vec::new();
    while big.len() < 32 << 20 {
        big.extend_from_slice(
            (lcg(&mut seed) * 1_000_003 % 9_000_000_000_000_000)
                .to_string()
                .as_bytes(),
        );
        big.push(b' ');
    }
    for (name, data) in [
        ("10^6 số (≈ 6,9 MB)", million),
        ("32 MiB (≈ 1,9 triệu số)", big),
    ] {
        let p = d.join("perf.txt");
        std::fs::write(&p, &data).expect("ghi");
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
        let want = reference(&data).expect("hợp lệ");
        let rust_secs = t.elapsed().as_secs_f64();
        println!(
            "{name}: stats.gt {secs:.2} s, đỉnh working set {:.0} MiB | Rust {rust_secs:.3} s | tỉ lệ thời gian ×{:.1} | (đầu ra {} byte)",
            peak / 1048576.0,
            secs / rust_secs.max(1e-9),
            want.len()
        );
    }
}
