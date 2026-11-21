//! GÓI 14 B1 — thông dịch (`gritc run --backend=interpreter`) ĐỌC được `□`/`■` thay vì lỗi nội bộ (ICE, mã 2).
//! Trước GÓI 14: MỌI chương trình đọc một hằng/static (kể cả literal) ⇒ "hằng ngoài phạm vi thông dịch" (đo GÓI 13, hai nền).
//! Hợp đồng sau sửa:
//!  - đọc `□`/`■` cho đúng giá trị (initializer literal, qua hàm, tham chiếu hằng khác, mảng/tuple); giá trị được GHI NHỚ (tính một lần);
//!  - vòng khởi tạo `■` (qua hàm) ⇒ thông dịch báo "chưa hỗ trợ/vòng khởi tạo", mã thoát 3 (cùng loại với LLVM `build`), KHÔNG ICE (mã 2);
//!  - vòng `□` vẫn do `check` chặn (E0703, FB-C36): không tới thông dịch.
use std::process::Command;

fn gritc(args: &[&str], src: &str, tag: &str) -> (i32, String) {
    let d = std::env::temp_dir().join(format!("grit_cri_{tag}_{}", std::process::id()));
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

fn interp_returns(src: &str, tag: &str, want: &str) {
    let (c, t) = gritc(&["run", "--backend=interpreter"], src, tag);
    assert_eq!(c, 0, "thông dịch phải chạy được: {t}\n{src}");
    assert!(
        t.contains(&format!("main trả về {want}")),
        "kỳ vọng main trả về {want}: {t}\n{src}"
    );
}

#[test]
fn reads_a_literal_const_and_a_literal_static() {
    interp_returns(
        "□ K: i32 ≔ 7i32;\nƒ main() → i32 ! ∅ { K + 1i32 }\n",
        "k1",
        "8",
    );
    interp_returns(
        "■ S: i32 ≔ 9i32;\nƒ main() → i32 ! ∅ { S * 2i32 }\n",
        "s1",
        "18",
    );
}

#[test]
fn reads_consts_and_statics_whose_initialiser_calls_a_function_or_reads_another_item() {
    interp_returns(
        "ƒ f(x: i32) → i32 ! ∅ { x + 5i32 }\n□ C: i32 ≔ f(10i32);\nƒ main() → i32 ! ∅ { C }\n",
        "f1",
        "15",
    );
    interp_returns(
        "□ A: i32 ≔ 4i32;\n□ B: i32 ≔ A + 3i32;\n■ S: i32 ≔ B * 2i32;\nƒ main() → i32 ! ∅ { S + A }\n",
        "chain",
        "18",
    );
    interp_returns(
        "ƒ g() → i32 ! ∅ { Y + 1i32 }\n■ Y: i32 ≔ 41i32;\n■ X: i32 ≔ g();\nƒ main() → i32 ! ∅ { X }\n",
        "static_via_fn",
        "42",
    );
}

#[test]
fn const_is_read_the_same_every_time_and_inside_loops() {
    interp_returns(
        "□ N: u32 ≔ 5u32;\nƒ main() → u32 ! ∅ { μs ≔ 0u32; μi ≔ 0u32; ⟳ i < 5u32 ↓ 5u32 - i { s ← s + N; i ← i + 1u32; } s }\n",
        "loop",
        "25",
    );
}

#[test]
fn static_initialisation_cycle_is_refused_with_exit_3_not_an_ice() {
    let cyc = [
        "ƒ f() → u32 ! ∅ { X }\n■ X: u32 ≔ f();\nƒ main() → i32 ! ∅ { r ≔ X; 0 }\n",
        "■ A: u32 ≔ B;\n■ B: u32 ≔ A;\nƒ main() → i32 ! ∅ { r ≔ A; 0 }\n",
    ];
    for (i, s) in cyc.iter().enumerate() {
        let (c, t) = gritc(&["run", "--backend=interpreter"], s, &format!("cyc{i}"));
        assert_eq!(c, 3, "vòng static ở thông dịch phải thoát mã 3: {t}");
        assert!(t.contains("vòng"), "{t}");
        assert!(
            !t.contains("lỗi nội bộ"),
            "KHÔNG được báo lỗi nội bộ (ICE): {t}"
        );
    }
}

#[test]
fn const_cycles_still_never_reach_the_interpreter() {
    let (c, t) = gritc(
        &["run", "--backend=interpreter"],
        "□ C: u32 ≔ C;\nƒ main() → i32 ! ∅ { r ≔ C; 0 }\n",
        "cc",
    );
    assert_eq!(c, 1, "{t}");
    assert!(t.contains("E0703"), "{t}");
}
