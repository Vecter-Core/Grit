//! G2.5/P1.d — `std::HashMap` với TOÀN BỘ khoá va chạm băm (`Hash::hash` hằng ⇒ mọi khoá cùng một chuỗi dò): mô hình Rust (`BTreeMap`) ≡ exe LLVM trên ≥ 30 chương trình
//! ngẫu nhiên MỖI hàm API (`new`, `insert` kể cả ghi đè, `get`, `contains_key`, `remove`, `len`, `entry_count`/`entry_at`), cả sau nhiều lần xoá/chèn (bia mộ + dựng lại bảng).
//! Thông dịch MIR không mô phỏng Arena ⇒ cột thông dịch không có (cùng quy ước `std2_lib.rs`). Hồ sơ: `GT_PROFILE=dev|release|release-o2`.
//! Kết quả: `HASHMAP_COLLIDE_OUT=<đường dẫn tuyệt đối .jsonl>` ghi nối một dòng.
#![cfg(feature = "llvm")]
#[path = "../../gt-mir/tests/support/progen.rs"]
mod progen;

use progen::Rng;
use std::collections::BTreeMap;
use std::io::Write;
use std::process::Command;

const STD: &str = include_str!("../../../lib/std/src/std.gt");
const STD_ORD: &str = include_str!("../../../lib/std/src/std_ord.gt");
const STD_ORD_INT: &str = include_str!("../../../lib/std/src/std_ord_int.gt");
const N: u64 = 32;

const DEFS: &str = "ƒ bn(b: bool) → u64 ! ∅ { ⎇ b { 1u64 } ⇝ { 0u64 } }
ƒ ou(o: Option<u64>) → u64 ! ∅ { ⌥ o { Some(v) ↦ v +% 1u64, None ↦ 0u64 } }
▢ Cl { v: u64 }
⊐ Copy for Cl { }
⊐ Ord for Cl {
  ƒ cmp(&self, o: &Cl) → Ordering ! ∅ {
    ⎇ self.v < o.v { Ordering::Less } ⇝ { ⎇ self.v > o.v { Ordering::Greater } ⇝ { Ordering::Equal } }
  }
}
⊐ std::Hash for Cl { ƒ hash(&self) → u64 ! ∅ { 3u64 } }
";

fn fold(acc: u64, v: u64) -> u64 {
    acc.wrapping_mul(31).wrapping_add(v)
}

struct Prog {
    src: String,
    want: u64,
    used: BTreeMap<&'static str, u64>,
}

fn prog(seed: u64) -> Prog {
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let pool = [6u64, 30, 90][r.below(3) as usize];
    let nops = 40 + r.below(120);
    let mut model: BTreeMap<u64, u64> = BTreeMap::new();
    let mut acc = 0u64;
    let mut used: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut body = String::new();
    let bump = |u: &mut BTreeMap<&'static str, u64>, n: &'static str| *u.entry(n).or_default() += 1;
    for _ in 0..nops {
        let k = if r.chance(8) {
            [0u64, u64::MAX][r.below(2) as usize]
        } else {
            r.below(pool)
        };
        let w = r.below(100);
        if w < 45 {
            let v = r.next() >> r.below(64);
            body += &format!("  m.insert(Cl {{ v: {k}u64 }}, {v}u64);\n");
            model.insert(k, v);
            bump(&mut used, "insert");
        } else if w < 65 {
            let had = model.remove(&k).is_some();
            body += &format!("  acc ← acc *% 31u64 +% bn(m.remove(&Cl {{ v: {k}u64 }}));\n");
            acc = fold(acc, u64::from(had));
            bump(&mut used, "remove");
        } else if w < 85 {
            body += &format!("  acc ← acc *% 31u64 +% ou(m.get(&Cl {{ v: {k}u64 }}));\n");
            acc = fold(acc, model.get(&k).map_or(0, |v| v.wrapping_add(1)));
            bump(&mut used, "get");
        } else if w < 93 {
            body += &format!("  acc ← acc *% 31u64 +% bn(m.contains_key(&Cl {{ v: {k}u64 }}));\n");
            acc = fold(acc, u64::from(model.contains_key(&k)));
            bump(&mut used, "contains_key");
        } else {
            body += "  acc ← acc *% 31u64 +% (m.len() as u64);\n";
            acc = fold(acc, model.len() as u64);
            bump(&mut used, "len");
        }
    }
    // mỗi chương trình có ĐỦ mọi phép ít nhất một lần
    let k0 = r.below(pool);
    body += &format!("  m.insert(Cl {{ v: {k0}u64 }}, 5u64);\n");
    model.insert(k0, 5);
    body += &format!("  acc ← acc *% 31u64 +% ou(m.get(&Cl {{ v: {k0}u64 }}));\n");
    acc = fold(acc, 6);
    body += &format!("  acc ← acc *% 31u64 +% bn(m.contains_key(&Cl {{ v: {k0}u64 }}));\n");
    acc = fold(acc, 1);
    body += "  acc ← acc *% 31u64 +% (m.len() as u64);\n";
    acc = fold(acc, model.len() as u64);
    body += &format!("  acc ← acc *% 31u64 +% bn(m.remove(&Cl {{ v: {k0}u64 }}));\n");
    model.remove(&k0);
    acc = fold(acc, 1);
    body += &format!("  m.insert(Cl {{ v: {k0}u64 }}, 9u64);\n");
    model.insert(k0, 9);
    for n in ["insert", "get", "contains_key", "len", "remove"] {
        bump(&mut used, n);
    }
    // duyệt mục: tổng (khoá+1)·giá trị không phụ thuộc thứ tự
    body += "  nn ≔ m.entry_count();\n  μi ≔ 0usize;\n  ⟳ i < nn ↓ nn - i {\n    ⌥ m.entry_at(i) { Some(e) ↦ { acc ← acc +% (e.0.v +% 1u64) *% e.1; }, None ↦ { } }\n    i ← i + 1usize;\n  }\n";
    for (k, v) in &model {
        acc = acc.wrapping_add(k.wrapping_add(1).wrapping_mul(*v));
    }
    bump(&mut used, "entry_count/entry_at");
    bump(&mut used, "new");
    Prog {
        src: format!(
            "{DEFS}ƒ main() → u64 ! {{mem}} {{\n  μar ≔ Arena::new();\n  μm ≔ std::HashMap::new(&ar);\n  μacc ≔ 0u64;\n{body}  acc\n}}\n"
        ),
        want: acc,
        used,
    }
}

fn run(src: &str, profile: &str, seed: u64) -> Result<String, String> {
    let d = std::env::temp_dir().join(format!("gt-hmc-{}-{seed}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục");
    for (n, t) in [
        ("std.gt", STD),
        ("std_ord.gt", STD_ORD),
        ("std_ord_int.gt", STD_ORD_INT),
        ("main.gt", src),
    ] {
        std::fs::write(d.join(n), t).expect("ghi");
    }
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args(["run", "--json", "--backend=llvm"])
        .arg(format!("--profile={profile}"))
        .arg(&d)
        .output()
        .expect("chạy gritc");
    let _ = std::fs::remove_dir_all(&d);
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let line = text
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .unwrap_or("");
    let v: serde_json::Value = serde_json::from_str(line).map_err(|e| {
        format!(
            "JSON hỏng ({e}): {text}\nstderr: {}",
            String::from_utf8_lossy(&o.stderr)
        )
    })?;
    if v["run"]["aborted"].as_bool().unwrap_or(false) {
        return Err(format!("abort {}", v["run"]["abort_kind"]));
    }
    let val = v["run"]["value"].as_str().unwrap_or("").to_string();
    if val.is_empty() {
        return Err(format!(
            "không có kết quả chạy (gritc đỏ?): {}",
            text.chars().take(600).collect::<String>()
        ));
    }
    Ok(val)
}

#[test]
fn hashmap_with_all_colliding_keys_matches_the_rust_model() {
    let profile = std::env::var("GT_PROFILE").unwrap_or_else(|_| "dev".to_string());
    let mut per_fn: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut bad = Vec::new();
    for seed in 1..=N {
        let p = prog(seed);
        for k in p.used.keys() {
            *per_fn.entry(*k).or_default() += 1;
        }
        match run(&p.src, &profile, seed) {
            Ok(got) if got == p.want.to_string() => {}
            Ok(got) => bad.push(format!(
                "#{seed}: exe {got} ≠ mô hình {}\n{}",
                p.want, p.src
            )),
            Err(e) => bad.push(format!("#{seed}: {e}\n{}", p.src)),
        }
    }
    let line = format!(
        "{{\"kind\":\"hashmap_collide\",\"profile\":\"{profile}\",\"programs\":{N},\"mismatch\":{},\"per_fn\":{:?}}}",
        bad.len(),
        per_fn
    );
    eprintln!("{line}");
    if let Ok(path) = std::env::var("HASHMAP_COLLIDE_OUT") {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("mở sổ");
        writeln!(f, "{line}").expect("ghi");
    }
    assert!(
        bad.is_empty(),
        "{} bất đồng:\n{}",
        bad.len(),
        bad.join("\n----\n")
    );
    for f in [
        "new",
        "insert",
        "get",
        "contains_key",
        "remove",
        "len",
        "entry_count/entry_at",
    ] {
        assert!(
            per_fn.get(f).copied().unwrap_or(0) >= 30,
            "hàm {f}: chỉ {:?} chương trình (< 30)",
            per_fn.get(f)
        );
    }
}
