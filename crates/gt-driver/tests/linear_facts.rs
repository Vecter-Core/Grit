//! A2 bước 2 Giai đoạn 2 (ADR-P2-05): M7 giữ SỰ KIỆN TUYẾN TÍNH (điều kiện nhánh/assert/hợp đồng như `x + y ≤ 100`, `x ≤ y + 10`) mà miền
//! khoảng/zone không biểu diễn được, và đưa chúng làm giả thiết cho bộ chứng minh có chứng chỉ (kernel Farkas, ADR-P2-04).
//! (1) ca viết tay: tiền điều kiện chỉ chứng minh được nhờ sự kiện tuyến tính ⇒ KHÔNG còn N0901 (không hạ thành runtime);
//!     ca đối chứng: biến bị gán lại sau điều kiện ⇒ sự kiện phải MẤT (tiền điều kiện sai thật ⇒ vẫn N0901).
//! (2) oracle soundness: chương trình sinh ngẫu nhiên (nhánh lồng với điều kiện tuyến tính, gán lại biến giữa chừng, một lời gọi có `⊳`);
//!     mỗi khi M7 KHẲNG ĐỊNH đã chứng minh `⊳` (không có N0901), MÔ HÌNH độc lập duyệt VÉT CẠN mọi đầu vào `u8 × u8` và phải xác nhận `⊳`
//!     đúng ở mọi lần tới lời gọi. Đồng thời đòi bộ sinh có đủ ca mà sự kiện thật sự cần (không rỗng).
use gt_driver::{Options, check};
use gt_resolve::SourceInput;

fn notes(src: &str) -> (Vec<String>, Vec<String>) {
    let res = check(
        &[SourceInput {
            name: "main.gt".to_string(),
            text: src.to_string(),
        }],
        &Options::default(),
    )
    .expect("đầu vào hợp lệ");
    let all: Vec<String> = res
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    let errs = all.iter().filter(|c| c.starts_with('E')).cloned().collect();
    (errs, all)
}

#[test]
fn preconditions_that_need_a_linear_branch_fact_are_proved() {
    let cases = [
        // tổng: x ≤ 100 − y  ⇒  x + y ≤ 100
        (
            "ƒ need(a: u32, b: u32) → u32 ! ∅ ⊳ a + b ≤ 100 { a + b }
ƒ f(x: u32, y: u32) → u32 ! ∅ {
  ⎇ y ≤ 100 { ⎇ x ≤ 100 - y { need(x, y) } ⇝ { 0 } } ⇝ { 0 }
}",
            "tổng",
        ),
        // bắc cầu qua biến thứ ba, hệ số: x + y ≤ 20 ∧ z ≤ 10 ⇒ x + y + z ≤ 30
        (
            "ƒ need(a: u32, b: u32, c: u32) → u32 ! ∅ ⊳ a + b + c ≤ 30 { a }
ƒ f(p: u8, q: u8, r: u8) → u32 ! ∅ {
  x ≔ p as u32; y ≔ q as u32; z ≔ r as u32;
  ⎇ x + y ≤ 20 { ⎇ z ≤ 10 { need(x, y, z) } ⇝ { 0 } } ⇝ { 0 }
}",
            "ba biến",
        ),
        // hệ số: 2·x + y ≤ 50 ⇒ x ≤ 25
        (
            "ƒ need(a: u32) → u32 ! ∅ ⊳ a ≤ 25 { a }
ƒ f(p: u8, q: u8) → u32 ! ∅ {
  x ≔ p as u32; y ≔ q as u32;
  ⎇ 2 * x + y ≤ 50 { need(x) } ⇝ { 0 }
}",
            "hệ số",
        ),
        // nhánh ⇝ (phủ định): ¬(x + y > 40) ⇒ x + y ≤ 40
        (
            "ƒ need(a: u32, b: u32) → u32 ! ∅ ⊳ a + b ≤ 40 { a }
ƒ f(p: u8, q: u8) → u32 ! ∅ {
  x ≔ p as u32; y ≔ q as u32;
  ⎇ x + y > 40 { 0 } ⇝ { need(x, y) }
}",
            "phủ định",
        ),
        // sự kiện sống qua điểm hợp khi ĐÚNG ở cả hai nhánh
        (
            "ƒ need(a: u32, b: u32) → u32 ! ∅ ⊳ a + b ≤ 60 { a }
ƒ f(p: u8, q: u8, c: bool) → u32 ! ∅ {
  x ≔ p as u32; y ≔ q as u32;
  ⎇ x + y ≤ 60 {
    μt ≔ 0u32;
    ⎇ c { t ← 1; } ⇝ { t ← 2; };
    need(x, y) + t
  } ⇝ { 0 }
}",
            "qua điểm hợp",
        ),
    ];
    for (src, name) in cases {
        let (errs, all) = notes(src);
        assert!(errs.is_empty(), "{name}: gritc từ chối {errs:?}\n{src}");
        assert!(
            !all.iter().any(|c| c == "N0901"),
            "{name}: tiền điều kiện lẽ ra chứng minh được nhờ sự kiện tuyến tính, nhưng bị hạ thành runtime: {all:?}\n{src}"
        );
    }
}

#[test]
fn a_linear_fact_dies_when_a_variable_it_mentions_is_reassigned() {
    let cases = [
        (
            "ƒ need(a: u32, b: u32) → u32 ! ∅ ⊳ a + b ≤ 60 { a }
ƒ f(p: u8, q: u8) → u32 ! ∅ {
  x ≔ p as u32; μy ≔ q as u32;
  ⎇ x + y ≤ 60 { y ← y + 30; need(x, y) } ⇝ { 0 }
}",
            "gán lại",
        ),
        (
            "ƒ need(a: u32, b: u32) → u32 ! ∅ ⊳ a + b ≤ 60 { a }
ƒ f(p: u8, q: u8, c: bool) → u32 ! ∅ {
  x ≔ p as u32; μy ≔ q as u32;
  ⎇ x + y ≤ 60 { ⎇ c { y ← y + 30; } ⇝ { }; need(x, y) } ⇝ { 0 }
}",
            "gán lại ở một nhánh",
        ),
        (
            "ƒ need(a: u32, b: u32) → u32 ! ∅ ⊳ a + b ≤ 60 { a }
ƒ f(p: u8, q: u8, c: bool) → u32 ! ∅ {
  x ≔ p as u32; y ≔ q as u32;
  ⎇ c { ⎇ x + y ≤ 60 { } ⇝ { }; } ⇝ { };
  need(x, y)
}",
            "chỉ đúng trong nhánh",
        ),
        (
            "ƒ bump(r: &!u32) ! ∅ { *r ← *r +% 30; }
ƒ need(a: u32, b: u32) → u32 ! ∅ ⊳ a + b ≤ 60 { a }
ƒ f(p: u8, q: u8) → u32 ! ∅ {
  x ≔ p as u32; μy ≔ q as u32;
  ⎇ x + y ≤ 60 { bump(&!y); need(x, y) } ⇝ { 0 }
}",
            "ghi qua &!",
        ),
    ];
    for (src, name) in cases {
        let (errs, all) = notes(src);
        assert!(errs.is_empty(), "{name}: gritc từ chối {errs:?}\n{src}");
        assert!(
            all.iter().any(|c| c == "N0901"),
            "{name}: tiền điều kiện SAI (có phản ví dụ) mà được coi là đã chứng minh: {all:?}\n{src}"
        );
    }
}

// ------------------------------------------------------------------ oracle

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

/// `Σ cᵢ·vᵢ ≤ k` trên các biến x(0) y(1) z(2)
#[derive(Clone)]
struct Lin {
    c: [i64; 3],
    k: i64,
}

impl Lin {
    fn holds(&self, v: [i64; 3]) -> bool {
        self.c[0] * v[0] + self.c[1] * v[1] + self.c[2] * v[2] <= self.k
    }
    /// `lhs ≤ rhs` với hệ số dương ở hai phía (không có phép trừ ⇒ không tràn/thiếu u32), tên biến theo `names`
    fn src(&self, names: [&str; 3]) -> String {
        let side = |pos: bool| -> String {
            let mut t: Vec<String> = Vec::new();
            for i in 0..3 {
                let c = if pos { self.c[i] } else { -self.c[i] };
                if c > 0 {
                    t.push(if c == 1 {
                        names[i].to_string()
                    } else {
                        format!("{c} * {}", names[i])
                    });
                }
            }
            t.join(" + ")
        };
        let mut l = side(true);
        let mut r = side(false);
        // Σ_pos ≤ k + Σ_neg
        if self.k >= 0 {
            r = if r.is_empty() {
                self.k.to_string()
            } else {
                format!("{r} + {}", self.k)
            };
        } else {
            l = if l.is_empty() {
                (-self.k).to_string()
            } else {
                format!("{l} + {}", -self.k)
            };
        }
        if l.is_empty() {
            l = "0".into();
        }
        format!("{l} ≤ {r}")
    }
}

enum St {
    If(Lin, Vec<St>, Vec<St>),
    /// z ← (x hoặc y) / 2
    SetZ(usize),
    Call,
}

struct Gen {
    r: Rng,
    /// điều kiện trên đường hiện tại (theo phía nhánh), và z đã bị gán lại SAU nó chưa
    path: Vec<(Lin, bool)>,
    placed: bool,
    /// điều kiện của các nhánh ĐÃ ĐÓNG (chỉ đúng bên trong nhánh đó) — `⊳` lấy từ đây thử phép GIAO ở điểm hợp
    stale: Vec<Lin>,
    from_stale: u32,
    pre: Option<Lin>,
    from_path: bool,
}

impl Gen {
    fn lin(&mut self) -> Lin {
        let mut c = [0i64; 3];
        let a = self.r.below(3) as usize;
        let mut b = self.r.below(3) as usize;
        if b == a {
            b = (a + 1) % 3;
        }
        c[a] = 1 + self.r.below(3) as i64;
        c[b] = if self.r.below(3) == 0 {
            -(1 + self.r.below(2) as i64)
        } else {
            1 + self.r.below(3) as i64
        };
        let k = if c[b] < 0 {
            self.r.below(60) as i64
        } else {
            self.r.below(500) as i64
        };
        Lin { c, k }
    }
    fn neg(l: &Lin) -> Lin {
        // ¬(Σ ≤ k) ⇔ −Σ ≤ −k − 1
        Lin {
            c: [-l.c[0], -l.c[1], -l.c[2]],
            k: -l.k - 1,
        }
    }
    fn block(&mut self, depth: u32) -> Vec<St> {
        let mut v = Vec::new();
        for _ in 0..1 + self.r.below(3) {
            match self.r.below(6) {
                0 | 1 | 2 if depth < 3 => {
                    let l = self.lin();
                    self.path.push((l.clone(), false));
                    let t = self.block(depth + 1);
                    self.path.pop();
                    self.path.push((Self::neg(&l), false));
                    let e = self.block(depth + 1);
                    self.path.pop();
                    self.stale.push(Self::neg(&l));
                    self.stale.push(l.clone());
                    v.push(St::If(l, t, e));
                }
                3 => {
                    for p in self.path.iter_mut() {
                        if p.0.c[2] != 0 {
                            p.1 = true;
                        }
                    }
                    v.push(St::SetZ(self.r.below(2) as usize));
                }
                _ if !self.placed && depth >= 1 && self.r.below(2) == 0 => {
                    self.placed = true;
                    // `⊳` = một điều kiện trên đường (có thể đã bị gán lại z — khi đó KHÔNG được chứng minh nhờ nó) + độ chùng; hoặc ngẫu nhiên
                    let pre = if !self.stale.is_empty() && self.r.below(2) == 0 {
                        self.from_stale += 1;
                        let i = self.r.below(self.stale.len() as u64) as usize;
                        let mut l = self.stale[i].clone();
                        l.k += self.r.below(4) as i64;
                        l
                    } else if !self.path.is_empty() && self.r.below(4) != 0 {
                        let i = self.r.below(self.path.len() as u64) as usize;
                        self.from_path = !self.path[i].1;
                        let mut l = self.path[i].0.clone();
                        l.k += self.r.below(4) as i64;
                        l
                    } else {
                        self.lin()
                    };
                    self.pre = Some(pre);
                    v.push(St::Call);
                }
                _ => {}
            }
        }
        v
    }
}

fn emit(sts: &[St], ind: usize, out: &mut String) {
    let pad = "  ".repeat(ind);
    for s in sts {
        match s {
            St::If(l, t, e) => {
                out.push_str(&format!("{pad}⎇ {} {{\n", l.src(["x", "y", "z"])));
                emit(t, ind + 1, out);
                out.push_str(&format!("{pad}}} ⇝ {{\n"));
                emit(e, ind + 1, out);
                out.push_str(&format!("{pad}}};\n"));
            }
            St::SetZ(w) => out.push_str(&format!("{pad}z ← {} / 2;\n", ["x", "y"][*w])),
            St::Call => out.push_str(&format!("{pad}acc ← need(x, y, z);\n")),
        }
    }
}

/// mô hình: chạy chương trình trên (x, y, z); trả về `Some(⊳ đúng?)` nếu tới lời gọi
fn sim(sts: &[St], v: &mut [i64; 3], pre: &Lin) -> Option<bool> {
    for s in sts {
        match s {
            St::If(l, t, e) => {
                if let Some(r) = sim(if l.holds(*v) { t } else { e }, v, pre) {
                    return Some(r);
                }
            }
            St::SetZ(w) => v[2] = v[*w] / 2,
            St::Call => return Some(pre.holds(*v)),
        }
    }
    None
}

#[test]
fn every_precondition_m7_claims_proved_holds_on_every_input() {
    let (mut proved, mut needs_fact_proved, mut from_path, mut total) = (0, 0, 0, 0);
    let mut stale_cases = 0;
    for seed in 1..=240u64 {
        let mut g = Gen {
            r: Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1),
            path: Vec::new(),
            placed: false,
            stale: Vec::new(),
            from_stale: 0,
            pre: None,
            from_path: false,
        };
        let mut body = g.block(0);
        if !g.placed {
            // lời gọi ở cuối hàm: mọi nhánh đã đóng ⇒ thử phép giao ở điểm hợp
            g.pre = Some(if !g.stale.is_empty() {
                g.from_stale += 1;
                let i = g.r.below(g.stale.len() as u64) as usize;
                let mut l = g.stale[i].clone();
                l.k += g.r.below(4) as i64;
                l
            } else {
                g.lin()
            });
            body.push(St::Call);
        }
        let pre = g.pre.clone().expect("có ⊳");
        stale_cases += g.from_stale;
        let mut src = format!(
            "ƒ need(p: u32, q: u32, r: u32) → u32 ! ∅ ⊳ {} {{ 0 }}\nƒ f(a: u8, b: u8) → u32 ! ∅ {{\n  x ≔ a as u32; y ≔ b as u32; μz ≔ x / 2; μacc ≔ 0u32;\n",
            pre.src(["p", "q", "r"])
        );
        emit(&body, 1, &mut src);
        src.push_str("  acc\n}\n");
        let (errs, all) = notes(&src);
        let refuted = errs == ["E0901"];
        assert!(
            errs.is_empty() || refuted,
            "#{seed}: gritc từ chối {errs:?}\n{src}"
        );
        total += 1;
        let claimed = !all.iter().any(|c| c == "N0901");
        // mô hình vét cạn
        let mut always = true;
        let mut reached = false;
        for a in 0..=255i64 {
            for b in 0..=255i64 {
                let mut v = [a, b, a / 2];
                if let Some(ok) = sim(&body, &mut v, &pre) {
                    reached = true;
                    if !ok {
                        always = false;
                    }
                }
            }
        }
        if refuted {
            // E0901: M7 khẳng định `⊳` SAI ⇒ mô hình phải tìm được đầu vào tới lời gọi làm nó sai
            assert!(
                reached && !always,
                "#{seed}: E0901 nhưng `⊳` không sai ở đầu vào nào\n{src}"
            );
        }
        if claimed && !refuted {
            assert!(
                always,
                "#{seed}: KHÔNG SOUND — M7 coi `⊳` đã chứng minh nhưng mô hình tìm được đầu vào làm nó sai\n{src}"
            );
            proved += 1;
        }
        if g.from_path && reached {
            from_path += 1;
            if claimed {
                needs_fact_proved += 1;
            }
        }
    }
    eprintln!(
        "linear_facts oracle: {total} chương trình, {proved} `⊳` chứng minh tĩnh, {needs_fact_proved}/{from_path} ca `⊳` lấy từ điều kiện đường đi (còn hiệu lực)"
    );
    assert!(
        stale_cases >= 10,
        "bộ sinh quá ít ca `⊳` từ nhánh đã đóng: {stale_cases}"
    );
    assert!(
        from_path >= 30,
        "bộ sinh quá ít ca `⊳` từ đường đi: {from_path}"
    );
    // độ đầy đủ: phần lớn ca `⊳` = điều kiện đường đi còn hiệu lực + độ chùng phải chứng minh được
    assert!(
        needs_fact_proved * 4 >= from_path * 3,
        "sự kiện tuyến tính chứng minh quá ít: {needs_fact_proved}/{from_path}"
    );
}
