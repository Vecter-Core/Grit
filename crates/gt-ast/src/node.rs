//! NodeId và trait `Node`. Mọi "node" của cây có `id` (NodeId) và `span`.
use gt_span::Span;

/// Độ sâu cây tối đa — NGUỒN DUY NHẤT của giới hạn (semantics §26). Parser từ chối chương trình sâu hơn (E0102); mọi pass
/// đệ quy trên AST được phép giả định cây không sâu hơn hằng này và PHẢI chạy qua `gt_stack::run_with_big_stack`.
pub const MAX_DEPTH: u32 = 256;

/// Định danh node: cấp **tuần tự từ 0, không lỗ** bởi parser theo thứ tự dựng tất định (D9).
/// Duy nhất trong một tệp; các pass sau gắn bảng kết quả bằng `Vec<T>` chỉ mục `index()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u32);

impl NodeId {
    #[must_use]
    pub fn new(raw: u32) -> NodeId {
        NodeId(raw)
    }

    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Bộ cấp NodeId tuần tự. Hết `u32` → `None` (không panic, không tràn lặng lẽ).
#[derive(Clone, Debug)]
pub struct NodeIdGen {
    next: Option<u32>,
    count: u32,
}

impl NodeIdGen {
    #[must_use]
    pub fn new() -> NodeIdGen {
        NodeIdGen::starting_at(0)
    }

    #[must_use]
    pub fn starting_at(first: u32) -> NodeIdGen {
        NodeIdGen {
            next: Some(first),
            count: 0,
        }
    }

    /// Cấp NodeId kế tiếp, hoặc `None` nếu đã hết.
    pub fn next_id(&mut self) -> Option<NodeId> {
        let cur = self.next?;
        self.next = cur.checked_add(1);
        self.count += 1;
        Some(NodeId(cur))
    }

    /// Số NodeId đã cấp.
    #[must_use]
    pub fn count(&self) -> u32 {
        self.count
    }
}

impl Default for NodeIdGen {
    fn default() -> Self {
        NodeIdGen::new()
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
