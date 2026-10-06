//! Bảng suy luận cục bộ: biến kiểu (chung / số nguyên / số thực), hợp nhất (unify), mặc định hoá literal.
use crate::ty::{FloatTy, IntTy, Ty};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarKind {
    General,
    /// Biến của literal số nguyên chưa có hậu tố: chỉ hợp nhất với kiểu nguyên (hoặc biến nguyên khác).
    Int,
    /// Biến của literal số thực chưa có hậu tố.
    Float,
}

#[derive(Clone, Debug)]
struct Slot {
    kind: VarKind,
    binding: Option<Ty>,
}

#[derive(Clone, Debug, Default)]
pub struct InferCtx {
    slots: Vec<Slot>,
}

impl InferCtx {
    #[must_use]
    pub fn new() -> InferCtx {
        InferCtx::default()
    }

    pub fn fresh(&mut self, kind: VarKind) -> Ty {
        let id = u32::try_from(self.slots.len()).unwrap_or(u32::MAX);
        self.slots.push(Slot {
            kind,
            binding: None,
        });
        Ty::Infer(id)
    }

    pub fn fresh_var(&mut self) -> Ty {
        self.fresh(VarKind::General)
    }

    #[must_use]
    pub fn kind_of(&self, id: u32) -> VarKind {
        self.slots
            .get(id as usize)
            .map_or(VarKind::General, |s| s.kind)
    }

    /// Theo chuỗi liên kết tới gốc (không đệ quy vào cấu trúc).
    #[must_use]
    pub fn shallow(&self, ty: &Ty) -> Ty {
        let mut cur = ty.clone();
        while let Ty::Infer(i) = cur {
            match self.slots.get(i as usize).and_then(|s| s.binding.clone()) {
                Some(next) => cur = next,
                None => break,
            }
        }
        cur
    }

    /// Thay mọi biến đã giải bằng giá trị của nó (đệ quy).
    #[must_use]
    pub fn resolve(&self, ty: &Ty) -> Ty {
        match self.shallow(ty) {
            Ty::Tuple(v) => Ty::Tuple(v.iter().map(|t| self.resolve(t)).collect()),
            Ty::Adt(d, v) => Ty::Adt(d, v.iter().map(|t| self.resolve(t)).collect()),
            Ty::Array(t, n) => Ty::Array(Box::new(self.resolve(&t)), n),
            Ty::Slice(t) => Ty::Slice(Box::new(self.resolve(&t))),
            Ty::Ref(m, t) => Ty::Ref(m, Box::new(self.resolve(&t))),
            Ty::RawPtr(m, t) => Ty::RawPtr(m, Box::new(self.resolve(&t))),
            Ty::Fn(p, r, e) => Ty::Fn(
                p.iter().map(|t| self.resolve(t)).collect(),
                Box::new(self.resolve(&r)),
                e,
            ),
            other => other,
        }
    }

    fn occurs(&self, id: u32, ty: &Ty) -> bool {
        match self.shallow(ty) {
            Ty::Infer(j) => j == id,
            Ty::Tuple(v) | Ty::Adt(_, v) => v.iter().any(|t| self.occurs(id, t)),
            Ty::Array(t, _) | Ty::Slice(t) | Ty::Ref(_, t) | Ty::RawPtr(_, t) => {
                self.occurs(id, &t)
            }
            Ty::Fn(p, r, _) => p.iter().any(|t| self.occurs(id, t)) || self.occurs(id, &r),
            _ => false,
        }
    }

    fn bind(&mut self, id: u32, ty: Ty) -> bool {
        // kiểu của biến nguyên/thực phải tương thích
        if let Ty::Infer(j) = ty {
            let (ka, kb) = (self.kind_of(id), self.kind_of(j));
            let merged = match (ka, kb) {
                (VarKind::General, k) | (k, VarKind::General) => k,
                (a, b) if a == b => a,
                _ => return false,
            };
            if let Some(s) = self.slots.get_mut(j as usize) {
                s.kind = merged;
            }
        } else {
            let ok = matches!(
                (self.kind_of(id), &ty),
                (VarKind::General, _) | (VarKind::Int, Ty::Int(_)) | (VarKind::Float, Ty::Float(_))
            );
            if !ok || self.occurs(id, &ty) {
                return false;
            }
        }
        if let Some(s) = self.slots.get_mut(id as usize) {
            s.binding = Some(ty);
        }
        true
    }

    /// Hợp nhất hai kiểu. `Never` chỉ hợp nhất với `Never` (ép kiểu `!` xử lý ở nơi gọi, ADR-018 §3).
    pub fn unify(&mut self, a: &Ty, b: &Ty) -> bool {
        let (a, b) = (self.shallow(a), self.shallow(b));
        match (&a, &b) {
            (Ty::Err, _) | (_, Ty::Err) => true,
            (Ty::Infer(i), Ty::Infer(j)) if i == j => true,
            (Ty::Infer(i), _) => self.bind(*i, b.clone()),
            (_, Ty::Infer(j)) => self.bind(*j, a.clone()),
            (Ty::Int(x), Ty::Int(y)) => x == y,
            (Ty::Float(x), Ty::Float(y)) => x == y,
            (Ty::Bool, Ty::Bool)
            | (Ty::Char, Ty::Char)
            | (Ty::Str, Ty::Str)
            | (Ty::Never, Ty::Never) => true,
            (Ty::Tuple(x), Ty::Tuple(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(p, q)| self.unify(p, q))
            }
            (Ty::Array(x, n), Ty::Array(y, m)) => n == m && self.unify(x, y),
            (Ty::Slice(x), Ty::Slice(y)) => self.unify(x, y),
            (Ty::Ref(m1, x), Ty::Ref(m2, y)) | (Ty::RawPtr(m1, x), Ty::RawPtr(m2, y)) => {
                m1 == m2 && self.unify(x, y)
            }
            (Ty::Adt(d1, x), Ty::Adt(d2, y)) => {
                d1 == d2 && x.len() == y.len() && x.iter().zip(y).all(|(p, q)| self.unify(p, q))
            }
            (Ty::Fn(p1, r1, e1), Ty::Fn(p2, r2, e2)) => {
                e1 == e2
                    && p1.len() == p2.len()
                    && p1.iter().zip(p2).all(|(p, q)| self.unify(p, q))
                    && self.unify(r1, r2)
            }
            (Ty::Param(x), Ty::Param(y)) => x == y,
            _ => false,
        }
    }

    /// Thử `f` rồi hoàn tác mọi ràng buộc đã thêm (dò ứng viên phương thức/impl).
    pub fn probe<R>(&mut self, f: impl FnOnce(&mut InferCtx) -> R) -> R {
        let saved = self.slots.clone();
        let r = f(self);
        self.slots = saved;
        r
    }

    /// Mặc định hoá: biến nguyên chưa giải → `i32`, biến thực → `f64` (ADR-018; `semantics.md` §2).
    pub fn default_literals(&mut self) {
        for i in 0..self.slots.len() {
            if self.slots[i].binding.is_none() {
                match self.slots[i].kind {
                    VarKind::Int => self.slots[i].binding = Some(Ty::Int(IntTy::I32)),
                    VarKind::Float => self.slots[i].binding = Some(Ty::Float(FloatTy::F64)),
                    VarKind::General => {}
                }
            }
        }
    }
}
