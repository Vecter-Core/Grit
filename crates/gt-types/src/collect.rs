//! Nạp định nghĩa kiểu: ADT, trait, alias, hàm, hằng, impl; hạ kiểu AST → `Ty`; lang item (`#lang`).
use crate::region::{self, RegionRef, RegionSig};
use crate::ty::{ParamId, SELF_INDEX, Ty};
use gt_ast::*;
use gt_diag::{Applicability, Code, Diagnostic, Suggestion};
use gt_resolve::{Def, DefId, DefKind, Res, ResolveOutput};
use gt_span::{FileId, Span};
use std::collections::BTreeMap;

pub(crate) const PRELUDE_FILE: u32 = u32::MAX;

/// Lang item: kiểu/trait mà bộ kiểm kiểu biết đặc biệt (ADR-018, Q21).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lang {
    Option,
    Copy,
    Range,
    /// `a..=b` (gồm đầu mút): khác kiểu với `a..b` để giá trị range không mất thông tin (ADR-022).
    RangeInclusive,
    Secret,
    Cap,
    /// `Drop` (ADR-020 §9, Q26): `⊐ Drop for` chỉ trong lib/core.
    Drop,
    /// `Arena`: vùng nhớ; mượn `&Arena` sinh region "gốc Arena" cho M6 (E0405 khác E0404).
    Arena,
    /// `Vec<'a, T>`: mảng tăng trưởng trong Arena (ADR-032 phụ lục A); literal ngoài lib/core bị cấm (E0313), `T` không được cần drop (E0318).
    Vec,
    /// `Text<'a>`: bộ đệm UTF-8 trên Arena (ADR-035, Q72): bất biến UTF-8 chỉ do intrinsic bảo đảm; literal ngoài lib/core bị cấm (E0313).
    Text,
    /// `Send` (M8): trait TỰ ĐỘNG — kiểu có thể chuyển sang luồng khác; suy cấu trúc (send.rs), cấm viết `⊐ Send for`.
    Send,
    /// Kiểu có MUTABILITY BÊN TRONG (`&self` ghi được): gt-borrowck coi mọi vị trí region bên trong là BẤT BIẾN, kể cả qua `&`
    /// (điều kiện cứng Giang 2026-10-07). Nhóm lang item này (Cell-like, Mutex, Atomic*) thuộc TCB (GRIT_PLAN mục 3.3).
    Cell,
    /// `Mutex<T, L>`: khoá tương hỗ có cấp `L` (kiểu đánh dấu mang `#level`); chỉ truy cập qua `with` (ADR-027).
    Mutex,
    /// `Atomic<T>`: thao tác nguyên tử, mọi thao tác nhận `Ordering` tường minh (ADR-027).
    Atomic,
    /// trait đánh dấu kiểu được phép đặt trong `Atomic<T>` (prelude impl cho bool và số nguyên).
    AtomicInt,
    /// `enum Ordering { Relaxed, Acquire, Release, AcqRel, SeqCst }`.
    AtomicOrdering,
    /// `Sync` (M8): trait TỰ ĐỘNG, suy theo cấu trúc (send.rs), cấm viết `⊐ Sync for`.
    Sync,
}

impl Lang {
    /// Nhóm lang item có MUTABILITY BÊN TRONG (ghi được qua `&`): Cell, Mutex, Atomic (ADR-027 §0).
    #[must_use]
    pub fn interior_mutable(self) -> bool {
        matches!(self, Lang::Cell | Lang::Mutex | Lang::Atomic)
    }

    fn from_name(s: &str) -> Option<Lang> {
        Some(match s {
            "option" => Lang::Option,
            "copy" => Lang::Copy,
            "range" => Lang::Range,
            "range_inclusive" => Lang::RangeInclusive,
            "secret" => Lang::Secret,
            "cap" => Lang::Cap,
            "arena" => Lang::Arena,
            "vec" => Lang::Vec,
            "text" => Lang::Text,
            "drop" => Lang::Drop,
            "send" => Lang::Send,
            "cell" => Lang::Cell,
            "mutex" => Lang::Mutex,
            "atomic" => Lang::Atomic,
            "atomic_int" => Lang::AtomicInt,
            "atomic_ordering" => Lang::AtomicOrdering,
            "sync" => Lang::Sync,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TraitRef {
    pub trait_def: DefId,
}

#[derive(Clone, Debug)]
pub struct GParam {
    pub id: ParamId,
    pub bounds: Vec<TraitRef>,
}

#[derive(Clone, Debug)]
pub struct FieldInfo {
    pub name: String,
    pub ty: Ty,
    pub public: bool,
}

#[derive(Clone, Debug)]
pub struct VariantInfo {
    pub def: DefId,
    pub name: String,
    pub fields: Vec<Ty>,
}

#[derive(Clone, Debug)]
pub enum AdtKind {
    Struct(Vec<FieldInfo>),
    Enum(Vec<VariantInfo>),
}

#[derive(Clone, Debug)]
pub struct AdtDef {
    pub def: DefId,
    /// Số tham số lifetime (`▢ Holder<'a>` → 1): số vị trí region của kiểu này ngoài các đối số kiểu (ADR-021).
    pub lt_params: usize,
    pub generics: Vec<GParam>,
    pub kind: AdtKind,
    pub lang: Option<Lang>,
    pub module: DefId,
    /// cấp khoá từ thuộc tính `#level("n")` (kiểu đánh dấu cho `Mutex<T, L>`; ADR-027 §3)
    pub level: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct FnSigInfo {
    pub node: NodeId,
    pub name: String,
    pub generics: Vec<GParam>,
    pub self_kind: Option<SelfKind>,
    /// Kiểu các tham số có tên (không gồm `self`).
    pub params: Vec<Ty>,
    pub ret: Ty,
    pub effects: Vec<String>,
    pub span: Span,
    /// có `⊳`/`⊲` (không tính biến thể `↓`) trong chữ ký: thành viên trait/impl được gọi qua tham số kiểu không có điểm gọi cụ thể để kiểm (FB-C25)
    pub has_contracts: bool,
}

#[derive(Clone, Debug)]
pub struct TraitDef {
    pub node: NodeId,
    pub members: Vec<FnSigInfo>,
    pub lang: Option<Lang>,
}

#[derive(Clone, Debug)]
pub struct ImplDef {
    pub node: NodeId,
    pub generics: Vec<GParam>,
    pub trait_ref: Option<TraitRef>,
    pub self_ty: Ty,
    pub members: Vec<FnSigInfo>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AliasDef {
    pub generics: Vec<GParam>,
}

pub(crate) struct ItemInfo<'a> {
    pub item: &'a Item,
    pub module: DefId,
}

/// Ngữ cảnh khi hạ kiểu: `Self` nghĩa là gì.
#[derive(Clone, Debug, Default)]
pub struct LowerCx {
    pub self_ty: Option<Ty>,
    /// Lifetime đang trong phạm vi (không gồm dấu `'`); `static` luôn hợp lệ.
    pub lts: Vec<String>,
    /// Cấm `&T` không ghi lifetime (trường struct/enum, alias): elision chỉ có ở chữ ký hàm và thân hàm.
    pub no_elide: bool,
    /// Các vị trí region của kiểu `Self` của impl (theo header `⊐ Holder<'a>`); rỗng ngoài impl.
    pub self_slots: Vec<Option<String>>,
}

pub(crate) struct Env<'a> {
    pub res: &'a ResolveOutput,
    pub names: Vec<String>,
    pub diags: Vec<Diagnostic>,
    pub items: BTreeMap<NodeId, ItemInfo<'a>>,
    pub def_by_node: BTreeMap<NodeId, DefId>,
    pub db: crate::db::TypeDb,
    pub aliases: BTreeMap<DefId, AliasDef>,
    alias_tys: BTreeMap<DefId, Option<Ty>>,
    pub fn_items: Vec<FnBody<'a>>,
    const_depth: u32,
    pub out_types: BTreeMap<NodeId, Ty>,
    pub out_locals: BTreeMap<gt_resolve::LocalId, (String, String, Ty)>,
    pub out_methods: BTreeMap<NodeId, crate::traits::MethodRes>,
    /// Chế độ tham chiếu (ADR-040) tại mỗi pattern phá cấu trúc nơi chế độ BẬT: `NodeId` → `duy nhất?`.
    pub out_pat_modes: BTreeMap<NodeId, bool>,
    /// Ép unsize `&[T; N]` → `&[T]` tại nút (GÓI 12): `NodeId` → đích `&!`?
    pub out_unsize: BTreeMap<NodeId, bool>,
    /// Bảng region của từng chữ ký (M4.5).
    pub region_sigs: BTreeMap<NodeId, RegionSig>,
    pub out_calls: Vec<crate::check::CallRec>,
    pub out_sites: BTreeMap<NodeId, crate::check::CallSite>,
}

/// Một thân hàm cần kiểm.
pub(crate) struct FnBody<'a> {
    pub sig: FnSigInfo,
    pub node: NodeId,
    pub body: Option<&'a Block>,
    pub ast_sig: &'a gt_ast::FnSig,
    pub module: DefId,
    /// Chỉ số trong `impls` nếu là thành viên impl.
    pub impl_idx: Option<usize>,
    /// Lifetime trong phạm vi của thân (của impl/trait chứa và của chính hàm).
    pub lts: Vec<String>,
    pub key: String,
}

impl<'a> Env<'a> {
    pub fn new(res: &'a ResolveOutput, names: Vec<String>) -> Env<'a> {
        Env {
            res,
            names,
            diags: Vec::new(),
            items: BTreeMap::new(),
            def_by_node: BTreeMap::new(),
            db: crate::db::TypeDb::default(),
            aliases: BTreeMap::new(),
            alias_tys: BTreeMap::new(),
            fn_items: Vec::new(),
            const_depth: 0,
            out_types: BTreeMap::new(),
            out_locals: BTreeMap::new(),
            out_methods: BTreeMap::new(),
            out_pat_modes: BTreeMap::new(),
            out_unsize: BTreeMap::new(),
            region_sigs: BTreeMap::new(),
            out_calls: Vec::new(),
            out_sites: BTreeMap::new(),
        }
    }

    pub fn defs(&self) -> &'a [Def] {
        &self.res.defs
    }

    pub fn fname(&self, f: FileId) -> String {
        if f.raw() == PRELUDE_FILE {
            "<prelude>".to_string()
        } else {
            self.names
                .get(f.index())
                .cloned()
                .unwrap_or_else(|| "?".to_string())
        }
    }

    pub fn is_prelude(&self, f: FileId) -> bool {
        f.raw() == PRELUDE_FILE
    }

    pub fn err(&mut self, code: &str, node: NodeId, span: Span, msg: String) -> Option<usize> {
        let c = Code::parse_or_internal(code);
        let d = Diagnostic::new(c, &self.fname(node.file()), span, msg);
        self.diags.push(d);
        Some(self.diags.len() - 1)
    }

    pub fn err_sugg(
        &mut self,
        code: &str,
        node: NodeId,
        span: Span,
        msg: String,
        sugg: Suggestion,
    ) {
        if let Some(i) = self.err(code, node, span, msg) {
            let d = self.diags.remove(i);
            self.diags.push(d.with_suggestion(sugg));
        }
    }

    // ---------------- lập chỉ mục ----------------

    pub fn index(&mut self) {
        for d in self.res.defs.iter().enumerate() {
            self.def_by_node.insert(d.1.node, DefId::from_index(d.0));
        }
        let res: &'a ResolveOutput = self.res;
        for p in &res.programs {
            let Some(&m) = self.def_by_node.get(&p.id) else {
                continue;
            };
            self.index_items(&p.items, m);
        }
        // prelude: module giả `$prelude` (không có node chương trình tương ứng)
        if let Some(p) = &res.prelude_program
            && let Some(m) = res.defs.iter().position(|d| d.path == "$prelude")
        {
            self.index_items(&p.items, DefId::from_index(m));
        }
    }

    fn index_items(&mut self, items: &'a [Item], module: DefId) {
        for item in items {
            self.items.insert(item.id, ItemInfo { item, module });
            if let ItemKind::Mod(m) = &item.kind
                && let Some(&d) = self.def_by_node.get(&item.id)
            {
                self.index_items(&m.items, d);
            }
        }
    }

    /// Cấp khoá từ `#level("n")` (n: số nguyên không âm viết trong chuỗi).
    fn level_of(item: &Item) -> Option<u64> {
        item.attrs.iter().find_map(|a| {
            if a.name.name != "level" {
                return None;
            }
            match a.args.first().map(|x| &x.kind) {
                Some(AttrArgKind::Str(t)) => t.trim().parse().ok(),
                _ => None,
            }
        })
    }

    fn lang_of(&mut self, item: &Item) -> Option<Lang> {
        for a in &item.attrs {
            if a.name.name != "lang" {
                continue;
            }
            if !self.is_prelude(item.id.file()) {
                self.err(
                    "E1002",
                    a.id,
                    a.span,
                    "`#lang` chỉ dùng được trong `lib/core` hoặc prelude (không dùng được trong chương trình thường)".to_string(),
                );
                continue;
            }
            if let Some(AttrArg {
                kind: AttrArgKind::Ident(n),
                ..
            }) = a.args.first()
                && let Some(l) = Lang::from_name(n)
            {
                return Some(l);
            }
        }
        None
    }

    // ---------------- generics ----------------

    fn gparams(&mut self, g: &Option<Generics>, owner: NodeId) -> Vec<GParam> {
        let Some(g) = g else { return Vec::new() };
        let mut out = Vec::new();
        let mut idx = 0u32;
        for p in &g.params {
            if let GenericParamKind::Type { bounds, .. } = &p.kind {
                let mut brefs = Vec::new();
                for b in bounds {
                    match self.res.uses.get(&b.id) {
                        Some(Res::Def(d)) if self.defs()[d.index()].kind == DefKind::Trait => {
                            brefs.push(TraitRef { trait_def: *d });
                        }
                        Some(Res::Err) | None => {}
                        _ => {
                            self.err(
                                "E0309",
                                b.id,
                                b.span,
                                "ràng buộc phải là một trait".to_string(),
                            );
                        }
                    }
                }
                out.push(GParam {
                    id: ParamId { owner, index: idx },
                    bounds: brefs,
                });
                idx += 1;
            }
        }
        out
    }

    // ---------------- hạ kiểu ----------------

    /// Marker hiệu ứng đa hình `poly`/`detach` (ADR-027 §3) chỉ dùng trong lib/core: người dùng không được tự nhận "hiệu ứng tuỳ đối số hàm".
    fn check_marker_effects(&mut self, e: &Effects, node: NodeId, span: Span) {
        if self.is_prelude(node.file()) {
            return;
        }
        if effect_names(e).iter().any(|n| n == "poly" || n == "detach") {
            self.err(
                "E1002",
                node,
                span,
                "hiệu ứng `poly`/`detach` chỉ dùng được trong `lib/core` hoặc prelude".to_string(),
            );
        }
    }

    pub fn lower_ty(&mut self, t: &Type, cx: &LowerCx) -> Ty {
        match &t.kind {
            TypeKind::Prim(p) => Ty::from_prim(*p),
            TypeKind::Never => Ty::Never,
            TypeKind::SelfType => self.self_of(cx, t.id, t.span),
            TypeKind::Path {
                path,
                lifetimes,
                args,
            } => self.lower_path_ty(t, path, lifetimes, args, cx),
            TypeKind::Ref {
                mutable,
                inner,
                lifetime,
            } => {
                self.check_lifetime(lifetime.as_ref(), cx, t);
                Ty::Ref(*mutable, Box::new(self.lower_ty(inner, cx)))
            }
            TypeKind::RawPtr { mutable, inner } => {
                Ty::RawPtr(*mutable, Box::new(self.lower_ty(inner, cx)))
            }
            TypeKind::Slice(inner) => {
                Ty::Slice(Box::new(self.elem_ty(inner, cx, "phần tử của slice")))
            }
            TypeKind::Array { elem, len } => {
                let e = self.elem_ty(elem, cx, "phần tử của mảng");
                match self.eval_usize(len) {
                    Some(n) => Ty::Array(Box::new(e), n),
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
            TypeKind::Tuple(v) => Ty::Tuple(
                v.iter()
                    .map(|x| self.elem_ty(x, cx, "phần tử của tuple"))
                    .collect(),
            ),
            TypeKind::Fn {
                params,
                ret,
                effects,
            } => {
                let p: Vec<Ty> = params
                    .iter()
                    .map(|x| self.elem_ty(x, cx, "tham số của kiểu hàm"))
                    .collect();
                let r = ret.as_ref().map_or_else(Ty::unit, |x| self.lower_ty(x, cx));
                self.check_marker_effects(effects, t.id, t.span);
                Ty::Fn(p, Box::new(r), effect_names(effects))
            }
        }
    }

    /// `&'a T` phải dùng lifetime đã khai báo (hoặc `'static`); `&T` không lifetime bị cấm ở nơi `no_elide` (E0315).
    fn check_lifetime(&mut self, lt: Option<&Lifetime>, cx: &LowerCx, t: &Type) {
        match lt {
            Some(l) => {
                if l.name != "static" && !cx.lts.contains(&l.name) {
                    self.err(
                        "E0315",
                        l.id,
                        l.span,
                        format!(
                            "lifetime `'{}` chưa được khai báo; khai báo ở `<'{}>` của hàm/kiểu chứa nó",
                            l.name, l.name
                        ),
                    );
                }
            }
            None if cx.no_elide => {
                self.err(
                    "E0315",
                    t.id,
                    t.span,
                    "tham chiếu ở đây phải ghi lifetime tường minh (`&'a T`) — elision chỉ dùng ở chữ ký hàm".to_string(),
                );
            }
            None => {}
        }
    }

    /// Lifetime khai báo trong một danh sách generics; trùng tên (kể cả che lifetime của phạm vi ngoài) là E0315.
    pub(crate) fn lifetimes(&mut self, g: &Option<Generics>, outer: &[String]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let Some(g) = g else { return out };
        for p in &g.params {
            if let GenericParamKind::Lifetime(l) = &p.kind {
                if out.contains(&l.name) || outer.contains(&l.name) {
                    self.err(
                        "E0315",
                        l.id,
                        l.span,
                        format!(
                            "lifetime `'{}` đã được khai báo (không che lifetime ngoài)",
                            l.name
                        ),
                    );
                } else {
                    out.push(l.name.clone());
                }
            }
        }
        out
    }

    /// Kiểu ở vị trí không cho phép `!` (tham số kiểu, trường, phần tử): E0314.
    fn elem_ty(&mut self, t: &Type, cx: &LowerCx, what: &str) -> Ty {
        let ty = self.lower_ty(t, cx);
        if ty == Ty::Never {
            self.err(
                "E0314",
                t.id,
                t.span,
                format!("kiểu `!` không hợp lệ làm {what} (kiểu không có giá trị)"),
            );
            return Ty::Err;
        }
        self.sized_ty(ty, t, what)
    }

    /// `[T]`/`str` chỉ hợp lệ sau `&`/`&!` (semantics §1): theo GIÁ TRỊ (tham số, kiểu trả về, trường, phần tử, đối số kiểu) là E0301.
    fn sized_ty(&mut self, ty: Ty, t: &Type, what: &str) -> Ty {
        if matches!(ty, Ty::Slice(_) | Ty::Str) {
            let k = if ty == Ty::Str { "str" } else { "[T]" };
            self.err(
                "E0301",
                t.id,
                t.span,
                format!("kiểu `{k}` không biết kích thước nên không thể là {what} theo giá trị; chỉ dùng được sau `&`/`&!`"),
            );
            return Ty::Err;
        }
        ty
    }

    fn self_of(&mut self, cx: &LowerCx, node: NodeId, span: Span) -> Ty {
        match &cx.self_ty {
            Some(t) => t.clone(),
            None => {
                self.err(
                    "E0301",
                    node,
                    span,
                    "`Self` không có nghĩa ở đây".to_string(),
                );
                Ty::Err
            }
        }
    }

    /// Số tham số lifetime của kiểu mà `path` chỉ tới (struct/enum/alias); 0 với mọi thứ khác.
    pub(crate) fn lt_count_of(&self, path: &Path) -> usize {
        let Some(Res::Def(d)) = self.res.uses.get(&path.id) else {
            return 0;
        };
        let node = self.defs()[d.index()].node;
        let g = match self.items.get(&node).map(|i| &i.item.kind) {
            Some(ItemKind::Struct(s)) => &s.generics,
            Some(ItemKind::Enum(e)) => &e.generics,
            Some(ItemKind::TypeAlias(a)) => &a.generics,
            _ => return 0,
        };
        g.as_ref().map_or(0, |g| {
            g.params
                .iter()
                .filter(|p| matches!(p.kind, GenericParamKind::Lifetime(_)))
                .count()
        })
    }

    /// Đối số lifetime của một kiểu đường dẫn (ADR-021): đã khai báo, đúng số lượng; bỏ trống chỉ được ở nơi cho phép elision.
    fn check_lt_args(&mut self, t: &Type, path: &Path, lifetimes: &[Lifetime], cx: &LowerCx) {
        for l in lifetimes {
            self.check_lifetime(Some(l), cx, t);
        }
        let want = self.lt_count_of(path);
        if !lifetimes.is_empty() && lifetimes.len() != want {
            self.err(
                "E0315",
                t.id,
                t.span,
                format!(
                    "kiểu này có {want} tham số lifetime nhưng nhận {} đối số lifetime",
                    lifetimes.len()
                ),
            );
        } else if lifetimes.is_empty() && want > 0 && cx.no_elide {
            self.err(
                "E0315",
                t.id,
                t.span,
                format!("kiểu này có {want} tham số lifetime; ở đây phải ghi tường minh (`<'a, …>`) — elision chỉ dùng ở chữ ký hàm"),
            );
        }
    }

    fn lower_path_ty(
        &mut self,
        t: &Type,
        path: &Path,
        lifetimes: &[Lifetime],
        args: &[Type],
        cx: &LowerCx,
    ) -> Ty {
        self.check_lt_args(t, path, lifetimes, cx);
        let Some(res) = self.res.uses.get(&path.id).cloned() else {
            return Ty::Err;
        };
        match res {
            Res::Err => Ty::Err,
            Res::Generic(owner, idx) => {
                if !args.is_empty() {
                    self.err(
                        "E0301",
                        t.id,
                        t.span,
                        "tham số kiểu không nhận đối số kiểu".to_string(),
                    );
                }
                Ty::Param(ParamId { owner, index: idx })
            }
            Res::SelfType => self.self_of(cx, t.id, t.span),
            Res::Def(d) => {
                let kind = self.defs()[d.index()].kind;
                match kind {
                    DefKind::Struct | DefKind::Enum => {
                        let want = self.db.adts.get(&d).map_or(0, |a| a.generics.len());
                        let a: Vec<Ty> = args
                            .iter()
                            .map(|x| self.elem_ty(x, cx, "đối số kiểu"))
                            .collect();
                        if a.len() != want {
                            let name = self.defs()[d.index()].path.clone();
                            self.err(
                                "E0301",
                                t.id,
                                t.span,
                                format!("kiểu `{name}` cần {want} đối số kiểu, có {}", a.len()),
                            );
                            return Ty::Err;
                        }
                        Ty::Adt(d, a)
                    }
                    DefKind::TypeAlias => {
                        let a: Vec<Ty> = args
                            .iter()
                            .map(|x| self.elem_ty(x, cx, "đối số kiểu"))
                            .collect();
                        self.expand_alias(d, &a, t)
                    }
                    _ => {
                        let name = self.defs()[d.index()].path.clone();
                        let k = kind.as_str();
                        self.err(
                            "E0301",
                            t.id,
                            t.span,
                            format!("`{name}` là {k}, không phải kiểu"),
                        );
                        Ty::Err
                    }
                }
            }
            Res::Assoc { .. } => {
                self.err(
                    "E0301",
                    t.id,
                    t.span,
                    "kiểu liên kết chưa được hỗ trợ ở Phase 1".to_string(),
                );
                Ty::Err
            }
            // tên của biến/`self` đặt ở vị trí kiểu: PHẢI có chẩn đoán (`Ty::Err` không chẩn đoán bị coi là thoả mọi ràng buộc ở phía sau)
            Res::Local(_) | Res::SelfValue => {
                self.err(
                    "E0301",
                    t.id,
                    t.span,
                    "tên này là biến (hoặc `self`), không phải kiểu".to_string(),
                );
                Ty::Err
            }
        }
    }

    fn expand_alias(&mut self, d: DefId, args: &[Ty], t: &Type) -> Ty {
        let want = self.aliases.get(&d).map_or(0, |a| a.generics.len());
        if args.len() != want {
            let name = self.defs()[d.index()].path.clone();
            self.err(
                "E0301",
                t.id,
                t.span,
                format!("alias `{name}` cần {want} đối số kiểu, có {}", args.len()),
            );
            return Ty::Err;
        }
        if !self.alias_tys.contains_key(&d) {
            self.alias_tys.insert(d, None); // đang hạ: phát hiện vòng
            let node = self.defs()[d.index()].node;
            let ty = match self.items.get(&node).map(|i| &i.item.kind) {
                Some(ItemKind::TypeAlias(a)) => {
                    let ty_ast: &'a Type = match &self.items[&node].item.kind {
                        ItemKind::TypeAlias(a2) => &a2.ty,
                        _ => &a.ty,
                    };
                    let g = match &self.items[&node].item.kind {
                        ItemKind::TypeAlias(a2) => a2.generics.clone(),
                        _ => None,
                    };
                    let lts = self.lifetimes(&g, &[]);
                    let cx = LowerCx {
                        self_ty: None,
                        lts,
                        no_elide: true,
                        self_slots: Vec::new(),
                    };
                    self.lower_ty(ty_ast, &cx)
                }
                _ => Ty::Err,
            };
            self.alias_tys.insert(d, Some(ty));
        }
        match self.alias_tys.get(&d).cloned() {
            Some(Some(body)) => {
                let params: Vec<ParamId> = self
                    .aliases
                    .get(&d)
                    .map(|a| a.generics.iter().map(|g| g.id).collect())
                    .unwrap_or_default();
                body.subst(&|p| params.iter().position(|q| *q == p).map(|i| args[i].clone()))
            }
            _ => {
                let name = self.defs()[d.index()].path.clone();
                self.err(
                    "E0301",
                    t.id,
                    t.span,
                    format!("alias `{name}` tham chiếu vòng quanh chính nó"),
                );
                Ty::Err
            }
        }
    }

    // ---------------- hằng ----------------

    /// Đánh giá hằng nguyên không âm (độ dài mảng): literal, hằng `□`, và `+ - *` của chúng.
    pub fn eval_usize(&mut self, e: &Expr) -> Option<u64> {
        self.const_depth += 1;
        let r = if self.const_depth > 32 {
            None
        } else {
            self.eval_usize_inner(e)
        };
        self.const_depth -= 1;
        r
    }

    fn eval_usize_inner(&mut self, e: &Expr) -> Option<u64> {
        match &e.kind {
            ExprKind::Lit(l) if !l.negative => match &l.kind {
                LitKind::Int(s) => parse_int_literal(s).and_then(|(v, _)| u64::try_from(v).ok()),
                _ => None,
            },
            ExprKind::Path(p) => {
                let Some(Res::Def(d)) = self.res.uses.get(&p.id).cloned() else {
                    return None;
                };
                let node = self.defs()[d.index()].node;
                let item: &'a Item = self.items.get(&node)?.item;
                match &item.kind {
                    ItemKind::Const(c) => self.eval_usize(&c.value),
                    _ => None,
                }
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let (a, b) = (self.eval_usize(lhs)?, self.eval_usize(rhs)?);
                match op {
                    BinaryOp::Add => a.checked_add(b),
                    BinaryOp::Sub => a.checked_sub(b),
                    BinaryOp::Mul => a.checked_mul(b),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    // ---------------- thu thập ----------------

    pub fn collect(&mut self) {
        let nodes: Vec<NodeId> = self.items.keys().copied().collect();
        // 1. vỏ: generics của ADT/trait/alias và lang item
        for n in &nodes {
            let item: &'a Item = self.items[n].item;
            let module = self.items[n].module;
            let lang = self.lang_of(item);
            match &item.kind {
                ItemKind::Struct(s) => {
                    if let Some(&d) = self.def_by_node.get(n) {
                        let g = self.gparams(&s.generics, item.id);
                        let lt_params = count_lifetimes(&s.generics);
                        self.db.adts.insert(
                            d,
                            AdtDef {
                                def: d,
                                lt_params,
                                generics: g,
                                kind: AdtKind::Struct(Vec::new()),
                                lang,
                                module,
                                level: Self::level_of(item),
                            },
                        );
                        if let Some(l) = lang {
                            self.db.lang.insert(l, d);
                        }
                    }
                }
                ItemKind::Enum(e) => {
                    if let Some(&d) = self.def_by_node.get(n) {
                        let g = self.gparams(&e.generics, item.id);
                        let lt_params = count_lifetimes(&e.generics);
                        self.db.adts.insert(
                            d,
                            AdtDef {
                                def: d,
                                lt_params,
                                generics: g,
                                kind: AdtKind::Enum(Vec::new()),
                                lang,
                                module,
                                level: Self::level_of(item),
                            },
                        );
                        if let Some(l) = lang {
                            self.db.lang.insert(l, d);
                        }
                    }
                }
                ItemKind::Trait(t) => {
                    if let Some(&d) = self.def_by_node.get(n) {
                        // (chỉ để kiểm ràng buộc; generics của trait chưa dùng được — KNOWN_LIMITATIONS)
                        self.gparams(&t.generics, item.id);
                        self.db.traits.insert(
                            d,
                            TraitDef {
                                node: item.id,
                                members: Vec::new(),
                                lang,
                            },
                        );
                        if let Some(l) = lang {
                            self.db.lang.insert(l, d);
                        }
                    }
                }
                ItemKind::TypeAlias(a) => {
                    if let Some(&d) = self.def_by_node.get(n) {
                        let g = self.gparams(&a.generics, item.id);
                        self.aliases.insert(d, AliasDef { generics: g });
                    }
                }
                _ => {}
            }
        }
        // 2. nội dung
        for n in &nodes {
            let item: &'a Item = self.items[n].item;
            let module = self.items[n].module;
            match &item.kind {
                ItemKind::Struct(s) => self.fill_struct(item, s),
                ItemKind::Enum(e) => self.fill_enum(item, e),
                ItemKind::Trait(t) => self.fill_trait(item, t),
                ItemKind::Fn(f) => self.fill_fn(item, f, module),
                ItemKind::Const(c) => {
                    let ty = self.lower_ty(&c.ty, &LowerCx::default());
                    if let Some(&d) = self.def_by_node.get(n) {
                        self.db.consts.insert(d, ty);
                    }
                }
                ItemKind::Static(c) => {
                    let ty = self.lower_ty(&c.ty, &LowerCx::default());
                    if let Some(&d) = self.def_by_node.get(n) {
                        self.db.consts.insert(d, ty);
                    }
                }
                ItemKind::Impl(i) => self.fill_impl(item, i, module),
                ItemKind::TypeAlias(_) | ItemKind::Mod(_) | ItemKind::Use(_) => {}
            }
        }
        // 3. kiểu có kích thước vô hạn (chứa chính nó THEO GIÁ TRỊ): E0301
        for n in &nodes {
            let item: &'a Item = self.items[n].item;
            if !matches!(item.kind, ItemKind::Struct(_) | ItemKind::Enum(_)) {
                continue;
            }
            if let Some(&d) = self.def_by_node.get(n)
                && self.contains_itself_by_value(d)
            {
                let name = self.defs()[d.index()].path.clone();
                self.err(
                    "E0301",
                    item.id,
                    item.span,
                    format!("kiểu `{name}` chứa chính nó theo giá trị nên có kích thước vô hạn; đặt vòng đệ quy sau một tham chiếu"),
                );
            }
        }
    }

    /// `d` có (trực tiếp hay qua trường/biến thể/tuple/mảng/đối số kiểu của ADT khác) một giá trị của chính `d` không? Tham chiếu và con trỏ
    /// thô cắt vòng. Đệ quy đa hình theo giá trị (`▢ A<T> { x: A<Option<T>> }`) bung vô hạn ⇒ vượt `DEPTH` cũng là vòng.
    fn contains_itself_by_value(&self, root: DefId) -> bool {
        const DEPTH: usize = 32;
        fn walk(
            db: &crate::db::TypeDb,
            root: DefId,
            ty: &Ty,
            seen: &mut std::collections::BTreeSet<(DefId, Vec<Ty>)>,
            depth: usize,
        ) -> bool {
            match ty {
                Ty::Tuple(v) => v.iter().any(|t| walk(db, root, t, seen, depth)),
                Ty::Array(t, _) => walk(db, root, t, seen, depth),
                Ty::Adt(d, args) => {
                    if *d == root {
                        return true;
                    }
                    if depth >= DEPTH {
                        return true;
                    }
                    if !seen.insert((*d, args.clone())) {
                        return false;
                    }
                    let Some(a) = db.adts.get(d) else {
                        // ADT không có trong db: không biết ⇒ coi là chứa chính nó (E0301, bảo thủ), không im lặng cho qua (FAIL_OPEN_AUDIT G2.1.b)
                        return true;
                    };
                    let sub = |t: &Ty| crate::traits::subst_with(t, &a.generics, args);
                    match &a.kind {
                        AdtKind::Struct(fs) => fs
                            .iter()
                            .any(|f| walk(db, root, &sub(&f.ty), seen, depth + 1)),
                        AdtKind::Enum(vs) => vs.iter().any(|v| {
                            v.fields
                                .iter()
                                .any(|t| walk(db, root, &sub(t), seen, depth + 1))
                        }),
                    }
                }
                _ => false,
            }
        }
        let Some(a) = self.db.adts.get(&root) else {
            return false;
        };
        let mut seen = std::collections::BTreeSet::new();
        let fields: Vec<&Ty> = match &a.kind {
            AdtKind::Struct(fs) => fs.iter().map(|f| &f.ty).collect(),
            AdtKind::Enum(vs) => vs.iter().flat_map(|v| v.fields.iter()).collect(),
        };
        fields
            .into_iter()
            .any(|t| walk(&self.db, root, t, &mut seen, 0))
    }

    fn fill_struct(&mut self, item: &'a Item, s: &'a StructItem) {
        let Some(&d) = self.def_by_node.get(&item.id) else {
            return;
        };
        let lts = self.lifetimes(&s.generics, &[]);
        let cx = LowerCx {
            self_ty: None,
            lts,
            no_elide: true,
            self_slots: Vec::new(),
        };
        let fields: Vec<FieldInfo> = s
            .fields
            .iter()
            .map(|f| FieldInfo {
                name: f.name.name.clone(),
                ty: self.elem_ty(&f.ty, &cx, "trường"),
                public: f.vis == Visibility::Public,
            })
            .collect();
        if let Some(a) = self.db.adts.get_mut(&d) {
            a.kind = AdtKind::Struct(fields);
        }
    }

    fn fill_enum(&mut self, item: &'a Item, e: &'a EnumItem) {
        let Some(&d) = self.def_by_node.get(&item.id) else {
            return;
        };
        let lts = self.lifetimes(&e.generics, &[]);
        let cx = LowerCx {
            self_ty: None,
            lts,
            no_elide: true,
            self_slots: Vec::new(),
        };
        let mut vs = Vec::new();
        for v in &e.variants {
            let Some(&vd) = self.def_by_node.get(&v.id) else {
                continue;
            };
            let fields: Vec<Ty> = v
                .fields
                .iter()
                .map(|t| self.elem_ty(t, &cx, "trường của biến thể"))
                .collect();
            vs.push(VariantInfo {
                def: vd,
                name: v.name.name.clone(),
                fields,
            });
        }
        if let Some(a) = self.db.adts.get_mut(&d) {
            a.kind = AdtKind::Enum(vs);
        }
    }

    fn sig_info(&mut self, sig: &'a gt_ast::FnSig, owner: NodeId, cx0: &LowerCx) -> FnSigInfo {
        let generics = self.gparams(&sig.generics, owner);
        let declared = self.lifetimes(&sig.generics, &cx0.lts);
        let mut cx = cx0.clone();
        cx.lts.extend(declared.iter().cloned());
        let cx = &cx;
        let mut self_kind = None;
        let mut params = Vec::new();
        let mut anon = 0u32;
        let mut inputs: Vec<Vec<RegionRef>> = Vec::new();
        let mut input_self: Vec<Vec<bool>> = Vec::new();
        // vị trí đầu vào dùng cho elision (không tính vị trí đến từ `Self`)
        let mut elide_in: Vec<RegionRef> = Vec::new();
        // các vị trí của kiểu `Self` (impl) — mọi chỗ `Self` trong chữ ký dùng CHUNG các region này
        let self_refs: Vec<RegionRef> = cx
            .self_slots
            .iter()
            .map(|s| region::region_of(s, &mut anon))
            .collect();
        let mut self_i = 0usize;
        for p in &sig.params {
            match &p.kind {
                ParamKind::SelfParam(k) => {
                    self_kind = Some(*k);
                    // `&self`/`&!self`: một vị trí region ẩn (đếm cho elision); rồi các vị trí của kiểu `Self` của impl
                    let mut rs = Vec::new();
                    let mut fs = Vec::new();
                    if *k != SelfKind::Value {
                        let r = RegionRef::Anon(anon);
                        anon += 1;
                        elide_in.push(r.clone());
                        rs.push(r);
                        fs.push(false);
                    }
                    for r in &self_refs {
                        rs.push(r.clone());
                        fs.push(true);
                    }
                    inputs.push(rs);
                    input_self.push(fs);
                }
                ParamKind::Named { ty, .. } => {
                    let pt = self.lower_ty(ty, cx);
                    params.push(self.sized_ty(pt, ty, "tham số"));
                    let mut sl = Vec::new();
                    region::slots(ty, &mut sl, &|p| self.lt_count_of(p), &cx.self_slots);
                    let mut rs = Vec::new();
                    let mut fs = Vec::new();
                    for (name, from_self) in &sl {
                        let r = if *from_self && !self_refs.is_empty() {
                            let r = self_refs[self_i % self_refs.len()].clone();
                            self_i += 1;
                            r
                        } else {
                            region::region_of(name, &mut anon)
                        };
                        if !from_self {
                            elide_in.push(r.clone());
                        }
                        rs.push(r);
                        fs.push(*from_self);
                    }
                    inputs.push(rs);
                    input_self.push(fs);
                }
            }
        }
        let ret = match &sig.ret {
            Some(t) => {
                let rt = self.lower_ty(t, cx);
                self.sized_ty(rt, t, "kiểu trả về")
            }
            None => Ty::unit(),
        };
        // elision của kiểu trả về (semantics §5): có đúng MỘT vị trí tham chiếu đầu vào thì mọi `&` ẩn ở đầu ra nhận region đó
        let all_in = elide_in;
        let mut output = Vec::new();
        let mut output_self: Vec<bool> = Vec::new();
        if let Some(rt) = &sig.ret {
            let mut sl = Vec::new();
            region::slots(rt, &mut sl, &|p| self.lt_count_of(p), &cx.self_slots);
            let mut reported = false;
            let mut out_self_i = 0usize;
            for (x, from_self) in &sl {
                if *from_self && !self_refs.is_empty() {
                    output.push(self_refs[out_self_i % self_refs.len()].clone());
                    output_self.push(true);
                    out_self_i += 1;
                    continue;
                }
                output_self.push(false);
                match x {
                    Some(_) => output.push(region::region_of(x, &mut 0)),
                    None if all_in.len() == 1 => output.push(all_in[0].clone()),
                    // không có tham chiếu đầu vào: chỉ có thể trả dữ liệu `'static` (vd literal chuỗi); M6 kiểm thân hàm
                    None if all_in.is_empty() => output.push(RegionRef::Static),
                    None => {
                        if !reported {
                            reported = true;
                            self.err(
                                "E0315",
                                rt.id,
                                rt.span,
                                format!(
                                    "không suy ra được lifetime của kiểu trả về: có {} vị trí tham chiếu ở đầu vào (cần đúng một, hoặc không có để mặc định `'static`); ghi `'r` tường minh",
                                    all_in.len()
                                ),
                            );
                        }
                    }
                }
            }
        }
        self.region_sigs.insert(
            owner,
            RegionSig {
                declared,
                inputs,
                output,
                input_self,
                output_self,
            },
        );
        FnSigInfo {
            node: owner,
            name: sig.name.name.clone(),
            generics,
            self_kind,
            params,
            ret,
            effects: effect_names(&sig.effects),
            span: sig.name.span,
            has_contracts: sig
                .contracts
                .iter()
                .any(|c| c.kind != gt_ast::ContractKind::Variant),
        }
    }

    fn fill_fn(&mut self, item: &'a Item, f: &'a FnItem, module: DefId) {
        let Some(&d) = self.def_by_node.get(&item.id) else {
            return;
        };
        self.check_marker_effects(&f.sig.effects, item.id, item.span);
        let info = self.sig_info(&f.sig, item.id, &LowerCx::default());
        let lts = self.region_sigs[&item.id].declared.clone();
        self.db.fns.insert(d, (info.clone(), module));
        let key = self.defs()[d.index()].path.clone();
        self.fn_items.push(FnBody {
            sig: info,
            node: item.id,
            body: Some(&f.body),
            ast_sig: &f.sig,
            module,
            impl_idx: None,
            lts,
            key,
        });
    }

    fn fill_trait(&mut self, item: &'a Item, t: &'a TraitItem) {
        let Some(&d) = self.def_by_node.get(&item.id) else {
            return;
        };
        let self_ty = Ty::Param(ParamId {
            owner: item.id,
            index: SELF_INDEX,
        });
        let tlts = self.lifetimes(&t.generics, &[]);
        let cx = LowerCx {
            self_ty: Some(self_ty),
            lts: tlts.clone(),
            no_elide: false,
            self_slots: Vec::new(),
        };
        let mut members = Vec::new();
        let tkey = self.defs()[d.index()].path.clone();
        for m in &t.members {
            let info = self.sig_info(&m.sig, m.id, &cx);
            let mut lts = tlts.clone();
            lts.extend(self.region_sigs[&m.id].declared.iter().cloned());
            members.push(info.clone());
            let module = self.items[&item.id].module;
            self.fn_items.push(FnBody {
                sig: info,
                node: m.id,
                body: None,
                ast_sig: &m.sig,
                module,
                impl_idx: None,
                lts,
                key: format!("{tkey}::{}", m.sig.name.name),
            });
        }
        if let Some(td) = self.db.traits.get_mut(&d) {
            td.members = members;
        }
    }

    fn fill_impl(&mut self, item: &'a Item, i: &'a ImplItem, module: DefId) {
        let generics = self.gparams(&i.generics, item.id);
        let ilts = self.lifetimes(&i.generics, &[]);
        let cx0 = LowerCx {
            self_ty: None,
            lts: ilts.clone(),
            no_elide: false,
            self_slots: Vec::new(),
        };
        let self_ty = self.lower_ty(&i.self_ty, &cx0);
        let trait_ref = match &i.trait_path {
            Some(p) => match self.res.uses.get(&p.id) {
                Some(Res::Def(d)) if self.defs()[d.index()].kind == DefKind::Trait => {
                    Some(TraitRef { trait_def: *d })
                }
                Some(Res::Err) | None => None,
                _ => {
                    self.err("E0311", p.id, p.span, "`⊐ … for` cần một trait".to_string());
                    None
                }
            },
            None => None,
        };
        let mut self_sl: Vec<region::Slot> = Vec::new();
        region::slots(&i.self_ty, &mut self_sl, &|p| self.lt_count_of(p), &[]);
        let cx = LowerCx {
            self_ty: Some(self_ty.clone()),
            lts: ilts.clone(),
            no_elide: false,
            self_slots: self_sl.into_iter().map(|(n, _)| n).collect(),
        };
        let idx = self.db.impls.len();
        let mut members = Vec::new();
        // khoá ổn định (không phụ thuộc NodeId): `module::Kiểu::hàm` hoặc `module::<Kiểu as Trait>::hàm`
        let mpath = self.defs()[module.index()].path.clone();
        let sname = self_ty.render(self.defs());
        let ikey = match &trait_ref {
            Some(tr) => {
                let tn = self.defs()[tr.trait_def.index()].path.clone();
                format!("{mpath}::<{sname} as {tn}>")
            }
            None => format!("{mpath}::{sname}"),
        };
        for m in &i.members {
            let info = self.sig_info(&m.func.sig, m.id, &cx);
            let mut lts = ilts.clone();
            lts.extend(self.region_sigs[&m.id].declared.iter().cloned());
            members.push(info.clone());
            self.fn_items.push(FnBody {
                sig: info,
                node: m.id,
                body: Some(&m.func.body),
                ast_sig: &m.func.sig,
                module,
                impl_idx: Some(idx),
                lts,
                key: format!("{ikey}::{}", m.func.sig.name.name),
            });
        }
        self.db.impls.push(ImplDef {
            node: item.id,
            generics,
            trait_ref,
            self_ty,
            members,
            span: item.span,
        });
    }
}

pub(crate) fn effect_names(e: &Effects) -> Vec<String> {
    let mut v: Vec<String> = match &e.kind {
        EffectsKind::Pure => Vec::new(),
        EffectsKind::Set(s) => s
            .iter()
            .map(|x| match &x.arg {
                Some(a) => format!("{}({})", x.name.name, a.name),
                None => x.name.name.clone(),
            })
            .collect(),
    };
    v.sort();
    v
}

/// Phân tích literal số nguyên: trả (giá trị, hậu tố). `_` bị bỏ; hỗ trợ `0x`, `0o`, `0b`.
pub(crate) fn parse_int_literal(s: &str) -> Option<(u128, Option<String>)> {
    let cleaned: String = s.chars().filter(|c| *c != '_').collect();
    let (radix, rest) = if let Some(r) = cleaned.strip_prefix("0x") {
        (16, r)
    } else if let Some(r) = cleaned.strip_prefix("0o") {
        (8, r)
    } else if let Some(r) = cleaned.strip_prefix("0b") {
        (2, r)
    } else {
        (10, cleaned.as_str())
    };
    // tách hậu tố: i8.. u128 isize usize (với hex, chữ `i`/`u` không phải chữ số hex nên cắt được)
    let mut split = rest.len();
    for suf in [
        "i128", "u128", "isize", "usize", "i16", "i32", "i64", "u16", "u32", "u64", "i8", "u8",
    ] {
        if let Some(pos) = rest.rfind(suf)
            && pos + suf.len() == rest.len()
            && pos < split
        {
            split = pos;
        }
    }
    let (digits, suffix) = rest.split_at(split);
    let v = u128::from_str_radix(digits, radix).ok()?;
    Some((v, (!suffix.is_empty()).then(|| suffix.to_string())))
}

/// Gợi ý chèn ` as T` sau một biểu thức (ép kiểu tường minh).
pub(crate) fn as_suggestion(file_name: &str, end: u32, ty: &str) -> Suggestion {
    Suggestion::new(
        format!("ép kiểu tường minh: ` as {ty}`"),
        Applicability::MaybeIncorrect,
        vec![gt_diag::Edit {
            file: file_name.to_string(),
            span: Span::new(end, end),
            replacement: format!(" as {ty}"),
        }],
    )
}

/// Số tham số lifetime trong một danh sách generics.
pub(crate) fn count_lifetimes(g: &Option<Generics>) -> usize {
    g.as_ref().map_or(0, |g| {
        g.params
            .iter()
            .filter(|p| matches!(p.kind, GenericParamKind::Lifetime(_)))
            .count()
    })
}

#[cfg(test)]
mod err_tests {
    use crate::testkit::with_env;
    use gt_ast::NodeId;
    use gt_span::{FileId, Span};

    /// FAIL_OPEN_AUDIT S-C: `Env::err` với mã sai định dạng không được trả `None` (chẩn đoán biến mất).
    #[test]
    fn err_with_a_malformed_code_still_records_a_diagnostic() {
        with_env("ƒ f() → u32 ! ∅ { 1 }", |env| {
            let n = env.diags.len();
            let idx = env.err(
                "E03",
                NodeId::new(FileId::new(0), 0),
                Span::new(0, 0),
                "thử".to_string(),
            );
            assert!(idx.is_some(), "chẩn đoán bị nuốt");
            assert_eq!(env.diags.len(), n + 1);
            assert_eq!(env.diags[n].code.as_str(), "E9901");
        });
    }
}
