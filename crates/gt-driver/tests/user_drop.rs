//! `Drop` người dùng (ADR-C18 bước 1): `drop` chạy ĐÚNG MỘT LẦN cho mỗi giá trị còn khởi tạo khi hết phạm vi, không chạy cho giá trị đã
//! move (kể cả move có điều kiện: cờ drop), chạy cho giá trị cũ khi gán đè, theo thứ tự ngược khai báo; glue: `drop` của chính kiểu rồi
//! các trường theo thứ tự khai báo (biến thể đang có của enum, phần tử mảng theo chỉ số).
//! Mỗi `drop` ghi `log ← log *% 31 +% id` vào một `Cell<u64>` chung ⇒ giá trị `main` trả về mã hoá CẢ dãy và thứ tự drop.
//! Kỳ vọng tính tay bằng `seq` (dãy id theo thứ tự). Chạy bằng thông dịch MIR; có backend LLVM thì chạy thêm exe thật và phải khớp.
use gt_driver::runner::run_main;
use gt_driver::{Options, RunEnd, check};
use gt_resolve::SourceInput;

const LIB: &str = "\
▢ D<'a> { log: &'a Cell<u64>, id: u64 }
⊐ <'a> Drop for D<'a> { ƒ drop(&!self) ! ∅ { self.log.set(self.log.get() *% 31u64 +% self.id); } }
ƒ mk<'a>(l: &'a Cell<u64>, id: u64) → D<'a> ! ∅ { D { log: l, id: id } }
ƒ consume<'a>(d: D<'a>) ! ∅ { }
";

/// mã hoá một dãy id như `drop` làm
fn seq(ids: &[u64]) -> u64 {
    ids.iter()
        .fold(0u64, |a, &x| a.wrapping_mul(31).wrapping_add(x))
}

fn inputs(main: &str) -> Vec<SourceInput> {
    vec![SourceInput {
        name: "main.gt".to_string(),
        text: format!("{LIB}{main}"),
    }]
}

fn int(v: Option<&str>) -> Option<u64> {
    let v = v?;
    let v = v.strip_prefix("Int(").unwrap_or(v);
    v.split(',').next()?.trim().parse().ok()
}

/// chạy `main` (trả `u64`) bằng thông dịch và, nếu có, bằng exe LLVM
fn run(src: &str, tag: &str) -> Vec<(&'static str, Option<u64>)> {
    let mut res = check(&inputs(src), &Options::default()).expect("đầu vào hợp lệ");
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
        let dir = std::env::temp_dir().join(format!("gt-user-drop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tạo thư mục");
        let exe = dir.join(format!("{tag}.exe"));
        let art = res.open_gate().expect("cổng");
        gt_driver::native::build_exe_profile(art, &exe, true, &att, gt_codegen_llvm::Opt::Dev)
            .unwrap_or_else(|e| panic!("{tag}: backend lỗi: {e}\n{src}"));
        let r = gt_driver::native::run_exe_timeout(&exe, Some(std::time::Duration::from_secs(30)));
        let _ = std::fs::remove_file(&exe);
        assert_eq!(r.end, RunEnd::Normal, "{tag}: exe {:?}", r.end);
        out.push(("llvm", int(r.value.as_deref())));
    }
    let art = res.take_gated().expect("cổng mở");
    let r = run_main(art);
    assert_eq!(
        r.end,
        RunEnd::Normal,
        "{tag}: thông dịch {:?}\n{src}",
        r.end
    );
    out.push(("interp", int(r.value.as_deref())));
    out
}

fn expect(tag: &str, body: &str, ids: &[u64]) {
    let src = format!(
        "ƒ main() → u64 ! ∅ {{\n  log ≔ Cell::new(0u64);\n  {{\n{body}\n  }};\n  log.get()\n}}\n"
    );
    for (backend, got) in run(&src, tag) {
        assert_eq!(
            got,
            Some(seq(ids)),
            "{tag} ({backend}): dãy drop sai, kỳ vọng {ids:?}\n{src}"
        );
    }
}

#[test]
fn drops_run_once_in_reverse_declaration_order_and_never_after_a_move() {
    expect(
        "reverse",
        "a ≔ mk(&log, 1u64); b ≔ mk(&log, 2u64);",
        &[2, 1],
    );
    expect(
        "moved",
        "a ≔ mk(&log, 1u64); consume(a); b ≔ mk(&log, 2u64);",
        &[1, 2],
    );
    expect("temp", "mk(&log, 7u64); mk(&log, 8u64);", &[7, 8]);
    expect(
        "reassign",
        "μa ≔ mk(&log, 1u64); a ← mk(&log, 2u64); b ≔ mk(&log, 3u64);",
        &[1, 3, 2],
    );
    expect(
        "loop",
        "⟳ i in 0u64..3u64 { d ≔ mk(&log, i +% 10u64); }",
        &[10, 11, 12],
    );
}

#[test]
fn a_conditional_move_uses_a_drop_flag() {
    let f = "ƒ cond<'a>(l: &'a Cell<u64>, c: bool) ! ∅ {\n  a ≔ mk(l, 1u64);\n  b ≔ mk(l, 2u64);\n  ⎇ c { consume(a); } ⇝ { }\n  x ≔ mk(l, 3u64);\n}\n";
    for (c, ids) in [("true", vec![1u64, 3, 2]), ("false", vec![3, 2, 1])] {
        let src = format!(
            "{f}ƒ main() → u64 ! ∅ {{\n  log ≔ Cell::new(0u64);\n  cond(&log, {c});\n  log.get()\n}}\n"
        );
        for (backend, got) in run(&src, &format!("cond_{c}")) {
            assert_eq!(got, Some(seq(&ids)), "cond {c} ({backend})\n{src}");
        }
    }
}

#[test]
fn glue_drops_the_value_then_its_fields_in_declaration_order() {
    let types = "▢ W<'a> { x: D<'a>, y: D<'a> }\n▢ P<'a> { log: &'a Cell<u64>, inner: D<'a>, id: u64 }\n⊐ <'a> Drop for P<'a> { ƒ drop(&!self) ! ∅ { self.log.set(self.log.get() *% 31u64 +% self.id); } }\n";
    let cases: [(&str, &str, &[u64]); 7] = [
        (
            "struct",
            "w ≔ W { x: mk(&log, 1u64), y: mk(&log, 2u64) };",
            &[1, 2],
        ),
        (
            "partial",
            "w ≔ W { x: mk(&log, 1u64), y: mk(&log, 2u64) }; consume(w.x); z ≔ mk(&log, 3u64);",
            &[1, 3, 2],
        ),
        (
            "own_then_fields",
            "p ≔ P { log: &log, inner: mk(&log, 4u64), id: 5u64 };",
            &[5, 4],
        ),
        ("tuple", "t ≔ (mk(&log, 1u64), mk(&log, 2u64));", &[1, 2]),
        (
            "array",
            "a ≔ [mk(&log, 1u64), mk(&log, 2u64), mk(&log, 3u64)];",
            &[1, 2, 3],
        ),
        (
            "option",
            "o ≔ Some(mk(&log, 6u64)); n: Option<D> ≔ None;",
            &[6],
        ),
        (
            "option_moved_out",
            "o ≔ Some(mk(&log, 6u64)); ⌥ o { Some(d) ↦ consume(d), None ↦ { } } z ≔ mk(&log, 9u64);",
            &[6, 9],
        ),
    ];
    for (tag, body, ids) in cases {
        let src = format!(
            "{types}ƒ main() → u64 ! ∅ {{\n  log ≔ Cell::new(0u64);\n  {{\n{body}\n  }};\n  log.get()\n}}\n"
        );
        for (backend, got) in run(&src, tag) {
            assert_eq!(got, Some(seq(ids)), "{tag} ({backend})\n{src}");
        }
    }
}

#[test]
fn a_returned_value_is_dropped_by_the_caller_not_the_callee() {
    let src = "ƒ make<'a>(l: &'a Cell<u64>) → D<'a> ! ∅ { d ≔ mk(l, 5u64); e ≔ mk(l, 6u64); d }\n\
               ƒ main() → u64 ! ∅ {\n  log ≔ Cell::new(0u64);\n  {\n    x ≔ make(&log);\n    y ≔ mk(&log, 7u64);\n  };\n  log.get()\n}\n";
    for (backend, got) in run(src, "ret") {
        assert_eq!(got, Some(seq(&[6, 7, 5])), "({backend})\n{src}");
    }
}

// ======================= ORACLE ngẫu nhiên =======================
// Chương trình sinh ngẫu nhiên (khối lồng, move toàn phần/có điều kiện/một phần trường, gán đè, biến tạm, vòng lặp có move theo vòng) với
// MÔ HÌNH Rust độc lập: mô hình biết đường đi thật (điều kiện `opq(true|false)` hoặc `i == m`), mô phỏng phạm vi và tính dãy drop. Điều kiện
// đi qua lời gọi `opq` vì bước hạ MIR CẮT nhánh `⎇ true/false` literal (khi đó không có move có điều kiện nào, cờ drop không được thử —
// đột biến tự động lượt 2 lộ ra điều này). Bộ sinh chỉ
// dùng lại biến CHẮC CHẮN còn khởi tạo theo luật tĩnh (biến bị move có điều kiện thì không dùng nữa, trừ gán đè), để gritc chấp nhận.

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
enum Var {
    /// (tên, id hiện tại nếu còn khởi tạo lúc chạy, dùng lại được theo luật tĩnh?)
    D(String, Option<u64>, bool),
    /// struct `W { x, y }` (không dùng lại sau khi tạo): (id x, id y) nếu còn khởi tạo
    W(Option<u64>, Option<u64>),
    /// `P` (có drop riêng): (id của P, id của trường inner)
    P(u64, u64),
}

struct Gen {
    r: Rng,
    src: String,
    log: Vec<u64>,
    next_id: u64,
    next_var: u32,
    scopes: Vec<Vec<Var>>,
    stats: [u32; 8],
}

impl Gen {
    fn id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
    /// biểu thức tạo một D: lời gọi (`Call`, đích là chỗ) hoặc literal struct (`Assign`)
    fn make(&mut self, id: u64) -> String {
        if self.r.below(2) == 0 {
            format!("mk(&log, {id}u64)")
        } else {
            format!("D {{ log: &log, id: {id}u64 }}")
        }
    }
    fn ind(&self) -> String {
        "  ".repeat(self.scopes.len() + 1)
    }
    fn end_scope(&mut self) {
        let vars = self.scopes.pop().unwrap_or_default();
        for v in vars.iter().rev() {
            match v {
                Var::D(_, Some(id), _) => self.log.push(*id),
                Var::W(x, y) => {
                    // W không có drop riêng: glue = trường theo thứ tự khai báo
                    if let Some(x) = x {
                        self.log.push(*x);
                    }
                    if let Some(y) = y {
                        self.log.push(*y);
                    }
                }
                Var::D(_, None, _) => {}
                Var::P(idp, inner) => self.log.extend([*idp, *inner]),
            }
        }
    }
    /// chọn một biến D dùng lại được (tĩnh) và còn khởi tạo: (tầng, chỉ số)
    fn pick_d(&mut self) -> Option<(usize, usize)> {
        let all: Vec<(usize, usize)> = self
            .scopes
            .iter()
            .enumerate()
            .flat_map(|(s, vs)| {
                vs.iter()
                    .enumerate()
                    .filter(|(_, v)| matches!(v, Var::D(_, Some(_), true)))
                    .map(move |(i, _)| (s, i))
            })
            .collect();
        if all.is_empty() {
            return None;
        }
        Some(all[self.r.below(all.len() as u64) as usize])
    }
    fn any_d(&mut self) -> Option<(usize, usize)> {
        let all: Vec<(usize, usize)> = self
            .scopes
            .iter()
            .enumerate()
            .flat_map(|(s, vs)| {
                vs.iter()
                    .enumerate()
                    // chỉ biến `v…` (khai `μ`, kiểu D); `o…` là `Option<D>` không gán đè được
                    .filter(|(_, v)| matches!(v, Var::D(n, _, _) if n.starts_with('v')))
                    .map(move |(i, _)| (s, i))
            })
            .collect();
        if all.is_empty() {
            return None;
        }
        Some(all[self.r.below(all.len() as u64) as usize])
    }
    fn stmt(&mut self, depth: u32) {
        let ind = self.ind();
        match self.r.below(13) {
            0..=2 => {
                let (id, n) = (self.id(), self.next_var);
                self.next_var += 1;
                let init = self.make(id);
                self.src.push_str(&format!("{ind}μv{n} ≔ {init};\n"));
                if let Some(s) = self.scopes.last_mut() {
                    s.push(Var::D(format!("v{n}"), Some(id), true));
                }
            }
            3 => {
                if let Some((s, i)) = self.pick_d()
                    && let Var::D(name, Some(id), _) = self.scopes[s][i].clone()
                {
                    self.src.push_str(&format!("{ind}consume({name});\n"));
                    self.log.push(id);
                    self.scopes[s][i] = Var::D(name, None, false);
                    self.stats[0] += 1;
                }
            }
            4 => {
                // move CÓ ĐIỀU KIỆN: sau đó không dùng lại được (tĩnh), nhưng mô hình biết đường thật
                if let Some((s, i)) = self.pick_d()
                    && let Var::D(name, Some(id), _) = self.scopes[s][i].clone()
                {
                    let c = self.r.below(2) == 0;
                    let how = if self.r.below(2) == 0 {
                        format!("consume({name});")
                    } else {
                        format!("z ≔ {name};")
                    };
                    self.src
                        .push_str(&format!("{ind}⎇ opq({c}) {{ {how} }} ⇝ {{ }}\n"));
                    if c {
                        self.log.push(id);
                        self.scopes[s][i] = Var::D(name, None, false);
                    } else {
                        self.scopes[s][i] = Var::D(name, Some(id), false);
                    }
                    self.stats[1] += 1;
                }
            }
            5 => {
                // gán đè: drop giá trị cũ nếu còn (cờ nếu tuỳ đường); sau đó dùng lại được
                if let Some((s, i)) = self.any_d()
                    && let Var::D(name, old, _) = self.scopes[s][i].clone()
                {
                    let id = self.id();
                    let init = self.make(id);
                    self.src.push_str(&format!("{ind}{name} ← {init};\n"));
                    if let Some(o) = old {
                        self.log.push(o);
                    }
                    self.scopes[s][i] = Var::D(name, Some(id), true);
                    self.stats[2] += 1;
                }
            }
            6 => {
                let id = self.id();
                self.src.push_str(&format!("{ind}mk(&log, {id}u64);\n"));
                self.log.push(id);
            }
            7 => {
                let (x, y, n) = (self.id(), self.id(), self.next_var);
                self.next_var += 1;
                self.src.push_str(&format!(
                    "{ind}μw{n} ≔ W {{ n: 0u64, x: mk(&log, {x}u64), y: mk(&log, {y}u64) }};\n"
                ));
                let mv = self.r.below(3);
                let (mut xs, mut ys) = (Some(x), Some(y));
                if mv == 1 {
                    self.src.push_str(&format!("{ind}consume(w{n}.x);\n"));
                    self.log.push(x);
                    xs = None;
                    self.stats[3] += 1;
                } else if mv == 2 {
                    let c = self.r.below(2) == 0;
                    self.src
                        .push_str(&format!("{ind}⎇ opq({c}) {{ consume(w{n}.y); }} ⇝ {{ }}\n"));
                    if c {
                        self.log.push(y);
                        ys = None;
                    }
                    self.stats[3] += 1;
                }
                if let Some(s) = self.scopes.last_mut() {
                    s.push(Var::W(xs, ys));
                }
            }
            10 => {
                // kiểu có `drop` riêng: gán đè một trường (cây move path có con) vẫn phải drop CẢ giá trị (drop riêng rồi trường)
                let (a, idp, c, n) = (self.id(), self.id(), self.id(), self.next_var);
                self.next_var += 1;
                self.src.push_str(&format!(
                    "{ind}μp{n} ≔ P {{ log: &log, inner: mk(&log, {a}u64), id: {idp}u64 }};\n{ind}p{n}.inner ← mk(&log, {c}u64);\n"
                ));
                self.log.push(a);
                if let Some(s) = self.scopes.last_mut() {
                    s.push(Var::P(idp, c));
                }
                self.stats[6] += 1;
            }
            11 => {
                // tham số có Drop bị move CÓ ĐIỀU KIỆN trong hàm nhận (cờ của THAM SỐ, đầu hay cuối danh sách)
                let (id, k) = (self.id(), self.id());
                let c = self.r.below(2) == 0;
                let call = if self.r.below(2) == 0 {
                    format!("maybe_first(mk(&log, {id}u64), &log, opq({c}), {k}u64);")
                } else {
                    format!("maybe_last(&log, opq({c}), {k}u64, mk(&log, {id}u64));")
                };
                self.src.push_str(&format!("{ind}{call}\n"));
                if c {
                    self.log.extend([id, k]);
                } else {
                    self.log.extend([k, id]);
                }
                self.stats[7] += 1;
            }
            8 if depth < 3 => {
                self.src.push_str(&format!("{ind}{{\n"));
                self.scopes.push(Vec::new());
                for _ in 0..1 + self.r.below(4) {
                    self.stmt(depth + 1);
                }
                self.end_scope();
                self.src.push_str(&format!("{ind}}};\n"));
            }
            9 if depth < 3 => {
                // vòng: mỗi vòng tạo a, move a khi i == m, tạo b; cuối vòng drop b rồi a (nếu còn)
                let (base_a, base_b, k, m) = (
                    self.id() * 100,
                    self.id() * 100,
                    1 + self.r.below(4),
                    self.r.below(5),
                );
                self.src.push_str(&format!(
                    "{ind}⟳ i in 0u64..{k}u64 {{ a ≔ mk(&log, {base_a}u64 +% i); ⎇ i == {m}u64 {{ consume(a); }} ⇝ {{ }} b ≔ mk(&log, {base_b}u64 +% i); }}\n"
                ));
                for i in 0..k {
                    if i == m {
                        self.log.push(base_a + i);
                        self.log.push(base_b + i);
                    } else {
                        self.log.push(base_b + i);
                        self.log.push(base_a + i);
                    }
                }
                self.stats[4] += 1;
            }
            _ => {
                // Option<D>: Some rồi có thể move ra bằng ⌥ (enum bị move một phần ⇒ drop mở theo thẻ)
                let (id, n) = (self.id(), self.next_var);
                self.next_var += 1;
                let mode = self.r.below(3);
                self.src
                    .push_str(&format!("{ind}o{n} ≔ Some(mk(&log, {id}u64));\n"));
                let m = format!("⌥ o{n} {{ Some(d) ↦ consume(d), None ↦ {{ }} }}");
                let out = match mode {
                    0 => false,
                    1 => {
                        self.src.push_str(&format!("{ind}{m}\n"));
                        true
                    }
                    _ => {
                        // move CÓ ĐIỀU KIỆN ra khỏi enum: trên đường còn lại phải drop phần tải qua thẻ
                        let c = self.r.below(2) == 0;
                        self.src
                            .push_str(&format!("{ind}⎇ opq({c}) {{ {m} }} ⇝ {{ }}\n"));
                        c
                    }
                };
                if out {
                    self.log.push(id);
                }
                if mode != 0 {
                    self.stats[5] += 1;
                }
                if let Some(s) = self.scopes.last_mut() {
                    // mô hình hoá như D (một id, tĩnh không dùng lại)
                    s.push(Var::D(format!("o{n}"), (!out).then_some(id), false));
                }
            }
        }
    }
}

fn random_program(seed: u64) -> (String, Vec<u64>, [u32; 8]) {
    let mut g = Gen {
        r: Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1),
        src: String::from(
            "▢ W<'a> { n: u64, x: D<'a>, y: D<'a> }\n\
             ▢ P<'a> { log: &'a Cell<u64>, inner: D<'a>, id: u64 }\n\
             ⊐ <'a> Drop for P<'a> { ƒ drop(&!self) ! ∅ { self.log.set(self.log.get() *% 31u64 +% self.id); } }\n\
             ƒ maybe_first<'a>(d: D<'a>, l: &'a Cell<u64>, c: bool, k: u64) ! ∅ { ⎇ c { consume(d); } ⇝ { } e ≔ mk(l, k); }\n\
             ƒ maybe_last<'a>(l: &'a Cell<u64>, c: bool, k: u64, d: D<'a>) ! ∅ { ⎇ c { consume(d); } ⇝ { } e ≔ mk(l, k); }\n\
             ƒ opq(x: bool) → bool ! ∅ { x }\n\
             ƒ main() → u64 ! ∅ {\n  log ≔ Cell::new(0u64);\n  {\n",
        ),
        log: Vec::new(),
        next_id: 0,
        next_var: 0,
        scopes: vec![Vec::new()],
        stats: [0; 8],
    };
    for _ in 0..4 + g.r.below(8) {
        g.stmt(0);
    }
    g.end_scope();
    g.src.push_str("  };\n  log.get()\n}\n");
    (g.src, g.log, g.stats)
}

#[test]
fn random_programs_drop_exactly_like_an_independent_scope_model_on_both_backends() {
    let mut tally = [0u32; 8];
    for seed in 1..=60u64 {
        let (src, ids, st) = random_program(seed);
        for (k, v) in st.iter().enumerate() {
            tally[k] += v;
        }
        for (backend, got) in run(&src, &format!("rnd{seed}")) {
            assert_eq!(
                got,
                Some(seq(&ids)),
                "#{seed} ({backend}): dãy drop lệch mô hình {ids:?}\n{src}"
            );
        }
    }
    // chống bộ sinh rỗng: mỗi loại move phải xuất hiện đủ
    let names = [
        "move",
        "move có điều kiện",
        "gán đè",
        "move một phần W",
        "vòng",
        "move khỏi Option",
        "gán đè trường của kiểu có drop",
        "tham số move có điều kiện",
    ];
    for (k, n) in names.iter().enumerate() {
        assert!(tally[k] >= 8, "bộ sinh quá ít ca `{n}`: {tally:?}");
    }
}
