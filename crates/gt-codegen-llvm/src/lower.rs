//! Hạ MIR → LLVM IR. Mọi local là một `alloca` (dev profile, không tối ưu): đơn giản, đúng, dễ đối chiếu với thông dịch.
use crate::{CodegenError, abort_kind};
use gt_ast::NodeId;
use gt_mir::{
    AggregateKind, AssertKind, BinOp, Body, BodyKind, Callee, Const, Operand, Place, PlaceElem,
    Rvalue, StatementKind, TerminatorKind, UnOp,
};
use gt_types::{AdtKind, IntTy, Ty, TypesOutput};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;
use inkwell::attributes::AttributeLoc;
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::intrinsics::Intrinsic;
use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, IntType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use std::collections::BTreeMap;

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
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Gọi `gritrt_report_int` với kết quả `main` (số nguyên/bool) — chỉ để đối chứng với thông dịch.
    pub report_result: bool,
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

struct Cg<'ctx, 'a> {
    ctx: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    t: &'a TypesOutput,
    bodies: &'a [Body],
    keys: &'a BTreeMap<NodeId, String>,
    funcs: BTreeMap<NodeId, FunctionValue<'ctx>>,
    pending: Vec<NodeId>,
    abort_fn: FunctionValue<'ctx>,
}

/// Biên dịch cả chương trình. Gọi SAU khi cổng phán quyết cho phép (xanh/vàng) — chính hàm này không kiểm lại M4–M9.
/// `bodies`: MIR SAU M7 (đã áp chế độ check); `keys`: tên đầy đủ theo NodeId (để nhận ra hàm prelude không có thân).
pub fn compile(
    t: &TypesOutput,
    bodies: &[Body],
    keys: &BTreeMap<NodeId, String>,
    opts: &Options,
) -> R<Compiled> {
    let ctx = Context::create();
    let module = ctx.create_module("grit");
    let builder = ctx.create_builder();
    let abort_ty = ctx.void_type().fn_type(&[ctx.i32_type().into()], false);
    let abort_fn = module.add_function("gritrt_abort", abort_ty, None);
    let noret = ctx.create_enum_attribute(
        inkwell::attributes::Attribute::get_named_enum_kind_id("noreturn"),
        0,
    );
    abort_fn.add_attribute(AttributeLoc::Function, noret);
    let mut cg = Cg {
        ctx: &ctx,
        module,
        builder,
        t,
        bodies,
        keys,
        funcs: BTreeMap::new(),
        pending: Vec::new(),
        abort_fn,
    };

    let main = find_main(bodies)?;
    let main_fn = cg.get_fn(main.node)?;
    while let Some(node) = cg.pending.pop() {
        let body = cg.body_of(node)?;
        cg.define_body(body)?;
    }
    cg.emit_entry(main, main_fn, opts)?;

    if let Err(e) = cg.module.verify() {
        return internal(format!("LLVM verifier từ chối IR: {}", e.to_string()));
    }
    let ir = cg.module.print_to_string().to_string();

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
    cg.module.set_triple(&triple);
    cg.module
        .set_data_layout(&tm.get_target_data().get_data_layout());
    let buf = llvm(tm.write_to_memory_buffer(&cg.module, FileType::Object))?;
    Ok(Compiled {
        ir,
        object: buf.as_slice().to_vec(),
    })
}

impl<'ctx, 'a> Cg<'ctx, 'a> {
    fn body_of(&self, node: NodeId) -> R<&'a Body> {
        let bodies: &'a [Body] = self.bodies;
        match bodies.iter().find(|b| b.node == node) {
            Some(b) => Ok(b),
            None => internal(format!("thiếu thân MIR cho node {node:?}")),
        }
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

    fn unit_type(&self) -> BasicTypeEnum<'ctx> {
        self.ctx.struct_type(&[], false).into()
    }

    fn ptr_type(&self) -> inkwell::types::PointerType<'ctx> {
        self.ctx.ptr_type(AddressSpace::default())
    }

    fn is_sized(ty: &Ty) -> bool {
        !matches!(ty, Ty::Str | Ty::Slice(_))
    }

    /// Kiểu LLVM của một kiểu Grit (không đa hình). Gặp thứ chưa hỗ trợ ⇒ `Unsupported`.
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
                if !Self::is_sized(inner) {
                    return unsupported("tham chiếu tới slice/str (con trỏ béo)");
                }
                self.ptr_type().into()
            }
            Ty::Fn(..) => self.ptr_type().into(),
            Ty::Adt(def, args) => {
                let Some(adt) = self.t.db.adts.get(def) else {
                    return internal("ADT không có trong db");
                };
                if !args.is_empty() || !adt.generics.is_empty() {
                    return unsupported("ADT đa hình (cần đơn hình hoá)");
                }
                if adt.lang.is_some() {
                    return unsupported(format!("kiểu lang item {:?}", adt.lang));
                }
                match &adt.kind {
                    AdtKind::Struct(fs) => {
                        let ts = fs
                            .iter()
                            .map(|f| self.lower_ty(&f.ty))
                            .collect::<R<Vec<_>>>()?;
                        self.ctx.struct_type(&ts, false).into()
                    }
                    AdtKind::Enum(_) => return unsupported("enum"),
                }
            }
            Ty::Float(_) => return unsupported("số thực"),
            Ty::Str | Ty::Slice(_) => return unsupported("kiểu không kích thước đứng riêng"),
            Ty::Param(_) | Ty::Infer(_) => return unsupported("tham số kiểu (cần đơn hình hoá)"),
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

    /// Khai báo (lần đầu) một hàm có thân MIR và xếp hàng để định nghĩa.
    fn get_fn(&mut self, node: NodeId) -> R<FunctionValue<'ctx>> {
        if let Some(f) = self.funcs.get(&node) {
            return Ok(*f);
        }
        let body = self.body_of(node)?;
        if body.kind != BodyKind::Fn {
            return unsupported("gọi thân không phải hàm");
        }
        let params: Vec<Ty> = (1..=body.arg_count)
            .map(|i| body.locals[i].ty.clone())
            .collect();
        let ft = self.fn_type(&params, body.return_ty())?;
        let name = format!("{}#{}.{}", body.key, node.file().raw(), node.raw());
        let f = self.module.add_function(&name, ft, None);
        self.funcs.insert(node, f);
        self.pending.push(node);
        Ok(f)
    }

    fn emit_entry(&mut self, main: &Body, main_fn: FunctionValue<'ctx>, opts: &Options) -> R<()> {
        let entry_ty = self.ctx.void_type().fn_type(&[], false);
        let entry = self.module.add_function("gt_entry", entry_ty, None);
        let bb = self.ctx.append_basic_block(entry, "entry");
        self.builder.position_at_end(bb);
        let r = llvm(self.builder.build_call(main_fn, &[], "main"))?;
        if opts.report_result {
            let v = r.try_as_basic_value();
            let value = v.basic();
            let rep_ty = self.ctx.void_type().fn_type(
                &[self.ctx.i64_type().into(), self.ctx.i32_type().into()],
                false,
            );
            let rep = self.module.add_function("gritrt_report_int", rep_ty, None);
            match main.return_ty() {
                Ty::Int(it) => {
                    let Some(BasicValueEnum::IntValue(iv)) = value else {
                        return internal("main không trả giá trị số nguyên");
                    };
                    if it.bits() > 64 {
                        return unsupported("báo kết quả main rộng hơn 64 bit");
                    }
                    let i64t = self.ctx.i64_type();
                    let w = if it.signed() {
                        llvm(self.builder.build_int_s_extend_or_bit_cast(iv, i64t, "w"))?
                    } else {
                        llvm(self.builder.build_int_z_extend_or_bit_cast(iv, i64t, "w"))?
                    };
                    let s = self.ctx.i32_type().const_int(u64::from(it.signed()), false);
                    llvm(self.builder.build_call(rep, &[w.into(), s.into()], ""))?;
                }
                Ty::Bool => {
                    let Some(BasicValueEnum::IntValue(iv)) = value else {
                        return internal("main không trả bool");
                    };
                    let w = llvm(
                        self.builder
                            .build_int_z_extend(iv, self.ctx.i64_type(), "w"),
                    )?;
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

    fn define_body(&mut self, body: &Body) -> R<()> {
        let f = self.funcs[&body.node];
        let mut fx = Fx {
            f,
            body,
            locals: Vec::new(),
            blocks: Vec::new(),
            traps: 0,
        };
        let entry = self.ctx.append_basic_block(f, "alloca");
        self.builder.position_at_end(entry);
        for (i, l) in body.locals.iter().enumerate() {
            let ty = self.lower_ty(&l.ty)?;
            let p = llvm(self.builder.build_alloca(ty, &format!("l{i}")))?;
            fx.locals.push(p);
        }
        for i in 0..body.arg_count {
            let Some(arg) = f.get_nth_param(u32::try_from(i).unwrap_or(u32::MAX)) else {
                return internal("thiếu tham số hàm");
            };
            llvm(self.builder.build_store(fx.locals[i + 1], arg))?;
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
        let noret = self.ctx.create_enum_attribute(
            inkwell::attributes::Attribute::get_named_enum_kind_id("noreturn"),
            0,
        );
        call.add_attribute(AttributeLoc::Function, noret);
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

    fn place(&mut self, fx: &Fx<'ctx, '_>, p: &Place) -> R<(PointerValue<'ctx>, Ty)> {
        let mut ptr = fx.locals[p.local.index()];
        let mut ty = fx.body.locals[p.local.index()].ty.clone();
        for e in &p.projection {
            match e {
                PlaceElem::Deref => {
                    let inner = match &ty {
                        Ty::Ref(_, i) | Ty::RawPtr(_, i) if Self::is_sized(i) => (**i).clone(),
                        _ => return unsupported("Deref qua con trỏ béo hoặc kiểu lạ"),
                    };
                    let pt = self.ptr_type();
                    let v = llvm(self.builder.build_load(pt, ptr, "deref"))?;
                    ptr = v.into_pointer_value();
                    ty = inner;
                }
                PlaceElem::Field(i, fty) => {
                    let ct = self.lower_ty(&ty)?;
                    let BasicTypeEnum::StructType(st) = ct else {
                        return internal("Field trên kiểu không phải struct");
                    };
                    ptr = llvm(self.builder.build_struct_gep(st, ptr, *i, "fld"))?;
                    ty = fty.clone();
                }
                PlaceElem::Index(l) => {
                    let Ty::Array(elem, _) = &ty else {
                        return unsupported("chỉ số trên slice");
                    };
                    let at = self.lower_ty(&ty)?;
                    let i64t = self.ctx.i64_type();
                    let idx = llvm(self.builder.build_load(i64t, fx.locals[l.index()], "idx"))?
                        .into_int_value();
                    // SAFETY-không-cần-unsafe: `build_in_bounds_gep` của inkwell là `unsafe fn`; chỉ số đã được `Assert` chặn trước đó.
                    ptr = self.gep_array(at, ptr, idx)?;
                    ty = (**elem).clone();
                }
                PlaceElem::Downcast(_) => return unsupported("Downcast (enum)"),
                PlaceElem::Subslice { .. } => return unsupported("Subslice"),
            }
        }
        Ok((ptr, ty))
    }

    #[allow(unsafe_code)]
    fn gep_array(
        &self,
        at: BasicTypeEnum<'ctx>,
        ptr: PointerValue<'ctx>,
        idx: IntValue<'ctx>,
    ) -> R<PointerValue<'ctx>> {
        let zero = self.ctx.i64_type().const_zero();
        // SAFETY: chỉ số được MIR kiểm `Assert(idx < len)` ngay trước mọi `Index`; IR Guard (bước 7) kiểm điều này.
        llvm(unsafe {
            self.builder
                .build_in_bounds_gep(at, ptr, &[zero, idx], "elem")
        })
    }

    fn operand(&mut self, fx: &Fx<'ctx, '_>, o: &Operand) -> R<(BasicValueEnum<'ctx>, Ty)> {
        match o {
            Operand::Copy(p) | Operand::Move(p) => {
                let (ptr, ty) = self.place(fx, p)?;
                let lt = self.lower_ty(&ty)?;
                let v = llvm(self.builder.build_load(lt, ptr, "ld"))?;
                Ok((v, ty))
            }
            Operand::Const(c) => self.constant(c),
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

    fn constant(&mut self, c: &Const) -> R<(BasicValueEnum<'ctx>, Ty)> {
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
            Const::FnItem { node, generics, ty } => {
                if !generics.is_empty() {
                    return unsupported("giá trị hàm đa hình");
                }
                let f = self.get_fn(*node)?;
                (f.as_global_value().as_pointer_value().into(), ty.clone())
            }
            Const::Float { .. } => return unsupported("literal số thực"),
            Const::Str(_) => return unsupported("literal chuỗi"),
            Const::Item { .. } => return unsupported("giá trị const/static"),
        })
    }

    // ---------------------------------------------------------------- câu lệnh

    fn assign(&mut self, fx: &mut Fx<'ctx, '_>, dest: &Place, rv: &Rvalue) -> R<()> {
        match rv {
            Rvalue::Aggregate(kind, ops) => {
                let (dp, dty) = self.place(fx, dest)?;
                return self.aggregate(fx, dp, &dty, kind, ops);
            }
            Rvalue::Repeat(op, n) => {
                let (dp, dty) = self.place(fx, dest)?;
                return self.repeat(fx, dp, &dty, op, *n);
            }
            _ => {}
        }
        let v = self.rvalue(fx, rv)?;
        let (dp, _) = self.place(fx, dest)?;
        llvm(self.builder.build_store(dp, v))?;
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
        match kind {
            AggregateKind::Tuple => {}
            AggregateKind::Array(_) => {}
            AggregateKind::Adt(def, var, args) => {
                let Some(adt) = self.t.db.adts.get(def) else {
                    return internal("ADT không có trong db");
                };
                if *var != 0 || !matches!(adt.kind, AdtKind::Struct(_)) || !args.is_empty() {
                    return unsupported("khởi tạo enum / ADT đa hình");
                }
            }
        }
        for (i, op) in ops.iter().enumerate() {
            let (v, _) = self.operand(fx, op)?;
            let idx = u32::try_from(i).unwrap_or(u32::MAX);
            let slot = match (kind, ct) {
                (AggregateKind::Array(_), BasicTypeEnum::ArrayType(_)) => {
                    let ix = self.ctx.i64_type().const_int(i as u64, false);
                    self.gep_array(ct, dp, ix)?
                }
                (_, BasicTypeEnum::StructType(st)) => {
                    llvm(self.builder.build_struct_gep(st, dp, idx, "agg"))?
                }
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
        let i64t = self.ctx.i64_type();
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
        let slot = self.gep_array(at, dp, i)?;
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
                let (ptr, ty) = self.place(fx, p)?;
                if !Self::is_sized(&ty) {
                    return unsupported("mượn chỗ không kích thước");
                }
                Ok(ptr.into())
            }
            Rvalue::Len(p) => {
                let (_, ty) = self.place(fx, p)?;
                match ty {
                    Ty::Array(_, n) => Ok(self.ctx.i64_type().const_int(n, false).into()),
                    _ => unsupported("Len của slice"),
                }
            }
            Rvalue::Cast(o, to) => {
                let (v, from) = self.operand(fx, o)?;
                self.cast(v, &from, to)
            }
            Rvalue::CastFits(o, to) => {
                let (v, from) = self.operand(fx, o)?;
                self.cast_fits(v, &from, to)
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
            Rvalue::Discriminant(_) => unsupported("Discriminant (enum)"),
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
        let bld = &self.builder;
        let v: IntValue<'ctx> = match op {
            BinOp::Add | BinOp::AddWrap => llvm(bld.build_int_add(a, b, "add"))?,
            BinOp::Sub | BinOp::SubWrap => llvm(bld.build_int_sub(a, b, "sub"))?,
            BinOp::Mul | BinOp::MulWrap => {
                if a.get_type().get_bit_width() > 64 {
                    return unsupported("nhân 128 bit (cần __multi3)");
                }
                llvm(bld.build_int_mul(a, b, "mul"))?
            }
            BinOp::Div | BinOp::Rem => {
                let bits = a.get_type().get_bit_width();
                if bits > 64 {
                    return unsupported("chia 128 bit (cần __divti3)");
                }
                // LLVM: chia 0 và MIN / -1 là UB ⇒ chặn tại chỗ như MỘT LỚP PHÒNG THỦ, dù MIR đã có Assert (D3).
                let zero = a.get_type().const_zero();
                let nz = llvm(bld.build_int_compare(IntPredicate::NE, b, zero, "nz"))?;
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
                llvm(bld.build_int_compare(pred, a, b, "cmp"))?
            }
            BinOp::BitAnd => llvm(bld.build_and(a, b, "and"))?,
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
                // Phase 1: không có destructor người dùng (cùng ngữ nghĩa thông dịch)
                llvm(
                    self.builder
                        .build_unconditional_branch(fx.blocks[target.index()]),
                )?;
            }
            TerminatorKind::Return => {
                let lt = self.lower_ty(fx.body.return_ty())?;
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

    fn call(
        &mut self,
        fx: &mut Fx<'ctx, '_>,
        callee: &Callee,
        args: &[Operand],
        dest: &Place,
        target: Option<gt_mir::BasicBlock>,
    ) -> R<()> {
        let mut vals: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
        let mut tys: Vec<Ty> = Vec::new();
        for a in args {
            let (v, t) = self.operand(fx, a)?;
            vals.push(v.into());
            tys.push(t);
        }
        let result: Option<BasicValueEnum<'ctx>> = match callee {
            Callee::Fn { node, generics } => {
                if !generics.is_empty() {
                    return unsupported("gọi hàm đa hình (cần đơn hình hoá)");
                }
                if self.bodies.iter().any(|b| b.node == *node) {
                    let f = self.get_fn(*node)?;
                    let c = llvm(self.builder.build_call(f, &vals, "call"))?;
                    c.try_as_basic_value().basic()
                } else {
                    let key = self.keys.get(node).cloned().unwrap_or_default();
                    if key.ends_with("core::abort") {
                        let k = self.ctx.i32_type().const_int(abort_kind::EXPLICIT, false);
                        let c = llvm(self.builder.build_call(self.abort_fn, &[k.into()], ""))?;
                        let _ = c;
                        None
                    } else {
                        return unsupported(format!("hàm không có thân MIR: {key}"));
                    }
                }
            }
            Callee::Value(o) => {
                let (fv, fty) = self.operand(fx, o)?;
                let Ty::Fn(ps, r, _) = &fty else {
                    return internal("gọi giá trị không phải hàm");
                };
                let ft = self.fn_type(ps, r)?;
                let c = llvm(self.builder.build_indirect_call(
                    ft,
                    fv.into_pointer_value(),
                    &vals,
                    "icall",
                ))?;
                c.try_as_basic_value().basic()
            }
            Callee::TraitMethod { .. } => return unsupported("gọi qua trait"),
        };
        match target {
            Some(t) => {
                if let Some(v) = result {
                    let (dp, _) = self.place(fx, dest)?;
                    llvm(self.builder.build_store(dp, v))?;
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
        let _ = &tys;
        Ok(())
    }
}

struct Fx<'ctx, 'b> {
    f: FunctionValue<'ctx>,
    body: &'b Body,
    locals: Vec<PointerValue<'ctx>>,
    blocks: Vec<BasicBlock<'ctx>>,
    traps: usize,
}

#[allow(dead_code)]
fn _assert_traits(_: &dyn BasicValue<'_>) {}
