//! gt-guard — IR Guard tối thiểu (ADR-029 §6, mã E1101). Hàm DUY NHẤT quyết định chấp nhận/từ chối IR là [`check_module`]; nó KHÔNG tin
//! `gt-codegen-llvm` (không phụ thuộc nó): chỉ đọc `Module` và đối chiếu với đầu vào độc lập (`GuardInput`).
//!
//! Luật (mọi vi phạm ⇒ một thông báo; rỗng = chấp nhận):
//! 1. LLVM verifier sạch.
//! 2. Metadata `!gt.attest` của module = attestation của lần `check` vừa rồi (MIR sau M7 + chế độ check + pass).
//! 3. Mọi thao tác nhạy cảm mang `!gt.obl` đúng lớp: atomic load/store/rmw/cmpxchg ⇒ `E0503`; `gritrt_spawn` ⇒ `E0501`;
//!    `gritrt_mutex_lock/unlock` ⇒ `E0502`; lời gọi runtime I/O qua `Cap` (`gritrt_arg_*`, `gritrt_fs_read`, `gritrt_file_*`, `gritrt_write_*`) ⇒ `E0310`
//!    (nghĩa vụ capability: nguồn gốc chỉ ở `main`, hiệu ứng `io` đã kiểm). (Mức hạt: THEO LỚP nghĩa vụ, chưa theo từng điểm — KNOWN_LIMITATIONS.)
//! 3b. MỌI lời gọi tới hàm `gritrt_*` phải nằm trong BẢNG [`RUNTIME`] (lớp hoặc "không cần lớp"); `gritrt_*` lạ ⇒ từ chối. Mọi lời gọi tới một hàm
//!    chỉ khai báo (không có thân) phải là `llvm.*` hoặc nằm trong bảng; hàm ngoài khác ⇒ từ chối (không có FFI ngoài runtime).
//! 4. `unreachable` chỉ đúng mẫu abort: khối tên `trap.*`, lệnh ngay trước là `call @gritrt_abort` có `noreturn`.
//! 5. Không có cờ sinh poison/UB trên số học (`nsw`/`nuw`/`exact`/fast-math/`disjoint`/`nneg`/`samesign`) và không `llvm.assume`.
use gt_llvm::inkwell::AtomicOrdering;
use gt_llvm::inkwell::attributes::{Attribute, AttributeLoc};
use gt_llvm::inkwell::module::Module;
use gt_llvm::inkwell::values::{AnyValue, InstructionOpcode, InstructionValue};

/// Đầu vào độc lập với codegen.
#[derive(Clone, Copy, Debug)]
pub struct GuardInput<'a> {
    /// attestation (M9) của lần `check` vừa rồi
    pub attestation: &'a str,
}

/// Tên metadata module/lệnh — hợp đồng với codegen.
pub const ATTEST_KEY: &str = "gt.attest";
pub const OBL_KIND: &str = "gt.obl";

/// Tên hàm đích của `call` (từ dạng chữ của lệnh: `... call ... @name(...)`).
fn callee_name(inst: &InstructionValue<'_>) -> Option<String> {
    if inst.get_opcode() != InstructionOpcode::Call {
        return None;
    }
    let text = inst.print_to_string().to_string();
    let after = text.split_once('@')?.1;
    let end = after.find('(')?;
    Some(after[..end].trim_matches('"').to_string())
}

fn obl_class(m: &Module<'_>, inst: &InstructionValue<'_>) -> Option<String> {
    let kind = m.get_context().get_kind_id(OBL_KIND);
    let md = inst.get_metadata(kind)?;
    let vals = md.get_node_values()?;
    let first = vals.first()?;
    let mv = first.into_metadata_value();
    let s = mv.get_string_value()?;
    Some(String::from_utf8_lossy(s).into_owned())
}

fn is_atomic(inst: &InstructionValue<'_>) -> bool {
    match inst.get_opcode() {
        InstructionOpcode::AtomicRMW | InstructionOpcode::AtomicCmpXchg => true,
        InstructionOpcode::Load | InstructionOpcode::Store => inst
            .get_atomic_ordering()
            .is_ok_and(|o| o != AtomicOrdering::NotAtomic),
        _ => false,
    }
}

/// Bảng lời gọi runtime hợp lệ: (tên, lớp nghĩa vụ phải mang; `None` = không nhạy cảm). Hợp đồng với `gt-codegen-llvm` và `runtime/gt-rt`
/// (docs/TCB_INTRINSICS.md). Thêm hàm runtime MỚI = thêm dòng ở đây (nếu quên, Guard từ chối ⇒ fail-closed).
pub const RUNTIME: [(&str, Option<&str>); 19] = [
    ("gritrt_abort", None),
    ("gritrt_exit", None),
    ("gritrt_start", None),
    ("gritrt_report_int", None),
    ("gritrt_alloc", None),
    ("gritrt_spawn", Some("E0501")),
    ("gritrt_mutex_lock", Some("E0502")),
    ("gritrt_mutex_unlock", Some("E0502")),
    ("gritrt_arg_ptr", Some("E0310")),
    ("gritrt_arg_len", Some("E0310")),
    ("gritrt_fs_read", Some("E0310")),
    ("gritrt_fs_open", Some("E0310")),
    ("gritrt_arena_new", Some("E0405")),
    ("gritrt_arena_alloc", Some("E0405")),
    ("gritrt_arena_reset", Some("E0405")),
    ("gritrt_file_len", Some("E0310")),
    ("gritrt_file_ptr", Some("E0310")),
    ("gritrt_write_u64", Some("E0310")),
    ("gritrt_write_str", Some("E0310")),
];

/// Lớp nghĩa vụ mà thao tác này PHẢI mang (`None`: không nhạy cảm).
fn required_class(inst: &InstructionValue<'_>) -> Option<&'static str> {
    if is_atomic(inst) {
        return Some("E0503");
    }
    let name = callee_name(inst)?;
    RUNTIME
        .iter()
        .find(|(n, _)| *n == name)
        .and_then(|(_, class)| *class)
}

const UB_FLAGS: [&str; 11] = [
    "nsw", "nuw", "exact", "nnan", "ninf", "nsz", "fast", "disjoint", "nneg", "samesign", "afn",
];

/// Kiểm một module. Trả danh sách vi phạm (rỗng = chấp nhận).
#[must_use]
pub fn check_module(m: &Module<'_>, input: &GuardInput<'_>) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();

    // 1. verifier
    if let Err(e) = m.verify() {
        v.push(format!("LLVM verifier từ chối: {}", e.to_string()));
    }

    // 2. attestation
    let got: Vec<String> = m
        .get_global_metadata(ATTEST_KEY)
        .iter()
        .flat_map(|md| md.get_node_values().unwrap_or_default())
        .filter_map(|x| {
            let mv = x.into_metadata_value();
            mv.get_string_value()
                .map(|s| String::from_utf8_lossy(s).into_owned())
        })
        .collect();
    match got.as_slice() {
        [a] if a == input.attestation => {}
        [] => v.push(
            "thiếu metadata !gt.attest (không chứng minh được IR sinh từ MIR đã duyệt)".to_string(),
        ),
        other => v.push(format!(
            "attestation lệch: IR mang {other:?}, bản ghi của check là {}",
            input.attestation
        )),
    }

    let noreturn_kind = Attribute::get_named_enum_kind_id("noreturn");
    for f in m.get_functions() {
        for bb in f.get_basic_blocks() {
            let bname = bb.get_name().to_string_lossy().into_owned();
            let mut prev: Option<InstructionValue<'_>> = None;
            let mut cur = bb.get_first_instruction();
            while let Some(inst) = cur {
                let fname = f.get_name().to_string_lossy().into_owned();
                // 3. thao tác nhạy cảm
                if let Some(want) = required_class(&inst) {
                    match obl_class(m, &inst).as_deref() {
                        Some(c) if c == want => {}
                        Some(c) => v.push(format!(
                            "{fname}/{bname}: thao tác nhạy cảm mang lớp nghĩa vụ `{c}`, cần `{want}`"
                        )),
                        None => v.push(format!(
                            "{fname}/{bname}: thao tác nhạy cảm thiếu metadata !{OBL_KIND} (cần `{want}`)"
                        )),
                    }
                }
                // 3b. đích của mọi lời gọi: hàm trong module (có thân), `llvm.*`, hoặc bảng RUNTIME
                if let Some(callee) = callee_name(&inst) {
                    let known = RUNTIME.iter().any(|(n, _)| *n == callee);
                    let defined = m
                        .get_function(&callee)
                        .is_some_and(|c| c.count_basic_blocks() > 0);
                    if callee.starts_with("gritrt_") {
                        if !known {
                            v.push(format!(
                                "{fname}/{bname}: lời gọi runtime `{callee}` không thuộc lớp nào (không có trong bảng RUNTIME)"
                            ));
                        }
                    } else if !defined && !callee.starts_with("llvm.") {
                        v.push(format!(
                            "{fname}/{bname}: lời gọi hàm ngoài `{callee}` (chỉ khai báo, không phải `llvm.*` hay runtime)"
                        ));
                    }
                }
                // 4. unreachable
                if inst.get_opcode() == InstructionOpcode::Unreachable {
                    let ok = bname.starts_with("trap.")
                        && prev.as_ref().is_some_and(|p| {
                            callee_name(p).as_deref() == Some("gritrt_abort")
                                && m.get_function("gritrt_abort").is_some_and(|a| {
                                    a.get_enum_attribute(AttributeLoc::Function, noreturn_kind)
                                        .is_some()
                                })
                        });
                    if !ok {
                        v.push(format!(
                            "{fname}/{bname}: `unreachable` không đúng mẫu abort (khối `trap.*`, ngay sau `call @gritrt_abort` noreturn)"
                        ));
                    }
                }
                // 5. cờ UB / assume
                let text = inst.print_to_string().to_string();
                let is_gep = inst.get_opcode() == InstructionOpcode::GetElementPtr;
                if !is_gep {
                    // bỏ phần sau `, !` (metadata) và chuỗi trong ngoặc kép
                    let body = text.split(", !").next().unwrap_or(&text);
                    for tok in body.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.') {
                        if UB_FLAGS.contains(&tok) {
                            v.push(format!(
                                "{fname}/{bname}: cờ sinh poison/UB `{tok}` trong `{}`",
                                text.trim()
                            ));
                            break;
                        }
                    }
                }
                if text.contains("llvm.assume") {
                    v.push(format!(
                        "{fname}/{bname}: `llvm.assume` bị cấm: {}",
                        text.trim()
                    ));
                }
                prev = Some(inst);
                cur = inst.get_next_instruction();
            }
        }
    }
    v
}
