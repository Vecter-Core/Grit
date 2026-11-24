//! C1 Giai đoạn 2 (ADR-P2-06): `rig test --auto` sinh test từ hợp đồng `⊳ ⊲` của hàm CÔNG KHAI. NGOÀI TCB: chỉ TÌM lỗi, không cấp phép; phán quyết `gritc` không đổi.
//! E2E qua tiến trình `rig`: dự án có hàm đúng (phải qua), hàm có `⊲` sai, hàm tràn số, hàm vi phạm `⊳` của hàm khác; kỳ vọng ĐÚNG hàm/loại abort bị báo,
//! hàm đúng KHÔNG bị báo, và lỗi chỉ xảy ra ở đầu vào THOẢ `⊳` (đầu vào vi phạm `⊳` phải bị loại, không bị báo như lỗi của hàm).
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn tmp(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("rig-auto-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().expect("cha")).expect("tạo thư mục");
    std::fs::write(p, text).expect("ghi");
}

fn project(name: &str, lib: &str) -> PathBuf {
    let d = tmp(name);
    write(
        &d,
        "rig.toml",
        &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n\n[dependencies]\n"),
    );
    write(&d, "src/main.gt", "ƒ main() → () ! ∅ { }\n");
    write(&d, "src/lib1.gt", lib);
    let o = rig(&d, &["lock"]);
    assert_eq!(o.status.code(), Some(0), "rig lock: {}", text(&o.stderr));
    d
}

fn rig(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rig"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("chạy rig")
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

#[test]
fn correct_public_functions_pass_and_inputs_violating_the_precondition_are_not_blamed() {
    let d = project(
        "good",
        "◈ ƒ half(x: i32) → i32 ! ∅ ⊳ x >= 0 ⊲ result * 2 <= x { x / 2 }\n\
         ◈ ƒ pick(a: bool, b: u8) → u8 ! ∅ ⊲ result <= b { ⎇ a { b } ⇝ { 0u8 } }\n\
         ◈ ƒ unit_fn() → i32 ! ∅ { 3 }\n",
    );
    let o = rig(&d, &["test", "--auto"]);
    let so = text(&o.stdout);
    assert_eq!(o.status.code(), Some(0), "{so}\n{}", text(&o.stderr));
    for f in ["lib1::half", "lib1::pick", "lib1::unit_fn"] {
        assert!(
            so.contains(&format!("auto {f} ... ok")),
            "thiếu `{f}` ok:\n{so}"
        );
    }
    // `half` có `⊳ x >= 0`: các ứng viên âm phải bị LOẠI (không phải lỗi), và phải còn ứng viên được chạy
    let line = so
        .lines()
        .find(|l| l.contains("lib1::half"))
        .expect("dòng half");
    assert!(
        line.contains("vì ⊳ sai") && !line.contains("chạy 0,") && !line.contains("bỏ 0 "),
        "half: phải loại ứng viên âm và vẫn chạy ứng viên khác: {line}"
    );
}

#[test]
fn a_wrong_postcondition_is_reported_with_a_counterexample() {
    let d = project(
        "post",
        "◈ ƒ bad(x: i32) → i32 ! ∅ ⊳ x >= 0 ⊲ result > x { x / 2 + 1 }\n\
         ◈ ƒ ok(x: i32) → i32 ! ∅ ⊳ x >= 0 ⊲ result <= x { x / 2 }\n",
    );
    let o = rig(&d, &["test", "--auto"]);
    let so = text(&o.stdout);
    assert_eq!(o.status.code(), Some(1), "{so}\n{}", text(&o.stderr));
    assert!(so.contains("auto lib1::bad ... FAILED"), "{so}");
    assert!(so.contains("postcondition"), "phải nêu loại abort: {so}");
    assert!(
        so.contains("lib1::bad("),
        "phải nêu phản ví dụ dạng lời gọi: {so}"
    );
    assert!(
        so.contains("auto lib1::ok ... ok"),
        "hàm đúng không được bị báo: {so}"
    );
}

#[test]
fn an_overflow_without_any_contract_is_found() {
    let d = project("ovf", "◈ ƒ dec(x: u8) → u8 ! ∅ { x - 1u8 }\n");
    let o = rig(&d, &["test", "--auto"]);
    let so = text(&o.stdout);
    assert_eq!(o.status.code(), Some(1), "{so}");
    assert!(so.contains("auto lib1::dec ... FAILED"), "{so}");
    assert!(
        so.contains("overflow") && so.contains("lib1::dec(0u8)"),
        "{so}"
    );
}

#[test]
fn a_function_that_breaks_a_callee_precondition_on_valid_input_is_found() {
    // `outer` không có `⊳`, nhưng gọi `inner` (cần x > 0) với `x - 0` ⇒ vi phạm ở x = 0 (hằng, không phải biến nên M7 không từ chối lúc dựng)
    let d = project(
        "callee",
        "ƒ inner(x: i32) → i32 ! ∅ ⊳ x > 0 { x }\n\
         ◈ ƒ outer(x: i32) → i32 ! ∅ ⊳ x >= 0 { ⎇ x > 100 { inner(x) } ⇝ { inner(x) } }\n",
    );
    let o = rig(&d, &["test", "--auto"]);
    let so = text(&o.stdout);
    // `inner(x)` với x = 0 vi phạm `⊳ x > 0` của inner, qua đường M7 không chứng minh được ⇒ kiểm lúc chạy ⇒ abort `precondition`
    assert_eq!(o.status.code(), Some(1), "{so}\n{}", text(&o.stderr));
    assert!(
        so.contains("auto lib1::outer ... FAILED") && so.contains("precondition"),
        "{so}"
    );
}

#[test]
fn functions_the_generator_cannot_drive_are_listed_as_skipped_never_silently_dropped() {
    let d = project(
        "skip",
        "◈ ƒ fl(x: f32) → f32 ! ∅ { x }\n\
         ◈ ƒ gen<T>(x: T) → T ! ∅ { x }\n\
         ƒ private(x: i32) → i32 ! ∅ { x }\n\
         ◈ ƒ fine(x: i32) → i32 ! ∅ { x }\n",
    );
    let o = rig(&d, &["test", "--auto"]);
    let so = text(&o.stdout);
    assert_eq!(o.status.code(), Some(0), "{so}\n{}", text(&o.stderr));
    assert!(so.contains("auto lib1::fine ... ok"), "{so}");
    assert!(
        so.contains("auto lib1::fl ... BỎ QUA"),
        "f32 chưa hỗ trợ phải được nêu: {so}"
    );
    assert!(
        so.contains("auto lib1::gen ... BỎ QUA"),
        "hàm generic phải được nêu: {so}"
    );
    assert!(!so.contains("private"), "hàm riêng tư không phải API: {so}");
}

#[test]
fn a_long_running_function_is_noted_as_slow_not_reported_as_a_counterexample() {
    // `⟳` luôn có biến thể `↓` được M8 chứng minh dừng ⇒ hết nhiên liệu chỉ là chạy lâu, không phải lỗi
    let d = project(
        "slow",
        "◈ ƒ spin(n: u32) → u32 ! ∅ { μi ≔ 0u32; ⟳ i < n ↓ n - i { i ← i + 1u32; } i }\n",
    );
    let o = rig(&d, &["test", "--auto"]);
    let so = text(&o.stdout);
    assert_eq!(o.status.code(), Some(0), "{so}\n{}", text(&o.stderr));
    let line = so
        .lines()
        .find(|l| l.contains("lib1::spin"))
        .expect("dòng spin");
    assert!(line.contains("ok") && line.contains("chậm"), "{line}");
}

#[test]
fn float_and_char_parameters_are_driven_and_an_entry_precondition_gritc_cannot_decide_is_not_blamed()
 {
    // M7 không suy luận số thực nên `⊳ x >= 0.0` KHÔNG bị loại bằng E0901: ứng viên âm sẽ abort `precondition` ngay ở cửa vào — đó không phải lỗi của hàm
    let d = project(
        "flt",
        "◈ ƒ halve(x: f64) → f64 ! ∅ ⊳ x >= 0.0 ⊲ result <= x { x * 0.5 }\n\
         ◈ ƒ wrong(x: f64) → f64 ! ∅ ⊳ x >= 0.0 ⊲ result > x { x * 0.5 }\n\
         ◈ ƒ boom(c: char) → u32 ! ∅ { 100u32 / (c as u32) }\n\
         ◈ ƒ calm(c: char) → u32 ! ∅ { c as u32 }\n",
    );
    let o = rig(&d, &["test", "--auto"]);
    let so = text(&o.stdout);
    assert_eq!(o.status.code(), Some(1), "{so}\n{}", text(&o.stderr));
    let halve = so
        .lines()
        .find(|l| l.contains("lib1::halve"))
        .expect("halve");
    assert!(
        halve.contains("... ok") && halve.contains("mơ hồ"),
        "{halve}"
    );
    assert!(
        so.contains("auto lib1::wrong ... FAILED") && so.contains("lib1::wrong("),
        "{so}"
    );
    assert!(so.contains("postcondition"), "{so}");
    assert!(
        so.contains("auto lib1::boom ... FAILED") && so.contains("lib1::boom('"),
        "{so}"
    );
    assert!(so.contains("auto lib1::calm ... ok"), "{so}");
}

#[test]
fn a_callee_precondition_violated_by_a_function_without_its_own_precondition_is_reported_even_for_floats()
 {
    // `outer` không có `⊳` riêng ⇒ mọi abort `precondition` chắc chắn đến từ `inner`
    let d = project(
        "fcallee",
        "ƒ inner(x: f64) → f64 ! ∅ ⊳ x > 0.0 { x }\n\
         ◈ ƒ outer(x: f64) → f64 ! ∅ { inner(x) }\n",
    );
    let o = rig(&d, &["test", "--auto"]);
    let so = text(&o.stdout);
    assert_eq!(o.status.code(), Some(1), "{so}\n{}", text(&o.stderr));
    assert!(
        so.contains("auto lib1::outer ... FAILED") && so.contains("precondition"),
        "{so}"
    );
}

/// F4 (Opus review): `targets` từng parse cứng theo edition 1. Parser hồi phục theo từng mục nên hàm vẫn được thấy (kiểm: test này cũng đạt trước sửa) — đây là test khói cho đường edition 2, không phải tái hiện lỗi.
#[test]
fn an_edition_2_package_is_parsed_with_its_edition_and_its_functions_are_tested() {
    let d = project(
        "ed2",
        "◈ ƒ nxt(x: u8) → Option<u8> ! ∅ { y ≔ x.checked_add(1u8)?; Some(y) }\n",
    );
    let toml = std::fs::read_to_string(d.join("rig.toml")).expect("đọc");
    write(
        &d,
        "rig.toml",
        &toml.replace("version = \"0.1.0\"", "version = \"0.1.0\"\nedition = 2"),
    );
    let l = rig(&d, &["lock"]);
    assert_eq!(l.status.code(), Some(0), "{}", text(&l.stderr));
    let o = rig(&d, &["test", "--auto"]);
    let so = text(&o.stdout);
    assert!(
        so.contains("auto lib1::nxt"),
        "hàm edition 2 phải được sinh test, không bị bỏ im lặng:\n{so}\n{}",
        text(&o.stderr)
    );
}
