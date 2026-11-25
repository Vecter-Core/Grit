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
    let (mut lowered, mut total) = (0usize, 0usize);
    let mut why: BTreeMap<String, usize> = BTreeMap::new();
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
        let fns: Vec<&gt_mir::Body> = art
            .mir
            .bodies
            .iter()
            .filter(|b| b.kind == gt_mir::BodyKind::Fn)
            .collect();
        total += fns.len();
        let (m, bad) = gt_glirgen::lower_module(&art.types.db, &art.mir.bodies);
        for (_, u) in bad {
            *why.entry(u.0).or_default() += 1;
        }
        let errs = gt_glir::verify(&m);
        assert!(
            errs.is_empty(),
            "{}: {errs:?}\n{}",
            p.display(),
            gt_glir::print(&m)
        );
        for f in &m.funcs {
            lowered += 1;
            let b = fns
                .iter()
                .find(|b| b.key.replace(['<', '>', ' ', ':'], "_") == f.name)
                .expect("thân gốc");
            let cover = gt_glirgen::check_abort_cover(b, f);
            assert!(cover.is_empty(), "{}: {cover:?}", b.key);
        }
        let text = gt_glir::print(&m);
        let back = gt_glir::parse(&text)
            .unwrap_or_else(|e| panic!("{}: in ra không đọc lại được: {e}", p.display()));
        assert_eq!(back, m, "{}: đọc(in) ≠ gốc", p.display());
    }
    let mut by: Vec<_> = why.into_iter().collect();
    by.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    eprintln!("GLIR_COVER lowered={lowered} total={total}");
    for (w, n) in by.iter().take(15) {
        eprintln!("  chưa hạ: {n:5}  {w}");
    }
    assert!(lowered >= 1, "không hạ được thân nào");
}

/// ORACLE VI SAI (ADR-P2-02 C3/C2): thân hạ được mà tham số/kết quả là số nguyên/bool/`()`/tuple/struct (lồng nhau) chạy trên GLIR interp (Rust thuần, theo BYTE) và
/// thông dịch MIR với đầu vào thiên về BIÊN; kết quả (giải mã byte theo kiểu Grit) và loại abort phải KHỚP; GLIR `Ub` (bước hạ quên chốt chặn) là LỖI; hết nhiên liệu ⇒ bỏ qua.
fn adt_fields(
    db: &gt_types::TypeDb,
    def: &gt_resolve::DefId,
    args: &[gt_types::Ty],
) -> Option<Vec<gt_types::Ty>> {
    let adt = db.adts.get(def)?;
    let gt_types::AdtKind::Struct(fs) = &adt.kind else {
        return None;
    };
    if adt.lang.is_some() {
        return None;
    }
    Some(
        fs.iter()
            .map(|f| {
                f.ty.subst(&|p| {
                    adt.generics
                        .iter()
                        .position(|g| g.id == p)
                        .and_then(|i| args.get(i).cloned())
                })
            })
            .collect(),
    )
}

fn plain(db: &gt_types::TypeDb, t: &gt_types::Ty, d: u32) -> bool {
    use gt_types::Ty;
    d <= 6
        && match t {
            Ty::Int(it) => it.bits() <= 64,
            Ty::Bool => true,
            Ty::Tuple(v) => v.iter().all(|x| plain(db, x, d + 1)),
            Ty::Adt(def, args) => {
                adt_fields(db, def, args).is_some_and(|fs| fs.iter().all(|x| plain(db, x, d + 1)))
            }
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
        Ty::Tuple(v) => V::Tuple(v.iter().map(|x| gen_val(db, x, rnd)).collect()),
        Ty::Adt(def, args) => V::Adt(
            0,
            adt_fields(db, def, args)
                .expect("plain")
                .iter()
                .map(|x| gen_val(db, x, rnd))
                .collect(),
        ),
        _ => unreachable!(),
    }
}

fn glir_ty_of(db: &gt_types::TypeDb, t: &gt_types::Ty) -> gt_glir::Ty {
    gt_glirgen::lower_ty(db, t).expect("kiểu plain hạ được")
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

fn diff_src(
    src: &str,
    rnd: &mut dyn FnMut() -> u64,
    main_only: bool,
    iters: usize,
) -> (usize, usize, usize) {
    use gt_glir::interp::{Arg, Outcome as GO};
    use gt_mir::interp::{Interp, Outcome as MO, V};
    use gt_types::Ty;
    let (mut bodies, mut runs, mut aborts) = (0usize, 0usize, 0usize);
    let Ok(res) = check(
        &[SourceInput {
            name: "main.gt".to_string(),
            text: src.to_string(),
        }],
        &Options::default(),
    ) else {
        return (0, 0, 0);
    };
    let Some(art) = res.open_gate() else {
        return (0, 0, 0);
    };
    let db = &art.types.db;
    let (module, _) = gt_glirgen::lower_module(db, &art.mir.bodies);
    for b in &art.mir.bodies {
        if b.kind != gt_mir::BodyKind::Fn
            || (main_only && !b.key.starts_with("main::"))
            || !b.key.starts_with("main::")
        {
            continue;
        }
        let gname = b.key.replace(['<', '>', ' ', ':'], "_");
        if !module.funcs.iter().any(|f| f.name == gname) {
            continue;
        }
        let ptys: Vec<Ty> = (1..=b.arg_count).map(|i| b.locals[i].ty.clone()).collect();
        let rty = &b.locals[0].ty;
        if !ptys.iter().all(|t| plain(db, t, 0)) || !plain(db, rty, 0) {
            continue;
        }
        bodies += 1;
        for _ in 0..iters {
            let margs: Vec<V> = ptys.iter().map(|t| gen_val(db, t, rnd)).collect();
            let mut gargs: Vec<Arg> = Vec::new();
            for (t, v) in ptys.iter().zip(&margs) {
                match t {
                    Ty::Tuple(ts) if ts.is_empty() => {}
                    Ty::Bool => gargs.push(Arg::Int(u128::from(matches!(v, V::Bool(true))))),
                    Ty::Int(it) => {
                        let V::Int(x, _) = v else { unreachable!() };
                        gargs.push(Arg::Int((*x as u128) & mask(it.bits())));
                    }
                    _ => {
                        let n = gt_glir::layout::layout(&glir_ty_of(db, t)).size as usize;
                        let mut buf = vec![None; n];
                        encode(db, t, v, &mut buf, 0);
                        gargs.push(Arg::Bytes(buf));
                    }
                }
            }
            let g = gt_glir::interp::run_module_args(&module, &gname, &gargs, 200_000);
            let m = Interp::new(&art.mir, 200_000)
                .with_types(&art.types)
                .call_key(&b.key, margs.clone());
            let m = match m {
                MO::Value(v) => Ok(v),
                MO::Abort(k) => Err(k),
                _ => continue,
            };
            runs += 1;
            let ctx = || {
                format!(
                    "{} {gargs:?}\n{}",
                    b.key,
                    gt_glir::print(&gt_glir::Module {
                        decls: vec![],
                        funcs: module
                            .funcs
                            .iter()
                            .filter(|f| f.name == gname)
                            .cloned()
                            .collect()
                    })
                )
            };
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
        }
    }
    (bodies, runs, aborts)
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
    let (mut bodies, mut runs, mut aborts) = (0usize, 0usize, 0usize);
    for p in files {
        let src = std::fs::read_to_string(&p).expect("đọc");
        let (b, r, a) = diff_src(&src, &mut rnd, true, 40);
        bodies += b;
        runs += r;
        aborts += a;
    }
    eprintln!("GLIR_DIFF bodies={bodies} runs={runs} aborts={aborts}");
    assert!(
        bodies >= 5 && runs >= 300 && aborts >= 5,
        "oracle quá nhỏ: bodies={bodies} runs={runs} aborts={aborts}"
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
    let (bodies, runs, aborts) = diff_src(&src, &mut rnd, true, 120);
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
    let b = art
        .mir
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
