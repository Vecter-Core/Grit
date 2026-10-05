//! TÍNH CHẤT BẮT BUỘC của plan (M3): kết quả phân giải không phụ thuộc thứ tự khai báo/thứ tự tệp (ADR-017 mục "phạm vi chính xác").
//! * Xáo thứ tự TỆP → mọi thứ giống TỪNG BYTE (kể cả vị trí và thứ tự chẩn đoán D19, vì vị trí trong tệp không đổi).
//! * Xáo thứ tự ITEM trong tệp → dạng chuẩn `canonical()` giống hệt và tập chẩn đoán (mã, thông điệp, vị trí logic) giống hệt;
//!   offset (và do đó thứ tự D19 theo offset) KHÔNG thể giống — D19 sắp theo offset.
mod common;
use common::*;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn chance(&mut self, n: u64) -> bool {
        self.next().is_multiple_of(n)
    }
    fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[self.below(xs.len())]
    }
    fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}

const FILES: [&str; 4] = ["ma", "mb", "mc", "md"];
const NAMES: [&str; 6] = ["f", "g", "h", "k", "P", "E"];
const LOCALS: [&str; 4] = ["x", "y", "f", "Some"];

fn stmt(r: &mut Rng) -> String {
    match r.below(9) {
        0 => format!("{}();", r.pick(&NAMES)),
        1 => format!("{}::{}();", r.pick(&FILES), r.pick(&NAMES)),
        2 => format!("{} ≔ 1;", r.pick(&LOCALS)),
        3 => format!("⟳ {} in 0..2 {{ }}", r.pick(&LOCALS)),
        4 => format!("⎇ true {{ {} ≔ 3; }}", r.pick(&LOCALS)),
        5 => format!("{} ← 4;", r.pick(&LOCALS)),
        6 => format!("{}::{}::g();", r.pick(&FILES), r.pick(&["m", "n"])),
        7 => format!("Q::{}(1);", r.pick(&NAMES)),
        _ => format!("{};", r.pick(&LOCALS)),
    }
}

fn body(r: &mut Rng) -> String {
    let n = r.below(4);
    (0..n).map(|_| stmt(r)).collect::<Vec<_>>().join(" ")
}

fn item(r: &mut Rng) -> String {
    let vis = if r.chance(2) { "◈ " } else { "" };
    match r.below(8) {
        0..=2 => format!(
            "{vis}ƒ {}() ! ∅ {{ {} }}",
            r.pick(&["f", "g", "h", "k"]),
            body(r)
        ),
        3 => format!("{vis}▢ {} {{ a: i32 }}", r.pick(&["P", "Q"])),
        4 => format!("{vis}⊔ {} {{ A, B(i32) }}", r.pick(&["E", "Q"])),
        5 => format!("⇲ {}::{};", r.pick(&FILES), r.pick(&NAMES)),
        6 => format!(
            "{vis}▦ {} {{ {vis}ƒ g() ! ∅ {{ {} }} }}",
            r.pick(&["m", "n"]),
            body(r)
        ),
        _ => format!("ƒ u{}(x: i32) → i32 ! ∅ {{ {} }}", r.below(3), body(r)),
    }
}

/// Mỗi tệp = danh sách item (chuỗi); không phụ thuộc thứ tự.
fn program(r: &mut Rng) -> Vec<(String, Vec<String>)> {
    let nfiles = 1 + r.below(4);
    (0..nfiles)
        .map(|i| {
            let n = r.below(6);
            (
                format!("{}.gt", FILES[i]),
                (0..n).map(|_| item(r)).collect(),
            )
        })
        .collect()
}

fn run_prog(files: &[(String, Vec<String>)]) -> gt_resolve::ResolveOutput {
    let owned: Vec<(String, String)> = files
        .iter()
        .map(|(n, items)| (n.clone(), items.join("\n")))
        .collect();
    let refs: Vec<(&str, &str)> = owned
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    run(&refs)
}

fn dump(out: &gt_resolve::ResolveOutput) -> String {
    out.report
        .diagnostics
        .iter()
        .map(|d| {
            format!(
                "{} {} {} {}..{} {:?}\n",
                d.code,
                d.file,
                d.message,
                d.span.start,
                d.span.end,
                d.labels
                    .iter()
                    .map(|l| format!("{}:{}..{}:{}", l.file, l.span.start, l.span.end, l.message))
                    .collect::<Vec<_>>()
            )
        })
        .collect()
}

#[test]
fn shuffling_file_order_changes_nothing_at_all() {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let mut with_errors = 0;
    for case in 0..400 {
        let prog = program(&mut r);
        let base = run_prog(&prog);
        if !base.report.diagnostics.is_empty() {
            with_errors += 1;
        }
        for _ in 0..3 {
            let mut shuffled = prog.clone();
            r.shuffle(&mut shuffled);
            let other = run_prog(&shuffled);
            assert_eq!(
                base.canonical(),
                other.canonical(),
                "case {case}: canonical khác\n{prog:?}"
            );
            assert_eq!(
                dump(&base),
                dump(&other),
                "case {case}: chẩn đoán khác\n{prog:?}"
            );
            assert_eq!(
                base.canonical_diagnostics(),
                other.canonical_diagnostics(),
                "case {case}"
            );
        }
    }
    // test phải thực sự gặp cả chương trình lỗi lẫn chương trình sạch
    assert!(with_errors > 100, "chỉ {with_errors} ca có lỗi");
}

#[test]
fn shuffling_item_order_keeps_canonical_resolution_and_the_logical_error_set() {
    let mut r = Rng(0xD1B5_4A32_D192_ED03);
    let (mut with_errors, mut clean) = (0, 0);
    for case in 0..400 {
        let prog = program(&mut r);
        let base = run_prog(&prog);
        if base.report.diagnostics.is_empty() {
            clean += 1;
        } else {
            with_errors += 1;
        }
        for _ in 0..3 {
            let mut shuffled = prog.clone();
            for (_, items) in &mut shuffled {
                r.shuffle(items);
            }
            r.shuffle(&mut shuffled);
            let other = run_prog(&shuffled);
            assert_eq!(
                base.canonical(),
                other.canonical(),
                "case {case}: canonical khác\n{prog:?}\n=>\n{shuffled:?}"
            );
            assert_eq!(
                base.canonical_diagnostics(),
                other.canonical_diagnostics(),
                "case {case}: tập chẩn đoán logic khác\n{prog:?}\n=>\n{shuffled:?}"
            );
            assert_eq!(
                base.report.diagnostics.len(),
                other.report.diagnostics.len(),
                "case {case}"
            );
        }
    }
    assert!(
        with_errors > 100 && clean > 20,
        "lỗi={with_errors} sạch={clean}"
    );
}

#[test]
fn rerunning_gives_identical_output_determinism() {
    let mut r = Rng(0x2545_F491_4F6C_DD1D);
    for _ in 0..100 {
        let prog = program(&mut r);
        let a = run_prog(&prog);
        let b = run_prog(&prog);
        assert_eq!(a.canonical(), b.canonical());
        assert_eq!(dump(&a), dump(&b));
    }
}
