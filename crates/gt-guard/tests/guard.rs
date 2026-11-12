//! IR Guard (ADR-029 §6): mỗi luật có test CHẤP NHẬN và test TỪ CHỐI trên IR dựng tay ("IR bị sửa tay bị từ chối").
use gt_guard::{ATTEST_KEY, GuardInput, INSTANCES_KEY, OBL_KIND, check_module};
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
    /// tên hàm đã định nghĩa qua `func` (tập cho phép và nội dung `!gt.instances` mặc định của harness)
    names: std::cell::RefCell<Vec<String>>,
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
    W {
        ctx,
        m,
        b,
        abort,
        names: std::cell::RefCell::new(Vec::new()),
    }
}

impl<'c> W<'c> {
    fn func(&self, name: &str) -> FunctionValue<'c> {
        let f = self
            .m
            .add_function(name, self.ctx.void_type().fn_type(&[], false), None);
        let bb = self.ctx.append_basic_block(f, "bb0");
        self.b.position_at_end(bb);
        self.names.borrow_mut().push(name.to_string());
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

    /// ghi `!gt.instances` đúng theo các hàm đã tạo (test muốn lệch thì dùng `check_with`)
    fn check(&self) -> Vec<String> {
        let names = self.names.borrow().clone();
        let entries: Vec<_> = names
            .iter()
            .map(|n| self.ctx.metadata_string(&format!("{n}={n}")).into())
            .collect();
        self.m
            .add_global_metadata(INSTANCES_KEY, &self.ctx.metadata_node(&entries))
            .expect("metadata");
        self.check_with(&names)
    }

    /// kiểm với tập cho phép tuỳ ý (không thêm metadata)
    fn check_with(&self, allowed: &[String]) -> Vec<String> {
        check_module(
            &self.m,
            &GuardInput {
                attestation: ATT,
                instances: allowed,
            },
        )
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

/// FAIL_OPEN_AUDIT G1: gọi `gritrt_*` qua CON TRỎ (lời gọi gián tiếp) làm `callee_name` trả `None` ⇒ luật "lớp nghĩa vụ" và "đích mọi lời gọi" không chạy.
/// Quy tắc mới: địa chỉ của hàm `gritrt_*` chỉ được dùng làm đích gọi TRỰC TIẾP (không lưu, không truyền làm đối số, không gọi gián tiếp).
#[test]
fn runtime_function_addresses_may_only_be_called_directly() {
    let ctx = Context::create();
    // (1) lưu địa chỉ rồi gọi gián tiếp một hàm nhạy cảm (đọc tệp) không mang !gt.obl
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let ptr = ctx.ptr_type(Default::default());
    let i64t = ctx.i64_type();
    let fs_read_ty = ctx.i32_type().fn_type(&[ptr.into(), i64t.into()], false);
    let fs_read = w.m.add_function("gritrt_fs_read", fs_read_ty, None);
    let slot = w.b.build_alloca(ptr, "slot").expect("alloca");
    w.b.build_store(slot, fs_read.as_global_value().as_pointer_value())
        .expect("store");
    let fp =
        w.b.build_load(ptr, slot, "fp")
            .expect("load")
            .into_pointer_value();
    w.b.build_indirect_call(
        fs_read_ty,
        fp,
        &[ptr.const_null().into(), i64t.const_zero().into()],
        "",
    )
    .expect("indirect");
    finish_ok(&w, f);
    let v = w.check();
    assert!(
        has(&v, "gritrt_fs_read"),
        "gọi gián tiếp hàm runtime bị CHẤP NHẬN: {v:?}"
    );

    // (2) chỉ lưu địa chỉ (không gọi) cũng bị từ chối
    let ctx2 = Context::create();
    let w = world(&ctx2, true, Some(ATT));
    let f = w.func("f");
    let ptr = ctx2.ptr_type(Default::default());
    let alloc = w.m.add_function(
        "gritrt_alloc",
        ptr.fn_type(&[ctx2.i64_type().into()], false),
        None,
    );
    let slot = w.b.build_alloca(ptr, "slot").expect("alloca");
    w.b.build_store(slot, alloc.as_global_value().as_pointer_value())
        .expect("store");
    finish_ok(&w, f);
    let v = w.check();
    assert!(
        has(&v, "gritrt_alloc"),
        "lưu địa chỉ hàm runtime bị CHẤP NHẬN: {v:?}"
    );

    // (3) đối chứng: gọi TRỰC TIẾP hàm không nhạy cảm (gritrt_alloc, lớp None) vẫn được chấp nhận
    let ctx3 = Context::create();
    let w = world(&ctx3, true, Some(ATT));
    let f = w.func("f");
    let ptr = ctx3.ptr_type(Default::default());
    let alloc = w.m.add_function(
        "gritrt_alloc",
        ptr.fn_type(&[ctx3.i64_type().into()], false),
        None,
    );
    w.b.build_call(alloc, &[ctx3.i64_type().const_int(16, false).into()], "")
        .expect("call");
    finish_ok(&w, f);
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

/// FB-9: `nuw` trên phép CỘNG vẫn bị từ chối (chỉ cụm `getelementptr inbounds nuw` của biểu thức hằng được miễn)
#[test]
fn nuw_on_arithmetic_is_still_rejected() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let i32t = ctx.i32_type();
    let a = w.b.build_alloca(i32t, "a").expect("a");
    let x = w.b.build_load(i32t, a, "x").expect("x").into_int_value();
    let _ = w.b.build_int_nuw_add(x, x, "nuwadd").expect("nuw");
    finish_ok(&w, f);
    let v = w.check();
    assert!(has(&v, "cờ sinh poison/UB `nuw`"), "{v:?}");
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

/// FAIL_OPEN_AUDIT G1.5: an arbitrary LLVM intrinsic can encode poison via an
/// immediate flag. The Guard must accept only the intrinsic families emitted
/// by this code generator, even when the LLVM verifier accepts the call.
#[test]
fn rejects_non_allowlisted_llvm_intrinsic_with_poison_flag() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let abs = gt_llvm::inkwell::intrinsics::Intrinsic::find("llvm.abs")
        .expect("intrinsic")
        .get_declaration(&w.m, &[ctx.i32_type().into()])
        .expect("decl");
    w.b.build_call(
        abs,
        &[
            ctx.i32_type().const_int(1, false).into(),
            ctx.bool_type().const_int(1, false).into(),
        ],
        "abs",
    )
    .expect("call");
    finish_ok(&w, f);
    let v = w.check();
    assert!(has(&v, "LLVM intrinsic ngoài danh sách cho phép"), "{v:?}");
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
    w.names.borrow_mut().push("callee_with_body".to_string());
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

/// FB-2 (hồi quy): tên hàm có ký tự không ASCII (`h⊲0` — hợp đồng) phải được nhận ra là hàm CÓ THÂN trong module; trước đây Guard so tên theo
/// dạng chữ đã thoát của LLVM nên từ chối nhầm lời gọi hợp lệ. Ngược lại hàm CHỈ KHAI BÁO tên Unicode vẫn bị từ chối.
#[test]
fn unicode_named_functions_are_resolved_by_api_not_by_printed_text() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let defined =
        w.m.add_function("main::h⊲0#3", ctx.void_type().fn_type(&[], false), None);
    let bb = ctx.append_basic_block(defined, "bb0");
    let here = w.b.get_insert_block().expect("blk");
    w.b.position_at_end(bb);
    w.b.build_return(None).expect("ret");
    w.b.position_at_end(here);
    w.b.build_call(defined, &[], "").expect("call");
    finish_ok(&w, f);
    w.names.borrow_mut().push("main::h⊲0#3".to_string());
    // lời gọi tới hàm Unicode CÓ THÂN được nhận ra bằng API (không còn báo "lời gọi hàm ngoài"); riêng TÊN không ASCII bị luật
    // symbol từ chối (Q69: codegen không bao giờ sinh tên như vậy nữa)
    let v = w.check();
    assert!(!has(&v, "lời gọi hàm ngoài"), "{v:?}");
    assert!(has(&v, "không phải ASCII"), "{v:?}");

    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let ext =
        w.m.add_function("hàm_ngoài⊳1", ctx.void_type().fn_type(&[], false), None);
    w.b.build_call(ext, &[], "").expect("call");
    finish_ok(&w, f);
    let v = w.check();
    assert!(has(&v, "lời gọi hàm ngoài `hàm_ngoài⊳1`"), "{v:?}");
}

#[test]
fn rejects_function_outside_instance_table() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    finish_ok(&w, f);
    let g = w.func("rogue");
    finish_ok(&w, g);
    // metadata liệt kê cả hai, nhưng bảng của M4 chỉ cho `f`
    let entries: Vec<_> = ["f", "rogue"]
        .iter()
        .map(|n| ctx.metadata_string(&format!("{n}={n}")).into())
        .collect();
    w.m.add_global_metadata(INSTANCES_KEY, &ctx.metadata_node(&entries))
        .expect("md");
    let v = w.check_with(&["f".to_string()]);
    assert!(has(&v, "không thuộc bảng instance"), "{v:?}");
}

#[test]
fn rejects_non_ascii_function_name() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("h⊲0");
    finish_ok(&w, f);
    let v = w.check();
    assert!(has(&v, "không phải ASCII"), "{v:?}");
}

#[test]
fn rejects_instances_metadata_that_disagrees_with_the_ir() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    finish_ok(&w, f);
    // metadata nói `g` mà IR định nghĩa `f`
    let node = ctx.metadata_node(&[ctx.metadata_string("g=g").into()]);
    w.m.add_global_metadata(INSTANCES_KEY, &node).expect("md");
    let v = w.check_with(&["f".to_string(), "g".to_string()]);
    assert!(has(&v, "!gt.instances lệch IR"), "{v:?}");
    // thiếu hẳn metadata cũng bị từ chối
    let ctx2 = Context::create();
    let w2 = world(&ctx2, true, Some(ATT));
    let f2 = w2.func("f");
    finish_ok(&w2, f2);
    let v2 = w2.check_with(&["f".to_string()]);
    assert!(has(&v2, "!gt.instances lệch IR"), "{v2:?}");
}

#[test]
fn rejects_module_without_any_instance() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let v = w.check();
    assert!(has(&v, "không định nghĩa instance nào"), "{v:?}");
}

#[test]
fn entry_function_is_allowed_outside_the_table() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    finish_ok(&w, f);
    let node = ctx.metadata_node(&[ctx.metadata_string("f=f").into()]);
    w.m.add_global_metadata(INSTANCES_KEY, &node).expect("md");
    let e = w.func("gt_entry");
    finish_ok(&w, e);
    // `gt_entry` KHÔNG nằm trong tập cho phép lẫn metadata mà vẫn được chấp nhận
    assert_eq!(w.check_with(&["f".to_string()]), Vec::<String>::new());
}

/// Hàm hỗ trợ do codegen sinh: `main` của C và `gt.tramp.<số>` được chấp nhận ngoài bảng instance; dạng gần giống thì không.
#[test]
fn support_functions_are_accepted_only_in_their_exact_forms() {
    let build = |extra: &[&str]| -> Vec<String> {
        let ctx = Context::create();
        let w = world(&ctx, true, Some(ATT));
        let f = w.func("f");
        finish_ok(&w, f);
        let node = ctx.metadata_node(&[ctx.metadata_string("f=f").into()]);
        w.m.add_global_metadata(INSTANCES_KEY, &node).expect("md");
        for n in extra {
            let g = w.func(n);
            finish_ok(&w, g);
        }
        w.check_with(&["f".to_string()])
    };
    assert_eq!(
        build(&["main", "gt.tramp.0", "gt.tramp.12"]),
        Vec::<String>::new()
    );
    for bad in [
        "gt.tramp.",
        "gt.trampX",
        "gt.tramp.1x",
        "gt.tramp",
        "Main",
        "gt_entry2",
    ] {
        let v = build(&[bad]);
        assert!(has(&v, "không thuộc bảng instance"), "{bad}: {v:?}");
    }
}

/// Tên hàm chỉ KHAI BÁO (runtime/llvm) cũng phải toàn ASCII; ký tự `-`, khoảng trắng, `$`, dấu nháy bị từ chối.
#[test]
fn symbol_charset_is_strict() {
    for bad in ["a-b", "a b", "a$b", "a\"b", ""] {
        let ctx = Context::create();
        let w = world(&ctx, true, Some(ATT));
        let f = w.func("f");
        finish_ok(&w, f);
        if !bad.is_empty() {
            w.m.add_function(bad, ctx.void_type().fn_type(&[], false), None);
        }
        let v = w.check();
        if bad.is_empty() {
            assert!(!has(&v, "không phải ASCII"), "{v:?}");
        } else {
            assert!(has(&v, "không phải ASCII"), "{bad}: {v:?}");
        }
    }
    // ký tự hợp lệ: chữ, số, `_`, `.`
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("Ab_9.z");
    finish_ok(&w, f);
    assert_eq!(w.check(), Vec::<String>::new());
}

// ---------------------------------------------------------------------------------------------------------------------------------
// GUARD-POST (ADR-036 §6a): `check_post` = mọi luật của `check_module` + không giảm số `gritrt_abort` theo (hàm, loại) + không FMA/thuộc tính FP.
// ---------------------------------------------------------------------------------------------------------------------------------
use gt_guard::{PreFacts, check_post, pre_facts};

fn trap_kind<'a>(w: &W<'a>, f: FunctionValue<'a>, name: &str, kind: u64) {
    let t = w.ctx.append_basic_block(f, name);
    w.b.position_at_end(t);
    w.b.build_call(
        w.abort,
        &[w.ctx.i32_type().const_int(kind, false).into()],
        "",
    )
    .expect("call");
    w.b.build_unreachable().expect("unreachable");
}

/// gắn `!gt.instances` đúng theo các hàm đã tạo
fn add_instances(w: &W<'_>) {
    let names = w.names.borrow().clone();
    let entries: Vec<_> = names
        .iter()
        .map(|n| w.ctx.metadata_string(&format!("{n}={n}")).into())
        .collect();
    w.m.add_global_metadata(INSTANCES_KEY, &w.ctx.metadata_node(&entries))
        .expect("metadata");
}

/// hàm `f` có các khối trap theo (tên, loại) rồi ret ở entry; đã gắn `!gt.instances`
fn module_with_traps<'a>(ctx: &'a Context, traps: &[(&str, u64)]) -> W<'a> {
    let w = world(ctx, true, Some(ATT));
    let f = w.func("f");
    for (n, k) in traps {
        trap_kind(&w, f, n, *k);
    }
    let e = f.get_first_basic_block().expect("entry");
    w.b.position_at_end(e);
    w.b.build_return(None).expect("ret");
    add_instances(&w);
    w
}

fn post(w: &W<'_>, pre: &PreFacts) -> Vec<String> {
    let names = w.names.borrow().clone();
    check_post(
        &w.m,
        &GuardInput {
            attestation: ATT,
            instances: &names,
        },
        pre,
    )
}

/// đặt con trỏ dựng trước lệnh `ret` của entry của `f`
fn before_ret(w: &W<'_>, name: &str) {
    let f = w.m.get_function(name).expect("hàm");
    let ret = f
        .get_first_basic_block()
        .expect("entry")
        .get_terminator()
        .expect("ret");
    w.b.position_before(&ret);
}

#[test]
fn post_accepts_equal_or_more_aborts_per_function_and_kind() {
    let c0 = Context::create();
    let before = module_with_traps(&c0, &[("trap.0", 1), ("trap.1", 2)]);
    let pre = pre_facts(&before.m);
    assert_eq!(post(&before, &pre), Vec::<String>::new());
    let c1 = Context::create();
    let more = module_with_traps(&c1, &[("trap.0", 1), ("trap.1", 2), ("trap.2", 2)]);
    assert_eq!(
        post(&more, &pre),
        Vec::<String>::new(),
        "thêm abort không phải vi phạm"
    );
}

#[test]
fn post_rejects_fewer_aborts_of_a_kind_even_if_another_kind_compensates() {
    let c0 = Context::create();
    let before = module_with_traps(&c0, &[("trap.0", 1), ("trap.1", 2)]);
    let pre = pre_facts(&before.m);
    // mất một abort loại 2
    let c1 = Context::create();
    let fewer = module_with_traps(&c1, &[("trap.0", 1)]);
    let v = post(&fewer, &pre);
    assert!(has(&v, "loại 2 giảm 1 → 0"), "{v:?}");
    // bù bằng một abort loại khác: số tổng không đổi nhưng VẪN bị từ chối (đếm theo loại)
    let c2 = Context::create();
    let swapped = module_with_traps(&c2, &[("trap.0", 1), ("trap.1", 3)]);
    let v = post(&swapped, &pre);
    assert!(has(&v, "loại 2 giảm 1 → 0"), "{v:?}");
    // chuyển abort sang hàm khác: bị từ chối (đếm theo hàm)
    let c3 = Context::create();
    let w = world(&c3, true, Some(ATT));
    let f = w.func("f");
    let g = w.func("g");
    trap_kind(&w, g, "trap.0", 1);
    trap_kind(&w, g, "trap.1", 2);
    for fv in [f, g] {
        w.b.position_at_end(fv.get_first_basic_block().expect("entry"));
        w.b.build_return(None).expect("ret");
    }
    add_instances(&w);
    let v = post(&w, &pre);
    assert!(
        has(&v, "f: số lời gọi gritrt_abort loại 1 giảm 1 → 0"),
        "{v:?}"
    );
}

#[test]
fn post_rejects_fma_fp_attributes_and_contract_flags() {
    for intr in ["llvm.fma.f64", "llvm.fmuladd.f64"] {
        let c = Context::create();
        let w = module_with_traps(&c, &[("trap.0", 1)]);
        let pre = pre_facts(&w.m);
        assert_eq!(post(&w, &pre), Vec::<String>::new());
        let f64t = c.f64_type();
        let ty = f64t.fn_type(&[f64t.into(), f64t.into(), f64t.into()], false);
        let d = w.m.add_function(intr, ty, None);
        before_ret(&w, "f");
        let z = f64t.const_float(1.0);
        w.b.build_call(d, &[z.into(), z.into(), z.into()], "")
            .expect("call");
        let v = post(&w, &pre);
        assert!(has(&v, "hợp nhất nhân-cộng"), "{intr}: {v:?}");
    }
    // từng thuộc tính hàm FP không an toàn
    for key in [
        "unsafe-fp-math",
        "no-nans-fp-math",
        "no-infs-fp-math",
        "no-signed-zeros-fp-math",
        "approx-func-fp-math",
        "less-precise-fpmad",
    ] {
        let c = Context::create();
        let w = module_with_traps(&c, &[("trap.0", 1)]);
        let pre = pre_facts(&w.m);
        w.m.get_function("f").expect("f").add_attribute(
            AttributeLoc::Function,
            c.create_string_attribute(key, "true"),
        );
        let v = post(&w, &pre);
        assert!(has(&v, &format!("`{key}`")), "{key}: {v:?}");
    }
    // thuộc tính chuỗi vô hại (ví dụ target-cpu) KHÔNG bị từ chối
    let c = Context::create();
    let w = module_with_traps(&c, &[("trap.0", 1)]);
    let pre = pre_facts(&w.m);
    w.m.get_function("f").expect("f").add_attribute(
        AttributeLoc::Function,
        c.create_string_attribute("target-cpu", "generic"),
    );
    assert_eq!(post(&w, &pre), Vec::<String>::new());
}

#[test]
fn post_rejects_each_fast_math_flag_on_float_ops() {
    use gt_llvm::inkwell::values::FastMathFlags;
    for (flag, tok) in [
        (FastMathFlags::AllowContract, "contract"),
        (FastMathFlags::AllowReassoc, "reassoc"),
        (FastMathFlags::AllowReciprocal, "arcp"),
        (FastMathFlags::NoNaNs, "nnan"),
        (FastMathFlags::NoInfs, "ninf"),
        (FastMathFlags::NoSignedZeros, "nsz"),
        (FastMathFlags::ApproxFunc, "afn"),
    ] {
        let c = Context::create();
        let w = module_with_traps(&c, &[("trap.0", 1)]);
        let pre = pre_facts(&w.m);
        let f64t = c.f64_type();
        before_ret(&w, "f");
        let p = w.b.build_alloca(f64t, "p").expect("p");
        let l = w.b.build_load(f64t, p, "l").expect("l").into_float_value();
        let r = w.b.build_float_add(l, l, "r").expect("fadd");
        assert_eq!(
            post(&w, &pre),
            Vec::<String>::new(),
            "{tok}: fadd thường phải được chấp nhận"
        );
        r.as_instruction()
            .expect("inst")
            .set_fast_math_flags(flag)
            .expect("flags");
        let v = post(&w, &pre);
        assert!(
            has(&v, &format!("cờ sinh poison/UB `{tok}`")),
            "{tok}: {v:?}"
        );
    }
}

/// Dựng `W` từ IR dạng chữ (cho lệnh mà trình dựng của inkwell không có: `invoke` trên LLVM 20, `callbr`). Hàm `f` là instance duy nhất.
fn world_from_text<'a>(ctx: &'a Context, text: &str) -> W<'a> {
    use gt_llvm::inkwell::memory_buffer::MemoryBuffer;
    let m = ctx
        .create_module_from_ir(MemoryBuffer::create_from_memory_range_copy(
            text.as_bytes(),
            "t",
        ))
        .unwrap_or_else(|e| {
            panic!(
                "IR chữ không phân tích được: {e}
{text}"
            )
        });
    let node = ctx.metadata_node(&[ctx.metadata_string(ATT).into()]);
    m.add_global_metadata(ATTEST_KEY, &node).expect("metadata");
    let abort = m.get_function("gritrt_abort").expect("gritrt_abort");
    W {
        ctx,
        m,
        b: ctx.create_builder(),
        abort,
        names: std::cell::RefCell::new(vec!["f".to_string()]),
    }
}

/// FAIL_OPEN_AUDIT G1 (luật 3c): `invoke` KHÔNG qua `callee_name` nên tránh luật "đích mọi lời gọi" (ở đây gọi một hàm ngoài chỉ khai báo) ⇒ phải bị CẤM thẳng.
#[test]
fn invoke_is_rejected_even_when_its_target_would_otherwise_be_flagged() {
    let ctx = Context::create();
    let w = world_from_text(
        &ctx,
        r#"
declare void @gritrt_abort(i32) noreturn
declare i32 @__gxx_personality_v0(...)
declare void @ngoai()
define void @f() personality ptr @__gxx_personality_v0 {
bb0:
  invoke void @ngoai() to label %ok unwind label %lp
ok:
  ret void
lp:
  %e = landingpad { ptr, i32 } cleanup
  resume { ptr, i32 } %e
}
"#,
    );
    let v = w.check();
    assert!(has(&v, "`invoke`/`callbr` bị cấm"), "{v:?}");
}

#[test]
fn callbr_is_rejected() {
    let ctx = Context::create();
    let w = world_from_text(
        &ctx,
        r#"
declare void @gritrt_abort(i32) noreturn
define void @f() {
bb0:
  callbr void asm sideeffect "", "!i"() to label %ok [label %other]
ok:
  ret void
other:
  ret void
}
"#,
    );
    let v = w.check();
    assert!(has(&v, "`invoke`/`callbr` bị cấm"), "{v:?}");
}

/// G1 (hướng ĐỘT BIẾN `&&` → `||`): địa chỉ của hàm runtime làm ĐỐI SỐ của chính lời gọi tới nó cũng là dùng ngoài vai trò đích.
#[test]
fn runtime_function_address_as_an_argument_of_its_own_call_is_rejected() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let ptr = ctx.ptr_type(Default::default());
    let i64t = ctx.i64_type();
    let valid = w.m.add_function(
        "gritrt_utf8_valid",
        ctx.i32_type().fn_type(&[ptr.into(), i64t.into()], false),
        None,
    );
    w.b.build_call(
        valid,
        &[
            valid.as_global_value().as_pointer_value().into(),
            i64t.const_zero().into(),
        ],
        "r",
    )
    .expect("call");
    finish_ok(&w, f);
    let v = w.check();
    assert!(has(&v, "gritrt_utf8_valid"), "{v:?}");
}

/// Đối chứng: gọi TRỰC TIẾP hàm runtime trả số nguyên / void / con trỏ vẫn được chấp nhận (mọi loại người dùng của địa chỉ hàm).
#[test]
fn direct_calls_of_runtime_functions_returning_int_void_and_pointer_are_accepted() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let ptr = ctx.ptr_type(Default::default());
    let i64t = ctx.i64_type();
    let valid = w.m.add_function(
        "gritrt_utf8_valid",
        ctx.i32_type().fn_type(&[ptr.into(), i64t.into()], false),
        None,
    );
    w.b.build_call(
        valid,
        &[ptr.const_null().into(), i64t.const_zero().into()],
        "r",
    )
    .expect("call");
    let report = w.m.add_function(
        "gritrt_report_int",
        ctx.void_type().fn_type(&[i64t.into()], false),
        None,
    );
    w.b.build_call(report, &[i64t.const_zero().into()], "")
        .expect("call");
    let alloc =
        w.m.add_function("gritrt_alloc", ptr.fn_type(&[i64t.into()], false), None);
    w.b.build_call(alloc, &[i64t.const_int(8, false).into()], "p")
        .expect("call");
    finish_ok(&w, f);
    assert_eq!(w.check(), Vec::<String>::new());
}

/// G1 (đột biến "chỉ xét lần dùng ĐẦU TIÊN"): địa chỉ hàm runtime vừa được gọi trực tiếp vừa bị lưu — dù thứ tự nào trong danh sách dùng, lần dùng sai phải bị bắt.
#[test]
fn a_bad_use_is_caught_wherever_it_sits_among_the_uses_of_a_runtime_address() {
    for store_first in [true, false] {
        let ctx = Context::create();
        let w = world(&ctx, true, Some(ATT));
        let f = w.func("f");
        let ptr = ctx.ptr_type(Default::default());
        let i64t = ctx.i64_type();
        let alloc =
            w.m.add_function("gritrt_alloc", ptr.fn_type(&[i64t.into()], false), None);
        let slot = w.b.build_alloca(ptr, "slot").expect("alloca");
        let call = |tag: &str| {
            w.b.build_call(alloc, &[i64t.const_int(8, false).into()], tag)
                .expect("call");
        };
        if store_first {
            w.b.build_store(slot, alloc.as_global_value().as_pointer_value())
                .expect("store");
            call("a");
            call("b");
        } else {
            call("a");
            call("b");
            w.b.build_store(slot, alloc.as_global_value().as_pointer_value())
                .expect("store");
        }
        finish_ok(&w, f);
        let v = w.check();
        assert!(has(&v, "gritrt_alloc"), "store_first={store_first}: {v:?}");
    }
}

/// FB-C27 (đọc mã đối kháng nhóm E, E-4/FAIL_OPEN_AUDIT guard:57): luật địa chỉ chỉ áp cho `gritrt_*`; một khai báo NGOÀI khác (vd `@system`) có địa chỉ được lưu rồi
/// gọi gián tiếp thì không bị luật "đích mọi lời gọi". Quy tắc đúng: địa chỉ của MỌI hàm chỉ khai báo (trừ `llvm.*`) chỉ được dùng làm đích gọi trực tiếp.
#[test]
fn external_declarations_may_only_be_called_directly_fb_c27() {
    let ctx = Context::create();
    let w = world(&ctx, true, Some(ATT));
    let f = w.func("f");
    let ptr = ctx.ptr_type(Default::default());
    let sys_ty = ctx.i32_type().fn_type(&[ptr.into()], false);
    let sys = w.m.add_function("system", sys_ty, None);
    let slot = w.b.build_alloca(ptr, "slot").expect("alloca");
    w.b.build_store(slot, sys.as_global_value().as_pointer_value())
        .expect("store");
    let fp =
        w.b.build_load(ptr, slot, "fp")
            .expect("load")
            .into_pointer_value();
    w.b.build_indirect_call(sys_ty, fp, &[ptr.const_null().into()], "")
        .expect("indirect");
    finish_ok(&w, f);
    let v = w.check();
    assert!(
        has(&v, "system"),
        "địa chỉ của khai báo ngoài bị dùng gián tiếp mà được CHẤP NHẬN: {v:?}"
    );
}
