//! Dogfood công cụ 2 (docs/DOGFOOD.md): `examples/json` — bộ phân tích JSON viết BẰNG GRIT, dựng bằng `rig build`.
//! Hai chuẩn đối chiếu ĐỘC LẬP: (1) bản tham chiếu Rust phản chiếu đúng thuật toán (kể cả vị trí lỗi và ngân sách độ sâu);
//! (2) `serde_json` làm oracle về TÍNH HỢP LỆ (khi lồng nông). Bất đồng với serde = lỗi của bản tham chiếu hoặc của tool ⇒ điều tra.
use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("rig-json-{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("tạo");
    for e in std::fs::read_dir(from).expect("đọc").flatten() {
        let p = e.path();
        if e.file_name() == "target" {
            continue;
        }
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).expect("chép");
        }
    }
}

fn build_json() -> PathBuf {
    let d = tmp("build");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/json"),
        &d,
    );
    let o = Command::new(env!("CARGO_BIN_EXE_rig"))
        .current_dir(&d)
        .args(["build", "--locked"])
        .output()
        .expect("chạy rig");
    assert_eq!(
        o.status.code(),
        Some(0),
        "rig build: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    d.join("target/json.exe")
}

// ---------------------------------------------------------------- tham chiếu Rust (phản chiếu examples/json/src/main.gt)

const MAX_BUDGET: u32 = 201;

fn is_ws(c: u8) -> bool {
    matches!(c, 32 | 10 | 9 | 13)
}
fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}
fn is_hex(c: u8) -> bool {
    c.is_ascii_hexdigit()
}
fn hexval(c: u8) -> u32 {
    (c as char).to_digit(16).unwrap_or(0)
}
fn is_cont(c: u8) -> bool {
    (128..=191).contains(&c)
}
fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && is_ws(b[i]) {
        i += 1;
    }
    i
}
fn match4(b: &[u8], i: usize, w: &[u8; 4]) -> bool {
    i + 4 <= b.len() && &b[i..i + 4] == w
}
fn utf8_len(b: &[u8], i: usize) -> usize {
    let c = b[i];
    let ok = |n: usize| i + n <= b.len();
    if (194..=223).contains(&c) {
        if ok(2) && is_cont(b[i + 1]) { 2 } else { 0 }
    } else if (224..=239).contains(&c) {
        if ok(3)
            && is_cont(b[i + 2])
            && ((c == 224 && (160..=191).contains(&b[i + 1]))
                || (c == 237 && (128..=159).contains(&b[i + 1]))
                || (c != 224 && c != 237 && is_cont(b[i + 1])))
        {
            3
        } else {
            0
        }
    } else if (240..=244).contains(&c) {
        if ok(4)
            && is_cont(b[i + 2])
            && is_cont(b[i + 3])
            && ((c == 240 && (144..=191).contains(&b[i + 1]))
                || (c == 244 && (128..=143).contains(&b[i + 1]))
                || ((241..=243).contains(&c) && is_cont(b[i + 1])))
        {
            4
        } else {
            0
        }
    } else {
        0
    }
}
fn number(b: &[u8], start: usize) -> (usize, u32) {
    let mut i = start;
    let mut err = 0;
    if i < b.len() && b[i] == b'-' {
        i += 1;
    }
    if i >= b.len() {
        err = 3;
    } else if b[i] == b'0' {
        i += 1;
    } else if (b'1'..=b'9').contains(&b[i]) {
        i += 1;
        while i < b.len() && is_digit(b[i]) {
            i += 1;
        }
    } else {
        err = 3;
    }
    if err == 0 && i < b.len() && b[i] == b'.' {
        i += 1;
        if i < b.len() && is_digit(b[i]) {
            while i < b.len() && is_digit(b[i]) {
                i += 1;
            }
        } else {
            err = 3;
        }
    }
    if err == 0 && i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        if i < b.len() && is_digit(b[i]) {
            while i < b.len() && is_digit(b[i]) {
                i += 1;
            }
        } else {
            err = 3;
        }
    }
    (i, err)
}
fn string(b: &[u8], start: usize) -> (usize, u32) {
    let (mut i, mut err, mut epos, mut done) = (start + 1, 0u32, 0usize, false);
    while i < b.len() && !done && err == 0 {
        let c = b[i];
        if c == b'"' {
            done = true;
            i += 1;
        } else if c < 32 {
            err = 4;
            epos = i;
            i += 1;
        } else if c == b'\\' {
            if i + 1 >= b.len() {
                err = 2;
                epos = b.len();
                i += 1;
            } else {
                let e = b[i + 1];
                if matches!(e, b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') {
                    i += 2;
                } else if e == b'u' {
                    if i + 6 <= b.len() && (2..6).all(|k| is_hex(b[i + k])) {
                        let cu = (2..6).fold(0u32, |a, k| a * 16 + hexval(b[i + k]));
                        if (0xD800..=0xDBFF).contains(&cu) {
                            if i + 12 <= b.len()
                                && b[i + 6] == b'\\'
                                && b[i + 7] == b'u'
                                && (8..12).all(|k| is_hex(b[i + k]))
                            {
                                let lo = (8..12).fold(0u32, |a, k| a * 16 + hexval(b[i + k]));
                                if (0xDC00..=0xDFFF).contains(&lo) {
                                    i += 12;
                                } else {
                                    err = 5;
                                    epos = i;
                                    i += 1;
                                }
                            } else {
                                err = 5;
                                epos = i;
                                i += 1;
                            }
                        } else if (0xDC00..=0xDFFF).contains(&cu) {
                            err = 5;
                            epos = i;
                            i += 1;
                        } else {
                            i += 6;
                        }
                    } else {
                        err = 5;
                        epos = i;
                        i += 1;
                    }
                } else {
                    err = 5;
                    epos = i;
                    i += 1;
                }
            }
        } else if c < 128 {
            i += 1;
        } else {
            let n = utf8_len(b, i);
            if n >= 1 {
                i += n;
            } else {
                err = 6;
                epos = i;
                i += 1;
            }
        }
    }
    if err == 0 && !done {
        err = 2;
        epos = b.len();
    }
    if err != 0 { (epos, err) } else { (i, 0) }
}

#[derive(Debug, PartialEq)]
enum R {
    Good(usize, u64, u32),
    Bad(usize, u32),
}

fn value(b: &[u8], start: usize, budget: u32) -> R {
    if budget == 0 {
        return R::Bad(start, 8);
    }
    let i = skip_ws(b, start);
    if i >= b.len() {
        return R::Bad(i, 2);
    }
    let c = b[i];
    if c == b'{' || c == b'[' {
        if budget < 2 {
            return R::Bad(i, 8);
        }
        return if c == b'{' {
            obj(b, i, budget - 1)
        } else {
            arr(b, i, budget - 1)
        };
    }
    if c == b'"' {
        let s = string(b, i);
        return if s.1 != 0 {
            R::Bad(s.0, s.1)
        } else {
            R::Good(s.0, 1, 1)
        };
    }
    if c == b'-' || is_digit(c) {
        let n = number(b, i);
        return if n.1 != 0 {
            R::Bad(n.0, n.1)
        } else {
            R::Good(n.0, 1, 1)
        };
    }
    if match4(b, i, b"true") {
        return R::Good(i + 4, 1, 1);
    }
    if i + 5 <= b.len() && b[i] == b'f' && match4(b, i + 1, b"alse") {
        return R::Good(i + 5, 1, 1);
    }
    if match4(b, i, b"null") {
        return R::Good(i + 4, 1, 1);
    }
    R::Bad(i, 1)
}

fn arr(b: &[u8], start: usize, budget: u32) -> R {
    let i0 = skip_ws(b, start + 1);
    if i0 < b.len() && b[i0] == b']' {
        return R::Good(i0 + 1, 1, 1);
    }
    let (mut pos, mut count, mut maxd) = (i0, 1u64, 0u32);
    loop {
        match value(b, pos, budget - 1) {
            R::Bad(p, e) => return R::Bad(p, e),
            R::Good(p, cnt, d) => {
                count += cnt;
                maxd = maxd.max(d);
                let q = skip_ws(b, p);
                if q >= b.len() {
                    return R::Bad(q, 2);
                }
                if b[q] == b',' {
                    pos = q + 1;
                } else if b[q] == b']' {
                    return R::Good(q + 1, count, maxd + 1);
                } else {
                    return R::Bad(q, 1);
                }
            }
        }
    }
}

fn obj(b: &[u8], start: usize, budget: u32) -> R {
    let i0 = skip_ws(b, start + 1);
    if i0 < b.len() && b[i0] == b'}' {
        return R::Good(i0 + 1, 1, 1);
    }
    let (mut pos, mut count, mut maxd) = (i0, 1u64, 0u32);
    loop {
        let k = skip_ws(b, pos);
        if k >= b.len() {
            return R::Bad(k, 2);
        }
        if b[k] != b'"' {
            return R::Bad(k, 1);
        }
        let s = string(b, k);
        if s.1 != 0 {
            return R::Bad(s.0, s.1);
        }
        let c = skip_ws(b, s.0);
        if c >= b.len() {
            return R::Bad(c, 2);
        }
        if b[c] != b':' {
            return R::Bad(c, 1);
        }
        match value(b, c + 1, budget - 1) {
            R::Bad(p, e) => return R::Bad(p, e),
            R::Good(p, cnt, d) => {
                count += cnt;
                maxd = maxd.max(d);
                let q = skip_ws(b, p);
                if q >= b.len() {
                    return R::Bad(q, 2);
                }
                if b[q] == b',' {
                    pos = q + 1;
                } else if b[q] == b'}' {
                    return R::Good(q + 1, count, maxd + 1);
                } else {
                    return R::Bad(q, 1);
                }
            }
        }
    }
}

fn reason(code: u32) -> &'static str {
    match code {
        1 => "unexpected character",
        2 => "unexpected end of input",
        3 => "bad number",
        4 => "control character in string",
        5 => "bad escape",
        6 => "invalid UTF-8",
        7 => "trailing characters",
        _ => "nesting too deep",
    }
}

fn reference(b: &[u8]) -> R {
    match value(b, 0, MAX_BUDGET) {
        R::Good(p, c, d) => {
            let q = skip_ws(b, p);
            if q == b.len() {
                R::Good(p, c, d)
            } else {
                R::Bad(q, 7)
            }
        }
        bad => bad,
    }
}

fn expected_line(path: &Path, r: &R) -> String {
    match r {
        R::Good(_, c, d) => format!("{}: valid elements={c} depth={d}\n", path.display()),
        R::Bad(p, code) => format!(
            "{}: invalid at byte {p} ({})\n",
            path.display(),
            reason(*code)
        ),
    }
}

fn run_tool(exe: &Path, path: &Path) -> String {
    let o = Command::new(exe).arg(path).output().expect("chạy json");
    assert_eq!(
        o.status.code(),
        Some(0),
        "tool thoát bất thường ({:?}) trên {}: {}",
        o.status.code(),
        path.display(),
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// So tool với tham chiếu (chính xác cả vị trí lỗi) và, nếu đủ nông, với serde_json về tính hợp lệ.
fn check_doc(exe: &Path, dir: &Path, name: &str, doc: &[u8]) {
    let p = dir.join(format!("{name}.json"));
    std::fs::write(&p, doc).expect("ghi");
    let want = reference(doc);
    let got = run_tool(exe, &p);
    assert_eq!(
        got,
        expected_line(&p, &want),
        "tool ≠ tham chiếu trên `{name}` ({} byte)",
        doc.len()
    );
    // oracle độc lập: serde_json về tính hợp lệ (bỏ qua khi tool báo lồng quá sâu: serde có giới hạn riêng 128)
    if !matches!(want, R::Bad(_, 8)) && doc.len() < 4_000_000 {
        // serde_json đổi số sang f64 nên từ chối số tràn f64 (`1e999`: "number out of range") dù đúng cú pháp RFC 8259; tool chỉ kiểm cú pháp
        // ⇒ coi "number out of range" là ĐỒNG Ý về cú pháp (khác biệt có chủ đích, ghi ở DOGFOOD)
        let serde_ok = match serde_json::from_slice::<serde_json::Value>(doc) {
            Ok(_) => true,
            Err(e) => e.to_string().contains("number out of range") && matches!(want, R::Good(..)),
        };
        let ours_ok = matches!(want, R::Good(..));
        assert_eq!(
            ours_ok,
            serde_ok,
            "tính hợp lệ khác serde_json trên `{name}`: tham chiếu={want:?}, serde_ok={serde_ok}, doc={:?}",
            String::from_utf8_lossy(&doc[..doc.len().min(200)])
        );
    }
}

const VALID: &[&str] = &[
    "null",
    "true",
    "false",
    "0",
    "-0",
    "1",
    "-1",
    "123456789012345678901234567890",
    "1.5",
    "-0.0",
    "1e5",
    "1E+5",
    "1e-5",
    "0.5e10",
    "\"\"",
    "\"a\"",
    "\"\\\"\"",
    "\"\\\\\"",
    "\"\\/\"",
    "\"\\b\\f\\n\\r\\t\"",
    "\"\\u0000\"",
    "\"\\u00e9\"",
    "\"\\uD83D\\uDE00\"",
    "\"é\"",
    "\"日本語\"",
    "\"😀\"",
    "\"\u{10FFFF}\"",
    "\"\u{7f}\"",
    "[]",
    "{}",
    "[[]]",
    "[{}]",
    "{\"a\":{}}",
    "[1,2,3]",
    "[ 1 , 2 , 3 ]",
    "{\"a\":1}",
    "{\"a\":1,\"b\":[true,false,null]}",
    "{\"a\":{\"b\":{\"c\":[1,[2,[3]]]}}}",
    "  \n\t\r [ ] \n",
    "[\"a\",\"b\",\"c\"]",
    "{\"k\":\"v\",\"k\":\"w\"}",
    "{\"\":0}",
    "[0,-0,0.0,-0.0,1e0,1E0,1e+0,1e-0]",
    "[1.7976931348623157e308,5e-324,1e999]",
];

const INVALID: &[&str] = &[
    "",
    " ",
    "\n",
    "nul",
    "nulll",
    "tru",
    "truee",
    "True",
    "NULL",
    "falsy",
    "01",
    "-",
    "-a",
    "+1",
    ".5",
    "1.",
    "1.e5",
    "1e",
    "1e+",
    "--1",
    "0x10",
    "1 2",
    "[1 2]",
    "[1,]",
    "[,1]",
    "[,]",
    "[1,,2]",
    "[",
    "]",
    "{",
    "}",
    "{\"a\"}",
    "{\"a\":}",
    "{\"a\":1,}",
    "{,}",
    "{1:2}",
    "{'a':1}",
    "['a']",
    "\"abc",
    "\"\\",
    "\"\\x\"",
    "\"\\u12\"",
    "\"\\u12G4\"",
    "\"\\uD800\"",
    "\"\\uDC00\"",
    "\"\\uD800\\u0041\"",
    "\"\\uD800x\"",
    "\"\n\"",
    "\"\t\"",
    "[1] x",
    "[1]]",
    "{} {}",
    "// c\n1",
    "/* c */ 1",
    "NaN",
    "Infinity",
    "-Infinity",
    "[NaN]",
    "undefined",
    "\u{feff}1",
    "[1,2,3",
    "{\"a\":[1,2,{\"b\":}]}",
    "[\"a\" \"b\"]",
    "{\"a\" 1}",
    "{\"a\":1 \"b\":2}",
    "[1,2]]",
    "[[1,2]",
    "\"\\u00\"",
];

#[test]
fn json_tool_matches_reference_and_serde_on_hand_written_documents() {
    let exe = build_json();
    let d = tmp("hand");
    for (i, s) in VALID.iter().enumerate() {
        check_doc(&exe, &d, &format!("v{i}"), s.as_bytes());
        assert!(
            matches!(reference(s.as_bytes()), R::Good(..)),
            "VALID[{i}] {s:?} phải hợp lệ"
        );
    }
    for (i, s) in INVALID.iter().enumerate() {
        check_doc(&exe, &d, &format!("i{i}"), s.as_bytes());
        assert!(
            matches!(reference(s.as_bytes()), R::Bad(..)),
            "INVALID[{i}] {s:?} phải không hợp lệ"
        );
    }
    // UTF-8 sai trong chuỗi (tệp nhị phân)
    let bad_utf8: &[&[u8]] = &[
        b"\"\xff\"",
        b"\"\xc0\x80\"",
        b"\"\xc1\xbf\"",
        b"\"\xe0\x80\x80\"",
        b"\"\xed\xa0\x80\"",
        b"\"\xf4\x90\x80\x80\"",
        b"\"\xf5\x80\x80\x80\"",
        b"\"\xc2\"",
        b"\"\xe2\x82\"",
        b"\"\xf0\x9f\x98\"",
        b"\"\x80\"",
        b"[\"ok\", \"\xfe\"]",
    ];
    for (i, s) in bad_utf8.iter().enumerate() {
        check_doc(&exe, &d, &format!("u{i}"), s);
        assert!(
            matches!(reference(s), R::Bad(_, 6) | R::Bad(_, 2)),
            "bad_utf8[{i}]"
        );
    }
}

/// Đo cụ thể số liệu của vài tài liệu để chống "cùng sai": đối chiếu với con số tính tay.
#[test]
fn json_tool_counts_and_depth_hand_checked() {
    let exe = build_json();
    let d = tmp("counts");
    for (doc, elements, depth) in [
        ("null", 1, 1),
        ("[]", 1, 1),
        ("[1,2,3]", 4, 2),
        ("{\"a\":1,\"b\":[true,false,null]}", 1 + 1 + 1 + 3, 3), // gốc + a + mảng(1) + 3 phần tử
        ("[[[]]]", 3, 3),
        ("[[1],[2,[3]]]", 1 + 1 + 1 + 1 + 1 + 1 + 1, 4), // gốc, [1],1, [2,[3]],2,[3],3
        ("{\"a\":{\"b\":{\"c\":1}}}", 4, 4),
    ] {
        let p = d.join("x.json");
        std::fs::write(&p, doc).expect("ghi");
        let got = run_tool(&exe, &p);
        assert_eq!(
            got,
            format!("{}: valid elements={elements} depth={depth}\n", p.display()),
            "{doc}"
        );
    }
}

/// Lồng sâu: quanh ngân sách, rồi RẤT sâu (1 triệu tầng): phải là lỗi có kiểm soát ("nesting too deep"), thoát 0, KHÔNG tràn stack.
#[test]
fn deep_nesting_is_a_controlled_error_not_a_stack_overflow() {
    let exe = build_json();
    let d = tmp("deep");
    for &levels in &[
        1usize, 50, 98, 99, 100, 101, 102, 150, 1000, 100_000, 1_000_000,
    ] {
        let mut a = "[".repeat(levels);
        a.push_str(&"]".repeat(levels));
        check_doc(&exe, &d, &format!("arr{levels}"), a.as_bytes());
        let mut o = String::new();
        for _ in 0..levels.min(1_000_000) {
            o.push_str("{\"a\":");
        }
        o.push('1');
        o.push_str(&"}".repeat(levels));
        check_doc(&exe, &d, &format!("obj{levels}"), o.as_bytes());
    }
    // biên chính xác (ghi vào DOGFOOD): tầng sâu nhất còn hợp lệ với ngân sách 201 và 2 đơn vị/tầng
    let ok = |n: usize| {
        matches!(
            reference(("[".repeat(n) + &"]".repeat(n)).as_bytes()),
            R::Good(..)
        )
    };
    assert!(ok(100) && !ok(101), "biên hợp lệ khác dự kiến (100/101)");
    // một tài liệu rất sâu nhưng thân không hợp lệ ở ngoài: tool phải báo `too deep` TRƯỚC khi đọc hết (không đệ quy sâu)
    let p = d.join("deep_garbage.json");
    std::fs::write(&p, "[".repeat(5_000_000)).expect("ghi");
    let got = run_tool(&exe, &p);
    assert!(got.contains("nesting too deep"), "{got}");
}

#[test]
fn json_tool_on_large_files_reports_throughput() {
    let exe = build_json();
    let d = tmp("big");
    // mảng 1.5 triệu số/chuỗi/boolean (~25 MB)
    let mut s = String::from("[");
    for i in 0..1_500_000u32 {
        if i > 0 {
            s.push(',');
        }
        match i % 5 {
            0 => s.push_str(&i.to_string()),
            1 => s.push_str(&format!("-{i}.{}e{}", i % 97, i % 7)),
            2 => s.push_str("\"héllo wörld\""),
            3 => s.push_str("true"),
            _ => s.push_str("null"),
        }
    }
    s.push(']');
    let p = d.join("big_array.json");
    std::fs::write(&p, &s).expect("ghi");
    let t = std::time::Instant::now();
    let got = run_tool(&exe, &p);
    let dt = t.elapsed();
    assert_eq!(got, expected_line(&p, &reference(s.as_bytes())));
    println!(
        "json.gt (LLVM -O0, mọi check giữ, cây trong arena) trên {:.1} MiB: {:.2} s ⇒ {:.0} MiB/s",
        s.len() as f64 / 1048576.0,
        dt.as_secs_f64(),
        s.len() as f64 / 1048576.0 / dt.as_secs_f64()
    );
    // đối tượng 200 nghìn khoá
    let mut o = String::from("{");
    for i in 0..200_000u32 {
        if i > 0 {
            o.push(',');
        }
        o.push_str(&format!("\"key{i}\":{{\"v\":[{i},{}]}}", i * 2));
    }
    o.push('}');
    let p2 = d.join("big_obj.json");
    std::fs::write(&p2, &o).expect("ghi");
    assert_eq!(
        run_tool(&exe, &p2),
        expected_line(&p2, &reference(o.as_bytes()))
    );
}

fn lcg(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed >> 33
}

/// Fuzz đột biến: lấy tài liệu hợp lệ, xoá/chèn/lật/cắt byte ngẫu nhiên; tool phải KHỚP tham chiếu từng ký tự đầu ra và khớp serde về hợp lệ.
#[test]
fn mutation_fuzz_against_reference_and_serde() {
    let exe = build_json();
    let d = tmp("fuzz");
    let mut seed = 20261013u64;
    let base: Vec<Vec<u8>> = VALID
        .iter()
        .map(|s| s.as_bytes().to_vec())
        .chain([
            br#"{"name":"Grit","tags":["a","b\u00e9","\ud83d\ude00"],"n":[1,2.5,-3e4,true,null],"o":{"x":{"y":[]}}}"#.to_vec(),
            "[\"日本語\",\"é\",{\"k\":[0,1,2,3,4,5,6,7,8,9]}]".as_bytes().to_vec(),
        ])
        .collect();
    let (mut valid_n, mut invalid_n) = (0, 0);
    for case in 0..700 {
        let mut doc = base[(lcg(&mut seed) as usize) % base.len()].clone();
        for _ in 0..(1 + lcg(&mut seed) % 3) {
            let len = doc.len();
            match lcg(&mut seed) % 4 {
                0 if len > 0 => {
                    let i = (lcg(&mut seed) as usize) % len;
                    doc[i] = lcg(&mut seed) as u8;
                }
                1 if len > 0 => {
                    let i = (lcg(&mut seed) as usize) % len;
                    doc.remove(i);
                }
                2 => {
                    let i = (lcg(&mut seed) as usize) % (len + 1);
                    let c =
                        b"[]{},:\"\\ -+.0123eE tfnul\xc3\xa9\xff"[(lcg(&mut seed) as usize) % 27];
                    doc.insert(i, c);
                }
                _ if len > 0 => doc.truncate((lcg(&mut seed) as usize) % len),
                _ => {}
            }
        }
        match reference(&doc) {
            R::Good(..) => valid_n += 1,
            _ => invalid_n += 1,
        }
        check_doc(&exe, &d, &format!("f{case}"), &doc);
    }
    println!(
        "fuzz: 700 tài liệu đột biến: {valid_n} hợp lệ, {invalid_n} không hợp lệ — tool ≡ tham chiếu ≡ serde_json (hợp lệ)"
    );
    assert!(
        valid_n > 20 && invalid_n > 100,
        "phân bố quá lệch: {valid_n}/{invalid_n}"
    );
}

#[test]
fn json_tool_error_paths_and_missing_args() {
    let exe = build_json();
    let d = tmp("err");
    let o = Command::new(&exe)
        .arg(d.join("khong_co.json"))
        .output()
        .expect("chạy");
    assert_eq!(o.status.code(), Some(2));
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(
        out.starts_with("json: ") && out.ends_with(": no such file\n"),
        "{out}"
    );
    let o = Command::new(&exe).output().expect("chạy");
    assert_eq!(o.status.code(), Some(70));
    assert!(String::from_utf8_lossy(&o.stderr).contains("abort: arg_out_of_range"));
}
