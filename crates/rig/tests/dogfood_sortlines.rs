//! Dogfood công cụ 4: `examples/sortlines` (Grit, dự án Rig, docs/DOGFOOD.md). Đối chiếu độc lập với bản tham chiếu Rust (`sort_by` theo byte,
//! ỔN ĐỊNH) về TỪNG BYTE đầu ra. Quy ước chung của hai bên (README của dự án): chỉ tách theo `\n`; `\r` là một byte của dòng; mỗi dòng ra
//! kèm `\n` (kể cả dòng cuối không có `\n`); tệp rỗng ra rỗng; so sánh từ điển theo byte.
//! Tính ổn định KHÔNG quan sát được ở đây (hai dòng bằng nhau về byte không phân biệt được): đã kiểm riêng ở `gt-driver/tests/llvm_sort.rs`.
use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("rig-sl-{}-{n}-{name}", std::process::id()));
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
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sortlines"),
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
    d.join("target/sortlines.exe")
}

/// tham chiếu độc lập
fn reference(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&[u8]> = data.split(|b| *b == b'\n').collect();
    if data.last() == Some(&b'\n') {
        lines.pop();
    }
    lines.sort(); // `[u8]: Ord` = từ điển theo byte; sắp xếp ổn định của std (tương đương `sort_by(|a, b| a.cmp(b))`)
    let mut out = Vec::with_capacity(data.len() + 1);
    for l in lines {
        out.extend_from_slice(l);
        out.push(b'\n');
    }
    out
}

fn lcg(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed >> 33
}

fn run_tool(exe: &Path, path: &Path, env: &[(&str, String)]) -> (Option<i32>, Vec<u8>, String) {
    let mut c = Command::new(exe);
    c.arg(path);
    for (k, v) in env {
        c.env(k, v);
    }
    let o = c.output().expect("chạy sortlines");
    (
        o.status.code(),
        o.stdout,
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn check_bytes(exe: &Path, dir: &Path, name: &str, data: &[u8]) {
    let p = dir.join(name);
    std::fs::write(&p, data).expect("ghi");
    let (code, out, err) = run_tool(exe, &p, &[]);
    assert_eq!(code, Some(0), "{name}: {err}");
    let want = reference(data);
    assert!(
        out == want,
        "{name}: đầu ra khác bản tham chiếu ({} byte vs {} byte)",
        out.len(),
        want.len()
    );
    let _ = std::fs::remove_file(&p);
}

#[test]
fn small_cases_match_the_reference_byte_for_byte() {
    let exe = build_tool();
    let d = tmp("small");
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", b"".to_vec()),
        ("only_newline", b"\n".to_vec()),
        ("two_newlines", b"\n\n".to_vec()),
        ("one_line_with_nl", b"hello\n".to_vec()),
        ("one_line_no_nl", b"hello".to_vec()),
        ("blank_interleaved", b"b\n\na\n\n\nc\n".to_vec()),
        ("crlf", b"b\r\na\r\nc\r\n".to_vec()),
        ("crlf_mixed", b"b\r\na\nc\r\na\r\n".to_vec()),
        ("no_trailing_nl", b"zeta\nalpha\nmid".to_vec()),
        (
            "utf8_and_high_bytes",
            "é\nz\nÿ\n\u{1F600}\na\n".as_bytes().to_vec(),
        ),
        (
            "raw_high_bytes",
            vec![0xff, b'\n', 0x80, b'\n', 0x7f, b'\n', 0x00, b'\n'],
        ),
        ("nul_inside", b"a\0b\na\0a\na\n\0\n".to_vec()),
        ("duplicates", b"x\nb\nx\na\nb\nx\n".to_vec()),
        ("prefix_order", b"ab\na\nabc\nab\n".to_vec()),
        ("cr_only_no_split", b"a\rb\rc".to_vec()),
    ];
    for (n, data) in &cases {
        check_bytes(&exe, &d, n, data);
    }
}

#[test]
fn error_paths_use_stderr_and_exit_code_2() {
    let exe = build_tool();
    let d = tmp("err");
    let (code, out, err) = run_tool(&exe, &d.join("khong_co.txt"), &[]);
    assert_eq!(code, Some(2));
    assert!(err.ends_with(": no such file\n"), "{err}");
    assert!(err.starts_with("sortlines: "), "{err}");
    assert!(out.is_empty());
    let o = Command::new(&exe).output().expect("chạy");
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "usage: sortlines <file>\n"
    );
    assert!(o.stdout.is_empty());
}

#[test]
fn one_very_long_line_of_1_mib() {
    let exe = build_tool();
    let d = tmp("long");
    let mut seed = 3u64;
    let line: Vec<u8> = (0..1024 * 1024)
        .map(|_| {
            let b = (lcg(&mut seed) % 255 + 1) as u8;
            if b == b'\n' { b'x' } else { b }
        })
        .collect();
    check_bytes(&exe, &d, "long_no_nl", &line);
    let mut with_nl = line.clone();
    with_nl.push(b'\n');
    check_bytes(&exe, &d, "long_nl", &with_nl);
    // hai dòng dài chung tiền tố 1 MiB chỉ khác byte cuối (so sánh phải đi hết)
    let mut a = line.clone();
    let mut b = line.clone();
    a.push(b'a');
    b.push(b'b');
    let mut two = b.clone();
    two.push(b'\n');
    two.extend_from_slice(&a);
    check_bytes(&exe, &d, "long_two_prefix", &two);
}

fn random_text(seed: &mut u64, bytes: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(bytes);
    while v.len() < bytes {
        let len = (lcg(seed) % 40) as usize;
        for _ in 0..len {
            // chữ thường, một ít byte cao/NUL/CR để thử so sánh byte
            let r = lcg(seed) % 100;
            v.push(match r {
                0 => 0,
                1 => 0xff,
                2 => b'\r',
                3 => 0xc3,
                _ => b'a' + (lcg(seed) % 6) as u8,
            });
        }
        v.push(b'\n');
    }
    v
}

#[test]
fn random_text_1_mib_and_32_mib() {
    let exe = build_tool();
    let d = tmp("big");
    let mut seed = 20261014u64;
    for (name, mib) in [("1MiB", 1usize), ("32MiB", 32)] {
        let data = random_text(&mut seed, mib * 1024 * 1024);
        check_bytes(&exe, &d, name, &data);
    }
}

#[test]
fn one_million_short_lines() {
    let exe = build_tool();
    let d = tmp("million");
    let mut seed = 77u64;
    let mut data = Vec::new();
    for _ in 0..1_000_000 {
        let len = (lcg(&mut seed) % 8) as usize;
        for _ in 0..len {
            data.push(b'a' + (lcg(&mut seed) % 4) as u8);
        }
        data.push(b'\n');
    }
    check_bytes(&exe, &d, "million", &data);
}

/// 4 MiB toàn dòng rỗng (4 194 304 dòng): trường hợp xấu nhất về SỐ dòng mà vẫn an toàn bộ nhớ (≈ 2 · 4M · 16 B = 128 MiB). KHÔNG chạy cỡ
/// 32 MiB toàn dòng rỗng (≈ 33,5 triệu dòng, ≈ 1 GiB) — xem KNOWN_LIMITATIONS.
#[test]
fn four_mib_of_empty_lines() {
    let exe = build_tool();
    let d = tmp("empty");
    check_bytes(&exe, &d, "empties", &vec![b'\n'; 4 * 1024 * 1024]);
    // toàn dòng rỗng cỡ vừa và tệp không có xuống dòng cuối
    check_bytes(&exe, &d, "empties_small", &vec![b'\n'; 1000]);
}

/// Công thức Arena: mỗi Vec `reserve(số dòng)` cần đúng MỘT khối `max(1 MiB, số_dòng·16 + 8)` byte. Giới hạn = khối đó thì chạy được trên đầu
/// vào hợp lệ (arena_exhausted KHÔNG xảy ra); nhỏ hơn 1 byte (khi khối > 1 MiB) thì abort có loại `arena_exhausted`, không hỏng.
#[test]
fn arena_formula_is_exact_and_exhaustion_is_controlled() {
    let exe = build_tool();
    let d = tmp("arena");
    // tệp toàn dòng rỗng 200 000 dòng và tệp 100 000 dòng ngắn: số dòng lớn hơn 65 536 ⇒ khối > 1 MiB
    let mut seed = 5u64;
    let mut shorts = Vec::new();
    for _ in 0..100_000 {
        shorts.push(b'a' + (lcg(&mut seed) % 5) as u8);
        shorts.push(b'\n');
    }
    for (name, data, nl) in [
        ("empties200k", vec![b'\n'; 200_000], 200_000usize),
        ("shorts100k", shorts, 100_000usize),
    ] {
        let p = d.join(name);
        std::fs::write(&p, &data).expect("ghi");
        let block = (nl * 16 + 8).max(1 << 20);
        let (code, out, err) = run_tool(&exe, &p, &[("GRITRT_ARENA_LIMIT", block.to_string())]);
        assert_eq!(code, Some(0), "{name}: {err}");
        assert!(
            out == reference(&data),
            "{name}: đầu ra sai với giới hạn đúng bằng công thức"
        );
        let (code, _, err) = run_tool(&exe, &p, &[("GRITRT_ARENA_LIMIT", (block - 1).to_string())]);
        assert_eq!(code, Some(70), "{name}: {err}");
        assert!(err.contains("arena_exhausted"), "{name}: {err}");
    }
    // tệp nhỏ: khối là chunk 1 MiB mặc định ⇒ giới hạn 1 MiB đủ
    let small = d.join("tiny");
    std::fs::write(&small, b"b\na\n").expect("ghi");
    let (code, out, _) = run_tool(
        &exe,
        &small,
        &[("GRITRT_ARENA_LIMIT", (1usize << 20).to_string())],
    );
    assert_eq!((code, out.as_slice()), (Some(0), &b"a\nb\n"[..]));
}

/// ĐO (không phải cổng; -O0, mọi check giữ): thời gian và bộ nhớ đỉnh (PeakWorkingSet64 lấy mẫu mỗi 20 ms qua PowerShell) so với Rust
/// `sort_by` ổn định trên cùng dữ liệu. Chạy bằng `--ignored --nocapture`.
#[test]
#[ignore = "đo hiệu năng: chạy bằng --ignored --nocapture"]
fn perf_report_against_rust_sort_by() {
    let exe = build_tool();
    let d = tmp("perf");
    let mut seed = 424242u64;
    let mut million = Vec::new();
    for _ in 0..1_000_000 {
        let len = (lcg(&mut seed) % 8) as usize;
        for _ in 0..len {
            million.push(b'a' + (lcg(&mut seed) % 4) as u8);
        }
        million.push(b'\n');
    }
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("1MiB text", random_text(&mut seed, 1 << 20)),
        ("32MiB text", random_text(&mut seed, 32 << 20)),
        ("1e6 short lines", million),
        ("4MiB empty lines", vec![b'\n'; 4 << 20]),
    ];
    for (name, data) in &cases {
        let p = d.join("perf.txt");
        std::fs::write(&p, data).expect("ghi");
        // chạy qua ống (pipe) như người dùng thật; đầu ra được rút bỏ trong PowerShell và KHÔNG kiểm ở đây (đúng/sai đã kiểm ở test khác)
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
        let want = reference(data);
        let rust_secs = t.elapsed().as_secs_f64();
        // bộ nhớ ước tính của tham chiếu Rust: tệp + Vec<&[u8]> (16 B/dòng) + đầu ra
        let nl = data.iter().filter(|b| **b == b'\n').count() as f64;
        let rust_mb = (data.len() as f64 * 2.0 + nl * 16.0) / 1048576.0;
        println!(
            "{name}: sortlines.gt {secs:.2} s, đỉnh working set {:.0} MiB | Rust sort_by {rust_secs:.3} s (≈ {rust_mb:.0} MiB ước tính) | tỉ lệ thời gian ×{:.1} | (đầu ra {} byte)",
            peak / 1048576.0,
            secs / rust_secs.max(1e-9),
            want.len()
        );
    }
}
