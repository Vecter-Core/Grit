//! IR Guard (ADR-029 §6): mỗi luật có test CHẤP NHẬN và test TỪ CHỐI trên IR dựng tay ("IR bị sửa tay bị từ chối").
use gt_guard::{ATTEST_KEY, GuardInput, OBL_KIND, check_module};
use gt_llvm::inkwell::AtomicOrdering;
use gt_llvm::inkwell::AtomicRMWBinOp;
use gt_llvm::inkwell::attributes::{Attribute, AttributeLoc};
use gt_llvm::inkwell::builder::Builder;
use gt_llvm::inkwell::context::Context;
use gt_llvm::inkwell::module::Module;
use gt_llvm::inkwell::values::{BasicValue, FunctionValue, InstructionValue};

const ATT: &str = "att-1234";

struct W<'c> {
    ctx: &'c Context,
    m: Module<'c>,
    b: Builder<'c>,
    abort: FunctionValue<'c>,
}

fn world<'a>(ctx: &'a Context, noreturn: bool, attest: Option<&str>) -> W<'a> {
    let m = ctx.create_module("t");
    let b = ctx.create_builder();
    let abort = m.add_function(
        "gritrt_abort",
        ctx.void_type().fn_type(&[ctx.i32_type().into()], false),
        None,
    );
    if noreturn {
        abort.add_attribute(
            AttributeLoc::Function,
            ctx.create_enum_attribute(Attribute::get_named_enum_kind_id("noreturn"), 0),
        );
    }
    if let Some(a) = attest {
        let node = ctx.metadata_node(&[ctx.metadata_string(a).into()]);
        m.add_global_metadata(ATTEST_KEY, &node).expect("metadata");
    }
    W { ctx, m, b, abort }
}

impl<'c> W<'c> {
    fn func(&self, name: &str) -> FunctionValue<'c> {
        let f = self
            .m
            .add_function(name, self.ctx.void_type().fn_type(&[], false), None);
        let bb = self.ctx.append_basic_block(f, "bb0");
        self.b.position_at_end(bb);
        f
    }

    /// khối `trap.0` đúng mẫu: call abort + unreachable
    fn good_trap(&self, f: FunctionValue<'c>) {
        let t = self.ctx.append_basic_block(f, "trap.0");
        self.b.position_at_end(t);
        self.b
            .build_call(
                self.abort,
                &[self.ctx.i32_type().const_int(1, false).into()],
                "",
            )
            .expect("call");
        self.b.build_unreachable().expect("unreachable");
    }

    fn tag(&self, inst: InstructionValue<'c>, class: &str) {
        let node = self
            .ctx
            .metadata_node(&[self.ctx.metadata_string(class).into()]);
        inst.set_metadata(node, self.ctx.get_kind_id(OBL_KIND))
            .expect("set");
    }

    fn check(&self) -> Vec<String> {
        check_module(&self.m, &GuardInput { attestation: ATT })
    }
}

/// hoàn tất hàm hợp lệ: entry → ret, cộng một khối trap đúng mẫu (mồ côi nhưng verifier cho phép)
fn finish_ok<'a>(w: &W<'a>, f: FunctionValue<'a>) {
    w.good_trap(f);
    let e = f.get_first_basic_block().expect("entry");
    w.b.position_at_end(e);
    w.b.build_return(None).expect("ret");
}

fn has(v: &[String], needle: &str) -> bool {
    v.iter().any(|s| s.contains(needle))
}

#[test]
fn accepts_well_formed_module() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    finish_ok(&w, f);
    assert_eq!(w.check(), Vec::<String>::new());
}

#[test]
fn rejects_wrong_or_missing_attestation() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some("khac"));
    let f = w.func("f");
    finish_ok(&w, f);
    assert!(has(&w.check(), "attestation lệch"), "{:?}", w.check());
    let ctx2 = Context::create();
    let w2 = world(&ctx2, true, None);
    let f2 = w2.func("f");
    finish_ok(&w2, f2);
    assert!(
        has(&w2.check(), "thiếu metadata !gt.attest"),
        "{:?}",
        w2.check()
    );
}

#[test]
fn rejects_stray_unreachable() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let _f = w.func("f");
    w.b.build_unreachable().expect("u"); // trong khối bb0, không phải trap.*
    assert!(has(&w.check(), "không đúng mẫu abort"), "{:?}", w.check());
}

#[test]
fn rejects_unreachable_in_trap_block_without_abort_call() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let t = ctx.append_basic_block(f, "trap.0");
    w.b.build_unconditional_branch(t).expect("br");
    w.b.position_at_end(t);
    w.b.build_unreachable().expect("u"); // tên đúng nhưng KHÔNG có call abort ngay trước
    assert!(has(&w.check(), "không đúng mẫu abort"), "{:?}", w.check());
}

#[test]
fn rejects_abort_without_noreturn() {
    let ctx = Context::create();
    let w = world(&ctx, false, Some(ATT)); // gritrt_abort KHÔNG noreturn
    let f = w.func("f");
    finish_ok(&w, f);
    assert!(has(&w.check(), "không đúng mẫu abort"), "{:?}", w.check());
}

#[test]
fn atomic_operations_need_matching_obligation() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let i64t = ctx.i64_type();
    let slot = w.b.build_alloca(i64t, "x").expect("alloca");
    let rmw =
        w.b.build_atomicrmw(
            AtomicRMWBinOp::Add,
            slot,
            i64t.const_int(1, false),
            AtomicOrdering::Release,
        )
        .expect("rmw");
    let inst = rmw.as_instruction().expect("inst");
    finish_ok(&w, f);
    assert!(has(&w.check(), "thiếu metadata !gt.obl"), "{:?}", w.check());
    w.tag(inst, "E0501"); // sai lớp
    assert!(has(&w.check(), "cần `E0503`"), "{:?}", w.check());
    w.tag(inst, "E0503");
    assert_eq!(w.check(), Vec::<String>::new());
}

#[test]
fn plain_load_store_are_not_sensitive_but_atomic_ones_are() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let i64t = ctx.i64_type();
    let slot = w.b.build_alloca(i64t, "x").expect("alloca");
    w.b.build_store(slot, i64t.const_zero()).expect("store"); // thường: không cần metadata
    let ld = w.b.build_load(i64t, slot, "l").expect("load");
    finish_ok(&w, f);
    assert_eq!(w.check(), Vec::<String>::new());
    // biến load thành atomic ⇒ nhạy cảm
    let li = ld.as_instruction_value().expect("i");
    li.set_atomic_ordering(AtomicOrdering::Acquire)
        .expect("ord");
    li.set_alignment(8).expect("al");
    assert!(has(&w.check(), "thiếu metadata !gt.obl"), "{:?}", w.check());
}

#[test]
fn runtime_calls_need_their_class() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let ptr = ctx.ptr_type(Default::default());
    let spawn = w.m.add_function(
        "gritrt_spawn",
        ctx.void_type().fn_type(&[ptr.into(), ptr.into()], false),
        None,
    );
    let lock = w.m.add_function(
        "gritrt_mutex_lock",
        ctx.void_type().fn_type(&[ptr.into()], false),
        None,
    );
    let p = ptr.const_null();
    let c1 = w.b.build_call(spawn, &[p.into(), p.into()], "").expect("c");
    let c2 = w.b.build_call(lock, &[p.into()], "").expect("c");
    finish_ok(&w, f);
    let v = w.check();
    assert_eq!(
        v.iter()
            .filter(|s| s.contains("thiếu metadata !gt.obl"))
            .count(),
        2,
        "{v:?}"
    );
    w.tag(c1.try_as_basic_value().expect_instruction("call"), "E0501");
    w.tag(c2.try_as_basic_value().expect_instruction("call"), "E0502");
    assert_eq!(w.check(), Vec::<String>::new());
}

#[test]
fn rejects_poison_flags_but_allows_plain_arithmetic_and_gep() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let i32t = ctx.i32_type();
    let a = w.b.build_alloca(i32t, "a").expect("a");
    let x = w.b.build_load(i32t, a, "x").expect("x").into_int_value();
    let st = ctx.struct_type(&[i32t.into(), i32t.into()], false);
    let g = w.b.build_alloca(st, "g").expect("g");
    let _ = w.b.build_struct_gep(st, g, 1, "f").expect("gep");
    let s = w.b.build_int_add(x, x, "plain").expect("add");
    finish_ok(&w, f);
    assert_eq!(
        w.check(),
        Vec::<String>::new(),
        "cộng thường + GEP phải được chấp nhận"
    );
    // thêm một phép cộng có `nsw` vào cuối khối entry (trước ret): dựng lại vị trí
    let e = f.get_first_basic_block().expect("entry");
    let ret = e.get_terminator().expect("ret");
    w.b.position_before(&ret);
    let _ = w.b.build_int_nsw_add(s, s, "wrapless").expect("nsw");
    let v = w.check();
    assert!(has(&v, "cờ sinh poison/UB `nsw`"), "{v:?}");
}

#[test]
fn rejects_unverifiable_ir() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let _f = w.func("f"); // khối bb0 không có terminator ⇒ verifier từ chối
    assert!(has(&w.check(), "LLVM verifier từ chối"), "{:?}", w.check());
}

/// Đột biến #1 (lib.rs:29 `condition → false`) KHÔNG bị test này giết và là TƯƠNG ĐƯƠNG theo hướng chấp nhận (xem KNOWN_LIMITATIONS "gt-guard"):
/// dạng chữ chỉ cho ra `@tên(` ở lệnh `call`/`invoke`; `invoke` là terminator nên không thể đứng ngay trước `unreachable` trong cùng khối.
/// Test này khoá hành vi mong muốn: `unreachable` sau một lệnh không phải call (dù nhắc `@gritrt_abort`) bị từ chối.
#[test]
fn unreachable_after_a_non_call_mentioning_abort_is_rejected() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let t = ctx.append_basic_block(f, "trap.0");
    w.b.build_unconditional_branch(t).expect("br");
    w.b.position_at_end(t);
    let slot =
        w.b.build_alloca(ctx.ptr_type(Default::default()), "s")
            .expect("alloca");
    // lệnh KHÔNG phải call nhưng có `@gritrt_abort` trong dạng chữ
    w.b.build_store(slot, w.abort.as_global_value().as_pointer_value())
        .expect("store");
    w.b.build_unreachable().expect("u");
    let v = w.check();
    assert!(has(&v, "không đúng mẫu abort"), "{v:?}");
}

/// Đột biến sống sót #2 (lib.rs:158 `condition → false`): `llvm.assume` phải bị cấm (cho phép tối ưu viên suy luận từ giả định tự khẳng định).
#[test]
fn rejects_llvm_assume() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let assume = gt_llvm::inkwell::intrinsics::Intrinsic::find("llvm.assume")
        .expect("intrinsic")
        .get_declaration(&w.m, &[])
        .expect("decl");
    w.b.build_call(assume, &[ctx.bool_type().const_int(1, false).into()], "")
        .expect("call");
    finish_ok(&w, f);
    let v = w.check();
    assert!(has(&v, "`llvm.assume` bị cấm"), "{v:?}");
}

fn declare<'c>(w: &W<'c>, name: &str, params: usize) -> FunctionValue<'c> {
    let ptr = w.ctx.ptr_type(Default::default());
    let ps: Vec<_> = (0..params).map(|_| ptr.into()).collect();
    w.m.add_function(name, w.ctx.void_type().fn_type(&ps, false), None)
}

/// Lớp I/O (E0310): mọi lời gọi runtime I/O qua `Cap` phải mang `!gt.obl = E0310`; thiếu hoặc sai lớp ⇒ từ chối.
#[test]
fn io_runtime_calls_need_the_capability_class() {
    for name in [
        "gritrt_arg_ptr",
        "gritrt_arg_len",
        "gritrt_fs_read",
        "gritrt_file_len",
        "gritrt_file_ptr",
        "gritrt_write_u64",
        "gritrt_write_str",
    ] {
        let ctx = Context::create();
        let w = world(&ctx, true, Some(ATT));
        let f = w.func("f");
        let callee = declare(&w, name, 0);
        let c = w.b.build_call(callee, &[], "").expect("call");
        finish_ok(&w, f);
        let inst = c.try_as_basic_value().expect_instruction("call");
        assert!(
            has(&w.check(), "thiếu metadata !gt.obl") && has(&w.check(), "`E0310`"),
            "{name}: {:?}",
            w.check()
        );
        w.tag(inst, "E0502"); // sai lớp
        assert!(has(&w.check(), "cần `E0310`"), "{name}: {:?}", w.check());
        w.tag(inst, "E0310");
        assert_eq!(w.check(), Vec::<String>::new(), "{name}");
    }
}

/// Bảng runtime đóng: `gritrt_*` lạ và hàm ngoài (FFI) bị từ chối; `llvm.*`, hàm có thân, và runtime không nhạy cảm thì được.
#[test]
fn unknown_runtime_and_external_calls_are_rejected_known_ones_accepted() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let unknown = declare(&w, "gritrt_exec", 0);
    w.b.build_call(unknown, &[], "").expect("call");
    let ext = declare(&w, "system", 1);
    let p = ctx.ptr_type(Default::default()).const_null();
    w.b.build_call(ext, &[p.into()], "").expect("call");
    finish_ok(&w, f);
    let v = w.check();
    assert!(has(&v, "`gritrt_exec` không thuộc lớp nào"), "{v:?}");
    assert!(has(&v, "hàm ngoài `system`"), "{v:?}");

    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("g");
    for name in ["gritrt_start", "gritrt_report_int", "gritrt_alloc"] {
        let c = declare(&w, name, 0);
        w.b.build_call(c, &[], "").expect("call");
    }
    let defined = w.m.add_function(
        "callee_with_body",
        ctx.void_type().fn_type(&[], false),
        None,
    );
    let bb = ctx.append_basic_block(defined, "bb0");
    let here = w.b.get_insert_block().expect("blk");
    w.b.position_at_end(bb);
    w.b.build_return(None).expect("ret");
    w.b.position_at_end(here);
    w.b.build_call(defined, &[], "").expect("call");
    let i32t = ctx.i32_type();
    let intr = gt_llvm::inkwell::intrinsics::Intrinsic::find("llvm.uadd.with.overflow")
        .expect("intrinsic")
        .get_declaration(&w.m, &[i32t.into()])
        .expect("decl");
    w.b.build_call(
        intr,
        &[i32t.const_zero().into(), i32t.const_zero().into()],
        "",
    )
    .expect("call");
    finish_ok(&w, f);
    assert_eq!(w.check(), Vec::<String>::new());
}
