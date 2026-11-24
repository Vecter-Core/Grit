//! `let` phân rã (edition 2, ADR-P2-10) — oracle động về THỨ TỰ DROP sau partial move. Chương trình ngẫu nhiên gồm: khai báo thường, phân rã tuple từ GIÁ TRỊ (phần `_` còn lại nằm trong tạm và bị drop
//! ngay cuối câu lệnh, theo thứ tự trường), phân rã từ CHỖ (biến `p`: chỉ phần bị bind bị move, phần còn lại ở lại trong `p` và bị drop khi `p` hết phạm vi), khối lồng.
//! Mô hình Rust độc lập: mỗi biến giữ danh sách id còn lại; hết phạm vi drop theo thứ tự NGƯỢC khai báo, mỗi biến drop phần còn lại theo thứ tự trường.
//! Thông dịch MIR VÀ exe LLVM (dev, release, release-o2) phải khớp (`log ← log *% 31 +% id` ở mỗi `drop`).
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

#[derive(Clone)]
enum St {
    /// `xK ≔ mk(&log, id);`
    Single(u64),
    /// `(aK, bK) ≔ (mk(id1), mk(id2));` — `keep[i] = false` ⇒ `_` ở vị trí i
    FromValue([u64; 2], [bool; 2]),
    /// `pK ≔ (mk(id1), mk(id2)); (aK, _) ≔ pK;` hoặc `(_, bK)`: chỉ MỘT trường được bind (`which`)
    FromPlace([u64; 2], usize),
    /// `{ … };`
    Block(Vec<St>),
}

fn gen_block(r: &mut Rng, depth: u32, ids: &mut u64) -> Vec<St> {
    let mut v = Vec::new();
    for _ in 0..1 + r.below(4) {
        let mut id = || {
            *ids += 1;
            *ids
        };
        match r.below(6) {
            0 => v.push(St::Single(id())),
            1 | 2 => {
                let k = [r.below(4) != 0, r.below(4) != 0];
                v.push(St::FromValue([id(), id()], k));
            }
            3 | 4 => {
                let w = r.below(2) as usize;
                v.push(St::FromPlace([id(), id()], w));
            }
            _ if depth < 2 => v.push(St::Block(gen_block(r, depth + 1, ids))),
            _ => {}
        }
    }
    v
}

fn emit(sts: &[St], ind: usize, out: &mut String, n: &mut u32) {
    let pad = "  ".repeat(ind);
    for s in sts {
        *n += 1;
        let k = *n;
        match s {
            St::Single(id) => out.push_str(&format!("{pad}x{k} ≔ mk(&log, {id}u64);\n")),
            St::FromValue([a, b], keep) => {
                let pa = if keep[0] {
                    format!("a{k}")
                } else {
                    "_".to_string()
                };
                let pb = if keep[1] {
                    format!("b{k}")
                } else {
                    "_".to_string()
                };
                out.push_str(&format!(
                    "{pad}({pa}, {pb}) ≔ (mk(&log, {a}u64), mk(&log, {b}u64));\n"
                ));
            }
            St::FromPlace([a, b], which) => {
                out.push_str(&format!(
                    "{pad}p{k} ≔ (mk(&log, {a}u64), mk(&log, {b}u64));\n"
                ));
                let pat = if *which == 0 {
                    format!("(a{k}, _)")
                } else {
                    format!("(_, b{k})")
                };
                out.push_str(&format!("{pad}{pat} ≔ p{k};\n"));
            }
            St::Block(b) => {
                out.push_str(&format!("{pad}{{\n"));
                emit(b, ind + 1, out, n);
                out.push_str(&format!("{pad}}};\n"));
            }
        }
    }
}

/// mô hình: `scopes` = ngăn xếp phạm vi, mỗi phạm vi là danh sách biến theo thứ tự khai báo, mỗi biến là danh sách id còn lại (theo thứ tự trường)
fn model(sts: &[St], log: &mut u64, scopes: &mut Vec<Vec<Vec<u64>>>) {
    let drop_id = |log: &mut u64, id: u64| *log = log.wrapping_mul(31).wrapping_add(id);
    scopes.push(Vec::new());
    for s in sts {
        match s {
            St::Single(id) => scopes.last_mut().expect("phạm vi").push(vec![*id]),
            St::FromValue(ids, keep) => {
                // phần `_` còn lại trong tạm: drop NGAY cuối câu lệnh, theo thứ tự trường
                for i in 0..2 {
                    if !keep[i] {
                        drop_id(log, ids[i]);
                    }
                }
                // biến được bind, theo thứ tự pattern, mỗi biến một id
                for i in 0..2 {
                    if keep[i] {
                        scopes.last_mut().expect("phạm vi").push(vec![ids[i]]);
                    }
                }
            }
            St::FromPlace(ids, which) => {
                // `p` giữ phần còn lại; biến mới bind phần được chọn (khai báo SAU p)
                let sc = scopes.last_mut().expect("phạm vi");
                sc.push(vec![ids[1 - which]]);
                sc.push(vec![ids[*which]]);
            }
            St::Block(b) => model(b, log, scopes),
        }
    }
    for var in scopes.pop().unwrap_or_default().into_iter().rev() {
        for id in var {
            drop_id(log, id);
        }
    }
}

const LIB: &str = "\
▢ D<'a> { log: &'a Cell<u64>, id: u64 }
⊐ <'a> Drop for D<'a> { ƒ drop(&!self) ! ∅ { self.log.set(self.log.get() *% 31u64 +% self.id); } }
ƒ mk<'a>(l: &'a Cell<u64>, id: u64) → D<'a> ! ∅ { D { log: l, id: id } }
";

fn program(seed: u64) -> (String, u64) {
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut ids = 0;
    let body = gen_block(&mut r, 0, &mut ids);
    let mut src = String::from(LIB);
    src.push_str("ƒ main() → u64 ! ∅ {\n  log ≔ Cell::new(0u64);\n  {\n");
    let mut n = 0;
    emit(&body, 2, &mut src, &mut n);
    src.push_str("  };\n  log.get()\n}\n");
    let mut log = 0u64;
    model(&body, &mut log, &mut Vec::new());
    (src, log)
}

fn parse_u64(v: Option<&str>) -> Option<u64> {
    let v = v?;
    let v = v.strip_prefix("Int(").unwrap_or(v);
    v.split(',').next()?.trim().parse().ok()
}

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
        let dir = std::env::temp_dir().join(format!("gt-letpat-{}", std::process::id()));
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
fn destructuring_let_drops_match_an_independent_model_on_every_backend() {
    let (mut from_value, mut from_place) = (0usize, 0usize);
    for seed in 1..=40u64 {
        let (src, want) = program(seed);
        from_value += src.matches("(mk(&log").count();
        from_place += src.matches("≔ p").count();
        for (backend, got) in run(&src, &format!("lp{seed}")) {
            assert_eq!(
                got,
                Some(want),
                "#{seed} ({backend}): thứ tự drop sai, kỳ vọng {want}\n{src}"
            );
        }
    }
    // bộ sinh không rỗng: có cả phân rã từ giá trị lẫn từ chỗ (partial move)
    assert!(from_value >= 30, "quá ít ca từ giá trị: {from_value}");
    assert!(from_place >= 30, "quá ít ca từ chỗ: {from_place}");
}
