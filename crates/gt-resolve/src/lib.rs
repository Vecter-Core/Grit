#![forbid(unsafe_code)]
//! gt-resolve (M3): giải tên và module — ADR-017.
//!
//! * Chương trình = tập tệp; mỗi tệp là một module tên = tên tệp bỏ `.gt`; `▦` lồng bên trong; thứ tự tệp/khai báo không có ý nghĩa.
//! * `⇲` (kể cả dạng nhóm), khả kiến `◈`, trùng (E0202), thiếu (E0201), private (E0203), vòng module (E0204),
//!   `Self`/`self`, và D7 cấm shadowing (E0205).
//! * Prelude là đầu vào tường minh (`Prelude`), không có tên nào viết cứng.
//! * Chỉ chạy khi mọi tệp parse sạch (fail-closed: AST không có node lỗi). Chạy trong luồng stack lớn (ADR-015).
mod graph;
mod model;
mod prelude;
mod tables;
mod walk;

pub use model::{Def, DefId, DefKind, LocalId, Res, ResolveOutput};
pub use prelude::{Prelude, PreludeError};

use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_lexer::TokenKind;
use gt_span::{SourceFile, SourceMap, SourceMapError, Span};
use std::collections::BTreeMap;
use tables::{LIMIT, Tables};

/// Một tệp nguồn của chương trình. `name` phải có dạng `<ident>.gt`; `<ident>` là tên module.
#[derive(Clone, Debug)]
pub struct SourceInput {
    pub name: String,
    pub text: String,
}

/// Đầu vào không hợp lệ (lỗi của bên gọi/driver, không phải lỗi trong văn bản chương trình).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputError {
    DuplicateFileName(String),
    /// Tên tệp không có dạng `<ident>.gt` hợp lệ.
    BadModuleName(String),
    /// Tệp dài hơn `u32::MAX` byte.
    TooLarge(String),
}

/// Tên module từ tên tệp: bỏ `.gt`, phải là đúng một định danh hợp lệ (không phải từ khoá).
fn module_name(file: &str) -> Result<String, InputError> {
    let bad = || InputError::BadModuleName(file.to_string());
    let stem = file.strip_suffix(".gt").ok_or_else(bad)?;
    let lexed = gt_lexer::lex(stem);
    // token lỗi (UNKNOWN) không bao giờ khớp mẫu một `Ident` nên không cần kiểm riêng `lexed.errors`
    let ok = matches!(lexed.tokens.as_slice(), [a, b] if matches!(&a.kind, TokenKind::Ident(s) if s == stem) && b.kind == TokenKind::Eof);
    if ok { Ok(stem.to_string()) } else { Err(bad()) }
}

/// Giải tên cho cả chương trình. Thứ tự `inputs` không ảnh hưởng kết quả.
pub fn resolve_program(
    inputs: &[SourceInput],
    prelude: &Prelude,
) -> Result<ResolveOutput, InputError> {
    let mut files = Vec::new();
    for i in inputs {
        module_name(&i.name)?;
        let f = SourceFile::new(i.name.clone(), i.text.clone())
            .map_err(|_| InputError::TooLarge(i.name.clone()))?;
        files.push(f);
    }
    let map = SourceMap::new(files)
        .map_err(|SourceMapError::DuplicateName(n)| InputError::DuplicateFileName(n))?;
    // FileId theo tên đã sắp: tên module cũng theo thứ tự đó
    let mut mods = Vec::new();
    for (_, f) in map.iter() {
        mods.push(module_name(f.name())?);
    }
    Ok(
        match gt_stack::run_with_big_stack(|| resolve_inner(&map, &mods, prelude)) {
            Ok(o) => o,
            Err(e) => ice(&e.to_string()),
        },
    )
}

fn ice(msg: &str) -> ResolveOutput {
    let mut bag = Diagnostics::new(LIMIT);
    if let Ok(code) = Code::parse("E9901") {
        bag.push(Diagnostic::new(
            code,
            "<resolver>",
            Span::new(0, 0),
            format!("lỗi nội bộ của compiler (ICE): {msg}"),
        ));
    }
    ResolveOutput {
        report: bag.finish(),
        defs: Vec::new(),
        uses: BTreeMap::new(),
        decls: BTreeMap::new(),
        programs: Vec::new(),
        module_names: Vec::new(),
        prelude_program: None,
        canonical: Vec::new(),
        logical: Vec::new(),
    }
}

fn syntax_failure(report: Report) -> ResolveOutput {
    ResolveOutput {
        report,
        defs: Vec::new(),
        uses: BTreeMap::new(),
        decls: BTreeMap::new(),
        programs: Vec::new(),
        module_names: Vec::new(),
        prelude_program: None,
        canonical: Vec::new(),
        logical: Vec::new(),
    }
}

fn resolve_inner(map: &SourceMap, mods: &[String], prelude: &Prelude) -> ResolveOutput {
    // 1. parse từng tệp; có lỗi cú pháp thì dừng (không phân giải trên AST không hoàn chỉnh)
    let mut parsed = Vec::new();
    let mut syntax = Diagnostics::new(LIMIT);
    let mut any_syntax = false;
    for (id, f) in map.iter() {
        let out = gt_parser::parse_file(id, f.name(), f.text());
        for d in out.report.diagnostics {
            any_syntax = true;
            syntax.push(d);
        }
        parsed.push(out.program);
    }
    if any_syntax {
        return syntax_failure(syntax.finish());
    }

    // 2. thu thập, gán DefId theo đường dẫn, trùng tên, nhập
    let file_names: Vec<String> = map.iter().map(|(_, f)| f.name().to_string()).collect();
    let texts: Vec<String> = map.iter().map(|(_, f)| f.text().to_string()).collect();
    let mut t = Tables::new(file_names, texts);
    t.collect_prelude(prelude);
    for ((id, _), (m, p)) in map.iter().zip(mods.iter().zip(&parsed)) {
        t.collect_file(id, m, p);
    }
    t.finalize_order();
    t.report_duplicates();
    t.resolve_all_uses();

    // 3. duyệt thân/chữ ký
    for ((id, _), p) in map.iter().zip(&parsed) {
        if let Some(&m) = t.item_def.get(&p.id) {
            walk::walk_items(&mut t, &p.items, m, id);
        }
    }

    // prelude: cùng luật giải tên (chữ ký/kiểu của nó phải phân giải để pass sau dùng được)
    {
        let pm = t.prelude;
        walk::walk_items(&mut t, &prelude.program.items, pm, prelude::prelude_file());
    }

    // 4. vòng phụ thuộc module
    graph::report_cycles(&mut t);

    // 5. kết quả
    let mut bag = Diagnostics::new(LIMIT);
    let mut logical = Vec::new();
    for l in t.diags.drain(..) {
        logical.push(format!("{} | {} | {}", l.diag.code, l.diag.message, l.site));
        bag.push(l.diag);
    }
    logical.sort();
    let mut canonical: Vec<String> = t
        .defs
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != t.root.index() && *i != t.prelude.index())
        .map(|(_, d)| {
            format!(
                "def {} {} {}",
                d.path,
                d.kind.as_str(),
                if d.public { "pub" } else { "priv" }
            )
        })
        .collect();
    canonical.append(&mut t.canon);
    canonical.sort();
    ResolveOutput {
        report: bag.finish(),
        defs: std::mem::take(&mut t.defs),
        uses: std::mem::take(&mut t.res),
        decls: std::mem::take(&mut t.decls),
        programs: parsed,
        module_names: mods.to_vec(),
        prelude_program: Some(prelude.program.clone()),
        canonical,
        logical,
    }
}
