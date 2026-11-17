//! `gritc lsp` (Seed 1.0 S6): máy chủ LSP TỐI THIỂU qua stdio (JSON-RPC, khung `Content-Length`).
//! Có: `initialize`/`shutdown`/`exit`, đồng bộ TOÀN VĂN (`didOpen`/`didChange`/`didSave`/`didClose`), `publishDiagnostics` (mỗi chẩn đoán gritc = một chẩn đoán LSP; chỉ MÃ và vị trí
//! là giao diện đóng băng), `textDocument/formatting` (gt-fmt, toàn tài liệu). `textDocument/hover` (trên chẩn đoán: thông điệp + tài liệu `gritc explain`), `definition` (định nghĩa cấp ITEM từ `gt-resolve`, theo TÊN), `documentSymbol`. KHÔNG có: hoàn thành mã, đổi tên, sửa nhanh, định nghĩa của biến cục bộ.
//! LSP CHỈ HIỂN THỊ: dữ liệu lấy từ `check`/`resolve` của gritc; không có đường nào đổi phán quyết.
//! Nguồn kiểm: tài liệu đang mở (bộ đệm) + các tệp `.gt` cùng thư mục trên đĩa (gói Rig: `src/`) + gói `std` nếu tìm thấy (`GRIT_STD` hoặc `lib/std` cạnh `gritc`).
//! Vị trí LSP là (dòng 0-based, cột UTF-16) tính từ offset BYTE của gritc.
use crate::pipeline::{Options, check};
use gt_diag::Level;
use gt_resolve::SourceInput;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

fn read_msg(r: &mut impl BufRead) -> std::io::Result<Option<Value>> {
    let mut len: Option<usize> = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let l = line.trim_end();
        if l.is_empty() {
            break;
        }
        if let Some(v) = l.strip_prefix("Content-Length:") {
            len = v.trim().parse().ok();
        }
    }
    let Some(n) = len else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "thiếu Content-Length",
        ));
    };
    let mut buf = vec![0u8; n];
    std::io::Read::read_exact(r, &mut buf)?;
    serde_json::from_slice(&buf)
        .map(Some)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
}

fn send(w: &mut impl Write, v: &Value) -> std::io::Result<()> {
    let body = v.to_string();
    write!(w, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    w.flush()
}

/// (dòng 0-based, cột UTF-16) của offset byte `off` trong `text` (offset quá cuối ⇒ kẹp về cuối; giữa ký tự ⇒ lùi về đầu ký tự)
fn position(text: &str, off: usize) -> (usize, usize) {
    let mut off = off.min(text.len());
    while !text.is_char_boundary(off) {
        off -= 1;
    }
    let before = &text[..off];
    let line = before.matches('\n').count();
    let start = before.rfind('\n').map_or(0, |i| i + 1);
    let col: usize = text[start..off].chars().map(char::len_utf16).sum();
    (line, col)
}

fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let b = rest.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 3 <= b.len()
            && let Some(v) = std::str::from_utf8(&b[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).ok().map(PathBuf::from)
}

fn find_std() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("GRIT_STD") {
        let p = PathBuf::from(p);
        return p.join("rig.toml").is_file().then_some(p.join("src"));
    }
    let exe = std::env::current_exe().ok()?;
    exe.ancestors()
        .skip(1)
        .map(|d| d.join("lib").join("std"))
        .find(|c| c.join("rig.toml").is_file())
        .map(|c| c.join("src"))
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || "main.gt".to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// Thêm mọi tệp `.gt` (theo tên, chưa có) của `dir` vào đầu vào.
fn add_dir(
    dir: &Path,
    inputs: &mut Vec<SourceInput>,
    seen: &mut std::collections::BTreeSet<String>,
) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut names: Vec<PathBuf> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "gt"))
        .collect();
    names.sort();
    for p in names {
        if seen.insert(file_name(&p))
            && let Ok(t) = std::fs::read_to_string(&p)
        {
            inputs.push(SourceInput {
                name: file_name(&p),
                text: t,
            });
        }
    }
}

/// Dựng đầu vào kiểm cho tài liệu `uri`: (tên tệp, văn bản, danh sách đầu vào).
fn build_inputs(
    uri: &str,
    docs: &BTreeMap<String, String>,
) -> Option<(String, String, Vec<SourceInput>)> {
    let path = uri_to_path(uri)?;
    let me = file_name(&path);
    let mut inputs: Vec<SourceInput> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let text = docs.get(uri).cloned().unwrap_or_default();
    inputs.push(SourceInput {
        name: me.clone(),
        text: text.clone(),
    });
    seen.insert(me.clone());
    // bộ đệm đang mở khác cùng thư mục, rồi tệp trên đĩa, rồi gói std
    if let Some(dir) = path.parent() {
        for (u, t) in docs {
            if let Some(p) = uri_to_path(u)
                && p.parent() == Some(dir)
                && seen.insert(file_name(&p))
            {
                inputs.push(SourceInput {
                    name: file_name(&p),
                    text: t.clone(),
                });
            }
        }
        add_dir(dir, &mut inputs, &mut seen);
    }
    if let Some(std_dir) = find_std() {
        add_dir(&std_dir, &mut inputs, &mut seen);
    }
    Some((me, text, inputs))
}

type RawDiag = (usize, usize, Level, String, String);

/// Chẩn đoán THÔ của chính tài liệu. Lỗi nội bộ của `check` KHÔNG được nuốt: thành chẩn đoán E9901 (FAIL_OPEN_AUDIT G2.1.b).
fn raw_diagnostics(me: &str, inputs: &[SourceInput]) -> Vec<RawDiag> {
    match check(inputs, &Options::default()) {
        Ok(res) => res
            .report
            .diagnostics
            .iter()
            .filter(|d| d.file == me)
            .map(|d| {
                (
                    d.span.start as usize,
                    d.span.end as usize,
                    d.level,
                    d.code.as_str().to_string(),
                    d.message.clone(),
                )
            })
            .collect(),
        Err(e) => vec![(
            0,
            0,
            Level::Error,
            "E9901".to_string(),
            format!("gritc không kiểm được tài liệu này: {e}"),
        )],
    }
}

/// Kiểm tài liệu `uri` (văn bản `docs[uri]`): trả các chẩn đoán LSP của CHÍNH tài liệu đó.
fn diagnostics_for(uri: &str, docs: &BTreeMap<String, String>) -> Vec<Value> {
    let Some((me, text, inputs)) = build_inputs(uri, docs) else {
        return Vec::new();
    };
    raw_diagnostics(&me, &inputs)
        .into_iter()
        .map(|(s, e, level, code, message)| {
            let (sl, sc) = position(&text, s);
            let (el, ec) = position(&text, e);
            let sev = match level {
                Level::Error => 1,
                Level::Warn => 2,
                Level::Note => 3,
            };
            json!({
                "range": {"start": {"line": sl, "character": sc}, "end": {"line": el, "character": ec}},
                "severity": sev,
                "code": code,
                "source": "gritc",
                "message": message,
            })
        })
        .collect()
}

/// offset byte của (dòng 0-based, cột UTF-16); `None` nếu ngoài văn bản
fn offset_of(text: &str, line: usize, col16: usize) -> Option<usize> {
    let mut start = 0usize;
    for _ in 0..line {
        start += text[start..].find('\n')? + 1;
    }
    let mut c = 0usize;
    for (i, ch) in text[start..].char_indices() {
        if ch == '\n' || c >= col16 {
            return (c >= col16).then_some(start + i);
        }
        c += ch.len_utf16();
    }
    (c >= col16).then_some(text.len())
}

/// định danh chứa offset (chữ/số/`_` Unicode): (đầu, cuối)
fn ident_at(text: &str, off: usize) -> Option<(usize, usize)> {
    let is_id = |c: char| c.is_alphanumeric() || c == '_';
    let mut lo = off.min(text.len());
    while lo > 0 {
        let prev = text[..lo].chars().next_back()?;
        if is_id(prev) {
            lo -= prev.len_utf8();
        } else {
            break;
        }
    }
    let mut hi = lo;
    for ch in text[lo..].chars() {
        if is_id(ch) {
            hi += ch.len_utf8();
        } else {
            break;
        }
    }
    (hi > lo && off <= hi).then_some((lo, hi))
}

/// `textDocument/hover`: vị trí nằm trong một chẩn đoán ⇒ thông điệp + tài liệu `gritc explain <mã>`; ngược lại `null`. CHỈ HIỂN THỊ.
fn hover_for(uri: &str, docs: &BTreeMap<String, String>, line: usize, col: usize) -> Value {
    let Some((me, text, inputs)) = build_inputs(uri, docs) else {
        return Value::Null;
    };
    let Some(off) = offset_of(&text, line, col) else {
        return Value::Null;
    };
    let hits: Vec<RawDiag> = raw_diagnostics(&me, &inputs)
        .into_iter()
        .filter(|(s, e, ..)| *s <= off && (off < *e || (*s == *e && off == *s)))
        .collect();
    if hits.is_empty() {
        return Value::Null;
    }
    let mut md = String::new();
    for (_, _, _, code, msg) in &hits {
        md.push_str(&format!("**{code}** — {msg}\n\n"));
        match crate::cli::explain_text(code) {
            Some(doc) => md.push_str(doc),
            None => md.push_str(&format!("(chưa có tài liệu `gritc explain {code}`)")),
        }
        md.push_str("\n\n---\n");
    }
    json!({"contents": {"kind": "markdown", "value": md}})
}

fn path_to_uri(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file://");
    if !s.starts_with('/') {
        out.push('/');
    }
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~:".contains(&b) {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Chạy `gt-resolve` trên đầu vào (tên tệp qua cùng phép ánh xạ module của cli): (kết quả, tên module theo chỉ số đầu vào).
fn resolve_inputs(inputs: &[SourceInput]) -> Option<(gt_resolve::ResolveOutput, Vec<String>)> {
    let prelude = gt_resolve::Prelude::from_source(&crate::pipeline::prelude_text()).ok()?;
    let mapped: Vec<SourceInput> = inputs
        .iter()
        .map(|i| SourceInput {
            name: gt_resolve::module_file_name_for(&i.name),
            text: i.text.clone(),
        })
        .collect();
    let mods: Vec<String> = mapped
        .iter()
        .map(|i| i.name.strip_suffix(".gt").unwrap_or(&i.name).to_string())
        .collect();
    let out = gt_resolve::resolve_program(&mapped, &prelude).ok()?;
    Some((out, mods))
}

fn range_json(text: &str, s: usize, e: usize) -> Value {
    let (sl, sc) = position(text, s);
    let (el, ec) = position(text, e);
    json!({"start": {"line": sl, "character": sc}, "end": {"line": el, "character": ec}})
}

/// `textDocument/definition`: định danh dưới con trỏ ⇒ mọi định nghĩa CẤP ITEM trùng tên trong chương trình (từ `gt-resolve`).
fn definition_for(uri: &str, docs: &BTreeMap<String, String>, line: usize, col: usize) -> Value {
    let Some((me, text, inputs)) = build_inputs(uri, docs) else {
        return Value::Null;
    };
    let Some(off) = offset_of(&text, line, col) else {
        return Value::Null;
    };
    let Some((lo, hi)) = ident_at(&text, off) else {
        return Value::Null;
    };
    let name = &text[lo..hi];
    let Some((out, mods)) = resolve_inputs(&inputs) else {
        return Value::Null;
    };
    let dir = uri_to_path(uri).and_then(|p| p.parent().map(Path::to_path_buf));
    let std_dir = find_std();
    let mut locs = Vec::new();
    for d in &out.defs {
        let last = d.path.rsplit("::").next().unwrap_or("");
        if last != name || d.path.starts_with("$prelude") {
            continue;
        }
        let Some(modname) = out.module_names.get(d.file.index()) else {
            continue;
        };
        let Some(k) = mods.iter().position(|m| m == modname) else {
            continue;
        };
        let duri = if inputs[k].name == me {
            uri.to_string()
        } else {
            let found = [dir.as_ref(), std_dir.as_ref()]
                .into_iter()
                .flatten()
                .map(|d| d.join(&inputs[k].name))
                .find(|p| p.is_file());
            match found {
                Some(p) => path_to_uri(&p),
                None => continue,
            }
        };
        let r = range_json(
            &inputs[k].text,
            d.name_span.start as usize,
            d.name_span.end as usize,
        );
        locs.push(json!({"uri": duri, "range": r}));
    }
    if locs.is_empty() {
        Value::Null
    } else {
        Value::Array(locs)
    }
}

/// `textDocument/documentSymbol`: item của CHÍNH tài liệu (từ `gt-resolve`).
fn symbols_for(uri: &str, docs: &BTreeMap<String, String>) -> Value {
    let Some((me, text, inputs)) = build_inputs(uri, docs) else {
        return Value::Array(Vec::new());
    };
    let Some((out, mods)) = resolve_inputs(&inputs) else {
        return Value::Array(Vec::new());
    };
    let my_mod = mods.first().cloned().unwrap_or_default();
    let _ = me;
    let mut syms = Vec::new();
    for d in &out.defs {
        if d.path.starts_with("$prelude") || out.module_names.get(d.file.index()) != Some(&my_mod) {
            continue;
        }
        let kind = match d.kind {
            gt_resolve::DefKind::Module => 2,
            gt_resolve::DefKind::Fn => 12,
            gt_resolve::DefKind::Struct => 23,
            gt_resolve::DefKind::Enum => 10,
            gt_resolve::DefKind::Trait => 11,
            gt_resolve::DefKind::TypeAlias => 26,
            gt_resolve::DefKind::Const | gt_resolve::DefKind::Static => 14,
            gt_resolve::DefKind::Variant => 22,
        };
        let r = range_json(&text, d.name_span.start as usize, d.name_span.end as usize);
        syms.push(json!({"name": d.path.rsplit("::").next().unwrap_or(""), "kind": kind, "range": r, "selectionRange": r}));
    }
    Value::Array(syms)
}

fn publish(w: &mut impl Write, uri: &str, docs: &BTreeMap<String, String>) -> std::io::Result<()> {
    let diags = if docs.contains_key(uri) {
        diagnostics_for(uri, docs)
    } else {
        Vec::new()
    };
    send(
        w,
        &json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": {"uri": uri, "diagnostics": diags}}),
    )
}

/// Chạy máy chủ tới khi `exit` hoặc hết đầu vào; trả mã thoát (0 nếu `shutdown` rồi `exit`, 1 nếu `exit` không có `shutdown`).
pub fn run() -> i32 {
    let stdin = std::io::stdin();
    let mut r = stdin.lock();
    let stdout = std::io::stdout();
    let mut w = stdout.lock();
    let mut docs: BTreeMap<String, String> = BTreeMap::new();
    let mut shutdown = false;
    loop {
        let msg = match read_msg(&mut r) {
            Ok(Some(m)) => m,
            Ok(None) => return i32::from(!shutdown),
            Err(_) => return 1,
        };
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let id = msg.get("id").cloned();
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let uri = params
            .pointer("/textDocument/uri")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let ok = match method {
            "initialize" => send(
                &mut w,
                &json!({"jsonrpc": "2.0", "id": id, "result": {
                    "capabilities": {"textDocumentSync": 1, "documentFormattingProvider": true, "hoverProvider": true, "definitionProvider": true, "documentSymbolProvider": true},
                    "serverInfo": {"name": "gritc", "version": env!("CARGO_PKG_VERSION")}}}),
            ),
            "shutdown" => {
                shutdown = true;
                send(
                    &mut w,
                    &json!({"jsonrpc": "2.0", "id": id, "result": Value::Null}),
                )
            }
            "exit" => return i32::from(!shutdown),
            "textDocument/didOpen" => {
                let text = params
                    .pointer("/textDocument/text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                docs.insert(uri.clone(), text);
                publish(&mut w, &uri, &docs)
            }
            "textDocument/didChange" => {
                if let Some(t) = params
                    .pointer("/contentChanges")
                    .and_then(Value::as_array)
                    .and_then(|a| a.last())
                    .and_then(|c| c.get("text"))
                    .and_then(Value::as_str)
                {
                    docs.insert(uri.clone(), t.to_string());
                }
                publish(&mut w, &uri, &docs)
            }
            "textDocument/didSave" => publish(&mut w, &uri, &docs),
            "textDocument/didClose" => {
                docs.remove(&uri);
                publish(&mut w, &uri, &docs)
            }
            "textDocument/hover" | "textDocument/definition" => {
                let line = params
                    .pointer("/position/line")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize;
                let col = params
                    .pointer("/position/character")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize;
                let result = if method == "textDocument/hover" {
                    hover_for(&uri, &docs, line, col)
                } else {
                    definition_for(&uri, &docs, line, col)
                };
                send(
                    &mut w,
                    &json!({"jsonrpc": "2.0", "id": id, "result": result}),
                )
            }
            "textDocument/documentSymbol" => {
                let result = symbols_for(&uri, &docs);
                send(
                    &mut w,
                    &json!({"jsonrpc": "2.0", "id": id, "result": result}),
                )
            }
            "textDocument/formatting" => {
                let result = match docs.get(&uri) {
                    Some(text) => match gt_fmt::format(&uri, text) {
                        Ok(new) => {
                            let (el, ec) = position(text, text.len());
                            json!([{"range": {"start": {"line": 0, "character": 0}, "end": {"line": el, "character": ec}}, "newText": new}])
                        }
                        Err(_) => Value::Null, // không định dạng được (cú pháp sai): KHÔNG sửa gì
                    },
                    None => Value::Null,
                };
                send(
                    &mut w,
                    &json!({"jsonrpc": "2.0", "id": id, "result": result}),
                )
            }
            _ => match id {
                Some(i) => send(
                    &mut w,
                    &json!({"jsonrpc": "2.0", "id": i, "error": {"code": -32601, "message": format!("không hỗ trợ: {method}")}}),
                ),
                None => Ok(()), // thông báo lạ: bỏ qua
            },
        };
        if ok.is_err() {
            return 1;
        }
    }
}
