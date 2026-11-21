//! GÓI 14 B2 (NEXT_PRODUCT_BATCH #11/#4) — các NHÁNH PHÒNG THỦ của phần dựng cạnh/kiểm vòng lặp chỉ tới được khi MIR vi phạm một bất biến mà giai đoạn trước bảo đảm.
//! Trước GÓI 14 chúng lặng lẽ "bảo thủ" hoặc bỏ qua (phép đo GÓI 12/13: 0 lần trên 996 chương trình ⇒ đột biến theo hướng chấp nhận-thêm sống sót). Nay mỗi vi phạm
//! là ICE E9901 tường minh. Ở đây ta DỰNG TAY MIR hỏng (sửa MIR thật của một chương trình đúng) và khẳng định E9901; đối chứng: MIR nguyên vẹn KHÔNG có E9901.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_ast::NodeId;
use gt_mir::{
    BinOp, Callee, Const, Local, MirOutput, Operand, Place, Rvalue, Statement, StatementKind,
    TerminatorKind,
};
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::check_termination;
use gt_types::TypesOutput;

const PROG: &str = "ƒ g(x: i32) → i32 ! ∅ { x + 1i32 }\nƒ main() → i32 ! ∅ { g(2i32) }\n";

fn codes(t: &TypesOutput, m: &MirOutput) -> (Vec<String>, String) {
    let info = build_prog_info(t, m);
    let o = check_termination(t, m, &info, &mut ZoneProver::new(24));
    let mut v: Vec<String> = o
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    v.sort();
    let msgs = o
        .report
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect::<Vec<_>>()
        .join(" | ");
    (v, msgs)
}

fn setup() -> (TypesOutput, MirOutput) {
    let (t, m) = common::mir(PROG);
    assert!(m.report.diagnostics.is_empty());
    (t, m)
}

#[test]
fn intact_mir_has_no_ice() {
    let (t, m) = setup();
    let (c, msgs) = codes(&t, &m);
    assert!(c.is_empty(), "{c:?} {msgs}");
}

#[test]
fn function_as_arithmetic_operand_is_an_explicit_ice() {
    let (t, mut m) = setup();
    let fnode = m
        .bodies
        .iter()
        .find(|b| b.key.ends_with("::g"))
        .expect("g")
        .node;
    let b = m
        .bodies
        .iter_mut()
        .find(|b| b.key.ends_with("::main"))
        .expect("main");
    let ty = b.locals[0].ty.clone();
    let source = b.blocks[0].terminator.as_ref().expect("terminator").source;
    b.blocks[0].statements.push(Statement {
        kind: StatementKind::Assign(
            Place::local(Local(0)),
            Rvalue::BinaryOp(
                BinOp::Add,
                Operand::Const(Const::FnItem {
                    node: fnode,
                    generics: vec![],
                    ty,
                }),
                Operand::Const(Const::Unit),
            ),
        ),
        source,
    });
    let (c, msgs) = codes(&t, &m);
    assert!(c.contains(&"E9901".to_string()), "{c:?} {msgs}");
    assert!(msgs.contains("toán hạng của phép tính là hàm"), "{msgs}");
}

#[test]
fn call_argument_pointing_past_the_locals_is_an_explicit_ice() {
    let (t, mut m) = setup();
    let b = m
        .bodies
        .iter_mut()
        .find(|b| b.key.ends_with("::main"))
        .expect("main");
    let mut hit = false;
    for bb in &mut b.blocks {
        if let Some(term) = bb.terminator.as_mut()
            && let TerminatorKind::Call { args, .. } = &mut term.kind
            && let Some(a) = args.first_mut()
        {
            *a = Operand::Copy(Place::local(Local(9999)));
            hit = true;
        }
    }
    assert!(hit, "không có lời gọi có đối số");
    let (c, msgs) = codes(&t, &m);
    assert!(c.contains(&"E9901".to_string()), "{c:?} {msgs}");
    assert!(msgs.contains("ngoài"), "{msgs}");
}

#[test]
fn call_to_a_node_with_neither_body_nor_signature_is_an_explicit_ice() {
    let (t, mut m) = setup();
    let bogus = {
        let any = m.bodies[0].node;
        NodeId::new(any.file(), 4_000_000)
    };
    let b = m
        .bodies
        .iter_mut()
        .find(|b| b.key.ends_with("::main"))
        .expect("main");
    let mut hit = false;
    for bb in &mut b.blocks {
        if let Some(term) = bb.terminator.as_mut()
            && let TerminatorKind::Call {
                callee: Callee::Fn { node, .. },
                ..
            } = &mut term.kind
        {
            *node = bogus;
            hit = true;
        }
    }
    assert!(hit, "không có Callee::Fn");
    let (c, msgs) = codes(&t, &m);
    assert!(c.contains(&"E9901".to_string()), "{c:?} {msgs}");
    assert!(msgs.contains("không có thân lẫn chữ ký"), "{msgs}");
}

#[test]
fn block_without_terminator_is_an_explicit_ice() {
    let (t, mut m) = setup();
    let b = m
        .bodies
        .iter_mut()
        .find(|b| b.key.ends_with("::g"))
        .expect("g");
    let last = b.blocks.len() - 1;
    b.blocks[last].terminator = None;
    let (c, msgs) = codes(&t, &m);
    assert!(c.contains(&"E9901".to_string()), "{c:?} {msgs}");
    assert!(msgs.contains("không có terminator"), "{msgs}");
}
