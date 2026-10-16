//! Kiểm ĐỘNG Arena của gt-rt (hợp đồng H2, mục C): ở hồ sơ `dev` mỗi lần `Arena::alloc`/`Vec`/`Text` cấp bộ nhớ, runtime ghi `[ptr, ptr+size)` và
//! kiểm (1) căn lề, (2) nằm trong chunk, (3) không chồng vùng nào cùng thế hệ (trước `reset`); sai ⇒ abort `arena_overlap` (kind 18, mã thoát 70).
//! Đây là BẰNG CHỨNG chạy thật ở hồ sơ dev, KHÔNG phải chứng minh; release/release-o2 KHÔNG kiểm (trừ khi `GRITRT_ARENA_CHECK=1`).
//! Chèn lỗi: `GRITC_FAULT=rt:arena-overlap|rt:arena-misalign` (⇒ `GRITRT_FAULT` cho exe) làm bộ cấp phát trả vùng chồng/lệch: kiểm động phải bắt.
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

/// nhiều lần cấp với căn lề khác nhau (u8, u64, u16) xen `Vec` và `Text`; một `reset` giữa chừng rồi cấp lại (thế hệ mới)
const PROG: &str = "ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  a ≔ ar.alloc(7u8);
  b ≔ ar.alloc(1000u64);
  c ≔ ar.alloc(30u16);
  d ≔ ar.alloc(5u64);
  s ≔ (*a as u64) + *b + (*c as u64) + *d;
  ar.reset();
  e ≔ ar.alloc(9u64);
  s2 ≔ s + *e;
  μar2 ≔ Arena::new();
  μv ≔ Vec::new(&!ar2);
  μi ≔ 0u64;
  ⟳ i < 300u64 ↓ 300u64 - i { v.push(i); i ← i + 1u64; }
  μar3 ≔ Arena::new();
  μt ≔ Text::new(&!ar3);
  t.push_str(\"xin chao\");
  s2 + (v.len() as u64) + (t.len() as u64)
}
";
// 7 + 1000 + 30 + 5 = 1042; reset (thế hệ mới) rồi + 9 = 1051; + 300 + 8 = 1359
const EXPECT: &str = "1359";

#[test]
fn valid_programs_pass_the_dynamic_check_in_dev() {
    let o = run(PROG, "dev", &[]);
    assert_eq!(o.exit, 0, "{o:?}");
    assert_eq!(o.value, EXPECT);
}

#[test]
fn injected_overlap_is_caught_in_dev() {
    let o = run(PROG, "dev", &[("GRITC_FAULT", "rt:arena-overlap")]);
    assert_eq!(o.exit, 70, "{o:?}");
    assert_eq!(o.kind, "arena_overlap", "{o:?}");
    assert!(o.stderr.contains("abort: arena_overlap"), "{o:?}");
    assert!(o.stderr.contains("chồng lấn"), "{o:?}");
}

#[test]
fn injected_misalignment_is_caught_in_dev() {
    let o = run(PROG, "dev", &[("GRITC_FAULT", "rt:arena-misalign")]);
    assert_eq!(o.exit, 70, "{o:?}");
    assert_eq!(o.kind, "arena_overlap", "{o:?}");
    assert!(o.stderr.contains("lệch căn lề"), "{o:?}");
}

/// kiểm động chỉ ở dev: release-o2 KHÔNG kiểm ⇒ chèn lỗi bị bỏ qua (chương trình chạy bình thường) — ghi rõ là GIỚI HẠN của bằng chứng
#[test]
fn release_profiles_do_not_check_unless_forced() {
    for p in ["release", "release-o2"] {
        let o = run(PROG, p, &[("GRITC_FAULT", "rt:arena-overlap")]);
        assert_eq!(o.exit, 0, "{p}: {o:?}");
        assert_eq!(o.value, EXPECT, "{p}");
        // ép bật bằng biến môi trường ⇒ bắt
        let o = run(
            PROG,
            p,
            &[
                ("GRITC_FAULT", "rt:arena-overlap"),
                ("GRITRT_ARENA_CHECK", "1"),
            ],
        );
        assert_eq!(o.kind, "arena_overlap", "{p}: {o:?}");
    }
}

/// tắt kiểm bằng `GRITRT_ARENA_CHECK=0` ở dev ⇒ lỗi chèn không bị bắt
#[test]
fn dev_check_can_be_switched_off_explicitly() {
    let o = run(
        PROG,
        "dev",
        &[
            ("GRITC_FAULT", "rt:arena-overlap"),
            ("GRITRT_ARENA_CHECK", "0"),
        ],
    );
    assert_eq!(o.exit, 0, "{o:?}");
}

/// bố cục xác định để cô lập từng nhánh kiểm: `pad1` (u8, ở đầu chunk), `pad2` (u64, căn lề 8 ⇒ chừa KHE 7 byte sau `pad1`), rồi `x` = `[u8; 4]` (căn lề 1):
/// - `arena_overlap_tail` đặt `x` chồng 1 byte vào ĐUÔI `pad2` ⇒ chỉ nhánh "vùng đứng trước" bắt được;
/// - `arena_overlap_head` đặt `x` chồng đúng 1 byte vào ĐẦU `pad2` mà không chạm `pad1` (nhờ khe) ⇒ chỉ nhánh "vùng đứng sau" bắt được (và biên `<` đúng 1 byte).
const PROG2: &str = "ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  p1 ≔ ar.alloc(1u8);
  p2 ≔ ar.alloc(2u64);
  x ≔ ar.alloc([1u8, 2u8, 3u8, 4u8]);
  (*p1 as u64) + *p2 + (x[0] as u64) + (x[3] as u64)
}
";

#[test]
fn isolated_layout_runs_clean_and_each_injected_fault_is_caught_by_its_own_branch() {
    let o = run(PROG2, "dev", &[]);
    assert_eq!(o.exit, 0, "{o:?}");
    assert_eq!(o.value, "8");
    for (fault, why) in [
        ("rt:arena-overlap", "chồng lấn"),
        ("rt:arena-overlap-tail", "chồng lấn"),
        ("rt:arena-overlap-head", "chồng lấn"),
        ("rt:arena-outside", "ngoài chunk"),
    ] {
        let o = run(PROG2, "dev", &[("GRITC_FAULT", fault)]);
        assert_eq!(o.exit, 70, "{fault}: {o:?}");
        assert_eq!(o.kind, "arena_overlap", "{fault}: {o:?}");
        assert!(o.stderr.contains(why), "{fault}: {o:?}");
    }
}
