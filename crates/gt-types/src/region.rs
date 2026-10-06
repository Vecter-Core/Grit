//! Region (lifetime có tên) ở chữ ký hàm — M4.5, `docs/spec/regions.md`. M4 KHÔNG suy luận region trong thân hàm (việc của M6);
//! module này chỉ trích xuất bảng region của từng chữ ký (kèm luật elision) để M6 dùng lại, không thiết kế lại.
use gt_ast::{Path, Type, TypeKind};

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
    /// Song song với `inputs`: vị trí đó đến từ kiểu của impl (`Self` trong `⊐ Holder<'a>`) chứ không phải do chữ ký tự ghi.
    /// M6 dùng để so chữ ký impl với trait (không tính vị trí của `Self`); luật elision cũng không đếm chúng.
    pub input_self: Vec<Vec<bool>>,
    /// Song song với `output`: vị trí đến từ `Self`.
    pub output_self: Vec<bool>,
}

/// Một vị trí region: tên (nếu ghi) và có đến từ `Self` không.
pub(crate) type Slot = (Option<String>, bool);

/// Mọi vị trí region trong một kiểu (theo thứ tự duyệt trước): mỗi `&…`, rồi mỗi tham số lifetime của kiểu đường dẫn
/// (`Holder<'a>` — ADR-021; bỏ trống thì mỗi tham số là một vị trí ẩn). `Some(tên)` nếu ghi lifetime, `None` nếu ẩn.
/// `lt_count` cho biết kiểu đường dẫn có bao nhiêu tham số lifetime; `self_slots` là các vị trí của kiểu `Self` (impl).
/// Không đi vào kiểu hàm `ƒ(…)`: nó có phạm vi elision riêng.
pub(crate) fn slots(
    t: &Type,
    out: &mut Vec<Slot>,
    lt_count: &dyn Fn(&Path) -> usize,
    self_slots: &[Option<String>],
) {
    match &t.kind {
        TypeKind::SelfType => {
            out.extend(self_slots.iter().map(|s| (s.clone(), true)));
        }
        TypeKind::Ref {
            lifetime, inner, ..
        } => {
            out.push((lifetime.as_ref().map(|l| l.name.clone()), false));
            slots(inner, out, lt_count, self_slots);
        }
        TypeKind::Path {
            path,
            lifetimes,
            args,
        } => {
            if lifetimes.is_empty() {
                out.extend(std::iter::repeat_n((None, false), lt_count(path)));
            } else {
                out.extend(lifetimes.iter().map(|l| (Some(l.name.clone()), false)));
            }
            for a in args {
                slots(a, out, lt_count, self_slots);
            }
        }
        TypeKind::Array { elem, .. } => slots(elem, out, lt_count, self_slots),
        TypeKind::Slice(inner) | TypeKind::RawPtr { inner, .. } => {
            slots(inner, out, lt_count, self_slots);
        }
        TypeKind::Tuple(v) => {
            for a in v {
                slots(a, out, lt_count, self_slots);
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
