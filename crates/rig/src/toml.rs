//! Bộ phân tích TOML CON, nghiêm ngặt, đúng phạm vi `rig.toml` (không phụ thuộc ngoài, ADR-030 §2): các mục `[tên]`, dòng `khoá = "chuỗi"`,
//! `khoá = { path = "chuỗi" }`, chú thích `#`. Bất cứ thứ gì khác (kiểu số, mảng, mục lồng, khoá trùng) ⇒ LỖI có số dòng — không đoán.
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Str(String),
    /// số nguyên dương trần (`edition = 1`): chỉ các chữ số thập phân, không dấu, không số 0 đầu (trừ `0`), vừa u64
    Int(u64),
    /// bảng nội tuyến `{ k = "v", … }` (chỉ giá trị chuỗi)
    Table(BTreeMap<String, String>),
}

/// mục → (khoá → (giá trị, số dòng))
pub type Doc = BTreeMap<String, BTreeMap<String, (Value, usize)>>;

fn is_key(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Chuỗi `"…"` với escape `\\ \" \n \t`; trả (nội dung, phần còn lại sau dấu `"` đóng).
fn parse_string(s: &str, line: usize) -> Result<(String, &str), String> {
    let mut chars = s.char_indices();
    match chars.next() {
        Some((_, '"')) => {}
        _ => return Err(format!("dòng {line}: cần chuỗi trong dấu \"…\"")),
    }
    let mut out = String::new();
    let mut esc = false;
    for (i, c) in chars {
        if esc {
            out.push(match c {
                '\\' => '\\',
                '"' => '"',
                'n' => '\n',
                't' => '\t',
                other => return Err(format!("dòng {line}: escape `\\{other}` không hỗ trợ")),
            });
            esc = false;
        } else if c == '\\' {
            esc = true;
        } else if c == '"' {
            return Ok((out, &s[i + 1..]));
        } else {
            out.push(c);
        }
    }
    Err(format!("dòng {line}: chuỗi không đóng"))
}

fn strip_comment(l: &str) -> &str {
    // `#` ngoài chuỗi bắt đầu chú thích
    let mut in_str = false;
    let mut esc = false;
    for (i, c) in l.char_indices() {
        if in_str {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '#' {
            return &l[..i];
        }
    }
    l
}

pub fn parse(text: &str) -> Result<Doc, String> {
    let mut doc: Doc = BTreeMap::new();
    let mut cur: Option<String> = None;
    for (idx, raw) in text.lines().enumerate() {
        let line = idx + 1;
        let l = strip_comment(raw).trim();
        if l.is_empty() {
            continue;
        }
        if let Some(rest) = l.strip_prefix('[') {
            let Some(name) = rest.strip_suffix(']') else {
                return Err(format!("dòng {line}: tiêu đề mục thiếu `]`"));
            };
            let name = name.trim();
            if !is_key(name) {
                return Err(format!(
                    "dòng {line}: tên mục `{name}` không hợp lệ (mục lồng/mảng bảng chưa hỗ trợ)"
                ));
            }
            if doc.contains_key(name) {
                return Err(format!("dòng {line}: mục `{name}` khai báo hai lần"));
            }
            doc.insert(name.to_string(), BTreeMap::new());
            cur = Some(name.to_string());
            continue;
        }
        let Some(sec) = cur.clone() else {
            return Err(format!("dòng {line}: khoá nằm ngoài mọi mục"));
        };
        let Some((k, v)) = l.split_once('=') else {
            return Err(format!("dòng {line}: cần `khoá = giá trị`"));
        };
        let k = k.trim();
        if !is_key(k) {
            return Err(format!("dòng {line}: khoá `{k}` không hợp lệ"));
        }
        let v = v.trim();
        let value = if v.starts_with('"') {
            let (s, rest) = parse_string(v, line)?;
            if !rest.trim().is_empty() {
                return Err(format!("dòng {line}: thừa ký tự sau chuỗi"));
            }
            Value::Str(s)
        } else if let Some(inner) = v.strip_prefix('{') {
            let Some(inner) = inner.trim_end().strip_suffix('}') else {
                return Err(format!("dòng {line}: bảng nội tuyến thiếu `}}`"));
            };
            let mut t = BTreeMap::new();
            let mut rest = inner.trim();
            while !rest.is_empty() {
                let Some((ik, after)) = rest.split_once('=') else {
                    return Err(format!("dòng {line}: cần `khoá = \"chuỗi\"` trong bảng"));
                };
                let ik = ik.trim();
                if !is_key(ik) {
                    return Err(format!("dòng {line}: khoá `{ik}` không hợp lệ"));
                }
                let (s, after) = parse_string(after.trim_start(), line)?;
                if t.insert(ik.to_string(), s).is_some() {
                    return Err(format!("dòng {line}: khoá `{ik}` trùng trong bảng"));
                }
                rest = after.trim_start();
                if let Some(r) = rest.strip_prefix(',') {
                    rest = r.trim_start();
                } else if !rest.is_empty() {
                    return Err(format!("dòng {line}: thiếu `,` giữa các cặp"));
                }
            }
            Value::Table(t)
        } else if !v.is_empty()
            && v.bytes().all(|b| b.is_ascii_digit())
            && (v == "0" || !v.starts_with('0'))
        {
            match v.parse::<u64>() {
                Ok(n) => Value::Int(n),
                Err(_) => return Err(format!("dòng {line}: số nguyên `{v}` quá lớn")),
            }
        } else {
            return Err(format!(
                "dòng {line}: giá trị phải là chuỗi \"…\", bảng {{ … }} hoặc số nguyên dương trần như `1` (số thập phân/âm/số 0 đầu/mảng/boolean chưa hỗ trợ)"
            ));
        };
        let sect = doc.entry(sec).or_default();
        if sect.insert(k.to_string(), (value, line)).is_some() {
            return Err(format!("dòng {line}: khoá `{k}` trùng"));
        }
    }
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest_shape() {
        let d = parse("# c\n[package]\nname = \"a\" # x\nversion = \"0.1.0\"\n\n[dependencies]\nb = { path = \"../b\" }\n").unwrap();
        assert_eq!(d["package"]["name"].0, Value::Str("a".into()));
        let Value::Table(t) = &d["dependencies"]["b"].0 else {
            panic!()
        };
        assert_eq!(t["path"], "../b");
    }

    #[test]
    fn rejects_everything_outside_the_subset_with_line_numbers() {
        for (src, needle) in [
            ("a = \"x\"\n", "ngoài mọi mục"),
            ("[p]\na = 1.5\n", "số nguyên"),
            ("[p]\na = -1\n", "số nguyên"),
            ("[p]\na = 01\n", "số nguyên"),
            ("[p]\na = 99999999999999999999\n", "quá lớn"),
            ("[p]\na = [\"x\"]\n", "chuỗi"),
            ("[p]\na = \"x\"\na = \"y\"\n", "trùng"),
            ("[p]\n[p]\n", "hai lần"),
            ("[p.q]\n", "không hợp lệ"),
            ("[p]\na = \"x\n", "không đóng"),
            ("[p]\na = \"x\" y\n", "thừa"),
            ("[p]\na = { path = \"x\" path = \"y\" }\n", "thiếu `,`"),
            ("[p]\na = { path = \"x\", path = \"y\" }\n", "trùng"),
            ("[p]\na = \"\\q\"\n", "escape"),
            ("[p]\na\n", "khoá = giá trị"),
        ] {
            let e = parse(src).unwrap_err();
            assert!(e.contains(needle), "{src:?} ⇒ {e}");
            assert!(e.starts_with("dòng "), "{e}");
        }
    }

    #[test]
    fn bare_positive_integers_parse_as_ints() {
        let d = parse("[p]\na = 1\nb = 0\nc = 42\n").unwrap();
        assert_eq!(d["p"]["a"].0, Value::Int(1));
        assert_eq!(d["p"]["b"].0, Value::Int(0));
        assert_eq!(d["p"]["c"].0, Value::Int(42));
    }

    #[test]
    fn hash_inside_string_is_not_a_comment() {
        let d = parse("[p]\na = \"x#y\"\n").unwrap();
        assert_eq!(d["p"]["a"].0, Value::Str("x#y".into()));
    }
}
