//! Rà soát FAIL-OPEN (lệnh Giang 2026-10-14, mục B): "không biết" KHÔNG được thành "chấp nhận". Các test dưới đây CHÈN vào MIR những nút mà Phase 1 không bao giờ sinh
//! (khối không có terminator; `Mutex::with` thiếu hàm đối số) và đòi `gt-effects` từ chối (E9901), thay vì lặng lẽ bỏ qua như trước.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_effects::check_effects;
use gt_mir::TerminatorKind;

fn codes_after(
    src: &str,
    mutate: impl FnOnce(&mut gt_mir::MirOutput),
) -> (Vec<String>, Vec<String>) {
    let (t, mut m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        common::mir_codes(&m)
    );
    let clean: Vec<String> = check_effects(&t, &m)
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    mutate(&mut m);
    let after: Vec<String> = check_effects(&t, &m)
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    (clean, after)
}

/// Hàm gọi một hàm có hiệu ứng `io` mà KHÔNG khai báo: bình thường E0801; nếu khối chứa lời gọi mất terminator thì trước đây KHÔNG có chẩn đoán nào
/// (hiệu ứng biến mất ⇒ chấp nhận). Nay phải là E9901.
#[test]
fn a_block_without_terminator_is_an_ice_not_silent_acceptance() {
    let src = "ƒ out() → () ! {io} { }\nƒ f() ! ∅ { out(); }\n";
    let (clean, after) = codes_after(src, |m| {
        for b in &mut m.bodies {
            if b.key.ends_with("::f") {
                for bb in &mut b.blocks {
                    bb.terminator = None;
                }
            }
        }
    });
    assert_eq!(
        clean,
        vec!["E0801".to_string()],
        "đối chứng: bình thường phải E0801"
    );
    assert!(
        after.iter().any(|c| c == "E9901"),
        "khối không terminator phải gây E9901, nhận {after:?}"
    );
}

const LV: &str = "#level(\"10\")\n▢ Lo { }\n";

/// `Mutex::with` mà đối số hàm biến mất: không kiểm được thứ tự khoá ⇒ phải từ chối (trước đây `return` lặng lẽ).
#[test]
fn mutex_with_without_a_function_argument_is_rejected() {
    let src = format!(
        "{LV}ƒ bump(v: &!u32, n: u32) → u32 ! ∅ {{ *v ←  *v +% n; *v }}\nƒ go(m: &Mutex<u32, Lo>) → u32 ! {{lock(Lo)}} {{ m.with(bump, 1u32) }}"
    );
    let (clean, after) = codes_after(&src, |m| {
        for b in &mut m.bodies {
            if b.key.ends_with("::go") {
                for bb in &mut b.blocks {
                    if let Some(t) = &mut bb.terminator
                        && let TerminatorKind::Call { args, .. } = &mut t.kind
                    {
                        args.truncate(1);
                    }
                }
            }
        }
    });
    assert!(
        clean.is_empty(),
        "đối chứng: chương trình gốc phải sạch, nhận {clean:?}"
    );
    assert!(
        after.iter().any(|c| c == "E9901"),
        "thiếu hàm đối số của `with` phải gây E9901, nhận {after:?}"
    );
}
