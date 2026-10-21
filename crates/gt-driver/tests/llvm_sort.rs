//! `examples/sortlines/src/msort.gt` (ADR-032 bước (iii)): sắp xếp trộn bottom-up viết bằng Grit, chạy trên NHỊ PHÂN THẬT và thông dịch,
//! đối chiếu với `sort_by_key` ổn định của Rust (tham chiếu độc lập). Mỗi chương trình sinh ra là một thư mục `msort.gt` + `main.gt`.
//! Cần `LLVM-C.dll` trên PATH.
#![cfg(feature = "llvm")]
use std::path::PathBuf;

/// hồ sơ do bộ chạy chọn (`GT_PROFILE=release` ⇒ chạy lại cả bộ ở hồ sơ release, ADR-036 Q77); mặc định dev
fn profile_arg() -> String {
    format!(
        "--profile={}",
        std::env::var("GT_PROFILE").unwrap_or_else(|_| "dev".to_string())
    )
}

use std::process::Command;

const MSORT: &str = include_str!("../../../examples/sortlines/src/msort.gt");

fn gritc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_gritc"))
}

fn scratch(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("gt-sort-test-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn lcg(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed >> 33
}

/// hash của dãy (khoá, chỉ số gốc) theo thứ tự đã sắp — cùng công thức ở Grit và Rust
fn hash_seq(items: &[(u64, u64)]) -> u64 {
    let mut h = 0u64;
    for (k, i) in items {
        h = h
            .wrapping_mul(1_000_003)
            .wrapping_add(k.wrapping_mul(131))
            .wrapping_add(*i);
    }
    h
}

fn main_src(keys: &[u64]) -> String {
    let lit = keys
        .iter()
        .map(|k| format!("{k}u64"))
        .collect::<Vec<_>>()
        .join(", ");
    let (keys_decl, n_expr) = if keys.is_empty() {
        ("keys ≔ [0u64];".to_string(), "0usize".to_string())
    } else {
        (format!("keys ≔ [{lit}];"), format!("{}usize", keys.len()))
    };
    format!(
        "▢ K {{ k: u64, i: u64 }}
⊐ Copy for K {{ }}
⊐ Ord for K {{
  ƒ cmp(&self, o: &K) → Ordering ! ∅ {{
    ⎇ self.k < o.k {{ Ordering::Less }} ⇝ {{ ⎇ self.k > o.k {{ Ordering::Greater }} ⇝ {{ Ordering::Equal }} }}
  }}
}}
ƒ main() → u64 ! {{mem}} {{
  {keys_decl}
  n ≔ {n_expr};
  μar1 ≔ Arena::new();
  μar2 ≔ Arena::new();
  μx ≔ Vec::new(&ar1);
  μy ≔ Vec::new(&ar2);
  μi ≔ 0usize;
  ⟳ i < n ↓ n - i {{ x.push(K {{ k: keys[i], i: i as u64 }}); y.push(K {{ k: 0u64, i: 0u64 }}); i ← i + 1; }}
  msort::sort(x.as_mut_slice(), y.as_mut_slice());
  s ≔ x.as_slice();
  μh ≔ 0u64;
  μj ≔ 0usize;
  ⟳ j < s.len() ↓ s.len() - j {{ h ← h *% 1000003u64 +% s[j].k *% 131u64 +% s[j].i; j ← j + 1usize; }}
  h
}}
"
    )
}

#[derive(Debug, PartialEq, Eq)]
struct Out {
    exit: i32,
    value: String,
}

fn run(keys: &[u64], backend: &str) -> Out {
    let d = scratch(backend);
    std::fs::write(d.join("msort.gt"), MSORT).expect("ghi");
    std::fs::write(d.join("main.gt"), main_src(keys)).expect("ghi");
    let o = gritc()
        .args(["run", "--json", &format!("--backend={backend}")])
        .arg(profile_arg())
        .arg(&d)
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
            "JSON hỏng ({e}): {text}\nstderr: {}",
            String::from_utf8_lossy(&o.stderr)
        )
    });
    let raw = v["run"]["value"].as_str().unwrap_or("").to_string();
    let value = raw
        .strip_prefix("Int(")
        .and_then(|s| s.split(',').next())
        .map_or(raw.clone(), str::to_string);
    let _ = std::fs::remove_dir_all(&d);
    Out {
        exit: o.status.code().unwrap_or(-1),
        value,
    }
}

fn expected(keys: &[u64]) -> u64 {
    let mut v: Vec<(u64, u64)> = keys.iter().copied().zip(0u64..).collect();
    v.sort_by_key(|x| x.0); // `sort_by_key` của Rust là ỔN ĐỊNH
    hash_seq(&v)
}

fn check(keys: &[u64], both: bool) {
    let want = expected(keys).to_string();
    let l = run(keys, "llvm");
    assert_eq!(
        (l.exit, l.value.as_str()),
        (0, want.as_str()),
        "llvm, n = {}",
        keys.len()
    );
    if both {
        let i = run(keys, "interpreter");
        assert_eq!(i, l, "thông dịch ≠ llvm, n = {}", keys.len());
    }
}

/// biên: rỗng, 1, 2, 3 phần tử, luỹ thừa 2 và ±1 (đường tăng trưởng Vec và số lượt trộn chẵn/lẻ), đã sắp, ngược, toàn bằng nhau
#[test]
fn boundary_shapes_match_stable_sort() {
    check(&[], true);
    check(&[5], true);
    check(&[2, 1], true);
    check(&[1, 2], true);
    check(&[3, 1, 2], true);
    for n in [4usize, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65] {
        let asc: Vec<u64> = (0..n as u64).collect();
        let desc: Vec<u64> = (0..n as u64).rev().collect();
        let same = vec![7u64; n];
        check(&asc, n <= 33);
        check(&desc, n <= 33);
        check(&same, n <= 33);
    }
}

/// 60 mảng ngẫu nhiên có nhiều khoá trùng (kiểm TÍNH ỔN ĐỊNH qua chỉ số gốc) và độ dài 0..=200; 20 mảng đầu cũng chạy ở thông dịch
#[test]
fn random_arrays_with_duplicates_match_stable_sort() {
    let mut seed = 20261014u64;
    for case in 0..60 {
        let n = (lcg(&mut seed) % 201) as usize;
        let range = [3u64, 10, 50, 1000, u64::from(u32::MAX)][(lcg(&mut seed) % 5) as usize];
        let keys: Vec<u64> = (0..n).map(|_| lcg(&mut seed) % range).collect();
        check(&keys, case < 20);
    }
}

/// 20 000 phần tử trên exe (khoảng 15 lượt trộn): khớp sắp xếp ổn định của Rust
#[test]
fn twenty_thousand_elements_on_the_binary() {
    let mut seed = 99u64;
    let keys: Vec<u64> = (0..20_000).map(|_| lcg(&mut seed) % 5000).collect();
    check(&keys, false);
}

/// `cmp_bytes`: thứ tự từ điển trên byte, tiền tố ngắn hơn đứng trước, byte cao (≥ 0x80) lớn hơn ASCII (không phải thứ tự có dấu)
#[test]
fn cmp_bytes_is_lexicographic_on_bytes() {
    let cases: Vec<(Vec<u8>, Vec<u8>)> = vec![
        (vec![], vec![]),
        (vec![], vec![1]),
        (vec![1], vec![]),
        (b"abc".to_vec(), b"abc".to_vec()),
        (b"abc".to_vec(), b"abd".to_vec()),
        (b"abd".to_vec(), b"abc".to_vec()),
        (b"ab".to_vec(), b"abc".to_vec()),
        (b"abc".to_vec(), b"ab".to_vec()),
        (vec![0x7f], vec![0x80]),
        (vec![0xff, 0x00], vec![0xff]),
        (vec![0x00], vec![0x00, 0x00]),
        ("é".as_bytes().to_vec(), b"z".to_vec()),
    ];
    let arr = |b: &[u8]| -> String {
        if b.is_empty() {
            "[0u8]".to_string()
        } else {
            format!(
                "[{}]",
                b.iter()
                    .map(|x| format!("{x}u8"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    };
    let mut body = String::from(
        "ƒ main() → u64 ! ∅ {
  μr ≔ 0u64;
",
    );
    let mut want = 0u64;
    for (n, (a, b)) in cases.iter().enumerate() {
        let (ra, rb) = (
            if a.is_empty() {
                "0..0".to_string()
            } else {
                "..".to_string()
            },
            if b.is_empty() {
                "0..0".to_string()
            } else {
                "..".to_string()
            },
        );
        body.push_str(&format!(
            "  a{n} ≔ {};
  b{n} ≔ {};
  r ← r *% 3u64 +% (⌥ msort::cmp_bytes(&a{n}[{ra}], &b{n}[{rb}]) {{ Ordering::Less ↦ 0u64, Ordering::Equal ↦ 1u64, Ordering::Greater ↦ 2u64 }});
",
            arr(a),
            arr(b)
        ));
        let c = match a.cmp(b) {
            std::cmp::Ordering::Less => 0u64,
            std::cmp::Ordering::Equal => 1,
            std::cmp::Ordering::Greater => 2,
        };
        want = want.wrapping_mul(3).wrapping_add(c);
    }
    body.push_str(
        "  r
}
",
    );
    for backend in ["llvm", "interpreter"] {
        let d = scratch(backend);
        std::fs::write(d.join("msort.gt"), MSORT).expect("ghi");
        std::fs::write(d.join("main.gt"), &body).expect("ghi");
        let o = gritc()
            .args(["run", "--json", &format!("--backend={backend}")])
            .arg(profile_arg())
            .arg(&d)
            .output()
            .expect("chạy");
        let text = String::from_utf8_lossy(&o.stdout).into_owned();
        let line = text
            .lines()
            .rev()
            .find(|l| l.starts_with('{'))
            .unwrap_or("");
        let v: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|e| {
            panic!("JSON ({e}): {text}\n{}", String::from_utf8_lossy(&o.stderr))
        });
        let raw = v["run"]["value"].as_str().unwrap_or("").to_string();
        let value = raw
            .strip_prefix("Int(")
            .and_then(|s| s.split(',').next())
            .map_or(raw.clone(), str::to_string);
        assert_eq!(value, want.to_string(), "{backend}\n{body}\n{text}");
    }
}

/// Nhiên liệu KHÔNG bao giờ cạn trên đầu vào hợp lệ (điều kiện của Giang, 2026-10-14): 10⁶ phần tử sinh bằng LCG NGAY TRONG Grit
/// (tránh nguồn 10 MB), chạy trên exe; thoát 0 (không abort `fuel_exhausted`) và khớp sắp xếp ổn định của Rust. Hai vòng nhiên liệu của
/// `msort` (`pass`: n + 1 vòng, `sort`: 64 vòng) có dư rất lớn: ~20 lượt, mỗi lượt `pass` dùng ≤ n/2 vòng.
#[test]
fn one_million_elements_never_exhaust_fuel() {
    const N: u64 = 1_000_000;
    const RANGE: u64 = 5000;
    let src = format!(
        "▢ K {{ k: u64, i: u64 }}
⊐ Copy for K {{ }}
⊐ Ord for K {{
  ƒ cmp(&self, o: &K) → Ordering ! ∅ {{
    ⎇ self.k < o.k {{ Ordering::Less }} ⇝ {{ ⎇ self.k > o.k {{ Ordering::Greater }} ⇝ {{ Ordering::Equal }} }}
  }}
}}
ƒ main() → u64 ! {{mem}} {{
  μar1 ≔ Arena::new();
  μar2 ≔ Arena::new();
  μx ≔ Vec::new(&ar1);
  μy ≔ Vec::new(&ar2);
  μseed ≔ 99u64;
  μi ≔ 0u64;
  ⟳ i < {N}u64 ↓ {N}u64 - i {{
    seed ← seed *% 6364136223846793005u64 +% 1442695040888963407u64;
    x.push(K {{ k: (seed / 8589934592u64) % {RANGE}u64, i: i }});
    y.push(K {{ k: 0u64, i: 0u64 }});
    i ← i + 1u64;
  }}
  msort::sort(x.as_mut_slice(), y.as_mut_slice());
  s ≔ x.as_slice();
  μh ≔ 0u64;
  μj ≔ 0usize;
  ⟳ j < s.len() ↓ s.len() - j {{ h ← h *% 1000003u64 +% s[j].k *% 131u64 +% s[j].i; j ← j + 1usize; }}
  h
}}
"
    );
    let d = scratch("fuel1m");
    std::fs::write(d.join("msort.gt"), MSORT).expect("ghi");
    std::fs::write(d.join("main.gt"), &src).expect("ghi");
    let t = std::time::Instant::now();
    let o = gritc()
        .args(["run", "--json", "--backend=llvm"])
        .arg(profile_arg())
        .arg(&d)
        .output()
        .expect("chạy gritc");
    let dt = t.elapsed();
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let line = text
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .unwrap_or("");
    let v: serde_json::Value = serde_json::from_str(line)
        .unwrap_or_else(|e| panic!("JSON ({e}): {text}\n{}", String::from_utf8_lossy(&o.stderr)));
    assert!(
        !v["run"]["aborted"].as_bool().unwrap_or(true),
        "abort ({}): nhiên liệu/biên cạn trên đầu vào hợp lệ là BUG: {text}",
        v["run"]["abort_kind"]
    );
    let mut seed = 99u64;
    let keys: Vec<u64> = (0..N)
        .map(|_| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed / 8_589_934_592) % RANGE
        })
        .collect();
    assert_eq!(
        v["run"]["value"].as_str().unwrap_or(""),
        expected(&keys).to_string(),
        "{text}"
    );
    println!("10^6 phần tử (gồm biên dịch): {:.1} s", dt.as_secs_f64());
    let _ = std::fs::remove_dir_all(&d);
}

/// Phần tử RỘNG (24 và 32 byte, căn lề hỗn hợp) cỡ lớn trên exe, đối chiếu `sort_by_key` ổn định của Rust (lệnh 2026-10-14, 1d):
/// 50 000 phần tử mỗi kiểu; khoá trong 0..200 (rất nhiều trùng); các trường còn lại suy từ chỉ số ⇒ ổn định quan sát được qua hash.
#[test]
fn wide_elements_24_and_32_bytes_match_stable_sort_at_scale() {
    // (tên, trường (tên, kiểu, modulo cho phép cast), chỉ số khoá)
    type Def = (&'static str, Vec<(&'static str, &'static str, u64)>, usize);
    let defs: Vec<Def> = vec![
        (
            "S24a",
            vec![("x", "u64", 0), ("y", "u8", 255), ("z", "u64", 0)],
            1,
        ),
        (
            "S24b",
            vec![
                ("p", "u8", 255),
                ("q", "u64", 0),
                ("r", "u8", 255),
                ("s", "u32", 4_294_967_295),
            ],
            1,
        ),
        (
            "S32a",
            vec![
                ("a", "u8", 255),
                ("b", "u64", 0),
                ("c", "u16", 65_535),
                ("d", "u64", 0),
            ],
            1,
        ),
        (
            "S32b",
            vec![
                ("a", "u32", 4_294_967_295),
                ("b", "u64", 0),
                ("c", "u8", 255),
                ("d", "u8", 255),
                ("e", "u64", 0),
            ],
            1,
        ),
    ];
    const N: u64 = 50_000;
    for (name, fields, key) in defs {
        let decl: Vec<String> = fields.iter().map(|(n, t, _)| format!("{n}: {t}")).collect();
        let kn = fields[key].0;
        // giá trị trường i-th của phần tử số `i` (Grit và Rust giống hệt)
        let init: Vec<String> = fields
            .iter()
            .enumerate()
            .map(|(fi, (n, t, m))| {
                if fi == key {
                    format!("{n}: ((i *% 2654435761u64) % 200u64) as {t}")
                } else if *m == 0 {
                    format!("{n}: i *% {}u64", 3 + 2 * fi as u64)
                } else {
                    format!("{n}: ((i *% {}u64) % {m}u64) as {t}", 3 + 2 * fi as u64)
                }
            })
            .collect();
        let hs: String = fields
            .iter()
            .enumerate()
            .map(|(fi, (n, _, _))| {
                format!(
                    " h ← h *% 1000003u64 +% (s.{n} as u64) *% {}u64;",
                    2 * fi + 3
                )
            })
            .collect();
        let src = format!(
            "▢ {name} {{ {decl} }}
⊐ Copy for {name} {{ }}
⊐ Ord for {name} {{
  ƒ cmp(&self, o: &Self) → Ordering ! ∅ {{
    ⎇ self.{kn} < o.{kn} {{ Ordering::Less }} ⇝ {{ ⎇ self.{kn} > o.{kn} {{ Ordering::Greater }} ⇝ {{ Ordering::Equal }} }}
  }}
}}
ƒ sh(s: &{name}) → u64 ! ∅ {{ μh ≔ 0u64;{hs} h }}
ƒ main() → u64 ! {{mem}} {{
  μar1 ≔ Arena::new();
  μar2 ≔ Arena::new();
  μx ≔ Vec::new(&ar1);
  μy ≔ Vec::new(&ar2);
  μi ≔ 0u64;
  ⟳ i < {N}u64 ↓ {N}u64 - i {{
    x.push({name} {{ {init} }});
    y.push({name} {{ {init} }});
    i ← i + 1u64;
  }}
  msort::sort(x.as_mut_slice(), y.as_mut_slice());
  t ≔ x.as_slice();
  μr ≔ 0u64;
  μk ≔ 0usize;
  ⟳ k < t.len() ↓ t.len() - k {{ r ← r *% 31u64 +% sh(&t[k]); k ← k + 1usize; }}
  r
}}
",
            decl = decl.join(", "),
            init = init.join(", ")
        );
        // mô hình Rust
        let elem = |i: u64| -> Vec<u64> {
            fields
                .iter()
                .enumerate()
                .map(|(fi, (_, _, m))| {
                    if fi == key {
                        i.wrapping_mul(2_654_435_761) % 200
                    } else if *m == 0 {
                        i.wrapping_mul(3 + 2 * fi as u64)
                    } else {
                        i.wrapping_mul(3 + 2 * fi as u64) % m
                    }
                })
                .collect()
        };
        let mut v: Vec<Vec<u64>> = (0..N).map(elem).collect();
        v.sort_by_key(|f| f[key]);
        let mut r = 0u64;
        for f in &v {
            let mut h = 0u64;
            for (fi, x) in f.iter().enumerate() {
                h = h
                    .wrapping_mul(1_000_003)
                    .wrapping_add(x.wrapping_mul(2 * fi as u64 + 3));
            }
            r = r.wrapping_mul(31).wrapping_add(h);
        }
        let d = scratch("wide");
        std::fs::write(d.join("msort.gt"), MSORT).expect("ghi");
        std::fs::write(d.join("main.gt"), &src).expect("ghi");
        let o = gritc()
            .args(["run", "--json", "--backend=llvm"])
            .arg(profile_arg())
            .arg(&d)
            .output()
            .expect("chạy gritc");
        let text = String::from_utf8_lossy(&o.stdout).into_owned();
        let line = text
            .lines()
            .rev()
            .find(|l| l.starts_with('{'))
            .unwrap_or("");
        let j: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|e| {
            panic!(
                "{name}: JSON ({e}): {text}\n{}",
                String::from_utf8_lossy(&o.stderr)
            )
        });
        assert_eq!(
            j["run"]["value"].as_str().unwrap_or(""),
            r.to_string(),
            "{name}: {text}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
