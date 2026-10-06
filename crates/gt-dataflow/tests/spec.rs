//! Test đặc tả dataflow (plan M5): kết quả CỐ ĐỊNH trên các CFG mẫu dựng tay.
mod common;
use common::*;
use gt_dataflow::*;
use gt_mir::*;
use gt_types::Ty;

fn set(b: &BitSet) -> Vec<usize> {
    b.iter().collect()
}

/// Kim cương: `_2` chỉ được gán ở một nhánh, rồi đọc ở điểm hợp.
fn diamond() -> Body {
    body(
        vec![i32t(), Ty::Bool, i32t()],
        1,
        vec![
            (
                vec![st(StatementKind::StorageLive(Local(2)))],
                switch(1, 2, 1),
            ),
            (
                vec![assign(2, Rvalue::Use(c(7)))],
                TerminatorKind::Goto(bb(3)),
            ),
            (vec![], TerminatorKind::Goto(bb(3))),
            (
                vec![
                    assign(0, use_copy(2)),
                    st(StatementKind::StorageDead(Local(2))),
                ],
                TerminatorKind::Return,
            ),
        ],
    )
}

#[test]
fn definitely_assigned_is_an_all_paths_analysis() {
    let b = diamond();
    let r = solve(&DefinitelyAssigned, &b).expect("hội tụ");
    // đầu bb3: chỉ tham số `_1` chắc chắn đã gán; `_2` chỉ trên một nhánh
    assert_eq!(set(&r.state[3].0), vec![1]);
    assert_eq!(set(&r.state[1].0), vec![1]);
    let errs = check_definitely_assigned(&b).expect_err("phải phát hiện");
    assert_eq!(errs, ["bb3[0]: đọc _2 khi chưa chắc chắn được gán"]);
}

#[test]
fn the_same_cfg_with_both_branches_assigning_is_clean() {
    let mut b = diamond();
    b.blocks[2].statements.push(assign(2, Rvalue::Use(c(8))));
    assert!(check_definitely_assigned(&b).is_ok());
    let r = solve(&DefinitelyAssigned, &b).expect("hội tụ");
    assert_eq!(set(&r.state[3].0), vec![1, 2]);
}

#[test]
fn maybe_init_and_maybe_uninit_on_the_diamond() {
    let b = diamond();
    let mp = MovePaths::build(&b);
    let init = solve(&MaybeInit { paths: &mp }, &b).expect("hội tụ");
    let uninit = solve(&MaybeUninit { paths: &mp }, &b).expect("hội tụ");
    let p2 = mp.roots[2];
    // ở đầu bb3, `_2` vừa CÓ THỂ đã khởi tạo vừa CÓ THỂ chưa
    assert!(init.state[3].contains(p2));
    assert!(uninit.state[3].contains(p2));
    // đầu bb1: chắc chắn chưa
    assert!(!init.state[1].contains(p2));
    assert!(uninit.state[1].contains(p2));
    // tham số luôn khởi tạo
    assert!(init.state[3].contains(mp.roots[1]));
    assert!(!uninit.state[3].contains(mp.roots[1]));
}

#[test]
fn moves_deinitialize_and_partial_moves_touch_only_the_subtree() {
    // _1: (P, P) không Copy (dùng tuple của tuple để có move path con); move _1.0 rồi _2 = move _1
    let pt = Ty::Tuple(vec![i32t(), i32t()]);
    let tt = Ty::Tuple(vec![pt.clone(), pt.clone()]);
    let f0 = PlaceElem::Field(0, pt.clone());
    let b = body(
        vec![Ty::unit(), tt.clone(), pt.clone(), tt],
        1,
        vec![
            (
                vec![
                    st(StatementKind::StorageLive(Local(2))),
                    st(StatementKind::Assign(
                        l(2),
                        Rvalue::Use(Operand::Move(l(1).project(f0.clone()))),
                    )),
                ],
                TerminatorKind::Goto(bb(1)),
            ),
            (
                vec![
                    st(StatementKind::StorageLive(Local(3))),
                    st(StatementKind::Assign(
                        l(3),
                        Rvalue::Use(Operand::Move(l(1))),
                    )),
                ],
                TerminatorKind::Return,
            ),
        ],
    );
    let mp = MovePaths::build(&b);
    let r = solve(&MaybeUninit { paths: &mp }, &b).expect("hội tụ");
    let p10 = mp.find(&l(1).project(f0));
    assert_ne!(p10, mp.roots[1], "phải có move path con cho _1.0");
    // đầu bb1: _1.0 chưa khởi tạo (đã move), _1 gốc vẫn khởi tạo (chỉ một phần bị move)
    assert!(r.state[1].contains(p10));
    assert!(!r.state[1].contains(mp.roots[1]));
    let r2 = solve(&MaybeInit { paths: &mp }, &b).expect("hội tụ");
    assert!(!r2.state[1].contains(p10));
}

#[test]
fn call_destination_is_initialized_only_on_the_return_edge() {
    let b = body(
        vec![i32t(), i32t()],
        0,
        vec![
            (
                vec![st(StatementKind::StorageLive(Local(1)))],
                TerminatorKind::Call {
                    callee: Callee::Value(Operand::Const(Const::Unit)),
                    args: vec![],
                    arg_spans: vec![],
                    dest: l(1),
                    target: Some(bb(1)),
                },
            ),
            (
                vec![
                    assign(0, use_copy(1)),
                    st(StatementKind::StorageDead(Local(1))),
                ],
                TerminatorKind::Return,
            ),
        ],
    );
    let r = solve(&DefinitelyAssigned, &b).expect("hội tụ");
    assert!(r.state[1].0.contains(1));
    assert!(check_definitely_assigned(&b).is_ok());
}

#[test]
fn drop_deinitializes_and_is_exempt_from_the_assignment_invariant() {
    let b = body(
        vec![Ty::unit(), i32t()],
        0,
        vec![
            (
                vec![st(StatementKind::StorageLive(Local(1)))],
                TerminatorKind::Drop {
                    place: l(1),
                    target: bb(1),
                },
            ),
            (
                vec![
                    st(StatementKind::StorageDead(Local(1))),
                    assign(0, Rvalue::Use(Operand::Const(Const::Unit))),
                ],
                TerminatorKind::Return,
            ),
        ],
    );
    // drop một chỗ chưa gán là hợp lệ (elaboration ở M10)
    assert!(check_definitely_assigned(&b).is_ok());
}

/// Vòng lặp: `_2` sống xuyên qua cạnh lùi.
fn counting_loop() -> Body {
    body(
        vec![i32t(), Ty::Bool, i32t()],
        1,
        vec![
            (
                vec![
                    st(StatementKind::StorageLive(Local(2))),
                    assign(2, Rvalue::Use(c(0))),
                ],
                TerminatorKind::Goto(bb(1)),
            ),
            (vec![], switch(1, 3, 2)),
            (
                vec![assign(
                    2,
                    Rvalue::BinaryOp(BinOp::AddWrap, Operand::Copy(l(2)), c(1)),
                )],
                TerminatorKind::Goto(bb(1)),
            ),
            (
                vec![
                    assign(0, use_copy(2)),
                    st(StatementKind::StorageDead(Local(2))),
                ],
                TerminatorKind::Return,
            ),
        ],
    )
}

#[test]
fn liveness_is_fixed_on_the_counting_loop() {
    let b = counting_loop();
    let r = solve(&Liveness, &b).expect("hội tụ");
    // trạng thái CUỐI khối (sau terminator, phân tích lùi)
    assert_eq!(set(&r.state[0]), vec![1, 2]); // đầu vòng cần _1 (điều kiện) và _2
    assert_eq!(set(&r.state[1]), vec![1, 2]);
    assert_eq!(set(&r.state[2]), vec![1, 2]);
    assert_eq!(set(&r.state[3]), Vec::<usize>::new()); // sau return
    // ngay trước câu lệnh đầu bb3 (`_0 = copy _2`) _2 còn sống; sau câu lệnh đó thì không
    let before = state_at(
        &Liveness,
        &b,
        &r,
        Location {
            block: bb(3),
            statement: 0,
        },
    );
    assert!(before.contains(0));
    let lv_entry_bb0 = {
        let mut s = r.state[0].clone();
        for stt in b.blocks[0].statements.iter().rev() {
            Liveness.statement_effect(
                &mut s,
                stt,
                Location {
                    block: bb(0),
                    statement: 0,
                },
            );
        }
        s
    };
    assert_eq!(
        set(&lv_entry_bb0),
        vec![1],
        "_2 được gán trong bb0 nên không sống ở đầu hàm"
    );
}

#[test]
fn return_uses_the_return_place() {
    let b = body(
        vec![i32t()],
        0,
        vec![(vec![assign(0, Rvalue::Use(c(1)))], TerminatorKind::Return)],
    );
    let r = solve(&Liveness, &b).expect("hội tụ");
    assert!(r.state[0].is_empty());
    assert!(check_definitely_assigned(&b).is_ok());
    let bad = body(vec![i32t()], 0, vec![(vec![], TerminatorKind::Return)]);
    assert_eq!(
        check_definitely_assigned(&bad).expect_err("phải lỗi"),
        ["bb0: đọc _0 khi chưa chắc chắn được gán"]
    );
}

#[test]
fn reverse_postorder_is_fixed_and_skips_unreachable_blocks() {
    let mut b = counting_loop();
    b.blocks.push(BlockData {
        statements: vec![],
        terminator: Some(Terminator {
            kind: TerminatorKind::Goto(bb(1)),
            source: src(),
        }),
    });
    let rpo: Vec<u32> = reverse_postorder(&b).iter().map(|x| x.0).collect();
    // DFS: 0 → 1 → 3 (xong) → 2 (xong) ⇒ hậu thứ tự [3, 2, 1, 0]; khối 4 không tới được bị bỏ
    assert_eq!(rpo, vec![0, 1, 2, 3]);
}
