//! Bố cục (cỡ/căn lề/offset trường) của kiểu GLIR — quy tắc x86-64: struct không đóng gói, mảng liền kề, `ptr`/`f64` 8/8, `iN` căn lề tự nhiên (≤ 8; i128 → 16).
//! `gt-layout::glir_layout` dùng lại hàm này; ĐỐI CHIẾU độc lập với `DataLayout` của LLVM nằm ở codegen (bản debug) và `layout_diff`.
use crate::Ty;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub size: u64,
    pub align: u64,
}

fn round_up(n: u64, a: u64) -> u64 {
    n.div_ceil(a) * a
}

#[must_use]
pub fn int_layout(bits: u32) -> Layout {
    let size = u64::from(bits.div_ceil(8));
    let align = if bits >= 128 {
        16
    } else {
        size.next_power_of_two().min(8)
    };
    Layout { size, align }
}

/// `(bố cục struct, offset từng trường)` của danh sách trường.
#[must_use]
pub fn struct_layout(fields: &[Layout]) -> (Layout, Vec<u64>) {
    let (mut off, mut align) = (0u64, 1u64);
    let mut offs = Vec::new();
    for f in fields {
        off = round_up(off, f.align);
        offs.push(off);
        off += f.size;
        align = align.max(f.align);
    }
    (
        Layout {
            size: round_up(off, align),
            align,
        },
        offs,
    )
}

#[must_use]
pub fn layout(t: &Ty) -> Layout {
    match t {
        Ty::I1 => int_layout(1),
        Ty::I8 => int_layout(8),
        Ty::I16 => int_layout(16),
        Ty::I32 => int_layout(32),
        Ty::I64 => int_layout(64),
        Ty::I128 => int_layout(128),
        Ty::F64 | Ty::Ptr => Layout { size: 8, align: 8 },
        Ty::Void => Layout { size: 0, align: 1 },
        Ty::Struct(fs) => struct_layout(&fs.iter().map(layout).collect::<Vec<_>>()).0,
        Ty::Array(e, n) => {
            let el = layout(e);
            Layout {
                size: el.size.saturating_mul(*n),
                align: el.align,
            }
        }
    }
}

/// Offset của trường `idx` trong struct `t` (`None` nếu `t` không phải struct hoặc `idx` ngoài phạm vi).
#[must_use]
pub fn field_offset(t: &Ty, idx: u32) -> Option<u64> {
    let Ty::Struct(fs) = t else { return None };
    struct_layout(&fs.iter().map(layout).collect::<Vec<_>>())
        .1
        .get(idx as usize)
        .copied()
}
