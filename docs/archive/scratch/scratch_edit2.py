import io

p = 'crates/gt-codegen-llvm/src/lower.rs'
s = io.open(p, encoding='utf-8').read()


def sub(a, b):
    global s
    assert a in s, a[:70]
    s = s.replace(a, b, 1)


sub('''#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Gọi `gritrt_report_int` với kết quả `main` (số nguyên/bool) — chỉ để đối chiếu với thông dịch.
    pub report_result: bool,
}''', '''#[derive(Clone, Debug, Default)]
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
}''')
sub('''pub fn compile(
    t: &TypesOutput,
    bodies: &[Body],
    keys: &BTreeMap<NodeId, String>,
    opts: &Options,
) -> R<Compiled> {''', '''/// `guard`: IR Guard (gt-guard) — chạy SAU codegen, TRƯỚC khi phát object; trả danh sách vi phạm (rỗng = chấp nhận) ⇒ nếu có thì
/// `CodegenError::Guard` và KHÔNG có object.
pub fn compile(
    t: &TypesOutput,
    bodies: &[Body],
    keys: &BTreeMap<NodeId, String>,
    opts: &Options,
    guard: &dyn for<'c> Fn(&Module<'c>) -> Vec<String>,
) -> R<Compiled> {''')
sub('''    cg.emit_entry(main, main_fn, opts)?;

    if let Err(e) = cg.module.verify() {
        return internal(format!("LLVM verifier từ chối IR: {}", e.to_string()));
    }
    let ir''', '''    cg.emit_entry(main, main_fn, opts)?;
    cg.emit_attestation(opts)?;
    if opts.inject == Some(Inject::StrayUnreachable) {
        let f = cg
            .module
            .add_function("gt.fault", ctx.void_type().fn_type(&[], false), None);
        let bb = ctx.append_basic_block(f, "bb0");
        cg.builder.position_at_end(bb);
        llvm(cg.builder.build_unreachable())?;
    }
    let violations = guard(&cg.module);
    if !violations.is_empty() {
        return Err(CodegenError::Guard(violations));
    }
    let ir''')
sub("    strings: BTreeMap<String, GlobalValue<'ctx>>,\n    tramps", "    strings: BTreeMap<String, GlobalValue<'ctx>>,\n    inject: Option<Inject>,\n    tramps")
sub("        strings: BTreeMap::new(),\n        tramps", "        strings: BTreeMap::new(),\n        inject: opts.inject,\n        tramps")
sub("    fn define_body(&mut self, inst: &Inst, body: &'a Body) -> R<()> {", '''    /// `!gt.attest` (module) + hằng chuỗi `gt.attestation` trong exe (để đối chiếu sau này).
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
    fn tag_obl(
        &self,
        inst: Option<inkwell::values::InstructionValue<'ctx>>,
        class: &str,
    ) -> R<()> {
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

    fn define_body(&mut self, inst: &Inst, body: &'a Body) -> R<()> {''')
sub('''                let v = llvm(self.builder.build_load(lt, p, "aload"))?;
                self.mark_atomic(v.as_instruction_value(), ord, &ty)?;''', '''                let v = llvm(self.builder.build_load(lt, p, "aload"))?;
                self.mark_atomic(v.as_instruction_value(), ord, &ty)?;
                self.tag_obl(v.as_instruction_value(), "E0503")?;''')
sub('''                self.mark_atomic(Some(st), ord, &ty)?;''', '''                self.mark_atomic(Some(st), ord, &ty)?;
                self.tag_obl(Some(st), "E0503")?;''')
sub('''                )?;
                Ok(Some(old.into()))''', '''                )?;
                self.tag_obl(old.as_instruction(), "E0503")?;
                Ok(Some(old.into()))''')
sub('''                let old = llvm(self.builder.build_extract_value(r, 0, "old"))?;''', '''                self.tag_obl(r.as_instruction(), "E0503")?;
                let old = llvm(self.builder.build_extract_value(r, 0, "old"))?;''')
sub('''                llvm(self.builder.build_call(lockf, &[lock.into()], ""))?;''', '''                let lc = llvm(self.builder.build_call(lockf, &[lock.into()], ""))?;
                self.tag_obl(lc.try_as_basic_value().instruction(), "E0502")?;''')
sub('''                llvm(self.builder.build_call(unlockf, &[lock.into()], ""))?;''', '''                let uc = llvm(self.builder.build_call(unlockf, &[lock.into()], ""))?;
                self.tag_obl(uc.try_as_basic_value().instruction(), "E0502")?;''')
sub('''                llvm(self.builder.build_call(
                    spawn,
                    &[
                        tramp.as_global_value().as_pointer_value().into(),
                        blk.into(),
                    ],
                    "",
                ))?;''', '''                let sc = llvm(self.builder.build_call(
                    spawn,
                    &[
                        tramp.as_global_value().as_pointer_value().into(),
                        blk.into(),
                    ],
                    "",
                ))?;
                self.tag_obl(sc.try_as_basic_value().instruction(), "E0501")?;''')
io.open(p, 'w', encoding='utf-8', newline='').write(s)

p = 'crates/gt-codegen-llvm/src/lib.rs'
s = io.open(p, encoding='utf-8').read()
sub("pub use lower::{Compiled, Options, compile};", "pub use lower::{Compiled, Inject, Options, compile};")
sub("    Internal(String),\n}", "    Internal(String),\n    /// IR Guard từ chối (E1101, ĐỎ): danh sách vi phạm; KHÔNG có object.\n    Guard(Vec<String>),\n}")
sub('            CodegenError::Internal(m) => write!(f, "lỗi nội bộ codegen: {m}"),', '            CodegenError::Internal(m) => write!(f, "lỗi nội bộ codegen: {m}"),\n            CodegenError::Guard(v) => write!(f, "IR Guard từ chối: {}", v.join("; ")),')
io.open(p, 'w', encoding='utf-8', newline='').write(s)
print("ok")
