//! Prelude là ĐẦU VÀO TƯỜNG MINH của resolver (ADR-017c): resolver không có tên nào viết cứng.
use gt_ast::Program;
use gt_diag::Report;
use gt_span::FileId;

/// Tên tệp giả của prelude trong chẩn đoán/nhãn.
pub(crate) const PRELUDE_FILE_NAME: &str = "<prelude>";

/// `FileId` dành riêng cho prelude (không bao giờ trùng `FileId` của tệp chương trình).
pub(crate) fn prelude_file() -> FileId {
    FileId::new(u32::MAX)
}

/// Prelude dựng từ mã nguồn Grit: item cấp cao thấy được bằng tên trần ở mọi module (bất kể `◈`); biến thể của mọi `enum` cấp cao
/// cũng thấy được bằng tên trần; module con (vd `core`) truy cập qua đường dẫn và thành viên cần `◈`.
/// M12 thay bằng `lib/core` thật qua đúng đường vào này.
#[derive(Debug)]
pub struct Prelude {
    pub(crate) program: Program,
}

/// Prelude không hợp lệ: lỗi cú pháp hoặc tên trùng.
#[derive(Debug)]
pub struct PreludeError {
    pub report: Report,
}

impl Prelude {
    /// Prelude rỗng (không tên nào ngoài các module-tệp).
    #[must_use]
    pub fn empty() -> Prelude {
        let out = gt_parser::parse_file(prelude_file(), PRELUDE_FILE_NAME, "");
        Prelude {
            program: out.program,
        }
    }

    /// Dựng từ mã nguồn; lỗi nếu có chẩn đoán cú pháp hoặc tên trùng trong prelude.
    pub fn from_source(src: &str) -> Result<Prelude, PreludeError> {
        let out = gt_parser::parse_file(prelude_file(), PRELUDE_FILE_NAME, src);
        if !out.report.diagnostics.is_empty() {
            return Err(PreludeError { report: out.report });
        }
        let prelude = Prelude {
            program: out.program,
        };
        let dups = crate::tables::prelude_duplicates(&prelude);
        if dups.diagnostics.is_empty() {
            Ok(prelude)
        } else {
            Err(PreludeError { report: dups })
        }
    }
}
