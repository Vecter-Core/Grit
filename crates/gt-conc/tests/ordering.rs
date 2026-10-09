//! Memory ordering của atomics (E0503; ADR-027 §4) — kiểm ở MIR nên mọi cú pháp gọi cùng đi qua một chỗ.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_conc::check_orderings;
use gt_diag::Diagnostics;

fn codes(src: &str) -> Vec<String> {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}
{src}",
        common::mir_codes(&m)
    );
    let mut bag = Diagnostics::new(200);
    check_orderings(&t, &m, &mut bag);
    let mut v: Vec<String> = bag
        .finish()
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    v.sort();
    v
}

fn ok(src: &str) {
    let c = codes(src);
    assert!(
        c.is_empty(),
        "kỳ vọng sạch, có {c:?}
{src}"
    );
}

fn only(src: &str, code: &str) {
    let c = codes(src);
    assert!(
        !c.is_empty() && c.iter().all(|x| x == code),
        "kỳ vọng chỉ {code}, có {c:?}
{src}"
    );
}

#[test]
fn atomic_orderings_must_be_literal_and_valid() {
    // hợp lệ
    ok("ƒ f(a: &Atomic<u32>) → u32 ! ∅ { a.load(Ordering::Acquire) }");
    ok("ƒ f(a: &Atomic<u32>) ! ∅ { a.store(1u32, Ordering::Release); }");
    ok("ƒ f(a: &Atomic<u32>) → u32 ! ∅ { a.fetch_add(1u32, Ordering::AcqRel) }");
    ok(
        "ƒ f(a: &Atomic<u32>) → Result<u32, u32> ! ∅ { a.compare_exchange(1u32, 2u32, Ordering::AcqRel, Ordering::Acquire) }",
    );
    ok("ƒ f(a: &Atomic<u32>) → u32 ! ∅ { o ≔ Ordering::SeqCst; a.load(o) }");
    // load với Release/AcqRel
    only(
        "ƒ f(a: &Atomic<u32>) → u32 ! ∅ { a.load(Ordering::Release) }",
        "E0503",
    );
    only(
        "ƒ f(a: &Atomic<u32>) → u32 ! ∅ { a.load(Ordering::AcqRel) }",
        "E0503",
    );
    // store với Acquire/AcqRel
    only(
        "ƒ f(a: &Atomic<u32>) ! ∅ { a.store(1u32, Ordering::Acquire); }",
        "E0503",
    );
    only(
        "ƒ f(a: &Atomic<u32>) ! ∅ { a.store(1u32, Ordering::AcqRel); }",
        "E0503",
    );
    // compare_exchange: thất bại Release/AcqRel, SeqCst khi thành công yếu hơn, Acquire khi thành công Relaxed/Release
    only(
        "ƒ f(a: &Atomic<u32>) → Result<u32, u32> ! ∅ { a.compare_exchange(1u32, 2u32, Ordering::SeqCst, Ordering::Release) }",
        "E0503",
    );
    only(
        "ƒ f(a: &Atomic<u32>) → Result<u32, u32> ! ∅ { a.compare_exchange(1u32, 2u32, Ordering::AcqRel, Ordering::SeqCst) }",
        "E0503",
    );
    only(
        "ƒ f(a: &Atomic<u32>) → Result<u32, u32> ! ∅ { a.compare_exchange(1u32, 2u32, Ordering::Relaxed, Ordering::Acquire) }",
        "E0503",
    );
    // ordering không viết thẳng (tính toán / tham số) ⇒ không đoán
    only(
        "ƒ f(a: &Atomic<u32>, o: Ordering) → u32 ! ∅ { a.load(o) }",
        "E0503",
    );
    only(
        "ƒ pick(c: bool) → Ordering ! ∅ { ⎇ c { Ordering::Acquire } ⇝ { Ordering::SeqCst } }\nƒ f(a: &Atomic<u32>, c: bool) → u32 ! ∅ { a.load(pick(c)) }",
        "E0503",
    );
    // cú pháp kết hợp cũng đi qua cùng kiểm tra
    only(
        "ƒ f(a: &Atomic<u32>) → u32 ! ∅ { Atomic::load(a, Ordering::Release) }",
        "E0503",
    );
}

#[test]
fn ordering_tracing_is_exact_and_scans_every_call() {
    // biến gán hai lần: không truy được một biến thể duy nhất ⇒ E0503 (dù giá trị cuối hợp lệ)
    only(
        "ƒ f(a: &Atomic<u32>) → u32 ! ∅ { μo ≔ Ordering::Release; o ← Ordering::Acquire; a.load(o) }",
        "E0503",
    );
    // chuỗi sao chép vẫn truy được
    ok("ƒ f(a: &Atomic<u32>) → u32 ! ∅ { o ≔ Ordering::Acquire; p ≔ o; q ≔ p; a.load(q) }");
    // lời gọi hỏng nằm SAU các khối/lời gọi khác (không dừng quét sớm)
    only(
        "ƒ g() ! ∅ { }\nƒ f(a: &Atomic<u32>, c: bool) → u32 ! ∅ { g(); ⎇ c { g(); } g(); a.load(Ordering::Release) }",
        "E0503",
    );
    // impl khác (của người dùng) đứng cạnh: vẫn tìm đúng impl của Atomic
    only(
        "▢ S { v: u32 }\n⊐ S { ƒ m(&self) ! ∅ { } ƒ n(&self) ! ∅ { } }\nƒ f(a: &Atomic<u32>, s: &S) ! ∅ { s.m(); a.store(1u32, Ordering::Acquire); }",
        "E0503",
    );
    // hàm của người dùng nhận Ordering KHÔNG phải atomic: không bị kiểm
    ok(
        "▢ S { v: u32 }\n⊐ S { ƒ m(&self, o: Ordering) ! ∅ { } }\nƒ f(s: &S, o: Ordering) ! ∅ { s.m(o); }",
    );
    // đúng MỘT chẩn đoán cho mỗi lời gọi sai
    assert_eq!(
        codes("ƒ f(a: &Atomic<u32>, o: Ordering) → u32 ! ∅ { a.load(o) }"),
        vec!["E0503"]
    );
    assert_eq!(
        codes("ƒ f(a: &Atomic<u32>) → u32 ! ∅ { a.load(Ordering::Release) }"),
        vec!["E0503"]
    );
}
