//! Seed 1.0 S6: `gritc lsp` nói LSP thật qua stdio — khởi tạo, chẩn đoán đúng mã và vị trí (UTF-16), xoá chẩn đoán khi sửa, định dạng, lỗi cho phương thức lạ, thoát sạch.
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

struct Server {
    child: std::process::Child,
    out: BufReader<std::process::ChildStdout>,
}

impl Server {
    fn start() -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_gritc"))
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("chạy gritc lsp");
        let out = BufReader::new(child.stdout.take().expect("stdout"));
        Server { child, out }
    }
    fn send(&mut self, v: &Value) {
        let body = v.to_string();
        let w = self.child.stdin.as_mut().expect("stdin");
        write!(w, "Content-Length: {}\r\n\r\n{}", body.len(), body).expect("ghi");
        w.flush().expect("flush");
    }
    fn recv(&mut self) -> Value {
        let mut len = 0usize;
        loop {
            let mut l = String::new();
            assert!(
                self.out.read_line(&mut l).expect("đọc") > 0,
                "máy chủ đóng sớm"
            );
            let l = l.trim_end().to_string();
            if l.is_empty() {
                break;
            }
            if let Some(v) = l.strip_prefix("Content-Length:") {
                len = v.trim().parse().expect("độ dài");
            }
        }
        let mut buf = vec![0u8; len];
        self.out.read_exact(&mut buf).expect("thân");
        serde_json::from_slice(&buf).expect("JSON")
    }
    fn open(&mut self, uri: &str, text: &str) -> Value {
        self.send(&json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"languageId":"grit","version":1,"text":text}}}));
        self.recv()
    }
}

#[test]
fn lsp_session_diagnostics_formatting_and_exit() {
    let mut s = Server::start();
    s.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}));
    let init = s.recv();
    assert_eq!(init["id"], 1);
    assert_eq!(
        init["result"]["capabilities"]["documentFormattingProvider"],
        true
    );
    assert_eq!(init["result"]["capabilities"]["textDocumentSync"], 1);
    s.send(&json!({"jsonrpc":"2.0","method":"initialized","params":{}}));

    let dir = std::env::temp_dir().join(format!("gt-lsp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo thư mục");
    let uri = format!("file://{}/main.gt", dir.display());

    // chương trình SAI kiểu; ký tự ngoài BMP (😀 = 2 đơn vị UTF-16, 4 byte UTF-8) nằm TRƯỚC lỗi trên CÙNG dòng ⇒ cột phải tính theo UTF-16
    let bad = "// chú thích\nƒ main() → u32 ! ∅ { s ≔ \"😀\"; true }\n";
    let p = s.open(&uri, bad);
    assert_eq!(p["method"], "textDocument/publishDiagnostics");
    assert_eq!(p["params"]["uri"], uri.as_str());
    let d = p["params"]["diagnostics"].as_array().expect("mảng");
    assert!(!d.is_empty(), "{p}");
    let e = d
        .iter()
        .find(|x| x["code"] == "E0301")
        .unwrap_or_else(|| panic!("thiếu E0301: {p}"));
    assert_eq!(e["severity"], 1);
    assert_eq!(e["source"], "gritc");
    assert_eq!(e["range"]["start"]["line"], 1, "{p}");
    let col_true = "ƒ main() → u32 ! ∅ { s ≔ \"😀\"; ".encode_utf16().count();
    assert_eq!(e["range"]["start"]["character"], col_true, "{p}");

    // sửa đúng ⇒ chẩn đoán rỗng
    s.send(&json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":"ƒ main() → u32 ! ∅ { 1u32 }\n"}]}}));
    let p = s.recv();
    assert!(
        p["params"]["diagnostics"]
            .as_array()
            .expect("mảng")
            .iter()
            .all(|x| x["severity"] != 1),
        "{p}"
    );

    // định dạng: mã lộn xộn ⇒ một TextEdit toàn văn bằng đầu ra gt-fmt
    s.send(&json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":3},"contentChanges":[{"text":"ƒ   main()  →  u32 ! ∅ {   1u32   }\n"}]}}));
    let _ = s.recv();
    s.send(&json!({"jsonrpc":"2.0","id":2,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri},"options":{"tabSize":2,"insertSpaces":true}}}));
    let f = s.recv();
    assert_eq!(f["id"], 2);
    let edits = f["result"].as_array().expect("danh sách sửa");
    assert_eq!(edits.len(), 1);
    assert!(
        edits[0]["newText"]
            .as_str()
            .expect("văn bản")
            .contains("ƒ main() → u32 ! ∅ { 1u32 }"),
        "{f}"
    );

    // cú pháp sai ⇒ không định dạng (null), không làm hỏng
    s.send(&json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":4},"contentChanges":[{"text":"ƒ main( {\n"}]}}));
    let p = s.recv();
    assert_eq!(p["params"]["diagnostics"][0]["code"], "E0101", "{p}");
    s.send(&json!({"jsonrpc":"2.0","id":3,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri},"options":{}}}));
    assert!(s.recv()["result"].is_null());

    // phương thức lạ có id ⇒ lỗi -32601; đóng tài liệu ⇒ xoá chẩn đoán
    s.send(&json!({"jsonrpc":"2.0","id":4,"method":"textDocument/completion","params":{}}));
    assert_eq!(s.recv()["error"]["code"], -32601);
    s.send(&json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":uri}}}));
    assert!(
        s.recv()["params"]["diagnostics"]
            .as_array()
            .expect("mảng")
            .is_empty()
    );

    // tắt sạch: shutdown rồi exit ⇒ mã 0
    s.send(&json!({"jsonrpc":"2.0","id":5,"method":"shutdown"}));
    assert!(s.recv()["result"].is_null());
    s.send(&json!({"jsonrpc":"2.0","method":"exit"}));
    let st = s.child.wait().expect("chờ");
    assert_eq!(st.code(), Some(0));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn exit_without_shutdown_is_nonzero() {
    let mut s = Server::start();
    s.send(&json!({"jsonrpc":"2.0","method":"exit"}));
    assert_eq!(s.child.wait().expect("chờ").code(), Some(1));
}

/// M13 (P3.b): hover trên chẩn đoán = thông điệp + `gritc explain`; định nghĩa cấp item; ký hiệu tài liệu; và LSP KHÔNG đổi phán quyết:
/// tập MÃ chẩn đoán LSP == tập mã của `gritc check --json` trên cùng tệp (LSP chỉ hiển thị).
#[test]
fn lsp_hover_definition_symbols_and_it_never_changes_the_verdict() {
    let dir = std::env::temp_dir().join(format!("gt-lsp2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tạo thư mục");
    // `helper` định nghĩa ở dòng 0; `main` gọi `helper` ở dòng 1 và có lỗi kiểu (trả bool cho u32)
    let src = "ƒ helper() → u32 ! ∅ { 1u32 }
ƒ main() → u32 ! ∅ { x ≔ helper(); true }
";
    let path = dir.join("main.gt");
    std::fs::write(&path, src).expect("ghi");
    let uri = format!("file://{}/main.gt", dir.display());
    let mut s = Server::start();
    s.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}));
    let init = s.recv();
    for k in [
        "hoverProvider",
        "definitionProvider",
        "documentSymbolProvider",
    ] {
        assert_eq!(init["result"]["capabilities"][k], true, "{k}");
    }
    let p = s.open(&uri, src);
    let lsp_codes: std::collections::BTreeSet<String> = p["params"]["diagnostics"]
        .as_array()
        .expect("mảng")
        .iter()
        .map(|d| d["code"].as_str().expect("mã").to_string())
        .collect();
    // phán quyết gốc từ CLI
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args(["check", "--json"])
        .arg(&path)
        .output()
        .expect("chạy gritc");
    let v: Value = serde_json::from_slice(&o.stdout).expect("JSON của gritc check");
    let cli_codes: std::collections::BTreeSet<String> = v["diagnostics"]
        .as_array()
        .expect("mảng")
        .iter()
        .map(|d| d["code"].as_str().expect("mã").to_string())
        .collect();
    assert_eq!(lsp_codes, cli_codes, "LSP và gritc check phải cùng tập mã");
    assert!(lsp_codes.contains("E0301"), "{lsp_codes:?}");

    // hover tại đầu chẩn đoán E0301 ⇒ markdown có mã và nội dung explain
    let e = p["params"]["diagnostics"]
        .as_array()
        .expect("mảng")
        .iter()
        .find(|d| d["code"] == "E0301")
        .expect("E0301")
        .clone();
    let (hl, hc) = (
        e["range"]["start"]["line"].clone(),
        e["range"]["start"]["character"].clone(),
    );
    s.send(&json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":hl,"character":hc}}}));
    let h = s.recv();
    let md = h["result"]["contents"]["value"].as_str().expect("markdown");
    assert!(md.contains("E0301"), "{h}");
    assert!(md.len() > 80, "thiếu nội dung explain: {h}");
    // hover ở vị trí không có chẩn đoán ⇒ null
    s.send(&json!({"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":0}}}));
    assert!(s.recv()["result"].is_null());

    // định nghĩa: `helper` ở dòng 1 (gọi) ⇒ dòng 0
    let call_col = "ƒ main() → u32 ! ∅ { x ≔ ".encode_utf16().count() + 2;
    s.send(&json!({"jsonrpc":"2.0","id":4,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":{"line":1,"character":call_col}}}));
    let d = s.recv();
    let locs = d["result"].as_array().unwrap_or_else(|| panic!("{d}"));
    assert_eq!(locs.len(), 1, "{d}");
    assert_eq!(locs[0]["uri"], uri.as_str());
    assert_eq!(locs[0]["range"]["start"]["line"], 0);
    // định danh không phải item ⇒ null
    s.send(&json!({"jsonrpc":"2.0","id":5,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":{"line":1,"character":0}}}));
    assert!(s.recv()["result"].is_null());

    // ký hiệu tài liệu: helper, main
    s.send(&json!({"jsonrpc":"2.0","id":6,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri}}}));
    let sy = s.recv();
    let names: Vec<&str> = sy["result"]
        .as_array()
        .expect("mảng")
        .iter()
        .map(|x| x["name"].as_str().expect("tên"))
        .collect();
    assert!(names.contains(&"helper") && names.contains(&"main"), "{sy}");

    s.send(&json!({"jsonrpc":"2.0","id":7,"method":"shutdown"}));
    let _ = s.recv();
    s.send(&json!({"jsonrpc":"2.0","method":"exit"}));
    let _ = s.child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}

/// P3.b: bản kê khai `editors/vscode` hợp lệ BẰNG MÁY (JSON đúng, mọi tệp được tham chiếu tồn tại, sự kiện kích hoạt khớp ngôn ngữ). CHƯA THỬ trong VS Code thật.
#[test]
fn vscode_manifest_is_valid_and_references_existing_files() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../editors/vscode");
    let pkg: Value =
        serde_json::from_slice(&std::fs::read(root.join("package.json")).expect("đọc"))
            .expect("JSON hợp lệ");
    assert_eq!(pkg["contributes"]["languages"][0]["id"], "grit");
    assert_eq!(pkg["activationEvents"][0], "onLanguage:grit");
    for rel in [
        pkg["main"].as_str().expect("main"),
        pkg["contributes"]["languages"][0]["configuration"]
            .as_str()
            .expect("configuration"),
        pkg["contributes"]["grammars"][0]["path"]
            .as_str()
            .expect("grammar"),
    ] {
        assert!(root.join(rel).is_file(), "thiếu {rel}");
    }
    for rel in [
        "language-configuration.json",
        "syntaxes/grit.tmLanguage.json",
    ] {
        serde_json::from_slice::<Value>(&std::fs::read(root.join(rel)).expect("đọc"))
            .unwrap_or_else(|e| panic!("{rel}: {e}"));
    }
}

/// FB-G2-05/06 (Gemini R2): tên tệp trùng biến của `std` (`a.gt`) không được làm IDE báo "0 lỗi"; tên không phải định danh (`my-app.gt`) không được E9901;
/// `rig.toml` có `edition = 2` ⇒ cú pháp edition 2 (`?`) không bị E0101 giả.
#[test]
fn lsp_file_names_and_edition_do_not_hide_diagnostics() {
    let base = std::env::temp_dir().join(format!("gt_lsp_names_{}", std::process::id()));
    let mut s = Server::start();
    s.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}));
    let _ = s.recv();
    let bad = "ƒ main() → u32 ! ∅ { x: u32 ≔ true; 0u32 }\n";
    let diag_codes = |v: &Value| -> Vec<String> {
        v["params"]["diagnostics"]
            .as_array()
            .expect("mảng")
            .iter()
            .map(|d| d["code"].as_str().unwrap_or("").to_string())
            .collect()
    };
    for name in ["a.gt", "my-app.gt", "123.gt"] {
        let dir = base.join(name.replace('.', "_"));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let uri = format!("file://{}", dir.join(name).display());
        let codes = diag_codes(&s.open(&uri, bad));
        assert!(!codes.is_empty(), "{name}: IDE báo 0 lỗi trên mã sai");
        assert!(
            !codes.iter().any(|c| c == "E9901") || name == "a.gt",
            "{name}: E9901 (BadModuleName?): {codes:?}"
        );
        if name != "a.gt" {
            assert!(
                codes.iter().any(|c| c == "E0301"),
                "{name}: thiếu E0301: {codes:?}"
            );
        }
    }
    let dir = base.join("ed2");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("rig.toml"),
        "[package]\nname = \"ed2\"\nversion = \"0.1.0\"\nedition = 2\n",
    )
    .expect("rig.toml");
    let uri = format!("file://{}", dir.join("p.gt").display());
    let codes = diag_codes(&s.open(
        &uri,
        "ƒ f(x: u32) → u32 ! ∅ { x }\nƒ main() → u32 ! ∅ { 0u32 }\n",
    ));
    assert!(
        codes.iter().all(|c| c != "E0101"),
        "edition 2 bị E0101: {codes:?}"
    );
    let _ = std::fs::remove_dir_all(&base);
}
