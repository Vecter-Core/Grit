//! Obligation IR (ADR-025 §1) — giao diện ổn định sang Phase 2. Tuần tự hoá được (JSON, khoá cố định, thứ tự xác định), định danh tất
//! định, trung lập với miền trừu tượng: BẤT KỲ bộ giải nào (khoảng+zone, vét cạn, SMT sau này) đọc được `goal`/`hyps`.
//! `Var(v)` luôn nghĩa là GIÁ TRỊ của biến `v` tại ĐIỂM của nghĩa vụ (goal và hyps cùng điểm).
use crate::int::I256;
use crate::interval::Ty;
use std::fmt::Write;

pub type VarId = u32;

/// Biểu thức số nguyên TOÁN HỌC (không modular): quấn vòng/ép kiểu được biểu diễn bằng khoảng kiểu tường minh trong `goal`.
/// `#[non_exhaustive]` (ADR-039): thêm biến thể KHÔNG phải thay đổi phá vỡ; bộ giải không hiểu một nút phải trả `Unknown`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Term {
    Int(I256),
    Var(VarId),
    Add(Box<Term>, Box<Term>),
    Sub(Box<Term>, Box<Term>),
    Mul(Box<Term>, Box<Term>),
    Neg(Box<Term>),
    /// Ứng dụng hàm KHÔNG diễn giải (ADR-039 §4): `size_of`, `ptr_add`, `select`… Phase 1 không sinh nút này; mọi bộ giải hiện có ⇒ `Unknown`.
    Uf(String, Vec<Term>),
}

/// Biến buộc của lượng từ có biên: `var` chạy trên `lo ≤ var < hi` (hoặc `≤ hi` khi `hi_inclusive`). `var` là biến MỚI (không trùng `VarDecl`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binder {
    pub var: VarId,
    pub lo: Term,
    pub hi: Term,
    pub hi_inclusive: bool,
}

/// `#[non_exhaustive]` (ADR-039). Các nút `Forall/Exists/Pred` Phase 1 không sinh; mọi bộ giải hiện có ⇒ `Unknown` (fail-closed).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
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
    /// Lượng từ CÓ BIÊN (ADR-039 §4): `∀ var ∈ [lo, hi): body`.
    Forall(Box<Binder>, Box<Prop>),
    /// `∃ var ∈ [lo, hi): body`.
    Exists(Box<Binder>, Box<Prop>),
    /// Ứng dụng vị từ KHÔNG diễn giải (vd `valid_write(ptr, off, n)`).
    Pred(String, Vec<Term>),
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
            Term::Uf(_, args) => args.iter().for_each(|t| t.vars(out)),
        }
    }

    #[must_use]
    pub fn to_text(&self) -> String {
        match self {
            Term::Uf(f, args) => format!(
                "{f}({})",
                args.iter()
                    .map(Term::to_text)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
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
            Prop::Forall(b, p) | Prop::Exists(b, p) => {
                b.lo.vars(out);
                b.hi.vars(out);
                p.vars(out, atoms);
            }
            Prop::Pred(_, args) => args.iter().for_each(|t| t.vars(out)),
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
            Prop::Forall(b, p) => {
                format!("forall v{} in {}: {}", b.var, b.range_text(), p.to_text())
            }
            Prop::Exists(b, p) => {
                format!("exists v{} in {}: {}", b.var, b.range_text(), p.to_text())
            }
            Prop::Pred(n, args) => format!(
                "{n}({})",
                args.iter()
                    .map(Term::to_text)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    /// Dạng cây CÓ CẤU TRÚC, tất định, để tuần tự hoá JSON (khoá `goal_ast`/`hyps_ast`; ĐÓNG BĂNG theo ADR-039 §2).
    #[must_use]
    pub fn to_ast_json(&self) -> String {
        let l = |ps: &[Prop]| {
            ps.iter()
                .map(Prop::to_ast_json)
                .collect::<Vec<_>>()
                .join(",")
        };
        let bin = |op: &str, a: &Term, b: &Term| {
            format!(
                "{{\"op\":\"{op}\",\"l\":{},\"r\":{}}}",
                a.to_ast_json(),
                b.to_ast_json()
            )
        };
        match self {
            Prop::True => "{\"op\":\"true\"}".into(),
            Prop::False => "{\"op\":\"false\"}".into(),
            Prop::Le(a, b) => bin("le", a, b),
            Prop::Lt(a, b) => bin("lt", a, b),
            Prop::Eq(a, b) => bin("eq", a, b),
            Prop::Ne(a, b) => bin("ne", a, b),
            Prop::And(v) => format!("{{\"op\":\"and\",\"args\":[{}]}}", l(v)),
            Prop::Or(v) => format!("{{\"op\":\"or\",\"args\":[{}]}}", l(v)),
            Prop::Not(p) => format!("{{\"op\":\"not\",\"arg\":{}}}", p.to_ast_json()),
            Prop::Atom(a) => format!("{{\"op\":\"atom\",\"id\":{a}}}"),
            Prop::Forall(b, p) => format!(
                "{{\"op\":\"forall\",{},\"body\":{}}}",
                b.to_ast_fields(),
                p.to_ast_json()
            ),
            Prop::Exists(b, p) => format!(
                "{{\"op\":\"exists\",{},\"body\":{}}}",
                b.to_ast_fields(),
                p.to_ast_json()
            ),
            Prop::Pred(n, args) => format!(
                "{{\"op\":\"pred\",\"name\":{},\"args\":[{}]}}",
                esc(n),
                args.iter()
                    .map(Term::to_ast_json)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

impl Binder {
    fn range_text(&self) -> String {
        format!(
            "[{}, {}{}",
            self.lo.to_text(),
            self.hi.to_text(),
            if self.hi_inclusive { "]" } else { ")" }
        )
    }

    fn to_ast_fields(&self) -> String {
        format!(
            "\"var\":{},\"lo\":{},\"hi\":{},\"hi_incl\":{}",
            self.var,
            self.lo.to_ast_json(),
            self.hi.to_ast_json(),
            self.hi_inclusive
        )
    }
}

impl Term {
    /// Dạng cây có cấu trúc, tất định (xem `Prop::to_ast_json`).
    #[must_use]
    pub fn to_ast_json(&self) -> String {
        match self {
            Term::Int(v) => format!("{{\"op\":\"int\",\"v\":\"{}\"}}", v.to_decimal()),
            Term::Var(v) => format!("{{\"op\":\"var\",\"id\":{v}}}"),
            Term::Add(a, b) => format!(
                "{{\"op\":\"add\",\"l\":{},\"r\":{}}}",
                a.to_ast_json(),
                b.to_ast_json()
            ),
            Term::Sub(a, b) => format!(
                "{{\"op\":\"sub\",\"l\":{},\"r\":{}}}",
                a.to_ast_json(),
                b.to_ast_json()
            ),
            Term::Mul(a, b) => format!(
                "{{\"op\":\"mul\",\"l\":{},\"r\":{}}}",
                a.to_ast_json(),
                b.to_ast_json()
            ),
            Term::Neg(a) => format!("{{\"op\":\"neg\",\"arg\":{}}}", a.to_ast_json()),
            Term::Uf(f, args) => format!(
                "{{\"op\":\"uf\",\"name\":{},\"args\":[{}]}}",
                esc(f),
                args.iter()
                    .map(Term::to_ast_json)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
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
    /// bất biến `⋈` ở đầu vòng lặp
    Invariant,
    /// nhiên liệu còn `> 0` khi thoát vòng kiểu "nhiên liệu" (`core::fuel_left`)
    Fuel,
    /// biến thể `↓` (người dùng viết) giảm nghiêm ngặt và bị chặn dưới (M8): E0702
    Variant,
    /// biến thể TỰ SUY từ điều kiện `while` giảm nghiêm ngặt (M8): không chứng minh được ⇒ E0701
    LoopBound,
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
            ObKind::Invariant => "invariant",
            ObKind::Fuel => "fuel",
            ObKind::Variant => "variant",
            ObKind::LoopBound => "loop_bound",
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
        let hyps_ast: Vec<String> = self.hyps.iter().map(Prop::to_ast_json).collect();
        format!(
            "{{\"id\":{},\"kind\":{},\"body\":{},\"block\":{},\"stmt\":{},\"span\":[{},{}],\"vars\":[{}],\"atoms\":[{}],\"goal\":{},\"hyps\":[{}],\"runtime\":{},\"goal_ast\":{},\"hyps_ast\":[{}]}}",
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
            runtime,
            self.goal.to_ast_json(),
            hyps_ast.join(",")
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
