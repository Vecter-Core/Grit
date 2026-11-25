//! FB-G2-04 (đọc kiểm Gemini G2-04): oracle `interp` và verifier phải bắt được các lỗi hạ mà trước đây lọt — con trỏ treo vào slot của
//! khung đã trả về, hằng kiểu không nguyên dựng trực tiếp (không qua parser), và đầu vào kiểu khổng lồ không được làm sập (panic) công cụ.
use gt_glir::interp::{Outcome, run_module};

fn module(src: &str) -> gt_glir::Module {
    let m = gt_glir::parse(src).expect("parse");
    assert!(gt_glir::verify(&m).is_empty(), "{:?}", gt_glir::verify(&m));
    m
}

const LEAK: &str = "glir 1
fn @leak() -> ptr {
slot $x: i32
entry:
  store i32 i32 7, $x
  ret ptr $x
}
fn @read() -> i32 {
entry:
  %p = call ptr @leak()
  %v = load i32 %p
  ret i32 %v
}
fn @write() -> void {
entry:
  %p = call ptr @leak()
  store i32 i32 1, %p
  ret void
}
fn @gep() -> ptr {
entry:
  %p = call ptr @leak()
  %q = indexptr i32 %p, i64 0
  ret ptr %q
}
fn @own() -> i32 {
slot $y: i32
entry:
  store i32 i32 5, $y
  %p = call ptr @id($y)
  %v = load i32 %p
  ret i32 %v
}
fn @id(%p: ptr) -> ptr {
entry:
  ret ptr %p
}
";

/// Hạ sai sinh con trỏ tới slot cục bộ sống qua `ret`: đọc/ghi/tính địa chỉ qua nó là `Ub`, kể cả trả nó ra ngoài cùng.
/// Con trỏ tới slot của khung CÒN SỐNG (qua `@id`) vẫn hợp lệ.
#[test]
fn dangling_pointer_into_a_returned_frame_is_ub() {
    let m = module(LEAK);
    for f in ["read", "write", "gep", "leak"] {
        let o = run_module(&m, f, &[], 1000);
        assert!(matches!(o, Outcome::Ub(_)), "@{f}: {o:?}");
    }
    assert_eq!(run_module(&m, "own", &[], 1000), Outcome::Value(Some(5)));
}

/// Hằng `Opd::Int` có kiểu không nguyên chỉ dựng được bằng mã (parser chặn) — verifier vẫn phải từ chối.
#[test]
fn verifier_rejects_non_integer_constants_built_in_code() {
    use gt_glir::{Block, Func, Module, Opd, Term, Ty};
    let m = Module {
        globals: vec![],
        decls: vec![],
        funcs: vec![Func {
            name: "f".into(),
            params: vec![],
            ret: Ty::Ptr,
            slots: vec![],
            blocks: vec![Block {
                name: "entry".into(),
                insts: vec![],
                term: Term::Ret(Some((Ty::Ptr, Opd::Int(Ty::Ptr, 0)))),
            }],
        }],
    };
    let errs = gt_glir::verify(&m);
    assert!(errs.iter().any(|e| e.starts_with("G0308")), "{errs:?}");
}

/// Kiểu khổng lồ: bố cục không panic (bão hoà), `checked_layout` báo tràn, thông dịch không cấp vùng khổng lồ và không panic khi tính
/// `fieldptr` trên struct cỡ bão hoà (đầu vào CHƯA qua verifier).
#[test]
fn huge_types_never_panic_the_layout_or_the_interpreter() {
    use gt_glir::Ty;
    use gt_glir::layout::{checked_layout, field_offset, layout};
    let huge = Ty::Struct(vec![Ty::I8, Ty::Array(Box::new(Ty::I64), u64::MAX), Ty::I8]);
    assert_eq!(layout(&huge).size, u64::MAX);
    assert_eq!(checked_layout(&huge), None);
    let _ = field_offset(&huge, 2);
    let m = gt_glir::parse(
        "glir 1
fn @big() -> i32 {
slot $s: [4611686018427387904 x i64]
entry:
  ret i32 i32 0
}
fn @fp(%p: ptr) -> ptr {
entry:
  %q = fieldptr {i8, [18446744073709551615 x i64], i8} %p, 2
  ret ptr %q
}
fn @main() -> i32 {
slot $x: i64
entry:
  %q = call ptr @fp($x)
  ret i32 i32 0
}
",
    )
    .expect("parse");
    assert!(!gt_glir::verify(&m).is_empty());
    assert_eq!(run_module(&m, "big", &[], 100), Outcome::OutOfFuel);
    assert!(matches!(run_module(&m, "main", &[], 100), Outcome::Ub(_)));
}
