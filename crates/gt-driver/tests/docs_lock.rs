//! Seed 1.0 S3d: `docs/STD_REFERENCE.md` được SINH bởi `tools/gen_docs.py` từ `lib/core/prelude.gt` + `lib/std/src/std.gt`; test khoá từng byte
//! (sửa tay tệp, hoặc đổi chữ ký ở nguồn mà quên sinh lại ⇒ ĐỎ) và kiểm bộ sinh tất định (hai lần chạy giống hệt).
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn python() -> &'static str {
    if Command::new("python3").arg("--version").output().is_ok() {
        "python3"
    } else {
        "python"
    }
}

fn generate() -> Vec<u8> {
    let o = Command::new(python())
        .arg(root().join("tools/gen_docs.py"))
        .arg("--stdout")
        .output()
        .expect("chạy tools/gen_docs.py");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    o.stdout
}

#[test]
fn std_reference_matches_generator() {
    let a = generate();
    assert_eq!(a, generate(), "bộ sinh không tất định");
    let on_disk = std::fs::read_to_string(root().join("docs/STD_REFERENCE.md")).expect("đọc docs/STD_REFERENCE.md");
    assert_eq!(
        String::from_utf8_lossy(&a).replace("\r\n", "\n"),
        on_disk.replace("\r\n", "\n"),
        "docs/STD_REFERENCE.md khác đầu ra của tools/gen_docs.py: sửa nguồn rồi `python3 tools/gen_docs.py`"
    );
}

/// Mọi hàm/phương thức công khai của std và prelude phải có mặt trong tài liệu (không thiếu vì bộ phân tích bỏ sót chữ ký).
#[test]
fn every_public_function_in_std_and_prelude_is_documented() {
    let doc = std::fs::read_to_string(root().join("docs/STD_REFERENCE.md")).expect("đọc docs/STD_REFERENCE.md");
    for src in ["lib/core/prelude.gt", "lib/std/src/std.gt"] {
        let text = std::fs::read_to_string(root().join(src)).expect("đọc nguồn");
        let mut n = 0;
        for line in text.lines() {
            let t = line.trim_start();
            let t = t.strip_prefix("◈ ").unwrap_or(t);
            if let Some(rest) = t.strip_prefix("ƒ ") {
                let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                if name.is_empty() || ["swap_at", "sift_down", "parse_digits", "first_non_ws", "end_after_last_non_ws", "next_cap"].contains(&name.as_str()) {
                    continue;
                }
                n += 1;
                assert!(doc.contains(&format!("ƒ {name}")), "{src}: hàm `{name}` thiếu trong STD_REFERENCE.md");
            }
        }
        assert!(n > 20, "{src}: chỉ thấy {n} hàm — bộ quét lỗi?");
    }
}
