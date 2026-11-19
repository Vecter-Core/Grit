//! GÓI 8 S0g(i): hàm có ≥ 33 hợp đồng `⊳` (vượt miền nguyên tử `UNREP_SLOTS`) từng làm gritc PANIC (mã 2, thông báo thô `unrep_atom ngoài miền`).
//! Yêu cầu: ICE đúng định dạng — E9901 trong chẩn đoán, mã thoát 2 (lỗi nội bộ), không panic, không output. 32 hợp đồng vẫn không có E9901.
//! Test ĐỎ trước sửa: 33 hợp đồng ⇒ process bị panic (stderr có "panicked") và không có E9901.
use std::path::PathBuf;
use std::process::Command;

fn gritc_on(n: usize, name: &str) -> (i32, String, String) {
    let mut pre = String::new();
    for i in 0..n {
        pre.push_str(&format!("  ⊳ a < {}u32\n", 100000 + i));
    }
    let src =
        format!("ƒ g(a: u32) → u32 ! ∅\n{pre}{{ a }}\nƒ main() → i32 ! ∅ {{ v ≔ g(3u32); 0 }}\n");
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("ice_contracts")
        .join(name);
    std::fs::create_dir_all(&dir).expect("tạo thư mục tạm");
    let p = dir.join("main.gt");
    std::fs::write(&p, src).expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .arg("check")
        .arg("--json")
        .arg(&p)
        .output()
        .expect("chạy gritc");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn thirty_three_contracts_is_an_ice_not_a_panic() {
    let (code, out, err) = gritc_on(33, "n33");
    assert!(!err.contains("panicked"), "gritc panic thay vì ICE: {err}");
    assert!(
        out.contains("\"E9901\""),
        "thiếu E9901 trong chẩn đoán JSON: {out}"
    );
    assert_eq!(code, 2, "ICE phải thoát 2 (lỗi nội bộ); ra {code}");
}

#[test]
fn thirty_two_contracts_still_checks_without_ice() {
    let (_code, out, err) = gritc_on(32, "n32");
    assert!(!err.contains("panicked"), "{err}");
    assert!(
        !out.contains("\"E9901\""),
        "32 hợp đồng không được gây ICE: {out}"
    );
}
