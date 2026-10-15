#![cfg(feature = "mutant-probe")]
//! ĐỐI CHIẾU PHÁN QUYẾT từng chương trình giữa bản gốc và các đột biến 361/591/554 của `regions.rs` (lệnh Giang 2026-10-14, A.3), dùng feature `mutant-probe`
//! (MỘT binary tính cả hai phán quyết). Chạy: `cargo test -p gt-borrowck --features mutant-probe --test probe_diff -- --nocapture`;
//! đủ ngưỡng 20 000 chương trình: `GT_PROBE_N=20000` (mặc định nhỏ để cổng chạy nhanh).
//!
//! Mỗi đột biến được báo: (i) bản đột biến CHẤP NHẬN mà bản gốc TỪ CHỐI (PHẢI bằng 0 cho 361/591/554; đột biến 238 là ĐỐI CHỨNG DƯƠNG, PHẢI > 0 trên
//! kịch bản `interior_mut` — nếu không thì chính phép đo mù); (ii) bản gốc chấp nhận mà đột biến từ chối (tập chênh); (iii) oracle động trên
//! tập chênh bằng bản gốc: mọi vi phạm là âm tính giả THẬT của bản gốc; (iv) các chương trình mẫu của tập chênh ghi ra tệp để đọc tay.
mod common;
mod support;
use common::*;
use gt_borrowck::{Mutant, check_program_probe, probe_constraint_edges};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use support::oracle::{Oracle, Outcome, V};
use support::probegen;
use support::progen::{self, Gen};

const FUEL: u64 = 300_000;
/// các thân chương trình tấn công (từ `interior_mut.rs`): nhét tham chiếu sống ngắn vào ô sống lâu
const ATTACKS: [&str; 8] = [
    "μlong ≔ 1; cell ≔ Cell::new(&long); r ≔ &cell; { short ≔ 2; r.set(&short); }; use_i(*cell.get());",
    "μlong ≔ 1; cell ≔ Cell::new(&long); cl ≔ Cl { c: &cell }; { short ≔ 2; cl2 ≔ cl; cl2.c.set(&short); }; use_i(*cell.get());",
    "μlong ≔ 1; cell ≔ Cell::new(&long); cl ≔ Cl { c: &cell }; { short ≔ 2; cl2 ≔ idg(cl); cl2.c.set(&short); }; use_i(*cell.get());",
    "μlong ≔ 1; cell ≔ Cell::new(&long); t ≔ (&cell, 0); { short ≔ 2; t2 ≔ t; t2.0.set(&short); }; use_i(*cell.get());",
    "μlong ≔ 1; cell ≔ Cell::new(&long); o ≔ Some(&cell); { short ≔ 2; ⌥ o { Some(q) ↦ { q.set(&short); }, None ↦ { } }; }; use_i(*cell.get());",
    "μlong ≔ 1; cell ≔ Cell::new(&long); a ≔ [&cell, &cell]; { short ≔ 2; a[0].set(&short); }; use_i(*cell.get());",
    "μlong ≔ 1; cell ≔ Cell::new(&long); { short ≔ 2; cell.set(&short); cell.set(&long); }; use_i(*cell.get());",
    "μlong ≔ 1; cell ≔ Cell::new(&long); r ≔ &cell; rr ≔ &r; { short ≔ 2; rr.set(&short); }; use_i(*cell.get());",
];

const MUTANTS: [(Mutant, &str); 4] = [
    (Mutant::M361, "361"),
    (Mutant::M591, "591"),
    (Mutant::M554, "554"),
    (Mutant::M238, "238 (đối chứng dương, LỎNG hơn)"),
];

#[derive(Default)]
struct Stat {
    /// tổng chương trình đã tính phán quyết
    total: u64,
    /// bản gốc chấp nhận
    orig_accept: u64,
    /// (i) đột biến chấp nhận, gốc từ chối
    mut_accepts_orig_rejects: u64,
    /// (ii) gốc chấp nhận, đột biến từ chối
    orig_accepts_mut_rejects: u64,
    /// (iii) trong tập (ii): oracle thấy vi phạm / chạy hết sạch / ngoài phạm vi
    oracle_violation: u64,
    oracle_clean: u64,
    oracle_other: u64,
    /// mẫu của (ii) để đọc tay và mẫu của (i)
    samples_ii: Vec<(String, String)>,
    samples_i: Vec<(String, String)>,
    violations: Vec<(String, String)>,
}

#[derive(Default)]
struct Report {
    /// theo loại cấu trúc (khoá = tên loại) → [mỗi đột biến] Stat
    by_kind: BTreeMap<String, Vec<Stat>>,
    skipped: BTreeMap<String, u64>,
    /// kiểm tiền đề: số chương trình đã so tập cạnh, và số chương trình mà tập cạnh gốc KHÔNG là tập con của đột biến chặt hơn
    edge_checked: u64,
    edge_violations: Vec<String>,
    /// số cạnh THÊM của mỗi đột biến so với gốc (tổng) — chứng tỏ đột biến có tác dụng ở mức ràng buộc
    edge_added: [u64; 4],
}

fn entry_sets() -> Vec<Vec<V>> {
    (0..8u8)
        .map(|m| (0..3).map(|i| V::Bool((m >> i) & 1 == 1)).collect())
        .collect()
}

impl Report {
    fn stats(&mut self, kind: &str) -> &mut Vec<Stat> {
        self.by_kind
            .entry(kind.to_string())
            .or_insert_with(|| (0..MUTANTS.len()).map(|_| Stat::default()).collect())
    }

    /// Phán quyết của bản gốc và mọi đột biến cho một chương trình đã qua M4/M5; kết quả được cộng vào MỖI loại trong `kinds`.
    /// `oracle`: có chạy oracle động trên tập chênh không (chỉ cho chương trình có `entry(c, d, e)`).
    fn judge(&mut self, kinds: &[String], name: &str, src: &str, oracle: bool) {
        let first = kinds.first().cloned().unwrap_or_default();
        let t = one(src);
        if !t.report.diagnostics.is_empty() {
            *self.skipped.entry(format!("{first}: M4 báo")).or_default() += 1;
            return;
        }
        let m = gt_mir::lower_program(&t);
        if !m.report.diagnostics.is_empty() {
            *self.skipped.entry(format!("{first}: M5 báo")).or_default() += 1;
            return;
        }
        let acc = |mu: Mutant| check_program_probe(&t, &m, mu).diagnostics.is_empty();
        let orig = acc(Mutant::None);
        // TIỀN ĐỀ: cạnh gốc ⊆ cạnh đột biến (361/591/554)
        let base = probe_constraint_edges(&t, &m, Mutant::None);
        self.edge_checked += 1;
        for (i, (mu, _)) in MUTANTS.iter().enumerate() {
            let e = probe_constraint_edges(&t, &m, *mu);
            for ((k, b), (_, x)) in base.iter().zip(&e) {
                self.edge_added[i] += x.difference(b).count() as u64;
                if i < 3 && !b.is_subset(x) {
                    self.edge_violations
                        .push(format!("{name} / {k} / đột biến {}", MUTANTS[i].1));
                }
            }
        }
        // tính MỘT lần cho mỗi đột biến
        struct One {
            a: bool,
            ora: Option<(Option<String>, bool, bool)>,
        }
        let mut ones = Vec::new();
        for (mu, _) in &MUTANTS {
            let a = acc(*mu);
            let mut ora = None;
            if orig && !a && oracle {
                let mut worst: Option<String> = None;
                let mut other = false;
                for args in entry_sets() {
                    let mut o = Oracle::new(&m, FUEL);
                    match o.call_key("main::entry", args) {
                        Outcome::Violation(v) => worst = Some(format!("{:?}", v.kind)),
                        Outcome::Bug(_) | Outcome::OutOfFuel => other = true,
                        _ => {}
                    }
                }
                ora = Some((worst, other, true));
            }
            ones.push(One { a, ora });
        }
        for kind in kinds {
            let stats = self.stats(kind);
            for (i, one) in ones.iter().enumerate() {
                let st = &mut stats[i];
                st.total += 1;
                if orig {
                    st.orig_accept += 1;
                }
                if one.a && !orig {
                    st.mut_accepts_orig_rejects += 1;
                    if st.samples_i.len() < 30 {
                        st.samples_i.push((name.to_string(), src.to_string()));
                    }
                }
                if orig && !one.a {
                    st.orig_accepts_mut_rejects += 1;
                    if st.samples_ii.len() < 30 {
                        st.samples_ii.push((name.to_string(), src.to_string()));
                    }
                    if let Some((worst, other, _)) = &one.ora {
                        if let Some(w) = worst {
                            st.oracle_violation += 1;
                            st.violations
                                .push((format!("{name} [{w}]"), src.to_string()));
                        } else if !other {
                            st.oracle_clean += 1;
                        } else {
                            st.oracle_other += 1;
                        }
                    }
                }
            }
        }
    }
}

fn env_u64(k: &str, d: u64) -> u64 {
    std::env::var(k)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(d)
}

fn corpus(kind: &str) -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join(kind);
    let mut v: Vec<(String, String)> = std::fs::read_dir(dir)
        .expect("thư mục corpus")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "gt"))
        .map(|p| {
            (
                format!(
                    "tests/{kind}/{}",
                    p.file_name().expect("tên").to_string_lossy()
                ),
                std::fs::read_to_string(&p)
                    .expect("đọc")
                    .replace("\r\n", "\n"),
            )
        })
        .collect();
    v.sort();
    v
}

fn write_samples(rep: &Report) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    let _ = std::fs::create_dir_all(&dir);
    let mut f = std::fs::File::create(dir.join("probe_diff_samples.txt")).expect("tạo");
    for (kind, stats) in &rep.by_kind {
        for (i, (_, mname)) in MUTANTS.iter().enumerate() {
            let st = &stats[i];
            for (n, s) in &st.samples_ii {
                let _ = writeln!(
                    f,
                    "=== (ii) đột biến {mname}: GỐC CHẤP NHẬN, ĐỘT BIẾN TỪ CHỐI — loại {kind} — {n}\n{s}"
                );
            }
            for (n, s) in &st.samples_i {
                let _ = writeln!(
                    f,
                    "=== (i) đột biến {mname}: ĐỘT BIẾN CHẤP NHẬN, GỐC TỪ CHỐI — loại {kind} — {n}\n{s}"
                );
            }
            for (n, s) in &st.violations {
                let _ = writeln!(
                    f,
                    "=== (iii) đột biến {mname}: ORACLE THẤY VI PHẠM TRÊN BẢN GỐC — loại {kind} — {n}\n{s}"
                );
            }
        }
    }
}

fn summarize(rep: &Report) -> String {
    let mut out = String::new();
    for (i, (_, mname)) in MUTANTS.iter().enumerate() {
        let (mut tot, mut oa, mut i_n, mut ii, mut ov, mut oc, mut oo) = (0, 0, 0, 0, 0, 0, 0);
        out.push_str(&format!("--- đột biến {mname}\n"));
        for (kind, stats) in &rep.by_kind {
            let st = &stats[i];
            tot += st.total;
            oa += st.orig_accept;
            i_n += st.mut_accepts_orig_rejects;
            ii += st.orig_accepts_mut_rejects;
            ov += st.oracle_violation;
            oc += st.oracle_clean;
            oo += st.oracle_other;
            out.push_str(&format!(
                "    {kind:<22} tổng {:>6} | gốc chấp nhận {:>6} | (i) {:>4} | (ii) {:>5} | oracle trên (ii): vi phạm {:>3}, sạch {:>4}, khác {:>3}\n",
                st.total, st.orig_accept, st.mut_accepts_orig_rejects, st.orig_accepts_mut_rejects,
                st.oracle_violation, st.oracle_clean, st.oracle_other
            ));
        }
        out.push_str(&format!(
            "    TỔNG {tot} | gốc chấp nhận {oa} | (i) {i_n} | (ii) {ii} | oracle trên (ii): vi phạm {ov}, sạch {oc}, khác {oo}\n"
        ));
    }
    for (k, v) in &rep.skipped {
        out.push_str(&format!("bỏ qua {k}: {v}\n"));
    }
    out
}

#[test]
fn mutants_361_591_554_never_accept_what_the_original_rejects() {
    let n_random = env_u64("GT_PROBE_N", 400);
    let mut rep = Report::default();
    // 1. corpus (không có `entry` ⇒ không chạy oracle)
    for dir in ["pass", "fail"] {
        for (name, src) in corpus(dir) {
            rep.judge(&[format!("corpus_{dir}")], &name, &src, false);
        }
    }
    // 2. bộ sinh ngẫu nhiên có sẵn (13 loại cấu trúc)
    for s in 0..n_random {
        let p = Gen::new(1_000_000 + s).program();
        let src = p.source();
        // cộng vào MỖI loại cấu trúc mà chương trình mang (13 loại của bộ sinh có sẵn) + một dòng chung
        let mut kinds: Vec<String> = p.tags.iter().map(|k| format!("progen:{k}")).collect();
        kinds.push("progen:ALL".to_string());
        rep.judge(&kinds, &format!("seed {s}"), &src, true);
    }
    // 2b. chương trình TẤN CÔNG của `interior_mut.rs` (đối chứng dương cho đột biến 238: bản gốc từ chối, 238 phải chấp nhận)
    for (i, body) in ATTACKS.iter().enumerate() {
        let src = format!(
            "ƒ use_i(x: i32) ! ∅ {{ }}
ƒ idg<T>(x: T) → T ! ∅ {{ x }}
▢ Cl<'a, 'b> {{ c: &'b Cell<&'a i32> }}
ƒ main() ! ∅ {{
{body}
}}
"
        );
        rep.judge(
            &["control_interior_mut_attack".to_string()],
            &format!("attack {i}"),
            &src,
            false,
        );
    }
    // 3. bộ sinh ĐỊNH HƯỚNG
    let per = n_random.max(70) / 2;
    for s in 0..per {
        for kd in 0..probegen::KINDS.len() {
            let p = probegen::generate(kd, 7_000_000 + s);
            rep.judge(
                &[format!("probe:{}", p.kind), "probe:ALL".to_string()],
                &format!("seed {s}"),
                &p.source,
                true,
            );
        }
    }
    write_samples(&rep);
    let sum = summarize(&rep);
    println!("{sum}");
    let _ = std::fs::write(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/probe_diff_summary.txt"),
        &sum,
    );
    let _ = progen::KINDS;
    println!(
        "TIỀN ĐỀ: {} chương trình; tập cạnh gốc ⊆ tập cạnh đột biến: {} vi phạm; cạnh THÊM của 361/591/554/238 = {:?}",
        rep.edge_checked,
        rep.edge_violations.len(),
        rep.edge_added
    );
    assert!(
        rep.edge_violations.is_empty(),
        "tiền đề đổ: đột biến BỎ cạnh của bản gốc: {:?}",
        &rep.edge_violations[..rep.edge_violations.len().min(10)]
    );
    // (i) bằng 0 cho 361/591/554
    for (i, (_, mname)) in MUTANTS.iter().enumerate().take(3) {
        let n: u64 = rep
            .by_kind
            .values()
            .map(|s| s[i].mut_accepts_orig_rejects)
            .sum();
        assert_eq!(
            n, 0,
            "đột biến {mname}: {n} chương trình bản ĐỘT BIẾN chấp nhận mà bản GỐC từ chối ⇒ lập luận đơn điệu ĐỔ (xem target/tmp/probe_diff_samples.txt)"
        );
    }
    // (iii) mọi vi phạm oracle trên tập chênh là âm tính giả của bản gốc
    for (i, (_, mname)) in MUTANTS.iter().enumerate().take(3) {
        let v: u64 = rep.by_kind.values().map(|s| s[i].oracle_violation).sum();
        assert_eq!(
            v, 0,
            "đột biến {mname}: oracle thấy vi phạm trên {v} chương trình bản GỐC chấp nhận ⇒ âm tính giả THẬT của bản gốc"
        );
    }
    // đối chứng dương: 238 phải bị bắt bởi kịch bản interior_mut (nếu không, phép đo mù)
    let control: u64 = rep
        .by_kind
        .iter()
        .filter(|(k, _)| k.contains("control_interior_mut"))
        .map(|(_, s)| s[3].mut_accepts_orig_rejects)
        .sum();
    println!(
        "đối chứng dương 238: {control}/{} chương trình tấn công bị đột biến 238 chấp nhận",
        ATTACKS.len()
    );
    assert!(
        control > 0,
        "ĐỐI CHỨNG DƯƠNG THẤT BẠI: đột biến 238 (lỏng hơn) không tạo chương trình nào mà đột biến chấp nhận còn gốc từ chối trên kịch bản interior_mut ⇒ phép đo mù"
    );
}
