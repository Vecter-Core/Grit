//! ADR-021: đối số lifetime trong kiểu (`Holder<'a>`, `Pair<'a, T>`); lifetime đứng trước đối số kiểu. Viết TRƯỚC khi sửa parser.
mod common;
use common::*;
use gt_ast::*;

fn param_ty(src: &str) -> Type {
    let out = parse_src(src);
    assert!(
        out.report.diagnostics.is_empty(),
        "{src:?}: {:?}",
        out.report.diagnostics
    );
    match &out.program.items[0].kind {
        ItemKind::Fn(f) => match &f.sig.params[0].kind {
            ParamKind::Named { ty, .. } => ty.clone(),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

fn ty(t: &str) -> String {
    sty(&param_ty(&format!("ƒ f(x: {t}) ! ∅ {{ }}")))
}

#[test]
fn lifetime_arguments_are_parsed_before_type_arguments() {
    assert_eq!(ty("Holder<'a>"), "Holder<'a>");
    assert_eq!(ty("Pair<'a, T>"), "Pair<'a,T>");
    assert_eq!(ty("Two<'a, 'b>"), "Two<'a,'b>");
    assert_eq!(ty("Two<'a, 'b, T, U>"), "Two<'a,'b,T,U>");
    assert_eq!(ty("&'a Holder<'a>"), "&'a Holder<'a>");
    assert_eq!(ty("Option<Holder<'a>>"), "Option<Holder<'a>>");
    assert_eq!(ty("Holder<'static>"), "Holder<'static>");
}

#[test]
fn the_ast_keeps_lifetimes_and_type_arguments_apart() {
    let t = param_ty("ƒ f(x: Pair<'a, u8>) ! ∅ { }");
    match t.kind {
        TypeKind::Path {
            lifetimes, args, ..
        } => {
            assert_eq!(
                lifetimes
                    .iter()
                    .map(|l| l.name.as_str())
                    .collect::<Vec<_>>(),
                ["a"]
            );
            assert_eq!(args.len(), 1);
        }
        other => panic!("{other:?}"),
    }
    let t = param_ty("ƒ f(x: Option<u8>) ! ∅ { }");
    match t.kind {
        TypeKind::Path { lifetimes, .. } => assert!(lifetimes.is_empty()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_lifetime_after_a_type_argument_is_a_syntax_error() {
    for bad in [
        "ƒ f(x: Pair<u8, 'a>) ! ∅ { }",
        "ƒ f(x: Pair<'a, u8, 'b>) ! ∅ { }",
    ] {
        assert_eq!(codes(&parse_src(bad)), ["E0101"], "{bad}");
    }
}

#[test]
fn lifetime_arguments_work_in_every_type_position() {
    for src in [
        "▢ S<'a> { h: Holder<'a> }",
        "⊔ E<'a> { A(Holder<'a>) }",
        "ƒ f<'a>(x: Holder<'a>) → Holder<'a> ! ∅ { x }",
        "ƒ f<'a>(x: &'a i32) ! ∅ { y: Holder<'a> ≔ mk(x); }",
        "⊐ <'a> Holder<'a> { ƒ get(&self) → &'a i32 ! ∅ { self.r } }",
        "⊐ <'a> Show for Holder<'a> { }",
        "≡ H<'a> ≔ Holder<'a>;",
    ] {
        let out = parse_src(src);
        assert!(
            out.report.diagnostics.is_empty(),
            "{src}: {:?}",
            out.report.diagnostics
        );
    }
}
