// Nạp LLVM-C.dll TRỄ cho `rig` (như gritc; ADR-029): `rig new/lock/fmt/clean` chạy được khi PATH không có LLVM.
fn main() {
    if std::env::var_os("CARGO_FEATURE_LLVM").is_some() {
        println!("cargo:rustc-link-arg-bins=/DELAYLOAD:LLVM-C.dll");
        println!("cargo:rustc-link-arg-tests=/DELAYLOAD:LLVM-C.dll");
        println!("cargo:rustc-link-arg-tests=/IGNORE:4199");
        println!("cargo:rustc-link-arg-bins=/IGNORE:4199");
    }
}
