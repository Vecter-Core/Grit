//! Obligation IR (ADR-025 §1) — giao diện ổn định sang Phase 2. Tuần tự hoá được (JSON, khoá cố định, thứ tự xác định), định danh tất
//! định, trung lập với miền trừu tượng: BẤT KỲ bộ giải nào (khoảng+zone, vét cạn, SMT sau này) đọc được `goal`/`hyps`.
//! `Var(v)` luôn nghĩa là GIÁ TRỊ của biến `v` tại ĐIỂM của nghĩa vụ (goal và hyps cùng điểm).
use crate::int::I256;
use crate::interval::Ty;
use std::fmt::Write;

pub type VarId = u32;

/// Biểu thức số nguyên TOÁN HỌC (không modular): quấn vòng/ép kiểu được biểu diễn bằng khoảng kiểu tường minh trong `goal`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    Int(I256),
    Var(VarId),
    Add(Box<Term>, Box<Term>),
    Sub(Box<Term>, Box<Term>),
    Mul(Box<Term>, Box<Term>),
    Neg(Box<Term>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Prop {
    True,
    False,
    Le(Term, Term),
    Lt(Term, Term),
    Eq(Term, Term),
    Ne(Term, Term),
    And(Vec<Prop>),
    Or(Vec<Prop>),
    Not(Box<Prop>),
    /// Nguyên tử logic không diễn giải (vd `is_some(o)`): tên nằm ở `Obligation::atoms`.
    Atom(VarId),
}

impl Term {
    #[must_use]
    pub fn int(v: i128) -> Term {
        Term::Int(I256::from_i128(v))
    }

    #[must_use]
    pub fn var(v: VarId) -> Term {
        Term::Var(v)
    }

    #[must_use]
    pub fn add(a: Term, b: Term) -> Term {
        Term::Add(Box::new(a), Box::new(b))
    }

    #[must_use]
    pub fn sub(a: Term, b: Term) -> Term {
        Term::Sub(Box::new(a), Box::new(b))
    }

    #[must_use]
    pub fn mul(a: Term, b: Term) -> Term {
        Term::Mul(Box::new(a), Box::new(b))
    }

    pub fn vars(&self, out: &mut Vec<VarId>) {
        match self {
            Term::Int(_) => {}
            Term::Var(v) => out.push(*v),
            Term::Add(a, b) | Term::Sub(a, b) | Term::Mul(a, b) => {
                a.vars(out);
                b.vars(out);
            }
            Term::Neg(a) => a.vars(out),
        }
    }

    #[must_use]
    pub fn to_text(&self) -> String {
        match self {
            Term::Int(v) => v.to_decimal(),
            Term::Var(v) => format!("v{v}"),
            Term::Add(a, b) => format!("({} + {})", a.to_text(), b.to_text()),
            Term::Sub(a, b) => format!("({} - {})", a.to_text(), b.to_text()),
            Term::Mul(a, b) => format!("({} * {})", a.to_text(), b.to_text()),
            Term::Neg(a) => format!("-{}", a.to_text()),
        }
    }
}

impl Prop {
    pub fn vars(&self, out: &mut Vec<VarId>, atoms: &mut Vec<VarId>) {
        match self {
            Prop::True | Prop::False => {}
            Prop::Le(a, b) | Prop::Lt(a, b) | Prop::Eq(a, b) | Prop::Ne(a, b) => {
                a.vars(out);
                b.vars(out);
            }
            Prop::And(v) | Prop::Or(v) => v.iter().for_each(|p| p.vars(out, atoms)),
            Prop::Not(p) => p.vars(out, atoms),
            Prop::Atom(a) => atoms.push(*a),
        }
    }

    #[must_use]
    pub fn not(self) -> Prop {
        match self {
            Prop::Not(p) => *p,
            Prop::True => Prop::False,
            Prop::False => Prop::True,
            p => Prop::Not(Box::new(p)),
        }
    }

    #[must_use]
    pub fn and(ps: Vec<Prop>) -> Prop {
        let mut flat = Vec::new();
        for p in ps {
            match p {
                Prop::True => {}
                Prop::And(v) => flat.extend(v),
                p => flat.push(p),
            }
        }
        match flat.len() {
            0 => Prop::True,
            1 => flat.remove(0),
            _ => Prop::And(flat),
        }
    }

    #[must_use]
    pub fn to_text(&self) -> String {
        match self {
            Prop::True => "true".into(),
            Prop::False => "false".into(),
            Prop::Le(a, b) => format!("{} <= {}", a.to_text(), b.to_text()),
            Prop::Lt(a, b) => format!("{} < {}", a.to_text(), b.to_text()),
            Prop::Eq(a, b) => format!("{} == {}", a.to_text(), b.to_text()),
            Prop::Ne(a, b) => format!("{} != {}", a.to_text(), b.to_text()),
            Prop::And(v) => format!(
                "({})",
                v.iter().map(Prop::to_text).collect::<Vec<_>>().join(" && ")
            ),
            Prop::Or(v) => format!(
                "({})",
                v.iter().map(Prop::to_text).collect::<Vec<_>>().join(" || ")
            ),
            Prop::Not(p) => format!("!{}", p.to_text()),
            Prop::Atom(a) => format!("a{a}"),
        }
    }
}

/// Loại nghĩa vụ (mở rộng được).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ObKind {
    /// `+ - *` tràn kiểu
    Overflow,
    /// `-x` với `x = MIN`
    NegMin,
    DivZero,
    RemZero,
    /// `MIN / -1`, `MIN % -1`
    DivOverflow,
    /// `i < len`
    Index,
    /// `from ≤ to ≤ len`
    SliceRange,
    CastLossy,
    /// literal ngoài phạm vi kiểu (không phụ thuộc đường tới)
    LiteralRange,
    /// tiền điều kiện `⊳` của callee tại điểm gọi
    Pre,
    /// hậu điều kiện `⊲` ở điểm thoát
    Post,
}

impl ObKind {
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            ObKind::Overflow => "overflow",
            ObKind::NegMin => "neg_min",
            ObKind::DivZero => "div_zero",
            ObKind::RemZero => "rem_zero",
            ObKind::DivOverflow => "div_overflow",
            ObKind::Index => "index",
            ObKind::SliceRange => "slice_range",
            ObKind::CastLossy => "cast_lossy",
            ObKind::LiteralRange => "literal_range",
            ObKind::Pre => "pre",
            ObKind::Post => "post",
        }
    }
}

/// Có kiểm được lúc chạy không (D25)?
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Checkability {
    /// kiểm được: tên cách kiểm (vd `assert:overflow`)
    Runtime(String),
    /// không kiểm được lúc chạy: lý do
    NotCheckable(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VarDecl {
    pub id: VarId,
    pub name: String,
    pub ty: Ty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Obligation {
    /// tất định: `<khoá thân>#<thứ tự trong thân theo RPO>`
    pub id: String,
    pub kind: ObKind,
    pub body: String,
    pub block: u32,
    /// chỉ số câu lệnh; `== số câu lệnh` là terminator
    pub stmt: u32,
    pub span: (u32, u32),
    pub vars: Vec<VarDecl>,
    pub atoms: Vec<(VarId, String)>,
    pub goal: Prop,
    pub hyps: Vec<Prop>,
    pub runtime: Checkability,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cert(pub String);

/// Phản ví dụ: gán cho biến (rỗng = "bị bác bỏ bằng lập luận trừu tượng, không có phản ví dụ cụ thể").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witness(pub Vec<(VarId, I256)>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Proved(Cert),
    /// `everywhere`: mục tiêu SAI với MỌI gán thoả giả thiết (nghĩa vụ "chắc chắn sai"); `false` = có phản ví dụ nhưng không phải mọi gán.
    Refuted {
        witness: Witness,
        everywhere: bool,
    },
    Unknown,
}

/// Ngân sách theo BƯỚC (D20), không theo giây.
#[derive(Clone, Debug)]
pub struct Budget {
    pub steps: u64,
}

impl Budget {
    #[must_use]
    pub fn new(steps: u64) -> Budget {
        Budget { steps }
    }

    /// Tiêu `n` bước; `false` nếu hết.
    pub fn spend(&mut self, n: u64) -> bool {
        if self.steps < n {
            self.steps = 0;
            false
        } else {
            self.steps -= n;
            true
        }
    }
}

/// Bộ giải: cài đặt thay được (khoảng+zone, vét cạn, SMT sau này) mà KHÔNG đổi `Obligation`/`Verdict`.
pub trait Prover {
    fn name(&self) -> &'static str;
    fn prove(&mut self, ob: &Obligation, budget: &mut Budget) -> Verdict;
}

// ------------------------------------------------------------------ JSON (tuần tự hoá, schema có version)

pub const SCHEMA_VERSION: u32 = 1;

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

impl Obligation {
    #[must_use]
    pub fn to_json(&self) -> String {
        let vars: Vec<String> = self
            .vars
            .iter()
            .map(|v| {
                format!(
                    "{{\"id\":{},\"name\":{},\"ty\":{}}}",
                    v.id,
                    esc(&v.name),
                    esc(&v.ty.name())
                )
            })
            .collect();
        let atoms: Vec<String> = self
            .atoms
            .iter()
            .map(|(i, n)| format!("{{\"id\":{i},\"name\":{}}}", esc(n)))
            .collect();
        let hyps: Vec<String> = self.hyps.iter().map(|h| esc(&h.to_text())).collect();
        let runtime = match &self.runtime {
            Checkability::Runtime(how) => format!("{{\"checkable\":true,\"via\":{}}}", esc(how)),
            Checkability::NotCheckable(why) => {
                format!("{{\"checkable\":false,\"why\":{}}}", esc(why))
            }
        };
        format!(
            "{{\"id\":{},\"kind\":{},\"body\":{},\"block\":{},\"stmt\":{},\"span\":[{},{}],\"vars\":[{}],\"atoms\":[{}],\"goal\":{},\"hyps\":[{}],\"runtime\":{}}}",
            esc(&self.id),
            esc(self.kind.name()),
            esc(&self.body),
            self.block,
            self.stmt,
            self.span.0,
            self.span.1,
            vars.join(","),
            atoms.join(","),
            esc(&self.goal.to_text()),
            hyps.join(","),
            runtime
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_is_stable_and_escaped() {
        let ob = Obligation {
            id: "main::f#0".into(),
            kind: ObKind::Overflow,
            body: "main::f".into(),
            block: 1,
            stmt: 2,
            span: (3, 9),
            vars: vec![VarDecl {
                id: 1,
                name: "x\"q".into(),
                ty: Ty::new(8, true),
            }],
            atoms: vec![],
            goal: Prop::Le(Term::var(1), Term::int(127)),
            hyps: vec![Prop::Le(Term::int(0), Term::var(1))],
            runtime: Checkability::Runtime("assert:overflow".into()),
        };
        let a = ob.to_json();
        assert_eq!(a, ob.to_json());
        assert!(a.contains("\"name\":\"x\\\"q\""));
        assert!(a.contains("\"ty\":\"i8\""));
        assert!(a.starts_with("{\"id\":\"main::f#0\",\"kind\":\"overflow\""));
    }

    #[test]
    fn prop_helpers_normalise() {
        assert_eq!(Prop::and(vec![Prop::True, Prop::True]), Prop::True);
        assert_eq!(Prop::and(vec![Prop::True, Prop::Atom(1)]), Prop::Atom(1));
        assert_eq!(Prop::Atom(1).not().not(), Prop::Atom(1));
        let mut b = Budget::new(5);
        assert!(b.spend(3));
        assert!(!b.spend(3));
        assert_eq!(b.steps, 0);
    }
}
