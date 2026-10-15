//! Dùng chung cho `ref_pattern_probe` / `ref_pattern_real`: vá MIR, cổng, chạy thông dịch + LLVM.
#![allow(dead_code)]
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
pub fn patch(art: &mut Artifacts, key: &str, vars: &[&str], unique: bool, write: bool) -> usize {
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

pub fn gate(src: &str) -> (gt_driver::pipeline::CheckResult, String) {
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

pub fn norm(v: &Option<String>) -> Option<String> {
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
pub fn probe(
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

/// chạy chương trình đã qua cổng ở thông dịch và exe LLVM (dev), trả (thông dịch, llvm) dạng `Kết thúc/Some("giá trị")`
pub fn run_both(src: &str) -> (String, String) {
    let (mut res, att) = gate(src);
    let art = res.take_gated().expect("cổng");
    let dir = std::env::temp_dir().join(format!("gt-refreal-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo thư mục");
    static C: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let exe = dir.join(format!(
        "p{}.exe",
        C.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    build_exe_profile(&art, &exe, true, &att, gt_codegen_llvm::Opt::Dev).expect("dựng exe");
    let llvm = run_exe_timeout(&exe, Some(Duration::from_secs(20)));
    let _ = std::fs::remove_file(&exe);
    let interp = run_main(art);
    (
        format!("{:?}/{:?}", interp.end, norm(&interp.value)),
        format!("{:?}/{:?}", llvm.end, llvm.value),
    )
}

/// Chữ ký khoản vay của một thân (đối chiếu ĐỘC LẬP, ADR-040 bước 3): (nguồn chuẩn hoá, loại, điểm sinh, điểm chết = chỗ dùng CUỐI của biến giữ khoản vay,
/// số chỗ dùng). Không so từng lệnh.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BorrowSig {
    pub source: String,
    pub unique: bool,
    pub birth: (usize, usize),
    pub death: (usize, usize),
    pub uses: usize,
}

fn local_name(body: &gt_mir::Body, l: Local) -> String {
    match &body.locals[l.index()].kind {
        gt_mir::LocalKind::Return => "ret".to_string(),
        gt_mir::LocalKind::Arg(n) => n.clone().unwrap_or_else(|| format!("arg{}", l.0)),
        gt_mir::LocalKind::User { name, .. } => name.clone(),
        gt_mir::LocalKind::Temp => "tmp".to_string(),
    }
}

fn place_str(body: &gt_mir::Body, p: &Place) -> String {
    let mut s = local_name(body, p.local);
    for e in &p.projection {
        match e {
            PlaceElem::Deref => s.push_str(".*"),
            PlaceElem::Field(i, _) => s.push_str(&format!(".{i}")),
            PlaceElem::Downcast(i) => s.push_str(&format!(".v{i}")),
            PlaceElem::Index(_) => s.push_str(".[i]"),
            PlaceElem::Subslice { .. } => s.push_str(".[..]"),
        }
    }
    s
}

fn mentions(text: &str, l: Local) -> bool {
    let pat = format!("Local({})", l.0);
    let mut from = 0;
    while let Some(i) = text[from..].find(&pat) {
        let end = from + i + pat.len();
        if !text[end..].starts_with(|c: char| c.is_ascii_digit()) {
            return true;
        }
        from = end;
    }
    false
}

pub fn borrow_sigs(body: &gt_mir::Body) -> Vec<BorrowSig> {
    let mut out = Vec::new();
    for (bi, b) in body.blocks.iter().enumerate() {
        for (si, st) in b.statements.iter().enumerate() {
            if let StatementKind::Assign(dst, Rvalue::Ref(id, kind, src)) = &st.kind {
                let d = dst.local;
                let mut last = (bi, si);
                let mut uses = 0;
                for (bj, b2) in body.blocks.iter().enumerate() {
                    for (sj, s2) in b2.statements.iter().enumerate() {
                        if (bj, sj) == (bi, si) {
                            continue;
                        }
                        if matches!(
                            s2.kind,
                            StatementKind::StorageLive(_) | StatementKind::StorageDead(_)
                        ) {
                            continue;
                        }
                        if mentions(&format!("{:?}", s2.kind), d) {
                            uses += 1;
                            last = last.max((bj, sj));
                        }
                    }
                    if let Some(t) = &b2.terminator
                        && mentions(&format!("{:?}", t.kind), d)
                        && !matches!(t.kind, TerminatorKind::Drop { .. })
                    {
                        uses += 1;
                        last = last.max((bj, b2.statements.len()));
                    }
                }
                let _ = id;
                out.push(BorrowSig {
                    source: place_str(body, src),
                    unique: *kind == BorrowKind::Unique,
                    birth: (bi, si),
                    death: last,
                    uses,
                });
            }
        }
    }
    out
}

pub fn sigs_of(src: &str, key: &str) -> Vec<BorrowSig> {
    let (mut res, _) = gate(src);
    let art = res.take_gated().expect("cổng");
    let b = art.obl.bodies.iter().find(|b| b.key == key).expect("thân");
    borrow_sigs(b)
}

/// như `sigs_of` nhưng trên MIR đã vá tay từ chương trình theo-giá-trị (bộ chuẩn của thăm dò)
pub fn sigs_of_patched(
    src: &str,
    key: &str,
    vars: &[&str],
    unique: bool,
    write: bool,
) -> Vec<BorrowSig> {
    let (mut res, _) = gate(src);
    let mut art = res.take_gated().expect("cổng");
    patch(&mut art, key, vars, unique, write);
    let b = art.obl.bodies.iter().find(|b| b.key == key).expect("thân");
    borrow_sigs(b)
}
