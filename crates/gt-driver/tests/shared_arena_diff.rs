//! G2.1.c (iv) — Arena CHIA SẺ (ADR-C03/C07): mỗi loại ≥ 30 chương trình ngẫu nhiên, mô hình Rust độc lập ≡ exe LLVM (thông dịch MIR không mô phỏng Arena,
//! nên cột thông dịch không có — cùng quy ước `std2_lib.rs`). Ở hồ sơ `dev` kiểm động `arena_overlap` BẬT (hai Vec/Text/alloc cùng một Arena mà tăng trưởng xen kẽ
//! mà trả vùng chồng lấn ⇒ abort ⇒ test ĐỎ). Hồ sơ: `GT_PROFILE=dev|release|release-o2`. Kết quả: `SHARED_ARENA_OUT=<đường dẫn tuyệt đối .jsonl>` ghi nối một dòng/loại.
//! Loại: `two_vec` (hai Vec xen kẽ push), `vec_text` (Vec + Text), `alloc_mix` (alloc xen kẽ tăng trưởng Vec), `struct_fn` (hai Vec trong struct, push qua hàm `&!`),
//! `reset_reuse` (Vec → reset → Vec mới trên cùng Arena).
#![cfg(feature = "llvm")]
#[path = "../../gt-mir/tests/support/progen.rs"]
mod progen;

use progen::Rng;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const N_PER_KIND: u64 = 32;
const HELPERS: &str = "ƒ cs(s: &[u64]) → u64 ! ∅ {
  μi ≔ 0usize;
  μt ≔ 0u64;
  ⟳ i < s.len() ↓ s.len() - i { t ← t +% s[i] *% ((i as u64) +% 1u64); i ← i + 1usize; }
  t
}
ƒ cb(s: &[u8]) → u64 ! ∅ {
  μi ≔ 0usize;
  μt ≔ 0u64;
  ⟳ i < s.len() ↓ s.len() - i { t ← t +% (s[i] as u64) *% ((i as u64) +% 1u64); i ← i + 1usize; }
  t
}
";

fn cs(v: &[u64]) -> u64 {
    v.iter().enumerate().fold(0u64, |t, (i, x)| {
        t.wrapping_add(x.wrapping_mul(i as u64 + 1))
    })
}

fn cb(v: &[u8]) -> u64 {
    v.iter().enumerate().fold(0u64, |t, (i, x)| {
        t.wrapping_add(u64::from(*x).wrapping_mul(i as u64 + 1))
    })
}

type Gen = fn(&mut Rng) -> Prog;

struct Prog {
    src: String,
    want: u64,
}

fn val(r: &mut Rng) -> u64 {
    r.below(1_000_000_000)
}

fn two_vec(r: &mut Rng) -> Prog {
    let n = 12 + r.below(48);
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let mut body = String::new();
    for _ in 0..n {
        let x = val(r);
        if r.chance(50) {
            body += &format!("  a.push({x}u64);\n");
            a.push(x);
        } else {
            body += &format!("  b.push({x}u64);\n");
            b.push(x);
        }
    }
    Prog {
        src: format!(
            "{HELPERS}ƒ main() → u64 ! {{mem}} {{\n  μar ≔ Arena::new();\n  μa ≔ Vec::new(&ar);\n  μb ≔ Vec::new(&ar);\n{body}  cs(a.as_slice()) +% cs(b.as_slice()) *% 3u64\n}}\n"
        ),
        want: cs(&a).wrapping_add(cs(&b).wrapping_mul(3)),
    }
}

fn vec_text(r: &mut Rng) -> Prog {
    const PIECES: [&str; 5] = ["ab", "xyz", "é", "日本", "q"];
    let n = 10 + r.below(30);
    let mut a = Vec::new();
    let mut text: Vec<u8> = Vec::new();
    let mut body = String::new();
    for _ in 0..n {
        match r.below(3) {
            0 => {
                let x = val(r);
                body += &format!("  a.push({x}u64);\n");
                a.push(x);
            }
            1 => {
                let p = PIECES[r.below(PIECES.len() as u64) as usize];
                body += &format!("  t.push_str(\"{p}\");\n");
                text.extend_from_slice(p.as_bytes());
            }
            _ => {
                body += "  t.push_char('k');\n";
                text.push(b'k');
            }
        }
    }
    let want = cb(&text)
        .wrapping_add(cs(&a).wrapping_mul(5))
        .wrapping_add(text.len() as u64);
    Prog {
        src: format!(
            "{HELPERS}ƒ main() → u64 ! {{mem}} {{\n  μar ≔ Arena::new();\n  μa ≔ Vec::new(&ar);\n  μt ≔ Text::new(&ar);\n{body}  cb(t.as_bytes()) +% cs(a.as_slice()) *% 5u64 +% (t.len() as u64)\n}}\n"
        ),
        want,
    }
}

fn alloc_mix(r: &mut Rng) -> Prog {
    let n = 12 + r.below(40);
    let mut a = Vec::new();
    let mut refs: Vec<u64> = Vec::new();
    let mut body = String::new();
    for _ in 0..n {
        let x = val(r);
        if r.chance(25) && refs.len() < 6 {
            body += &format!("  x{} ≔ ar.alloc({x}u64);\n", refs.len());
            refs.push(x);
        } else {
            body += &format!("  a.push({x}u64);\n");
            a.push(x);
        }
    }
    let mut tail = String::from("cs(a.as_slice())");
    let mut want = cs(&a);
    for (i, x) in refs.iter().enumerate() {
        tail += &format!(" +% *x{i} *% {}u64", i + 1);
        want = want.wrapping_add(x.wrapping_mul(i as u64 + 1));
    }
    Prog {
        src: format!(
            "{HELPERS}ƒ main() → u64 ! {{mem}} {{\n  μar ≔ Arena::new();\n  μa ≔ Vec::new(&ar);\n{body}  {tail}\n}}\n"
        ),
        want,
    }
}

fn struct_fn(r: &mut Rng) -> Prog {
    let n = 12 + r.below(40);
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let mut body = String::new();
    for _ in 0..n {
        let x = val(r);
        if r.chance(50) {
            body += &format!("  fill(&!h1, {x}u64);\n");
            a.push(x);
        } else {
            body += &format!("  fill(&!h2, {x}u64);\n");
            b.push(x);
        }
    }
    Prog {
        src: format!(
            "{HELPERS}▢ H<'a> {{ v: Vec<'a, u64> }}\nƒ fill<'a>(h: &!H<'a>, k: u64) ! {{mem}} {{ h.v.push(k); }}\nƒ main() → u64 ! {{mem}} {{\n  μar ≔ Arena::new();\n  μh1 ≔ H {{ v: Vec::new(&ar) }};\n  μh2 ≔ H {{ v: Vec::new(&ar) }};\n{body}  cs(h1.v.as_slice()) +% cs(h2.v.as_slice()) *% 3u64\n}}\n"
        ),
        want: cs(&a).wrapping_add(cs(&b).wrapping_mul(3)),
    }
}

fn reset_reuse(r: &mut Rng) -> Prog {
    let (n1, n2) = (6 + r.below(30), 6 + r.below(30));
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (mut p1, mut p2) = (String::new(), String::new());
    for _ in 0..n1 {
        let x = val(r);
        p1 += &format!("  a.push({x}u64);\n");
        a.push(x);
    }
    for _ in 0..n2 {
        let x = val(r);
        p2 += &format!("  b.push({x}u64);\n");
        b.push(x);
    }
    Prog {
        src: format!(
            "{HELPERS}ƒ main() → u64 ! {{mem}} {{\n  μar ≔ Arena::new();\n  μa ≔ Vec::new(&ar);\n{p1}  r1 ≔ cs(a.as_slice());\n  ar.reset();\n  μb ≔ Vec::new(&ar);\n{p2}  r1 *% 31u64 +% cs(b.as_slice())\n}}\n"
        ),
        want: cs(&a).wrapping_mul(31).wrapping_add(cs(&b)),
    }
}

fn run_exe(src: &str, profile: &str, tag: &str) -> Result<String, String> {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d: PathBuf =
        std::env::temp_dir().join(format!("gt-shared-arena-{}-{n}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    let f = d.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args(["run", "--json", "--backend=llvm"])
        .arg(format!("--profile={profile}"))
        .arg(&f)
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
    Ok(v["run"]["value"].as_str().unwrap_or("").to_string())
}

#[test]
fn shared_arena_programs_match_the_independent_rust_model_on_the_binary() {
    let profile = std::env::var("GT_PROFILE").unwrap_or_else(|_| "dev".to_string());
    let kinds: [(&str, Gen); 5] = [
        ("two_vec", two_vec),
        ("vec_text", vec_text),
        ("alloc_mix", alloc_mix),
        ("struct_fn", struct_fn),
        ("reset_reuse", reset_reuse),
    ];
    let mut report = Vec::new();
    for (name, gen_) in kinds {
        let (mut ok, mut bad) = (0u64, Vec::new());
        for seed in 1..=N_PER_KIND {
            let mut r = Rng(0x9E37_79B9_7F4A_7C15 ^ (seed * 7919 + name.len() as u64 * 104_729));
            let p = gen_(&mut r);
            match run_exe(&p.src, &profile, name) {
                Ok(got) if got == p.want.to_string() => ok += 1,
                Ok(got) => bad.push(format!(
                    "{name}#{seed}: exe {got} ≠ mô hình {}\n{}",
                    p.want, p.src
                )),
                Err(e) => bad.push(format!("{name}#{seed}: {e}\n{}", p.src)),
            }
        }
        report.push(format!(
            "{{\"kind\":\"{name}\",\"profile\":\"{profile}\",\"programs\":{N_PER_KIND},\"match\":{ok},\"mismatch\":{}}}",
            bad.len()
        ));
        assert!(
            bad.is_empty(),
            "{} bất đồng ở loại {name}:\n{}",
            bad.len(),
            bad.join("\n----\n")
        );
        assert!(ok >= 30, "{name}: chỉ {ok} chương trình khớp (< 30)");
    }
    if let Ok(path) = std::env::var("SHARED_ARENA_OUT") {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("mở sổ kết quả");
        for l in &report {
            writeln!(f, "{l}").expect("ghi");
        }
    }
    eprintln!("{}", report.join("\n"));
}
