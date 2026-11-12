//! Rig v0 (M11) ở mức tiến trình. Cần `LLVM-C.dll` trên PATH và `gt_rt.lib` đã dựng (xem docs/ENVIRONMENT.md) cho các test dựng exe.
use gt_driver::sha256::sha256_hex;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn tmp(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("rig-test-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn rig(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rig"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("chạy rig")
}

fn code(o: &Output) -> i32 {
    o.status.code().unwrap_or(-1)
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().expect("cha")).expect("tạo thư mục");
    std::fs::write(p, text).expect("ghi");
}

fn manifest(name: &str, deps: &[(&str, &str)]) -> String {
    let mut s = format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n\n[dependencies]\n");
    for (n, p) in deps {
        s.push_str(&format!("{n} = {{ path = \"{p}\" }}\n"));
    }
    s
}

/// 3 gói: base ← mid ← app, và app cũng phụ thuộc thẳng base (hình thoi). Trả thư mục gốc chứa app/, mid/, base/.
fn three_packages(root: &Path, test_expect: u32) {
    write(&root.join("base"), "rig.toml", &manifest("base", &[]));
    write(
        &root.join("base"),
        "src/base.gt",
        "◈ ƒ add(a: u32, b: u32) → u32 ! ∅ { a +% b }\n",
    );
    write(
        &root.join("mid"),
        "rig.toml",
        &manifest("mid", &[("base", "../base")]),
    );
    write(
        &root.join("mid"),
        "src/mid.gt",
        "◈ ƒ add3(a: u32) → u32 ! ∅ { base::add(a, 3u32) }\n",
    );
    write(
        &root.join("app"),
        "rig.toml",
        &manifest("app", &[("base", "../base"), ("mid", "../mid")]),
    );
    write(
        &root.join("app"),
        "src/main.gt",
        "ƒ main() → u32 ! ∅ { mid::add3(base::add(1u32, 1u32)) }\n",
    );
    write(
        &root.join("app"),
        "tests/t_check.gt",
        &format!(
            "ƒ zero() → u32 ! ∅ {{ 0 }}\nƒ bad() → u32 ! ∅ {{ 1u32 / zero() }}\nƒ main() ! ∅ {{ ⎇ mid::add3(base::add(1u32, 1u32)) == {test_expect}u32 {{ }} ⇝ {{ bad(); }} }}\n"
        ),
    );
}

fn sha_of(p: &Path) -> String {
    sha256_hex(&std::fs::read(p).expect("đọc exe"))
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("tạo");
    for e in std::fs::read_dir(from).expect("đọc").flatten() {
        let p = e.path();
        let t = to.join(e.file_name());
        if p.is_dir() {
            if e.file_name() == "target" {
                continue;
            }
            copy_dir(&p, &t);
        } else {
            std::fs::copy(&p, &t).expect("chép");
        }
    }
}

#[test]
fn new_check_build_run_roundtrip() {
    let d = tmp("new");
    let o = rig(&d, &["new", "hello"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    let p = d.join("hello");
    assert!(
        p.join("rig.toml").is_file()
            && p.join("src/main.gt").is_file()
            && p.join("rig.lock").is_file()
    );
    assert_eq!(code(&rig(&p, &["check"])), 0);
    assert_eq!(
        code(&rig(&p, &["fmt", "--check"])),
        0,
        "gói mới phải đúng định dạng chuẩn"
    );
    let o = rig(&p, &["build"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(p.join("target/hello.exe").is_file());
    assert_eq!(code(&rig(&p, &["run"])), 0);
    // tên gói không hợp lệ / thư mục đã có
    assert_eq!(code(&rig(&d, &["new", "Bad-Name"])), 11);
    assert_ne!(code(&rig(&d, &["new", "hello"])), 0);
    // clean
    assert_eq!(code(&rig(&p, &["clean"])), 0);
    assert!(!p.join("target").exists());
}

#[test]
fn three_package_project_builds_runs_and_tests_with_diamond_loaded_once() {
    let d = tmp("three");
    three_packages(&d, 5);
    let app = d.join("app");
    let o = rig(&app, &["lock"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    let lock = std::fs::read_to_string(app.join("rig.lock")).expect("lock");
    // hình thoi: base xuất hiện MỘT lần dù đi qua hai đường
    assert_eq!(lock.matches("[[package]]").count(), 3, "{lock}");
    assert_eq!(lock.matches("name = \"base\"").count(), 1, "{lock}");
    for l in lock.lines().filter(|l| l.starts_with("hash = ")) {
        let h = l
            .trim_start_matches("hash = \"sha256:")
            .trim_end_matches('"');
        assert!(
            h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()),
            "{l}"
        );
    }
    let o = rig(&app, &["build", "--locked"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert_eq!(code(&rig(&app, &["run"])), 0);
    let o = rig(&app, &["test"]);
    assert_eq!(code(&o), 0, "{}{}", out(&o), err(&o));
    assert!(
        out(&o).contains("test t_check ... ok") && out(&o).contains("1 qua, 0 hỏng"),
        "{}",
        out(&o)
    );
}

#[test]
fn failing_test_exits_1_and_names_the_abort() {
    let d = tmp("failtest");
    three_packages(&d, 6); // 1+1+3 = 5 ≠ 6 ⇒ chia cho 0 ⇒ abort
    let app = d.join("app");
    let o = rig(&app, &["test"]);
    assert_eq!(code(&o), 1, "{}{}", out(&o), err(&o));
    assert!(
        out(&o).contains("FAILED (abort: div_by_zero)"),
        "{}",
        out(&o)
    );
}

/// TIÊU CHÍ CỨNG của M11: hai lần build ⇒ exe GIỐNG HỆT (SHA-256); thử cả sau `rig clean` và khi dự án nằm ở thư mục khác.
#[test]
fn builds_are_reproducible_byte_for_byte() {
    let d = tmp("repro");
    three_packages(&d, 5);
    let app = d.join("app");
    assert_eq!(code(&rig(&app, &["build"])), 0);
    let exe = app.join("target/app.exe");
    let h1 = sha_of(&exe);
    assert_eq!(code(&rig(&app, &["build"])), 0);
    assert_eq!(sha_of(&exe), h1, "dựng lại ngay: khác nhau");
    assert_eq!(code(&rig(&app, &["clean"])), 0);
    assert_eq!(code(&rig(&app, &["build"])), 0);
    assert_eq!(sha_of(&exe), h1, "sau rig clean: khác nhau");
    // cùng nguồn ở đường dẫn khác ⇒ cùng nhị phân (không rò đường dẫn/thời gian)
    let d2 = tmp("repro2");
    for n in ["app", "mid", "base"] {
        copy_dir(&d.join(n), &d2.join(n));
    }
    std::thread::sleep(std::time::Duration::from_millis(1100)); // khác dấu thời gian tệp/đồng hồ
    assert_eq!(code(&rig(&d2.join("app"), &["build"])), 0);
    assert_eq!(
        sha_of(&d2.join("app/target/app.exe")),
        h1,
        "thư mục khác: khác nhau"
    );
}

#[test]
fn module_name_collisions_and_prefix_rule_are_loud_errors() {
    // luật tiền tố: tệp của gói phụ thuộc phải là <gói>.gt hoặc <gói>_….gt
    let d = tmp("prefix");
    write(&d.join("lib"), "rig.toml", &manifest("lib", &[]));
    write(&d.join("lib"), "src/helper.gt", "◈ ƒ f() → i32 ! ∅ { 1 }\n");
    write(
        &d.join("app"),
        "rig.toml",
        &manifest("app", &[("lib", "../lib")]),
    );
    write(&d.join("app"), "src/main.gt", "ƒ main() → i32 ! ∅ { 0 }\n");
    let o = rig(&d.join("app"), &["check"]);
    assert_eq!(code(&o), 11, "{}", err(&o));
    assert!(
        err(&o).contains("tiền tố tên gói") && err(&o).contains("helper.gt"),
        "{}",
        err(&o)
    );
    // hợp lệ: lib_util.gt và lib.gt
    std::fs::remove_file(d.join("lib/src/helper.gt")).expect("xoá");
    write(
        &d.join("lib"),
        "src/lib_util.gt",
        "◈ ƒ f() → i32 ! ∅ { 1 }\n",
    );
    assert_eq!(code(&rig(&d.join("app"), &["check"])), 0);
    // va chạm: gói gốc có tệp trùng tên module của gói phụ thuộc ⇒ lỗi rõ, KHÔNG im lặng chọn một
    write(
        &d.join("app"),
        "src/lib_util.gt",
        "◈ ƒ g() → i32 ! ∅ { 2 }\n",
    );
    let o = rig(&d.join("app"), &["check"]);
    assert_eq!(code(&o), 11, "{}", err(&o));
    assert!(
        err(&o).contains("module `lib_util` có ở cả gói `lib` và gói `app`"),
        "{}",
        err(&o)
    );
}

#[test]
fn dependency_cycles_and_same_name_two_paths_are_errors() {
    let d = tmp("graph");
    // vòng a → b → a
    write(&d.join("a"), "rig.toml", &manifest("a", &[("b", "../b")]));
    write(&d.join("a"), "src/main.gt", "ƒ main() → i32 ! ∅ { 0 }\n");
    write(&d.join("b"), "rig.toml", &manifest("b", &[("a", "../a")]));
    write(&d.join("b"), "src/b.gt", "◈ ƒ f() → i32 ! ∅ { 1 }\n");
    let o = rig(&d.join("a"), &["check"]);
    assert_eq!(code(&o), 11, "{}", err(&o));
    assert!(err(&o).contains("phụ thuộc vòng"), "{}", err(&o));
    // cùng tên `x` trỏ tới hai thư mục khác nhau (một trực tiếp, một qua `m`)
    let d = tmp("graph2");
    for dir in ["x1", "x2"] {
        write(&d.join(dir), "rig.toml", &manifest("x", &[]));
        write(&d.join(dir), "src/x.gt", "◈ ƒ f() → i32 ! ∅ { 1 }\n");
    }
    write(&d.join("m"), "rig.toml", &manifest("m", &[("x", "../x2")]));
    write(&d.join("m"), "src/m.gt", "◈ ƒ g() → i32 ! ∅ { 2 }\n");
    write(
        &d.join("app"),
        "rig.toml",
        &manifest("app", &[("x", "../x1"), ("m", "../m")]),
    );
    write(&d.join("app"), "src/main.gt", "ƒ main() → i32 ! ∅ { 0 }\n");
    let o = rig(&d.join("app"), &["check"]);
    assert_eq!(code(&o), 11, "{}", err(&o));
    assert!(
        err(&o).contains("hai gói khác nhau cùng tên `x`"),
        "{}",
        err(&o)
    );
    // tên khai ≠ tên thật của gói
    let d = tmp("graph3");
    write(&d.join("real"), "rig.toml", &manifest("real", &[]));
    write(&d.join("real"), "src/real.gt", "◈ ƒ f() → i32 ! ∅ { 1 }\n");
    write(
        &d.join("app"),
        "rig.toml",
        &manifest("app", &[("other", "../real")]),
    );
    write(&d.join("app"), "src/main.gt", "ƒ main() → i32 ! ∅ { 0 }\n");
    let o = rig(&d.join("app"), &["check"]);
    assert_eq!(code(&o), 11, "{}", err(&o));
    assert!(
        err(&o).contains("được khai là phụ thuộc `other`"),
        "{}",
        err(&o)
    );
}

#[test]
fn lock_missing_is_created_stale_is_always_refused_and_locked_flag_is_strict() {
    let d = tmp("lock");
    three_packages(&d, 5);
    let app = d.join("app");
    // thiếu lock + --locked ⇒ 10, và KHÔNG tạo lock
    let o = rig(&app, &["check", "--locked"]);
    assert_eq!(code(&o), 10, "{}", err(&o));
    assert!(!app.join("rig.lock").exists());
    // thiếu lock, không --locked ⇒ tạo
    let o = rig(&app, &["check"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(app.join("rig.lock").is_file() && err(&o).contains("đã tạo"));
    // sửa nội dung gói phụ thuộc ⇒ lệch ⇒ từ chối (có và không --locked), mã 10, nêu gói nào
    write(
        &d.join("base"),
        "src/base.gt",
        "◈ ƒ add(a: u32, b: u32) → u32 ! ∅ { a +% b +% 0u32 }\n",
    );
    for args in [
        &["check"][..],
        &["build"],
        &["check", "--locked"],
        &["run"],
        &["test"],
    ] {
        let o = rig(&app, args);
        assert_eq!(code(&o), 10, "{args:?}: {}", err(&o));
        assert!(
            err(&o).contains("gói `base` đã đổi nội dung"),
            "{args:?}: {}",
            err(&o)
        );
    }
    assert!(
        !app.join("target/app.exe").exists(),
        "lock lệch ⇒ không dựng"
    );
    // khoá lại có chủ đích
    assert_eq!(code(&rig(&app, &["lock"])), 0);
    assert_eq!(code(&rig(&app, &["check", "--locked"])), 0);
    // sửa tay rig.lock ⇒ lệch
    let l = std::fs::read_to_string(app.join("rig.lock")).expect("lock");
    let l2 = l.replacen("sha256:", "sha256:0", 1);
    std::fs::write(app.join("rig.lock"), l2).expect("ghi");
    assert_eq!(code(&rig(&app, &["check"])), 10);
}

/// Mã thoát ở HAI PHÍA: của gritc chuyển nguyên, của rig nằm vùng riêng 10–13 (không lẫn mã 3 của gritc).
#[test]
fn exit_codes_gritc_passthrough_and_rig_own_range() {
    let d = tmp("codes");
    let o = rig(&d, &["new", "p"]);
    assert_eq!(code(&o), 0);
    let p = d.join("p");
    // gritc ĐỎ ⇒ 1 (chuyển nguyên), KHÔNG exe
    write(&p, "src/main.gt", "ƒ main() → u32 ! ∅ { x }\n");
    assert_eq!(code(&rig(&p, &["lock"])), 0);
    let o = rig(&p, &["build"]);
    assert_eq!(code(&o), 1, "{}", err(&o));
    assert!(!p.join("target/p.exe").exists(), "ĐỎ ⇒ KHÔNG exe");
    assert_eq!(code(&rig(&p, &["check"])), 1);
    assert_eq!(code(&rig(&p, &["run"])), 1);
    // gritc báo đầu vào sai (tên module không hợp lệ) ⇒ mã 3 của gritc chuyển nguyên
    write(&p, "src/main.gt", "ƒ main() → i32 ! ∅ { 0 }\n");
    write(&p, "src/1bad.gt", "◈ ƒ f() → i32 ! ∅ { 1 }\n");
    assert_eq!(code(&rig(&p, &["lock"])), 0);
    let o = rig(&p, &["check"]);
    assert_eq!(code(&o), 3, "{}", err(&o));
    std::fs::remove_file(p.join("src/1bad.gt")).expect("xoá");
    assert_eq!(code(&rig(&p, &["lock"])), 0);
    // thiếu main ⇒ 11 (lỗi dự án của rig, không phải ICE của gritc)
    write(&p, "src/main.gt", "ƒ helper() → i32 ! ∅ { 0 }\n");
    assert_eq!(code(&rig(&p, &["lock"])), 0);
    let o = rig(&p, &["build"]);
    assert_eq!(code(&o), 11, "{}", err(&o));
    // vùng rig riêng: usage 12, lệnh lạ 12, lock lệch 10, manifest sai 11
    assert_eq!(code(&rig(&p, &["--zzz"])), 12);
    assert_eq!(code(&rig(&p, &["unknown"])), 12);
    assert_eq!(code(&rig(&p, &[])), 12);
    write(
        &p,
        "rig.toml",
        "[package]\nname = \"p\"\nversion = \"0.1.0\"\nauthor = \"x\"\n",
    );
    assert_eq!(code(&rig(&p, &["check"])), 11);
}

#[test]
fn manifest_errors_are_fail_closed_with_clear_messages() {
    let d = tmp("toml");
    write(&d, "src/main.gt", "ƒ main() → i32 ! ∅ { 0 }\n");
    let cases: &[(&str, &str)] = &[
        (
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\nextra = \"x\"\n",
            "khoá package.`extra` không hỗ trợ",
        ),
        (
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n[features]\n",
            "mục `[features]` không hỗ trợ",
        ),
        (
            "[package]\nname = \"a\"\nname = \"b\"\nversion = \"0.1.0\"\n",
            "trùng",
        ),
        (
            "[package]\nname = \"a\"\nversion = \"1.0\"\n",
            "MAJOR.MINOR.PATCH",
        ),
        (
            "[package]\nname = \"A\"\nversion = \"0.1.0\"\n",
            "không hợp lệ",
        ),
        ("[package]\nname = \"a\"\n", "thiếu package.version"),
        ("[dependencies]\n", "thiếu mục [package]"),
        ("[package]\nname = 3\nversion = \"0.1.0\"\n", "chuỗi"),
        (
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n[dependencies]\nb = \"1.0\"\n",
            "chưa có registry",
        ),
        (
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n[dependencies]\nb = { version = \"1\" }\n",
            "chưa có registry",
        ),
        (
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n[dependencies]\nb = { }\n",
            "thiếu `path`",
        ),
        (
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n[dependencies]\nb = { path = \"x\" , path = \"y\" }\n",
            "trùng",
        ),
        ("[package]\nname = \"a\"\nversion = [\"0.1.0\"]\n", "chuỗi"),
        ("[package.sub]\nx = \"1\"\n", "không hợp lệ"),
    ];
    for (text, needle) in cases {
        write(&d, "rig.toml", text);
        let o = rig(&d, &["check"]);
        assert_eq!(code(&o), 11, "{text:?}: {}", err(&o));
        assert!(
            err(&o).contains(needle),
            "{text:?} ⇒ cần `{needle}`: {}",
            err(&o)
        );
    }
}

#[test]
fn add_edits_manifest_relocks_and_rolls_back_on_failure() {
    let d = tmp("add");
    write(&d.join("base"), "rig.toml", &manifest("base", &[]));
    write(
        &d.join("base"),
        "src/base.gt",
        "◈ ƒ one() → u32 ! ∅ { 1 }\n",
    );
    assert_eq!(code(&rig(&d, &["new", "app"])), 0);
    let app = d.join("app");
    let o = rig(&app, &["add", "../base"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    let m = std::fs::read_to_string(app.join("rig.toml")).expect("toml");
    assert!(m.contains("base = { path = \"../base\" }"), "{m}");
    assert!(
        std::fs::read_to_string(app.join("rig.lock"))
            .expect("lock")
            .contains("name = \"base\"")
    );
    write(&app, "src/main.gt", "ƒ main() → u32 ! ∅ { base::one() }\n");
    assert_eq!(code(&rig(&app, &["lock"])), 0);
    assert_eq!(code(&rig(&app, &["build", "--locked"])), 0);
    // thêm lần hai ⇒ lỗi dự án, manifest không đổi
    let before = std::fs::read_to_string(app.join("rig.toml")).expect("toml");
    assert_eq!(code(&rig(&app, &["add", "../base"])), 11);
    assert_eq!(
        std::fs::read_to_string(app.join("rig.toml")).expect("toml"),
        before
    );
    // gói trỏ tới không phải gói rig ⇒ lỗi, manifest không đổi
    std::fs::create_dir_all(d.join("notpkg")).expect("tạo");
    assert_ne!(code(&rig(&app, &["add", "../notpkg"])), 0);
    assert_eq!(
        std::fs::read_to_string(app.join("rig.toml")).expect("toml"),
        before
    );
}

#[test]
fn fmt_rewrites_and_check_flags_unformatted_sources() {
    let d = tmp("fmt");
    assert_eq!(code(&rig(&d, &["new", "f"])), 0);
    let p = d.join("f");
    write(&p, "src/main.gt", "ƒ   main()  →  i32 ! ∅ {   0   }\n");
    assert_eq!(code(&rig(&p, &["lock"])), 0);
    assert_eq!(code(&rig(&p, &["fmt", "--check"])), 1);
    assert_eq!(code(&rig(&p, &["fmt"])), 0);
    assert_eq!(code(&rig(&p, &["fmt", "--check"])), 0);
    // fmt đổi nguồn ⇒ lock lệch: bắt buộc khoá lại (không im lặng chấp nhận)
    assert_eq!(code(&rig(&p, &["check"])), 10);
    assert_eq!(code(&rig(&p, &["lock"])), 0);
    assert_eq!(code(&rig(&p, &["check"])), 0);
}

// ---------------------------------------------------------------------------------------------------------------------------------
// F.2 (lệnh Giang 2026-10-15): generic và trait CHÉO GÓI cho thư viện chuẩn viết bằng Grit.
// ---------------------------------------------------------------------------------------------------------------------------------
const STD_FWD: &str = "◈ ▢ W<T> { ◈ v: T }
⊐ Ord for u32 {
  ƒ cmp(&self, o: &Self) → Ordering ! ∅ {
    ⎇ *self < *o { Ordering::Less } ⇝ { ⎇ *self > *o { Ordering::Greater } ⇝ { Ordering::Equal } }
  }
}
⊐ <T: Ord> Ord for W<T> {
  ƒ cmp(&self, o: &Self) → Ordering ! ∅ { self.v.cmp(&o.v) }
}
";

fn std_app(root: &Path, std_src: &str, main_src: &str) {
    write(&root.join("std"), "rig.toml", &manifest("std", &[]));
    write(&root.join("std"), "src/std.gt", std_src);
    write(
        &root.join("app"),
        "rig.toml",
        &manifest("app", &[("std", "../std")]),
    );
    write(&root.join("app"), "src/main.gt", main_src);
}

fn run_exe(app: &Path) -> String {
    let o = Command::new(app.join("target/app.exe"))
        .output()
        .expect("chạy exe");
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// Mẫu bọc-và-chuyển-tiếp `⊐ <T: Ord> Ord for W<T> { cmp → self.v.cmp(&o.v) }` định nghĩa Ở GÓI KHÁC, dùng với `W<W<W<u32>>>`: M8 (E0703) CHẤP NHẬN, LLVM chạy đúng.
#[test]
fn cross_package_generic_trait_forwarder_passes_e0703_and_runs_correctly() {
    for (a, b, want) in [(5, 3, "3"), (3, 5, "1"), (4, 4, "2")] {
        let d = tmp("fwd");
        std_app(
            &d,
            STD_FWD,
            &format!(
                "ƒ main() ! {{io}} {{\n  a ≔ std::W {{ v: std::W {{ v: std::W {{ v: {a}u32 }} }} }};\n  b ≔ std::W {{ v: std::W {{ v: std::W {{ v: {b}u32 }} }} }};\n  out ≔ root_out();\n  r ≔ ⌥ a.cmp(&b) {{ Ordering::Less ↦ 1u64, Ordering::Greater ↦ 3u64, _ ↦ 2u64 }};\n  out.write_u64(r);\n}}\n"
            ),
        );
        let app = d.join("app");
        let o = rig(&app, &["build"]);
        assert_eq!(code(&o), 0, "{}", err(&o));
        assert_eq!(run_exe(&app), want, "a={a} b={b}");
    }
}

/// Vòng THẬT chéo gói vẫn bị E0703 (không nới): impl tự gọi lại chính nó qua trait.
#[test]
fn cross_package_true_cycles_are_still_e0703() {
    let main_w = "ƒ main() → u32 ! ∅ { a ≔ std::W { v: 1u32 }; b ≔ std::W { v: 2u32 }; ⌥ a.cmp(&b) { Ordering::Less ↦ 1u32, _ ↦ 0u32 } }
";
    let main_u =
        "ƒ main() → u32 ! ∅ { a ≔ 1u32; b ≔ 2u32; ⌥ a.cmp(&b) { Ordering::Less ↦ 1u32, _ ↦ 0u32 } }
";
    for (bad, main) in [
        (
            "◈ ▢ W<T> { ◈ v: T }
⊐ Ord for u32 {
  ƒ cmp(&self, o: &Self) → Ordering ! ∅ { self.cmp(o) }
}
",
            main_u,
        ),
        (
            "◈ ▢ W<T> { ◈ v: T }
⊐ Ord for u32 {
  ƒ cmp(&self, o: &Self) → Ordering ! ∅ { Ordering::Equal }
}
⊐ <T: Ord> Ord for W<T> {
  ƒ cmp(&self, o: &Self) → Ordering ! ∅ { self.cmp(o) }
}
",
            main_w,
        ),
    ] {
        let d = tmp("cyc");
        std_app(&d, bad, main);
        let o = rig(&d.join("app"), &["check"]);
        assert_ne!(code(&o), 0, "{}", err(&o));
        assert!(
            err(&o).contains("E0703") || out(&o).contains("E0703"),
            "{}{}",
            out(&o),
            err(&o)
        );
    }
}

/// ADR-040 (Q87) đã mở khoá chặn số 1 của gói `std` (F.2): `⊐ <T: Ord> Ord for Option<T>` viết được bằng khớp mẫu qua tham chiếu (`a: &T`), định nghĩa Ở GÓI KHÁC;
/// M8 (E0703) chấp nhận và LLVM chạy đúng cho mọi tổ hợp (Some/Some, Some/None, None/Some, None/None) và cả `Option<Option<u32>>` (đệ quy theo kiểu, không theo hàm).
#[test]
fn option_ord_in_another_package_uses_reference_patterns_passes_e0703_and_runs_correctly() {
    let std_src = "⊐ Ord for u32 {
  ƒ cmp(&self, o: &Self) → Ordering ! ∅ {
    ⎇ *self < *o { Ordering::Less } ⇝ { ⎇ *self > *o { Ordering::Greater } ⇝ { Ordering::Equal } }
  }
}
⊐ <T: Ord> Ord for Option<T> {
  ƒ cmp(&self, o: &Self) → Ordering ! ∅ {
    ⌥ self {
      Some(a) ↦ ⌥ o { Some(b) ↦ a.cmp(b), None ↦ Ordering::Greater },
      None ↦ ⌥ o { Some(_) ↦ Ordering::Less, None ↦ Ordering::Equal }
    }
  }
}
";
    let cases: [(&str, &str, &str); 5] = [
        ("Some(5u32)", "Some(3u32)", "3"),
        ("Some(3u32)", "Some(5u32)", "1"),
        ("Some(4u32)", "Some(4u32)", "2"),
        ("Some(9u32)", "None", "3"),
        ("None", "Some(0u32)", "1"),
    ];
    for (a, b, want) in cases {
        let d = tmp("optord");
        std_app(
            &d,
            std_src,
            &format!(
                "ƒ main() ! {{io}} {{
  a: Option<u32> ≔ {a};
  b: Option<u32> ≔ {b};
  out ≔ root_out();
  r ≔ ⌥ a.cmp(&b) {{ Ordering::Less ↦ 1u64, Ordering::Greater ↦ 3u64, _ ↦ 2u64 }};
  out.write_u64(r);
}}
"
            ),
        );
        let app = d.join("app");
        let o = rig(&app, &["build"]);
        assert_eq!(code(&o), 0, "{a} vs {b}: {}", err(&o));
        assert_eq!(run_exe(&app), want, "{a} vs {b}");
    }
}

/// Seed 1.0 G14: `rig new` thêm phụ thuộc `std` (nếu tìm thấy `lib/std` cạnh rig hoặc qua `GRIT_STD`) để chương trình dùng thật gọi được `std::…` ngay;
/// `--no-std` tắt. Gói mới có `std` phải `check` được và `rig.lock` có đủ hai gói.
#[test]
fn new_adds_the_std_dependency_g14() {
    let d = tmp("newstd");
    let o = rig(&d, &["new", "withstd"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    let toml = std::fs::read_to_string(d.join("withstd/rig.toml")).expect("toml");
    assert!(toml.contains("std = { path = "), "{toml}");
    let lock = std::fs::read_to_string(d.join("withstd/rig.lock")).expect("lock");
    assert!(lock.contains("std"), "{lock}");
    assert_eq!(code(&rig(&d.join("withstd"), &["check"])), 0);
    let o = rig(&d, &["new", "nostd", "--no-std"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    let toml = std::fs::read_to_string(d.join("nostd/rig.toml")).expect("toml");
    assert!(!toml.contains("std ="), "{toml}");
}
