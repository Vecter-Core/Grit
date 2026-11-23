//! Biểu diễn kiểu của M4. Kiểu là giá trị thuần (so sánh/băm/sắp xếp theo cấu trúc) để kết quả tất định.
use gt_ast::{NodeId, PrimType};
use gt_resolve::{Def, DefId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IntTy {
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
}

impl IntTy {
    #[must_use]
    pub fn signed(self) -> bool {
        matches!(
            self,
            IntTy::I8 | IntTy::I16 | IntTy::I32 | IntTy::I64 | IntTy::I128 | IntTy::Isize
        )
    }

    /// Số bit (isize/usize: 64 — Phase 1 chỉ có mục tiêu x86-64, `semantics.md` §1).
    #[must_use]
    pub fn bits(self) -> u32 {
        match self {
            IntTy::I8 | IntTy::U8 => 8,
            IntTy::I16 | IntTy::U16 => 16,
            IntTy::I32 | IntTy::U32 => 32,
            IntTy::I64 | IntTy::U64 | IntTy::Isize | IntTy::Usize => 64,
            IntTy::I128 | IntTy::U128 => 128,
        }
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            IntTy::I8 => "i8",
            IntTy::I16 => "i16",
            IntTy::I32 => "i32",
            IntTy::I64 => "i64",
            IntTy::I128 => "i128",
            IntTy::Isize => "isize",
            IntTy::U8 => "u8",
            IntTy::U16 => "u16",
            IntTy::U32 => "u32",
            IntTy::U64 => "u64",
            IntTy::U128 => "u128",
            IntTy::Usize => "usize",
        }
    }

    /// Giá trị nhỏ nhất (i128 đủ cho mọi kiểu có dấu; kiểu không dấu: 0).
    #[must_use]
    pub fn min_val(self) -> i128 {
        if self.signed() {
            if self.bits() == 128 {
                i128::MIN
            } else {
                -(1i128 << (self.bits() - 1))
            }
        } else {
            0
        }
    }

    /// Giá trị lớn nhất (u128).
    #[must_use]
    pub fn max_val(self) -> u128 {
        if self.signed() {
            if self.bits() == 128 {
                i128::MAX as u128
            } else {
                (1u128 << (self.bits() - 1)) - 1
            }
        } else if self.bits() == 128 {
            u128::MAX
        } else {
            (1u128 << self.bits()) - 1
        }
    }

    #[must_use]
    pub fn from_suffix(s: &str) -> Option<IntTy> {
        Some(match s {
            "i8" => IntTy::I8,
            "i16" => IntTy::I16,
            "i32" => IntTy::I32,
            "i64" => IntTy::I64,
            "i128" => IntTy::I128,
            "isize" => IntTy::Isize,
            "u8" => IntTy::U8,
            "u16" => IntTy::U16,
            "u32" => IntTy::U32,
            "u64" => IntTy::U64,
            "u128" => IntTy::U128,
            "usize" => IntTy::Usize,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FloatTy {
    F32,
    F64,
}

impl FloatTy {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            FloatTy::F32 => "f32",
            FloatTy::F64 => "f64",
        }
    }
}

/// Tham số kiểu: `owner` là NodeId của item/thành viên khai báo, `index` là thứ tự trong danh sách generics của nó.
/// `index == SELF_INDEX` là `Self` của một `⊏`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParamId {
    pub owner: NodeId,
    pub index: u32,
}

pub const SELF_INDEX: u32 = u32::MAX;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Ty {
    Int(IntTy),
    Float(FloatTy),
    Bool,
    Char,
    Str,
    /// Kiểu đáy `!` (ADR-018 §3).
    Never,
    /// `()` là `Tuple(vec![])`.
    Tuple(Vec<Ty>),
    Array(Box<Ty>, u64),
    Slice(Box<Ty>),
    /// (có thể thay đổi?, kiểu trỏ tới). Lifetime/region chưa nằm trong kiểu ở M4 (M4.5/M6).
    Ref(bool, Box<Ty>),
    RawPtr(bool, Box<Ty>),
    /// struct/enum cùng tham số kiểu.
    Adt(DefId, Vec<Ty>),
    /// Kiểu hàm: tham số, kết quả, hiệu ứng (đã sắp, tên).
    Fn(Vec<Ty>, Box<Ty>, Vec<String>),
    Param(ParamId),
    Infer(u32),
    /// Đã báo lỗi; không sinh lỗi dây chuyền.
    Err,
}

impl Ty {
    #[must_use]
    pub fn unit() -> Ty {
        Ty::Tuple(Vec::new())
    }

    #[must_use]
    pub fn is_unit(&self) -> bool {
        matches!(self, Ty::Tuple(v) if v.is_empty())
    }

    #[must_use]
    pub fn from_prim(p: PrimType) -> Ty {
        use PrimType::*;
        match p {
            I8 => Ty::Int(IntTy::I8),
            I16 => Ty::Int(IntTy::I16),
            I32 => Ty::Int(IntTy::I32),
            I64 => Ty::Int(IntTy::I64),
            I128 => Ty::Int(IntTy::I128),
            U8 => Ty::Int(IntTy::U8),
            U16 => Ty::Int(IntTy::U16),
            U32 => Ty::Int(IntTy::U32),
            U64 => Ty::Int(IntTy::U64),
            U128 => Ty::Int(IntTy::U128),
            Isize => Ty::Int(IntTy::Isize),
            Usize => Ty::Int(IntTy::Usize),
            F32 => Ty::Float(FloatTy::F32),
            F64 => Ty::Float(FloatTy::F64),
            Bool => Ty::Bool,
            Char => Ty::Char,
            Str => Ty::Str,
        }
    }

    /// Có chứa biến suy luận chưa giải?
    #[must_use]
    pub fn has_infer(&self) -> bool {
        match self {
            Ty::Infer(_) => true,
            Ty::Tuple(v) | Ty::Adt(_, v) => v.iter().any(Ty::has_infer),
            Ty::Array(t, _) | Ty::Slice(t) | Ty::Ref(_, t) | Ty::RawPtr(_, t) => t.has_infer(),
            Ty::Fn(p, r, _) => p.iter().any(Ty::has_infer) || r.has_infer(),
            _ => false,
        }
    }

    /// Có chứa tham số kiểu?
    #[must_use]
    pub fn has_param(&self) -> bool {
        match self {
            Ty::Param(_) => true,
            Ty::Tuple(v) | Ty::Adt(_, v) => v.iter().any(Ty::has_param),
            Ty::Array(t, _) | Ty::Slice(t) | Ty::Ref(_, t) | Ty::RawPtr(_, t) => t.has_param(),
            Ty::Fn(p, r, _) => p.iter().any(Ty::has_param) || r.has_param(),
            _ => false,
        }
    }

    /// Có nhắc tới ADT `d` (ở bất kỳ độ sâu nào của cú pháp kiểu)?
    #[must_use]
    pub fn mentions_adt(&self, d: gt_resolve::DefId) -> bool {
        match self {
            Ty::Adt(x, v) => *x == d || v.iter().any(|t| t.mentions_adt(d)),
            Ty::Tuple(v) => v.iter().any(|t| t.mentions_adt(d)),
            Ty::Array(t, _) | Ty::Slice(t) | Ty::Ref(_, t) | Ty::RawPtr(_, t) => t.mentions_adt(d),
            Ty::Fn(p, r, _) => p.iter().any(|t| t.mentions_adt(d)) || r.mentions_adt(d),
            _ => false,
        }
    }

    /// Độ lồng của kiểu (dùng cho ngưỡng đệ quy đa hình, ADR-018 §1).
    #[must_use]
    pub fn depth(&self) -> u32 {
        match self {
            Ty::Tuple(v) | Ty::Adt(_, v) => 1 + v.iter().map(Ty::depth).max().unwrap_or(0),
            Ty::Array(t, _) | Ty::Slice(t) | Ty::Ref(_, t) | Ty::RawPtr(_, t) => 1 + t.depth(),
            Ty::Fn(p, r, _) => {
                1 + p
                    .iter()
                    .map(Ty::depth)
                    .chain([r.depth()])
                    .max()
                    .unwrap_or(0)
            }
            _ => 0,
        }
    }

    /// Thay `Param` theo `map` (tra theo ParamId).
    #[must_use]
    pub fn subst(&self, map: &dyn Fn(ParamId) -> Option<Ty>) -> Ty {
        match self {
            Ty::Param(p) => map(*p).unwrap_or_else(|| self.clone()),
            Ty::Tuple(v) => Ty::Tuple(v.iter().map(|t| t.subst(map)).collect()),
            Ty::Adt(d, v) => Ty::Adt(*d, v.iter().map(|t| t.subst(map)).collect()),
            Ty::Array(t, n) => Ty::Array(Box::new(t.subst(map)), *n),
            Ty::Slice(t) => Ty::Slice(Box::new(t.subst(map))),
            Ty::Ref(m, t) => Ty::Ref(*m, Box::new(t.subst(map))),
            Ty::RawPtr(m, t) => Ty::RawPtr(*m, Box::new(t.subst(map))),
            Ty::Fn(p, r, e) => Ty::Fn(
                p.iter().map(|t| t.subst(map)).collect(),
                Box::new(r.subst(map)),
                e.clone(),
            ),
            _ => self.clone(),
        }
    }

    /// Viết kiểu cho người đọc (tên ADT lấy từ bảng định nghĩa; chỉ phần cuối của đường dẫn).
    #[must_use]
    pub fn render(&self, defs: &[Def]) -> String {
        self.render_with(defs, false)
    }

    /// Dạng CHUẨN không nhập nhằng (đường dẫn đầy đủ của ADT): chỉ để băm khoá instance (ADR-032 phụ lục A, Q69).
    #[must_use]
    pub fn canon(&self, defs: &[Def]) -> String {
        self.render_with(defs, true)
    }

    fn render_with(&self, defs: &[Def], full: bool) -> String {
        let name = |d: &DefId| -> String {
            defs.get(d.index())
                .map(|x| {
                    if full {
                        x.path.clone()
                    } else {
                        x.path.rsplit("::").next().unwrap_or(&x.path).to_string()
                    }
                })
                .unwrap_or_else(|| "?".to_string())
        };
        match self {
            Ty::Int(i) => i.name().to_string(),
            Ty::Float(f) => f.name().to_string(),
            Ty::Bool => "bool".to_string(),
            Ty::Char => "char".to_string(),
            Ty::Str => "str".to_string(),
            Ty::Never => "!".to_string(),
            Ty::Tuple(v) if v.is_empty() => "()".to_string(),
            Ty::Tuple(v) if v.len() == 1 => format!("({},)", v[0].render_with(defs, full)),
            Ty::Tuple(v) => format!(
                "({})",
                v.iter()
                    .map(|t| t.render_with(defs, full))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Ty::Array(t, n) => format!("[{}; {n}]", t.render_with(defs, full)),
            Ty::Slice(t) => format!("[{}]", t.render_with(defs, full)),
            Ty::Ref(false, t) => format!("&{}", t.render_with(defs, full)),
            Ty::Ref(true, t) => format!("&!{}", t.render_with(defs, full)),
            Ty::RawPtr(false, t) => format!("*□ {}", t.render_with(defs, full)),
            Ty::RawPtr(true, t) => format!("*μ {}", t.render_with(defs, full)),
            Ty::Adt(d, a) if a.is_empty() => name(d),
            Ty::Adt(d, a) => format!(
                "{}<{}>",
                name(d),
                a.iter()
                    .map(|t| t.render_with(defs, full))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Ty::Fn(p, r, e) => {
                let eff = if e.is_empty() {
                    "∅".to_string()
                } else {
                    format!("{{{}}}", e.join(", "))
                };
                format!(
                    "ƒ({}) → {} ! {eff}",
                    p.iter()
                        .map(|t| t.render_with(defs, full))
                        .collect::<Vec<_>>()
                        .join(", "),
                    r.render_with(defs, full)
                )
            }
            Ty::Param(p) if p.index == SELF_INDEX => "Self".to_string(),
            Ty::Param(p) => format!("T{}", p.index),
            Ty::Infer(_) => "_".to_string(),
            Ty::Err => "{lỗi}".to_string(),
        }
    }
}
