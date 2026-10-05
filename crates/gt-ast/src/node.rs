//! NodeId và trait `Node`. Mọi "node" của cây có `id` (NodeId) và `span`.
use gt_span::{FileId, Span};

/// Độ sâu cây tối đa — NGUỒN DUY NHẤT của giới hạn (semantics §26). Parser từ chối chương trình sâu hơn (E0102); mọi pass
/// đệ quy trên AST được phép giả định cây không sâu hơn hằng này và PHẢI chạy qua `gt_stack::run_with_big_stack`.
pub const MAX_DEPTH: u32 = 256;

/// Định danh node: `(FileId, chỉ số)`. Chỉ số cấp **tuần tự từ 0, không lỗ** trong từng tệp bởi parser theo thứ tự dựng
/// tất định (D9); `FileId` làm khoá DUY NHẤT toàn chương trình (ADR-017a). Thứ tự: tệp trước, rồi chỉ số.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId {
    file: FileId,
    index: u32,
}

impl NodeId {
    #[must_use]
    pub fn new(file: FileId, index: u32) -> NodeId {
        NodeId { file, index }
    }

    #[must_use]
    pub fn file(self) -> FileId {
        self.file
    }

    /// Chỉ số trong tệp (dày từ 0).
    #[must_use]
    pub fn raw(self) -> u32 {
        self.index
    }

    /// Chỉ số trong tệp dưới dạng `usize` (để chỉ mục `Vec` theo tệp).
    #[must_use]
    pub fn index(self) -> usize {
        self.index as usize
    }
}

/// Bộ cấp NodeId tuần tự cho MỘT tệp. Hết `u32` → `None` (không panic, không tràn lặng lẽ).
#[derive(Clone, Debug)]
pub struct NodeIdGen {
    file: FileId,
    next: Option<u32>,
    count: u32,
}

impl NodeIdGen {
    #[must_use]
    pub fn new(file: FileId) -> NodeIdGen {
        NodeIdGen::starting_at(file, 0)
    }

    #[must_use]
    pub fn starting_at(file: FileId, first: u32) -> NodeIdGen {
        NodeIdGen {
            file,
            next: Some(first),
            count: 0,
        }
    }

    /// Cấp NodeId kế tiếp, hoặc `None` nếu đã hết.
    pub fn next_id(&mut self) -> Option<NodeId> {
        let cur = self.next?;
        self.next = cur.checked_add(1);
        self.count += 1;
        Some(NodeId::new(self.file, cur))
    }

    /// Số NodeId đã cấp.
    #[must_use]
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Tệp mà bộ cấp này phục vụ.
    #[must_use]
    pub fn file(&self) -> FileId {
        self.file
    }
}

/// Mọi node trong cây có NodeId và Span.
pub trait Node {
    fn id(&self) -> NodeId;
    fn span(&self) -> Span;
}

/// Cài `Node` cho các struct có trường `id` và `span`.
macro_rules! impl_node {
    ($($t:ty),* $(,)?) => {
        $(impl $crate::node::Node for $t {
            fn id(&self) -> $crate::node::NodeId { self.id }
            fn span(&self) -> gt_span::Span { self.span }
        })*
    };
}
pub(crate) use impl_node;
