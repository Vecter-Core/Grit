//! FB-13: đường dẫn tới hàm KẾT HỢP dùng làm GIÁ TRỊ (`B::get`, `Atomic::load`) từng làm `as_operand`/`as_rvalue` gọi nhau vô hạn ⇒ tràn ngăn xếp (gritc chết
//! thay vì báo lỗi hoặc hạ được). Nay phải hạ thành `Const::FnItem` (hoặc ICE E9901), KHÔNG BAO GIỜ đệ quy vô hạn.
mod common;
use gt_mir::{Const, Operand, Rvalue, StatementKind};

const SRC: &str = "▢ B { v: u32 }\n⊐ B {\n  ƒ get(&self) → u32 ! ∅ { self.v }\n}\nƒ main() → u32 ! ∅ {\n  b ≔ B { v: 3u32 };\n  f: ƒ(&B) → u32 ! ∅ ≔ B::get;\n  f(&b)\n}\n";

#[test]
fn associated_function_path_used_as_a_value_lowers_to_a_fn_item() {
    let (_t, m) = common::mir(SRC);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}",
        common::mir_codes(&m)
    );
    let body = m
        .bodies
        .iter()
        .find(|b| b.key.ends_with("main"))
        .expect("main");
    let has_fn_item = body.blocks.iter().any(|bb| {
        bb.statements.iter().any(|s| {
            matches!(
                &s.kind,
                StatementKind::Assign(_, Rvalue::Use(Operand::Const(Const::FnItem { .. })))
            )
        })
    });
    assert!(has_fn_item, "không có Const::FnItem cho `B::get`");
}
