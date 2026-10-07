//! Số nguyên có dấu 256 bit dạng dấu–độ lớn (ADR-025 §4): đủ chứa CHÍNH XÁC tích hai giá trị 128 bit (kể cả `u128::MAX`), nên cận
//! của miền khoảng không bao giờ phải "quấn vòng". Mọi phép toán đều KIỂM TRÀN: tràn ⇒ trả `None`, người gọi đẩy cận lên ±∞
//! ("lên đỉnh"), không bao giờ quấn vòng im lặng. Tự cài, không phụ thuộc crate ngoài (Q41).
use std::cmp::Ordering;

/// `neg` ⇒ giá trị âm; bất biến: không âm khi bằng 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct I256 {
    neg: bool,
    hi: u128,
    lo: u128,
}

fn cmp_mag(a: (u128, u128), b: (u128, u128)) -> Ordering {
    a.0.cmp(&b.0).then(a.1.cmp(&b.1))
}

fn add_mag(a: (u128, u128), b: (u128, u128)) -> Option<(u128, u128)> {
    let (lo, c) = a.1.overflowing_add(b.1);
    let hi = a.0.checked_add(b.0)?.checked_add(u128::from(c))?;
    Some((hi, lo))
}

/// `a ≥ b` bắt buộc.
fn sub_mag(a: (u128, u128), b: (u128, u128)) -> (u128, u128) {
    let (lo, bo) = a.1.overflowing_sub(b.1);
    let hi = a.0 - b.0 - u128::from(bo);
    (hi, lo)
}

/// Tích 128×128 → 256 bit chính xác.
fn mul_wide(a: u128, b: u128) -> (u128, u128) {
    let (a0, a1) = (a & u128::from(u64::MAX), a >> 64);
    let (b0, b1) = (b & u128::from(u64::MAX), b >> 64);
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let mid = (p00 >> 64) + (p01 & u128::from(u64::MAX)) + (p10 & u128::from(u64::MAX));
    let lo = (p00 & u128::from(u64::MAX)) | (mid << 64);
    let hi = p11 + (p01 >> 64) + (p10 >> 64) + (mid >> 64);
    (hi, lo)
}

impl I256 {
    pub const ZERO: I256 = I256 {
        neg: false,
        hi: 0,
        lo: 0,
    };

    fn make(neg: bool, hi: u128, lo: u128) -> I256 {
        I256 {
            neg: neg && (hi != 0 || lo != 0),
            hi,
            lo,
        }
    }

    #[must_use]
    pub fn from_i128(v: i128) -> I256 {
        I256::make(v < 0, 0, v.unsigned_abs())
    }

    #[must_use]
    pub fn from_u128(v: u128) -> I256 {
        I256::make(false, 0, v)
    }

    #[must_use]
    pub fn to_i128(self) -> Option<i128> {
        if self.hi != 0 {
            return None;
        }
        if self.neg {
            if self.lo <= 1u128 << 127 {
                Some((self.lo as i128).wrapping_neg())
            } else {
                None
            }
        } else {
            i128::try_from(self.lo).ok()
        }
    }

    #[must_use]
    pub fn to_u128(self) -> Option<u128> {
        if self.hi == 0 && !self.neg {
            Some(self.lo)
        } else {
            None
        }
    }

    #[must_use]
    pub fn is_neg(self) -> bool {
        self.neg
    }

    #[must_use]
    pub fn is_zero(self) -> bool {
        self.hi == 0 && self.lo == 0
    }

    #[must_use]
    pub fn negate(self) -> I256 {
        I256::make(!self.neg, self.hi, self.lo)
    }

    #[must_use]
    pub fn abs(self) -> I256 {
        I256::make(false, self.hi, self.lo)
    }

    #[must_use]
    pub fn checked_add(self, o: I256) -> Option<I256> {
        let (a, b) = ((self.hi, self.lo), (o.hi, o.lo));
        if self.neg == o.neg {
            let (h, l) = add_mag(a, b)?;
            Some(I256::make(self.neg, h, l))
        } else {
            match cmp_mag(a, b) {
                Ordering::Equal => Some(I256::ZERO),
                Ordering::Greater => {
                    let (h, l) = sub_mag(a, b);
                    Some(I256::make(self.neg, h, l))
                }
                Ordering::Less => {
                    let (h, l) = sub_mag(b, a);
                    Some(I256::make(o.neg, h, l))
                }
            }
        }
    }

    #[must_use]
    pub fn checked_sub(self, o: I256) -> Option<I256> {
        self.checked_add(o.negate())
    }

    #[must_use]
    pub fn checked_mul(self, o: I256) -> Option<I256> {
        let neg = self.neg != o.neg;
        let (h, l) = match (self.hi, o.hi) {
            (0, 0) => mul_wide(self.lo, o.lo),
            (0, _) => {
                // o.hi != 0 ⇒ chỉ vừa khi self.lo ≤ 1 (self.hi = 0)
                if self.lo == 0 {
                    (0, 0)
                } else if self.lo == 1 {
                    (o.hi, o.lo)
                } else {
                    return None;
                }
            }
            (_, 0) => {
                if o.lo == 0 {
                    (0, 0)
                } else if o.lo == 1 {
                    (self.hi, self.lo)
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        Some(I256::make(neg, h, l))
    }

    /// Chia cắt về 0; chỉ khi cả hai vừa 128 bit (đủ cho mọi cận của kiểu nguyên). `None` nếu chia 0 hoặc quá rộng.
    #[must_use]
    pub fn div_trunc(self, o: I256) -> Option<I256> {
        if o.is_zero() || self.hi != 0 || o.hi != 0 {
            return None;
        }
        Some(I256::make(self.neg != o.neg, 0, self.lo / o.lo))
    }

    /// Dư theo phép chia cắt về 0 (dấu theo số bị chia).
    #[must_use]
    pub fn rem_trunc(self, o: I256) -> Option<I256> {
        if o.is_zero() || self.hi != 0 || o.hi != 0 {
            return None;
        }
        Some(I256::make(self.neg, 0, self.lo % o.lo))
    }

    /// `2^n` (n ≤ 255).
    #[must_use]
    pub fn pow2(n: u32) -> Option<I256> {
        if n < 128 {
            Some(I256::make(false, 0, 1u128 << n))
        } else if n < 256 {
            Some(I256::make(false, 1u128 << (n - 128), 0))
        } else {
            None
        }
    }

    #[must_use]
    pub fn to_decimal(self) -> String {
        // chỉ để in/JSON: in thập phân chính xác bằng chia 10^k trên (hi, lo) — dùng phép chia lặp trên u128 hai phần
        let (mut hi, mut lo) = (self.hi, self.lo);
        if hi == 0 && lo == 0 {
            return "0".into();
        }
        let mut digits = Vec::new();
        while hi != 0 || lo != 0 {
            // chia (hi, lo) cho 10^19 bằng phép chia dài theo 64 bit
            let base: u128 = 10_000_000_000_000_000_000;
            let limbs = [(hi >> 64) as u64, hi as u64, (lo >> 64) as u64, lo as u64];
            let mut rem: u128 = 0;
            let mut out = [0u64; 4];
            for (i, l) in limbs.iter().enumerate() {
                let cur = (rem << 64) | u128::from(*l);
                out[i] = (cur / base) as u64;
                rem = cur % base;
            }
            hi = (u128::from(out[0]) << 64) | u128::from(out[1]);
            lo = (u128::from(out[2]) << 64) | u128::from(out[3]);
            let mut chunk = rem;
            for _ in 0..19 {
                digits.push(char::from(b'0' + (chunk % 10) as u8));
                chunk /= 10;
                if hi == 0 && lo == 0 && chunk == 0 {
                    break;
                }
            }
        }
        while digits.len() > 1 && digits.last() == Some(&'0') {
            digits.pop();
        }
        if self.neg {
            digits.push('-');
        }
        digits.iter().rev().collect()
    }
}

impl PartialOrd for I256 {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for I256 {
    fn cmp(&self, o: &Self) -> Ordering {
        match (self.neg, o.neg) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => cmp_mag((self.hi, self.lo), (o.hi, o.lo)),
            (true, true) => cmp_mag((o.hi, o.lo), (self.hi, self.lo)),
        }
    }
}

impl From<i128> for I256 {
    fn from(v: i128) -> I256 {
        I256::from_i128(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big(v: i128) -> I256 {
        I256::from_i128(v)
    }

    #[test]
    fn matches_i128_on_edge_values() {
        let vals: [i128; 11] = [
            0,
            1,
            -1,
            2,
            -2,
            255,
            i64::MAX as i128,
            i64::MIN as i128,
            i128::MAX,
            i128::MIN,
            1 << 100,
        ];
        for a in vals {
            for b in vals {
                assert_eq!(
                    big(a).checked_add(big(b)).and_then(I256::to_i128),
                    a.checked_add(b),
                    "{a}+{b}"
                );
                assert_eq!(
                    big(a).checked_sub(big(b)).and_then(I256::to_i128),
                    a.checked_sub(b),
                    "{a}-{b}"
                );
                assert_eq!(
                    big(a).checked_mul(big(b)).and_then(I256::to_i128),
                    a.checked_mul(b),
                    "{a}*{b}"
                );
                assert_eq!(big(a).cmp(&big(b)), a.cmp(&b));
                if b != 0 && a != i128::MIN && b != i128::MIN {
                    assert_eq!(
                        big(a).div_trunc(big(b)).and_then(I256::to_i128),
                        Some(a / b),
                        "{a}/{b}"
                    );
                    assert_eq!(
                        big(a).rem_trunc(big(b)).and_then(I256::to_i128),
                        Some(a % b),
                        "{a}%{b}"
                    );
                }
            }
        }
    }

    #[test]
    fn products_of_128_bit_values_are_exact_and_overflow_is_detected() {
        let m = I256::from_u128(u128::MAX);
        let sq = m.checked_mul(m).expect("vừa 256 bit");
        assert_eq!(sq.checked_sub(sq), Some(I256::ZERO));
        // (2^128 - 1)^2 = 2^256 - 2^129 + 1 vừa trong độ lớn 256 bit
        assert!(sq > I256::pow2(255).expect("2^255"));
        // 2^255 * 2 tràn độ lớn 256 bit ⇒ None
        assert_eq!(I256::pow2(255).and_then(|p| p.checked_mul(big(2))), None);
        assert_eq!(I256::pow2(255).and_then(|p| p.checked_add(p)), None);
        // phép nhân có dấu
        assert_eq!(
            big(-3).checked_mul(big(4)).and_then(I256::to_i128),
            Some(-12)
        );
        assert!(big(-3).checked_mul(big(4)).is_some_and(I256::is_neg));
        assert_eq!(I256::ZERO.negate(), I256::ZERO);
    }

    #[test]
    fn decimal_printing_is_exact() {
        assert_eq!(big(0).to_decimal(), "0");
        assert_eq!(big(-12345).to_decimal(), "-12345");
        assert_eq!(
            I256::from_u128(u128::MAX).to_decimal(),
            "340282366920938463463374607431768211455"
        );
        assert_eq!(
            big(i128::MIN).to_decimal(),
            "-170141183460469231731687303715884105728"
        );
        let sq = I256::from_u128(u128::MAX)
            .checked_mul(I256::from_u128(u128::MAX))
            .expect("vừa");
        assert_eq!(
            sq.to_decimal(),
            "115792089237316195423570985008687907852589419931798687112530834793049593217025"
        );
    }
}
