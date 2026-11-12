//! K1 csvstat — bản tham chiếu. Dùng: csvstat [FILE|-]. Đặc tả đầy đủ: docs/CAPSTONE.md.
use std::io::{Read, Write};

fn parse_i64(f: &[u8]) -> Option<i64> {
    let (neg, digits) = match f.first() {
        Some(b'-') => (true, &f[1..]),
        _ => (false, f),
    };
    if digits.is_empty() || digits.len() > 19 || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let mut v: i128 = 0;
    for d in digits {
        v = v * 10 + i128::from(d - b'0');
    }
    if neg {
        v = -v;
    }
    i64::try_from(v).ok()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut data = Vec::new();
    match args.as_slice() {
        [] => {
            std::io::stdin().read_to_end(&mut data).ok();
        }
        [p] if p == "-" => {
            std::io::stdin().read_to_end(&mut data).ok();
        }
        [p] => match std::fs::read(p) {
            Ok(d) => data = d,
            Err(_) => {
                eprintln!("csvstat: cannot read {p}");
                std::process::exit(2);
            }
        },
        _ => {
            eprintln!("usage: csvstat [FILE|-]");
            std::process::exit(2);
        }
    }
    let mut lines: Vec<&[u8]> = data.split(|&b| b == b'\n').collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        return;
    }
    let header: Vec<&[u8]> = lines[0].split(|&b| b == b',').collect();
    let k = header.len();
    let mut numeric = vec![true; k];
    let mut count = vec![0u64; k];
    let mut sum = vec![0i64; k];
    let mut min = vec![0i64; k];
    let mut max = vec![0i64; k];
    for (idx, line) in lines.iter().enumerate().skip(1) {
        if line.is_empty() {
            continue;
        }
        let f: Vec<&[u8]> = line.split(|&b| b == b',').collect();
        if f.len() != k {
            eprintln!("csvstat: line {}: expected {} fields, found {}", idx + 1, k, f.len());
            std::process::exit(1);
        }
        for c in 0..k {
            if !numeric[c] {
                continue;
            }
            match parse_i64(f[c]) {
                None => numeric[c] = false,
                Some(v) => {
                    match sum[c].checked_add(v) {
                        Some(s) => sum[c] = s,
                        None => {
                            eprintln!("csvstat: line {}: overflow", idx + 1);
                            std::process::exit(1);
                        }
                    }
                    if count[c] == 0 || v < min[c] {
                        min[c] = v;
                    }
                    if count[c] == 0 || v > max[c] {
                        max[c] = v;
                    }
                    count[c] += 1;
                }
            }
        }
    }
    let mut out = Vec::new();
    for c in 0..k {
        out.extend_from_slice(header[c]);
        if !numeric[c] {
            out.extend_from_slice(b": non-numeric\n");
        } else if count[c] == 0 {
            out.extend_from_slice(b": count=0\n");
        } else {
            let n = i128::from(count[c]);
            let s = i128::from(sum[c]);
            let ip = s / n;
            let fp = (s % n).abs() * 10000 / n;
            let sign = if s < 0 && ip == 0 { "-" } else { "" };
            out.extend_from_slice(
                format!(": count={} sum={} min={} max={} mean={}{}.{:04}\n", count[c], sum[c], min[c], max[c], sign, ip, fp).as_bytes(),
            );
        }
    }
    std::io::stdout().write_all(&out).ok();
}
