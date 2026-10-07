//! Zone (DBM, Miné 2001): ràng buộc hiệu `x_i − x_j ≤ c` giữa các biến nguyên (ADR-025 §2). `m[i][j] = Some(c)` là cận trên của
//! `x_i − x_j`; `None` = không ràng buộc. Mọi số học trên hằng là `i128` CÓ KIỂM TRÀN: tràn ⇒ bỏ ràng buộc (None), không quấn vòng.
//! Luôn giữ ở dạng ĐÓNG BAO (bắc cầu) sau mỗi thao tác; âm chu trình ⇒ rỗng (không tới được).

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Zone {
    n: usize,
    m: Vec<Option<i128>>,
    empty: bool,
}

fn add(a: Option<i128>, b: Option<i128>) -> Option<i128> {
    a?.checked_add(b?)
}

/// Cận lớn hơn (nới lỏng): `None` (∞) thắng.
fn max(a: Option<i128>, b: Option<i128>) -> Option<i128> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        _ => None,
    }
}

impl Zone {
    #[must_use]
    pub fn new(n: usize) -> Zone {
        let mut m = vec![None; n * n];
        for i in 0..n {
            m[i * n + i] = Some(0);
        }
        Zone { n, m, empty: false }
    }

    #[must_use]
    #[allow(clippy::len_without_is_empty)] // `n` là số biến, không phải số phần tử; rỗng là `is_empty_set`
    pub fn len(&self) -> usize {
        self.n
    }

    #[must_use]
    pub fn is_empty_set(&self) -> bool {
        self.empty
    }

    #[must_use]
    pub fn get(&self, i: usize, j: usize) -> Option<i128> {
        if self.empty {
            Some(i128::MIN)
        } else {
            self.m[i * self.n + j]
        }
    }

    fn set(&mut self, i: usize, j: usize, v: Option<i128>) {
        let n = self.n;
        self.m[i * n + j] = v;
    }

    /// Thêm `x_i − x_j ≤ c` rồi đóng bao tăng dần (O(n²)).
    pub fn add_le(&mut self, i: usize, j: usize, c: i128) {
        if self.empty || i == j && c >= 0 {
            return;
        }
        let n = self.n;
        if let Some(cur) = self.m[i * n + j]
            && cur <= c
        {
            return;
        }
        self.set(i, j, Some(c));
        for k in 0..n {
            for l in 0..n {
                // đường k → … x_k − x_i ≤ m[k][i]; cạnh i→j; x_j − x_l ≤ m[j][l]
                let via = add(add(self.m[k * n + i], Some(c)), self.m[j * n + l]);
                if let Some(v) = via {
                    let cur = self.m[k * n + l];
                    if cur.is_none_or(|x| v < x) {
                        self.m[k * n + l] = Some(v);
                    }
                }
            }
        }
        for k in 0..n {
            if self.m[k * n + k].is_some_and(|v| v < 0) {
                self.empty = true;
                return;
            }
        }
    }

    /// Quên biến `x` (gán đè không biết): xoá hàng và cột của nó.
    pub fn forget(&mut self, x: usize) {
        if self.empty {
            return;
        }
        let n = self.n;
        for k in 0..n {
            if k != x {
                self.m[x * n + k] = None;
                self.m[k * n + x] = None;
            }
        }
        self.m[x * n + x] = Some(0);
    }

    /// `x := x + c` (đã chứng minh không tràn): dịch hàng/cột.
    pub fn shift(&mut self, x: usize, c: i128) {
        if self.empty {
            return;
        }
        let n = self.n;
        for k in 0..n {
            if k == x {
                continue;
            }
            // x_new − x_k = (x_old + c) − x_k ≤ m[x][k] + c ;  x_k − x_new ≤ m[k][x] − c
            self.m[x * n + k] = add(self.m[x * n + k], Some(c));
            self.m[k * n + x] = add(self.m[k * n + x], c.checked_neg());
        }
    }

    /// `x := y + c` (x ≠ y): quên `x` rồi thêm hai ràng buộc, đóng bao.
    pub fn assign_copy(&mut self, x: usize, y: usize, c: i128) {
        self.forget(x);
        self.add_le(x, y, c);
        if let Some(nc) = c.checked_neg() {
            self.add_le(y, x, nc);
        }
    }

    #[must_use]
    pub fn join(&self, o: &Zone) -> Zone {
        if self.empty {
            return o.clone();
        }
        if o.empty {
            return self.clone();
        }
        let mut r = self.clone();
        for k in 0..self.m.len() {
            r.m[k] = max(self.m[k], o.m[k]);
        }
        r
    }

    /// Widening chuẩn: giữ ràng buộc của `self` chỉ khi `new` không làm nó lỏng hơn; ngược lại bỏ.
    #[must_use]
    pub fn widen(&self, new: &Zone) -> Zone {
        if self.empty {
            return new.clone();
        }
        if new.empty {
            return self.clone();
        }
        let mut r = self.clone();
        for k in 0..self.m.len() {
            r.m[k] = match (self.m[k], new.m[k]) {
                (Some(a), Some(b)) if b <= a => Some(a),
                _ => None,
            };
        }
        r
    }

    /// `self ⊑ o` (self mạnh hơn hoặc bằng o).
    #[must_use]
    pub fn leq(&self, o: &Zone) -> bool {
        if self.empty {
            return true;
        }
        if o.empty {
            return false;
        }
        (0..self.m.len()).all(|k| match (self.m[k], o.m[k]) {
            (_, None) => true,
            (Some(a), Some(b)) => a <= b,
            (None, Some(_)) => false,
        })
    }

    /// Giao (`meet`) với ràng buộc của `o`.
    pub fn meet_with(&mut self, o: &Zone) {
        if self.empty {
            return;
        }
        if o.empty {
            self.empty = true;
            return;
        }
        let n = self.n;
        for i in 0..n {
            for j in 0..n {
                if let Some(c) = o.m[i * n + j] {
                    self.add_le(i, j, c);
                }
            }
        }
    }

    /// Bỏ MỌI quan hệ (chỉ còn khoảng): dùng khi hết ngân sách (D20).
    pub fn drop_all(&mut self) {
        if self.empty {
            return;
        }
        let n = self.n;
        for i in 0..n {
            for j in 0..n {
                self.m[i * n + j] = if i == j { Some(0) } else { None };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_is_transitive_and_detects_emptiness() {
        let mut z = Zone::new(3);
        z.add_le(0, 1, -1); // x0 − x1 ≤ −1
        z.add_le(1, 2, -2); // x1 − x2 ≤ −2
        assert_eq!(z.get(0, 2), Some(-3));
        z.add_le(2, 0, 2); // x2 − x0 ≤ 2 ⇒ chu trình −3 + 2 < 0
        assert!(z.is_empty_set());
    }

    #[test]
    fn assignment_shift_and_forget() {
        let mut z = Zone::new(3);
        z.assign_copy(0, 1, 0); // x0 = x1
        assert_eq!(z.get(0, 1), Some(0));
        assert_eq!(z.get(1, 0), Some(0));
        z.shift(0, 1); // x0 = x1 + 1
        assert_eq!(z.get(0, 1), Some(1));
        assert_eq!(z.get(1, 0), Some(-1));
        z.forget(0);
        assert_eq!(z.get(0, 1), None);
    }

    #[test]
    fn join_widen_and_order() {
        let mut a = Zone::new(2);
        a.add_le(0, 1, -1);
        let mut b = Zone::new(2);
        b.add_le(0, 1, -3);
        let j = a.join(&b);
        assert_eq!(j.get(0, 1), Some(-1));
        assert!(b.leq(&j) && a.leq(&j));
        let w = b.widen(&a); // −1 lỏng hơn −3 ⇒ bỏ
        assert_eq!(w.get(0, 1), None);
        assert!(a.widen(&b).get(0, 1) == Some(-1));
    }

    #[test]
    fn overflow_drops_constraints_instead_of_wrapping() {
        let mut z = Zone::new(3);
        z.add_le(0, 1, i128::MAX);
        z.add_le(1, 2, i128::MAX);
        assert_eq!(z.get(0, 2), None); // MAX + MAX tràn ⇒ không ràng buộc
        z.shift(0, i128::MAX);
        z.shift(0, i128::MAX);
        assert_eq!(z.get(0, 1), None);
    }
}
