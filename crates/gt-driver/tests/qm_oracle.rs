//! Toán tử `e?` (edition 2, ADR-P2-09) — oracle động. Chương trình ngẫu nhiên: hàm `f` là dãy câu lệnh gồm bước có thể lỗi dùng `?` (đứng riêng, làm toán hạng của biểu thức,
//! trong vòng lặp `⟳` có biến thể, trong khối lồng) xen kẽ khai báo giá trị `Drop` (`D`, mỗi `drop` ghi `log ← log *% 31 +% id`). Một MÔ HÌNH Rust độc lập (không chung mã với compiler)
//! tính giá trị và THỨ TỰ DROP: thoát sớm tại `?` lỗi phải drop mọi `D` đang sống, từ khối trong ra khối ngoài, ngược thứ tự khai báo, ĐÚNG MỘT LẦN (không rò, không double-drop).
//! Nửa số chương trình dùng `Option` thay `Result`. Thông dịch MIR VÀ exe LLVM (dev, release, release-o2) phải khớp mô hình.
use gt_driver::{Options, RunEnd, check};
use gt_resolve::SourceInput;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// bước: lỗi (mã `code`) khi `x % m == r`, ngược lại `Ok(x +% c)`
#[derive(Clone, Copy)]
struct Step {
    m: u32,
    r: u32,
    c: u32,
    code: u8,
}

#[derive(Clone)]
enum St {
    /// `v ← step(v)?;`
    Plain(usize),
    /// `v ← v +% step(v +% 1)?;` — `?` làm toán hạng của biểu thức
    Expr(usize),
    /// `dK ≔ mk(log, id);`
    Decl(u64),
    /// `μi ≔ 0; ⟳ i < n ↓ n - i { v ← step(v)?; i ← i + 1; }`
    Loop(usize, u32),
    /// `{ … };`
    Block(Vec<St>),
}

fn gen_block(r: &mut Rng, steps: usize, depth: u32, ids: &mut u64) -> Vec<St> {
    let mut v = Vec::new();
    for _ in 0..1 + r.below(4) {
        match r.below(7) {
            0 | 1 => v.push(St::Plain(r.below(steps as u64) as usize)),
            2 => v.push(St::Expr(r.below(steps as u64) as usize)),
            3 | 4 => {
                *ids += 1;
                v.push(St::Decl(*ids));
            }
            5 => v.push(St::Loop(
                r.below(steps as u64) as usize,
                1 + r.below(3) as u32,
            )),
            _ if depth < 2 => v.push(St::Block(gen_block(r, steps, depth + 1, ids))),
            _ => {}
        }
    }
    v
}

/// kết quả mô hình của một lần gọi `f(x)`: (`Some(v)` hoặc lỗi `Err(mã)`), và đã thoát sớm khi còn D sống?
struct Exec {
    log: u64,
    early_with_live: bool,
}

fn model_block(
    sts: &[St],
    steps: &[Step],
    v: &mut u32,
    scopes: &mut Vec<Vec<u64>>,
    ex: &mut Exec,
) -> Result<(), u8> {
    scopes.push(Vec::new());
    for s in sts {
        let r = match s {
            St::Plain(k) => step(&steps[*k], *v).map(|n| *v = n),
            St::Expr(k) => step(&steps[*k], v.wrapping_add(1)).map(|n| *v = v.wrapping_add(n)),
            St::Decl(id) => {
                scopes.last_mut().expect("phạm vi").push(*id);
                Ok(())
            }
            St::Loop(k, n) => {
                let mut res = Ok(());
                for _ in 0..*n {
                    match step(&steps[*k], *v) {
                        Ok(nv) => *v = nv,
                        Err(e) => {
                            res = Err(e);
                            break;
                        }
                    }
                }
                res
            }
            St::Block(b) => model_block(b, steps, v, scopes, ex),
        };
        if let Err(e) = r {
            // thoát sớm: drop khối hiện tại (ngược thứ tự khai báo); khối cha drop ở lượt trả về của nó
            if scopes.iter().any(|s| !s.is_empty()) {
                ex.early_with_live = true;
            }
            drop_scope(scopes, ex);
            return Err(e);
        }
    }
    drop_scope(scopes, ex);
    Ok(())
}

fn drop_scope(scopes: &mut Vec<Vec<u64>>, ex: &mut Exec) {
    for id in scopes.pop().unwrap_or_default().into_iter().rev() {
        ex.log = ex.log.wrapping_mul(31).wrapping_add(id);
    }
}

fn step(s: &Step, x: u32) -> Result<u32, u8> {
    if x % s.m == s.r {
        Err(s.code)
    } else {
        Ok(x.wrapping_add(s.c))
    }
}

fn emit_block(sts: &[St], ind: usize, out: &mut String, n: &mut u32) {
    let pad = "  ".repeat(ind);
    for s in sts {
        match s {
            St::Plain(k) => out.push_str(&format!("{pad}v ← s{k}(v)?;\n")),
            St::Expr(k) => out.push_str(&format!("{pad}v ← v +% s{k}(v +% 1u32)?;\n")),
            St::Decl(id) => {
                *n += 1;
                out.push_str(&format!("{pad}d{n} ≔ mk(log, {id}u64);\n"));
            }
            St::Loop(k, cnt) => {
                *n += 1;
                let i = format!("i{n}");
                out.push_str(&format!(
                    "{pad}μ{i} ≔ 0u32;\n{pad}⟳ {i} < {cnt}u32 ↓ {cnt}u32 - {i} {{ v ← s{k}(v)?; {i} ← {i} + 1u32; }}\n"
                ));
            }
            St::Block(b) => {
                out.push_str(&format!("{pad}{{\n"));
                emit_block(b, ind + 1, out, n);
                out.push_str(&format!("{pad}}};\n"));
            }
        }
    }
}

const LIB: &str = "\
▢ D<'a> { log: &'a Cell<u64>, id: u64 }
⊐ <'a> Drop for D<'a> { ƒ drop(&!self) ! ∅ { self.log.set(self.log.get() *% 31u64 +% self.id); } }
ƒ mk<'a>(l: &'a Cell<u64>, id: u64) → D<'a> ! ∅ { D { log: l, id: id } }
";

const INPUTS: u32 = 12;

fn program(seed: u64) -> (String, u64, [u32; 3]) {
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let opt = seed.is_multiple_of(2);
    let nsteps = 2 + r.below(3) as usize;
    let steps: Vec<Step> = (0..nsteps)
        .map(|k| {
            let m = 3 + r.below(6) as u32;
            Step {
                m,
                r: r.below(u64::from(m)) as u32,
                c: 1 + r.below(100) as u32,
                code: 1 + k as u8 * 10 + r.below(9) as u8,
            }
        })
        .collect();
    let mut ids = 0;
    let body = gen_block(&mut r, nsteps, 0, &mut ids);
    let mut src = String::from(LIB);
    for (k, s) in steps.iter().enumerate() {
        if opt {
            src.push_str(&format!(
                "ƒ s{k}(x: u32) → Option<u32> ! ∅ {{ ⎇ x % {}u32 == {}u32 {{ None }} ⇝ {{ Some(x +% {}u32) }} }}\n",
                s.m, s.r, s.c
            ));
        } else {
            src.push_str(&format!(
                "ƒ s{k}(x: u32) → Result<u32, u8> ! ∅ {{ ⎇ x % {}u32 == {}u32 {{ Err({}u8) }} ⇝ {{ Ok(x +% {}u32) }} }}\n",
                s.m, s.r, s.code, s.c
            ));
        }
    }
    let ret = if opt {
        "Option<u32>"
    } else {
        "Result<u32, u8>"
    };
    src.push_str(&format!(
        "ƒ f<'a>(x: u32, log: &'a Cell<u64>) → {ret} ! ∅ {{\n  μv ≔ x;\n"
    ));
    let mut n = 0;
    emit_block(&body, 1, &mut src, &mut n);
    src.push_str(if opt {
        "  Some(v)\n}\n"
    } else {
        "  Ok(v)\n}\n"
    });
    let fold = if opt {
        "⌥ r { Some(v) ↦ (v as u64), None ↦ 7777u64 }"
    } else {
        "⌥ r { Ok(v) ↦ (v as u64), Err(e) ↦ 1000u64 + (e as u64) }"
    };
    src.push_str(&format!(
        "ƒ main() → u64 ! ∅ {{\n  log ≔ Cell::new(0u64);\n  μacc ≔ 0u64;\n  μi ≔ 0u32;\n  ⟳ i < {INPUTS}u32 ↓ {INPUTS}u32 - i {{\n    r ≔ f(i *% 7u32, &log);\n    acc ← acc *% 31u64 +% ({fold});\n    i ← i + 1u32;\n  }}\n  acc *% 1000003u64 +% log.get()\n}}\n"
    ));
    // mô hình
    let (mut acc, mut log) = (0u64, 0u64);
    let (mut early, mut oks, mut errs) = (0u32, 0u32, 0u32);
    for i in 0..INPUTS {
        let mut v = i.wrapping_mul(7);
        let mut ex = Exec {
            log,
            early_with_live: false,
        };
        let mut scopes = Vec::new();
        let res = model_block(&body, &steps, &mut v, &mut scopes, &mut ex);
        log = ex.log;
        acc = acc.wrapping_mul(31).wrapping_add(match res {
            Ok(()) => {
                oks += 1;
                u64::from(v)
            }
            Err(e) => {
                errs += 1;
                if opt { 7777 } else { 1000 + u64::from(e) }
            }
        });
        if ex.early_with_live {
            early += 1;
        }
    }
    (
        src,
        acc.wrapping_mul(1_000_003).wrapping_add(log),
        [early, oks, errs],
    )
}

fn parse_u64(v: Option<&str>) -> Option<u64> {
    let v = v?;
    let v = v.strip_prefix("Int(").unwrap_or(v);
    v.split(',').next()?.trim().parse().ok()
}

/// (backend, giá trị main)
fn run(src: &str, tag: &str) -> Vec<(String, Option<u64>)> {
    let opts = Options {
        edition: 2,
        ..Options::default()
    };
    let mut res = check(
        &[SourceInput {
            name: "main.gt".to_string(),
            text: src.to_string(),
        }],
        &opts,
    )
    .expect("đầu vào hợp lệ");
    let errs: Vec<String> = res
        .report
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str().starts_with('E'))
        .map(|d| format!("{}: {}", d.code.as_str(), d.message))
        .collect();
    assert!(errs.is_empty(), "{tag}: gritc từ chối: {errs:?}\n{src}");
    #[allow(unused_mut)]
    let mut out = Vec::new();
    #[cfg(feature = "llvm")]
    {
        use gt_driver::pipeline::{attestation, mir_dump};
        let att = {
            let art = res.open_gate().expect("cổng mở");
            attestation(&res, &mir_dump(art))
        };
        let dir = std::env::temp_dir().join(format!("gt-qm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tạo thư mục");
        for (pname, prof) in [
            ("dev", gt_codegen_llvm::Opt::Dev),
            ("release", gt_codegen_llvm::Opt::Release),
            ("release-o2", gt_codegen_llvm::Opt::ReleaseO2),
        ] {
            let exe = dir.join(format!("{tag}-{pname}.exe"));
            gt_driver::native::build_exe_profile(
                res.open_gate().expect("cổng"),
                &exe,
                true,
                &att,
                prof,
            )
            .unwrap_or_else(|e| panic!("{tag}/{pname}: backend lỗi: {e}\n{src}"));
            let r =
                gt_driver::native::run_exe_timeout(&exe, Some(std::time::Duration::from_secs(30)));
            let _ = std::fs::remove_file(&exe);
            assert_eq!(
                r.end,
                RunEnd::Normal,
                "{tag}/{pname}: exe {:?}\n{src}",
                r.end
            );
            out.push((format!("llvm-{pname}"), parse_u64(r.value.as_deref())));
        }
    }
    let art = res.take_gated().expect("cổng mở");
    let r = gt_driver::runner::run_main(art);
    assert_eq!(
        r.end,
        RunEnd::Normal,
        "{tag}: thông dịch {:?}\n{src}",
        r.end
    );
    out.push(("interp".to_string(), parse_u64(r.value.as_deref())));
    out
}

#[test]
fn question_mark_matches_an_independent_model_on_every_backend_including_drop_order_on_early_exit()
{
    let (mut early, mut oks, mut errs) = (0u32, 0u32, 0u32);
    for seed in 1..=40u64 {
        let (src, want, [e, o, x]) = program(seed);
        early += e;
        oks += o;
        errs += x;
        for (backend, got) in run(&src, &format!("qm{seed}")) {
            assert_eq!(
                got,
                Some(want),
                "#{seed} ({backend}): giá trị/thứ tự drop sai, kỳ vọng {want}\n{src}"
            );
        }
    }
    // bộ sinh không rỗng: có cả đường thành công lẫn đường thoát sớm, và thoát sớm khi còn D sống
    assert!(oks >= 40, "quá ít lần thành công: {oks}");
    assert!(errs >= 40, "quá ít lần thoát sớm: {errs}");
    assert!(
        early >= 20,
        "quá ít lần thoát sớm khi còn giá trị Drop sống: {early}"
    );
}
