//! Dogfood công cụ (EDITION 2): `examples/rpn` — máy tính hậu tố viết bằng `?`, `let` phân rã và or-pattern (ADR-P2-09/10/11). Đối chiếu độc lập với bản
//! tham chiếu Rust (đọc từng byte, cùng luật: ngăn xếp 16, số học u64 kiểm tràn, lỗi theo thứ tự gặp) về stdout/stderr/mã thoát trên đầu vào ngẫu nhiên.
use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("rig-rpn-{}-{n}-{name}", std::process::id()));
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

fn build_tool() -> (PathBuf, PathBuf) {
    let d = tmp("build");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/rpn"),
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
    (d.clone(), d.join("target/rpn.exe"))
}

/// tham chiếu độc lập: `Ok(giá trị)` hoặc `Err(thông báo)`
fn reference(src: &[u8]) -> Result<u64, &'static str> {
    let mut st: Vec<u64> = Vec::new();
    let (mut cur, mut have) = (0u64, false);
    fn flush(st: &mut Vec<u64>, cur: u64, have: bool) -> Result<(), &'static str> {
        if have {
            if st.len() >= 16 {
                return Err("ngăn xếp đầy");
            }
            st.push(cur);
        }
        Ok(())
    }
    for &c in src {
        match c {
            b' ' | b'\t' => {
                flush(&mut st, cur, have)?;
                cur = 0;
                have = false;
            }
            b'0'..=b'9' => {
                cur = cur
                    .checked_mul(10)
                    .and_then(|t| t.checked_add(u64::from(c - b'0')))
                    .ok_or("tràn số")?;
                have = true;
            }
            b'+' | b'-' | b'*' | b'/' => {
                flush(&mut st, cur, have)?;
                cur = 0;
                have = false;
                let b = st.pop().ok_or("thiếu toán hạng")?;
                let a = st.pop().ok_or("thiếu toán hạng")?;
                let r = match c {
                    b'+' => a.checked_add(b),
                    b'-' => a.checked_sub(b),
                    b'*' => a.checked_mul(b),
                    _ => {
                        if b == 0 {
                            return Err("chia cho 0");
                        }
                        Some(a / b)
                    }
                };
                st.push(r.ok_or("tràn số")?);
            }
            _ => return Err("ký tự lạ"),
        }
    }
    flush(&mut st, cur, have)?;
    let r = st.pop().ok_or("biểu thức rỗng")?;
    if st.is_empty() {
        Ok(r)
    } else {
        Err("còn dư toán hạng")
    }
}

#[test]
fn rpn_matches_an_independent_rust_reference_on_random_inputs() {
    let (_dir, exe) = build_tool();
    let mut s = 0x1234_5678_9ABC_DEF1u64;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let atoms: [&str; 18] = [
        "0",
        "1",
        "2",
        "7",
        "10",
        "255",
        "99999",
        "4294967296",
        "18446744073709551615",
        "18446744073709551616",
        "+",
        "-",
        "*",
        "/",
        "+",
        "x",
        " ",
        "\t",
    ];
    let mut cases: Vec<String> = [
        "3 4 + 2 *",
        "1 +",
        "4 0 /",
        "2 3",
        "",
        "1 x",
        "10 3 -",
        "1 2 3 + +",
        "  7  ",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    for _ in 0..120 {
        let n = 1 + (next() % 14) as usize;
        let mut e = String::new();
        for _ in 0..n {
            let a = atoms[(next() % atoms.len() as u64) as usize];
            e.push_str(a);
            if a.len() > 1 || a.as_bytes()[0].is_ascii_digit() || next() % 3 != 0 {
                e.push(' ');
            }
        }
        cases.push(e);
    }
    // ngăn xếp đầy: 17 số rồi toán tử
    cases.push(format!("{}+", "1 ".repeat(17)));
    let (mut oks, mut errs) = (0, 0);
    for c in &cases {
        let o = Command::new(&exe).arg(c).output().expect("chạy rpn");
        match reference(c.as_bytes()) {
            Ok(v) => {
                oks += 1;
                assert_eq!(o.status.code(), Some(0), "{c:?}");
                assert_eq!(
                    String::from_utf8_lossy(&o.stdout),
                    format!("{v}\n"),
                    "{c:?}"
                );
            }
            Err(m) => {
                errs += 1;
                assert_eq!(o.status.code(), Some(1), "{c:?}");
                assert_eq!(
                    String::from_utf8_lossy(&o.stderr),
                    format!("rpn: {m}\n"),
                    "{c:?}"
                );
            }
        }
    }
    assert!(oks >= 8 && errs >= 30, "bộ sinh lệch: ok={oks} err={errs}");
}
