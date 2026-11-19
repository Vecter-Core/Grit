//! GÓI 7 FB-C33 (nghiêm trọng, D12): đệ quy QUA `spawn` (hàm worker được truyền làm đối số của intrinsic không có thân) không được xét như cạnh gọi, nên
//! `ƒ f(n) ↓ n { spawn(f, n); }` (biến thể không giảm) và vòng tương hỗ qua `spawn` không `↓`/`div` từng được chấp nhận (mỗi lượt để lại một luồng: không dừng).
//! Phát hiện bởi bộ tấn công `attack/` (phiên kiểm toán); tái hiện độc lập ở GÓI 7. Test này ĐỎ trước bản sửa: các chương trình phải bị từ chối (E0703/E0702).
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::check_termination;

fn codes(src: &str) -> Vec<String> {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        common::mir_codes(&m)
    );
    let info = build_prog_info(&t, &m);
    let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
    o.report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect()
}

fn rejected(src: &str) {
    let c = codes(src);
    assert!(
        c.iter().any(|x| x == "E0703" || x == "E0702"),
        "đệ quy qua spawn được chấp nhận (FB-C33): {c:?}\n{src}"
    );
}

#[test]
fn self_recursion_through_spawn_without_variant_is_rejected() {
    rejected("ƒ f(n: u64) → () ! ∅ { spawn(f, n); }\nƒ main() → i32 ! ∅ { f(1u64); 0 }\n");
}

#[test]
fn self_recursion_through_spawn_with_non_decreasing_variant_is_rejected() {
    rejected("ƒ f(n: u64) → () ! ∅ ↓ n { spawn(f, n); }\nƒ main() → i32 ! ∅ { f(1u64); 0 }\n");
}

#[test]
fn mutual_recursion_through_spawn_is_rejected() {
    rejected(
        "ƒ a(n: u64) → () ! ∅ { spawn(b, n); }\nƒ b(n: u64) → () ! ∅ { a(n); }\nƒ main() → i32 ! ∅ { a(1u64); 0 }\n",
    );
}

#[test]
fn spawning_a_non_recursive_worker_is_still_accepted() {
    // đối chứng: spawn hàm lá không đệ quy phải còn được chấp nhận (không từ chối quá tay)
    let c = codes(
        "ƒ w(n: u64) → () ! ∅ { }\nƒ run() → () ! ∅ { spawn(w, 1u64); }\nƒ main() → i32 ! ∅ { run(); 0 }\n",
    );
    assert!(c.is_empty(), "{c:?}");
}

/// GÓI 8 S1b: bộ sinh nhỏ cho đệ quy qua `spawn`: nhiều hình dạng (trực tiếp, tương hỗ 2–3 hàm, qua hàm trung gian, qua generic),
/// mỗi hình dạng có/không `↓` không giảm. Mọi chương trình KHÔNG có `div` phải bị từ chối. Bộ này bắt được FB-C33 khi tắt bản sửa d220202 (kiểm tay).
fn spawn_family(shape: usize, depth: usize) -> String {
    let mut s = String::new();
    match shape {
        0 => s.push_str("ƒ f(n: u64) → () ! ∅ { spawn(f, n); }\n"),
        1 => s.push_str("ƒ a(n: u64) → () ! ∅ { spawn(b, n); }\nƒ b(n: u64) → () ! ∅ { a(n); }\n"),
        2 => s.push_str("ƒ f(n: u64) → () ! ∅ ↓ n { spawn(f, n); }\n"),
        3 => s.push_str("ƒ id<T>(x: T) → T ! ∅ { x }\nƒ f(n: u64) → () ! ∅ { spawn(id(f), n); }\n"),
        _ => s.push_str("ƒ w(n: u64) → () ! ∅ { w2(n); }\nƒ w2(n: u64) → () ! ∅ { spawn(w, n); }\n"),
    }
    // chuỗi hàm trung gian không đệ quy trước điểm vào (độ sâu tuỳ ý, không đổi chu trình)
    for k in 0..depth {
        s.push_str(&format!("ƒ pad{k}(n: u64) → () ! ∅ {{ }}\n"));
    }
    let entry = match shape {
        1 => "a",
        4 => "w",
        _ => "f",
    };
    s.push_str(&format!("ƒ main() → i32 ! ∅ {{ {entry}(1u64); 0 }}\n"));
    s
}

#[test]
fn spawn_family_recursion_without_variant_is_rejected() {
    let mut accepted = Vec::new();
    for shape in 0..5 {
        for depth in 0..4 {
            let src = spawn_family(shape, depth);
            let c = codes(&src);
            if c.is_empty() {
                accepted.push((shape, depth));
            }
            if shape != 2 && shape != 3 && shape != 4 {
                // hình dạng 0,1: đệ quy không `↓` ⇒ phải từ chối
                assert!(c.iter().any(|x| x == "E0703" || x == "E0702"), "shape {shape} depth {depth} được chấp nhận: {src}");
            }
        }
    }
    // hình dạng 2 (↓ không giảm) và 3/4 (qua generic/trung gian) cũng phải bị từ chối; chấp nhận ở đây là FB-C nghiêm trọng
    assert!(accepted.is_empty(), "các hình dạng spawn-đệ quy được chấp nhận: {accepted:?}");
}
