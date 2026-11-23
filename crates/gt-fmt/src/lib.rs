#![forbid(unsafe_code)]
//! gt-fmt (M2, thư viện; chưa có lệnh `gritc fmt` — gt-driver ở M9 mới nối) — MỘT dạng chuẩn duy nhất (canonical Unicode), in lại từ AST.
//!
//! * AST không có ngoặc nhóm nên printer tự thêm ngoặc theo độ ưu tiên và theo luật `expr_ns`.
//! * Comment `//` nằm ngoài AST (bảng trivia của lexer); `attach` đặt lại theo TOKEN gần nhất (ADR-016, Q16b):
//!   dòng riêng → token kế tiếp, cuối dòng → token liền trước. E1301 chỉ khi bước xác minh thất bại (phương án cuối).
//!   **Không bao giờ xoá hay dời comment sang token khác.**
//! * Chạy qua `gt_stack::run_with_big_stack` (printer đệ quy trên AST).
mod attach;
mod printer;

use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_span::Span;

/// gt-fmt từ chối định dạng: lỗi cú pháp (chẩn đoán của parser) hoặc comment không gắn được (E1301), hoặc ICE.
#[derive(Debug)]
pub struct FmtError {
    pub report: Report,
}

impl FmtError {
    /// Mã các chẩn đoán (theo thứ tự đã sắp xếp).
    #[must_use]
    pub fn codes(&self) -> Vec<String> {
        self.report
            .diagnostics
            .iter()
            .map(|d| d.code.as_str().to_string())
            .collect()
    }
}

const LIMIT: usize = 100;

/// Định dạng `source`; `file` chỉ dùng cho chẩn đoán.
pub fn format(file: &str, source: &str) -> Result<String, FmtError> {
    format_with_edition(file, source, 1)
}

/// Như `format`, theo `edition` (ADR-P2-01/09): edition ≥ 2 nhận `e?`; mọi bước lex/phân tích/xác minh dùng CÙNG edition.
pub fn format_with_edition(file: &str, source: &str, edition: u32) -> Result<String, FmtError> {
    match gt_stack::run_with_big_stack(|| format_inner(file, source, edition)) {
        Ok(r) => r,
        Err(e) => Err(ice(file, &e.to_string())),
    }
}

fn ice(file: &str, msg: &str) -> FmtError {
    let mut bag = Diagnostics::new(LIMIT);
    if let Ok(code) = Code::parse("E9901") {
        bag.push(Diagnostic::new(
            code,
            file,
            Span::new(0, 0),
            format!("lỗi nội bộ của compiler (ICE): {msg}"),
        ));
    }
    FmtError {
        report: bag.finish(),
    }
}

fn format_inner(file: &str, source: &str, edition: u32) -> Result<String, FmtError> {
    let parsed = gt_parser::parse_file_with_edition(gt_span::FileId::new(0), file, source, edition);
    if !parsed.report.diagnostics.is_empty() {
        return Err(FmtError {
            report: parsed.report,
        });
    }
    let starts = parsed.trivia.iter().map(|t| t.span.start).collect();
    let base = printer::Fmt::new(starts).program(&parsed.program);
    match attach::place(source, &base, edition) {
        Ok(out) => Ok(out),
        Err(attach::PlaceError::Align(m)) => {
            Err(ice(file, &format!("gt-fmt: căn token thất bại ({m})")))
        }
        Err(attach::PlaceError::Verify) => {
            let span = parsed.trivia.first().map_or(Span::new(0, 0), |t| t.span);
            let mut bag = Diagnostics::new(LIMIT);
            if let Ok(code) = Code::parse("E1301") {
                bag.push(
                    Diagnostic::new(
                        code,
                        file,
                        span,
                        "gt-fmt: không đặt được comment mà vẫn giữ nguyên chương trình và dãy comment".to_string(),
                    )
                    .with_span_label("comment đầu tiên của tệp")
                    .with_note("đây là phương án cuối cùng của cơ chế đặt comment theo token; hãy báo lỗi cùng tệp nguồn"),
                );
            }
            Err(FmtError {
                report: bag.finish(),
            })
        }
    }
}
