//! Item, kiểu, generics, hiệu ứng, hợp đồng, doc comment, attribute. Viết TRƯỚC khi có parser (TDD).
mod common;
use common::*;
use gt_ast::*;

fn items(src: &str) -> Vec<Item> {
    let out = parse_src(src);
    assert!(
        out.report.diagnostics.is_empty(),
        "{src:?}: {:?}",
        out.report.diagnostics
    );
    out.program.items
}

fn the_fn(src: &str) -> FnItem {
    match items(src).remove(0).kind {
        ItemKind::Fn(f) => f,
        other => panic!("không phải hàm: {other:?}"),
    }
}

#[test]
fn function_signature_parts() {
    let f = the_fn(
        "ƒ add<T: Area + Copy, 'a>(μ a: &'a T, b: i32) → i32 ! {io, lock(L1)} ⊳ a >= 0 ⊲ b >= 0 { a + b }",
    );
    assert_eq!(f.sig.name.name, "add");
    let g = f.sig.generics.as_ref().expect("generics");
    assert_eq!(g.params.len(), 2);
    match &g.params[0].kind {
        GenericParamKind::Type { name, bounds } => {
            assert_eq!(name.name, "T");
            assert_eq!(
                bounds.iter().map(spath).collect::<Vec<_>>(),
                ["Area", "Copy"]
            );
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(&g.params[1].kind, GenericParamKind::Lifetime(l) if l.name == "a"));
    assert_eq!(f.sig.params.len(), 2);
    match &f.sig.params[0].kind {
        ParamKind::Named { mutable, name, ty } => {
            assert!(*mutable);
            assert_eq!(name.name, "a");
            assert_eq!(sty(ty), "&'a T");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(sty(f.sig.ret.as_ref().expect("ret")), "I32");
    match &f.sig.effects.kind {
        EffectsKind::Set(e) => {
            assert_eq!(e.len(), 2);
            assert_eq!(e[0].name.name, "io");
            assert_eq!(e[1].name.name, "lock");
            assert_eq!(e[1].arg.as_ref().map(|a| a.name.as_str()), Some("L1"));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(f.sig.contracts.len(), 2);
    assert_eq!(f.sig.contracts[0].kind, ContractKind::Pre);
    assert_eq!(f.sig.contracts[1].kind, ContractKind::Post);
    assert_eq!(sx(f.body.tail.as_ref().expect("tail")), "(+ a b)");
}

#[test]
fn pure_and_empty_effect_sets_and_no_return_type() {
    let f = the_fn("ƒ f() ! ∅ { }");
    assert!(f.sig.ret.is_none());
    assert_eq!(f.sig.effects.kind, EffectsKind::Pure);
    let g = the_fn("ƒ f() → () ! {} { }");
    assert!(matches!(&g.sig.effects.kind, EffectsKind::Set(v) if v.is_empty()));
    assert_eq!(sty(g.sig.ret.as_ref().expect("ret")), "()");
    // effects là BẮT BUỘC (D10)
    assert_eq!(codes(&parse_src("ƒ f() → i32 { 1 }")), ["E0101"]);
}

#[test]
fn contract_before_body_uses_no_struct_rule() {
    // Q15a: `⊳ x { x }` — x là điều kiện, `{ x }` là thân
    let f = the_fn("ƒ f(x: bool) → bool ! ∅ ⊳ x { x }");
    assert_eq!(f.sig.contracts.len(), 1);
    assert_eq!(sx(f.body.tail.as_ref().expect("tail")), "x");
    // struct literal trần trong hợp đồng bị từ chối
    assert_eq!(
        codes(&parse_src("ƒ f() → bool ! ∅ ⊳ P { x: 1 } { 1 }"))[0],
        "E0101"
    );
}

#[test]
fn types() {
    let t = |ty: &str| -> String {
        let f = the_fn(&format!("ƒ f(p: {ty}) ! ∅ {{ }}"));
        match &f.sig.params[0].kind {
            ParamKind::Named { ty, .. } => sty(ty),
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(t("i128"), "I128");
    assert_eq!(t("u8"), "U8");
    assert_eq!(t("Option<i32>"), "Option<I32>");
    assert_eq!(t("Cap<Fs>"), "Cap<Fs>");
    assert_eq!(t("Vec<Vec<u8>>"), "Vec<Vec<U8>>"); // `>>` là hai GT
    assert_eq!(t("&i32"), "&I32");
    assert_eq!(t("&!i32"), "&!I32");
    assert_eq!(t("&'a i32"), "&'a I32");
    assert_eq!(t("&[i32]"), "&[I32]");
    assert_eq!(t("[u8; 4]"), "[U8; 4]");
    assert_eq!(t("[T]"), "[T]");
    assert_eq!(t("()"), "()");
    assert_eq!(t("(i32,)"), "(I32)");
    assert_eq!(t("(i32, bool)"), "(I32,Bool)");
    assert_eq!(t("((i32, i32), i32)"), "((I32,I32),I32)");
    assert_eq!(t("(i32)"), "I32"); // ngoặc nhóm
    assert_eq!(t("Self"), "Self");
    assert_eq!(t("!"), "!");
    assert_eq!(t("*□ i32"), "*const I32");
    assert_eq!(t("*μ i32"), "*mut I32");
    assert_eq!(t("*\\const i32"), "*const I32");
    assert_eq!(t("*\\mut i32"), "*mut I32");
    assert_eq!(t("ƒ(i32, bool) → u8 ! ∅"), "fn(I32,Bool)->U8");
    assert_eq!(t("ƒ() ! {io}"), "fn()->()");
    assert_eq!(t("a::b::C<T>"), "a::b::C<T>");
}

#[test]
fn never_type_return_and_effects_marker() {
    // `→ ! ! {div}`: kiểu never rồi hiệu ứng
    let f = the_fn("ƒ abort(msg: &str) → ! ! {div} { ⟳ { } }");
    assert_eq!(sty(f.sig.ret.as_ref().expect("ret")), "!");
    assert!(matches!(&f.sig.effects.kind, EffectsKind::Set(v) if v.len() == 1));
}

#[test]
fn struct_enum_trait_impl() {
    let its = items(
        "▢ Point { ◈ x: i32, y: i32 }\n\
         ⊔ Shape { Circle(i32), Pair(i32, i32), Empty, }\n\
         ⊏ Area<T> { ƒ area(&self) → i32 ! ∅; ƒ scale(&!self, k: i32) ! ∅ ⊳ k > 0; }\n\
         ⊐ Area for Sq { ƒ area(&self) → i32 ! ∅ { self.w } }\n\
         ⊐ Point { ƒ new() → Self ! ∅ { Self { x: 0, y: 0 } } }\n\
         ⊐<T> Wrap<T> { }",
    );
    assert_eq!(its.len(), 6);
    match &its[0].kind {
        ItemKind::Struct(s) => {
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].vis, Visibility::Public);
            assert_eq!(s.fields[1].vis, Visibility::Private);
        }
        other => panic!("{other:?}"),
    }
    match &its[1].kind {
        ItemKind::Enum(e) => {
            assert_eq!(
                e.variants
                    .iter()
                    .map(|v| v.fields.len())
                    .collect::<Vec<_>>(),
                [1, 2, 0]
            );
        }
        other => panic!("{other:?}"),
    }
    match &its[2].kind {
        ItemKind::Trait(t) => {
            assert_eq!(t.members.len(), 2);
            assert!(matches!(
                t.members[0].sig.params[0].kind,
                ParamKind::SelfParam(SelfKind::Ref)
            ));
            assert!(matches!(
                t.members[1].sig.params[0].kind,
                ParamKind::SelfParam(SelfKind::RefMut)
            ));
            assert_eq!(t.members[1].sig.contracts.len(), 1);
        }
        other => panic!("{other:?}"),
    }
    match &its[3].kind {
        ItemKind::Impl(i) => {
            assert_eq!(i.trait_path.as_ref().map(spath).as_deref(), Some("Area"));
            assert_eq!(sty(&i.self_ty), "Sq");
            assert_eq!(i.members.len(), 1);
        }
        other => panic!("{other:?}"),
    }
    match &its[4].kind {
        ItemKind::Impl(i) => assert!(i.trait_path.is_none()),
        other => panic!("{other:?}"),
    }
    match &its[5].kind {
        ItemKind::Impl(i) => assert!(i.generics.is_some()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn mod_use_type_const_static() {
    let its = items(
        "▦ m { ◈ ƒ secret() → i32 ! ∅ { 1 } }\n\
         ⇲ m::secret;\n\
         ⇲ a::b::{x, y};\n\
         ≡ Id ≔ u32;\n\
         ≡ Pair<T> ≔ (T, T);\n\
         □ LIMIT: u32 ≔ 100;\n\
         ■ GREETING: &str ≔ \"hi\";",
    );
    assert_eq!(its.len(), 7);
    match &its[0].kind {
        ItemKind::Mod(m) => {
            assert_eq!(m.items.len(), 1);
            assert_eq!(m.items[0].vis, Visibility::Public);
        }
        other => panic!("{other:?}"),
    }
    match &its[2].kind {
        ItemKind::Use(u) => {
            assert_eq!(spath(&u.path), "a::b");
            assert_eq!(
                u.names.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(),
                ["x", "y"]
            );
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(&its[3].kind, ItemKind::TypeAlias(t) if t.name.name == "Id"));
    assert!(
        matches!(&its[5].kind, ItemKind::Const(c) if c.name.name == "LIMIT" && sx(&c.value) == "100")
    );
    assert!(matches!(&its[6].kind, ItemKind::Static(s) if s.name.name == "GREETING"));
    // ASCII alias cho cùng kết quả
    let a = items(
        "\\type Id := u32; \\const L: u32 := 1; \\static S: u32 := 2; \\mod m { } \\use a::b;",
    );
    let b = items("≡ Id ≔ u32; □ L: u32 ≔ 1; ■ S: u32 ≔ 2; ▦ m { } ⇲ a::b;");
    assert_eq!(mask_spans(&format!("{a:?}")), mask_spans(&format!("{b:?}")));
}

#[test]
fn docs_and_attributes_attach_to_nodes() {
    let its = items(
        "/// Cộng hai số.\n\
         /// Dòng hai.\n\
         #allow(S101, \"lý do\")\n\
         #noreturn\n\
         ◈ ƒ f() ! ∅ { }\n\
         ▢ P {\n\
           /// hoành độ\n\
           x: i32,\n\
         }\n\
         ⊔ E {\n\
           /// biến thể\n\
           A,\n\
         }",
    );
    assert_eq!(
        its[0]
            .docs
            .iter()
            .map(|d| d.text.as_str())
            .collect::<Vec<_>>(),
        ["Cộng hai số.", "Dòng hai."]
    );
    assert_eq!(its[0].attrs.len(), 2);
    assert_eq!(its[0].attrs[0].name.name, "allow");
    assert_eq!(its[0].attrs[0].args.len(), 2);
    assert!(matches!(&its[0].attrs[0].args[0].kind, AttrArgKind::Ident(s) if s == "S101"));
    assert!(matches!(&its[0].attrs[0].args[1].kind, AttrArgKind::Str(s) if s == "lý do"));
    assert!(its[0].attrs[1].args.is_empty());
    assert_eq!(its[0].vis, Visibility::Public);
    match &its[1].kind {
        ItemKind::Struct(s) => assert_eq!(s.fields[0].docs[0].text, "hoành độ"),
        other => panic!("{other:?}"),
    }
    match &its[2].kind {
        ItemKind::Enum(e) => assert_eq!(e.variants[0].docs[0].text, "biến thể"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn self_param_only_in_trait_or_impl() {
    assert_eq!(codes(&parse_src("ƒ f(&self) ! ∅ { }"))[0], "E0101");
    assert!(codes(&parse_src("⊐ T { ƒ f(&self) ! ∅ { } }")).is_empty());
}

#[test]
fn reserved_keywords_at_item_level() {
    assert_eq!(
        codes(&parse_src("\\extern f() -> i32 ! \\nil ;"))[0],
        "E0101"
    );
    assert_eq!(
        codes(&parse_src("ƒ f<T>() → i32 ! ∅ \\where T: Copy { 1 }"))[0],
        "E0101"
    );
}

#[test]
fn node_ids_are_dense_unique_and_spans_nest() {
    let out = parse_src("ƒ add(a: i32, b: i32) → i32 ! ∅ { a + b }");
    assert!(out.report.diagnostics.is_empty());
    assert!(out.node_count > 10);
    // span của Program bao cả tệp; span của Item nằm trong Program
    assert_eq!(out.program.span.start, 0);
    assert!(out.program.items[0].span.end <= out.program.span.end);
    let ItemKind::Fn(f) = &out.program.items[0].kind else {
        panic!()
    };
    assert!(f.body.span.start > f.sig.name.span.end);
}

#[test]
fn recursion_variant_and_self_expression_in_signatures() {
    // Q15h: `↓ expr` là contract kiểu Variant, ở cả hàm lẫn chữ ký trait
    let f = the_fn("ƒ fact(n: u32) → u64 ! ∅ ⊳ n >= 0 ↓ n { n as u64 }");
    assert_eq!(
        f.sig.contracts.iter().map(|c| c.kind).collect::<Vec<_>>(),
        [ContractKind::Pre, ContractKind::Variant]
    );
    let its = items("⊏ T { ƒ f(&self, n: u32) ! ∅ ↓ n; }");
    match &its[0].kind {
        ItemKind::Trait(t) => assert_eq!(t.members[0].sig.contracts[0].kind, ContractKind::Variant),
        other => panic!("{other:?}"),
    }
    // `↓ P { … } {` bị từ chối (Q15a)
    assert_eq!(codes(&parse_src("ƒ f() ! ∅ ↓ P { x: 1 } { }"))[0], "E0101");
    // ASCII alias của ↓
    assert_eq!(
        mask_spans(&format!("{:?}", items(r"\fn f(n: u32) ! \nil \dec n { }"))),
        mask_spans(&format!("{:?}", items("ƒ f(n: u32) ! ∅ ↓ n { }")))
    );
}
