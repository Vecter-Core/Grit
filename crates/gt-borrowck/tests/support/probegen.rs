#![allow(dead_code)]
//! Bộ sinh ĐỊNH HƯỚNG cho đối chiếu đột biến 361/591/554 (lệnh Giang 2026-10-14, A.3): các cấu trúc nơi chiều ngược của `relate` có thể tác dụng.
//! Mỗi chương trình là `entry(c, d, e)`; sau phần dựng là các hành động xung đột THEO THỨ TỰ NGẪU NHIÊN (ghi vào nguồn, đọc tham chiếu, đọc nguồn) nên
//! nhiều chương trình vi phạm (cố ý) và nhiều chương trình sạch.
use super::progen::{PRELUDE, Rng};

pub const KINDS: [&str; 10] = [
    "merge_branches",
    "merge_match",
    "loop_reassign",
    "call_two_args",
    "repeat_ref",
    "nested_ref",
    "self_method",
    "univ_repeat",
    "univ_method",
    "univ_pair",
];

/// Phần khai báo thêm cho kịch bản phương thức nhận `&self`.
const EXTRA: &str = "\
▢ Pt { v: i32 }
⊐ Pt {
  ƒ get(&self) → &i32 ! ∅ { &self.v }
  ƒ pick_o<'a>(&self, o: &'a i32) → &'a i32 ! ∅ { o }
}
⊐ <'a> Hs<'a> {
  ƒ m(&self, o: &'a i32) ! ∅ { }
  ƒ gt(&self) → &'a i32 ! ∅ { self.r }
  ƒ both(&self, o: &'a i32) → &'a i32 ! ∅ { o }
}
ƒ pair<'a>(x: &'a i32, y: &'a i32) → (&'a i32, &'a i32) ! ∅ { (x, y) }
";

pub struct Probe {
    pub kind: &'static str,
    pub source: String,
}

fn k(r: &mut Rng) -> i64 {
    r.below(50) as i64
}

/// các hành động xung đột: ghi nguồn, đọc nguồn, dùng tham chiếu
fn actions(r: &mut Rng, srcs: &[&str], uses: &[String]) -> String {
    let mut out = String::new();
    for _ in 0..1 + r.below(4) {
        let n = srcs.len() + uses.len() + 1;
        let i = r.below(n as u64) as usize;
        if i < srcs.len() {
            if r.chance(60) {
                out.push_str(&format!("  {} ← {};\n", srcs[i], k(r)));
            } else {
                out.push_str(&format!("  use_i({});\n", srcs[i]));
            }
        } else if i < srcs.len() + uses.len() {
            out.push_str(&format!("  {}\n", uses[i - srcs.len()]));
        } else {
            out.push_str("  use_i(0);\n");
        }
    }
    out
}

#[must_use]
pub fn generate(kind: usize, seed: u64) -> Probe {
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let (kx, ky) = (k(&mut r), k(&mut r));
    let mut b = format!("  μx ≔ {kx};\n  μy ≔ {ky};\n");
    let name = KINDS[kind % KINDS.len()];
    match name {
        // hai nguồn vào MỘT đích qua hai nhánh `⎇`
        "merge_branches" => {
            b.push_str("  μr ≔ &x;\n");
            if r.chance(70) {
                b.push_str("  ⎇ c { r ← &y; } ⇝ { }\n");
            } else {
                b.push_str("  ⎇ c { r ← &y; } ⇝ { r ← &x; }\n");
            }
            let uses = vec!["use_i(*r);".to_string()];
            b.push_str(&actions(&mut r, &["x", "y"], &uses));
            if r.chance(60) {
                b.push_str("  use_i(*r);\n");
            }
        }
        // hai nguồn vào một đích qua `⌥` (Option dựng bằng `e`)
        "merge_match" => {
            b.push_str("  μr ≔ &x;\n  o ≔ ⎇ e { Some(&y) } ⇝ { None };\n");
            b.push_str("  ⌥ o { Some(q) ↦ { r ← q; }, None ↦ { } };\n");
            let uses = vec!["use_i(*r);".to_string()];
            b.push_str(&actions(&mut r, &["x", "y"], &uses));
            if r.chance(60) {
                b.push_str("  use_i(*r);\n");
            }
        }
        // gán lại tham chiếu trong vòng lặp
        "loop_reassign" => {
            b.push_str("  μr ≔ &x;\n  ⟳ i in 0..3 {\n");
            let uses = vec!["use_i(*r);".to_string()];
            b.push_str(&actions(&mut r, &["x", "y"], &uses));
            b.push_str("    r ← &y;\n  }\n");
            if r.chance(60) {
                b.push_str("  use_i(*r);\n");
            }
        }
        // lời gọi trả tham chiếu từ hai đối số
        "call_two_args" => {
            let f = *r.pick(&["pick", "pick2", "idr_pick"]);
            let call = match f {
                "pick" => "pick(&x, &y)".to_string(),
                "pick2" => "pick2(&x, &y)".to_string(),
                _ => "idr(pick(&x, &y))".to_string(),
            };
            b.push_str(&format!("  μr ≔ {call};\n"));
            if r.chance(40) {
                b.push_str("  ⎇ c { r ← &y; } ⇝ { }\n");
            }
            let uses = vec!["use_i(*r);".to_string()];
            b.push_str(&actions(&mut r, &["x", "y"], &uses));
            if r.chance(60) {
                b.push_str("  use_i(*r);\n");
            }
        }
        // `[x; n]` với x chứa tham chiếu
        "repeat_ref" => {
            b.push_str("  μa ≔ [&x; 3];\n");
            if r.chance(60) {
                b.push_str("  a[1] ← &y;\n");
            }
            let uses = vec!["use_i(*a[0]);".to_string(), "use_i(*a[2]);".to_string()];
            b.push_str(&actions(&mut r, &["x", "y"], &uses));
            if r.chance(60) {
                b.push_str("  use_i(*a[1]);\n");
            }
        }
        // tham chiếu lồng
        "nested_ref" => {
            b.push_str("  μr1 ≔ &x;\n  μr2 ≔ &r1;\n");
            if r.chance(50) {
                b.push_str("  ⎇ c { r1 ← &y; } ⇝ { }\n");
            }
            let uses = vec!["use_i(**r2);".to_string(), "use_i(*r1);".to_string()];
            b.push_str(&actions(&mut r, &["x", "y"], &uses));
        }
        // HÀM CÓ HAI REGION PHỔ QUÁT hợp nguồn vào một vật chứa bằng `[x; n]` (nơi chiều ngược của 361 có thể tác dụng)
        "univ_repeat" => {
            let mut body = String::from(
                "  μarr ≔ [x; 2];
",
            );
            match r.below(4) {
                0 => body.push_str(
                    "  ⎇ c { arr ← [y; 2]; } ⇝ { }
",
                ),
                1 => body.push_str(
                    "  arr[1] ← y;
",
                ),
                2 => body.push_str(
                    "  μarr2 ≔ [y; 2];
  ⎇ c { arr ← arr2; } ⇝ { }
",
                ),
                _ => body.push_str(
                    "  ⎇ c { arr ← [y; 3]; } ⇝ { arr ← [x; 3]; }
",
                ),
            }
            if r.chance(70) {
                body.push_str(
                    "  use_i(*arr[0]);
",
                );
            }
            let helper = format!(
                "ƒ t<'a, 'b>(x: &'a i32, y: &'b i32, c: bool) ! ∅ {{
{body}}}
"
            );
            return univ_finish("univ_repeat", &helper, "t(&x, &y, c);", kx, ky);
        }
        // phương thức nhận `&self` (pointee có region) với đối số từ region phổ quát KHÁC (nơi 554 tác dụng: &self → &!self)
        "univ_method" => {
            let call = match r.below(6) {
                0 => "  h.m(y);
"
                .to_string(),
                1 => "  μq ≔ h.both(y);
  use_i(*q);
"
                .to_string(),
                2 => "  ⎇ c { h.m(y); } ⇝ { h.m(h.gt()); }
"
                .to_string(),
                // gọi theo đường dẫn: đối số là CHÍNH `h` (không qua tạm reborrow)
                3 => "  Hs::m(h, y);
"
                .to_string(),
                4 => "  μq ≔ Hs::both(h, y);
  use_i(*q);
"
                .to_string(),
                _ => "  ⎇ c { Hs::m(h, y); } ⇝ { Hs::m(h, Hs::gt(h)); }
"
                .to_string(),
            };
            let helper = format!(
                "ƒ t<'p, 'q, 's>(h: &'p Hs<'q>, y: &'s i32, c: bool) ! ∅ {{
{call}}}
"
            );
            return univ_finish(
                "univ_method",
                &helper,
                "z ≔ Hs { r: &x }; t(&z, &y, c);",
                kx,
                ky,
            );
        }
        // kết quả lời gọi có HAI vị trí region cùng một biến của callee (nơi 591 có thể tác dụng)
        "univ_pair" => {
            // biến thể HAI VỊ TRÍ region cùng biến của callee + một region phổ quát thứ ba trộn vào MỘT trong hai vị trí (591 tác dụng)
            if r.chance(60) {
                let variant = match r.below(3) {
                    0 => "  ⎇ c { p ← (w, y); } ⇝ { }\n",
                    1 => "  ⎇ c { p ← (w, y); } ⇝ { p ← (w, x); }\n",
                    _ => "  p ← (w, y);\n",
                };
                let helper = format!(
                    "ƒ t<'p, 'z>(x: &'p i32, y: &'p i32, w: &'z i32, c: bool) → &'p i32 ! ∅ {{\n  μp ≔ pair(x, y);\n{variant}  p.1\n}}\n"
                );
                return univ_finish(
                    "univ_pair",
                    &helper,
                    "μw ≔ 9;\n  r ≔ t(&x, &y, &w, c);\n  use_i(*r);",
                    kx,
                    ky,
                );
            }
            let tail = match r.below(3) {
                0 => {
                    "  use_i(*p.0);
  use_i(*p.1);
"
                }
                1 => {
                    "  ⎇ c { p ← pair(y, x); } ⇝ { }
  use_i(*p.0);
"
                }
                _ => {
                    "  μw ≔ p.0;
  ⎇ c { w ← p.1; } ⇝ { }
  use_i(*w);
"
                }
            };
            let helper = format!(
                "ƒ t<'a, 'b>(x: &'a i32, y: &'b i32, c: bool) ! ∅ {{
  μp ≔ pair(x, y);
{tail}}}
"
            );
            return univ_finish("univ_pair", &helper, "t(&x, &y, c);", kx, ky);
        }
        // phương thức nhận `&self` trả tham chiếu
        _ => {
            b.push_str(&format!("  μp ≔ Pt {{ v: {} }};\n", k(&mut r)));
            let how = r.below(3);
            let init = match how {
                0 => "  μr ≔ p.get();\n",
                1 => "  μr ≔ p.pick_o(&y);\n",
                _ => {
                    "  μr ≔ p.pick_o(p.get());
"
                }
            };
            b.push_str(init);
            if r.chance(40) {
                b.push_str("  ⎇ c { r ← &y; } ⇝ { }\n");
            }
            let mut uses = vec!["use_i(*r);".to_string()];
            uses.push("use_i(*p.get());".to_string());
            let mut acts = actions(&mut r, &["y"], &uses);
            if r.chance(50) {
                acts.push_str("  p.v ← 7;\n");
            }
            b.push_str(&acts);
            if r.chance(60) {
                b.push_str("  use_i(*r);\n");
            }
        }
    }
    let source = format!(
        "{PRELUDE}{EXTRA}ƒ idr_pick<'a>(x: &'a i32) → &'a i32 ! ∅ {{ x }}\nƒ entry(c: bool, d: bool, e: bool) → i32 ! {{mem}} {{\n{b}  0\n}}\n"
    );
    Probe { kind: name, source }
}

/// ghép hàm trợ giúp (có tham số tham chiếu phổ quát) với `entry(c, d, e)` gọi nó bằng tham chiếu tới biến cục bộ
fn univ_finish(kind: &'static str, helper: &str, call: &str, kx: i64, ky: i64) -> Probe {
    let source = format!(
        "{PRELUDE}{EXTRA}{helper}ƒ entry(c: bool, d: bool, e: bool) → i32 ! {{mem}} {{
  μx ≔ {kx};
  μy ≔ {ky};
  {call}
  0
}}
"
    );
    Probe { kind, source }
}
