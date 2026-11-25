//! Payload enum hẹp ở backend LLVM — oracle động (nghi vấn cố vấn 2026-11-25, xem `docs/HISTORY.md`).
//! LLVM biểu diễn enum là `{ i64 tag, [k x i64] payload }`; biến thể có trường hẹp (`u8`, `bool`, `i16`, …) để lại byte CHƯA GHI trong
//! các ô `i64` của payload, và mọi lần move/copy enum là load/store CẢ struct. Theo LangRef, load số nguyên trên byte undef cho bit undef
//! (byte poison thì cả giá trị poison); nghi vấn: tối ưu hoá có thể làm hỏng byte ĐÃ GHI của payload.
//! Bộ sinh: 2–3 enum ngẫu nhiên (2–4 biến thể, 0–3 trường thuộc `u8/u16/u32/u64/bool/i8/i16`, enum trước, `Option<u8>`, `Option<E_j>`),
//! giá trị dựng theo hạt `s` ở thời gian chạy, đi qua chuỗi luồng ngẫu nhiên (hàm đồng nhất, bọc `Some`/tuple rồi mở bằng `⌥`,
//! gán đè tại chỗ bằng biến thể khác theo điều kiện runtime), rồi đọc lại MỌI trường bằng `⌥` và băm. Mô hình Rust ĐỘC LẬP tính băm;
//! thông dịch MIR VÀ exe LLVM (dev, release, release-o2) phải khớp.
//! Xanh KHÔNG chứng minh vắng lỗi: chỉ là bằng chứng thực nghiệm trên LLVM đang liên kết (20.x) cho các dạng đã sinh.
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

const P: u64 = 1_000_003;

#[derive(Clone, Copy)]
enum Prim {
    U8,
    U16,
    U32,
    U64,
    Bool,
    I8,
    I16,
}

#[derive(Clone, Copy)]
enum FTy {
    Prim(Prim),
    Enum(usize),
    OptU8,
    OptEnum(usize),
}

struct Field {
    ty: FTy,
    c: u64,
    d: u64,
}

struct Variant {
    fields: Vec<Field>,
}

struct EnumDef {
    variants: Vec<Variant>,
}

fn prim_name(p: Prim) -> &'static str {
    match p {
        Prim::U8 => "u8",
        Prim::U16 => "u16",
        Prim::U32 => "u32",
        Prim::U64 => "u64",
        Prim::Bool => "bool",
        Prim::I8 => "i8",
        Prim::I16 => "i16",
    }
}

fn ty_name(t: FTy) -> String {
    match t {
        FTy::Prim(p) => prim_name(p).to_string(),
        FTy::Enum(j) => format!("E{j}"),
        FTy::OptU8 => "Option<u8>".to_string(),
        FTy::OptEnum(j) => format!("Option<E{j}>"),
    }
}

/// biểu thức Grit dựng giá trị trường từ hạt `x` (văn bản u64)
fn build_expr(t: FTy, x: &str) -> String {
    match t {
        FTy::Prim(Prim::U8) => format!("(({x}) % 256u64) as u8"),
        FTy::Prim(Prim::U16) => format!("(({x}) % 65536u64) as u16"),
        FTy::Prim(Prim::U32) => format!("(({x}) % 4294967296u64) as u32"),
        FTy::Prim(Prim::U64) => format!("({x})"),
        FTy::Prim(Prim::Bool) => format!("(({x}) % 2u64) == 1u64"),
        FTy::Prim(Prim::I8) => format!("((((({x}) % 256u64) as i64) - 128i64) as i8)"),
        FTy::Prim(Prim::I16) => format!("((((({x}) % 65536u64) as i64) - 32768i64) as i16)"),
        FTy::Enum(j) => format!("mk{j}({x})"),
        FTy::OptU8 => format!("mkou({x})"),
        FTy::OptEnum(j) => format!("mkoe{j}({x})"),
    }
}

/// biểu thức Grit đổi binding `v` thành u64 để băm
fn conv_expr(t: FTy, v: &str) -> String {
    match t {
        FTy::Prim(Prim::Bool) => format!("bu({v})"),
        FTy::Prim(Prim::I8) => format!("((({v}) as i64) + 128i64) as u64"),
        FTy::Prim(Prim::I16) => format!("((({v}) as i64) + 32768i64) as u64"),
        FTy::Prim(_) => format!("{v} as u64"),
        FTy::Enum(j) => format!("f{j}({v})"),
        FTy::OptU8 => format!("fou({v})"),
        FTy::OptEnum(j) => format!("foe{j}({v})"),
    }
}

/// mô hình: băm của giá trị trường dựng từ hạt `x`
fn model_field(defs: &[EnumDef], t: FTy, x: u64) -> u64 {
    match t {
        FTy::Prim(Prim::U8 | Prim::I8) => x % 256,
        FTy::Prim(Prim::U16 | Prim::I16) => x % 65536,
        FTy::Prim(Prim::U32) => x % (1 << 32),
        FTy::Prim(Prim::U64) => x,
        FTy::Prim(Prim::Bool) => {
            if x % 2 == 1 {
                1
            } else {
                2
            }
        }
        FTy::Enum(j) => model_enum(defs, j, x),
        FTy::OptU8 => {
            if x.is_multiple_of(3) {
                5
            } else {
                6u64.wrapping_mul(P).wrapping_add(x % 256)
            }
        }
        FTy::OptEnum(j) => {
            if x.is_multiple_of(3) {
                5
            } else {
                6u64.wrapping_mul(P).wrapping_add(model_enum(defs, j, x))
            }
        }
    }
}

fn model_enum(defs: &[EnumDef], k: usize, s: u64) -> u64 {
    let e = &defs[k];
    let vi = (s % e.variants.len() as u64) as usize;
    let mut h = vi as u64 + 1;
    for f in &e.variants[vi].fields {
        let x = s.wrapping_mul(f.c).wrapping_add(f.d);
        h = h.wrapping_mul(P).wrapping_add(model_field(defs, f.ty, x));
    }
    h
}

/// luồng di chuyển giá trị enum trước khi đọc
enum Step {
    Id,
    Id2,
    /// gán đè tại chỗ khi `s % 3 == r`, bằng hạt `s *% 7 +% r`
    Reassign(u64),
    WrapSome,
    WrapTuple,
}

fn program(seed: u64) -> (String, u64) {
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let ne = 2 + r.below(2) as usize;
    let mut defs: Vec<EnumDef> = Vec::new();
    for k in 0..ne {
        let nv = 2 + r.below(3) as usize;
        let variants = (0..nv)
            .map(|_| {
                let nf = r.below(4) as usize;
                let fields = (0..nf)
                    .map(|_| {
                        let ty = match r.below(10) {
                            0..=6 => FTy::Prim(
                                [
                                    Prim::U8,
                                    Prim::U16,
                                    Prim::U32,
                                    Prim::U64,
                                    Prim::Bool,
                                    Prim::I8,
                                    Prim::I16,
                                ][r.below(7) as usize],
                            ),
                            7 if k > 0 => FTy::Enum(r.below(k as u64) as usize),
                            8 if k > 0 => FTy::OptEnum(r.below(k as u64) as usize),
                            _ => FTy::OptU8,
                        };
                        Field {
                            ty,
                            c: 1 + r.below(997),
                            d: r.below(1000),
                        }
                    })
                    .collect();
                Variant { fields }
            })
            .collect();
        defs.push(EnumDef { variants });
    }
    let mut src = String::new();
    src.push_str("ƒ bu(flag: bool) → u64 ! ∅ { ⎇ flag { 1u64 } ⇝ { 2u64 } }\n");
    src.push_str("ƒ mkou(x: u64) → Option<u8> ! ∅ { ⎇ x % 3u64 == 0u64 { None } ⇝ { Some((x % 256u64) as u8) } }\n");
    src.push_str(&format!(
        "ƒ fou(o: Option<u8>) → u64 ! ∅ {{ ⌥ o {{ None ↦ 5u64, Some(v) ↦ 6u64 *% {P}u64 +% v as u64 }} }}\n"
    ));
    for (k, e) in defs.iter().enumerate() {
        let vs: Vec<String> = e
            .variants
            .iter()
            .enumerate()
            .map(|(vi, v)| {
                if v.fields.is_empty() {
                    format!("E{k}V{vi}")
                } else {
                    let ts: Vec<String> = v.fields.iter().map(|f| ty_name(f.ty)).collect();
                    format!("E{k}V{vi}({})", ts.join(", "))
                }
            })
            .collect();
        src.push_str(&format!("⊔ E{k} {{ {} }}\n", vs.join(", ")));
        // dựng
        src.push_str(&format!(
            "ƒ mk{k}(s: u64) → E{k} ! ∅ {{\n  ⌥ s % {}u64 {{\n",
            e.variants.len()
        ));
        for (vi, v) in e.variants.iter().enumerate() {
            let pat = if vi + 1 == e.variants.len() {
                "_".to_string()
            } else {
                format!("{vi}u64")
            };
            if v.fields.is_empty() {
                src.push_str(&format!("    {pat} ↦ E{k}V{vi},\n"));
            } else {
                let args: Vec<String> = v
                    .fields
                    .iter()
                    .map(|f| build_expr(f.ty, &format!("s *% {}u64 +% {}u64", f.c, f.d)))
                    .collect();
                src.push_str(&format!("    {pat} ↦ E{k}V{vi}({}),\n", args.join(", ")));
            }
        }
        src.push_str("  }\n}\n");
        // đọc
        src.push_str(&format!("ƒ f{k}(e: E{k}) → u64 ! ∅ {{\n  ⌥ e {{\n"));
        for (vi, v) in e.variants.iter().enumerate() {
            if v.fields.is_empty() {
                src.push_str(&format!("    E{k}V{vi} ↦ {}u64,\n", vi + 1));
            } else {
                let binds: Vec<String> = (0..v.fields.len()).map(|m| format!("a{m}")).collect();
                let mut h = format!("{}u64", vi + 1);
                for (m, f) in v.fields.iter().enumerate() {
                    h = format!("({h}) *% {P}u64 +% {}", conv_expr(f.ty, &format!("a{m}")));
                }
                src.push_str(&format!("    E{k}V{vi}({}) ↦ {h},\n", binds.join(", ")));
            }
        }
        src.push_str("  }\n}\n");
        src.push_str(&format!("ƒ id{k}(e: E{k}) → E{k} ! ∅ {{ e }}\n"));
        src.push_str(&format!(
            "ƒ mkoe{k}(x: u64) → Option<E{k}> ! ∅ {{ ⎇ x % 3u64 == 0u64 {{ None }} ⇝ {{ Some(mk{k}(x)) }} }}\n"
        ));
        src.push_str(&format!(
            "ƒ foe{k}(o: Option<E{k}>) → u64 ! ∅ {{ ⌥ o {{ None ↦ 5u64, Some(v) ↦ 6u64 *% {P}u64 +% f{k}(v) }} }}\n"
        ));
        src.push_str(&format!(
            "ƒ unsome{k}(o: Option<E{k}>, s: u64) → E{k} ! ∅ {{ ⌥ o {{ Some(v) ↦ v, None ↦ mk{k}(s) }} }}\n"
        ));
        src.push_str(&format!(
            "ƒ untup{k}(t: (u8, E{k}, u16)) → E{k} ! ∅ {{ ⌥ t {{ (_, v, _) ↦ v }} }}\n"
        ));
    }
    // luồng cho từng enum
    let flows: Vec<Vec<Step>> = (0..ne)
        .map(|_| {
            let n = 1 + r.below(5) as usize;
            (0..n)
                .map(|_| match r.below(5) {
                    0 => Step::Id,
                    1 => Step::Id2,
                    2 => Step::Reassign(r.below(3)),
                    3 => Step::WrapSome,
                    _ => Step::WrapTuple,
                })
                .collect()
        })
        .collect();
    const N: u64 = 24;
    src.push_str(&format!(
        "ƒ main() → u64 ! ∅ {{\n  μacc ≔ 0u64;\n  μs ≔ 0u64;\n  ⟳ s < {N}u64 ↓ {N}u64 - s {{\n"
    ));
    for (k, fl) in flows.iter().enumerate() {
        src.push_str(&format!("    μx{k} ≔ mk{k}(s);\n"));
        for (m, st) in fl.iter().enumerate() {
            match st {
                Step::Id => src.push_str(&format!("    x{k} ← id{k}(x{k});\n")),
                Step::Id2 => src.push_str(&format!("    x{k} ← id{k}(id{k}(x{k}));\n")),
                Step::Reassign(rr) => src.push_str(&format!(
                    "    ⎇ s % 3u64 == {rr}u64 {{ x{k} ← mk{k}(s *% 7u64 +% {rr}u64); }}\n"
                )),
                Step::WrapSome => src.push_str(&format!(
                    "    w{k}_{m} ≔ Some(x{k});\n    x{k} ← unsome{k}(w{k}_{m}, 0u64);\n"
                )),
                Step::WrapTuple => src.push_str(&format!(
                    "    w{k}_{m} ≔ (9u8, x{k}, 700u16);\n    x{k} ← untup{k}(w{k}_{m});\n"
                )),
            }
        }
        src.push_str(&format!("    acc ← acc *% 31u64 +% f{k}(x{k});\n"));
    }
    src.push_str("    s ← s + 1u64;\n  }\n  acc\n}\n");
    // mô hình
    let mut acc = 0u64;
    for s in 0..N {
        for (k, fl) in flows.iter().enumerate() {
            let mut seed_k = s;
            for st in fl {
                if let Step::Reassign(rr) = st
                    && s % 3 == *rr
                {
                    seed_k = s.wrapping_mul(7).wrapping_add(*rr);
                }
            }
            acc = acc
                .wrapping_mul(31)
                .wrapping_add(model_enum(&defs, k, seed_k));
        }
    }
    (src, acc)
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
        let dir = std::env::temp_dir().join(format!("gt-enumpay-{}", std::process::id()));
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
fn narrow_enum_payloads_survive_moves_on_every_backend_and_match_an_independent_model() {
    let mut narrow = 0usize;
    // `GRIT_ENUMPAY_SEEDS=<n>`: chạy sâu hơn bằng tay (mặc định 30)
    let n: u64 = std::env::var("GRIT_ENUMPAY_SEEDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    for seed in 1..=n {
        let (src, want) = program(seed);
        narrow += ["u8", "u16", "bool", "i8", "i16"]
            .iter()
            .map(|t| src.matches(&format!("{t},")).count() + src.matches(&format!("{t})")).count())
            .sum::<usize>();
        for (backend, got) in run(&src, &format!("ep{seed}")) {
            assert_eq!(
                got,
                Some(want),
                "#{seed} ({backend}): payload enum sai, kỳ vọng {want}\n{src}"
            );
        }
    }
    // bộ sinh không rỗng: nhiều trường hẹp trong khai báo biến thể
    assert!(n < 30 || narrow >= 60, "quá ít trường hẹp: {narrow}");
}

/// Ca cố định nhắm thẳng nghi vấn: biến thể hẹp ghi đè TẠI CHỖ lên biến thể rộng và ngược lại, chuyển qua nhiều hàm/biến,
/// đọc payload bằng `⌥`. Kết quả tính tay (không qua mô hình bộ sinh).
#[test]
fn narrow_variant_overwriting_wide_variant_in_place_reads_back_exactly() {
    let src = "⊔ W { N(u8), M(bool, u8), Q(u64, u64) }\n\
ƒ id(e: W) → W ! ∅ { e }\n\
ƒ rd(e: W) → u64 ! ∅ { ⌥ e { N(a) ↦ a as u64, M(p, q) ↦ ⎇ p { 1000u64 +% q as u64 } ⇝ { 2000u64 +% q as u64 }, Q(a, c) ↦ a +% c } }\n\
ƒ main() → u64 ! ∅ {\n\
  μacc ≔ 0u64;\n\
  μi ≔ 0u64;\n\
  ⟳ i < 16u64 ↓ 16u64 - i {\n\
    μw ≔ Q(18446744073709551615u64 - i, 18446744073709551615u64);\n\
    w ← id(w);\n\
    ⎇ i % 2u64 == 0u64 { w ← N((i % 256u64) as u8); } ⇝ { w ← M(i % 4u64 == 1u64, (200u64 + i) as u8); }\n\
    w ← id(id(w));\n\
    o ≔ Some(w);\n\
    r ≔ ⌥ o { Some(v) ↦ rd(v), None ↦ 7u64 };\n\
    acc ← acc *% 31u64 +% r;\n\
    i ← i + 1u64;\n\
  }\n\
  acc\n\
}\n";
    let mut want = 0u64;
    for i in 0..16u64 {
        let r = if i % 2 == 0 {
            i
        } else if i % 4 == 1 {
            1000 + 200 + i
        } else {
            2000 + 200 + i
        };
        want = want.wrapping_mul(31).wrapping_add(r);
    }
    for (backend, got) in run(src, "fixed") {
        assert_eq!(got, Some(want), "({backend}): payload enum sai\n{src}");
    }
}

/// Ca cố định: enum payload hẹp nằm TRONG struct (`s.e`) và mảng (`arr[i]`), ghi đè tại chỗ bằng biến thể khác, sửa từng byte payload
/// qua `&!` + `⌥` (ghi `u8`/`u16` vào payload đã có), sao chép struct/mảng qua hàm. Không có mô hình độc lập: thông dịch MIR là chuẩn,
/// mọi hồ sơ LLVM phải khớp nó (giá trị cũng được chốt để phát hiện cả lệch chung).
#[test]
fn narrow_payload_inside_struct_and_array_mutated_through_ref_agrees_across_backends() {
    let src = r#"⊔ E { A(u8), B(bool, u8, u16), C(u64, u64), D }
▢ S { k: u8, e: E, t: bool }
ƒ bump(r: &!E, d: u8) → u64 ! ∅ {
  ⌥ r {
    E::A(x) ↦ { *x ← *x +% d; *x as u64 },
    E::B(p, x, y) ↦ { *x ← *x +% d; *y ← *y +% 1u16; ⎇ *p { 1u64 } ⇝ { 2u64 } },
    E::C(x, y) ↦ { *x ← *x +% 1u64; 3u64 },
    E::D ↦ 4u64,
  }
}
ƒ rd(e: &E) → u64 ! ∅ {
  ⌥ e {
    E::A(x) ↦ 10u64 +% *x as u64,
    E::B(p, x, y) ↦ ⎇ *p { 1000u64 +% *x as u64 *% 7u64 +% *y as u64 } ⇝ { 5000u64 +% *x as u64 +% *y as u64 *% 3u64 },
    E::C(x, y) ↦ *x +% *y,
    E::D ↦ 99u64,
  }
}
ƒ mk(i: u64) → E ! ∅ {
  ⌥ i % 4u64 {
    0u64 ↦ E::A((i % 256u64) as u8),
    1u64 ↦ E::B(i % 8u64 == 1u64, ((i *% 37u64) % 256u64) as u8, ((i *% 1234u64) % 65536u64) as u16),
    2u64 ↦ E::C(i *% 1000000007u64, 18446744073709551615u64 - i),
    _ ↦ E::D,
  }
}
ƒ pass(s: S) → S ! ∅ { s }
ƒ main() → u64 ! ∅ {
  μacc ≔ 0u64;
  μi ≔ 0u64;
  ⟳ i < 40u64 ↓ 40u64 - i {
    μs ≔ S { k: 3u8, e: mk(i *% 3u64 +% 2u64), t: i % 2u64 == 0u64 };
    s ← pass(s);
    s.e ← mk(i);
    aa ≔ bump(&!s.e, (i % 200u64) as u8);
    s ← pass(pass(s));
    μarr ≔ [mk(i +% 1u64), mk(i +% 2u64), mk(i +% 3u64)];
    arr[1] ← mk(i +% 5u64);
    bb ≔ bump(&!arr[(i % 3u64) as usize], 77u8);
    cc ≔ rd(&arr[0]) +% rd(&arr[1]) *% 3u64 +% rd(&arr[2]) *% 5u64;
    acc ← acc *% 31u64 +% aa +% bb *% 7u64 +% rd(&s.e) *% 11u64 +% cc *% 13u64 +% s.k as u64;
    i ← i + 1u64;
  }
  acc
}
"#;
    let out = run(src, "nested");
    for (backend, got) in &out {
        assert_eq!(
            *got,
            Some(15_457_300_751_181_112_113),
            "({backend}): payload enum sai\n{src}"
        );
    }
}
