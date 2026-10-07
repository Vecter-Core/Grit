//! Validator phải TỪ CHỐI MIR hỏng (đặc biệt: thiếu/thừa `StorageDead` trên một đường thoát), và kiểm đầy đủ E0307 /
//! `→ !` trên CFG dựng tay (trường hợp nguồn không tạo ra được vì M4 đã chặn trước).
mod common;
use common::*;
use gt_mir::validate::validate;
use gt_mir::*;
use gt_types::Ty;

fn lowered(src: &str, key: &str) -> (gt_types::TypesOutput, Body) {
    let (t, m) = mir(src);
    assert!(m.report.diagnostics.is_empty(), "{:?}", mir_codes(&m));
    let b = m.body(key).expect("thân").clone();
    (t, b)
}

const NESTED: &str = "ƒ f(n: u32) → i32 ! {mem} { a ≔ mk_buf(); ⟳ i in 0..n { b ≔ mk_buf(); ⎇ i == 2 { ⊠; } ⎇ i == 3 { ⏎ 7; } } 0 }";

#[test]
fn lowered_mir_is_valid() {
    let (t, b) = lowered(NESTED, "main::f");
    assert!(validate(&b, &t.db).is_ok());
}

#[test]
fn removing_any_single_storage_dead_on_a_reachable_path_is_detected() {
    let (t, b) = lowered(NESTED, "main::f");
    let mut tried = 0;
    for (bi, bb) in b.blocks.iter().enumerate() {
        for (si, s) in bb.statements.iter().enumerate() {
            if matches!(s.kind, StatementKind::StorageDead(_)) {
                tried += 1;
                let mut bad = b.clone();
                bad.blocks[bi].statements[si].kind = StatementKind::Nop;
                assert!(
                    validate(&bad, &t.db).is_err(),
                    "bỏ StorageDead ở bb{bi}[{si}] mà validator không thấy"
                );
            }
        }
    }
    assert!(tried >= 10, "chỉ {tried} StorageDead");
}

#[test]
fn duplicating_a_storage_dead_or_using_a_dead_local_is_detected() {
    let (t, b) = lowered(NESTED, "main::f");
    let (bi, si, l) = b
        .blocks
        .iter()
        .enumerate()
        .find_map(|(bi, bb)| {
            bb.statements
                .iter()
                .enumerate()
                .find_map(|(si, s)| match s.kind {
                    StatementKind::StorageDead(l) => Some((bi, si, l)),
                    _ => None,
                })
        })
        .expect("có StorageDead");
    let mut dup = b.clone();
    let s = dup.blocks[bi].statements[si].clone();
    dup.blocks[bi].statements.insert(si, s.clone());
    assert!(validate(&dup, &t.db).is_err());
    // dùng sau khi chết
    let mut use_after = b.clone();
    use_after.blocks[bi].statements.insert(
        si + 1,
        Statement {
            kind: StatementKind::FakeRead(FakeReadCause::LetInit, Place::local(l)),
            source: s.source,
        },
    );
    assert!(validate(&use_after, &t.db).is_err());
}

#[test]
fn structural_errors_are_detected() {
    let (t, b) = lowered("ƒ f(x: &i32) → i32 ! ∅ { y ≔ &x; **y }", "main::f");
    assert!(validate(&b, &t.db).is_ok());
    // khoản mượn trùng id
    let mut bad = b.clone();
    bad.borrows.push(bad.borrows[0].clone());
    assert!(validate(&bad, &t.db).is_err());
    // số biến region sai
    let mut bad = b.clone();
    bad.locals[1].regions.clear();
    assert!(validate(&bad, &t.db).is_err());
    // kiểu phép gán sai
    let mut bad = b.clone();
    bad.locals[0].ty = Ty::Bool;
    assert!(validate(&bad, &t.db).is_err());
    // nhảy tới khối không tồn tại
    let mut bad = b.clone();
    bad.blocks[0].terminator = Some(Terminator {
        kind: TerminatorKind::Goto(BasicBlock(99)),
        source: bad.blocks[0].terminator.as_ref().expect("t").source,
    });
    assert!(validate(&bad, &t.db).is_err());
}

#[test]
fn never_function_with_a_reachable_return_is_e0307() {
    // nguồn hợp lệ: `→ !` với vòng vô hạn — không có Return tới được
    let (t, b) = lowered("ƒ f() → ! ! {div} { ⟳ { } }", "main::f");
    assert!(completeness(&t, &b).is_empty());
    // dựng tay: đổi khối cuối thành `return` tới được
    let mut bad = b.clone();
    let last = bad.blocks.len() - 1;
    let src = bad.blocks[last].terminator.as_ref().expect("t").source;
    bad.blocks[0].terminator = Some(Terminator {
        kind: TerminatorKind::Return,
        source: src,
    });
    let d = completeness(&t, &bad);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code.as_str(), "E0307");
    assert!(d[0].message.contains("→ !"));
}

#[test]
fn fall_off_is_reported_only_when_reachable() {
    let (t, m) = mir("ƒ f(c: bool) → i32 ! ∅ { ⎇ c { ⏎ 1; } ⏎ 2; }");
    assert!(m.report.diagnostics.is_empty());
    assert!(
        m.body("main::f").expect("f").fall_off.is_none(),
        "lối rơi không tới được phải bị bỏ"
    );
    let (_, m) = (
        t,
        gt_mir::lower_program(&run(&[(
            "main.gt",
            "ƒ f(c: bool) → i32 ! ∅ { ⎇ c { ⏎ 1; } }",
        )])),
    );
    let codes: Vec<&str> = m
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .collect();
    assert_eq!(codes, ["E0307"]);
}

#[test]
fn literal_true_and_false_conditions_are_folded_but_nothing_else() {
    // `⟳ true` không có cạnh ra; `⎇ false` không có nhánh then
    let (_, m) = mir("ƒ f() → i32 ! {div} { ⟳ true { } x ≔ 1; }");
    assert!(m.report.diagnostics.is_empty(), "{:?}", mir_codes(&m));
    let (_, m) = mir("ƒ f() → i32 ! ∅ { ⎇ false { ⏎ 1; } ⏎ 2; }");
    assert!(m.report.diagnostics.is_empty());
    // điều kiện không phải literal (dù luôn đúng) KHÔNG được gấp: vẫn có lối rơi → E0307
    let t = run(&[("main.gt", "ƒ f(x: i32) → i32 ! ∅ { ⎇ x == x { ⏎ 1; } }")]);
    let m = gt_mir::lower_program(&t);
    let codes: Vec<&str> = m
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .collect();
    assert_eq!(codes, ["E0307"]);
}

#[test]
fn quantifier_outside_a_contract_is_a_type_error_not_an_ice() {
    // lượng từ ngoài hợp đồng là lỗi của chương trình người dùng (E0316, M4); ICE chỉ còn là chốt phòng thủ không với tới được
    let t = run(&[("main.gt", "ƒ f(n: u32) → bool ! ∅ { ∀ i ∈ (0..n) : i < n }")]);
    let codes: Vec<&str> = t
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .collect();
    assert_eq!(codes, ["E0316"], "{:?}", show(&t));
    // MIR không chạy trên chương trình sai kiểu (fail-closed): không có thân, không có ICE
    let m = gt_mir::lower_program(&t);
    assert!(m.bodies.is_empty() && m.report.diagnostics.is_empty());
}

#[test]
fn implicit_reborrow_replaces_move_of_a_unique_reference_place_at_coercion_points() {
    let src = "▢ P { n: i32 } ƒ bump(p: &!P) ! ∅ { } ƒ f(p: &!P, c: bool) → &!P ! ∅ { bump(p); q: &!P ≔ p; bump(q); ⎇ c { ⏎ p; } p }";
    let (t, b) = lowered(src, "main::f");
    assert!(validate(&b, &t.db).is_ok());
    let unique_reborrows = b
        .borrows
        .iter()
        .filter(|x| {
            x.kind == BorrowKind::Unique
                && x.place.projection == [PlaceElem::Deref]
                && x.place.local == Local(1)
        })
        .count();
    // bump(p), `q ≔ p`, ⏎ p: ba khoản reborrow trên `*p`; `bump(q)` reborrow trên `*q` (q là một CHỖ `&!P`); tail `p` là move
    assert_eq!(unique_reborrows, 3, "{:?}", b.borrows);
    // không có `move _1` nào ngoài tail
    let moves_of_p = b
        .blocks
        .iter()
        .flat_map(|x| &x.statements)
        .filter(|s| matches!(&s.kind, StatementKind::Assign(_, Rvalue::Use(Operand::Move(p))) if *p == Place::local(Local(1))))
        .count();
    assert_eq!(moves_of_p, 1);
    // `&P` (Copy) và giá trị không phải chỗ: không reborrow
    let (_, b2) = lowered(
        "▢ P { n: i32 } ƒ g(p: &P) → i32 ! ∅ { a ≔ p; b ≔ p; a.n +% b.n }",
        "main::g",
    );
    assert!(b2.borrows.iter().all(|x| x.kind == BorrowKind::Shared));
}
