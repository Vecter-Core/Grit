//! Hai cài đặt của `Prover` (ADR-025 §12): bộ giải CHÍNH (khoảng + zone) và bộ giải LIỆT KÊ VÉT CẠN trên độ rộng bit nhỏ. Bộ thứ hai là
//! oracle độc lập (không dùng `Env`/`Interval`/`Zone`; số học riêng bằng `I256` kiểm tràn) và chứng minh giao diện cắm được.
use crate::env::Env;
use crate::int::I256;
use crate::interval::Tri;
use crate::ir::{Budget, Cert, Obligation, Prop, Prover, Term, VarDecl, VarId, Verdict, Witness};

/// Bộ giải chính: dựng `Env` từ `hyps` rồi lượng giá `goal`.
#[derive(Default)]
pub struct ZoneProver {
    /// số biến tối đa được cấp zone (xem ZONE_MAX_VARS ở lib)
    pub zone_cap: usize,
}

impl ZoneProver {
    #[must_use]
    pub fn new(zone_cap: usize) -> ZoneProver {
        ZoneProver { zone_cap }
    }

    fn env_of(&self, ob: &Obligation) -> Env {
        let cap = if self.zone_cap == 0 {
            24
        } else {
            self.zone_cap
        };
        let zvars: Vec<VarId> = ob.vars.iter().map(|v| v.id).take(cap).collect();
        let mut e = Env::new(&zvars);
        for v in &ob.vars {
            e.declare(v.id, v.ty);
        }
        for h in &ob.hyps {
            e.assume(h, true);
        }
        e
    }
}

impl Prover for ZoneProver {
    fn name(&self) -> &'static str {
        "interval+zone"
    }

    fn prove(&mut self, ob: &Obligation, budget: &mut Budget) -> Verdict {
        if !budget.spend(1 + ob.hyps.len() as u64) {
            return Verdict::Unknown;
        }
        let e = self.env_of(ob);
        if e.is_dead() {
            return Verdict::Proved(Cert("giả thiết mâu thuẫn (điểm không tới được)".into()));
        }
        match e.eval_prop(&ob.goal) {
            Tri::Yes => {
                let mut vs = Vec::new();
                let mut at = Vec::new();
                ob.goal.vars(&mut vs, &mut at);
                vs.sort_unstable();
                vs.dedup();
                let facts: Vec<String> = vs
                    .iter()
                    .map(|v| {
                        let iv = e.interval(*v);
                        format!("v{v}∈[{:?},{:?}]", iv.lo, iv.hi)
                    })
                    .collect();
                Verdict::Proved(Cert(facts.join("; ")))
            }
            Tri::No => Verdict::Refuted {
                witness: Witness(vec![]),
                everywhere: true,
            },
            Tri::Maybe => Verdict::Unknown,
        }
    }
}

// ====================================================================== bộ giải vét cạn

/// Liệt kê mọi gán cho các biến xuất hiện trong `goal`/`hyps` (mỗi biến chạy hết khoảng kiểu của nó; nguyên tử logic nhận {sai, đúng}).
pub struct ExhaustiveProver {
    /// số gán tối đa được liệt kê (vượt ⇒ `Unknown`)
    pub max_points: u64,
}

impl Default for ExhaustiveProver {
    fn default() -> Self {
        ExhaustiveProver {
            max_points: 1 << 24,
        }
    }
}

fn eval_term(t: &Term, vals: &std::collections::BTreeMap<VarId, I256>) -> Option<I256> {
    Some(match t {
        Term::Int(c) => *c,
        Term::Var(v) => *vals.get(v)?,
        Term::Add(a, b) => eval_term(a, vals)?.checked_add(eval_term(b, vals)?)?,
        Term::Sub(a, b) => eval_term(a, vals)?.checked_sub(eval_term(b, vals)?)?,
        Term::Mul(a, b) => eval_term(a, vals)?.checked_mul(eval_term(b, vals)?)?,
        Term::Neg(a) => eval_term(a, vals)?.negate(),
    })
}

fn eval_prop(
    p: &Prop,
    vals: &std::collections::BTreeMap<VarId, I256>,
    atoms: &std::collections::BTreeMap<VarId, bool>,
) -> Option<bool> {
    Some(match p {
        Prop::True => true,
        Prop::False => false,
        Prop::Le(a, b) => eval_term(a, vals)? <= eval_term(b, vals)?,
        Prop::Lt(a, b) => eval_term(a, vals)? < eval_term(b, vals)?,
        Prop::Eq(a, b) => eval_term(a, vals)? == eval_term(b, vals)?,
        Prop::Ne(a, b) => eval_term(a, vals)? != eval_term(b, vals)?,
        Prop::And(v) => {
            for q in v {
                if !eval_prop(q, vals, atoms)? {
                    return Some(false);
                }
            }
            true
        }
        Prop::Or(v) => {
            for q in v {
                if eval_prop(q, vals, atoms)? {
                    return Some(true);
                }
            }
            false
        }
        Prop::Not(q) => !eval_prop(q, vals, atoms)?,
        Prop::Atom(a) => *atoms.get(a)?,
    })
}

impl Prover for ExhaustiveProver {
    fn name(&self) -> &'static str {
        "exhaustive"
    }

    fn prove(&mut self, ob: &Obligation, budget: &mut Budget) -> Verdict {
        let mut used: Vec<VarId> = Vec::new();
        let mut atoms_used: Vec<VarId> = Vec::new();
        ob.goal.vars(&mut used, &mut atoms_used);
        for h in &ob.hyps {
            h.vars(&mut used, &mut atoms_used);
        }
        used.sort_unstable();
        used.dedup();
        atoms_used.sort_unstable();
        atoms_used.dedup();
        let decls: Vec<&VarDecl> = used
            .iter()
            .filter_map(|v| ob.vars.iter().find(|d| d.id == *v))
            .collect();
        if decls.len() != used.len() {
            return Verdict::Unknown; // biến chưa khai báo kiểu
        }
        // số điểm
        let mut points: u128 = 1;
        for d in &decls {
            if d.ty.bits > 20 {
                return Verdict::Unknown;
            }
            points = points.saturating_mul(1u128 << d.ty.bits);
        }
        for _ in &atoms_used {
            points = points.saturating_mul(2);
        }
        if points > u128::from(self.max_points) || !budget.spend(points as u64) {
            return Verdict::Unknown;
        }
        let (mut sat, mut viol) = (0u64, 0u64);
        let mut witness: Option<Vec<(VarId, I256)>> = None;
        // bộ đếm hỗn hợp: biến nguyên từ min..=max, nguyên tử {0,1}
        let los: Vec<i128> = decls
            .iter()
            .map(|d| d.ty.min().to_i128().unwrap_or(0))
            .collect();
        let his: Vec<i128> = decls
            .iter()
            .map(|d| d.ty.max().to_i128().unwrap_or(0))
            .collect();
        let mut cur: Vec<i128> = los.clone();
        let mut acur: Vec<bool> = vec![false; atoms_used.len()];
        loop {
            let vals: std::collections::BTreeMap<VarId, I256> = decls
                .iter()
                .zip(&cur)
                .map(|(d, c)| (d.id, I256::from_i128(*c)))
                .collect();
            let atoms: std::collections::BTreeMap<VarId, bool> = atoms_used
                .iter()
                .copied()
                .zip(acur.iter().copied())
                .collect();
            let hyps_ok = ob
                .hyps
                .iter()
                .all(|h| eval_prop(h, &vals, &atoms) == Some(true));
            if hyps_ok {
                sat += 1;
                match eval_prop(&ob.goal, &vals, &atoms) {
                    Some(true) => {}
                    _ => {
                        viol += 1;
                        if witness.is_none() {
                            witness = Some(vals.iter().map(|(k, v)| (*k, *v)).collect());
                        }
                    }
                }
            }
            // tăng bộ đếm
            let mut i = 0;
            let mut carried = true;
            while i < cur.len() {
                if cur[i] < his[i] {
                    cur[i] += 1;
                    carried = false;
                    break;
                }
                cur[i] = los[i];
                i += 1;
            }
            if carried {
                let mut j = 0;
                while j < acur.len() {
                    if acur[j] {
                        acur[j] = false;
                        j += 1;
                    } else {
                        acur[j] = true;
                        carried = false;
                        break;
                    }
                }
                if carried {
                    break;
                }
            }
        }
        if sat == 0 {
            Verdict::Proved(Cert("giả thiết không thoả (vét cạn)".into()))
        } else if viol == 0 {
            Verdict::Proved(Cert(format!(
                "{sat} gán thoả giả thiết, mục tiêu đúng với mọi gán"
            )))
        } else {
            Verdict::Refuted {
                witness: Witness(witness.unwrap_or_default()),
                everywhere: viol == sat,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interval::Ty;
    use crate::ir::{Checkability, ObKind};

    fn ob(vars: &[(VarId, Ty)], goal: Prop, hyps: Vec<Prop>) -> Obligation {
        Obligation {
            id: "t#0".into(),
            kind: ObKind::Overflow,
            body: "t".into(),
            block: 0,
            stmt: 0,
            span: (0, 0),
            vars: vars
                .iter()
                .map(|(id, ty)| VarDecl {
                    id: *id,
                    name: format!("v{id}"),
                    ty: *ty,
                })
                .collect(),
            atoms: vec![],
            goal,
            hyps,
            runtime: Checkability::Runtime("test".into()),
        }
    }

    #[test]
    fn both_provers_agree_on_a_simple_index_obligation() {
        let u4 = Ty::new(4, false);
        let o = ob(
            &[(1, u4), (2, u4)],
            Prop::Lt(Term::var(1), Term::var(2)),
            vec![Prop::Lt(Term::var(1), Term::var(2))],
        );
        let mut b = Budget::new(1 << 20);
        assert!(matches!(
            ZoneProver::new(8).prove(&o, &mut b),
            Verdict::Proved(_)
        ));
        assert!(matches!(
            ExhaustiveProver::default().prove(&o, &mut b),
            Verdict::Proved(_)
        ));
        // không có giả thiết: cả hai không chứng minh được, vét cạn cho phản ví dụ nhưng không "everywhere"
        let o2 = ob(
            &[(1, u4), (2, u4)],
            Prop::Lt(Term::var(1), Term::var(2)),
            vec![],
        );
        assert_eq!(ZoneProver::new(8).prove(&o2, &mut b), Verdict::Unknown);
        assert!(matches!(
            ExhaustiveProver::default().prove(&o2, &mut b),
            Verdict::Refuted {
                everywhere: false,
                ..
            }
        ));
    }

    #[test]
    fn definite_failure_is_reported_by_both() {
        let u8t = Ty::new(3, false);
        let o = ob(
            &[(1, u8t)],
            Prop::Lt(Term::var(1), Term::int(2)),
            vec![Prop::Le(Term::int(5), Term::var(1))],
        );
        let mut b = Budget::new(1 << 20);
        assert!(matches!(
            ZoneProver::new(8).prove(&o, &mut b),
            Verdict::Refuted {
                everywhere: true,
                ..
            }
        ));
        assert!(matches!(
            ExhaustiveProver::default().prove(&o, &mut b),
            Verdict::Refuted {
                everywhere: true,
                ..
            }
        ));
    }

    #[test]
    fn budget_exhaustion_is_unknown_never_proved() {
        let t = Ty::new(4, false);
        let o = ob(&[(1, t)], Prop::Le(Term::int(0), Term::var(1)), vec![]);
        assert_eq!(
            ZoneProver::new(8).prove(&o, &mut Budget::new(0)),
            Verdict::Unknown
        );
        assert_eq!(
            ExhaustiveProver::default().prove(&o, &mut Budget::new(3)),
            Verdict::Unknown
        );
    }
}
