//! K4 grep-lite — bản tham chiếu. Dùng: grep-lite [-c] [-n] PATTERN FILE... Đặc tả: docs/CAPSTONE.md.
use std::io::Write;

fn contains(h: &[u8], n: &[u8]) -> bool {
    n.is_empty() || h.windows(n.len()).any(|w| w == n)
}

fn main() {
    let mut count_only = false;
    let mut numbers = false;
    let mut rest: Vec<Vec<u8>> = Vec::new();
    let mut opts_done = false;
    for a in std::env::args_os().skip(1) {
        let b = a.to_string_lossy().into_owned().into_bytes();
        if !opts_done && b == b"-c" {
            count_only = true;
        } else if !opts_done && b == b"-n" {
            numbers = true;
        } else if !opts_done && b == b"--" {
            opts_done = true;
        } else {
            opts_done = true;
            rest.push(b);
        }
    }
    if rest.len() < 2 {
        eprintln!("usage: grep-lite [-c] [-n] PATTERN FILE...");
        std::process::exit(2);
    }
    let pat = rest[0].clone();
    let mut out: Vec<u8> = Vec::new();
    let mut matched = false;
    let mut error = false;
    for f in &rest[1..] {
        let path = String::from_utf8_lossy(f).to_string();
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(_) => {
                eprintln!("grep-lite: {path}: cannot open");
                error = true;
                continue;
            }
        };
        let mut lines: Vec<&[u8]> = data.split(|&b| b == b'\n').collect();
        if lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        let mut n = 0u64;
        for (i, l) in lines.iter().enumerate() {
            if contains(l, &pat) {
                n += 1;
                matched = true;
                if !count_only {
                    out.extend_from_slice(f);
                    out.push(b':');
                    if numbers {
                        out.extend_from_slice(format!("{}:", i + 1).as_bytes());
                    }
                    out.extend_from_slice(l);
                    out.push(b'\n');
                }
            }
        }
        if count_only {
            out.extend_from_slice(f);
            out.extend_from_slice(format!(":{n}\n").as_bytes());
        }
    }
    std::io::stdout().write_all(&out).ok();
    std::process::exit(if error { 2 } else if matched { 0 } else { 1 });
}
