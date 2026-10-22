//! Seed 1.0 S6c: đầu ra THẬT của `gritc check/run --json` phải hợp lệ với `docs/spec/diag_schema.json` (schema_version 1, đóng băng: chỉ THÊM trường).
//! Bộ kiểm hợp lệ tự viết (không có thư viện JSON-Schema trong cây phụ thuộc): hỗ trợ đúng các từ khoá schema dùng — type (một hoặc mảng), required, properties,
//! items, enum, const, minimum, pattern (qua hàm kiểm tay), $ref nội bộ. Từ khoá LẠ trong schema ⇒ test ĐỎ (không âm thầm bỏ qua).
use serde_json::Value;
use std::process::Command;

fn schema() -> Value {
    let p =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/spec/diag_schema.json");
    serde_json::from_str(&std::fs::read_to_string(p).expect("đọc schema")).expect("schema JSON")
}

const KNOWN: &[&str] = &[
    "$schema",
    "$id",
    "$ref",
    "$defs",
    "title",
    "description",
    "type",
    "required",
    "properties",
    "items",
    "enum",
    "const",
    "minimum",
    "pattern",
];

fn type_ok(v: &Value, t: &str) -> bool {
    match t {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "integer" => v.is_i64() || v.is_u64(),
        "number" => v.is_number(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        _ => false,
    }
}

fn pattern_ok(pat: &str, s: &str) -> bool {
    // schema dùng mẫu mã chẩn đoán `^[ESN][0-9]{3,4}$`-dạng; kiểm bằng tay và báo nếu mẫu khác đi
    assert!(
        pat.starts_with('^') && pat.contains("[0-9]"),
        "mẫu lạ chưa hỗ trợ: {pat}"
    );
    let mut c = s.chars();
    matches!(c.next(), Some('E' | 'S' | 'N'))
        && s.len() >= 4
        && s[1..].chars().all(|x| x.is_ascii_digit())
}

fn validate(root: &Value, sc: &Value, v: &Value, path: &str, errs: &mut Vec<String>) {
    let Some(obj) = sc.as_object() else { return };
    for k in obj.keys() {
        assert!(
            KNOWN.contains(&k.as_str()),
            "từ khoá schema chưa hỗ trợ bởi bộ kiểm: `{k}` tại {path}"
        );
    }
    if let Some(r) = obj.get("$ref").and_then(Value::as_str) {
        let target = r
            .strip_prefix('#')
            .and_then(|p| root.pointer(p))
            .unwrap_or_else(|| panic!("$ref lạ {r}"));
        validate(root, target, v, path, errs);
        return;
    }
    if let Some(t) = obj.get("type") {
        let ok = match t {
            Value::String(s) => type_ok(v, s),
            Value::Array(a) => a.iter().any(|x| x.as_str().is_some_and(|s| type_ok(v, s))),
            _ => false,
        };
        if !ok {
            errs.push(format!("{path}: kiểu sai (mong {t}, có {v})"));
            return;
        }
    }
    if let Some(c) = obj.get("const")
        && c != v
    {
        errs.push(format!("{path}: mong hằng {c}, có {v}"));
    }
    if let Some(Value::Array(e)) = obj.get("enum")
        && !e.contains(v)
    {
        errs.push(format!("{path}: {v} ngoài enum {e:?}"));
    }
    if let (Some(m), Some(n)) = (obj.get("minimum").and_then(Value::as_i64), v.as_i64())
        && n < m
    {
        errs.push(format!("{path}: {n} < {m}"));
    }
    if let (Some(p), Some(s)) = (obj.get("pattern").and_then(Value::as_str), v.as_str())
        && !pattern_ok(p, s)
    {
        errs.push(format!("{path}: `{s}` không khớp mẫu {p}"));
    }
    if let (Some(Value::Array(req)), Some(o)) = (obj.get("required"), v.as_object()) {
        for r in req.iter().filter_map(Value::as_str) {
            if !o.contains_key(r) {
                errs.push(format!("{path}: thiếu trường bắt buộc `{r}`"));
            }
        }
    }
    if let (Some(Value::Object(props)), Some(o)) = (obj.get("properties"), v.as_object()) {
        for (k, sub) in props {
            if let Some(x) = o.get(k) {
                validate(root, sub, x, &format!("{path}/{k}"), errs);
            }
        }
    }
    if let (Some(items), Some(a)) = (obj.get("items"), v.as_array()) {
        for (i, x) in a.iter().enumerate() {
            validate(root, items, x, &format!("{path}/{i}"), errs);
        }
    }
}

fn gritc_json(args: &[&str], src: &str, tag: &str) -> Value {
    let dir = std::env::temp_dir().join(format!("gt-schema-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("tạo thư mục");
    let f = dir.join("prog.gt");
    std::fs::write(&f, src).expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_gritc"))
        .args(args)
        .arg("--json")
        .arg(&f)
        .output()
        .expect("chạy gritc");
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let line = text
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .unwrap_or_else(|| panic!("không có JSON: {text}"));
    let _ = std::fs::remove_dir_all(&dir);
    serde_json::from_str(line).expect("JSON hợp lệ")
}

#[test]
fn real_gritc_json_output_validates_against_the_frozen_schema() {
    let sc = schema();
    let cases: Vec<(&[&str], &str, &str)> = vec![
        (&["check"], "ƒ main() → u32 ! ∅ { 1u32 }\n", "green"),
        (&["check"], "ƒ main() → u32 ! ∅ { true }\n", "type"),
        (&["check"], "ƒ main( {\n", "syntax"),
        (
            &["check"],
            "ƒ f(x: u32) → u32 ! ∅ { x + 1u32 }\nƒ main() → u32 ! ∅ { f(4294967295u32) }\n",
            "overflow",
        ),
        (&["check"], "ƒ main() → u32 ! ∅ { x }\n", "name"),
        (&["run"], "ƒ main() → u32 ! ∅ { 7u32 }\n", "run"),
        (&["run"], "ƒ main() → u32 ! ∅ { 1u32 / 0u32 }\n", "runfail"),
    ];
    let mut all = Vec::new();
    for (args, src, tag) in cases {
        let v = gritc_json(args, src, tag);
        let mut errs = Vec::new();
        validate(&sc, &sc, &v, "", &mut errs);
        if !errs.is_empty() {
            all.push(format!("[{tag}] {errs:?}\n{v}"));
        }
    }
    assert!(all.is_empty(), "{}", all.join("\n"));
}

/// Độ nhạy: một báo cáo thiếu trường bắt buộc / sai kiểu phải bị bộ kiểm TỪ CHỐI (không phải bộ kiểm luôn gật).
#[test]
fn the_validator_rejects_malformed_reports() {
    let sc = schema();
    let mut good = gritc_json(&["check"], "ƒ main() → u32 ! ∅ { true }\n", "neg");
    let mut errs = Vec::new();
    validate(&sc, &sc, &good, "", &mut errs);
    assert!(errs.is_empty(), "{errs:?}");
    good["summary"]
        .as_object_mut()
        .expect("summary")
        .remove("errors");
    let mut errs = Vec::new();
    validate(&sc, &sc, &good, "", &mut errs);
    assert!(!errs.is_empty(), "thiếu summary.errors mà vẫn hợp lệ");
    let mut bad = gritc_json(&["check"], "ƒ main() → u32 ! ∅ { true }\n", "neg2");
    bad["schema_version"] = Value::from(2);
    let mut errs = Vec::new();
    validate(&sc, &sc, &bad, "", &mut errs);
    assert!(!errs.is_empty(), "schema_version 2 mà vẫn hợp lệ");
}
