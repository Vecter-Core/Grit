//! THĂM DÒ codegen cho ADR-040 (Q87) TRƯỚC khi viết gt-types: dựng tay MIR có chuỗi `Deref → Downcast → Field → Ref` bằng cách VÁ MIR của chương trình
//! thật (khớp mẫu theo giá trị trên `*r` với trường Copy): biến bind `x` đổi kiểu thành `&V`/`&!V`, phép gán bind thành `Rvalue::Ref` trên đúng vị trí
//! `(*r) as V . f`, mọi chỗ dùng `x` thêm `Deref` ở đầu. Chạy thông dịch (MIR đã vá) và exe LLVM (cùng MIR đã vá) rồi so với kết quả CHƯA vá.
#![cfg(feature = "llvm")]
use gt_driver::native::{build_exe_profile, run_exe_timeout};
use gt_driver::pipeline::{Artifacts, attestation};
use gt_driver::runner::run_main;
use gt_driver::{Options, check};
use gt_mir::{
    BasicBlock, BorrowData, BorrowId, BorrowKind, Callee, Const, Local, Operand, Place, PlaceElem,
    RegionVid, Rvalue, StatementKind, TerminatorKind,
};
use gt_resolve::SourceInput;
use gt_types::{IntTy, Ty};
use std::time::Duration;

fn deref_local(p: &mut Place, l: Local) {
    if p.local == l {
        p.projection.insert(0, PlaceElem::Deref);
    }
}

fn fix_op(o: &mut Operand, l: Local) {
    match o {
        Operand::Copy(p) | Operand::Move(p) => deref_local(p, l),
        Operand::Const(_) => {}
    }
}

fn fix_rv(r: &mut Rvalue, l: Local) {
    match r {
        Rvalue::Use(o)
        | Rvalue::Repeat(o, _)
        | Rvalue::Cast(o, _)
        | Rvalue::CastFits(o, _)
        | Rvalue::UnaryOp(_, o) => fix_op(o, l),
        Rvalue::Ref(_, _, p) | Rvalue::Len(p) | Rvalue::Discriminant(p) => deref_local(p, l),
        Rvalue::BinaryOp(_, a, b) | Rvalue::CheckedBinaryOp(_, a, b) => {
            fix_op(a, l);
            fix_op(b, l);
        }
        Rvalue::Aggregate(_, v) => v.iter_mut().for_each(|o| fix_op(o, l)),
    }
}

/// Vá hàm `key`: mọi phép gán `x = Use(place)` với `x` là biến người dùng tên trong `vars` ⇒ `x = &[mut] place`.
/// `write` (chỉ khi `unique`): thêm ngay sau đó `*x = *x +wrap 100` (ghi qua tham chiếu duy nhất).
fn patch(art: &mut Artifacts, key: &str, vars: &[&str], unique: bool, write: bool) -> usize {
    let kind = if unique {
        BorrowKind::Unique
    } else {
        BorrowKind::Shared
    };
    let body = art
        .obl
        .bodies
        .iter_mut()
        .find(|b| b.key == key)
        .expect("không có thân");
    let targets: Vec<Local> = body
        .locals
        .iter()
        .enumerate()
        .filter(|(_, d)| {
            matches!(&d.kind, gt_mir::LocalKind::User { name, .. } if vars.contains(&name.as_str()))
        })
        .map(|(i, _)| Local(i as u32))
        .collect();
    let mut patched = 0;
    for &x in &targets {
        // đổi kiểu local
        let old = body.locals[x.index()].ty.clone();
        let old_regs = body.locals[x.index()].regions.clone();
        body.locals[x.index()].ty = Ty::Ref(unique, Box::new(old));
        let fresh = RegionVid(body.region_count);
        body.region_count += 1;
        let mut regs = vec![fresh];
        regs.extend(old_regs);
        body.locals[x.index()].regions = regs;
        // phép gán bind ⇒ Ref
        for bi in 0..body.blocks.len() {
            let mut si = 0;
            while si < body.blocks[bi].statements.len() {
                let st = body.blocks[bi].statements[si].clone();
                if let StatementKind::Assign(dst, Rvalue::Use(Operand::Copy(src))) = &st.kind
                    && dst.local == x
                    && dst.projection.is_empty()
                {
                    let id = BorrowId(body.borrows.len() as u32);
                    body.borrows.push(BorrowData {
                        place: src.clone(),
                        kind,
                        region: fresh,
                        source: st.source,
                    });
                    body.blocks[bi].statements[si].kind =
                        StatementKind::Assign(dst.clone(), Rvalue::Ref(id, kind, src.clone()));
                    patched += 1;
                    if write {
                        let mut star = Place::local(x);
                        star.projection.push(PlaceElem::Deref);
                        let add = gt_mir::Statement {
                            kind: StatementKind::Assign(
                                star.clone(),
                                Rvalue::BinaryOp(
                                    gt_mir::BinOp::AddWrap,
                                    Operand::Copy(star),
                                    Operand::Const(Const::Int {
                                        bits: 100,
                                        ty: IntTy::U32,
                                    }),
                                ),
                            ),
                            source: st.source,
                        };
                        body.blocks[bi].statements.insert(si + 1, add);
                        si += 1;
                    }
                } else {
                    // mọi chỗ dùng khác của x ⇒ thêm Deref (bỏ qua chính phép gán bind, StorageLive/Dead và phép ghi vừa chèn)
                    let skip = matches!(&st.kind, StatementKind::Assign(d, _) if d.local == x && d.projection == [PlaceElem::Deref] && write);
                    if !skip {
                        match &mut body.blocks[bi].statements[si].kind {
                            StatementKind::Assign(d, r) => {
                                deref_local(d, x);
                                fix_rv(r, x);
                            }
                            StatementKind::FakeRead(_, p) => deref_local(p, x),
                            _ => {}
                        }
                    }
                }
                si += 1;
            }
            if let Some(t) = &mut body.blocks[bi].terminator {
                match &mut t.kind {
                    TerminatorKind::SwitchInt { discr, .. } => fix_op(discr, x),
                    TerminatorKind::Drop { place, .. } => deref_local(place, x),
                    TerminatorKind::Call {
                        callee, args, dest, ..
                    } => {
                        if let Callee::Value(o) = callee {
                            fix_op(o, x);
                        }
                        args.iter_mut().for_each(|o| fix_op(o, x));
                        deref_local(dest, x);
                    }
                    TerminatorKind::Assert { cond, kind, .. } => {
                        fix_op(cond, x);
                        use gt_mir::AssertKind as K;
                        match kind {
                            K::Overflow(_, a, b) => {
                                fix_op(a, x);
                                fix_op(b, x);
                            }
                            K::OverflowNeg(a) | K::DivisionByZero(a) | K::RemainderByZero(a) => {
                                fix_op(a, x)
                            }
                            K::IndexOutOfBounds { len, index } => {
                                fix_op(len, x);
                                fix_op(index, x);
                            }
                            K::SliceRange { from, to, len } => {
                                fix_op(from, x);
                                fix_op(to, x);
                                fix_op(len, x);
                            }
                            K::CastLossy { value, .. } => fix_op(value, x),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let _ = BasicBlock(0);
    patched
}

fn gate(src: &str) -> (gt_driver::pipeline::CheckResult, String) {
    let inputs = [SourceInput {
        name: "main.gt".to_string(),
        text: src.to_string(),
    }];
    let res = check(&inputs, &Options::default()).expect("check");
    assert!(
        res.verdict.allows_output(),
        "{:?}\n{src}",
        res.report
            .diagnostics
            .iter()
            .map(|d| format!("{} {}", d.code.as_str(), d.message))
            .collect::<Vec<_>>()
    );
    let att = {
        let art = res.open_gate().expect("cổng mở");
        attestation(&res, &gt_driver::pipeline::mir_dump(art))
    };
    (res, att)
}

fn norm(v: &Option<String>) -> Option<String> {
    let s = v.as_deref()?;
    if s == "Unit" {
        return None;
    }
    if let Some(inner) = s.strip_prefix("Int(") {
        return inner.split(',').next().map(str::to_string);
    }
    Some(s.to_string())
}

/// trả (giá trị chưa vá, giá trị thông dịch đã vá, giá trị LLVM đã vá); số phép gán đã vá
fn probe(
    src: &str,
    key: &str,
    vars: &[&str],
    unique: bool,
    write: bool,
) -> (String, String, String, usize) {
    let (mut base, _) = gate(src);
    let base_run = run_main(base.take_gated().expect("cổng"));
    let base_v = format!("{:?}/{:?}", base_run.end, norm(&base_run.value));
    let (mut res, att) = gate(src);
    let mut art = res.take_gated().expect("cổng");
    let n = patch(&mut art, key, vars, unique, write);
    let dir = std::env::temp_dir().join(format!("gt-refprobe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo thư mục");
    static C: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let exe = dir.join(format!(
        "p{}.exe",
        C.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    build_exe_profile(&art, &exe, true, &att, gt_codegen_llvm::Opt::Dev)
        .expect("dựng exe từ MIR đã vá");
    let llvm = run_exe_timeout(&exe, Some(Duration::from_secs(20)));
    let _ = std::fs::remove_file(&exe);
    let interp = run_main(art);
    (
        base_v,
        format!("{:?}/{:?}", interp.end, norm(&interp.value)),
        format!("{:?}/{:?}", llvm.end, llvm.value),
        n,
    )
}

const ENUMS: &str = "⊔ E { A(u32), B(u32, u32), C, D(u32, u32, u32) }\n";

#[test]
fn deref_downcast_field_ref_one_and_many_binds_shared() {
    let src = format!(
        "{ENUMS}ƒ f(r: &E) → u32 ! ∅ {{ ⌥ *r {{ E::A(x) ↦ x + 1, E::B(x, y) ↦ x * 10 + y, E::C ↦ 7, E::D(x, y, z) ↦ x * 100 + y * 10 + z }} }}
ƒ main() → u32 ! ∅ {{ f(&E::A(5)) + f(&E::B(3, 4)) * 2 + f(&E::C) + f(&E::D(1, 2, 3)) }}\n"
    );
    let (b, i, l, n) = probe(&src, "main::f", &["x", "y", "z"], false, false);
    println!("shared: gốc={b} đã vá thông dịch={i} đã vá llvm={l} số bind vá={n}");
    assert_eq!(n, 1 + 2 + 3);
    assert_eq!(b, i, "MIR đã vá khác MIR gốc (thông dịch)");
    assert_eq!(i, l, "thông dịch ≠ llvm");
}

#[test]
fn deref_downcast_field_ref_unique_with_write_through() {
    // `&!E`: ghi qua tham chiếu duy nhất ⇒ giá trị gốc đổi (đọc lại sau cuộc gọi)
    let src = format!(
        "{ENUMS}ƒ g(r: &!E) → u32 ! ∅ {{ ⌥ *r {{ E::A(x) ↦ x, E::B(x, y) ↦ y, E::C ↦ 0, E::D(x, y, z) ↦ z }} }}
ƒ h(r: &E) → u32 ! ∅ {{ ⌥ *r {{ E::A(x) ↦ x, E::B(x, y) ↦ x + y, E::C ↦ 0, E::D(x, y, z) ↦ x + y + z }} }}
ƒ main() → u32 ! ∅ {{ μv ≔ E::B(3, 4); a ≔ g(&!v); b ≔ h(&v); a * 1000 + b }}\n"
    );
    let (b, i, l, n) = probe(&src, "main::g", &["y", "x", "z"], true, true);
    println!("unique+ghi: gốc={b} đã vá thông dịch={i} đã vá llvm={l} số bind vá={n}");
    assert!(n >= 4);
    // gốc: g(&!v)=4, h(&v)=7 ⇒ 4007. Đã vá có ghi: bind `y` (trường 1) +100 ⇒ g đọc 104 rồi h đọc 3+104=107 ⇒ 104107... kết quả thật in ra ở trên;
    // yêu cầu: thông dịch ≡ llvm
    assert_eq!(i, l, "thông dịch ≠ llvm");
    assert_ne!(b, i, "phép ghi qua &! phải làm giá trị gốc đổi");
}

#[test]
fn deref_downcast_field_ref_slice_field() {
    let src = "⊔ S<'a> { N(&'a [u32]), M(u32, &'a [u32]) }
ƒ f<'a>(r: &S<'a>) → u32 ! ∅ { ⌥ *r { S::N(s) ↦ s[0] + s[1], S::M(k, s) ↦ k + s[2] } }
ƒ main() → u32 ! ∅ { a ≔ [10u32, 20, 30]; f(&S::N(&a[..])) + f(&S::M(5, &a[..])) }\n";
    let (b, i, l, n) = probe(src, "main::f", &["s", "k"], false, false);
    println!("lát: gốc={b} đã vá={i} llvm={l} n={n}");
    assert_eq!(n, 3);
    assert_eq!(b, i);
    assert_eq!(i, l);
    assert!(l.contains("65"), "{l} / {b}");
}

#[test]
fn deref_field_ref_struct_and_tuple_without_downcast() {
    let src = "▢ P { x: u32, y: u32 }
ƒ f(p: &P) → u32 ! ∅ { ⌥ *p { P { x, y } ↦ x * 10 + y } }
ƒ t(p: &(u32, u32)) → u32 ! ∅ { ⌥ *p { (a, b) ↦ a * 10 + b } }
ƒ main() → u32 ! ∅ { f(&P { x: 4, y: 2 }) + t(&(1, 3)) }\n";
    let (b, i, l, _) = probe(src, "main::f", &["x", "y"], false, false);
    assert_eq!(b, i);
    assert!(l.contains("55"), "{l}");
    let (b, i, l, _) = probe(src, "main::t", &["a", "b"], false, false);
    assert_eq!(b, i);
    assert!(l.contains("55"), "{l}");
}
