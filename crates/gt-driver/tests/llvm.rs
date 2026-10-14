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

/// literal số thực có dấu `_` và hậu tố `f64`: `1_000.5` phải thành 1000,5 (đột biến thay `_` bằng khoảng trắng ⇒ parse lỗi ⇒ ICE)
#[test]
fn float_literal_with_underscores_and_suffix() {
    check_value(
        "ƒ main() → u64 ! ∅ { ⌥ (1_000.5f64).to_i64() { Some(v) ↦ v as u64, None ↦ 7u64 } }
",
        "1000",
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

const SUM_SLICE: &str = "ƒ sum(xs: &[i64]) → i64 ! ∅ {
  μs ≔ 0;
  μi ≔ 0;
  ⟳ i < xs.len() ↓ xs.len() - i {
    s ← s + xs[i];
    i ← i + 1;
  }
  s
}
";

#[test]
fn slices_subslices_and_bounds() {
    let src = format!(
        "{SUM_SLICE}ƒ main() → i64 ! ∅ {{ a ≔ [1i64, 2, 3, 4, 5]; sum(&a[..]) +% sum(&a[1..4]) }}\n"
    );
    check_value(&src, "24"); // 15 + (2+3+4)
    // cắt ngoài biên ⇒ abort (không đọc ngoài mảng)
    let bad = format!(
        "{SUM_SLICE}ƒ f(n: usize) → i64 ! ∅ {{ a ≔ [1i64, 2, 3]; sum(&a[0..n]) }}\nƒ main() → i64 ! ∅ {{ f(9) }}\n"
    );
    let l = run(&bad, "llvm");
    assert_eq!((l.exit, l.aborted), (70, true), "{l:?}");
    assert_eq!(l.abort_kind, "slice_range");
    assert_eq!(run(&bad, "interpreter"), l, "thông dịch ≠ llvm");
}

#[test]
fn enums_generics_option_monomorphization() {
    check_value(
        "⊔ Shape { Circle(i32), Rect(i32, i32), Empty }\nƒ area(s: Shape) → i32 ! ∅ { ⌥ s { Circle(r) ↦ r *% r, Rect(w, h) ↦ w *% h, Empty ↦ 0 } }\nƒ pick(o: Option<i32>) → i32 ! ∅ { ⌥ o { Some(v) ↦ v, None ↦ 0 } }\nƒ id<T>(x: T) → T ! ∅ { x }\nƒ main() → i32 ! ∅ {\n  area(Shape::Circle(3)) +% area(Shape::Rect(4, 5)) +% area(Shape::Empty) +% pick(Some(7)) +% pick(None) +% id(5i32) +% (id(2u8) as i32)\n}\n",
        "43",
    );
}

#[test]
fn string_literal_is_a_fat_pointer() {
    check_value(
        "ƒ f(s: &str) → u32 ! ∅ { 7 }\nƒ main() → u32 ! ∅ { f(\"héllo\") }\n",
        "7",
    );
}

const THREADS: &str = "#level(\"1\")\n▢ Lo { }\n■ COUNT: Atomic<u64> ≔ Atomic::new(0u64);\n■ SUM: Mutex<u64, Lo> ≔ Mutex::new(0u64);\nƒ add_one(v: &!u64, n: u64) → u64 ! ∅ { *v ← *v +% n; *v }\nƒ read(v: &!u64, n: u64) → u64 ! ∅ { *v }\nƒ worker(n: u32) ! {lock(Lo)} {\n  μi ≔ 0u32;\n  ⟳ i < 1000 ↓ 1000 - i {\n    SUM.with(add_one, 1u64);\n    COUNT.fetch_add(1u64, AtomicOrdering::Release);\n    i ← i + 1;\n  }\n}\nƒ main() → u64 ! {lock(Lo)} {\n  spawn(worker, 0u32);\n  spawn(worker, 1u32);\n  spawn(worker, 2u32);\n  spawn(worker, 3u32);\n  μspin ≔ 0u64;\n  ⟳ spin < 4000000000u64 ∧ COUNT.load(AtomicOrdering::Acquire) < 4000 ↓ 4000000000 - spin {\n    spin ← spin + 1;\n  }\n  COUNT.load(AtomicOrdering::SeqCst) +% SUM.with(read, 0u64)\n}\n";

/// Đa luồng THẬT: 4 luồng HĐH × 1000 lần (khoá thật + atomic thật). Không có đua dữ liệu thì tổng luôn 8000; lặp nhiều lần để lộ lỗi.
#[test]
fn real_threads_atomic_and_static_mutex() {
    for round in 0..8 {
        let l = run(THREADS, "llvm");
        assert_eq!(l.exit, 0, "vòng {round}: {l:?}");
        assert_eq!(
            l.value.as_deref(),
            Some("8000"),
            "vòng {round}: mất cập nhật ⇒ khoá/atomic sai"
        );
    }
}

/// IR phải dùng atomics/khoá THẬT (không phải load/store thường) với đúng thứ tự bộ nhớ đã viết trong mã nguồn.
#[test]
fn threads_ir_uses_real_atomics_with_requested_orderings() {
    let dir = scratch("ir");
    let f = dir.join("prog.gt");
    std::fs::write(&f, THREADS).expect("ghi");
    let ir_path = dir.join("out.ll");
    let o = gritc()
        .env("GRITC_DUMP_IR", &ir_path)
        .args(["build", "--backend=llvm", "-o"])
        .arg(dir.join("t.exe"))
        .arg(&f)
        .output()
        .expect("chạy");
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let ir = std::fs::read_to_string(&ir_path).expect("IR");
    assert!(
        ir.contains("atomicrmw add ptr") && ir.contains(" release"),
        "fetch_add phải là atomicrmw add release:
{ir}"
    );
    assert!(
        ir.contains("load atomic i64") && ir.contains("acquire"),
        "load Acquire:
{ir}"
    );
    assert!(ir.contains("seq_cst"), "load SeqCst");
    assert!(
        ir.contains("call void @gritrt_mutex_lock")
            && ir.contains("call void @gritrt_mutex_unlock")
    );
    assert!(ir.contains("call void @gritrt_spawn"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Chưa hỗ trợ ⇒ gritc thoát 3 với thông báo rõ và KHÔNG sinh exe (fail-closed, không đoán).
#[test]
fn unsupported_construct_is_refused_without_exe() {
    let dir = scratch("unsup");
    let f = dir.join("prog.gt");
    // mảng cỡ > u32 không hạ được (số thực f64 đã được hỗ trợ từ 2026-10-14)
    std::fs::write(&f, "ƒ main() → u8 ! ∅ { a ≔ [0u8; 5000000000]; a[0] }\n").expect("ghi");
    let exe = dir.join("u.exe");
    let o = gritc()
        .args(["build", "--backend=llvm", "-o"])
        .arg(&exe)
        .arg(&f)
        .output()
        .expect("chạy");
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    assert_eq!(o.status.code(), Some(3), "{err}");
    assert!(err.contains("backend llvm chưa hỗ trợ"), "{err}");
    assert!(!exe.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// IR Guard (E1101) ở mức tiến trình: IR bị làm hỏng có chủ đích ⇒ gritc ĐỎ (mã 1), KHÔNG exe, KHÔNG chạy.
#[test]
fn ir_guard_rejects_tampered_ir_at_process_level() {
    for (fault, needle) in [
        ("ir:unreachable", "không đúng mẫu abort"),
        ("ir:strip-obl", "thiếu metadata !gt.obl"),
        ("ir:attest", "attestation lệch"),
    ] {
        let dir = scratch("guard");
        let f = dir.join("prog.gt");
        std::fs::write(&f, THREADS).expect("ghi");
        let exe = dir.join("g.exe");
        let o = gritc()
            .env("GRITC_FAULT", fault)
            .args(["build", "--backend=llvm", "-o"])
            .arg(&exe)
            .arg(&f)
            .output()
            .expect("chạy");
        let err = String::from_utf8_lossy(&o.stderr).into_owned();
        assert_eq!(o.status.code(), Some(1), "{fault}: {err}");
        assert!(
            err.contains("E1101") && err.contains(needle),
            "{fault}: {err}"
        );
        assert!(!exe.exists(), "{fault}: ĐỎ ⇒ KHÔNG được có exe");
        assert!(
            !dir.join("g.exe.obj").exists(),
            "{fault}: không để lại object"
        );
        // run: cũng mã 1 và không chạy chương trình (đa luồng sẽ in kết quả nếu chạy)
        let o = gritc()
            .env("GRITC_FAULT", fault)
            .args(["run", "--json", "--backend=llvm"])
            .arg(&f)
            .output()
            .expect("chạy");
        assert_eq!(o.status.code(), Some(1), "{fault}");
        let text = String::from_utf8_lossy(&o.stdout).into_owned();
        let v: serde_json::Value = serde_json::from_str(text.lines().last().unwrap_or(""))
            .unwrap_or_else(|e| panic!("JSON ({e}): {text}"));
        assert_eq!(v["verdict"], "red", "{fault}");
        assert!(
            v["run"].is_null() || v.get("run").is_none(),
            "{fault}: không được có kết quả chạy"
        );
        assert!(text.contains("E1101"), "{fault}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// exe mang attestation của lần check (chứng cứ truy vết): chuỗi có trong nhị phân và khớp JSON của `build`.
#[test]
fn exe_carries_the_attestation_of_the_check() {
    let dir = scratch("att");
    let f = dir.join("prog.gt");
    std::fs::write(&f, "ƒ main() → u32 ! ∅ { 7 }\n").expect("ghi");
    let exe = dir.join("a.exe");
    let o = gritc()
        .args(["build", "--json", "--backend=llvm", "-o"])
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
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let v: serde_json::Value =
        serde_json::from_str(text.lines().last().unwrap_or("")).expect("JSON");
    let att = v["attestation"].as_str().expect("attestation").to_string();
    assert_eq!(att.len(), 64, "{att}");
    let bytes = std::fs::read(&exe).expect("đọc exe");
    assert!(
        bytes.windows(att.len()).any(|w| w == att.as_bytes()),
        "exe không mang attestation {att}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn sample(name: &str) -> (String, String) {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/samples")
        .join(format!("{name}.gt"));
    let src = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("không đọc được {}: {e}", p.display()))
        .replace("\r\n", "\n");
    let expect = src
        .lines()
        .find_map(|l| l.strip_prefix("// expect-result:"))
        .unwrap_or_else(|| panic!("{name}: thiếu `// expect-result:`"))
        .trim()
        .to_string();
    (src, expect)
}

/// 3 chương trình mẫu THẬT (ADR-029 §7 (a)): từ nguồn Grit tới nhị phân, kết quả đúng với số tính tay trong tệp.
#[test]
fn sample_numeric_loop_and_recursion() {
    let (src, expect) = sample("numeric");
    check_value(&src, &expect);
}

#[test]
fn sample_arrays_and_slices_with_checked_bounds() {
    let (src, expect) = sample("arrays_slices");
    check_value(&src, &expect);
}

#[test]
fn sample_real_threads_atomic_static_mutex_spawn() {
    let (src, expect) = sample("threads");
    // thông dịch tham chiếu không hỗ trợ static/spawn: KHÔNG có đối chiếu; chỉ giá trị cố định, lặp để lộ đua dữ liệu
    for round in 0..5 {
        let l = run(&src, "llvm");
        assert_eq!(l.exit, 0, "vòng {round}: {l:?}");
        assert_eq!(l.value.as_deref(), Some(expect.as_str()), "vòng {round}");
    }
}

/// Mặc định (không `--backend`) là llvm khi bản dựng có feature `llvm` (ADR-029 Q63) và đầu ra nêu rõ backend.
#[test]
fn default_backend_is_llvm_and_is_stated() {
    let dir = scratch("default");
    let f = dir.join("prog.gt");
    std::fs::write(&f, "ƒ main() → u32 ! ∅ { 7 }\n").expect("ghi");
    let o = gritc()
        .args(["run", "--json"])
        .arg(&f)
        .output()
        .expect("chạy");
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let v: serde_json::Value =
        serde_json::from_str(text.lines().last().unwrap_or("")).expect("JSON");
    assert_eq!(v["backend"], "llvm", "{text}");
    assert_eq!(v["run"]["value"], "7");
    let o = gritc().arg(&f).arg("run").output().expect("chạy");
    assert_ne!(o.status.code(), Some(0)); // lệnh phải đứng trước
    let o = gritc().arg("--version").output().expect("chạy");
    assert!(String::from_utf8_lossy(&o.stdout).contains("backend mặc định: llvm"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Golden test trên LLVM IR (ADR-029 §7 (c)): IR của vài chương trình nhỏ phải KHÔNG đổi ngoài ý muốn. Cập nhật có chủ đích:
/// `GT_UPDATE_GOLDEN=1` rồi xem `git diff tests/golden_ll`. IR phụ thuộc đúng LLVM 20.1.8 (docs/ENVIRONMENT.md).
fn golden(name: &str, src: &str) {
    let dir = scratch(name);
    let f = dir.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let ir_path = dir.join("out.ll");
    let o = gritc()
        .env("GRITC_DUMP_IR", &ir_path)
        .args(["build", "--backend=llvm", "-o"])
        .arg(dir.join("g.exe"))
        .arg(&f)
        .output()
        .expect("chạy");
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let ir = std::fs::read_to_string(&ir_path)
        .expect("IR")
        .replace("\r\n", "\n");
    let gold = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden_ll")
        .join(format!("{name}.ll"));
    if std::env::var_os("GT_UPDATE_GOLDEN").is_some() {
        std::fs::write(&gold, &ir).expect("ghi golden");
    } else {
        let want = std::fs::read_to_string(&gold)
            .unwrap_or_else(|e| {
                panic!(
                    "thiếu {}: {e} (chạy với GT_UPDATE_GOLDEN=1)",
                    gold.display()
                )
            })
            .replace("\r\n", "\n");
        assert_eq!(
            ir,
            want,
            "IR của `{name}` đổi so với golden {}",
            gold.display()
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn golden_ir_checked_add_and_abort_blocks() {
    golden(
        "checked_add",
        "ƒ add(a: u8, b: u8) → u8 ! ∅ { a + b }\nƒ main() → u8 ! ∅ { add(200, 100) }\n",
    );
}

#[test]
fn golden_ir_slice_bounds() {
    golden(
        "slice_sum",
        &format!("{SUM_SLICE}ƒ main() → i64 ! ∅ {{ a ≔ [1i64, 2, 3]; sum(&a[..]) }}\n"),
    );
}

#[test]
fn golden_ir_static_atomic_and_mutex() {
    golden(
        "static_atomic",
        "■ COUNT: Atomic<u32> ≔ Atomic::new(0u32);\nƒ worker(n: u32) ! ∅ { COUNT.fetch_add(n, AtomicOrdering::Relaxed); }\nƒ main() → u32 ! ∅ { worker(1u32); COUNT.load(AtomicOrdering::SeqCst) }\n",
    );
}

const IO_PROG: &str = "ƒ say(out: &Cap<Out>) ! {io} { out.write_u64(42u64); }\nƒ main() ! {io} { out ≔ root_out(); say(&out); }\n";

/// Lớp nghĩa vụ I/O (E0310) ở mức tiến trình: chương trình I/O chạy đúng; IR bị bỏ nhãn / thêm lời gọi runtime lạ / hàm ngoài ⇒ ĐỎ E1101.
#[test]
fn io_calls_carry_a_class_and_guard_rejects_tampered_io_ir() {
    let dir = scratch("io");
    let f = dir.join("prog.gt");
    std::fs::write(&f, IO_PROG).expect("ghi");
    let exe = dir.join("io.exe");
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
    let r = Command::new(&exe).output().expect("chạy exe");
    assert_eq!(String::from_utf8_lossy(&r.stdout), "42");
    for (fault, needle) in [
        ("ir:strip-obl", "`E0310`"),
        ("ir:runtime-call", "`gritrt_exec` không thuộc lớp nào"),
        ("ir:external-call", "hàm ngoài `system`"),
    ] {
        let exe2 = dir.join("io2.exe");
        let _ = std::fs::remove_file(&exe2);
        let o = gritc()
            .env("GRITC_FAULT", fault)
            .args(["build", "--backend=llvm", "-o"])
            .arg(&exe2)
            .arg(&f)
            .output()
            .expect("chạy");
        let err = String::from_utf8_lossy(&o.stderr).into_owned();
        assert_eq!(o.status.code(), Some(1), "{fault}: {err}");
        assert!(
            err.contains("E1101") && err.contains(needle),
            "{fault}: {err}"
        );
        assert!(!exe2.exists(), "{fault}: ĐỎ ⇒ KHÔNG exe");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// FB-2 (hồi quy mức tiến trình): hàm hợp đồng `⊳`/`⊲` được gọi như hàm thường có tên Unicode trong IR; IR Guard không được từ chối.
#[test]
fn contract_functions_with_unicode_symbol_names_pass_the_guard() {
    check_value(
        "ƒ h(p: u32) → u32 ! ∅ ⊳ p < 10u32 ⊲ result >= 0u32 { p }\nƒ entry(a: u32) → u32 ! ∅ { h(a) }\nƒ main() → u32 ! ∅ { entry(3u32) }\n",
        "3",
    );
}

/// Nhiên liệu cạn là một loại abort RIÊNG (`fuel_exhausted`), hai backend khớp, và check xuất hiện trong báo cáo "check còn lại" (loại `fuel`, ghi chú N0905).
#[test]
fn fuel_exhaustion_is_its_own_abort_kind_and_is_reported() {
    let src = "ƒ entry(n: u64) → u64 ! ∅ {\n  μfuel ≔ n;\n  μi ≔ 0u64;\n  ⟳ fuel > 0u64 ∧ i < 1000u64 ↓ fuel { fuel ← fuel - 1u64; i ← i + 1u64; }\n  core::fuel_left(fuel);\n  i\n}\nƒ main() → u64 ! ∅ { entry(5u64) }\n";
    check_abort(src, "fuel_exhausted");
    // đủ nhiên liệu: chạy bình thường, kết quả 1000
    let ok = src.replace("entry(5u64)", "entry(2000u64)");
    check_value(&ok, "1000");
    // báo cáo: check kiểu `fuel` còn lại (không chứng minh được) + ghi chú N0905
    let dir = scratch("fuel");
    let f = dir.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let o = gritc()
        .args(["check", "--json"])
        .arg(&f)
        .output()
        .expect("chạy");
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(
        text.contains("\"kind\":\"fuel\"") && text.contains("N0905"),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
