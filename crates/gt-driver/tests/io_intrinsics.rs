//! Seed 1.0 S2 — bốn intrinsic mới (TCB; docs/TCB_INTRINSICS.md): `Vec::truncate`, `str::as_bytes`, `Cap<In>::read_all`, `Cap<Fs>::write`.
//! Ba nguồn đối chiếu: (1) mô hình Rust độc lập ở đây, (2) thông dịch MIR (chỉ cho hai intrinsic thuần), (3) exe LLVM thật. Mỗi intrinsic ≥ 30 chương trình/ca
//! (hạt giống cố định). Hồ sơ: `GT_PROFILE=dev|release|release-o2` (mặc định dev).
#![cfg(feature = "llvm")]
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn profile() -> String {
    std::env::var("GT_PROFILE").unwrap_or_else(|_| "dev".to_string())
}

fn scratch(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("gt-iox-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// `gritc run` một chương trình thuần (không I/O) ở backend cho trước; trả giá trị `main`.
fn run_value(src: &str, backend: &str) -> String {
    let d = scratch(backend);
    let f = d.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args([
            "run",
            "--json",
            &format!("--backend={backend}"),
            &format!("--profile={}", profile()),
        ])
        .arg(&f)
        .output()
        .expect("chạy gritc");
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let line = text
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .unwrap_or("");
    let v: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|e| {
        panic!(
            "JSON hỏng ({e}): {text}\n{}",
            String::from_utf8_lossy(&o.stderr)
        )
    });
    assert_eq!(o.status.code(), Some(0), "{backend}: {text}\n{src}");
    let raw = v["run"]["value"].as_str().unwrap_or("").to_string();
    let _ = std::fs::remove_dir_all(&d);
    raw.strip_prefix("Int(")
        .and_then(|s| s.split(',').next())
        .map_or(raw.clone(), str::to_string)
}

// =============================================================================================== Vec::truncate
/// 40 chương trình: dãy ngẫu nhiên các thao tác `push`, `truncate(k)` (k có thể > len: không đổi), rồi băm nội dung + len. Mô hình: `Vec::truncate` của Rust.
#[test]
fn vec_truncate_matches_the_model_on_both_backends() {
    let mut rng = Rng(0x7EC7_0001);
    for case in 0..40 {
        let mut model: Vec<u64> = Vec::new();
        let mut body = String::new();
        for _ in 0..(3 + rng.below(25)) {
            match rng.below(4) {
                0 | 1 => {
                    let x = rng.below(1000);
                    model.push(x);
                    body.push_str(&format!("  v.push({x}u64);\n"));
                }
                2 => {
                    let k = rng.below(model.len() as u64 + 4);
                    model.truncate(usize::try_from(k).expect("k"));
                    body.push_str(&format!("  v.truncate({k}usize);\n"));
                }
                _ => {
                    model.clear();
                    body.push_str("  v.truncate(0usize);\n");
                }
            }
        }
        let mut h: u64 = model.len() as u64;
        for x in &model {
            h = h.wrapping_mul(31).wrapping_add(*x);
        }
        let src = format!(
            "ƒ main() → u64 ! {{mem}} {{
  μar ≔ Arena::new();
  μv ≔ Vec::new(&ar);
{body}  s ≔ v.as_slice();
  μh ≔ v.len() as u64;
  μi ≔ 0usize;
  ⟳ i < s.len() ↓ s.len() - i {{ h ← h *% 31u64 +% s[i]; i ← i + 1usize; }}
  h
}}
"
        );
        let want = h.to_string();
        assert_eq!(run_value(&src, "llvm"), want, "llvm, ca {case}\n{src}");
        assert_eq!(
            run_value(&src, "interpreter"),
            want,
            "thông dịch, ca {case}\n{src}"
        );
    }
}

/// `truncate` không bao giờ TĂNG độ dài và sau `truncate(n)` rồi `push` phần tử mới ghi đè đúng chỗ.
#[test]
fn vec_truncate_never_grows_and_push_reuses_the_slot() {
    let src = "ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  μv ≔ Vec::new(&ar);
  v.push(10u64); v.push(20u64); v.push(30u64);
  v.truncate(1usize);
  v.truncate(99usize);
  v.push(77u64);
  s ≔ v.as_slice();
  (v.len() as u64) * 1000000u64 + s[0] * 1000u64 + s[1]
}
";
    for b in ["llvm", "interpreter"] {
        assert_eq!(run_value(src, b), "2010077", "{b}");
    }
}

// =============================================================================================== str::as_bytes
/// 40 chuỗi (ASCII, rỗng, nhiều byte UTF-8, có `\n`): tổng có trọng số các byte + độ dài. Mô hình: `str::as_bytes` của Rust.
#[test]
fn str_as_bytes_matches_the_model_on_both_backends() {
    let samples: Vec<String> = {
        let mut v: Vec<String> = [
            "",
            "a",
            "ab",
            "hello world",
            "Xin chào Việt Nam",
            "日本語",
            "😀x😀",
            "tab\there",
            "line1\\nline2",
            "ÿ",
            "é",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
        let mut rng = Rng(0x57A5_0001);
        let alphabet: Vec<char> = "abcXYZ019 _-éàạ日😀".chars().collect();
        while v.len() < 40 {
            let n = rng.below(12);
            v.push(
                (0..n)
                    .map(|_| {
                        alphabet[usize::try_from(rng.below(alphabet.len() as u64)).expect("i")]
                    })
                    .collect(),
            );
        }
        v
    };
    for (i, s) in samples.iter().enumerate() {
        let mut h: u64 = s.len() as u64;
        for b in s.as_bytes() {
            h = h.wrapping_mul(131).wrapping_add(u64::from(*b));
        }
        // chuỗi Grit: thoát `"` và `\`; ký tự khác (kể cả nhiều byte) viết thẳng
        let lit = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\t', "\\t")
            .replace('\n', "\\n");
        let src = format!(
            "ƒ main() → u64 ! ∅ {{
  s ≔ \"{lit}\";
  b ≔ s.as_bytes();
  μh ≔ b.len() as u64;
  μi ≔ 0usize;
  ⟳ i < b.len() ↓ b.len() - i {{ h ← h *% 131u64 +% (b[i] as u64); i ← i + 1usize; }}
  h
}}
"
        );
        let want = h.to_string();
        assert_eq!(run_value(&src, "llvm"), want, "llvm, ca {i}: {s:?}\n{src}");
        assert_eq!(
            run_value(&src, "interpreter"),
            want,
            "thông dịch, ca {i}: {s:?}\n{src}"
        );
    }
}

// =============================================================================================== exe có I/O
const STDIN_SRC: &str = "ƒ main() → i32 ! {io} {
  out ≔ root_out();
  ⌥ root_in().read_all() {
    Ok(f) ↦ {
      b ≔ f.bytes();
      μh ≔ b.len() as u64;
      μi ≔ 0usize;
      ⟳ i < b.len() ↓ b.len() - i { h ← h *% 131u64 +% (b[i] as u64); i ← i + 1usize; }
      out.write_u64(b.len() as u64);
      out.write_str(\" \");
      out.write_u64(h);
      0
    },
    Err(_) ↦ { out.write_str(\"stdin error\"); 1 },
  }
}
";

const WRITE_SRC: &str = "ƒ main() → i32 ! {io} {
  out ≔ root_out();
  env ≔ root_env();
  fs ≔ root_fs();
  ⌥ root_in().read_all() {
    Ok(f) ↦ {
      ⌥ fs.write(env.arg(1usize), f.bytes()) {
        Ok(_) ↦ { out.write_str(\"ok\"); },
        Err(e) ↦ {
          out.write_str(\"err \");
          k ≔ ⌥ e { IoError::NotFound ↦ 0u64, IoError::Denied ↦ 1u64, IoError::Invalid ↦ 2u64, IoError::Other ↦ 3u64 };
          out.write_u64(k);
        },
      }
      0
    },
    Err(_) ↦ { 1 },
  }
}
";

fn build(src: &str, name: &str) -> (PathBuf, PathBuf) {
    let d = scratch(name);
    let f = d.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let exe = d.join("prog.exe");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args(["build", &format!("--profile={}", profile()), "-o"])
        .arg(&exe)
        .arg(&f)
        .output()
        .expect("chạy gritc");
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    (d, exe)
}

fn run_exe(exe: &Path, args: &[&str], stdin: &[u8]) -> (i32, Vec<u8>) {
    let mut c = Command::new(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("chạy exe");
    {
        let mut si = c.stdin.take().expect("stdin");
        // ghi trên luồng riêng: đầu vào lớn không được làm treo khi exe chưa đọc
        let data = stdin.to_vec();
        std::thread::spawn(move || {
            let _ = si.write_all(&data);
        });
    }
    let o = c.wait_with_output().expect("chờ exe");
    (o.status.code().unwrap_or(-1), o.stdout)
}

fn fnv_like(b: &[u8]) -> u64 {
    let mut h = b.len() as u64;
    for x in b {
        h = h.wrapping_mul(131).wrapping_add(u64::from(*x));
    }
    h
}

/// 36 đầu vào stdin (rỗng, 1 byte, byte NUL/cao, không xuống dòng cuối, CRLF, 1 MiB ngẫu nhiên, 4 MiB `\n`, UTF-8 sai…): độ dài và băm khớp mô hình.
#[test]
fn stdin_read_all_matches_the_model() {
    let (d, exe) = build(STDIN_SRC, "stdin");
    let mut rng = Rng(0x57D1_0001);
    let mut inputs: Vec<Vec<u8>> = vec![
        vec![],
        b"a".to_vec(),
        b"\n".to_vec(),
        vec![0],
        vec![255],
        b"no newline".to_vec(),
        b"line\r\nline\r\n".to_vec(),
        (0..=255u8).collect(),
        "Xin chào\n日本語\n".as_bytes().to_vec(),
        vec![0xC3, 0x28, 0xA0, 0xA1, 0xE2, 0x28, 0xA1],
        vec![b'\n'; 4 << 20],
    ];
    while inputs.len() < 35 {
        let n = match rng.below(4) {
            0 => rng.below(8),
            1 => rng.below(300),
            2 => rng.below(70_000),
            _ => 1 << 20,
        };
        inputs.push((0..n).map(|_| rng.below(256) as u8).collect());
    }
    inputs.push(
        (0..(1u32 << 20))
            .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
            .collect(),
    );
    for (i, input) in inputs.iter().enumerate() {
        let (code, out) = run_exe(&exe, &[], input);
        assert_eq!(code, 0, "ca {i}");
        assert_eq!(
            String::from_utf8_lossy(&out),
            format!("{} {}", input.len(), fnv_like(input)),
            "ca {i} (n={})",
            input.len()
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// stdin đóng/không có: `read_all` trả tệp rỗng (EOF), KHÔNG lỗi.
#[test]
fn stdin_closed_reads_as_empty() {
    let (d, exe) = build(STDIN_SRC, "stdin-closed");
    let o = Command::new(&exe)
        .stdin(Stdio::null())
        .output()
        .expect("chạy");
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        format!("0 {}", fnv_like(b""))
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// 34 ca ghi tệp: nội dung ngẫu nhiên/biên ghi đúng (đọc lại bằng Rust), ghi đè tệp có sẵn (cắt cụt), nội dung rỗng; lỗi: thư mục cha không có ⇒ NotFound (0),
/// đường dẫn là THƯ MỤC ⇒ mã theo `ErrorKind` của HĐH (Unix thường là Other=3; Windows là PermissionDenied=1).
#[test]
fn fs_write_matches_the_model_and_reports_errors() {
    let (d, exe) = build(WRITE_SRC, "write");
    let work = d.join("w");
    std::fs::create_dir_all(&work).expect("tạo");
    let mut rng = Rng(0xF5F5_0001);
    for i in 0..32 {
        let n = match i % 4 {
            0 => 0,
            1 => rng.below(10),
            2 => rng.below(5000),
            _ => rng.below(300_000),
        };
        let data: Vec<u8> = (0..n).map(|_| rng.below(256) as u8).collect();
        let p = work.join(format!("f{i}.bin"));
        if i % 3 == 0 {
            std::fs::write(&p, vec![b'X'; 1000]).expect("tệp có sẵn dài hơn"); // phải bị cắt cụt
        }
        let (code, out) = run_exe(&exe, &[p.to_str().expect("utf8")], &data);
        assert_eq!(code, 0, "ca {i}");
        assert_eq!(String::from_utf8_lossy(&out), "ok", "ca {i}");
        assert_eq!(
            std::fs::read(&p).expect("đọc lại"),
            data,
            "ca {i}: nội dung"
        );
    }
    // thư mục cha không tồn tại
    let (code, out) = run_exe(
        &exe,
        &[work.join("no_such_dir").join("x").to_str().expect("utf8")],
        b"zzz",
    );
    assert_eq!(code, 0);
    assert_eq!(String::from_utf8_lossy(&out), "err 0");
    // đường dẫn là thư mục
    let (code, out) = run_exe(&exe, &[work.to_str().expect("utf8")], b"zzz");
    assert_eq!(code, 0);
    let expected = match std::fs::write(&work, b"zzz")
        .expect_err("ghi vào thư mục phải lỗi")
        .kind()
    {
        std::io::ErrorKind::NotFound => "err 0",
        std::io::ErrorKind::PermissionDenied => "err 1",
        _ => "err 3",
    };
    assert_eq!(String::from_utf8_lossy(&out), expected);
    let _ = std::fs::remove_dir_all(&d);
}
