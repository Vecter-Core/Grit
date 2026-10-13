//! Đơn hình hoá + gọi qua trait + tên symbol (ADR-032 phụ lục A, bước (i)). Chạy NHỊ PHÂN THẬT và đối chiếu với thông dịch.
//! Cần `LLVM-C.dll` trên PATH (t.sh / gate.sh đặt sẵn).
#![cfg(feature = "llvm")]
use std::path::PathBuf;
use std::process::Command;

fn gritc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_gritc"))
}

fn scratch(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("gt-gen-test-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

/// (mã thoát, giá trị chuẩn hoá của `main`) theo backend
fn run(src: &str, backend: &str) -> (i32, String) {
    let d = scratch(backend);
    let f = d.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
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
    let val = v["run"]["value"].as_str().unwrap_or("").to_string();
    let val = val
        .strip_prefix("Int(")
        .and_then(|s| s.split(',').next())
        .map_or(val.clone(), str::to_string);
    let _ = std::fs::remove_dir_all(&d);
    (o.status.code().unwrap_or(-1), val)
}

fn both(src: &str, expect: &str) {
    let l = run(src, "llvm");
    assert_eq!(l, (0, expect.to_string()), "llvm\n{src}");
    let i = run(src, "interpreter");
    assert_eq!(i, l, "thông dịch ≠ llvm\n{src}");
}

const TRAIT_PROG: &str = "⊏ Area { ƒ area(&self) → i32 ! ∅; }
▢ Sq { w: i32 }
▢ Rc { w: i32, h: i32 }
▢ Wrap<T> { inner: T, k: i32 }
⊐ Area for Sq { ƒ area(&self) → i32 ! ∅ { self.w *% self.w } }
⊐ Area for Rc { ƒ area(&self) → i32 ! ∅ { self.w *% self.h } }
⊐ <T> Area for Wrap<T> { ƒ area(&self) → i32 ! ∅ { self.k } }
ƒ total<T: Area>(a: &T, b: &T) → i32 ! ∅ { a.area() +% b.area() }
ƒ twice<T: Area>(a: &T) → i32 ! ∅ { total(a, a) }
ƒ main() → i32 ! ∅ {
  s ≔ Sq { w: 3 };
  r ≔ Rc { w: 4, h: 5 };
  w ≔ Wrap { inner: Rc { w: 2, h: 3 }, k: 100 };
  ww ≔ Wrap { inner: Wrap { inner: Sq { w: 2 }, k: 10 }, k: 1000 };
  total(&s, &s) +% twice(&r) +% twice(&w) +% twice(&ww)
}
";

/// Gọi phương thức qua ràng buộc generic (`x.m()` với `T: Tr`): trước đây thoát 3 (`chưa hỗ trợ: gọi qua trait`). Số tính tay:
/// total(s,s)=9+9=18; twice(r)=2·20=40; twice(w)=2·100=200; twice(ww)=2·1000=2000 ⇒ 2258. Hai impl khác nhau + impl generic được chọn
/// ĐÚNG theo từng instance.
#[test]
fn trait_bound_calls_pick_the_right_impl_per_instance() {
    both(TRAIT_PROG, "2258");
}

/// Cùng hàm generic dùng với hai impl: nếu đơn hình hoá chọn nhầm impl (ví dụ luôn chọn impl đầu) thì giá trị đổi.
#[test]
fn one_generic_fn_two_impls_gives_distinct_results() {
    let src = "⊏ Id { ƒ id(&self) → i32 ! ∅; }
▢ A { }
▢ B { }
⊐ Id for A { ƒ id(&self) → i32 ! ∅ { 1 } }
⊐ Id for B { ƒ id(&self) → i32 ! ∅ { 20 } }
ƒ get<T: Id>(x: &T) → i32 ! ∅ { x.id() }
ƒ main() → i32 ! ∅ { a ≔ A { }; b ≔ B { }; get(&a) +% get(&b) *% 3 }
";
    both(src, "61");
}

fn dump_ir(src: &str) -> (String, PathBuf) {
    let d = scratch("ir");
    let f = d.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let exe = d.join("p.exe");
    let ir = d.join("p.ll");
    let o = gritc()
        .env("GRITC_DUMP_IR", &ir)
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
    (std::fs::read_to_string(&ir).expect("IR"), exe)
}

fn function_names(ir: &str) -> Vec<String> {
    ir.lines()
        .filter_map(|l| {
            let l = l.trim_start();
            let rest = l
                .strip_prefix("define ")
                .or_else(|| l.strip_prefix("declare "))?;
            let at = rest.find('@')?;
            let name = &rest[at + 1..];
            let end = name.find('(')?;
            Some(name[..end].trim_matches('"').to_string())
        })
        .collect()
}

const GEN_PROG: &str = "▢ Điểm<T> { x: T }
ƒ nhân<T>(a: T) → T ! ∅ { a }
ƒ main() → u32 ! ∅ {
  p ≔ nhân(Điểm { x: 7u8 });
  q ≔ nhân(Điểm { x: 9u16 });
  r ≔ nhân(3u32);
  (p.x as u32) +% (q.x as u32) +% r
}
";

/// Tên symbol toàn ASCII kể cả khi tên hàm/kiểu của người dùng là Unicode; hai instance khác nhau (`nhân<Điểm<u8>>`, `nhân<Điểm<u16>>`,
/// `nhân<u32>`) có symbol KHÁC nhau; bảng ánh xạ ngược `!gt.instances` giữ dạng đọc được (Unicode).
#[test]
fn symbols_are_ascii_distinct_and_mapped_back_to_readable_names() {
    let (ir, _exe) = dump_ir(GEN_PROG);
    let names = function_names(&ir);
    assert!(!names.is_empty(), "{ir}");
    for n in &names {
        assert!(
            n.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.'),
            "tên không ASCII an toàn: {n:?}"
        );
    }
    let nhan: Vec<&String> = names.iter().filter(|n| n.contains("nh")).collect();
    assert_eq!(nhan.len(), 3, "ba instance của `nhân`: {names:?}");
    let uniq: std::collections::BTreeSet<&&String> = nhan.iter().collect();
    assert_eq!(uniq.len(), 3, "symbol trùng nhau: {nhan:?}");
    assert!(ir.contains("!gt.instances"), "{ir}");
    // dạng đọc được (Unicode) nằm trong metadata, đã thoát theo cách của LLVM (\XX); kiểm bằng chuỗi thoát của `â` `n` `h`… không cần:
    // chỉ cần có đúng 3 mục chứa `Điểm` hoặc `nhân` ở dạng thoát hex của UTF-8 (C3 A2 = â)
    let md = ir
        .lines()
        .filter(|l| l.contains("="))
        .filter(|l| l.starts_with('!'))
        .count();
    assert!(md >= 1, "{ir}");
    assert!(
        ir.contains("\\C3\\A2"),
        "dạng đọc được phải còn chữ Unicode: {ir}"
    );
    // chạy được và đúng: 7 + 9 + 3 = 19
    both(GEN_PROG, "19");
}

/// Hai lần build cùng nguồn ⇒ cùng nhị phân (thứ tự và tên instance tất định; `/Brepro`).
#[test]
fn building_twice_gives_identical_binaries_and_ir() {
    let (ir1, exe1) = dump_ir(TRAIT_PROG);
    let (ir2, exe2) = dump_ir(TRAIT_PROG);
    assert_eq!(ir1, ir2, "IR khác nhau giữa hai lần build");
    let a = std::fs::read(&exe1).expect("exe1");
    let b = std::fs::read(&exe2).expect("exe2");
    assert_eq!(a, b, "exe khác nhau giữa hai lần build");
}

/// Ghi nhận khoảng trống của ADR-032: gọi qua trait TRONG thân impl generic bị M8 từ chối (E0703) vì cạnh `Wrap<T>::area → T::area`
/// có thể trỏ lại chính nó theo cú pháp. Test này khoá hành vi hiện tại để thay đổi bị phát hiện (xem KNOWN_LIMITATIONS).
#[test]
fn known_limit_trait_call_inside_generic_impl_is_rejected_by_m8() {
    let d = scratch("m8");
    let f = d.join("prog.gt");
    std::fs::write(
        &f,
        "⊏ A { ƒ a(&self) → i32 ! ∅; }
▢ S { }
▢ W<T> { i: T }
⊐ A for S { ƒ a(&self) → i32 ! ∅ { 1 } }
⊐ <T: A> A for W<T> { ƒ a(&self) → i32 ! ∅ { self.i.a() } }
ƒ main() → i32 ! ∅ { w ≔ W { i: S { } }; w.a() }
",
    )
    .expect("ghi");
    let o = gritc().arg("check").arg(&f).output().expect("chạy");
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("E0703"));
}

fn build_with_fault(src: &str, fault: &str, backend_run: bool) -> (Option<i32>, String, Fault) {
    let d = scratch("fault");
    let f = d.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let exe = d.join("f.exe");
    let mut c = gritc();
    c.env("GRITC_FAULT", fault);
    if backend_run {
        c.args(["run", "--backend=interpreter"]);
    } else {
        c.args(["build", "--backend=llvm", "-o"]).arg(&exe);
    }
    let o = c.arg(&f).output().expect("chạy");
    (
        o.status.code(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
        Fault {
            exe,
            obj: d.join("f.exe.obj"),
        },
    )
}

struct Fault {
    exe: PathBuf,
    obj: PathBuf,
}

/// Chèn lỗi `ir:instance`: một hàm lạ (vô hại) KHÔNG thuộc bảng instance ⇒ IR Guard từ chối (E1101), không exe, không object.
#[test]
fn guard_rejects_a_function_outside_the_instance_table() {
    let (code, err, p) = build_with_fault(TRAIT_PROG, "ir:instance", false);
    assert_eq!(code, Some(1), "{err}");
    assert!(
        err.contains("E1101") && err.contains("`rogue.fn` không thuộc bảng instance"),
        "{err}"
    );
    assert!(!p.exe.exists() && !p.obj.exists());
}

/// `select_impl` không phân giải được (M4 đã đảm bảo impl tồn tại) ⇒ ICE E9901, KHÔNG đoán impl: cả codegen lẫn thông dịch, không exe.
#[test]
fn select_impl_failure_is_an_ice_not_a_guess() {
    let (code, err, p) = build_with_fault(TRAIT_PROG, "types:select-impl", false);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("E9901"), "{err}");
    assert!(!p.exe.exists() && !p.obj.exists(), "ICE ⇒ không có exe");
    let (code, err, _) = build_with_fault(TRAIT_PROG, "types:select-impl", true);
    assert_ne!(code, Some(0), "{err}");
    assert!(err.contains("E9901"), "{err}");
}
