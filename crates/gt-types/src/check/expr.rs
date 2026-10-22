//! Kiểu hoá biểu thức.
use super::*;
use crate::collect::AdtKind;
use crate::infer::VarKind;
use crate::traits::subst_with;
use crate::ty::FloatTy;

impl<'e, 'a> Checker<'e, 'a> {
    /// Kiểm `e` và ép sang `target` (điểm ép kiểu).
    pub(crate) fn expr_expect(&mut self, e: &Expr, target: &Ty) {
        let t = self.expr(e, Some(target));
        self.coerce(&t, target, e.id, e.span, e.span.end);
    }

    pub(crate) fn expr(&mut self, e: &Expr, expect: Option<&Ty>) -> Ty {
        let t = self.expr_inner(e, expect);
        self.types.insert(e.id, t.clone());
        t
    }

    fn is_numeric(&self, t: &Ty) -> bool {
        match self.infer.shallow(t) {
            Ty::Int(_) | Ty::Float(_) => true,
            Ty::Infer(i) => matches!(self.infer.kind_of(i), VarKind::Int | VarKind::Float),
            _ => false,
        }
    }

    fn is_int(&self, t: &Ty) -> bool {
        match self.infer.shallow(t) {
            Ty::Int(_) => true,
            Ty::Infer(i) => self.infer.kind_of(i) == VarKind::Int,
            _ => false,
        }
    }

    fn is_float(&self, t: &Ty) -> bool {
        match self.infer.shallow(t) {
            Ty::Float(_) => true,
            Ty::Infer(i) => self.infer.kind_of(i) == VarKind::Float,
            _ => false,
        }
    }

    fn expr_inner(&mut self, e: &Expr, expect: Option<&Ty>) -> Ty {
        match &e.kind {
            ExprKind::Lit(l) => self.lit(l),
            ExprKind::Path(p) => self.path_value(e, p),
            ExprKind::Unary { op, operand } => self.unary(e, *op, operand, expect),
            ExprKind::Binary { op, lhs, rhs } => self.binary(e, *op, lhs, rhs, expect),
            ExprKind::Cast { expr, ty } => self.cast(e, expr, ty),
            ExprKind::Call { callee, args } => self.call(e, callee, args, expect),
            ExprKind::Field { base, name } => self.field(e, base, name),
            ExprKind::Index { base, index } => self.index(e, base, index),
            ExprKind::Range {
                start,
                end,
                inclusive,
            } => self.range(e, start.as_deref(), end.as_deref(), *inclusive),
            ExprKind::Tuple(v) => {
                let hints: Vec<Option<Ty>> = match expect.map(|t| self.infer.shallow(t)) {
                    Some(Ty::Tuple(h)) if h.len() == v.len() => h.into_iter().map(Some).collect(),
                    _ => vec![None; v.len()],
                };
                Ty::Tuple(
                    v.iter()
                        .zip(hints)
                        .map(|(x, h)| self.expr(x, h.as_ref()))
                        .collect(),
                )
            }
            ExprKind::Array(v) => {
                let elem = match expect.map(|t| self.infer.shallow(t)) {
                    Some(Ty::Array(t, _)) => *t,
                    _ => self.fresh(),
                };
                for x in v {
                    self.expr_expect(x, &elem);
                }
                Ty::Array(Box::new(elem), v.len() as u64)
            }
            ExprKind::ArrayRepeat { value, len } => {
                let hint = match expect.map(|t| self.infer.shallow(t)) {
                    Some(Ty::Array(t, _)) => Some(*t),
                    _ => None,
                };
                let vt = self.expr(value, hint.as_ref());
                match self.env.eval_usize(len) {
                    Some(n) => Ty::Array(Box::new(vt), n),
                    None => {
                        self.err(
                            "E0301",
                            len.id,
                            len.span,
                            "độ dài mảng phải là hằng số nguyên xác định lúc biên dịch".to_string(),
                        );
                        Ty::Err
                    }
                }
            }
            ExprKind::StructLit { path, fields } => self.struct_lit(e, path, fields, expect),
            ExprKind::Block(b) => self.block(b, expect),
            ExprKind::If {
                cond,
                then_block,
                else_branch,
            } => self.if_expr(cond, then_block, else_branch.as_deref(), expect),
            ExprKind::Match { scrutinee, arms } => self.match_expr(e, scrutinee, arms, expect),
            ExprKind::Loop(l) => self.loop_expr(l),
            ExprKind::Unsafe { proofs, body } => {
                for p in proofs {
                    self.expr_expect(p, &Ty::Bool);
                }
                self.in_unsafe += 1;
                let t = self.block(body, expect);
                self.in_unsafe -= 1;
                t
            }
            ExprKind::Quantifier {
                var, domain, body, ..
            } => {
                if self.in_contract == 0 {
                    self.err(
                        "E0316",
                        e.id,
                        e.span,
                        "lượng từ `∀`/`∃` chỉ hợp lệ trong hợp đồng (`⊳ ⊲ ↓ ⋈`), không dùng trong biểu thức thường".to_string(),
                    );
                }
                let dt = self.expr(domain, None);
                let elem = self.range_elem(&dt, domain.id, domain.span);
                self.declare_local(var, elem, false);
                self.expr_expect(body, &Ty::Bool);
                Ty::Bool
            }
        }
    }

    // ---------------- literal ----------------

    fn lit(&mut self, l: &Lit) -> Ty {
        match &l.kind {
            LitKind::Bool(_) => Ty::Bool,
            LitKind::Char(_) => Ty::Char,
            LitKind::Str(_) => Ty::Ref(false, Box::new(Ty::Str)),
            LitKind::Float(s) => {
                if s.ends_with("f32") {
                    self.err(
                        "E0319",
                        l.id,
                        l.span,
                        "`f32` chưa hỗ trợ ở Phase 1 (ADR-035 §B): dùng `f64`".to_string(),
                    );
                    Ty::Float(FloatTy::F32)
                } else if s.ends_with("f64") {
                    Ty::Float(FloatTy::F64)
                } else {
                    self.infer.fresh(VarKind::Float)
                }
            }
            LitKind::Int(s) => {
                let is_dec = !(s.starts_with("0x") || s.starts_with("0o") || s.starts_with("0b"));
                if is_dec && s.ends_with("f32") {
                    self.err(
                        "E0319",
                        l.id,
                        l.span,
                        "`f32` chưa hỗ trợ ở Phase 1 (ADR-035 §B): dùng `f64`".to_string(),
                    );
                    return Ty::Float(FloatTy::F32);
                }
                if is_dec && s.ends_with("f64") {
                    return Ty::Float(FloatTy::F64);
                }
                match crate::collect::parse_int_literal(s) {
                    Some((_, suf)) => match suf.as_deref().and_then(IntTy::from_suffix) {
                        Some(it) => Ty::Int(it),
                        None => self.infer.fresh(VarKind::Int),
                    },
                    None => {
                        self.err(
                            "E0301",
                            l.id,
                            l.span,
                            format!("literal số nguyên `{s}` không hợp lệ hoặc quá lớn"),
                        );
                        Ty::Err
                    }
                }
            }
        }
    }

    // ---------------- toán tử ----------------

    fn unary(&mut self, e: &Expr, op: UnaryOp, operand: &Expr, expect: Option<&Ty>) -> Ty {
        match op {
            UnaryOp::Not => {
                self.expr_expect(operand, &Ty::Bool);
                Ty::Bool
            }
            UnaryOp::Neg => {
                let t = self.expr(operand, expect);
                let r = self.infer.shallow(&t);
                match r {
                    Ty::Int(it) if !it.signed() => {
                        self.err(
                            "E0301",
                            e.id,
                            e.span,
                            format!("không thể lấy số âm của kiểu không dấu `{}`", it.name()),
                        );
                    }
                    _ if !self.is_numeric(&t) && r != Ty::Err => {
                        let s = self.show(&t);
                        self.err("E0301", e.id, e.span, format!("`-` cần kiểu số, gặp `{s}`"));
                    }
                    _ => {}
                }
                t
            }
            UnaryOp::Deref => {
                let t = self.expr(operand, None);
                match self.infer.shallow(&t) {
                    Ty::Ref(_, inner) => *inner,
                    Ty::RawPtr(_, inner) => *inner,
                    Ty::Err => Ty::Err,
                    Ty::Infer(_) => {
                        self.err(
                            "E0301",
                            e.id,
                            e.span,
                            "không thể `*` một giá trị có kiểu chưa biết; hãy thêm chú thích"
                                .to_string(),
                        );
                        Ty::Err
                    }
                    other => {
                        let s = self.show(&other);
                        self.err(
                            "E0301",
                            e.id,
                            e.span,
                            format!("không thể giải tham chiếu `*` kiểu `{s}`"),
                        );
                        Ty::Err
                    }
                }
            }
            UnaryOp::Borrow => {
                let t = self.expr(operand, None);
                Ty::Ref(false, Box::new(t))
            }
            UnaryOp::BorrowMut => {
                let t = self.expr(operand, None);
                if Self::is_place(operand) && !self.place_mutable(operand) {
                    let n = self.place_name(operand);
                    self.immutable_err(
                        operand,
                        format!(
                            "mượn duy nhất `&!` một chỗ bất biến `{n}` (D5): khai báo bằng `μ`"
                        ),
                    );
                }
                Ty::Ref(true, Box::new(t))
            }
        }
    }

    fn binary(
        &mut self,
        e: &Expr,
        op: BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        expect: Option<&Ty>,
    ) -> Ty {
        use BinaryOp::*;
        match op {
            And | Or | Implies => {
                self.expr_expect(lhs, &Ty::Bool);
                self.expr_expect(rhs, &Ty::Bool);
                Ty::Bool
            }
            InSet => {
                let lt = self.expr(lhs, None);
                let rt = self.expr(rhs, None);
                let elem = self.range_elem(&rt, rhs.id, rhs.span);
                self.coerce(&lt, &elem, lhs.id, lhs.span, lhs.span.end);
                Ty::Bool
            }
            Eq | Ne | Lt | Gt | Le | Ge => {
                let lt = self.expr(lhs, None);
                let rt = self.expr(rhs, Some(&lt));
                self.same_operand(&lt, &rt, rhs);
                let r = self.infer.shallow(&lt);
                let float = self.is_float(&lt);
                if matches!(op, Eq | Ne) {
                    if float {
                        self.err(
                            "E0303",
                            e.id,
                            e.span,
                            "`==`/`≠` trên số thực bị cấm (D4): so sánh theo ngưỡng (`|a - b| < eps`) hoặc dùng hàm `core` có tên".to_string(),
                        );
                    } else if !self.eq_ok(&lt) && r != Ty::Err {
                        let s = self.show(&lt);
                        self.err(
                            "E0301",
                            e.id,
                            e.span,
                            format!("`==`/`≠` chưa hỗ trợ cho kiểu `{s}`"),
                        );
                    }
                } else {
                    let ok = self.is_numeric(&lt) || matches!(r, Ty::Char | Ty::Err);
                    if !ok {
                        let s = self.show(&lt);
                        self.err(
                            "E0301",
                            e.id,
                            e.span,
                            format!("so sánh thứ tự cần kiểu số hoặc `char`, gặp `{s}`"),
                        );
                    }
                }
                Ty::Bool
            }
            Add | Sub | Mul | Div | Rem | AddWrap | SubWrap | MulWrap | AddSat | SubSat
            | MulSat | AddChecked | SubChecked | MulChecked => {
                let lt = self.expr(lhs, expect);
                let rt = self.expr(rhs, Some(&lt));
                self.same_operand(&lt, &rt, rhs);
                let intonly = !matches!(op, Add | Sub | Mul | Div | Rem);
                let r = self.infer.shallow(&lt);
                if matches!(op, Rem) && self.is_float(&lt) {
                    self.err(
                        "E0319",
                        e.id,
                        e.span,
                        "`%` trên số thực chưa hỗ trợ ở Phase 1 (cần `fmod` ngoài bảng runtime của IR Guard; ADR-035 §B)".to_string(),
                    );
                }
                if r != Ty::Err {
                    let ok = if intonly {
                        self.is_int(&lt)
                    } else {
                        self.is_numeric(&lt)
                    };
                    if !ok {
                        let s = self.show(&lt);
                        let need = if intonly {
                            "số nguyên"
                        } else {
                            "kiểu số"
                        };
                        self.err(
                            "E0301",
                            e.id,
                            e.span,
                            format!("toán tử số học cần {need}, gặp `{s}`"),
                        );
                    }
                }
                if matches!(op, AddChecked | SubChecked | MulChecked) {
                    self.option_of(lt, e.id, e.span)
                } else {
                    lt
                }
            }
        }
    }

    /// Hai toán hạng phải cùng kiểu (D1): hợp nhất, không được thì E0302/E0301.
    fn same_operand(&mut self, lt: &Ty, rt: &Ty, rhs: &Expr) {
        let (a, b) = (self.infer.shallow(lt), self.infer.shallow(rt));
        if a == Ty::Never || b == Ty::Never {
            // `!` chỉ ép được tại điểm ép kiểu (ADR-018 §3.1); toán hạng của toán tử không phải điểm ép
            if !(a == Ty::Never && b == Ty::Never) {
                self.err("E0301", rhs.id, rhs.span, "kiểu `!` không dùng được làm toán hạng của toán tử (chỉ ép được tại điểm gán/đối số/trả về/nhánh)".to_string());
            }
            return;
        }
        if !self.infer.unify(&a, &b) {
            // báo ở vế phải: cần kiểu của vế trái
            self.mismatch(&b, &a, rhs.id, rhs.span, rhs.span.end);
        }
    }

    fn eq_ok(&self, t: &Ty) -> bool {
        match self.infer.resolve(t) {
            Ty::Int(_) | Ty::Bool | Ty::Char | Ty::Err | Ty::Infer(_) => true,
            // `&str` so theo BYTE (`gritrt_bytes_eq`). Tuple và `&T` (T ≠ str) CHƯA có: thông dịch từng so địa chỉ còn LLVM từ chối (FB-C13); dùng `*a == *b`
            Ty::Ref(false, inner) => matches!(*inner, Ty::Str),
            _ => false,
        }
    }

    fn cast(&mut self, e: &Expr, inner: &Expr, ty: &Type) -> Ty {
        let cx = self.cx.clone();
        let target = self.env.lower_ty(ty, &cx);
        let from = self.expr(inner, None);
        let (f, t) = (self.infer.shallow(&from), self.infer.shallow(&target));
        // `is_float` thấy cả biến suy diễn số thực (literal `1.5` chưa chú thích): trước đây chỉ `Ty::Float` nên `x ≔ 1.5; x as i32` lọt (FB-C14)
        if self.is_float(&f) && matches!(t, Ty::Int(_) | Ty::Char | Ty::Bool) {
            self.err(
                "E0319",
                e.id,
                e.span,
                "`as` từ số thực sang số nguyên/ký tự/bool bị cấm (ADR-035 Q70): dùng `f64::to_i64` (trả `Option`: NaN, ±∞, ngoài miền ⇒ `None`)".to_string(),
            );
            return target;
        }
        if matches!(t, Ty::Float(FloatTy::F32)) || matches!(f, Ty::Float(FloatTy::F32)) {
            self.err(
                "E0319",
                e.id,
                e.span,
                "`f32` chưa hỗ trợ ở Phase 1 (ADR-035 §B): dùng `f64`".to_string(),
            );
            return target;
        }
        let ok = match (&f, &t) {
            (Ty::Err, _) | (_, Ty::Err) => true,
            _ if self.is_numeric(&f) && matches!(t, Ty::Int(_) | Ty::Float(_)) => true,
            (Ty::Bool, Ty::Int(_)) | (Ty::Char, Ty::Int(_)) => true,
            (Ty::Int(IntTy::U8), Ty::Char) => true,
            (a, b) if a == b => true,
            _ => false,
        };
        if !ok {
            let (sf, st) = (self.show(&f), self.show(&t));
            self.err(
                "E0301",
                e.id,
                e.span,
                format!("không thể ép kiểu `{sf}` sang `{st}` bằng `as`"),
            );
        }
        target
    }

    // ---------------- trường, chỉ số, range ----------------

    fn field(&mut self, e: &Expr, base: &Expr, name: &FieldName) -> Ty {
        let bt = self.expr(base, None);
        let mut cur = self.infer.shallow(&bt);
        while let Ty::Ref(_, inner) = cur.clone() {
            cur = self.infer.shallow(&inner);
        }
        match (&cur, name) {
            (Ty::Err, _) => Ty::Err,
            (Ty::Tuple(v), FieldName::Index(i)) => match v.get(i.value as usize) {
                Some(t) => t.clone(),
                None => {
                    self.err(
                        "E0304",
                        e.id,
                        e.span,
                        format!("tuple có {} phần tử, không có `.{}`", v.len(), i.value),
                    );
                    Ty::Err
                }
            },
            (Ty::Adt(d, args), FieldName::Named(n)) => {
                let Some(adt) = self.env.db.adts.get(d).cloned() else {
                    return Ty::Err;
                };
                match &adt.kind {
                    AdtKind::Struct(fields) => match fields.iter().find(|f| f.name == n.name) {
                        Some(f) => {
                            if !f.public && !self.is_within(self.module, adt.module) {
                                self.err(
                                    "E0203",
                                    n.id,
                                    n.span,
                                    format!("trường `{}` là private (thiếu `◈`)", n.name),
                                );
                            }
                            subst_with(&f.ty, &adt.generics, args)
                        }
                        None => {
                            let have: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
                            self.err(
                                "E0304",
                                n.id,
                                n.span,
                                format!(
                                    "không có trường `{}`; các trường: {}",
                                    n.name,
                                    have.join(", ")
                                ),
                            );
                            Ty::Err
                        }
                    },
                    AdtKind::Enum(_) => {
                        self.err(
                            "E0304",
                            n.id,
                            n.span,
                            "enum không có trường; dùng `⌥` để lấy dữ liệu của biến thể"
                                .to_string(),
                        );
                        Ty::Err
                    }
                }
            }
            (Ty::Infer(_), _) => {
                self.err(
                    "E0301",
                    e.id,
                    e.span,
                    "kiểu của vế trái chưa đủ thông tin để truy cập trường; hãy thêm chú thích"
                        .to_string(),
                );
                Ty::Err
            }
            _ => {
                let s = self.show(&cur);
                self.err(
                    "E0304",
                    e.id,
                    e.span,
                    format!("kiểu `{s}` không có trường này"),
                );
                Ty::Err
            }
        }
    }

    fn index(&mut self, e: &Expr, base: &Expr, index: &Expr) -> Ty {
        let bt = self.expr(base, None);
        let mut cur = self.infer.shallow(&bt);
        while let Ty::Ref(_, inner) = cur.clone() {
            cur = self.infer.shallow(&inner);
        }
        let elem = match &cur {
            Ty::Array(t, _) | Ty::Slice(t) => (**t).clone(),
            Ty::Err => {
                let _ = self.expr(index, None);
                return Ty::Err;
            }
            other => {
                let s = self.show(other);
                self.err(
                    "E0301",
                    e.id,
                    e.span,
                    format!("không thể đánh chỉ số kiểu `{s}`"),
                );
                let _ = self.expr(index, None);
                return Ty::Err;
            }
        };
        // chỉ số khoảng: `a..b`, `a..=b`, `a..`, `..b`, `..` cho phần tử kiểu slice
        if let ExprKind::Range { start, end, .. } = &index.kind {
            let usz = Ty::Int(IntTy::Usize);
            for x in [start, end].into_iter().flatten() {
                self.expr_expect(x, &usz);
            }
            self.types.insert(index.id, Ty::Err);
            return Ty::Slice(Box::new(elem));
        }
        self.expr_expect(index, &Ty::Int(IntTy::Usize));
        elem
    }

    fn range(&mut self, e: &Expr, start: Option<&Expr>, end: Option<&Expr>, inclusive: bool) -> Ty {
        match (start, end) {
            (Some(a), Some(b)) => {
                let at = self.expr(a, None);
                let bt = self.expr(b, Some(&at));
                let (x, y) = (self.infer.shallow(&at), self.infer.shallow(&bt));
                if !self.infer.unify(&x, &y) {
                    self.mismatch(&y, &x, b.id, b.span, b.span.end);
                }
                let ok = self.is_int(&at) || matches!(self.infer.shallow(&at), Ty::Char | Ty::Err);
                if !ok {
                    let s = self.show(&at);
                    self.err(
                        "E0301",
                        e.id,
                        e.span,
                        format!("range cần số nguyên hoặc `char`, gặp `{s}`"),
                    );
                }
                self.range_of(at, inclusive, e.id, e.span)
            }
            _ => {
                self.err(
                    "E0301",
                    e.id,
                    e.span,
                    "range mở (`a..`, `..b`, `..`) chỉ có nghĩa làm chỉ số slice, không dùng độc lập (Q14)".to_string(),
                );
                Ty::Err
            }
        }
    }

    /// Phần tử của một miền lặp/lượng từ: `Range<T>` → `T`.
    pub(crate) fn range_elem(&mut self, t: &Ty, node: NodeId, span: Span) -> Ty {
        match self.infer.shallow(t) {
            Ty::Adt(d, args) if self.is_range_def(d) => args.first().cloned().unwrap_or(Ty::Err),
            Ty::Err => Ty::Err,
            other => {
                let s = self.show(&other);
                self.err(
                    "E0301",
                    node,
                    span,
                    format!("cần một range (`a..b`), gặp `{s}`"),
                );
                Ty::Err
            }
        }
    }

    // ---------------- struct literal ----------------

    fn struct_lit(
        &mut self,
        e: &Expr,
        path: &Path,
        fields: &[FieldInit],
        expect: Option<&Ty>,
    ) -> Ty {
        let adt_ty = match self.env.res.uses.get(&path.id).cloned() {
            Some(Res::Def(d)) if self.env.db.adts.contains_key(&d) => {
                let n = self.env.db.adts[&d].generics.len();
                let args: Vec<Ty> = (0..n).map(|_| self.fresh()).collect();
                Ty::Adt(d, args)
            }
            Some(Res::SelfType) => self.cx.self_ty.clone().unwrap_or(Ty::Err),
            Some(Res::Err) | None => return Ty::Err,
            _ => {
                self.err(
                    "E0301",
                    path.id,
                    path.span,
                    "literal `P { … }` cần tên một struct".to_string(),
                );
                return Ty::Err;
            }
        };
        if let Some(x) = expect {
            let probe = self.infer.probe(|inf| inf.unify(&adt_ty, x));
            if probe {
                self.infer.unify(&adt_ty, x);
            }
        }
        let Ty::Adt(d, args) = self.infer.resolve(&adt_ty) else {
            return Ty::Err;
        };
        let Some(adt) = self.env.db.adts.get(&d).cloned() else {
            return Ty::Err;
        };
        let AdtKind::Struct(def_fields) = &adt.kind else {
            self.err(
                "E0301",
                path.id,
                path.span,
                "literal `P { … }` cần struct, không phải enum".to_string(),
            );
            return Ty::Err;
        };
        let forged = matches!(
            adt.lang,
            Some(Lang::Secret | Lang::Cap | Lang::Vec | Lang::Text)
        ) && !self.env.is_prelude(e.id.file());
        if forged {
            let name = self.env.defs()[d.index()].path.clone();
            self.err(
                "E0313",
                e.id,
                e.span,
                format!("không thể tạo `{name}` bằng literal: kiểu capability/secret/Vec phải do `lib/core` cấp (D29, ADR-032)"),
            );
        }
        let mut seen: Vec<&str> = Vec::new();
        for f in fields {
            if seen.contains(&f.name.name.as_str()) {
                self.err(
                    "E0304",
                    f.name.id,
                    f.name.span,
                    format!("trường `{}` được khởi tạo nhiều lần", f.name.name),
                );
                continue;
            }
            seen.push(&f.name.name);
            match def_fields.iter().find(|x| x.name == f.name.name) {
                Some(df) => {
                    if !forged && !df.public && !self.is_within(self.module, adt.module) {
                        self.err(
                            "E0203",
                            f.name.id,
                            f.name.span,
                            format!("trường `{}` là private (thiếu `◈`)", f.name.name),
                        );
                    }
                    let ft = subst_with(&df.ty, &adt.generics, &args);
                    self.expr_expect(&f.value, &ft);
                }
                None => {
                    let have: Vec<&str> = def_fields.iter().map(|x| x.name.as_str()).collect();
                    self.err(
                        "E0304",
                        f.name.id,
                        f.name.span,
                        format!(
                            "struct không có trường `{}`; các trường: {}",
                            f.name.name,
                            have.join(", ")
                        ),
                    );
                    let _ = self.expr(&f.value, None);
                }
            }
        }
        let missing: Vec<&str> = def_fields
            .iter()
            .map(|x| x.name.as_str())
            .filter(|n| !seen.contains(n))
            .collect();
        if !missing.is_empty() {
            let fname = self.env.fname(e.id.file());
            let ins = e.span.end.saturating_sub(1);
            let text = missing
                .iter()
                .map(|m| format!(", {m}: …"))
                .collect::<String>();
            let sugg = Suggestion::new(
                "thêm các trường còn thiếu",
                Applicability::MaybeIncorrect,
                vec![gt_diag::Edit {
                    file: fname,
                    span: Span::new(ins, ins),
                    replacement: text,
                }],
            );
            self.err_sugg(
                "E0304",
                e.id,
                e.span,
                format!("thiếu trường: {}", missing.join(", ")),
                sugg,
            );
        }
        Ty::Adt(d, args)
    }

    // ---------------- if / match / loop ----------------

    /// Kiểu chung của các nhánh (ADR-018 §3.2): nhánh `!` bị bỏ qua; mọi nhánh `!` thì cả biểu thức là `!`.
    fn join(&mut self, arms: &[(Ty, NodeId, Span)], expect: Option<&Ty>) -> Ty {
        let mut cur: Option<Ty> = expect
            .map(|t| self.infer.shallow(t))
            .filter(|t| !matches!(t, Ty::Infer(_)));
        let mut any = false;
        for (t, n, sp) in arms {
            any = true;
            if self.infer.shallow(t) == Ty::Never {
                continue;
            }
            match &cur {
                None => cur = Some(t.clone()),
                Some(c) => {
                    let c = c.clone();
                    self.coerce(t, &c, *n, *sp, sp.end);
                }
            }
        }
        match cur {
            Some(c) => c,
            None if any => Ty::Never,
            None => Ty::unit(),
        }
    }

    fn if_expr(
        &mut self,
        cond: &Expr,
        then_b: &Block,
        els: Option<&Expr>,
        expect: Option<&Ty>,
    ) -> Ty {
        self.expr_expect(cond, &Ty::Bool);
        let tt = self.block(then_b, expect);
        match els {
            None => {
                let unit = Ty::unit();
                self.coerce(&tt, &unit, then_b.id, then_b.span, then_b.span.end);
                unit
            }
            Some(x) => {
                let et = self.expr(x, expect);
                self.join(&[(tt, then_b.id, then_b.span), (et, x.id, x.span)], expect)
            }
        }
    }

    fn match_expr(&mut self, e: &Expr, scrut: &Expr, arms: &[Arm], expect: Option<&Ty>) -> Ty {
        let st = self.expr(scrut, None);
        let mut tys = Vec::new();
        let mut irs = Vec::new();
        for a in arms {
            let ir = self.pat(&a.pattern, &st);
            irs.push(ir);
            let bt = self.expr(&a.body, expect);
            tys.push((bt, a.body.id, a.body.span));
        }
        // ADR-040: pattern phá cấu trúc trên `&U`/`&!U` khớp `U` ⇒ vét cạn xét trên `U` (nhánh `_`/tên trần phủ mọi giá trị nên không phụ thuộc kiểu)
        let st = match self.infer.shallow(&st) {
            Ty::Ref(_, inner) if arms.iter().any(|a| self.destructures(&a.pattern)) => *inner,
            _ => st,
        };
        self.matches.push(MatchCheck {
            node: e.id,
            span: e.span,
            scrut: st,
            arms: irs,
            insert_at: arms
                .last()
                .map_or(e.span.end.saturating_sub(1), |a| a.span.end),
        });
        self.join(&tys, expect)
    }

    fn loop_expr(&mut self, l: &Loop) -> Ty {
        match &l.kind {
            LoopKind::Infinite => {}
            LoopKind::While(c) => self.expr_expect(c, &Ty::Bool),
            LoopKind::ForIn { var, iter } => {
                let it = self.expr(iter, None);
                let elem = self.iter_elem(&it, iter);
                self.declare_local(var, elem, false);
            }
        }
        self.in_contract += 1;
        if let Some(v) = &l.variant {
            let t = self.expr(v, None);
            let ok = self.is_int(&t) || matches!(self.infer.shallow(&t), Ty::Err);
            if !ok {
                let s = self.show(&t);
                self.err(
                    "E0301",
                    v.id,
                    v.span,
                    format!("biến thể `↓` phải có kiểu số nguyên, gặp `{s}`"),
                );
            }
        }
        for i in &l.invariants {
            self.expr_expect(i, &Ty::Bool);
        }
        self.in_contract -= 1;
        self.loops.push(false);
        let bt = self.block(&l.body, None);
        let broke = self.loops.pop().unwrap_or(false);
        let unit = Ty::unit();
        self.coerce(&bt, &unit, l.body.id, l.body.span, l.body.span.end);
        match &l.kind {
            LoopKind::Infinite if !broke => Ty::Never,
            _ => unit,
        }
    }

    /// Phần tử khi lặp: `Range<T>` → `T`; `&[T]`/`&[T; N]` → `&T` (`semantics.md` §21).
    fn iter_elem(&mut self, t: &Ty, iter: &Expr) -> Ty {
        match self.infer.shallow(t) {
            Ty::Adt(d, args) if self.is_range_def(d) => args.first().cloned().unwrap_or(Ty::Err),
            Ty::Ref(m, inner) => match self.infer.shallow(&inner) {
                Ty::Slice(e) | Ty::Array(e, _) => Ty::Ref(m, e),
                _ => self.not_iterable(t, iter),
            },
            Ty::Err => Ty::Err,
            _ => self.not_iterable(t, iter),
        }
    }

    /// `Range<T>` hoặc `RangeInclusive<T>`.
    fn is_range_def(&self, d: DefId) -> bool {
        Some(d) == self.lang(Lang::Range) || Some(d) == self.lang(Lang::RangeInclusive)
    }

    fn not_iterable(&mut self, t: &Ty, iter: &Expr) -> Ty {
        let s = self.show(t);
        self.err(
            "E0301",
            iter.id,
            iter.span,
            format!("`⟳ x in …` cần range hoặc tham chiếu tới slice/mảng, gặp `{s}`"),
        );
        Ty::Err
    }
}
