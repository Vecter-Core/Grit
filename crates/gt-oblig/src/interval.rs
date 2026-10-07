//! Miền khoảng trên số nguyên TOÁN HỌC (ADR-025 §4), tham số hoá theo độ rộng bit (`Ty`) để test vét cạn ở 2..8 bit rồi tin công thức
//! cho 16..128. Cận là `I256` hoặc ±∞; mọi tràn trong CHÍNH bộ phân tích đẩy cận lên ±∞ theo hướng làm khoảng RỘNG hơn (sound),
//! không bao giờ quấn vòng. Cận dưới làm tròn xuống (−∞ khi không biểu diễn được), cận trên làm tròn lên (+∞).
use crate::int::I256;
use std::cmp::Ordering;

/// Kiểu số nguyên trừu tượng: độ rộng 1..=128 bit, có/không dấu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ty {
    pub bits: u32,
    pub signed: bool,
}

impl Ty {
    #[must_use]
    pub fn new(bits: u32, signed: bool) -> Ty {
        Ty { bits, signed }
    }

    #[must_use]
    pub fn min(self) -> I256 {
        if self.signed {
            // −2^(bits−1)
            I256::pow2(self.bits - 1).map_or(I256::ZERO, I256::negate)
        } else {
            I256::ZERO
        }
    }

    #[must_use]
    pub fn max(self) -> I256 {
        let p = if self.signed {
            self.bits - 1
        } else {
            self.bits
        };
        // 2^p − 1
        I256::pow2(p)
            .and_then(|v| v.checked_sub(I256::from_i128(1)))
            .unwrap_or(I256::ZERO)
    }

    /// Chu kỳ `2^bits`.
    #[must_use]
    pub fn period(self) -> I256 {
        I256::pow2(self.bits).unwrap_or(I256::ZERO)
    }

    #[must_use]
    pub fn range(self) -> Interval {
        Interval {
            lo: B::Fin(self.min()),
            hi: B::Fin(self.max()),
        }
    }

    #[must_use]
    pub fn name(self) -> String {
        format!("{}{}", if self.signed { 'i' } else { 'u' }, self.bits)
    }
}

/// Cận mở rộng.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum B {
    NegInf,
    Fin(I256),
    PosInf,
}

impl PartialOrd for B {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for B {
    fn cmp(&self, o: &Self) -> Ordering {
        match (self, o) {
            (B::NegInf, B::NegInf) | (B::PosInf, B::PosInf) => Ordering::Equal,
            (B::NegInf, _) | (_, B::PosInf) => Ordering::Less,
            (_, B::NegInf) | (B::PosInf, _) => Ordering::Greater,
            (B::Fin(a), B::Fin(b)) => a.cmp(b),
        }
    }
}

impl B {
    #[must_use]
    pub fn from_i128(v: i128) -> B {
        B::Fin(I256::from_i128(v))
    }

    #[must_use]
    pub fn fin(self) -> Option<I256> {
        match self {
            B::Fin(v) => Some(v),
            _ => None,
        }
    }

    fn neg(self) -> B {
        match self {
            B::NegInf => B::PosInf,
            B::PosInf => B::NegInf,
            B::Fin(v) => B::Fin(v.negate()),
        }
    }

    /// Cộng làm tròn XUỐNG (cận dưới): bất cứ thứ gì không biểu diễn được ⇒ −∞.
    fn add_lo(a: B, b: B) -> B {
        match (a, b) {
            (B::Fin(x), B::Fin(y)) => x.checked_add(y).map_or(B::NegInf, B::Fin),
            _ => B::NegInf,
        }
    }

    /// Cộng làm tròn LÊN (cận trên).
    fn add_hi(a: B, b: B) -> B {
        match (a, b) {
            (B::Fin(x), B::Fin(y)) => x.checked_add(y).map_or(B::PosInf, B::Fin),
            _ => B::PosInf,
        }
    }

    /// Tích chính xác nếu biểu diễn được (`0·∞ = 0`); `None` nếu tràn I256.
    fn mul_exact(a: B, b: B) -> Option<B> {
        let sign = |x: B| match x {
            B::NegInf => -1,
            B::PosInf => 1,
            B::Fin(v) => {
                if v.is_zero() {
                    0
                } else if v.is_neg() {
                    -1
                } else {
                    1
                }
            }
        };
        match (a, b) {
            (B::Fin(x), B::Fin(y)) => x.checked_mul(y).map(B::Fin),
            _ => {
                let s = sign(a) * sign(b);
                Some(match s {
                    0 => B::Fin(I256::ZERO),
                    1 => B::PosInf,
                    _ => B::NegInf,
                })
            }
        }
    }
}

/// Khoảng KHÔNG RỖNG `[lo, hi]` (rỗng = unreachable, biểu diễn bằng `Option` ở nơi dùng).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Interval {
    pub lo: B,
    pub hi: B,
}

/// Giá trị chân lý ba trị.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tri {
    Yes,
    No,
    Maybe,
}

impl Interval {
    #[must_use]
    pub fn new(lo: B, hi: B) -> Interval {
        debug_assert!(lo <= hi);
        Interval { lo, hi }
    }

    #[must_use]
    pub fn point(v: I256) -> Interval {
        Interval {
            lo: B::Fin(v),
            hi: B::Fin(v),
        }
    }

    #[must_use]
    pub fn top() -> Interval {
        Interval {
            lo: B::NegInf,
            hi: B::PosInf,
        }
    }

    #[must_use]
    pub fn as_point(self) -> Option<I256> {
        match (self.lo, self.hi) {
            (B::Fin(a), B::Fin(b)) if a == b => Some(a),
            _ => None,
        }
    }

    #[must_use]
    pub fn contains(self, v: I256) -> bool {
        self.lo <= B::Fin(v) && B::Fin(v) <= self.hi
    }

    #[must_use]
    pub fn contains_zero(self) -> bool {
        self.contains(I256::ZERO)
    }

    #[must_use]
    pub fn join(self, o: Interval) -> Interval {
        Interval {
            lo: self.lo.min(o.lo),
            hi: self.hi.max(o.hi),
        }
    }

    #[must_use]
    pub fn meet(self, o: Interval) -> Option<Interval> {
        let lo = self.lo.max(o.lo);
        let hi = self.hi.min(o.hi);
        if lo <= hi {
            Some(Interval { lo, hi })
        } else {
            None
        }
    }

    /// `self ⊆ o`.
    #[must_use]
    pub fn subset_of(self, o: Interval) -> bool {
        o.lo <= self.lo && self.hi <= o.hi
    }

    /// Widening với ngưỡng (`thresholds` tăng dần): cận không ổn định nhảy tới ngưỡng kế tiếp hoặc ±∞.
    #[must_use]
    pub fn widen(self, new: Interval, thresholds: &[I256]) -> Interval {
        let lo = if new.lo < self.lo {
            thresholds
                .iter()
                .rev()
                .map(|t| B::Fin(*t))
                .find(|t| *t <= new.lo)
                .unwrap_or(B::NegInf)
        } else {
            self.lo
        };
        let hi = if new.hi > self.hi {
            thresholds
                .iter()
                .map(|t| B::Fin(*t))
                .find(|t| *t >= new.hi)
                .unwrap_or(B::PosInf)
        } else {
            self.hi
        };
        Interval { lo, hi }
    }

    #[must_use]
    pub fn neg(self) -> Interval {
        Interval {
            lo: self.hi.neg(),
            hi: self.lo.neg(),
        }
    }

    #[must_use]
    pub fn add(self, o: Interval) -> Interval {
        Interval {
            lo: B::add_lo(self.lo, o.lo),
            hi: B::add_hi(self.hi, o.hi),
        }
    }

    #[must_use]
    pub fn sub(self, o: Interval) -> Interval {
        self.add(o.neg())
    }

    #[must_use]
    pub fn mul(self, o: Interval) -> Interval {
        let corners = [
            (self.lo, o.lo),
            (self.lo, o.hi),
            (self.hi, o.lo),
            (self.hi, o.hi),
        ];
        let mut lo = B::PosInf;
        let mut hi = B::NegInf;
        for (a, b) in corners {
            match B::mul_exact(a, b) {
                Some(p) => {
                    lo = lo.min(p);
                    hi = hi.max(p);
                }
                None => {
                    // tràn I256: dấu của tích quyết định hướng; cận dưới ⇒ −∞, cận trên ⇒ +∞ (luôn sound)
                    lo = B::NegInf;
                    hi = B::PosInf;
                }
            }
        }
        // cận dưới không bao giờ là +∞ (khoảng hợp lệ): làm tròn xuống
        if lo == B::PosInf {
            lo = B::NegInf;
        }
        if hi == B::NegInf {
            hi = B::PosInf;
        }
        Interval { lo, hi }
    }

    fn div_one_sign(self, d: Interval) -> Interval {
        // d không chứa 0 và cùng dấu; cả hai cận hữu hạn mới tính chính xác
        let (Some(al), Some(ah), Some(dl), Some(dh)) =
            (self.lo.fin(), self.hi.fin(), d.lo.fin(), d.hi.fin())
        else {
            return Interval::top();
        };
        let mut lo = None;
        let mut hi = None;
        for (a, b) in [(al, dl), (al, dh), (ah, dl), (ah, dh)] {
            let Some(q) = a.div_trunc(b) else {
                return Interval::top();
            };
            lo = Some(lo.map_or(q, |x: I256| x.min(q)));
            hi = Some(hi.map_or(q, |x: I256| x.max(q)));
        }
        match (lo, hi) {
            (Some(l), Some(h)) => Interval {
                lo: B::Fin(l),
                hi: B::Fin(h),
            },
            _ => Interval::top(),
        }
    }

    /// Chia cắt về 0 với `d ≠ 0` (đã được kiểm bằng nghĩa vụ chia 0 đi trước): tách `d` thành phần âm và phần dương.
    #[must_use]
    pub fn div_trunc_nz(self, d: Interval) -> Option<Interval> {
        let one = I256::from_i128(1);
        let neg_part = d.meet(Interval {
            lo: B::NegInf,
            hi: B::Fin(one.negate()),
        });
        let pos_part = d.meet(Interval {
            lo: B::Fin(one),
            hi: B::PosInf,
        });
        let mut out: Option<Interval> = None;
        for part in [neg_part, pos_part].into_iter().flatten() {
            let q = self.div_one_sign(part);
            out = Some(out.map_or(q, |o| o.join(q)));
        }
        out
    }

    /// Dư theo chia cắt về 0 với `d ≠ 0`: dấu theo số bị chia, `|r| < |d|`.
    #[must_use]
    pub fn rem_trunc_nz(self, d: Interval) -> Option<Interval> {
        // |d| lớn nhất và nhỏ nhất (d ≠ 0)
        let one = I256::from_i128(1);
        let abs_hi = |b: B| match b {
            B::Fin(v) => B::Fin(v.abs()),
            _ => B::PosInf,
        };
        let neg_part = d.meet(Interval {
            lo: B::NegInf,
            hi: B::Fin(one.negate()),
        });
        let pos_part = d.meet(Interval {
            lo: B::Fin(one),
            hi: B::PosInf,
        });
        let mut dmax = B::NegInf;
        let mut dmin = B::PosInf;
        for p in [neg_part, pos_part].into_iter().flatten() {
            let a = abs_hi(p.lo);
            let b = abs_hi(p.hi);
            dmax = dmax.max(a.max(b));
            dmin = dmin.min(a.min(b));
        }
        if dmax == B::NegInf {
            return None;
        }
        // |r| ≤ dmax − 1
        let bound = match dmax {
            B::Fin(m) => B::Fin(m.checked_sub(one)?),
            _ => B::PosInf,
        };
        let nbound = bound.neg();
        // a ≥ 0 và a < dmin ⇒ r = a
        if self.lo >= B::Fin(I256::ZERO) && self.hi < dmin {
            return Some(self);
        }
        let lo = if self.lo >= B::Fin(I256::ZERO) {
            B::Fin(I256::ZERO)
        } else {
            self.lo.max(nbound)
        };
        let hi = if self.hi <= B::Fin(I256::ZERO) {
            B::Fin(I256::ZERO)
        } else {
            self.hi.min(bound)
        };
        Some(Interval { lo, hi })
    }

    // ---------------------------------------------------------------- liên hệ với kiểu

    /// Khoảng toán học nằm trọn trong kiểu / rời hẳn / chưa biết.
    #[must_use]
    pub fn fits(self, ty: Ty) -> Tri {
        let r = ty.range();
        if self.subset_of(r) {
            Tri::Yes
        } else if self.meet(r).is_none() {
            Tri::No
        } else {
            Tri::Maybe
        }
    }

    /// Giao với kiểu (đường tràn đã abort nên giá trị tiếp tục luôn nằm trong kiểu). `None` ⇒ không có giá trị nào tiếp tục.
    #[must_use]
    pub fn clamp(self, ty: Ty) -> Option<Interval> {
        self.meet(ty.range())
    }

    /// `+% -% *%`: kết quả toán học `self` quấn vào kiểu. Nằm trọn một chu kỳ ⇒ dịch chính xác; ngược lại toàn kiểu.
    #[must_use]
    pub fn wrap(self, ty: Ty) -> Interval {
        let (B::Fin(lo), B::Fin(hi)) = (self.lo, self.hi) else {
            return ty.range();
        };
        let p = ty.period();
        let (min, max) = (ty.min(), ty.max());
        // k = ⌊(lo − min) / p⌋ (chia xuống); dịch về [min, max]
        let Some(off) = lo.checked_sub(min) else {
            return ty.range();
        };
        let Some(q) = off.div_trunc(p) else {
            return ty.range();
        };
        // với off âm, chia cắt về 0 làm tròn lên; chỉnh về làm tròn xuống
        let k = match off.rem_trunc(p) {
            Some(r) if off.is_neg() && !r.is_zero() => match q.checked_sub(I256::from_i128(1)) {
                Some(v) => v,
                None => return ty.range(),
            },
            _ => q,
        };
        let Some(shift) = k.checked_mul(p) else {
            return ty.range();
        };
        let (Some(nlo), Some(nhi)) = (lo.checked_sub(shift), hi.checked_sub(shift)) else {
            return ty.range();
        };
        if nlo >= min && nhi <= max {
            Interval {
                lo: B::Fin(nlo),
                hi: B::Fin(nhi),
            }
        } else {
            ty.range()
        }
    }

    /// `+^ -^ *^`: bão hoà — kẹp từng cận vào kiểu (chính xác vì hàm đơn điệu).
    #[must_use]
    pub fn saturate(self, ty: Ty) -> Interval {
        let (min, max) = (B::Fin(ty.min()), B::Fin(ty.max()));
        Interval {
            lo: self.lo.max(min).min(max),
            hi: self.hi.max(min).min(max),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iv(a: i128, b: i128) -> Interval {
        Interval::new(B::from_i128(a), B::from_i128(b))
    }

    #[test]
    fn type_bounds_are_exact_for_every_width() {
        assert_eq!(Ty::new(8, true).min(), I256::from_i128(-128));
        assert_eq!(Ty::new(8, true).max(), I256::from_i128(127));
        assert_eq!(Ty::new(8, false).max(), I256::from_i128(255));
        assert_eq!(Ty::new(128, false).max(), I256::from_u128(u128::MAX));
        assert_eq!(Ty::new(128, true).min(), I256::from_i128(i128::MIN));
        assert_eq!(Ty::new(128, true).max(), I256::from_i128(i128::MAX));
        assert_eq!(
            Ty::new(64, false).max(),
            I256::from_u128(u128::from(u64::MAX))
        );
        assert_eq!(Ty::new(1, true).min(), I256::from_i128(-1));
    }

    #[test]
    fn arithmetic_never_wraps_silently() {
        // u128::MAX + u128::MAX vẫn là số toán học chính xác, không quấn vòng
        let m = Interval::point(I256::from_u128(u128::MAX));
        let s = m.add(m);
        assert!(s.lo.fin().is_some() && s.lo == s.hi);
        assert_eq!(s.fits(Ty::new(128, false)), Tri::No);
        // tích 128 bit đầy đủ vừa I256
        let p = m.mul(m);
        assert!(p.as_point().is_some());
        // tràn I256: đẩy lên ±∞, không quấn
        let big = Interval::point(I256::pow2(254).expect("2^254"));
        let q = big.mul(big);
        assert_eq!((q.lo, q.hi), (B::NegInf, B::PosInf));
    }

    #[test]
    fn wrap_shifts_exactly_within_a_period_and_goes_full_otherwise() {
        let t = Ty::new(8, false);
        assert_eq!(iv(250, 260).wrap(t), iv(0, 255)); // vắt qua biên chu kỳ ⇒ toàn kiểu
        assert_eq!(iv(256, 260).wrap(t), iv(0, 4));
        assert_eq!(iv(-3, -1).wrap(t), iv(253, 255));
        assert_eq!(iv(10, 20).wrap(t), iv(10, 20));
    }
}
