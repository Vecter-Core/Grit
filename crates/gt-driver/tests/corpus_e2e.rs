//! E2E CORPUS — bộ chạy DUY NHẤT cho corpus `tests/{pass,fail,warn}`: chỉ nói chuyện với `gritc check --json` (giao diện đóng băng, `docs/spec/diag_schema.json`),
//! KHÔNG gọi API Rust nào của các crate bên trong. Nhờ vậy viết lại toàn bộ ruột compiler thì bộ này vẫn chạy nguyên.
//!
//! Luật (đọc từ dòng `// expect:` đầu tệp, hoặc tệp `expect` của ca nhiều tệp — `tests/README.md`):
//! * `pass`     ⇒ phán quyết XANH (không lỗi, không cảnh báo);
//! * `fail Exxx` ⇒ phán quyết ĐỎ và tập mã mức "error" là ĐÚNG `{Exxx}` (không thiếu, không thừa mã khác);
//! * `warn Sxxx` ⇒ phán quyết VÀNG (cổng vẫn mở, mã thoát 0) và có cảnh báo `Sxxx`, không có lỗi.
//! Tệp đơn được chép thành `main.gt` (tên tệp = tên module, mẫu dùng `main::…`); ca nhiều tệp được chạy nguyên thư mục.
//!
//! Thay cho chín bộ `corpus.rs` theo crate (mỗi bộ chỉ chạy một phần pipeline qua API riêng). Mẫu mới chỉ cần thêm tệp `.gt`, không viết Rust.
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Mẫu mà chương trình thật sự kích hoạt THÊM một lỗi đúng của pass khác (không phải lỗi của gritc). Bảng phải NGẮN: thêm mục = phải nêu lý do.
/// Cách sửa gốc là viết lại mẫu cho tối thiểu; chưa làm vì golden `tests/golden/borrowck/diagnostics.txt` đang ghim vị trí của chúng.
const KNOWN_EXTRA: &[(&str, &[&str], &str)] = &[
    (
        "E0401_move_in_while.gt",
        &["E0701"],
        "vòng `⟳ c` không có biến thể ↓ nên gt-term đúng khi báo thêm E0701",
    ),
    (
        "E1001_raw_ptr_return.gt",
        &["E0703"],
        "`f` đệ quy không có ↓ nên gt-term đúng khi báo thêm E0703",
    ),
];

struct Case {
    kind: &'static str,
    name: String,
    path: PathBuf,
    expect: String,
}

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests")
}

fn expect_of(path: &Path) -> Option<String> {
    let (file, prefix) = if path.is_dir() {
        (path.join("expect"), "expect:")
    } else {
        (path.to_path_buf(), "// expect:")
    };
    std::fs::read_to_string(file)
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix(prefix).map(|r| r.trim().to_string()))
}

fn cases() -> Vec<Case> {
    let mut v = Vec::new();
    for kind in ["pass", "fail", "warn"] {
        let mut ents: Vec<PathBuf> = std::fs::read_dir(tests_dir().join(kind))
            .expect("đọc thư mục corpus")
            .map(|e| e.expect("entry").path())
            .filter(|p| p.is_dir() || p.extension().is_some_and(|x| x == "gt"))
            .collect();
        ents.sort();
        for path in ents {
            let name = path.file_name().expect("tên").to_string_lossy().into_owned();
            let expect =
                expect_of(&path).unwrap_or_else(|| panic!("{kind}/{name}: thiếu dòng expect"));
            v.push(Case {
                kind,
                name,
                path,
                expect,
            });
        }
    }
    v
}

struct Outcome {
    verdict: String,
    exit: i32,
    errors: BTreeSet<String>,
    warns: BTreeSet<String>,
}

fn run_gritc(target: &Path) -> Result<Outcome, String> {
    let out = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .arg("check")
        .arg(target)
        .arg("--json")
        .output()
        .map_err(|e| format!("không chạy được gritc: {e}"))?;
    let j: Value = serde_json::from_slice(&out.stdout).map_err(|e| {
        format!(
            "stdout không phải JSON ({e}); mã thoát {:?}; stderr: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        )
    })?;
    let mut errors = BTreeSet::new();
    let mut warns = BTreeSet::new();
    for d in j["diagnostics"].as_array().ok_or("thiếu diagnostics")? {
        let code = d["code"].as_str().ok_or("thiếu code")?.to_string();
        match d["level"].as_str() {
            Some("error") => errors.insert(code),
            Some("warn") => warns.insert(code),
            _ => false,
        };
    }
    Ok(Outcome {
        verdict: j["verdict"].as_str().ok_or("thiếu verdict")?.to_string(),
        exit: out.status.code().ok_or("không có mã thoát")?,
        errors,
        warns,
    })
}

fn judge(c: &Case, o: &Outcome) -> Result<(), String> {
    let mut words = c.expect.split_whitespace();
    let (kind, code) = (words.next().unwrap_or(""), words.next());
    if kind != c.kind {
        return Err(format!("thư mục {} nhưng expect là `{}`", c.kind, c.expect));
    }
    match (kind, code) {
        ("pass", None) => {
            if o.verdict != "green" || o.exit != 0 {
                return Err(format!(
                    "kỳ vọng XANH, phán quyết {} (mã thoát {}), lỗi {:?}, cảnh báo {:?}",
                    o.verdict, o.exit, o.errors, o.warns
                ));
            }
        }
        ("fail", Some(code)) => {
            let mut want: BTreeSet<String> = BTreeSet::from([code.to_string()]);
            if let Some((_, extra, _)) = KNOWN_EXTRA.iter().find(|(n, _, _)| *n == c.name) {
                want.extend(extra.iter().map(|s| s.to_string()));
            }
            if o.verdict != "red" || o.exit != 1 || o.errors != want {
                return Err(format!(
                    "kỳ vọng ĐỎ với đúng {want:?}, nhận phán quyết {} (mã thoát {}), lỗi {:?}",
                    o.verdict, o.exit, o.errors
                ));
            }
        }
        ("warn", Some(code)) => {
            if o.verdict != "yellow" || o.exit != 0 || !o.errors.is_empty() || !o.warns.contains(code)
            {
                return Err(format!(
                    "kỳ vọng VÀNG có {code}, nhận phán quyết {} (mã thoát {}), lỗi {:?}, cảnh báo {:?}",
                    o.verdict, o.exit, o.errors, o.warns
                ));
            }
        }
        _ => return Err(format!("dòng expect lạ: `{}`", c.expect)),
    }
    Ok(())
}

#[test]
fn every_corpus_sample_gets_exactly_its_expected_verdict_from_the_real_gritc() {
    let cases = cases();
    let count = |k: &str| cases.iter().filter(|c| c.kind == k).count();
    // chống "glob rỗng": corpus co lại một cách im lặng ⇒ ĐỎ (số liệu đo 2026-11-21; giảm có chủ ý thì sửa số ở đây cùng commit)
    assert!(count("pass") >= 430, "chỉ {} mẫu pass", count("pass"));
    assert!(count("fail") >= 530, "chỉ {} mẫu fail", count("fail"));
    assert!(count("warn") >= 12, "chỉ {} mẫu warn", count("warn"));

    let scratch_root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("corpus_e2e");
    let next = AtomicUsize::new(0);
    let bad = Mutex::new(Vec::<String>::new());
    std::thread::scope(|s| {
        for w in 0..2 {
            let (cases, next, bad) = (&cases, &next, &bad);
            let scratch = scratch_root.join(format!("w{w}"));
            s.spawn(move || {
                std::fs::create_dir_all(&scratch).expect("tạo thư mục tạm");
                loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    let Some(c) = cases.get(i) else { break };
                    let target = if c.path.is_dir() {
                        c.path.clone()
                    } else {
                        let text = std::fs::read_to_string(&c.path).expect("đọc mẫu");
                        std::fs::write(scratch.join("main.gt"), text.replace("\r\n", "\n"))
                            .expect("ghi main.gt");
                        scratch.clone()
                    };
                    let r = run_gritc(&target).and_then(|o| judge(c, &o));
                    if let Err(e) = r {
                        bad.lock().expect("khoá").push(format!("{}/{}: {e}", c.kind, c.name));
                    }
                }
            });
        }
    });
    let mut bad = bad.into_inner().expect("khoá");
    bad.sort();
    assert!(
        bad.is_empty(),
        "{} mẫu sai kỳ vọng:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
