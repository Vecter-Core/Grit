//! Backend LLVM (M10, ADR-029): chạy NHỊ PHÂN THẬT và đối chiếu với thông dịch tham chiếu. Thông dịch KHÔNG phải bằng chứng cho codegen
//! (Giang 2026-10-12): mỗi test kiểm cả kết quả mong đợi CỐ ĐỊNH (số tính tay) lẫn đồng nhất với thông dịch; hai điều đó độc lập.
//! Cần `LLVM-C.dll` trên PATH (t.sh / gate.sh đặt sẵn) và `runtime/gt-rt/target/release/gt_rt.lib` đã dựng.
#![cfg(feature = "llvm")]
use std::path::PathBuf;
use std::process::Command;

fn gritc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_gritc"))
}

fn scratch(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("gt-llvm-test-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

#[derive(Debug, PartialEq, Eq)]
struct Out {
    exit: i32,
    aborted: bool,
    abort_kind: String,
    /// số nguyên chuẩn hoá (bool ⇒ 0/1); `None` nếu `main` trả `()`
    value: Option<String>,
}

fn normalize(v: &serde_json::Value) -> Option<String> {
    let s = v.as_str()?;
    if s == "Unit" {
        return None;
    }
    if let Some(inner) = s.strip_prefix("Int(") {
        return inner.split(',').next().map(str::to_string);
    }
    match s {
        "Bool(true)" => Some("1".to_string()),
        "Bool(false)" => Some("0".to_string()),
        other => Some(other.to_string()),
    }
}

fn run(src: &str, backend: &str) -> Out {
    let dir = scratch(backend);
    let f = dir.join("prog.gt");
    std::fs::write(&f, src).expect("ghi nguồn");
    let o = gritc()
        .args(["run", "--json", &format!("--backend={backend}")])
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
            "JSON hỏng ({e}): {text}\nstderr: {}",
            String::from_utf8_lossy(&o.stderr)
        )
    });
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        v["backend"], backend,
        "đầu ra phải nêu đúng backend: {text}"
    );
    let run = &v["run"];
    Out {
        exit: o.status.code().unwrap_or(-1),
        aborted: run["aborted"].as_bool().unwrap_or(false),
        abort_kind: run["abort_kind"].as_str().unwrap_or("").to_string(),
        value: normalize(&run["value"]),
    }
}

/// Chạy bằng LLVM, kiểm kết quả mong đợi cố định, rồi đối chiếu thông dịch.
fn check_value(src: &str, expect: &str) {
    let l = run(src, "llvm");
    assert_eq!(l.exit, 0, "llvm: {l:?}\n{src}");
    assert_eq!(
        l.value.as_deref(),
        Some(expect),
        "llvm (giá trị mong đợi cố định)\n{src}"
    );
    let i = run(src, "interpreter");
    assert_eq!(i, l, "thông dịch ≠ llvm\n{src}");
}

fn check_abort(src: &str, kind: &str) {
    let l = run(src, "llvm");
    assert_eq!(l.exit, 70, "llvm: {l:?}\n{src}");
    assert!(l.aborted, "{l:?}");
    assert_eq!(l.abort_kind, kind, "loại abort của llvm\n{src}");
    let i = run(src, "interpreter");
    assert_eq!(i, l, "thông dịch ≠ llvm\n{src}");
}

#[test]
fn sum_loop() {
    check_value(
        "ƒ sum_to(n: u32) → u32 ! ∅ {\n  μs ≔ 0u32;\n  μi ≔ 0u32;\n  ⟳ i < n ↓ n - i {\n    s ← s +% i;\n    i ← i + 1;\n  }\n  s\n}\nƒ main() → u32 ! ∅ { sum_to(100) }\n",
        "4950",
    );
}

#[test]
fn recursion_factorial_u64() {
    check_value(
        "ƒ fact(n: u32) → u64 ! ∅ ↓ n { ⎇ n == 0 { 1 } ⇝ { n as u64 *% fact(n - 1) } }\nƒ main() → u64 ! ∅ { fact(10) }\n",
        "3628800",
    );
}

#[test]
fn signed_arithmetic_and_compare() {
    check_value(
        "ƒ f(a: i32, b: i32) → i32 ! ∅ { ⎇ a < b { b - a } ⇝ { a - b } }\nƒ main() → i32 ! ∅ { f(-7, 5) +% f(5, -7) }\n",
        "24",
    );
    // so sánh có dấu: -1 < 1 đúng; nếu codegen so sánh không dấu sẽ ra sai
    check_value(
        "ƒ lt(a: i8, b: i8) → bool ! ∅ { a < b }\nƒ main() → bool ! ∅ { lt(-1, 1) }\n",
        "1",
    );
    check_value(
        "ƒ lt(a: u8, b: u8) → bool ! ∅ { a < b }\nƒ main() → bool ! ∅ { lt(255, 1) }\n",
        "0",
    );
}

#[test]
fn wrapping_and_saturating() {
    check_value(
        "ƒ f(a: u8, b: u8) → u8 ! ∅ { a +% b }\nƒ main() → u8 ! ∅ { f(200, 100) }\n",
        "44",
    );
    check_value(
        "ƒ f(a: i8, b: i8) → i8 ! ∅ { a *% b }\nƒ main() → i8 ! ∅ { f(100, 3) }\n",
        "44",
    );
}

#[test]
fn casts() {
    check_value(
        "ƒ f(x: i32) → u8 ! ∅ { x as u8 }\nƒ main() → u8 ! ∅ { f(200) }\n",
        "200",
    );
    check_value(
        "ƒ f(x: i8) → i32 ! ∅ { x as i32 }\nƒ main() → i32 ! ∅ { f(-5) }\n",
        "-5",
    );
    check_value(
        "ƒ f(x: u8) → i32 ! ∅ { x as i32 }\nƒ main() → i32 ! ∅ { f(250) }\n",
        "250",
    );
}

#[test]
fn tuples_structs_arrays_refs() {
    check_value(
        "ƒ f() → i32 ! ∅ { μt ≔ (1, 2); a ≔ &!t.0; b ≔ &!t.1; *a ← 3; *b ← 4; t.0 +% t.1 }\nƒ main() → i32 ! ∅ { f() }\n",
        "7",
    );
    check_value(
        "▢ P { x: i32, y: i32 }\nƒ norm1(p: &P) → i32 ! ∅ { p.x +% p.y }\nƒ main() → i32 ! ∅ { p ≔ P { x: 3, y: 4 }; norm1(&p) }\n",
        "7",
    );
    check_value(
        "ƒ f() → i32 ! ∅ { a ≔ [10, 20, 30]; i ≔ 2usize; a[i] +% a[0] }\nƒ main() → i32 ! ∅ { f() }\n",
        "40",
    );
}

#[test]
fn abort_overflow_div_zero_bounds() {
    check_abort(
        "ƒ add(a: u8, b: u8) → u8 ! ∅ { a + b }\nƒ main() → u8 ! ∅ { add(200, 100) }\n",
        "overflow",
    );
    check_abort(
        "ƒ f(n: u32) → u32 ! ∅ { 100 / n }\nƒ main() → u32 ! ∅ { f(0) }\n",
        "div_by_zero",
    );
    check_abort(
        "ƒ f(n: u32) → u32 ! ∅ { 100 % n }\nƒ main() → u32 ! ∅ { f(0) }\n",
        "rem_by_zero",
    );
    check_abort(
        "ƒ f(i: usize) → i32 ! ∅ { a ≔ [1, 2, 3]; a[i] }\nƒ main() → i32 ! ∅ { f(7) }\n",
        "index_out_of_bounds",
    );
    check_abort(
        "ƒ f(x: i32) → u8 ! ∅ { x as u8 }\nƒ main() → u8 ! ∅ { f(300) }\n",
        "cast_lossy",
    );
}

#[test]
fn signed_min_div_minus_one_aborts_not_ub() {
    // MIN / -1 là UB của LLVM; phải abort (overflow), không được chạy tiếp hay crash
    check_abort(
        "ƒ f(a: i32, b: i32) → i32 ! ∅ { a / b }\nƒ main() → i32 ! ∅ { f(-2147483647 - 1, -1) }\n",
        "overflow",
    );
}

#[test]
fn red_program_produces_no_exe_and_no_run() {
    let dir = scratch("red");
    let f = dir.join("red.gt");
    std::fs::write(&f, "ƒ main() → u32 ! ∅ { x }\n").expect("ghi");
    let exe = dir.join("red.exe");
    let o = gritc()
        .args(["build", "--backend=llvm", "-o"])
        .arg(&exe)
        .arg(&f)
        .output()
        .expect("chạy");
    assert_eq!(
        o.status.code(),
        Some(1),
        "ĐỎ ⇒ mã 1, không phải mã chương trình"
    );
    assert!(!exe.exists(), "ĐỎ ⇒ KHÔNG được có exe");
    let o = gritc()
        .args(["run", "--backend=llvm"])
        .arg(&f)
        .output()
        .expect("chạy");
    assert_eq!(o.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(&dir);
}

/// ĐIỀU KIỆN CỦA GIANG: exe do gritc sinh KHÔNG phụ thuộc LLVM-C.dll lúc chạy (và không DLL lạ nào ngoài hệ thống/CRT).
#[test]
fn built_exe_does_not_depend_on_llvm_dll() {
    let dir = scratch("deps");
    let f = dir.join("prog.gt");
    std::fs::write(&f, "ƒ main() → u32 ! ∅ { 7 }\n").expect("ghi");
    let exe = dir.join("p.exe");
    let o = gritc()
        .args(["build", "--backend=llvm", "-o"])
        .arg(&exe)
        .arg(&f)
        .output()
        .expect("chạy");
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(exe.is_file());
    // chạy với PATH KHÔNG có LLVM: exe phải chạy bình thường
    let clean_path = std::env::var("PATH")
        .unwrap_or_default()
        .split(';')
        .filter(|d| !std::path::Path::new(d).join("LLVM-C.dll").exists())
        .collect::<Vec<_>>()
        .join(";");
    let r = Command::new(&exe)
        .env("PATH", clean_path)
        .output()
        .expect("chạy exe");
    assert_eq!(
        r.status.code(),
        Some(0),
        "exe không chạy được khi PATH không có LLVM"
    );
    // dumpbin /dependents
    let tool =
        find_msvc_tools::find_tool("x86_64-pc-windows-msvc", "dumpbin.exe").expect("dumpbin");
    let d = tool
        .to_command()
        .args(["/nologo", "/dependents"])
        .arg(&exe)
        .output()
        .expect("dumpbin");
    let text = String::from_utf8_lossy(&d.stdout).to_lowercase();
    assert!(
        text.contains("kernel32.dll"),
        "dumpbin không đọc được: {text}"
    );
    let allowed = [
        "kernel32.dll",
        "ntdll.dll",
        "userenv.dll",
        "ws2_32.dll",
        "dbghelp.dll",
        "vcruntime",
        "api-ms-win-crt",
        "advapi32.dll",
        "bcrypt.dll",
        "ucrtbase.dll",
        "msvcp",
        "api-ms-win-core",
    ];
    for l in text.lines().map(str::trim).filter(|l| l.ends_with(".dll")) {
        assert!(!l.contains("llvm"), "exe phụ thuộc LLVM: {l}");
        assert!(
            allowed.iter().any(|a| l.starts_with(a)),
            "DLL ngoài danh sách hệ thống/CRT: {l}\n{text}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// `gritc check` KHÔNG cần LLVM (nạp trễ): chạy với PATH không có LLVM-C.dll.
#[test]
fn check_runs_without_llvm_on_path() {
    let dir = scratch("nollvm");
    let f = dir.join("prog.gt");
    std::fs::write(&f, "ƒ main() → u32 ! ∅ { 7 }\n").expect("ghi");
    let clean_path = std::env::var("PATH")
        .unwrap_or_default()
        .split(';')
        .filter(|d| !std::path::Path::new(d).join("LLVM-C.dll").exists())
        .collect::<Vec<_>>()
        .join(";");
    let o = gritc()
        .env("PATH", &clean_path)
        .arg("check")
        .arg(&f)
        .output()
        .expect("chạy");
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    // backend llvm mà thiếu DLL ⇒ mã 3 + thông báo rõ, không sinh exe
    let exe = dir.join("p.exe");
    let o = gritc()
        .env("PATH", &clean_path)
        .args(["build", "--backend=llvm", "-o"])
        .arg(&exe)
        .arg(&f)
        .output()
        .expect("chạy");
    assert_eq!(
        o.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(String::from_utf8_lossy(&o.stderr).contains("LLVM-C.dll"));
    assert!(!exe.exists());
    let _ = std::fs::remove_dir_all(&dir);
}
