//! Kiểu hoá pattern và dựng IR cho bộ vét cạn.
use super::*;
use crate::collect::AdtKind;
use crate::traits::{fresh_args, subst_with};

impl<'e, 'a> Checker<'e, 'a> {
    /// Kiểm pattern `p` với kiểu `ty`; khai báo biến; trả IR vét cạn.
    pub(crate) fn pat(&mut self, p: &Pattern, ty: &Ty) -> XPat {
        match &p.kind {
            PatternKind::Wildcard | PatternKind::Rest => XPat::Wild,
            PatternKind::Binding(ident) => match self.env.res.uses.get(&ident.id).cloned() {
                Some(Res::Def(vd)) if self.env.defs()[vd.index()].kind == DefKind::Variant => {
                    self.variant_pat(p, vd, &[], ty)
                }
                _ => {
                    self.declare_local(ident, ty.clone(), false);
                    XPat::Wild
                }
            },
            PatternKind::Lit(l) => self.lit_pat(p, l, ty),
            PatternKind::Range {
                start,
                end,
                inclusive,
            } => self.range_pat(p, start.as_ref(), end.as_ref(), *inclusive, ty),
            PatternKind::Variant { path, args } => match self.env.res.uses.get(&path.id).cloned() {
                Some(Res::Def(vd)) if self.env.defs()[vd.index()].kind == DefKind::Variant => {
                    self.variant_pat(p, vd, args, ty)
                }
                Some(Res::Err) | None => {
                    self.bind_all(args);
                    XPat::Wild
                }
                _ => {
                    self.err(
                        "E0301",
                        p.id,
                        p.span,
                        "pattern `P(…)` cần tên một biến thể của enum".to_string(),
                    );
                    self.bind_all(args);
                    XPat::Wild
                }
            },
            PatternKind::Struct { path, fields, rest } => {
                self.struct_pat(p, path, fields, *rest, ty)
            }
            PatternKind::Tuple(v) => self.tuple_pat(p, v, ty),
        }
    }

    /// Khai báo biến trong các pattern con khi pattern cha đã lỗi (tránh lỗi dây chuyền "chưa có kiểu").
    fn bind_all(&mut self, args: &[Pattern]) {
        for a in args {
            let _ = self.pat(a, &Ty::Err);
        }
    }

    fn pat_mismatch(&mut self, p: &Pattern, want: &Ty, got: &Ty) {
        let (sw, sg) = (self.show(want), self.show(got));
        self.err(
            "E0301",
            p.id,
            p.span,
            format!("pattern có kiểu `{sg}` không khớp kiểu cần `{sw}`"),
        );
    }

    fn lit_pat(&mut self, p: &Pattern, l: &Lit, ty: &Ty) -> XPat {
        let lt = self.expr_lit_type(l);
        if !self.infer.unify(&lt, ty) {
            self.pat_mismatch(p, ty, &lt);
            return XPat::Wild;
        }
        match &l.kind {
            LitKind::Bool(b) => XPat::Bool(*b),
            LitKind::Int(s) => match crate::collect::parse_int_literal(s) {
                Some((v, _)) => {
                    let v = i128::try_from(v).unwrap_or(i128::MAX);
                    let v = if l.negative { -v } else { v };
                    XPat::Range(v, v)
                }
                None => XPat::Opaque,
            },
            LitKind::Char(c) => match decode_char(c) {
                Some(cp) => XPat::Range(i128::from(cp), i128::from(cp)),
                None => XPat::Opaque,
            },
            LitKind::Str(_) | LitKind::Float(_) => XPat::Opaque,
        }
    }

    fn lit_value(l: &Lit) -> Option<i128> {
        match &l.kind {
            LitKind::Int(s) => {
                let (v, _) = crate::collect::parse_int_literal(s)?;
                let v = i128::try_from(v).ok()?;
                Some(if l.negative { -v } else { v })
            }
            LitKind::Char(c) => decode_char(c).map(i128::from),
            _ => None,
        }
    }

    fn range_pat(
        &mut self,
        p: &Pattern,
        start: Option<&Lit>,
        end: Option<&Lit>,
        inclusive: bool,
        ty: &Ty,
    ) -> XPat {
        let mut lo = i128::MIN;
        let mut hi = i128::MAX;
        for (l, is_start) in [(start, true), (end, false)] {
            let Some(l) = l else { continue };
            let lt = self.expr_lit_type(l);
            if !self.infer.unify(&lt, ty) {
                self.pat_mismatch(p, ty, &lt);
                return XPat::Wild;
            }
            let rt = self.infer.shallow(ty);
            let okty = matches!(rt, Ty::Int(_) | Ty::Char | Ty::Err)
                || matches!(rt, Ty::Infer(i) if self.infer.kind_of(i) == crate::infer::VarKind::Int);
            if !okty {
                let s = self.show(ty);
                self.err(
                    "E0301",
                    p.id,
                    p.span,
                    format!("pattern khoảng chỉ dùng cho số nguyên và `char`, gặp `{s}`"),
                );
                return XPat::Wild;
            }
            if let Some(v) = Self::lit_value(l) {
                if is_start {
                    lo = v;
                } else {
                    hi = if inclusive { v } else { v - 1 };
                }
            }
        }
        XPat::Range(lo, hi)
    }

    fn expr_lit_type(&mut self, l: &Lit) -> Ty {
        // dùng đúng luật kiểu của literal biểu thức (kể cả ghi nhận kiểm phạm vi)
        let e = Expr {
            id: l.id,
            span: l.span,
            kind: ExprKind::Lit(l.clone()),
        };
        self.expr(&e, None)
    }

    fn variant_pat(&mut self, p: &Pattern, vd: DefId, args: &[Pattern], ty: &Ty) -> XPat {
        let parent = self.env.defs()[vd.index()].parent;
        let Some(adt) = parent.and_then(|x| self.env.adts.get(&x)).cloned() else {
            self.bind_all(args);
            return XPat::Wild;
        };
        let AdtKind::Enum(vs) = &adt.kind else {
            self.bind_all(args);
            return XPat::Wild;
        };
        let Some(vi) = vs.iter().position(|v| v.def == vd) else {
            self.bind_all(args);
            return XPat::Wild;
        };
        let targs = fresh_args(&mut self.infer, &adt.generics);
        let aty = Ty::Adt(adt.def, targs.clone());
        if !self.infer.unify(&aty, ty) {
            self.pat_mismatch(p, ty, &aty);
            self.bind_all(args);
            return XPat::Wild;
        }
        let fields: Vec<Ty> = vs[vi]
            .fields
            .iter()
            .map(|f| subst_with(f, &adt.generics, &targs))
            .collect();
        let subs = self.sub_patterns(p, args, &fields, &vs[vi].name);
        XPat::Variant(vi, subs)
    }

    /// Khớp danh sách pattern con (có thể có `..`) với các kiểu trường.
    fn sub_patterns(
        &mut self,
        p: &Pattern,
        args: &[Pattern],
        fields: &[Ty],
        what: &str,
    ) -> Vec<XPat> {
        let rest_at = args
            .iter()
            .position(|a| matches!(a.kind, PatternKind::Rest));
        let fixed = args.len() - usize::from(rest_at.is_some());
        let bad = if rest_at.is_some() {
            fixed > fields.len()
        } else {
            fixed != fields.len()
        };
        if bad {
            self.err(
                "E0304",
                p.id,
                p.span,
                format!("`{what}` có {} trường, pattern có {fixed}", fields.len()),
            );
            self.bind_all(args);
            return vec![XPat::Wild; fields.len()];
        }
        let mut out = vec![XPat::Wild; fields.len()];
        match rest_at {
            None => {
                for (i, a) in args.iter().enumerate() {
                    out[i] = self.pat(a, &fields[i]);
                }
            }
            Some(r) => {
                for (i, a) in args[..r].iter().enumerate() {
                    out[i] = self.pat(a, &fields[i]);
                }
                let tail = args.len() - r - 1;
                for (j, a) in args[r + 1..].iter().enumerate() {
                    let idx = fields.len() - tail + j;
                    out[idx] = self.pat(a, &fields[idx]);
                }
            }
        }
        out
    }

    fn tuple_pat(&mut self, p: &Pattern, v: &[Pattern], ty: &Ty) -> XPat {
        let has_rest = v.iter().any(|a| matches!(a.kind, PatternKind::Rest));
        let elems: Vec<Ty> = match self.infer.shallow(ty) {
            Ty::Tuple(e) => e,
            Ty::Infer(_) if !has_rest => {
                let e: Vec<Ty> = v.iter().map(|_| self.fresh()).collect();
                self.infer.unify(ty, &Ty::Tuple(e.clone()));
                e
            }
            Ty::Err => {
                self.bind_all(v);
                return XPat::Wild;
            }
            other => {
                let want = Ty::Tuple(v.iter().map(|_| Ty::Err).collect());
                let _ = want;
                let s = self.show(&other);
                self.err(
                    "E0301",
                    p.id,
                    p.span,
                    format!("pattern tuple không khớp kiểu `{s}`"),
                );
                self.bind_all(v);
                return XPat::Wild;
            }
        };
        let subs = self.sub_patterns(p, v, &elems, "tuple");
        XPat::Single(subs)
    }

    fn struct_pat(
        &mut self,
        p: &Pattern,
        path: &Path,
        fields: &[FieldPat],
        rest: bool,
        ty: &Ty,
    ) -> XPat {
        let aty = match self.env.res.uses.get(&path.id).cloned() {
            Some(Res::Def(d)) if self.env.adts.contains_key(&d) => {
                let n = self.env.adts[&d].generics.len();
                let args: Vec<Ty> = (0..n).map(|_| self.fresh()).collect();
                Ty::Adt(d, args)
            }
            Some(Res::SelfType) => self.cx.self_ty.clone().unwrap_or(Ty::Err),
            Some(Res::Err) | None => {
                self.bind_fields(fields);
                return XPat::Wild;
            }
            _ => {
                self.err(
                    "E0301",
                    path.id,
                    path.span,
                    "pattern `P { … }` cần tên một struct".to_string(),
                );
                self.bind_fields(fields);
                return XPat::Wild;
            }
        };
        if !self.infer.unify(&aty, ty) {
            self.pat_mismatch(p, ty, &aty);
            self.bind_fields(fields);
            return XPat::Wild;
        }
        let Ty::Adt(d, targs) = self.infer.resolve(&aty) else {
            return XPat::Wild;
        };
        let Some(adt) = self.env.adts.get(&d).cloned() else {
            return XPat::Wild;
        };
        let AdtKind::Struct(defs) = &adt.kind else {
            self.err(
                "E0301",
                p.id,
                p.span,
                "pattern `P { … }` cần struct, không phải enum".to_string(),
            );
            self.bind_fields(fields);
            return XPat::Wild;
        };
        let mut out = vec![XPat::Wild; defs.len()];
        let mut seen: Vec<&str> = Vec::new();
        for f in fields {
            let Some(i) = defs.iter().position(|x| x.name == f.name.name) else {
                let have: Vec<&str> = defs.iter().map(|x| x.name.as_str()).collect();
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
                if let Some(sub) = &f.pattern {
                    let _ = self.pat(sub, &Ty::Err);
                } else {
                    self.declare_local(&f.name, Ty::Err, false);
                }
                continue;
            };
            if seen.contains(&f.name.name.as_str()) {
                self.err(
                    "E0304",
                    f.name.id,
                    f.name.span,
                    format!("trường `{}` xuất hiện nhiều lần trong pattern", f.name.name),
                );
                continue;
            }
            seen.push(&f.name.name);
            if !defs[i].public && !self.is_within(self.module, adt.module) {
                self.err(
                    "E0203",
                    f.name.id,
                    f.name.span,
                    format!("trường `{}` là private (thiếu `◈`)", f.name.name),
                );
            }
            let ft = subst_with(&defs[i].ty, &adt.generics, &targs);
            out[i] = match &f.pattern {
                Some(sub) => self.pat(sub, &ft),
                None => {
                    self.declare_local(&f.name, ft, false);
                    XPat::Wild
                }
            };
        }
        if !rest {
            let missing: Vec<&str> = defs
                .iter()
                .map(|x| x.name.as_str())
                .filter(|n| !seen.contains(n))
                .collect();
            if !missing.is_empty() {
                self.err(
                    "E0304",
                    p.id,
                    p.span,
                    format!(
                        "pattern thiếu trường: {} (thêm `..` để bỏ qua)",
                        missing.join(", ")
                    ),
                );
            }
        }
        XPat::Single(out)
    }

    fn bind_fields(&mut self, fields: &[FieldPat]) {
        for f in fields {
            match &f.pattern {
                Some(sub) => {
                    let _ = self.pat(sub, &Ty::Err);
                }
                None => self.declare_local(&f.name, Ty::Err, false),
            }
        }
    }
}

/// Giải mã literal ký tự (văn bản nguồn giữa hai dấu nháy, có thể có escape đơn giản) thành điểm mã.
fn decode_char(s: &str) -> Option<u32> {
    let inner = s
        .strip_prefix('\'')
        .and_then(|x| x.strip_suffix('\''))
        .unwrap_or(s);
    let mut it = inner.chars();
    let first = it.next()?;
    if first != '\\' {
        return if it.next().is_none() {
            Some(first as u32)
        } else {
            None
        };
    }
    match it.next()? {
        'n' => Some(10),
        't' => Some(9),
        'r' => Some(13),
        '0' => Some(0),
        '\\' => Some(92),
        '\'' => Some(39),
        '"' => Some(34),
        'u' => {
            let hex: String = it.filter(|c| c.is_ascii_hexdigit()).collect();
            u32::from_str_radix(&hex, 16).ok()
        }
        _ => None,
    }
}
