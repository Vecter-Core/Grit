#![forbid(unsafe_code)]
//! gt-parser (M2): recursive descent + Pratt, khôi phục lỗi ở `;` / `}`, giới hạn độ sâu `gt_ast::MAX_DEPTH`.
//!
//! * Không có node lỗi trong AST: câu lệnh/item hỏng bị bỏ, các phần hợp lệ vẫn được giữ (AST chỉ dùng khi không có ĐỎ).
//! * Mọi lỗi lexer được chuyển thành `Diagnostic` ở đây (`convert::lex_error`).
//! * Luôn tiến về phía trước: mỗi vòng khôi phục tiêu thụ ít nhất một token.
//! * Chạy qua `gt_stack::run_with_big_stack`: panic nội bộ → ICE `E9901` (semantics §26).
mod convert;
mod exprs;
mod parser;
mod pats;
mod types;

use gt_ast::Program;
use gt_diag::{Diagnostics, Report};
use gt_lexer::Trivia;

/// Số chẩn đoán tối đa giữ lại (D19); phần dư được đếm trong `Report::truncated`.
pub const DIAGNOSTIC_LIMIT: usize = 100;

/// Kết quả phân tích một tệp.
#[derive(Debug)]
pub struct ParseOutput {
    pub program: Program,
    /// Chẩn đoán của lexer + parser: đã sắp xếp tất định, bỏ trùng, cắt theo giới hạn.
    pub report: Report,
    /// Comment `//` (không nằm trong AST; `gt-fmt` gắn lại — ADR-012).
    pub trivia: Vec<Trivia>,
    /// Số node đã cấp NodeId (NodeId dày đặc `0..node_count`).
    pub node_count: u32,
    /// Số bước công việc của parser (kiểm chứng tính tuyến tính / luôn tiến lên).
    pub work: usize,
    /// Độ sâu lớn nhất đã chạm (`<= gt_ast::MAX_DEPTH` khi không có E0102) — dùng để kiểm chứng luật độ sâu.
    pub max_depth: u32,
}

/// Phân tích `source` (tên tệp `file` chỉ dùng cho chẩn đoán).
///
/// Chạy trong luồng stack lớn của `gt-stack`; panic nội bộ thành ICE `E9901` (không làm sập tiến trình; ĐỎ/ICE ⇒ không chạy).
#[must_use]
pub fn parse(file: &str, source: &str) -> ParseOutput {
    parse_file(gt_span::FileId::new(0), file, source)
}

/// Như `parse`, nhưng mọi `NodeId` mang `file_id` (chương trình nhiều tệp — ADR-017a). `FileId` do `gt_span::SourceMap` cấp.
#[must_use]
pub fn parse_file(file_id: gt_span::FileId, file: &str, source: &str) -> ParseOutput {
    parse_file_with_edition(file_id, file, source, 1)
}

/// Như `parse_file`, theo `edition` (ADR-P2-01/09): edition ≥ 2 nhận toán tử hậu tố `?`; edition 1 giữ hành vi đóng băng.
#[must_use]
pub fn parse_file_with_edition(
    file_id: gt_span::FileId,
    file: &str,
    source: &str,
    edition: u32,
) -> ParseOutput {
    match gt_stack::run_with_big_stack(|| parse_inner(file_id, file, source, edition)) {
        Ok(out) => out,
        Err(e) => ice(file_id, file, source, &e.to_string()),
    }
}

/// Kết quả khi parser gặp lỗi nội bộ: chương trình rỗng + ICE `E9901`.
fn ice(file_id: gt_span::FileId, file: &str, source: &str, msg: &str) -> ParseOutput {
    use gt_ast::{NodeId, NodeIdGen};
    use gt_diag::{Code, Diagnostic};
    use gt_span::Span;
    let mut ids = NodeIdGen::new(file_id);
    let id = ids.next_id().unwrap_or_else(|| NodeId::new(file_id, 0));
    let len = u32::try_from(source.len()).unwrap_or(u32::MAX);
    let mut bag = Diagnostics::new(DIAGNOSTIC_LIMIT);
    if let Ok(code) = Code::parse("E9901") {
        bag.push(Diagnostic::new(
            code,
            file,
            Span::new(0, 0),
            format!("lỗi nội bộ của compiler (ICE): {msg}"),
        ));
    }
    ParseOutput {
        program: Program {
            id,
            span: Span::new(0, len),
            items: vec![],
        },
        report: bag.finish(),
        trivia: vec![],
        node_count: 1,
        work: 0,
        max_depth: 0,
    }
}

fn parse_inner(file_id: gt_span::FileId, file: &str, source: &str, edition: u32) -> ParseOutput {
    let lexed = gt_lexer::lex_with_edition(source, edition);
    let mut p = parser::Parser::new(file_id, file, source.len(), lexed.tokens);
    p.edition = edition;
    for e in &lexed.errors {
        if let Some(d) = convert::lex_error(file, e) {
            p.diags.push(d);
        }
    }
    let program = p.parse_program();
    let mut bag = Diagnostics::new(DIAGNOSTIC_LIMIT);
    for d in std::mem::take(&mut p.diags) {
        bag.push(d);
    }
    ParseOutput {
        program,
        report: bag.finish(),
        trivia: lexed.trivia,
        node_count: p.ids.count(),
        work: p.work,
        max_depth: p.max_depth,
    }
}
