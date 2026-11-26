//! Đọc văn bản GLIR (định dạng của `print`). Lỗi ⇒ `Err("dòng N: …")`; không panic với mọi đầu vào.
use crate::print::{BINOPS, CASTS, FBINOPS, FPREDS, FUNOPS, ORDS, PREDS, RMWOPS};
use crate::{Block, Decl, Func, Inst, Module, Opd, Term, Ty};

type R<T> = Result<T, String>;

fn tyname(t: &str) -> Option<Ty> {
    Some(match t {
        "i1" => Ty::I1,
        "i8" => Ty::I8,
        "i16" => Ty::I16,
        "i32" => Ty::I32,
        "i64" => Ty::I64,
        "i128" => Ty::I128,
        "f64" => Ty::F64,
        "ptr" => Ty::Ptr,
        "void" => Ty::Void,
        _ => return None,
    })
}

/// Đọc một kiểu từ đầu `s`; trả (kiểu, phần còn lại).
fn parse_ty(s: &str, depth: u32) -> R<(Ty, &str)> {
    if depth > 64 {
        return Err("kiểu lồng quá sâu".into());
    }
    let s = s.trim_start();
    if let Some(rest) = s.strip_prefix('{') {
        let mut fs = Vec::new();
        let mut r = rest.trim_start();
        if let Some(r2) = r.strip_prefix('}') {
            return Ok((Ty::Struct(fs), r2));
        }
        loop {
            let (t, r2) = parse_ty(r, depth + 1)?;
            fs.push(t);
            let r2 = r2.trim_start();
            if let Some(r3) = r2.strip_prefix(',') {
                r = r3;
            } else if let Some(r3) = r2.strip_prefix('}') {
                return Ok((Ty::Struct(fs), r3));
            } else {
                return Err("thiếu `,` hoặc `}` trong kiểu struct".into());
            }
        }
    }
    if let Some(rest) = s.strip_prefix('[') {
        let (n, r) = rest.split_once(" x ").ok_or("kiểu mảng cần `[n x T]`")?;
        let n: u64 = n.trim().parse().map_err(|_| "độ dài mảng sai")?;
        let (t, r) = parse_ty(r, depth + 1)?;
        let r = r.trim_start().strip_prefix(']').ok_or("thiếu `]`")?;
        return Ok((Ty::Array(Box::new(t), n), r));
    }
    let end = s
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(s.len());
    let t = tyname(&s[..end]).ok_or_else(|| format!("kiểu lạ `{}`", &s[..end]))?;
    Ok((t, &s[end..]))
}

fn only_ty(s: &str) -> R<Ty> {
    let (t, r) = parse_ty(s, 0)?;
    if r.trim().is_empty() {
        Ok(t)
    } else {
        Err(format!("thừa sau kiểu: `{}`", r.trim()))
    }
}

fn parse_opd(s: &str) -> R<Opd> {
    let s = s.trim();
    if let Some(v) = s.strip_prefix('%') {
        return Ok(Opd::Val(ident(v)?));
    }
    if let Some(v) = s.strip_prefix('$') {
        return Ok(Opd::Slot(ident(v)?));
    }
    if let Some(v) = s.strip_prefix('@') {
        return Ok(Opd::Global(ident(v)?));
    }
    if s == "null" {
        return Ok(Opd::Null);
    }
    let (t, r) = parse_ty(s, 0)?;
    if t == Ty::F64 {
        // `f64 0x` + ĐÚNG 16 chữ số hex thường (dạng duy nhất mà `print` sinh ⇒ in(đọc(x)) = x)
        let h = r
            .trim()
            .strip_prefix("0x")
            .filter(|h| h.len() == 16 && h.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')))
            .ok_or_else(|| {
                format!(
                    "hằng f64 phải là `0x` + 16 chữ số hex thường, có `{}`",
                    r.trim()
                )
            })?;
        return u64::from_str_radix(h, 16)
            .map(Opd::F64)
            .map_err(|_| format!("hằng f64 sai `{h}`"));
    }
    if !t.is_int() {
        return Err("hằng phải có kiểu số nguyên".into());
    }
    let n: i128 = r
        .trim()
        .parse()
        .map_err(|_| format!("hằng sai `{}`", r.trim()))?;
    Ok(Opd::Int(t, n))
}

fn ident(s: &str) -> R<String> {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        Ok(s.to_string())
    } else {
        Err(format!("tên sai `{s}`"))
    }
}

/// Tách theo `,` ở độ sâu 0 của `{}`/`[]`/`()`.
fn split_top(s: &str) -> Vec<&str> {
    let (mut depth, mut start, mut out) = (0i32, 0usize, Vec::new());
    for (i, c) in s.char_indices() {
        match c {
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if !s.trim().is_empty() {
        out.push(&s[start..]);
    }
    out
}

fn lookup<T: Copy>(tbl: &[(T, &str)], s: &str) -> R<T> {
    tbl.iter()
        .find(|(_, n)| *n == s)
        .map(|(k, _)| *k)
        .ok_or_else(|| format!("từ khoá lạ `{s}`"))
}

/// `<ty> <rest>` với kiểu ở đầu.
fn ty_then(s: &str) -> R<(Ty, &str)> {
    parse_ty(s, 0)
}

fn parse_inst(line: &str) -> R<Inst> {
    let (d, body) = match line.split_once(" = ") {
        Some((d, b)) if d.starts_with('%') => (Some(ident(&d[1..])?), b),
        _ => (None, line),
    };
    let (op, rest) = body.split_once(' ').unwrap_or((body, ""));
    let need = |d: Option<String>| d.ok_or_else(|| format!("`{op}` cần `%d =`"));
    match op {
        "bin" => {
            let (o, r) = rest.split_once(' ').ok_or("bin thiếu toán tử")?;
            let (t, r) = ty_then(r)?;
            let [a, b] = split_top(r)[..] else {
                return Err("bin cần 2 toán hạng".into());
            };
            Ok(Inst::Bin {
                d: need(d)?,
                op: lookup(&BINOPS, o)?,
                ty: t,
                a: parse_opd(a)?,
                b: parse_opd(b)?,
            })
        }
        "icmp" => {
            let (o, r) = rest.split_once(' ').ok_or("icmp thiếu vị từ")?;
            let (t, r) = ty_then(r)?;
            let [a, b] = split_top(r)[..] else {
                return Err("icmp cần 2 toán hạng".into());
            };
            Ok(Inst::ICmp {
                d: need(d)?,
                pred: lookup(&PREDS, o)?,
                ty: t,
                a: parse_opd(a)?,
                b: parse_opd(b)?,
            })
        }
        "fbin" | "fcmp" => {
            let (o, r) = rest.split_once(' ').ok_or("thiếu toán tử/vị từ")?;
            let (t, r) = ty_then(r)?;
            let [a, b] = split_top(r)[..] else {
                return Err(format!("{op} cần 2 toán hạng"));
            };
            let (d, a, b) = (need(d)?, parse_opd(a)?, parse_opd(b)?);
            Ok(if op == "fbin" {
                Inst::FBin {
                    d,
                    op: lookup(&FBINOPS, o)?,
                    ty: t,
                    a,
                    b,
                }
            } else {
                Inst::FCmp {
                    d,
                    pred: lookup(&FPREDS, o)?,
                    ty: t,
                    a,
                    b,
                }
            })
        }
        "atomicload" => {
            let (o, r) = rest.split_once(' ').ok_or("atomicload thiếu thứ tự")?;
            let (t, r) = ty_then(r)?;
            Ok(Inst::AtomicLoad {
                d: need(d)?,
                ord: lookup(&ORDS, o)?,
                ty: t,
                ptr: parse_opd(r)?,
            })
        }
        "atomicstore" => {
            if d.is_some() {
                return Err("`atomicstore` không định nghĩa giá trị (`%d =` thừa)".into());
            }
            let (o, r) = rest.split_once(' ').ok_or("atomicstore thiếu thứ tự")?;
            let (t, r) = ty_then(r)?;
            let [v, p] = split_top(r)[..] else {
                return Err("atomicstore cần 2 toán hạng".into());
            };
            Ok(Inst::AtomicStore {
                ord: lookup(&ORDS, o)?,
                ty: t,
                val: parse_opd(v)?,
                ptr: parse_opd(p)?,
            })
        }
        "atomicrmw" => {
            let (o, r) = rest.split_once(' ').ok_or("atomicrmw thiếu phép")?;
            let (od, r) = r.split_once(' ').ok_or("atomicrmw thiếu thứ tự")?;
            let (t, r) = ty_then(r)?;
            let [p, v] = split_top(r)[..] else {
                return Err("atomicrmw cần 2 toán hạng".into());
            };
            Ok(Inst::AtomicRmw {
                d: need(d)?,
                op: lookup(&RMWOPS, o)?,
                ord: lookup(&ORDS, od)?,
                ty: t,
                ptr: parse_opd(p)?,
                val: parse_opd(v)?,
            })
        }
        "cmpxchg" => {
            let (s1, r) = rest.split_once(' ').ok_or("cmpxchg thiếu thứ tự")?;
            let (s2, r) = r.split_once(' ').ok_or("cmpxchg thiếu thứ tự thất bại")?;
            let (t, r) = ty_then(r)?;
            let [p, c, n] = split_top(r)[..] else {
                return Err("cmpxchg cần 3 toán hạng".into());
            };
            Ok(Inst::CmpXchg {
                d: need(d)?,
                succ: lookup(&ORDS, s1)?,
                fail: lookup(&ORDS, s2)?,
                ty: t,
                ptr: parse_opd(p)?,
                cmp: parse_opd(c)?,
                new: parse_opd(n)?,
            })
        }
        "funop" => {
            let (o, r) = rest.split_once(' ').ok_or("funop thiếu toán tử")?;
            let (t, r) = ty_then(r)?;
            Ok(Inst::FUn {
                d: need(d)?,
                op: lookup(&FUNOPS, o)?,
                ty: t,
                v: parse_opd(r)?,
            })
        }
        "select" => {
            let (t, r) = ty_then(rest)?;
            let [c, a, b] = split_top(r)[..] else {
                return Err("select cần 3 toán hạng".into());
            };
            Ok(Inst::Select {
                d: need(d)?,
                ty: t,
                c: parse_opd(c)?,
                a: parse_opd(a)?,
                b: parse_opd(b)?,
            })
        }
        "cast" => {
            let (o, r) = rest.split_once(' ').ok_or("cast thiếu toán tử")?;
            let (from, r) = ty_then(r)?;
            let (v, to) = r.rsplit_once(" to ").ok_or("cast thiếu `to`")?;
            Ok(Inst::Cast {
                d: need(d)?,
                op: lookup(&CASTS, o)?,
                from,
                v: parse_opd(v)?,
                to: only_ty(to)?,
            })
        }
        "load" => {
            let (t, r) = ty_then(rest)?;
            Ok(Inst::Load {
                d: need(d)?,
                ty: t,
                ptr: parse_opd(r)?,
            })
        }
        "store" => {
            // FB-G2-04: `%x = store …` không được nuốt im lặng `%x`
            if d.is_some() {
                return Err("`store` không định nghĩa giá trị (`%d =` thừa)".into());
            }
            let (t, r) = ty_then(rest)?;
            let [v, p] = split_top(r)[..] else {
                return Err("store cần 2 toán hạng".into());
            };
            Ok(Inst::Store {
                ty: t,
                val: parse_opd(v)?,
                ptr: parse_opd(p)?,
            })
        }
        "fieldptr" => {
            let (t, r) = ty_then(rest)?;
            let [p, i] = split_top(r)[..] else {
                return Err("fieldptr cần 2 toán hạng".into());
            };
            Ok(Inst::FieldPtr {
                d: need(d)?,
                ty: t,
                ptr: parse_opd(p)?,
                idx: i.trim().parse().map_err(|_| "chỉ số trường sai")?,
            })
        }
        "indexptr" => {
            let (t, r) = ty_then(rest)?;
            let [p, i] = split_top(r)[..] else {
                return Err("indexptr cần 2 toán hạng".into());
            };
            Ok(Inst::IndexPtr {
                d: need(d)?,
                elem: t,
                ptr: parse_opd(p)?,
                idx: parse_opd(i)?,
            })
        }
        "call" => {
            let (ret, r) = ty_then(rest)?;
            let r = r
                .trim_start()
                .strip_prefix('@')
                .ok_or("call thiếu `@tên`")?;
            let (callee, r) = r.split_once('(').ok_or("call thiếu `(`")?;
            let args = r.trim_end().strip_suffix(')').ok_or("call thiếu `)`")?;
            let args = split_top(args)
                .into_iter()
                .map(parse_opd)
                .collect::<R<Vec<_>>>()?;
            Ok(Inst::Call {
                d,
                ret,
                callee: ident(callee)?,
                args,
            })
        }
        "phi" => {
            let (t, r) = ty_then(rest)?;
            let mut ins = Vec::new();
            for part in split_top(r) {
                let p = part
                    .trim()
                    .strip_prefix('[')
                    .and_then(|p| p.strip_suffix(']'))
                    .ok_or("phi cần `[v, bb]`")?;
                let (v, b) = p.rsplit_once(',').ok_or("phi cần `[v, bb]`")?;
                ins.push((parse_opd(v)?, ident(b.trim())?));
            }
            Ok(Inst::Phi {
                d: need(d)?,
                ty: t,
                ins,
            })
        }
        other => Err(format!("lệnh lạ `{other}`")),
    }
}

fn parse_term(line: &str) -> R<Option<Term>> {
    let (op, rest) = line.split_once(' ').unwrap_or((line, ""));
    Ok(Some(match op {
        "br" => Term::Br(ident(rest.trim())?),
        "condbr" => {
            let [c, a, b] = split_top(rest)[..] else {
                return Err("condbr cần 3 phần".into());
            };
            Term::CondBr(parse_opd(c)?, ident(a.trim())?, ident(b.trim())?)
        }
        "ret" if rest.trim() == "void" => Term::Ret(None),
        "ret" => {
            let (t, r) = ty_then(rest)?;
            Term::Ret(Some((t, parse_opd(r)?)))
        }
        "abort" => Term::Abort(ident(rest.trim())?),
        _ => return Ok(None),
    }))
}

pub fn parse(src: &str) -> R<Module> {
    let mut m = Module::default();
    let mut lines = src
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with("//"));
    let (n0, first) = lines.next().ok_or("rỗng")?;
    if first != "glir 1" {
        return Err(format!("dòng {n0}: cần `glir 1`"));
    }
    let wrap = |n: usize, e: String| format!("dòng {n}: {e}");
    while let Some((n, l)) = lines.next() {
        if let Some(rest) = l.strip_prefix("global @") {
            let (name, hex) = rest
                .split_once(" = ")
                .ok_or_else(|| wrap(n, "global cần `@tên = hex`".into()))?;
            let hex = hex.trim();
            let mut bytes = Vec::new();
            if hex != "-" {
                if hex.len() % 2 != 0 || !hex.is_ascii() {
                    return Err(wrap(n, "hex của global sai".into()));
                }
                for i in (0..hex.len()).step_by(2) {
                    bytes.push(
                        u8::from_str_radix(&hex[i..i + 2], 16)
                            .map_err(|_| wrap(n, "hex của global sai".into()))?,
                    );
                }
            }
            m.globals.push(crate::Global {
                name: ident(name).map_err(|e| wrap(n, e))?,
                bytes,
            });
        } else if let Some(rest) = l.strip_prefix("declare @") {
            let (name, r) = rest
                .split_once('(')
                .ok_or_else(|| wrap(n, "declare thiếu `(`".into()))?;
            let (ps, ret) = r
                .rsplit_once(") -> ")
                .ok_or_else(|| wrap(n, "declare thiếu `) ->`".into()))?;
            let params = split_top(ps)
                .into_iter()
                .map(only_ty)
                .collect::<R<Vec<_>>>()
                .map_err(|e| wrap(n, e))?;
            m.decls.push(Decl {
                name: ident(name).map_err(|e| wrap(n, e))?,
                params,
                ret: only_ty(ret).map_err(|e| wrap(n, e))?,
            });
        } else if let Some(rest) = l.strip_prefix("fn @") {
            let (name, r) = rest
                .split_once('(')
                .ok_or_else(|| wrap(n, "fn thiếu `(`".into()))?;
            let (ps, ret) = r
                .rsplit_once(") -> ")
                .ok_or_else(|| wrap(n, "fn thiếu `) ->`".into()))?;
            let ret = ret
                .trim()
                .strip_suffix('{')
                .ok_or_else(|| wrap(n, "fn thiếu `{`".into()))?;
            let mut params = Vec::new();
            for p in split_top(ps) {
                let (pn, pt) = p
                    .trim()
                    .strip_prefix('%')
                    .and_then(|p| p.split_once(':'))
                    .ok_or_else(|| wrap(n, "tham số cần `%tên: kiểu`".into()))?;
                params.push((
                    ident(pn.trim()).map_err(|e| wrap(n, e))?,
                    only_ty(pt).map_err(|e| wrap(n, e))?,
                ));
            }
            let mut f = Func {
                name: ident(name).map_err(|e| wrap(n, e))?,
                params,
                ret: only_ty(ret).map_err(|e| wrap(n, e))?,
                slots: Vec::new(),
                blocks: Vec::new(),
            };
            let mut cur: Option<(String, Vec<Inst>)> = None;
            loop {
                let Some((n, l)) = lines.next() else {
                    return Err(wrap(n, "hàm không đóng `}`".into()));
                };
                if l == "}" {
                    if cur.is_some() {
                        return Err(wrap(n, "khối cuối thiếu terminator".into()));
                    }
                    break;
                }
                if let Some(s) = l.strip_prefix("slot $") {
                    // FB-G2-04: slot thuộc hàm, chỉ đứng TRƯỚC khối đầu tiên (đúng dạng `print`)
                    if cur.is_some() || !f.blocks.is_empty() {
                        return Err(wrap(n, "`slot` phải đứng trước khối đầu tiên".into()));
                    }
                    let (sn, st) = s
                        .split_once(':')
                        .ok_or_else(|| wrap(n, "slot cần `$tên: kiểu`".into()))?;
                    f.slots.push((
                        ident(sn.trim()).map_err(|e| wrap(n, e))?,
                        only_ty(st).map_err(|e| wrap(n, e))?,
                    ));
                    continue;
                }
                if let Some(label) = l.strip_suffix(':') {
                    if cur.is_some() {
                        return Err(wrap(n, "khối trước thiếu terminator".into()));
                    }
                    cur = Some((ident(label).map_err(|e| wrap(n, e))?, Vec::new()));
                    continue;
                }
                let Some((name, insts)) = cur.as_mut() else {
                    return Err(wrap(n, "lệnh ngoài khối".into()));
                };
                if let Some(t) = parse_term(l).map_err(|e| wrap(n, e))? {
                    f.blocks.push(Block {
                        name: std::mem::take(name),
                        insts: std::mem::take(insts),
                        term: t,
                    });
                    cur = None;
                } else {
                    insts.push(parse_inst(l).map_err(|e| wrap(n, e))?);
                }
            }
            m.funcs.push(f);
        } else {
            return Err(wrap(n, format!("không hiểu `{l}`")));
        }
    }
    Ok(m)
}
