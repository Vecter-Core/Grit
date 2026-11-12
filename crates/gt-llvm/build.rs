//! Liên kết `LLVM-C` của LLVM 20 (ADR-029 §2).
//!
//! * Windows (nền tảng chính): ĐỘNG `LLVM-C.lib` ↔ `LLVM-C.dll` của LLVM 20.1.8 bản dựng sẵn; nạp DLL TRỄ nên `gritc check/fmt/explain` chạy được khi
//!   PATH không có LLVM; thiếu DLL lúc cần ⇒ gritc báo và thoát 3. Thư mục lấy từ `GRIT_LLVM_DIR`.
//! * Linux: liên kết `libLLVM-20.so` (gói `libllvm20`); thư mục chứa nó lấy từ `GRIT_LLVM_LIB_DIR` (mặc định `/usr/lib/x86_64-linux-gnu`).
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows();
    } else {
        linux();
    }
}

fn windows() {
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

fn linux() {
    println!("cargo:rerun-if-env-changed=GRIT_LLVM_LIB_DIR");
    let dir = std::env::var("GRIT_LLVM_LIB_DIR")
        .unwrap_or_else(|_| "/usr/lib/x86_64-linux-gnu".to_string());
    let lib = std::path::Path::new(&dir).join("libLLVM-20.so");
    assert!(
        lib.is_file(),
        "không thấy {} — cài gói `libllvm20` hoặc đặt GRIT_LLVM_LIB_DIR tới thư mục chứa libLLVM-20.so",
        lib.display()
    );
    println!("cargo:rustc-link-search=native={dir}");
    println!("cargo:rustc-link-lib=dylib=LLVM-20");
}
