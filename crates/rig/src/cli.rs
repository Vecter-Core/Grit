//! Dòng lệnh `rig`: new | check | build | run | test | add | lock | fmt | clean.
//!
//! MÃ THOÁT (D17, ADR-030 §4): mã của GRITC được CHUYỂN NGUYÊN khi lỗi đến từ gritc (0 qua | 1 ĐỎ | 2 ICE | 3 lỗi dùng/IO/chưa hỗ trợ của gritc |
//! `run`: 70 abort, 100+s). Lỗi của CHÍNH RIG dùng vùng riêng 10–13 để không lẫn nghĩa với mã 3 của gritc:
//! 10 `rig.lock` thiếu (với `--locked`) hoặc lệch | 11 dự án/manifest sai (rig.toml, vòng, trùng tên/module, thiếu main) | 12 dùng sai lệnh | 13 IO/môi trường của rig.
//! `rig test` có test hỏng ⇒ 1.
use crate::lock;
use crate::project::{self, Project};
use gt_driver::pipeline::{CheckResult, DEFAULT_CHECKS, Options};
use gt_driver::render::human_report;
use gt_driver::verdict::{RunEnd, check_exit_code, run_exit_code};
use gt_resolve::SourceInput;
use std::path::{Path, PathBuf};

const USAGE: &str = "dùng: rig <lệnh> …\n  rig new <tên>            tạo gói mới (rig.toml, src/main.gt, rig.lock)\n  rig check [[--locked] [--deny-yellow]  khoá + gritc check toàn bộ đồ thị gói\n  rig build [--locked] [--deny-yellow]  check rồi dựng target/<tên>.exe (hai lần dựng ⇒ nhị phân giống hệt)\n  rig run                  build rồi chạy\n  rig test                 mỗi tests/*.gt là một chương trình có `main`; qua = thoát 0\n  rig add <đường-dẫn>      thêm phụ thuộc cục bộ vào rig.toml rồi khoá lại\n  rig lock                 ghi rig.lock (băm nội dung từng gói)\n  rig fmt [--check]        định dạng src/ và tests/ của gói gốc\n  rig clean                xoá target/\nMã thoát: mã gritc chuyển nguyên (0 qua | 1 ĐỎ/test hỏng | 2 ICE | 3 lỗi của gritc | run: 70 abort, 100+s) | mã riêng của rig: 10 rig.lock thiếu/lệch | 11 dự án/manifest sai | 12 dùng sai | 13 IO/môi trường\n";

/// Mã thoát của lỗi do CHÍNH rig (không lẫn với mã gritc 0–3/70/71/100+).
pub const E_LOCK: i32 = 10;
pub const E_PROJECT: i32 = 11;
pub const E_USAGE: i32 = 12;
pub const E_IO: i32 = 13;

fn usage_err(msg: &str) -> i32 {
    eprintln!("rig: {msg}\n{USAGE}");
    E_USAGE
}

fn fail_with(code: i32, msg: &str) -> i32 {
    eprintln!("rig: {msg}");
    code
}

/// lỗi IO/môi trường của rig
fn fail(msg: &str) -> i32 {
    fail_with(E_IO, msg)
}

struct Args {
    cmd: String,
    pos: Vec<String>,
    deny_yellow: bool,
    check: bool,
    locked: bool,
    no_std: bool,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let mut it = args.iter();
    let cmd = it.next().ok_or_else(|| "thiếu lệnh".to_string())?.clone();
    let mut a = Args {
        cmd,
        pos: Vec::new(),
        deny_yellow: false,
        check: false,
        locked: false,
        no_std: false,
    };
    for s in it {
        match s.as_str() {
            "--deny-yellow" => a.deny_yellow = true,
            "--check" => a.check = true,
            "--locked" => a.locked = true,
            "--no-std" => a.no_std = true,
            x if x.starts_with('-') => return Err(format!("đối số lạ `{x}`")),
            x => a.pos.push(x.to_string()),
        }
    }
    Ok(a)
}

/// Đọc dự án + ĐỐI CHIẾU rig.lock (chưa có ⇒ tạo và báo; lệch ⇒ từ chối, mã 3).
fn load_locked(dir: &Path, locked: bool) -> Result<Project, i32> {
    let p = project::load(dir).map_err(|e| fail_with(E_PROJECT, &e))?;
    let lock_path = dir.join("rig.lock");
    match std::fs::read_to_string(&lock_path) {
        Ok(existing) => lock::verify(&p, &existing).map_err(|e| fail_with(E_LOCK, &e))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && locked => {
            return Err(fail_with(
                E_LOCK,
                "--locked: không có rig.lock (chạy `rig lock` trước)",
            ));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::fs::write(&lock_path, lock::render(&p))
                .map_err(|e| fail(&format!("không ghi được rig.lock: {e}")))?;
            eprintln!(
                "rig: chưa có rig.lock — đã tạo (băm nội dung {} gói)",
                p.pkgs.len()
            );
        }
        Err(e) => return Err(fail(&format!("không đọc được rig.lock: {e}"))),
    }
    Ok(p)
}

/// Cổng phán quyết của gritc trên một tập nguồn. `Err(mã)` = đã in lý do.
fn gate(sources: &[(String, String)], deny_yellow: bool) -> Result<CheckResult, i32> {
    let inputs: Vec<SourceInput> = sources
        .iter()
        .map(|(n, t)| SourceInput {
            name: n.clone(),
            text: t.clone(),
        })
        .collect();
    let res = gt_driver::check(
        &inputs,
        &Options {
            checks: DEFAULT_CHECKS,
            deny_yellow,
            fault: None,
        },
    )
    .map_err(|e| {
        // lỗi đầu vào do gritc từ chối (vd tên module không hợp lệ): chuyển mã 3 của gritc
        eprintln!("rig: gritc: đầu vào không hợp lệ: {e}");
        3
    })?;
    eprint!("{}", human_report(&res, false));
    if !res.verdict.allows_output() {
        return Err(check_exit_code(res.verdict));
    }
    Ok(res)
}

#[cfg(feature = "llvm")]
fn compile(res: &mut CheckResult, out: &Path) -> Result<(), i32> {
    use gt_driver::native::{NativeError, build_exe};
    let Some(art) = res.open_gate() else {
        return Err(2);
    };
    if let Err(e) = gt_driver::runner::find_main(&art.obl.bodies) {
        return Err(fail_with(E_PROJECT, &e));
    }
    let att = gt_driver::pipeline::attestation(res, &gt_driver::pipeline::mir_dump(art));
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| fail(&format!("không tạo được {}: {e}", parent.display())))?;
    }
    match build_exe(art, out, false, &att) {
        Ok(()) => Ok(()),
        Err(NativeError::Guard(v)) => {
            res.reject_ir(&v);
            res.backend = "llvm";
            eprint!("{}", human_report(res, false));
            Err(check_exit_code(res.verdict))
        }
        Err(e) => {
            eprintln!("rig: backend llvm: {e}");
            Err(e.exit_code())
        }
    }
}

#[cfg(not(feature = "llvm"))]
fn compile(_res: &mut CheckResult, _out: &Path) -> Result<(), i32> {
    Err(fail(
        "bản rig này dựng không có backend llvm (--no-default-features)",
    ))
}

fn exe_path(dir: &Path, name: &str) -> PathBuf {
    dir.join("target").join(format!("{name}.exe"))
}

fn cwd() -> Result<PathBuf, i32> {
    std::env::current_dir().map_err(|e| fail(&format!("không biết thư mục hiện tại: {e}")))
}

fn cmd_check(a: &Args) -> i32 {
    let dir = match cwd() {
        Ok(d) => d,
        Err(c) => return c,
    };
    let p = match load_locked(&dir, a.locked) {
        Ok(p) => p,
        Err(c) => return c,
    };
    match gate(&p.sources(false), a.deny_yellow) {
        Ok(_) => 0,
        Err(c) => c,
    }
}

fn build_root(a: &Args, dir: &Path) -> Result<(Project, PathBuf), i32> {
    let p = load_locked(dir, a.locked)?;
    let mut res = gate(&p.sources(false), a.deny_yellow)?;
    let out = exe_path(dir, &p.root().name);
    compile(&mut res, &out)?;
    Ok((p, out))
}

fn cmd_build(a: &Args) -> i32 {
    let dir = match cwd() {
        Ok(d) => d,
        Err(c) => return c,
    };
    match build_root(a, &dir) {
        Ok((_, out)) => {
            eprintln!("rig: đã dựng {}", out.display());
            0
        }
        Err(c) => c,
    }
}

fn cmd_run(a: &Args) -> i32 {
    let dir = match cwd() {
        Ok(d) => d,
        Err(c) => return c,
    };
    let out = match build_root(a, &dir) {
        Ok((_, o)) => o,
        Err(c) => return c,
    };
    match std::process::Command::new(&out).status() {
        Ok(st) => match st.code() {
            Some(0) => 0,
            Some(70) => run_exit_code(&RunEnd::Aborted(String::new())),
            Some(c) => run_exit_code(&RunEnd::ProgramExit(
                u8::try_from(c.clamp(1, 255)).unwrap_or(255),
            )),
            None => 2,
        },
        Err(e) => fail(&format!("không chạy được {}: {e}", out.display())),
    }
}

#[cfg(feature = "llvm")]
fn cmd_test(a: &Args) -> i32 {
    let dir = match cwd() {
        Ok(d) => d,
        Err(c) => return c,
    };
    let p = match load_locked(&dir, a.locked) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let tdir = dir.join("tests");
    let mut tests: Vec<(String, String)> = Vec::new();
    // thư mục tests/ KHÔNG có thì thôi, nhưng có mà đọc lỗi/mục lỗi thì phải báo — không được âm thầm chạy ít test hơn (FAIL_OPEN_AUDIT R3)
    let rd = match std::fs::read_dir(&tdir) {
        Ok(rd) => Some(rd),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return fail(&format!("không đọc được thư mục tests/: {e}")),
    };
    if let Some(rd) = rd {
        for e in rd {
            let e = match e {
                Ok(e) => e,
                Err(err) => return fail(&format!("lỗi đọc mục trong tests/: {err}")),
            };
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".gt") {
                match std::fs::read_to_string(e.path()) {
                    Ok(t) => tests.push((name, t.replace("\r\n", "\n"))),
                    Err(err) => return fail(&format!("không đọc được tests/{name}: {err}")),
                }
            }
        }
    }
    tests.sort();
    if tests.is_empty() {
        println!("0 test (thư mục tests/ trống hoặc không có)");
        return 0;
    }
    let (mut pass, mut failed, mut ice) = (0, 0, false);
    for (name, text) in &tests {
        let stem = name.trim_end_matches(".gt");
        let mut srcs = p.sources(true);
        if srcs.iter().any(|(n, _)| n == name) {
            return fail_with(
                E_PROJECT,
                &format!("tests/{name} trùng tên module với một tệp src/"),
            );
        }
        srcs.push((name.clone(), text.clone()));
        srcs.sort();
        let res = {
            let inputs: Vec<SourceInput> = srcs
                .iter()
                .map(|(n, t)| SourceInput {
                    name: n.clone(),
                    text: t.clone(),
                })
                .collect();
            gt_driver::check(
                &inputs,
                &Options {
                    checks: DEFAULT_CHECKS,
                    deny_yellow: a.deny_yellow,
                    fault: None,
                },
            )
        };
        let mut res = match res {
            Ok(r) => r,
            Err(e) => {
                eprintln!("rig: gritc: tests/{name}: đầu vào không hợp lệ: {e}");
                return 3;
            }
        };
        if !res.verdict.allows_output() {
            eprint!("{}", human_report(&res, false));
            println!(
                "test {stem} ... FAILED (cổng gritc: {})",
                res.verdict.name()
            );
            ice |= check_exit_code(res.verdict) == 2;
            failed += 1;
            continue;
        }
        let out = dir.join("target").join("tests").join(format!("{stem}.exe"));
        match compile(&mut res, &out) {
            Ok(()) => {}
            Err(c) => {
                println!("test {stem} ... FAILED (dựng thất bại, mã {c})");
                ice |= c == 2;
                failed += 1;
                continue;
            }
        }
        match gt_driver::native::run_exe(&out).end {
            RunEnd::Normal | RunEnd::ProgramExit(0) => {
                println!("test {stem} ... ok");
                pass += 1;
            }
            RunEnd::Aborted(k) => {
                println!("test {stem} ... FAILED (abort: {k})");
                failed += 1;
            }
            other => {
                println!("test {stem} ... FAILED ({other:?})");
                failed += 1;
            }
        }
    }
    println!("kết quả: {pass} qua, {failed} hỏng");
    if ice { 2 } else { i32::from(failed > 0) }
}

#[cfg(not(feature = "llvm"))]
fn cmd_test(_a: &Args) -> i32 {
    fail("bản rig này dựng không có backend llvm (--no-default-features)")
}

fn cmd_lock() -> i32 {
    let dir = match cwd() {
        Ok(d) => d,
        Err(c) => return c,
    };
    let p = match project::load(&dir) {
        Ok(p) => p,
        Err(e) => return fail_with(E_PROJECT, &e),
    };
    match std::fs::write(dir.join("rig.lock"), lock::render(&p)) {
        Ok(()) => {
            eprintln!("rig: đã ghi rig.lock ({} gói)", p.pkgs.len());
            0
        }
        Err(e) => fail(&format!("không ghi được rig.lock: {e}")),
    }
}

/// Thư mục `lib/std` của bản Grit đang chạy: biến `GRIT_STD`, hoặc `lib/std` ở một thư mục tổ tiên của tệp `rig` (bố cục kho: `target/<hồ-sơ>/rig`).
fn find_std() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("GRIT_STD") {
        let p = PathBuf::from(p);
        return p.join("rig.toml").is_file().then_some(p);
    }
    let exe = std::env::current_exe().ok()?;
    exe.ancestors()
        .skip(1)
        .map(|d| d.join("lib").join("std"))
        .find(|c| c.join("rig.toml").is_file())
}

fn cmd_new(a: &Args) -> i32 {
    let [name] = a.pos.as_slice() else {
        return usage_err("`rig new` cần đúng một tên gói");
    };
    if !project::valid_name(name) {
        return fail_with(
            E_PROJECT,
            &format!("tên gói `{name}` không hợp lệ (chữ thường/số/`_`, bắt đầu bằng chữ cái)"),
        );
    }
    let dir = match cwd() {
        Ok(d) => d.join(name),
        Err(c) => return c,
    };
    if dir.exists() {
        return fail(&format!("{} đã tồn tại", dir.display()));
    }
    let std_dir = if a.no_std { None } else { find_std() };
    let std_dep = match &std_dir {
        Some(p) => format!(
            "std = {{ path = \"{}\" }}\n",
            p.display().to_string().replace('\\', "/")
        ),
        None => String::new(),
    };
    if std_dir.is_none() && !a.no_std {
        eprintln!(
            "rig: không tìm thấy lib/std (đặt GRIT_STD hoặc chạy `rig add <đường-dẫn lib/std>`); gói mới không có `std`"
        );
    }
    let mk = || -> std::io::Result<()> {
        std::fs::create_dir_all(dir.join("src"))?;
        std::fs::write(
            dir.join("rig.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n\n[dependencies]\n{std_dep}"
            ),
        )?;
        std::fs::write(
            dir.join("src").join("main.gt"),
            format!("// {name}: gói Grit mới (rig new)\nƒ main() → i32 ! ∅ {{ 0 }}\n"),
        )?;
        std::fs::write(dir.join(".gitignore"), "target/\n")?;
        Ok(())
    };
    if let Err(e) = mk() {
        let _ = std::fs::remove_dir_all(&dir);
        return fail(&format!("không tạo được gói: {e}"));
    }
    let p = match project::load(&dir) {
        Ok(p) => p,
        Err(e) => return fail_with(E_PROJECT, &e),
    };
    if let Err(e) = std::fs::write(dir.join("rig.lock"), lock::render(&p)) {
        return fail(&format!("không ghi được rig.lock: {e}"));
    }
    eprintln!("rig: đã tạo gói `{name}` tại {}", dir.display());
    0
}

fn cmd_add(a: &Args) -> i32 {
    let [path] = a.pos.as_slice() else {
        return usage_err("`rig add` cần đúng một đường dẫn");
    };
    let dir = match cwd() {
        Ok(d) => d,
        Err(c) => return c,
    };
    let dep_name = match project::parse_manifest(&dir.join(path)) {
        Ok(m) => m.name,
        Err(e) => return fail_with(E_PROJECT, &e),
    };
    let manifest = dir.join("rig.toml");
    let original = match std::fs::read_to_string(&manifest) {
        Ok(t) => t.replace("\r\n", "\n"),
        Err(e) => return fail(&format!("không đọc được rig.toml: {e}")),
    };
    let deps = match project::parse_manifest(&dir) {
        Ok(m) => m.deps,
        Err(e) => return fail_with(E_PROJECT, &e),
    };
    if deps.iter().any(|(n, _)| *n == dep_name) {
        return fail_with(E_PROJECT, &format!("đã có phụ thuộc `{dep_name}`"));
    }
    let entry = format!(
        "{dep_name} = {{ path = \"{}\" }}\n",
        path.replace('\\', "/")
    );
    let mut lines: Vec<String> = original.lines().map(str::to_string).collect();
    match lines.iter().position(|l| l.trim() == "[dependencies]") {
        Some(i) => {
            // chèn sau dòng khoá cuối của mục (trước mục kế hoặc cuối tệp)
            let mut at = i + 1;
            while at < lines.len() && !lines[at].trim_start().starts_with('[') {
                at += 1;
            }
            while at > i + 1 && lines[at - 1].trim().is_empty() {
                at -= 1;
            }
            lines.insert(at, entry.trim_end().to_string());
        }
        None => {
            lines.push(String::new());
            lines.push("[dependencies]".to_string());
            lines.push(entry.trim_end().to_string());
        }
    }
    let edited = format!("{}\n", lines.join("\n"));
    if let Err(e) = std::fs::write(&manifest, &edited) {
        return fail(&format!("không ghi được rig.toml: {e}"));
    }
    match project::load(&dir) {
        Ok(p) => match std::fs::write(dir.join("rig.lock"), lock::render(&p)) {
            Ok(()) => {
                eprintln!("rig: đã thêm phụ thuộc `{dep_name}` ({path}) và khoá lại");
                0
            }
            Err(e) => fail(&format!("không ghi được rig.lock: {e}")),
        },
        Err(e) => {
            // hoàn nguyên: không để rig.toml hỏng
            let _ = std::fs::write(&manifest, original);
            fail_with(
                E_PROJECT,
                &format!("không thêm được `{dep_name}`: {e} (đã hoàn nguyên rig.toml)"),
            )
        }
    }
}

fn cmd_fmt(a: &Args) -> i32 {
    let dir = match cwd() {
        Ok(d) => d,
        Err(c) => return c,
    };
    if let Err(e) = project::load(&dir) {
        return fail_with(E_PROJECT, &e);
    }
    let mut code = 0;
    for sub in ["src", "tests"] {
        let rd = match std::fs::read_dir(dir.join(sub)) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return fail(&format!("không đọc được thư mục {sub}/: {e}")),
        };
        let mut files: Vec<PathBuf> = Vec::new();
        for e in rd {
            // mục đọc lỗi không được âm thầm bị bỏ khỏi việc định dạng/kiểm (FAIL_OPEN_AUDIT R3)
            let p = match e {
                Ok(e) => e.path(),
                Err(err) => return fail(&format!("lỗi đọc mục trong {sub}/: {err}")),
            };
            if p.extension().is_some_and(|x| x == "gt") {
                files.push(p);
            }
        }
        files.sort();
        for f in files {
            let name = f
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            let text = match std::fs::read_to_string(&f) {
                Ok(t) => t.replace("\r\n", "\n"),
                Err(e) => return fail(&format!("không đọc được {}: {e}", f.display())),
            };
            match gt_fmt::format(&name, &text) {
                Ok(out) if out == text => {}
                Ok(out) => {
                    if a.check {
                        eprintln!("rig fmt: {sub}/{name} chưa đúng định dạng chuẩn");
                        code = 1;
                    } else {
                        let mut tmp = f.as_os_str().to_owned();
                        tmp.push(".tmp");
                        let tmp = PathBuf::from(tmp);
                        if let Err(e) =
                            std::fs::write(&tmp, &out).and_then(|()| std::fs::rename(&tmp, &f))
                        {
                            let _ = std::fs::remove_file(&tmp);
                            return fail(&format!("không ghi được {}: {e}", f.display()));
                        }
                        eprintln!(
                            "rig fmt: đã định dạng {sub}/{name} (chạy `rig lock` nếu là gói đã khoá)"
                        );
                    }
                }
                Err(e) => {
                    // từ chối định dạng: tuyệt đối KHÔNG sửa tệp
                    for d in &e.report.diagnostics {
                        eprintln!("rig fmt: {sub}/{name}: {}: {}", d.code.as_str(), d.message);
                    }
                    code = e.report.exit_code(false).max(code);
                }
            }
        }
    }
    code
}

fn cmd_clean() -> i32 {
    let dir = match cwd() {
        Ok(d) => d,
        Err(c) => return c,
    };
    if !dir.join("rig.toml").is_file() {
        return fail_with(
            E_PROJECT,
            "không có rig.toml ở thư mục hiện tại (không xoá gì)",
        );
    }
    let t = dir.join("target");
    if t.exists() {
        if let Err(e) = std::fs::remove_dir_all(&t) {
            return fail(&format!("không xoá được target/: {e}"));
        }
        eprintln!("rig: đã xoá target/");
    }
    0
}

#[must_use]
pub fn main(args: Vec<String>) -> i32 {
    if matches!(args.first().map(String::as_str), Some("--help" | "-h")) {
        print!("{USAGE}");
        return 0;
    }
    let a = match parse(&args) {
        Ok(a) => a,
        Err(e) => return usage_err(&e),
    };
    match a.cmd.as_str() {
        "new" => cmd_new(&a),
        "check" => cmd_check(&a),
        "build" => cmd_build(&a),
        "run" => cmd_run(&a),
        "test" => cmd_test(&a),
        "add" => cmd_add(&a),
        "lock" => cmd_lock(),
        "fmt" => cmd_fmt(&a),
        "clean" => cmd_clean(),
        other => usage_err(&format!("lệnh lạ `{other}`")),
    }
}
