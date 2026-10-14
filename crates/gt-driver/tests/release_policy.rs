//! Chính sách chế độ (lệnh Giang 2026-10-14, mục 1 và 5): `--checks=proven` và `--profile=release*` là THỬ NGHIỆM (ghi trong help, in ghi chú N0606),
//! KHÔNG là mặc định, KHÔNG dùng được cùng nhau; hồ sơ release/release-o2 tái lập (SHA-256 exe qua hai lần build, build ở thư mục khác và sau "clean").
use gt_driver::sha256::sha256_hex;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn gritc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_gritc"))
}

fn scratch(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("gt-pol-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo");
    d
}

const PROG: &str = "ƒ f(n: u64) → u64 ! ∅ { μs ≔ 0u64; μi ≔ 0u64; ⟳ i < n ↓ n - i { s ← s + i; i ← i + 1; } s }\nƒ main() → u64 ! ∅ { f(100u64) }\n";

fn prog(dir: &Path) -> PathBuf {
    let f = dir.join("prog.gt");
    std::fs::write(&f, PROG).expect("ghi");
    f
}

fn out_text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// (1) help: `--checks=proven` ghi rõ THỬ NGHIỆM; `release*` ghi "bằng chứng thực nghiệm" (đã đạt điều kiện Q82, ADR-036 §9) và mặc định vẫn là dev
#[test]
fn help_marks_proven_experimental_and_release_as_evidence_backed() {
    let o = gritc().arg("--help").output().expect("chạy");
    let t = out_text(&o);
    assert!(
        t.contains("proven là THỬ NGHIỆM"),
        "help thiếu nhãn thử nghiệm cho --checks=proven: {t}"
    );
    assert!(
        t.contains("mặc định: dev; release và release-o2 đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh"),
        "help thiếu mô tả release (mặc định dev + bằng chứng thực nghiệm): {t}"
    );
}

/// (2) mặc định: `--checks=all`, hồ sơ dev; KHÔNG có ghi chú N0606; không có băm IR (không chạy bộ tối ưu)
#[test]
fn defaults_are_all_checks_and_dev_profile_without_the_experimental_note() {
    let dir = scratch("def");
    let f = prog(&dir);
    let exe = dir.join("p.exe");
    let o = gritc()
        .args(["build", "--json", "-o"])
        .arg(&exe)
        .arg(&f)
        .output()
        .expect("chạy");
    assert_eq!(o.status.code(), Some(0), "{}", out_text(&o));
    let t = out_text(&o);
    assert!(!t.contains("N0606"), "mặc định không được có N0606: {t}");
    assert!(
        !t.contains("băm IR"),
        "mặc định (dev) không được chạy bộ tối ưu: {t}"
    );
    let line = String::from_utf8_lossy(&o.stdout)
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .unwrap_or("")
        .to_string();
    let v: serde_json::Value = serde_json::from_str(&line).expect("JSON");
    assert_eq!(v["checks"]["mode"], "all", "{line}");
    // `--checks=all` tường minh cũng không có N0606
    let o2 = gritc()
        .args(["check", "--checks=all"])
        .arg(&f)
        .output()
        .expect("chạy");
    assert!(!out_text(&o2).contains("N0606"));
}

/// (3) `--checks=proven` in ghi chú N0606 MỖI lần chạy (check, build, run) và ghi chế độ "proven" trong báo cáo
#[test]
fn proven_mode_prints_the_note_every_time() {
    let dir = scratch("proven");
    let f = prog(&dir);
    for round in 0..2 {
        let o = gritc()
            .args(["check", "--checks=proven"])
            .arg(&f)
            .output()
            .expect("chạy");
        assert!(
            out_text(&o).contains("N0606"),
            "check lần {round}: {}",
            out_text(&o)
        );
    }
    let exe = dir.join("p.exe");
    let o = gritc()
        .args(["build", "--checks=proven", "-o"])
        .arg(&exe)
        .arg(&f)
        .output()
        .expect("chạy");
    assert_eq!(o.status.code(), Some(0), "{}", out_text(&o));
    assert!(out_text(&o).contains("N0606"), "build: {}", out_text(&o));
    let o = gritc()
        .args(["run", "--json", "--checks=proven"])
        .arg(&f)
        .output()
        .expect("chạy");
    assert!(out_text(&o).contains("N0606"), "run: {}", out_text(&o));
    let line = String::from_utf8_lossy(&o.stdout)
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .unwrap_or("")
        .to_string();
    let v: serde_json::Value = serde_json::from_str(&line).expect("JSON");
    assert_eq!(v["checks"]["mode"], "proven");
}

/// (4) hồ sơ release/release-o2 KHÔNG dùng được `--checks=proven` (mã 3, không sinh exe); dev + proven vẫn dùng được
#[test]
fn release_profiles_refuse_proven_mode() {
    let dir = scratch("refuse");
    let f = prog(&dir);
    for prof in ["release", "release-o2"] {
        for cmd in ["build", "run"] {
            let exe = dir.join(format!("{prof}-{cmd}.exe"));
            let mut c = gritc();
            c.args([cmd, "--checks=proven", &format!("--profile={prof}")]);
            if cmd == "build" {
                c.arg("-o").arg(&exe);
            }
            let o = c.arg(&f).output().expect("chạy");
            assert_eq!(o.status.code(), Some(3), "{prof} {cmd}: {}", out_text(&o));
            assert!(
                out_text(&o).contains("không được dùng cùng"),
                "{prof} {cmd}: {}",
                out_text(&o)
            );
            assert!(!exe.exists(), "{prof} {cmd}: không được có exe");
        }
    }
    let o = gritc()
        .args(["run", "--checks=proven", "--profile=dev"])
        .arg(&f)
        .output()
        .expect("chạy");
    assert_eq!(o.status.code(), Some(0), "{}", out_text(&o));
}

fn sha(p: &Path) -> String {
    sha256_hex(&std::fs::read(p).expect("đọc exe"))
}

fn build_to(profile: &str, src: &Path, out: &Path) {
    let o = gritc()
        .args(["build", &format!("--profile={profile}"), "-o"])
        .arg(out)
        .arg(src)
        .output()
        .expect("chạy");
    assert_eq!(o.status.code(), Some(0), "{profile}: {}", out_text(&o));
}

/// tái lập: SHA-256 exe giống nhau qua (a) hai lần build cùng chỗ, (b) build ở thư mục KHÁC (đường dẫn khác), (c) sau khi XOÁ sạch thư mục dựng ("clean"),
/// cho dev, release và release-o2; một chương trình đơn và một dự án nhiều tệp (examples/sortlines)
#[test]
fn builds_are_reproducible_for_every_profile() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let multi = root.join("examples/sortlines/src");
    for profile in ["dev", "release", "release-o2"] {
        let d1 = scratch("rep1");
        let single = prog(&d1);
        for (label, src) in [("đơn", single.clone()), ("nhiều tệp", multi.clone())] {
            let a = d1.join("a.exe");
            build_to(profile, &src, &a);
            let h1 = sha(&a);
            build_to(profile, &src, &a);
            assert_eq!(
                h1,
                sha(&a),
                "{profile}/{label}: hai lần build cùng chỗ khác băm"
            );
            // thư mục khác
            let d2 = scratch("rep2");
            let b = d2.join("sub").join("b.exe");
            std::fs::create_dir_all(b.parent().expect("cha")).expect("tạo");
            build_to(profile, &src, &b);
            assert_eq!(h1, sha(&b), "{profile}/{label}: thư mục khác ⇒ băm khác");
            // "clean": xoá sạch rồi dựng lại
            std::fs::remove_dir_all(&d2).expect("xoá");
            std::fs::create_dir_all(&d2).expect("tạo lại");
            let c = d2.join("c.exe");
            build_to(profile, &src, &c);
            assert_eq!(h1, sha(&c), "{profile}/{label}: sau clean ⇒ băm khác");
            let _ = std::fs::remove_dir_all(&d2);
        }
        let _ = std::fs::remove_dir_all(&d1);
    }
}
