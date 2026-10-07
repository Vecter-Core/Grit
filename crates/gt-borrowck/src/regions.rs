//! Biến region và ràng buộc outlives của một thân MIR (ADR-023 §1, §4, §10–11).
//!
//! `a : b` nghĩa là `a` outlives `b`: giá trị(`b`) ⊆ giá trị(`a`). Mỗi vị trí region của một place là một "bó" (`Vec<usize>`)
//! — thường chỉ một biến; trường của ADT là bó chứa MỌI region của ADT (Ty không có ánh xạ trường → tham số lifetime), nên
//! khi đọc trường mọi region của ADT outlive giá trị đọc, khi ghi giá trị outlive mọi region của ADT (chặt hơn, sound).
use gt_mir::types::{const_ty, region_slots};
use gt_mir::*;
use gt_span::Span;
use gt_types::{AdtKind, GParam, ParamId, RegionRef, SELF_INDEX, Ty, TypeDb, TypesOutput};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

/// BỘ ĐẾM ĐO ĐẠC cho test (không ảnh hưởng kết quả): số lần đường "bảo thủ" dự phòng được dùng cho lời gọi `Fn`/`TraitMethod`
/// (chữ ký không ánh xạ được) và số lần `relate` gặp số vị trí region không khớp. Với chương trình qua M4 cả hai phải bằng 0 —
/// các nhánh `return false`/kiểm độ dài trong `call_sig`/`walk`/`relate` chỉ là chốt chặn phòng thủ (xem ADR-023 §10a, đột biến).
static FALLBACKS: AtomicUsize = AtomicUsize::new(0);
static RELATE_MISMATCHES: AtomicUsize = AtomicUsize::new(0);

/// `(số lần dùng đường dự phòng cho Fn/TraitMethod, số lần relate lệch độ dài)` từ đầu tiến trình.
#[doc(hidden)]
#[must_use]
pub fn defensive_counters() -> (usize, usize) {
    (
        FALLBACKS.load(Ordering::Relaxed),
        RELATE_MISMATCHES.load(Ordering::Relaxed),
    )
}

pub(crate) type Slots = Vec<Vec<usize>>;

#[derive(Clone, Debug)]
pub(crate) struct Cons {
    pub a: usize,
    pub b: usize,
    pub at: Option<Location>,
    pub span: Span,
}

pub(crate) struct Gen<'a> {
    pub t: &'a TypesOutput,
    pub db: &'a TypeDb,
    pub body: &'a Body,
    pub n: usize,
    pub static_r: usize,
    /// Region phổ quát (chữ ký của chính hàm này): biến → tên. Phần tử 0 là `'static`.
    pub univ: Vec<(usize, RegionRef)>,
    univ_map: BTreeMap<RegionRef, usize>,
    pub cons: Vec<Cons>,
    adt_inv: BTreeMap<gt_resolve::DefId, bool>,
    cur: (Option<Location>, Span),
}

fn flat(s: &Slots) -> Vec<usize> {
    let mut v: Vec<usize> = s.iter().flatten().copied().collect();
    v.sort_unstable();
    v.dedup();
    v
}

impl<'a> Gen<'a> {
    pub fn new(t: &'a TypesOutput, body: &'a Body) -> Gen<'a> {
        let n = body.region_count as usize;
        let mut g = Gen {
            t,
            db: &t.db,
            body,
            n: n + 1,
            static_r: n,
            univ: vec![(n, RegionRef::Static)],
            univ_map: BTreeMap::new(),
            cons: Vec::new(),
            adt_inv: BTreeMap::new(),
            cur: (None, body.span),
        };
        g.univ_map.insert(RegionRef::Static, n);
        g.setup_universals();
        g
    }

    pub fn fresh(&mut self) -> usize {
        self.n += 1;
        self.n - 1
    }

    fn universal(&mut self, r: &RegionRef) -> usize {
        if let Some(v) = self.univ_map.get(r) {
            return *v;
        }
        let v = self.fresh();
        self.univ_map.insert(r.clone(), v);
        self.univ.push((v, r.clone()));
        v
    }

    fn alias(&mut self, a: usize, b: usize) {
        self.add(a, b);
        self.add(b, a);
    }

    pub fn add(&mut self, a: usize, b: usize) {
        if a != b {
            self.cons.push(Cons {
                a,
                b,
                at: self.cur.0,
                span: self.cur.1,
            });
        }
    }

    /// `A : B` cho mọi cặp.
    fn outl(&mut self, a: &[usize], b: &[usize]) {
        for x in a {
            for y in b {
                self.add(*x, *y);
            }
        }
    }

    fn setup_universals(&mut self) {
        let rsig = self.t.regions.get(&self.body.node).cloned();
        let body = self.body;
        for i in 1..=body.arg_count {
            let regs = &body.locals[i].regions;
            let refs: &[RegionRef] = rsig
                .as_ref()
                .and_then(|s| s.inputs.get(i - 1))
                .map_or(&[], |v| v.as_slice());
            for (k, r) in regs.iter().enumerate() {
                let u = match refs.get(k) {
                    Some(rr) => self.universal(rr),
                    None => {
                        // không khớp chữ ký (không nên xảy ra): coi là một region phổ quát riêng
                        let v = self.fresh();
                        self.univ.push((v, RegionRef::Anon(10_000 + v as u32)));
                        v
                    }
                };
                self.alias(r.0 as usize, u);
            }
        }
        let outs: Vec<RegionRef> = rsig.map_or_else(Vec::new, |s| s.output.clone());
        for (k, r) in body.locals[0].regions.iter().enumerate() {
            let u = if body.kind == BodyKind::Fn {
                match outs.get(k) {
                    Some(rr) => self.universal(rr),
                    None => {
                        let v = self.fresh();
                        self.univ.push((v, RegionRef::Anon(20_000 + v as u32)));
                        v
                    }
                }
            } else {
                self.static_r
            };
            self.alias(r.0 as usize, u);
        }
    }

    // ---------------- place / operand ----------------

    fn nslots(&self, ty: &Ty) -> usize {
        region_slots(self.db, ty)
    }

    fn local_slots(&self, l: Local) -> Slots {
        self.body.locals[l.index()]
            .regions
            .iter()
            .map(|r| vec![r.0 as usize])
            .collect()
    }

    /// Slots của một chỗ và kiểu của nó. `derefs`: với mỗi `Deref` của một tham chiếu, (bó region của tham chiếu, là `&`?).
    pub fn place_slots(&self, p: &Place) -> (Slots, Ty, Vec<(Vec<usize>, bool)>) {
        let mut slots = self.local_slots(p.local);
        let mut ty = self.body.locals[p.local.index()].ty.clone();
        let mut derefs = Vec::new();
        for e in &p.projection {
            match (e, ty.clone()) {
                (PlaceElem::Deref, Ty::Ref(m, inner)) => {
                    let first = slots.first().cloned().unwrap_or_default();
                    derefs.push((first, !m));
                    slots = slots.into_iter().skip(1).collect();
                    ty = *inner;
                }
                (PlaceElem::Deref, Ty::RawPtr(_, inner)) => ty = *inner,
                (PlaceElem::Field(i, fty), Ty::Tuple(ts)) => {
                    let off: usize = ts.iter().take(*i as usize).map(|t| self.nslots(t)).sum();
                    let n = self.nslots(fty);
                    slots = slots.into_iter().skip(off).take(n).collect();
                    ty = fty.clone();
                }
                (PlaceElem::Field(_, fty), Ty::Adt(..)) => {
                    let mut bundle = flat(&slots);
                    if bundle.is_empty() {
                        bundle = vec![self.static_r];
                    }
                    slots = vec![bundle; self.nslots(fty)];
                    ty = fty.clone();
                }
                (PlaceElem::Downcast(_), t) => ty = t,
                (PlaceElem::Index(_), Ty::Array(t, _) | Ty::Slice(t)) => ty = *t,
                (PlaceElem::Subslice { .. }, Ty::Array(t, _) | Ty::Slice(t)) => {
                    ty = Ty::Slice(t);
                }
                _ => {}
            }
        }
        (slots, ty, derefs)
    }

    fn operand_slots(&self, o: &Operand) -> Slots {
        match o {
            Operand::Copy(p) | Operand::Move(p) => self.place_slots(p).0,
            Operand::Const(c) => vec![vec![self.static_r]; self.nslots(&const_ty(c))],
        }
    }

    fn adt_invariant(&mut self, d: gt_resolve::DefId) -> bool {
        if let Some(v) = self.adt_inv.get(&d) {
            return *v;
        }
        self.adt_inv.insert(d, false); // chặn vòng
        let tys: Vec<Ty> = match self.db.adts.get(&d).map(|a| &a.kind) {
            Some(AdtKind::Struct(fs)) => fs.iter().map(|f| f.ty.clone()).collect(),
            Some(AdtKind::Enum(vs)) => vs.iter().flat_map(|v| v.fields.clone()).collect(),
            None => Vec::new(),
        };
        let inv = tys.iter().any(|t| self.ty_invariant(t));
        self.adt_inv.insert(d, inv);
        inv
    }

    fn ty_invariant(&mut self, t: &Ty) -> bool {
        match t {
            Ty::Ref(true, _) | Ty::RawPtr(..) | Ty::Fn(..) => true,
            Ty::Ref(false, i) | Ty::Array(i, _) | Ty::Slice(i) => self.ty_invariant(i),
            Ty::Tuple(v) => v.iter().any(|x| self.ty_invariant(x)),
            Ty::Adt(d, args) => self.adt_invariant(*d) || args.iter().any(|x| self.ty_invariant(x)),
            _ => false,
        }
    }

    /// `src` → `dst` (src outlives dst ở vị trí hiệp biến; hai chiều ở vị trí bất biến).
    fn relate(&mut self, ty: &Ty, src: &[Vec<usize>], dst: &[Vec<usize>], inv: bool) {
        if src.len() != dst.len() {
            RELATE_MISMATCHES.fetch_add(1, Ordering::Relaxed);
            let (a, b) = (flat(&src.to_vec()), flat(&dst.to_vec()));
            self.outl(&a, &b);
            return;
        }
        match ty {
            Ty::Ref(m, inner) => {
                if src.is_empty() {
                    return;
                }
                self.outl(&src[0], &dst[0]);
                if inv {
                    self.outl(&dst[0], &src[0]);
                }
                self.relate(inner, &src[1..], &dst[1..], inv || *m);
            }
            Ty::Tuple(ts) => {
                let mut o = 0;
                for t in ts {
                    let n = self.nslots(t);
                    self.relate(t, &src[o..o + n], &dst[o..o + n], inv);
                    o += n;
                }
            }
            Ty::Array(t, _) | Ty::Slice(t) => self.relate(t, src, dst, inv),
            Ty::RawPtr(_, t) => self.relate(t, src, dst, true),
            Ty::Adt(d, args) => {
                let lt = self.db.adts.get(d).map_or(0, |a| a.lt_params);
                let invariant = inv || self.adt_invariant(*d);
                for i in 0..lt.min(src.len()) {
                    self.outl(&src[i], &dst[i]);
                    if invariant {
                        self.outl(&dst[i], &src[i]);
                    }
                }
                let mut o = lt;
                for a in args {
                    let n = self.nslots(a);
                    if o + n <= src.len() {
                        self.relate(a, &src[o..o + n], &dst[o..o + n], invariant);
                    }
                    o += n;
                }
            }
            _ => {}
        }
    }

    // ---------------- sinh ràng buộc ----------------

    pub fn generate(&mut self) {
        let body = self.body;
        for (bi, bb) in body.blocks.iter().enumerate() {
            for (si, s) in bb.statements.iter().enumerate() {
                self.cur = (
                    Some(Location {
                        block: BasicBlock(bi as u32),
                        statement: si,
                    }),
                    s.source.span,
                );
                if let StatementKind::Assign(dst, rv) = &s.kind {
                    self.assign(dst, rv);
                }
            }
            if let Some(t) = &bb.terminator {
                self.cur = (
                    Some(Location {
                        block: BasicBlock(bi as u32),
                        statement: bb.statements.len(),
                    }),
                    t.source.span,
                );
                if let TerminatorKind::Call {
                    callee, args, dest, ..
                } = &t.kind
                {
                    self.call(callee, args, dest);
                }
            }
        }
    }

    fn assign(&mut self, dst: &Place, rv: &Rvalue) {
        let (dslots, dty, _) = self.place_slots(dst);
        match rv {
            Rvalue::Use(o) => {
                let s = self.operand_slots(o);
                self.relate(&dty, &s, &dslots, false);
            }
            Rvalue::Repeat(o, _) => {
                let s = self.operand_slots(o);
                if let Ty::Array(el, _) = &dty {
                    self.relate(el, &s, &dslots, false);
                }
            }
            Rvalue::Ref(b, _, place) => {
                let region = self.body.borrows[b.index()].region.0 as usize;
                let (pslots, pty, derefs) = self.place_slots(place);
                // dst: &'b T
                if let Some(d0) = dslots.first() {
                    let d0 = d0.clone();
                    self.outl(&[region], &d0);
                }
                if let Ty::Ref(m, inner) = &dty {
                    let rest: Slots = dslots.iter().skip(1).cloned().collect();
                    self.relate(inner, &pslots, &rest, *m);
                }
                let _ = pty;
                // reborrow: từ deref trong cùng ra ngoài; dừng sau tham chiếu CHUNG đầu tiên
                for (r, shared) in derefs.iter().rev() {
                    self.outl(r, &[region]);
                    if *shared {
                        break;
                    }
                }
            }
            Rvalue::Aggregate(kind, ops) => match kind {
                AggregateKind::Tuple => {
                    let Ty::Tuple(ts) = &dty else { return };
                    let mut o = 0;
                    for (op, t) in ops.iter().zip(ts) {
                        let n = self.nslots(t);
                        let s = self.operand_slots(op);
                        if o + n <= dslots.len() {
                            self.relate(t, &s, &dslots[o..o + n], false);
                        }
                        o += n;
                    }
                }
                AggregateKind::Array(el) => {
                    for op in ops {
                        let s = self.operand_slots(op);
                        self.relate(el, &s, &dslots, false);
                    }
                }
                AggregateKind::Adt(d, ..) => {
                    let all = flat(&dslots);
                    // ADT chứa `&!`/con trỏ thô/kiểu hàm là BẤT BIẾN (ADR-023 §11): dựng nó phải đồng nhất region trong của đối số với
                    // bó region của ADT (hai chiều); chỉ hiệp biến thì `Sl { s: &!p }` cho phép thu hẹp region của `p` (âm tính giả
                    // tìm thấy khi đột biến `adt_invariant` sống sót)
                    let invariant = self.adt_invariant(*d);
                    for op in ops {
                        let s = flat(&self.operand_slots(op));
                        self.outl(&s, &all);
                        if invariant {
                            self.outl(&all, &s);
                        }
                    }
                }
            },
            // không sinh/lan truyền tham chiếu
            Rvalue::Len(_)
            | Rvalue::Cast(..)
            | Rvalue::CastFits(..)
            | Rvalue::BinaryOp(..)
            | Rvalue::CheckedBinaryOp(..)
            | Rvalue::UnaryOp(..)
            | Rvalue::Discriminant(_) => {}
        }
    }

    // ---------------- lời gọi ----------------

    fn call(&mut self, callee: &gt_mir::Callee, args: &[Operand], dest: &Place) {
        let arg_slots: Vec<Slots> = args.iter().map(|o| self.operand_slots(o)).collect();
        let (dslots, _dty, _) = self.place_slots(dest);
        let mut extra: Vec<usize> = Vec::new();
        if let gt_mir::Callee::Value(o) = callee {
            extra = flat(&self.operand_slots(o));
        }
        let mapped = match callee {
            gt_mir::Callee::Fn { node, generics } => {
                self.call_sig(*node, None, generics, &arg_slots, &dslots)
            }
            gt_mir::Callee::TraitMethod {
                trait_def,
                member,
                self_ty,
                generics,
            } => {
                let node = self
                    .db
                    .traits
                    .get(trait_def)
                    .and_then(|t| t.members.get(*member))
                    .map(|m| m.node);
                match node {
                    Some(n) => self.call_sig(
                        n,
                        Some((*trait_def, self_ty.clone())),
                        generics,
                        &arg_slots,
                        &dslots,
                    ),
                    None => false,
                }
            }
            gt_mir::Callee::Value(_) => false,
        };
        if !mapped {
            if !matches!(callee, gt_mir::Callee::Value(_)) {
                FALLBACKS.fetch_add(1, Ordering::Relaxed);
            }
            // bảo thủ: mọi region đầu vào (và của callee) outlive nhau và outlive mọi region của đích
            let mut all: Vec<usize> = arg_slots.iter().flat_map(flat_ref).collect();
            all.extend(extra);
            all.sort_unstable();
            all.dedup();
            let d = flat(&dslots);
            for a in &all {
                for b in &all {
                    self.add(*a, *b);
                }
                for x in &d {
                    self.add(*a, *x);
                }
            }
        }
    }

    /// Ánh xạ chữ ký callee (RegionSig) lên region của lời gọi. `false` nếu không khớp (dùng đường bảo thủ).
    fn call_sig(
        &mut self,
        node: gt_ast::NodeId,
        trait_ctx: Option<(gt_resolve::DefId, Ty)>,
        generics: &[Ty],
        arg_slots: &[Slots],
        dslots: &Slots,
    ) -> bool {
        let Some(sig) = self.db.sig_by_node(node).cloned() else {
            return false;
        };
        let Some(rsig) = self.t.regions.get(&node).cloned() else {
            return false;
        };
        // tham số kiểu: của impl chứa (nếu có) rồi của chính hàm
        let mut gps: Vec<GParam> = Vec::new();
        let mut self_decl: Option<Ty> = None;
        let mut trait_node = None;
        if let Some(imp) = self
            .db
            .impls
            .iter()
            .find(|i| i.members.iter().any(|m| m.node == node))
        {
            gps.extend(imp.generics.iter().cloned());
            self_decl = Some(imp.self_ty.clone());
        } else if let Some((td, _)) = &trait_ctx {
            trait_node = self.db.traits.get(td).map(|t| t.node);
            self_decl = trait_node.map(|n| {
                Ty::Param(ParamId {
                    owner: n,
                    index: SELF_INDEX,
                })
            });
        }
        gps.extend(sig.generics.iter().cloned());
        if gps.len() != generics.len() {
            return false;
        }
        let mut inst: BTreeMap<ParamId, Ty> = BTreeMap::new();
        for (g, t) in gps.iter().zip(generics) {
            inst.insert(g.id, t.clone());
        }
        if let (Some(n), Some((_, st))) = (trait_node, &trait_ctx) {
            inst.insert(
                ParamId {
                    owner: n,
                    index: SELF_INDEX,
                },
                st.clone(),
            );
        }
        let sub = |t: &Ty| t.subst(&|p| inst.get(&p).cloned());
        // kiểu khai báo của từng tham số (self trước)
        let mut declared: Vec<Ty> = Vec::new();
        if let Some(k) = sig.self_kind {
            let Some(sd) = &self_decl else { return false };
            declared.push(match k {
                gt_ast::SelfKind::Value => sd.clone(),
                gt_ast::SelfKind::Ref => Ty::Ref(false, Box::new(sd.clone())),
                gt_ast::SelfKind::RefMut => Ty::Ref(true, Box::new(sd.clone())),
            });
        }
        declared.extend(sig.params.iter().cloned());
        if declared.len() != arg_slots.len() || rsig.inputs.len() != declared.len() {
            return false;
        }
        let mut names: BTreeMap<RegionRef, usize> = BTreeMap::new();
        let mut pvars: BTreeMap<ParamId, Vec<usize>> = BTreeMap::new();
        let mut inst_params: Vec<Ty> = Vec::new();
        let mut lists: Vec<Slots> = Vec::new();
        for (d, pos) in declared.iter().zip(&rsig.inputs) {
            let it = sub(d);
            let mut iter = pos.iter();
            let mut out = Vec::new();
            if !self.walk(d, &it, &mut iter, &mut names, &mut pvars, &mut out)
                || iter.next().is_some()
            {
                return false;
            }
            inst_params.push(it);
            lists.push(out.into_iter().map(|v| vec![v]).collect());
        }
        let ret_inst = sub(&sig.ret);
        let mut iter = rsig.output.iter();
        let mut rout = Vec::new();
        if !self.walk(
            &sig.ret, &ret_inst, &mut iter, &mut names, &mut pvars, &mut rout,
        ) || iter.next().is_some()
        {
            return false;
        }
        for ((it, l), a) in inst_params.iter().zip(&lists).zip(arg_slots) {
            self.relate(it, a, l, false);
        }
        let rl: Slots = rout.into_iter().map(|v| vec![v]).collect();
        self.relate(&ret_inst, &rl, dslots, false);
        true
    }

    /// Duyệt kiểu KHAI BÁO song song với kiểu đã thế và danh sách vị trí của RegionSig; sinh biến cho từng vị trí của kiểu đã thế.
    #[allow(clippy::too_many_arguments)]
    fn walk(
        &mut self,
        decl: &Ty,
        inst: &Ty,
        pos: &mut std::slice::Iter<'_, RegionRef>,
        names: &mut BTreeMap<RegionRef, usize>,
        pvars: &mut BTreeMap<ParamId, Vec<usize>>,
        out: &mut Vec<usize>,
    ) -> bool {
        match (decl, inst) {
            (Ty::Param(p), i) => {
                let n = self.nslots(i);
                let v = pvars.entry(*p).or_default();
                while v.len() < n {
                    let f = self.n;
                    self.n += 1;
                    v.push(f);
                }
                out.extend(v.iter().take(n).copied());
                true
            }
            (Ty::Ref(_, d), Ty::Ref(_, i)) => {
                let Some(r) = pos.next() else { return false };
                let v = self.name_var(r, names);
                out.push(v);
                self.walk(d, i, pos, names, pvars, out)
            }
            (Ty::Adt(dd, da), Ty::Adt(di, ia)) if dd == di && da.len() == ia.len() => {
                let lt = self.db.adts.get(dd).map_or(0, |a| a.lt_params);
                for _ in 0..lt {
                    let Some(r) = pos.next() else { return false };
                    let v = self.name_var(r, names);
                    out.push(v);
                }
                da.iter()
                    .zip(ia)
                    .all(|(d, i)| self.walk(d, i, pos, names, pvars, out))
            }
            (Ty::Tuple(d), Ty::Tuple(i)) if d.len() == i.len() => d
                .iter()
                .zip(i)
                .all(|(d, i)| self.walk(d, i, pos, names, pvars, out)),
            (Ty::Array(d, _), Ty::Array(i, _))
            | (Ty::Slice(d), Ty::Slice(i))
            | (Ty::RawPtr(_, d), Ty::RawPtr(_, i)) => self.walk(d, i, pos, names, pvars, out),
            (Ty::Fn(..), _) | (_, Ty::Fn(..)) => true,
            (d, i) => {
                // kiểu nguyên thuỷ cùng loại: không vị trí nào
                self.nslots(i) == 0 && self.nslots(d) == 0
            }
        }
    }

    fn name_var(&mut self, r: &RegionRef, names: &mut BTreeMap<RegionRef, usize>) -> usize {
        if *r == RegionRef::Static {
            return self.static_r;
        }
        if let Some(v) = names.get(r) {
            return *v;
        }
        let v = self.n;
        self.n += 1;
        names.insert(r.clone(), v);
        v
    }
}

fn flat_ref(s: &Slots) -> Vec<usize> {
    flat(s)
}
