//! K3 kvstore — bản tham chiếu. Lệnh từ stdin, mỗi dòng một lệnh. Đặc tả: docs/CAPSTONE.md.
use std::collections::HashMap;
use std::io::{Read, Write};

fn bad_key(k: &[u8]) -> bool {
    k.is_empty() || k.iter().any(|&b| b == b'=' || b == b' ' || b == b'\t')
}

fn main() {
    let mut data = Vec::new();
    std::io::stdin().read_to_end(&mut data).ok();
    let mut lines: Vec<&[u8]> = data.split(|&b| b == b'\n').collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    let mut m: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
    let mut out: Vec<u8> = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (cmd, rest) = match line.iter().position(|&b| b == b' ') {
            Some(p) => (&line[..p], Some(&line[p + 1..])),
            None => (line, None),
        };
        match (cmd, rest) {
            (b"set", Some(r)) => {
                let (k, v) = match r.iter().position(|&b| b == b' ') {
                    Some(p) => (&r[..p], &r[p + 1..]),
                    None => (r, &r[r.len()..]),
                };
                if bad_key(k) {
                    out.extend_from_slice(b"error: bad key\n");
                } else {
                    m.insert(k.to_vec(), v.to_vec());
                    out.extend_from_slice(b"ok\n");
                }
            }
            (b"get", Some(k)) => match m.get(k) {
                Some(v) => {
                    out.extend_from_slice(v);
                    out.push(b'\n');
                }
                None => out.extend_from_slice(b"(nil)\n"),
            },
            (b"del", Some(k)) => {
                out.extend_from_slice(if m.remove(k).is_some() { b"1\n" } else { b"0\n" });
            }
            (b"count", None) => out.extend_from_slice(format!("{}\n", m.len()).as_bytes()),
            (b"dump", None) => {
                let mut ks: Vec<&Vec<u8>> = m.keys().collect();
                ks.sort();
                for k in ks {
                    out.extend_from_slice(k);
                    out.push(b'=');
                    out.extend_from_slice(&m[k]);
                    out.push(b'\n');
                }
            }
            (b"save", Some(f)) => {
                let mut ks: Vec<&Vec<u8>> = m.keys().collect();
                ks.sort();
                let mut buf = Vec::new();
                for k in ks {
                    buf.extend_from_slice(k);
                    buf.push(b'=');
                    buf.extend_from_slice(&m[k]);
                    buf.push(b'\n');
                }
                let path = String::from_utf8_lossy(f).to_string();
                match std::fs::write(&path, &buf) {
                    Ok(()) => out.extend_from_slice(b"ok\n"),
                    Err(_) => out.extend_from_slice(b"error: cannot write\n"),
                }
            }
            (b"load", Some(f)) => {
                let path = String::from_utf8_lossy(f).to_string();
                match std::fs::read(&path) {
                    Err(_) => out.extend_from_slice(b"error: cannot read\n"),
                    Ok(d) => {
                        let mut ls: Vec<&[u8]> = d.split(|&b| b == b'\n').collect();
                        if ls.last().is_some_and(|l| l.is_empty()) {
                            ls.pop();
                        }
                        let mut staged: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
                        let mut err: Option<usize> = None;
                        for (i, l) in ls.iter().enumerate() {
                            match l.iter().position(|&b| b == b'=') {
                                Some(p) if !bad_key(&l[..p]) => staged.push((l[..p].to_vec(), l[p + 1..].to_vec())),
                                _ => {
                                    err = Some(i + 1);
                                    break;
                                }
                            }
                        }
                        match err {
                            Some(n) => out.extend_from_slice(format!("error: bad line {n}\n").as_bytes()),
                            None => {
                                let n = staged.len();
                                for (k, v) in staged {
                                    m.insert(k, v);
                                }
                                out.extend_from_slice(format!("loaded {n}\n").as_bytes());
                            }
                        }
                    }
                }
            }
            (b"set" | b"get" | b"del" | b"save" | b"load" | b"count" | b"dump", _) => {
                out.extend_from_slice(b"error: usage\n")
            }
            _ => out.extend_from_slice(b"error: unknown command\n"),
        }
    }
    std::io::stdout().write_all(&out).ok();
}
