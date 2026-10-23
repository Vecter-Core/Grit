//! M4.5: capability ở mức kiểu (D28, D29, E0310) và `Arena` là lang item — `docs/errors/E0310.md`, ADR-019.
mod common;
use common::*;

#[test]
fn only_main_may_call_a_root_capability_source() {
    ok("ƒ main() ! {io} { fs ≔ root_fs(); }");
    ok(
        "ƒ main() ! {io} { a ≔ root_fs(); b ≔ root_net(); c ≔ root_env(); d ≔ root_clock(); e ≔ root_alloc(); }",
    );
    err("ƒ run() ! {io} { fs ≔ root_fs(); }", "E0310", 1);
    err("ƒ mainx() ! {io} { fs ≔ root_fs(); }", "E0310", 1);
    err("ƒ Main() ! {io} { fs ≔ root_fs(); }", "E0310", 1);
}

#[test]
fn entry_is_the_top_level_main_of_a_file_not_a_nested_or_member_main() {
    err("▦ m { ◈ ƒ main() ! {io} { fs ≔ root_fs(); } }", "E0310", 1);
    err(
        "▢ A { n: i32 } ⊐ A { ƒ main(&self) ! {io} { fs ≔ root_fs(); } }",
        "E0310",
        1,
    );
    err(
        "⊏ T { ƒ main(&self) ! {io}; } ▢ A { n: i32 } ⊐ T for A { ƒ main(&self) ! {io} { fs ≔ root_fs(); } }",
        "E0310",
        1,
    );
    // một tệp khác cũng có `main` cấp cao của riêng nó (mỗi tệp là một module, ADR-017)
    let out = run(&[
        ("a.gt", "◈ ƒ main() ! {io} { fs ≔ root_fs(); }"),
        ("b.gt", "ƒ other() ! {io} { }"),
    ]);
    assert!(out.report.diagnostics.is_empty(), "{:?}", show(&out));
    let out = run(&[
        ("a.gt", "ƒ main() ! {io} { }"),
        ("b.gt", "◈ ƒ f() ! {io} { fs ≔ root_fs(); }"),
    ]);
    assert_eq!(codes(&out), ["E0310"], "{:?}", show(&out));
}

#[test]
fn a_capability_may_be_threaded_through_signatures_only() {
    // nhận qua tham số (kể cả tham chiếu, tuple, Option) là hợp lệ ở mọi hàm
    ok("ƒ f(fs: &Cap<Fs>) → usize ! {io} { fs.read(\"a\").len() }");
    ok("ƒ f(fs: Cap<Fs>) → usize ! {io} { fs.read(\"a\").len() }");
    ok("ƒ f(p: (Cap<Fs>, u8)) → u8 ! ∅ { p.1 }");
    ok("ƒ f(o: Option<Cap<Fs>>) → u8 ! ∅ { 1 }");
    ok("ƒ f(a: [Cap<Fs>; 1]) → u8 ! ∅ { 1 }");
    // chia nhỏ từ capability đang giữ: hợp lệ, kể cả khi trả về Cap
    ok("ƒ f(fs: &Cap<Fs>) → Cap<Fs> ! {io} { fs.sub(\"d\") }");
    ok("ƒ f(fs: &Cap<Fs>) → Cap<Fs> ! {io} { fs.sub(\"d\").sub(\"e\") }");
    // hàm generic chuyển tiếp: tham số T = Cap<Fs> nên không phải nguồn gốc
    ok("ƒ pass<T>(x: T) → T ! ∅ { x } ƒ f(fs: Cap<Fs>) → Cap<Fs> ! ∅ { pass(fs) }");
}

#[test]
fn a_return_type_containing_a_capability_without_capability_params_is_a_root_source() {
    for ret in [
        "Cap<Fs>",
        "(Cap<Fs>, u8)",
        "Option<Cap<Fs>>",
        "[Cap<Fs>; 1]",
        "Option<(u8, Cap<Net>)>",
    ] {
        // chỉ LỜI GỌI bị chặn (khai báo thì vô hại: không có thân nào tạo được giá trị)
        let decl = format!("ƒ mk() → {ret} ! ∅ {{ core::abort(\"x\") }}");
        ok(&format!("{decl} ƒ main() ! {{div}} {{ x ≔ mk(); }}"));
        err(
            &format!("{decl} ƒ f() ! {{div}} {{ x ≔ mk(); }}"),
            "E0310",
            1,
        );
    }
}

#[test]
fn a_generic_that_conjures_a_capability_from_nothing_is_a_root_source() {
    // `T` được suy ra là `Cap<Fs>` từ chú thích; kiểm sau khi giải kiểu nên vẫn bị chặn
    let decl = "ƒ conjure<T>() → T ! {div} { core::abort(\"x\") }";
    err(
        &format!("{decl} ƒ f() ! {{div}} {{ x: Cap<Fs> ≔ conjure(); }}"),
        "E0310",
        1,
    );
    ok(&format!(
        "{decl} ƒ main() ! {{div}} {{ x: Cap<Fs> ≔ conjure(); }}"
    ));
    // T không phải capability: bình thường
    ok(&format!("{decl} ƒ f() ! {{div}} {{ x: u8 ≔ conjure(); }}"));
}

#[test]
fn referencing_a_root_source_as_a_value_is_also_rejected() {
    err("ƒ f() ! {io} { g ≔ root_fs; }", "E0310", 1);
    ok("ƒ main() ! {io} { g ≔ root_fs; }");
}

#[test]
fn there_is_no_global_capability_const_or_static() {
    err("□ FS: Cap<Fs> ≔ root_fs();", "E0310", 1);
    err("■ FS: Cap<Fs> ≔ root_fs();", "E0310", 1);
    err("□ FS: Option<Cap<Fs>> ≔ Some(root_fs());", "E0310", 1);
}

#[test]
fn every_diagnostic_site_is_reported_not_just_the_first() {
    err(
        "ƒ f() ! {io} { a ≔ root_fs(); b ≔ root_net(); c ≔ root_env(); }",
        "E0310",
        3,
    );
}

#[test]
fn other_capability_kinds_have_their_own_operations() {
    ok("ƒ f(c: &Cap<Net>) → usize ! {io} { c.connect(\"h\").len() }");
    ok("ƒ f(c: &Cap<Env>) → usize ! {io} { c.var(\"HOME\") }");
    ok("ƒ f(c: &Cap<Clock>) → u64 ! {io} { c.now() }");
    ok("ƒ f(c: &Cap<Alloc>) → Arena ! {mem} { c.arena() }");
    // một capability không dùng được thao tác của loại khác (kiểu, không phải E0310)
    err(
        "ƒ f(c: &Cap<Net>) → usize ! {io} { c.var(\"x\") }",
        "E0309",
        1,
    );
    err("ƒ f(c: &Cap<Fs>) → u64 ! {io} { c.now() }", "E0309", 1);
}

#[test]
fn capabilities_are_affine_types_never_copy() {
    let dup = "ƒ dup<T: Copy>(x: T) → T ! ∅ { x } ";
    for t in [
        "Cap<Fs>",
        "Cap<Net>",
        "Cap<Env>",
        "Cap<Clock>",
        "Cap<Alloc>",
    ] {
        err(&format!("{dup}ƒ f(c: {t}) ! ∅ {{ dup(c); }}"), "E0309", 1);
    }
    // trong tuple/Option cũng không Copy
    err(
        &format!("{dup}ƒ f(c: (Cap<Fs>, u8)) ! ∅ {{ dup(c); }}"),
        "E0309",
        1,
    );
}

#[test]
fn arena_is_a_lang_item_that_is_not_copy_and_cannot_be_forged_by_attribute() {
    let out = ok("ƒ f() ! ∅ { }");
    assert!(out.resolve.defs.iter().any(|d| d.path.ends_with("::Arena")));
    let dup = "ƒ dup<T: Copy>(x: T) → T ! ∅ { x } ";
    err(&format!("{dup}ƒ f(a: Arena) ! ∅ {{ dup(a); }}"), "E0309", 1);
    // `&Arena` thì Copy (tham chiếu chung)
    ok(&format!("{dup}ƒ f(a: &Arena) ! ∅ {{ dup(a); }}"));
    // `#lang(arena)` ngoài prelude là E1002
    err("#lang(arena) ▢ MyArena { x: i32 }", "E1002", 1);
}

/// FB-C18 (kiểu (a), NGHIÊM TRỌNG, điều kiện cứng #12): `main` có THAM SỐ ở tệp khác được miễn E0310 như "entry" nhưng KHÔNG phải entry thật (`find_main` chỉ nhận `main` không
/// tham số) ⇒ hàm đó đúc `Cap` tuỳ ý, và hàm bất kỳ gọi nó. Entry phải không tham số.
#[test]
fn a_file_level_main_with_parameters_is_not_an_entry_fb_c18() {
    let out = run(&[
        (
            "main.gt",
            "ƒ helper() → u64 ! {io} { util::main(5u64) } ƒ main() ! {io} { }",
        ),
        (
            "util.gt",
            "◈ ƒ main(x: u64) → u64 ! {io} { fs ≔ root_fs(); x }",
        ),
    ]);
    assert_eq!(codes(&out), ["E0310"], "{:?}", show(&out));
}

/// FB-C18b: `Cap` bọc trong struct người dùng vẫn là capability: gọi hàm trả `W { c: Cap<…> }` mà không nhận Cap nào cũng là đúc từ hư không (`has_cap` phải đi vào trường).
#[test]
fn a_capability_wrapped_in_a_struct_is_still_a_capability_fb_c18() {
    err(
        "▢ W { c: Cap<Fs> } ƒ main() → W ! {io} { W { c: root_fs() } } ƒ helper() ! {io} { w ≔ main(); }",
        "E0310",
        1,
    );
    // đối chứng: nhận Cap qua tham số thì được
    ok("▢ W { c: Cap<Fs> } ƒ wrap(c: Cap<Fs>) → W ! ∅ { W { c: c } } ƒ main() ! {io} { w ≔ wrap(root_fs()); }");
}
