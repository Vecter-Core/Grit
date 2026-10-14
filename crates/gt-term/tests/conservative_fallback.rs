//! Đường DỰ PHÒNG của M8 (ADR-033 phụ lục): khi đồ thị instance không dựng được, M8 lùi về đồ thị theo NÚT bảo thủ (mọi `TraitMethod` nối
//! tới mọi impl). Test này chạy trong tiến trình riêng vì dùng chốt toàn cục (`set_force_conservative_for_test`, `set_graph_limit_for_test`).
//! Đường dự phòng PHẢI không bao giờ chấp nhận thứ mà đường chính từ chối (không FAIL-OPEN), và cận số nút phải chính xác.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::{
    check_termination, conservative_fallbacks, set_force_conservative_for_test,
    set_graph_limit_for_test,
};

fn codes(src: &str) -> Vec<String> {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
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

fn messages(src: &str) -> Vec<String> {
    let (t, m) = common::mir(src);
    let info = build_prog_info(&t, &m);
    let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
    o.report
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

const FWD_OK: &str = "⊏ A { ƒ a(&self) → i32 ! ∅; }\n▢ S { }\n▢ W<T> { i: T }\n⊐ A for S { ƒ a(&self) → i32 ! ∅ { 1 } }\n⊐ <T: A> A for W<T> { ƒ a(&self) → i32 ! ∅ { self.i.a() +% 10 } }\nƒ main() → i32 ! ∅ { x ≔ W { i: S { } }; x.a() }";
const CYCLE: &str = "⊏ A { ƒ a(&self) → i32 ! ∅; }\n▢ X { }\n▢ W<T> { i: T }\n⊐ A for X { ƒ a(&self) → i32 ! ∅ { y ≔ W { i: X { } }; y.a() } }\n⊐ <T: A> A for W<T> { ƒ a(&self) → i32 ! ∅ { self.i.a() } }\nƒ main() → i32 ! ∅ { x ≔ X { }; x.a() }";
const SELF_REC: &str = "ƒ r(n: u32) → u32 ! ∅ { r(n) }";
const MUTUAL: &str = "ƒ p(n: u32) → u32 ! ∅ { q(n) }\nƒ q(n: u32) → u32 ! ∅ { p(n) }";
const NO_REC: &str =
    "ƒ h(n: u32) → u32 ! ∅ { n }\nƒ g(n: u32) → u32 ! ∅ { h(n) }\nƒ main() → u32 ! ∅ { g(1) }";
/// đệ quy qua giá trị hàm (cạnh gián tiếp) có biến thể
const VIA_VALUE: &str = "ƒ f(n: u32) → u32 ! ∅ ↓ n { μg: ƒ(u32) → u32 ! ∅ ≔ f; g(n) }";
/// vòng qua phương thức trait ở cấp NÚT với biến thể (gián tiếp ⇒ phải bị báo)
const VIA_TRAIT: &str = "⊏ T { ƒ m(&self, n: u32) → u32 ! ∅; }\n▢ S { v: i32 }\nƒ r<X: T>(x: &X, n: u32) → u32 ! ∅ ↓ n { x.m(n) }\n⊐ T for S { ƒ m(&self, n: u32) → u32 ! ∅ ↓ n { r(self, n) } }";
/// một hàm không đệ quy gọi trước, có `main` sau (kiểm đường `else continue` của đồ thị bảo thủ khi khối cuối không phải lời gọi)
const NO_CALL_BLOCKS_THEN_REC: &str = "ƒ leaf(n: u32) → u32 ! ∅ { n }\nƒ rr(n: u32) → u32 ! ∅ { ⎇ n == 0 { leaf(0) } ⇝ { rr(n - 1) } }";

/// hàm vừa có cạnh TRỰC TIẾP giảm vừa có cạnh gián tiếp (trait) trong cùng SCC: phải bị báo E0703
const TRAIT_PLUS_DIRECT: &str = "⊏ T { ƒ t(&self, n: u32) → u32 ! ∅; }
▢ S { }
▢ S2 { }
⊐ T for S2 { ƒ t(&self, n: u32) → u32 ! ∅ { n } }
⊐ T for S { ƒ t(&self, n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { x ≔ S { }; f(&x, n - 1) } } }
ƒ f<U: T>(u: &U, n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { u.t(n - 1) +% f(u, n - 1) } }
ƒ main() → u32 ! ∅ { s ≔ S { }; f(&s, 3) }";

#[test]
fn conservative_path_and_graph_limit() {
    // ---- cận số nút: ĐÚNG biên. Chương trình CYCLE có 4 instance (main, X::a, W<X>::a, ... ) — đo số nút thật bằng cách tăng dần cận
    let before = conservative_fallbacks();
    assert_eq!(codes(FWD_OK), Vec::<String>::new());
    assert_eq!(
        conservative_fallbacks(),
        before,
        "đường chính phải dựng được"
    );
    // tìm cận nhỏ nhất `m` để đồ thị instance dựng được: với cận m-1 phải lùi, với m thì không
    let mut m = 0usize;
    for lim in 1..40usize {
        set_graph_limit_for_test(lim);
        let b = conservative_fallbacks();
        let _ = codes(FWD_OK);
        if conservative_fallbacks() == b {
            m = lim;
            break;
        }
    }
    assert!(m > 0, "không tìm thấy cận đủ lớn");
    // biên chính xác: cận m dựng được (đã thấy), cận m-1 lùi về bảo thủ
    set_graph_limit_for_test(m - 1);
    let b = conservative_fallbacks();
    let c = codes(FWD_OK);
    assert_eq!(
        conservative_fallbacks(),
        b + 1,
        "cận m-1 = {} phải lùi",
        m - 1
    );
    // đường dự phòng bảo thủ KHÔNG chấp nhận forwarder (từ chối oan như trước) nhưng cũng không fail-open
    assert!(c.iter().all(|x| x == "E0703"), "{c:?}");
    set_graph_limit_for_test(m);
    let b = conservative_fallbacks();
    assert_eq!(codes(FWD_OK), Vec::<String>::new(), "cận đúng m = {m}");
    assert_eq!(conservative_fallbacks(), b);
    set_graph_limit_for_test(0);

    // ---- đường bảo thủ ép buộc: KHÔNG BAO GIỜ chấp nhận thứ đường chính từ chối
    set_force_conservative_for_test(true);
    let before = conservative_fallbacks();
    for (name, src) in [
        ("vòng thật qua impl generic", CYCLE),
        ("tự đệ quy", SELF_REC),
        ("đệ quy tương hỗ", MUTUAL),
        ("đệ quy qua giá trị hàm", VIA_VALUE),
        ("đệ quy qua phương thức trait", VIA_TRAIT),
        ("đệ quy sau hàm lá", NO_CALL_BLOCKS_THEN_REC),
        (
            "cạnh trait cạnh cạnh trực tiếp trong SCC",
            TRAIT_PLUS_DIRECT,
        ),
    ] {
        let c = codes(src);
        // đệ quy gián tiếp có biến thể có thể kèm E0702 (biến thể không giảm); mọi trường hợp PHẢI có E0703
        assert!(
            c.contains(&"E0703".to_string()) && c.iter().all(|x| x == "E0703" || x == "E0702"),
            "{name}: đường bảo thủ phải từ chối bằng E0703, có {c:?}"
        );
    }
    // đúng MỘT chẩn đoán và nó nêu `f` (hàm có cạnh gián tiếp trong SCC), không phải `t` (chỉ có cạnh trực tiếp)
    let m = messages(TRAIT_PLUS_DIRECT);
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(m[0].contains("main::f`"), "{m:?}");
    // forwarder: đường bảo thủ từ chối oan (hành vi cũ, ghi nhận), nhưng KHÔNG ICE
    let c = codes(FWD_OK);
    assert!(c.iter().all(|x| x == "E0703"), "{c:?}");
    // chương trình không đệ quy vẫn sạch ở đường bảo thủ
    assert_eq!(codes(NO_REC), Vec::<String>::new());
    assert!(
        conservative_fallbacks() >= before + 8,
        "mỗi lần ép phải đếm một lần lùi"
    );
    set_force_conservative_for_test(false);
}
