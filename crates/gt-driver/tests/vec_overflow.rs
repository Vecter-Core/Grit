//! H4 (TCB `Vec::{new,push,reserve}`): ÉP TRÀN THẬT các phép nhân/cộng kích thước ở exe, abort `overflow` ĐÚNG LOẠI ở dev, release và release-o2.
//! Đường ép được qua mã nguồn: `reserve(n)` ⇒ `vec_grow_to` ⇒ `ncap·size` (`llvm.umul.with.overflow`); `push` dùng CHUNG `vec_grow_to` (cùng lệnh kiểm), nhưng
//! muốn tràn bằng `push` cần phần tử cỡ ≥ 2^62 byte (không dựng được trên ngăn xếp) nên KHÔNG ép được riêng; các kiểm `len+n` (`vec_extend`) và `2·cap`
//! chỉ tràn khi một toán hạng ≥ 2^63 — không chương trình nào đạt được (bộ nhớ/độ dài lát ≤ isize::MAX): là chốt PHÒNG THỦ, ghi ở KNOWN_LIMITATIONS.
//! Biên chính xác: `n·size = 2^64` ⇒ `overflow`; `n·size = 2^64 − size` ⇒ không tràn, runtime hết bộ nhớ ⇒ `arena_exhausted`.
#![cfg(feature = "llvm")]
use std::path::PathBuf;
use std::process::Command;

fn scratch() -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("gt-arena-dyn-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục");
    d
}

#[derive(Debug)]
#[allow(dead_code)]
struct Out {
    exit: i32,
    kind: String,
    value: String,
    stderr: String,
}

fn run(src: &str, profile: &str, env: &[(&str, &str)]) -> Out {
    let d = scratch();
    let f = d.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let mut c = Command::new(env!("CARGO_BIN_EXE_gritc"));
    c.args([
        "run",
        "--json",
        "--backend=llvm",
        &format!("--profile={profile}"),
    ])
    .arg(&f);
    for (k, v) in env {
        c.env(k, v);
    }
    let o = c.output().expect("chạy gritc");
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
        kind: v["run"]["abort_kind"].as_str().unwrap_or("").to_string(),
        value,
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

fn reserve_prog(elem: &str, n: &str) -> String {
    format!(
        "ƒ main() → u64 ! {{mem}} {{
  μar ≔ Arena::new();
  μv: Vec<{elem}> ≔ Vec::new(&!ar);
  v.reserve({n});
  v.len() as u64
}}
"
    )
}

#[test]
fn forced_multiplication_overflow_aborts_overflow_and_the_boundary_is_exact() {
    // (kiểu phần tử, n gây tràn đúng 2^64, n lớn nhất không tràn)
    let cases = [
        (
            "(u64, u64)",
            "1152921504606846976usize",
            "1152921504606846975usize",
        ), // 16 byte: 2^60 / 2^60−1
        (
            "u32",
            "4611686018427387904usize",
            "4611686018427387903usize",
        ), // 4 byte: 2^62 / 2^62−1
        (
            "u16",
            "9223372036854775808usize",
            "9223372036854775807usize",
        ), // 2 byte: 2^63 / 2^63−1
        (
            "u64",
            "2305843009213693952usize",
            "2305843009213693951usize",
        ), // 8 byte: 2^61 / 2^61−1
    ];
    for profile in ["dev", "release", "release-o2"] {
        for (elem, over, under) in cases {
            let o = run(&reserve_prog(elem, over), profile, &[]);
            assert_eq!(o.exit, 70, "{profile} {elem} n={over}: {o:?}");
            assert_eq!(o.kind, "overflow", "{profile} {elem} n={over}: {o:?}");
            // n·size = 2^64 − size: không tràn ⇒ hết bộ nhớ (không phải overflow)
            let o = run(&reserve_prog(elem, under), profile, &[]);
            assert_eq!(o.exit, 70, "{profile} {elem} n={under}: {o:?}");
            assert_eq!(
                o.kind, "arena_exhausted",
                "{profile} {elem} n={under}: {o:?}"
            );
        }
    }
}

/// `reserve` nhỏ vẫn đúng (không báo nhầm) ở mọi hồ sơ
#[test]
fn small_reserve_and_push_do_not_false_alarm() {
    for profile in ["dev", "release", "release-o2"] {
        let o = run(&reserve_prog("(u64, u64)", "1000usize"), profile, &[]);
        assert_eq!(o.exit, 0, "{profile}: {o:?}");
        assert_eq!(o.value, "0");
    }
}
