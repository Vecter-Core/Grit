#![forbid(unsafe_code)]
//! gt-fmt (M2, thư viện; chưa có lệnh `gritc fmt` — gt-driver ở M9 mới nối) — MỘT dạng chuẩn duy nhất (canonical Unicode), in lại từ AST.
//!
//! * AST không có ngoặc nhóm nên printer tự thêm ngoặc theo độ ưu tiên và theo luật `expr_ns`.
//! * Comment `//` nằm ngoài AST (bảng trivia của lexer); gt-fmt gắn lại theo quy tắc ở `semantics.md` §25.
//!   **Không gắn được thì TỪ CHỐI (E1301); không bao giờ xoá hay dời comment.**
//! * Chạy qua `gt_stack::run_with_big_stack` (printer đệ quy trên AST).
mod comments;
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
    match gt_stack::run_with_big_stack(|| format_inner(file, source)) {
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

fn format_inner(file: &str, source: &str) -> Result<String, FmtError> {
    let parsed = gt_parser::parse(file, source);
    if !parsed.report.diagnostics.is_empty() {
        return Err(FmtError {
            report: parsed.report,
        });
    }
    let mut f = printer::Fmt::new(file, source, parsed.trivia);
    let out = f.program(&parsed.program);
    let diags = f.finish();
    if diags.is_empty() {
        Ok(out)
    } else {
        let mut bag = Diagnostics::new(LIMIT);
        for d in diags {
            bag.push(d);
        }
        Err(FmtError {
            report: bag.finish(),
        })
    }
}
