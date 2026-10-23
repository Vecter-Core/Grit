//! FB-C21 / Seed 1.0 S7 (đọc mã đối kháng nhóm D, D10): thứ tự khởi tạo `■` chỉ theo phụ thuộc TRỰC TIẾP trong thân initializer ⇒ static đọc
//! gián tiếp qua hàm (kể cả hàm là giá trị hàm) được đọc khi còn ZERO-INIT (kết quả sai; gọi con trỏ hàm NULL ⇒ SIGSEGV từ chương trình "xanh").
//! Đúng: bao đóng BẮC CẦU qua đồ thị gọi/giá trị hàm; vòng phụ thuộc ⇒ từ chối (fail-closed, mã 3), không chạy.
#![cfg(feature = "llvm")]
use std::process::Command;

fn run(name: &str, src: &str, profile: &str) -> (Option<i32>, String) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("gt-sinit-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    let f = d.join(format!("{name}.gt"));
    std::fs::write(&f, src).expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args(["run", "--backend=llvm", &format!("--profile={profile}")])
        .arg(&f)
        .output()
        .expect("chạy gritc");
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    (o.status.code(), out)
}

const PROFILES: [&str; 3] = ["dev", "release", "release-o2"];

#[test]
fn a_static_read_through_a_called_function_is_initialised_first_fb_c21() {
    // A đọc Z qua f(); Z khai sau A
    let src = "■ A: u32 ≔ f();\n■ Z: u32 ≔ 9u32;\nƒ f() → u32 ! ∅ { Z + 1u32 }\nƒ main() → u32 ! ∅ { A }\n";
    for p in PROFILES {
        let (code, out) = run("order", src, p);
        assert!(out.contains("main trả về 10"), "[{p}] {code:?}\n{out}");
    }
}

#[test]
fn a_static_function_pointer_used_by_another_initialiser_is_set_first_fb_c21() {
    // A gọi qua con trỏ hàm F (khai sau) — trước sửa: gọi con trỏ NULL ⇒ SIGSEGV
    let src = "ƒ id(x: u32) → u32 ! ∅ { x + 1u32 }\n■ A: u32 ≔ run();\n■ F: ƒ(u32) → u32 ! ∅ ≔ id;\nƒ run() → u32 ! ∅ { F(41u32) }\nƒ main() → u32 ! ∅ { A }\n";
    for p in PROFILES {
        let (code, out) = run("fnptr", src, p);
        assert!(out.contains("main trả về 42"), "[{p}] {code:?}\n{out}");
    }
}

#[test]
fn a_transitive_chain_through_two_functions_is_ordered_fb_c21() {
    let src = "■ A: u32 ≔ f();\n■ B: u32 ≔ 5u32;\n■ C: u32 ≔ 100u32;\nƒ f() → u32 ! ∅ { g() + C }\nƒ g() → u32 ! ∅ { B * 2u32 }\nƒ main() → u32 ! ∅ { A }\n";
    for p in PROFILES {
        let (code, out) = run("chain", src, p);
        assert!(out.contains("main trả về 110"), "[{p}] {code:?}\n{out}");
    }
}

#[test]
fn a_dependency_cycle_between_initialisers_is_rejected_not_run_fb_c21() {
    let src =
        "■ A: u32 ≔ f();\n■ B: u32 ≔ 1u32 + A;\nƒ f() → u32 ! ∅ { B }\nƒ main() → u32 ! ∅ { A }\n";
    for p in PROFILES {
        let (code, out) = run("cycle", src, p);
        assert_eq!(code, Some(3), "[{p}] phải bị từ chối (mã 3)\n{out}");
        assert!(out.contains("vòng"), "[{p}] thiếu lý do\n{out}");
    }
}
