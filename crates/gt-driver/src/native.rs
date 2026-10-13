//! Backend LLVM của `gritc` (ADR-029 §5, §8): MIR sau M7 → object (gt-codegen-llvm) → liên kết với `gt-rt` → exe; `run` chạy exe thật.
//! Chỉ nhận [`Artifacts`] (qua cổng XANH/VÀNG). `LLVM-C.dll` chỉ cần Ở ĐÂY (nạp trễ); exe sinh ra KHÔNG phụ thuộc nó.
use crate::pipeline::Artifacts;
use crate::runner::RunReport;
use crate::verdict::RunEnd;
use gt_codegen_llvm::{CodegenError, Inject, Options};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Lỗi của backend LLVM; `Env` ⇒ mã thoát 3 (môi trường), `Codegen` ⇒ chưa hỗ trợ (3) hoặc nội bộ (2), `Link` ⇒ nội bộ (2).
#[derive(Debug)]
pub enum NativeError {
    Env(String),
    Codegen(CodegenError),
    Link(String),
    /// IR Guard từ chối (E1101, ĐỎ ⇒ mã thoát 1; KHÔNG có object/exe)
    Guard(Vec<String>),
}

impl NativeError {
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            NativeError::Env(_) => 3,
            NativeError::Guard(_) | NativeError::Codegen(CodegenError::Guard(_)) => 1,
            NativeError::Codegen(CodegenError::Unsupported(_)) => 3,
            NativeError::Codegen(CodegenError::Internal(_)) | NativeError::Link(_) => 2,
        }
    }
}

impl std::fmt::Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeError::Env(m) | NativeError::Link(m) => write!(f, "{m}"),
            NativeError::Codegen(e) => write!(f, "{e}"),
            NativeError::Guard(v) => write!(f, "IR Guard từ chối: {}", v.join("; ")),
        }
    }
}

/// `LLVM-C.dll` phải nằm trên PATH (nạp trễ: thiếu thì dừng Ở ĐÂY với thông báo rõ, không để tiến trình chết lúc nạp).
fn check_llvm_dll() -> Result<(), NativeError> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    if std::env::split_paths(&path).any(|d| d.join("LLVM-C.dll").is_file()) {
        Ok(())
    } else {
        Err(NativeError::Env(
            "không thấy LLVM-C.dll trên PATH: backend llvm cần LLVM 20.1.8 (đặt PATH gồm thư mục bin của GRIT_LLVM_DIR, xem docs/ENVIRONMENT.md) — hoặc dùng --backend=interpreter"
                .to_string(),
        ))
    }
}

/// Thư viện `gt_rt.lib`: biến `GRIT_RT_LIB`, hoặc bản dựng cạnh mã nguồn (dev: `runtime/gt-rt/target/release/gt_rt.lib`).
fn rt_lib() -> Result<PathBuf, NativeError> {
    if let Some(p) = std::env::var_os("GRIT_RT_LIB") {
        let p = PathBuf::from(p);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(NativeError::Env(format!(
                "GRIT_RT_LIB={} không tồn tại",
                p.display()
            )))
        };
    }
    let dev =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime/gt-rt/target/release/gt_rt.lib");
    if dev.is_file() {
        Ok(dev)
    } else {
        Err(NativeError::Env(format!(
            "không thấy gt_rt.lib ({}): dựng bằng `cargo build --release --manifest-path runtime/gt-rt/Cargo.toml` hoặc đặt GRIT_RT_LIB",
            dev.display()
        )))
    }
}

fn linker() -> Result<Command, NativeError> {
    let Some(tool) = find_msvc_tools::find_tool("x86_64-pc-windows-msvc", "link.exe") else {
        return Err(NativeError::Env(
            "không tìm thấy link.exe của MSVC (cần Visual Studio Build Tools)".to_string(),
        ));
    };
    Ok(tool.to_command())
}

/// `GRITC_FAULT=ir:unreachable|ir:strip-obl|ir:attest`: chèn lỗi vào IR để KIỂM IR Guard (chỉ làm gritc từ chối thêm).
fn inject_from_env() -> Option<Inject> {
    match std::env::var("GRITC_FAULT").ok()?.strip_prefix("ir:")? {
        "unreachable" => Some(Inject::StrayUnreachable),
        "strip-obl" => Some(Inject::StripObligation),
        "attest" => Some(Inject::BadAttestation),
        "runtime-call" => Some(Inject::UnknownRuntimeCall),
        "external-call" => Some(Inject::ExternalCall),
        "instance" => Some(Inject::RogueInstance),
        _ => None,
    }
}

/// Biên dịch + liên kết ra `out` (.exe). `report_result`: exe báo kết quả `main` qua `GRITRT_RESULT_FILE` (dùng cho `run`).
/// `attestation`: giá trị của lần `check` (xem `pipeline::attestation`); IR Guard đối chiếu nó với metadata trong IR.
pub fn build_exe(
    art: &Artifacts,
    out: &Path,
    report_result: bool,
    attestation: &str,
) -> Result<(), NativeError> {
    check_llvm_dll()?;
    let rt = rt_lib()?;
    crate::runner::apply_types_fault_from_env();
    let allowed = gt_codegen_llvm::expected_instances(&art.types, &art.obl.bodies);
    let compiled = gt_codegen_llvm::compile(
        &art.types,
        &art.obl.bodies,
        &art.mir.keys,
        &Options {
            report_result,
            attestation: attestation.to_string(),
            inject: inject_from_env(),
        },
        &|m| {
            gt_guard::check_module(
                m,
                &gt_guard::GuardInput {
                    attestation,
                    instances: &allowed,
                },
            )
        },
    )
    .map_err(|e| match e {
        CodegenError::Guard(v) => NativeError::Guard(v),
        other => NativeError::Codegen(other),
    })?;
    // chẩn đoán: GRITC_DUMP_IR=<tệp> ghi IR LLVM ra tệp (không ảnh hưởng kết quả)
    if let Some(p) = std::env::var_os("GRITC_DUMP_IR") {
        let _ = std::fs::write(p, &compiled.ir);
    }
    let mut obj = out.as_os_str().to_owned();
    obj.push(".obj");
    let obj = PathBuf::from(obj);
    std::fs::write(&obj, &compiled.object)
        .map_err(|e| NativeError::Link(format!("không ghi được {}: {e}", obj.display())))?;
    let mut cmd = linker()?;
    // /Brepro: bỏ dấu thời gian khỏi PE ⇒ cùng nguồn cho cùng nhị phân (tái lập, M11); không sinh PDB nên không rò đường dẫn
    cmd.arg("/NOLOGO")
        .arg("/Brepro")
        .arg("/SUBSYSTEM:CONSOLE")
        .arg(format!("/OUT:{}", out.display()))
        .arg(&obj)
        .arg(&rt)
        .args([
            "kernel32.lib",
            "ntdll.lib",
            "userenv.lib",
            "ws2_32.lib",
            "dbghelp.lib",
            "/defaultlib:msvcrt",
        ]);
    let r = cmd.output();
    let _ = std::fs::remove_file(&obj);
    match r {
        Ok(o) if o.status.success() && out.is_file() => Ok(()),
        Ok(o) => {
            let _ = std::fs::remove_file(out);
            Err(NativeError::Link(format!(
                "liên kết thất bại ({}): {}{}",
                o.status,
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            )))
        }
        Err(e) => Err(NativeError::Link(format!("không chạy được link.exe: {e}"))),
    }
}

/// Chạy exe đã dựng bằng `build_exe(.., true)`: ánh xạ mã thoát (0 / 70 abort / khác ⇒ trạng thái chương trình).
#[must_use]
pub fn run_exe(exe: &Path) -> RunReport {
    run_exe_timeout(exe, None)
}

/// Như [`run_exe`] nhưng quá hạn ⇒ giết tiến trình và trả [`RunEnd::Limit`] (dùng cho kiểm thử ngẫu nhiên: không để treo).
#[must_use]
pub fn run_exe_timeout(exe: &Path, limit: Option<std::time::Duration>) -> RunReport {
    let side = |ext: &str| {
        let mut p = exe.as_os_str().to_owned();
        p.push(ext);
        PathBuf::from(p)
    };
    let (result_file, out_file, err_file) = (side(".result"), side(".stdout"), side(".stderr"));
    let _ = std::fs::remove_file(&result_file);
    let spawn = (|| -> std::io::Result<std::process::ExitStatus> {
        let mut child = Command::new(exe)
            .env("GRITRT_RESULT_FILE", &result_file)
            .stdout(std::fs::File::create(&out_file)?)
            .stderr(std::fs::File::create(&err_file)?)
            .spawn()?;
        let start = std::time::Instant::now();
        loop {
            if let Some(st) = child.try_wait()? {
                return Ok(st);
            }
            if limit.is_some_and(|l| start.elapsed() > l) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "quá hạn"));
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    })();
    let value = std::fs::read_to_string(&result_file).ok();
    let err = std::fs::read_to_string(&err_file).unwrap_or_default();
    let out = std::fs::read_to_string(&out_file).unwrap_or_default();
    for f in [&result_file, &out_file, &err_file] {
        let _ = std::fs::remove_file(f);
    }
    match spawn {
        Ok(st) => {
            eprint!("{err}");
            print!("{out}");
            let end = match st.code() {
                Some(0) => RunEnd::Normal,
                Some(70) => RunEnd::Aborted(
                    err.lines()
                        .rev()
                        .find_map(|l| l.strip_prefix("abort: "))
                        .unwrap_or("unknown")
                        .to_string(),
                ),
                Some(c) => RunEnd::ProgramExit(u8::try_from(c.clamp(1, 255)).unwrap_or(255)),
                None => RunEnd::Internal("chương trình bị hệ điều hành dừng".to_string()),
            };
            RunReport { end, value }
        }
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => RunReport {
            end: RunEnd::Limit,
            value: None,
        },
        Err(e) => RunReport {
            end: RunEnd::Internal(format!("không chạy được {}: {e}", exe.display())),
            value: None,
        },
    }
}
