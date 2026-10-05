#![forbid(unsafe_code)]
//! gt-diag (M2): diagnostics engine — mã lỗi, mức ĐỎ/VÀNG/GHI CHÚ, edit có cấu trúc,
//! xuất dạng người đọc và JSON có version (docs/spec/diag_schema.json).
mod code;
mod diag;
mod edit;
mod json;
mod render;
mod report;

pub use code::{Code, CodeError, Level, known_codes};
pub use diag::{Applicability, Diagnostic, Edit, Label, Suggestion};
pub use edit::{EditError, apply_edits};
pub use json::{SCHEMA_VERSION, diagnostic_to_json};
pub use render::{RenderOptions, render_human, render_human_files};
pub use report::{Diagnostics, Report};
