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
    ok("ƒ f(a: &Atomic<u32>) → u32 ! ∅ { a.load(AtomicOrdering::Acquire) }");
    ok("ƒ f(a: &Atomic<u32>) ! ∅ { a.store(1u32, AtomicOrdering::Release); }");
    ok("ƒ f(a: &Atomic<u32>) → u32 ! ∅ { a.fetch_add(1u32, AtomicOrdering::AcqRel) }");
    ok(
        "ƒ f(a: &Atomic<u32>) → Result<u32, u32> ! ∅ { a.compare_exchange(1u32, 2u32, AtomicOrdering::AcqRel, AtomicOrdering::Acquire) }",
    );
    ok("ƒ f(a: &Atomic<u32>) → u32 ! ∅ { o ≔ AtomicOrdering::SeqCst; a.load(o) }");
    // load với Release/AcqRel
    only(
        "ƒ f(a: &Atomic<u32>) → u32 ! ∅ { a.load(AtomicOrdering::Release) }",
        "E0503",
    );
    only(
        "ƒ f(a: &Atomic<u32>) → u32 ! ∅ { a.load(AtomicOrdering::AcqRel) }",
        "E0503",
    );
    // store với Acquire/AcqRel
    only(
        "ƒ f(a: &Atomic<u32>) ! ∅ { a.store(1u32, AtomicOrdering::Acquire); }",
        "E0503",
    );
    only(
        "ƒ f(a: &Atomic<u32>) ! ∅ { a.store(1u32, AtomicOrdering::AcqRel); }",
        "E0503",
    );
    // compare_exchange: thất bại Release/AcqRel, SeqCst khi thành công yếu hơn, Acquire khi thành công Relaxed/Release
    only(
        "ƒ f(a: &Atomic<u32>) → Result<u32, u32> ! ∅ { a.compare_exchange(1u32, 2u32, AtomicOrdering::SeqCst, AtomicOrdering::Release) }",
        "E0503",
    );
    only(
        "ƒ f(a: &Atomic<u32>) → Result<u32, u32> ! ∅ { a.compare_exchange(1u32, 2u32, AtomicOrdering::AcqRel, AtomicOrdering::SeqCst) }",
        "E0503",
    );
    only(
        "ƒ f(a: &Atomic<u32>) → Result<u32, u32> ! ∅ { a.compare_exchange(1u32, 2u32, AtomicOrdering::Relaxed, AtomicOrdering::Acquire) }",
        "E0503",
    );
    // ordering không viết thẳng (tính toán / tham số) ⇒ không đoán
    only(
        "ƒ f(a: &Atomic<u32>, o: AtomicOrdering) → u32 ! ∅ { a.load(o) }",
        "E0503",
    );
    only(
        "ƒ pick(c: bool) → AtomicOrdering ! ∅ { ⎇ c { AtomicOrdering::Acquire } ⇝ { AtomicOrdering::SeqCst } }\nƒ f(a: &Atomic<u32>, c: bool) → u32 ! ∅ { a.load(pick(c)) }",
        "E0503",
    );
    // cú pháp kết hợp cũng đi qua cùng kiểm tra
    only(
        "ƒ f(a: &Atomic<u32>) → u32 ! ∅ { Atomic::load(a, AtomicOrdering::Release) }",
        "E0503",
    );
}

#[test]
fn ordering_tracing_is_exact_and_scans_every_call() {
    // biến gán hai lần: không truy được một biến thể duy nhất ⇒ E0503 (dù giá trị cuối hợp lệ)
    only(
        "ƒ f(a: &Atomic<u32>) → u32 ! ∅ { μo ≔ AtomicOrdering::Release; o ← AtomicOrdering::Acquire; a.load(o) }",
        "E0503",
    );
    // chuỗi sao chép vẫn truy được
    ok("ƒ f(a: &Atomic<u32>) → u32 ! ∅ { o ≔ AtomicOrdering::Acquire; p ≔ o; q ≔ p; a.load(q) }");
    // lời gọi hỏng nằm SAU các khối/lời gọi khác (không dừng quét sớm)
    only(
        "ƒ g() ! ∅ { }\nƒ f(a: &Atomic<u32>, c: bool) → u32 ! ∅ { g(); ⎇ c { g(); } g(); a.load(AtomicOrdering::Release) }",
        "E0503",
    );
    // impl khác (của người dùng) đứng cạnh: vẫn tìm đúng impl của Atomic
    only(
        "▢ S { v: u32 }\n⊐ S { ƒ m(&self) ! ∅ { } ƒ n(&self) ! ∅ { } }\nƒ f(a: &Atomic<u32>, s: &S) ! ∅ { s.m(); a.store(1u32, AtomicOrdering::Acquire); }",
        "E0503",
    );
    // hàm của người dùng nhận AtomicOrdering KHÔNG phải atomic: không bị kiểm
    ok(
        "▢ S { v: u32 }\n⊐ S { ƒ m(&self, o: AtomicOrdering) ! ∅ { } }\nƒ f(s: &S, o: AtomicOrdering) ! ∅ { s.m(o); }",
    );
    // đúng MỘT chẩn đoán cho mỗi lời gọi sai
    assert_eq!(
        codes("ƒ f(a: &Atomic<u32>, o: AtomicOrdering) → u32 ! ∅ { a.load(o) }"),
        vec!["E0503"]
    );
    assert_eq!(
        codes("ƒ f(a: &Atomic<u32>) → u32 ! ∅ { a.load(AtomicOrdering::Release) }"),
        vec!["E0503"]
    );
}

/// FAIL_OPEN_AUDIT S-E (lỗi THẬT, đạt tới được từ Grit): `Atomic::load` dùng làm giá trị hàm rồi gọi qua biến (`Callee::Value`) lách kiểm ordering (check xanh dù
/// `load` với `Release`; backend LLVM từ chối "chưa hỗ trợ", thông dịch chạy). Quy tắc: phương thức của `Atomic` KHÔNG được dùng làm giá trị hàm (chỉ gọi trực tiếp).
#[test]
fn atomic_methods_used_as_function_values_are_rejected() {
    // gọi qua biến với ordering SAI
    only(
        "ƒ main() → u32 ! ∅ {
  a ≔ Atomic::new(5u32);
  f: ƒ(&Atomic<u32>, AtomicOrdering) → u32 ! ∅ ≔ Atomic::load;
  f(&a, AtomicOrdering::Release)
}
",
        "E0503",
    );
    // gọi qua biến với ordering ĐÚNG: vẫn bị từ chối (không kiểm được ở chỗ gọi gián tiếp)
    only(
        "ƒ main() → u32 ! ∅ {
  a ≔ Atomic::new(5u32);
  f: ƒ(&Atomic<u32>, AtomicOrdering) → u32 ! ∅ ≔ Atomic::load;
  f(&a, AtomicOrdering::Acquire)
}
",
        "E0503",
    );
    // chỉ lưu giá trị, không gọi
    only(
        "ƒ main() → u32 ! ∅ {
  f: ƒ(&Atomic<u32>, AtomicOrdering) → u32 ! ∅ ≔ Atomic::load;
  0
}
",
        "E0503",
    );
    // đối chứng: gọi TRỰC TIẾP với ordering đúng vẫn sạch
    ok("ƒ main() → u32 ! ∅ {
  a ≔ Atomic::new(5u32);
  Atomic::load(&a, AtomicOrdering::Acquire)
}
");
}

/// Chính xác của luật S-E (diệt đột biến `is_atomic_member`): hàm THƯỜNG và phương thức của kiểu KHÁC dùng làm giá trị hàm KHÔNG bị E0503; phương thức Atomic nằm trong
/// khởi tạo struct cũng bị bắt.
#[test]
fn only_atomic_methods_are_rejected_as_function_values() {
    ok("ƒ inc(x: u32) → u32 ! ∅ { x +% 1 }
ƒ main() → u32 ! ∅ {
  a ≔ Atomic::new(5u32);
  f ≔ inc;
  f(a.load(AtomicOrdering::Acquire))
}
");
    ok("▢ B { v: u32 }
⊐ B {
  ƒ get(&self) → u32 ! ∅ { self.v }
}
ƒ main() → u32 ! ∅ {
  a ≔ Atomic::new(5u32);
  b ≔ B { v: 3u32 };
  g: ƒ(&B) → u32 ! ∅ ≔ B::get;
  g(&b) +% a.load(AtomicOrdering::Acquire)
}
");
    // phương thức Atomic nằm trực tiếp trong khởi tạo struct (toán hạng Aggregate)
    only(
        "▢ H { f: ƒ(u32) → Atomic<u32> ! ∅ }
ƒ main() → u32 ! ∅ {
  h ≔ H { f: Atomic::new };
  0
}
",
        "E0503",
    );
}
