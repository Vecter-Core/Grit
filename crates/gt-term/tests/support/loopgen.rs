//! Bộ sinh chương trình có VÒNG LẶP và ĐỆ QUY cho oracle điều kiện dừng của M8 (ADR-026 §3). Công cụ test, không thuộc TCB.
//! Sự thật nền ("chương trình có dừng không") đến từ việc CHẠY trong thông dịch MIR với nhiên liệu: giới hạn nhỏ (≤ 20), lồng ≤ 2 nên
//! mọi chương trình dừng thật đều xong trong vài nghìn bước; cạn nhiên liệu hoặc đệ quy quá sâu ⇒ coi là không dừng.
#![allow(dead_code)]
use crate::progen::Rng;

pub struct Prog {
    pub src: String,
    pub ty: &'static str,
    pub entry_params: usize,
    pub has_loop: bool,
    pub has_rec: bool,
}

pub struct LGen {
    rng: Rng,
    n: usize,
    ty: &'static str,
    signed: bool,
}

impl LGen {
    pub fn new(seed: u64) -> LGen {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let (ty, signed) = [
            ("i32", true),
            ("u32", false),
            ("i64", true),
            ("u8", false),
            ("i16", true),
        ][rng.below(5) as usize];
        LGen {
            rng,
            n: 0,
            ty,
            signed,
        }
    }

    fn fresh(&mut self, p: &str) -> String {
        self.n += 1;
        format!("{p}{}", self.n)
    }

    fn lit(&self, v: i128) -> String {
        format!("{v}{}", self.ty)
    }

    fn small(&mut self) -> i128 {
        self.rng.below(20) as i128
    }

    /// cận của vòng: literal nhỏ hoặc biến/tham số
    fn bound(&mut self, vars: &[String]) -> String {
        if self.rng.chance(55) || vars.is_empty() {
            let v = 1 + self.small();
            self.lit(v)
        } else {
            vars[self.rng.below(vars.len() as u64) as usize].clone()
        }
    }

    fn cond_atom(&mut self, i: &str, vars: &[String]) -> (String, Option<String>) {
        let b = self.bound(vars);
        let (op, up) = match self.rng.below(5) {
            0 => ("<", Some(format!("{b} - {i}"))),
            1 => ("<=", Some(format!("{b} - {i} + {}", self.lit(1)))),
            2 => (">", Some(format!("{i} - {b}"))),
            3 => (">=", Some(format!("{i} - {b} + {}", self.lit(1)))),
            _ => ("!=", None),
        };
        (format!("{i} {op} {b}"), up)
    }

    /// một vòng lặp hoàn chỉnh (chuỗi câu lệnh), `depth` = số vòng lồng còn lại
    fn lp(&mut self, vars: &mut Vec<String>, depth: u32) -> String {
        let i = self.fresh("i");
        let step_kind = self.rng.below(100);
        let (cond, good_var) = self.cond_atom(&i, vars);
        let init = {
            let v = self.small();
            self.lit(v)
        };
        // điều kiện ghép
        let cond = match self.rng.below(10) {
            0 => {
                let (c2, _) = self.cond_atom(&i, vars);
                format!("{cond} ∧ {c2}")
            }
            1 => {
                let (c2, _) = self.cond_atom(&i, vars);
                format!("{cond} ∨ {c2}")
            }
            _ => cond,
        };
        // thân: bước trên i
        let c = 1 + self.rng.below(3) as i128;
        // 65% bước ĐÚNG hướng với điều kiện (lên với `<`/`<=`, xuống với `>`/`>=`): để đủ chương trình được chấp nhận
        let step_kind = if self.rng.chance(65) {
            if cond.contains('<') {
                0
            } else if cond.contains('>') {
                // GÓI 12: đi xuống bằng TRỪ hoặc bằng CHIA (`/`, `%` hằng) — phủ luật quan hệ chia/dư của bộ chứng minh (ADR-C16)
                if self.rng.chance(75) { 40 } else { 93 }
            } else {
                step_kind
            }
        } else {
            step_kind
        };
        let step = match step_kind {
            0..=34 => format!("{i} ← {i} + {};", self.lit(c)),
            35..=54 => format!("{i} ← {i} - {};", self.lit(c)),
            55..=62 => format!("{i} ← {i} +% {};", self.lit(c)),
            63..=70 => format!(
                "⎇ {} < {} {{ {i} ← {i} + {}; }}",
                vars.first().cloned().unwrap_or_else(|| self.lit(0)),
                self.lit(3),
                self.lit(1)
            ),
            71..=78 => format!("{i} ← {i} * {};", self.lit(2)),
            79..=84 => String::new(),
            85..=92 if !vars.is_empty() => format!(
                "{i} ← {};",
                vars[self.rng.below(vars.len() as u64) as usize]
            ),
            // GÓI 12: chia cho 1..=3 (chia 1 không giảm; kẹt ở 0 nếu điều kiện cho phép 0; số âm TĂNG khi chia cắt về 0)
            93..=96 => format!("{i} ← {i} / {};", self.lit(c)),
            97..=98 => format!("{i} ← {i} % {};", self.lit(1 + c)),
            _ => format!("{i} ← {i} + {};", self.lit(0)),
        };
        let variant = match self.rng.below(10) {
            0..=3 => String::new(),
            4..=6 => good_var.map_or(String::new(), |v| format!(" ↓ {v}")),
            7 => format!(" ↓ {i}"),
            8 => format!(" ↓ {}", self.lit(5)),
            _ => format!(" ↓ {} - {i}", self.lit(30)),
        };
        let mut inner = String::new();
        if depth > 1 && self.rng.chance(35) {
            let mut scoped = vars.clone();
            scoped.push(i.clone());
            inner = self.lp(&mut scoped, depth - 1);
        }
        let mut extra = String::new();
        if self.rng.chance(20) {
            extra = format!("⎇ {i} == {} {{ ⊠; }} ", self.lit(7));
        }
        if self.rng.chance(10) {
            extra.push_str(&format!("⎇ {i} == {} {{ ⊡; }} ", self.lit(3)));
        }
        let ret = if self.rng.chance(15) {
            format!("⎇ {i} == {} {{ ⏎ {i}; }} ", self.lit(9))
        } else {
            String::new()
        };
        vars.push(i.clone());
        format!("μ{i} ≔ {init}; ⟳ {cond}{variant} {{ {inner}{extra}{ret}{step} }} ")
    }

    pub fn program(&mut self) -> Prog {
        let t = self.ty;
        let mut src = String::new();
        let mut has_rec = false;
        // hàm đệ quy phụ
        if self.rng.chance(45) {
            has_rec = true;
            let call = match self.rng.below(7) {
                0..=2 => format!("r(n - {})", self.lit(1)),
                3 => "r(n)".to_string(),
                4 => format!("r(n + {})", self.lit(1)),
                5 => format!("r(n / {})", self.lit(2)),
                _ => format!("r(n - {})", self.lit(2)),
            };
            let var = match self.rng.below(6) {
                0..=2 => " ↓ n".to_string(),
                3 => String::new(),
                4 => format!(" ↓ {}", self.lit(3)),
                _ => " ↓ n".to_string(),
            };
            let base = if self.signed && self.rng.chance(50) {
                "n <= 0"
            } else {
                "n == 0"
            };
            let base = base.replace('0', &format!("0{t}"));
            src.push_str(&format!(
                "ƒ r(n: {t}) → {t} ! ∅{var} {{ ⎇ {base} {{ {} }} ⇝ {{ {call} }} }}\n",
                self.lit(0)
            ));
        }
        let np = 3;
        let mut vars: Vec<String> = (0..np).map(|k| format!("a{k}")).collect();
        let mut body = String::new();
        let nloops = 1 + self.rng.below(2);
        let mut has_loop = false;
        for _ in 0..nloops {
            body.push_str(&self.lp(&mut vars, 2));
            has_loop = true;
        }
        if has_rec {
            body.push_str(&format!(
                "z ≔ r({}); ",
                vars[self.rng.below(np as u64) as usize]
            ));
        }
        let params: Vec<String> = (0..np).map(|k| format!("a{k}: {t}")).collect();
        src.push_str(&format!(
            "ƒ entry({}) → {t} ! ∅ {{ {body}a0 }}\n",
            params.join(", ")
        ));
        Prog {
            src,
            ty: t,
            entry_params: np,
            has_loop,
            has_rec,
        }
    }
}
