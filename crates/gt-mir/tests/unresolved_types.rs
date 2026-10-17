//! FAIL_OPEN_AUDIT S-D: M5 chỉ chạy khi M4 sạch ⇒ MIR không bao giờ được chứa `Ty::Err`/`Ty::Infer` (ở kiểu local, kiểu trường của phép chiếu, hằng, phép ép kiểu, đối số
//! ADT, tham số thế của lời gọi). Nếu có là M4/M5 sai ⇒ validator PHẢI từ chối (ICE E9901), không để lọt xuống codegen ("Ty::Err sau cổng").
mod common;
use common::*;
use gt_mir::validate::validate;
use gt_mir::*;
use gt_types::Ty;

const SRC: &str = "▢ B { v: u32 }\n⊐ B {\n  ƒ get(&self) → u32 ! ∅ { self.v }\n}\nƒ id(x: u32) → u32 ! ∅ { x }\nƒ main() → u64 ! ∅ {\n  b ≔ B { v: id(3u32) };\n  f: ƒ(&B) → u32 ! ∅ ≔ B::get;\n  x ≔ f(&b);\n  x as u64\n}\n";

fn body() -> (gt_types::TypesOutput, Body) {
    let (t, m) = mir(SRC);
    assert!(m.report.diagnostics.is_empty(), "{:?}", mir_codes(&m));
    let b = m.body("main::main").expect("thân").clone();
    assert!(
        validate(&b, &t.db).is_ok(),
        "đối chứng: thân nguyên vẹn hợp lệ"
    );
    (t, b)
}

fn rejects(t: &gt_types::TypesOutput, b: &Body, what: &str) {
    let errs = validate(b, &t.db).expect_err(&format!("{what}: validator chấp nhận Ty::Err/Infer"));
    assert!(
        errs.iter().any(|e| e.contains("Err/Infer")),
        "{what}: lỗi không nêu kiểu chưa giải quyết: {errs:?}"
    );
}

fn extra_local(b: &mut Body, ty: Ty) {
    let mut d = b.locals[1].clone();
    d.ty = ty;
    d.regions.clear();
    b.locals.push(d);
}

#[test]
fn err_and_infer_in_local_types_are_rejected_at_any_depth() {
    let (t, b) = body();
    for (what, ty) in [
        ("Err", Ty::Err),
        ("Infer", Ty::Infer(7)),
        ("Tuple(Infer)", Ty::Tuple(vec![Ty::Bool, Ty::Infer(1)])),
        ("Array(Err)", Ty::Array(Box::new(Ty::Err), 3)),
        ("Slice(Err)", Ty::Slice(Box::new(Ty::Err))),
        ("RawPtr(Err)", Ty::RawPtr(false, Box::new(Ty::Err))),
        (
            "Fn(Err)",
            Ty::Fn(vec![Ty::Err], Box::new(Ty::Bool), Vec::new()),
        ),
        (
            "Fn ret Infer",
            Ty::Fn(Vec::new(), Box::new(Ty::Infer(2)), Vec::new()),
        ),
    ] {
        let mut bad = b.clone();
        extra_local(&mut bad, ty);
        rejects(&t, &bad, what);
    }
}

#[test]
fn err_in_projection_cast_const_and_aggregate_types_is_rejected() {
    let (t, b) = body();
    // (0) tham số thế (generics) của một lời gọi `Callee::Fn`
    {
        let mut bad = b.clone();
        let mut done = false;
        for bb in &mut bad.blocks {
            if let Some(t) = &mut bb.terminator
                && let TerminatorKind::Call {
                    callee: Callee::Fn { generics, .. },
                    ..
                } = &mut t.kind
                && !done
            {
                generics.push(Ty::Err);
                done = true;
            }
        }
        assert!(done, "không có lời gọi Callee::Fn để sửa");
        rejects(&t, &bad, "Callee::Fn generics");
    }
    // (1) kiểu trường của một phép chiếu trên đích một phép gán
    let mut bad = b.clone();
    let mut done = false;
    for bb in &mut bad.blocks {
        for s in &mut bb.statements {
            if let StatementKind::Assign(p, _) = &mut s.kind
                && !done
            {
                p.projection.push(PlaceElem::Field(0, Ty::Err));
                done = true;
            }
        }
    }
    assert!(done);
    rejects(&t, &bad, "Field(Err)");
    // (2) đích của phép ép kiểu / (3) hằng hàm / (4) đối số ADT trong Aggregate
    for which in 2..=5 {
        let mut bad = b.clone();
        let mut done = false;
        for bb in &mut bad.blocks {
            for s in &mut bb.statements {
                if let StatementKind::Assign(_, rv) = &mut s.kind
                    && !done
                {
                    match (which, &mut *rv) {
                        (2, Rvalue::Cast(_, ty) | Rvalue::CastFits(_, ty)) => {
                            *ty = Ty::Err;
                            done = true;
                        }
                        (3, Rvalue::Use(Operand::Const(Const::FnItem { ty, .. }))) => {
                            *ty = Ty::Infer(0);
                            done = true;
                        }
                        (4, Rvalue::Aggregate(AggregateKind::Adt(_, _, args), _)) => {
                            args.push(Ty::Err);
                            done = true;
                        }
                        (5, Rvalue::Use(Operand::Const(Const::FnItem { generics, .. }))) => {
                            generics.push(Ty::Err);
                            done = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        assert!(done, "không tìm thấy chỗ để sửa (which={which})");
        rejects(&t, &bad, &format!("which={which}"));
    }
}
