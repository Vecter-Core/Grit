//! Binary `gritc` THẬT, kiểm ở mức tiến trình (ADR-028 §3): mã thoát phân vùng, "ĐỎ/ICE ⇒ không output, không chạy", "VÀNG ⇒ chạy + in cảnh báo",
//! `--deny-yellow`, `explain`, `fmt`, lỗi dùng.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn gritc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_gritc"))
}

fn scratch(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("gritc_cli")
        .join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn write(dir: &Path, name: &str, src: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, src).expect("ghi");
    p
}

fn code(o: &Output) -> i32 {
    o.status.code().expect("có mã thoát")
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

const GOOD: &str = "ƒ main() → u32 ! ∅ { 1u32 + 2u32 }\n";
const RED: &str = "ƒ main() → u8 ! ∅ { x: u8 ≔ 300; x }\n";
const YELLOW: &str = "ƒ main() → &str ! ∅ { k ≔ \"AKIAIOSFODNN7EXAMPLE\"; k }\n";
const ABORTS: &str = "ƒ main() → u8 ! ∅ { μs ≔ 250u8; ⟳ i in 0..10u32 { s ← s + 1; } s }\n";

#[test]
fn check_exit_codes_green_red_and_usage() {
    let d = scratch("check_codes");
    let good = write(&d, "good.gt", GOOD);
    let red = write(&d, "red.gt", RED);
    assert_eq!(
        code(&gritc().arg("check").arg(&good).output().expect("chạy")),
        0
    );
    let o = gritc().arg("check").arg(&red).output().expect("chạy");
    assert_eq!(code(&o), 1);
    assert!(err(&o).contains("E0601"), "{}", err(&o));
    assert!(err(&o).contains("backend: interpreter"));
    // lỗi dùng ⇒ 3 (không lẫn với ĐỎ = 1)
    for args in [
        vec![],
        vec!["bogus"],
        vec!["check"],
        vec!["check", "/không/có/tệp.gt"],
        vec!["check", "--checks=bogus", "x.gt"],
        vec!["check", "--lạ", "x.gt"],
        vec!["build", "-o"],
    ] {
        let o = gritc().args(&args).output().expect("chạy");
        assert_eq!(code(&o), 3, "{args:?}: {}", err(&o));
    }
}

#[test]
fn red_run_and_build_produce_no_output_and_never_execute() {
    let d = scratch("red_nothing");
    let red = write(&d, "red.gt", RED);
    let outp = d.join("red.gtmir");
    let o = gritc()
        .arg("build")
        .arg("--backend=interpreter")
        .arg(&red)
        .arg("-o")
        .arg(&outp)
        .output()
        .expect("chạy");
    assert_eq!(code(&o), 1);
    assert!(!outp.exists(), "ĐỎ mà vẫn ghi gói");
    assert!(
        !Path::new(&format!("{}.tmp", outp.display())).exists(),
        "để lại tệp tạm"
    );
    let o = gritc()
        .arg("run")
        .arg("--backend=interpreter")
        .arg(&red)
        .output()
        .expect("chạy");
    assert_eq!(code(&o), 1);
    assert!(!out(&o).contains("chạy xong"), "ĐỎ mà vẫn chạy");
    // không tệp nào khác sinh ra
    let names: Vec<String> = std::fs::read_dir(&d)
        .expect("dir")
        .map(|e| e.expect("e").file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["red.gt".to_string()], "{names:?}");
}

#[test]
fn ice_run_and_build_produce_no_output_and_never_execute() {
    let d = scratch("ice_nothing");
    let good = write(&d, "good.gt", GOOD);
    // chèn lỗi vào MỌI loại pass: panic ⇒ ICE (2); bỏ pass ⇒ thiếu pass bắt buộc ⇒ ICE (2)
    for fault in [
        "panic:m6.borrowck",
        "panic:m7.obligations",
        "panic:m7.const_bounds",
        "panic:m5.return_paths",
        "panic:m8.termination",
        "panic:m9.secrets",
        "skip:m7.obligations",
        "skip:m5.mir",
        "skip:m8.conc",
        "skip:selftest",
    ] {
        let outp = d.join("g.gtmir");
        let _ = std::fs::remove_file(&outp);
        let o = gritc()
            .env("GRITC_FAULT", fault)
            .arg("run")
            .arg("--backend=interpreter")
            .arg(&good)
            .output()
            .expect("chạy");
        assert_eq!(code(&o), 2, "{fault}: {}", err(&o));
        assert!(!out(&o).contains("chạy xong"), "{fault}: ICE mà vẫn chạy");
        let o = gritc()
            .env("GRITC_FAULT", fault)
            .arg("build")
            .arg("--backend=interpreter")
            .arg(&good)
            .arg("-o")
            .arg(&outp)
            .output()
            .expect("chạy");
        assert_eq!(code(&o), 2, "{fault}");
        assert!(!outp.exists(), "{fault}: ICE mà vẫn ghi gói");
        assert!(err(&o).contains("ice"), "{fault}: {}", err(&o));
    }
}

#[test]
fn yellow_runs_and_warns_and_deny_yellow_blocks() {
    let d = scratch("yellow");
    let y = write(&d, "y.gt", YELLOW);
    let o = gritc()
        .arg("run")
        .arg("--backend=interpreter")
        .arg(&y)
        .output()
        .expect("chạy");
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(err(&o).contains("S101"), "phải in cảnh báo: {}", err(&o));
    assert!(
        out(&o).contains("chạy xong"),
        "VÀNG phải vẫn chạy: {}",
        out(&o)
    );
    // --deny-yellow ⇒ ĐỎ: không chạy, không ghi gói
    let o = gritc()
        .arg("run")
        .arg("--backend=interpreter")
        .arg("--deny-yellow")
        .arg(&y)
        .output()
        .expect("chạy");
    assert_eq!(code(&o), 1);
    assert!(!out(&o).contains("chạy xong"));
    let outp = d.join("y.gtmir");
    let o = gritc()
        .args(["build", "--deny-yellow", "--backend=interpreter"])
        .arg(&y)
        .arg("-o")
        .arg(&outp)
        .output()
        .expect("chạy");
    assert_eq!(code(&o), 1);
    assert!(!outp.exists());
    // build VÀNG bình thường: ghi gói, mã 0
    let o = gritc()
        .arg("build")
        .arg("--backend=interpreter")
        .arg(&y)
        .arg("-o")
        .arg(&outp)
        .output()
        .expect("chạy");
    assert_eq!(code(&o), 0);
    assert!(outp.exists());
}

#[test]
fn run_exit_codes_are_partitioned_from_check_codes_both_sides() {
    let d = scratch("partition");
    let good = write(&d, "good.gt", GOOD);
    let ab = write(&d, "ab.gt", ABORTS);
    let red = write(&d, "red.gt", RED);
    // phía gritc (giai đoạn check): ĐỎ ⇒ 1
    let c_red = code(
        &gritc()
            .arg("run")
            .arg("--backend=interpreter")
            .arg(&red)
            .output()
            .expect("chạy"),
    );
    assert_eq!(c_red, 1);
    // phía chương trình (giai đoạn run): bình thường ⇒ 0, abort ⇒ 70
    assert_eq!(
        code(
            &gritc()
                .arg("run")
                .arg("--backend=interpreter")
                .arg(&good)
                .output()
                .expect("chạy")
        ),
        0
    );
    let o = gritc()
        .arg("run")
        .arg("--backend=interpreter")
        .arg(&ab)
        .output()
        .expect("chạy");
    assert_eq!(code(&o), 70, "{}{}", out(&o), err(&o));
    assert!(err(&o).contains("abort"));
    // 70 không bao giờ là mã của gritc ở giai đoạn check
    assert!(![0, 1, 2, 3].contains(&70));
    // JSON nói rõ giai đoạn và khớp mã thoát
    let o = gritc()
        .args(["run", "--json", "--backend=interpreter"])
        .arg(&ab)
        .output()
        .expect("chạy");
    assert_eq!(code(&o), 70);
    let j = out(&o);
    assert!(j.contains("\"phase\":\"run\""), "{j}");
    assert!(
        j.contains("\"exit_status\":70") && j.contains("\"aborted\":true"),
        "{j}"
    );
    assert!(j.contains("\"backend\":\"interpreter\""));
    let o = gritc()
        .args(["run", "--json", "--backend=interpreter"])
        .arg(&red)
        .output()
        .expect("chạy");
    assert_eq!(code(&o), 1);
    let j = out(&o);
    assert!(
        j.contains("\"phase\":\"check\"")
            && j.contains("\"verdict\":\"red\"")
            && !j.contains("\"run\":"),
        "{j}"
    );
}

#[test]
fn json_output_has_backend_checks_and_passes() {
    let d = scratch("json");
    let good = write(
        &d,
        "good.gt",
        "ƒ main() → u32 ! ∅ { xs ≔ [1u32, 2u32]; xs[0] + xs[1] }\n",
    );
    let o = gritc()
        .args(["check", "--json"])
        .arg(&good)
        .output()
        .expect("chạy");
    assert_eq!(code(&o), 0);
    let j = out(&o);
    for need in [
        "\"schema_version\":1",
        "\"tool\":\"gritc\"",
        "\"backend\":\"interpreter\"",
        "\"phase\":\"check\"",
        "\"verdict\":\"green\"",
        "\"checks\":{\"mode\":\"all\"",
        "{\"name\":\"m7.const_bounds\",\"status\":\"done\"}",
        "{\"name\":\"m5.return_paths\",\"status\":\"done\"}",
    ] {
        assert!(j.contains(need), "thiếu {need}: {j}");
    }
    // chế độ proven
    let o = gritc()
        .args(["check", "--json", "--checks=proven"])
        .arg(&good)
        .output()
        .expect("chạy");
    assert!(out(&o).contains("\"checks\":{\"mode\":\"proven\""));
}

#[test]
fn build_package_is_deterministic_and_attested() {
    let d = scratch("package");
    let g = write(&d, "g.gt", GOOD);
    let p1 = d.join("a.gtmir");
    let p2 = d.join("b.gtmir");
    assert_eq!(
        code(
            &gritc()
                .arg("build")
                .arg("--backend=interpreter")
                .arg(&g)
                .arg("-o")
                .arg(&p1)
                .output()
                .expect("chạy")
        ),
        0
    );
    assert_eq!(
        code(
            &gritc()
                .arg("build")
                .arg("--backend=interpreter")
                .arg(&g)
                .arg("-o")
                .arg(&p2)
                .output()
                .expect("chạy")
        ),
        0
    );
    let (a, b) = (
        std::fs::read_to_string(&p1).expect("đọc"),
        std::fs::read_to_string(&p2).expect("đọc"),
    );
    assert_eq!(a, b, "cùng nguồn + cùng phiên bản ⇒ cùng gói (D9)");
    assert!(a.starts_with("GTMIR1\nattestation: "));
    assert!(a.contains("backend: interpreter\nchecks: all\n"));
    let att = a
        .lines()
        .nth(1)
        .expect("dòng")
        .trim_start_matches("attestation: ");
    assert_eq!(att.len(), 64);
    // đổi nguồn ⇒ đổi attestation
    let g2 = write(&d, "g2.gt", "ƒ main() → u32 ! ∅ { 1u32 + 4u32 }\n");
    let p3 = d.join("c.gtmir");
    assert_eq!(
        code(
            &gritc()
                .arg("build")
                .arg("--backend=interpreter")
                .arg(&g2)
                .arg("-o")
                .arg(&p3)
                .output()
                .expect("chạy")
        ),
        0
    );
    let c = std::fs::read_to_string(&p3).expect("đọc");
    assert_ne!(c.lines().nth(1), a.lines().nth(1));
}

#[test]
fn explain_reads_docs_errors() {
    let o = gritc().args(["explain", "E0601"]).output().expect("chạy");
    assert_eq!(code(&o), 0);
    assert!(out(&o).contains("E0601"));
    for c in ["S101", "N1401", "E1401", "E0502", "E0503", "E0701"] {
        let o = gritc().args(["explain", c]).output().expect("chạy");
        assert_eq!(code(&o), 0, "{c}");
        assert!(out(&o).contains(c));
    }
    assert_eq!(
        code(&gritc().args(["explain", "E7777"]).output().expect("chạy")),
        3
    );
    assert_eq!(
        code(&gritc().args(["explain", "xyz"]).output().expect("chạy")),
        3
    );
    assert_eq!(code(&gritc().args(["explain"]).output().expect("chạy")), 3);
}

#[test]
fn fmt_rewrites_checks_and_refuses_without_touching_the_file() {
    let d = scratch("fmt");
    let messy = "ƒ   main( )  →  u32 !  ∅ {   1u32+2u32 }\n";
    let f = write(&d, "m.gt", messy);
    // --check: chưa chuẩn ⇒ 1, không sửa tệp
    let o = gritc()
        .args(["fmt", "--check"])
        .arg(&f)
        .output()
        .expect("chạy");
    assert_eq!(code(&o), 1);
    assert_eq!(std::fs::read_to_string(&f).expect("đọc"), messy);
    // định dạng thật
    assert_eq!(code(&gritc().arg("fmt").arg(&f).output().expect("chạy")), 0);
    let after = std::fs::read_to_string(&f).expect("đọc");
    assert_ne!(after, messy);
    assert_eq!(
        code(
            &gritc()
                .args(["fmt", "--check"])
                .arg(&f)
                .output()
                .expect("chạy")
        ),
        0
    );
    // lần hai không đổi (idempotent)
    assert_eq!(code(&gritc().arg("fmt").arg(&f).output().expect("chạy")), 0);
    assert_eq!(std::fs::read_to_string(&f).expect("đọc"), after);
    // comment được giữ nguyên
    let c = write(
        &d,
        "c.gt",
        "// ghi chú đầu\nƒ main() → u32 ! ∅ {   1u32 } // cuối dòng\n",
    );
    assert_eq!(code(&gritc().arg("fmt").arg(&c).output().expect("chạy")), 0);
    let ct = std::fs::read_to_string(&c).expect("đọc");
    assert!(
        ct.contains("// ghi chú đầu") && ct.contains("// cuối dòng"),
        "{ct}"
    );
    // lỗi cú pháp ⇒ từ chối (1), tệp nguyên vẹn
    let bad = "ƒ main( → { \n";
    let b = write(&d, "b.gt", bad);
    let o = gritc().arg("fmt").arg(&b).output().expect("chạy");
    assert_eq!(code(&o), 1, "{}", err(&o));
    assert_eq!(std::fs::read_to_string(&b).expect("đọc"), bad);
}

#[test]
fn directory_input_and_version() {
    let d = scratch("dir");
    write(&d, "a.gt", "▢ P { x: i32 }\n");
    write(&d, "main.gt", "ƒ main() → u32 ! ∅ { 7u32 }\n");
    let o = gritc()
        .arg("run")
        .arg("--backend=interpreter")
        .arg(&d)
        .output()
        .expect("chạy");
    assert_eq!(code(&o), 0, "{}{}", out(&o), err(&o));
    assert!(out(&o).contains("Int(7"));
    let o = gritc().arg("--version").output().expect("chạy");
    assert_eq!(code(&o), 0);
    let want = if cfg!(feature = "llvm") {
        "backend mặc định: llvm"
    } else {
        "backend mặc định: interpreter"
    };
    assert!(out(&o).contains(want), "{}", out(&o));
}

/// G1 (Seed 1.0): tệp được chỉ định là module gốc BẤT KỂ tên (trước đây `01_add.gt`, `my-prog.gt`, `in.gt` bị từ chối với `BadModuleName`).
#[test]
fn file_with_non_identifier_name_is_accepted_as_the_root_module() {
    let d = scratch("badname");
    for (name, ok) in [
        ("01_add.gt", true),
        ("my-prog.gt", true),
        ("in.gt", true),
        ("tên-chương-trình.gt", true),
        ("plain.gt", true),
    ] {
        write(&d, name, "ƒ main() → u32 ! ∅ { 7u32 }\n");
        let o = gritc()
            .args(["run", "--backend=interpreter"])
            .arg(d.join(name))
            .output()
            .expect("chạy");
        assert_eq!(code(&o) == 0, ok, "{name}: {}{}", out(&o), err(&o));
        assert!(out(&o).contains("Int(7"), "{name}: {}{}", out(&o), err(&o));
    }
    // chẩn đoán vẫn trỏ về tên tệp người dùng nhìn thấy được (đã chuẩn hoá thành định danh hợp lệ)
    write(&d, "7bad-name.gt", "ƒ main() → u32 ! ∅ { true }\n");
    let o = gritc().arg("check").arg(d.join("7bad-name.gt")).output().expect("chạy");
    assert_eq!(code(&o), 1, "{}{}", out(&o), err(&o));
    assert!(out(&o).contains("E0301") || err(&o).contains("E0301"), "{}{}", out(&o), err(&o));
}
