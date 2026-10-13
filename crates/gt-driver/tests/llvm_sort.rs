//! `examples/sortlines/src/msort.gt` (ADR-032 bước (iii)): sắp xếp trộn bottom-up viết bằng Grit, chạy trên NHỊ PHÂN THẬT và thông dịch,
//! đối chiếu với `sort_by_key` ổn định của Rust (tham chiếu độc lập). Mỗi chương trình sinh ra là một thư mục `msort.gt` + `main.gt`.
//! Cần `LLVM-C.dll` trên PATH.
#![cfg(feature = "llvm")]
use std::path::PathBuf;
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
  μx ≔ Vec::new(&!ar1);
  μy ≔ Vec::new(&!ar2);
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
