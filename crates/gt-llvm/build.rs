//! Liên kết ĐỘNG `LLVM-C.lib` ↔ `LLVM-C.dll` của LLVM 20.1.8 bản dựng sẵn (ADR-029 §2; spike đạt 2026-10-12).
//! Nạp DLL TRỄ: `gritc check/fmt/explain` chạy được khi PATH không có LLVM; thiếu DLL lúc cần ⇒ gritc báo và thoát 3.
fn main() {
    println!("cargo:rerun-if-env-changed=GRIT_LLVM_DIR");
    let Ok(dir) = std::env::var("GRIT_LLVM_DIR") else {
        panic!(
            "thiếu GRIT_LLVM_DIR (thư mục LLVM 20.1.8, xem docs/ENVIRONMENT.md); KHÔNG đoán đường dẫn"
        );
    };
    let lib = std::path::Path::new(&dir).join("lib").join("LLVM-C.lib");
    assert!(
        lib.is_file(),
        "không thấy {} — GRIT_LLVM_DIR sai?",
        lib.display()
    );
    println!("cargo:rustc-link-search=native={dir}/lib");
    println!("cargo:rustc-link-lib=dylib=LLVM-C");
    println!("cargo:rustc-link-lib=dylib=delayimp");
    // `rustc-link-arg` chỉ áp cho mục tiêu của CHÍNH crate này; crate dùng nó (gt-driver) tự phát /DELAYLOAD cho bin/test của nó.
    println!("cargo:rustc-link-arg=/DELAYLOAD:LLVM-C.dll");
}
