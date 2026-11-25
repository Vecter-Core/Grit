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
        for b in &art.mir.bodies {
            if b.kind != gt_mir::BodyKind::Fn {
                continue;
            }
            total += 1;
            match gt_glirgen::lower_body(&art.types.db, b) {
                Ok(f) => {
                    lowered += 1;
                    let m = gt_glir::Module {
                        decls: Vec::new(),
                        funcs: vec![f],
                    };
                    let errs = gt_glir::verify(&m);
                    assert!(
                        errs.is_empty(),
                        "{} :: {}: {errs:?}\n{}",
                        p.display(),
                        b.key,
                        gt_glir::print(&m)
                    );
                    let text = gt_glir::print(&m);
                    let back = gt_glir::parse(&text).unwrap_or_else(|e| {
                        panic!("{}: in ra không đọc lại được: {e}\n{text}", b.key)
                    });
                    assert_eq!(back, m, "{}: đọc(in) ≠ gốc", b.key);
                }
                Err(u) => *why.entry(u.0).or_default() += 1,
            }
        }
    }
    let mut by: Vec<_> = why.into_iter().collect();
    by.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    eprintln!("GLIR_COVER lowered={lowered} total={total}");
    for (w, n) in by.iter().take(15) {
        eprintln!("  chưa hạ: {n:5}  {w}");
    }
    assert!(lowered >= 1, "không hạ được thân nào");
}

/// ORACLE VI SAI (ADR-P2-02 C3): thân hạ được (tham số số nguyên/bool, kết quả số nguyên/bool/`()`) chạy trên GLIR interp (Rust thuần, độc lập) và thông dịch MIR
/// với đầu vào thiên về BIÊN; kết quả và loại abort phải KHỚP; GLIR `Ub` (bước hạ quên chốt chặn) là LỖI; hết nhiên liệu ⇒ bỏ qua.
fn diff_src(src: &str, rnd: &mut dyn FnMut() -> u64, main_only: bool) -> (usize, usize, usize) {
    use gt_mir::interp::{Interp, Outcome as MO, V};
    use gt_types::Ty;
    let (mut bodies, mut runs, mut aborts) = (0usize, 0usize, 0usize);
    {
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
        for b in &art.mir.bodies {
            if b.kind != gt_mir::BodyKind::Fn
                || (main_only && !b.key.starts_with("main::"))
                || !b.key.starts_with("main::")
            {
                continue;
            }
            let Ok(f) = gt_glirgen::lower_body(&art.types.db, b) else {
                continue;
            };
            let ptys: Vec<Ty> = (1..=b.arg_count).map(|i| b.locals[i].ty.clone()).collect();
            let simple = |t: &Ty| matches!(t, Ty::Int(_) | Ty::Bool);
            let rty = &b.locals[0].ty;
            if !ptys.iter().all(simple)
                || !(simple(rty) || matches!(rty, Ty::Tuple(v) if v.is_empty()))
            {
                continue;
            }
            bodies += 1;
            for _ in 0..120 {
                let mut margs = Vec::new();
                let mut gargs = Vec::new();
                for t in &ptys {
                    let (bits, w, signed) = match t {
                        Ty::Bool => (u128::from(rnd() % 2 == 1), 1, false),
                        Ty::Int(it) => {
                            let w = it.bits();
                            let m = if w >= 128 {
                                u128::MAX
                            } else {
                                (1u128 << w) - 1
                            };
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
                            (
                                pool[(rnd() % pool.len() as u64) as usize] & m,
                                w,
                                it.signed(),
                            )
                        }
                        _ => unreachable!(),
                    };
                    gargs.push(bits);
                    margs.push(match t {
                        Ty::Bool => V::Bool(bits == 1),
                        Ty::Int(it) => {
                            let v = if signed && (bits >> (w - 1)) & 1 == 1 {
                                (bits as i128) - (1i128 << w)
                            } else {
                                bits as i128
                            };
                            V::Int(v, *it)
                        }
                        _ => unreachable!(),
                    });
                }
                let g = gt_glir::interp::run(&f, &gargs, 200_000);
                let m = Interp::new(&art.mir, 200_000)
                    .with_types(&art.types)
                    .call_key(&b.key, margs.clone());
                let norm = |o: MO| -> Option<Result<Option<u128>, String>> {
                    Some(match o {
                        MO::Value(V::Unit) => Ok(None),
                        MO::Value(V::Bool(x)) => Ok(Some(u128::from(x))),
                        MO::Value(V::Int(v, it)) => Ok(Some(
                            (v as u128)
                                & if it.bits() >= 128 {
                                    u128::MAX
                                } else {
                                    (1u128 << it.bits()) - 1
                                },
                        )),
                        MO::Abort(k) => Err(k),
                        _ => return None,
                    })
                };
                let Some(want) = norm(m) else { continue };
                runs += 1;
                let got = match g {
                    gt_glir::interp::Outcome::Value(v) => Ok(v),
                    gt_glir::interp::Outcome::Abort(k) => Err(k),
                    gt_glir::interp::Outcome::OutOfFuel => continue,
                    gt_glir::interp::Outcome::Ub(u) => panic!(
                        "{} {:?}: GLIR chạm hành vi không xác định ({u}); MIR cho {want:?}\n{}",
                        b.key,
                        gargs,
                        gt_glir::print(&gt_glir::Module {
                            decls: vec![],
                            funcs: vec![f.clone()]
                        })
                    ),
                };
                if want.is_err() {
                    aborts += 1;
                }
                assert_eq!(
                    got,
                    want,
                    "{} {:?}\n{}",
                    b.key,
                    gargs,
                    gt_glir::print(&gt_glir::Module {
                        decls: vec![],
                        funcs: vec![f]
                    })
                );
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
        let (b, r, a) = diff_src(&src, &mut rnd, true);
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
    let (bodies, runs, aborts) = diff_src(&src, &mut rnd, true);
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
