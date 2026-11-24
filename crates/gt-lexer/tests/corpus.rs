//! Chạy lexer trên toàn bộ corpus `tests/` của repo và đối chiếu kỳ vọng (header `// expect:` + `.tokens`).
use gt_lexer::{ErrorCode, lex, render_tokens};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests")
}

fn gt_files(sub: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(tests_dir().join(sub))
        .unwrap_or_else(|e| panic!("không đọc được tests/{sub}: {e}"))
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .collect();
    v.sort();
    assert!(!v.is_empty(), "tests/{sub} không có file .gt");
    v
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn name(p: &Path) -> String {
    p.file_name()
        .expect("tên file")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn lexer_corpus_tokens_match_expected() {
    for gt in gt_files("lexer") {
        let src = read(&gt);
        let out = lex(&src);
        assert!(
            out.is_clean(),
            "{}: lexer báo lỗi {:?}",
            name(&gt),
            out.errors
        );
        let expected: Vec<String> = read(&gt.with_extension("tokens"))
            .lines()
            .map(str::trim_end)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(str::to_string)
            .collect();
        assert_eq!(render_tokens(&out), expected, "{}", name(&gt));
    }
}

#[test]
fn pass_and_warn_corpus_lex_clean() {
    for sub in ["pass", "warn"] {
        for gt in gt_files(sub) {
            let out = lex(&read(&gt));
            assert!(
                out.is_clean(),
                "tests/{sub}/{}: lexer báo lỗi {:?}",
                name(&gt),
                out.errors
            );
        }
    }
}

/// Các mẫu `fail` mà lỗi nằm ở tầng lexer (E0101 mức token). Mọi E0101 khác là lỗi của parser (M2).
const LEXICAL_E0101: &[&str] = &[
    "E0101_block_comment.gt",
    "E0101_char_multi.gt",
    "E0101_compound_assign.gt",
    "E0101_invalid_char_escape.gt",
    "E0101_invalid_string_escape.gt",
    "E0101_question_mark_needs_edition_2.gt", // edition 1: `?` đứng riêng là ký tự ngoài ngôn ngữ (ADR-P2-09)
    "E0101_or_pattern_needs_edition_2.gt", // edition 1: `|` đứng riêng là ký tự ngoài ngôn ngữ (ADR-P2-11)
];

fn code_of(header: &str) -> Option<ErrorCode> {
    match header.split_whitespace().nth(3)? {
        "E0101" => Some(ErrorCode::E0101),
        "E1201" => Some(ErrorCode::E1201),
        "E1202" => Some(ErrorCode::E1202),
        "E1203" => Some(ErrorCode::E1203),
        "E1204" => Some(ErrorCode::E1204),
        _ => None, // mã của tầng sau
    }
}

#[test]
fn fail_corpus_lexer_verdict() {
    for gt in gt_files("fail") {
        let src = read(&gt);
        let header = src.lines().next().unwrap_or_default().to_string();
        let out = lex(&src);
        let got: BTreeSet<ErrorCode> = out.errors.iter().map(|e| e.code).collect();
        let f = name(&gt);
        match code_of(&header) {
            Some(code) if code != ErrorCode::E0101 => {
                assert_eq!(
                    got,
                    BTreeSet::from([code]),
                    "{f}: phải bị chặn đúng mã {}",
                    code.as_str()
                );
            }
            Some(_) if LEXICAL_E0101.contains(&f.as_str()) => {
                assert_eq!(got, BTreeSet::from([ErrorCode::E0101]), "{f}");
            }
            _ => assert!(
                got.is_empty(),
                "{f}: lỗi này thuộc tầng sau nhưng lexer đã báo {:?}",
                out.errors
            ),
        }
    }
}
