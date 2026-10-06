//! Vét cạn `⌥` (D6, E0305) — thuật toán ma trận pattern (Maranget) trên IR pattern đã kiểm kiểu.
//! Số nguyên và `char` dùng phân tách khoảng (nên `..0 ↦`, `0..=9 ↦`, `10.. ↦` phủ hết `i32`); `str`, số thực và mọi miền vô hạn
//! khác cần nhánh `_`/tên (ADR-018 §5). Literal không đóng góp vào vét cạn trừ khi lấp đủ miền.
use crate::collect::{AdtKind, Env};
use crate::ty::{IntTy, Ty};

#[derive(Clone, Debug)]
pub enum Pat {
    Wild,
    /// Biến thể của enum: (chỉ số biến thể, các trường con).
    Variant(usize, Vec<Pat>),
    Bool(bool),
    /// Tuple hoặc struct: một hàm tạo duy nhất, các thành phần theo thứ tự.
    Single(Vec<Pat>),
    /// Khoảng kín `[lo, hi]` trên số nguyên hoặc điểm mã của `char`.
    Range(i128, i128),
    /// Literal thuộc miền vô hạn (`str`, số thực): không bao giờ vét cạn.
    Opaque,
}

#[derive(Clone, Debug, PartialEq)]
enum Ctor {
    Variant(usize),
    Bool(bool),
    Single,
    Range(i128, i128),
}

fn int_domain(ty: &Ty) -> Option<(i128, i128)> {
    match ty {
        Ty::Int(it) => Some((it.min_val(), clamp_u128(it.max_val()))),
        Ty::Char => Some((0, 0x10FFFF)),
        _ => None,
    }
}

fn clamp_u128(v: u128) -> i128 {
    i128::try_from(v).unwrap_or(i128::MAX)
}

impl Env<'_> {
    /// Danh sách ctor đầy đủ của miền hữu hạn; `None` nếu miền vô hạn (chỉ nhánh bắt-mọi-thứ mới phủ).
    fn all_ctors(&self, ty: &Ty, col: &[&Pat]) -> Option<Vec<Ctor>> {
        match ty {
            Ty::Bool => Some(vec![Ctor::Bool(false), Ctor::Bool(true)]),
            Ty::Tuple(_) => Some(vec![Ctor::Single]),
            Ty::Adt(d, _) => match self.adts.get(d).map(|a| &a.kind) {
                Some(AdtKind::Struct(_)) => Some(vec![Ctor::Single]),
                Some(AdtKind::Enum(vs)) => Some((0..vs.len()).map(Ctor::Variant).collect()),
                None => None,
            },
            Ty::Int(_) | Ty::Char => {
                let (min, max) = int_domain(ty)?;
                let mut cuts: Vec<i128> = vec![min];
                for p in col {
                    if let Pat::Range(lo, hi) = p {
                        cuts.push((*lo).clamp(min, max));
                        if *hi < max {
                            cuts.push((*hi + 1).clamp(min, max));
                        }
                    }
                }
                cuts.sort_unstable();
                cuts.dedup();
                let mut out = Vec::new();
                for (i, &lo) in cuts.iter().enumerate() {
                    let hi = cuts.get(i + 1).map_or(max, |n| *n - 1);
                    if lo <= hi {
                        out.push(Ctor::Range(lo, hi));
                    }
                }
                Some(out)
            }
            _ => None,
        }
    }

    fn sub_tys(&self, ty: &Ty, c: &Ctor) -> Vec<Ty> {
        match (ty, c) {
            (Ty::Tuple(v), Ctor::Single) => v.clone(),
            (Ty::Adt(d, args), Ctor::Single) => {
                match self.adts.get(d).map(|a| (&a.kind, &a.generics)) {
                    Some((AdtKind::Struct(fs), gens)) => fs
                        .iter()
                        .map(|f| crate::traits::subst_with(&f.ty, gens, args))
                        .collect(),
                    _ => Vec::new(),
                }
            }
            (Ty::Adt(d, args), Ctor::Variant(i)) => {
                match self.adts.get(d).map(|a| (&a.kind, &a.generics)) {
                    Some((AdtKind::Enum(vs), gens)) => vs
                        .get(*i)
                        .map(|v| {
                            v.fields
                                .iter()
                                .map(|f| crate::traits::subst_with(f, gens, args))
                                .collect()
                        })
                        .unwrap_or_default(),
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    fn describe(&self, ty: &Ty, c: &Ctor) -> String {
        match (ty, c) {
            (_, Ctor::Bool(b)) => b.to_string(),
            (Ty::Adt(d, _), Ctor::Variant(i)) => match self.adts.get(d).map(|a| &a.kind) {
                Some(AdtKind::Enum(vs)) => {
                    let v = &vs[*i];
                    if v.fields.is_empty() {
                        v.name.clone()
                    } else {
                        format!("{}({})", v.name, vec!["_"; v.fields.len()].join(", "))
                    }
                }
                _ => "_".to_string(),
            },
            (Ty::Char, Ctor::Range(lo, hi)) => {
                let f = |v: i128| {
                    u32::try_from(v)
                        .ok()
                        .and_then(char::from_u32)
                        .map_or("?".to_string(), |c| format!("{c:?}"))
                };
                if lo == hi {
                    f(*lo)
                } else {
                    format!("{}..={}", f(*lo), f(*hi))
                }
            }
            (Ty::Int(it), Ctor::Range(lo, hi)) => {
                let (min, max) = (it.min_val(), clamp_u128(it.max_val()));
                match (*lo == min, *hi == max) {
                    (true, true) => "_".to_string(),
                    (true, false) => format!("..={hi}"),
                    (false, true) => format!("{lo}.."),
                    (false, false) if lo == hi => lo.to_string(),
                    (false, false) => format!("{lo}..={hi}"),
                }
            }
            _ => "_".to_string(),
        }
    }

    fn specialize(rows: &[Vec<Pat>], c: &Ctor, arity: usize) -> Vec<Vec<Pat>> {
        let mut out = Vec::new();
        for r in rows {
            let (head, rest) = (&r[0], &r[1..]);
            let sub: Option<Vec<Pat>> = match (head, c) {
                (Pat::Wild, _) => Some(vec![Pat::Wild; arity]),
                (Pat::Variant(i, args), Ctor::Variant(j)) if i == j => Some(args.clone()),
                (Pat::Bool(a), Ctor::Bool(b)) if a == b => Some(Vec::new()),
                (Pat::Single(args), Ctor::Single) => Some(args.clone()),
                (Pat::Range(lo, hi), Ctor::Range(clo, chi)) if lo <= clo && chi <= hi => {
                    Some(Vec::new())
                }
                _ => None,
            };
            if let Some(mut s) = sub {
                s.resize(arity, Pat::Wild);
                s.extend(rest.iter().cloned());
                out.push(s);
            }
        }
        out
    }

    /// Có giá trị nào KHÔNG được ma trận phủ? Trả phản ví dụ (mỗi cột một mô tả).
    fn missing(&self, rows: &[Vec<Pat>], tys: &[Ty]) -> Option<Vec<String>> {
        let Some((t0, rest_tys)) = tys.split_first() else {
            return rows.is_empty().then(Vec::new);
        };
        let col: Vec<&Pat> = rows.iter().map(|r| &r[0]).collect();
        match self.all_ctors(t0, &col) {
            Some(ctors) => {
                for c in ctors {
                    let sub = self.sub_tys(t0, &c);
                    let spec = Self::specialize(rows, &c, sub.len());
                    let mut next_tys = sub.clone();
                    next_tys.extend(rest_tys.iter().cloned());
                    if let Some(mut w) = self.missing(&spec, &next_tys) {
                        let rest_w = w.split_off(sub.len());
                        let head = self.describe(t0, &c);
                        let mut out = vec![head];
                        out.extend(rest_w);
                        return Some(out);
                    }
                }
                None
            }
            None => {
                // miền vô hạn: chỉ hàng bắt-mọi-thứ phủ được
                let default: Vec<Vec<Pat>> = rows
                    .iter()
                    .filter(|r| matches!(r[0], Pat::Wild))
                    .map(|r| r[1..].to_vec())
                    .collect();
                self.missing(&default, rest_tys).map(|mut w| {
                    w.insert(0, "_".to_string());
                    w
                })
            }
        }
    }

    /// `None` nếu vét cạn; `Some(mô tả)` của một giá trị chưa được phủ.
    pub(crate) fn match_missing(&self, scrut: &Ty, arms: &[Pat]) -> Option<String> {
        let rows: Vec<Vec<Pat>> = arms.iter().map(|p| vec![p.clone()]).collect();
        self.missing(&rows, std::slice::from_ref(scrut))
            .map(|w| w.join(", "))
    }
}

#[allow(dead_code)]
pub(crate) fn int_range_of(ty: IntTy) -> (i128, i128) {
    (ty.min_val(), clamp_u128(ty.max_val()))
}
