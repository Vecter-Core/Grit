//! GÓI 13 B1 — đường bảo thủ ÉP BUỘC (chốt toàn cục ⇒ tiến trình riêng): hàm `x` bị dùng như giá trị và gọi một intrinsic KHÔNG thân với đối số KHÔNG phải
//! hàm (`write_str`). Đột biến `dyn_arg && takes_fn_param` → `||`/`true` thêm cạnh `x → mọi hàm bị thoát` (gồm chính `x`) ⇒ E0703 oan. Mã gốc PHẢI xanh.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::{check_termination, set_force_conservative_for_test};

fn codes(src: &str) -> Vec<String> {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}
{src}",
        common::mir_codes(&m)
    );
    let info = build_prog_info(&t, &m);
    let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
    let mut v: Vec<String> = o
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    v.sort();
    v
}

const PROG: &str = "ƒ x(out: &!Cap<Out>) ! {io} { out.write_str(\"a\"); }
ƒ main() ! {io} { μout ≔ root_out(); g ≔ x; g(&!out); }
";

#[test]
fn forced_conservative_graph_does_not_reject_an_escaped_fn_calling_a_plain_intrinsic() {
    set_force_conservative_for_test(true);
    let c = codes(PROG);
    set_force_conservative_for_test(false);
    assert!(c.is_empty(), "{c:?}");
}
