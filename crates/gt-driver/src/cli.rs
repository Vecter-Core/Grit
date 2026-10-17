//! Dòng lệnh `gritc` (ADR-028 §1): `check`, `build`, `run`, `explain`, `fmt`. Toàn bộ logic phán quyết ở `pipeline`/`verdict`; ở đây chỉ phân tích
//! đối số, đọc đầu vào, gọi cổng và ánh xạ mã thoát. Không có đối số ẩn, không cấu hình ngầm.
use crate::pipeline::{CheckResult, Fault, Options, check};
use crate::render::{Phase, human_report, json_report};
use crate::runner::{RunReport, run_main, write_package};
use crate::verdict::{RunEnd, check_exit_code, run_exit_code};
use gt_oblig::driver::Checks;
use gt_resolve::SourceInput;
use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/explain_table.rs"));

const USAGE: &str = "dùng: gritc <lệnh> …\n  gritc check <tệp|thư mục> [--json] [--deny-yellow] [--checks=all|proven (mặc định: all; proven là THỬ NGHIỆM: xoá check đã chứng minh, in ghi chú N0606, không dùng với release)]\n  gritc build <tệp|thư mục> [-o ra.exe|ra.gtmir] [--backend=llvm|interpreter (mặc định: llvm nếu bản dựng có)] [--profile=dev|release|release-o2 (mặc định: dev; release và release-o2 đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh: ADR-036 §9)] [--json] [--deny-yellow] [--checks=all|proven (mặc định: all; proven là THỬ NGHIỆM: xoá check đã chứng minh, in ghi chú N0606, không dùng với release)]\n  gritc run   <tệp|thư mục> [--backend=llvm|interpreter (mặc định: llvm nếu bản dựng có)] [--profile=dev|release|release-o2 (mặc định: dev; release và release-o2 đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh: ADR-036 §9)] [--json] [--deny-yellow] [--checks=all|proven (mặc định: all; proven là THỬ NGHIỆM: xoá check đã chứng minh, in ghi chú N0606, không dùng với release)]\n  gritc explain <mã>\n  gritc fmt <tệp> [--check]\n  gritc --version\nMã thoát: 0 qua | 1 ĐỎ (hoặc VÀNG khi --deny-yellow) | 2 ICE | 3 lỗi dùng/IO | run: 70 abort, 71 hết giới hạn bộ chạy, 100+s trạng thái chương trình\n";

#[derive(Debug)]
struct Cli {
    cmd: String,
    path: Option<String>,
    json: bool,
    deny_yellow: bool,
    checks: Checks,
    out: Option<String>,
    fmt_check: bool,
    backend: Backend,
    profile: gt_codegen_llvm::Opt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Backend {
    Interpreter,
    Llvm,
}

/// Backend mặc định (ADR-029 Q63): `llvm` khi bản dựng có feature `llvm`, ngược lại `interpreter`. Luôn được ghi rõ trong đầu ra.
const fn default_backend() -> Backend {
    if cfg!(feature = "llvm") {
        Backend::Llvm
    } else {
        Backend::Interpreter
    }
}

fn usage_err(msg: &str) -> i32 {
    eprintln!("gritc: {msg}\n{USAGE}");
    3
}

fn parse(args: &[String]) -> Result<Cli, String> {
    let mut it = args.iter();
    let cmd = it.next().ok_or_else(|| "thiếu lệnh".to_string())?.clone();
    let mut cli = Cli {
        cmd,
        path: None,
        json: false,
        deny_yellow: false,
        checks: crate::pipeline::DEFAULT_CHECKS,
        out: None,
        fmt_check: false,
        backend: default_backend(),
        profile: gt_codegen_llvm::Opt::Dev,
    };
    while let Some(a) = it.next() {
        match a.as_str() {
            "--json" => cli.json = true,
            "--deny-yellow" => cli.deny_yellow = true,
            "--check" => cli.fmt_check = true,
            "--checks=all" => cli.checks = Checks::All,
            "--checks=proven" => cli.checks = Checks::Proven,
            "--backend=interpreter" => cli.backend = Backend::Interpreter,
            "--backend=llvm" => cli.backend = Backend::Llvm,
            "--profile=dev" => cli.profile = gt_codegen_llvm::Opt::Dev,
            "--profile=release" => cli.profile = gt_codegen_llvm::Opt::Release,
            "--profile=release-o2" => cli.profile = gt_codegen_llvm::Opt::ReleaseO2,
            "-o" => {
                cli.out = Some(
                    it.next()
                        .ok_or_else(|| "`-o` cần đường dẫn".to_string())?
                        .clone(),
                );
            }
            s if s.starts_with("--backend=") => {
                return Err(format!(
                    "`{s}`: chỉ nhận --backend=llvm hoặc --backend=interpreter"
                ));
            }
            s if s.starts_with("--profile=") => {
                return Err(format!(
                    "`{s}`: chỉ nhận --profile=dev, --profile=release hoặc --profile=release-o2"
                ));
            }
            s if s.starts_with("--checks=") => {
                return Err(format!("`{s}`: chỉ nhận --checks=all hoặc --checks=proven"));
            }
            s if s.starts_with('-') => return Err(format!("đối số lạ `{s}`")),
            s => {
                if cli.path.is_some() {
                    return Err(format!("thừa đối số `{s}`"));
                }
                cli.path = Some(s.to_string());
            }
        }
    }
    // chế độ XOÁ check (`--checks=proven`) là THỬ NGHIỆM và KHÔNG được dùng bởi hồ sơ release (lệnh Giang 2026-10-14, mục 5)
    if cli.checks == Checks::Proven && cli.profile.is_release() {
        return Err(
            "`--checks=proven` (thử nghiệm: xoá check đã chứng minh) không được dùng cùng `--profile=release*`: hồ sơ release luôn giữ mọi check"
                .to_string(),
        );
    }
    Ok(cli)
}

/// Đọc đầu vào: một tệp `.gt` hoặc mọi `.gt` trong thư mục (sắp theo tên).
fn load(path: &str) -> Result<Vec<SourceInput>, String> {
    let p = Path::new(path);
    let read = |q: &Path| -> Result<SourceInput, String> {
        let text = std::fs::read_to_string(q)
            .map_err(|e| format!("không đọc được {}: {e}", q.display()))?
            .replace("\r\n", "\n");
        let name = q
            .file_name()
            .map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned());
        Ok(SourceInput { name, text })
    };
    if p.is_dir() {
        let mut files: Vec<PathBuf> = Vec::new();
        for e in std::fs::read_dir(p).map_err(|e| format!("không đọc được thư mục {path}: {e}"))?
        {
            // mục thư mục không đọc được KHÔNG được âm thầm bị bỏ khỏi tập đầu vào (sẽ cho phán quyết xanh trên tập con) — FAIL_OPEN_AUDIT R3
            let q = e
                .map_err(|e| format!("không đọc được mục trong thư mục {path}: {e}"))?
                .path();
            if q.extension().is_some_and(|x| x == "gt") {
                files.push(q);
            }
        }
        files.sort();
        if files.is_empty() {
            return Err(format!("thư mục {path} không có tệp .gt"));
        }
        files.iter().map(|q| read(q)).collect()
    } else if p.is_file() {
        Ok(vec![read(p)?])
    } else {
        Err(format!("không có tệp hoặc thư mục {path}"))
    }
}

fn options(cli: &Cli) -> Options {
    // Chèn lỗi để KIỂM cổng (chỉ làm gritc từ chối thêm): GRITC_FAULT=panic:<pass> | skip:<pass>
    let fault = std::env::var("GRITC_FAULT").ok().and_then(|v| {
        let (k, name) = v.split_once(':')?;
        match k {
            "panic" => Some(Fault::Panic(name.to_string())),
            "skip" => Some(Fault::Skip(name.to_string())),
            _ => None,
        }
    });
    Options {
        checks: cli.checks,
        deny_yellow: cli.deny_yellow,
        fault,
    }
}

fn emit(res: &CheckResult, cli: &Cli, phase: Phase, run: Option<&RunReport>, att: Option<&str>) {
    if cli.json {
        println!("{}", json_report(res, cli.deny_yellow, phase, run, att));
    } else {
        eprint!("{}", human_report(res, false));
    }
}

fn cmd_check(cli: &Cli) -> i32 {
    let Some(path) = &cli.path else {
        return usage_err("thiếu đường dẫn");
    };
    let inputs = match load(path) {
        Ok(i) => i,
        Err(e) => return usage_err(&e),
    };
    let res = match check(&inputs, &options(cli)) {
        Ok(r) => r,
        Err(e) => return usage_err(&e),
    };
    emit(&res, cli, Phase::Check, None, None);
    check_exit_code(res.verdict)
}

/// Lỗi backend llvm ⇒ thông báo + mã thoát. IR Guard từ chối ⇒ ĐỎ (E1101, mã 1) in như mọi chẩn đoán; còn lại ⇒ thông báo môi trường/nội bộ.
#[cfg(feature = "llvm")]
fn native_failure(res: &mut CheckResult, cli: &Cli, e: &crate::native::NativeError) -> i32 {
    if let crate::native::NativeError::Guard(v) = e {
        res.reject_ir(v);
        res.backend = "llvm";
        let phase = if cli.cmd == "run" {
            Phase::Run
        } else {
            Phase::Check
        };
        emit(res, cli, phase, None, None);
        return check_exit_code(res.verdict);
    }
    eprintln!("gritc: backend llvm: {e}");
    e.exit_code()
}

#[cfg(feature = "llvm")]
fn build_llvm(cli: &Cli, mut res: CheckResult, path: &str) -> i32 {
    let code = check_exit_code(res.verdict);
    let Some(art) = res.open_gate() else {
        return 2;
    };
    let out = cli.out.clone().unwrap_or_else(|| {
        let stem = Path::new(path)
            .file_stem()
            .map_or_else(|| "out".to_string(), |s| s.to_string_lossy().into_owned());
        format!("{stem}.exe")
    });
    let att = crate::pipeline::attestation(&res, &crate::pipeline::mir_dump(art));
    match crate::native::build_exe_profile(art, Path::new(&out), false, &att, cli.profile) {
        Err(e) => return native_failure(&mut res, cli, &e),
        Ok(info) => {
            if let Some(post) = &info.ir_post_sha256 {
                eprintln!(
                    "gritc: hồ sơ release; băm IR trước tối ưu {} và sau tối ưu {post}",
                    info.ir_pre_sha256
                );
            }
        }
    }
    res.backend = "llvm";
    emit(&res, cli, Phase::Check, None, Some(&att));
    if !cli.json {
        eprintln!("gritc: đã ghi {out} (backend: llvm)");
    }
    code
}

#[cfg(not(feature = "llvm"))]
fn build_llvm(_cli: &Cli, _res: CheckResult, _path: &str) -> i32 {
    usage_err("bản gritc này dựng không có backend llvm (--no-default-features)")
}

/// Dựng exe vào thư mục tạm, chạy, dọn. `Err(mã)`: lỗi môi trường/codegen đã in thông báo.
#[cfg(feature = "llvm")]
fn run_llvm(
    art: &crate::pipeline::Artifacts,
    res: &mut CheckResult,
    cli: &Cli,
) -> Result<RunReport, i32> {
    let dir = std::env::temp_dir().join(format!("gritc-{}", std::process::id()));
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Err(usage_err(&format!("không tạo được {}: {e}", dir.display())));
    }
    let exe = dir.join("prog.exe");
    let att = crate::pipeline::attestation(res, &crate::pipeline::mir_dump(art));
    let r = match crate::native::build_exe_profile(art, &exe, true, &att, cli.profile) {
        Ok(_) => Ok(crate::native::run_exe(&exe)),
        Err(e) => Err(native_failure(res, cli, &e)),
    };
    let _ = std::fs::remove_dir_all(&dir);
    r
}

#[cfg(not(feature = "llvm"))]
fn run_llvm(
    _art: &crate::pipeline::Artifacts,
    _res: &mut CheckResult,
    _cli: &Cli,
) -> Result<RunReport, i32> {
    Err(usage_err(
        "bản gritc này dựng không có backend llvm (--no-default-features)",
    ))
}

fn cmd_build(cli: &Cli) -> i32 {
    let Some(path) = &cli.path else {
        return usage_err("thiếu đường dẫn");
    };
    let inputs = match load(path) {
        Ok(i) => i,
        Err(e) => return usage_err(&e),
    };
    let res = match check(&inputs, &options(cli)) {
        Ok(r) => r,
        Err(e) => return usage_err(&e),
    };
    let code = check_exit_code(res.verdict);
    // CỔNG: chỉ XANH/VÀNG mới có sản phẩm để ghi. ĐỎ/ICE ⇒ không tạo tệp nào.
    let Some(art) = res.open_gate() else {
        emit(&res, cli, Phase::Check, None, None);
        return code;
    };
    if cli.backend == Backend::Llvm {
        return build_llvm(cli, res, path);
    }
    let out = cli.out.clone().unwrap_or_else(|| {
        let stem = Path::new(path)
            .file_stem()
            .map_or_else(|| "out".to_string(), |s| s.to_string_lossy().into_owned());
        format!("{stem}.gtmir")
    });
    match write_package(Path::new(&out), &res, art) {
        Ok(att) => {
            emit(&res, cli, Phase::Check, None, Some(&att));
            if !cli.json {
                eprintln!("gritc: đã ghi gói {out} (backend: {})", res.backend);
            }
            code
        }
        Err(e) => usage_err(&e),
    }
}

fn cmd_run(cli: &Cli) -> i32 {
    let Some(path) = &cli.path else {
        return usage_err("thiếu đường dẫn");
    };
    let inputs = match load(path) {
        Ok(i) => i,
        Err(e) => return usage_err(&e),
    };
    let res = match check(&inputs, &options(cli)) {
        Ok(r) => r,
        Err(e) => return usage_err(&e),
    };
    let mut res = res;
    if cli.backend == Backend::Llvm {
        res.backend = "llvm";
    }
    let code = check_exit_code(res.verdict);
    if !res.verdict.allows_output() {
        emit(&res, cli, Phase::Check, None, None);
        return code;
    }
    // VÀNG: chạy và in cảnh báo (trước khi chạy)
    if !cli.json {
        eprint!("{}", human_report(&res, false));
    }
    // CỔNG: sản phẩm chỉ lấy được khi XANH/VÀNG
    let Some(art) = res.take_gated() else {
        eprintln!("gritc: lỗi nội bộ: cổng mở nhưng không có sản phẩm để chạy");
        return 2;
    };
    let rr = if cli.backend == Backend::Llvm {
        match run_llvm(&art, &mut res, cli) {
            Ok(r) => {
                res.backend = "llvm";
                r
            }
            Err(code) => return code,
        }
    } else {
        run_main(art)
    };
    let exit = run_exit_code(&rr.end);
    if cli.json {
        println!(
            "{}",
            json_report(&res, cli.deny_yellow, Phase::Run, Some(&rr), None)
        );
    } else {
        match &rr.end {
            RunEnd::Normal | RunEnd::ProgramExit(0) => println!(
                "chạy xong (backend: {}); main trả về {}",
                res.backend,
                rr.value.as_deref().unwrap_or("()")
            ),
            RunEnd::Aborted(k) => eprintln!(
                "chương trình abort: {k} (mã thoát {exit}, backend: {})",
                res.backend
            ),
            RunEnd::Limit => eprintln!(
                "bộ chạy tham chiếu hết giới hạn (nhiên liệu/độ sâu); chương trình chưa kết thúc (mã thoát {exit})"
            ),
            RunEnd::ProgramExit(s) => {
                eprintln!("chương trình thoát với trạng thái {s} (mã thoát {exit})")
            }
            RunEnd::Internal(m) => eprintln!("gritc: lỗi nội bộ khi chạy: {m}"),
        }
    }
    exit
}

fn cmd_explain(cli: &Cli) -> i32 {
    let Some(code) = &cli.path else {
        return usage_err("thiếu mã (vd `gritc explain E0601`)");
    };
    if gt_diag::Code::parse(code).is_err() {
        return usage_err(&format!(
            "`{code}` không phải mã hợp lệ (dạng E0601, S101, N0601)"
        ));
    }
    match EXPLAIN.iter().find(|(c, _)| c == code) {
        Some((_, doc)) => {
            println!("{doc}");
            0
        }
        None => usage_err(&format!(
            "chưa có tài liệu cho mã `{code}` trong docs/errors/"
        )),
    }
}

fn cmd_fmt(cli: &Cli) -> i32 {
    let Some(path) = &cli.path else {
        return usage_err("thiếu đường dẫn");
    };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t.replace("\r\n", "\n"),
        Err(e) => return usage_err(&format!("không đọc được {path}: {e}")),
    };
    let name = Path::new(path)
        .file_name()
        .map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
    match gt_fmt::format(&name, &text) {
        Ok(out) => {
            if cli.fmt_check {
                if out == text {
                    0
                } else {
                    eprintln!("gritc fmt: {path} chưa đúng định dạng chuẩn");
                    1
                }
            } else {
                if out != text {
                    let mut tmp = Path::new(path).as_os_str().to_owned();
                    tmp.push(".tmp");
                    let tmp = PathBuf::from(tmp);
                    if let Err(e) =
                        std::fs::write(&tmp, &out).and_then(|()| std::fs::rename(&tmp, path))
                    {
                        let _ = std::fs::remove_file(&tmp);
                        return usage_err(&format!("không ghi được {path}: {e}"));
                    }
                }
                0
            }
        }
        Err(e) => {
            // từ chối định dạng (E1301 khi không gắn được comment, lỗi cú pháp, ICE): tuyệt đối KHÔNG sửa tệp
            let sf = gt_span::SourceFile::new(name.clone(), text.clone()).ok();
            for d in &e.report.diagnostics {
                match &sf {
                    Some(f) => eprint!(
                        "{}",
                        gt_diag::render_human(d, f, &gt_diag::RenderOptions { color: false })
                    ),
                    None => eprintln!("{}: {}", d.code.as_str(), d.message),
                }
            }
            e.report.exit_code(false)
        }
    }
}

/// Điểm vào: trả mã thoát.
#[must_use]
pub fn main(args: Vec<String>) -> i32 {
    if args.first().map(String::as_str) == Some("--version") {
        println!(
            "gritc {} (backend mặc định: {})",
            env!("CARGO_PKG_VERSION"),
            if default_backend() == Backend::Llvm {
                "llvm"
            } else {
                "interpreter"
            }
        );
        return 0;
    }
    if matches!(args.first().map(String::as_str), Some("--help" | "-h")) {
        print!("{USAGE}");
        return 0;
    }
    let cli = match parse(&args) {
        Ok(c) => c,
        Err(e) => return usage_err(&e),
    };
    match cli.cmd.as_str() {
        "check" => cmd_check(&cli),
        "build" => cmd_build(&cli),
        "run" => cmd_run(&cli),
        "explain" => cmd_explain(&cli),
        "fmt" => cmd_fmt(&cli),
        other => usage_err(&format!("lệnh lạ `{other}`")),
    }
}
