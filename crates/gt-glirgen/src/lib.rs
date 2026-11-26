//! Hạ kiểu Grit → kiểu GLIR (C2). Quy tắc ĐÚNG bằng `lower_ty` của codegen LLVM: `bool` = i1, `char` = i32, `()`/`!` = struct rỗng, con trỏ thường `ptr`,
//! con trỏ béo `{ptr, i64}`, `Mutex<T, L>` = `{T, i64}`, enum = `{i64, [k × i64]}`. Bố cục (cỡ/căn lề) của kiểu GLIR được `gt_layout::glir_layout`
//! tính độc lập và PHẢI bằng `gt_layout::layout_of` trên kiểu Grit gốc (test vi sai ở gt-driver).
use gt_glir::Ty as G;
use gt_types::{AdtKind, FloatTy, Lang, Ty, TypeDb};

fn int(bits: u32) -> Result<G, String> {
    Ok(match bits {
        1 => G::I1,
        8 => G::I8,
        16 => G::I16,
        32 => G::I32,
        64 => G::I64,
        128 => G::I128,
        b => return Err(format!("độ rộng số nguyên {b} chưa hỗ trợ trong GLIR")),
    })
}

pub(crate) fn variant_fields(
    db: &TypeDb,
    def: gt_resolve::DefId,
    args: &[Ty],
    v: usize,
) -> Result<Vec<Ty>, String> {
    let adt = db.adts.get(&def).ok_or("ADT không có trong db")?;
    if adt.generics.len() != args.len() {
        return Err("số đối số kiểu của ADT không khớp".into());
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

/// Kiểu GLIR của một kiểu Grit ĐÃ CỤ THỂ.
pub fn lower_ty(db: &TypeDb, ty: &Ty) -> Result<G, String> {
    lower_inner(db, ty, 0)
}

fn lower_inner(db: &TypeDb, ty: &Ty, depth: u32) -> Result<G, String> {
    if depth > 256 {
        return Err("kiểu lồng quá sâu".into());
    }
    let rec = |t: &Ty| lower_inner(db, t, depth + 1);
    Ok(match ty {
        Ty::Int(it) => int(it.bits())?,
        Ty::Bool => G::I1,
        Ty::Char => G::I32,
        Ty::Never => G::Struct(Vec::new()),
        Ty::Float(FloatTy::F64) => G::F64,
        Ty::Float(_) => return Err("f32".into()),
        Ty::Tuple(v) => G::Struct(v.iter().map(rec).collect::<Result<_, _>>()?),
        Ty::Array(e, n) => G::Array(Box::new(rec(e)?), *n),
        Ty::Ref(_, i) | Ty::RawPtr(_, i) => {
            if matches!(**i, Ty::Str | Ty::Slice(_)) {
                G::Struct(vec![G::Ptr, G::I64])
            } else {
                G::Ptr
            }
        }
        Ty::Fn(..) => G::Ptr,
        Ty::Adt(def, args) => {
            let adt = db.adts.get(def).ok_or("ADT không có trong db")?;
            match (&adt.kind, adt.lang) {
                (AdtKind::Struct(_), Some(Lang::Mutex)) => {
                    let fs = variant_fields(db, *def, args, 0)?;
                    let [t0] = fs.as_slice() else {
                        return Err("Mutex phải có đúng một trường".into());
                    };
                    G::Struct(vec![rec(t0)?, G::I64])
                }
                (AdtKind::Struct(_), _) => G::Struct(
                    variant_fields(db, *def, args, 0)?
                        .iter()
                        .map(rec)
                        .collect::<Result<_, _>>()?,
                ),
                (AdtKind::Enum(vs), _) => {
                    let mut max = 0u64;
                    for v in 0..vs.len() {
                        let st = G::Struct(
                            variant_fields(db, *def, args, v)?
                                .iter()
                                .map(rec)
                                .collect::<Result<_, _>>()?,
                        );
                        let l = gt_layout::glir_layout(&st);
                        if l.align > 8 {
                            return Err("biến thể enum cần căn lề > 8".into());
                        }
                        max = max.max(l.size);
                    }
                    G::Struct(vec![G::I64, G::Array(Box::new(G::I64), max.div_ceil(8))])
                }
            }
        }
        Ty::Str | Ty::Slice(_) => return Err("kiểu không kích thước đứng riêng".into()),
        Ty::Param(_) | Ty::Infer(_) => return Err("tham số kiểu chưa được thế".into()),
        Ty::Err => return Err("Ty::Err sau cổng".into()),
    })
}

pub mod body;
pub use body::{
    Lowered, Sigs, Unsupported, check_abort_cover, glir_name, inst_name, lower_body, lower_module,
    runtime_decl,
};
