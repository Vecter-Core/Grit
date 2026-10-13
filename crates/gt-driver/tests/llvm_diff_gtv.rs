//! So sánh codegen ≡ thông dịch ≡ MÔ HÌNH ĐỘC LẬP cho generic, trait và `Vec` (ADR-034, lệnh 2026-10-14 bước (iv)).
//! Điểm mù cần xử lý: thông dịch và codegen CÙNG dùng `TypeDb::select_impl`, nên chọn sai impl thì cả hai sai giống nhau. Vì vậy giá trị
//! kỳ vọng ở đây tính bằng MÔ HÌNH RUST CỦA BỘ SINH (bảng impl của bộ sinh, bộ đánh giá hàm generic theo cấu trúc chương trình sinh ra);
//! không gọi `select_impl`/gt-types/gt-mir. Ba nguồn: mô hình, thông dịch, exe LLVM.
//! Loại chương trình (ngưỡng ≥ 30 mỗi loại đã so sánh được): generic_unbounded, bound_call, impl_generic, multi_trait, vec_ops,
//! vec_index_oob (ba nguồn), vec_arena_exhausted (exe-only, `GRITRT_ARENA_LIMIT`).
//! Chạy theo lô: `GT_GTV_START=1 GT_GTV_COUNT=25 GT_GTV_OUT=docs/CODEGEN_DIFF_RESULTS.jsonl cargo test -p gt-driver --test llvm_diff_gtv -- --ignored batch`
//! Kiểm độ nhạy: `GT_GTV_FIRST=1` làm `select_impl` chọn impl đầu ⇒ lô PHẢI có bất đồng.
#![cfg(feature = "llvm")]
#[path = "../../gt-mir/tests/support/progen.rs"]
mod progen;

use gt_driver::native::build_exe;
use gt_driver::pipeline::attestation;
use gt_driver::runner::run_main;
use gt_driver::{Options, RunEnd, check};
use gt_resolve::SourceInput;
use progen::Rng;
use std::collections::BTreeMap;
use std::io::Write;
use std::process::Command;

const KINDS: [&str; 7] = [
    "generic_unbounded",
    "bound_call",
    "impl_generic",
    "multi_trait",
    "vec_ops",
    "vec_index_oob",
    "vec_arena_exhausted",
];

/// kết quả kỳ vọng: giá trị (chuỗi số) hoặc tên abort
type Want = Result<String, &'static str>;

// ------------------------------------------------------------------------------------------------ mô hình generic/trait

#[derive(Clone, Debug)]
enum Val {
    Leaf(usize, i64),
    W(Box<Val>, i64),
}

struct Model {
    /// `Tr1`/`Tr2` cho kiểu lá A,B,C: (nhân, cộng)
    t1: [(i64, i64); 3],
    t2: [(i64, i64); 3],
    /// `⊐ <T> Tr1 for W<T>`: k·a + b
    w1: (i64, i64),
    /// `⊐ <T: Tr1> Tr2 for W<T>`: m1(inner)·a + k
    w2: i64,
}

impl Model {
    fn m1(&self, v: &Val) -> i64 {
        match v {
            Val::Leaf(t, x) => x.wrapping_mul(self.t1[*t].0).wrapping_add(self.t1[*t].1),
            Val::W(_, k) => k.wrapping_mul(self.w1.0).wrapping_add(self.w1.1),
        }
    }
    fn m2(&self, v: &Val) -> i64 {
        match v {
            Val::Leaf(t, x) => x.wrapping_mul(self.t2[*t].0).wrapping_add(self.t2[*t].1),
            Val::W(inner, k) => self.m1(inner).wrapping_mul(self.w2).wrapping_add(*k),
        }
    }
}

#[derive(Clone, Debug)]
enum Term {
    M1 { method_syntax: bool },
    M2,
    Call(usize),
    Konst(i64),
}

#[derive(Clone, Debug)]
struct GFn {
    /// bit 0 = Tr1, bit 1 = Tr2
    mask: u8,
    c0: i64,
    terms: Vec<Term>,
}

fn eval_fn(fns: &[GFn], i: usize, m: &Model, x: &Val) -> i64 {
    let f = &fns[i];
    let mut r = f.c0;
    for t in &f.terms {
        let tv = match t {
            Term::M1 { .. } => m.m1(x),
            Term::M2 => m.m2(x),
            Term::Call(j) => eval_fn(fns, *j, m, x),
            Term::Konst(c) => *c,
        };
        r = r.wrapping_mul(3).wrapping_add(tv);
    }
    r
}

fn fn_src(fns: &[GFn], i: usize) -> String {
    let f = &fns[i];
    let bound = match f.mask {
        0 => String::new(),
        1 => ": Tr1".to_string(),
        2 => ": Tr2".to_string(),
        _ => ": Tr1 + Tr2".to_string(),
    };
    let mut s = format!("ƒ f{i}<T{bound}>(x: &T) → i64 ! ∅ {{ μr ≔ {}i64; ", f.c0);
    for t in &f.terms {
        let e = match t {
            Term::M1 { method_syntax } => {
                if *method_syntax && f.mask == 1 {
                    "x.m()".to_string()
                } else {
                    "Tr1::m(x)".to_string()
                }
            }
            Term::M2 => {
                if f.mask == 2 && false {
                    "x.m()".to_string()
                } else {
                    "Tr2::m(x)".to_string()
                }
            }
            Term::Call(j) => format!("f{j}(x)"),
            Term::Konst(c) => format!("{c}i64"),
        };
        s.push_str(&format!("r ← r *% 3i64 +% {e}; "));
    }
    s.push_str("r }\n");
    s
}

fn val_src(v: &Val) -> String {
    match v {
        Val::Leaf(t, x) => format!("{} {{ v: {x}i64 }}", ["A", "B", "C"][*t]),
        Val::W(inner, k) => format!("W {{ inner: {}, k: {k}i64 }}", val_src(inner)),
    }
}

fn gen_val(rng: &mut Rng, allow_w: bool) -> Val {
    let leaf = Val::Leaf(rng.below(3) as usize, rng.below(1000) as i64);
    if allow_w && rng.chance(55) {
        let inner = if rng.chance(30) {
            Val::W(Box::new(leaf), rng.below(500) as i64)
        } else {
            leaf
        };
        Val::W(Box::new(inner), rng.below(500) as i64)
    } else {
        leaf
    }
}

/// chương trình generic/trait; trả (nguồn, kỳ vọng)
fn gen_generic(kind: &str, rng: &mut Rng) -> (String, Want) {
    let m = Model {
        t1: [0, 1, 2].map(|_| (rng.below(9) as i64 + 1, rng.below(50) as i64)),
        t2: [0, 1, 2].map(|_| (rng.below(9) as i64 + 1, rng.below(50) as i64)),
        w1: (rng.below(9) as i64 + 1, rng.below(50) as i64),
        w2: rng.below(9) as i64 + 1,
    };
    // hàm generic: mỗi hàm chỉ gọi hàm có chỉ số nhỏ hơn và mặt nạ ràng buộc ⊆ của nó
    let masks: &[u8] = match kind {
        "generic_unbounded" => &[0],
        "bound_call" => &[1],
        "impl_generic" => &[1, 3],
        _ => &[1, 2, 3],
    };
    let nfn = 2 + rng.below(3) as usize;
    let mut fns: Vec<GFn> = Vec::new();
    for i in 0..nfn {
        let mask = masks[rng.below(masks.len() as u64) as usize];
        let mut terms = Vec::new();
        for _ in 0..(1 + rng.below(3)) {
            let mut opts: Vec<u8> = vec![3]; // hằng
            if mask & 1 != 0 {
                opts.push(0);
            }
            if mask & 2 != 0 {
                opts.push(1);
            }
            let callable: Vec<usize> = (0..i).filter(|&j| fns[j].mask & !mask == 0).collect();
            if !callable.is_empty() {
                opts.push(2);
            }
            match opts[rng.below(opts.len() as u64) as usize] {
                0 => terms.push(Term::M1 {
                    method_syntax: rng.chance(50),
                }),
                1 => terms.push(Term::M2),
                2 => terms.push(Term::Call(
                    callable[rng.below(callable.len() as u64) as usize],
                )),
                _ => terms.push(Term::Konst(rng.below(100) as i64)),
            }
        }
        fns.push(GFn {
            mask,
            c0: rng.below(20) as i64,
            terms,
        });
    }
    let mut src = String::new();
    src.push_str("⊏ Tr1 { ƒ m(&self) → i64 ! ∅; }\n⊏ Tr2 { ƒ m(&self) → i64 ! ∅; }\n");
    src.push_str("▢ A { v: i64 }\n▢ B { v: i64 }\n▢ C { v: i64 }\n▢ W<T> { inner: T, k: i64 }\n▢ P<X, Y> { a: X, b: Y }\n");
    for (t, name) in ["A", "B", "C"].iter().enumerate() {
        src.push_str(&format!(
            "⊐ Tr1 for {name} {{ ƒ m(&self) → i64 ! ∅ {{ self.v *% {}i64 +% {}i64 }} }}\n",
            m.t1[t].0, m.t1[t].1
        ));
        src.push_str(&format!(
            "⊐ Tr2 for {name} {{ ƒ m(&self) → i64 ! ∅ {{ self.v *% {}i64 +% {}i64 }} }}\n",
            m.t2[t].0, m.t2[t].1
        ));
    }
    src.push_str(&format!(
        "⊐ <T> Tr1 for W<T> {{ ƒ m(&self) → i64 ! ∅ {{ self.k *% {}i64 +% {}i64 }} }}\n",
        m.w1.0, m.w1.1
    ));
    src.push_str(&format!(
        "⊐ <T: Tr1> Tr2 for W<T> {{ ƒ m(&self) → i64 ! ∅ {{ Tr1::m(&self.inner) *% {}i64 +% self.k }} }}\n",
        m.w2
    ));
    src.push_str("ƒ idt<T>(x: T) → T ! ∅ { x }\nƒ swp<X, Y>(p: P<X, Y>) → P<Y, X> ! ∅ { P { a: p.b, b: p.a } }\n");
    for i in 0..fns.len() {
        src.push_str(&fn_src(&fns, i));
    }
    // main: vài lời gọi trên giá trị cụ thể
    let allow_w = kind == "impl_generic" || kind == "multi_trait";
    let mut main = String::from("ƒ main() → i64 ! ∅ {\n  μh ≔ 0i64;\n");
    let mut want: i64 = 0;
    let ncalls = 3 + rng.below(4) as usize;
    for n in 0..ncalls {
        let i = rng.below(fns.len() as u64) as usize;
        let v = gen_val(rng, allow_w);
        main.push_str(&format!("  v{n} ≔ {};\n", val_src(&v)));
        let r = eval_fn(&fns, i, &m, &v);
        main.push_str(&format!("  h ← h *% 31i64 +% f{i}(&v{n});\n"));
        want = want.wrapping_mul(31).wrapping_add(r);
        if kind == "generic_unbounded" || rng.chance(25) {
            // `idt` và `swp` trên giá trị thật: P { a: lá, b: số } → hoán đổi → đọc lại
            if let Val::Leaf(t, x) = &v {
                let ty = ["A", "B", "C"][*t];
                let k = rng.below(100) as i64;
                main.push_str(&format!(
                    "  q{n} ≔ swp(P {{ a: {ty} {{ v: {x}i64 }}, b: {k}i64 }});\n  h ← h *% 31i64 +% idt(q{n}.b.v) +% q{n}.a;\n"
                ));
                want = want.wrapping_mul(31).wrapping_add(*x).wrapping_add(k);
            }
        }
    }
    main.push_str("  h\n}\n");
    src.push_str(&main);
    (src, Ok(want.to_string()))
}

// ------------------------------------------------------------------------------------------------ mô hình Vec

#[derive(Clone, Copy, Debug)]
enum Elem {
    U8,
    U16,
    U32,
    U64,
    I32,
    I64,
    Bool,
    Pst,
    Tup,
}

const ELEMS: [Elem; 9] = [
    Elem::U8,
    Elem::U16,
    Elem::U32,
    Elem::U64,
    Elem::I32,
    Elem::I64,
    Elem::Bool,
    Elem::Pst,
    Elem::Tup,
];

impl Elem {
    fn ty(self) -> &'static str {
        match self {
            Elem::U8 => "u8",
            Elem::U16 => "u16",
            Elem::U32 => "u32",
            Elem::U64 => "u64",
            Elem::I32 => "i32",
            Elem::I64 => "i64",
            Elem::Bool => "bool",
            Elem::Pst => "Pst",
            Elem::Tup => "(u8, u32)",
        }
    }
    /// phần tử ngẫu nhiên: ba trường số (không âm) — chỉ một phần dùng tuỳ kiểu
    fn sample(self, rng: &mut Rng) -> [u64; 3] {
        fn hi(rng: &mut Rng, bits: u32) -> u64 {
            rng.next() & ((1u64 << bits) - 1)
        }
        match self {
            Elem::U8 => [hi(rng, 8), 0, 0],
            Elem::U16 => [hi(rng, 16), 0, 0],
            Elem::U32 => [hi(rng, 32), 0, 0],
            Elem::U64 => [rng.next(), 0, 0],
            Elem::I32 => [hi(rng, 31), 0, 0],
            Elem::I64 => [hi(rng, 62), 0, 0],
            Elem::Bool => [rng.below(2), 0, 0],
            Elem::Pst => {
                let a = hi(rng, 8);
                let b = rng.next();
                [a, b, hi(rng, 16)]
            }
            Elem::Tup => {
                let a = hi(rng, 8);
                [a, hi(rng, 32), 0]
            }
        }
    }
    fn lit(self, f: [u64; 3]) -> String {
        match self {
            Elem::U8 => format!("{}u8", f[0]),
            Elem::U16 => format!("{}u16", f[0]),
            Elem::U32 => format!("{}u32", f[0]),
            Elem::U64 => format!("{}u64", f[0]),
            Elem::I32 => format!("{}i32", f[0]),
            Elem::I64 => format!("{}i64", f[0]),
            Elem::Bool => (if f[0] == 1 { "true" } else { "false" }).to_string(),
            Elem::Pst => format!("Pst {{ a: {}u8, b: {}u64, c: {}u16 }}", f[0], f[1], f[2]),
            Elem::Tup => format!("({}u8, {}u32)", f[0], f[1]),
        }
    }
    /// biểu thức Grit đổi `s[j]` thành u64 và công thức Rust tương ứng
    fn conv_src(self) -> &'static str {
        match self {
            Elem::Bool => "(⎇ s[j] { 1u64 } ⇝ { 0u64 })",
            Elem::Pst => "((s[j].a as u64) *% 3u64 +% s[j].b *% 5u64 +% (s[j].c as u64))",
            Elem::Tup => "((s[j].0 as u64) *% 7u64 +% (s[j].1 as u64))",
            _ => "(s[j] as u64)",
        }
    }
    fn conv(self, f: [u64; 3]) -> u64 {
        match self {
            Elem::Pst => f[0]
                .wrapping_mul(3)
                .wrapping_add(f[1].wrapping_mul(5))
                .wrapping_add(f[2]),
            Elem::Tup => f[0].wrapping_mul(7).wrapping_add(f[1]),
            _ => f[0],
        }
    }
}

fn vec_prelude(e: Elem) -> String {
    let mut s = String::from("▢ Pst { a: u8, b: u64, c: u16 }\n");
    s.push_str(&format!(
        "ƒ fold(s: &[{}]) → u64 ! ∅ {{ μh ≔ 0u64; μj ≔ 0usize; ⟳ j < s.len() ↓ s.len() - j {{ h ← h *% 131u64 +% {}; j ← j + 1usize; }} h }}\n",
        e.ty(),
        e.conv_src()
    ));
    s
}

fn gen_vec_ops(rng: &mut Rng) -> (String, Want) {
    let e = ELEMS[rng.below(ELEMS.len() as u64) as usize];
    let n_push = 1 + rng.below(70) as usize;
    let mut model: Vec<[u64; 3]> = Vec::new();
    let mut body = String::new();
    let mut nset = 0;
    for _ in 0..n_push {
        let f = e.sample(rng);
        body.push_str(&format!("  v.push({});\n", e.lit(f)));
        model.push(f);
        if rng.chance(25) {
            let idx = rng.below(model.len() as u64) as usize;
            let g = e.sample(rng);
            body.push_str(&format!(
                "  m{nset} ≔ v.as_mut_slice();\n  m{nset}[{idx}] ← {};\n",
                e.lit(g)
            ));
            model[idx] = g;
            nset += 1;
        }
    }
    let mut h = 0u64;
    for f in &model {
        h = h.wrapping_mul(131).wrapping_add(e.conv(*f));
    }
    h = h.wrapping_add(model.len() as u64);
    let src = format!(
        "{}ƒ main() → u64 ! {{mem}} {{\n  μar ≔ Arena::new();\n  μv ≔ Vec::new(&!ar);\n{body}  fold(v.as_slice()) +% (v.len() as u64)\n}}\n",
        vec_prelude(e)
    );
    (src, Ok(h.to_string()))
}

fn gen_vec_oob(rng: &mut Rng) -> (String, Want) {
    let n = rng.below(40) as usize + 1;
    let d = rng.below(3) as usize;
    let mut body = String::new();
    for _ in 0..n {
        body.push_str(&format!("  v.push({}u64);\n", rng.below(1000)));
    }
    let tail = if rng.chance(50) {
        format!("  s ≔ v.as_slice();\n  s[{}]\n", n + d)
    } else {
        format!("  m ≔ v.as_mut_slice();\n  m[{}] ← 1u64;\n  0u64\n", n + d)
    };
    let src = format!(
        "ƒ main() → u64 ! {{mem}} {{\n  μar ≔ Arena::new();\n  μv ≔ Vec::new(&!ar);\n{body}{tail}}}\n"
    );
    (src, Err("index_out_of_bounds"))
}

/// giới hạn arena của kịch bản này (byte)
const ARENA_LIMIT: u64 = 2 * 1024 * 1024;

fn gen_vec_exhaust(rng: &mut Rng) -> (String, Want) {
    // vùng không mơ hồ: ≤ 10 000 u64 (cấp phát tối đa ~128 KiB trong chunk đầu 1 MiB) ⇒ chạy; ≥ 300 000 (một lần cấp ≥ 2 MiB) ⇒ cạn
    let ok = rng.chance(50);
    let n = if ok {
        100 + rng.below(9900)
    } else {
        300_000 + rng.below(400_000)
    };
    let src = format!(
        "ƒ main() → u64 ! {{mem}} {{\n  μar ≔ Arena::new();\n  μv ≔ Vec::new(&!ar);\n  μi ≔ 0u64;\n  ⟳ i < {n}u64 ↓ {n}u64 - i {{ v.push(i); i ← i + 1u64; }}\n  v.len() as u64\n}}\n"
    );
    (
        src,
        if ok {
            Ok(n.to_string())
        } else {
            Err("arena_exhausted")
        },
    )
}

// ------------------------------------------------------------------------------------------------ chạy

#[derive(Default)]
struct Tally {
    programs: BTreeMap<&'static str, u64>,
    compared: BTreeMap<&'static str, u64>,
    dropped: BTreeMap<String, u64>,
    mismatch: u64,
    detail: Vec<String>,
}

fn norm_interp(v: &Option<String>) -> Option<String> {
    let s = v.as_deref()?;
    s.strip_prefix("Int(")
        .and_then(|i| i.split(',').next())
        .map(str::to_string)
}

fn run_exe_env(
    exe: &std::path::Path,
    env: &[(&str, String)],
) -> (Option<i32>, String, Option<String>) {
    let result = exe.with_extension("result");
    let _ = std::fs::remove_file(&result);
    let mut c = Command::new(exe);
    c.env("GRITRT_RESULT_FILE", &result);
    for (k, v) in env {
        c.env(k, v);
    }
    let o = match c.output() {
        Ok(o) => o,
        Err(e) => return (None, format!("không chạy được exe: {e}"), None),
    };
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    let val = std::fs::read_to_string(&result).ok();
    let _ = std::fs::remove_file(&result);
    (o.status.code(), err, val)
}

fn run_one(kind: &'static str, seed: u64, t: &mut Tally, dir: &std::path::Path) {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x67AB | 1);
    *t.programs.entry(kind).or_default() += 1;
    let (source, want) = match kind {
        "vec_ops" => gen_vec_ops(&mut rng),
        "vec_index_oob" => gen_vec_oob(&mut rng),
        "vec_arena_exhausted" => gen_vec_exhaust(&mut rng),
        k => gen_generic(k, &mut rng),
    };
    let mut res = match check(
        &[SourceInput {
            name: "main.gt".into(),
            text: source.clone(),
        }],
        &Options::default(),
    ) {
        Ok(r) => r,
        Err(err) => {
            t.mismatch += 1;
            t.detail.push(format!(
                "#{kind}/{seed}: đầu vào không hợp lệ: {err}\n{source}"
            ));
            return;
        }
    };
    if !res.verdict.allows_output() {
        let code = res
            .report
            .diagnostics
            .iter()
            .find(|d| d.level == gt_diag::Level::Error)
            .map_or("ICE".to_string(), |d| d.code.as_str().to_string());
        *t.dropped.entry(format!("{kind}: ĐỎ {code}")).or_default() += 1;
        if code == "E9901" {
            t.mismatch += 1;
            t.detail.push(format!("#{kind}/{seed}: ICE\n{source}"));
        }
        return;
    }
    let exe = dir.join(format!("g{seed}.exe"));
    {
        let art = res.open_gate().expect("cổng mở");
        let att = attestation(&res, &gt_driver::pipeline::mir_dump(art));
        if let Err(err) = build_exe(art, &exe, true, &att) {
            // chưa hạ được (thoát 3) hoặc Guard từ chối: bỏ khỏi so sánh, báo tỉ lệ
            *t.dropped
                .entry(format!("{kind}: chưa hạ được"))
                .or_default() += 1;
            if matches!(err, gt_driver::native::NativeError::Guard(_)) {
                t.mismatch += 1;
                t.detail
                    .push(format!("#{kind}/{seed}: Guard từ chối: {err}\n{source}"));
            }
            return;
        }
    }
    let env: Vec<(&str, String)> = if kind == "vec_arena_exhausted" {
        vec![("GRITRT_ARENA_LIMIT", ARENA_LIMIT.to_string())]
    } else {
        Vec::new()
    };
    let (code, err, val) = run_exe_env(&exe, &env);
    let _ = std::fs::remove_file(&exe);
    let llvm_ok = match &want {
        Ok(w) => code == Some(0) && val.as_deref() == Some(w.as_str()),
        Err(a) => code == Some(70) && err.contains(&format!("abort: {a}")),
    };
    // thông dịch: bỏ qua với kịch bản cạn Arena (thông dịch không có giới hạn bộ nhớ)
    let interp_ok = if kind == "vec_arena_exhausted" {
        true
    } else {
        let r = run_main(res.take_gated().expect("sản phẩm"));
        match (&want, &r.end) {
            (Ok(w), RunEnd::Normal) => norm_interp(&r.value).as_deref() == Some(w.as_str()),
            (Err(a), RunEnd::Aborted(b)) => b == a,
            _ => false,
        }
    };
    if llvm_ok && interp_ok {
        *t.compared.entry(kind).or_default() += 1;
    } else {
        t.mismatch += 1;
        t.detail.push(format!(
            "#{kind}/{seed}: kỳ vọng {want:?} / llvm (mã {code:?}, giá trị {val:?}, stderr {err:?}) khớp={llvm_ok} / thông dịch khớp={interp_ok}\n{source}"
        ));
    }
}

fn env_u64(k: &str, d: u64) -> u64 {
    std::env::var(k)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(d)
}

fn run_batch(start: u64, count: u64) -> Tally {
    let dir = std::env::temp_dir().join(format!("gt-gtv-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo");
    if std::env::var("GT_GTV_FIRST").is_ok_and(|v| v == "1") {
        gt_types::set_select_impl_first_for_test(true);
    }
    let mut t = Tally::default();
    for s in start..start + count {
        run_one(KINDS[(s % KINDS.len() as u64) as usize], s, &mut t, &dir);
    }
    gt_types::set_select_impl_first_for_test(false);
    let _ = std::fs::remove_dir_all(&dir);
    t
}

fn summary(t: &Tally) -> String {
    format!(
        "{{\"programs\":{:?},\"compared\":{:?},\"dropped\":{:?},\"mismatch\":{}}}",
        t.programs, t.compared, t.dropped, t.mismatch
    )
}

#[test]
#[ignore = "chạy theo lô bằng GT_GTV_START/COUNT/OUT"]
fn batch() {
    let (start, count) = (env_u64("GT_GTV_START", 1), env_u64("GT_GTV_COUNT", 25));
    let t = run_batch(start, count);
    let tag = if std::env::var("GT_GTV_FIRST").is_ok_and(|v| v == "1") {
        "generic-trait-vec-SENSITIVITY-select-first"
    } else {
        "generic-trait-vec"
    };
    let line = format!(
        "{{\"suite\":\"{tag}\",\"start\":{start},\"count\":{count},\"result\":{}}}",
        summary(&t)
    );
    println!("{line}");
    if let Some(out) = std::env::var_os("GT_GTV_OUT") {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(out)
            .expect("mở");
        writeln!(f, "{line}").expect("ghi");
        for d in &t.detail {
            writeln!(f, "  DETAIL {d}").expect("ghi");
        }
    }
    if std::env::var("GT_GTV_FIRST").is_ok_and(|v| v == "1") {
        assert!(
            t.mismatch > 0,
            "độ nhạy: select_impl chọn impl đầu mà lô vẫn XANH ⇒ bộ sinh mù"
        );
    } else {
        assert_eq!(t.mismatch, 0, "bất đồng: {:#?}", t.detail);
    }
}

/// lô nhỏ cố định cho cổng đầy đủ: mỗi loại ≥ 2 chương trình
#[test]
fn tiny_fixed_batch() {
    let t = run_batch(1, 14);
    println!("{}", summary(&t));
    assert_eq!(t.mismatch, 0, "bất đồng: {:#?}", t.detail);
    let compared: u64 = t.compared.values().sum();
    assert!(
        compared >= 10,
        "quá ít chương trình so sánh được: {}",
        summary(&t)
    );
}
