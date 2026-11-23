//! C3 bước 1 Giai đoạn 2 (ADR-P2-01, duyệt 2026-11-23): khoá `edition` ở `[package]` của `rig.toml`.
//! Luật: vắng ⇒ edition 1; `edition = 1` ⇒ như vắng (không đổi cú pháp/ngữ nghĩa edition 1); mọi giá trị khác (số chưa hỗ trợ, chuỗi, thập phân, âm, số 0 đầu,
//! thừa ký tự) ⇒ TỪ CHỐI mã 11 (fail-closed: không đoán nghĩa của edition lạ); áp cho CẢ gói phụ thuộc; khoá lạ khác vẫn bị từ chối như trước.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn tmp(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("rig-edition-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().expect("cha")).expect("tạo thư mục");
    std::fs::write(p, text).expect("ghi");
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

fn project(dir: &Path, edition_line: &str) {
    write(
        dir,
        "rig.toml",
        &format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n{edition_line}\n[dependencies]\n"
        ),
    );
    write(dir, "src/main.gt", "ƒ main() → () ! ∅ { }\n");
}

#[test]
fn edition_one_and_a_missing_key_behave_identically() {
    for (label, line) in [("vắng", ""), ("edition = 1", "edition = 1\n")] {
        let d = tmp("ok");
        project(&d, line);
        let l = rig(&d, &["lock"]);
        assert_eq!(code(&l), 0, "{label}: rig lock: {}", err(&l));
        let c = rig(&d, &["check", "--locked"]);
        assert_eq!(code(&c), 0, "{label}: rig check: {}", err(&c));
    }
}

#[test]
fn any_edition_value_other_than_a_supported_integer_is_refused_with_code_11() {
    let bad = [
        ("edition = 2\n", "edition 2"),
        ("edition = 0\n", "edition 0"),
        ("edition = 99\n", "edition 99"),
        ("edition = \"1\"\n", "số nguyên"),
        ("edition = 1.0\n", "số nguyên"),
        ("edition = -1\n", "số nguyên"),
        ("edition = 01\n", "số nguyên"),
        ("edition = 1x\n", "số nguyên"),
        ("edition = \n", "giá trị"),
    ];
    for (line, expect) in bad {
        let d = tmp("bad");
        project(&d, line);
        let o = rig(&d, &["lock"]);
        assert_eq!(
            code(&o),
            11,
            "`{}` phải bị từ chối mã 11: {}",
            line.trim(),
            err(&o)
        );
        assert!(
            err(&o).contains(expect),
            "`{}`: thông báo phải nêu `{expect}`: {}",
            line.trim(),
            err(&o)
        );
        assert!(
            !d.join("rig.lock").exists(),
            "`{}`: bị từ chối thì không được ghi rig.lock",
            line.trim()
        );
    }
}

#[test]
fn an_unsupported_edition_in_a_dependency_package_is_refused_too() {
    let root = tmp("dep");
    let app = root.join("app");
    let lib = root.join("lib");
    write(
        &app,
        "rig.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nlib = { path = \"../lib\" }\n",
    );
    write(&app, "src/main.gt", "ƒ main() → () ! ∅ { }\n");
    write(
        &lib,
        "rig.toml",
        "[package]\nname = \"lib\"\nversion = \"0.1.0\"\nedition = 2\n\n[dependencies]\n",
    );
    write(&lib, "src/lib.gt", "◈ ƒ one() → i32 ! ∅ { 1 }\n");
    let o = rig(&app, &["lock"]);
    assert_eq!(code(&o), 11, "{}", err(&o));
    assert!(err(&o).contains("edition 2"), "{}", err(&o));
}

#[test]
fn other_unknown_package_keys_are_still_refused() {
    let d = tmp("other");
    project(&d, "edtion = 1\n");
    let o = rig(&d, &["lock"]);
    assert_eq!(code(&o), 11, "{}", err(&o));
    assert!(err(&o).contains("edtion"), "{}", err(&o));
}
