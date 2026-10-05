//! Kiểu dữ liệu công khai của kết quả phân giải (ADR-017).
use gt_ast::NodeId;
use gt_diag::Report;
use gt_span::{FileId, Span};
use std::collections::BTreeMap;

/// Định danh một định nghĩa (module, hàm, struct, enum, trait, alias, const, static, biến thể). Được cấp theo THỨ TỰ ĐƯỜNG DẪN
/// ĐỦ TÊN (không theo thứ tự khai báo) nên độc lập thứ tự khai báo và thứ tự tệp.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DefId(u32);

impl DefId {
    pub(crate) fn from_index(i: usize) -> DefId {
        DefId(u32::try_from(i).unwrap_or(u32::MAX))
    }

    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Biến cục bộ: `owner` là NodeId của hàm/hằng chứa nó, `index` là thứ tự khai báo trong thân (tham số trước).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalId {
    pub owner: NodeId,
    pub index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DefKind {
    Module,
    Fn,
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Const,
    Static,
    Variant,
}

impl DefKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            DefKind::Module => "module",
            DefKind::Fn => "fn",
            DefKind::Struct => "struct",
            DefKind::Enum => "enum",
            DefKind::Trait => "trait",
            DefKind::TypeAlias => "alias",
            DefKind::Const => "const",
            DefKind::Static => "static",
            DefKind::Variant => "variant",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Def {
    /// Đường dẫn đủ tên: `tệp::module::tên`; prelude có tiền tố `$prelude::`; gốc ẩn danh là chuỗi rỗng.
    pub path: String,
    pub kind: DefKind,
    pub file: FileId,
    pub name_span: Span,
    /// NodeId của item (hoặc biến thể).
    pub node: NodeId,
    /// `◈` (hoặc thành viên cấp cao của prelude / biến thể).
    pub public: bool,
    /// Module chứa (hoặc enum chứa, với biến thể). `None` cho gốc và prelude.
    pub parent: Option<DefId>,
    /// Biến thể có trường (`Some(T)`)? Chỉ có nghĩa với `DefKind::Variant`.
    pub has_fields: bool,
}

/// Kết quả phân giải của MỘT vị trí dùng tên.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Res {
    Def(DefId),
    Local(LocalId),
    /// Tham số generics: (NodeId item chứa, thứ tự).
    Generic(NodeId, u32),
    SelfType,
    SelfValue,
    /// `Kiểu::tên` với `tên` không phải biến thể: mục kết hợp (impl/trait) — M4 kiểm. `base` là phần đã phân giải.
    Assoc {
        base: Box<Res>,
        member: String,
    },
    /// Đã báo lỗi (hoặc cố ý im lặng vì đã có lỗi gốc) — không sinh lỗi dây chuyền.
    Err,
}

/// Kết quả toàn bộ giai đoạn giải tên.
#[derive(Debug)]
pub struct ResolveOutput {
    /// Chẩn đoán (D19: sắp theo tệp, vị trí, mã, thông điệp), gồm chẩn đoán cú pháp nếu có tệp lỗi cú pháp.
    pub report: Report,
    /// Mọi định nghĩa, theo thứ tự `DefId`. Rỗng nếu có lỗi cú pháp (không phân giải).
    pub defs: Vec<Def>,
    /// Mỗi vị trí dùng đường dẫn (NodeId của `Path`, hoặc của `Ident` cho pattern chỉ có ident là biến thể) → kết quả.
    pub uses: BTreeMap<NodeId, Res>,
    /// Mỗi chỗ khai báo biến cục bộ (NodeId của `Ident`) → LocalId.
    pub decls: BTreeMap<NodeId, LocalId>,
    pub(crate) canonical: Vec<String>,
    pub(crate) logical: Vec<String>,
}

impl ResolveOutput {
    /// Dạng chuẩn ĐỘC LẬP thứ tự khai báo/thứ tự tệp: một dòng cho mỗi định nghĩa, nhập, vị trí dùng và khai báo cục bộ,
    /// khoá bằng đường dẫn item bao + số thứ tự trong item (không dùng NodeId/offset). Đã sắp.
    #[must_use]
    pub fn canonical(&self) -> String {
        self.canonical.join("\n")
    }

    /// Chẩn đoán dạng logic `mã | thông điệp | vị trí logic (item bao)` — không có offset — đã sắp. Dùng để so sánh bất biến thứ tự.
    #[must_use]
    pub fn canonical_diagnostics(&self) -> Vec<String> {
        self.logical.clone()
    }

    #[must_use]
    pub fn def(&self, id: DefId) -> Option<&Def> {
        self.defs.get(id.index())
    }
}
