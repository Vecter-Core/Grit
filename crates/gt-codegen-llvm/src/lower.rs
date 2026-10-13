//! Hạ MIR → LLVM IR. Mọi local là một `alloca` (dev profile, không tối ưu): đơn giản, đúng, dễ đối chiếu với thông dịch.
//!
//! Đơn hình hoá: mỗi (hàm, đối số kiểu cụ thể) là một hàm LLVM; mọi kiểu trong thân được thế tham số trước khi hạ.
//! Con trỏ béo (`&[T]`, `&str`) = `{ ptr, i64 }`. Enum = `{ i64 tag, [k x i64] payload }`. `Mutex<T,L>` = `{ T, i64 khoá }`.
use crate::{CodegenError, abort_kind};
use gt_ast::NodeId;
use gt_llvm::inkwell;
use gt_mir::{
    AggregateKind, AssertKind, BinOp, Body, BodyKind, Callee, Const, Local, Operand, Place,
    PlaceElem, Rvalue, StatementKind, TerminatorKind, UnOp,
};
use gt_types::{AdtDef, AdtKind, IntTy, Lang, ParamId, Ty, TypesOutput};
use inkwell::AddressSpace;
use inkwell::AtomicOrdering;
use inkwell::AtomicRMWBinOp;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::intrinsics::Intrinsic;
use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetData, TargetMachine,
};
use inkwell::types::{
    BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, IntType, StructType,
};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, GlobalValue, IntValue,
    PointerValue,
};
use std::collections::{BTreeMap, BTreeSet};

type R<T> = Result<T, CodegenError>;

fn unsupported<T>(what: impl Into<String>) -> R<T> {
    Err(CodegenError::Unsupported(what.into()))
}

fn internal<T>(what: impl Into<String>) -> R<T> {
    Err(CodegenError::Internal(what.into()))
}

fn llvm<T, E: std::fmt::Display>(r: Result<T, E>) -> R<T> {
    r.map_err(|e| CodegenError::Internal(format!("LLVM: {e}")))
}

/// Tuỳ chọn biên dịch.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Gọi `gritrt_report_int` với kết quả `main` (số nguyên/bool) — chỉ để đối chiếu với thông dịch.
    pub report_result: bool,
    /// attestation (M9) của lần `check`; ghi vào `!gt.attest` và vào một hằng chuỗi của exe để IR Guard đối chiếu.
    pub attestation: String,
    /// CHÈN LỖI để kiểm IR Guard (`GRITC_FAULT=ir:…`): chỉ làm IR XẤU hơn ⇒ Guard phải từ chối; không bao giờ làm đầu ra tốt hơn.
    pub inject: Option<Inject>,
}

/// Lỗi cố ý chèn vào IR (chỉ để kiểm cổng).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inject {
    /// thêm một hàm có `unreachable` trần
    StrayUnreachable,
    /// bỏ metadata nghĩa vụ `!gt.obl` của thao tác nhạy cảm
    StripObligation,
    /// ghi attestation sai
    BadAttestation,
    /// thêm lời gọi tới một hàm `gritrt_*` KHÔNG có trong bảng runtime
    UnknownRuntimeCall,
    /// thêm lời gọi tới một hàm ngoài (không phải runtime/llvm)
    ExternalCall,
}

/// Kết quả: IR dạng chữ và object COFF.
#[derive(Debug, Clone)]
pub struct Compiled {
    pub ir: String,
    pub object: Vec<u8>,
}

/// Tìm `main` (cùng quy tắc với `gritc run`): thân hạng nhất `<tệp>::main`, không tham số.
fn find_main(bodies: &[Body]) -> R<&Body> {
    let c: Vec<&Body> = bodies
        .iter()
        .filter(|b| {
            b.kind == BodyKind::Fn
                && b.key.ends_with("::main")
                && b.key.matches("::").count() == 1
                && b.arg_count == 0
        })
        .collect();
    match c.as_slice() {
        [b] => Ok(b),
        [] => internal("không có hàm `main` không tham số ở cấp tệp"),
        _ => internal("có nhiều hàm `main`"),
    }
}

/// Một instance đơn hình hoá.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Inst {
    node: NodeId,
    args: Vec<Ty>,
}

/// Hàm không có thân MIR mà backend tự hạ (lang item / runtime).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Intr {
    Abort,
    SliceLen,
    Spawn,
    AtomicNew,
    AtomicLoad,
    AtomicStore,
    AtomicSwap,
    AtomicFetchAdd,
    AtomicFetchSub,
    AtomicCas,
    MutexNew,
    MutexWith,
    CellNew,
    CellGet,
    CellSet,
    /// `root_fs()`/`root_env()`/… của prelude: giá trị năng lực gốc (struct rỗng về nghĩa; quyền do hiệu ứng `io` kiểm ở M8)
    RootCap,
    EnvArg,
    FsRead,
    FileLen,
    FileBytes,
    OutU64,
    OutStr,
    Exit,
    FsOpen,
    ArenaNew,
    ArenaAlloc,
    ArenaReset,
}

struct Cg<'ctx, 'a> {
    ctx: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    td: TargetData,
    t: &'a TypesOutput,
    bodies: &'a [Body],
    keys: &'a BTreeMap<NodeId, String>,
    funcs: BTreeMap<Inst, FunctionValue<'ctx>>,
    pending: Vec<Inst>,
    abort_fn: FunctionValue<'ctx>,
    nfuncs: usize,
    /// static: node → (global, kiểu)
    statics: BTreeMap<NodeId, (GlobalValue<'ctx>, Ty)>,
    static_order: Vec<NodeId>,
    static_refs: BTreeMap<NodeId, BTreeSet<NodeId>>,
    strings: BTreeMap<String, GlobalValue<'ctx>>,
    inject: Option<Inject>,
    tramps: BTreeMap<Ty, FunctionValue<'ctx>>,
}

/// Biên dịch cả chương trình. Gọi SAU khi cổng phán quyết cho phép (xanh/vàng) — chính hàm này không kiểm lại M4–M9.
/// `bodies`: MIR SAU M7 (đã áp chế độ check); `keys`: tên đầy đủ theo NodeId (để nhận ra hàm prelude không có thân).
/// `guard`: IR Guard (gt-guard) — chạy SAU codegen, TRƯỚC khi phát object; trả danh sách vi phạm (rỗng = chấp nhận) ⇒ nếu có thì
/// `CodegenError::Guard` và KHÔNG có object.
pub fn compile(
    t: &TypesOutput,
    bodies: &[Body],
    keys: &BTreeMap<NodeId, String>,
    opts: &Options,
    guard: &dyn for<'c> Fn(&Module<'c>) -> Vec<String>,
) -> R<Compiled> {
    Target::initialize_x86(&InitializationConfig::default());
    let triple = TargetMachine::get_default_triple();
    let target = llvm(Target::from_triple(&triple))?;
    let Some(tm) = target.create_target_machine(
        &triple,
        "generic",
        "",
        OptimizationLevel::None,
        RelocMode::Default,
        CodeModel::Default,
    ) else {
        return internal("không tạo được TargetMachine");
    };
    let ctx = Context::create();
    let module = ctx.create_module("grit");
    module.set_triple(&triple);
    let td = tm.get_target_data();
    module.set_data_layout(&td.get_data_layout());
    let builder = ctx.create_builder();
    let abort_ty = ctx.void_type().fn_type(&[ctx.i32_type().into()], false);
    let abort_fn = module.add_function("gritrt_abort", abort_ty, None);
    abort_fn.add_attribute(AttributeLoc::Function, noreturn(&ctx));
    let mut cg = Cg {
        ctx: &ctx,
        module,
        builder,
        td,
        t,
        bodies,
        keys,
        funcs: BTreeMap::new(),
        pending: Vec::new(),
        abort_fn,
        nfuncs: 0,
        statics: BTreeMap::new(),
        static_order: Vec::new(),
        static_refs: BTreeMap::new(),
        strings: BTreeMap::new(),
        inject: opts.inject,
        tramps: BTreeMap::new(),
    };

    let main = find_main(bodies)?;
    let main_inst = Inst {
        node: main.node,
        args: Vec::new(),
    };
    let main_fn = cg.get_fn(&main_inst)?;
    // lặp tới khi hết hàng đợi VÀ mọi static đã có hàm khởi tạo
    let mut done_statics = 0;
    loop {
        while let Some(inst) = cg.pending.pop() {
            let body = cg.body_of(inst.node)?;
            cg.define_body(&inst, body)?;
        }
        if done_statics == cg.static_order.len() {
            break;
        }
        for i in done_statics..cg.static_order.len() {
            let node = cg.static_order[i];
            cg.get_fn(&Inst {
                node,
                args: Vec::new(),
            })?;
        }
        done_statics = cg.static_order.len();
    }
    cg.emit_entry(main, main_fn, opts)?;
    cg.emit_attestation(opts)?;
    if opts.inject == Some(Inject::StrayUnreachable) {
        let f = cg
            .module
            .add_function("gt.fault", ctx.void_type().fn_type(&[], false), None);
        let bb = ctx.append_basic_block(f, "bb0");
        cg.builder.position_at_end(bb);
        llvm(cg.builder.build_unreachable())?;
    }
    if matches!(
        opts.inject,
        Some(Inject::UnknownRuntimeCall | Inject::ExternalCall)
    ) {
        let name = if opts.inject == Some(Inject::ExternalCall) {
            "system"
        } else {
            "gritrt_exec"
        };
        let ext = cg
            .module
            .add_function(name, ctx.void_type().fn_type(&[], false), None);
        let f = cg
            .module
            .add_function("gt.fault2", ctx.void_type().fn_type(&[], false), None);
        let bb = ctx.append_basic_block(f, "bb0");
        cg.builder.position_at_end(bb);
        llvm(cg.builder.build_call(ext, &[], ""))?;
        llvm(cg.builder.build_return(None))?;
    }
    let violations = guard(&cg.module);
    if !violations.is_empty() {
        return Err(CodegenError::Guard(violations));
    }
    let ir = cg.module.print_to_string().to_string();
    let buf = llvm(tm.write_to_memory_buffer(&cg.module, FileType::Object))?;
    Ok(Compiled {
        ir,
        object: buf.as_slice().to_vec(),
    })
}

fn noreturn(ctx: &Context) -> Attribute {
    ctx.create_enum_attribute(Attribute::get_named_enum_kind_id("noreturn"), 0)
}

fn subst_inst(ty: &Ty, params: &[ParamId], args: &[Ty]) -> Ty {
    ty.subst(&|p| {
        params
            .iter()
            .position(|q| *q == p)
            .and_then(|i| args.get(i).cloned())
    })
}

/// Trạng thái riêng của một hàm đang hạ.
struct Fx<'ctx, 'b> {
    f: FunctionValue<'ctx>,
    body: &'b Body,
    params: Vec<ParamId>,
    args: Vec<Ty>,
    /// kiểu đã thế của mọi local
    local_tys: Vec<Ty>,
    locals: Vec<PointerValue<'ctx>>,
    blocks: Vec<BasicBlock<'ctx>>,
    traps: usize,
    /// local gán đúng một lần bằng aggregate không trường của `Ordering` ⇒ chỉ số biến thể (hằng lúc biên dịch)
    ord_consts: BTreeMap<usize, u32>,
    /// local tạm gán MỘT lần từ `Const::Item(static)` ⇒ là BÍ DANH của static (KHÔNG sao chép)
    static_alias: BTreeMap<usize, NodeId>,
    owner_static: Option<NodeId>,
}

/// Một chỗ đã tính: con trỏ tới ô nhớ + kiểu + (chỉ khi không kích thước) độ dài + biến thể đang xem.
struct PlaceVal<'ctx> {
    ptr: PointerValue<'ctx>,
    ty: Ty,
    meta: Option<IntValue<'ctx>>,
    variant: Option<u32>,
}

impl<'ctx, 'a> Cg<'ctx, 'a> {
    fn body_of(&self, node: NodeId) -> R<&'a Body> {
        let bodies: &'a [Body] = self.bodies;
        match bodies.iter().find(|b| b.node == node) {
            Some(b) => Ok(b),
            None => internal(format!("thiếu thân MIR cho node {node:?}")),
        }
    }

    fn has_body(&self, node: NodeId) -> bool {
        self.bodies.iter().any(|b| b.node == node)
    }

    fn int_type(&self, it: IntTy) -> IntType<'ctx> {
        self.iw(it.bits())
    }

    /// Số nguyên `bits` bit; `bits` luôn ≥ 1 (mọi kiểu số nguyên của Grit rộng ≥ 8 bit, bool = 1).
    fn iw(&self, bits: u32) -> IntType<'ctx> {
        let n = std::num::NonZeroU32::new(bits).expect("độ rộng số nguyên > 0");
        self.ctx
            .custom_width_int_type(n)
            .expect("độ rộng số nguyên hợp lệ")
    }

    fn i64t(&self) -> IntType<'ctx> {
        self.ctx.i64_type()
    }

    fn unit_type(&self) -> BasicTypeEnum<'ctx> {
        self.ctx.struct_type(&[], false).into()
    }

    fn ptr_type(&self) -> inkwell::types::PointerType<'ctx> {
        self.ctx.ptr_type(AddressSpace::default())
    }

    fn fat_type(&self) -> StructType<'ctx> {
        self.ctx
            .struct_type(&[self.ptr_type().into(), self.i64t().into()], false)
    }

    fn is_unsized(ty: &Ty) -> bool {
        matches!(ty, Ty::Str | Ty::Slice(_))
    }

    fn adt_subst(adt: &AdtDef, args: &[Ty], ty: &Ty) -> Ty {
        ty.subst(&|p| {
            adt.generics
                .iter()
                .position(|g| g.id == p)
                .and_then(|i| args.get(i).cloned())
        })
    }

    /// Kiểu của các trường của một biến thể (struct: biến thể 0) sau khi thế đối số.
    fn variant_fields(&self, def: gt_resolve::DefId, args: &[Ty], v: u32) -> R<Vec<Ty>> {
        let Some(adt) = self.t.db.adts.get(&def) else {
            return internal("ADT không có trong db");
        };
        if adt.generics.len() != args.len() {
            return internal("số đối số kiểu của ADT không khớp");
        }
        let raw: Vec<&Ty> = match &adt.kind {
            AdtKind::Struct(fs) => fs.iter().map(|f| &f.ty).collect(),
            AdtKind::Enum(vs) => match vs.get(v as usize) {
                Some(var) => var.fields.iter().collect(),
                None => return internal("biến thể không tồn tại"),
            },
        };
        Ok(raw.iter().map(|t| Self::adt_subst(adt, args, t)).collect())
    }

    fn variant_struct(&self, def: gt_resolve::DefId, args: &[Ty], v: u32) -> R<StructType<'ctx>> {
        let fs = self
            .variant_fields(def, args, v)?
            .iter()
            .map(|t| self.lower_ty(t))
            .collect::<R<Vec<_>>>()?;
        Ok(self.ctx.struct_type(&fs, false))
    }

    /// Kiểu LLVM của một kiểu Grit ĐÃ CỤ THỂ. Gặp thứ chưa hỗ trợ ⇒ `Unsupported`.
    fn lower_ty(&self, ty: &Ty) -> R<BasicTypeEnum<'ctx>> {
        Ok(match ty {
            Ty::Int(it) => self.int_type(*it).into(),
            Ty::Bool => self.ctx.bool_type().into(),
            Ty::Char => self.ctx.i32_type().into(),
            Ty::Never => self.unit_type(),
            Ty::Tuple(v) => {
                let fs = v.iter().map(|t| self.lower_ty(t)).collect::<R<Vec<_>>>()?;
                self.ctx.struct_type(&fs, false).into()
            }
            Ty::Array(e, n) => {
                let n = u32::try_from(*n).or_else(|_| unsupported("mảng quá lớn"))?;
                self.lower_ty(e)?.array_type(n).into()
            }
            Ty::Ref(_, inner) | Ty::RawPtr(_, inner) => {
                if Self::is_unsized(inner) {
                    self.fat_type().into()
                } else {
                    self.ptr_type().into()
                }
            }
            Ty::Fn(..) => self.ptr_type().into(),
            Ty::Adt(def, args) => {
                let Some(adt) = self.t.db.adts.get(def) else {
                    return internal("ADT không có trong db");
                };
                match (&adt.kind, adt.lang) {
                    (AdtKind::Struct(_), Some(Lang::Mutex)) => {
                        let fs = self.variant_fields(*def, args, 0)?;
                        let [t0] = fs.as_slice() else {
                            return internal("Mutex phải có đúng một trường");
                        };
                        // { T, khoá 8 byte } — trường `v` giữ chỉ số 0
                        self.ctx
                            .struct_type(&[self.lower_ty(t0)?, self.i64t().into()], false)
                            .into()
                    }
                    (AdtKind::Struct(_), _) => self.variant_struct(*def, args, 0)?.into(),
                    (AdtKind::Enum(vs), _) => {
                        let mut max = 0u64;
                        for v in 0..vs.len() {
                            let st = self.variant_struct(*def, args, v as u32)?;
                            if self.td.get_abi_alignment(&st) > 8 {
                                return unsupported("biến thể enum cần căn lề > 8");
                            }
                            max = max.max(self.td.get_abi_size(&st));
                        }
                        let k = u32::try_from(max.div_ceil(8))
                            .or_else(|_| unsupported("enum quá lớn"))?;
                        self.ctx
                            .struct_type(
                                &[self.i64t().into(), self.i64t().array_type(k).into()],
                                false,
                            )
                            .into()
                    }
                }
            }
            Ty::Float(_) => return unsupported("số thực"),
            Ty::Str | Ty::Slice(_) => return unsupported("kiểu không kích thước đứng riêng"),
            Ty::Param(_) | Ty::Infer(_) => return unsupported("tham số kiểu chưa được thế"),
            Ty::Err => return internal("Ty::Err sau cổng"),
        })
    }

    fn fn_type(&self, params: &[Ty], ret: &Ty) -> R<FunctionType<'ctx>> {
        let ps = params
            .iter()
            .map(|t| self.lower_ty(t).map(BasicMetadataTypeEnum::from))
            .collect::<R<Vec<_>>>()?;
        Ok(self.lower_ty(ret)?.fn_type(&ps, false))
    }

    /// Thứ tự tham số kiểu của một thân: generics của impl (nếu thành viên) rồi của hàm.
    fn params_of(&self, node: NodeId) -> Vec<ParamId> {
        for imp in &self.t.db.impls {
            for m in &imp.members {
                if m.node == node {
                    return imp
                        .generics
                        .iter()
                        .chain(m.generics.iter())
                        .map(|g| g.id)
                        .collect();
                }
            }
        }
        for (sig, _) in self.t.db.fns.values() {
            if sig.node == node {
                return sig.generics.iter().map(|g| g.id).collect();
            }
        }
        Vec::new()
    }

    /// Khai báo (lần đầu) một instance có thân MIR và xếp hàng để định nghĩa.
    fn get_fn(&mut self, inst: &Inst) -> R<FunctionValue<'ctx>> {
        if let Some(f) = self.funcs.get(inst) {
            return Ok(*f);
        }
        let body = self.body_of(inst.node)?;
        let params = self.params_of(body.sig_node);
        if params.len() != inst.args.len() {
            return unsupported(format!(
                "`{}`: số đối số kiểu ({}) không khớp số tham số ({}) — có thể là thành viên trait/mặc định",
                body.key,
                inst.args.len(),
                params.len()
            ));
        }
        let sub = |ty: &Ty| subst_inst(ty, &params, &inst.args);
        let ft = match body.kind {
            // thân hợp đồng `⊳`/`⊲` là hàm thuần (tham số của hàm [+ result]) → bool: hạ như hàm thường
            BodyKind::Fn | BodyKind::Pre | BodyKind::Post => {
                let ps: Vec<Ty> = (1..=body.arg_count)
                    .map(|i| sub(&body.locals[i].ty))
                    .collect();
                self.fn_type(&ps, &sub(body.return_ty()))?
            }
            BodyKind::Const | BodyKind::Static => self.fn_type(&[], &sub(body.return_ty()))?,
        };
        let name = format!("{}#{}", body.key, self.nfuncs);
        self.nfuncs += 1;
        let f = self.module.add_function(&name, ft, None);
        self.funcs.insert(inst.clone(), f);
        self.pending.push(inst.clone());
        Ok(f)
    }

    fn emit_entry(&mut self, main: &Body, main_fn: FunctionValue<'ctx>, opts: &Options) -> R<()> {
        let entry_ty = self.ctx.void_type().fn_type(&[], false);
        let entry = self.module.add_function("gt_entry", entry_ty, None);
        let bb = self.ctx.append_basic_block(entry, "entry");
        self.builder.position_at_end(bb);
        // khởi tạo static (theo thứ tự phụ thuộc) TRƯỚC main
        for node in self.static_init_order() {
            let g = self.statics[&node].0;
            let f = self.funcs[&Inst {
                node,
                args: Vec::new(),
            }];
            let call = llvm(self.builder.build_call(f, &[], "sinit"))?;
            let Some(v) = call.try_as_basic_value().basic() else {
                return internal("hàm khởi tạo static không trả giá trị");
            };
            llvm(self.builder.build_store(g.as_pointer_value(), v))?;
        }
        let r = llvm(self.builder.build_call(main_fn, &[], "main"))?;
        if opts.report_result {
            let value = r.try_as_basic_value().basic();
            let rep_ty = self
                .ctx
                .void_type()
                .fn_type(&[self.i64t().into(), self.ctx.i32_type().into()], false);
            let rep = self.module.add_function("gritrt_report_int", rep_ty, None);
            match main.return_ty() {
                Ty::Int(it) => {
                    let Some(BasicValueEnum::IntValue(iv)) = value else {
                        return internal("main không trả giá trị số nguyên");
                    };
                    if it.bits() > 64 {
                        return unsupported("báo kết quả main rộng hơn 64 bit");
                    }
                    let w = if it.signed() {
                        llvm(
                            self.builder
                                .build_int_s_extend_or_bit_cast(iv, self.i64t(), "w"),
                        )?
                    } else {
                        llvm(
                            self.builder
                                .build_int_z_extend_or_bit_cast(iv, self.i64t(), "w"),
                        )?
                    };
                    let s = self.ctx.i32_type().const_int(u64::from(it.signed()), false);
                    llvm(self.builder.build_call(rep, &[w.into(), s.into()], ""))?;
                }
                Ty::Bool => {
                    let Some(BasicValueEnum::IntValue(iv)) = value else {
                        return internal("main không trả bool");
                    };
                    let w = llvm(self.builder.build_int_z_extend(iv, self.i64t(), "w"))?;
                    let s = self.ctx.i32_type().const_zero();
                    llvm(self.builder.build_call(rep, &[w.into(), s.into()], ""))?;
                }
                t if t.is_unit() => {}
                other => return unsupported(format!("báo kết quả main kiểu {other:?}")),
            }
        }
        llvm(self.builder.build_return(None))?;

        let i32t = self.ctx.i32_type();
        let start_ty = i32t.fn_type(&[self.ptr_type().into()], false);
        let start = self.module.add_function("gritrt_start", start_ty, None);
        let cmain = self
            .module
            .add_function("main", i32t.fn_type(&[], false), None);
        let bb = self.ctx.append_basic_block(cmain, "entry");
        self.builder.position_at_end(bb);
        let r = llvm(self.builder.build_call(
            start,
            &[entry.as_global_value().as_pointer_value().into()],
            "r",
        ))?;
        let Some(rv) = r.try_as_basic_value().basic() else {
            return internal("gritrt_start không trả giá trị");
        };
        llvm(self.builder.build_return(Some(&rv)))?;
        Ok(())
    }

    /// Thứ tự khởi tạo static: phụ thuộc TRỰC TIẾP trước (DFS hậu thứ tự, ổn định theo thứ tự phát hiện).
    fn static_init_order(&self) -> Vec<NodeId> {
        fn visit(
            n: NodeId,
            refs: &BTreeMap<NodeId, BTreeSet<NodeId>>,
            seen: &mut BTreeSet<NodeId>,
            out: &mut Vec<NodeId>,
        ) {
            if !seen.insert(n) {
                return;
            }
            if let Some(ds) = refs.get(&n) {
                for d in ds {
                    visit(*d, refs, seen, out);
                }
            }
            out.push(n);
        }
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for n in &self.static_order {
            visit(*n, &self.static_refs, &mut seen, &mut out);
        }
        out
    }

    /// `!gt.attest` (module) + hằng chuỗi `gt.attestation` trong exe (để đối chiếu sau này).
    fn emit_attestation(&mut self, opts: &Options) -> R<()> {
        let att = if opts.inject == Some(Inject::BadAttestation) {
            "tampered".to_string()
        } else {
            opts.attestation.clone()
        };
        let node = self
            .ctx
            .metadata_node(&[self.ctx.metadata_string(&att).into()]);
        llvm(self.module.add_global_metadata("gt.attest", &node))?;
        let init = self.ctx.const_string(opts.attestation.as_bytes(), true);
        let g = self
            .module
            .add_global(init.get_type(), None, "gt.attestation");
        g.set_initializer(&init);
        g.set_constant(true);
        Ok(())
    }

    /// Gắn lớp nghĩa vụ (`!gt.obl`) cho một thao tác nhạy cảm (IR Guard đòi có).
    fn tag_obl(&self, inst: Option<inkwell::values::InstructionValue<'ctx>>, class: &str) -> R<()> {
        if self.inject == Some(Inject::StripObligation) {
            return Ok(());
        }
        let Some(inst) = inst else {
            return internal("thao tác nhạy cảm không phải lệnh");
        };
        let node = self
            .ctx
            .metadata_node(&[self.ctx.metadata_string(class).into()]);
        llvm(inst.set_metadata(node, self.ctx.get_kind_id("gt.obl")))
    }

    fn define_body(&mut self, inst: &Inst, body: &'a Body) -> R<()> {
        let f = self.funcs[inst];
        let params = self.params_of(body.sig_node);
        let local_tys: Vec<Ty> = body
            .locals
            .iter()
            .map(|l| subst_inst(&l.ty, &params, &inst.args))
            .collect();
        let mut fx = Fx {
            f,
            body,
            params,
            args: inst.args.clone(),
            local_tys,
            locals: Vec::new(),
            blocks: Vec::new(),
            traps: 0,
            ord_consts: BTreeMap::new(),
            static_alias: BTreeMap::new(),
            owner_static: (body.kind == BodyKind::Static).then_some(body.node),
        };
        self.prescan(&mut fx)?;
        let entry = self.ctx.append_basic_block(f, "alloca");
        self.builder.position_at_end(entry);
        for i in 0..body.locals.len() {
            if fx.static_alias.contains_key(&i) {
                // bí danh: không cần ô nhớ riêng (chỗ thật là global của static)
                fx.locals.push(self.ptr_type().const_null());
                continue;
            }
            let ty = self.lower_ty(&fx.local_tys[i])?;
            let p = llvm(self.builder.build_alloca(ty, &format!("l{i}")))?;
            fx.locals.push(p);
        }
        if matches!(body.kind, BodyKind::Fn | BodyKind::Pre | BodyKind::Post) {
            for i in 0..body.arg_count {
                let Some(arg) = f.get_nth_param(u32::try_from(i).unwrap_or(u32::MAX)) else {
                    return internal("thiếu tham số hàm");
                };
                llvm(self.builder.build_store(fx.locals[i + 1], arg))?;
            }
        }
        for i in 0..body.blocks.len() {
            fx.blocks
                .push(self.ctx.append_basic_block(f, &format!("bb{i}")));
        }
        llvm(self.builder.build_unconditional_branch(fx.blocks[0]))?;
        for (i, bd) in body.blocks.iter().enumerate() {
            self.builder.position_at_end(fx.blocks[i]);
            for s in &bd.statements {
                match &s.kind {
                    StatementKind::Assign(p, rv) => self.assign(&mut fx, p, rv)?,
                    StatementKind::StorageLive(_)
                    | StatementKind::StorageDead(_)
                    | StatementKind::FakeRead(..)
                    | StatementKind::Nop => {}
                }
            }
            let Some(term) = &bd.terminator else {
                return internal("khối MIR thiếu terminator");
            };
            self.terminator(&mut fx, &term.kind)?;
        }
        Ok(())
    }

    /// Quét thân: (a) local gán một lần bằng `Ordering::X` ⇒ hằng; (b) local TẠM gán một lần từ `Const::Item(static)` ⇒ bí danh.
    fn prescan(&mut self, fx: &mut Fx<'ctx, 'a>) -> R<()> {
        let body = fx.body;
        let mut assigns: BTreeMap<usize, usize> = BTreeMap::new();
        let ord_def = self.t.db.lang.get(&Lang::Ordering).copied();
        let mut ord: BTreeMap<usize, u32> = BTreeMap::new();
        let mut stat: BTreeMap<usize, NodeId> = BTreeMap::new();
        for bd in &body.blocks {
            for s in &bd.statements {
                let StatementKind::Assign(p, rv) = &s.kind else {
                    continue;
                };
                if p.projection.is_empty() {
                    *assigns.entry(p.local.index()).or_insert(0) += 1;
                    match rv {
                        Rvalue::Aggregate(AggregateKind::Adt(d, v, _), ops)
                            if ops.is_empty() && Some(*d) == ord_def =>
                        {
                            ord.insert(p.local.index(), *v);
                        }
                        Rvalue::Use(Operand::Const(Const::Item { node, .. }))
                            if self
                                .bodies
                                .iter()
                                .any(|b| b.node == *node && b.kind == BodyKind::Static) =>
                        {
                            stat.insert(p.local.index(), *node);
                        }
                        _ => {}
                    }
                } else {
                    // ghi vào một thành phần: không còn là "gán đúng một lần"
                    *assigns.entry(p.local.index()).or_insert(0) += 2;
                }
            }
            if let Some(t) = &bd.terminator
                && let TerminatorKind::Call { dest, .. } = &t.kind
            {
                *assigns.entry(dest.local.index()).or_insert(0) += 2;
            }
        }
        for (l, v) in ord {
            if assigns.get(&l) == Some(&1) {
                fx.ord_consts.insert(l, v);
            }
        }
        for (l, node) in stat {
            let is_temp = matches!(body.locals[l].kind, gt_mir::LocalKind::Temp);
            if assigns.get(&l) == Some(&1) && is_temp {
                fx.static_alias.insert(l, node);
                self.static_global(node)?;
                if let Some(owner) = fx.owner_static {
                    self.static_refs.entry(owner).or_default().insert(node);
                }
            }
        }
        Ok(())
    }

    /// Global của một static (tạo lần đầu, xếp hàng hàm khởi tạo).
    fn static_global(&mut self, node: NodeId) -> R<GlobalValue<'ctx>> {
        if let Some((g, _)) = self.statics.get(&node) {
            return Ok(*g);
        }
        let body = self.body_of(node)?;
        let ty = body.return_ty().clone();
        let lt = self.lower_ty(&ty)?;
        let name = format!("static.{}", body.key);
        let g = self.module.add_global(lt, None, &name);
        g.set_initializer(&lt.const_zero());
        self.statics.insert(node, (g, ty));
        self.static_order.push(node);
        Ok(g)
    }

    /// Khối `trap.N`: `call @gritrt_abort(kind)` rồi `unreachable` — chỗ DUY NHẤT sinh `unreachable` (ADR-029 §3).
    fn trap_block(&mut self, fx: &mut Fx<'ctx, '_>, kind: u64) -> R<BasicBlock<'ctx>> {
        let here = self.builder.get_insert_block();
        let bb = self
            .ctx
            .append_basic_block(fx.f, &format!("trap.{}", fx.traps));
        fx.traps += 1;
        self.builder.position_at_end(bb);
        let k = self.ctx.i32_type().const_int(kind, false);
        let call = llvm(self.builder.build_call(self.abort_fn, &[k.into()], ""))?;
        call.add_attribute(AttributeLoc::Function, noreturn(self.ctx));
        llvm(self.builder.build_unreachable())?;
        if let Some(h) = here {
            self.builder.position_at_end(h);
        }
        Ok(bb)
    }

    /// `if !cond { abort(kind) }` rồi tiếp tục ở khối `ok`.
    fn guard(&mut self, fx: &mut Fx<'ctx, '_>, ok_cond: IntValue<'ctx>, kind: u64) -> R<()> {
        let trap = self.trap_block(fx, kind)?;
        let ok = self.ctx.append_basic_block(fx.f, "guard.ok");
        llvm(self.builder.build_conditional_branch(ok_cond, ok, trap))?;
        self.builder.position_at_end(ok);
        Ok(())
    }

    // ---------------------------------------------------------------- chỗ (place) và toán hạng

    #[allow(unsafe_code)]
    fn gep(
        &self,
        elem: BasicTypeEnum<'ctx>,
        ptr: PointerValue<'ctx>,
        idx: &[IntValue<'ctx>],
    ) -> R<PointerValue<'ctx>> {
        // SAFETY: mọi chỉ số đi vào đây đã được MIR kiểm bằng `Assert` ngay trước (`Index`/`Subslice`), hoặc là hằng của bố cục;
        // IR Guard (bước 7) kiểm các `Assert` biên còn nguyên.
        llvm(unsafe { self.builder.build_in_bounds_gep(elem, ptr, idx, "gep") })
    }

    fn sub(fx: &Fx<'ctx, '_>, ty: &Ty) -> Ty {
        subst_inst(ty, &fx.params, &fx.args)
    }

    fn place(&mut self, fx: &Fx<'ctx, '_>, p: &Place) -> R<PlaceVal<'ctx>> {
        let mut pv = PlaceVal {
            ptr: fx.locals[p.local.index()],
            ty: fx.local_tys[p.local.index()].clone(),
            meta: None,
            variant: None,
        };
        if let Some(node) = fx.static_alias.get(&p.local.index()) {
            // bí danh: chỗ thật là global của static
            pv.ptr = self.static_global(*node)?.as_pointer_value();
            pv.ty = self.body_of(*node)?.return_ty().clone();
        }
        for e in &p.projection {
            match e {
                PlaceElem::Deref => {
                    let (inner, unsized_) = match &pv.ty {
                        Ty::Ref(_, i) | Ty::RawPtr(_, i) => ((**i).clone(), Self::is_unsized(i)),
                        other => return internal(format!("Deref trên {other:?}")),
                    };
                    if unsized_ {
                        let fat = llvm(self.builder.build_load(self.fat_type(), pv.ptr, "fat"))?;
                        let fv = fat.into_struct_value();
                        let ptr = llvm(self.builder.build_extract_value(fv, 0, "p"))?;
                        let len = llvm(self.builder.build_extract_value(fv, 1, "n"))?;
                        pv.ptr = ptr.into_pointer_value();
                        pv.meta = Some(len.into_int_value());
                    } else {
                        let v = llvm(self.builder.build_load(self.ptr_type(), pv.ptr, "deref"))?;
                        pv.ptr = v.into_pointer_value();
                        pv.meta = None;
                    }
                    pv.ty = inner;
                    pv.variant = None;
                }
                PlaceElem::Field(i, fty) => {
                    let fty = Self::sub(fx, fty);
                    match (&pv.ty, pv.variant) {
                        (Ty::Adt(def, args), Some(v)) => {
                            let (def, args) = (*def, args.clone());
                            let et = self.lower_ty(&pv.ty)?.into_struct_type();
                            let payload =
                                llvm(self.builder.build_struct_gep(et, pv.ptr, 1, "pay"))?;
                            let vs = self.variant_struct(def, &args, v)?;
                            pv.ptr = llvm(self.builder.build_struct_gep(vs, payload, *i, "vf"))?;
                        }
                        _ => {
                            let BasicTypeEnum::StructType(st) = self.lower_ty(&pv.ty)? else {
                                return internal("Field trên kiểu không phải struct");
                            };
                            pv.ptr = llvm(self.builder.build_struct_gep(st, pv.ptr, *i, "fld"))?;
                        }
                    }
                    pv.ty = fty;
                    pv.variant = None;
                    pv.meta = None;
                }
                PlaceElem::Index(l) => {
                    let idx = llvm(self.builder.build_load(
                        self.i64t(),
                        fx.locals[l.index()],
                        "idx",
                    ))?
                    .into_int_value();
                    match pv.ty.clone() {
                        Ty::Array(elem, _) => {
                            let at = self.lower_ty(&pv.ty)?;
                            pv.ptr = self.gep(at, pv.ptr, &[self.i64t().const_zero(), idx])?;
                            pv.ty = *elem;
                        }
                        Ty::Slice(elem) => {
                            let et = self.lower_ty(&elem)?;
                            pv.ptr = self.gep(et, pv.ptr, &[idx])?;
                            pv.ty = *elem;
                            pv.meta = None;
                        }
                        other => return unsupported(format!("chỉ số trên {other:?}")),
                    }
                }
                PlaceElem::Downcast(v) => pv.variant = Some(*v),
                PlaceElem::Subslice { from, to } => {
                    let (elem, len) = match pv.ty.clone() {
                        Ty::Array(e, n) => (*e, self.i64t().const_int(n, false)),
                        Ty::Slice(e) => match pv.meta {
                            Some(m) => (*e, m),
                            None => return internal("slice không có độ dài"),
                        },
                        other => return unsupported(format!("Subslice trên {other:?}")),
                    };
                    let f = llvm(self.builder.build_load(
                        self.i64t(),
                        fx.locals[from.index()],
                        "from",
                    ))?
                    .into_int_value();
                    let t = match to {
                        Some(l) => llvm(self.builder.build_load(
                            self.i64t(),
                            fx.locals[l.index()],
                            "to",
                        ))?
                        .into_int_value(),
                        None => len,
                    };
                    let et = self.lower_ty(&elem)?;
                    pv.ptr = self.gep(et, pv.ptr, &[f])?;
                    pv.meta = Some(llvm(self.builder.build_int_sub(t, f, "sublen"))?);
                    pv.ty = Ty::Slice(Box::new(elem));
                }
            }
        }
        Ok(pv)
    }

    fn load_place(&mut self, pv: &PlaceVal<'ctx>) -> R<BasicValueEnum<'ctx>> {
        if Self::is_unsized(&pv.ty) {
            return unsupported("đọc giá trị không kích thước");
        }
        let lt = self.lower_ty(&pv.ty)?;
        llvm(self.builder.build_load(lt, pv.ptr, "ld"))
    }

    fn operand(&mut self, fx: &mut Fx<'ctx, '_>, o: &Operand) -> R<(BasicValueEnum<'ctx>, Ty)> {
        match o {
            Operand::Copy(p) | Operand::Move(p) => {
                let pv = self.place(fx, p)?;
                let v = self.load_place(&pv)?;
                Ok((v, pv.ty))
            }
            Operand::Const(c) => self.constant(fx, c),
        }
    }

    fn const_int(&self, it: IntType<'ctx>, bits: u128) -> IntValue<'ctx> {
        let lo = bits as u64;
        let hi = (bits >> 64) as u64;
        if it.get_bit_width() <= 64 {
            it.const_int(lo, false)
        } else {
            it.const_int_arbitrary_precision(&[lo, hi])
        }
    }

    fn string_global(&mut self, s: &str) -> GlobalValue<'ctx> {
        if let Some(g) = self.strings.get(s) {
            return *g;
        }
        let init = self.ctx.const_string(s.as_bytes(), false);
        let g = self.module.add_global(
            init.get_type(),
            None,
            &format!("str.{}", self.strings.len()),
        );
        g.set_initializer(&init);
        g.set_constant(true);
        g.set_unnamed_addr(true);
        self.strings.insert(s.to_string(), g);
        g
    }

    fn constant(&mut self, fx: &mut Fx<'ctx, '_>, c: &Const) -> R<(BasicValueEnum<'ctx>, Ty)> {
        Ok(match c {
            Const::Int { bits, ty } => (
                self.const_int(self.int_type(*ty), *bits).into(),
                Ty::Int(*ty),
            ),
            Const::Bool(b) => (
                self.ctx.bool_type().const_int(u64::from(*b), false).into(),
                Ty::Bool,
            ),
            Const::Char(ch) => (
                self.ctx.i32_type().const_int(u64::from(*ch), false).into(),
                Ty::Char,
            ),
            Const::Unit => (self.unit_type().const_zero(), Ty::unit()),
            Const::Str(s) => {
                let g = self.string_global(s);
                let fat = self.fat_type().get_undef();
                let fat =
                    llvm(
                        self.builder
                            .build_insert_value(fat, g.as_pointer_value(), 0, "s0"),
                    )?;
                let len = self.i64t().const_int(s.len() as u64, false);
                let fat = llvm(self.builder.build_insert_value(fat, len, 1, "s1"))?;
                (fat.as_basic_value_enum(), Ty::Ref(false, Box::new(Ty::Str)))
            }
            Const::FnItem { node, generics, ty } => {
                let args: Vec<Ty> = generics.iter().map(|g| Self::sub(fx, g)).collect();
                if !self.has_body(*node) {
                    return unsupported("giá trị hàm không có thân MIR (hàm của lang/runtime)");
                }
                let f = self.get_fn(&Inst { node: *node, args })?;
                (
                    f.as_global_value().as_pointer_value().into(),
                    Self::sub(fx, ty),
                )
            }
            Const::Item { node, ty } => {
                let ty = Self::sub(fx, ty);
                let body = self.body_of(*node)?;
                match body.kind {
                    BodyKind::Const => {
                        let f = self.get_fn(&Inst {
                            node: *node,
                            args: Vec::new(),
                        })?;
                        let c = llvm(self.builder.build_call(f, &[], "const"))?;
                        let Some(v) = c.try_as_basic_value().basic() else {
                            return internal("const không trả giá trị");
                        };
                        (v, ty)
                    }
                    BodyKind::Static => {
                        // dùng giá trị của static không qua bí danh: CHỈ cho kiểu không có mutability bên trong (đọc một bản sao)
                        if self.has_interior_mut(&ty) {
                            return unsupported(
                                "sao chép giá trị của static có mutability bên trong (Atomic/Mutex/Cell)",
                            );
                        }
                        let g = self.static_global(*node)?;
                        if let Some(owner) = fx.owner_static {
                            self.static_refs.entry(owner).or_default().insert(*node);
                        }
                        let lt = self.lower_ty(&ty)?;
                        let v = llvm(self.builder.build_load(lt, g.as_pointer_value(), "sld"))?;
                        (v, ty)
                    }
                    _ => return internal("Const::Item không phải const/static"),
                }
            }
            Const::Float { .. } => return unsupported("literal số thực"),
        })
    }

    fn has_interior_mut(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Adt(d, args) => {
                let Some(adt) = self.t.db.adts.get(d) else {
                    return true;
                };
                if adt.lang.is_some_and(Lang::interior_mutable) {
                    return true;
                }
                let nvars = match &adt.kind {
                    AdtKind::Struct(_) => 1,
                    AdtKind::Enum(vs) => vs.len(),
                };
                (0..nvars).any(|v| {
                    self.variant_fields(*d, args, v as u32)
                        .is_ok_and(|fs| fs.iter().any(|t| self.has_interior_mut(t)))
                })
            }
            Ty::Tuple(v) => v.iter().any(|t| self.has_interior_mut(t)),
            Ty::Array(t, _) | Ty::Slice(t) => self.has_interior_mut(t),
            Ty::Param(_) | Ty::Infer(_) | Ty::Err => true,
            _ => false,
        }
    }

    // ---------------------------------------------------------------- câu lệnh

    fn assign(&mut self, fx: &mut Fx<'ctx, '_>, dest: &Place, rv: &Rvalue) -> R<()> {
        if dest.projection.is_empty() && fx.static_alias.contains_key(&dest.local.index()) {
            // bí danh: KHÔNG sao chép static
            return Ok(());
        }
        match rv {
            Rvalue::Aggregate(kind, ops) => {
                let dp = self.place(fx, dest)?;
                return self.aggregate(fx, dp.ptr, &dp.ty, kind, ops);
            }
            Rvalue::Repeat(op, n) => {
                let dp = self.place(fx, dest)?;
                return self.repeat(fx, dp.ptr, &dp.ty, op, *n);
            }
            _ => {}
        }
        let v = self.rvalue(fx, rv)?;
        let dp = self.place(fx, dest)?;
        llvm(self.builder.build_store(dp.ptr, v))?;
        Ok(())
    }

    fn aggregate(
        &mut self,
        fx: &mut Fx<'ctx, '_>,
        dp: PointerValue<'ctx>,
        dty: &Ty,
        kind: &AggregateKind,
        ops: &[Operand],
    ) -> R<()> {
        let ct = self.lower_ty(dty)?;
        // (con trỏ gốc của các trường, kiểu struct chứa chúng)
        let (base, st): (PointerValue<'ctx>, Option<StructType<'ctx>>) = match kind {
            AggregateKind::Tuple => (dp, Some(ct.into_struct_type())),
            AggregateKind::Array(_) => (dp, None),
            AggregateKind::Adt(def, var, args) => {
                let args: Vec<Ty> = args.iter().map(|a| Self::sub(fx, a)).collect();
                let Some(adt) = self.t.db.adts.get(def) else {
                    return internal("ADT không có trong db");
                };
                match &adt.kind {
                    AdtKind::Struct(_) => (dp, Some(ct.into_struct_type())),
                    AdtKind::Enum(_) => {
                        let et = ct.into_struct_type();
                        let tag = llvm(self.builder.build_struct_gep(et, dp, 0, "tag"))?;
                        llvm(
                            self.builder
                                .build_store(tag, self.i64t().const_int(u64::from(*var), false)),
                        )?;
                        let pay = llvm(self.builder.build_struct_gep(et, dp, 1, "pay"))?;
                        (pay, Some(self.variant_struct(*def, &args, *var)?))
                    }
                }
            }
        };
        for (i, op) in ops.iter().enumerate() {
            let (v, _) = self.operand(fx, op)?;
            let slot = match (kind, st) {
                (AggregateKind::Array(_), _) => {
                    let ix = self.i64t().const_int(i as u64, false);
                    self.gep(ct, base, &[self.i64t().const_zero(), ix])?
                }
                (_, Some(st)) => llvm(self.builder.build_struct_gep(
                    st,
                    base,
                    u32::try_from(i).unwrap_or(u32::MAX),
                    "agg",
                ))?,
                _ => return internal("Aggregate trên kiểu không khớp"),
            };
            llvm(self.builder.build_store(slot, v))?;
        }
        Ok(())
    }

    fn repeat(
        &mut self,
        fx: &mut Fx<'ctx, '_>,
        dp: PointerValue<'ctx>,
        dty: &Ty,
        op: &Operand,
        n: u64,
    ) -> R<()> {
        let at = self.lower_ty(dty)?;
        let (v, _) = self.operand(fx, op)?;
        let i64t = self.i64t();
        let counter = llvm(self.builder.build_alloca(i64t, "rep.i"))?;
        llvm(self.builder.build_store(counter, i64t.const_zero()))?;
        let cond = self.ctx.append_basic_block(fx.f, "rep.cond");
        let body = self.ctx.append_basic_block(fx.f, "rep.body");
        let end = self.ctx.append_basic_block(fx.f, "rep.end");
        llvm(self.builder.build_unconditional_branch(cond))?;
        self.builder.position_at_end(cond);
        let i = llvm(self.builder.build_load(i64t, counter, "i"))?.into_int_value();
        let lt = llvm(self.builder.build_int_compare(
            IntPredicate::ULT,
            i,
            i64t.const_int(n, false),
            "lt",
        ))?;
        llvm(self.builder.build_conditional_branch(lt, body, end))?;
        self.builder.position_at_end(body);
        let slot = self.gep(at, dp, &[i64t.const_zero(), i])?;
        llvm(self.builder.build_store(slot, v))?;
        let next = llvm(
            self.builder
                .build_int_add(i, i64t.const_int(1, false), "next"),
        )?;
        llvm(self.builder.build_store(counter, next))?;
        llvm(self.builder.build_unconditional_branch(cond))?;
        self.builder.position_at_end(end);
        Ok(())
    }

    fn rvalue(&mut self, fx: &mut Fx<'ctx, '_>, rv: &Rvalue) -> R<BasicValueEnum<'ctx>> {
        match rv {
            Rvalue::Use(o) => Ok(self.operand(fx, o)?.0),
            Rvalue::Ref(_, _, p) => {
                let pv = self.place(fx, p)?;
                if Self::is_unsized(&pv.ty) {
                    let Some(len) = pv.meta else {
                        return internal("mượn chỗ không kích thước mà không có độ dài");
                    };
                    let fat = self.fat_type().get_undef();
                    let fat = llvm(self.builder.build_insert_value(fat, pv.ptr, 0, "f0"))?;
                    let fat = llvm(self.builder.build_insert_value(fat, len, 1, "f1"))?;
                    Ok(fat.as_basic_value_enum())
                } else {
                    Ok(pv.ptr.into())
                }
            }
            Rvalue::Len(p) => {
                let pv = self.place(fx, p)?;
                match pv.ty {
                    Ty::Array(_, n) => Ok(self.i64t().const_int(n, false).into()),
                    Ty::Slice(_) | Ty::Str => match pv.meta {
                        Some(m) => Ok(m.into()),
                        None => internal("slice không có độ dài"),
                    },
                    other => unsupported(format!("Len của {other:?}")),
                }
            }
            Rvalue::Cast(o, to) => {
                let (v, from) = self.operand(fx, o)?;
                let to = Self::sub(fx, to);
                self.cast(v, &from, &to)
            }
            Rvalue::CastFits(o, to) => {
                let (v, from) = self.operand(fx, o)?;
                let to = Self::sub(fx, to);
                self.cast_fits(v, &from, &to)
            }
            Rvalue::UnaryOp(op, o) => {
                let (v, ty) = self.operand(fx, o)?;
                let iv = v.into_int_value();
                match (op, &ty) {
                    (UnOp::Not, Ty::Bool) => Ok(llvm(self.builder.build_not(iv, "not"))?.into()),
                    (UnOp::Neg, Ty::Int(_)) => {
                        Ok(llvm(self.builder.build_int_neg(iv, "neg"))?.into())
                    }
                    _ => unsupported(format!("phép một ngôi {op:?} trên {ty:?}")),
                }
            }
            Rvalue::BinaryOp(op, a, b) => {
                let (x, ty) = self.operand(fx, a)?;
                let (y, _) = self.operand(fx, b)?;
                self.binop(fx, *op, x, y, &ty)
            }
            Rvalue::CheckedBinaryOp(op, a, b) => {
                let (x, ty) = self.operand(fx, a)?;
                let (y, _) = self.operand(fx, b)?;
                self.checked(*op, x, y, &ty)
            }
            Rvalue::Discriminant(p) => {
                let pv = self.place(fx, p)?;
                let et = self.lower_ty(&pv.ty)?.into_struct_type();
                let tag = llvm(self.builder.build_struct_gep(et, pv.ptr, 0, "tag"))?;
                llvm(self.builder.build_load(self.i64t(), tag, "disc"))
            }
            Rvalue::Aggregate(..) | Rvalue::Repeat(..) => {
                internal("Aggregate/Repeat đi đường riêng")
            }
        }
    }

    fn signed_of(ty: &Ty) -> R<bool> {
        match ty {
            Ty::Int(it) => Ok(it.signed()),
            Ty::Bool | Ty::Char => Ok(false),
            other => unsupported(format!("phép số học trên {other:?}")),
        }
    }

    fn binop(
        &mut self,
        fx: &mut Fx<'ctx, '_>,
        op: BinOp,
        x: BasicValueEnum<'ctx>,
        y: BasicValueEnum<'ctx>,
        ty: &Ty,
    ) -> R<BasicValueEnum<'ctx>> {
        let (BasicValueEnum::IntValue(a), BasicValueEnum::IntValue(b)) = (x, y) else {
            return unsupported(format!("phép hai ngôi {op:?} trên {ty:?}"));
        };
        let signed = Self::signed_of(ty)?;
        let v: IntValue<'ctx> = match op {
            BinOp::Add | BinOp::AddWrap => llvm(self.builder.build_int_add(a, b, "add"))?,
            BinOp::Sub | BinOp::SubWrap => llvm(self.builder.build_int_sub(a, b, "sub"))?,
            BinOp::Mul | BinOp::MulWrap => {
                if a.get_type().get_bit_width() > 64 {
                    return unsupported("nhân 128 bit (cần __multi3)");
                }
                llvm(self.builder.build_int_mul(a, b, "mul"))?
            }
            BinOp::Div | BinOp::Rem => {
                let bits = a.get_type().get_bit_width();
                if bits > 64 {
                    return unsupported("chia 128 bit (cần __divti3)");
                }
                // LLVM: chia 0 và MIN / -1 là UB ⇒ chặn tại chỗ như MỘT LỚP PHÒNG THỦ, dù MIR đã có Assert (D3).
                let zero = a.get_type().const_zero();
                let nz = llvm(
                    self.builder
                        .build_int_compare(IntPredicate::NE, b, zero, "nz"),
                )?;
                self.guard(
                    fx,
                    nz,
                    if op == BinOp::Rem {
                        abort_kind::REM_ZERO
                    } else {
                        abort_kind::DIV_ZERO
                    },
                )?;
                if signed {
                    let min = a.get_type().const_int(1u64 << (bits - 1), false);
                    let minus1 = a.get_type().const_all_ones();
                    let is_min =
                        llvm(
                            self.builder
                                .build_int_compare(IntPredicate::EQ, a, min, "ismin"),
                        )?;
                    let is_m1 =
                        llvm(
                            self.builder
                                .build_int_compare(IntPredicate::EQ, b, minus1, "ism1"),
                        )?;
                    let bad = llvm(self.builder.build_and(is_min, is_m1, "bad"))?;
                    let ok = llvm(self.builder.build_not(bad, "okd"))?;
                    self.guard(fx, ok, abort_kind::OVERFLOW)?;
                }
                match (op, signed) {
                    (BinOp::Div, true) => llvm(self.builder.build_int_signed_div(a, b, "sdiv"))?,
                    (BinOp::Div, false) => llvm(self.builder.build_int_unsigned_div(a, b, "udiv"))?,
                    (_, true) => llvm(self.builder.build_int_signed_rem(a, b, "srem"))?,
                    (_, false) => llvm(self.builder.build_int_unsigned_rem(a, b, "urem"))?,
                }
            }
            BinOp::AddSat | BinOp::SubSat => {
                let name = format!(
                    "llvm.{}{}.sat",
                    if signed { 's' } else { 'u' },
                    if op == BinOp::AddSat { "add" } else { "sub" }
                );
                self.call_intrinsic(&name, a, b)?.into_int_value()
            }
            BinOp::MulSat => return unsupported("nhân bão hoà"),
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let pred = match (op, signed) {
                    (BinOp::Eq, _) => IntPredicate::EQ,
                    (BinOp::Ne, _) => IntPredicate::NE,
                    (BinOp::Lt, true) => IntPredicate::SLT,
                    (BinOp::Lt, false) => IntPredicate::ULT,
                    (BinOp::Le, true) => IntPredicate::SLE,
                    (BinOp::Le, false) => IntPredicate::ULE,
                    (BinOp::Gt, true) => IntPredicate::SGT,
                    (BinOp::Gt, false) => IntPredicate::UGT,
                    (_, true) => IntPredicate::SGE,
                    (_, false) => IntPredicate::UGE,
                };
                llvm(self.builder.build_int_compare(pred, a, b, "cmp"))?
            }
            BinOp::BitAnd => llvm(self.builder.build_and(a, b, "and"))?,
        };
        Ok(v.into())
    }

    fn call_intrinsic(
        &self,
        name: &str,
        a: IntValue<'ctx>,
        b: IntValue<'ctx>,
    ) -> R<BasicValueEnum<'ctx>> {
        let Some(intr) = Intrinsic::find(name) else {
            return internal(format!("không có intrinsic {name}"));
        };
        let Some(decl) = intr.get_declaration(&self.module, &[a.get_type().into()]) else {
            return internal(format!("không khai báo được {name}"));
        };
        let call = llvm(self.builder.build_call(decl, &[a.into(), b.into()], "i"))?;
        match call.try_as_basic_value().basic() {
            Some(v) => Ok(v),
            None => internal("intrinsic không trả giá trị"),
        }
    }

    fn checked(
        &mut self,
        op: BinOp,
        x: BasicValueEnum<'ctx>,
        y: BasicValueEnum<'ctx>,
        ty: &Ty,
    ) -> R<BasicValueEnum<'ctx>> {
        let (BasicValueEnum::IntValue(a), BasicValueEnum::IntValue(b)) = (x, y) else {
            return unsupported("phép kiểm tràn trên kiểu không phải số nguyên");
        };
        let signed = Self::signed_of(ty)?;
        let which = match op {
            BinOp::Add => "add",
            BinOp::Sub => "sub",
            BinOp::Mul => {
                if a.get_type().get_bit_width() > 64 {
                    return unsupported("nhân có kiểm tràn 128 bit (cần __muloti4)");
                }
                "mul"
            }
            other => return unsupported(format!("phép kiểm tràn {other:?}")),
        };
        let name = format!(
            "llvm.{}{which}.with.overflow",
            if signed { 's' } else { 'u' }
        );
        self.call_intrinsic(&name, a, b)
    }

    fn int_bits_signed(ty: &Ty) -> Option<(u32, bool)> {
        match ty {
            Ty::Int(it) => Some((it.bits(), it.signed())),
            Ty::Bool => Some((1, false)),
            Ty::Char => Some((32, false)),
            _ => None,
        }
    }

    fn cast(&mut self, v: BasicValueEnum<'ctx>, from: &Ty, to: &Ty) -> R<BasicValueEnum<'ctx>> {
        // ép `&[T; N]` → `&[T]` (unsize): con trỏ béo { ptr, N }
        if let (Ty::Ref(_, a), Ty::Ref(_, b)) = (from, to)
            && let (Ty::Array(ea, n), Ty::Slice(eb)) = (&**a, &**b)
            && ea == eb
        {
            let fat = self.fat_type().get_undef();
            let fat = llvm(
                self.builder
                    .build_insert_value(fat, v.into_pointer_value(), 0, "u0"),
            )?;
            let len = self.i64t().const_int(*n, false);
            let fat = llvm(self.builder.build_insert_value(fat, len, 1, "u1"))?;
            return Ok(fat.as_basic_value_enum());
        }
        let (Some((fb, fs)), Ty::Int(ti)) = (Self::int_bits_signed(from), to) else {
            return unsupported(format!("ép kiểu {from:?} → {to:?}"));
        };
        let iv = v.into_int_value();
        let tt = self.int_type(*ti);
        let tb = ti.bits();
        let r = if tb < fb {
            llvm(self.builder.build_int_truncate(iv, tt, "trunc"))?
        } else if tb > fb {
            if fs {
                llvm(self.builder.build_int_s_extend(iv, tt, "sext"))?
            } else {
                llvm(self.builder.build_int_z_extend(iv, tt, "zext"))?
            }
        } else {
            iv
        };
        Ok(r.into())
    }

    /// `x as T` không mất thông tin ⇔ sau khi nới cả `x` và `x as T` lên cùng độ rộng (theo dấu của riêng từng giá trị) thì bằng nhau.
    fn cast_fits(
        &mut self,
        v: BasicValueEnum<'ctx>,
        from: &Ty,
        to: &Ty,
    ) -> R<BasicValueEnum<'ctx>> {
        let (Some((fb, fs)), Ty::Int(ti)) = (Self::int_bits_signed(from), to) else {
            return unsupported(format!("kiểm ép kiểu {from:?} → {to:?}"));
        };
        if matches!(from, Ty::Bool) {
            return Ok(self.ctx.bool_type().const_int(1, false).into());
        }
        let y = self.cast(v, from, to)?.into_int_value();
        let w = self.iw(fb.max(ti.bits()) + 1);
        let xw = if fs {
            llvm(self.builder.build_int_s_extend(v.into_int_value(), w, "xw"))?
        } else {
            llvm(self.builder.build_int_z_extend(v.into_int_value(), w, "xw"))?
        };
        let yw = if ti.signed() {
            llvm(self.builder.build_int_s_extend(y, w, "yw"))?
        } else {
            llvm(self.builder.build_int_z_extend(y, w, "yw"))?
        };
        Ok(llvm(
            self.builder
                .build_int_compare(IntPredicate::EQ, xw, yw, "fits"),
        )?
        .into())
    }

    // ---------------------------------------------------------------- terminator

    fn terminator(&mut self, fx: &mut Fx<'ctx, '_>, k: &TerminatorKind) -> R<()> {
        match k {
            TerminatorKind::Goto(b) => {
                llvm(
                    self.builder
                        .build_unconditional_branch(fx.blocks[b.index()]),
                )?;
            }
            TerminatorKind::Drop { target, .. } => {
                // Phase 1: không có destructor người dùng (cùng ngữ nghĩa thông dịch; KNOWN_LIMITATIONS)
                llvm(
                    self.builder
                        .build_unconditional_branch(fx.blocks[target.index()]),
                )?;
            }
            TerminatorKind::Return => {
                let lt = self.lower_ty(&fx.local_tys[0])?;
                let v = llvm(self.builder.build_load(lt, fx.locals[0], "ret"))?;
                llvm(self.builder.build_return(Some(&v)))?;
            }
            TerminatorKind::Unreachable => {
                let trap = self.trap_block(fx, abort_kind::UNREACHABLE)?;
                llvm(self.builder.build_unconditional_branch(trap))?;
            }
            TerminatorKind::SwitchInt {
                discr,
                targets,
                otherwise,
            } => {
                let (v, _) = self.operand(fx, discr)?;
                let BasicValueEnum::IntValue(iv) = v else {
                    return internal("SwitchInt trên giá trị không phải số nguyên");
                };
                let cases: Vec<(IntValue<'ctx>, BasicBlock<'ctx>)> = targets
                    .iter()
                    .map(|(k, b)| (self.const_int(iv.get_type(), *k), fx.blocks[b.index()]))
                    .collect();
                llvm(
                    self.builder
                        .build_switch(iv, fx.blocks[otherwise.index()], &cases),
                )?;
            }
            TerminatorKind::Assert {
                cond,
                expected,
                kind,
                target,
            } => {
                let (v, _) = self.operand(fx, cond)?;
                let BasicValueEnum::IntValue(iv) = v else {
                    return internal("Assert trên giá trị không phải bool");
                };
                let trap = self.trap_block(fx, Self::assert_kind(kind))?;
                let ok = fx.blocks[target.index()];
                let (t, f) = if *expected { (ok, trap) } else { (trap, ok) };
                llvm(self.builder.build_conditional_branch(iv, t, f))?;
            }
            TerminatorKind::Call {
                callee,
                args,
                dest,
                target,
                ..
            } => self.call(fx, callee, args, dest, *target)?,
        }
        Ok(())
    }

    fn assert_kind(k: &AssertKind) -> u64 {
        match k {
            AssertKind::Overflow(..) => abort_kind::OVERFLOW,
            AssertKind::OverflowNeg(_) => abort_kind::OVERFLOW_NEG,
            AssertKind::DivisionByZero(_) => abort_kind::DIV_ZERO,
            AssertKind::RemainderByZero(_) => abort_kind::REM_ZERO,
            AssertKind::IndexOutOfBounds { .. } => abort_kind::BOUNDS,
            AssertKind::SliceRange { .. } => abort_kind::SLICE_RANGE,
            AssertKind::CastLossy { .. } => abort_kind::CAST_LOSSY,
            AssertKind::Precondition => abort_kind::PRECONDITION,
            AssertKind::Postcondition => abort_kind::POSTCONDITION,
            AssertKind::Invariant => abort_kind::INVARIANT,
        }
    }

    fn intr_of(&self, key: &str) -> Option<Intr> {
        let m = key.rsplit("::").next().unwrap_or("");
        if key.ends_with("core::abort") {
            return Some(Intr::Abort);
        }
        if key == "$prelude::[T0]::len" {
            return Some(Intr::SliceLen);
        }
        if key == "$prelude::spawn" {
            return Some(Intr::Spawn);
        }
        if key.starts_with("$prelude::root_") {
            return Some(Intr::RootCap);
        }
        if key.contains("Cap<Env>") && m == "arg" {
            return Some(Intr::EnvArg);
        }
        if key.contains("Cap<Fs>") && m == "read" {
            return Some(Intr::FsRead);
        }
        if key.ends_with("core::exit") {
            return Some(Intr::Exit);
        }
        if key.contains("Cap<Fs>") && m == "open" {
            return Some(Intr::FsOpen);
        }
        if key.contains("Arena") && m == "new" || key.contains("Cap<Alloc>") && m == "arena" {
            return Some(Intr::ArenaNew);
        }
        if key.contains("Arena") && m == "alloc" {
            return Some(Intr::ArenaAlloc);
        }
        if key.contains("Arena") && m == "reset" {
            return Some(Intr::ArenaReset);
        }
        if key.contains("File") && m == "len" {
            return Some(Intr::FileLen);
        }
        if key.contains("File") && m == "bytes" {
            return Some(Intr::FileBytes);
        }
        if key.contains("Cap<Out>") && m == "write_u64" {
            return Some(Intr::OutU64);
        }
        if key.contains("Cap<Out>") && m == "write_str" {
            return Some(Intr::OutStr);
        }
        let is = |n: &str| key.contains(n);
        Some(match (m, is("Atomic"), is("Mutex"), is("Cell")) {
            ("new", true, _, _) => Intr::AtomicNew,
            ("load", true, _, _) => Intr::AtomicLoad,
            ("store", true, _, _) => Intr::AtomicStore,
            ("swap", true, _, _) => Intr::AtomicSwap,
            ("fetch_add", true, _, _) => Intr::AtomicFetchAdd,
            ("fetch_sub", true, _, _) => Intr::AtomicFetchSub,
            ("compare_exchange", true, _, _) => Intr::AtomicCas,
            ("new", _, true, _) => Intr::MutexNew,
            ("with", _, true, _) => Intr::MutexWith,
            ("new", _, _, true) => Intr::CellNew,
            ("get", _, _, true) => Intr::CellGet,
            ("set", _, _, true) => Intr::CellSet,
            _ => return None,
        })
    }

    fn call(
        &mut self,
        fx: &mut Fx<'ctx, '_>,
        callee: &Callee,
        args: &[Operand],
        dest: &Place,
        target: Option<gt_mir::BasicBlock>,
    ) -> R<()> {
        let mut vals: Vec<BasicValueEnum<'ctx>> = Vec::new();
        let mut tys: Vec<Ty> = Vec::new();
        for a in args {
            let (v, t) = self.operand(fx, a)?;
            vals.push(v);
            tys.push(t);
        }
        let result: Option<BasicValueEnum<'ctx>> = match callee {
            Callee::Fn { node, generics } => {
                let gargs: Vec<Ty> = generics.iter().map(|g| Self::sub(fx, g)).collect();
                if self.has_body(*node) {
                    let f = self.get_fn(&Inst {
                        node: *node,
                        args: gargs,
                    })?;
                    let mv: Vec<BasicMetadataValueEnum<'ctx>> =
                        vals.iter().map(|v| (*v).into()).collect();
                    let c = llvm(self.builder.build_call(f, &mv, "call"))?;
                    c.try_as_basic_value().basic()
                } else {
                    let key = self.keys.get(node).cloned().unwrap_or_default();
                    let Some(intr) = self.intr_of(&key) else {
                        return unsupported(format!("hàm không có thân MIR: {key}"));
                    };
                    let dest_ty = self.place(fx, dest)?.ty;
                    self.intrinsic(fx, intr, &key, &gargs, args, &vals, &tys, &dest_ty)?
                }
            }
            Callee::Value(o) => {
                let (fv, fty) = self.operand(fx, o)?;
                let Ty::Fn(ps, r, _) = &fty else {
                    return internal("gọi giá trị không phải hàm");
                };
                let ft = self.fn_type(ps, r)?;
                let mv: Vec<BasicMetadataValueEnum<'ctx>> =
                    vals.iter().map(|v| (*v).into()).collect();
                let c = llvm(self.builder.build_indirect_call(
                    ft,
                    fv.into_pointer_value(),
                    &mv,
                    "icall",
                ))?;
                c.try_as_basic_value().basic()
            }
            Callee::TraitMethod { .. } => return unsupported("gọi qua trait"),
        };
        match target {
            Some(t) => {
                if let Some(v) = result {
                    let dp = self.place(fx, dest)?;
                    llvm(self.builder.build_store(dp.ptr, v))?;
                }
                llvm(
                    self.builder
                        .build_unconditional_branch(fx.blocks[t.index()]),
                )?;
            }
            None => {
                // callee trả `!`: nếu vì lý do nào đó nó VẪN trả về thì abort, không chạy tiếp vào hành vi không xác định
                let trap = self.trap_block(fx, abort_kind::UNREACHABLE)?;
                llvm(self.builder.build_unconditional_branch(trap))?;
            }
        }
        Ok(())
    }

    // ---------------------------------------------------------------- lang item / runtime

    /// Chỉ số biến thể hằng của đối số `Ordering` thứ `i` (phải là local gán một lần bằng `Ordering::X`).
    fn ordering_arg(&self, fx: &Fx<'ctx, '_>, args: &[Operand], i: usize) -> R<AtomicOrdering> {
        let v = match args.get(i) {
            Some(Operand::Copy(p) | Operand::Move(p)) if p.projection.is_empty() => {
                fx.ord_consts.get(&p.local.index()).copied()
            }
            _ => None,
        };
        let Some(v) = v else {
            return unsupported(
                "thứ tự bộ nhớ không phải hằng lúc biên dịch (E0503 đòi tường minh)",
            );
        };
        Ok(match v {
            0 => AtomicOrdering::Monotonic,
            1 => AtomicOrdering::Acquire,
            2 => AtomicOrdering::Release,
            3 => AtomicOrdering::AcquireRelease,
            4 => AtomicOrdering::SequentiallyConsistent,
            _ => return internal("biến thể Ordering lạ"),
        })
    }

    fn atomic_slot(&self, recv: BasicValueEnum<'ctx>, ty: &Ty) -> R<PointerValue<'ctx>> {
        let Ty::Int(_) = ty else {
            return unsupported(format!("Atomic<{ty:?}> (chỉ số nguyên; bool chưa hỗ trợ)"));
        };
        let BasicValueEnum::PointerValue(p) = recv else {
            return internal("người nhận Atomic không phải con trỏ");
        };
        Ok(p) // trường `v` là trường 0 của struct {T} ⇒ cùng địa chỉ
    }

    #[allow(clippy::too_many_arguments)]
    fn intrinsic(
        &mut self,
        fx: &mut Fx<'ctx, '_>,
        intr: Intr,
        key: &str,
        gargs: &[Ty],
        args: &[Operand],
        vals: &[BasicValueEnum<'ctx>],
        tys: &[Ty],
        dest_ty: &Ty,
    ) -> R<Option<BasicValueEnum<'ctx>>> {
        match intr {
            Intr::RootCap => Ok(Some(self.lower_ty(dest_ty)?.const_zero())),
            Intr::Exit => {
                let ft = self
                    .ctx
                    .void_type()
                    .fn_type(&[self.ctx.i32_type().into()], false);
                let f = self.rt_fn("gritrt_exit", ft);
                f.add_attribute(AttributeLoc::Function, noreturn(self.ctx));
                let c = llvm(self.builder.build_call(f, &[vals[0].into()], ""))?;
                c.add_attribute(AttributeLoc::Function, noreturn(self.ctx));
                Ok(None)
            }
            Intr::ArenaNew => {
                let ft = self.ctx.i32_type().fn_type(&[], false);
                let f = self.rt_fn("gritrt_arena_new", ft);
                let c = llvm(self.builder.build_call(f, &[], "an"))?;
                let Some(h) = c.try_as_basic_value().basic() else {
                    return internal("gritrt_arena_new không trả giá trị");
                };
                self.tag_obl(h.as_instruction_value(), "E0405")?;
                let a = self.lower_ty(dest_ty)?.into_struct_type().get_undef();
                let a = llvm(self.builder.build_insert_value(a, h, 0, "arena"))?;
                Ok(Some(a.as_basic_value_enum()))
            }
            Intr::ArenaAlloc => {
                let Some(t) = gargs.first() else {
                    return internal("Arena::alloc thiếu đối số kiểu");
                };
                let lt = self.lower_ty(t)?;
                let size = self.td.get_abi_size(&lt);
                let align = u64::from(self.td.get_abi_alignment(&lt)).max(1);
                let i32t = self.ctx.i32_type();
                let h = llvm(
                    self.builder
                        .build_load(i32t, vals[0].into_pointer_value(), "ah"),
                )?;
                let ft = self.ptr_type().fn_type(
                    &[i32t.into(), self.i64t().into(), self.i64t().into()],
                    false,
                );
                let f = self.rt_fn("gritrt_arena_alloc", ft);
                let c = llvm(self.builder.build_call(
                    f,
                    &[
                        h.into(),
                        self.i64t().const_int(size, false).into(),
                        self.i64t().const_int(align, false).into(),
                    ],
                    "aa",
                ))?;
                let Some(p) = c.try_as_basic_value().basic() else {
                    return internal("gritrt_arena_alloc không trả giá trị");
                };
                self.tag_obl(p.as_instruction_value(), "E0405")?;
                llvm(self.builder.build_store(p.into_pointer_value(), vals[1]))?;
                Ok(Some(p))
            }
            Intr::ArenaReset => {
                let i32t = self.ctx.i32_type();
                let h = llvm(
                    self.builder
                        .build_load(i32t, vals[0].into_pointer_value(), "ah"),
                )?;
                let ft = self.ctx.void_type().fn_type(&[i32t.into()], false);
                let f = self.rt_fn("gritrt_arena_reset", ft);
                let c = llvm(self.builder.build_call(f, &[h.into()], ""))?;
                self.tag_obl(c.try_as_basic_value().instruction(), "E0405")?;
                Ok(Some(self.unit_type().const_zero()))
            }
            Intr::FsOpen => {
                let (p, n) = self.split_fat(vals[1])?;
                let i32t = self.ctx.i32_type();
                let ft = i32t.fn_type(&[self.ptr_type().into(), self.i64t().into()], false);
                let f = self.rt_fn("gritrt_fs_open", ft);
                let h = self.call_basic(f, &[p.into(), n.into()])?.into_int_value();
                // Result<File, IoError>: Ok = biến thể 0 {File}, Err = biến thể 1 {IoError}; mã lỗi âm -(k+1) ↦ IoError thứ k
                let Ty::Adt(rd, targs) = dest_ty else {
                    return internal("Cap<Fs>::open không trả Result");
                };
                let et = self.lower_ty(dest_ty)?.into_struct_type();
                let tmp = llvm(self.builder.build_alloca(et, "open.res"))?;
                let ok = llvm(self.builder.build_int_compare(
                    IntPredicate::SGE,
                    h,
                    i32t.const_zero(),
                    "isok",
                ))?;
                let okb = self.ctx.append_basic_block(fx.f, "open.ok");
                let errb = self.ctx.append_basic_block(fx.f, "open.err");
                let end = self.ctx.append_basic_block(fx.f, "open.end");
                llvm(self.builder.build_conditional_branch(ok, okb, errb))?;
                // Ok(File { x: h })
                self.builder.position_at_end(okb);
                let tag = llvm(self.builder.build_struct_gep(et, tmp, 0, "tag"))?;
                llvm(self.builder.build_store(tag, self.i64t().const_zero()))?;
                let pay = llvm(self.builder.build_struct_gep(et, tmp, 1, "pay"))?;
                let v0 = self.variant_struct(*rd, targs, 0)?;
                let f0 = llvm(self.builder.build_struct_gep(v0, pay, 0, "okf"))?;
                let file_ty = self.lower_ty(&targs[0])?.into_struct_type();
                let fx0 = llvm(self.builder.build_struct_gep(file_ty, f0, 0, "fx"))?;
                llvm(self.builder.build_store(fx0, h))?;
                llvm(self.builder.build_unconditional_branch(end))?;
                // Err(IoError)
                self.builder.position_at_end(errb);
                let tag = llvm(self.builder.build_struct_gep(et, tmp, 0, "tag"))?;
                llvm(
                    self.builder
                        .build_store(tag, self.i64t().const_int(1, false)),
                )?;
                let pay = llvm(self.builder.build_struct_gep(et, tmp, 1, "pay"))?;
                let v1 = self.variant_struct(*rd, targs, 1)?;
                let f1 = llvm(self.builder.build_struct_gep(v1, pay, 0, "errf"))?;
                let ioe_ty = self.lower_ty(&targs[1])?.into_struct_type();
                let etag = llvm(self.builder.build_struct_gep(ioe_ty, f1, 0, "etag"))?;
                let neg = llvm(self.builder.build_int_neg(h, "neg"))?;
                let k = llvm(
                    self.builder
                        .build_int_sub(neg, i32t.const_int(1, false), "k"),
                )?;
                let k64 = llvm(self.builder.build_int_z_extend(k, self.i64t(), "k64"))?;
                llvm(self.builder.build_store(etag, k64))?;
                llvm(self.builder.build_unconditional_branch(end))?;
                self.builder.position_at_end(end);
                Ok(Some(llvm(self.builder.build_load(et, tmp, "open.v"))?))
            }
            Intr::EnvArg => {
                let i64t = self.i64t();
                let ptr_ty = self.ptr_type().fn_type(&[i64t.into()], false);
                let len_ty = i64t.fn_type(&[i64t.into()], false);
                let (fp, fl) = (
                    self.rt_fn("gritrt_arg_ptr", ptr_ty),
                    self.rt_fn("gritrt_arg_len", len_ty),
                );
                let a = vals[1].into_int_value();
                let p = self.call_basic(fp, &[a.into()])?;
                let l = self.call_basic(fl, &[a.into()])?;
                self.fat_value(p.into_pointer_value(), l.into_int_value())
            }
            Intr::FsRead => {
                let (p, n) = self.split_fat(vals[1])?;
                let i32t = self.ctx.i32_type();
                let ft = i32t.fn_type(&[self.ptr_type().into(), self.i64t().into()], false);
                let f = self.rt_fn("gritrt_fs_read", ft);
                let h = self.call_basic(f, &[p.into(), n.into()])?;
                let file = self.lower_ty(dest_ty)?.into_struct_type().get_undef();
                let file = llvm(self.builder.build_insert_value(file, h, 0, "file"))?;
                Ok(Some(file.as_basic_value_enum()))
            }
            Intr::FileLen | Intr::FileBytes => {
                let i32t = self.ctx.i32_type();
                let h = llvm(
                    self.builder
                        .build_load(i32t, vals[0].into_pointer_value(), "fh"),
                )?;
                let len_ty = self.i64t().fn_type(&[i32t.into()], false);
                let fl = self.rt_fn("gritrt_file_len", len_ty);
                let l = self.call_basic(fl, &[h.into()])?;
                if intr == Intr::FileLen {
                    return Ok(Some(l));
                }
                let ptr_ty = self.ptr_type().fn_type(&[i32t.into()], false);
                let fp = self.rt_fn("gritrt_file_ptr", ptr_ty);
                let p = self.call_basic(fp, &[h.into()])?;
                self.fat_value(p.into_pointer_value(), l.into_int_value())
            }
            Intr::OutU64 => {
                let ft = self.ctx.void_type().fn_type(&[self.i64t().into()], false);
                let f = self.rt_fn("gritrt_write_u64", ft);
                let c = llvm(self.builder.build_call(f, &[vals[1].into()], ""))?;
                self.tag_obl(c.try_as_basic_value().instruction(), "E0310")?;
                Ok(Some(self.unit_type().const_zero()))
            }
            Intr::OutStr => {
                let (p, n) = self.split_fat(vals[1])?;
                let ft = self
                    .ctx
                    .void_type()
                    .fn_type(&[self.ptr_type().into(), self.i64t().into()], false);
                let f = self.rt_fn("gritrt_write_str", ft);
                let c = llvm(self.builder.build_call(f, &[p.into(), n.into()], ""))?;
                self.tag_obl(c.try_as_basic_value().instruction(), "E0310")?;
                Ok(Some(self.unit_type().const_zero()))
            }
            Intr::Abort => {
                let k = self.ctx.i32_type().const_int(abort_kind::EXPLICIT, false);
                let c = llvm(self.builder.build_call(self.abort_fn, &[k.into()], ""))?;
                c.add_attribute(AttributeLoc::Function, noreturn(self.ctx));
                Ok(None)
            }
            Intr::SliceLen => {
                let fv = vals[0].into_struct_value();
                Ok(Some(llvm(self.builder.build_extract_value(fv, 1, "len"))?))
            }
            Intr::AtomicNew | Intr::MutexNew | Intr::CellNew => {
                let [v] = vals else {
                    return internal(format!("{key}: sai số đối số"));
                };
                // kết quả: struct {v} (Mutex: {v, khoá=0})
                let Some(def) = self.adt_of_intr(intr) else {
                    return internal("không thấy lang item");
                };
                let rty = Ty::Adt(def, gargs.to_vec());
                let ct = self.lower_ty(&rty)?.into_struct_type();
                let s = ct.get_undef();
                let s = llvm(self.builder.build_insert_value(s, *v, 0, "n0"))?;
                let s = if intr == Intr::MutexNew {
                    llvm(
                        self.builder
                            .build_insert_value(s, self.i64t().const_zero(), 1, "n1"),
                    )?
                } else {
                    s
                };
                Ok(Some(s.as_basic_value_enum()))
            }
            Intr::AtomicLoad => {
                let ty = gargs.first().cloned().unwrap_or_else(Ty::unit);
                let p = self.atomic_slot(vals[0], &ty)?;
                let ord = self.ordering_arg(fx, args, 1)?;
                if matches!(
                    ord,
                    AtomicOrdering::Release | AtomicOrdering::AcquireRelease
                ) {
                    return unsupported("load nguyên tử với Release/AcqRel (không hợp lệ)");
                }
                let lt = self.lower_ty(&ty)?;
                let v = llvm(self.builder.build_load(lt, p, "aload"))?;
                self.mark_atomic(v.as_instruction_value(), ord, &ty)?;
                self.tag_obl(v.as_instruction_value(), "E0503")?;
                Ok(Some(v))
            }
            Intr::AtomicStore => {
                let ty = gargs.first().cloned().unwrap_or_else(Ty::unit);
                let p = self.atomic_slot(vals[0], &ty)?;
                let ord = self.ordering_arg(fx, args, 2)?;
                if matches!(
                    ord,
                    AtomicOrdering::Acquire | AtomicOrdering::AcquireRelease
                ) {
                    return unsupported("store nguyên tử với Acquire/AcqRel (không hợp lệ)");
                }
                let st = llvm(self.builder.build_store(p, vals[1]))?;
                self.mark_atomic(Some(st), ord, &ty)?;
                self.tag_obl(Some(st), "E0503")?;
                Ok(Some(self.unit_type().const_zero()))
            }
            Intr::AtomicSwap | Intr::AtomicFetchAdd | Intr::AtomicFetchSub => {
                let ty = gargs.first().cloned().unwrap_or_else(Ty::unit);
                let p = self.atomic_slot(vals[0], &ty)?;
                let ord = self.ordering_arg(fx, args, 2)?;
                let op = match intr {
                    Intr::AtomicSwap => AtomicRMWBinOp::Xchg,
                    Intr::AtomicFetchAdd => AtomicRMWBinOp::Add,
                    _ => AtomicRMWBinOp::Sub,
                };
                let old = llvm(
                    self.builder
                        .build_atomicrmw(op, p, vals[1].into_int_value(), ord),
                )?;
                self.tag_obl(old.as_instruction(), "E0503")?;
                Ok(Some(old.into()))
            }
            Intr::AtomicCas => {
                let ty = gargs.first().cloned().unwrap_or_else(Ty::unit);
                let p = self.atomic_slot(vals[0], &ty)?;
                let succ = self.ordering_arg(fx, args, 3)?;
                let fail = self.ordering_arg(fx, args, 4)?;
                if matches!(
                    fail,
                    AtomicOrdering::Release | AtomicOrdering::AcquireRelease
                ) {
                    return unsupported(
                        "compare_exchange: thứ tự thất bại Release/AcqRel (không hợp lệ)",
                    );
                }
                let r = llvm(self.builder.build_cmpxchg(
                    p,
                    vals[1].into_int_value(),
                    vals[2].into_int_value(),
                    succ,
                    fail,
                ))?;
                self.tag_obl(r.as_instruction(), "E0503")?;
                let old = llvm(self.builder.build_extract_value(r, 0, "old"))?;
                let ok = llvm(self.builder.build_extract_value(r, 1, "ok"))?.into_int_value();
                // Result<T,T>: Ok = biến thể 0, Err = biến thể 1; payload = giá trị cũ
                let failed = llvm(self.builder.build_not(ok, "failed"))?;
                let tag = llvm(self.builder.build_int_z_extend(failed, self.i64t(), "tag"))?;
                let result_def = self
                    .t
                    .db
                    .adts
                    .values()
                    .find(|a| self.is_result(a))
                    .map(|a| a.def);
                let Some(rd) = result_def else {
                    return internal("không thấy enum Result của prelude");
                };
                let targs = vec![ty.clone(), ty.clone()];
                let rty = Ty::Adt(rd, targs.clone());
                let et = self.lower_ty(&rty)?.into_struct_type();
                let tmp = llvm(self.builder.build_alloca(et, "cas.res"))?;
                let tp = llvm(self.builder.build_struct_gep(et, tmp, 0, "tag"))?;
                llvm(self.builder.build_store(tp, tag))?;
                let pay = llvm(self.builder.build_struct_gep(et, tmp, 1, "pay"))?;
                // cả Ok(T) lẫn Err(T) đều có trường 0 = T ở cùng vị trí ⇒ dùng bố cục biến thể 0
                let vs = self.variant_struct(rd, &targs, 0)?;
                let f0 = llvm(self.builder.build_struct_gep(vs, pay, 0, "f0"))?;
                llvm(self.builder.build_store(f0, old))?;
                Ok(Some(llvm(self.builder.build_load(et, tmp, "cas.v"))?))
            }
            Intr::MutexWith => {
                // with(&self, f, a): khoá thật qua gt-rt; f(&!T, Arg) → R
                let recv = vals[0].into_pointer_value();
                let mty = match &tys[0] {
                    Ty::Ref(_, inner) => (**inner).clone(),
                    other => return internal(format!("Mutex::with trên {other:?}")),
                };
                let mt = self.lower_ty(&mty)?.into_struct_type();
                let lock = llvm(self.builder.build_struct_gep(mt, recv, 1, "lock"))?;
                let val = llvm(self.builder.build_struct_gep(mt, recv, 0, "val"))?;
                let lk_ty = self
                    .ctx
                    .void_type()
                    .fn_type(&[self.ptr_type().into()], false);
                let lockf = self.rt_fn("gritrt_mutex_lock", lk_ty);
                let unlockf = self.rt_fn("gritrt_mutex_unlock", lk_ty);
                let lc = llvm(self.builder.build_call(lockf, &[lock.into()], ""))?;
                self.tag_obl(lc.try_as_basic_value().instruction(), "E0502")?;
                let Ty::Fn(ps, r, _) = &tys[1] else {
                    return internal("Mutex::with: đối số f không phải hàm");
                };
                let ft = self.fn_type(ps, r)?;
                let c = llvm(self.builder.build_indirect_call(
                    ft,
                    vals[1].into_pointer_value(),
                    &[val.into(), vals[2].into()],
                    "with",
                ))?;
                let uc = llvm(self.builder.build_call(unlockf, &[lock.into()], ""))?;
                self.tag_obl(uc.try_as_basic_value().instruction(), "E0502")?;
                Ok(c.try_as_basic_value().basic())
            }
            Intr::CellGet => {
                let Some(Ty::Ref(_, inner)) = tys.first() else {
                    return internal("Cell::get: người nhận lạ");
                };
                let ct = self.lower_ty(inner)?.into_struct_type();
                let Some(vt) = ct.get_field_type_at_index(0) else {
                    return internal("Cell không có trường");
                };
                Ok(Some(llvm(self.builder.build_load(
                    vt,
                    vals[0].into_pointer_value(),
                    "cget",
                ))?))
            }
            Intr::CellSet => {
                llvm(
                    self.builder
                        .build_store(vals[0].into_pointer_value(), vals[1]),
                )?;
                Ok(Some(self.unit_type().const_zero()))
            }
            Intr::Spawn => {
                // spawn(f, a): hộp {f, a} trên heap của runtime; luồng thật chạy bước đệm gọi f(a)
                let a_ty = tys[1].clone();
                let tramp = self.trampoline(&a_ty, &tys[0])?;
                let bt = self
                    .ctx
                    .struct_type(&[self.ptr_type().into(), self.lower_ty(&a_ty)?], false);
                let size = self.i64t().const_int(self.td.get_abi_size(&bt), false);
                let alloc_ty = self.ptr_type().fn_type(&[self.i64t().into()], false);
                let alloc = self.rt_fn("gritrt_alloc", alloc_ty);
                let blk = llvm(self.builder.build_call(alloc, &[size.into()], "blk"))?;
                let Some(blk) = blk.try_as_basic_value().basic() else {
                    return internal("gritrt_alloc không trả giá trị");
                };
                let blk = blk.into_pointer_value();
                let fp = llvm(self.builder.build_struct_gep(bt, blk, 0, "bf"))?;
                llvm(self.builder.build_store(fp, vals[0]))?;
                let ap = llvm(self.builder.build_struct_gep(bt, blk, 1, "ba"))?;
                llvm(self.builder.build_store(ap, vals[1]))?;
                let sp_ty = self
                    .ctx
                    .void_type()
                    .fn_type(&[self.ptr_type().into(), self.ptr_type().into()], false);
                let spawn = self.rt_fn("gritrt_spawn", sp_ty);
                let sc = llvm(self.builder.build_call(
                    spawn,
                    &[
                        tramp.as_global_value().as_pointer_value().into(),
                        blk.into(),
                    ],
                    "",
                ))?;
                self.tag_obl(sc.try_as_basic_value().instruction(), "E0501")?;
                Ok(Some(self.unit_type().const_zero()))
            }
        }
    }

    /// gọi hàm runtime có giá trị trả về
    fn call_basic(
        &self,
        f: FunctionValue<'ctx>,
        args: &[BasicMetadataValueEnum<'ctx>],
    ) -> R<BasicValueEnum<'ctx>> {
        let c = llvm(self.builder.build_call(f, args, "rt"))?;
        match c.try_as_basic_value().basic() {
            Some(v) => {
                // mọi hàm runtime có giá trị trả về dùng qua đây là I/O qua `Cap` ⇒ lớp E0310 (IR Guard đòi)
                self.tag_obl(v.as_instruction_value(), "E0310")?;
                Ok(v)
            }
            None => internal("hàm runtime không trả giá trị"),
        }
    }

    fn fat_value(
        &self,
        p: PointerValue<'ctx>,
        n: IntValue<'ctx>,
    ) -> R<Option<BasicValueEnum<'ctx>>> {
        let fat = self.fat_type().get_undef();
        let fat = llvm(self.builder.build_insert_value(fat, p, 0, "f0"))?;
        let fat = llvm(self.builder.build_insert_value(fat, n, 1, "f1"))?;
        Ok(Some(fat.as_basic_value_enum()))
    }

    /// tách con trỏ béo `{ptr, len}`
    fn split_fat(&self, v: BasicValueEnum<'ctx>) -> R<(PointerValue<'ctx>, IntValue<'ctx>)> {
        let BasicValueEnum::StructValue(sv) = v else {
            return internal("đối số kỳ vọng con trỏ béo");
        };
        let p = llvm(self.builder.build_extract_value(sv, 0, "p"))?;
        let n = llvm(self.builder.build_extract_value(sv, 1, "n"))?;
        Ok((p.into_pointer_value(), n.into_int_value()))
    }

    fn is_result(&self, a: &AdtDef) -> bool {
        let name = &self.t.defs()[a.def.index()].path;
        name.ends_with("Result") && matches!(&a.kind, AdtKind::Enum(v) if v.len() == 2)
    }

    fn adt_of_intr(&self, intr: Intr) -> Option<gt_resolve::DefId> {
        let l = match intr {
            Intr::AtomicNew => Lang::Atomic,
            Intr::MutexNew => Lang::Mutex,
            _ => Lang::Cell,
        };
        self.t.db.lang.get(&l).copied()
    }

    fn rt_fn(&mut self, name: &str, ty: FunctionType<'ctx>) -> FunctionValue<'ctx> {
        match self.module.get_function(name) {
            Some(f) => f,
            None => self.module.add_function(name, ty, None),
        }
    }

    fn mark_atomic(
        &self,
        inst: Option<inkwell::values::InstructionValue<'ctx>>,
        ord: AtomicOrdering,
        ty: &Ty,
    ) -> R<()> {
        let Some(inst) = inst else {
            return internal("thao tác nguyên tử không phải lệnh");
        };
        llvm(inst.set_atomic_ordering(ord))?;
        let sz = self.td.get_abi_size(&self.lower_ty(ty)?);
        llvm(inst.set_alignment(u32::try_from(sz).unwrap_or(8)))?;
        Ok(())
    }

    /// Bước đệm theo kiểu đối số: `void tramp(ptr blk)` đọc {f, a} rồi gọi `f(a)`.
    fn trampoline(&mut self, a_ty: &Ty, f_ty: &Ty) -> R<FunctionValue<'ctx>> {
        if let Some(t) = self.tramps.get(a_ty) {
            return Ok(*t);
        }
        let Ty::Fn(ps, r, _) = f_ty else {
            return internal("spawn: f không phải hàm");
        };
        let ft = self.fn_type(ps, r)?;
        let at = self.lower_ty(a_ty)?;
        let bt = self.ctx.struct_type(&[self.ptr_type().into(), at], false);
        let fnty = self
            .ctx
            .void_type()
            .fn_type(&[self.ptr_type().into()], false);
        let f = self
            .module
            .add_function(&format!("gt.tramp.{}", self.tramps.len()), fnty, None);
        let here = self.builder.get_insert_block();
        let bb = self.ctx.append_basic_block(f, "entry");
        self.builder.position_at_end(bb);
        let Some(blk) = f.get_nth_param(0) else {
            return internal("thiếu tham số bước đệm");
        };
        let blk = blk.into_pointer_value();
        let fp = llvm(self.builder.build_struct_gep(bt, blk, 0, "bf"))?;
        let fv = llvm(self.builder.build_load(self.ptr_type(), fp, "f"))?;
        let ap = llvm(self.builder.build_struct_gep(bt, blk, 1, "ba"))?;
        let av = llvm(self.builder.build_load(at, ap, "a"))?;
        llvm(
            self.builder
                .build_indirect_call(ft, fv.into_pointer_value(), &[av.into()], ""),
        )?;
        llvm(self.builder.build_return(None))?;
        if let Some(h) = here {
            self.builder.position_at_end(h);
        }
        self.tramps.insert(a_ty.clone(), f);
        Ok(f)
    }
}

#[allow(dead_code)]
fn _assert_traits(_: &dyn BasicValue<'_>, _: Local) {}
