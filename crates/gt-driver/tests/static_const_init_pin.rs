//! GÓI 12 P3.c — GHIM hành vi hiện tại của khởi tạo `■` (static) và `□` (hằng) có initializer không phải literal, ở cả hai backend (mã thoát + thông báo).
//! Đây là bản ghi hành vi ĐO ĐƯỢC (2026-11-20, gritc 1.0.0-rc8 dev), KHÔNG phải tuyên bố hành vi đúng:
//!  - vòng khởi tạo `■` (qua hàm): `check` XANH (mục 8/12 NEXT_PRODUCT_BATCH, chờ ADR mã lỗi mới); LLVM từ chối (`build`/`run` mã 3, KHÔNG sinh exe);
//!    thông dịch báo "lỗi nội bộ" (ICE, mã 2) — với chương trình người dùng đây là ICE chứ không phải chẩn đoán;
//!  - `■`/`□` KHÔNG vòng, initializer gọi hàm: LLVM chạy đúng; thông dịch cũng ICE ("hằng ngoài phạm vi thông dịch") — hạn chế của backend interpreter;
//!  - vòng `□` (FB-C36): `check` ĐỎ E0703 (mã thoát 1), không có exe.
//!
//! Khi sửa theo NEXT_PRODUCT_BATCH #12 (check đỏ cho vòng `■`), test `static_cycles_*` PHẢI đổi — đó là chủ ý của việc ghim.
use std::process::Command;

fn gritc(args: &[&str], src: &str, tag: &str) -> (i32, String) {
    let d = std::env::temp_dir().join(format!("grit_pin_{tag}_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    let f = d.join(format!("{tag}.gt"));
    std::fs::write(&f, src).expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args(args)
        .arg(&f)
        .output()
        .expect("chạy gritc");
    let mut text = String::from_utf8_lossy(&o.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&o.stderr));
    (o.status.code().unwrap_or(-1), text)
}

const S_CYCLE_F: &str = "ƒ f() → u32 ! ∅ { X }\n■ X: u32 ≔ f();\nƒ main() → i32 ! ∅ { r ≔ X; 0 }\n";
const S_CYCLE_DEEP: &str = "ƒ h() → u32 ! ∅ { X }\nƒ g() → u32 ! ∅ { h() }\n■ X: u32 ≔ g();\nƒ main() → i32 ! ∅ { r ≔ X; 0 }\n";
const S_CYCLE_MUTUAL: &str = "■ A: u32 ≔ B;\n■ B: u32 ≔ A;\nƒ main() → i32 ! ∅ { r ≔ A; 0 }\n";
const S_OK: &str =
    "ƒ f() → u32 ! ∅ { Y }\n■ Y: u32 ≔ 7;\n■ X: u32 ≔ f();\nƒ main() → i32 ! ∅ { r ≔ X; 0 }\n";
const S_OK2: &str = "ƒ f() → u32 ! ∅ { 3u32 }\n■ X: u32 ≔ f();\nƒ main() → i32 ! ∅ { r ≔ X; 0 }\n";
const S_OK3: &str = "■ Y: u32 ≔ 9;\n■ X: u32 ≔ Y;\nƒ main() → i32 ! ∅ { r ≔ X; 0 }\n";
const C_OK: &str = "ƒ f() → u32 ! ∅ { 3u32 }\n□ C: u32 ≔ f();\nƒ main() → i32 ! ∅ { r ≔ C; 0 }\n";
const C_OK2: &str =
    "□ K: u32 ≔ 5u32;\nƒ f() → u32 ! ∅ { K }\n□ C: u32 ≔ f();\nƒ main() → i32 ! ∅ { r ≔ C; 0 }\n";
const C_OK3: &str = "□ A: u32 ≔ 1u32;\n□ B: u32 ≔ A;\nƒ main() → i32 ! ∅ { r ≔ B; 0 }\n";
const C_CYCLE_SELF: &str = "□ C: u32 ≔ C;\nƒ main() → i32 ! ∅ { r ≔ C; 0 }\n";
const C_CYCLE_MUTUAL: &str = "□ A: u32 ≔ B;\n□ B: u32 ≔ A;\nƒ main() → i32 ! ∅ { r ≔ A; 0 }\n";
const C_CYCLE_F: &str = "ƒ f() → u32 ! ∅ { C }\n□ C: u32 ≔ f();\nƒ main() → i32 ! ∅ { r ≔ C; 0 }\n";

fn check_green(src: &str, tag: &str) {
    let (c, t) = gritc(&["check"], src, tag);
    assert_eq!(c, 0, "check phải XANH (hành vi ghim): {t}");
}

fn interp_ice(src: &str, tag: &str) {
    let (c, t) = gritc(&["run", "--backend=interpreter"], src, tag);
    assert_eq!(c, 2, "thông dịch phải ICE (mã 2): {t}");
    assert!(t.contains("lỗi nội bộ khi chạy"), "{t}");
    assert!(t.contains("hằng ngoài phạm vi thông dịch"), "{t}");
}

#[test]
fn static_cycles_check_green_interpreter_ice() {
    for (s, tag) in [
        (S_CYCLE_F, "sc1"),
        (S_CYCLE_DEEP, "sc2"),
        (S_CYCLE_MUTUAL, "sc3"),
    ] {
        check_green(s, tag);
        interp_ice(s, tag);
    }
}

#[test]
fn acyclic_static_and_const_initialisers_check_green_interpreter_ice() {
    for (s, tag) in [
        (S_OK, "ok1"),
        (S_OK2, "ok2"),
        (S_OK3, "ok3"),
        (C_OK, "ok4"),
        (C_OK2, "ok5"),
        (C_OK3, "ok6"),
    ] {
        check_green(s, tag);
        interp_ice(s, tag);
    }
}

#[test]
fn const_cycles_are_rejected_by_check_and_never_build() {
    for (s, tag) in [
        (C_CYCLE_SELF, "cc1"),
        (C_CYCLE_MUTUAL, "cc2"),
        (C_CYCLE_F, "cc3"),
    ] {
        let (c, t) = gritc(&["check"], s, tag);
        assert_eq!(c, 1, "{t}");
        assert!(t.contains("E0703"), "{t}");
        let (c, t) = gritc(&["run", "--backend=interpreter"], s, tag);
        assert_eq!(c, 1, "vòng hằng không được chạy: {t}");
    }
}

#[cfg(feature = "llvm")]
mod llvm {
    use super::*;

    #[test]
    fn static_cycles_are_refused_by_the_llvm_backend_with_no_exe() {
        for (s, tag) in [
            (S_CYCLE_F, "lc1"),
            (S_CYCLE_DEEP, "lc2"),
            (S_CYCLE_MUTUAL, "lc3"),
        ] {
            let out =
                std::env::temp_dir().join(format!("grit_pin_lc_{tag}_{}.exe", std::process::id()));
            let _ = std::fs::remove_file(&out);
            let (c, t) = gritc(
                &["build", "--backend=llvm", "-o", out.to_str().unwrap()],
                s,
                tag,
            );
            assert_eq!(c, 3, "{t}");
            assert!(t.contains("vòng phụ thuộc khởi tạo giữa các static"), "{t}");
            assert!(
                !out.exists(),
                "KHÔNG được sinh exe cho vòng khởi tạo static"
            );
        }
    }

    #[test]
    fn acyclic_initialisers_run_correctly_on_llvm() {
        for (s, tag) in [
            (S_OK, "lo1"),
            (S_OK2, "lo2"),
            (S_OK3, "lo3"),
            (C_OK, "lo4"),
            (C_OK2, "lo5"),
            (C_OK3, "lo6"),
        ] {
            let (c, t) = gritc(&["run", "--backend=llvm"], s, tag);
            assert_eq!(c, 0, "{t}");
            assert!(t.contains("main trả về 0"), "{t}");
        }
    }
}
