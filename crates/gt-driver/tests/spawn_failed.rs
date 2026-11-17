//! GÓI 3 S2.b (hợp đồng H8): tạo luồng `spawn` thất bại ⇒ abort TÊN `spawn_failed` (stderr `abort: spawn_failed`, mã 70), KHÔNG chạy tiếp lặng lẽ.
//! Ép thất bại bằng cơ chế CHỈ có trong bản dựng kiểm thử của runtime: `cargo build --features test-fault` (biến `GRITRT_TEST_FORCE_SPAWN_FAILURE`).
//! (1) runtime test-fault + biến ⇒ abort đúng loại ở dev VÀ release-o2; (2) ĐỐI CHỨNG: runtime sản xuất (không feature) bỏ qua biến ⇒ chương trình chạy bình thường
//! (chứng minh đường ép lỗi KHÔNG nằm trong bản sản xuất).
#![cfg(feature = "llvm")]
use std::path::{Path, PathBuf};
use std::process::Command;

const PROG: &str = "■ DONE: Atomic<u32> ≔ Atomic::new(0u32);\n\
    ƒ worker(n: u32) ! ∅ { DONE.fetch_add(n, AtomicOrdering::SeqCst); }\n\
    ƒ main() → u32 ! ∅ { spawn(worker, 1u32); 7u32 }\n";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("grit_spawnfail_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo");
    d
}

/// Dựng runtime với feature `test-fault` vào thư mục target riêng (không đụng bản sản xuất ở runtime/gt-rt/target).
fn build_fault_runtime() -> PathBuf {
    let target = scratch("rtfault");
    let st = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args([
            "build",
            "--release",
            "--features",
            "test-fault",
            "--manifest-path",
        ])
        .arg(root().join("runtime/gt-rt/Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target)
        .env_remove("CARGO_BUILD_TARGET")
        .status()
        .expect("chạy cargo");
    assert!(st.success(), "dựng runtime test-fault thất bại");
    for n in ["gt_rt.lib", "libgt_rt.a"] {
        let p = target.join("release").join(n);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "không thấy thư viện runtime test-fault trong {}",
        target.display()
    );
}

fn run(src: &Path, profile: &str, rt: Option<&Path>, force: bool) -> (Option<i32>, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_gritc"));
    c.args(["run", "--backend=llvm", &format!("--profile={profile}")])
        .arg(src);
    if let Some(rt) = rt {
        c.env("GRIT_RT_LIB", rt);
    }
    if force {
        c.env("GRITRT_TEST_FORCE_SPAWN_FAILURE", "1");
    }
    let o = c.output().expect("chạy gritc");
    (
        o.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
    )
}

#[test]
fn forced_spawn_failure_aborts_with_spawn_failed_in_dev_and_release_o2_and_production_runtime_ignores_the_switch()
 {
    let dir = scratch("prog");
    let src = dir.join("main.gt");
    std::fs::write(&src, PROG).expect("ghi");
    let fault_rt = build_fault_runtime();
    for profile in ["dev", "release-o2"] {
        // không ép lỗi ⇒ chạy bình thường (7)
        let (code, out) = run(&src, profile, Some(&fault_rt), false);
        assert_eq!(code, Some(0), "{profile} (không ép): {out}");
        assert!(out.contains("main trả về 7"), "{profile} (không ép): {out}");
        // ép lỗi + runtime test-fault ⇒ abort có tên, mã 70 (gritc báo mã của exe hoặc 70)
        let (code, out) = run(&src, profile, Some(&fault_rt), true);
        assert!(out.contains("abort: spawn_failed"), "{profile} (ép): {out}");
        assert_ne!(
            code,
            Some(0),
            "{profile} (ép): không được chạy tiếp thành công: {out}"
        );
        assert!(
            !out.contains("main trả về 7"),
            "{profile} (ép): chạy tiếp lặng lẽ! {out}"
        );
        // ĐỐI CHỨNG: runtime sản xuất mặc định bỏ qua biến ⇒ không abort
        let (code, out) = run(&src, profile, None, true);
        assert_eq!(code, Some(0), "{profile} (sản xuất, có biến): {out}");
        assert!(
            !out.contains("spawn_failed"),
            "{profile} (sản xuất): đường ép lỗi lọt vào bản sản xuất! {out}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
