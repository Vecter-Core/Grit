//! `gt-layout` (ADR-P2-02 bước 1): cỡ/căn lề của kiểu Grit ĐÃ CỤ THỂ, tính bằng Rust thuần theo ĐÚNG quy tắc mà codegen LLVM hiện hạ:
//! struct/tuple không đóng gói (trường theo thứ tự khai báo, đệm theo căn lề), mảng liền kề, con trỏ thường 8/8, con trỏ béo `{ptr, i64}` 8/16,
//! `Mutex<T, L>` = `{T, i64}`, enum = `{i64, [k × i64]}` (k = ⌈cỡ lớn nhất của một biến thể / 8⌉; mỗi biến thể căn lề ≤ 8).
//! TRẠNG THÁI: chỉ để ĐỐI CHIẾU (codegen xác nhận cỡ/căn lề của MỌI kiểu nó hạ bằng `DataLayout` của LLVM; lệch ⇒ ICE) — chưa là nguồn sự thật của bố cục.
use gt_types::{AdtKind, FloatTy, Lang, Ty, TypeDb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub size: u64,
    pub align: u64,
}

fn round_up(n: u64, a: u64) -> u64 {
    n.div_ceil(a) * a
}

/// Bố cục struct không đóng gói của các trường (thứ tự khai báo).
#[must_use]
pub fn struct_layout(fields: &[Layout]) -> Layout {
    let mut off = 0u64;
    let mut align = 1u64;
    for f in fields {
        off = round_up(off, f.align) + f.size;
        align = align.max(f.align);
    }
    Layout {
        size: round_up(off, align),
        align,
    }
}

pub(crate) fn int_layout(bits: u32) -> Layout {
    let size = u64::from(bits.div_ceil(8));
    // x86-64: iN căn lề tự nhiên tới 64 bit; i128 căn lề 16 (LLVM ≥ 18)
    let align = if bits >= 128 {
        16
    } else {
        size.next_power_of_two().min(8)
    };
    Layout { size, align }
}

fn is_unsized(t: &Ty) -> bool {
    matches!(t, Ty::Str | Ty::Slice(_))
}

fn fields_of(
    db: &TypeDb,
    def: gt_resolve::DefId,
    args: &[Ty],
    v: usize,
) -> Result<Vec<Ty>, String> {
    let adt = db.adts.get(&def).ok_or("ADT không có trong db")?;
    if adt.generics.len() != args.len() {
        return Err("số đối số kiểu của ADT không khớp".to_string());
    }
    let raw: Vec<&Ty> = match &adt.kind {
        AdtKind::Struct(fs) => fs.iter().map(|f| &f.ty).collect(),
        AdtKind::Enum(vs) => vs
            .get(v)
            .ok_or("biến thể không tồn tại")?
            .fields
            .iter()
            .collect(),
    };
    Ok(raw
        .iter()
        .map(|t| {
            t.subst(&|p| {
                adt.generics
                    .iter()
                    .position(|g| g.id == p)
                    .and_then(|i| args.get(i).cloned())
            })
        })
        .collect())
}

/// Bố cục của một kiểu GLIR: dùng chung quy tắc với `gt_glir::layout` (nguồn duy nhất của offset/cỡ GLIR).
#[must_use]
pub fn glir_layout(t: &gt_glir::Ty) -> Layout {
    let l = gt_glir::layout::layout(t);
    Layout {
        size: l.size,
        align: l.align,
    }
}

/// Cỡ + căn lề của `ty` (đã cụ thể). `Err` khi kiểu chưa hỗ trợ (cùng tập mà codegen từ chối).
pub fn layout_of(db: &TypeDb, ty: &Ty) -> Result<Layout, String> {
    layout_inner(db, ty, 0)
}

fn layout_inner(db: &TypeDb, ty: &Ty, depth: u32) -> Result<Layout, String> {
    if depth > 256 {
        return Err("kiểu lồng quá sâu".to_string());
    }
    let rec = |t: &Ty| layout_inner(db, t, depth + 1);
    Ok(match ty {
        Ty::Int(it) => int_layout(it.bits()),
        Ty::Bool => Layout { size: 1, align: 1 },
        Ty::Char => Layout { size: 4, align: 4 },
        Ty::Never => Layout { size: 0, align: 1 },
        Ty::Float(FloatTy::F64) => Layout { size: 8, align: 8 },
        Ty::Float(_) => return Err("f32".to_string()),
        Ty::Tuple(v) => struct_layout(&v.iter().map(rec).collect::<Result<Vec<_>, _>>()?),
        Ty::Array(e, n) => {
            let el = rec(e)?;
            Layout {
                size: el.size.checked_mul(*n).ok_or("mảng quá lớn")?,
                align: el.align,
            }
        }
        Ty::Ref(_, inner) | Ty::RawPtr(_, inner) => {
            if is_unsized(inner) {
                Layout { size: 16, align: 8 }
            } else {
                Layout { size: 8, align: 8 }
            }
        }
        Ty::Fn(..) => Layout { size: 8, align: 8 },
        Ty::Adt(def, args) => {
            let adt = db.adts.get(def).ok_or("ADT không có trong db")?;
            match (&adt.kind, adt.lang) {
                (AdtKind::Struct(_), Some(Lang::Mutex)) => {
                    let fs = fields_of(db, *def, args, 0)?;
                    let [t0] = fs.as_slice() else {
                        return Err("Mutex phải có đúng một trường".to_string());
                    };
                    struct_layout(&[rec(t0)?, Layout { size: 8, align: 8 }])
                }
                (AdtKind::Struct(_), _) => {
                    let fs = fields_of(db, *def, args, 0)?;
                    struct_layout(&fs.iter().map(rec).collect::<Result<Vec<_>, _>>()?)
                }
                (AdtKind::Enum(vs), _) => {
                    let mut max = 0u64;
                    for v in 0..vs.len() {
                        let fs = fields_of(db, *def, args, v)?;
                        let l = struct_layout(&fs.iter().map(rec).collect::<Result<Vec<_>, _>>()?);
                        if l.align > 8 {
                            return Err("biến thể enum cần căn lề > 8".to_string());
                        }
                        max = max.max(l.size);
                    }
                    let k = max.div_ceil(8);
                    struct_layout(&[
                        Layout { size: 8, align: 8 },
                        Layout {
                            size: 8 * k,
                            align: 8,
                        },
                    ])
                }
            }
        }
        Ty::Str | Ty::Slice(_) => return Err("kiểu không kích thước đứng riêng".to_string()),
        Ty::Param(_) | Ty::Infer(_) => return Err("tham số kiểu chưa được thế".to_string()),
        Ty::Err => return Err("Ty::Err sau cổng".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_padding_follows_declaration_order() {
        let l = |s, a| Layout { size: s, align: a };
        // {u8, u32, u8}: 1 + 3 đệm + 4 + 1 + 3 đệm = 12
        assert_eq!(struct_layout(&[l(1, 1), l(4, 4), l(1, 1)]), l(12, 4));
        assert_eq!(struct_layout(&[]), l(0, 1));
    }
}
