//! Kiểu của place/toán hạng/rvalue trong MIR, và số vị trí region của một kiểu (cùng quy ước `RegionSig`, ADR-019/021).
use crate::mir::*;
use gt_types::{AdtKind, IntTy, Lang, Ty, TypeDb};

/// Số vị trí region của `ty` theo thứ tự duyệt trước: mỗi `&`/`&!`, mỗi tham số lifetime của ADT; không đi vào kiểu hàm;
/// con trỏ thô không có region riêng.
#[must_use]
pub fn region_slots(db: &TypeDb, ty: &Ty) -> usize {
    match ty {
        Ty::Ref(_, inner) => 1 + region_slots(db, inner),
        Ty::Adt(d, args) => {
            db.adts.get(d).map_or(0, |a| a.lt_params)
                + args.iter().map(|a| region_slots(db, a)).sum::<usize>()
        }
        Ty::Tuple(v) => v.iter().map(|a| region_slots(db, a)).sum(),
        Ty::Array(t, _) | Ty::Slice(t) | Ty::RawPtr(_, t) => region_slots(db, t),
        _ => 0,
    }
}

/// Kiểu của các trường một biến thể (`variant = 0` cho struct), đã thế đối số kiểu.
#[must_use]
pub fn variant_fields(db: &TypeDb, def: gt_resolve::DefId, variant: u32, args: &[Ty]) -> Vec<Ty> {
    let Some(adt) = db.adts.get(&def) else {
        return Vec::new();
    };
    let sub = |t: &Ty| {
        t.subst(&|p| {
            adt.generics
                .iter()
                .position(|g| g.id == p)
                .and_then(|i| args.get(i).cloned())
        })
    };
    match &adt.kind {
        AdtKind::Struct(fs) => fs.iter().map(|f| sub(&f.ty)).collect(),
        AdtKind::Enum(vs) => vs
            .get(variant as usize)
            .map(|v| v.fields.iter().map(sub).collect())
            .unwrap_or_default(),
    }
}

/// Kiểu của một chỗ. `None` nếu phép chiếu không hợp lệ với kiểu (validator báo).
#[must_use]
pub fn place_ty(body: &Body, place: &Place) -> Option<Ty> {
    let mut ty = body.locals.get(place.local.index())?.ty.clone();
    for e in &place.projection {
        ty = match (e, ty) {
            (PlaceElem::Deref, Ty::Ref(_, t) | Ty::RawPtr(_, t)) => *t,
            (PlaceElem::Field(_, t), Ty::Tuple(_) | Ty::Adt(..)) => t.clone(),
            (PlaceElem::Index(_), Ty::Array(t, _) | Ty::Slice(t)) => *t,
            (PlaceElem::Subslice { .. }, Ty::Array(t, _) | Ty::Slice(t)) => Ty::Slice(t),
            (PlaceElem::Downcast(_), t @ Ty::Adt(..)) => t,
            _ => return None,
        };
    }
    Some(ty)
}

#[must_use]
pub fn const_ty(c: &Const) -> Ty {
    match c {
        Const::Int { ty, .. } => Ty::Int(*ty),
        Const::Float { ty, .. } => Ty::Float(*ty),
        Const::Bool(_) => Ty::Bool,
        Const::Char(_) => Ty::Char,
        Const::Str(_) => Ty::Ref(false, Box::new(Ty::Str)),
        Const::Unit => Ty::unit(),
        Const::FnItem { ty, .. } | Const::Item { ty, .. } => ty.clone(),
    }
}

#[must_use]
pub fn operand_ty(body: &Body, op: &Operand) -> Option<Ty> {
    match op {
        Operand::Copy(p) | Operand::Move(p) => place_ty(body, p),
        Operand::Const(c) => Some(const_ty(c)),
    }
}

/// Kiểu của rvalue. `lang`: để dựng kiểu kết quả của phép so sánh/độ dài không cần thêm thông tin.
#[must_use]
pub fn rvalue_ty(body: &Body, db: &TypeDb, rv: &Rvalue) -> Option<Ty> {
    Some(match rv {
        Rvalue::Use(o) => operand_ty(body, o)?,
        Rvalue::Repeat(o, n) => Ty::Array(Box::new(operand_ty(body, o)?), *n),
        Rvalue::Ref(_, k, p) => Ty::Ref(*k == BorrowKind::Unique, Box::new(place_ty(body, p)?)),
        Rvalue::Len(_) => Ty::Int(IntTy::Usize),
        Rvalue::Cast(_, t) => t.clone(),
        Rvalue::CastFits(..) => Ty::Bool,
        Rvalue::BinaryOp(op, a, _) => {
            if op.is_comparison() {
                Ty::Bool
            } else {
                operand_ty(body, a)?
            }
        }
        Rvalue::CheckedBinaryOp(_, a, _) => Ty::Tuple(vec![operand_ty(body, a)?, Ty::Bool]),
        Rvalue::UnaryOp(_, o) => operand_ty(body, o)?,
        Rvalue::Discriminant(_) => Ty::Int(IntTy::Usize),
        Rvalue::Aggregate(k, ops) => match k {
            AggregateKind::Tuple => Ty::Tuple(
                ops.iter()
                    .map(|o| operand_ty(body, o))
                    .collect::<Option<Vec<_>>>()?,
            ),
            AggregateKind::Array(t) => Ty::Array(Box::new(t.clone()), ops.len() as u64),
            AggregateKind::Adt(d, _, args) => {
                let _ = db;
                Ty::Adt(*d, args.clone())
            }
        },
    })
}

/// `Option<T>` nếu prelude có lang item `option`.
#[must_use]
pub fn option_of(db: &TypeDb, t: Ty) -> Option<Ty> {
    db.lang.get(&Lang::Option).map(|d| Ty::Adt(*d, vec![t]))
}
