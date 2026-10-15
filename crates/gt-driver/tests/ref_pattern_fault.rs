//! ADR-040: chèn lỗi `GRITC_FAULT=mir:pat-mode` làm hạ MIR suy chế độ khớp qua tham chiếu SAI ⇒ phải ra ICE E9901 (không bao giờ hạ lặng lẽ).
use std::process::Command;

fn run(src: &str, fault: Option<&str>) -> (i32, String) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "gt-patfault-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d).expect("tạo thư mục");
    let f = d.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let mut c = Command::new(env!("CARGO_BIN_EXE_gritc"));
    c.arg("check").arg(&f);
    if let Some(v) = fault {
        c.env("GRITC_FAULT", v);
    }
    let o = c.output().expect("chạy gritc");
    let _ = std::fs::remove_dir_all(&d);
    (
        o.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
    )
}

#[test]
fn a_wrong_pattern_mode_in_mir_is_ice_e9901_never_silent() {
    let shared = "⊔ E { A(u32), C }\nƒ f(r: &E) → u32 ! ∅ { ⌥ r { E::A(x) ↦ *x, E::C ↦ 0 } }\nƒ main() → u32 ! ∅ { f(&E::A(1)) }\n";
    let unique = "▢ P { x: u32 }\nƒ f(p: &!P) → u32 ! ∅ { ⌥ p { P { x } ↦ { *x ← 2; *x } } }\nƒ main() → u32 ! ∅ { μp ≔ P { x: 0 }; f(&!p) }\n";
    for src in [shared, unique] {
        let (code, out) = run(src, None);
        assert_eq!(code, 0, "đối chứng phải sạch: {out}");
        assert!(!out.contains("E9901"));
        let (code, out) = run(src, Some("mir:pat-mode"));
        println!("exit {code}: {}", out.lines().next().unwrap_or(""));
        assert!(out.contains("E9901"), "không có ICE: {out}");
        // đúng khẳng định của HẠ MIR (không phải một ICE phụ ở pass sau): thông báo phải nêu chế độ khớp
        assert!(
            out.contains("chế độ khớp qua tham chiếu"),
            "ICE không phải từ khẳng định chế độ: {out}"
        );
        assert_ne!(code, 0);
    }
}
