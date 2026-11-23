//! ORACLE cho `std::Slab` (GÓI 12, ADR-C17): kho đối tượng sống lâu có tay cầm thế hệ (như `MemoryPool` của Zig, `slotmap` của Rust).
//! Chuỗi thao tác ngẫu nhiên (thêm / xoá / đọc / đọc bằng KHOÁ CŨ sau khi ô đã được tái dùng / đếm) chạy QUA CÙNG CỔNG của `gritc` như mã người
//! dùng (`check` M4–M9) rồi bằng thông dịch MIR; kết quả so với MÔ HÌNH Rust độc lập viết dưới đây (không dùng mã Grit nào).
//! Tính chất bộ nhớ khẳng định tường minh: `capacity()` (số ô từng cấp, không bao giờ giảm) == số phần tử SỐNG TỐI ĐA từng có — ô bị xoá
//! luôn được tái dùng trước khi cấp ô mới, nên thêm/xoá liên tục suốt đời chương trình không làm bộ nhớ tăng quá đỉnh.
//! Luôn chạy bằng thông dịch; bản dựng có backend LLVM thì 10 chương trình đầu còn được dựng thành exe THẬT và phải ra cùng số.
use gt_driver::runner::run_main;
use gt_driver::{Options, RunEnd, check};
use gt_resolve::SourceInput;

const STD: &str = include_str!("../../../lib/std/src/std.gt");
const STD_ORD: &str = include_str!("../../../lib/std/src/std_ord.gt");
const STD_ORD_INT: &str = include_str!("../../../lib/std/src/std_ord_int.gt");

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

/// Mô hình độc lập: ô = (giá trị, thế hệ, sống); danh sách ô trống LIFO; khoá = (chỉ số, thế hệ).
#[derive(Default)]
struct Model {
    slots: Vec<(u64, u64, bool)>,
    free: Vec<usize>,
    live: usize,
    peak: usize,
}

impl Model {
    fn insert(&mut self, v: u64) -> (usize, u64) {
        self.live += 1;
        self.peak = self.peak.max(self.live);
        if let Some(i) = self.free.pop() {
            self.slots[i].0 = v;
            self.slots[i].2 = true;
            (i, self.slots[i].1)
        } else {
            self.slots.push((v, 0, true));
            (self.slots.len() - 1, 0)
        }
    }
    fn get(&self, k: (usize, u64)) -> Option<u64> {
        self.slots
            .get(k.0)
            .filter(|s| s.2 && s.1 == k.1)
            .map(|s| s.0)
    }
    fn remove(&mut self, k: (usize, u64)) -> Option<u64> {
        let v = self.get(k)?;
        let s = &mut self.slots[k.0];
        s.2 = false;
        s.1 = s.1.wrapping_add(1);
        self.free.push(k.0);
        self.live -= 1;
        Some(v)
    }
}

fn ov(o: Option<u64>) -> u64 {
    o.map_or(0, |x| x.wrapping_add(1))
}

/// Sinh (mã `main.gt`, giá trị kỳ vọng). Thao tác dùng lại MỌI khoá đã cấp, kể cả khoá đã xoá (thử phát hiện khoá cũ).
fn program(seed: u64, ops: usize) -> (String, u64, usize) {
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut m = Model::default();
    let mut keys: Vec<(usize, u64)> = Vec::new();
    let mut acc: u64 = 7;
    let mut src = String::from(
        "ƒ ov(o: Option<u64>) → u64 ! ∅ { ⌥ o { Some(x) ↦ x +% 1u64, None ↦ 0u64 } }\n\
         ƒ main() → u64 ! {mem} {\n  ar ≔ Arena::new();\n  μs ≔ std::Slab::new(&ar);\n  μacc ≔ 7u64;\n",
    );
    let mix = |acc: u64, x: u64| acc.wrapping_mul(31).wrapping_add(x);
    for _ in 0..ops {
        let pick = if keys.is_empty() { 0 } else { r.below(10) };
        match pick {
            0..=3 => {
                let v = r.below(1000);
                let k = m.insert(v);
                src.push_str(&format!("  k{} ≔ s.insert({v}u64);\n", keys.len()));
                keys.push(k);
            }
            4..=6 => {
                let j = r.below(keys.len() as u64) as usize;
                acc = mix(acc, ov(m.remove(keys[j])));
                src.push_str(&format!("  acc ← acc *% 31u64 +% ov(s.remove(k{j}));\n"));
            }
            7..=8 => {
                let j = r.below(keys.len() as u64) as usize;
                acc = mix(acc, ov(m.get(keys[j])));
                src.push_str(&format!("  acc ← acc *% 31u64 +% ov(s.get(k{j}));\n"));
            }
            _ => {
                acc = mix(acc, m.live as u64);
                src.push_str("  acc ← acc *% 31u64 +% (s.len() as u64);\n");
            }
        }
    }
    // tính chất bộ nhớ: số ô từng cấp == số sống tối đa (mô hình), và Grit phải cho cùng số
    assert_eq!(m.slots.len(), m.peak, "mô hình sai: ô cấp ≠ đỉnh sống");
    acc = mix(acc, m.slots.len() as u64);
    src.push_str("  acc *% 31u64 +% (s.capacity() as u64)\n}\n");
    (src, acc, m.peak)
}

fn inputs(main: &str) -> Vec<SourceInput> {
    [
        ("std.gt", STD),
        ("std_ord.gt", STD_ORD),
        ("std_ord_int.gt", STD_ORD_INT),
        ("main.gt", main),
    ]
    .iter()
    .map(|(n, t)| SourceInput {
        name: (*n).to_string(),
        text: (*t).to_string(),
    })
    .collect()
}

#[test]
fn slab_matches_an_independent_model_and_never_grows_past_peak_live() {
    let mut reused_stale = 0u32;
    for seed in 1..=40u64 {
        let (src, want, peak) = program(seed, 120);
        let mut res = check(&inputs(&src), &Options::default()).expect("đầu vào hợp lệ");
        let codes: Vec<String> = res
            .report
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str().starts_with('E'))
            .map(|d| format!("{}: {}", d.code.as_str(), d.message))
            .collect();
        assert!(codes.is_empty(), "#{seed}: gritc từ chối: {codes:?}\n{src}");
        #[cfg(feature = "llvm")]
        if seed <= 10 {
            use gt_driver::pipeline::{attestation, mir_dump};
            let att = {
                let art = res.open_gate().expect("cổng mở");
                attestation(&res, &mir_dump(art))
            };
            let dir = std::env::temp_dir().join(format!("gt-std-slab-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("tạo thư mục");
            let exe = dir.join(format!("slab{seed}.exe"));
            gt_driver::native::build_exe_profile(
                res.open_gate().expect("cổng"),
                &exe,
                true,
                &att,
                gt_codegen_llvm::Opt::Dev,
            )
            .unwrap_or_else(|e| panic!("#{seed}: backend lỗi: {e}"));
            let r =
                gt_driver::native::run_exe_timeout(&exe, Some(std::time::Duration::from_secs(30)));
            let _ = std::fs::remove_file(&exe);
            assert_eq!(r.end, RunEnd::Normal, "#{seed}: exe {:?}", r.end);
            let got: Option<u64> = r.value.as_deref().and_then(|v| v.trim().parse().ok());
            assert_eq!(got, Some(want), "#{seed}: exe LLVM ≠ mô hình\n{src}");
        }
        let art = res.take_gated().expect("cổng mở (XANH/VÀNG)");
        let run = run_main(art);
        assert_eq!(run.end, RunEnd::Normal, "#{seed}: {:?}\n{src}", run.end);
        let got = run
            .value
            .as_deref()
            .and_then(|v| v.strip_prefix("Int("))
            .and_then(|v| v.split(',').next())
            .and_then(|v| v.parse::<u64>().ok());
        assert_eq!(
            got,
            Some(want),
            "#{seed}: Slab ≠ mô hình (peak {peak})\n{src}"
        );
        if src.contains("s.get(k") && src.contains("s.remove(k") {
            reused_stale += 1;
        }
    }
    assert!(
        reused_stale >= 30,
        "bộ sinh quá ít ca xoá + đọc lại: {reused_stale}"
    );
}
