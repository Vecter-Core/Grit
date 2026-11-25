//! E2E (quyết định Giang 2026-11-21): mỗi tệp `tests/glir/{pass,fail}/*.glir` có dòng `// expect: pass` hoặc `// expect: fail <MÃ>`;
//! `fail` đòi ĐÚNG mã đó xuất hiện (parse lỗi ⇒ `G0001`). Mọi tệp đọc được phải in-đọc-in ổn định, và `pass` phải qua verifier.
use std::path::Path;

fn files(sub: &str) -> Vec<std::path::PathBuf> {
    let d = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/glir")
        .join(sub);
    let mut v: Vec<_> = std::fs::read_dir(d)
        .expect("thư mục")
        .map(|e| e.expect("mục").path())
        .filter(|p| p.extension().is_some_and(|x| x == "glir"))
        .collect();
    v.sort();
    v
}

fn expectation(src: &str) -> String {
    src.lines()
        .find_map(|l| l.strip_prefix("// expect: "))
        .expect("thiếu `// expect:`")
        .trim()
        .to_string()
}

fn run(src: &str) -> Vec<String> {
    match gt_glir::parse(src) {
        Err(e) => vec![format!("G0001: {e}")],
        Ok(m) => gt_glir::verify(&m),
    }
}

#[test]
fn every_glir_sample_gets_exactly_its_expected_verdict() {
    let (mut np, mut nf) = (0, 0);
    for sub in ["pass", "fail"] {
        for p in files(sub) {
            let src = std::fs::read_to_string(&p).expect("đọc");
            let want = expectation(&src);
            let errs = run(&src);
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            if let Some(code) = want.strip_prefix("fail ") {
                nf += 1;
                assert_eq!(sub, "fail", "{name}");
                assert!(
                    errs.iter().any(|e| e.starts_with(code)),
                    "{name}: kỳ vọng {code}, có {errs:?}"
                );
            } else {
                np += 1;
                assert_eq!(want, "pass", "{name}");
                assert_eq!(sub, "pass", "{name}");
                assert!(errs.is_empty(), "{name}: kỳ vọng hợp lệ, có {errs:?}");
            }
        }
    }
    assert!(np >= 3 && nf >= 15, "corpus quá nhỏ: pass={np} fail={nf}");
}

#[test]
fn print_parse_print_is_a_fixed_point_on_every_readable_sample() {
    for sub in ["pass", "fail"] {
        for p in files(sub) {
            let src = std::fs::read_to_string(&p).expect("đọc");
            let Ok(m) = gt_glir::parse(&src) else {
                continue;
            };
            let once = gt_glir::print(&m);
            let m2 = gt_glir::parse(&once).unwrap_or_else(|e| {
                panic!("{}: in ra không đọc lại được: {e}\n{once}", p.display())
            });
            assert_eq!(m, m2, "{}: đọc(in(m)) ≠ m", p.display());
            assert_eq!(gt_glir::print(&m2), once, "{}", p.display());
        }
    }
}

#[test]
fn the_parser_never_panics_on_truncated_or_mangled_input() {
    let src = std::fs::read_to_string(files("pass")[0].clone()).expect("đọc");
    let chars: Vec<char> = src.chars().collect();
    for cut in 0..chars.len() {
        let t: String = chars[..cut].iter().collect();
        let _ = run(&t);
        let mut m = chars.clone();
        m[cut] = ['%', '{', ']', ',', '\n', '9'][cut % 6];
        let _ = run(&m.iter().collect::<String>());
    }
}
