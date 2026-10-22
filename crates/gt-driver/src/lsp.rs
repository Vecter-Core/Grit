//! `gritc lsp` (Seed 1.0 S6): máy chủ LSP TỐI THIỂU qua stdio (JSON-RPC, khung `Content-Length`).
//! Có: `initialize`/`shutdown`/`exit`, đồng bộ TOÀN VĂN (`didOpen`/`didChange`/`didSave`/`didClose`), `publishDiagnostics` (mỗi chẩn đoán gritc = một chẩn đoán LSP; chỉ MÃ và vị trí
//! là giao diện đóng băng), `textDocument/formatting` (gt-fmt, toàn tài liệu). KHÔNG có: hover, đi tới định nghĩa, hoàn thành mã, đổi tên, sửa nhanh.
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

/// Kiểm tài liệu `uri` (văn bản `docs[uri]`): trả các chẩn đoán LSP của CHÍNH tài liệu đó.
fn diagnostics_for(uri: &str, docs: &BTreeMap<String, String>) -> Vec<Value> {
    let Some(path) = uri_to_path(uri) else {
        return Vec::new();
    };
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
    let Ok(res) = check(&inputs, &Options::default()) else {
        return Vec::new();
    };
    res.report
        .diagnostics
        .iter()
        .filter(|d| d.file == me)
        .map(|d| {
            let (s, e) = (d.span.start as usize, d.span.end as usize);
            let (sl, sc) = position(&text, s);
            let (el, ec) = position(&text, e);
            let sev = match d.level {
                Level::Error => 1,
                Level::Warn => 2,
                Level::Note => 3,
            };
            json!({
                "range": {"start": {"line": sl, "character": sc}, "end": {"line": el, "character": ec}},
                "severity": sev,
                "code": d.code.as_str(),
                "source": "gritc",
                "message": d.message,
            })
        })
        .collect()
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
                    "capabilities": {"textDocumentSync": 1, "documentFormattingProvider": true},
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
