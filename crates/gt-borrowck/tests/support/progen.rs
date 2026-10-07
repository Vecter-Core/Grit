//! Bộ sinh chương trình Grit cho oracle động (ADR-024): các "kịch bản" (move, mượn, reborrow, vòng lặp, match, move một
//! phần, Arena, lời gọi theo `RegionSig`, struct lồng) ghép ngẫu nhiên; mỗi chương trình là `entry(c: bool)` — chạy cả `c = false`
//! và `c = true` để phủ cả hai nhánh. Thứ tự hành động trong một kịch bản là NGẪU NHIÊN nên nhiều chương trình vi phạm (cố ý).
//! Chương trình được giữ ở dạng CÂY (`St`) để áp phép đột biến ngữ nghĩa ("tấn công") rồi in lại.
#![allow(dead_code)]
use std::collections::BTreeSet;

pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
    pub fn chance(&mut self, pct: u64) -> bool {
        self.below(100) < pct
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

/// Phần khai báo chung (giống header của corpus M6 + vài hàm cho kịch bản).
pub const PRELUDE: &str = "\
▢ B { v: i32 }
▢ W { b: B, n: i32 }
▢ Hs<'a> { r: &'a i32 }
ƒ eat(b: B) ! ∅ { }
ƒ eatw(w: W) ! ∅ { }
ƒ eata(a: Arena) ! {mem} { }
ƒ peek(b: &B) → i32 ! ∅ { b.v }
ƒ bump(b: &!B) ! ∅ { b.v ← b.v +% 1; }
ƒ mk() → B ! ∅ { B { v: 0 } }
ƒ mkw() → W ! ∅ { W { b: B { v: 0 }, n: 0 } }
ƒ two(a: &!i32, b: &!i32) ! ∅ { *a ← 1; *b ← 2; }
ƒ mix(a: &i32, b: &!i32) ! ∅ { *b ← *a; }
ƒ idr<'a>(x: &'a i32) → &'a i32 ! ∅ { x }
ƒ pick<'a>(x: &'a i32, y: &'a i32) → &'a i32 ! ∅ { x }
ƒ pick2<'a, 'b>(x: &'a i32, y: &'b i32) → &'a i32 ! ∅ { x }
ƒ first(a: &[i32]) → &i32 ! ∅ { &a[0] }
ƒ use_i(x: i32) ! ∅ { }
▢ Hm<'a> { r: &!'a i32 }
ƒ idg<T>(x: T) → T ! ∅ { x }
ƒ takes(r: &!i32) ! ∅ { *r ← 1; }
ƒ look(r: &i32) ! ∅ { use_i(*r); }
";

/// Loại cấu trúc cần đo độ phủ (ADR-024 §2).
pub const KINDS: [&str; 9] = [
    "move",
    "reborrow",
    "borrow_in_loop",
    "borrow_via_match",
    "partial_move",
    "arena_reset",
    "call_regionsig",
    "nested_struct",
    "ref_flow",
];

#[derive(Clone, Debug, PartialEq)]
pub enum Role {
    Plain,
    /// tạo tham chiếu `var` tới chỗ nguyên thuỷ (i32) `base`
    MakeRef {
        var: String,
        base: String,
        unique: bool,
    },
    UseRef {
        var: String,
    },
    /// `var ≔ arena.alloc(..)`
    MakeArenaRef {
        var: String,
        arena: String,
    },
    /// move cả biến kiểu `B`
    Move {
        var: String,
    },
}

#[derive(Clone, Debug)]
pub struct Raw {
    pub text: String,
    pub role: Role,
}

#[derive(Clone, Debug)]
pub enum St {
    Raw(Raw),
    Branch(Vec<St>, Vec<St>),
    Loop(u8, Vec<St>),
    Match {
        scrut: String,
        pv: String,
        some: Vec<St>,
        none: Vec<St>,
    },
}

fn raw(text: impl Into<String>) -> St {
    St::Raw(Raw {
        text: text.into(),
        role: Role::Plain,
    })
}

fn raw_r(text: impl Into<String>, role: Role) -> St {
    St::Raw(Raw {
        text: text.into(),
        role,
    })
}

#[derive(Clone, Debug)]
pub struct Prog {
    pub body: Vec<St>,
    pub tags: BTreeSet<&'static str>,
}

impl Prog {
    #[must_use]
    pub fn source(&self) -> String {
        let mut out = String::from(PRELUDE);
        out.push_str("ƒ entry(c: bool, d: bool, e: bool) → i32 ! {mem} {\n");
        print_block(&self.body, 1, 0, &mut out);
        out.push_str("  0\n}\n");
        out
    }
}

/// `depth` = số `⎇` bao quanh: mỗi tầng một điều kiện RIÊNG (`c`, `d`, `e`) để mọi tổ hợp nhánh lồng nhau đều TỚI ĐƯỢC
/// (dùng chung một điều kiện làm nhánh `else` của `⎇ c` chứa `⎇ c` không bao giờ chạy). `Option` được dựng bằng `e`.
fn print_block(b: &[St], ind: usize, depth: usize, out: &mut String) {
    let pad = "  ".repeat(ind);
    for s in b {
        match s {
            St::Raw(r) => out.push_str(&format!("{pad}{}\n", r.text)),
            St::Branch(a, e) => {
                let cond = ['c', 'd', 'e'][depth.min(2)];
                out.push_str(&format!("{pad}⎇ {cond} {{\n"));
                print_block(a, ind + 1, depth + 1, out);
                if e.is_empty() {
                    out.push_str(&format!("{pad}}};\n"));
                } else {
                    out.push_str(&format!("{pad}}} ⇝ {{\n"));
                    print_block(e, ind + 1, depth + 1, out);
                    out.push_str(&format!("{pad}}};\n"));
                }
            }
            St::Loop(k, body) => {
                out.push_str(&format!("{pad}⟳ i{ind} in 0..{k} {{\n"));
                print_block(body, ind + 1, depth, out);
                out.push_str(&format!("{pad}}}\n"));
            }
            St::Match {
                scrut,
                pv,
                some,
                none,
            } => {
                out.push_str(&format!("{pad}⌥ {scrut} {{\n"));
                out.push_str(&format!("{pad}  Some({pv}) ↦ {{\n"));
                print_block(some, ind + 2, depth, out);
                out.push_str(&format!("{pad}  }},\n{pad}  None ↦ {{\n"));
                print_block(none, ind + 2, depth, out);
                out.push_str(&format!("{pad}  }}\n{pad}}};\n"));
            }
        }
    }
}

pub struct Gen {
    pub rng: Rng,
    n: usize,
    tags: BTreeSet<&'static str>,
}

impl Gen {
    #[must_use]
    pub fn new(seed: u64) -> Gen {
        Gen {
            rng: Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1),
            n: 0,
            tags: BTreeSet::new(),
        }
    }

    fn fresh(&mut self, p: &str) -> String {
        self.n += 1;
        format!("{p}{}", self.n)
    }

    fn k(&mut self) -> i64 {
        self.rng.below(50) as i64
    }

    /// Một chương trình: 1–3 kịch bản nối tiếp, mỗi kịch bản có thể nằm trong nhánh `⎇ c` hay vòng lặp.
    pub fn program(&mut self) -> Prog {
        self.tags.clear();
        let mut body = Vec::new();
        for _ in 0..1 + self.rng.below(3) {
            let sc = self.scenario();
            match self.rng.below(10) {
                0 | 1 => {
                    let other = if self.rng.chance(50) {
                        Vec::new()
                    } else {
                        self.scenario()
                    };
                    body.push(St::Branch(sc, other));
                }
                2 => {
                    let k = 2 + self.rng.below(2) as u8; // ≥ 2: lần lặp thứ hai lộ ra vi phạm qua cạnh lùi
                    if sc.iter().any(has_ref_role) {
                        self.tags.insert("borrow_in_loop");
                    }
                    body.push(St::Loop(k, sc));
                }
                _ => body.extend(sc),
            }
        }
        Prog {
            body,
            tags: self.tags.clone(),
        }
    }

    fn scenario(&mut self) -> Vec<St> {
        match self.rng.below(100) {
            0..=19 => self.int_borrow(),
            20..=31 => self.reborrow(),
            32..=47 => self.moves(),
            48..=55 => self.partial_move_nested(),
            56..=65 => self.match_scenario(),
            66..=76 => self.arena(),
            77..=86 => self.calls(),
            87..=92 => self.holder(),
            _ => self.flow(),
        }
    }

    /// Tham chiếu CHẢY qua vật chứa (tuple, mảng, `Option`, `Hs`, `Hm` giữ `&!`, `&&`) bằng sao chép/di chuyển/dựng lại/hàm generic,
    /// rồi hành động xung đột theo thứ tự ngẫu nhiên. Phủ các ràng buộc `relate`/`walk` của borrowck (hai chiều: thiếu ràng buộc
    /// ⇒ âm tính giả; thừa ràng buộc ⇒ ứng viên dương tính giả "KHÁC").
    fn flow(&mut self) -> Vec<St> {
        self.tags.insert("ref_flow");
        let x = self.fresh("x");
        let y = self.fresh("y");
        let chain = self.fresh("ch");
        let (kx, ky) = (self.k(), self.k());
        let kind = self.rng.below(6);
        let c0 = self.fresh("c");
        let unique = kind == 4;
        let mut out = vec![raw(format!("μ{x} ≔ {kx};")), raw(format!("μ{y} ≔ {ky};"))];
        let make = match kind {
            0 => format!("{c0} ≔ (&{x}, &{y});"),
            1 => format!("{c0} ≔ [&{x}, &{y}];"),
            2 => format!("{c0} ≔ Some(&{x});"),
            3 => format!("{c0} ≔ Hs {{ r: &{x} }};"),
            4 => format!("{c0} ≔ Hm {{ r: &!{x} }};"),
            _ => format!("{c0}r ≔ &{x}; {c0} ≔ &{c0}r;"),
        };
        out.push(raw_r(
            make,
            Role::MakeRef {
                var: chain.clone(),
                base: x.clone(),
                unique,
            },
        ));
        let mut cur = c0;
        for _ in 0..self.rng.below(3) {
            let next = self.fresh("c");
            let st = match (kind, self.rng.below(3)) {
                (_, 2) => format!("{next} ≔ idg({cur});"),
                (0, 0) => format!("{next} ≔ {cur};"),
                (0, _) => format!("{next} ≔ ({cur}.0, {cur}.1);"),
                (1, 0) => format!("{next} ≔ {cur};"),
                (1, _) => format!("{next} ≔ [{cur}[0], {cur}[1]];"),
                (2, 0) => format!("{next} ≔ {cur};"),
                (2, _) => format!("{next} ≔ ⌥ {cur} {{ Some(q) ↦ Some(q), None ↦ None }};"),
                (3, 0) => format!("{next} ≔ {cur};"),
                (3, _) => format!("{next} ≔ Hs {{ r: {cur}.r }};"),
                (4, 0) => format!("{next} ≔ {cur};"),
                (4, _) => format!("{next} ≔ Hm {{ r: {cur}.r }};"),
                (_, 0) => format!("{next} ≔ {cur};"),
                (_, _) => format!("{next} ≔ &*{cur};"),
            };
            out.push(raw(st));
            cur = next;
        }
        let read = match kind {
            0 => format!("use_i(*{cur}.0);"),
            1 => format!("use_i(*{cur}[0]);"),
            2 => format!("⌥ {cur} {{ Some(q) ↦ {{ use_i(*q); }}, None ↦ {{ }} }};"),
            3 => format!("use_i(*{cur}.r);"),
            4 => format!("*{cur}.r ← 3;"),
            _ => format!("use_i(**{cur});"),
        };
        // đọc phần tử thứ HAI (trỏ vào `y`): không phải lần dùng của khoản vay trên `x` nên KHÔNG mang vai `UseRef`
        let read_y = match kind {
            0 => Some(format!("use_i(*{cur}.1);")),
            1 => Some(format!("use_i(*{cur}[1]);")),
            _ => None,
        };
        for _ in 0..1 + self.rng.below(4) {
            let k = self.k();
            out.push(match self.rng.below(6) {
                5 if read_y.is_some() => raw(read_y.clone().unwrap_or_default()),
                5 => raw(format!("use_i({y});")),
                0 | 1 => raw_r(read.clone(), Role::UseRef { var: chain.clone() }),
                2 => raw(format!("{x} ← {k};")),
                3 => raw(format!("{y} ← {k};")),
                _ => raw(format!("use_i({x});")),
            });
        }
        out
    }

    // ---------------------------------------------------------------- kịch bản

    /// `μx ≔ k; r ≔ &x / &!x;` rồi các hành động ngẫu nhiên (có thể nằm trong vòng lặp).
    fn int_borrow(&mut self) -> Vec<St> {
        let x = self.fresh("x");
        let r = self.fresh("r");
        let unique = self.rng.chance(45);
        let k0 = self.k();
        let mut out = vec![raw(format!("μ{x} ≔ {k0};"))];
        out.push(raw_r(
            if unique {
                format!("{r} ≔ &!{x};")
            } else {
                format!("{r} ≔ &{x};")
            },
            Role::MakeRef {
                var: r.clone(),
                base: x.clone(),
                unique,
            },
        ));
        let mut acts = Vec::new();
        for _ in 0..1 + self.rng.below(4) {
            acts.push(self.int_action(&x, &r, unique));
        }
        if self.rng.chance(25) {
            self.tags.insert("borrow_in_loop");
            out.push(St::Loop(2 + self.rng.below(2) as u8, acts));
        } else {
            out.extend(acts);
        }
        out
    }

    fn int_action(&mut self, x: &str, r: &str, unique: bool) -> St {
        let k = self.k();
        match self.rng.below(10) {
            0 | 1 => {
                if unique {
                    raw_r(format!("*{r} ← {k};"), Role::UseRef { var: r.into() })
                } else {
                    raw_r(format!("use_i(*{r});"), Role::UseRef { var: r.into() })
                }
            }
            2 => raw(format!("{x} ← {k};")),
            3 => raw(format!("use_i({x});")),
            4 => {
                let s = self.fresh("s");
                raw(format!("{s} ≔ &{x}; use_i(*{s});"))
            }
            5 => {
                let t = self.fresh("t");
                raw(format!("{t} ≔ &!{x}; *{t} ← {k};"))
            }
            // truyền tham chiếu cho hàm: `&!` được reborrow ngầm (ADR-023 §0.1), `&` được sao chép
            8 => raw_r(
                if unique {
                    format!("takes({r});")
                } else {
                    format!("look({r});")
                },
                Role::UseRef { var: r.into() },
            ),
            // sao chép tham chiếu bằng gán: `&` là Copy, `&!` bị MOVE (dùng lại `r` sau đó là dùng-sau-move)
            6 => {
                let v = self.fresh("v");
                raw(format!("{v} ≔ {r}; use_i(*{v});"))
            }
            _ => {
                let v = self.fresh("v");
                if unique {
                    raw(format!("{v} ≔ {r}; *{v} ← {k};"))
                } else {
                    raw(format!("{v} ≔ {r}; use_i(*{v});"))
                }
            }
        }
    }

    /// `u ≔ &!x; v ≔ &!*u / &*u;` rồi hành động ngẫu nhiên.
    fn reborrow(&mut self) -> Vec<St> {
        self.tags.insert("reborrow");
        let x = self.fresh("x");
        let u = self.fresh("u");
        let v = self.fresh("v");
        let vu = self.rng.chance(60);
        let k0 = self.k();
        let mut out = vec![
            raw(format!("μ{x} ≔ {k0};")),
            raw_r(
                format!("{u} ≔ &!{x};"),
                Role::MakeRef {
                    var: u.clone(),
                    base: x.clone(),
                    unique: true,
                },
            ),
            raw(if vu {
                format!("{v} ≔ &!*{u};")
            } else {
                format!("{v} ≔ &*{u};")
            }),
        ];
        for _ in 0..1 + self.rng.below(4) {
            let k = self.k();
            out.push(match self.rng.below(5) {
                0 | 1 => {
                    if vu {
                        raw(format!("*{v} ← {k};"))
                    } else {
                        raw(format!("use_i(*{v});"))
                    }
                }
                2 => raw_r(format!("*{u} ← {k};"), Role::UseRef { var: u.clone() }),
                3 => raw(format!("{x} ← {k};")),
                _ => raw(format!("use_i({x});")),
            });
        }
        out
    }

    /// Biến `B` bị move rồi dùng/gán lại theo thứ tự ngẫu nhiên.
    fn moves(&mut self) -> Vec<St> {
        self.tags.insert("move");
        let b = self.fresh("b");
        let mut out = vec![raw(format!("μ{b} ≔ mk();"))];
        for _ in 0..1 + self.rng.below(4) {
            let mv = || Role::Move { var: b.clone() };
            let st = match self.rng.below(7) {
                0 => raw_r(format!("eat({b});"), mv()),
                1 => {
                    let y = self.fresh("y");
                    raw_r(format!("{y} ≔ {b};"), mv())
                }
                2 => raw(format!("use_i({b}.v);")),
                3 => raw(format!("{b} ← mk();")),
                4 => raw(format!("use_i(peek(&{b}));")),
                5 => {
                    let inner = vec![raw_r(format!("eat({b});"), mv())];
                    St::Branch(inner, vec![])
                }
                _ => raw(format!("bump(&!{b});")),
            };
            out.push(st);
        }
        out
    }

    /// `W { b: B, n: i32 }`: move/mượn từng trường, gán lại, dùng cả struct.
    fn partial_move_nested(&mut self) -> Vec<St> {
        self.tags.insert("nested_struct");
        let w = self.fresh("w");
        let mut out = vec![raw(format!("μ{w} ≔ mkw();"))];
        let mut live_refs: Vec<(String, bool)> = Vec::new();
        for _ in 0..2 + self.rng.below(4) {
            let k = self.k();
            let st = match self.rng.below(10) {
                0 => {
                    self.tags.insert("partial_move");
                    let y = self.fresh("y");
                    raw(format!("{y} ≔ {w}.b;"))
                }
                1 => {
                    self.tags.insert("partial_move");
                    raw(format!("eat({w}.b);"))
                }
                2 => raw(format!("use_i({w}.n);")),
                3 => raw(format!("{w}.b ← mk();")),
                4 => raw(format!("eatw({w});")),
                5 => {
                    let r = self.fresh("r");
                    live_refs.push((r.clone(), false));
                    raw_r(
                        format!("{r} ≔ &{w}.n;"),
                        Role::MakeRef {
                            var: r,
                            base: format!("{w}.n"),
                            unique: false,
                        },
                    )
                }
                6 => {
                    let u = self.fresh("u");
                    live_refs.push((u.clone(), true));
                    raw_r(
                        format!("{u} ≔ &!{w}.n;"),
                        Role::MakeRef {
                            var: u,
                            base: format!("{w}.n"),
                            unique: true,
                        },
                    )
                }
                7 => raw(format!("{w}.n ← {k};")),
                8 if !live_refs.is_empty() => {
                    let (v, u) = self.rng.pick(&live_refs).clone();
                    if u {
                        raw_r(format!("*{v} ← {k};"), Role::UseRef { var: v })
                    } else {
                        raw_r(format!("use_i(*{v});"), Role::UseRef { var: v })
                    }
                }
                _ => raw(format!("bump(&!{w}.b);")),
            };
            out.push(st);
        }
        out
    }

    /// `Option<B>` / `Option<i32>`, `⌥` với mượn trong nhánh; move payload; dùng lại `o` sau khi move.
    fn match_scenario(&mut self) -> Vec<St> {
        self.tags.insert("borrow_via_match");
        let o = self.fresh("o");
        let pv = self.fresh("p");
        if self.rng.chance(50) {
            // Option<B>
            let mut some = Vec::new();
            let mut moved = false;
            for _ in 0..1 + self.rng.below(3) {
                let st = match self.rng.below(5) {
                    0 if !moved => {
                        moved = true;
                        self.tags.insert("move");
                        raw_r(format!("eat({pv});"), Role::Move { var: pv.clone() })
                    }
                    1 => raw(format!("use_i(peek(&{pv}));")),
                    2 => {
                        let r = self.fresh("r");
                        raw(format!("{r} ≔ &{pv}; use_i(peek({r}));"))
                    }
                    3 => raw(format!("use_i({pv}.v);")),
                    _ => raw(format!("use_i({pv}.v);")),
                };
                some.push(st);
            }
            let mut out = vec![raw(format!("μ{o} ≔ ⎇ e {{ Some(mk()) }} ⇝ {{ None }};"))];
            out.push(St::Match {
                scrut: o.clone(),
                pv: pv.clone(),
                some,
                none: vec![],
            });
            // dùng lại `o` sau (hợp lệ chỉ khi payload không bị move)
            match self.rng.below(4) {
                0 => out.push(St::Match {
                    scrut: o.clone(),
                    pv: format!("{pv}b"),
                    some: vec![raw(format!("use_i(peek(&{pv}b));"))],
                    none: vec![],
                }),
                1 => out.push(raw(format!("{o} ← None;"))),
                _ => {}
            }
            out
        } else {
            // Option<i32>: mượn payload Copy trong nhánh
            let k = self.k();
            let r = self.fresh("r");
            let mut some = vec![raw(format!("{r} ≔ &{pv}; use_i(*{r});"))];
            if self.rng.chance(40) {
                some.push(raw(format!("use_i({pv});")));
            }
            vec![
                raw(format!("μ{o} ≔ ⎇ e {{ Some({k}) }} ⇝ {{ None }};")),
                St::Match {
                    scrut: o,
                    pv,
                    some,
                    none: vec![],
                },
            ]
        }
    }

    /// Arena: cấp phát, dùng, reset/move/gán đè theo thứ tự ngẫu nhiên.
    fn arena(&mut self) -> Vec<St> {
        let a = self.fresh("a");
        let p = self.fresh("p");
        let k0 = self.k();
        let mut out = vec![
            raw(format!("μ{a} ≔ Arena::new();")),
            raw_r(
                format!("{p} ≔ {a}.alloc({k0});"),
                Role::MakeArenaRef {
                    var: p.clone(),
                    arena: a.clone(),
                },
            ),
        ];
        let mut qs: Vec<String> = Vec::new();
        let mut arena_alive = true;
        for _ in 0..1 + self.rng.below(4) {
            let k = self.k();
            let st = match self.rng.below(7) {
                0 | 1 => raw_r(format!("use_i(*{p});"), Role::UseRef { var: p.clone() }),
                2 if arena_alive => {
                    self.tags.insert("arena_reset");
                    raw(format!("{a}.reset();"))
                }
                3 if arena_alive => {
                    let q = self.fresh("q");
                    qs.push(q.clone());
                    raw_r(
                        format!("{q} ≔ {a}.alloc({k});"),
                        Role::MakeArenaRef {
                            var: q,
                            arena: a.clone(),
                        },
                    )
                }
                4 if !qs.is_empty() => {
                    let q = self.rng.pick(&qs).clone();
                    raw_r(format!("use_i(*{q});"), Role::UseRef { var: q })
                }
                5 if arena_alive && self.rng.chance(40) => {
                    arena_alive = false;
                    raw(format!("eata({a});"))
                }
                _ => raw(format!("use_i(*{p});")),
            };
            out.push(st);
        }
        out
    }

    /// Lời gọi theo `RegionSig`: `idr`, `pick`, `pick2`, `first`, `two`, `mix`.
    fn calls(&mut self) -> Vec<St> {
        self.tags.insert("call_regionsig");
        let x = self.fresh("x");
        let y = self.fresh("y");
        let r = self.fresh("r");
        let (kx, ky) = (self.k(), self.k());
        let mut out = vec![raw(format!("μ{x} ≔ {kx};")), raw(format!("μ{y} ≔ {ky};"))];
        // đôi khi hai đối số cùng một biến (vi phạm với `two`)
        let yy = if self.rng.chance(15) {
            x.clone()
        } else {
            y.clone()
        };
        match self.rng.below(6) {
            0 => out.push(raw_r(
                format!("{r} ≔ idr(&{x});"),
                Role::MakeRef {
                    var: r.clone(),
                    base: x.clone(),
                    unique: false,
                },
            )),
            1 => out.push(raw(format!("{r} ≔ pick(&{x}, &{yy});"))),
            2 => out.push(raw(format!("{r} ≔ pick2(&{x}, &{yy});"))),
            3 => {
                let arr = self.fresh("arr");
                out.push(raw(format!("μ{arr} ≔ [{kx}, {ky}];")));
                out.push(raw(format!("{r} ≔ first(&{arr}[..]);")));
                // ghi vào phần tử khác (oracle chính xác theo chỉ số; borrowck coi mọi chỉ số chồng lấn)
                if self.rng.chance(60) {
                    let i = self.rng.below(2);
                    out.push(raw(format!("{arr}[{i}] ← 7;")));
                }
            }
            4 => {
                out.push(raw(format!("two(&!{x}, &!{yy});")));
                out.push(raw(format!("{r} ≔ &{x};")));
            }
            _ => {
                out.push(raw(format!("mix(&{x}, &!{yy});")));
                out.push(raw(format!("{r} ≔ &{y};")));
            }
        }
        for _ in 0..self.rng.below(3) {
            let k = self.k();
            out.push(match self.rng.below(4) {
                0 => raw_r(format!("use_i(*{r});"), Role::UseRef { var: r.clone() }),
                1 => raw(format!("{x} ← {k};")),
                2 => raw(format!("{y} ← {k};")),
                _ => raw_r(format!("use_i(*{r});"), Role::UseRef { var: r.clone() }),
            });
        }
        out
    }

    /// `Hs { r: &x }`: tham chiếu giữ trong struct (bó region của ADT).
    fn holder(&mut self) -> Vec<St> {
        self.tags.insert("nested_struct");
        let x = self.fresh("x");
        let h = self.fresh("h");
        let k0 = self.k();
        let mut out = vec![
            raw(format!("μ{x} ≔ {k0};")),
            raw_r(
                format!("{h} ≔ Hs {{ r: &{x} }};"),
                Role::MakeRef {
                    var: h.clone(),
                    base: x.clone(),
                    unique: false,
                },
            ),
        ];
        for _ in 0..1 + self.rng.below(3) {
            let k = self.k();
            out.push(match self.rng.below(4) {
                0 | 1 => raw_r(format!("use_i(*{h}.r);"), Role::UseRef { var: h.clone() }),
                2 => raw(format!("{x} ← {k};")),
                _ => raw(format!("use_i({x});")),
            });
        }
        out
    }
}

fn has_ref_role(s: &St) -> bool {
    match s {
        St::Raw(r) => matches!(
            r.role,
            Role::MakeRef { .. } | Role::UseRef { .. } | Role::MakeArenaRef { .. }
        ),
        St::Branch(a, b) => a.iter().chain(b.iter()).any(has_ref_role),
        St::Loop(_, b) => b.iter().any(has_ref_role),
        St::Match { some, none, .. } => some.iter().chain(none.iter()).any(has_ref_role),
    }
}

// ======================================================================= đột biến ngữ nghĩa ("tấn công")

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Attack {
    UseAfterMove,
    SecondBorrow,
    ArenaReset,
}

#[derive(Clone, Debug)]
pub struct Site {
    pub attack: Attack,
    /// đường tới `Vec<St>` chứa vị trí (chỉ số trong cây, theo thứ tự duyệt)
    pub block_id: usize,
    pub at: usize,
    pub insert: Vec<(String, &'static [&'static str])>,
}

/// Tất cả vị trí chèn áp dụng được (mỗi vị trí kèm các văn bản chèn khả dĩ và mã borrowck mong đợi).
pub fn sites(p: &Prog) -> Vec<Site> {
    let mut out = Vec::new();
    let mut id = 0usize;
    collect(&p.body, &mut id, &mut out);
    out
}

fn collect(b: &[St], id: &mut usize, out: &mut Vec<Site>) {
    let my = *id;
    *id += 1;
    for (i, s) in b.iter().enumerate() {
        if let St::Raw(r) = s {
            match &r.role {
                Role::Move { var } => out.push(Site {
                    attack: Attack::UseAfterMove,
                    block_id: my,
                    at: i + 1,
                    insert: vec![(format!("use_i({var}.v);"), &["E0401"])],
                }),
                Role::MakeRef { var, base, unique } => {
                    // cần một lần dùng SAU trong cùng khối
                    let used_later = b[i + 1..].iter().position(|t| {
                        matches!(t, St::Raw(Raw { role: Role::UseRef { var: v }, .. }) if v == var)
                    });
                    if let Some(off) = used_later {
                        let j = i + 1 + off;
                        let t = format!("zz{my}_{i}");
                        let ins: Vec<(String, &'static [&'static str])> = if *unique {
                            vec![
                                (format!("use_i({base});"), &["E0403"]),
                                (format!("{t} ≔ &!{base}; *{t} ← 0;"), &["E0402"]),
                                (format!("{t} ≔ &{base}; use_i(*{t});"), &["E0403"]),
                            ]
                        } else {
                            vec![
                                (format!("{base} ← 0;"), &["E0406"]),
                                (format!("{t} ≔ &!{base}; *{t} ← 0;"), &["E0403"]),
                            ]
                        };
                        for k in i + 1..=j {
                            out.push(Site {
                                attack: Attack::SecondBorrow,
                                block_id: my,
                                at: k,
                                insert: ins.clone(),
                            });
                        }
                    }
                }
                Role::MakeArenaRef { var, arena } => {
                    let used_later = b[i + 1..].iter().position(|t| {
                        matches!(t, St::Raw(Raw { role: Role::UseRef { var: v }, .. }) if v == var)
                    });
                    if let Some(off) = used_later {
                        let j = i + 1 + off;
                        for k in i + 1..=j {
                            out.push(Site {
                                attack: Attack::ArenaReset,
                                block_id: my,
                                at: k,
                                insert: vec![
                                    (format!("{arena}.reset();"), &["E0405"]),
                                    (format!("eata({arena});"), &["E0405"]),
                                    (format!("{arena} ← Arena::new();"), &["E0405"]),
                                ],
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
    for s in b {
        match s {
            St::Branch(a, e) => {
                collect(a, id, out);
                collect(e, id, out);
            }
            St::Loop(_, x) => collect(x, id, out),
            St::Match { some, none, .. } => {
                collect(some, id, out);
                collect(none, id, out);
            }
            St::Raw(_) => {}
        }
    }
}

/// Áp một phép chèn: trả chương trình mới (cây được sao chép).
#[must_use]
pub fn apply(p: &Prog, site: &Site, which: usize) -> Prog {
    let mut q = p.clone();
    let mut id = 0usize;
    let text = site.insert[which].0.clone();
    insert_at(&mut q.body, &mut id, site.block_id, site.at, &text);
    q
}

fn insert_at(b: &mut Vec<St>, id: &mut usize, target: usize, at: usize, text: &str) -> bool {
    let my = *id;
    *id += 1;
    if my == target {
        b.insert(at, raw(text));
        return true;
    }
    for s in b.iter_mut() {
        let done = match s {
            St::Branch(a, e) => {
                insert_at(a, id, target, at, text) || insert_at(e, id, target, at, text)
            }
            St::Loop(_, x) => insert_at(x, id, target, at, text),
            St::Match { some, none, .. } => {
                insert_at(some, id, target, at, text) || insert_at(none, id, target, at, text)
            }
            St::Raw(_) => false,
        };
        if done {
            return true;
        }
    }
    false
}
