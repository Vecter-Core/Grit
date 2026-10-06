#![forbid(unsafe_code)]
//! gt-types (M4): kiểm kiểu — ADR-018 (đơn hình hoá lazy, bảng impl, kiểu `!`), `semantics.md` §1, §22.
//!
//! Chỉ chạy khi giải tên (M3) sạch (fail-closed). Thân hàm generic kiểm đúng một lần; instance do bộ thu thập tính từ các gốc.
mod check;
mod collect;
mod exhaust;
mod infer;
mod mono;
mod traits;
mod ty;
mod validate;

pub use check::CallRec;
pub use collect::Lang;
pub use mono::{Instance, MAX_INSTANCE_DEPTH};
pub use traits::MethodRes;
pub use ty::{FloatTy, IntTy, ParamId, Ty};

use collect::{Env, ItemInfo};
use gt_ast::{ItemKind, NodeId};
use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_resolve::{
    Def, DefId, InputError, LocalId, Prelude, ResolveOutput, SourceInput, resolve_program,
};
use gt_span::Span;
use std::collections::BTreeMap;

const LIMIT: usize = 100;

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
    /// Bảng instance đã sắp xếp (lazy mono).
    pub instances: Vec<Instance>,
    /// Key of every fn with a body by NodeId (to read the instance table).
    pub fn_keys: BTreeMap<NodeId, String>,
}

impl TypesOutput {
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
        instances: Vec::new(),
        fn_keys: BTreeMap::new(),
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
            instances: Vec::new(),
            fn_keys: BTreeMap::new(),
        });
    }
    let mut names: Vec<String> = inputs.iter().map(|i| i.name.clone()).collect();
    names.sort();
    let result = gt_stack::run_with_big_stack(|| run(&resolve, names));
    match result {
        Ok((report, types, locals, methods, instances, fn_keys)) => Ok(TypesOutput {
            report,
            resolve,
            types,
            locals,
            methods,
            instances,
            fn_keys,
        }),
        Err(e) => Ok(ice(&e.to_string(), resolve)),
    }
}

type Outputs = (
    Report,
    BTreeMap<NodeId, Ty>,
    BTreeMap<LocalId, (String, String, Ty)>,
    BTreeMap<NodeId, MethodRes>,
    Vec<Instance>,
    BTreeMap<NodeId, String>,
);

fn run(resolve: &ResolveOutput, names: Vec<String>) -> Outputs {
    let mut env = Env::new(resolve, names);
    env.index();
    env.collect();
    env.validate_impls();
    let bodies = std::mem::take(&mut env.fn_items);
    for f in &bodies {
        if env.is_prelude(f.node.file()) || f.body.is_none() {
            continue;
        }
        let (impl_gens, self_ty) = match f.impl_idx {
            Some(i) => (
                env.impls[i].generics.clone(),
                Some(env.impls[i].self_ty.clone()),
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
        let ty = env.consts.get(&d).cloned().unwrap_or(Ty::Err);
        let key = env.defs()[d.index()].path.clone();
        check::Checker::check_const(&mut env, key, module, n, ty, value);
    }
    let instances = env.collect_instances(&bodies);
    let fn_keys: BTreeMap<NodeId, String> = bodies
        .iter()
        .filter(|b| b.body.is_some())
        .map(|b| (b.node, b.key.clone()))
        .collect();
    let mut bag = Diagnostics::new(LIMIT);
    for d in std::mem::take(&mut env.diags) {
        bag.push(d);
    }
    (
        bag.finish(),
        env.out_types,
        env.out_locals,
        env.out_methods,
        instances,
        fn_keys,
    )
}
