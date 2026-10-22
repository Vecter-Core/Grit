//! Backend LLVM của `gritc` (ADR-029 §5, §8): MIR sau M7 → object (gt-codegen-llvm) → liên kết với `gt-rt` → exe; `run` chạy exe thật.
//! Chỉ nhận [`Artifacts`] (qua cổng XANH/VÀNG). `LLVM-C.dll` chỉ cần Ở ĐÂY (nạp trễ); exe sinh ra KHÔNG phụ thuộc nó.
use crate::pipeline::Artifacts;
use crate::runner::RunReport;
use crate::verdict::RunEnd;
use gt_codegen_llvm::{CodegenError, Inject, Opt, Options};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Lỗi MÔI TRƯỜNG thoáng qua khi mở/chạy tệp vừa được ghi (quét virus/chỉ mục của Windows giữ tệp: "Access is denied" os error 5, sharing violation 32). Thử lại
/// có kiểm soát CHỈ cho loại lỗi này ở bước KHỞI CHẠY tiến trình; KHÔNG BAO GIỜ thử lại một kết quả (mã thoát/đầu ra) — kết quả sai vẫn là sai (FB flake llvm_sort).
fn transient_io(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::PermissionDenied || matches!(e.raw_os_error(), Some(5 | 32))
}

/// Thử `f` tối đa `ATTEMPTS` lần, chỉ lặp khi lỗi là [`transient_io`]; chờ tăng dần giữa các lần.
pub(crate) fn retry_transient<T>(mut f: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    const ATTEMPTS: u32 = 8;
    let mut i = 0;
    loop {
        match f() {
            Err(e) if transient_io(&e) && i + 1 < ATTEMPTS => {
                i += 1;
                std::thread::sleep(std::time::Duration::from_millis(150 * u64::from(i)));
            }
            r => return r,
        }
    }
}

/// `link.exe` báo không mở được tệp đầu ra (LNK1104: tệp đang bị quét/giữ): lỗi môi trường thoáng qua, chỉ thử lại bước liên kết.
fn transient_link_failure(output: &str) -> bool {
    output.contains("LNK1104")
}

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
#[cfg(not(windows))]
fn check_llvm_dll() -> Result<(), NativeError> {
    Ok(())
}

#[cfg(windows)]
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

/// Đường dẫn thư viện runtime đang dùng (cho `gritc doctor`).
pub fn runtime_lib_path() -> Result<PathBuf, NativeError> {
    rt_lib()
}

/// Tên + phiên bản trình liên kết đang dùng (Linux: `cc`; Windows: `link.exe` của MSVC) — cho `gritc doctor`.
pub fn linker_probe() -> Result<String, NativeError> {
    let mut c = linker()?;
    #[cfg(not(windows))]
    {
        c.arg("--version");
        let o = c
            .output()
            .map_err(|e| NativeError::Env(format!("không chạy được `cc`: {e}")))?;
        let first = String::from_utf8_lossy(&o.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .to_string();
        if o.status.success() {
            Ok(first)
        } else {
            Err(NativeError::Env("`cc --version` thất bại".to_string()))
        }
    }
    #[cfg(windows)]
    {
        let _ = &mut c;
        Ok("link.exe (MSVC)".to_string())
    }
}

/// Thư viện runtime (`gt_rt.lib` ở Windows, `libgt_rt.a` ở Linux): biến `GRIT_RT_LIB`, hoặc bản dựng cạnh mã nguồn (dev: `runtime/gt-rt/target/release/`).
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
    let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join(if cfg!(windows) {
        "../../runtime/gt-rt/target/release/gt_rt.lib"
    } else {
        "../../runtime/gt-rt/target/release/libgt_rt.a"
    });
    if dev.is_file() {
        Ok(dev)
    } else {
        Err(NativeError::Env(format!(
            "không thấy gt_rt.lib ({}): dựng bằng `cargo build --release --manifest-path runtime/gt-rt/Cargo.toml` hoặc đặt GRIT_RT_LIB",
            dev.display()
        )))
    }
}

#[cfg(not(windows))]
fn linker() -> Result<Command, NativeError> {
    Ok(Command::new("cc"))
}

#[cfg(windows)]
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
    let v = std::env::var("GRITC_FAULT").ok()?;
    if let Some(k) = v.strip_prefix("post:") {
        return match k {
            "drop-abort" => Some(Inject::PostDropAbort),
            "fma" => Some(Inject::PostFma),
            "fast-math" => Some(Inject::PostFastMath),
            "fp-attr" => Some(Inject::PostFpAttr),
            "nsw" => Some(Inject::PostNsw),
            "unreachable" => Some(Inject::PostUnreachable),
            "untagged-spawn" => Some(Inject::PostUntaggedSpawn),
            _ => None,
        };
    }
    match v.strip_prefix("ir:")? {
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
    build_exe_profile(art, out, report_result, attestation, Opt::Dev).map(|_| ())
}

/// Băm SHA-256 của IR trước và sau tối ưu (ADR-036 §6a: attestation phủ cả hai). Hồ sơ `dev`: không có IR sau tối ưu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildInfo {
    pub ir_pre_sha256: String,
    pub ir_post_sha256: Option<String>,
}

/// Như `build_exe` nhưng chọn hồ sơ (`dev` = -O0; `release` = danh sách trắng pass, ADR-036).
pub fn build_exe_profile(
    art: &Artifacts,
    out: &Path,
    report_result: bool,
    attestation: &str,
    opt: Opt,
) -> Result<BuildInfo, NativeError> {
    check_llvm_dll()?;
    let rt = rt_lib()?;
    crate::runner::apply_types_fault_from_env();
    let allowed = gt_codegen_llvm::expected_instances(&art.types, &art.obl.bodies);
    // sự kiện của IR TRƯỚC tối ưu (do Guard-pre chụp) cho Guard-post, và băm hai IR
    let facts: std::cell::RefCell<gt_guard::PreFacts> =
        std::cell::RefCell::new(gt_guard::PreFacts::default());
    let hashes: std::cell::RefCell<(String, Option<String>)> =
        std::cell::RefCell::new((String::new(), None));
    let input = gt_guard::GuardInput {
        attestation,
        instances: &allowed,
    };
    let compiled = gt_codegen_llvm::compile(
        &art.types,
        &art.obl.bodies,
        &art.mir.keys,
        &Options {
            report_result,
            attestation: attestation.to_string(),
            inject: inject_from_env(),
            opt,
        },
        &|m| {
            *facts.borrow_mut() = gt_guard::pre_facts(m);
            hashes.borrow_mut().0 =
                crate::sha256::sha256_hex(m.print_to_string().to_string().as_bytes());
            gt_guard::check_module(m, &input)
        },
        &|m| {
            hashes.borrow_mut().1 = Some(crate::sha256::sha256_hex(
                m.print_to_string().to_string().as_bytes(),
            ));
            gt_guard::check_post(m, &input, &facts.borrow())
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
    #[cfg(windows)]
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
    #[cfg(not(windows))]
    cmd.arg("-no-pie")
        .arg("-Wl,--build-id=none")
        .arg("-o")
        .arg(out)
        .arg(&obj)
        .arg(&rt)
        .args(["-lpthread", "-ldl", "-lm"]);
    let mut r = cmd.output();
    for k in 1..=5u64 {
        let again = matches!(&r, Ok(o) if !o.status.success()
            && transient_link_failure(&format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))));
        if !again {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(300 * k));
        r = cmd.output();
    }
    let _ = std::fs::remove_file(&obj);
    match r {
        Ok(o) if o.status.success() && out.is_file() => {
            let (pre, post) = hashes.into_inner();
            Ok(BuildInfo {
                ir_pre_sha256: pre,
                ir_post_sha256: post,
            })
        }
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
        // `GRITC_FAULT=rt:arena-overlap|rt:arena-misalign` ⇒ `GRITRT_FAULT=arena_overlap|arena_misalign` cho exe (chèn lỗi cấp phát của gt-rt; chỉ có tác dụng
        // khi kiểm động Arena BẬT, tức hồ sơ dev; để chứng minh kiểm bắt được)
        let rt_fault = std::env::var("GRITC_FAULT")
            .ok()
            .and_then(|v| v.strip_prefix("rt:").map(|x| x.replace('-', "_")));
        let mut cmd = Command::new(exe);
        if let Some(f) = &rt_fault {
            cmd.env("GRITRT_FAULT", f);
        }
        cmd.env("GRITRT_RESULT_FILE", &result_file)
            .stdout(std::fs::File::create(&out_file)?)
            .stderr(std::fs::File::create(&err_file)?);
        let mut child = retry_transient(|| cmd.spawn())?;
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

#[cfg(test)]
mod transient_tests {
    use super::*;
    use std::io::{Error, ErrorKind};

    #[test]
    fn retries_only_transient_denials_and_never_hides_other_errors() {
        // 3 lần "Access is denied" rồi thành công ⇒ Ok sau 4 lần gọi
        let mut calls = 0;
        let r: std::io::Result<u32> = retry_transient(|| {
            calls += 1;
            if calls <= 3 {
                Err(Error::from_raw_os_error(5))
            } else {
                Ok(7)
            }
        });
        assert_eq!(r.expect("ok"), 7);
        assert_eq!(calls, 4);
        // lỗi KHÔNG thoáng qua: đúng 1 lần gọi, lỗi nguyên vẹn
        let mut calls = 0;
        let r: std::io::Result<u32> = retry_transient(|| {
            calls += 1;
            Err(Error::new(ErrorKind::NotFound, "không có"))
        });
        assert_eq!(r.expect_err("lỗi").kind(), ErrorKind::NotFound);
        assert_eq!(calls, 1);
        // từ chối kéo dài: dừng sau đúng 8 lần, trả lỗi (không treo, không nuốt)
        let mut calls = 0;
        let r: std::io::Result<u32> = retry_transient(|| {
            calls += 1;
            Err(Error::from(ErrorKind::PermissionDenied))
        });
        assert!(r.is_err());
        assert_eq!(calls, 8);
    }

    #[test]
    fn only_lnk1104_counts_as_a_transient_link_failure() {
        assert!(transient_link_failure(
            "LINK : fatal error LNK1104: cannot open file 'x.exe'"
        ));
        assert!(!transient_link_failure(
            "error LNK2019: unresolved external symbol"
        ));
        assert!(!transient_link_failure(""));
    }
}
