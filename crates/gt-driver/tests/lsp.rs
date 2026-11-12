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
    s.send(&json!({"jsonrpc":"2.0","id":4,"method":"textDocument/hover","params":{}}));
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
