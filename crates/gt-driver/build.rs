// Dựng bảng `gritc explain`: nhúng mọi `docs/errors/*.md` lúc biên dịch (include_str!), sắp theo mã để tất định.
use std::{env, fs, path::PathBuf};

fn main() {
    // Nạp LLVM-C.dll TRỄ (ADR-029): `rustc-link-arg` của crate thư viện KHÔNG lan sang crate dùng nó, nên phát ở đây cho bin và test.
    if env::var_os("CARGO_FEATURE_LLVM").is_some() {
        println!("cargo:rustc-link-arg-bins=/DELAYLOAD:LLVM-C.dll");
        println!("cargo:rustc-link-arg-tests=/DELAYLOAD:LLVM-C.dll");
    }
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .join("../../docs/errors");
    println!("cargo:rerun-if-changed=../../docs/errors");
    let mut entries: Vec<(String, PathBuf)> = fs::read_dir(&dir)
        .expect("docs/errors")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .filter_map(|p| {
            let code = p.file_stem()?.to_string_lossy().into_owned();
            Some((code, p))
        })
        .collect();
    entries.sort();
    let mut out = String::from(
        "/// Tài liệu mã lỗi nhúng từ docs/errors (mã, nội dung).\npub const EXPLAIN: &[(&str, &str)] = &[\n",
    );
    for (code, path) in &entries {
        let abs = fs::canonicalize(path).expect("canonicalize");
        let abs = abs.to_string_lossy().replace(char::from(92), "/");
        let abs = abs.strip_prefix("//?/").unwrap_or(&abs).to_string();
        out.push_str(&format!("    ({code:?}, include_str!({abs:?})),\n"));
    }
    out.push_str("];\n");
    let target = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("explain_table.rs");
    fs::write(target, out).expect("ghi explain_table.rs");
}
