//! Hồ sơ `release` (ADR-036 §6): Guard-post từ chối từng loại lỗi chèn SAU tối ưu bằng E1101; báo cáo "check còn lại" không đổi; băm IR hai giai đoạn.
use std::path::PathBuf;
use std::process::{Command, Output};

/// hồ sơ release dưới kiểm: `GT_PROFILE` (`release` hoặc `release-o2`; mặc định `release`)
fn rel() -> String {
    std::env::var("GT_PROFILE").unwrap_or_else(|_| "release".to_string())
}

const PROG: &str = "ƒ f(n: u64) → u64 ! ∅ { μs ≔ 0u64; μi ≔ 0u64; ⟳ i < n ↓ n - i { s ← s + i; i ← i + 1; } s }\nƒ main() → u64 ! ∅ { f(100u64) }\n";

fn gritc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_gritc"))
}

fn scratch(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("gt-relg-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo");
    d
}

fn build(profile: &str, fault: Option<&str>, json: bool) -> (Output, PathBuf, PathBuf) {
    let dir = scratch("b");
    let f = dir.join("prog.gt");
    std::fs::write(&f, PROG).expect("ghi");
    let exe = dir.join("p.exe");
    let mut c = gritc();
    if let Some(x) = fault {
        c.env("GRITC_FAULT", x);
    }
    c.args(["build", "--backend=llvm", &format!("--profile={profile}")]);
    if json {
        c.arg("--json");
    }
    let o = c.arg("-o").arg(&exe).arg(&f).output().expect("chạy gritc");
    (o, exe, dir)
}

/// MỖI luật của Guard-post có một lỗi chèn riêng; phải bị từ chối bằng E1101, mã 1, KHÔNG có exe/object.
#[test]
fn guard_post_rejects_every_injected_fault() {
    for (fault, needle) in [
        ("post:drop-abort", "số lời gọi gritrt_abort loại"),
        ("post:fma", "hợp nhất nhân-cộng"),
        ("post:fast-math", "cờ sinh poison/UB"),
        ("post:fp-attr", "thuộc tính hàm số thực không an toàn"),
        ("post:nsw", "cờ sinh poison/UB `nsw`"),
        ("post:unreachable", "không đúng mẫu abort"),
        ("post:untagged-spawn", "thiếu metadata !gt.obl"),
    ] {
        let (o, exe, dir) = build(&rel(), Some(fault), false);
        let err = String::from_utf8_lossy(&o.stderr).into_owned();
        assert_eq!(o.status.code(), Some(1), "{fault}: {err}");
        assert!(
            err.contains("E1101") && err.contains(needle),
            "{fault}: {err}"
        );
        assert!(!exe.exists(), "{fault}: ĐỎ ⇒ KHÔNG được có exe");
        assert!(
            !dir.join("p.exe.obj").exists(),
            "{fault}: không để lại object"
        );
        // cùng chương trình, KHÔNG lỗi chèn ⇒ dựng được (lỗi trên thật sự do lỗi chèn)
        let (ok, exe2, _) = build(&rel(), None, false);
        assert_eq!(
            ok.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&ok.stderr)
        );
        assert!(exe2.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// lỗi chèn TRƯỚC tối ưu (Guard-pre) vẫn bị từ chối ở hồ sơ release
#[test]
fn guard_pre_still_runs_before_the_optimizer_in_release() {
    for (fault, needle) in [
        ("ir:unreachable", "không đúng mẫu abort"),
        ("ir:attest", "attestation lệch"),
        ("ir:instance", "không thuộc bảng instance"),
    ] {
        let (o, exe, _) = build(&rel(), Some(fault), false);
        let err = String::from_utf8_lossy(&o.stderr).into_owned();
        assert_eq!(o.status.code(), Some(1), "{fault}: {err}");
        assert!(
            err.contains("E1101") && err.contains(needle),
            "{fault}: {err}"
        );
        assert!(!exe.exists(), "{fault}");
    }
}

/// báo cáo "check còn lại" KHÔNG đổi theo hồ sơ (release chỉ đổi mức tối ưu LLVM, không bỏ check của Grit; ADR-036 §6f)
#[test]
fn release_does_not_change_the_remaining_checks_report() {
    let (d, _, _) = build("dev", None, true);
    let (r, _, _) = build(&rel(), None, true);
    assert_eq!(d.status.code(), Some(0));
    assert_eq!(r.status.code(), Some(0));
    let (ds, rs) = (
        String::from_utf8_lossy(&d.stdout).into_owned(),
        String::from_utf8_lossy(&r.stdout).into_owned(),
    );
    assert!(
        ds.contains("kept_not_proven"),
        "báo cáo phải có check còn lại: {ds}"
    );
    assert_eq!(ds, rs, "báo cáo JSON phải y hệt giữa dev và release");
}

/// băm IR trước/sau tối ưu: release in hai băm SHA-256 KHÁC nhau; dev không có băm sau
#[test]
fn release_reports_two_ir_hashes_dev_none() {
    let (r, _, _) = build(&rel(), None, false);
    let err = String::from_utf8_lossy(&r.stderr).into_owned();
    let line = err
        .lines()
        .find(|l| l.contains("băm IR trước tối ưu"))
        .unwrap_or_else(|| panic!("thiếu dòng băm: {err}"));
    let hex: Vec<&str> = line
        .split(|c: char| !c.is_ascii_hexdigit())
        .filter(|t| t.len() == 64)
        .collect();
    assert_eq!(hex.len(), 2, "{line}");
    assert_ne!(hex[0], hex[1], "IR sau tối ưu phải khác IR trước");
    let (d, _, _) = build("dev", None, false);
    assert!(!String::from_utf8_lossy(&d.stderr).contains("băm IR"));
}
