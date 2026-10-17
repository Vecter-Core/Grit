//! `Vec<'a, T>` trên Arena (ADR-032 phụ lục A, bước (ii)): chạy NHỊ PHÂN THẬT và đối chiếu với thông dịch; abort `arena_exhausted` chỉ
//! kiểm trên exe (thông dịch không mô phỏng bộ nhớ). Cần `LLVM-C.dll` trên PATH.
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

fn gritc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_gritc"))
}

fn scratch(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("gt-vec-test-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

#[derive(Debug, PartialEq, Eq)]
struct Out {
    exit: i32,
    aborted: bool,
    kind: String,
    value: String,
}

fn run_env(src: &str, backend: &str, env: &[(&str, &str)]) -> Out {
    let d = scratch(backend);
    let f = d.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let mut c = gritc();
    c.args(["run", "--json", &format!("--backend={backend}")])
        .arg(profile_arg())
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
        aborted: v["run"]["aborted"].as_bool().unwrap_or(false),
        kind: v["run"]["abort_kind"].as_str().unwrap_or("").to_string(),
        value,
    }
}

/// hai backend cùng cho `expect`, mã thoát 0
fn both(src: &str, expect: &str) {
    let l = run_env(src, "llvm", &[]);
    assert_eq!(l.exit, 0, "llvm: {l:?}\n{src}");
    assert_eq!(l.value, expect, "llvm\n{src}");
    let i = run_env(src, "interpreter", &[]);
    assert_eq!(i, l, "thông dịch ≠ llvm\n{src}");
}

const SUM_LOOP: &str = "ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  μv ≔ Vec::new(&!ar);
  μi ≔ 0u64;
  ⟳ i < 1000u64 ↓ 1000u64 - i { v.push(i *% 3u64); i ← i + 1u64; }
  s ≔ v.as_slice();
  μt ≔ 0u64;
  μj ≔ 0usize;
  ⟳ j < s.len() ↓ s.len() - j { t ← t +% s[j]; j ← j + 1usize; }
  t +% (v.len() as u64)
}
";

/// 3·(0+…+999) + 1000 = 1 498 500 + 1000 = 1 499 500; tăng trưởng 4 → 8 → … → 1024 (9 lần cấp khối mới) giữ nguyên nội dung.
#[test]
fn push_grow_slice_and_len_agree_on_both_backends() {
    both(SUM_LOOP, "1499500");
}

/// phần tử là struct có căn lề hỗn hợp (u8, u64, u16): kích thước/căn lề lấy từ bố cục thật; 9 phần tử ⇒ qua hai lần tăng trưởng.
#[test]
fn struct_elements_with_mixed_alignment() {
    let src = "▢ P { a: u8, b: u64, c: u16 }
ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  μv ≔ Vec::new(&!ar);
  μi ≔ 0u8;
  ⟳ i < 9u8 ↓ 9u8 - i { v.push(P { a: i, b: (i as u64) *% 10u64, c: (i as u16) *% 100u16 }); i ← i + 1u8; }
  s ≔ v.as_slice();
  μt ≔ 0u64;
  μj ≔ 0usize;
  ⟳ j < s.len() ↓ s.len() - j { t ← t +% (s[j].a as u64) +% s[j].b +% (s[j].c as u64); j ← j + 1usize; }
  t
}
";
    // Σa = 36, Σb = 360, Σc = 3600
    both(src, "3996");
}

/// phần tử kích thước 0 (`()`): `size = 0` ⇒ cấp 0 byte; len vẫn đếm đúng.
#[test]
fn zero_sized_elements_count_correctly() {
    let src = "ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  μv ≔ Vec::new(&!ar);
  μi ≔ 0u32;
  ⟳ i < 10u32 ↓ 10u32 - i { v.push(()); i ← i + 1u32; }
  v.len() as u64
}
";
    both(src, "10");
}

/// `Vec<&[u8]>` (dạng dùng cho sortlines): phần tử là con trỏ béo; lát đọc lại đúng nội dung.
#[test]
fn vec_of_slice_refs() {
    let src = "ƒ main() → u64 ! {mem} {
  a ≔ [1u8, 2u8, 3u8, 4u8, 5u8];
  μar ≔ Arena::new();
  μv ≔ Vec::new(&!ar);
  v.push(&a[0..2]);
  v.push(&a[1..5]);
  v.push(&a[4..5]);
  v.push(&a[0..0]);
  v.push(&a[2..4]);
  s ≔ v.as_slice();
  (s[0].len() as u64) *% 10000u64 +% (s[1].len() as u64) *% 1000u64 +% (s[2][0] as u64) *% 100u64 +% (s[3].len() as u64) *% 10u64 +% (s[4][1] as u64)
}
";
    // 2·10000 + 4·1000 + 5·100 + 0·10 + 4
    both(src, "24504");
}

/// `as_mut_slice`: ghi qua lát sửa đúng phần tử; sau đó đọc bằng `as_slice`.
#[test]
fn write_through_mutable_slice() {
    let src = "ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  μv ≔ Vec::new(&!ar);
  v.push(1u64);
  v.push(2u64);
  v.push(3u64);
  m ≔ v.as_mut_slice();
  m[1] ← 40u64;
  v.as_slice()[0] +% v.as_slice()[1] +% v.as_slice()[2]
}
";
    both(src, "44");
}

/// Chỉ số ngoài biên qua lát của Vec: abort `index_out_of_bounds` ở cả hai backend.
#[test]
fn index_out_of_bounds_through_the_slice_aborts_on_both() {
    let src = "ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  μv ≔ Vec::new(&!ar);
  v.push(1u64);
  v.as_slice()[1]
}
";
    let l = run_env(src, "llvm", &[]);
    assert_eq!(
        (l.exit, l.aborted, l.kind.as_str()),
        (70, true, "index_out_of_bounds"),
        "{l:?}"
    );
    let i = run_env(src, "interpreter", &[]);
    assert_eq!(i, l, "thông dịch ≠ llvm");
}

/// Tăng trưởng 10⁶ phần tử trên exe (thông dịch quá chậm cho cỡ này): tổng 0..10⁶ − 1 và len.
#[test]
fn growth_to_one_million_elements() {
    let src = "ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  μv ≔ Vec::new(&!ar);
  μi ≔ 0u64;
  ⟳ i < 1000000u64 ↓ 1000000u64 - i { v.push(i); i ← i + 1u64; }
  s ≔ v.as_slice();
  μt ≔ 0u64;
  μj ≔ 0usize;
  ⟳ j < s.len() ↓ s.len() - j { t ← t +% s[j]; j ← j + 1usize; }
  t +% (v.len() as u64)
}
";
    let l = run_env(src, "llvm", &[]);
    assert_eq!(l.exit, 0, "{l:?}");
    // Σ i (i<10⁶) = 499 999 500 000; + 1 000 000
    assert_eq!(l.value, "500000500000");
}

/// Hết Arena ⇒ abort riêng `arena_exhausted` (kind 17, mã thoát 70). Dùng giới hạn của runtime (`GRITRT_ARENA_LIMIT`, chỉ làm cấp phát
/// thất bại SỚM hơn): 2 MiB mỗi arena, đẩy 10⁶ u64 (8 MiB) ⇒ cạn. Không đặt giới hạn thì cùng chương trình chạy bình thường.
#[test]
fn arena_exhaustion_is_its_own_abort_kind() {
    let src = "ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  μv ≔ Vec::new(&!ar);
  μi ≔ 0u64;
  ⟳ i < 1000000u64 ↓ 1000000u64 - i { v.push(i); i ← i + 1u64; }
  v.len() as u64
}
";
    let limited = run_env(src, "llvm", &[("GRITRT_ARENA_LIMIT", "2097152")]);
    assert_eq!(
        (limited.exit, limited.aborted, limited.kind.as_str()),
        (70, true, "arena_exhausted"),
        "{limited:?}"
    );
    let free = run_env(src, "llvm", &[]);
    assert_eq!((free.exit, free.value.as_str()), (0, "1000000"), "{free:?}");
}

/// R3 (FAIL_OPEN_AUDIT): giá trị SAI của `GRITRT_ARENA_LIMIT` không được âm thầm thành "không giới hạn" — runtime abort `arena_exhausted`.
#[test]
fn malformed_arena_limit_is_not_silently_unlimited() {
    let src = "ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  x ≔ ar.alloc(7u64);
  *x
}
";
    for bad in ["2MiB", "-1", "", "1.5"] {
        let r = run_env(src, "llvm", &[("GRITRT_ARENA_LIMIT", bad)]);
        assert_eq!(
            (r.exit, r.kind.as_str()),
            (70, "arena_exhausted"),
            "giá trị {bad:?}: {r:?}"
        );
    }
    let ok = run_env(src, "llvm", &[("GRITRT_ARENA_LIMIT", "1048576")]);
    assert_eq!((ok.exit, ok.value.as_str()), (0, "7"), "{ok:?}");
}

/// `Arena::alloc` cũng dùng chung đường `arena_exhausted`.
#[test]
fn arena_alloc_exhaustion_uses_the_same_kind() {
    let src = "ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  μi ≔ 0u64;
  μt ≔ 0u64;
  ⟳ i < 300000u64 ↓ 300000u64 - i { x ≔ ar.alloc(i); t ← t +% *x; i ← i + 1u64; }
  t
}
";
    let limited = run_env(src, "llvm", &[("GRITRT_ARENA_LIMIT", "1048576")]);
    assert_eq!(
        (limited.exit, limited.kind.as_str()),
        (70, "arena_exhausted"),
        "{limited:?}"
    );
}

/// Báo cáo "check còn lại" liệt kê MỖI lời gọi có thể hết Arena (loại `arena_exhausted`, trạng thái `intrinsic_runtime`) ở cả hai chế độ.
#[test]
fn checks_report_lists_arena_exhaustion_sites() {
    let d = scratch("rep");
    let f = d.join("prog.gt");
    std::fs::write(&f, SUM_LOOP).expect("ghi");
    for mode in ["all", "proven"] {
        let o = gritc()
            .args(["check", "--json", &format!("--checks={mode}")])
            .arg(&f)
            .output()
            .expect("chạy");
        let text = String::from_utf8_lossy(&o.stdout).into_owned();
        let v: serde_json::Value = serde_json::from_str(text.lines().last().unwrap_or(""))
            .unwrap_or_else(|e| panic!("JSON ({e}): {text}"));
        let kept = v["checks"]["kept"].as_array().expect("kept");
        let n = kept
            .iter()
            .filter(|c| c["kind"] == "arena_exhausted" && c["status"] == "intrinsic_runtime")
            .count();
        assert_eq!(
            n, 1,
            "{mode}: đúng một lời gọi `push` trong SUM_LOOP: {kept:?}"
        );
    }
}

/// `reserve(n)`: một lần cấp khối đủ `n` phần tử (không gấp đôi); nội dung cũ được giữ; `n ≤ cap` không làm gì; hai backend khớp.
#[test]
fn reserve_preallocates_and_keeps_contents() {
    let src = "ƒ main() → u64 ! {mem} {
  μar ≔ Arena::new();
  μv ≔ Vec::new(&!ar);
  v.push(7u64);
  v.push(8u64);
  v.reserve(1000usize);
  v.reserve(10usize);
  μi ≔ 0u64;
  ⟳ i < 1000u64 ↓ 1000u64 - i { v.push(i); i ← i + 1u64; }
  s ≔ v.as_slice();
  s[0] *% 1000000u64 +% s[1] *% 10000u64 +% s[2] +% s[1001] +% (v.len() as u64)
}
";
    // 7·10⁶ + 8·10⁴ + 0 + 999 + 1002 = 7 082 001
    both(src, "7082001");
}

/// công thức cỡ của sortlines: `reserve(n)` cho Vec cần ĐÚNG một khối `n·16 + align` ⇒ với giới hạn = khối đó chạy được, ít hơn 1 byte
/// thì abort `arena_exhausted` (kiểm chặt: công thức không dư thừa, không thiếu).
#[test]
fn reserve_arena_demand_is_exactly_one_block() {
    let src = "▢ L<'a> { b: &'a [u8], n: u64 }
ƒ main() → u64 ! {mem} {
  e ≔ [0u8];
  μar ≔ Arena::new();
  μv ≔ Vec::new(&!ar);
  v.reserve(100000usize);
  μi ≔ 0u64;
  ⟳ i < 100000u64 ↓ 100000u64 - i { v.push(L { b: &e[0..0], n: i }); i ← i + 1u64; }
  v.len() as u64
}
";
    // khối = 100000·24 (L = {&[u8], u64} = 24 byte) + 8 (căn lề 8) = 2_400_008 > chunk 1 MiB
    let ok = run_env(src, "llvm", &[("GRITRT_ARENA_LIMIT", "2400008")]);
    assert_eq!((ok.exit, ok.value.as_str()), (0, "100000"), "{ok:?}");
    let tight = run_env(src, "llvm", &[("GRITRT_ARENA_LIMIT", "2400007")]);
    assert_eq!(
        (tight.exit, tight.kind.as_str()),
        (70, "arena_exhausted"),
        "{tight:?}"
    );
}
