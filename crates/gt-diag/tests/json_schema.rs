//! JSON có version + schema (docs/spec/diag_schema.json). Viết TRƯỚC khi có cài đặt (TDD).
//! LSP (M13) và giao thức AI (8.E) dựa vào dạng này nên mọi trường được khoá bằng test.
use gt_diag::{Applicability, Code, Diagnostic, Diagnostics, Edit, SCHEMA_VERSION, Suggestion};
use gt_span::{SourceFile, Span};
use serde_json::Value;

fn file(text: &str) -> SourceFile {
    SourceFile::new("t.gt", text.to_string()).expect("nhỏ")
}

fn diag(code: &str, start: u32, end: u32, msg: &str) -> Diagnostic {
    Diagnostic::new(
        Code::parse(code).expect("mã"),
        "t.gt",
        Span::new(start, end),
        msg,
    )
}

fn schema() -> Value {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/spec/diag_schema.json"
    );
    serde_json::from_str(&std::fs::read_to_string(p).expect("diag_schema.json"))
        .expect("schema là JSON hợp lệ")
}

/// Bộ kiểm JSON Schema tối thiểu: type, required, properties, items, enum, const, minimum, $ref cục bộ.
fn validate(root: &Value, schema: &Value, v: &Value, path: &str, errs: &mut Vec<String>) {
    if let Some(r) = schema.get("$ref").and_then(Value::as_str) {
        let name = r.strip_prefix("#/$defs/").expect("chỉ hỗ trợ $ref cục bộ");
        return validate(root, &root["$defs"][name], v, path, errs);
    }
    if let Some(t) = schema.get("type") {
        let types: Vec<&str> = match t {
            Value::String(s) => vec![s.as_str()],
            Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
            _ => vec![],
        };
        let ok = types.iter().any(|t| match *t {
            "object" => v.is_object(),
            "array" => v.is_array(),
            "string" => v.is_string(),
            "integer" => v.is_i64() || v.is_u64(),
            "boolean" => v.is_boolean(),
            "null" => v.is_null(),
            _ => false,
        });
        if !ok {
            errs.push(format!("{path}: kiểu {v} không thuộc {types:?}"));
            return;
        }
    }
    if let Some(c) = schema.get("const")
        && c != v
    {
        errs.push(format!("{path}: phải bằng {c}, có {v}"));
    }
    if let Some(e) = schema.get("enum").and_then(Value::as_array)
        && !e.contains(v)
    {
        errs.push(format!("{path}: {v} không nằm trong enum {e:?}"));
    }
    if let (Some(min), Some(n)) = (schema.get("minimum").and_then(Value::as_i64), v.as_i64())
        && n < min
    {
        errs.push(format!("{path}: {n} < {min}"));
    }
    if let (Some(req), Some(obj)) = (
        schema.get("required").and_then(Value::as_array),
        v.as_object(),
    ) {
        for k in req.iter().filter_map(Value::as_str) {
            if !obj.contains_key(k) {
                errs.push(format!("{path}: thiếu trường bắt buộc {k}"));
            }
        }
    }
    if let (Some(props), Some(obj)) = (
        schema.get("properties").and_then(Value::as_object),
        v.as_object(),
    ) {
        for (k, sub) in props {
            if let Some(child) = obj.get(k) {
                validate(root, sub, child, &format!("{path}.{k}"), errs);
            }
        }
    }
    if let (Some(items), Some(arr)) = (schema.get("items"), v.as_array()) {
        for (i, child) in arr.iter().enumerate() {
            validate(root, items, child, &format!("{path}[{i}]"), errs);
        }
    }
}

fn check_schema(json: &str) -> Value {
    let v: Value =
        serde_json::from_str(json).unwrap_or_else(|e| panic!("JSON không hợp lệ: {e}\n{json}"));
    let s = schema();
    let mut errs = vec![];
    validate(&s, &s, &v, "$", &mut errs);
    assert!(errs.is_empty(), "vi phạm schema: {errs:#?}\n{json}");
    v
}

fn report_json(ds: Vec<Diagnostic>, f: &SourceFile) -> String {
    let mut bag = Diagnostics::new(100);
    for d in ds {
        bag.push(d);
    }
    bag.finish().to_json(&[f], false)
}

#[test]
fn schema_file_matches_code_version() {
    let s = schema();
    assert_eq!(s["properties"]["schema_version"]["const"], SCHEMA_VERSION);
    assert_eq!(SCHEMA_VERSION, 1);
    assert_eq!(s["$schema"], "https://json-schema.org/draft/2020-12/schema");
}

#[test]
fn envelope_and_field_order_are_stable() {
    let f = file("ƒ f( { 1 }\n");
    let json = report_json(vec![diag("E0101", 6, 7, "m").with_span_label("l")], &f);
    assert!(json.starts_with("{\"schema_version\":1,\"tool\":\"gritc\",\"diagnostics\":[{\"code\":\"E0101\",\"level\":\"error\","), "{json}");
    assert!(
        json.ends_with(
            "\"summary\":{\"errors\":1,\"warnings\":0,\"notes\":0,\"truncated\":0,\"exit_code\":1}}"
        ),
        "{json}"
    );
    assert_eq!(
        json,
        report_json(vec![diag("E0101", 6, 7, "m").with_span_label("l")], &f),
        "phải y hệt từng byte"
    );
    let v = check_schema(&json);
    assert_eq!(
        v["diagnostics"][0]["span"]["start"],
        serde_json::json!({"offset":6,"line":1,"col":6})
    );
    assert_eq!(
        v["diagnostics"][0]["span"]["end"],
        serde_json::json!({"offset":7,"line":1,"col":7})
    );
    assert_eq!(v["diagnostics"][0]["span_label"], "l");
    assert_eq!(v["diagnostics"][0]["file"], "t.gt");
}

#[test]
fn every_level_and_no_optional_parts() {
    let f = file("abc\n");
    let json = report_json(
        vec![
            diag("E0101", 0, 1, "e"),
            diag("S101", 1, 2, "w"),
            diag("N0601", 2, 3, "n"),
        ],
        &f,
    );
    let v = check_schema(&json);
    let levels: Vec<&str> = v["diagnostics"]
        .as_array()
        .expect("arr")
        .iter()
        .map(|d| d["level"].as_str().expect("s"))
        .collect();
    assert_eq!(levels, ["error", "warn", "note"]);
    for d in v["diagnostics"].as_array().expect("arr") {
        assert!(d["span_label"].is_null());
        assert_eq!(d["labels"], serde_json::json!([]));
        assert_eq!(d["notes"], serde_json::json!([]));
        assert_eq!(d["suggestions"], serde_json::json!([]));
    }
    assert_eq!(
        v["summary"],
        serde_json::json!({"errors":1,"warnings":1,"notes":1,"truncated":0,"exit_code":1})
    );
}

#[test]
fn empty_multiline_and_out_of_range_spans() {
    let f = file("a\nbb\nccc\n");
    let json = report_json(
        vec![
            diag("E0101", 3, 3, "rỗng"),
            diag("E0305", 0, 7, "nhiều dòng"),
            diag("E0201", 50, 60, "ngoài tệp"),
        ],
        &f,
    );
    let v = check_schema(&json);
    let ds = v["diagnostics"].as_array().expect("arr");
    // sắp theo vị trí: [0..7) trước, rồi [3..3), rồi 50..60
    assert_eq!(ds[0]["message"], "nhiều dòng");
    assert_eq!(
        ds[0]["span"]["end"],
        serde_json::json!({"offset":7,"line":3,"col":3})
    );
    assert_eq!(ds[1]["message"], "rỗng");
    assert_eq!(ds[1]["span"]["start"], ds[1]["span"]["end"]);
    assert_eq!(
        ds[2]["span"]["start"],
        serde_json::json!({"offset":50,"line":null,"col":null})
    );
}

#[test]
fn unknown_file_gives_null_positions() {
    let f = file("abc");
    let mut d = diag("E0101", 0, 1, "m");
    d.file = "khac.gt".to_string();
    let v = check_schema(&report_json(vec![d], &f));
    assert_eq!(
        v["diagnostics"][0]["span"]["start"],
        serde_json::json!({"offset":0,"line":null,"col":null})
    );
}

#[test]
fn labels_notes_and_suggestions() {
    let f = file("let a;\nlet b;\n");
    let d = diag("E0401", 11, 12, "dùng sau khi move")
        .with_label("t.gt", Span::new(4, 5), "đã move ở đây")
        .with_note("n1")
        .with_note("n2")
        .with_suggestion(Suggestion::new(
            "bọc clone",
            Applicability::MaybeIncorrect,
            vec![Edit::new(Span::new(11, 12), "b.clone()")],
        ))
        .with_suggestion(Suggestion::new(
            "xoá",
            Applicability::MachineApplicable,
            vec![
                Edit::new(Span::new(7, 14), ""),
                Edit::new(Span::new(0, 0), "// "),
            ],
        ));
    let v = check_schema(&report_json(vec![d], &f));
    let d0 = &v["diagnostics"][0];
    assert_eq!(d0["labels"][0]["message"], "đã move ở đây");
    assert_eq!(
        d0["labels"][0]["span"]["start"],
        serde_json::json!({"offset":4,"line":1,"col":5})
    );
    assert_eq!(d0["notes"], serde_json::json!(["n1", "n2"]));
    assert_eq!(d0["suggestions"][0]["applicability"], "maybe-incorrect");
    assert_eq!(d0["suggestions"][0]["edits"][0]["replacement"], "b.clone()");
    assert_eq!(d0["suggestions"][1]["applicability"], "machine-applicable");
    assert_eq!(
        d0["suggestions"][1]["edits"].as_array().expect("arr").len(),
        2
    );
}

#[test]
fn string_escaping_roundtrips() {
    let nasty = "dấu \" nháy, \\ gạch, \n xuống dòng, \t tab, \u{1} điều khiển, \u{7f}, ƒ≔μ, \u{2028}\u{2029}, 😀, \u{202e}bidi";
    let f = file("x");
    let d = diag("E0101", 0, 1, nasty)
        .with_span_label(nasty)
        .with_note(nasty);
    let v = check_schema(&report_json(vec![d], &f));
    assert_eq!(v["diagnostics"][0]["message"], nasty);
    assert_eq!(v["diagnostics"][0]["span_label"], nasty);
    assert_eq!(v["diagnostics"][0]["notes"][0], nasty);
}

#[test]
fn many_diagnostics_and_truncation_in_summary() {
    let f = file("0123456789");
    let mut bag = Diagnostics::new(3);
    for i in 0..10u32 {
        bag.push(diag("E0101", i, i + 1, &format!("lỗi {i}")));
    }
    let v = check_schema(&bag.finish().to_json(&[&f], false));
    assert_eq!(v["diagnostics"].as_array().expect("arr").len(), 3);
    assert_eq!(v["summary"]["truncated"], 7);
    assert_eq!(v["summary"]["errors"], 3);
}

#[test]
fn summary_exit_code_and_deny_yellow() {
    let f = file("abc");
    let mut bag = Diagnostics::new(10);
    bag.push(diag("S101", 0, 1, "w"));
    let r = bag.finish();
    assert_eq!(
        check_schema(&r.to_json(&[&f], false))["summary"]["exit_code"],
        0
    );
    assert_eq!(
        check_schema(&r.to_json(&[&f], true))["summary"]["exit_code"],
        1
    );
    // ICE
    let mut bag = Diagnostics::new(10);
    bag.push(diag("E9901", 0, 1, "ICE"));
    assert_eq!(
        check_schema(&bag.finish().to_json(&[&f], false))["summary"]["exit_code"],
        2
    );
}

/// Các trường JSON phát ra phải đúng bằng danh sách `properties` trong schema (không lệch code ↔ schema).
#[test]
fn emitted_keys_equal_schema_properties() {
    let f = file("let a;\nlet b;\n");
    let d = diag("E0401", 11, 12, "m")
        .with_span_label("l")
        .with_label("t.gt", Span::new(4, 5), "x")
        .with_note("n")
        .with_suggestion(Suggestion::new(
            "s",
            Applicability::MaybeIncorrect,
            vec![Edit::new(Span::new(11, 12), "r")],
        ));
    let v = check_schema(&report_json(vec![d], &f));
    let s = schema();
    let keys = |o: &Value| -> Vec<String> {
        let mut k: Vec<String> = o.as_object().expect("obj").keys().cloned().collect();
        k.sort();
        k
    };
    let prop_keys = |def: &Value| keys(&def["properties"]);
    assert_eq!(keys(&v), prop_keys(&s));
    assert_eq!(keys(&v["summary"]), prop_keys(&s["properties"]["summary"]));
    let dd = &v["diagnostics"][0];
    assert_eq!(keys(dd), prop_keys(&s["$defs"]["diagnostic"]));
    assert_eq!(keys(&dd["span"]), prop_keys(&s["$defs"]["span"]));
    assert_eq!(
        keys(&dd["span"]["start"]),
        prop_keys(&s["$defs"]["position"])
    );
    assert_eq!(keys(&dd["labels"][0]), prop_keys(&s["$defs"]["label"]));
    assert_eq!(
        keys(&dd["suggestions"][0]),
        prop_keys(&s["$defs"]["suggestion"])
    );
    assert_eq!(
        keys(&dd["suggestions"][0]["edits"][0]),
        prop_keys(&s["$defs"]["edit"])
    );
}

#[test]
fn json_is_safe_to_embed_in_scripts() {
    // U+2028/U+2029 hợp lệ trong JSON nhưng phá chuỗi JavaScript: phải được escape thành \u2028 / \u2029
    let f = file("x");
    let d = diag("E0101", 0, 1, "a\u{2028}b\u{2029}c");
    let json = report_json(vec![d], &f);
    assert!(
        !json.contains('\u{2028}') && !json.contains('\u{2029}'),
        "{json}"
    );
    assert!(
        json.contains("\\u2028") && json.contains("\\u2029"),
        "{json}"
    );
    assert_eq!(
        check_schema(&json)["diagnostics"][0]["message"],
        "a\u{2028}b\u{2029}c"
    );
}

/// Thứ tự khoá JSON cố định: serialize hai lần ra cùng từng byte, và khoá xuất hiện đúng thứ tự đã khoá trong schema.
#[test]
fn json_key_order_is_fixed_and_bytes_are_identical() {
    let f = file("let a;\nlet b;\n");
    let mk = || {
        diag("E0401", 11, 12, "m")
            .with_span_label("l")
            .with_label("t.gt", Span::new(4, 5), "x")
            .with_note("n")
            .with_suggestion(Suggestion::new(
                "s",
                Applicability::MaybeIncorrect,
                vec![Edit::new(Span::new(11, 12), "r")],
            ))
    };
    let mut bag = Diagnostics::new(10);
    bag.push(mk());
    let report = bag.finish();
    let first = report.to_json(&[&f], false);
    let second = report.to_json(&[&f], false);
    assert_eq!(
        first, second,
        "cùng một Report phải serialize ra cùng từng byte"
    );
    assert_eq!(
        first,
        report_json(vec![mk()], &f),
        "dựng lại từ đầu vẫn cùng từng byte"
    );
    // thứ tự khoá cấp cao nhất
    let order = |json: &str, keys: &[&str]| {
        let pos: Vec<usize> = keys
            .iter()
            .map(|k| {
                json.find(&format!("\"{k}\":"))
                    .unwrap_or_else(|| panic!("thiếu khoá {k}"))
            })
            .collect();
        assert!(
            pos.windows(2).all(|w| w[0] < w[1]),
            "sai thứ tự khoá {keys:?}: {pos:?}"
        );
    };
    order(
        &first,
        &["schema_version", "tool", "diagnostics", "summary"],
    );
    order(
        &first,
        &[
            "code",
            "level",
            "message",
            "file",
            "span",
            "span_label",
            "labels",
            "notes",
            "suggestions",
        ],
    );
    let summary = &first[first.find("\"summary\":").expect("summary")..];
    order(
        summary,
        &["errors", "warnings", "notes", "truncated", "exit_code"],
    );
    let sugg = &first[first.find("\"suggestions\":").expect("suggestions")..];
    order(sugg, &["message", "applicability", "edits"]);
    order(&first, &["start", "end"]);
    order(&first, &["offset", "line", "col"]);
}

// ---------- ADR-017a: nhãn và edit trỏ sang TỆP KHÁC (schema v1 đã có `file`; không tăng schema_version) ----------

fn two_files() -> (SourceFile, SourceFile) {
    (
        SourceFile::new("b.gt", "ƒ g() ! ∅ { a::secret(); }\n".to_string()).expect("nhỏ"),
        SourceFile::new("a.gt", "ƒ secret() ! ∅ { }\n".to_string()).expect("nhỏ"),
    )
}

fn cross_file_diag() -> Diagnostic {
    // `secret` trong b.gt: ký tự thứ 13.. (byte offset); định nghĩa trong a.gt: "secret" ở byte 3..9 (sau `ƒ ` 2 byte + dấu cách)
    let (b, a) = two_files();
    let use_at = b.text().find("secret").expect("secret") as u32;
    let def_at = a.text().find("secret").expect("secret") as u32;
    Diagnostic::new(
        Code::parse("E0203").expect("mã"),
        "b.gt",
        Span::new(use_at, use_at + 6),
        "`secret` là private",
    )
    .with_label("a.gt", Span::new(def_at, def_at + 6), "định nghĩa ở đây")
    .with_suggestion(Suggestion::new(
        "thêm `◈` trước định nghĩa",
        Applicability::MaybeIncorrect,
        vec![Edit {
            file: "a.gt".to_string(),
            span: Span::new(0, 0),
            replacement: "◈ ".to_string(),
        }],
    ))
}

#[test]
fn labels_and_edits_in_another_file_get_positions_from_their_own_file() {
    let (b, a) = two_files();
    let json = {
        let mut bag = Diagnostics::new(100);
        bag.push(cross_file_diag());
        bag.finish().to_json(&[&b, &a], false)
    };
    let v = check_schema(&json);
    let d = &v["diagnostics"][0];
    assert_eq!(d["file"], "b.gt");
    // vị trí chính theo b.gt: cột 16 (ký tự), dòng 1
    assert_eq!(d["span"]["start"]["line"], 1);
    assert_eq!(d["span"]["start"]["col"], 16);
    // nhãn ở a.gt: line/col tính theo a.gt (cột 3 — sau `ƒ `), KHÔNG theo b.gt
    let l = &d["labels"][0];
    assert_eq!(l["file"], "a.gt");
    assert_eq!(l["span"]["start"]["line"], 1);
    assert_eq!(l["span"]["start"]["col"], 3);
    // edit ở a.gt: chèn ở đầu tệp a.gt
    let e = &d["suggestions"][0]["edits"][0];
    assert_eq!(e["file"], "a.gt");
    assert_eq!(e["span"]["start"]["offset"], 0);
    assert_eq!(e["span"]["start"]["col"], 1);
    assert_eq!(v["schema_version"], 1);
}

#[test]
fn a_file_missing_from_the_source_list_gives_null_line_and_col_not_a_crash() {
    let (b, _a) = two_files();
    let mut bag = Diagnostics::new(100);
    bag.push(cross_file_diag());
    let json = bag.finish().to_json(&[&b], false);
    let v = check_schema(&json);
    let l = &v["diagnostics"][0]["labels"][0];
    assert_eq!(l["file"], "a.gt");
    assert!(l["span"]["start"]["line"].is_null());
    assert!(l["span"]["start"]["col"].is_null());
    assert!(l["span"]["start"]["offset"].is_number());
}
