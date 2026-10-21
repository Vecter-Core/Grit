//! H6 (TCB `Text::{new,push_str,push_char,push_utf8,from_utf8}`, `str::slice`): VI SAI ở exe thật giữa kiểm UTF-8 của Grit (`Text::push_utf8`) và `std::str::from_utf8`
//! của Rust trên ≥ 100 000 chuỗi byte ngẫu nhiên + biên (cắt giữa ký tự, overlong, surrogate, > U+10FFFF, byte nối lạc, NUL, ranh giới các điểm mã).
//! Lưu ý trung thực: `gritrt_utf8_valid` hiện CHÍNH LÀ `std::str::from_utf8` bọc lại, nên vi sai ở mức hàm runtime là tầm thường; phép đo có nghĩa là
//! vi sai TOÀN ĐƯỜNG (đọc tệp → lát → `push_utf8` → phán quyết) và việc `Text` KHÔNG bao giờ nhận byte sai. Phần `str::slice`: cắt giữa ký tự ⇒ abort `slice_range`,
//! đối chiếu `is_char_boundary` của Rust trên mọi cặp (from, to) của nhiều chuỗi đa byte.
#![cfg(feature = "llvm")]
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("gt-utf8-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo");
    d
}

fn build(d: &Path, name: &str, src: &str, profile: &str) -> PathBuf {
    let f = d.join(format!("{name}.gt"));
    std::fs::write(&f, src).expect("ghi");
    let exe = d.join(format!("{name}.exe"));
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args(["build", &format!("--profile={profile}"), "-o"])
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
    exe
}

/// mỗi bản ghi CỐ ĐỊNH 16 byte: 1 byte độ dài n (0..=15) rồi n byte rồi đệm 0 (bước vòng lặp hằng ⇒ M8 chứng minh dừng); chương trình đẩy từng bản ghi vào MỘT `Text` bằng `push_utf8` và in '1' (nhận) hoặc '0' (từ chối)
const BATCH: &str = "ƒ main() ! {io, mem} {
  fs ≔ root_fs();
  env ≔ root_env();
  out ≔ root_out();
  alloc ≔ root_alloc();
  f ≔ fs.read(env.arg(1usize));
  b ≔ f.bytes();
  μar ≔ alloc.arena();
  μt ≔ Text::new(&ar);
  total ≔ b.len();
  μi ≔ 0usize;
  ⟳ i < total ↓ total - i {
    ⎇ i + 16usize <= total {
      n ≔ b[i] as usize;
      ok ≔ n <= 15usize ∧ t.push_utf8(&b[i + 1usize..i + 1usize + n]);
      ⎇ ok { out.write_str(\"1\"); } ⇝ { out.write_str(\"0\"); }
      i ← i + 16usize;
    } ⇝ {
      i ← total;
    }
  }
}
";

/// bản ghi: byte `from`, byte `to`, rồi chuỗi (hợp lệ UTF-8); dựng `Text`, cắt `[from, to)` và in độ dài lát. Cắt sai ⇒ abort `slice_range`.
const SLICE: &str = "ƒ main() ! {io, mem} {
  fs ≔ root_fs();
  env ≔ root_env();
  out ≔ root_out();
  alloc ≔ root_alloc();
  f ≔ fs.read(env.arg(1usize));
  b ≔ f.bytes();
  μar ≔ alloc.arena();
  μt ≔ Text::new(&ar);
  ⎇ b.len() >= 2usize {
    ok ≔ t.push_utf8(&b[2usize..b.len()]);
    ⎇ ok {
      s ≔ t.as_str();
      r ≔ s.slice(b[0] as usize, b[1] as usize);
      out.write_u64(r.len() as u64);
    } ⇝ {
      out.write_str(\"invalid\");
    }
  } ⇝ { }
}
";

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn encode(cp: u32, out: &mut Vec<u8>) {
    // mã hoá "thô" cho cả điểm mã không hợp lệ (surrogate) để sinh byte sai có chủ ý
    if cp < 0x80 {
        out.push(cp as u8);
    } else if cp < 0x800 {
        out.extend([0xC0 | (cp >> 6) as u8, 0x80 | (cp & 0x3F) as u8]);
    } else if cp < 0x1_0000 {
        out.extend([
            0xE0 | (cp >> 12) as u8,
            0x80 | ((cp >> 6) & 0x3F) as u8,
            0x80 | (cp & 0x3F) as u8,
        ]);
    } else {
        out.extend([
            0xF0 | (cp >> 18) as u8,
            0x80 | ((cp >> 12) & 0x3F) as u8,
            0x80 | ((cp >> 6) & 0x3F) as u8,
            0x80 | (cp & 0x3F) as u8,
        ]);
    }
}

const BOUNDARY_CPS: [u32; 14] = [
    0, 0x7F, 0x80, 0x7FF, 0x800, 0xD7FF, 0xE000, 0xFFFF, 0x1_0000, 0x10_FFFF, 0x41, 0xE9, 0x4E2D,
    0x1F600,
];

fn random_record(r: &mut Rng) -> Vec<u8> {
    let mut v = Vec::new();
    match r.below(10) {
        0..=2 => {
            // byte ngẫu nhiên hoàn toàn
            for _ in 0..r.below(13) {
                v.push(r.next() as u8);
            }
        }
        3..=4 => {
            // hợp lệ: vài điểm mã ngẫu nhiên/biên
            for _ in 0..r.below(4) {
                let cp = if r.below(2) == 0 {
                    BOUNDARY_CPS[r.below(BOUNDARY_CPS.len() as u64) as usize]
                } else {
                    let c = r.below(0x11_0000) as u32;
                    if (0xD800..0xE000).contains(&c) {
                        0x61
                    } else {
                        c
                    }
                };
                encode(cp, &mut v);
            }
        }
        5 => {
            // hợp lệ rồi CẮT giữa ký tự
            for _ in 0..1 + r.below(3) {
                encode(BOUNDARY_CPS[2 + r.below(12) as usize], &mut v);
            }
            let cut = 1 + r.below(v.len() as u64) as usize;
            v.truncate(cut.min(v.len().saturating_sub(1)).max(1));
        }
        6 => {
            // overlong
            let pats: [&[u8]; 8] = [
                &[0xC0, 0x80],
                &[0xC1, 0xBF],
                &[0xE0, 0x80, 0x80],
                &[0xE0, 0x9F, 0xBF],
                &[0xF0, 0x80, 0x80, 0x80],
                &[0xF0, 0x8F, 0xBF, 0xBF],
                &[0xC0],
                &[0xE0, 0x80],
            ];
            v.extend(pats[r.below(8) as usize]);
        }
        7 => {
            // surrogate / vượt U+10FFFF
            let pats: [&[u8]; 8] = [
                &[0xED, 0xA0, 0x80],
                &[0xED, 0xBF, 0xBF],
                &[0xED, 0x9F, 0xBF],
                &[0xF4, 0x90, 0x80, 0x80],
                &[0xF4, 0x8F, 0xBF, 0xBF],
                &[0xF5, 0x80, 0x80, 0x80],
                &[0xF8, 0x88, 0x80, 0x80, 0x80],
                &[0xFF],
            ];
            v.extend(pats[r.below(8) as usize]);
        }
        8 => {
            // byte nối lạc / byte đầu lạc / NUL
            for _ in 0..1 + r.below(4) {
                v.push([0x80, 0xBF, 0xC2, 0xE1, 0xF1, 0x00, 0x61][r.below(7) as usize]);
            }
        }
        _ => {
            // hợp lệ + MỘT byte hỏng
            for _ in 0..1 + r.below(3) {
                encode(BOUNDARY_CPS[r.below(14) as usize], &mut v);
            }
            let i = r.below(v.len() as u64 + 1) as usize;
            v.insert(i, r.next() as u8);
        }
    }
    v.truncate(15);
    v
}

#[test]
fn text_push_utf8_agrees_with_rust_from_utf8_on_100k_strings() {
    let n_records: usize = std::env::var("UTF8_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100_000);
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let mut file = Vec::new();
    let mut want = String::new();
    let (mut valid, mut invalid) = (0usize, 0usize);
    for _ in 0..n_records {
        let rec = random_record(&mut r);
        let ok = std::str::from_utf8(&rec).is_ok();
        want.push(if ok { '1' } else { '0' });
        if ok {
            valid += 1;
        } else {
            invalid += 1;
        }
        file.push(rec.len() as u8);
        file.extend(&rec);
        file.extend(std::iter::repeat_n(0u8, 15 - rec.len()));
    }
    println!("bản ghi: {n_records}, hợp lệ {valid}, không hợp lệ {invalid}");
    assert!(valid >= 10_000 && invalid >= 10_000);
    let d = scratch("batch");
    let data = d.join("in.bin");
    std::fs::write(&data, &file).expect("ghi");
    for profile in ["dev", "release-o2"] {
        let exe = build(
            &d,
            &format!("batch_{}", profile.replace('-', "_")),
            BATCH,
            profile,
        );
        let o = Command::new(&exe).arg(&data).output().expect("chạy");
        assert_eq!(
            o.status.code(),
            Some(0),
            "{profile}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        let got = String::from_utf8_lossy(&o.stdout).into_owned();
        assert_eq!(got.len(), want.len(), "{profile}: số phán quyết");
        let first = got.bytes().zip(want.bytes()).position(|(a, b)| a != b);
        assert!(first.is_none(), "{profile}: lệch ở bản ghi {first:?}");
        println!("{profile}: {n_records} bản ghi khớp Rust");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn str_slice_aborts_slice_range_exactly_when_rust_says_not_a_char_boundary() {
    let strings = ["abc", "é", "aé", "中", "a中b", "😀", "a😀é中z", ""];
    let d = scratch("slice");
    let mut cases = 0;
    for profile in ["dev", "release-o2"] {
        let exe = build(
            &d,
            &format!("slice_{}", profile.replace('-', "_")),
            SLICE,
            profile,
        );
        for s in strings {
            let len = s.len();
            for from in 0..=len + 2 {
                for to in 0..=len + 2 {
                    let mut rec = vec![from as u8, to as u8];
                    rec.extend(s.as_bytes());
                    let f = d.join("rec.bin");
                    std::fs::write(&f, &rec).expect("ghi");
                    let o = Command::new(&exe).arg(&f).output().expect("chạy");
                    let ok = from <= to && s.is_char_boundary_checked(from, to);
                    let err = String::from_utf8_lossy(&o.stderr);
                    if ok {
                        assert_eq!(
                            o.status.code(),
                            Some(0),
                            "{profile} {s:?}[{from}..{to}]: {err}"
                        );
                        assert_eq!(
                            String::from_utf8_lossy(&o.stdout),
                            (to - from).to_string(),
                            "{profile} {s:?}[{from}..{to}]"
                        );
                    } else {
                        assert_eq!(
                            o.status.code(),
                            Some(70),
                            "{profile} {s:?}[{from}..{to}]: {err}"
                        );
                        assert!(
                            err.contains("abort: slice_range"),
                            "{profile} {s:?}[{from}..{to}]: {err}"
                        );
                    }
                    cases += 1;
                }
            }
        }
    }
    println!("str::slice: {cases} ca khớp Rust");
    assert!(cases >= 600);
    let _ = std::fs::remove_dir_all(&d);
}

trait CharBoundary {
    /// `from ≤ to ≤ len` và cả hai đầu ở biên ký tự (đúng định nghĩa của Rust `is_char_boundary`)
    fn is_char_boundary_checked(&self, from: usize, to: usize) -> bool;
}
impl CharBoundary for str {
    fn is_char_boundary_checked(&self, from: usize, to: usize) -> bool {
        from <= to && to <= self.len() && self.is_char_boundary(from) && self.is_char_boundary(to)
    }
}
