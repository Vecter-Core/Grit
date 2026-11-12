//! K2 calc — bản tham chiếu. Đọc stdin, mỗi dòng không rỗng là một biểu thức số nguyên i64. Đặc tả: docs/CAPSTONE.md.
use std::io::{Read, Write};

struct P<'a> {
    s: &'a [u8],
    i: usize,
}
type R = Result<i64, (usize, &'static str)>; // (cột 1-based, thông báo)

impl P<'_> {
    fn skip(&mut self) {
        while self.i < self.s.len() && (self.s[self.i] == b' ' || self.s[self.i] == b'\t') {
            self.i += 1;
        }
    }
    fn expr(&mut self) -> R {
        let mut v = self.term()?;
        loop {
            self.skip();
            let op = self.s.get(self.i).copied();
            let col = self.i + 1;
            match op {
                Some(b'+') => {
                    self.i += 1;
                    let r = self.term()?;
                    v = v.checked_add(r).ok_or((col, "overflow"))?;
                }
                Some(b'-') => {
                    self.i += 1;
                    let r = self.term()?;
                    v = v.checked_sub(r).ok_or((col, "overflow"))?;
                }
                _ => return Ok(v),
            }
        }
    }
    fn term(&mut self) -> R {
        let mut v = self.unary()?;
        loop {
            self.skip();
            let op = self.s.get(self.i).copied();
            let col = self.i + 1;
            match op {
                Some(b'*') => {
                    self.i += 1;
                    let r = self.unary()?;
                    v = v.checked_mul(r).ok_or((col, "overflow"))?;
                }
                Some(b'/') => {
                    self.i += 1;
                    let r = self.unary()?;
                    if r == 0 {
                        return Err((col, "division by zero"));
                    }
                    v = v.checked_div(r).ok_or((col, "overflow"))?;
                }
                _ => return Ok(v),
            }
        }
    }
    fn unary(&mut self) -> R {
        self.skip();
        if self.s.get(self.i) == Some(&b'-') {
            let col = self.i + 1;
            self.i += 1;
            let v = self.unary()?;
            return v.checked_neg().ok_or((col, "overflow"));
        }
        self.primary()
    }
    fn primary(&mut self) -> R {
        self.skip();
        let col = self.i + 1;
        match self.s.get(self.i).copied() {
            None => Err((col, "unexpected end of input")),
            Some(b'(') => {
                self.i += 1;
                let v = self.expr()?;
                self.skip();
                if self.s.get(self.i) == Some(&b')') {
                    self.i += 1;
                    Ok(v)
                } else if self.i >= self.s.len() {
                    Err((self.i + 1, "unexpected end of input"))
                } else {
                    Err((self.i + 1, "expected ')'"))
                }
            }
            Some(c) if c.is_ascii_digit() => {
                let mut v: i64 = 0;
                while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
                    let d = i64::from(self.s[self.i] - b'0');
                    v = v.checked_mul(10).and_then(|x| x.checked_add(d)).ok_or((col, "integer too large"))?;
                    self.i += 1;
                }
                Ok(v)
            }
            Some(_) => Err((col, "unexpected character")),
        }
    }
}

fn main() {
    let mut data = Vec::new();
    std::io::stdin().read_to_end(&mut data).ok();
    let mut lines: Vec<&[u8]> = data.split(|&b| b == b'\n').collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    let mut out = Vec::new();
    let mut failed = false;
    for line in lines {
        if line.iter().all(|&b| b == b' ' || b == b'\t') {
            continue;
        }
        let mut p = P { s: line, i: 0 };
        let r = p.expr().and_then(|v| {
            p.skip();
            if p.i < p.s.len() {
                Err((p.i + 1, "trailing input"))
            } else {
                Ok(v)
            }
        });
        match r {
            Ok(v) => out.extend_from_slice(format!("{v}\n").as_bytes()),
            Err((col, msg)) => {
                failed = true;
                out.extend_from_slice(format!("error: {msg} at {col}\n").as_bytes());
            }
        }
    }
    std::io::stdout().write_all(&out).ok();
    if failed {
        std::process::exit(1);
    }
}
