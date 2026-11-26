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

/// Đột biến tự động (`tools/mut.py`, Vec 2026-11-26) sống: hạ `Vec` cấp khối với căn lề 1 cho phần tử `i32`. `load`/`store` LỆCH căn lề là UB của
/// LLVM (codegen gắn căn lề ABI) ⇒ thông dịch GLIR phải báo `Ub`: vùng cấp với căn lề nhỏ hơn kiểu, hoặc offset không là bội căn lề của kiểu.
#[test]
fn misaligned_load_or_store_is_ub() {
    let m = module(
        "glir 1
declare @gritrt_arena_new() -> i32
declare @gritrt_arena_alloc(i32, i64, i64) -> ptr
fn @f(%al: i64, %off: i64) -> i32 {
entry:
  %h = call i32 @gritrt_arena_new()
  %p = call ptr @gritrt_arena_alloc(%h, i64 16, %al)
  %q = indexptr i8 %p, %off
  store i32 i32 7, %q
  %v = load i32 %q
  ret i32 %v
}
fn @s() -> i32 {
slot $a: [2 x i32]
entry:
  %q = indexptr i8 $a, i64 2
  %v = load i16 %q
  %w = cast zext i16 %v to i32
  ret i32 %w
}
",
    );
    assert_eq!(run_module(&m, "f", &[4, 4], 1000), Outcome::Value(Some(7)));
    assert_eq!(run_module(&m, "f", &[8, 0], 1000), Outcome::Value(Some(7)));
    for (al, off) in [(1, 0), (2, 0), (4, 2), (8, 1)] {
        let o = run_module(&m, "f", &[al, off], 1000);
        assert!(matches!(o, Outcome::Ub(_)), "căn {al}, offset {off}: {o:?}");
    }
    // slot `[2 x i32]` căn 4: đọc i16 tại offset 2 (căn đúng cho i16) chỉ lỗi vì byte chưa ghi — không phải vì căn lề
    let o = run_module(&m, "s", &[], 1000);
    assert!(
        matches!(&o, Outcome::Ub(u) if !u.contains("căn lề")),
        "{o:?}"
    );
}

/// Đột biến tự động (`tools/mut.py`, mô hình runtime của thông dịch GLIR, 2026-11-26, `docs/MUT_GLIR_TOS.jsonl`) sống: các chốt `Ub` của mô hình
/// `gritrt_*` chưa từng bị chạm (bước hạ đúng không gây ra chúng) ⇒ đột biến bỏ chốt vẫn qua. Mỗi chốt là tuyến phòng thủ bắt bước hạ SAI, nên phải có
/// ca âm: GLIR viết tay (không qua verifier khi cố ý sai số đối số) gọi runtime sai cách ⇒ `Ub` (không panic), và biên hợp lệ ⇒ chạy được.
const RT: &str = "glir 1
global @g = 41
declare @gritrt_arena_new() -> i32
declare @gritrt_arena_alloc(i32, i64, i64) -> ptr
declare @gritrt_arena_reset(i32) -> void
declare @gritrt_utf8_valid(ptr, i64) -> i32
declare @gritrt_heap_alloc(i64, i64) -> ptr
declare @gritrt_heap_free(ptr, i64, i64) -> void
declare @gritrt_secret_wipe(ptr, i64) -> void
fn @utf8null() -> i32 {
entry:
  %r = call i32 @gritrt_utf8_valid(null, i64 0)
  ret i32 %r
}
fn @utf8nullread() -> i32 {
entry:
  %r = call i32 @gritrt_utf8_valid(null, i64 1)
  ret i32 %r
}
fn @utf8reset() -> i32 {
entry:
  %h = call i32 @gritrt_arena_new()
  %p = call ptr @gritrt_arena_alloc(%h, i64 4, i64 1)
  store i8 i8 97, %p
  call void @gritrt_arena_reset(%h)
  %r = call i32 @gritrt_utf8_valid(%p, i64 1)
  ret i32 %r
}
fn @free(%off: i64, %size: i64, %align: i64, %twice: i1) -> i32 {
entry:
  %p = call ptr @gritrt_heap_alloc(i64 8, i64 8)
  %q = indexptr i8 %p, %off
  call void @gritrt_heap_free(%q, %size, %align)
  condbr %twice, again, done
again:
  call void @gritrt_heap_free(%q, %size, %align)
  ret i32 i32 2
done:
  ret i32 i32 1
}
fn @wipeslot() -> i32 {
slot $s: i32
entry:
  store i32 i32 7, $s
  call void @gritrt_secret_wipe($s, i64 4)
  %v = load i32 $s
  ret i32 %v
}
fn @wipeglobal() -> i32 {
entry:
  call void @gritrt_secret_wipe(@g, i64 1)
  ret i32 i32 0
}
fn @wipenull() -> i32 {
entry:
  call void @gritrt_secret_wipe(null, i64 0)
  ret i32 i32 0
}
fn @wipefreed() -> i32 {
entry:
  %p = call ptr @gritrt_heap_alloc(i64 4, i64 4)
  call void @gritrt_heap_free(%p, i64 4, i64 4)
  call void @gritrt_secret_wipe(%p, i64 4)
  ret i32 i32 0
}
fn @arena(%size: i64, %align: i64) -> i32 {
entry:
  %h = call i32 @gritrt_arena_new()
  %p = call ptr @gritrt_arena_alloc(%h, %size, %align)
  ret i32 i32 1
}
fn @heap(%size: i64, %align: i64) -> i32 {
entry:
  %p = call ptr @gritrt_heap_alloc(%size, %align)
  call void @gritrt_heap_free(%p, %size, %align)
  ret i32 i32 1
}
fn @nohandle() -> i32 {
entry:
  %p = call ptr @gritrt_arena_alloc(i32 0, i64 4, i64 4)
  ret i32 i32 1
}
fn @arity() -> i32 {
entry:
  %h = call i32 @gritrt_arena_new()
  %p = call ptr @gritrt_arena_alloc(%h, i64 4)
  ret i32 i32 1
}
";

#[test]
fn runtime_models_reject_misuse_and_accept_their_boundaries() {
    use gt_glir::interp::MAX_SLOT_BYTES;
    // `@arity` gọi sai số đối số (verifier từ chối) ⇒ dựng không qua `module()`
    let m = gt_glir::parse(RT).expect("parse");
    let errs = gt_glir::verify(&m);
    assert!(
        errs.len() == 1 && errs[0].contains("@arity"),
        "chỉ `@arity` được sai: {errs:?}"
    );
    let run = |f: &str, a: &[u128]| run_module(&m, f, a, 1000);
    let ub = |f: &str, a: &[u128]| {
        let o = run(f, a);
        assert!(matches!(o, Outcome::Ub(_)), "@{f}{a:?}: {o:?}");
    };
    // utf8_valid: n = 0 không đọc (kể cả null) ⇒ 1; n > 0 qua null hoặc vùng Arena đã reset ⇒ Ub
    assert_eq!(run("utf8null", &[]), Outcome::Value(Some(1)));
    ub("utf8nullread", &[]);
    ub("utf8reset", &[]);
    // heap_free: đúng (đầu khối, cỡ, căn lề) ⇒ chạy; lệch đầu khối, sai cỡ/căn lề, trả hai lần ⇒ Ub
    assert_eq!(run("free", &[0, 8, 8, 0]), Outcome::Value(Some(1)));
    ub("free", &[4, 8, 8, 0]);
    ub("free", &[0, 4, 8, 0]);
    ub("free", &[0, 8, 4, 0]);
    ub("free", &[0, 8, 8, 1]);
    // secret_wipe: vùng ghi được ĐẦU TIÊN (ngay sau global) ⇒ chạy, ghi 0; global chỉ-đọc, null, khối đã trả ⇒ Ub
    assert_eq!(run("wipeslot", &[]), Outcome::Value(Some(0)));
    ub("wipeglobal", &[]);
    ub("wipenull", &[]);
    ub("wipefreed", &[]);
    // căn lề: luỹ thừa 2 tới 4096 ⇒ chạy; 3, 0, 8192 ⇒ Ub. Cỡ: đúng trần `MAX_SLOT_BYTES` ⇒ chạy; vượt ⇒ OutOfFuel (giới hạn oracle)
    for f in ["arena", "heap"] {
        assert_eq!(run(f, &[16, 4096]), Outcome::Value(Some(1)), "@{f}");
        for al in [3, 0, 8192] {
            ub(f, &[16, al]);
        }
        let max = u128::from(MAX_SLOT_BYTES);
        assert_eq!(run(f, &[max, 8]), Outcome::Value(Some(1)), "@{f} cỡ trần");
        assert_eq!(run(f, &[max + 1, 8]), Outcome::OutOfFuel, "@{f} vượt trần");
    }
    // tay cầm Arena chưa cấp (0 khi chưa có `arena_new`) và sai số đối số ⇒ Ub (không panic)
    ub("nohandle", &[]);
    ub("arity", &[]);
}

/// Đột biến tự động (R2, 2026-11-26) sống ở mô hình `gritrt_mutex_lock/unlock` (mới, đợt 3): chốt ô khoá null/chỉ-đọc/đã trả/lệch căn lề chưa có ca âm.
#[test]
fn mutex_model_rejects_bad_lock_cells() {
    let m = module(
        "glir 1
global @g = 0000000000000000
declare @gritrt_mutex_lock(ptr) -> void
declare @gritrt_heap_alloc(i64, i64) -> ptr
declare @gritrt_heap_free(ptr, i64, i64) -> void
fn @ok() -> i32 {
slot $l: i64
entry:
  store i64 i64 0, $l
  call void @gritrt_mutex_lock($l)
  ret i32 i32 1
}
fn @glob() -> i32 {
entry:
  call void @gritrt_mutex_lock(@g)
  ret i32 i32 1
}
fn @nul() -> i32 {
entry:
  call void @gritrt_mutex_lock(null)
  ret i32 i32 1
}
fn @freed() -> i32 {
entry:
  %p = call ptr @gritrt_heap_alloc(i64 8, i64 8)
  store i64 i64 0, %p
  call void @gritrt_heap_free(%p, i64 8, i64 8)
  call void @gritrt_mutex_lock(%p)
  ret i32 i32 1
}
fn @skew() -> i32 {
slot $l: [2 x i64]
entry:
  %p = indexptr i8 $l, i64 4
  call void @gritrt_mutex_lock(%p)
  ret i32 i32 1
}
",
    );
    assert_eq!(run_module(&m, "ok", &[], 1000), Outcome::Value(Some(1)));
    for f in ["glob", "nul", "freed", "skew"] {
        let o = run_module(&m, f, &[], 1000);
        assert!(matches!(o, Outcome::Ub(_)), "@{f}: {o:?}");
    }
}
