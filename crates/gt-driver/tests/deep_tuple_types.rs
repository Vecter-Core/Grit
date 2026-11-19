//! GÓI 8 S0g(ii): tuple lồng 33 tầng KHÔNG chứa con trỏ thô bị E1001 oan (E1001 là mã của `⚠`/con trỏ thô). Nguyên nhân: `has_raw_ptr` có giới hạn
//! độ sâu 32 và coi "quá sâu" là "có con trỏ thô" (fail-closed) ⇒ kiểu hợp lệ bị báo như thiếu `unsafe`. Test ĐỎ trước sửa: tuple 33 và 100 tầng bị E1001.
//! Kiểu Grit là cây hữu hạn (không đệ quy vô hạn qua ADT), nên giới hạn chỉ để bảo vệ ngăn xếp; mức mới 256 tầng.
use std::path::PathBuf;
use std::process::Command;

fn check_nested(depth: usize, name: &str) -> (i32, String) {
    // kiểu: (((u32,),),) lồng `depth` tầng; giá trị tương ứng
    let mut ty = String::from("u32");
    let mut val = String::from("1u32");
    for _ in 0..depth {
        ty = format!("({ty},)");
        val = format!("({val},)");
    }
    let src = format!("ƒ f() → i32 ! ∅ {{ t: {ty} ≔ {val}; 0 }}\nƒ main() → i32 ! ∅ {{ f() }}\n");
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("deep_tuple").join(name);
    std::fs::create_dir_all(&dir).expect("tạo thư mục tạm");
    let p = dir.join("main.gt");
    std::fs::write(&p, src).expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .arg("check")
        .arg(&p)
        .output()
        .expect("chạy gritc");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into_owned() + &String::from_utf8_lossy(&o.stderr),
    )
}

#[test]
fn tuple_of_33_levels_is_not_a_false_e1001() {
    let (code, out) = check_nested(33, "d33");
    assert!(!out.contains("E1001"), "E1001 oan với tuple 33 tầng (không có con trỏ thô): {out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn tuple_of_100_levels_is_not_a_false_e1001() {
    let (code, out) = check_nested(100, "d100");
    assert!(!out.contains("E1001"), "E1001 oan với tuple 100 tầng: {out}");
    assert_eq!(code, 0, "{out}");
}
