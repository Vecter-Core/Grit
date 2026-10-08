//! ORACLE ĐỘNG (ADR-024): thông dịch MIR có theo dõi khoản vay/move/Arena, độc lập với phân tích của borrowck.
//! (1) kiểm chính oracle trên chương trình viết tay, (2) hiệu chuẩn trên corpus, (3) chiến dịch ngẫu nhiên có ĐỘ PHỦ theo
//! loại cấu trúc, (4) chiến dịch "tấn công" (đột biến ngữ nghĩa, kiểm cả hai phía).
mod common;
mod support;
use common::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use support::oracle::{Oracle, Outcome, V, VKind};
use support::progen::{self, Attack, Gen, KINDS, Prog};

const FUEL: u64 = 300_000;

/// Kết quả của một chương trình: mã borrowck + kết quả oracle ở `c = false` và `c = true`.
struct Eval {
    bc: Vec<String>,
    runs: Vec<Outcome>,
    loans: usize,
}

impl Eval {
    fn accepted(&self) -> bool {
        self.bc.is_empty()
    }
    fn violation(&self) -> Option<VKind> {
        self.runs.iter().find_map(|o| match o {
            Outcome::Violation(v) => Some(v.kind),
            _ => None,
        })
    }
    fn bug(&self) -> Option<&str> {
        self.runs.iter().find_map(|o| match o {
            Outcome::Bug(m) => Some(m.as_str()),
            _ => None,
        })
    }
}

/// `Err` = chương trình không qua M4/M5 (lỗi của bộ sinh, không phải của borrowck).
fn evaluate(src: &str, args_sets: &[Vec<V>], key: &str) -> Result<Eval, String> {
    let t = one(src);
    if !t.report.diagnostics.is_empty() {
        return Err(format!("M4: {:?}", show(&t)));
    }
    let m = gt_mir::lower_program(&t);
    if !m.report.diagnostics.is_empty() {
        return Err("M5 báo lỗi".into());
    }
    let r = gt_borrowck::check_program(&t, &m);
    let mut bc: Vec<String> = r
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    bc.sort();
    bc.dedup();
    let mut runs = Vec::new();
    let mut loans = 0;
    for a in args_sets {
        let mut o = Oracle::new(&m, FUEL);
        runs.push(o.call_key(key, a.clone()));
        loans += o.stats.loans;
    }
    Ok(Eval { bc, runs, loans })
}

fn entry(src: &str) -> Result<Eval, String> {
    // 8 tổ hợp của (c, d, e): mọi nhánh lồng nhau đều được chạy; tổ hợp đầu (toàn false) là cái `run1` lấy
    let sets: Vec<Vec<V>> = (0..8u8)
        .map(|m| (0..3).map(|i| V::Bool((m >> i) & 1 == 1)).collect())
        .collect();
    evaluate(src, &sets, "main::entry")
}

// ===================================================================== (1) chính oracle

fn run1(body: &str) -> Outcome {
    let src = format!(
        "{}ƒ entry(c: bool, d: bool, e: bool) → i32 ! {{mem}} {{\n{body}\n  0\n}}\n",
        progen::PRELUDE
    );
    let e = entry(&src).expect("qua M4/M5");
    // lấy kết quả `c = false`
    e.runs.into_iter().next().expect("một lần chạy")
}

fn kind_of(body: &str) -> Option<VKind> {
    match run1(body) {
        Outcome::Violation(v) => Some(v.kind),
        Outcome::Bug(m) => panic!("oracle gặp MIR ngoài phạm vi: {m}"),
        _ => None,
    }
}

#[test]
fn oracle_sees_each_kind_of_violation_and_accepts_the_valid_twins() {
    // (chương trình, loại vi phạm mong đợi) — mỗi vi phạm đi kèm một cặp sinh đôi HỢP LỆ
    let bad = [
        (
            "μx ≔ 1; r ≔ &x; x ← 2; use_i(*r);",
            VKind::WriteWhileBorrowed,
        ),
        (
            "μx ≔ 1; r ≔ &x; y ≔ &!x; *y ← 2; use_i(*r);",
            VKind::UniqueVsShared,
        ),
        (
            "μx ≔ 1; a ≔ &!x; b ≔ &!x; *b ← 2; *a ← 3;",
            VKind::UniqueVsUnique,
        ),
        ("μx ≔ 1; a ≔ &!x; use_i(x); *a ← 3;", VKind::UniqueVsShared),
        ("b ≔ mk(); eat(b); use_i(b.v);", VKind::UseAfterMove),
        (
            "μa ≔ Arena::new(); p ≔ a.alloc(1); a.reset(); use_i(*p);",
            VKind::ArenaEscape,
        ),
        (
            "μa ≔ Arena::new(); p ≔ a.alloc(1); eata(a); use_i(*p);",
            VKind::ArenaEscape,
        ),
        ("μp ≔ &0; { y ≔ 5; p ← &y; }; use_i(*p);", VKind::Dangling),
        (
            "μu ≔ 1; a ≔ &!u; v ≔ &!*a; *a ← 1; *v ← 2;",
            VKind::WriteWhileBorrowed,
        ),
    ];
    for (body, want) in bad {
        assert_eq!(kind_of(body), Some(want), "{body}");
    }
    let good = [
        "μx ≔ 1; r ≔ &x; y ≔ *r; x ← 2; use_i(y);",
        "μx ≔ 1; r ≔ &x; use_i(*r); y ≔ &!x; *y ← 2;",
        "μx ≔ 1; a ≔ &!x; *a ← 2; b ≔ &!x; *b ← 3;",
        "μx ≔ 1; a ≔ &!x; *a ← 3; use_i(x);",
        "b ≔ mk(); eat(b);",
        "μa ≔ Arena::new(); p ≔ a.alloc(1); use_i(*p); a.reset();",
        "μa ≔ Arena::new(); p ≔ a.alloc(1); use_i(*p); eata(a);",
        "μp ≔ &0; μq ≔ 5; p ← &q; use_i(*p);",
        "μu ≔ 1; a ≔ &!u; v ≔ &!*a; *v ← 2; *a ← 1;",
        "μb ≔ mk(); eat(b); b ← mk(); use_i(b.v);",
        "μw ≔ mkw(); x ≔ w.b; use_i(w.n); eat(x);",
        "μa ≔ [1, 2]; r ≔ &a[0]; a[1] ← 5; use_i(*r);", // oracle chính xác theo chỉ số (borrowck thì từ chối)
    ];
    for body in good {
        assert_eq!(kind_of(body), None, "{body}");
    }
}

#[test]
fn oracle_flags_partial_moves_moves_out_of_references_and_dangling_returns() {
    assert_eq!(
        kind_of("μw ≔ mkw(); x ≔ w.b; eatw(w);"),
        Some(VKind::UseAfterMove)
    );
    assert_eq!(
        kind_of("μw ≔ mkw(); x ≔ w.b; w.b.v ← 1;"),
        Some(VKind::UseAfterMove)
    );
    assert_eq!(
        kind_of("μw ≔ mkw(); x ≔ w.b; y ≔ w.b;"),
        Some(VKind::UseAfterMove)
    );
    // dùng lại trường CHƯA move: hợp lệ
    assert_eq!(kind_of("μw ≔ mkw(); x ≔ w.b; use_i(w.n);"), None);
    // move trong vòng lặp: lần lặp thứ hai dùng lại giá trị đã move
    assert_eq!(
        kind_of("b ≔ mk(); ⟳ i in 0..2 { eat(b); }"),
        Some(VKind::UseAfterMove)
    );
}

// ===================================================================== (2) hiệu chuẩn trên corpus

fn corpus(kind: &str) -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join(kind);
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("thư mục")
        .map(|e| e.expect("e").path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "gt"))
        .collect();
    v.sort();
    v
}

/// Chạy `f` của mẫu nếu mọi tham số của nó là `bool`.
fn run_sample_f(src: &str) -> Option<Vec<Outcome>> {
    let t = one(src);
    if !t.report.diagnostics.is_empty() {
        return None;
    }
    let m = gt_mir::lower_program(&t);
    let b = m.body("main::f")?;
    let nb = b.arg_count;
    if (1..=nb).any(|i| !matches!(b.locals[i].ty, gt_types::Ty::Bool)) || nb > 2 {
        return None;
    }
    let sets: Vec<Vec<V>> = match nb {
        0 => vec![vec![]],
        1 => vec![vec![V::Bool(false)], vec![V::Bool(true)]],
        _ => vec![
            vec![V::Bool(false), V::Bool(false)],
            vec![V::Bool(false), V::Bool(true)],
            vec![V::Bool(true), V::Bool(false)],
            vec![V::Bool(true), V::Bool(true)],
        ],
    };
    Some(
        sets.into_iter()
            .map(|a| Oracle::new(&m, FUEL).call_key("main::f", a))
            .collect(),
    )
}

#[test]
fn oracle_is_calibrated_on_the_corpus() {
    let read = |p: &Path| {
        std::fs::read_to_string(p)
            .expect("đọc")
            .replace("\r\n", "\n")
    };
    // mẫu ĐÚNG: oracle không được thấy vi phạm (nếu có = lỗi oracle hoặc mẫu sai)
    let (mut good_run, mut good_bad) = (0, Vec::new());
    for p in corpus("pass") {
        let name = p.file_stem().expect("n").to_string_lossy().to_string();
        if !name.contains("_bc_") {
            continue;
        }
        let Some(runs) = run_sample_f(&read(&p)) else {
            continue;
        };
        good_run += 1;
        for o in runs {
            match o {
                Outcome::Violation(v) => good_bad.push(format!("{name}: {v:?}")),
                Outcome::Bug(m) => good_bad.push(format!("{name}: oracle Bug {m}")),
                _ => {}
            }
        }
    }
    assert!(
        good_bad.is_empty(),
        "oracle thấy vi phạm ở mẫu ĐÚNG:\n{}",
        good_bad.join("\n")
    );
    // mẫu XẤU: thống kê oracle thấy được bao nhiêu (phần còn lại là quy tắc tĩnh/chữ ký/mẫu không chạy được)
    let (mut bad_run, mut seen, mut missed) = (0, 0, Vec::new());
    let mut by_code: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for p in corpus("fail") {
        let name = p.file_stem().expect("n").to_string_lossy().to_string();
        if !name.starts_with("E04") {
            continue;
        }
        let Some(runs) = run_sample_f(&read(&p)) else {
            continue;
        };
        bad_run += 1;
        let e = by_code.entry(name[..5].to_string()).or_default();
        e.0 += 1;
        let viol = runs.iter().any(|o| matches!(o, Outcome::Violation(_)));
        if runs.iter().any(|o| matches!(o, Outcome::Bug(_))) {
            missed.push(format!("{name}: oracle Bug"));
        }
        if viol {
            seen += 1;
            e.1 += 1;
        } else {
            missed.push(name);
        }
    }
    eprintln!(
        "hiệu chuẩn: mẫu đúng chạy được {good_run} (0 vi phạm); mẫu xấu chạy được {bad_run}, oracle thấy {seen}"
    );
    eprintln!("theo mã (chạy được, thấy): {by_code:?}");
    eprintln!("oracle KHÔNG thấy: {missed:?}");
    assert!(good_run >= 100, "chỉ {good_run} mẫu đúng chạy được");
    assert!(bad_run >= 80, "chỉ {bad_run} mẫu xấu chạy được");
    assert!(
        seen * 100 >= bad_run * 80,
        "oracle chỉ thấy {seen}/{bad_run} mẫu xấu — oracle yếu"
    );
}

// ===================================================================== (3) chiến dịch ngẫu nhiên

/// ADR-024 §2: ngưỡng độ phủ. Mỗi loại cấu trúc phải có ≥ này chương trình mà borrowck CHẤP NHẬN và oracle chạy SẠCH
/// (nơi âm tính giả sẽ lộ ra) VÀ ≥ này chương trình oracle thấy vi phạm (độ nhạy của oracle trên loại đó).
/// Lý do: quy tắc "ba" — với n ≥ 300 mẫu, một lỗi có tỉ lệ ≥ 1% bị bỏ sót với xác suất < 5%.
const THRESHOLD: usize = 300;
const PROGRAMS: u64 = 20_000;

/// `GT_ORACLE_PROGRAMS=n` chạy bản NHANH (đột biến mã nguồn dùng nó); chỉ bản đầy đủ mới kiểm ngưỡng độ phủ.
fn programs() -> u64 {
    std::env::var("GT_ORACLE_PROGRAMS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(PROGRAMS)
}

fn quick() -> bool {
    programs() != PROGRAMS
}

#[derive(Default, Clone, Copy)]
struct Cov {
    total: usize,
    accepted_clean: usize,
    violated: usize,
}

#[test]
fn random_programs_agree_between_borrowck_and_the_dynamic_oracle() {
    let mut cov: BTreeMap<&str, Cov> = KINDS.iter().map(|k| (*k, Cov::default())).collect();
    let (mut acc_clean, mut acc_viol, mut rej_viol, mut rej_clean) =
        (0usize, Vec::new(), 0usize, 0usize);
    let mut rej_clean_by: BTreeMap<String, usize> = BTreeMap::new();
    let mut rej_clean_cat: BTreeMap<&str, usize> = BTreeMap::new();
    let mut rej_clean_examples: Vec<(String, Vec<String>)> = Vec::new();
    let mut code_disagree: Vec<String> = Vec::new();
    let mut harness: Vec<String> = Vec::new();
    let mut loans_total = 0usize;
    for seed in 1..=programs() {
        let mut g = Gen::new(seed);
        let prog = g.program();
        let src = prog.source();
        let e = match entry(&src) {
            Ok(e) => e,
            Err(why) => {
                harness.push(format!("#{seed}: bộ sinh sai: {why}\n{src}"));
                continue;
            }
        };
        if let Some(b) = e.bug() {
            harness.push(format!("#{seed}: oracle Bug: {b}\n{src}"));
            continue;
        }
        loans_total += e.loans;
        let viol = e.violation();
        for k in &prog.tags {
            let c = cov.get_mut(k).expect("loại");
            c.total += 1;
            if e.accepted() && viol.is_none() {
                c.accepted_clean += 1;
            }
            if viol.is_some() {
                c.violated += 1;
            }
        }
        match (e.accepted(), viol) {
            (true, None) => acc_clean += 1,
            (true, Some(k)) => acc_viol.push(format!(
                "#{seed} ({k:?}): borrowck CHẤP NHẬN nhưng oracle thấy vi phạm\n{src}"
            )),
            (false, Some(k)) => {
                rej_viol += 1;
                if !e.bc.iter().any(|c| c == k.code()) {
                    code_disagree.push(format!(
                        "#{seed}: oracle {} ≠ borrowck {:?}",
                        k.code(),
                        e.bc
                    ));
                }
            }
            (false, None) => {
                rej_clean += 1;
                *rej_clean_by.entry(e.bc.join(",")).or_default() += 1;
                // phân loại theo NGUYÊN NHÂN KHÔNG-ĐẦY-ĐỦ đã biết (ADR-023 §1, §6, §11)
                let body = src.replace(progen::PRELUDE, "");
                let cat = if body.contains("pick(")
                    || body.contains("pick2(")
                    || body.contains("first(")
                {
                    "chữ ký RegionSig (kết quả buộc với MỌI đối số cùng tên region; oracle chỉ thấy thân thật)"
                } else if body.contains("Cell::new") && body.contains(".set(&z") {
                    "mutability bên trong: kiểu `Cell<&'a T>` buộc mọi tham chiếu lưu vào sống tới lần dùng cuối của ô, kể cả khi sau đó ghi đè (Rust cũng từ chối)"
                } else if body.contains("Sl { s: &!") {
                    "ADT bất biến `Sl<'a>`: `&!'a &'a` ghim `p` suốt `'a` (Rust cũng từ chối dùng `p` sau đó)"
                } else if body.contains("≔ (&") || body.contains("≔ [&") {
                    "liveness theo BIẾN: vật chứa giữ nhiều tham chiếu, mọi region trong kiểu của nó cùng sống (NLL, Rust cũng vậy)"
                } else if body.contains("[0] ←") || body.contains("[1] ←") {
                    "chỉ số mảng (borrowck coi mọi chỉ số chồng lấn)"
                } else {
                    "KHÁC (cần xem tay)"
                };
                *rej_clean_cat.entry(cat).or_default() += 1;
                if cat.starts_with("KHÁC") && rej_clean_examples.len() < 40 {
                    rej_clean_examples.push((src.clone(), e.bc.clone()));
                }
            }
        }
    }
    let clean_total = acc_clean + rej_clean;
    eprintln!(
        "== {} chương trình ngẫu nhiên; khoản vay đã theo dõi: {loans_total}",
        programs()
    );
    eprintln!("borrowck chấp nhận & oracle sạch: {acc_clean}");
    eprintln!(
        "borrowck chấp nhận & oracle thấy vi phạm (ÂM TÍNH GIẢ): {}",
        acc_viol.len()
    );
    eprintln!(
        "borrowck từ chối & oracle thấy vi phạm: {rej_viol} (khác mã: {})",
        code_disagree.len()
    );
    eprintln!(
        "borrowck từ chối & oracle sạch (ứng viên dương tính giả): {rej_clean} = {:.2}% của {clean_total} chương trình oracle sạch",
        100.0 * rej_clean as f64 / clean_total.max(1) as f64
    );
    eprintln!("ứng viên dương tính giả theo mã: {rej_clean_by:?}");
    eprintln!("ứng viên dương tính giả theo nguyên nhân: {rej_clean_cat:#?}");
    for k in KINDS {
        let c = cov[k];
        eprintln!(
            "độ phủ {k:>18}: tổng {:>6}, chấp nhận+sạch {:>6}, oracle thấy vi phạm {:>6}",
            c.total, c.accepted_clean, c.violated
        );
    }
    for (i, (s, codes)) in rej_clean_examples.iter().enumerate().take(0) {
        eprintln!("--- ví dụ {i} {codes:?}\n{s}");
    }
    if let Ok(dir) = std::env::var("GT_FP_DUMP") {
        let mut all = String::new();
        for (i, (s, codes)) in rej_clean_examples.iter().enumerate() {
            all.push_str(&format!(
                "//// ví dụ {i} {codes:?}\n{}\n",
                s.trim_start_matches(progen::PRELUDE)
            ));
        }
        std::fs::write(Path::new(&dir).join("fp_examples.txt"), all).expect("ghi");
    }

    // mọi ứng viên dương tính giả phải thuộc một nguyên nhân KHÔNG-ĐẦY-ĐỦ đã biết (nhóm "KHÁC" = ứng viên lỗi)
    let other = rej_clean_cat
        .iter()
        .find(|(c, _)| c.starts_with("KHÁC"))
        .map_or(0, |(_, n)| *n);
    assert_eq!(
        other,
        0,
        "ứng viên dương tính giả CHƯA giải thích được:
{}",
        rej_clean_examples
            .iter()
            .take(3)
            .map(|(s, c)| format!(
                "{c:?}
{s}"
            ))
            .collect::<Vec<_>>()
            .join(
                "
====
"
            )
    );
    assert!(
        harness.is_empty(),
        "{}",
        harness
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n====\n")
    );
    assert!(
        acc_viol.is_empty(),
        "ÂM TÍNH GIẢ THẬT ({}):\n{}",
        acc_viol.len(),
        acc_viol
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n====\n")
    );
    // đường dự phòng (chốt chặn phòng thủ trong regions.rs) không bao giờ được dùng cho chương trình qua M4
    assert_eq!(gt_borrowck::defensive_counters(), (0, 0));
    for k in KINDS.iter().filter(|_| !quick()) {
        let c = cov[k];
        assert!(
            c.accepted_clean >= THRESHOLD,
            "{k}: chỉ {} chương trình chấp nhận+sạch (< {THRESHOLD}) — CHƯA được tính là đã kiểm",
            c.accepted_clean
        );
        assert!(
            c.violated >= THRESHOLD,
            "{k}: oracle chỉ thấy vi phạm ở {} chương trình (< {THRESHOLD})",
            c.violated
        );
    }
}

// ===================================================================== (4) tấn công

#[test]
fn semantic_attacks_are_caught_by_both_sides() {
    let want_per_attack: usize = if quick() { 40 } else { 400 };
    let mut done: BTreeMap<Attack, usize> = BTreeMap::new();
    let mut disagree: Vec<String> = Vec::new();
    let mut tried = 0usize;
    let mut bases = 0usize;
    let mut seed = 1_000_000u64;
    while done.values().copied().min().unwrap_or(0) < want_per_attack || done.len() < 3 {
        seed += 1;
        if seed > 1_000_000 + 60_000 {
            break;
        }
        let mut g = Gen::new(seed);
        let base: Prog = g.program();
        let sites = progen::sites(&base);
        if sites.is_empty() {
            continue;
        }
        // chọn vị trí thiếu nhất trước (cân bằng giữa 3 loại tấn công)
        let site = sites
            .iter()
            .min_by_key(|s| (done.get(&s.attack).copied().unwrap_or(0), g.rng.below(1000)))
            .expect("không rỗng")
            .clone();
        if done.get(&site.attack).copied().unwrap_or(0) >= want_per_attack * 3 {
            continue;
        }
        // cơ sở phải HỢP LỆ ở cả hai phía
        let Ok(b) = entry(&base.source()) else {
            continue;
        };
        if !b.accepted() || b.violation().is_some() || b.bug().is_some() {
            continue;
        }
        bases += 1;
        let which = g.rng.below(site.insert.len() as u64) as usize;
        let mutant = progen::apply(&base, &site, which);
        let src = mutant.source();
        let expect = site.insert[which].1;
        tried += 1;
        let e = match entry(&src) {
            Ok(e) => e,
            Err(why) => {
                disagree.push(format!("đột biến không qua M4/M5: {why}\n{src}"));
                continue;
            }
        };
        let borrowck_ok = e.bc.iter().any(|c| expect.contains(&c.as_str()));
        let oracle_ok = e.violation().is_some();
        if borrowck_ok && oracle_ok {
            *done.entry(site.attack).or_default() += 1;
        } else {
            disagree.push(format!(
                "{:?}: mong {expect:?}; borrowck {:?}; oracle {:?}\n{src}",
                site.attack,
                e.bc,
                e.runs
                    .iter()
                    .map(|o| match o {
                        Outcome::Violation(v) => format!("{:?}", v.kind),
                        o => format!("{o:?}"),
                    })
                    .collect::<Vec<_>>()
            ));
        }
    }
    if let Ok(dir) = std::env::var("GT_FP_DUMP") {
        let all: String = disagree
            .iter()
            .map(|d| {
                format!(
                    "//// {}
",
                    d.replace(progen::PRELUDE, "")
                )
            })
            .collect();
        std::fs::write(Path::new(&dir).join("attack_disagree.txt"), all).expect("ghi");
    }
    eprintln!(
        "tấn công: cơ sở hợp lệ {bases}, đã thử {tried}, hai phía cùng bắt: {done:?}, bất đồng {}",
        disagree.len()
    );
    assert!(
        disagree.is_empty(),
        "HAI PHÍA BẤT ĐỒNG ({}):\n{}",
        disagree.len(),
        disagree
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n====\n")
    );
    for a in [
        Attack::UseAfterMove,
        Attack::SecondBorrow,
        Attack::ArenaReset,
    ] {
        assert!(
            done.get(&a).copied().unwrap_or(0) >= want_per_attack * 3 / 4,
            "{a:?}: chỉ {:?} tấn công được kiểm",
            done.get(&a)
        );
    }
}
