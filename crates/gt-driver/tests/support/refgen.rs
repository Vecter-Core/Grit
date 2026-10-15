//! Bộ sinh chương trình khớp mẫu qua tham chiếu (ADR-040, Q87) với MÔ HÌNH tính kết quả bằng Rust (độc lập với hạ MIR). Phủ: biến thể 0/1/2/3 trường,
//! struct (đủ trường, `..`), tuple trên `&(A, B)` và trên `(&A, &B)`, lồng 2 tầng cấu trúc (không phải tham chiếu lồng), `&U` và `&!U`; cộng ca ĐỎ có chủ ý:
//! mẫu hằng trên `&U` (E0301), tham chiếu lồng (E0301), thiếu nhánh (E0305), dùng bind như giá trị (E0301).
#![allow(dead_code)]
#[path = "../../../gt-mir/tests/support/progen.rs"]
mod progen;
pub use progen::Rng;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expect {
    Value(u32),
    Red(&'static str),
}

pub struct RefProg {
    pub src: String,
    pub expect: Expect,
    /// các dạng đã phủ (cho thống kê)
    pub forms: Vec<&'static str>,
}

/// một thân nhánh: `writes` (chỉ `&!`) rồi tổng `Σ field_i *% mult_i +% konst` (đọc SAU khi ghi)
#[derive(Clone)]
struct Arm {
    writes: Vec<(usize, u32)>,
    terms: Vec<(usize, u32)>,
    konst: u32,
}

fn arm(rng: &mut Rng, arity: usize, unique: bool) -> Arm {
    let mut writes = Vec::new();
    if unique {
        for i in 0..arity {
            if rng.chance(70) {
                writes.push((i, 1 + rng.below(9000) as u32));
            }
        }
    }
    let mut terms = Vec::new();
    for i in 0..arity {
        if rng.chance(80) {
            terms.push((i, 1 + rng.below(40) as u32));
        }
    }
    Arm {
        writes,
        terms,
        konst: rng.below(100) as u32,
    }
}

fn names(prefix: &str, n: usize) -> Vec<String> {
    (0..n).map(|i| format!("{prefix}{i}")).collect()
}

/// các tên bind cần cho một nhánh (`_` cho trường không dùng)
fn binders(a: &Arm, arity: usize, prefix: &str) -> Vec<String> {
    let ns = names(prefix, arity);
    (0..arity)
        .map(|i| {
            if a.writes.iter().any(|w| w.0 == i) || a.terms.iter().any(|t| t.0 == i) {
                ns[i].clone()
            } else {
                "_".to_string()
            }
        })
        .collect()
}

fn body(a: &Arm, prefix: &str, unique: bool) -> String {
    let ns = names(prefix, 8);
    let mut s = String::new();
    if unique && !a.writes.is_empty() {
        s.push_str("{ ");
        for (i, k) in &a.writes {
            s.push_str(&format!("*{} ← *{} +% {}; ", ns[*i], ns[*i], k));
        }
    }
    let mut e = String::new();
    for (i, m) in &a.terms {
        e.push_str(&format!("*{} *% {} +% ", ns[*i], m));
    }
    e.push_str(&format!("{}", a.konst));
    if unique && !a.writes.is_empty() {
        s.push_str(&e);
        s.push_str(" }");
        s
    } else {
        e
    }
}

fn apply(a: &Arm, vals: &mut [u32]) -> u32 {
    for (i, k) in &a.writes {
        vals[*i] = vals[*i].wrapping_add(*k);
    }
    let mut r = 0u32;
    for (i, m) in &a.terms {
        r = r.wrapping_add(vals[*i].wrapping_mul(*m));
    }
    r.wrapping_add(a.konst)
}

fn pat_args(bs: &[String]) -> String {
    if bs.is_empty() {
        String::new()
    } else {
        format!("({})", bs.join(", "))
    }
}

pub fn program(seed: u64) -> RefProg {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xA5A5_1234_5678_9ABC | 1);
    if seed % 6 == 5 {
        return red_program(&mut rng);
    }
    let mut src = String::new();
    let mut forms: Vec<&'static str> = Vec::new();
    let mut main = String::from("ƒ main() → u32 ! ∅ {\n  μacc ≔ 0u32;\n");
    let mut acc = 0u32;
    let mut add = |main: &mut String, call: String, v: u32| {
        main.push_str(&format!("  acc ← acc +% {call};\n"));
        acc = acc.wrapping_add(v);
    };
    // ---- enum 0..3 trường, &E và &!E
    src.push_str("⊔ E { V0, V1(u32), V2(u32, u32), V3(u32, u32, u32) }\n");
    for unique in [false, true] {
        let tag = if unique { "un" } else { "sh" };
        let arms: Vec<Arm> = (0..4).map(|ar| arm(&mut rng, ar, unique)).collect();
        let mut order: Vec<usize> = (0..4).collect();
        for i in (1..4).rev() {
            let j = rng.below(i as u64 + 1) as usize;
            order.swap(i, j);
        }
        let mut f = format!(
            "ƒ e_{tag}(r: {}E) → u32 ! ∅ {{ ⌥ r {{ ",
            if unique { "&!" } else { "&" }
        );
        let mut parts = Vec::new();
        for &v in &order {
            let bs = binders(&arms[v], v, "a");
            parts.push(format!(
                "E::V{v}{} ↦ {}",
                pat_args(&bs),
                body(&arms[v], "a", unique)
            ));
        }
        f.push_str(&parts.join(", "));
        f.push_str(" } }\n");
        src.push_str(&f);
        forms.push(if unique {
            "ref-unique(&!U)"
        } else {
            "ref-shared(&U)"
        });
        if unique && arms.iter().any(|a| !a.writes.is_empty()) {
            forms.push("unique-with-write");
        }
        forms.extend([
            "variant-0-fields",
            "variant-1-field",
            "variant-2-fields",
            "variant-3-fields",
        ]);
        for v in 0..4usize {
            let vals: Vec<u32> = (0..v).map(|_| rng.below(1000) as u32).collect();
            let lit = if v == 0 {
                "E::V0".to_string()
            } else {
                format!(
                    "E::V{v}({})",
                    vals.iter()
                        .map(|x| format!("{x}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            let mut st = vals.clone();
            if unique {
                main.push_str(&format!("  μv{tag}{v} ≔ {lit};\n"));
                let r = apply(&arms[v], &mut st);
                add(&mut main, format!("e_un(&!v{tag}{v})"), r);
                let sum = st.iter().fold(0u32, |a, x| a.wrapping_add(*x));
                add(&mut main, format!("e_sum(&v{tag}{v})"), sum);
            } else {
                let r = apply(&arms[v], &mut st);
                add(&mut main, format!("e_sh(&{lit})"), r);
            }
        }
    }
    // ---- tên trần và `_` trên &E (không đổi: tên trần bind chính tham chiếu; `_` phủ phần còn lại)
    src.push_str(
        "ƒ bare(r: &E) → u32 ! ∅ { ⌥ r { E::V0 ↦ 1, _ ↦ 2 } }
ƒ bare2(r: &E) → u32 ! ∅ { ⌥ r { x ↦ e_sum(x) } }
",
    );
    for v in 0..4usize {
        let vals: Vec<u32> = (0..v).map(|_| rng.below(1000) as u32).collect();
        let lit = if v == 0 {
            "E::V0".to_string()
        } else {
            format!(
                "E::V{v}({})",
                vals.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        add(
            &mut main,
            format!("bare(&{lit})"),
            if v == 0 { 1 } else { 2 },
        );
        add(
            &mut main,
            format!("bare2(&{lit})"),
            vals.iter().fold(0u32, |a, x| a.wrapping_add(*x)),
        );
    }
    forms.extend(["bare-name", "wildcard"]);
    src.push_str("ƒ e_sum(r: &E) → u32 ! ∅ { ⌥ r { E::V0 ↦ 0, E::V1(a) ↦ *a, E::V2(a, b) ↦ *a +% *b, E::V3(a, b, c) ↦ *a +% *b +% *c } }\n");
    // ---- struct
    src.push_str("▢ P { x: u32, y: u32, z: u32 }\n");
    for unique in [false, true] {
        let tag = if unique { "un" } else { "sh" };
        let a = arm(&mut rng, 3, unique);
        let rest = rng.chance(40);
        let used: Vec<usize> = (0..3)
            .filter(|i| a.writes.iter().any(|w| w.0 == *i) || a.terms.iter().any(|t| t.0 == *i))
            .collect();
        let fields = ["x", "y", "z"];
        let mut pats = Vec::new();
        for (i, f) in fields.iter().enumerate() {
            if used.contains(&i) {
                pats.push(format!("{f}: p{i}"));
            } else if !rest {
                pats.push(format!("{f}: _"));
            }
        }
        if rest {
            pats.push("..".to_string());
        }
        src.push_str(&format!(
            "ƒ p_{tag}(r: {}P) → u32 ! ∅ {{ ⌥ r {{ P {{ {} }} ↦ {} }} }}\n",
            if unique { "&!" } else { "&" },
            pats.join(", "),
            body(&a, "p", unique)
        ));
        let vals: Vec<u32> = (0..3).map(|_| rng.below(1000) as u32).collect();
        let mut st = vals.clone();
        let r = apply(&a, &mut st);
        if unique {
            main.push_str(&format!(
                "  μpv ≔ P {{ x: {}, y: {}, z: {} }};\n",
                vals[0], vals[1], vals[2]
            ));
            add(&mut main, "p_un(&!pv)".to_string(), r);
            add(
                &mut main,
                "p_sum(&pv)".to_string(),
                st.iter().fold(0u32, |a, x| a.wrapping_add(*x)),
            );
        } else {
            add(
                &mut main,
                format!(
                    "p_sh(&P {{ x: {}, y: {}, z: {} }})",
                    vals[0], vals[1], vals[2]
                ),
                r,
            );
        }
        forms.push(if rest {
            "struct-with-rest(..)"
        } else {
            "struct-all-fields"
        });
        if unique && !a.writes.is_empty() {
            forms.push("unique-with-write");
        }
    }
    src.push_str("ƒ p_sum(r: &P) → u32 ! ∅ { ⌥ r { P { x, y, z } ↦ *x +% *y +% *z } }\n");
    // ---- tuple (A, B) qua tham chiếu, và tuple của tham chiếu
    let (m1, m2) = (1 + rng.below(30) as u32, 1 + rng.below(30) as u32);
    let (w1, w2) = (1 + rng.below(500) as u32, 1 + rng.below(500) as u32);
    src.push_str(&format!(
        "ƒ t_sh(r: &(u32, u32)) → u32 ! ∅ {{ ⌥ r {{ (a, b) ↦ *a *% {m1} +% *b *% {m2} }} }}
ƒ t_un(r: &!(u32, u32)) → u32 ! ∅ {{ ⌥ r {{ (a, b) ↦ {{ *a ← *a +% {w1}; *b ← *b +% {w2}; *a *% {m1} +% *b *% {m2} }} }} }}
ƒ t_refs(r: (&u32, &u32)) → u32 ! ∅ {{ ⌥ r {{ (a, b) ↦ *a *% {m1} +% *b *% {m2} }} }}
"
    ));
    let (ta, tb) = (rng.below(1000) as u32, rng.below(1000) as u32);
    let f = |a: u32, b: u32| a.wrapping_mul(m1).wrapping_add(b.wrapping_mul(m2));
    add(&mut main, format!("t_sh(&({ta}, {tb}))"), f(ta, tb));
    main.push_str(&format!("  μtv ≔ ({ta}, {tb});\n"));
    let (na, nb) = (ta.wrapping_add(w1), tb.wrapping_add(w2));
    add(&mut main, "t_un(&!tv)".to_string(), f(na, nb));
    add(&mut main, "t_sh(&tv)".to_string(), f(na, nb));
    add(&mut main, format!("t_refs((&{ta}, &{tb}))"), f(ta, tb));
    forms.extend([
        "tuple-on-&(A,B)",
        "tuple-on-&!(A,B)",
        "tuple-of-refs(&A,&B)",
    ]);
    forms.push("unique-with-write");
    // ---- lồng 2 tầng cấu trúc (không phải tham chiếu lồng): enum chứa struct
    src.push_str("⊔ Q { N(P), M(u32, P) }\n");
    let (k1, k2) = (1 + rng.below(20) as u32, 1 + rng.below(20) as u32);
    src.push_str(&format!(
        "ƒ q_sh(r: &Q) → u32 ! ∅ {{ ⌥ r {{ Q::N(P {{ x: a, .. }}) ↦ *a *% {k1}, Q::M(k, P {{ y: b, z: c, .. }}) ↦ *k +% *b *% {k2} +% *c }} }}
ƒ q_un(r: &!Q) → u32 ! ∅ {{ ⌥ r {{ Q::N(P {{ x: a, .. }}) ↦ {{ *a ← *a +% 7; *a *% {k1} }}, Q::M(k, P {{ y: b, z: c, .. }}) ↦ {{ *k ← *k +% 3; *b ← *b +% 5; *k +% *b *% {k2} +% *c }} }} }}
"
    ));
    let (x, y, z, k) = (
        rng.below(100) as u32,
        rng.below(100) as u32,
        rng.below(100) as u32,
        rng.below(100) as u32,
    );
    add(
        &mut main,
        format!("q_sh(&Q::N(P {{ x: {x}, y: {y}, z: {z} }}))"),
        x.wrapping_mul(k1),
    );
    add(
        &mut main,
        format!("q_sh(&Q::M({k}, P {{ x: {x}, y: {y}, z: {z} }}))"),
        k.wrapping_add(y.wrapping_mul(k2)).wrapping_add(z),
    );
    main.push_str(&format!(
        "  μq1 ≔ Q::N(P {{ x: {x}, y: {y}, z: {z} }});\n  μq2 ≔ Q::M({k}, P {{ x: {x}, y: {y}, z: {z} }});\n"
    ));
    add(
        &mut main,
        "q_un(&!q1)".to_string(),
        x.wrapping_add(7).wrapping_mul(k1),
    );
    let (k3, y5) = (k.wrapping_add(3), y.wrapping_add(5));
    add(
        &mut main,
        "q_un(&!q2)".to_string(),
        k3.wrapping_add(y5.wrapping_mul(k2)).wrapping_add(z),
    );
    add(
        &mut main,
        "q_sh(&q1)".to_string(),
        x.wrapping_add(7).wrapping_mul(k1),
    );
    add(
        &mut main,
        "q_sh(&q2)".to_string(),
        k3.wrapping_add(y5.wrapping_mul(k2)).wrapping_add(z),
    );
    forms.extend(["nested-two-levels", "unique-with-write"]);
    main.push_str("  acc\n}\n");
    src.push_str(&main);
    RefProg {
        src,
        expect: Expect::Value(acc),
        forms,
    }
}

/// ca ĐỎ có chủ ý theo seed (kiểm biên dịch, không cần backend)
pub fn red(seed: u64) -> RefProg {
    red_program(&mut Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ 0x5EED_0000_1111_2222
        | 1))
}

fn red_program(rng: &mut Rng) -> RefProg {
    let k = rng.below(4);
    let (src, code, form) = match k {
        0 => (
            "ƒ f(r: &u32) → u32 ! ∅ { ⌥ r { 0 ↦ 1, _ ↦ 2 } }\nƒ main() → u32 ! ∅ { f(&3) }\n",
            "E0301",
            "literal-on-ref",
        ),
        1 => (
            "▢ P { x: u32 }\nƒ f(o: &Option<&P>) → u32 ! ∅ { ⌥ o { Some(P { x }) ↦ *x, None ↦ 0 } }\nƒ main() → u32 ! ∅ { 0 }\n",
            "E0301",
            "nested-ref",
        ),
        2 => (
            "ƒ f(o: &Option<u32>) → u32 ! ∅ { ⌥ o { Some(v) ↦ *v } }\nƒ main() → u32 ! ∅ { f(&Some(1)) }\n",
            "E0305",
            "nonexhaustive",
        ),
        _ => (
            "ƒ f(o: &Option<u32>) → u32 ! ∅ { ⌥ o { Some(v) ↦ v, None ↦ 0 } }\nƒ main() → u32 ! ∅ { f(&Some(1)) }\n",
            "E0301",
            "bind-as-value",
        ),
    };
    RefProg {
        src: src.to_string(),
        expect: Expect::Red(code),
        forms: vec![form],
    }
}
