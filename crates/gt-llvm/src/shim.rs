//! Thay shim C của llvm-sys (bị bỏ qua khi `no-llvm-linking`, ADR-029 kết quả spike): các hàm inline của llvm-c/Target.h.
//! Chỉ X86 (Phase 1). Đây là chỗ DUY NHẤT của crate dùng `unsafe`/FFI thô.
#![allow(unsafe_code, non_snake_case)]

// FB-C50 (Gemini G-09b): chỉ X86 — dựng cho kiến trúc khác phải lỗi biên dịch RÕ RÀNG thay vì lỗi liên kết khó hiểu
#[cfg(not(target_arch = "x86_64"))]
compile_error!("gt-llvm/shim.rs chỉ hỗ trợ x86_64 (Phase 1)");

unsafe extern "C" {
    fn LLVMInitializeX86TargetInfo();
    fn LLVMInitializeX86Target();
    fn LLVMInitializeX86TargetMC();
    fn LLVMInitializeX86AsmPrinter();
    fn LLVMInitializeX86AsmParser();
    fn LLVMInitializeX86Disassembler();
}

#[unsafe(no_mangle)]
pub extern "C" fn LLVM_InitializeAllTargetInfos() {
    unsafe { LLVMInitializeX86TargetInfo() }
}
#[unsafe(no_mangle)]
pub extern "C" fn LLVM_InitializeAllTargets() {
    unsafe { LLVMInitializeX86Target() }
}
#[unsafe(no_mangle)]
pub extern "C" fn LLVM_InitializeAllTargetMCs() {
    unsafe { LLVMInitializeX86TargetMC() }
}
#[unsafe(no_mangle)]
pub extern "C" fn LLVM_InitializeAllAsmPrinters() {
    unsafe { LLVMInitializeX86AsmPrinter() }
}
#[unsafe(no_mangle)]
pub extern "C" fn LLVM_InitializeAllAsmParsers() {
    unsafe { LLVMInitializeX86AsmParser() }
}
#[unsafe(no_mangle)]
pub extern "C" fn LLVM_InitializeAllDisassemblers() {
    unsafe { LLVMInitializeX86Disassembler() }
}
#[unsafe(no_mangle)]
pub extern "C" fn LLVM_InitializeNativeTarget() -> i32 {
    unsafe {
        LLVMInitializeX86TargetInfo();
        LLVMInitializeX86Target();
        LLVMInitializeX86TargetMC();
    }
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn LLVM_InitializeNativeAsmPrinter() -> i32 {
    unsafe { LLVMInitializeX86AsmPrinter() };
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn LLVM_InitializeNativeAsmParser() -> i32 {
    unsafe { LLVMInitializeX86AsmParser() };
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn LLVM_InitializeNativeDisassembler() -> i32 {
    unsafe { LLVMInitializeX86Disassembler() };
    0
}
