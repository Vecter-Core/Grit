//! ADR-P2-02 C3: đường hạ MIR→GLIR (gt-glirgen) trên MỌI mẫu `tests/pass`: mọi hàm hạ được phải qua VERIFIER GLIR (0 lỗi) và in-đọc-in ổn định;
//! hàm chưa hạ được được ĐẾM theo lý do (in bằng `--nocapture`). Độ phủ chỉ được tăng (ngưỡng dưới ghi ở cuối, nâng dần).
#![cfg(feature = "llvm")]
use gt_driver::{Options, check};
use gt_resolve::SourceInput;
use std::collections::BTreeMap;

#[test]
fn every_lowered_body_passes_the_glir_verifier_and_coverage_is_measured() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/pass");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .expect("dir")
        .map(|e| e.expect("e").path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .collect();
    files.sort();
    // HAI số (ADR-P2-02, thân generic theo instance): theo THÂN (hạ được ⇔ ≥ 1 instance hạ được) và theo INSTANCE
    let (mut lowered, mut total) = (0usize, 0usize);
    let (mut inst_ok, mut inst_total) = (0usize, 0usize);
    let mut why: BTreeMap<String, usize> = BTreeMap::new();
    let mut why_body: BTreeMap<String, usize> = BTreeMap::new();
    // khảo sát: lời gọi (trực tiếp) tới callee KHÔNG có thân MIR (intrinsic/runtime), theo khoá: (số lời gọi, số thân chứa)
    let mut nobody: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for p in files {
        let src = std::fs::read_to_string(&p).expect("đọc");
        let Ok(res) = check(
            &[SourceInput {
                name: "main.gt".to_string(),
                text: src,
            }],
            &Options::default(),
        ) else {
            continue;
        };
        let Some(art) = res.open_gate() else { continue };
        let mir = glir_input_mir(art);
        let fns: Vec<&gt_mir::Body> = mir
            .bodies
            .iter()
            .filter(|b| b.kind == gt_mir::BodyKind::Fn)
            .collect();
        total += fns.len();
        let has_body: std::collections::BTreeSet<_> = mir.bodies.iter().map(|b| b.node).collect();
        for b in &fns {
            let mut keys: Vec<String> = Vec::new();
            for bd in &b.blocks {
                if let Some(gt_mir::Terminator {
                    kind:
                        gt_mir::TerminatorKind::Call {
                            callee: gt_mir::Callee::Fn { node, .. },
                            ..
                        },
                    ..
                }) = &bd.terminator
                    && !has_body.contains(node)
                {
                    keys.push(mir.keys.get(node).cloned().unwrap_or_else(|| "?".into()));
                }
            }
            let distinct: std::collections::BTreeSet<_> = keys.iter().cloned().collect();
            for k in keys {
                nobody.entry(k).or_default().0 += 1;
            }
            for k in distinct {
                nobody.entry(k).or_default().1 += 1;
            }
        }
        let lo = gt_glirgen::lower_module(&art.types, &mir);
        for (_, _, u) in &lo.bad {
            *why.entry(u.0.clone()).or_default() += 1;
        }
        inst_total += lo.module.funcs.len() + lo.bad.len();
        let ok_nodes: std::collections::BTreeSet<_> = lo.funcs.values().map(|i| i.node).collect();
        for b in &fns {
            if ok_nodes.contains(&b.node) {
                lowered += 1;
            } else if let Some((_, _, u)) = lo.bad.iter().find(|(_, i, _)| i.node == b.node) {
                *why_body.entry(u.0.clone()).or_default() += 1;
            } else {
                assert!(
                    lo.no_instance.contains(&b.key),
                    "{}: thân không được tính",
                    b.key
                );
                *why_body
                    .entry("thân generic không có instance".into())
                    .or_default() += 1;
            }
        }
        let m = &lo.module;
        let errs = gt_glir::verify(m);
        assert!(
            errs.is_empty(),
            "{}: {errs:?}\n{}",
            p.display(),
            gt_glir::print(m)
        );
        for f in &m.funcs {
            inst_ok += 1;
            let inst = &lo.funcs[&f.name];
            // thân gốc: hàm hoặc thân hợp đồng `⊳`/`⊲` (FB-P2-03)
            let b = mir
                .bodies
                .iter()
                .find(|b| b.node == inst.node)
                .expect("thân gốc");
            let cover = gt_glirgen::check_abort_cover(b, f);
            assert!(cover.is_empty(), "{}: {cover:?}", b.key);
        }
        let text = gt_glir::print(m);
        let back = gt_glir::parse(&text)
            .unwrap_or_else(|e| panic!("{}: in ra không đọc lại được: {e}", p.display()));
        assert_eq!(&back, m, "{}: đọc(in) ≠ gốc", p.display());
    }
    eprintln!("GLIR_COVER bodies lowered={lowered} total={total}");
    let mut by: Vec<_> = why_body.into_iter().collect();
    by.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (w, n) in by.iter().take(200) {
        eprintln!("  thân chưa hạ: {n:5}  {w}");
    }
    eprintln!("GLIR_COVER instances lowered={inst_ok} total={inst_total}");
    let mut by: Vec<_> = why.into_iter().collect();
    by.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (w, n) in by.iter().take(200) {
        eprintln!("  instance chưa hạ: {n:5}  {w}");
    }
    let mut by: Vec<_> = nobody.into_iter().collect();
    by.sort_by_key(|(k, (n, _))| (std::cmp::Reverse(*n), k.clone()));
    for (k, (n, nb)) in &by {
        eprintln!("  callee không thân: {n:5} lời gọi trong {nb:4} thân  {k}");
    }
    // ngưỡng dưới (chỉ được nâng): thân generic theo instance, ADR-P2-02 — 2026-11-25; lời gọi runtime Arena — 2026-11-26 (3524/13385 đo trên MIR
    // TRƯỚC M7); FB-P2-03 — 2026-11-26: đo lại trên MIR SAU M7 (đầu vào của codegen; thêm thân hợp đồng ⊳/⊲ làm instance): 3523/13404;
    // `root_*`, `Cap<Alloc>::arena`, phép bit u32 prelude — 2026-11-26: 3531/13415; `Cell::new/get/set` — 2026-11-26: 3534/13418
    assert!(
        lowered >= 3534 && inst_ok >= 13418,
        "độ phủ GLIR giảm: thân {lowered}/{total}, instance {inst_ok}/{inst_total}"
    );
}

/// ORACLE VI SAI (ADR-P2-02 C3/C2): thân hạ được mà tham số/kết quả là số nguyên/bool/`()`/tuple/struct (lồng nhau) chạy trên GLIR interp (Rust thuần, theo BYTE) và
/// thông dịch MIR với đầu vào thiên về BIÊN; kết quả (giải mã byte theo kiểu Grit) và loại abort phải KHỚP; GLIR `Ub` (bước hạ quên chốt chặn) là LỖI; hết nhiên liệu ⇒ bỏ qua.
fn adt_fields(
    db: &gt_types::TypeDb,
    def: &gt_resolve::DefId,
    args: &[gt_types::Ty],
) -> Option<Vec<gt_types::Ty>> {
    let adt = db.adts.get(def)?;
    let gt_types::AdtKind::Struct(_) = &adt.kind else {
        return None;
    };
    if adt.lang.is_some() {
        return None;
    }
    adt_variant_fields(db, def, args, 0)
}

/// kiểu các trường của biến thể `v` (struct: biến thể 0); `None` nếu không có
fn adt_variant_fields(
    db: &gt_types::TypeDb,
    def: &gt_resolve::DefId,
    args: &[gt_types::Ty],
    v: usize,
) -> Option<Vec<gt_types::Ty>> {
    let adt = db.adts.get(def)?;
    let raw: Vec<&gt_types::Ty> = match &adt.kind {
        gt_types::AdtKind::Struct(fs) => fs.iter().map(|f| &f.ty).collect(),
        gt_types::AdtKind::Enum(vs) => vs.get(v)?.fields.iter().collect(),
    };
    Some(
        raw.iter()
            .map(|t| {
                t.subst(&|p| {
                    adt.generics
                        .iter()
                        .position(|g| g.id == p)
                        .and_then(|i| args.get(i).cloned())
                })
            })
            .collect(),
    )
}

fn n_variants(db: &gt_types::TypeDb, def: &gt_resolve::DefId) -> Option<usize> {
    match &db.adts.get(def)?.kind {
        gt_types::AdtKind::Enum(vs) => Some(vs.len()),
        gt_types::AdtKind::Struct(_) => None,
    }
}

fn plain(db: &gt_types::TypeDb, t: &gt_types::Ty, d: u32) -> bool {
    use gt_types::Ty;
    d <= 6
        && match t {
            Ty::Int(it) => it.bits() <= 64,
            Ty::Bool => true,
            Ty::Tuple(v) => v.iter().all(|x| plain(db, x, d + 1)),
            Ty::Array(e, n) => *n <= 8 && plain(db, e, d + 1),
            Ty::Adt(def, args) => match n_variants(db, def) {
                Some(n) => (0..n).all(|v| {
                    adt_variant_fields(db, def, args, v)
                        .is_some_and(|fs| fs.iter().all(|x| plain(db, x, d + 1)))
                }),
                None => adt_fields(db, def, args)
                    .is_some_and(|fs| fs.iter().all(|x| plain(db, x, d + 1))),
            },
            _ => false,
        }
}

fn mask(w: u32) -> u128 {
    if w >= 128 {
        u128::MAX
    } else {
        (1u128 << w) - 1
    }
}

fn gen_val(
    db: &gt_types::TypeDb,
    t: &gt_types::Ty,
    rnd: &mut dyn FnMut() -> u64,
) -> gt_mir::interp::V {
    use gt_mir::interp::V;
    use gt_types::Ty;
    match t {
        Ty::Bool => V::Bool(rnd() % 2 == 1),
        Ty::Int(it) => {
            let (w, m) = (it.bits(), mask(it.bits()));
            let top = 1u128 << (w - 1);
            let pool = [
                0,
                1,
                2,
                3,
                7,
                100,
                m,
                m - 1,
                top,
                top - 1,
                top + 1,
                u128::from(rnd()) & m,
            ];
            let bits = pool[(rnd() % pool.len() as u64) as usize] & m;
            let v = if it.signed() && (bits >> (w - 1)) & 1 == 1 {
                (bits as i128) - (1i128 << w)
            } else {
                bits as i128
            };
            V::Int(v, *it)
        }
        Ty::Tuple(v) if v.is_empty() => V::Unit,
        Ty::Array(e, n) => V::Array((0..*n).map(|_| gen_val(db, e, rnd)).collect()),
        Ty::Tuple(v) => V::Tuple(v.iter().map(|x| gen_val(db, x, rnd)).collect()),
        Ty::Adt(def, args) => {
            let v = n_variants(db, def).map_or(0, |n| (rnd() % n as u64) as usize);
            V::Adt(
                v as u32,
                adt_variant_fields(db, def, args, v)
                    .expect("plain")
                    .iter()
                    .map(|x| gen_val(db, x, rnd))
                    .collect(),
            )
        }
        _ => unreachable!(),
    }
}

fn glir_ty_of(db: &gt_types::TypeDb, t: &gt_types::Ty) -> gt_glir::Ty {
    gt_glirgen::lower_ty(db, t).expect("kiểu plain hạ được")
}

/// (kiểu trường, offset tuyệt đối-tương-đối-`base` từng trường) của biến thể `v` của enum `t`: tag i64 ở 0, payload bắt đầu ở offset trường 1 của struct enum
fn enum_payload(
    db: &gt_types::TypeDb,
    t: &gt_types::Ty,
    def: &gt_resolve::DefId,
    args: &[gt_types::Ty],
    v: usize,
) -> (Vec<gt_types::Ty>, Vec<usize>) {
    let ge = glir_ty_of(db, t);
    let pbase = gt_glir::layout::field_offset(&ge, 1).expect("payload") as usize;
    let fts = adt_variant_fields(db, def, args, v).expect("plain");
    let vs = gt_glir::Ty::Struct(fts.iter().map(|x| glir_ty_of(db, x)).collect());
    let offs = (0..fts.len())
        .map(|i| pbase + gt_glir::layout::field_offset(&vs, i as u32).expect("offset") as usize)
        .collect();
    (fts, offs)
}

fn encode(
    db: &gt_types::TypeDb,
    t: &gt_types::Ty,
    v: &gt_mir::interp::V,
    buf: &mut [Option<u8>],
    base: usize,
) {
    use gt_mir::interp::V;
    use gt_types::Ty;
    match (t, v) {
        (Ty::Bool, V::Bool(b)) => buf[base] = Some(u8::from(*b)),
        (Ty::Int(it), V::Int(x, _)) => {
            let n = (it.bits() / 8) as usize;
            for (i, slot) in buf[base..base + n].iter_mut().enumerate() {
                *slot = Some((((*x as u128) >> (8 * i)) & 0xff) as u8);
            }
        }
        (Ty::Tuple(ts), V::Unit) if ts.is_empty() => {}
        (Ty::Array(e, _), V::Array(vs)) => {
            let sz = gt_glir::layout::layout(&glir_ty_of(db, e)).size as usize;
            for (i, ev) in vs.iter().enumerate() {
                encode(db, e, ev, buf, base + i * sz);
            }
        }
        (Ty::Adt(def, args), V::Adt(vi, vs)) if n_variants(db, def).is_some() => {
            for i in 0..8 {
                buf[base + i] = Some((u64::from(*vi) >> (8 * i)) as u8);
            }
            let (fts, off) = enum_payload(db, t, def, args, *vi as usize);
            for (i, (ft, fv)) in fts.iter().zip(vs).enumerate() {
                encode(db, ft, fv, buf, base + off[i]);
            }
        }
        (Ty::Tuple(_) | Ty::Adt(..), _) => {
            let (fts, fvs): (Vec<gt_types::Ty>, &Vec<V>) = match (t, v) {
                (Ty::Tuple(ts), V::Tuple(vs)) => (ts.clone(), vs),
                (Ty::Adt(def, args), V::Adt(_, vs)) => {
                    (adt_fields(db, def, args).expect("plain"), vs)
                }
                _ => panic!("encode: giá trị không khớp kiểu"),
            };
            let g = glir_ty_of(db, t);
            for (i, (ft, fv)) in fts.iter().zip(fvs).enumerate() {
                let off = gt_glir::layout::field_offset(&g, i as u32).expect("offset") as usize;
                encode(db, ft, fv, buf, base + off);
            }
        }
        _ => panic!("encode: kiểu/giá trị lạ {t:?}/{v:?}"),
    }
}

fn decode_eq(
    db: &gt_types::TypeDb,
    t: &gt_types::Ty,
    v: &gt_mir::interp::V,
    bytes: &[Option<u8>],
    base: usize,
) -> bool {
    use gt_mir::interp::V;
    use gt_types::Ty;
    match (t, v) {
        (Ty::Bool, V::Bool(b)) => bytes[base] == Some(u8::from(*b)),
        (Ty::Int(it), V::Int(x, _)) => {
            let n = (it.bits() / 8) as usize;
            (0..n).all(|i| bytes[base + i] == Some((((*x as u128) >> (8 * i)) & 0xff) as u8))
        }
        (Ty::Tuple(ts), V::Unit) if ts.is_empty() => true,
        (Ty::Array(e, _), V::Array(vs)) => {
            let sz = gt_glir::layout::layout(&glir_ty_of(db, e)).size as usize;
            vs.iter()
                .enumerate()
                .all(|(i, ev)| decode_eq(db, e, ev, bytes, base + i * sz))
        }
        (Ty::Adt(def, args), V::Adt(vi, vs)) if n_variants(db, def).is_some() => {
            (0..8).all(|i| bytes[base + i] == Some((u64::from(*vi) >> (8 * i)) as u8)) && {
                let (fts, off) = enum_payload(db, t, def, args, *vi as usize);
                fts.iter()
                    .zip(vs)
                    .enumerate()
                    .all(|(i, (ft, fv))| decode_eq(db, ft, fv, bytes, base + off[i]))
            }
        }
        (Ty::Tuple(ts), V::Tuple(vs)) => {
            let g = glir_ty_of(db, t);
            ts.iter().zip(vs).enumerate().all(|(i, (ft, fv))| {
                decode_eq(
                    db,
                    ft,
                    fv,
                    bytes,
                    base + gt_glir::layout::field_offset(&g, i as u32).expect("offset") as usize,
                )
            })
        }
        (Ty::Adt(def, args), V::Adt(_, vs)) => {
            let g = glir_ty_of(db, t);
            let fts = adt_fields(db, def, args).expect("plain");
            fts.iter().zip(vs).enumerate().all(|(i, (ft, fv))| {
                decode_eq(
                    db,
                    ft,
                    fv,
                    bytes,
                    base + gt_glir::layout::field_offset(&g, i as u32).expect("offset") as usize,
                )
            })
        }
        _ => false,
    }
}

/// Số liệu của một lượt oracle: (instance chạy, lượt so, lượt abort khớp, instance GENERIC (args ≠ []) chạy)
type DiffStats = (usize, usize, usize, usize);

/// Chạy oracle trên MỌI instance hạ được (thân không generic = instance args=[]) có chữ ký plain SAU KHI THẾ: thông dịch MIR `call_instance` vs GLIR.
fn diff_src(
    src: &str,
    rnd: &mut dyn FnMut() -> u64,
    main_only: bool,
    seen: &mut std::collections::BTreeSet<String>,
    iters: usize,
) -> DiffStats {
    use gt_glir::interp::{Arg, Outcome as GO};
    use gt_mir::interp::{Interp, Outcome as MO, V};
    use gt_types::Ty;
    let (mut bodies, mut runs, mut aborts, mut generic) = (0usize, 0usize, 0usize, 0usize);
    let Ok(res) = check(
        &[SourceInput {
            name: "main.gt".to_string(),
            text: src.to_string(),
        }],
        &Options::default(),
    ) else {
        return (0, 0, 0, 0);
    };
    let Some(art) = res.open_gate() else {
        return (0, 0, 0, 0);
    };
    let db = &art.types.db;
    let mir = glir_input_mir(art);
    let lo = gt_glirgen::lower_module(&art.types, &mir);
    let module = &lo.module;
    // tối đa 4 instance generic cho MỖI thân (mẫu bùng nổ instance gần trần E0317 sinh hàng nghìn instance cùng thân)
    let mut per_body: BTreeMap<gt_ast::NodeId, usize> = BTreeMap::new();
    for (gname, inst) in &lo.funcs {
        let Some(b) = mir
            .bodies
            .iter()
            .find(|b| b.kind == gt_mir::BodyKind::Fn && b.node == inst.node)
        else {
            continue;
        };
        if (main_only && !b.key.starts_with("main::"))
            || !(b.key.starts_with("main::") || b.key.starts_with("$prelude::"))
            || (!b.key.starts_with("main::") && !seen.insert(gname.clone()))
        {
            continue;
        }
        // kiểu tham số/kết quả ĐÃ THẾ theo instance (độc lập với `Lw::sub` của bước hạ: thế bằng `Ty::subst` trực tiếp)
        let ps = db.fn_params(b.sig_node);
        let sub = |t: &Ty| {
            t.subst(&|p| {
                ps.iter()
                    .position(|q| *q == p)
                    .and_then(|i| inst.args.get(i).cloned())
            })
        };
        let ptys: Vec<Ty> = (1..=b.arg_count).map(|i| sub(&b.locals[i].ty)).collect();
        let rty = &sub(&b.locals[0].ty);
        if !ptys.iter().all(|t| plain(db, t, 0) || matches!(t, Ty::Ref(_, i) if (plain(db, i, 0) && !matches!(**i, Ty::Tuple(ref v) if v.is_empty())) || matches!(&**i, Ty::Str) || matches!(&**i, Ty::Slice(e) if plain(db, e, 0)))) || !plain(db, rty, 0) {
            continue;
        }
        if !inst.args.is_empty() {
            let c = per_body.entry(inst.node).or_default();
            if *c >= 4 {
                continue;
            }
            *c += 1;
            generic += 1;
        }
        bodies += 1;
        // không tham số ⇒ tất định: một lượt là đủ
        let iters = if ptys.is_empty() { 1 } else { iters };
        for _ in 0..iters {
            let mut mi = Interp::new(&mir, 200_000).with_types(&art.types);
            let mut margs: Vec<V> = Vec::new();
            let mut gargs: Vec<Arg> = Vec::new();
            // (kiểu pointee, tham chiếu MIR, số phần tử nếu là slice/str) của từng tham số tham chiếu, theo thứ tự
            let mut refs: Vec<(Ty, V, Option<usize>)> = Vec::new();
            for t in &ptys {
                if let Ty::Ref(_, inner) = t {
                    match &**inner {
                        Ty::Str => {
                            let pool = ["", "a", "héllo", "xyz", "日本語", "a\u{0}b", "ab"];
                            let st = pool[(rnd() % pool.len() as u64) as usize];
                            let bytes: Vec<Option<u8>> = st.bytes().map(Some).collect();
                            gargs.push(Arg::Fat(bytes, st.len() as u64));
                            let r = mi.alloc_ref(V::Str(st.to_string()));
                            refs.push(((**inner).clone(), r.clone(), Some(st.len())));
                            margs.push(r);
                        }
                        Ty::Slice(e) => {
                            let n = (rnd() % 5) as usize;
                            let vals: Vec<V> = (0..n).map(|_| gen_val(db, e, rnd)).collect();
                            let sz = gt_glir::layout::layout(&glir_ty_of(db, e)).size as usize;
                            let mut buf = vec![None; n * sz];
                            for (i, ev) in vals.iter().enumerate() {
                                encode(db, e, ev, &mut buf, i * sz);
                            }
                            gargs.push(Arg::Fat(buf, n as u64));
                            let r = mi.alloc_ref(V::Array(vals));
                            refs.push(((**e).clone(), r.clone(), Some(n)));
                            margs.push(r);
                        }
                        _ => {
                            let v = gen_val(db, inner, rnd);
                            let n = gt_glir::layout::layout(&glir_ty_of(db, inner)).size as usize;
                            let mut buf = vec![None; n];
                            encode(db, inner, &v, &mut buf, 0);
                            gargs.push(Arg::Ref(buf));
                            let r = mi.alloc_ref(v);
                            refs.push(((**inner).clone(), r.clone(), None));
                            margs.push(r);
                        }
                    }
                    continue;
                }
                let v = gen_val(db, t, rnd);
                match t {
                    Ty::Tuple(ts) if ts.is_empty() => {}
                    Ty::Bool => gargs.push(Arg::Int(u128::from(matches!(v, V::Bool(true))))),
                    Ty::Int(it) => {
                        let V::Int(x, _) = &v else { unreachable!() };
                        gargs.push(Arg::Int((*x as u128) & mask(it.bits())));
                    }
                    _ => {
                        let n = gt_glir::layout::layout(&glir_ty_of(db, t)).size as usize;
                        let mut buf = vec![None; n];
                        encode(db, t, &v, &mut buf, 0);
                        gargs.push(Arg::Bytes(buf));
                    }
                }
                margs.push(v);
            }
            let (g, fin) = gt_glir::interp::run_module_refs(module, gname, &gargs, 200_000);
            let m = mi.call_instance(&b.key, inst.args.clone(), margs.clone());
            let m = match m {
                MO::Value(v) => Ok(v),
                MO::Abort(k) => Err(k),
                _ => continue,
            };
            runs += 1;
            let ctx = || {
                format!(
                    "{} {:?} {gargs:?}\n{}",
                    b.key,
                    inst.args,
                    gt_glir::print(&gt_glir::Module {
                        globals: vec![],
                        decls: vec![],
                        funcs: module
                            .funcs
                            .iter()
                            .filter(|f| &f.name == gname)
                            .cloned()
                            .collect()
                    })
                )
            };
            let finished = matches!(g, GO::Value(_) | GO::Agg(_));
            match (g, m) {
                (GO::OutOfFuel, _) => {}
                (GO::Ub(u), m) => panic!(
                    "{}: GLIR chạm hành vi không xác định ({u}); MIR cho {m:?}",
                    ctx()
                ),
                (GO::Abort(k), Err(mk)) => {
                    aborts += 1;
                    assert_eq!(k, mk, "{}", ctx());
                }
                (GO::Value(None), Ok(V::Unit)) => {}
                (GO::Value(Some(x)), Ok(V::Int(y, it))) => {
                    assert_eq!(x, (y as u128) & mask(it.bits()), "{}", ctx())
                }
                (GO::Value(Some(x)), Ok(V::Bool(y))) => assert_eq!(x, u128::from(y), "{}", ctx()),
                (GO::Agg(bytes), Ok(v)) => assert!(
                    decode_eq(db, rty, &v, &bytes, 0),
                    "{}: GLIR trả {bytes:?}, MIR trả {v:?}",
                    ctx()
                ),
                (g, m) => panic!("{}: GLIR {g:?} ≠ MIR {m:?}", ctx()),
            }
            // trạng thái sau lời gọi của vùng mà `&T`/`&!T` trỏ tới phải khớp (chỉ khi không abort giữa chừng)
            if finished {
                for ((inner, r, cnt), bytes) in refs.iter().zip(&fin) {
                    let fv = mi.read_ref(r).expect("vùng tham chiếu còn đó");
                    let ok = match (cnt, &fv) {
                        (None, _) => decode_eq(db, inner, &fv, bytes, 0),
                        (Some(_), V::Str(st)) => {
                            bytes.iter().map(|b| b.unwrap_or(0)).collect::<Vec<u8>>()
                                == st.as_bytes()
                        }
                        (Some(_), V::Array(vs)) => {
                            let sz = gt_glir::layout::layout(&glir_ty_of(db, inner)).size as usize;
                            vs.iter()
                                .enumerate()
                                .all(|(i, ev)| decode_eq(db, inner, ev, bytes, i * sz))
                        }
                        _ => false,
                    };
                    assert!(
                        ok,
                        "{}: trạng thái sau lời gọi của vùng tham chiếu: GLIR {bytes:?}, MIR {fv:?}",
                        ctx()
                    );
                }
            }
        }
    }
    (bodies, runs, aborts, generic)
}

#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_boundary_inputs() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/pass");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .expect("dir")
        .map(|e| e.expect("e").path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .collect();
    files.sort();
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (mut bodies, mut runs, mut aborts, mut generic) = (0usize, 0usize, 0usize, 0usize);
    let mut seen = std::collections::BTreeSet::new();
    for p in files {
        let src = std::fs::read_to_string(&p).expect("đọc");
        let (b, r, a, g) = diff_src(&src, &mut rnd, false, &mut seen, 40);
        bodies += b;
        runs += r;
        aborts += a;
        generic += g;
    }
    eprintln!("GLIR_DIFF instances={bodies} generic={generic} runs={runs} aborts={aborts}");
    assert!(
        bodies >= 5 && runs >= 300 && aborts >= 5 && generic >= 20,
        "oracle quá nhỏ: instances={bodies} generic={generic} runs={runs} aborts={aborts}"
    );
}

/// Chương trình SINH: mọi phép × mọi độ rộng có/không dấu (số học có kiểm, bao quanh, so sánh, Not/Neg, ép kiểu giữa mọi cặp độ rộng), để oracle vi sai
/// không phụ thuộc vào việc corpus có hàm số học đủ đa dạng (đột biến công thức tràn có dấu từng SỐNG khi chỉ dùng corpus).
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_arithmetic() {
    let tys = ["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64"];
    let mut src = String::new();
    for t in tys {
        for (n, op) in [
            ("add", "+"),
            ("sub", "-"),
            ("mul", "*"),
            ("div", "/"),
            ("rem", "%"),
        ] {
            src.push_str(&format!(
                "ƒ {n}_{t}(a: {t}, b: {t}) → {t} ! ∅ {{ a {op} b }}\n"
            ));
        }
        for (n, op) in [
            ("lt", "<"),
            ("le", "<="),
            ("gt", ">"),
            ("ge", ">="),
            ("eq", "=="),
            ("ne", "≠"),
        ] {
            src.push_str(&format!(
                "ƒ {n}_{t}(a: {t}, b: {t}) → bool ! ∅ {{ a {op} b }}\n"
            ));
        }
        for t2 in tys {
            if t != t2 {
                src.push_str(&format!(
                    "ƒ cast_{t}_{t2}(a: {t}) → {t2} ! ∅ {{ a as {t2} }}\n"
                ));
            }
        }
        if t.starts_with('i') {
            src.push_str(&format!("ƒ neg_{t}(a: {t}) → {t} ! ∅ {{ -a }}\n"));
        }
    }
    src.push_str("ƒ main() → i32 ! ∅ { 0 }\n");
    let mut seed = 0xD1B5_4A32_D192_ED03u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, _) = diff_src(&src, &mut rnd, true, &mut Default::default(), 120);
    eprintln!("GLIR_DIFF_GEN bodies={bodies} runs={runs} aborts={aborts}");
    assert!(
        bodies >= 100 && runs >= 12000 && aborts >= 1000,
        "oracle sinh quá nhỏ: bodies={bodies} runs={runs} aborts={aborts}"
    );
}

/// Kiểm (d) phải BẮT được một khối Abort bị làm mất: lấy một thân có `Assert`, thay khối `Abort` đầu tiên bằng `Ret`/`Br` ⇒ `G0501`.
#[test]
fn abort_cover_check_catches_a_dropped_abort_block() {
    let src = "ƒ add(a: u8, b: u8) → u8 ! ∅ { a + b }\nƒ main() → i32 ! ∅ { 0 }\n";
    let res = check(
        &[SourceInput {
            name: "main.gt".to_string(),
            text: src.to_string(),
        }],
        &Options::default(),
    )
    .expect("check");
    let art = res.open_gate().expect("cổng");
    let mir = glir_input_mir(art);
    let b = mir
        .bodies
        .iter()
        .find(|b| b.key == "main::add")
        .expect("thân add");
    let f = gt_glirgen::lower_body(&art.types.db, b).expect("hạ được");
    assert!(
        gt_glirgen::check_abort_cover(b, &f).is_empty(),
        "bản gốc phải đủ"
    );
    let mut broken = f.clone();
    let i = broken
        .blocks
        .iter()
        .position(|x| matches!(x.term, gt_glir::Term::Abort(_)))
        .expect("có khối Abort");
    broken.blocks[i].term = gt_glir::Term::Ret(Some((
        gt_glir::Ty::I8,
        gt_glir::Opd::Int(gt_glir::Ty::I8, 0),
    )));
    let errs = gt_glirgen::check_abort_cover(b, &broken);
    assert!(
        errs.iter().any(|e| e.starts_with("G0501")),
        "phải báo G0501, có {errs:?}"
    );
}

/// Chương trình SINH có enum: tag + payload hẹp/rộng/lồng, `⌥` đọc payload, sao chép qua hàm, `Option`/`Result` bọc enum. Kiểm GLIR (tag `i64` + payload `[k × i64]` nhìn như
/// struct biến thể) ≡ thông dịch MIR, gồm cả byte payload chưa ghi lúc sao chép.
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_enums() {
    let src = "\
▢ P { a: u8, b: u32 }
⊔ E { A, B(u8), C(u32, u8), D(bool, i16), F(P) }
ƒ id_e(e: E) → E ! ∅ { e }
ƒ mk_b(x: u8) → E ! ∅ { E::B(x) }
ƒ mk_c(x: u32, y: u8) → E ! ∅ { E::C(x, y) }
ƒ mk_f(a: u8, b: u32) → E ! ∅ { E::F(P { a: a, b: b }) }
ƒ pick(e: E) → u32 ! ∅ {
  ⌥ e { E::A ↦ 1u32, E::B(x) ↦ x as u32, E::C(x, y) ↦ x +% (y as u32), E::D(f, z) ↦ ⎇ f { z as u32 } ⇝ { 7u32 }, E::F(p) ↦ p.b +% (p.a as u32) }
}
ƒ twice(e: E) → u32 ! ∅ { f ≔ id_e(e); g ≔ id_e(f); pick(g) }
ƒ wrap(e: E) → Option<E> ! ∅ { Some(e) }
ƒ unwrap_or_a(o: Option<E>) → E ! ∅ { ⌥ o { Some(x) ↦ x, None ↦ E::A } }
ƒ opt_u8(x: u8, b: bool) → Option<u8> ! ∅ { ⎇ b { Some(x) } ⇝ { None } }
ƒ res(x: u8) → Result<u8, bool> ! ∅ { ⎇ x > 100u8 { Err(true) } ⇝ { Ok(x) } }
ƒ chain(o: Option<u8>) → u8 ! ∅ { ⌥ o { Some(x) ↦ x +% 1u8, None ↦ 0u8 } }
ƒ main() → i32 ! ∅ { 0 }
";
    let mut seed = 0xA5A5_1234_DEAD_BEEFu64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, _) = diff_src(src, &mut rnd, true, &mut Default::default(), 150);
    eprintln!("GLIR_DIFF_ENUM bodies={bodies} runs={runs} aborts={aborts}");
    assert!(
        bodies >= 9 && runs >= 1000,
        "oracle enum quá nhỏ: bodies={bodies} runs={runs}"
    );
}

/// Chương trình SINH có mảng cố định: chỉ số biến (kèm kiểm biên ⇒ abort `index_out_of_bounds`), `Len`, `Repeat`, mảng chứa tuple/enum, ghi qua `&!`.
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_arrays() {
    let src = "\
ƒ get4(a: [u32; 4], i: usize) → u32 ! ∅ { a[i] }
ƒ get3b(a: [u8; 3], i: usize) → u8 ! ∅ { a[i] }
ƒ len_of(a: &[u16; 5]) → usize ! ∅ { s: &[u16] ≔ a; s.len() }
ƒ rep(x: u32) → [u32; 4] ! ∅ { [x; 4] }
ƒ lit(x: u8, y: u8) → [u8; 3] ! ∅ { [x, y, x] }
ƒ pairs(a: [(u8, u32); 2], i: usize) → u32 ! ∅ { a[i].1 }
ƒ set(a: &![u32; 3], i: usize, v: u32) ! ∅ { a[i] ← v; }
ƒ sum3(a: [i16; 3]) → i16 ! ∅ { a[0] +% a[1] +% a[2] }
ƒ opt_arr(a: [Option<u8>; 2], i: usize) → u8 ! ∅ { ⌥ a[i] { Some(x) ↦ x, None ↦ 0u8 } }
ƒ main() → i32 ! ∅ { 0 }
";
    let mut seed = 0x0123_4567_89AB_CDEFu64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, _) = diff_src(src, &mut rnd, true, &mut Default::default(), 150);
    eprintln!("GLIR_DIFF_ARR bodies={bodies} runs={runs} aborts={aborts}");
    assert!(
        bodies >= 7 && runs >= 1000 && aborts >= 50,
        "oracle mảng quá nhỏ: bodies={bodies} runs={runs} aborts={aborts}"
    );
}

/// Chương trình SINH có slice/`&str`: tham số `&[T]`/`&!`-slice/`&str` (con trỏ béo, cấp bởi `Arg::Fat`), chỉ số biến kèm kiểm biên, `Len`, lát con `&s[a..b]`,
/// chuỗi hằng (global), ép `&[T;N]`→`&[T]`; so kết quả VÀ trạng thái vùng sau lời gọi với thông dịch MIR.
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_slices() {
    let src = "\
ƒ first(s: &[u32]) → u32 ! ∅ { s[0usize] }
ƒ at(s: &[u8], i: usize) → u8 ! ∅ { s[i] }
ƒ count(s: &[u16]) → usize ! ∅ { s.len() }
ƒ tail_len(s: &[u32]) → usize ! ∅ { t ≔ &s[1usize..]; t.len() }
ƒ mid(s: &[u8], a: usize, b: usize) → usize ! ∅ { t ≔ &s[a..b]; t.len() }
ƒ sum_pair(s: &[i32]) → i32 ! ∅ { s[0usize] +% s[1usize] }
ƒ put(s: &![u32], i: usize, v: u32) ! ∅ { s[i] ← v; }
ƒ lit_len() → usize ! ∅ { t ≔ \"héllo\"; t.len() }
ƒ str_len(t: &str) → usize ! ∅ { t.len() }
ƒ arr_as_slice(a: &[u8; 4]) → usize ! ∅ { s: &[u8] ≔ a; s.len() }
ƒ main() → i32 ! ∅ { 0 }
";
    let mut seed = 0xFEED_FACE_0BAD_F00Du64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, _) = diff_src(src, &mut rnd, true, &mut Default::default(), 150);
    eprintln!("GLIR_DIFF_SLICE bodies={bodies} runs={runs} aborts={aborts}");
    assert!(
        bodies >= 7 && runs >= 1000 && aborts >= 100,
        "oracle slice quá nhỏ: bodies={bodies} runs={runs} aborts={aborts}"
    );
}

/// Chương trình SINH có GENERIC (ADR-P2-02, thân generic theo instance): hàm generic gọi hàm generic, phương thức trait qua ràng buộc (`T: Score`
/// ⇒ `Callee::TraitMethod` phân giải theo instance), struct generic + `⊐ <T>`, `Option<T>`, mảng `[T; 3]`, đa hình hai tham số. Oracle chạy TRỰC TIẾP
/// từng instance generic (`call_instance`) so với hàm GLIR của instance đó. Phương thức trait CÓ generic riêng (`conv<U>`) gọi qua ràng buộc
/// (`Callee::TraitMethod` với `generics` ≠ []). Ngưỡng `generic` = đúng số instance generic hiện có (mất một instance ⇒ đỏ).
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_generics() {
    let src = "\
⊏ Score { ƒ score(&self) → u32 ! ∅; }
▢ A { x: u8, y: u32 }
▢ B { k: i16 }
⊐ Copy for B { }
⊐ Score for A { ƒ score(&self) → u32 ! ∅ { (self.x as u32) + self.y } }
⊐ Score for B { ƒ score(&self) → u32 ! ∅ { ⎇ self.k < 0i16 { 7u32 } ⇝ { (self.k as u32) * 3u32 } } }
ƒ id<T>(x: T) → T ! ∅ { x }
ƒ pick<T>(c: bool, x: T, y: T) → T ! ∅ { ⎇ c { id(x) } ⇝ { y } }
ƒ pair<T, U>(x: T, y: U) → (U, T) ! ∅ { (y, x) }
ƒ sc<T: Score>(a: T) → u32 ! ∅ { s ≔ &a; s.score() }
ƒ twice<T: Score>(a: &T) → u32 ! ∅ { a.score() + a.score() }
ƒ via<T: Score>(a: T, b: T, c: bool) → u32 ! ∅ { p ≔ pick(c, a, b); sc(p) }
▢ Two<T> { a: T, b: T }
⊐ <T: Copy> Two<T> {
  ƒ fst(&self) → T ! ∅ { self.a }
  ƒ swap(&self) → Two<T> ! ∅ { Two { a: self.b, b: self.a } }
}
ƒ mk<T>(a: T, b: T) → Two<T> ! ∅ { Two { a: a, b: b } }
ƒ wrap<T>(x: T) → Option<T> ! ∅ { Some(x) }
ƒ or_else<T>(o: Option<T>, d: T) → T ! ∅ { ⌥ o { Some(v) ↦ v, None ↦ d } }
ƒ at<T: Copy>(a: [T; 3], i: usize) → T ! ∅ { a[i] }
ƒ use1(a: A, b: A, c: bool) → u32 ! ∅ { via(a, b, c) }
ƒ use2(x: B) → u32 ! ∅ { twice(&x) }
ƒ use3(x: u8, y: i64) → (i64, u8) ! ∅ { pair(x, y) }
ƒ use4(a: u16, b: u16) → u16 ! ∅ { m ≔ mk(a, b); n ≔ m.swap(); n.fst() }
ƒ use5(x: i32, c: bool, d: i32) → i32 ! ∅ { o ≔ ⎇ c { wrap(x) } ⇝ { None }; or_else(o, d) }
ƒ use6(a: [i8; 3], i: usize) → i8 ! ∅ { at(a, i) }
ƒ use7(a: [B; 3], i: usize) → u32 ! ∅ { b ≔ at(a, i); sc(b) }
ƒ use8(x: bool, y: (u8, bool)) → (bool, (u8, bool)) ! ∅ { pair(y, x) }
ƒ use9(a: B, b: B, c: bool) → u32 ! ∅ { via(a, b, c) }
ƒ use10(a: Two<u8>) → u8 ! ∅ { a.swap().fst() }
⊏ Conv { ƒ conv<U: Copy>(&self, u: U) → (U, u32) ! ∅; }
⊐ Conv for A { ƒ conv<U: Copy>(&self, u: U) → (U, u32) ! ∅ { (u, self.y + 1u32) } }
ƒ cv<T: Conv, V: Copy>(t: &T, v: V) → (V, u32) ! ∅ { t.conv(v) }
ƒ use11(a: A, v: i16) → (i16, u32) ! ∅ { cv(&a, v) }
ƒ use12(a: A, v: bool) → (bool, u32) ! ∅ { a.conv(v) }
ƒ main() → i32 ! ∅ { 0 }
";
    let mut seed = 0x5EED_6E4E_21C5_0001u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (insts, runs, aborts, generic) =
        diff_src(src, &mut rnd, true, &mut Default::default(), 150);
    eprintln!("GLIR_DIFF_GENERIC instances={insts} generic={generic} runs={runs} aborts={aborts}");
    assert!(
        generic >= 23 && runs >= 5000 && aborts >= 100,
        "oracle generic quá nhỏ: instances={insts} generic={generic} runs={runs} aborts={aborts}"
    );
}

/// Chương trình SINH dùng `Arena` (lời gọi RUNTIME ⇒ `declare @gritrt_arena_*`; ADR-P2-02): cấp số/struct/enum/mảng/`()`, hai lần cấp cùng arena, `reset` rồi
/// cấp lại, hai arena (reset arena KIA không được làm treo con trỏ của arena này), truyền `&Arena` qua hàm, cấp trong vòng lặp, tràn sau khi đọc qua con trỏ arena.
/// GLIR chạy MÔ HÌNH runtime (`gt_glir::interp::runtime_model_sig`: vùng mới mỗi lần cấp, `reset` giải phóng ⇒ đọc sau là `Ub`) so với thông dịch MIR.
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_arenas() {
    let src = "\
▢ P { a: u8, b: i32 }
⊔ E { A, B(u16), C(i32, bool) }
⊐ Copy for E { }
ƒ one(x: i32) → i32 ! {mem} { a ≔ Arena::new(); p ≔ a.alloc(x); *p +% 1i32 }
ƒ two(x: i32, y: u8) → i32 ! {mem} { a ≔ Arena::new(); p ≔ a.alloc(x); q ≔ a.alloc(y); *p +% (*q as i32) }
ƒ st(x: u8, y: i32) → i32 ! {mem} { a ≔ Arena::new(); p ≔ a.alloc(P { a: x, b: y }); p.b +% (p.a as i32) }
ƒ rs(x: i32, y: i32) → i32 ! {mem} { μa ≔ Arena::new(); p ≔ a.alloc(x); v ≔ *p; a.reset(); q ≔ a.alloc(y); v +% *q }
ƒ put(a: &Arena, x: i32) → i32 ! {mem} { p ≔ a.alloc(x); *p }
ƒ via(x: i32) → i32 ! {mem} { a ≔ Arena::new(); put(&a, x) +% put(&a, 2i32) }
ƒ two_ar(x: i32, y: i32) → i32 ! {mem} { a ≔ Arena::new(); μb ≔ Arena::new(); p ≔ a.alloc(x); b.reset(); q ≔ b.alloc(y); *p +% *q }
ƒ en(x: u16, k: bool) → u16 ! {mem} { a ≔ Arena::new(); e ≔ ⎇ k { E::B(x) } ⇝ { E::A }; p ≔ a.alloc(e); ⌥ *p { E::B(v) ↦ v, E::A ↦ 1u16, E::C(z, w) ↦ 2u16 } }
ƒ arr(x: u32, i: usize) → u32 ! {mem} { a ≔ Arena::new(); p ≔ a.alloc([x, 1u32, 2u32]); p[i] }
ƒ lp(n: u8) → i32 ! {mem} { a ≔ Arena::new(); μs ≔ 0i32; ⟳ i in 0u8..n { p ≔ a.alloc(i as i32); s ← s +% *p; } s }
ƒ ck(x: i32, y: i32) → i32 ! {mem} { a ≔ Arena::new(); p ≔ a.alloc(x); *p + y }
ƒ unit_alloc(x: u8) → u8 ! {mem} { a ≔ Arena::new(); p ≔ a.alloc(()); x }
ƒ main() → i32 ! ∅ { 0 }
";
    let mut seed = 0xA7E4_A000_1234_5678u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, _) = diff_src(src, &mut rnd, true, &mut Default::default(), 150);
    eprintln!("GLIR_DIFF_ARENA bodies={bodies} runs={runs} aborts={aborts}");
    // MỌI hàm có chữ ký plain phải hạ được và được so (11 hàm dùng Arena + `main`; `put` nhận `&Arena` nên không chạy trực tiếp)
    assert!(
        bodies >= 12 && runs >= 1500 && aborts >= 50,
        "oracle arena quá nhỏ: bodies={bodies} runs={runs} aborts={aborts}"
    );
}

/// MIR làm đầu vào cho hạ GLIR (và cho thông dịch MIR của oracle). PHẢI là MIR mà codegen LLVM nhận (`native.rs`: `art.obl.bodies`) — FB-P2-03:
/// thân SAU M7 (Assert tiền điều kiện đã chèn tại điểm gọi, Assert chứng minh được đã xoá), cùng khoá với `art.mir`.
fn glir_input_mir(art: &gt_driver::Artifacts) -> gt_mir::MirOutput {
    gt_mir::MirOutput {
        report: art.mir.report.clone(),
        bodies: art.obl.bodies.clone(),
        keys: art.mir.keys.clone(),
    }
}

/// FB-P2-03: tiền điều kiện `⊳` không chứng minh được ⇒ M7 chèn `Assert(precondition)` tại điểm gọi (N0901) vào thân SAU M7 — thứ codegen LLVM
/// hạ. GLIR phải có khối `Abort(precondition)` tương ứng (kiểm (d) đối chiếu với thân SAU M7).
#[test]
fn glir_is_lowered_from_the_post_m7_mir_that_codegen_consumes() {
    let src = "ƒ g(x: u32) → u32 ! ∅ ⊳ x < 10u32 { x }\nƒ f(x: u32) → u32 ! ∅ { g(x) }\nƒ main() → i32 ! ∅ { 0 }\n";
    let res = check(
        &[SourceInput {
            name: "main.gt".to_string(),
            text: src.to_string(),
        }],
        &Options::default(),
    )
    .expect("check");
    let art = res.open_gate().expect("cổng");
    let mir = glir_input_mir(art);
    let lo = gt_glirgen::lower_module(&art.types, &mir);
    let ob = art
        .obl
        .bodies
        .iter()
        .find(|b| b.key == "main::f")
        .expect("thân f sau M7");
    let f = lo
        .module
        .funcs
        .iter()
        .find(|g| lo.funcs[&g.name].node == ob.node)
        .expect("f hạ được");
    let cover = gt_glirgen::check_abort_cover(ob, f);
    assert!(
        cover.is_empty(),
        "GLIR thiếu chốt mà codegen có: {cover:?}\n{}",
        gt_glir::print(&lo.module)
    );
}

/// Chương trình SINH dùng intrinsic KHÔNG có thân MIR ngoài Arena: phép bit u32 của prelude (`band/bor/bxor/bnot/shl/shr/rotl/rotr`; hạ TẠI CHỖ như `bit_op`
/// của codegen, gồm chốt `n < 32` ⇒ abort `precondition`, xoay không dịch ≥ 32 khi n = 0), `root_alloc` (giá trị `Cap` zero) và `Cap<Alloc>::arena` (= `Arena::new`).
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_intrinsics() {
    let src = "\
ƒ a_and(x: u32, y: u32) → u32 ! ∅ { band(x, y) }
ƒ a_or(x: u32, y: u32) → u32 ! ∅ { bor(x, y) }
ƒ a_xor(x: u32, y: u32) → u32 ! ∅ { bxor(x, y) }
ƒ a_not(x: u32) → u32 ! ∅ { bnot(x) }
ƒ a_shl(x: u32, n: u32) → u32 ! ∅ { shl(x, n) }
ƒ a_shr(x: u32, n: u32) → u32 ! ∅ { shr(x, n) }
ƒ a_rotl(x: u32, n: u32) → u32 ! ∅ { rotl(x, n) }
ƒ a_rotr(x: u32, n: u32) → u32 ! ∅ { rotr(x, n) }
ƒ a_mix(x: u32, n: u8) → u32 ! ∅ { k ≔ (n % 32u8) as u32; bxor(rotl(x, k), shr(x, k)) }
ƒ use_cap(c: &Cap<Alloc>, x: i32) → i32 ! {mem} { ar ≔ c.arena(); p ≔ ar.alloc(x); *p }
ƒ main() → i32 ! {mem} { al ≔ root_alloc(); ar ≔ al.arena(); p ≔ ar.alloc(41i32); *p +% use_cap(&al, 1i32) }
";
    let mut seed = 0xB175_0FF5_E7C0_FFEEu64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, _) = diff_src(src, &mut rnd, true, &mut Default::default(), 200);
    eprintln!("GLIR_DIFF_INTR bodies={bodies} runs={runs} aborts={aborts}");
    // 9 hàm phép bit + `main` (root_alloc, Cap<Alloc>::arena); `use_cap` nhận `&Cap` nên chỉ chạy qua `main`
    assert!(
        bodies >= 10 && runs >= 1800 && aborts >= 100,
        "oracle intrinsic quá nhỏ: bodies={bodies} runs={runs} aborts={aborts}"
    );
}

/// Chương trình SINH dùng `Cell<T>` (intrinsic không thân MIR; hạ tại chỗ như `Intr::Cell*` của codegen: trường 0 của `{v: T}`): số, bool, tuple, struct,
/// ghi qua `&Cell` trong hàm khác, tràn sau `get`.
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_cells() {
    let src = "\
▢ P { a: u8, b: i32 }
⊐ Copy for P { }
ƒ c1(x: i32, y: i32) → i32 ! ∅ { c ≔ Cell::new(x); a ≔ c.get(); c.set(y); a +% c.get() }
ƒ bump(c: &Cell<u8>) ! ∅ { c.set(c.get() +% 1u8); }
ƒ c2(x: u8) → u8 ! ∅ { c ≔ Cell::new(x); bump(&c); bump(&c); c.get() }
ƒ c3(x: u8, y: u16) → u16 ! ∅ { c ≔ Cell::new((x, y)); t ≔ c.get(); c.set((t.0 +% 1u8, t.1)); u ≔ c.get(); (u.0 as u16) +% u.1 }
ƒ c4(x: u32) → u32 ! ∅ { c ≔ Cell::new(x); c.get() + 1u32 }
ƒ c5(x: u8, y: i32) → i32 ! ∅ { c ≔ Cell::new(P { a: x, b: y }); p ≔ c.get(); c.set(P { a: 0u8, b: p.b +% 1i32 }); c.get().b +% (p.a as i32) }
ƒ c6(x: bool) → bool ! ∅ { c ≔ Cell::new(x); c.set(¬c.get()); c.get() }
ƒ main() → i32 ! ∅ { 0 }
";
    let mut seed = 0xCE11_CE11_0000_1111u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, _) = diff_src(src, &mut rnd, true, &mut Default::default(), 150);
    eprintln!("GLIR_DIFF_CELL bodies={bodies} runs={runs} aborts={aborts}");
    // 6 hàm dùng Cell + `main` (`bump` nhận `&Cell` nên chỉ chạy qua `c2`)
    assert!(
        bodies >= 7 && runs >= 900 && aborts >= 10,
        "oracle Cell quá nhỏ: bodies={bodies} runs={runs} aborts={aborts}"
    );
}
