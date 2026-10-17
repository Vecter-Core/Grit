#![forbid(unsafe_code)]
//! gt-types (M4): kiểm kiểu — ADR-018 (đơn hình hoá lazy, bảng impl, kiểu `!`), `semantics.md` §1, §22.
//!
//! Chỉ chạy khi giải tên (M3) sạch (fail-closed). Thân hàm generic kiểm đúng một lần; instance do bộ thu thập tính từ các gốc.
/// `true` chỉ khi bật feature đo `allow-intrinsic-values` (tắt E0313). Bản `gritc` PHẢI là `false`.
pub const ALLOW_INTRINSIC_VALUES: bool = cfg!(feature = "allow-intrinsic-values");

mod check;
mod collect;
mod db;
mod exhaust;
mod infer;
mod mono;
mod region;
mod send;
#[cfg(test)]
mod testkit;
mod traits;
mod ty;
mod validate;

pub use check::{CallRec, CallSite, CallTarget};
pub use collect::{
    AdtDef, AdtKind, FieldInfo, FnSigInfo, GParam, ImplDef, Lang, TraitDef, VariantInfo,
};
pub use db::{TypeDb, set_select_impl_fault_for_test, set_select_impl_first_for_test};
pub use mono::{Instance, MAX_INSTANCE_DEPTH, MAX_PROGRAM_INSTANCES, fnv1a64, instance_name};
pub use region::{RegionRef, RegionSig};
pub use traits::Bound;
pub use traits::MethodRes;
pub use ty::{FloatTy, IntTy, ParamId, SELF_INDEX, Ty};

use collect::{Env, ItemInfo};
use gt_ast::{ItemKind, NodeId};
use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_resolve::{
    Def, DefId, InputError, LocalId, Prelude, ResolveOutput, SourceInput, resolve_program,
};
use gt_span::Span;
use std::collections::BTreeMap;

const LIMIT: usize = 100;

/// Giá trị của literal số nguyên (văn bản nguồn, có thể có `_`, tiền tố cơ số, hậu tố kiểu): (giá trị, hậu tố). Dùng chung với MIR.
#[must_use]
pub fn int_literal_value(s: &str) -> Option<(u128, Option<String>)> {
    collect::parse_int_literal(s)
}

/// Điểm mã của literal ký tự (văn bản nguồn gồm dấu `'`, có escape).
#[must_use]
pub fn char_literal_value(s: &str) -> Option<u32> {
    check::decode_char(s)
}

#[derive(Debug)]
pub struct TypesOutput {
    /// Chẩn đoán của giải tên (nếu có lỗi thì M4 không chạy) hoặc của kiểm kiểu (D19).
    pub report: Report,
    pub resolve: ResolveOutput,
    /// Kiểu cuối cùng của mọi biểu thức/khối đã kiểm.
    pub types: BTreeMap<NodeId, Ty>,
    /// (khoá item, tên, kiểu) của mọi biến cục bộ.
    pub locals: BTreeMap<LocalId, (String, String, Ty)>,
    pub methods: BTreeMap<NodeId, MethodRes>,
    /// Chế độ khớp qua tham chiếu (ADR-040) tại pattern gốc của mỗi chuỗi tham chiếu: `true` = `&!`. MIR PHẢI suy ra cùng chế độ, lệch ⇒ ICE.
    pub pat_modes: BTreeMap<NodeId, bool>,
    /// Bảng instance đã sắp xếp (lazy mono).
    pub instances: Vec<Instance>,
    /// Key of every fn with a body by NodeId (to read the instance table).
    pub fn_keys: BTreeMap<NodeId, String>,
    /// Bảng region của từng chữ ký hàm/thành viên (M4.5; M6 dùng lại).
    pub regions: BTreeMap<NodeId, RegionSig>,
    /// Bảng chỉ-đọc (ADT, trait, chữ ký, impl, lang) cho các pass sau (ADR-020 §1).
    pub db: TypeDb,
    /// Mọi chỗ dùng hàm/phương thức (khoá: NodeId của biểu thức đường dẫn hoặc lời gọi phương thức), đã giải kiểu.
    pub sites: BTreeMap<NodeId, CallSite>,
    /// Item (NodeId) → định danh của nó.
    pub def_by_node: BTreeMap<NodeId, DefId>,
    /// Mọi thân cần hạ MIR (không gồm prelude), theo thứ tự tất định.
    pub bodies: Vec<BodyRef>,
    /// Hàm của PRELUDE có hợp đồng `⊳`/`⊲` (chỉ hợp đồng được kiểm kiểu và hạ MIR, không phải thân hàm): ADR-025 §6.
    pub prelude_contracts: Vec<BodyRef>,
    /// Tên tệp theo `FileId` (cho chẩn đoán của các pass sau).
    pub file_names: Vec<String>,
}

/// Loại thân cần hạ MIR.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyKind {
    Fn,
    Const,
    Static,
}

/// Một thân cần hạ MIR: hàm (tự do / thành viên impl) hoặc `□`/`■`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodyRef {
    pub node: NodeId,
    pub key: String,
    pub kind: BodyKind,
    /// Impl chứa (nếu là thành viên impl).
    pub impl_node: Option<NodeId>,
}

impl TypesOutput {
    /// Tên tệp chứa `node` (prelude: `<prelude>`).
    #[must_use]
    pub fn file_name(&self, node: NodeId) -> String {
        if node.file().raw() == collect::PRELUDE_FILE {
            "<prelude>".to_string()
        } else {
            self.file_names
                .get(node.file().index())
                .cloned()
                .unwrap_or_else(|| "?".to_string())
        }
    }

    #[must_use]
    pub fn defs(&self) -> &[Def] {
        &self.resolve.defs
    }

    /// Kiểu (dạng chữ) của biến `name` trong item `item_key` (đường dẫn đủ tên, vd `main::f`).
    #[must_use]
    pub fn local_type(&self, item_key: &str, name: &str) -> Option<String> {
        self.locals
            .values()
            .find(|(k, n, _)| k == item_key && n == name)
            .map(|(_, _, t)| t.render(self.defs()))
    }

    /// Bảng instance dạng chữ `khoá<kiểu, …>` (đã sắp).
    #[must_use]
    pub fn instance_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .instances
            .iter()
            .map(|i| {
                let k = self
                    .fn_keys
                    .get(&i.node)
                    .cloned()
                    .unwrap_or_else(|| "?".to_string());
                if i.args.is_empty() {
                    k
                } else {
                    let a: Vec<String> = i.args.iter().map(|t| t.render(self.defs())).collect();
                    format!("{k}<{}>", a.join(", "))
                }
            })
            .collect();
        v.sort();
        v
    }

    /// Ràng buộc trait có hiệu lực trong thân hàm `node` (của impl chứa và của chính hàm) — cho `is_copy` đa hình.
    #[must_use]
    pub fn scope_of(&self, node: NodeId) -> Vec<Bound> {
        let mut gens: Vec<&GParam> = Vec::new();
        if let Some(imp) = self
            .db
            .impls
            .iter()
            .find(|i| i.members.iter().any(|m| m.node == node))
        {
            gens.extend(imp.generics.iter());
        }
        if let Some(sig) = self.db.sig_by_node(node) {
            gens.extend(sig.generics.iter());
        }
        traits::scope_bounds(&gens)
    }

    /// Mọi biến cục bộ của một item dạng `tên: kiểu`, theo thứ tự khai báo.
    #[must_use]
    pub fn locals_of(&self, item_key: &str) -> Vec<String> {
        self.locals
            .values()
            .filter(|(k, _, _)| k == item_key)
            .map(|(_, n, t)| format!("{n}: {}", t.render(self.defs())))
            .collect()
    }
}

fn ice(msg: &str, resolve: ResolveOutput) -> TypesOutput {
    let mut bag = Diagnostics::new(LIMIT);
    if let Ok(code) = Code::parse("E9901") {
        bag.push(Diagnostic::new(
            code,
            "<types>",
            Span::new(0, 0),
            format!("lỗi nội bộ của compiler (ICE): {msg}"),
        ));
    }
    TypesOutput {
        report: bag.finish(),
        resolve,
        types: BTreeMap::new(),
        locals: BTreeMap::new(),
        methods: BTreeMap::new(),
        pat_modes: BTreeMap::new(),
        instances: Vec::new(),
        fn_keys: BTreeMap::new(),
        regions: BTreeMap::new(),
        db: TypeDb::default(),
        sites: BTreeMap::new(),
        def_by_node: BTreeMap::new(),
        bodies: Vec::new(),
        prelude_contracts: Vec::new(),
        file_names: Vec::new(),
    }
}

/// Giải tên rồi kiểm kiểu cho cả chương trình. Thứ tự `inputs` không ảnh hưởng kết quả.
pub fn check_program(inputs: &[SourceInput], prelude: &Prelude) -> Result<TypesOutput, InputError> {
    let mut resolve = resolve_program(inputs, prelude)?;
    if !resolve.report.diagnostics.is_empty() {
        let report = std::mem::replace(&mut resolve.report, Diagnostics::new(LIMIT).finish());
        return Ok(TypesOutput {
            report,
            resolve,
            types: BTreeMap::new(),
            locals: BTreeMap::new(),
            methods: BTreeMap::new(),
            pat_modes: BTreeMap::new(),
            instances: Vec::new(),
            fn_keys: BTreeMap::new(),
            regions: BTreeMap::new(),
            db: TypeDb::default(),
            sites: BTreeMap::new(),
            def_by_node: BTreeMap::new(),
            bodies: Vec::new(),
            prelude_contracts: Vec::new(),
            file_names: Vec::new(),
        });
    }
    let mut names: Vec<String> = inputs.iter().map(|i| i.name.clone()).collect();
    names.sort();
    let file_names = names.clone();
    let result = gt_stack::run_with_big_stack(|| run(&resolve, names));
    match result {
        Ok(o) => Ok(TypesOutput {
            report: o.report,
            resolve,
            types: o.types,
            locals: o.locals,
            methods: o.methods,
            pat_modes: o.pat_modes,
            instances: o.instances,
            fn_keys: o.fn_keys,
            regions: o.regions,
            db: o.db,
            sites: o.sites,
            def_by_node: o.def_by_node,
            bodies: o.bodies,
            prelude_contracts: o.prelude_contracts,
            file_names,
        }),
        Err(e) => Ok(ice(&e.to_string(), resolve)),
    }
}

struct Outputs {
    report: Report,
    types: BTreeMap<NodeId, Ty>,
    locals: BTreeMap<LocalId, (String, String, Ty)>,
    methods: BTreeMap<NodeId, MethodRes>,
    pat_modes: BTreeMap<NodeId, bool>,
    instances: Vec<Instance>,
    fn_keys: BTreeMap<NodeId, String>,
    regions: BTreeMap<NodeId, RegionSig>,
    db: TypeDb,
    sites: BTreeMap<NodeId, CallSite>,
    def_by_node: BTreeMap<NodeId, DefId>,
    bodies: Vec<BodyRef>,
    prelude_contracts: Vec<BodyRef>,
}

fn run(resolve: &ResolveOutput, names: Vec<String>) -> Outputs {
    let mut env = Env::new(resolve, names);
    env.index();
    env.collect();
    env.validate_impls();
    let bodies = std::mem::take(&mut env.fn_items);
    let mut prelude_contracts: Vec<BodyRef> = Vec::new();
    for f in &bodies {
        if env.is_prelude(f.node.file())
            && f.body.is_some()
            && f.ast_sig
                .contracts
                .iter()
                .any(|c| c.kind != gt_ast::ContractKind::Variant)
        {
            let (impl_gens, self_ty) = match f.impl_idx {
                Some(i) => (
                    env.db.impls[i].generics.clone(),
                    Some(env.db.impls[i].self_ty.clone()),
                ),
                None => (Vec::new(), None),
            };
            check::Checker::check_fn_contracts(&mut env, f, &impl_gens, self_ty);
            prelude_contracts.push(BodyRef {
                node: f.node,
                key: f.key.clone(),
                kind: BodyKind::Fn,
                impl_node: f.impl_idx.map(|i| env.db.impls[i].node),
            });
        }
        if env.is_prelude(f.node.file()) || f.body.is_none() {
            continue;
        }
        let (impl_gens, self_ty) = match f.impl_idx {
            Some(i) => (
                env.db.impls[i].generics.clone(),
                Some(env.db.impls[i].self_ty.clone()),
            ),
            None => (Vec::new(), None),
        };
        check::Checker::check_fn(&mut env, f, &impl_gens, self_ty);
    }
    // hằng và static
    let consts: Vec<(DefId, NodeId)> = env
        .items
        .iter()
        .filter(|(n, _)| !env.is_prelude(n.file()))
        .filter_map(
            |(n, info): (&NodeId, &ItemInfo<'_>)| match &info.item.kind {
                ItemKind::Const(_) | ItemKind::Static(_) => {
                    env.def_by_node.get(n).map(|d| (*d, *n))
                }
                _ => None,
            },
        )
        .collect();
    for (d, n) in consts {
        let info = &env.items[&n];
        let (module, value) = match &info.item.kind {
            ItemKind::Const(c) => (info.module, &c.value),
            ItemKind::Static(c) => (info.module, &c.value),
            _ => continue,
        };
        let ty = env.db.consts.get(&d).cloned().unwrap_or(Ty::Err);
        let key = env.defs()[d.index()].path.clone();
        check::Checker::check_const(&mut env, key, module, n, ty, value);
        // `■` static chia sẻ giữa mọi luồng: kiểu phải Sync (ADR-027 §2, D11)
        if matches!(env.items[&n].item.kind, ItemKind::Static(_))
            && !env.is_prelude(n.file())
            && let Err(why) = env.sync_like(&env.db.consts.get(&d).cloned().unwrap_or(Ty::Err), &[])
        {
            let sp = env.items[&n].item.span;
            let code = if send::is_internal(&why) {
                "E9901"
            } else {
                "E0501"
            };
            env.err(
                code,
                n,
                sp,
                format!("`■` static phải có kiểu `Sync` (chia sẻ giữa các luồng): {why}"),
            );
        }
    }
    let instances = env.collect_instances(&bodies);
    let fn_keys: BTreeMap<NodeId, String> = bodies
        .iter()
        .filter(|b| b.body.is_some())
        .map(|b| (b.node, b.key.clone()))
        .collect();
    // thân cần hạ MIR: hàm có thân (không prelude) theo NodeId, rồi `□`/`■`
    let mut out_bodies: Vec<BodyRef> = bodies
        .iter()
        .filter(|b| b.body.is_some() && !env.is_prelude(b.node.file()))
        .map(|b| BodyRef {
            node: b.node,
            key: b.key.clone(),
            kind: BodyKind::Fn,
            impl_node: b.impl_idx.map(|i| env.db.impls[i].node),
        })
        .collect();
    for (n, info) in &env.items {
        if env.is_prelude(n.file()) {
            continue;
        }
        let kind = match &info.item.kind {
            ItemKind::Const(_) => BodyKind::Const,
            ItemKind::Static(_) => BodyKind::Static,
            _ => continue,
        };
        if let Some(d) = env.def_by_node.get(n) {
            out_bodies.push(BodyRef {
                node: *n,
                key: env.defs()[d.index()].path.clone(),
                kind,
                impl_node: None,
            });
        }
    }
    out_bodies.sort_by_key(|b| b.node);
    let mut bag = Diagnostics::new(LIMIT);
    for d in std::mem::take(&mut env.diags) {
        bag.push(d);
    }
    Outputs {
        report: bag.finish(),
        types: env.out_types,
        locals: env.out_locals,
        methods: env.out_methods,
        pat_modes: env.out_pat_modes,
        instances,
        fn_keys,
        regions: env.region_sigs,
        db: env.db,
        sites: env.out_sites,
        def_by_node: env.def_by_node,
        bodies: out_bodies,
        prelude_contracts,
    }
}
