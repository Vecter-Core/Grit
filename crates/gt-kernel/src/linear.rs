//! TCB (ADR-P2-04): dịch một `Obligation` thành DNF các ràng buộc tuyến tính nguyên `Σ aᵢ·xᵢ + c ≤ 0`. Công thức được dịch là
//! `cận kiểu ∧ giả thiết ∧ ¬mục tiêu`; nghĩa vụ ĐÚNG ⇔ công thức này vô nghiệm. Mọi chỗ không hiểu chỉ được làm công thức YẾU đi
//! (thay bằng `True`, hoặc coi tích phi tuyến là một biến mờ) ⇒ chứng minh bản yếu vô nghiệm vẫn đúng cho bản gốc.
use gt_oblig::ir::{Obligation, Prop, Term};
use std::collections::BTreeMap;

/// Biến của ràng buộc: biến của nghĩa vụ, hoặc một số hạng phi tuyến coi như biến mờ (khoá = dạng `Debug` của số hạng, tất định).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    Var(u32),
    Opaque(String),
}

/// `Σ coef·key + c` (ràng buộc nghĩa là biểu thức này `≤ 0`). Hệ số 0 không được lưu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lin {
    pub coef: BTreeMap<Key, i128>,
    pub c: i128,
}

impl Lin {
    fn konst(c: i128) -> Lin {
        Lin {
            coef: BTreeMap::new(),
            c,
        }
    }
    fn key(k: Key) -> Lin {
        let mut coef = BTreeMap::new();
        coef.insert(k, 1);
        Lin { coef, c: 0 }
    }
    /// `self + m·o` (tràn ⇒ `None`)
    pub fn add_scaled(&self, o: &Lin, m: i128) -> Option<Lin> {
        let mut r = self.clone();
        for (k, v) in &o.coef {
            let e = r.coef.entry(k.clone()).or_insert(0);
            *e = e.checked_add(v.checked_mul(m)?)?;
            if *e == 0 {
                r.coef.remove(k);
            }
        }
        r.c = r.c.checked_add(o.c.checked_mul(m)?)?;
        Some(r)
    }
    pub fn scale(&self, m: i128) -> Option<Lin> {
        Lin::konst(0).add_scaled(self, m)
    }
}

/// Số hạng → dạng tuyến tính. Tích hai số hạng không hằng / `Uf` ⇒ biến mờ (sound: chỉ bỏ quên quan hệ). Hằng/tràn ngoài i128 ⇒ `None`.
fn lin(t: &Term) -> Option<Lin> {
    match t {
        Term::Int(v) => v.to_i128().map(Lin::konst),
        Term::Var(v) => Some(Lin::key(Key::Var(*v))),
        Term::Add(a, b) => lin(a)?.add_scaled(&lin(b)?, 1),
        Term::Sub(a, b) => lin(a)?.add_scaled(&lin(b)?, -1),
        Term::Neg(a) => lin(a)?.scale(-1),
        Term::Mul(a, b) => {
            let (la, lb) = (lin(a)?, lin(b)?);
            if la.coef.is_empty() {
                lb.scale(la.c)
            } else if lb.coef.is_empty() {
                la.scale(lb.c)
            } else {
                Some(Lin::key(Key::Opaque(format!("{t:?}"))))
            }
        }
        // `Uf` và loại số hạng tương lai (`#[non_exhaustive]`): biến mờ — chỉ bỏ quên quan hệ, sound
        _ => Some(Lin::key(Key::Opaque(format!("{t:?}")))),
    }
}

/// Một ca của DNF: hội các ràng buộc `≤ 0` và các literal nguyên tử logic `(nguyên tử, dương?)`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Case {
    pub cons: Vec<Lin>,
    pub atoms: Vec<(u32, bool)>,
}

/// DNF: danh sách ca. `vec![]` = vô nghiệm hiển nhiên (False); `vec![Case::default()]` = True.
pub type Dnf = Vec<Case>;

fn one(cons: Vec<Lin>) -> Dnf {
    vec![Case {
        cons,
        atoms: Vec::new(),
    }]
}

/// Số ca tối đa của DNF (vượt ⇒ không dịch được ⇒ không chứng minh).
pub const MAX_CASES: usize = 64;

fn product(a: Dnf, b: &Dnf) -> Option<Dnf> {
    if a.len().checked_mul(b.len())? > MAX_CASES {
        return None;
    }
    let mut out = Vec::new();
    for x in &a {
        for y in b {
            let mut c = x.clone();
            c.cons.extend(y.cons.iter().cloned());
            c.atoms.extend(y.atoms.iter().copied());
            out.push(c);
        }
    }
    Some(out)
}

fn le(a: &Term, b: &Term, plus: i128) -> Option<Lin> {
    // a − b + plus ≤ 0
    lin(a)?
        .add_scaled(&lin(b)?, -1)?
        .add_scaled(&Lin::konst(plus), 1)
}

/// `p` (khi `pos`) hoặc `¬p` (khi `!pos`) ⇒ DNF. Lá không hiểu ⇒ True (làm yếu). `None` = vượt giới hạn ca.
pub fn dnf(p: &Prop, pos: bool) -> Option<Dnf> {
    let atom = |l: Option<Lin>| -> Dnf {
        match l {
            Some(l) => one(vec![l]),
            None => one(Vec::new()),
        }
    };
    let two = |x: Option<Lin>, y: Option<Lin>| -> Dnf {
        match (x, y) {
            (Some(x), Some(y)) => vec![
                Case {
                    cons: vec![x],
                    atoms: Vec::new(),
                },
                Case {
                    cons: vec![y],
                    atoms: Vec::new(),
                },
            ],
            _ => one(Vec::new()),
        }
    };
    Some(match (p, pos) {
        (Prop::True, true) | (Prop::False, false) => one(Vec::new()),
        (Prop::True, false) | (Prop::False, true) => vec![],
        // nguyên tử logic: literal (dương hoặc âm) — kernel chỉ dùng nó để phát hiện `a ∧ ¬a`
        (Prop::Atom(a), _) => vec![Case {
            cons: Vec::new(),
            atoms: vec![(*a, pos)],
        }],
        (Prop::Le(a, b), true) => atom(le(a, b, 0)),
        (Prop::Le(a, b), false) => atom(le(b, a, 1)), // a > b ⇔ b − a + 1 ≤ 0
        (Prop::Lt(a, b), true) => atom(le(a, b, 1)),
        (Prop::Lt(a, b), false) => atom(le(b, a, 0)), // a ≥ b
        (Prop::Eq(a, b), true) => match (le(a, b, 0), le(b, a, 0)) {
            (Some(x), Some(y)) => one(vec![x, y]),
            _ => one(Vec::new()),
        },
        (Prop::Eq(a, b), false) | (Prop::Ne(a, b), true) => two(le(a, b, 1), le(b, a, 1)),
        (Prop::Ne(a, b), false) => match (le(a, b, 0), le(b, a, 0)) {
            (Some(x), Some(y)) => one(vec![x, y]),
            _ => one(Vec::new()),
        },
        (Prop::Not(q), _) => dnf(q, !pos)?,
        (Prop::And(v), true) | (Prop::Or(v), false) => {
            let mut acc: Dnf = one(Vec::new());
            for q in v {
                acc = product(acc, &dnf(q, pos)?)?;
            }
            acc
        }
        (Prop::Or(v), true) | (Prop::And(v), false) => {
            let mut acc: Dnf = Vec::new();
            for q in v {
                acc.extend(dnf(q, pos)?);
                if acc.len() > MAX_CASES {
                    return None;
                }
            }
            acc
        }
        // vị từ, lượng từ: kernel không diễn giải ⇒ True (làm yếu, sound)
        _ => one(Vec::new()),
    })
}

/// Công thức cần chứng minh VÔ NGHIỆM: cận kiểu của mọi biến ∧ mọi giả thiết ∧ ¬mục tiêu, ở dạng DNF (thứ tự tất định).
pub fn refutation_dnf(ob: &Obligation) -> Option<Dnf> {
    let mut bounds: Vec<Lin> = Vec::new();
    for v in &ob.vars {
        let x = Lin::key(Key::Var(v.id));
        // min ≤ x ⇔ min − x ≤ 0 ; x ≤ max ⇔ x − max ≤ 0 (cận không vừa i128 ⇒ bỏ, làm yếu)
        if let Some(lo) = v.ty.min().to_i128() {
            bounds.push(Lin::konst(lo).add_scaled(&x, -1)?);
        }
        if let Some(hi) = v.ty.max().to_i128() {
            bounds.push(x.add_scaled(&Lin::konst(hi), -1)?);
        }
    }
    let mut acc: Dnf = one(bounds);
    for h in &ob.hyps {
        acc = product(acc, &dnf(h, true)?)?;
    }
    product(acc, &dnf(&ob.goal, false)?)
}
