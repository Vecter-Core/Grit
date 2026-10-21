//! gt-guard — IR Guard tối thiểu (ADR-029 §6, mã E1101). Hàm DUY NHẤT quyết định chấp nhận/từ chối IR là [`check_module`]; nó KHÔNG tin
//! `gt-codegen-llvm` (không phụ thuộc nó): chỉ đọc `Module` và đối chiếu với đầu vào độc lập (`GuardInput`).
//!
//! Luật (mọi vi phạm ⇒ một thông báo; rỗng = chấp nhận):
//! 1. LLVM verifier sạch.
//! 2. Metadata `!gt.attest` của module = attestation của lần `check` vừa rồi (MIR sau M7 + chế độ check + pass).
//! 3. Mọi thao tác nhạy cảm mang `!gt.obl` đúng lớp: atomic load/store/rmw/cmpxchg ⇒ `E0503`; `gritrt_spawn` ⇒ `E0501`;
//!    `gritrt_mutex_lock/unlock` ⇒ `E0502`; lời gọi runtime I/O qua `Cap` (`gritrt_arg_*`, `gritrt_fs_read`, `gritrt_file_*`, `gritrt_write_*`) ⇒ `E0310`
//!    (nghĩa vụ capability: nguồn gốc chỉ ở `main`, hiệu ứng `io` đã kiểm). (Mức hạt: THEO LỚP nghĩa vụ, chưa theo từng điểm — KNOWN_LIMITATIONS.)
//! 3. (b) MỌI lời gọi tới hàm `gritrt_*` phải nằm trong BẢNG [`RUNTIME`] (lớp hoặc "không cần lớp"); `gritrt_*` lạ ⇒ từ chối. Mọi lời gọi tới một hàm
//!    chỉ khai báo (không có thân) phải là `llvm.*` hoặc nằm trong bảng; hàm ngoài khác ⇒ từ chối (không có FFI ngoài runtime).
//! 4. `unreachable` chỉ đúng mẫu abort: khối tên `trap.*`, lệnh ngay trước là `call @gritrt_abort` có `noreturn`.
//! 5. Không có cờ sinh poison/UB trên số học (`nsw`/`nuw`/`exact`/fast-math/`disjoint`/`nneg`/`samesign`) và không `llvm.assume`.
use gt_llvm::inkwell::AtomicOrdering;
use gt_llvm::inkwell::attributes::{Attribute, AttributeLoc};
use gt_llvm::inkwell::module::Module;
use gt_llvm::inkwell::values::{
    AnyValue, AnyValueEnum, BasicValue, CallSiteValue, InstructionOpcode, InstructionValue,
};

/// Đầu vào độc lập với codegen.
#[derive(Clone, Copy, Debug)]
pub struct GuardInput<'a> {
    /// attestation (M9) của lần `check` vừa rồi
    pub attestation: &'a str,
    /// Tập symbol ĐƯỢC PHÉP định nghĩa (tính độc lập với codegen từ bảng instance của M4: `gt_codegen_llvm::expected_instances`).
    pub instances: &'a [String],
}

/// Tên metadata module/lệnh — hợp đồng với codegen.
pub const ATTEST_KEY: &str = "gt.attest";
pub const OBL_KIND: &str = "gt.obl";
/// Metadata module: bảng `symbol=dạng đọc được` của mọi instance đã định nghĩa (ADR-032 phụ lục A, Q69).
pub const INSTANCES_KEY: &str = "gt.instances";
/// Hàm vào của chương trình (do codegen sinh, không thuộc bảng instance).
pub const ENTRY_FN: &str = "gt_entry";
/// Hàm `main` của C (gọi `gritrt_start(gt_entry)`).
pub const C_MAIN_FN: &str = "main";
/// Tiền tố hàm bọc `spawn` (`gt.tramp.<số>`), do codegen sinh, không thuộc bảng instance.
pub const TRAMP_PREFIX: &str = "gt.tramp.";

/// Hàm hỗ trợ do codegen sinh (không phải instance): chỉ đúng ba dạng này.
fn support_fn(name: &str) -> bool {
    name == ENTRY_FN
        || name == C_MAIN_FN
        || name
            .strip_prefix(TRAMP_PREFIX)
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Tên hàm đích của lệnh `call` TRỰC TIẾP (qua API LLVM, không qua dạng chữ: tên có ký tự không ASCII như `h⊲0` được LLVM in dạng thoát
/// `\E2\8A\B2` nên so tên theo dạng chữ sẽ SAI — lỗi FB-2). Gọi gián tiếp (con trỏ hàm) hoặc không phải `call` ⇒ `None`.
fn callee_name(inst: &InstructionValue<'_>) -> Option<String> {
    if inst.get_opcode() != InstructionOpcode::Call {
        return None;
    }
    let call = CallSiteValue::try_from(*inst).ok()?;
    let f = call.get_called_fn_value()?;
    Some(f.get_name().to_string_lossy().into_owned())
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
pub const RUNTIME: [(&str, Option<&str>); 26] = [
    ("gritrt_abort", None),
    ("gritrt_exit", None),
    ("gritrt_start", None),
    ("gritrt_dev_checks", None),
    ("gritrt_report_int", None),
    ("gritrt_alloc", None),
    ("gritrt_spawn", Some("E0501")),
    ("gritrt_mutex_lock", Some("E0502")),
    ("gritrt_mutex_unlock", Some("E0502")),
    ("gritrt_arg_count", Some("E0310")),
    ("gritrt_ewrite_u64", Some("E0310")),
    ("gritrt_ewrite_str", Some("E0310")),
    ("gritrt_arg_ptr", Some("E0310")),
    ("gritrt_arg_len", Some("E0310")),
    ("gritrt_fs_read", Some("E0310")),
    ("gritrt_fs_open", Some("E0310")),
    ("gritrt_stdin_read", Some("E0310")),
    ("gritrt_fs_write", Some("E0310")),
    ("gritrt_utf8_valid", None),
    ("gritrt_arena_new", Some("E0405")),
    ("gritrt_arena_alloc", Some("E0405")),
    ("gritrt_arena_reset", Some("E0405")),
    ("gritrt_file_len", Some("E0310")),
    ("gritrt_file_ptr", Some("E0310")),
    ("gritrt_write_u64", Some("E0310")),
    ("gritrt_write_str", Some("E0310")),
];

/// Tên symbol hợp lệ: toàn ASCII `[A-Za-z0-9_.]` (FB-2: tên Unicode làm Guard/bộ liên kết/trình gỡ lỗi rối).
fn ascii_symbol(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
}

/// Lệnh tạo ra một giá trị (inkwell phân loại người dùng theo KIỂU kết quả: `call` trả con trỏ ⇒ `PointerValue`, trả số ⇒ `IntValue`, `void` ⇒ `InstructionValue`).
fn inst_of<'c>(v: AnyValueEnum<'c>) -> Option<InstructionValue<'c>> {
    match v {
        AnyValueEnum::InstructionValue(i) => Some(i),
        AnyValueEnum::IntValue(x) => x.as_instruction(),
        AnyValueEnum::PointerValue(x) => x.as_instruction(),
        AnyValueEnum::FloatValue(x) => x.as_instruction(),
        AnyValueEnum::StructValue(x) => x.as_instruction(),
        AnyValueEnum::ArrayValue(x) => x.as_instruction(),
        AnyValueEnum::VectorValue(x) => x.as_instruction(),
        AnyValueEnum::PhiValue(x) => Some(x.as_instruction()),
        _ => None,
    }
}

/// FAIL_OPEN_AUDIT G1: `callee_name` chỉ nhận lời gọi TRỰC TIẾP, nên gọi `gritrt_*` qua con trỏ (hoặc `invoke`/`callbr`) tránh được luật "lớp nghĩa vụ" và luật "đích
/// mọi lời gọi". Quy tắc đóng lỗ: (a) địa chỉ của hàm `gritrt_*` chỉ được dùng làm ĐÍCH của một `call` TRỰC TIẾP (không lưu, không truyền làm đối số, không nằm trong
/// hằng/khởi tạo); (b) không có `invoke`/`callbr` (codegen không bao giờ phát; Phase 1 không có unwind).
fn runtime_address_violations(m: &Module<'_>) -> Vec<String> {
    let mut v = Vec::new();
    for f in m.get_functions() {
        let name = f.get_name().to_string_lossy().into_owned();
        if !name.starts_with("gritrt_") {
            continue;
        }
        let fptr = f.as_global_value().as_pointer_value();
        let mut u = fptr.get_first_use();
        while let Some(use_) = u {
            // inkwell trả lời gọi dưới dạng `CallSiteValue` hoặc `InstructionValue` tuỳ đường: chuẩn hoá về lệnh
            let user_inst = inst_of(use_.get_user());
            let direct = match user_inst {
                Some(i) if i.get_opcode() == InstructionOpcode::Call => {
                    // f là đích gọi, và KHÔNG xuất hiện làm đối số (toán hạng cuối là đích)
                    let n = i.get_num_operands();
                    callee_name(&i).as_deref() == Some(name.as_str())
                        && (0..n.saturating_sub(1)).all(|k| {
                            !matches!(
                                i.get_operand(k),
                                Some(gt_llvm::inkwell::values::Operand::Value(
                                    gt_llvm::inkwell::values::BasicValueEnum::PointerValue(p)
                                )) if p == fptr
                            )
                        })
                }
                _ => false,
            };
            if !direct {
                v.push(format!(
                    "địa chỉ hàm runtime `{name}` bị dùng ngoài vai trò đích của lời gọi trực tiếp (lưu/truyền/gọi gián tiếp bị cấm: tránh luật lớp nghĩa vụ)"
                ));
                break;
            }
            u = use_.get_next_use();
        }
    }
    v
}

/// Luật instance: (a) mọi tên hàm toàn ASCII; (b) mọi hàm ĐỊNH NGHĨA (trừ `gt_entry`) thuộc tập cho phép; (c) có ít nhất một instance;
/// (d) `!gt.instances` liệt kê ĐÚNG tập hàm đã định nghĩa (bảng ánh xạ ngược không lệch IR).
fn instance_violations(m: &Module<'_>, input: &GuardInput<'_>) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    let mut defined: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for f in m.get_functions() {
        let name = f.get_name().to_string_lossy().into_owned();
        if !ascii_symbol(&name) {
            v.push(format!("tên hàm không phải ASCII an toàn: {name:?}"));
        }
        if f.count_basic_blocks() == 0 || support_fn(&name) {
            continue;
        }
        if !input.instances.contains(&name) {
            v.push(format!("hàm `{name}` không thuộc bảng instance của M4"));
        }
        defined.insert(name);
    }
    if defined.is_empty() {
        v.push("IR không định nghĩa instance nào".to_string());
    }
    let listed: std::collections::BTreeSet<String> = m
        .get_global_metadata(INSTANCES_KEY)
        .iter()
        .flat_map(|md| md.get_node_values().unwrap_or_default())
        .filter_map(|x| {
            let mv = x.into_metadata_value();
            mv.get_string_value().map(|s| {
                let t = String::from_utf8_lossy(s).into_owned();
                t.split('=').next().unwrap_or("").to_string()
            })
        })
        .collect();
    if listed != defined {
        v.push(format!(
            "!gt.instances lệch IR: metadata {listed:?}, hàm đã định nghĩa {defined:?}"
        ));
    }
    v
}

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

const UB_FLAGS: [&str; 14] = [
    "nsw", "nuw", "exact", "nnan", "ninf", "nsz", "fast", "disjoint", "nneg", "samesign", "afn",
    "arcp", "contract", "reassoc",
];

/// Thuộc tính HÀM dạng chuỗi bật số học thực không an toàn (ADR-036 §6b): cấm ở mọi hồ sơ.
const UNSAFE_FP_ATTRS: [&str; 6] = [
    "unsafe-fp-math",
    "no-nans-fp-math",
    "no-infs-fp-math",
    "no-signed-zeros-fp-math",
    "approx-func-fp-math",
    "less-precise-fpmad",
];

/// Hàm intrinsic hợp nhất nhân-cộng (FMA đổi kết quả bit so với IEEE từng phép): cấm.
fn fused_intrinsic(name: &str) -> bool {
    name.starts_with("llvm.fmuladd") || name.starts_with("llvm.fma")
}

/// Số lời gọi `gritrt_abort` theo (hàm, loại abort = hằng số đối số 0; `?` nếu không phải hằng).
pub type AbortProfile = std::collections::BTreeMap<(String, String), usize>;

/// Sự kiện của IR TRƯỚC tối ưu mà Guard-post cần (ADR-036 §6a).
#[derive(Clone, Debug, Default)]
pub struct PreFacts {
    pub aborts: AbortProfile,
}

fn abort_kind_of(inst: &InstructionValue<'_>) -> String {
    inst.get_operand(0)
        .and_then(|o| match o {
            gt_llvm::inkwell::values::Operand::Value(v) => Some(v),
            gt_llvm::inkwell::values::Operand::Block(_) => None,
        })
        .and_then(|v| match v {
            gt_llvm::inkwell::values::BasicValueEnum::IntValue(i) => i.get_zero_extended_constant(),
            _ => None,
        })
        .map_or_else(|| "?".to_string(), |k| k.to_string())
}

/// Đếm lời gọi `gritrt_abort` theo (hàm, loại).
#[must_use]
pub fn abort_profile(m: &Module<'_>) -> AbortProfile {
    let mut p = AbortProfile::new();
    for f in m.get_functions() {
        let fname = f.get_name().to_string_lossy().into_owned();
        for bb in f.get_basic_blocks() {
            let mut cur = bb.get_first_instruction();
            while let Some(inst) = cur {
                if callee_name(&inst).as_deref() == Some("gritrt_abort") {
                    *p.entry((fname.clone(), abort_kind_of(&inst))).or_insert(0) += 1;
                }
                cur = inst.get_next_instruction();
            }
        }
    }
    p
}

/// Chụp sự kiện của IR trước tối ưu.
#[must_use]
pub fn pre_facts(m: &Module<'_>) -> PreFacts {
    PreFacts {
        aborts: abort_profile(m),
    }
}

/// GUARD-POST (ADR-036 §6a): kiểm IR SAU tối ưu. Gồm TOÀN BỘ luật của [`check_module`] (verifier, attestation, instance, nghĩa vụ,
/// bảng runtime, `unreachable` chỉ đúng mẫu abort ⇒ không có `unreachable` MỚI, không cờ UB/fast-math/contract/reassoc, không assume) CỘNG:
/// (R1) với mỗi (hàm, loại abort) số lời gọi `gritrt_abort` KHÔNG giảm so với IR trước tối ưu — giảm = dấu hiệu bộ tối ưu đã xoá một kiểm tra;
/// (R2) không `llvm.fma`/`llvm.fmuladd`; (R3) không thuộc tính hàm `unsafe-fp-math`… (xem `UNSAFE_FP_ATTRS`).
#[must_use]
pub fn check_post(m: &Module<'_>, input: &GuardInput<'_>, pre: &PreFacts) -> Vec<String> {
    let mut v = check_module(m, input);
    let post = abort_profile(m);
    for ((f, k), n) in &pre.aborts {
        let now = post.get(&(f.clone(), k.clone())).copied().unwrap_or(0);
        if now < *n {
            v.push(format!(
                "{f}: số lời gọi gritrt_abort loại {k} giảm {n} → {now} sau tối ưu (bộ tối ưu đã xoá một kiểm tra)"
            ));
        }
    }
    for f in m.get_functions() {
        let fname = f.get_name().to_string_lossy().into_owned();
        for a in f.attributes(AttributeLoc::Function) {
            if a.is_string() {
                let key = a.get_string_kind_id().to_string_lossy().into_owned();
                if UNSAFE_FP_ATTRS.contains(&key.as_str()) {
                    v.push(format!(
                        "{fname}: thuộc tính hàm số thực không an toàn `{key}`"
                    ));
                }
            }
        }
        for bb in f.get_basic_blocks() {
            let mut cur = bb.get_first_instruction();
            while let Some(inst) = cur {
                if callee_name(&inst).is_some_and(|c| fused_intrinsic(&c)) {
                    v.push(format!(
                        "{fname}: hợp nhất nhân-cộng (FMA) bị cấm: {}",
                        inst.print_to_string().to_string().trim()
                    ));
                }
                cur = inst.get_next_instruction();
            }
        }
    }
    v
}

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

    v.extend(instance_violations(m, input));
    v.extend(runtime_address_violations(m));

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
                // 3c. không `invoke`/`callbr` (không qua được `callee_name`)
                if matches!(
                    inst.get_opcode(),
                    InstructionOpcode::Invoke | InstructionOpcode::CallBr
                ) {
                    v.push(format!(
                        "{fname}/{bname}: `invoke`/`callbr` bị cấm (không có unwind trong Phase 1; tránh luật đích lời gọi)"
                    ));
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
                    // BIỂU THỨC HẰNG `getelementptr inbounds nuw (i8, ptr @g, i64 K)` (LLVM 20 chuẩn hoá GEP struct thành offset byte sau mem2reg/sroa/early-cse) mang
                    // cờ GEP hợp lệ y như LỆNH GEP (đã được miễn ở trên): không phải số học của chương trình. FB-9: thiếu miễn này làm hồ sơ release từ chối mọi
                    // chương trình có `static Mutex`. CHỈ bỏ cụm từ GEP; `add nuw`/`mul nsw`... vẫn bị từ chối.
                    let body = body
                        .replace("getelementptr inbounds nuw", "getelementptr")
                        .replace("getelementptr nuw", "getelementptr");
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
