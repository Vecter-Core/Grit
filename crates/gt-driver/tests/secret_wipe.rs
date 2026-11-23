//! A1 Giai đoạn 2 (ADR-P2-03): xoá `Secret` có bảo đảm. Mỗi giá trị `Secret` phải bị xoá (ghi 0 qua `gritrt_secret_wipe`) khi CHẾT (drop glue: hết phạm vi,
//! gán đè) và chỗ nguồn phải bị xoá ngay sau khi nó bị MOVE ra (vd `declassify(self)`), vì move để lại bản sao bit.
//! Oracle: chương trình sinh ngẫu nhiên (khai báo `Secret::new`, `declassify` — có hoặc không điều kiện —, khối lồng) với MÔ HÌNH độc lập tính
//! số lần xoá = số `declassify` thực sự chạy + số Secret còn sống lúc hết phạm vi; so với số đếm của thông dịch VÀ exe LLVM (`GRITRT_SECRET_REPORT=1`)
//! ở hồ sơ dev, release, release-o2 (O2 không được loại bỏ thao tác xoá). Giá trị `main` trả về (tổng các giá trị đã declassify) cũng phải khớp.
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

/// biến Secret trong phạm vi: (tên, giá trị, còn sống lúc chạy, dùng lại được theo luật tĩnh)
struct Gen {
    r: Rng,
    src: String,
    scopes: Vec<Vec<(String, u64, bool, bool)>>,
    n: u32,
    wipes: u64,
    sum: u64,
    stats: [u32; 3],
}

impl Gen {
    fn ind(&self) -> String {
        "  ".repeat(self.scopes.len() + 1)
    }
    fn end_scope(&mut self) {
        for (_, _, alive, _) in self.scopes.pop().unwrap_or_default() {
            if alive {
                self.wipes += 1;
            }
        }
    }
    fn stmt(&mut self, depth: u32) {
        let ind = self.ind();
        let live: Vec<(usize, usize)> = self
            .scopes
            .iter()
            .enumerate()
            .flat_map(|(s, v)| {
                v.iter()
                    .enumerate()
                    .filter(|(_, x)| x.2 && x.3)
                    .map(move |(i, _)| (s, i))
            })
            .collect();
        match self.r.below(5) {
            0 | 1 => {
                let v = 1 + self.r.below(1000);
                let name = format!("s{}", self.n);
                self.n += 1;
                self.src
                    .push_str(&format!("{ind}{name} ≔ Secret::new({v}u64);\n"));
                if let Some(sc) = self.scopes.last_mut() {
                    sc.push((name, v, true, true));
                }
            }
            2 if !live.is_empty() => {
                let (s, i) = live[self.r.below(live.len() as u64) as usize];
                let (name, v) = (self.scopes[s][i].0.clone(), self.scopes[s][i].1);
                self.src
                    .push_str(&format!("{ind}acc ← acc +% {name}.declassify();\n"));
                self.sum = self.sum.wrapping_add(v);
                self.wipes += 1;
                self.scopes[s][i].2 = false;
                self.scopes[s][i].3 = false;
                self.stats[0] += 1;
            }
            3 if !live.is_empty() => {
                // declassify CÓ ĐIỀU KIỆN: chạy ⇒ xoá ngay sau move; không chạy ⇒ còn sống, xoá khi hết phạm vi (cờ drop)
                let (s, i) = live[self.r.below(live.len() as u64) as usize];
                let (name, v) = (self.scopes[s][i].0.clone(), self.scopes[s][i].1);
                let c = self.r.below(2) == 0;
                self.src.push_str(&format!(
                    "{ind}⎇ opq({c}) {{ acc ← acc +% {name}.declassify(); }} ⇝ {{ }}\n"
                ));
                if c {
                    self.sum = self.sum.wrapping_add(v);
                    self.wipes += 1;
                    self.scopes[s][i].2 = false;
                }
                self.scopes[s][i].3 = false;
                self.stats[1] += 1;
            }
            _ if depth < 3 => {
                self.src.push_str(&format!("{ind}{{\n"));
                self.scopes.push(Vec::new());
                for _ in 0..1 + self.r.below(4) {
                    self.stmt(depth + 1);
                }
                self.end_scope();
                self.src.push_str(&format!("{ind}}};\n"));
                self.stats[2] += 1;
            }
            _ => {}
        }
    }
}

fn program(seed: u64) -> (String, u64, u64, [u32; 3]) {
    let mut g = Gen {
        r: Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1),
        src: String::from(
            "ƒ opq(x: bool) → bool ! ∅ { x }\nƒ main() → u64 ! ∅ {\n  μacc ≔ 0u64;\n  {\n",
        ),
        scopes: vec![Vec::new()],
        n: 0,
        wipes: 0,
        sum: 0,
        stats: [0; 3],
    };
    for _ in 0..3 + g.r.below(8) {
        g.stmt(0);
    }
    g.end_scope();
    g.src.push_str("  };\n  acc\n}\n");
    (g.src, g.sum, g.wipes, g.stats)
}

fn inputs(main: &str) -> Vec<SourceInput> {
    vec![SourceInput {
        name: "main.gt".to_string(),
        text: main.to_string(),
    }]
}

/// (backend, giá trị main, số lần xoá Secret)
fn run(src: &str, tag: &str) -> Vec<(String, Option<u64>, u64)> {
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
        let dir = std::env::temp_dir().join(format!("gt-secret-wipe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tạo thư mục");
        for (pname, prof) in [
            ("dev", gt_codegen_llvm::Opt::Dev),
            ("release", gt_codegen_llvm::Opt::Release),
            ("release-o2", gt_codegen_llvm::Opt::ReleaseO2),
        ] {
            let exe = dir.join(format!("{tag}-{pname}.exe"));
            let result = dir.join(format!("{tag}-{pname}.result"));
            gt_driver::native::build_exe_profile(
                res.open_gate().expect("cổng"),
                &exe,
                true,
                &att,
                prof,
            )
            .unwrap_or_else(|e| panic!("{tag}/{pname}: backend lỗi: {e}\n{src}"));
            let o = std::process::Command::new(&exe)
                .env("GRITRT_SECRET_REPORT", "1")
                .env("GRITRT_RESULT_FILE", &result)
                .output()
                .expect("chạy exe");
            let err = String::from_utf8_lossy(&o.stderr).into_owned();
            assert_eq!(o.status.code(), Some(0), "{tag}/{pname}: {err}\n{src}");
            let wipes = err
                .lines()
                .find_map(|l| l.strip_prefix("gritrt_secret_wipes="))
                .and_then(|n| n.split_whitespace().next()?.parse::<u64>().ok())
                .unwrap_or_else(|| panic!("{tag}/{pname}: exe không báo số lần xoá: {err}"));
            let val = std::fs::read_to_string(&result)
                .ok()
                .and_then(|v| v.trim().parse().ok());
            let _ = std::fs::remove_file(&exe);
            let _ = std::fs::remove_file(&result);
            out.push((format!("llvm-{pname}"), val, wipes));
        }
    }
    let art = res.take_gated().expect("cổng mở");
    let (r, stats) = gt_driver::runner::run_main_stats(art, 10_000_000);
    assert_eq!(
        r.end,
        RunEnd::Normal,
        "{tag}: thông dịch {:?}\n{src}",
        r.end
    );
    let val = r.value.as_deref().and_then(|v| {
        v.strip_prefix("Int(")?
            .split(',')
            .next()?
            .trim()
            .parse()
            .ok()
    });
    out.push(("interp".to_string(), val, stats.secret_wipes));
    out
}

#[test]
fn every_secret_is_wiped_when_it_dies_and_its_source_after_a_move_on_every_backend_and_profile() {
    let mut tally = [0u32; 3];
    for seed in 1..=24u64 {
        let (src, sum, wipes, st) = program(seed);
        for (k, v) in st.iter().enumerate() {
            tally[k] += v;
        }
        for (backend, got, w) in run(&src, &format!("sw{seed}")) {
            assert_eq!(got, Some(sum), "#{seed} ({backend}): giá trị sai\n{src}");
            assert_eq!(
                w, wipes,
                "#{seed} ({backend}): số lần xoá Secret sai (mô hình {wipes})\n{src}"
            );
        }
    }
    for (k, n) in ["declassify", "declassify có điều kiện", "khối lồng"]
        .iter()
        .enumerate()
    {
        assert!(tally[k] >= 8, "bộ sinh quá ít ca `{n}`: {tally:?}");
    }
}

#[test]
fn hand_written_secret_lifetimes_wipe_exactly_as_specified() {
    // (thân, tổng declassify, số lần xoá kỳ vọng) — tính tay theo ADR-P2-03
    let cases: [(&str, u64, u64); 5] = [
        ("s ≔ Secret::new(5u64);", 0, 1),
        ("s ≔ Secret::new(5u64); acc ← acc +% s.declassify();", 5, 1),
        (
            "μs ≔ Secret::new(5u64); s ← Secret::new(7u64); acc ← acc +% s.declassify();",
            7,
            // tạm kết quả `new(7)` move sang tạm giữ (1) + drop giá trị cũ (1) + tạm giữ move vào `s` (1) + `declassify` (1) — MIR đã đọc
            4,
        ),
        (
            "a ≔ Secret::new(1u64); b ≔ Secret::new(2u64); { c ≔ Secret::new(3u64); };",
            0,
            3,
        ),
        // qua hàm `keep`: tạm đối số (1) + `k` move vào `_0` (1) + `_0` khi trả về (1) + `s` hết phạm vi (1)
        ("s ≔ keep(Secret::new(1u64));", 0, 4),
    ];
    for (i, (body, sum, wipes)) in cases.iter().enumerate() {
        let src = format!(
            "ƒ keep(k: Secret<u64>) → Secret<u64> ! ∅ {{ k }}\nƒ main() → u64 ! ∅ {{\n  μacc ≔ 0u64;\n  {{\n    {body}\n  }};\n  acc\n}}\n"
        );
        for (backend, got, w) in run(&src, &format!("hand{i}")) {
            assert_eq!(got, Some(*sum), "ca {i} ({backend})\n{src}");
            assert_eq!(w, *wipes, "ca {i} ({backend}): số lần xoá\n{src}");
        }
    }
}
