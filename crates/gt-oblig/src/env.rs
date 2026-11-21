//! Môi trường trừu tượng: khoảng cho mỗi biến nguyên + zone trên tập con biến + nguyên tử logic đã biết (ADR-025 §2). Dùng chung
//! cho bộ giải chính (dựng lại từ `hyps`) và cho bộ phân tích luồng (trạng thái tại mỗi điểm).
//! Soundness: mọi `assume` chỉ THU HẸP bằng suy luận hợp lệ; `assign` chỉ ghi quan hệ zone khi phép gán là tuyến tính đơn vị đã
//! chứng minh/được bảo đảm không tràn (điều kiện (a) của Giang); số học hằng zone có kiểm tràn (tràn ⇒ bỏ ràng buộc).
use crate::int::I256;
use crate::interval::{B, Interval, Tri, Ty};
use crate::ir::{Prop, Term, VarId};
use crate::zone::Zone;
use std::collections::BTreeMap;

/// Dạng tuyến tính đơn vị: `Σ s_k · x_k + c` (tối đa 2 biến, hệ số ±1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lin {
    pub vars: Vec<(i8, VarId)>,
    pub c: I256,
}

#[must_use]
pub fn lin(t: &Term) -> Option<Lin> {
    fn go(t: &Term, sign: i8, out: &mut Lin) -> Option<()> {
        match t {
            Term::Int(c) => {
                out.c = if sign > 0 {
                    out.c.checked_add(*c)?
                } else {
                    out.c.checked_sub(*c)?
                };
                Some(())
            }
            Term::Var(v) => {
                out.vars.push((sign, *v));
                Some(())
            }
            Term::Add(a, b) => {
                go(a, sign, out)?;
                go(b, sign, out)
            }
            Term::Sub(a, b) => {
                go(a, sign, out)?;
                go(b, -sign, out)
            }
            Term::Neg(a) => go(a, -sign, out),
            Term::Mul(..) | Term::Uf(..) => None,
        }
    }
    let mut l = Lin {
        vars: vec![],
        c: I256::ZERO,
    };
    go(t, 1, &mut l)?;
    // gộp cùng biến
    let mut merged: Vec<(i8, VarId)> = Vec::new();
    for (s, v) in l.vars {
        if let Some(e) = merged.iter_mut().find(|(_, w)| *w == v) {
            e.0 += s;
        } else {
            merged.push((s, v));
        }
    }
    merged.retain(|(s, _)| *s != 0);
    if merged.iter().any(|(s, _)| s.abs() > 1) || merged.len() > 4 {
        return None;
    }
    l.vars = merged;
    Some(l)
}

/// `a − b` dạng tuyến tính (gộp biến, bỏ hệ số 0); `None` nếu quá 2 biến hoặc tràn.
#[must_use]
pub fn lin_diff(a: &Lin, b: &Lin) -> Option<Lin> {
    let mut vars = a.vars.clone();
    for (s, v) in &b.vars {
        vars.push((-s, *v));
    }
    let mut merged: Vec<(i8, VarId)> = Vec::new();
    for (s, v) in vars {
        if let Some(e) = merged.iter_mut().find(|(_, w)| *w == v) {
            e.0 += s;
        } else {
            merged.push((s, v));
        }
    }
    merged.retain(|(s, _)| *s != 0);
    if merged.iter().any(|(s, _)| s.abs() > 1) || merged.len() > 4 {
        return None;
    }
    Some(Lin {
        vars: merged,
        c: a.c.checked_sub(b.c)?,
    })
}

fn i128_of(v: I256) -> Option<i128> {
    v.to_i128()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Env {
    iv: BTreeMap<VarId, (Ty, Interval)>,
    zv: BTreeMap<VarId, usize>,
    zone: Zone,
    atoms: BTreeMap<VarId, bool>,
    dead: bool,
}

impl Env {
    /// `zone_vars`: biến được cấp chỗ trong zone (theo thứ tự cho trước, tất định).
    #[must_use]
    pub fn new(zone_vars: &[VarId]) -> Env {
        let zv: BTreeMap<VarId, usize> = zone_vars
            .iter()
            .copied()
            .enumerate()
            .map(|(i, v)| (v, i))
            .collect();
        Env {
            iv: BTreeMap::new(),
            zone: Zone::new(zv.len()),
            zv,
            atoms: BTreeMap::new(),
            dead: false,
        }
    }

    pub fn declare(&mut self, v: VarId, ty: Ty) {
        self.iv.insert(v, (ty, ty.range()));
    }

    #[must_use]
    pub fn is_dead(&self) -> bool {
        self.dead
    }

    pub fn kill(&mut self) {
        self.dead = true;
    }

    #[must_use]
    pub fn ty(&self, v: VarId) -> Option<Ty> {
        self.iv.get(&v).map(|x| x.0)
    }

    #[must_use]
    pub fn interval(&self, v: VarId) -> Interval {
        self.iv.get(&v).map_or_else(Interval::top, |x| x.1)
    }

    #[must_use]
    pub fn atom(&self, a: VarId) -> Option<bool> {
        self.atoms.get(&a).copied()
    }

    pub fn set_atom(&mut self, a: VarId, val: bool) {
        self.atoms.insert(a, val);
    }

    pub fn forget_atom(&mut self, a: VarId) {
        self.atoms.remove(&a);
    }

    /// Giữ lại chỉ các nguyên tử thoả `keep`.
    pub fn retain_atoms(&mut self, keep: impl Fn(VarId) -> bool) {
        self.atoms.retain(|a, _| keep(*a));
    }

    #[must_use]
    pub fn zone_bound(&self, x: VarId, y: VarId) -> Option<i128> {
        let (i, j) = (*self.zv.get(&x)?, *self.zv.get(&y)?);
        self.zone.get(i, j)
    }

    #[must_use]
    pub fn in_zone(&self, v: VarId) -> bool {
        self.zv.contains_key(&v)
    }

    pub fn vars(&self) -> impl Iterator<Item = VarId> + '_ {
        self.iv.keys().copied()
    }

    pub fn atom_ids(&self) -> impl Iterator<Item = (VarId, bool)> + '_ {
        self.atoms.iter().map(|(k, v)| (*k, *v))
    }

    /// Quên mọi thứ về `v` (gán đè không biết): về khoảng của kiểu, xoá quan hệ zone.
    pub fn forget(&mut self, v: VarId) {
        if let Some((ty, iv)) = self.iv.get_mut(&v) {
            *iv = ty.range();
        }
        if let Some(i) = self.zv.get(&v).copied() {
            self.zone.forget(i);
        }
    }

    /// Bỏ mọi quan hệ zone (hết ngân sách, D20): còn khoảng.
    pub fn drop_relations(&mut self) {
        self.zone.drop_all();
    }

    // ------------------------------------------------------------ lượng giá

    #[must_use]
    pub fn eval_term(&self, t: &Term) -> Interval {
        match t {
            Term::Int(c) => Interval::point(*c),
            Term::Var(v) => self.interval(*v),
            Term::Add(a, b) => self.eval_term(a).add(self.eval_term(b)),
            Term::Sub(a, b) => self.eval_term(a).sub(self.eval_term(b)),
            Term::Mul(a, b) => self.eval_term(a).mul(self.eval_term(b)),
            Term::Neg(a) => self.eval_term(a).neg(),
            // hàm không diễn giải: không biết gì về giá trị ⇒ khoảng tuỳ ý (sound)
            Term::Uf(..) => Interval::top(),
        }
    }

    /// Khoảng của `t` giao với cận zone của chính `t` (dạng tuyến tính `c + Σ ±x`): đọc được `i − n ≤ −1` mà khoảng đơn lẻ không thấy.
    /// Cả hai là biên ngoài ĐÚNG nên giao cũng đúng; nếu giao rỗng (chỉ khi trạng thái chết) trả khoảng thuần để không tạo khoảng sai.
    #[must_use]
    pub fn eval_term_zone(&self, t: &Term) -> Interval {
        let iv = self.eval_term(t);
        let Some(p) = lin(t) else { return iv };
        let (up, lo) = self.zone_diff(&p);
        let (mut l, mut h) = (iv.lo, iv.hi);
        if let Some(u) = up
            && B::Fin(u) < h
        {
            h = B::Fin(u);
        }
        if let Some(v) = lo
            && B::Fin(v) > l
        {
            l = B::Fin(v);
        }
        if l > h { iv } else { Interval { lo: l, hi: h } }
    }

    /// Cận trên/dưới của `P = a − b` dùng zone (hai biến ngược dấu cùng trong zone).
    fn zone_diff(&self, p: &Lin) -> (Option<I256>, Option<I256>) {
        // hai vế cùng biểu thức (các biến triệt tiêu): P = c CHÍNH XÁC (vd `x == x`, `x + 1 > x`)
        if p.vars.is_empty() {
            return (Some(p.c), Some(p.c));
        }
        // P = c + Σ (x_i − y_i) (mỗi biến dương ghép một biến âm; hệ số ±1 do `lin`):
        //   x_i − y_i ≤ m[x_i][y_i]  nên  P ≤ c + Σ m[x_i][y_i];   y_i − x_i ≤ m[y_i][x_i]  nên  P ≥ c − Σ m[y_i][x_i].
        // Hai cận độc lập nên mỗi cận chọn phép ghép TỐT NHẤT riêng (≤ 4 cặp ⇒ ≤ 24 hoán vị); ghép nào cũng cho cận ĐÚNG.
        let pos: Vec<VarId> = p
            .vars
            .iter()
            .filter(|(s, _)| *s > 0)
            .map(|(_, v)| *v)
            .collect();
        let neg: Vec<VarId> = p
            .vars
            .iter()
            .filter(|(s, _)| *s < 0)
            .map(|(_, v)| *v)
            .collect();
        if pos.len() != neg.len() || pos.len() > 4 || p.vars.iter().any(|(s, _)| s.abs() != 1) {
            return (None, None);
        }
        let n = pos.len();
        let mut idx: Vec<usize> = (0..n).collect();
        let (mut best_up, mut best_lo): (Option<I256>, Option<I256>) = (None, None);
        let mut consider = |perm: &[usize]| {
            let mut up = Some(p.c);
            let mut lo = Some(p.c);
            for (i, &j) in perm.iter().enumerate() {
                up = match (up, self.zone_bound(pos[i], neg[j])) {
                    (Some(u), Some(m)) => u.checked_add(I256::from_i128(m)),
                    _ => None,
                };
                lo = match (lo, self.zone_bound(neg[j], pos[i])) {
                    (Some(l), Some(m)) => l.checked_sub(I256::from_i128(m)),
                    _ => None,
                };
            }
            if let Some(u) = up
                && best_up.is_none_or(|b| u < b)
            {
                best_up = Some(u);
            }
            if let Some(l) = lo
                && best_lo.is_none_or(|b| l > b)
            {
                best_lo = Some(l);
            }
        };
        // Heap: duyệt mọi hoán vị
        fn heap(k: usize, a: &mut Vec<usize>, f: &mut impl FnMut(&[usize])) {
            if k <= 1 {
                f(a);
                return;
            }
            for i in 0..k {
                heap(k - 1, a, f);
                if k.is_multiple_of(2) {
                    a.swap(i, k - 1);
                } else {
                    a.swap(0, k - 1);
                }
            }
        }
        heap(n, &mut idx, &mut consider);
        (best_up, best_lo)
    }

    fn tri_le(&self, a: &Term, b: &Term, strict: bool) -> Tri {
        // a ≤ b  (hoặc a < b)
        let (ia, ib) = (self.eval_term(a), self.eval_term(b));
        let one = I256::from_i128(1);
        // chắc chắn đúng: hi(a) + strict ≤ lo(b)
        if let (B::Fin(ha), B::Fin(lb)) = (ia.hi, ib.lo) {
            let lhs = if strict {
                ha.checked_add(one)
            } else {
                Some(ha)
            };
            if lhs.is_some_and(|v| v <= lb) {
                return Tri::Yes;
            }
        }
        // chắc chắn sai: a ≤ b sai ⇔ a ≥ b + 1 ; a < b sai ⇔ a ≥ b
        if let (B::Fin(la), B::Fin(hb)) = (ia.lo, ib.hi) {
            let need = if strict {
                Some(hb)
            } else {
                hb.checked_add(one)
            };
            if need.is_some_and(|n| la >= n) {
                return Tri::No;
            }
        }
        // zone: P = a − b
        if let (Some(la), Some(lb)) = (lin(a), lin(b))
            && let Some(p) = lin_diff(&la, &lb)
        {
            let (up, lo) = self.zone_diff(&p);
            // a ≤ b ⇔ P ≤ 0 ; a < b ⇔ P ≤ −1
            if let Some(u) = up
                && u <= I256::from_i128(-i128::from(strict))
            {
                return Tri::Yes;
            }
            // sai: a ≤ b sai ⇔ P ≥ 1 ; a < b sai ⇔ P ≥ 0
            if let Some(l) = lo
                && l >= I256::from_i128(i128::from(!strict))
            {
                return Tri::No;
            }
        }
        Tri::Maybe
    }

    #[must_use]
    pub fn eval_prop(&self, p: &Prop) -> Tri {
        fn not(t: Tri) -> Tri {
            match t {
                Tri::Yes => Tri::No,
                Tri::No => Tri::Yes,
                Tri::Maybe => Tri::Maybe,
            }
        }
        match p {
            Prop::True => Tri::Yes,
            Prop::False => Tri::No,
            Prop::Le(a, b) => self.tri_le(a, b, false),
            Prop::Lt(a, b) => self.tri_le(a, b, true),
            Prop::Eq(a, b) => {
                let (l, r) = (self.tri_le(a, b, false), self.tri_le(b, a, false));
                match (l, r) {
                    (Tri::Yes, Tri::Yes) => Tri::Yes,
                    (Tri::No, _) | (_, Tri::No) => Tri::No,
                    _ => Tri::Maybe,
                }
            }
            Prop::Ne(a, b) => not(self.eval_prop(&Prop::Eq(a.clone(), b.clone()))),
            Prop::And(v) => {
                let mut r = Tri::Yes;
                for q in v {
                    match self.eval_prop(q) {
                        Tri::No => return Tri::No,
                        Tri::Maybe => r = Tri::Maybe,
                        Tri::Yes => {}
                    }
                }
                r
            }
            Prop::Or(v) => {
                let mut r = Tri::No;
                for q in v {
                    match self.eval_prop(q) {
                        Tri::Yes => return Tri::Yes,
                        Tri::Maybe => r = Tri::Maybe,
                        Tri::No => {}
                    }
                }
                r
            }
            Prop::Not(q) => not(self.eval_prop(q)),
            // nút của Phase 2: không đánh giá được ⇒ Maybe (không bao giờ Yes)
            Prop::Forall(..) | Prop::Exists(..) | Prop::Pred(..) => Tri::Maybe,
            Prop::Atom(a) => match self.atoms.get(a) {
                Some(true) => Tri::Yes,
                Some(false) => Tri::No,
                None => Tri::Maybe,
            },
        }
    }

    // ------------------------------------------------------------ thu hẹp

    fn refine_hi(&mut self, v: VarId, hi: B) {
        if let Some((_, iv)) = self.iv.get_mut(&v) {
            if hi < iv.lo {
                self.dead = true;
            } else if hi < iv.hi {
                iv.hi = hi;
            }
        }
    }

    fn refine_lo(&mut self, v: VarId, lo: B) {
        if let Some((_, iv)) = self.iv.get_mut(&v) {
            if lo > iv.hi {
                self.dead = true;
            } else if lo > iv.lo {
                iv.lo = lo;
            }
        }
    }

    /// Lan truyền zone → khoảng (và kiểm rỗng). Lặp tới ổn định (tối đa số biến + 1 lượt).
    pub fn reduce(&mut self) {
        if self.zone.is_empty_set() {
            self.dead = true;
        }
        if self.dead {
            return;
        }
        let zvars: Vec<(VarId, usize)> = self.zv.iter().map(|(v, i)| (*v, *i)).collect();
        for _ in 0..=zvars.len() {
            let mut changed = false;
            for &(x, i) in &zvars {
                for &(y, j) in &zvars {
                    if x == y {
                        continue;
                    }
                    let Some(c) = self.zone.get(i, j) else {
                        continue;
                    };
                    let cc = I256::from_i128(c);
                    let (ix, iy) = (self.interval(x), self.interval(y));
                    // x ≤ y + c ⇒ hi(x) ≤ hi(y) + c ; lo(y) ≥ lo(x) − c
                    if let B::Fin(hy) = iy.hi
                        && let Some(h) = hy.checked_add(cc)
                        && B::Fin(h) < ix.hi
                    {
                        self.refine_hi(x, B::Fin(h));
                        changed = true;
                    }
                    if let B::Fin(lx) = ix.lo
                        && let Some(l) = lx.checked_sub(cc)
                        && B::Fin(l) > iy.lo
                    {
                        self.refine_lo(y, B::Fin(l));
                        changed = true;
                    }
                }
            }
            if self.dead || !changed {
                break;
            }
        }
    }

    /// Giả thiết `a ≤ b` (hoặc `a < b`).
    fn assume_le(&mut self, a: &Term, b: &Term, strict: bool) {
        let (ia, ib) = (self.eval_term(a), self.eval_term(b));
        // rỗng nếu chắc chắn sai
        if self.tri_le(a, b, strict) == Tri::No {
            self.dead = true;
            return;
        }
        let (Some(la), Some(lb)) = (lin(a), lin(b)) else {
            // phi tuyến: chỉ dùng khoảng trên biến trần
            if let Term::Var(x) = a
                && let B::Fin(hb) = ib.hi
            {
                let h = if strict {
                    hb.checked_sub(I256::from_i128(1))
                } else {
                    Some(hb)
                };
                if let Some(h) = h {
                    self.refine_hi(*x, B::Fin(h));
                }
            }
            if let Term::Var(y) = b
                && let B::Fin(la2) = ia.lo
            {
                let l = if strict {
                    la2.checked_add(I256::from_i128(1))
                } else {
                    Some(la2)
                };
                if let Some(l) = l {
                    self.refine_lo(*y, B::Fin(l));
                }
            }
            self.reduce();
            return;
        };
        // P = a − b ≤ −strict ⇒ Σ s_k x_k + c ≤ 0 với c = P.c (+ 1 nếu strict)
        let Some(p) = lin_diff(&la, &lb) else { return };
        let merged = p.vars;
        let mut c = p.c;
        if strict {
            let Some(c2) = c.checked_add(I256::from_i128(1)) else {
                return;
            };
            c = c2;
        }
        // Σ s_k x_k + c ≤ 0
        match merged.as_slice() {
            [] => {
                if c > I256::ZERO {
                    self.dead = true;
                }
            }
            [(s, x)] => {
                // s·x ≤ −c
                let Some(nc) = Some(c.negate()) else { return };
                if *s > 0 {
                    self.refine_hi(*x, B::Fin(nc));
                } else {
                    // −x ≤ −c ⇒ x ≥ c
                    self.refine_lo(*x, B::Fin(c));
                }
            }
            [(s1, x), (s2, y)] => {
                let (x, y) = (*x, *y);
                if *s1 > 0 && *s2 < 0 {
                    self.assume_diff(x, y, c.negate());
                } else if *s1 < 0 && *s2 > 0 {
                    self.assume_diff(y, x, c.negate());
                } else if *s1 > 0 {
                    // x + y ≤ −c ⇒ hi(x) ≤ −c − lo(y), hi(y) ≤ −c − lo(x)
                    let nc = c.negate();
                    let (ix, iy) = (self.interval(x), self.interval(y));
                    if let B::Fin(ly) = iy.lo
                        && let Some(h) = nc.checked_sub(ly)
                    {
                        self.refine_hi(x, B::Fin(h));
                    }
                    if let B::Fin(lx) = ix.lo
                        && let Some(h) = nc.checked_sub(lx)
                    {
                        self.refine_hi(y, B::Fin(h));
                    }
                } else {
                    // −x − y ≤ −c ⇒ x + y ≥ c ⇒ lo(x) ≥ c − hi(y), lo(y) ≥ c − hi(x)
                    let (ix, iy) = (self.interval(x), self.interval(y));
                    if let B::Fin(hy) = iy.hi
                        && let Some(l) = c.checked_sub(hy)
                    {
                        self.refine_lo(x, B::Fin(l));
                    }
                    if let B::Fin(hx) = ix.hi
                        && let Some(l) = c.checked_sub(hx)
                    {
                        self.refine_lo(y, B::Fin(l));
                    }
                }
            }
            _ => {}
        }
        self.reduce();
    }

    /// `x − y ≤ k` (zone nếu cả hai ở zone; luôn lan truyền sang khoảng qua `reduce`).
    fn assume_diff(&mut self, x: VarId, y: VarId, k: I256) {
        let Some(kk) = i128_of(k) else { return };
        if let (Some(i), Some(j)) = (self.zv.get(&x).copied(), self.zv.get(&y).copied()) {
            self.zone.add_le(i, j, kk);
        } else {
            // chỉ khoảng: hi(x) ≤ hi(y) + k ; lo(y) ≥ lo(x) − k
            let (ix, iy) = (self.interval(x), self.interval(y));
            if let B::Fin(hy) = iy.hi
                && let Some(h) = hy.checked_add(k)
            {
                self.refine_hi(x, B::Fin(h));
            }
            if let B::Fin(lx) = ix.lo
                && let Some(l) = lx.checked_sub(k)
            {
                self.refine_lo(y, B::Fin(l));
            }
        }
    }

    /// Giả thiết `p` đúng (`positive`) hay sai.
    pub fn assume(&mut self, p: &Prop, positive: bool) {
        if self.dead {
            return;
        }
        match (p, positive) {
            (Prop::True, true) | (Prop::False, false) => {}
            (Prop::True, false) | (Prop::False, true) => self.dead = true,
            (Prop::Not(q), pos) => self.assume(q, !pos),
            (Prop::And(v), true) => {
                for q in v {
                    self.assume(q, true);
                }
            }
            (Prop::Or(v), false) => {
                for q in v {
                    self.assume(q, false);
                }
            }
            (Prop::And(v), false) => {
                self.assume_disjunction(v.iter().map(|q| (q, false)).collect())
            }
            (Prop::Or(v), true) => self.assume_disjunction(v.iter().map(|q| (q, true)).collect()),
            (Prop::Le(a, b), true) => self.assume_le(a, b, false),
            (Prop::Lt(a, b), true) => self.assume_le(a, b, true),
            (Prop::Le(a, b), false) => self.assume_le(b, a, true),
            (Prop::Lt(a, b), false) => self.assume_le(b, a, false),
            (Prop::Eq(a, b), true) => {
                self.assume_le(a, b, false);
                self.assume_le(b, a, false);
            }
            (Prop::Ne(a, b), false) => {
                self.assume_le(a, b, false);
                self.assume_le(b, a, false);
            }
            (Prop::Ne(a, b), true) | (Prop::Eq(a, b), false) => self.assume_ne(a, b),
            // nút của Phase 2: giả thiết không học được gì (mất chính xác, vẫn sound)
            (Prop::Forall(..) | Prop::Exists(..) | Prop::Pred(..), _) => {}
            (Prop::Atom(a), pos) => {
                if self.atoms.get(a).is_some_and(|old| *old != pos) {
                    self.dead = true;
                } else {
                    self.atoms.insert(*a, pos);
                }
            }
        }
    }

    /// Phép tuyển: tính trạng thái cho từng nhánh rồi hợp (chỉ giữ thông tin chung).
    fn assume_disjunction(&mut self, alts: Vec<(&Prop, bool)>) {
        let mut acc: Option<Env> = None;
        for (q, pos) in alts {
            if self.eval_prop(q) == if pos { Tri::No } else { Tri::Yes } {
                continue; // nhánh chắc chắn sai
            }
            let mut e = self.clone();
            e.assume(q, pos);
            if e.dead {
                continue;
            }
            acc = Some(match acc {
                None => e,
                Some(a) => a.join(&e),
            });
        }
        match acc {
            Some(e) => *self = e,
            None => self.dead = true,
        }
    }

    /// `a ≠ b`: chỉ cắt biên khi một vế là hằng sát biên của vế kia.
    fn assume_ne(&mut self, a: &Term, b: &Term) {
        if self.tri_le(a, b, false) == Tri::Yes && self.tri_le(b, a, false) == Tri::Yes {
            self.dead = true;
            return;
        }
        let one = I256::from_i128(1);
        let (Some(la), Some(lb)) = (lin(a), lin(b)) else {
            return;
        };
        // hiệu hằng bằng 0 (cùng biểu thức hai vế) ⇒ `a ≠ a` là mâu thuẫn
        if lin_diff(&la, &lb).is_some_and(|d| d.vars.is_empty() && d.c == I256::from_i128(0)) {
            self.dead = true;
            return;
        }
        // dạng  s·x + c ≠ k  với một biến và một hằng
        let (var_side, other) = match (la.vars.as_slice(), lb.vars.as_slice()) {
            ([(s, x)], []) => ((*s, *x, la.c), lb.c),
            ([], [(s, x)]) => ((*s, *x, lb.c), la.c),
            _ => return,
        };
        let (s, x, c) = var_side;
        // s·x + c ≠ other ⇒ x ≠ s·(other − c)
        let Some(d) = other.checked_sub(c) else {
            return;
        };
        let val = if s > 0 { d } else { d.negate() };
        let iv = self.interval(x);
        if iv.lo == B::Fin(val) {
            if let Some(n) = val.checked_add(one) {
                self.refine_lo(x, B::Fin(n));
            }
        } else if iv.hi == B::Fin(val)
            && let Some(n) = val.checked_sub(one)
        {
            self.refine_hi(x, B::Fin(n));
        }
        self.reduce();
    }

    // ------------------------------------------------------------ gán

    /// `v := t` (kết quả TOÁN HỌC của `t`; đường tràn đã abort nên giá trị tiếp tục nằm trong kiểu của `v`).
    pub fn assign(&mut self, v: VarId, t: &Term) {
        let Some(ty) = self.ty(v) else { return };
        let val = self.eval_term(t);
        let l = lin(t);
        // quan hệ zone chỉ khi tuyến tính đơn vị MỘT biến (+ hằng) — điều kiện (a): đã chứng minh/bảo đảm không tràn
        let relation = match l.as_ref().map(|l| l.vars.as_slice()) {
            Some([(1, y)]) => l.as_ref().and_then(|l| i128_of(l.c)).map(|c| (*y, c)),
            _ => None,
        };
        // tính zone TRƯỚC khi ghi khoảng mới (cần khoảng cũ của y khi y == v)
        let zi = self.zv.get(&v).copied();
        match (zi, relation) {
            (Some(i), Some((y, c))) if y == v => self.zone.shift(i, c),
            (Some(i), Some((y, c))) => match self.zv.get(&y).copied() {
                Some(j) => self.zone.assign_copy(i, j, c),
                None => self.zone.forget(i),
            },
            (Some(i), None) => self.zone.forget(i),
            _ => {}
        }
        match val.meet(ty.range()) {
            Some(iv) => {
                if let Some(e) = self.iv.get_mut(&v) {
                    e.1 = iv;
                }
            }
            None => {
                self.dead = true;
                return;
            }
        }
        self.reduce();
    }

    /// `v := khoảng` với thêm sự thật `v_mới ≤ v_cũ + c` (GÓI 12: `x ← x / k`, `x ← a % x`): giữ quan hệ cận trên của `v` dịch `c`.
    pub fn assign_interval_upper_shift(&mut self, v: VarId, iv: Interval, c: i128) {
        let Some(ty) = self.ty(v) else { return };
        if let Some(i) = self.zv.get(&v).copied() {
            self.zone.shift_upper(i, c);
        }
        match iv.meet(ty.range()) {
            Some(x) => {
                if let Some(e) = self.iv.get_mut(&v) {
                    e.1 = x;
                }
            }
            None => {
                self.dead = true;
                return;
            }
        }
        self.reduce();
    }

    /// `v := khoảng` (không quan hệ): kết quả của phép quấn/bão hoà/gọi hàm.
    pub fn assign_interval(&mut self, v: VarId, iv: Interval) {
        let Some(ty) = self.ty(v) else { return };
        if let Some(i) = self.zv.get(&v).copied() {
            self.zone.forget(i);
        }
        match iv.meet(ty.range()) {
            Some(x) => {
                if let Some(e) = self.iv.get_mut(&v) {
                    e.1 = x;
                }
            }
            None => self.dead = true,
        }
    }

    // ------------------------------------------------------------ dàn

    #[must_use]
    pub fn join(&self, o: &Env) -> Env {
        if self.dead {
            return o.clone();
        }
        if o.dead {
            return self.clone();
        }
        let mut r = self.clone();
        for (v, (ty, iv)) in &mut r.iv {
            match o.iv.get(v) {
                Some((_, oi)) => *iv = iv.join(*oi),
                // biến thiếu ở phía kia = KHÔNG BIẾT (như `leq` coi `None` là đỉnh): khoảng đủ của kiểu, không giữ khoảng hẹp của một phía
                None => *iv = ty.range(),
            }
        }
        r.zone = self.zone.join(&o.zone);
        r.atoms = self
            .atoms
            .iter()
            .filter(|(k, v)| o.atoms.get(k) == Some(v))
            .map(|(k, v)| (*k, *v))
            .collect();
        r
    }

    #[must_use]
    pub fn widen(&self, new: &Env, thresholds: &[I256]) -> Env {
        if self.dead {
            return new.clone();
        }
        if new.dead {
            return self.clone();
        }
        let mut r = self.clone();
        for (v, (ty, iv)) in &mut r.iv {
            match new.iv.get(v) {
                Some((_, ni)) => {
                    // không vượt kiểu
                    *iv = iv.widen(*ni, thresholds).meet(ty.range()).unwrap_or(*iv);
                }
                None => *iv = ty.range(), // thiếu ở phía mới = không biết
            }
        }
        r.zone = self.zone.widen(&new.zone);
        r.atoms = self
            .atoms
            .iter()
            .filter(|(k, v)| new.atoms.get(k) == Some(v))
            .map(|(k, v)| (*k, *v))
            .collect();
        r
    }

    /// `self ⊑ o` (self mạnh hơn hoặc bằng).
    #[must_use]
    pub fn leq(&self, o: &Env) -> bool {
        if self.dead {
            return true;
        }
        if o.dead {
            return false;
        }
        self.iv
            .iter()
            .all(|(v, (_, iv))| o.iv.get(v).is_none_or(|(_, oi)| iv.subset_of(*oi)))
            && self.zone.leq(&o.zone)
            && o.atoms.iter().all(|(k, v)| self.atoms.get(k) == Some(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(bits: u32) -> Ty {
        Ty::new(bits, false)
    }

    #[test]
    fn index_proof_from_a_branch_condition_needs_the_zone() {
        // i < len ⇒ i ≤ MAX − 1 (khoảng) và i − len ≤ −1 (zone); mục tiêu `i < len` được chứng minh
        let mut e = Env::new(&[1, 2]);
        e.declare(1, u(64));
        e.declare(2, u(64));
        e.assume(&Prop::Lt(Term::var(1), Term::var(2)), true);
        assert!(!e.is_dead());
        assert_eq!(e.eval_prop(&Prop::Lt(Term::var(1), Term::var(2))), Tri::Yes);
        // i + 1 không tràn: i + 1 ≤ MAX
        assert_eq!(
            e.eval_prop(&Prop::Le(
                Term::add(Term::var(1), Term::int(1)),
                Term::Int(u(64).max())
            )),
            Tri::Yes
        );
        // chỉ khoảng thuần KHÔNG chứng minh được `i < len` (không có zone)
        let mut f = Env::new(&[]);
        f.declare(1, u(64));
        f.declare(2, u(64));
        f.assume(&Prop::Lt(Term::var(1), Term::var(2)), true);
        assert_eq!(
            f.eval_prop(&Prop::Lt(Term::var(1), Term::var(2))),
            Tri::Maybe
        );
    }

    /// R3 (FAIL_OPEN_AUDIT gt-oblig/env.rs `join`/`widen`): biến có ở MỘT phía mà không có ở phía kia ("thiếu" = không biết, như `leq` coi `None` là đỉnh) phải thành KHOẢNG ĐỦ
    /// của kiểu, không được giữ khoảng hẹp của một phía (quá chính xác ⇒ có thể chứng minh nhầm).
    #[test]
    fn join_and_widen_treat_a_variable_missing_on_one_side_as_unknown() {
        let mut a = Env::new(&[]);
        a.declare(1, u(8));
        a.assume(&Prop::Le(Term::var(1), Term::int(2)), true);
        assert_eq!(a.eval_prop(&Prop::Le(Term::var(1), Term::int(2))), Tri::Yes);
        let b = Env::new(&[]); // không khai báo biến 1
        let j = a.join(&b);
        assert_eq!(
            j.eval_prop(&Prop::Le(Term::var(1), Term::int(2))),
            Tri::Maybe,
            "join giữ khoảng hẹp của một phía"
        );
        let w = a.widen(&b, &[]);
        assert_eq!(
            w.eval_prop(&Prop::Le(Term::var(1), Term::int(2))),
            Tri::Maybe,
            "widen giữ khoảng hẹp của một phía"
        );
    }

    #[test]
    fn assignment_updates_relations_and_shift_preserves_them() {
        let mut e = Env::new(&[1, 2]);
        e.declare(1, u(64));
        e.declare(2, u(64));
        e.assume(&Prop::Lt(Term::var(1), Term::var(2)), true);
        e.assign(1, &Term::add(Term::var(1), Term::int(1))); // i := i + 1
        assert_eq!(e.eval_prop(&Prop::Le(Term::var(1), Term::var(2))), Tri::Yes);
        assert_eq!(
            e.eval_prop(&Prop::Lt(Term::var(1), Term::var(2))),
            Tri::Maybe
        );
        e.assign(1, &Term::int(0)); // i := 0 ⇒ quên quan hệ
        assert_eq!(e.zone_bound(1, 2), None);
    }

    #[test]
    fn contradictory_assumptions_make_the_state_unreachable() {
        let mut e = Env::new(&[1]);
        e.declare(1, u(8));
        e.assume(&Prop::Lt(Term::var(1), Term::int(5)), true);
        e.assume(&Prop::Le(Term::int(9), Term::var(1)), true);
        assert!(e.is_dead());
    }

    #[test]
    fn join_widen_leq_are_consistent() {
        let mut a = Env::new(&[1]);
        a.declare(1, u(8));
        a.assign(1, &Term::int(1));
        let mut b = a.clone();
        b.assign(1, &Term::int(5));
        let j = a.join(&b);
        assert!(a.leq(&j) && b.leq(&j));
        let w = a.widen(&b, &[I256::from_i128(10)]);
        assert_eq!(
            w.interval(1),
            Interval::new(B::from_i128(1), B::from_i128(10))
        );
        assert!(b.leq(&w));
    }
}
