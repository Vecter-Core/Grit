//! ADR-P2-02 bước 1: `gt-layout` (Rust thuần) phải KHỚP `DataLayout` của LLVM về cỡ/căn lề với mọi kiểu codegen hạ (kiểm ngay trong `lower_ty`, bản dựng debug;
//! lệch ⇒ ICE "gt-layout lệch…"). Test này dựng một chương trình dùng NHIỀU hình dạng kiểu để cả kiểm đó lẫn gt-layout bị thử thật (đột biến: đổi căn lề `char`
//! làm test này ĐỎ, trong khi bộ `llvm` cũ không có `char` nên sống).
#![cfg(feature = "llvm")]
use gt_driver::{Options, check};
use gt_resolve::SourceInput;

const SRC: &str = "\
▢ P { a: u8, b: u32, c: u8 }
▢ Q { p: P, ch: char, x: f64, big: u128, n: i16 }
⊔ E { A, B(u8), C(u64, u8), D(P), F(char, u16) }
⊔ G<T> { N, S(T), Pair(T, T) }
#level(\"10\")
▢ L1 { }
ƒ main() → u32 ! ∅ {
  q ≔ Q { p: P { a: 1u8, b: 2u32, c: 3u8 }, ch: 'z', x: 1.5, big: 7u128, n: -3i16 };
  e1 ≔ E::C(5u64, 6u8);
  e2 ≔ E::F('k', 9u16);
  g1: G<u8> ≔ G::S(4u8);
  g2: G<(u8, char)> ≔ G::Pair((1u8, 'a'), (2u8, 'b'));
  arr: [(u8, u32); 3] ≔ [(1u8, 2u32), (3u8, 4u32), (5u8, 6u32)];
  sl: &[u32] ≔ &[1u32, 2u32, 3u32];
  s: &str ≔ \"héllo\";
  o: Option<(u8, char)> ≔ Some((1u8, 'x'));
  r: Result<u64, u8> ≔ Ok(9u64);
  m: Mutex<u16, L1> ≔ Mutex::new(5u16);
  b ≔ true;
  f: ƒ(u8) → u8 ! ∅ ≔ idu8;
  _x ≔ (q, e1, e2, g1, g2, arr, sl, s, o, r, b, f);
  0u32
}
ƒ idu8(x: u8) → u8 ! ∅ { x }
";

#[test]
fn gt_layout_agrees_with_llvm_datalayout_on_a_zoo_of_types() {
    let res = check(
        &[SourceInput {
            name: "main.gt".to_string(),
            text: SRC.to_string(),
        }],
        &Options::default(),
    )
    .expect("check");
    let errs: Vec<String> = res
        .report
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str().starts_with('E'))
        .map(|d| format!("{}: {}", d.code.as_str(), d.message))
        .collect();
    assert!(errs.is_empty(), "gritc từ chối: {errs:?}");
    let att = {
        let art = res.open_gate().expect("cổng");
        gt_driver::pipeline::attestation(&res, &gt_driver::pipeline::mir_dump(art))
    };
    let dir = std::env::temp_dir().join(format!("gt-layout-diff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo");
    let exe = dir.join("zoo.exe");
    gt_driver::native::build_exe_profile(
        res.open_gate().expect("cổng"),
        &exe,
        true,
        &att,
        gt_codegen_llvm::Opt::Dev,
    )
    .unwrap_or_else(|e| panic!("backend (gồm đối chiếu gt-layout): {e}"));
    let _ = std::fs::remove_file(&exe);
}
