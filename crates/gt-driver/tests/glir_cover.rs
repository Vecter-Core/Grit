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
    // `root_*`, `Cap<Alloc>::arena`, phép bit u32 prelude — 2026-11-26: 3531/13415; `Cell::new/get/set` — 2026-11-26: 3534/13418;
    // trần độ sâu `in_subset` 8 → 128 + ngân sách nút (tuple lồng sâu của `e0317_*`) — 2026-11-26: 3548/21610; `Vec::*` — 2026-11-26: 3563/21625;
    // `Text::*`, `str::slice/as_bytes`, `u8 as char` — 2026-11-26: 3568/21630; `Owned::*` + drop glue (trừ xoá Secret) — 2026-11-26: 3575/21638;
    // `Secret::*` + `Wipe`/xoá `_0`/glue `Wipe` (ADR-P2-03) — 2026-11-26: 3581/21644; `f64` (fbin/funop/fcmp/sitofp/uitofp/fptosi/bitcast) —
    // 2026-11-26: 3586/21649; `Atomic::*` (atomicload/store/rmw/cmpxchg) + `Mutex::new/with` (f hằng, gọi trực tiếp) — 2026-11-26: 3592/21656
    assert!(
        lowered >= 3592 && inst_ok >= 21656,
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

/// Độ sâu kiểu tối đa mà oracle sinh đầu vào (≥ độ sâu tuple lồng của mẫu `e0317_*` và oracle tuple sâu).
const PLAIN_MAX_DEPTH: u32 = 16;

fn plain(db: &gt_types::TypeDb, t: &gt_types::Ty, d: u32) -> bool {
    use gt_types::Ty;
    d <= PLAIN_MAX_DEPTH
        && match t {
            Ty::Int(it) => it.bits() <= 64,
            Ty::Bool | Ty::Float(gt_types::FloatTy::F64) => true,
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

fn is_cap(db: &gt_types::TypeDb, t: &gt_types::Ty) -> bool {
    matches!(t, gt_types::Ty::Adt(def, _) if db.adts.get(def).is_some_and(|a| a.lang == Some(gt_types::Lang::Cap)))
}

/// `&Cap<X>` (X là struct số nguyên của prelude): tham số bằng chứng quyền, oracle truyền giá trị ZERO
fn cap_ref(db: &gt_types::TypeDb, t: &gt_types::Ty) -> bool {
    matches!(t, gt_types::Ty::Ref(false, i) if is_cap(db, i))
}

/// giá trị ZERO (thông dịch MIR) của kiểu struct/tuple số nguyên (giá trị `Cap` do `root_*` tạo)
fn zero_val(db: &gt_types::TypeDb, t: &gt_types::Ty) -> gt_mir::interp::V {
    use gt_mir::interp::V;
    use gt_types::Ty;
    match t {
        Ty::Int(it) => V::Int(0, *it),
        Ty::Bool => V::Bool(false),
        Ty::Tuple(v) if v.is_empty() => V::Unit,
        Ty::Tuple(v) => V::Tuple(v.iter().map(|x| zero_val(db, x)).collect()),
        Ty::Adt(def, args) => V::Adt(
            0,
            adt_variant_fields(db, def, args, 0)
                .expect("struct")
                .iter()
                .map(|x| zero_val(db, x))
                .collect(),
        ),
        _ => panic!("zero_val: kiểu lạ {t:?}"),
    }
}

fn mask(w: u32) -> u128 {
    if w >= 128 {
        u128::MAX
    } else {
        (1u128 << w) - 1
    }
}

/// So hai mẫu bit `f64` theo hợp đồng GLIR (ADR-P2-02 quyết định 2026-11-26): mẫu bit của NaN do phép tính sinh ra KHÔNG được bảo đảm ⇒ mọi NaN
/// coi là bằng nhau (chuẩn hoá trước khi so); mọi giá trị khác (kể cả ±0, ±∞, số dưới chuẩn) so ĐÚNG từng bit.
fn f64_bits_eq(a: u64, b: u64) -> bool {
    let canon = |x: u64| {
        if f64::from_bits(x).is_nan() {
            0x7ff8_0000_0000_0000
        } else {
            x
        }
    };
    canon(a) == canon(b)
}

/// Đầu vào `f64` thiên về biên: ±0, ±1, 0.1, 2^53±1, ±2^63 (biên `to_i64`), 2^63−1024 (số f64 lớn nhất < 2^63), ±∞, NaN (chuẩn, âm, có payload),
/// số dưới chuẩn nhỏ nhất, MAX, MIN_POSITIVE, và mẫu bit ngẫu nhiên.
fn gen_f64(rnd: &mut dyn FnMut() -> u64) -> u64 {
    let pool: [u64; 22] = [
        0.0f64.to_bits(),
        (-0.0f64).to_bits(),
        1.0f64.to_bits(),
        (-1.0f64).to_bits(),
        0.1f64.to_bits(),
        2.5f64.to_bits(),
        (-7.75f64).to_bits(),
        9_007_199_254_740_992.0f64.to_bits(),
        9_007_199_254_740_994.0f64.to_bits(),
        9_223_372_036_854_775_808.0f64.to_bits(),
        (-9_223_372_036_854_775_808.0f64).to_bits(),
        9_223_372_036_854_774_784.0f64.to_bits(),
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        0x7ff8_0000_0000_0000,
        0xfff8_0000_0000_0000,
        0x7ff0_0000_0000_0001,
        1,
        f64::MAX.to_bits(),
        f64::MIN_POSITIVE.to_bits(),
        (1e300f64).to_bits(),
        rnd(),
    ];
    pool[(rnd() % pool.len() as u64) as usize]
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
        Ty::Float(_) => V::Float(f64::from_bits(gen_f64(rnd))),
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
        (Ty::Float(_), V::Float(x)) => {
            for (i, slot) in buf[base..base + 8].iter_mut().enumerate() {
                *slot = Some((x.to_bits() >> (8 * i)) as u8);
            }
        }
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
        (Ty::Float(_), V::Float(x)) => {
            let mut g = 0u64;
            for i in 0..8 {
                match bytes[base + i] {
                    Some(b) => g |= u64::from(b) << (8 * i),
                    None => return false,
                }
            }
            f64_bits_eq(g, x.to_bits())
        }
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
    // module hạ được phải qua VERIFIER (như `every_lowered` làm với corpus): lệnh sai kiểu mà thông dịch tình cờ chạy được vẫn là lỗi hạ
    let verr = gt_glir::verify(module);
    assert!(verr.is_empty(), "verifier GLIR: {verr:?}");
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
        if !ptys.iter().all(|t| plain(db, t, 0) || cap_ref(db, t) || matches!(t, Ty::Ref(_, i) if (plain(db, i, 0) && !matches!(**i, Ty::Tuple(ref v) if v.is_empty())) || matches!(&**i, Ty::Str) || matches!(&**i, Ty::Slice(e) if plain(db, e, 0)))) || !plain(db, rty, 0) {
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
                if cap_ref(db, t) {
                    // `&Cap<X>`: giá trị bằng chứng ZERO (như `root_*`/`Intr::RootCap`), không đổi sau lời gọi (không so)
                    let Ty::Ref(_, inner) = t else { unreachable!() };
                    let n = gt_glir::layout::layout(&glir_ty_of(db, inner)).size as usize;
                    gargs.push(Arg::Ref(vec![Some(0); n]));
                    let r = mi.alloc_ref(zero_val(db, inner));
                    refs.push(((**inner).clone(), r.clone(), None));
                    margs.push(r);
                    continue;
                }
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
                    Ty::Float(_) => {
                        let V::Float(x) = &v else { unreachable!() };
                        gargs.push(Arg::Int(u128::from(x.to_bits())));
                    }
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
            let (g, fin, gstats) =
                gt_glir::interp::run_module_stats(module, gname, &gargs, 200_000);
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
            // mô hình runtime: số lần xoá Secret (ADR-P2-03) và số khối heap của `Owned` còn sống (rò rỉ) phải KHỚP thông dịch MIR
            if !matches!(g, GO::OutOfFuel | GO::Ub(_)) {
                assert_eq!(
                    (gstats.secret_wipes, gstats.heap_live),
                    (mi.secret_wipes(), mi.heap_live()),
                    "{}: (số lần xoá Secret, khối heap còn sống) GLIR ≠ MIR",
                    ctx()
                );
            }
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
                (GO::Value(Some(x)), Ok(V::Float(y))) => assert!(
                    u64::try_from(x).is_ok_and(|x| f64_bits_eq(x, y.to_bits())),
                    "{}: GLIR f64 {x:#x}, MIR {:#x} ({y:e})",
                    ctx(),
                    y.to_bits()
                ),
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
                    if is_cap(db, inner) {
                        continue;
                    }
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

/// Chương trình SINH có tuple LỒNG SÂU (ADR-P2-02; trần độ sâu 8 cũ của `in_subset` từng chặn 7936 instance của mẫu `e0317_*`): tham số/kết quả tuple
/// lồng 7/9/12/14 mức, dựng bằng hàm generic `nest` (instance sâu tới 14), đọc trường sâu, phép có kiểm (abort `overflow`) trên trường sâu, sao chép qua `id`.
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_deep_tuples() {
    // T_0 = u8, T_n = (T_{n-1}, u16)
    let ty = |n: usize| (0..n).fold("u8".to_string(), |t, _| format!("({t}, u16)"));
    let mut src = String::from(
        "ƒ nest<T: Copy>(x: T, k: u16) → (T, u16) ! ∅ { (x, k +% 1u16) }\n\
         ƒ id<T: Copy>(x: T) → T ! ∅ { x }\n",
    );
    for n in [7usize, 9, 12, 14] {
        let t = ty(n);
        // dựng T_n từ (a, k) qua n lần `nest`
        let mut build = String::from("t0 ≔ a; ");
        for i in 1..=n {
            build.push_str(&format!("t{i} ≔ nest(t{}, k); ", i - 1));
        }
        // trường sâu nhất (u8) và trường u16 ở mức 1
        let deep_u8 = format!("x{}", ".0".repeat(n));
        let deep_u16 = format!("x{}.1", ".0".repeat(n - 1));
        src.push_str(&format!(
            "ƒ mk{n}(a: u8, k: u16) → {t} ! ∅ {{ {build}t{n} }}\n"
        ));
        src.push_str(&format!("ƒ cp{n}(x: {t}) → {t} ! ∅ {{ id(x) }}\n"));
        src.push_str(&format!(
            "ƒ rd{n}(x: {t}) → u16 ! ∅ {{ ({deep_u8} as u16) +% {deep_u16} +% x.1 }}\n"
        ));
        src.push_str(&format!(
            "ƒ ck{n}(x: {t}, y: u8) → u8 ! ∅ {{ {deep_u8} + y }}\n"
        ));
        src.push_str(&format!(
            "ƒ rt{n}(a: u8, k: u16) → u16 ! ∅ {{ x ≔ mk{n}(a, k); y ≔ cp{n}(x); rd{n}(y) }}\n"
        ));
    }
    src.push_str("ƒ main() → i32 ! ∅ { 0 }\n");
    let mut seed = 0xDEE9_7095_1E00_0042u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, generic) =
        diff_src(&src, &mut rnd, true, &mut Default::default(), 100);
    eprintln!("GLIR_DIFF_DEEP bodies={bodies} generic={generic} runs={runs} aborts={aborts}");
    // 4 độ sâu × 5 hàm + `main` + 8 instance generic (`nest`/`id`, tối đa 4 mỗi thân); mọi hàm phải hạ được và được so
    assert!(
        bodies >= 29 && generic >= 8 && runs >= 2800 && aborts >= 140,
        "oracle tuple sâu quá nhỏ: bodies={bodies} generic={generic} runs={runs} aborts={aborts}"
    );
}

/// Kiểm tập con của bước hạ phải DỪNG trên kiểu lớn theo hàm mũ và kiểu có chu trình qua tham chiếu (ngân sách nút `SUBSET_MAX_NODES`; trần độ sâu đã
/// nới 8 → 128 nên KHÔNG có ngân sách thì `▢ S30 { a: S29, b: S29 }` duyệt 2^31 nút ⇒ treo) và fail-closed (hàm nhận `&S30` không hạ), hàm khác vẫn
/// hạ. Treo quá 60 giây ⇒ đỏ.
#[test]
fn lowering_terminates_on_exponential_and_cyclic_types() {
    let mut src = String::from(
        "▢ N<'a> { v: u8, l: &'a N<'a>, r: &'a N<'a> }\n\
         ƒ f(n: &N) → u8 ! ∅ { n.v }\n\
         ƒ g(n: &N) → u8 ! ∅ { f(n.l) +% f(n.r) }\n\
         ƒ h(x: u8) → u8 ! ∅ { x +% 1u8 }\n\
         ƒ main() → i32 ! ∅ { 0 }\n\
         ▢ S0 { a: u8 }\n",
    );
    for i in 1..=30 {
        src.push_str(&format!("▢ S{i} {{ a: S{}, b: S{} }}\n", i - 1, i - 1));
    }
    src.push_str(&format!(
        "ƒ w(s: &S30) → u8 ! ∅ {{ s.b{} }}\n",
        ".a".repeat(30)
    ));
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let res = check(
            &[SourceInput {
                name: "main.gt".to_string(),
                text: src,
            }],
            &Options::default(),
        )
        .expect("check");
        let art = res.open_gate().expect("cổng");
        let mir = glir_input_mir(art);
        let lo = gt_glirgen::lower_module(&art.types, &mir);
        let keys: Vec<String> = lo
            .funcs
            .values()
            .filter_map(|i| mir.bodies.iter().find(|b| b.node == i.node))
            .map(|b| b.key.clone())
            .collect();
        let _ = tx.send(keys);
    });
    let keys = rx
        .recv_timeout(std::time::Duration::from_secs(60))
        .expect("hạ GLIR không dừng trên kiểu lớn theo hàm mũ / có chu trình qua tham chiếu");
    assert!(keys.iter().any(|k| k == "main::h"), "{keys:?}");
    assert!(
        !keys.iter().any(|k| k == "main::f" || k == "main::w"),
        "{keys:?}"
    );
}

/// Chương trình SINH dùng `Vec<T>` (intrinsic không thân MIR; hạ như `Intr::Vec*` của codegen: `{arena, null, 0, 0}`, tăng trưởng gấp đôi tối thiểu 4 qua
/// `gritrt_arena_alloc` + sao chép từng phần tử, chốt tràn `overflow`): số/struct có đệm/enum/`()`, tăng trưởng qua nhiều lần cấp, `truncate` rồi `push`, `reserve`,
/// `as_mut_slice` ghi, hai Vec chung Arena, Vec rỗng (`{null, 0}` + lát con rỗng), `&!Vec` qua hàm, kiểm biên và tràn sau khi đọc.
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_vecs() {
    let src = "\
▢ P { a: u8, b: i32 }
⊐ Copy for P { }
ƒ v1(x: u32, y: u32) → u32 ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); v.push(x); v.push(y); s ≔ v.as_slice(); s[0usize] +% s[1usize] }
ƒ v_len(n: u8) → usize ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); ⟳ i in 0u8..n { v.push(i); } v.len() }
ƒ v_sum(n: u8) → u32 ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); ⟳ i in 0u8..n { v.push((i as u32) *% 3u32); } s ≔ v.as_slice(); μt ≔ 0u32; ⟳ j in 0usize..s.len() { t ← t +% s[j]; } t }
ƒ v_get(n: u8, i: usize) → u16 ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); ⟳ k in 0u8..n { v.push((k as u16) +% 7u16); } s ≔ v.as_slice(); s[i] }
ƒ v_trunc(n: u8, k: usize) → usize ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); ⟳ i in 0u8..n { v.push(i as u64); } v.truncate(k); v.len() }
ƒ v_trunc_get(n: u8, k: usize, i: usize) → u64 ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); ⟳ j in 0u8..n { v.push(j as u64); } v.truncate(k); v.push(99u64); s ≔ v.as_slice(); s[i] }
ƒ v_rsv(n: u8, k: u8) → u32 ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); v.reserve((k % 40u8) as usize); ⟳ i in 0u8..n { v.push(i as u32); } s ≔ v.as_slice(); μt ≔ 0u32; ⟳ j in 0usize..s.len() { t ← t +% s[j]; } t +% (v.len() as u32) }
ƒ v_rsv2(n: u8, k: u8) → u32 ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); ⟳ i in 0u8..(n % 20u8) { v.push(i as u32); } v.reserve((k % 40u8) as usize); v.push(7u32); s ≔ v.as_slice(); μt ≔ 0u32; ⟳ j in 0usize..s.len() { t ← t +% s[j]; } t }
ƒ v_st(n: u8, b: i32) → i32 ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); ⟳ i in 0u8..n { v.push(P { a: i, b: b +% (i as i32) }); } s ≔ v.as_slice(); μt ≔ 0i32; ⟳ j in 0usize..s.len() { t ← t +% s[j].b +% (s[j].a as i32); } t }
ƒ v_en(n: u8, i: usize) → u16 ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); ⟳ k in 0u8..n { ⎇ k % 2u8 == 0u8 { v.push(Some(k as u16)); } ⇝ { v.push(None); } } s ≔ v.as_slice(); ⌥ s[i] { Some(x) ↦ x, None ↦ 500u16 } }
ƒ v_mut(n: u8, i: usize, x: u32) → u32 ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); ⟳ k in 0u8..n { v.push(k as u32); } m ≔ v.as_mut_slice(); m[i] ← x; s ≔ v.as_slice(); s[i] +% (v.len() as u32) }
ƒ v_unit(n: u8) → usize ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); ⟳ i in 0u8..n { v.push(()); } v.len() }
ƒ v_two(n: u8) → u32 ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); μw ≔ Vec::new(&a); ⟳ i in 0u8..n { v.push(i as u32); w.push((i as u32) +% 100u32); } s ≔ v.as_slice(); t ≔ w.as_slice(); μr ≔ 0u32; ⟳ j in 0usize..s.len() { r ← r +% s[j] +% t[j]; } r }
ƒ v_ck(x: u8, y: u8) → u8 ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); v.push(x); s ≔ v.as_slice(); s[0usize] + y }
ƒ v_empty() → usize ! {mem} { μa ≔ Arena::new(); v: Vec<u8> ≔ Vec::new(&a); s ≔ v.as_slice(); t ≔ &s[0usize..0usize]; s.len() +% t.len() }
ƒ fill(v: &!Vec<u16>, n: u8) ! {mem} { ⟳ i in 0u8..n { v.push(i as u16); } }
ƒ v_via(n: u8) → usize ! {mem} { μa ≔ Arena::new(); μv ≔ Vec::new(&a); fill(&!v, n); fill(&!v, 3u8); v.len() }
ƒ main() → i32 ! ∅ { 0 }
";
    let mut seed = 0x7EC7_0000_5EED_0001u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, _) = diff_src(src, &mut rnd, true, &mut Default::default(), 120);
    eprintln!("GLIR_DIFF_VEC bodies={bodies} runs={runs} aborts={aborts}");
    // 16 hàm dùng Vec + `main` (`fill` nhận `&!Vec` nên chỉ chạy qua `v_via`)
    assert!(
        bodies >= 17 && runs >= 1800 && aborts >= 350,
        "oracle Vec quá nhỏ: bodies={bodies} runs={runs} aborts={aborts}"
    );
}

/// Chương trình SINH dùng `Text`/`str::slice`/`str::as_bytes` (intrinsic không thân MIR; hạ như `Intr::Text*`/`StrSlice` của codegen: Text = `{Vec<u8>}`,
/// `vec_extend` tăng trưởng tối thiểu 16, `push_char` mã hoá UTF-8 1–4 byte bằng `select`, `push_utf8`/`from_utf8` qua `gritrt_utf8_valid` (mô hình runtime
/// của GLIR interp), `slice` 4 chốt `slice_range` gồm BIÊN KÝ TỰ): chuỗi tham số nhiều byte, kiểm biên, tràn `u8` khi cộng.
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_texts() {
    let src = "\
ƒ t_new_len() → usize ! {mem} { a ≔ Arena::new(); t ≔ Text::new(&a); t.len() }
ƒ t_push(s: &str) → usize ! {mem} { a ≔ Arena::new(); μt ≔ Text::new(&a); t.push_str(s); t.push_str(s); t.len() }
ƒ t_byte(s: &str, i: usize) → u8 ! {mem} { a ≔ Arena::new(); μt ≔ Text::new(&a); t.push_str(s); t.push_str(\"xy\"); b ≔ t.as_bytes(); b[i] }
ƒ t_char(k: u8, i: usize) → u8 ! {mem} { a ≔ Arena::new(); μt ≔ Text::new(&a); ⌥ k % 5u8 { 0u8 ↦ t.push_char('a'), 1u8 ↦ t.push_char('é'), 2u8 ↦ t.push_char('日'), 3u8 ↦ t.push_char('😀'), _ ↦ t.push_char((k % 128u8) as char) } t.push_char('ß'); b ≔ t.as_bytes(); b[i] +% (t.len() as u8) }
ƒ t_cnum(k: u8) → usize ! {mem} { a ≔ Arena::new(); μt ≔ Text::new(&a); t.push_char(k as char); t.len() }
ƒ t_slice(s: &str, x: usize, y: usize) → usize ! ∅ { u ≔ s.slice(x, y); u.len() }
ƒ t_slice_byte(s: &str, x: usize, y: usize, i: usize) → u8 ! ∅ { u ≔ s.slice(x, y); b ≔ u.as_bytes(); b[i] }
ƒ t_utf8(x: u8, y: u8) → usize ! {mem} { a ≔ Arena::new(); μt ≔ Text::new(&a); t.push_str(\"q\"); arr ≔ [x, y]; ok ≔ t.push_utf8(&arr); ⎇ ok { t.len() } ⇝ { 999usize +% t.len() } }
ƒ t_from(x: u8, y: u8, z: u8) → usize ! {mem} { a ≔ Arena::new(); arr ≔ [x, y, z]; ⌥ Text::from_utf8(&a, &arr) { Some(t) ↦ t.len() +% (t.as_bytes()[0usize] as usize), None ↦ 77usize } }
ƒ t_as_str(s: &str, x: usize) → usize ! {mem} { a ≔ Arena::new(); μt ≔ Text::new(&a); t.push_str(s); v ≔ t.as_str(); w ≔ v.slice(x, v.len()); w.len() }
ƒ t_grow(n: u8, i: usize) → u8 ! {mem} { a ≔ Arena::new(); μt ≔ Text::new(&a); ⟳ k in 0u8..n { t.push_str(\"abc\"); t.push_char('é'); } b ≔ t.as_bytes(); b[i] }
ƒ t_lit() → usize ! {mem} { a ≔ Arena::new(); μt ≔ Text::new(&a); t.push_str(\"héllo\"); t.push_char('ß'); t.len() }
ƒ t_bytes_of(s: &str, i: usize) → u8 ! ∅ { b ≔ s.as_bytes(); b[i] }
ƒ main() → i32 ! ∅ { 0 }
";
    let mut seed = 0x7E47_0000_5EED_0002u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, _) = diff_src(src, &mut rnd, true, &mut Default::default(), 150);
    eprintln!("GLIR_DIFF_TEXT bodies={bodies} runs={runs} aborts={aborts}");
    // 13 hàm + `main`
    assert!(
        bodies >= 14 && runs >= 1200 && aborts >= 600,
        "oracle Text quá nhỏ: bodies={bodies} runs={runs} aborts={aborts}"
    );
}

/// Chương trình SINH dùng `Owned<T>` + drop glue (hạ như `Intr::Owned*`/`drop_glue_at` của codegen: `gritrt_heap_alloc/free(size, align)`, glue `Free`
/// sau glue của `T`, `drop` người dùng (`Call`), `Fields`/`Variants`/`Array`): `into_inner` trên một nhánh (cờ drop), `Owned<Owned<T>>`, `Option<Owned>`,
/// mảng `Owned`, `drop` người dùng ghi qua `&Cell` (thứ tự/hiệu ứng glue quan sát được), cấp trong vòng lặp. Oracle so cả số khối heap còn sống (rò rỉ /
/// trả hai lần ⇒ `Ub` của mô hình GLIR) với `heap_live` của thông dịch MIR; tham số `&Cap<Alloc>` là giá trị zero.
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_owned() {
    let src = "\
▢ P { a: u8, b: i32 }
⊔ E { A, B(u16) }
⊔ W { X(Owned<u8>), Y(u16, Owned<u32>), Z }
▢ D { n: u8 }
⊐ Drop for D { ƒ drop(&!self) ! ∅ { } }
▢ H<'a> { c: &'a Cell<u32>, k: u32 }
⊐ <'a> Drop for H<'a> { ƒ drop(&!self) ! ∅ { self.c.set(self.c.get() +% self.k); } }
ƒ o1(al: &Cap<Alloc>, x: u64) → u64 ! {mem} { o ≔ Owned::new(al, x); *o.get() }
ƒ o2(al: &Cap<Alloc>, x: u32, y: u32) → u32 ! {mem} { μp ≔ Owned::new(al, x); w ≔ p.get_mut(); *w ← *w +% y; p.into_inner() }
ƒ o3(al: &Cap<Alloc>, a: u8, b: i32) → i32 ! {mem} { o ≔ Owned::new(al, P { a: a, b: b }); q ≔ o.get(); q.b +% (q.a as i32) }
ƒ o4(al: &Cap<Alloc>, x: u16, k: bool) → u16 ! {mem} { e ≔ ⎇ k { E::B(x) } ⇝ { E::A }; o ≔ Some(Owned::new(al, e)); ⌥ o { Some(p) ↦ ⌥ *p.get() { E::B(v) ↦ v, E::A ↦ 3u16 }, None ↦ 0u16 } }
ƒ o5(al: &Cap<Alloc>, x: u8, y: u8) → u8 ! {mem} { o ≔ Owned::new(al, x); *o.get() + y }
ƒ o6(al: &Cap<Alloc>, x: u32) → u32 ! {mem} { o ≔ Owned::new(al, Owned::new(al, x)); *o.get().get() }
ƒ o7(al: &Cap<Alloc>, x: u32, k: bool) → u32 ! {mem} { o ≔ Owned::new(al, x); ⎇ k { o.into_inner() } ⇝ { 1u32 } }
ƒ o8(al: &Cap<Alloc>, x: u8) → u8 ! {mem} { o ≔ Owned::new(al, D { n: x }); o.get().n }
ƒ o9in(al: &Cap<Alloc>, c: &Cell<u32>, k: u32) ! {mem} { o ≔ Owned::new(al, H { c: c, k: k }); h ≔ H { c: c, k: 1u32 }; }
ƒ o9(al: &Cap<Alloc>, x: u32, k: u32) → u32 ! {mem} { c ≔ Cell::new(x); o9in(al, &c, k); c.get() }
ƒ o10(al: &Cap<Alloc>, x: u8, n: u8) → u8 ! {mem} { μs ≔ 0u8; ⟳ i in 0u8..n { o ≔ Owned::new(al, x +% i); s ← s +% *o.get(); } s }
ƒ o11(al: &Cap<Alloc>, x: u16) → u16 ! {mem} { a ≔ [Owned::new(al, x), Owned::new(al, 2u16)]; *a[1usize].get() +% *a[0usize].get() }
ƒ o12(al: &Cap<Alloc>, x: u8) → u8 ! {mem} { o ≔ Owned::new(al, ()); x }
ƒ o99(al: &Cap<Alloc>, x: u8) → u8 ! {mem} { o ≔ Owned::new(al, ()); o.into_inner(); x }
ƒ o13(al: &Cap<Alloc>, x: u8, k: u8) → u8 ! {mem} { w ≔ ⌥ k % 3u8 { 0u8 ↦ W::X(Owned::new(al, x)), 1u8 ↦ W::Y(7u16, Owned::new(al, 9u32)), _ ↦ W::Z }; o ≔ ⎇ k > 100u8 { Some(Owned::new(al, x)) } ⇝ { None }; x +% k }
ƒ main() → i32 ! ∅ { 0 }
";
    let mut seed = 0x0E4E_D000_5EED_0003u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, _) = diff_src(src, &mut rnd, true, &mut Default::default(), 150);
    eprintln!("GLIR_DIFF_OWNED bodies={bodies} runs={runs} aborts={aborts}");
    // 14 hàm `oN` + `D::drop` + `main` (`o9in` nhận `&Cell` nên chỉ chạy qua `o9`; `o99`: `Owned<()>` + `into_inner`, tên xếp cuối để không xáo dòng ngẫu nhiên của các hàm cũ)
    assert!(
        bodies >= 16 && runs >= 1900 && aborts >= 50,
        "oracle Owned quá nhỏ: bodies={bodies} runs={runs} aborts={aborts}"
    );
}

/// Đột biến tự động (`tools/mut.py`, Vec 2026-11-26) sống: `runtime_call` trả "đã hạ" cho MỌI callee không thân MIR (kể cả nhóm DỪNG chờ kiến trúc:
/// I/O, `Atomic`, `spawn`) ⇒ lời gọi bị BỎ IM LẶNG. Ngưỡng phủ chỉ là cận dưới nên không bắt được; test này đòi các hàm đó KHÔNG hạ (fail-closed).
#[test]
fn runtime_calls_without_a_model_are_never_silently_dropped() {
    let src = "\
ƒ clock(c: &Cap<Clock>) → u64 ! {io} { c.now() }
ƒ wk(x: u32) ! ∅ { }
ƒ sp(x: u32) ! ∅ { spawn(wk, x); }
ƒ out(o: &Cap<Out>, x: u64) ! {io} { o.write_u64(x); }
ƒ ok(x: u8) → u8 ! ∅ { x +% 1u8 }
ƒ main() → i32 ! ∅ { 0 }
";
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
    let keys: Vec<String> = lo
        .funcs
        .values()
        .filter_map(|i| mir.bodies.iter().find(|b| b.node == i.node))
        .map(|b| b.key.clone())
        .collect();
    assert!(keys.iter().any(|k| k == "main::ok"), "{keys:?}");
    // (`Atomic` có mô hình tuần tự từ 2026-11-26 — thay bằng `spawn`, vẫn DỪNG)
    for k in ["main::clock", "main::sp", "main::out"] {
        assert!(!keys.iter().any(|x| x == k), "{k} không được hạ: {keys:?}");
    }
}

/// Chương trình SINH dùng `Secret<T>` (ADR-P2-03; hạ như codegen: `Secret::new/declassify` tại chỗ, `Wipe` sau move ⇒ `gritrt_secret_wipe` khi kiểu sau
/// khi thế chứa Secret, `_0` chứa Secret bị xoá sau khi nạp ở `Return`, drop glue `Wipe` (glue của `v` rồi xoá cả Secret), `Owned::into_inner` của
/// `Owned<Secret<T>>` xoá khối heap trước khi trả): Secret qua hàm, trong enum/tuple/mảng/`Option`/`Owned`, trên một nhánh (cờ drop), trong vòng lặp.
/// Oracle so SỐ LẦN XOÁ (mô hình `gritrt_secret_wipe` của GLIR interp, đếm) với `secret_wipes()` của thông dịch MIR, cùng giá trị/abort/rò rỉ heap.
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_secrets() {
    let src = "\
▢ K { a: u32, b: u8 }
⊐ Copy for K { }
⊔ M { S(Secret<u16>), P(u16) }
ƒ keep(s: Secret<u64>) → Secret<u64> ! ∅ { s }
ƒ s1(x: u64) → u64 ! ∅ { s ≔ Secret::new(x); t ≔ keep(s); t.declassify() }
ƒ s2(x: u64) → u8 ! ∅ { s ≔ Secret::new(x); t ≔ keep(s); 1u8 }
ƒ s3(x: u32, b: u8) → u32 ! ∅ { s ≔ Secret::new(K { a: x, b: b }); k ≔ s.declassify(); k.a +% (k.b as u32) }
ƒ s4(x: u16, c: bool) → u16 ! ∅ { m ≔ ⎇ c { M::S(Secret::new(x)) } ⇝ { M::P(x) }; ⌥ m { M::S(s) ↦ s.declassify(), M::P(v) ↦ v +% 1u16 } }
ƒ s5(x: u16, c: bool) → u16 ! ∅ { m ≔ ⎇ c { M::S(Secret::new(x)) } ⇝ { M::P(x) }; x }
ƒ s6(x: u8, y: u8) → u8 ! ∅ { s ≔ Secret::new(x); v ≔ s.declassify(); v + y }
ƒ s7(x: u32) → u32 ! ∅ { o ≔ Some(Secret::new(x)); t ≔ (Secret::new(1u8), x); t.1 }
ƒ s8(x: u64, c: bool) → u64 ! ∅ { s ≔ Secret::new(x); ⎇ c { keep(s).declassify() } ⇝ { 5u64 } }
ƒ s9(x: u16) → u16 ! ∅ { a ≔ [Secret::new(x), Secret::new(2u16)]; x +% 3u16 }
ƒ s10(al: &Cap<Alloc>, x: u32) → u32 ! {mem} { o ≔ Owned::new(al, Secret::new(x)); s ≔ o.into_inner(); s.declassify() }
ƒ s11(al: &Cap<Alloc>, x: u32) → u32 ! {mem} { o ≔ Owned::new(al, Secret::new(x)); x }
ƒ mk(x: u16) → Secret<u16> ! ∅ { Secret::new(x) }
ƒ s12(x: u16, n: u8) → u16 ! ∅ { μt ≔ 0u16; ⟳ i in 0u8..n { s ≔ mk(x +% (i as u16)); t ← t +% s.declassify(); } t }
▢ B<T> { v: T }
ƒ mv<T>(x: T) → T ! ∅ { y ≔ x; y }
ƒ s13(x: u64) → u64 ! ∅ { s ≔ mv(Secret::new(x)); a ≔ mv(x); s.declassify() +% a }
ƒ s14(x: u32) → u32 ! ∅ { b ≔ B { v: x }; c ≔ mv(b); c.v }
ƒ s15(x: u8) → u8 ! ∅ { s ≔ Secret::new(()); t ≔ s.declassify(); x }
ƒ s16(al: &Cap<Alloc>, x: u32, k: bool) → u32 ! {mem} { s ≔ Secret::new(Owned::new(al, x)); ⎇ k { x } ⇝ { x +% 1u32 } }
ƒ main() → i32 ! ∅ { 0 }
";
    let mut seed = 0x5EC7_E700_5EED_0004u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, _) = diff_src(src, &mut rnd, true, &mut Default::default(), 150);
    eprintln!("GLIR_DIFF_SECRET bodies={bodies} runs={runs} aborts={aborts}");
    // 16 hàm `sN` + `main` (`keep`/`mk`/`mv` có Secret/T trong chữ ký nên chỉ chạy qua người gọi). Đột biến tự động 2026-11-26
    // (`docs/MUT_GLIR_TOS.jsonl`) sống ⇒ thêm `s13`/`s14` (instance generic: chỉ instance chứa Secret được xoá sau move), `s15` (`Secret<()>`),
    // `s16` (`Secret<Owned<T>>`: glue của `v` chạy TRƯỚC khi xoá — không chạy ⇒ rò khối heap)
    assert!(
        bodies >= 17 && runs >= 1650 && aborts >= 50,
        "oracle Secret quá nhỏ: bodies={bodies} runs={runs} aborts={aborts}"
    );
}

/// Chương trình SINH dùng `f64` (ADR-P2-02 quyết định 2026-11-26: `fbin`/`funop`/`fcmp`, `sitofp`/`uitofp`/`fptosi`/`bitcast`, hằng theo mẫu bit): số học,
/// so sánh có thứ tự với NaN/±0/±∞, `sqrt`, `is_nan`, `to_bits` (của ĐẦU VÀO hoặc của kết quả đã chặn NaN — mẫu bit NaN do phép tính sinh ra không được
/// bảo đảm), `f64_from_bits`, `to_i64` ở biên ±2^63, ép số nguyên → f64 (kể cả `CastLossy` của i64/u64 ngoài ±2^53 ⇒ abort), f64 trong struct/mảng/
/// `Option`/tham chiếu, vòng lặp tích luỹ. So kết quả f64 theo bit, mọi NaN coi là bằng nhau (`f64_bits_eq`).
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_floats() {
    let src = "\
▢ Q { a: f64, b: u8 }
⊐ Copy for Q { }
ƒ f_add(x: f64, y: f64) → f64 ! ∅ { x + y }
ƒ f_ar(x: f64, y: f64) → f64 ! ∅ { (x * y - x) / (y + 1.5f64) }
ƒ f_neg(x: f64, y: f64) → f64 ! ∅ { -(x - y) }
ƒ f_cmp(x: f64, y: f64) → u8 ! ∅ { μr ≔ 0u8; ⎇ x < y { r ← r +% 1u8; } ⎇ x <= y { r ← r +% 2u8; } ⎇ x > y { r ← r +% 4u8; } ⎇ x >= y { r ← r +% 8u8; } r }
ƒ f_nan(x: f64, y: f64) → bool ! ∅ { (x / y).is_nan() }
ƒ f_sqrt(x: f64) → f64 ! ∅ { x.sqrt() }
ƒ f_toi(x: f64) → i64 ! ∅ { ⌥ x.to_i64() { Some(v) ↦ v, None ↦ -1i64 } }
ƒ f_toi2(x: f64, y: f64) → i64 ! ∅ { ⌥ (x * y).to_i64() { Some(v) ↦ v +% 1i64, None ↦ 77i64 } }
ƒ f_bits(x: f64) → u64 ! ∅ { x.to_bits() }
ƒ f_bits2(x: f64, y: f64) → u64 ! ∅ { z ≔ x * y; ⎇ z.is_nan() { 7u64 } ⇝ { z.to_bits() } }
ƒ f_from(b: u64, y: f64) → f64 ! ∅ { core::f64_from_bits(b) + y }
ƒ f_cast(a: i32, b: u32, c: u8) → f64 ! ∅ { (a as f64) + (b as f64) * (c as f64) }
ƒ f_cast64(a: i64, b: u64) → f64 ! ∅ { (a as f64) - (b as f64) }
ƒ f_cast_sz(a: usize, b: i16) → f64 ! ∅ { (a as f64) * (b as f64) }
ƒ f_struct(x: f64, k: u8) → Q ! ∅ { Q { a: x * 0.5f64, b: k } }
ƒ f_sarg(q: Q) → f64 ! ∅ { q.a + (q.b as f64) }
ƒ f_loop(n: u8, x: f64) → f64 ! ∅ { μs ≔ 0.0f64; ⟳ i in 0u8..n { s ← s + x * (i as f64); } s }
ƒ f_arr(x: f64, i: usize) → f64 ! ∅ { a ≔ [x, -x, 0.5f64]; a[i] }
ƒ f_ref(p: &f64, y: f64) → f64 ! ∅ { *p * y }
ƒ f_mut(p: &!f64, y: f64) ! ∅ { *p ← *p - y; }
ƒ f_opt(x: f64, k: bool) → Option<f64> ! ∅ { ⎇ k { Some(x / 3.0f64) } ⇝ { None } }
ƒ f_lit() → f64 ! ∅ { 0.1f64 + 0.2f64 + 1_000.000_1f64 }
ƒ f_tup(x: f64, y: f64) → (f64, bool) ! ∅ { (x - y, x > y) }
ƒ f_call(x: f64, y: f64) → f64 ! ∅ { f_add(x, y) * f_ar(y, x) }
ƒ f_sl(s: &[f64], i: usize) → f64 ! ∅ { s[i] + 1.0f64 }
ƒ f_c53(d: i8) → f64 ! ∅ { (9_007_199_254_740_992i64 +% (d as i64)) as f64 }
ƒ f_c53n(d: i8) → f64 ! ∅ { (-9_007_199_254_740_992i64 +% (d as i64)) as f64 }
ƒ f_c53u(d: i8) → f64 ! ∅ { (9_007_199_254_740_992u64 +% (d as u64)) as f64 }
ƒ main() → i32 ! ∅ { 0 }
";
    let mut seed = 0xF10A_7000_5EED_0005u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, _) = diff_src(src, &mut rnd, true, &mut Default::default(), 150);
    eprintln!("GLIR_DIFF_F64 bodies={bodies} runs={runs} aborts={aborts}");
    // 28 hàm `f_*` + `main` (`f_c53*`: biên ±2^53 của `CastLossy` i64/u64 → f64)
    assert!(
        bodies >= 29 && runs >= 4000 && aborts >= 300,
        "oracle f64 quá nhỏ: bodies={bodies} runs={runs} aborts={aborts}"
    );
}

/// Chương trình SINH dùng `Atomic<T>` và `Mutex<T, L>::with` (ADR-P2-02 quyết định 2026-11-26: `atomicload/atomicstore/atomicrmw/cmpxchg` có thứ tự bộ
/// nhớ GHI LẠI, thông dịch TUẦN TỰ — cùng mô hình một luồng với thông dịch MIR; `with` ⇒ `gritrt_mutex_lock` + gọi TRỰC TIẾP `f` + `unlock`, mô hình ô
/// khoá 0/1): mọi thứ tự hợp lệ, `fetch_add/sub` bao quanh ở biên, `compare_exchange` thành công/thất bại (`Result`), Atomic qua `&`, `with` lồng trong
/// vòng lặp và trả giá trị, `f` abort khi khoá đang giữ.
#[test]
fn glir_interpreter_agrees_with_the_mir_interpreter_on_generated_atomics() {
    let src = "\
#level(\"10\")
▢ Lo { }
ƒ a_ld(x: u32) → u32 ! ∅ { a ≔ Atomic::new(x); a.load(AtomicOrdering::SeqCst) }
ƒ a_add(x: u32, y: u32) → u32 ! ∅ { a ≔ Atomic::new(x); o ≔ a.fetch_add(y, AtomicOrdering::Relaxed); o +% a.load(AtomicOrdering::Acquire) }
ƒ a_sub(x: i32, y: i32) → i32 ! ∅ { a ≔ Atomic::new(x); o ≔ a.fetch_sub(y, AtomicOrdering::AcqRel); a.load(AtomicOrdering::Relaxed) -% o }
ƒ a_swap(x: u64, y: u64) → u64 ! ∅ { a ≔ Atomic::new(x); o ≔ a.swap(y, AtomicOrdering::Release); a.store(o +% 1u64, AtomicOrdering::Release); a.load(AtomicOrdering::SeqCst) }
ƒ a_cas(x: u32, c: u32, n: u32) → u32 ! ∅ { a ≔ Atomic::new(x); ⌥ a.compare_exchange(c, n, AtomicOrdering::SeqCst, AtomicOrdering::Relaxed) { Ok(v) ↦ v +% a.load(AtomicOrdering::SeqCst), Err(v) ↦ 1000u32 +% v } }
ƒ a_cas2(x: i64, k: bool) → i64 ! ∅ { a ≔ Atomic::new(x); c ≔ ⎇ k { x } ⇝ { x +% 1i64 }; r ≔ a.compare_exchange(c, 9i64, AtomicOrdering::AcqRel, AtomicOrdering::Acquire); ⌥ r { Ok(v) ↦ a.load(AtomicOrdering::SeqCst) +% v, Err(v) ↦ v } }
ƒ bump(a: &Atomic<u32>, n: u8) ! ∅ { ⟳ i in 0u8..n { a.fetch_add(i as u32, AtomicOrdering::Relaxed); } }
ƒ a_ref(x: u32, n: u8) → u32 ! ∅ { a ≔ Atomic::new(x); bump(&a, n); bump(&a, 2u8); a.load(AtomicOrdering::SeqCst) }
ƒ a_ck(x: usize, y: usize) → usize ! ∅ { a ≔ Atomic::new(x); a.load(AtomicOrdering::SeqCst) + y }
ƒ inc(v: &!u32, n: u32) → u32 ! ∅ { *v ← *v +% n; *v }
ƒ chk(v: &!u8, n: u8) → u8 ! ∅ { *v ← *v + n; *v }
ƒ nop(v: &!u16, n: ()) ! ∅ { *v ← *v +% 1u16; }
ƒ m_one(x: u32, n: u32) → u32 ! {lock(Lo)} { m: Mutex<u32, Lo> ≔ Mutex::new(x); m.with(inc, n) }
ƒ m_loop(x: u32, k: u8) → u32 ! {lock(Lo)} { m: Mutex<u32, Lo> ≔ Mutex::new(x); μs ≔ 0u32; ⟳ i in 0u8..k { s ← s +% m.with(inc, i as u32); } s }
ƒ m_ck(x: u8, y: u8) → u8 ! {lock(Lo)} { m: Mutex<u8, Lo> ≔ Mutex::new(x); a ≔ m.with(chk, y); m.with(chk, 0u8) +% a }
ƒ m_unit(x: u16) → u16 ! {lock(Lo)} { m: Mutex<u16, Lo> ≔ Mutex::new(x); m.with(nop, ()); m.with(nop, ()); x }
ƒ main() → i32 ! ∅ { 0 }
";
    let mut seed = 0xA70A_1C00_5EED_0006u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (bodies, runs, aborts, _) = diff_src(src, &mut rnd, true, &mut Default::default(), 150);
    eprintln!("GLIR_DIFF_ATOMIC bodies={bodies} runs={runs} aborts={aborts}");
    // 12 hàm `a_*`/`m_*` + `inc`/`chk`/`nop` (tham số `&!`) + `main`; `bump` nhận `&Atomic` (ngoài `plain`) nên chỉ chạy qua `a_ref`
    assert!(
        bodies >= 16 && runs >= 1650 && aborts >= 100,
        "oracle Atomic/Mutex quá nhỏ: bodies={bodies} runs={runs} aborts={aborts}"
    );
}
