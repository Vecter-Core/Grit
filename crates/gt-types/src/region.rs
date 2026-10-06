//! Region (lifetime có tên) ở chữ ký hàm — M4.5, `docs/spec/regions.md`. M4 KHÔNG suy luận region trong thân hàm (việc của M6);
//! module này chỉ trích xuất bảng region của từng chữ ký (kèm luật elision) để M6 dùng lại, không thiết kế lại.
use gt_ast::{Type, TypeKind};

/// Một vị trí region trong chữ ký.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegionRef {
    /// `'a` do người dùng ghi (không gồm dấu `'`).
    Named(String),
    /// `'static`.
    Static,
    /// Region ẩn (elision) của tham chiếu đầu vào thứ `n` (đánh số theo thứ tự xuất hiện, `self` đầu tiên).
    Anon(u32),
}

/// Bảng region của một chữ ký hàm (khoá: NodeId của hàm/thành viên).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegionSig {
    /// Lifetime khai báo ở CHÍNH hàm này (không gồm của impl/trait chứa nó), theo thứ tự khai báo.
    pub declared: Vec<String>,
    /// Với mỗi tham số (kể cả `self` đứng đầu): các vị trí region của nó, theo thứ tự duyệt trước.
    pub inputs: Vec<Vec<RegionRef>>,
    /// Các vị trí region của kiểu trả về, theo thứ tự duyệt trước; vị trí ẩn đã được gán theo luật elision.
    pub output: Vec<RegionRef>,
}

/// Mọi vị trí tham chiếu `&…` trong một kiểu (theo thứ tự duyệt trước): `Some(tên)` nếu có lifetime, `None` nếu ẩn.
/// Không đi vào kiểu hàm `ƒ(…)`: nó có phạm vi elision riêng.
pub(crate) fn slots(t: &Type, out: &mut Vec<Option<String>>) {
    match &t.kind {
        TypeKind::Ref {
            lifetime, inner, ..
        } => {
            out.push(lifetime.as_ref().map(|l| l.name.clone()));
            slots(inner, out);
        }
        TypeKind::Path { args, .. } => {
            for a in args {
                slots(a, out);
            }
        }
        TypeKind::Array { elem, .. } => slots(elem, out),
        TypeKind::Slice(inner) | TypeKind::RawPtr { inner, .. } => slots(inner, out),
        TypeKind::Tuple(v) => {
            for a in v {
                slots(a, out);
            }
        }
        _ => {}
    }
}

pub(crate) fn region_of(slot: &Option<String>, anon: &mut u32) -> RegionRef {
    match slot {
        Some(n) if n == "static" => RegionRef::Static,
        Some(n) => RegionRef::Named(n.clone()),
        None => {
            let r = RegionRef::Anon(*anon);
            *anon += 1;
            r
        }
    }
}
