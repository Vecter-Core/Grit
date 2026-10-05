//! gt-ast (M2): kiểu node, NodeId, tính độc lập bề mặt. Viết TRƯỚC khi có cài đặt (TDD).
use gt_ast::*;
use gt_span::{FileId, Span};

struct B {
    g: NodeIdGen,
    ids: Vec<NodeId>,
}

impl B {
    fn new() -> B {
        B {
            g: NodeIdGen::new(FileId::new(0)),
            ids: Vec::new(),
        }
    }
    fn id(&mut self) -> NodeId {
        let id = self.g.next_id().expect("còn NodeId");
        self.ids.push(id);
        id
    }
    fn sp() -> Span {
        Span::new(0, 0)
    }
    fn ident(&mut self, n: &str) -> Ident {
        Ident {
            id: self.id(),
            span: Self::sp(),
            name: n.to_string(),
        }
    }
    fn segment(&mut self, n: &str) -> PathSegment {
        PathSegment {
            id: self.id(),
            span: Self::sp(),
            kind: SegmentKind::Name(n.to_string()),
        }
    }
    fn path(&mut self, n: &str) -> Path {
        let seg = self.segment(n);
        Path {
            id: self.id(),
            span: Self::sp(),
            segments: vec![seg],
        }
    }
    fn ty(&mut self, kind: TypeKind) -> Type {
        Type {
            id: self.id(),
            span: Self::sp(),
            kind,
        }
    }
    fn prim(&mut self, p: PrimType) -> Type {
        self.ty(TypeKind::Prim(p))
    }
    fn expr(&mut self, kind: ExprKind) -> Expr {
        Expr {
            id: self.id(),
            span: Self::sp(),
            kind,
        }
    }
    fn lit(&mut self, kind: LitKind) -> Lit {
        Lit {
            id: self.id(),
            span: Self::sp(),
            negative: false,
            kind,
        }
    }
    fn var(&mut self, n: &str) -> Expr {
        let p = self.path(n);
        self.expr(ExprKind::Path(p))
    }
    fn pat(&mut self, kind: PatternKind) -> Pattern {
        Pattern {
            id: self.id(),
            span: Self::sp(),
            kind,
        }
    }
    fn stmt(&mut self, kind: StmtKind) -> Stmt {
        Stmt {
            id: self.id(),
            span: Self::sp(),
            kind,
        }
    }
    fn block(&mut self, stmts: Vec<Stmt>, tail: Option<Expr>) -> Block {
        Block {
            id: self.id(),
            span: Self::sp(),
            stmts,
            tail: tail.map(Box::new),
        }
    }
    fn effects_pure(&mut self) -> Effects {
        Effects {
            id: self.id(),
            span: Self::sp(),
            kind: EffectsKind::Pure,
        }
    }
    fn param(&mut self, n: &str, ty: Type) -> Param {
        let name = self.ident(n);
        Param {
            id: self.id(),
            span: Self::sp(),
            kind: ParamKind::Named {
                mutable: false,
                name,
                ty,
            },
        }
    }

    /// ƒ add(a: i32, b: i32) → i32 ! ∅ { a + b }
    fn add_fn(&mut self) -> Item {
        let name = self.ident("add");
        let t1 = self.prim(PrimType::I32);
        let p1 = self.param("a", t1);
        let t2 = self.prim(PrimType::I32);
        let p2 = self.param("b", t2);
        let ret = self.prim(PrimType::I32);
        let effects = self.effects_pure();
        let a = self.var("a");
        let b = self.var("b");
        let sum = self.expr(ExprKind::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(a),
            rhs: Box::new(b),
        });
        let body = self.block(vec![], Some(sum));
        let sig = FnSig {
            name,
            generics: None,
            params: vec![p1, p2],
            ret: Some(ret),
            effects,
            contracts: vec![],
        };
        Item {
            id: self.id(),
            span: Self::sp(),
            docs: vec![],
            attrs: vec![],
            vis: Visibility::Private,
            kind: ItemKind::Fn(FnItem { sig, body }),
        }
    }
}

fn assert_node<T: Node>(n: &T) -> (NodeId, Span) {
    (n.id(), n.span())
}

#[test]
fn node_id_gen_is_dense_sequential_and_overflow_safe() {
    let mut g = NodeIdGen::new(FileId::new(3));
    let ids: Vec<u32> = (0..5).map(|_| g.next_id().expect("id").raw()).collect();
    assert_eq!(ids, [0, 1, 2, 3, 4]);
    assert_eq!(g.count(), 5);
    assert_eq!(NodeId::new(FileId::new(0), 7).index(), 7);
    // hết u32: trả None, không panic, không tràn lặng lẽ
    let mut edge = NodeIdGen::starting_at(FileId::new(0), u32::MAX);
    assert_eq!(edge.next_id(), Some(NodeId::new(FileId::new(0), u32::MAX)));
    assert_eq!(edge.next_id(), None);
    assert_eq!(edge.next_id(), None);
    // NodeId có thứ tự (tệp trước, rồi chỉ số) và hash
    assert!(NodeId::new(FileId::new(0), 1) < NodeId::new(FileId::new(0), 2));
    assert!(NodeId::new(FileId::new(0), 99) < NodeId::new(FileId::new(1), 0));
}

/// ADR-017a: NodeId mang FileId → hai node cùng chỉ số ở hai tệp khác nhau là hai khoá khác nhau.
#[test]
fn node_ids_of_different_files_never_collide() {
    let mut g0 = NodeIdGen::new(FileId::new(0));
    let mut g1 = NodeIdGen::new(FileId::new(1));
    let (a, b) = (g0.next_id().expect("id"), g1.next_id().expect("id"));
    assert_eq!(a.raw(), b.raw(), "cùng chỉ số trong tệp");
    assert_ne!(a, b, "nhưng khác khoá toàn cục");
    assert_eq!(a.file(), FileId::new(0));
    assert_eq!(b.file(), FileId::new(1));
    let mut set = std::collections::HashSet::new();
    assert!(set.insert(a));
    assert!(set.insert(b));
}

#[test]
fn hand_built_function_has_unique_dense_ids_and_spans() {
    let mut b = B::new();
    let item = b.add_fn();
    let mut raw: Vec<u32> = b.ids.iter().map(|i| i.raw()).collect();
    let n = raw.len() as u32;
    raw.sort_unstable();
    raw.dedup();
    assert_eq!(raw.len() as u32, n, "NodeId phải duy nhất");
    assert_eq!(raw, (0..n).collect::<Vec<_>>(), "NodeId phải dày đặc từ 0");
    let (id, span) = assert_node(&item);
    assert_eq!(span, Span::new(0, 0));
    assert!(id.raw() < n);
    // sao chép và so sánh cấu trúc (nền tảng của test round-trip alias↔canonical ở parser)
    assert_eq!(item.clone(), item);
    let mut b2 = B::new();
    assert_eq!(
        b2.add_fn(),
        item,
        "dựng lại cùng cây cho cùng NodeId (tất định)"
    );
}

#[test]
fn doc_comments_and_attributes_are_node_children() {
    let mut b = B::new();
    let mut item = b.add_fn();
    let doc = DocComment {
        id: b.id(),
        span: Span::new(0, 10),
        text: "Cộng hai số.".to_string(),
    };
    let attr_name = b.ident("allow");
    let arg1 = AttrArg {
        id: b.id(),
        span: Span::new(0, 0),
        kind: AttrArgKind::Ident("S101".to_string()),
    };
    let arg2 = AttrArg {
        id: b.id(),
        span: Span::new(0, 0),
        kind: AttrArgKind::Str("lý do".to_string()),
    };
    let attr = Attribute {
        id: b.id(),
        span: Span::new(0, 0),
        name: attr_name,
        args: vec![arg1, arg2],
    };
    item.docs.push(doc.clone());
    item.attrs.push(attr);
    item.vis = Visibility::Public;
    assert_eq!(item.docs[0].text, "Cộng hai số.");
    assert_eq!(item.attrs[0].args.len(), 2);
    assert_eq!(assert_node(&doc).1, Span::new(0, 10));
    assert_ne!(
        item,
        b.add_fn(),
        "docs/attrs/vis là một phần của node, ảnh hưởng so sánh"
    );
}

/// Mọi biến thể phải dựng được và khớp mẫu vét cạn: nếu thêm/bớt biến thể mà không cập nhật, test này không biên dịch.
#[test]
fn every_grammar_construct_is_representable() {
    let mut b = B::new();

    // ----- ItemKind (10) -----
    let unit = |b: &mut B| b.block(vec![], None);
    let mk_sig = |b: &mut B, n: &str| FnSig {
        name: b.ident(n),
        generics: None,
        params: vec![],
        ret: None,
        effects: b.effects_pure(),
        contracts: vec![],
    };
    let sig = mk_sig(&mut b, "f");
    let body = unit(&mut b);
    let t_alias_ty = b.prim(PrimType::U32);
    let c_ty = b.prim(PrimType::U32);
    let c_val = b.lit(LitKind::Int("100".into()));
    let c_expr = b.expr(ExprKind::Lit(c_val));
    let s_ty = b.prim(PrimType::Str);
    let s_val = b.lit(LitKind::Str("hi".into()));
    let s_expr = b.expr(ExprKind::Lit(s_val));
    let item_kinds = vec![
        ItemKind::Fn(FnItem { sig, body }),
        ItemKind::Struct(StructItem {
            name: b.ident("P"),
            generics: None,
            fields: vec![],
        }),
        ItemKind::Enum(EnumItem {
            name: b.ident("E"),
            generics: None,
            variants: vec![],
        }),
        ItemKind::Trait(TraitItem {
            name: b.ident("T"),
            generics: None,
            members: vec![],
        }),
        ItemKind::Impl(ImplItem {
            generics: None,
            trait_path: None,
            self_ty: b.prim(PrimType::I32),
            members: vec![],
        }),
        ItemKind::Mod(ModItem {
            name: b.ident("m"),
            items: vec![],
        }),
        ItemKind::Use(UseItem {
            path: b.path("m"),
            names: vec![],
        }),
        ItemKind::TypeAlias(TypeAliasItem {
            name: b.ident("Id"),
            generics: None,
            ty: t_alias_ty,
        }),
        ItemKind::Const(ConstItem {
            name: b.ident("LIMIT"),
            ty: c_ty,
            value: c_expr,
        }),
        ItemKind::Static(StaticItem {
            name: b.ident("GREETING"),
            ty: s_ty,
            value: s_expr,
        }),
    ];
    let item_names: Vec<&str> = item_kinds
        .iter()
        .map(|k| match k {
            ItemKind::Fn(_) => "fn",
            ItemKind::Struct(_) => "struct",
            ItemKind::Enum(_) => "enum",
            ItemKind::Trait(_) => "trait",
            ItemKind::Impl(_) => "impl",
            ItemKind::Mod(_) => "mod",
            ItemKind::Use(_) => "use",
            ItemKind::TypeAlias(_) => "type",
            ItemKind::Const(_) => "const",
            ItemKind::Static(_) => "static",
        })
        .collect();
    assert_eq!(item_names.len(), 10);

    // ----- TypeKind (10) -----
    let elem = Box::new(b.prim(PrimType::U8));
    let len = b.lit(LitKind::Int("4".into()));
    let len = Box::new(b.expr(ExprKind::Lit(len)));
    let path = b.path("Option");
    let args = vec![b.prim(PrimType::I32)];
    let inner = Box::new(b.prim(PrimType::I32));
    let lt = Lifetime {
        id: b.id(),
        span: Span::new(0, 0),
        name: "a".into(),
    };
    let eff = b.effects_pure();
    let type_kinds = vec![
        TypeKind::Prim(PrimType::Bool),
        TypeKind::SelfType,
        TypeKind::Path { path, args },
        TypeKind::Ref {
            lifetime: Some(lt),
            mutable: true,
            inner: inner.clone(),
        },
        TypeKind::Array {
            elem: elem.clone(),
            len,
        },
        TypeKind::Slice(elem),
        TypeKind::Tuple(vec![]),
        TypeKind::RawPtr {
            mutable: false,
            inner,
        },
        TypeKind::Fn {
            params: vec![],
            ret: None,
            effects: eff,
        },
        TypeKind::Never,
    ];
    let type_names: Vec<&str> = type_kinds
        .iter()
        .map(|k| match k {
            TypeKind::Prim(_) => "prim",
            TypeKind::SelfType => "Self",
            TypeKind::Path { .. } => "path",
            TypeKind::Ref { .. } => "ref",
            TypeKind::Array { .. } => "array",
            TypeKind::Slice(_) => "slice",
            TypeKind::Tuple(_) => "tuple",
            TypeKind::RawPtr { .. } => "rawptr",
            TypeKind::Fn { .. } => "fn",
            TypeKind::Never => "never",
        })
        .collect();
    assert_eq!(type_names.len(), 10);

    // ----- StmtKind (6) -----
    let init = b.var("x");
    let tgt = b.var("x");
    let val = b.var("y");
    let ex = b.var("z");
    let ret = b.var("r");
    let stmt_kinds = [
        StmtKind::Let {
            mutable: true,
            name: b.ident("x"),
            ty: None,
            init,
        },
        StmtKind::Assign {
            target: tgt,
            value: val,
        },
        StmtKind::Expr {
            expr: ex,
            semi: false,
        },
        StmtKind::Return(Some(ret)),
        StmtKind::Break,
        StmtKind::Continue,
    ];
    let stmt_names: Vec<&str> = stmt_kinds
        .iter()
        .map(|k| match k {
            StmtKind::Let { .. } => "let",
            StmtKind::Assign { .. } => "assign",
            StmtKind::Expr { .. } => "expr",
            StmtKind::Return(_) => "return",
            StmtKind::Break => "break",
            StmtKind::Continue => "continue",
        })
        .collect();
    assert_eq!(stmt_names.len(), 6);

    // ----- PatternKind (8) -----
    let pl = |b: &mut B, neg: bool, v: &str| {
        let mut l = b.lit(LitKind::Int(v.into()));
        l.negative = neg;
        l
    };
    let pat_kinds = [
        PatternKind::Wildcard,
        PatternKind::Rest,
        PatternKind::Binding(b.ident("v")),
        PatternKind::Lit(pl(&mut b, true, "1")),
        PatternKind::Range {
            start: Some(pl(&mut b, false, "10")),
            end: None,
            inclusive: false,
        },
        PatternKind::Variant {
            path: b.path("Some"),
            args: vec![],
        },
        PatternKind::Struct {
            path: b.path("Point"),
            fields: vec![FieldPat {
                id: b.id(),
                span: Span::new(0, 0),
                name: b.ident("x"),
                pattern: None,
            }],
            rest: true,
        },
        PatternKind::Tuple(vec![]),
    ];
    let pat_names: Vec<&str> = pat_kinds
        .iter()
        .map(|k| match k {
            PatternKind::Wildcard => "_",
            PatternKind::Rest => "..",
            PatternKind::Binding(_) => "bind",
            PatternKind::Lit(_) => "lit",
            PatternKind::Range { .. } => "range",
            PatternKind::Variant { .. } => "variant",
            PatternKind::Struct { .. } => "struct",
            PatternKind::Tuple(_) => "tuple",
        })
        .collect();
    assert_eq!(pat_names.len(), 8);

    // ----- ExprKind (19) và LoopKind (3) -----
    let e = |b: &mut B| b.var("e");
    let bx = |b: &mut B| Box::new(b.var("e"));
    let l = b.lit(LitKind::Bool(true));
    let blk1 = b.block(vec![], None);
    let blk2 = b.block(vec![], None);
    let blk3 = b.block(vec![], None);
    let blk4 = b.block(vec![], None);
    let blk5 = b.block(vec![], None);
    let arm_pat = b.pat(PatternKind::Wildcard);
    let arm_body = e(&mut b);
    let arm = Arm {
        id: b.id(),
        span: Span::new(0, 0),
        pattern: arm_pat,
        body: arm_body,
    };
    let fi = FieldInit {
        id: b.id(),
        span: Span::new(0, 0),
        name: b.ident("x"),
        value: e(&mut b),
    };
    let expr_kinds = vec![
        ExprKind::Lit(l),
        ExprKind::Path(b.path("p")),
        ExprKind::Unary {
            op: UnaryOp::Deref,
            operand: bx(&mut b),
        },
        ExprKind::Binary {
            op: BinaryOp::Implies,
            lhs: bx(&mut b),
            rhs: bx(&mut b),
        },
        ExprKind::Cast {
            expr: bx(&mut b),
            ty: b.prim(PrimType::U64),
        },
        ExprKind::Call {
            callee: bx(&mut b),
            args: vec![],
        },
        ExprKind::Field {
            base: bx(&mut b),
            name: FieldName::Named(b.ident("x")),
        },
        ExprKind::Index {
            base: bx(&mut b),
            index: bx(&mut b),
        },
        ExprKind::Range {
            start: None,
            end: Some(bx(&mut b)),
            inclusive: true,
        },
        ExprKind::Tuple(vec![]),
        ExprKind::Array(vec![]),
        ExprKind::ArrayRepeat {
            value: bx(&mut b),
            len: bx(&mut b),
        },
        ExprKind::StructLit {
            path: b.path("Point"),
            fields: vec![fi],
        },
        ExprKind::Block(blk1),
        ExprKind::If {
            cond: bx(&mut b),
            then_block: blk2,
            else_branch: None,
        },
        ExprKind::Match {
            scrutinee: bx(&mut b),
            arms: vec![arm],
        },
        ExprKind::Loop(Loop {
            kind: LoopKind::Infinite,
            variant: None,
            invariants: vec![],
            body: blk3,
        }),
        ExprKind::Unsafe {
            proofs: vec![],
            body: blk4,
        },
        ExprKind::Quantifier {
            kind: QuantifierKind::ForAll,
            var: b.ident("i"),
            domain: bx(&mut b),
            body: bx(&mut b),
        },
    ];
    let expr_names: Vec<&str> = expr_kinds
        .iter()
        .map(|k| match k {
            ExprKind::Lit(_) => "lit",
            ExprKind::Path(_) => "path",
            ExprKind::Unary { .. } => "unary",
            ExprKind::Binary { .. } => "binary",
            ExprKind::Cast { .. } => "cast",
            ExprKind::Call { .. } => "call",
            ExprKind::Field { .. } => "field",
            ExprKind::Index { .. } => "index",
            ExprKind::Range { .. } => "range",
            ExprKind::Tuple(_) => "tuple",
            ExprKind::Array(_) => "array",
            ExprKind::ArrayRepeat { .. } => "repeat",
            ExprKind::StructLit { .. } => "structlit",
            ExprKind::Block(_) => "block",
            ExprKind::If { .. } => "if",
            ExprKind::Match { .. } => "match",
            ExprKind::Loop(_) => "loop",
            ExprKind::Unsafe { .. } => "unsafe",
            ExprKind::Quantifier { .. } => "quantifier",
        })
        .collect();
    assert_eq!(expr_names.len(), 19);
    let loop_kinds = [
        LoopKind::Infinite,
        LoopKind::While(bx(&mut b)),
        LoopKind::ForIn {
            var: b.ident("x"),
            iter: bx(&mut b),
        },
    ];
    for k in &loop_kinds {
        match k {
            LoopKind::Infinite | LoopKind::While(_) | LoopKind::ForIn { .. } => {}
        }
    }
    let _ = blk5;
    // mọi NodeId đã cấp vẫn duy nhất
    let mut raw: Vec<u32> = b.ids.iter().map(|i| i.raw()).collect();
    let n = raw.len();
    raw.sort_unstable();
    raw.dedup();
    assert_eq!(raw.len(), n);
}

#[test]
fn contracts_effects_generics_and_trait_impl_members() {
    let mut b = B::new();
    let pre = b.var("p");
    let contract = Contract {
        id: b.id(),
        span: Span::new(0, 0),
        kind: ContractKind::Pre,
        expr: pre,
    };
    let eff_a = b.ident("io");
    let eff_b = b.ident("lock");
    let eff_arg = b.ident("L1");
    let effects = Effects {
        id: b.id(),
        span: Span::new(0, 0),
        kind: EffectsKind::Set(vec![
            Effect {
                id: b.id(),
                span: Span::new(0, 0),
                name: eff_a,
                arg: None,
            },
            Effect {
                id: b.id(),
                span: Span::new(0, 0),
                name: eff_b,
                arg: Some(eff_arg),
            },
        ]),
    };
    let bound = b.path("Area");
    let gname = b.ident("T");
    let generics = Generics {
        id: b.id(),
        span: Span::new(0, 0),
        params: vec![
            GenericParam {
                id: b.id(),
                span: Span::new(0, 0),
                kind: GenericParamKind::Type {
                    name: gname,
                    bounds: vec![bound],
                },
            },
            GenericParam {
                id: b.id(),
                span: Span::new(0, 0),
                kind: GenericParamKind::Lifetime(Lifetime {
                    id: b.id(),
                    span: Span::new(0, 0),
                    name: "a".into(),
                }),
            },
        ],
    };
    let self_param = Param {
        id: b.id(),
        span: Span::new(0, 0),
        kind: ParamKind::SelfParam(SelfKind::RefMut),
    };
    let sig = FnSig {
        name: b.ident("area"),
        generics: Some(generics),
        params: vec![self_param],
        ret: None,
        effects,
        contracts: vec![contract],
    };
    let tm = TraitMember {
        id: b.id(),
        span: Span::new(0, 0),
        docs: vec![],
        sig,
    };
    assert_eq!(assert_node(&tm).0, tm.id);
    let body = b.block(vec![], None);
    let sig2 = FnSig {
        name: b.ident("f"),
        generics: None,
        params: vec![],
        ret: None,
        effects: b.effects_pure(),
        contracts: vec![],
    };
    let im = ImplMember {
        id: b.id(),
        span: Span::new(0, 0),
        docs: vec![],
        func: FnItem { sig: sig2, body },
    };
    assert_eq!(assert_node(&im).0, im.id);
    // các kiểu khác cũng là Node
    let _ = (assert_node(&b.ident("x")), assert_node(&b.path("p")));
    let st = b.stmt(StmtKind::Break);
    let _ = assert_node(&st);
    let fd = FieldDef {
        id: b.id(),
        span: Span::new(0, 0),
        docs: vec![],
        vis: Visibility::Public,
        name: b.ident("x"),
        ty: b.prim(PrimType::I32),
    };
    let _ = assert_node(&fd);
    let var = Variant {
        id: b.id(),
        span: Span::new(0, 0),
        docs: vec![],
        name: b.ident("Circle"),
        fields: vec![b.prim(PrimType::I32)],
    };
    let _ = assert_node(&var);
    let prog = Program {
        id: b.id(),
        span: Span::new(0, 0),
        items: vec![b.add_fn()],
    };
    let _ = assert_node(&prog);
}

/// AST không biết dạng bề mặt: gt-ast KHÔNG được phụ thuộc gt-lexer và không chứa khái niệm token/alias.
#[test]
fn ast_is_independent_of_surface_syntax() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("Cargo.toml");
    assert!(
        !manifest.contains("gt-lexer"),
        "gt-ast không được phụ thuộc gt-lexer"
    );
    let deps: Vec<&str> = manifest
        .lines()
        .skip_while(|l| !l.starts_with("[dependencies]"))
        .skip(1)
        .take_while(|l| !l.starts_with('['))
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert_eq!(deps.len(), 1, "chỉ được phụ thuộc gt-span: {deps:?}");
    assert!(deps[0].starts_with("gt-span"));
    // mã nguồn không nhắc token/alias
    for f in ["lib.rs", "node.rs", "item.rs", "ty.rs", "expr.rs", "pat.rs"] {
        let p = format!("{}/src/{f}", env!("CARGO_MANIFEST_DIR"));
        let Ok(src) = std::fs::read_to_string(&p) else {
            continue;
        };
        for banned in [
            "gt_lexer",
            "TokenKind",
            "KW_",
            "ascii",
            "ASCII",
            "canonical",
            "Canonical",
        ] {
            let code: String = src
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                !code.contains(banned),
                "{f} chứa {banned:?} — AST không được biết dạng bề mặt"
            );
        }
    }
}
