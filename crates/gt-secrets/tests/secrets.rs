//! Bộ quét secret (M9): S101 mẫu, S102 entropy, S201 declassify, S202 chảy tới sink io, `#allow` có lý do.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;

fn run(src: &str) -> gt_secrets::SecretsOutput {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        common::mir_codes(&m)
    );
    gt_secrets::check(&t, &m, &[("main.gt".to_string(), src.to_string())])
}

fn codes(src: &str) -> Vec<String> {
    let mut v: Vec<String> = run(src)
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    v.sort();
    v
}

fn lit(name: &str, v: &str) -> String {
    format!("ƒ f() → &str ! ∅ {{ {name} ≔ \"{v}\"; {name} }}")
}

#[test]
fn s101_known_patterns_in_string_literals() {
    for v in [
        "AKIAIOSFODNN7EXAMPLE",
        "ASIAIOSFODNN7EXAMPLE",
        "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
        "github_pat_11ABCDEFG0abcdefghijkl_mnop",
        "xoxb-1234567890-abcdefghij",
        "AIzaSyA-abcdefghijklmnopqrstuvwxyz01234",
        "sk_live_abcdefghijklmnopqrstuvwx",
        "-----BEGIN RSA PRIVATE KEY----- MIIE -----END RSA PRIVATE KEY-----",
        "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijklmnop",
    ] {
        assert_eq!(codes(&lit("v", v)), vec!["S101"], "{v}");
    }
}

#[test]
fn s101_in_comment_and_non_matches() {
    assert_eq!(
        codes("// token AKIAIOSFODNN7EXAMPLE\nƒ f() → u32 ! ∅ { 1 }"),
        vec!["S101"]
    );
    for v in [
        "AKIA",
        "AKIAshort",
        "ghp_short",
        "hello world",
        "xoxz-1234567890-abcdefghij",
        "eyJabc.def.ghi",
    ] {
        assert!(
            codes(&lit("v", v)).is_empty(),
            "{v}: {:?}",
            codes(&lit("v", v))
        );
    }
}

#[test]
fn s102_high_entropy_literal_assigned_to_secret_like_name() {
    for name in [
        "api_key",
        "token",
        "secret",
        "passwd",
        "client_secret",
        "apiToken",
    ] {
        assert_eq!(
            codes(&lit(name, "x8F3kQ9zLp2Vw7Rm")),
            vec!["S102"],
            "{name}"
        );
    }
    // tên mật khẩu: ngưỡng thấp hơn khi có chữ số
    assert_eq!(codes(&lit("password", "Tr0ub4dor3x")), vec!["S102"]);
    // hằng
    assert_eq!(
        codes("□ API_KEY: &str ≔ \"x8F3kQ9zLp2Vw7Rm\";"),
        vec!["S102"]
    );
}

#[test]
fn s102_non_matches() {
    assert!(codes(&lit("name", "x8F3kQ9zLp2Vw7Rm")).is_empty()); // tên không gợi ý
    assert!(codes(&lit("api_key", "")).is_empty());
    assert!(codes(&lit("key", "aaaaaaaaaaaaaaaa")).is_empty()); // entropy thấp
    assert!(codes(&lit("token", "hello world")).is_empty());
    assert!(codes(&lit("password", "abc")).is_empty());
}

#[test]
fn s201_declassify_and_s202_flow_to_io_sinks() {
    assert_eq!(
        codes("ƒ f(s: Secret<u32>) → u32 ! ∅ { s.declassify() }"),
        vec!["S201"]
    );
    // declassify rồi in ra: S201 + S202
    assert_eq!(
        codes("ƒ log(x: u32) ! {io} { }\nƒ f(s: Secret<u32>) ! {io} { log(s.declassify()); }"),
        vec!["S201", "S202"]
    );
    // truyền thẳng Secret vào hàm io
    assert_eq!(
        codes("ƒ sink(s: Secret<u32>) ! {io} { }\nƒ f(s: Secret<u32>) ! {io} { sink(s); }"),
        vec!["S202"]
    );
    // taint lan qua phép tính và gán
    assert_eq!(
        codes(
            "ƒ log(x: u32) ! {io} { }\nƒ f(s: Secret<u32>) ! {io} { a ≔ s.declassify(); b ≔ a + 1; log(b); }"
        ),
        vec!["S201", "S202"]
    );
}

#[test]
fn clean_flows_stay_silent() {
    // Secret đi qua hàm KHÔNG io
    assert!(codes("ƒ keep(s: Secret<u32>) → Secret<u32> ! ∅ { s }\nƒ f(s: Secret<u32>) → Secret<u32> ! ∅ { keep(s) }").is_empty());
    // io với dữ liệu thường
    assert!(codes("ƒ log(x: u32) ! {io} { }\nƒ f(n: u32) ! {io} { log(n); }").is_empty());
    // chỉ nhận Secret rồi bỏ đi
    assert!(codes("ƒ drop_it(s: Secret<u32>) ! ∅ { }").is_empty());
}

#[test]
fn allow_needs_a_reason_and_a_yellow_code() {
    let body = "ƒ f() → &str ! ∅ { v ≔ \"AKIAIOSFODNN7EXAMPLE\"; v }";
    // hợp lệ: tắt được, và báo lại để kiểm toán
    let o = run(&format!(
        "#allow(S101, \"khoá mẫu trong tài liệu\")\n{body}"
    ));
    assert!(
        o.report.diagnostics.is_empty(),
        "{:?}",
        o.report.diagnostics
    );
    assert_eq!(o.allowed.len(), 1);
    assert_eq!(o.allowed[0].code, "S101");
    assert_eq!(o.allowed[0].reason, "khoá mẫu trong tài liệu");
    // thiếu lý do / lý do rỗng / mã lạ / mã ĐỎ / thừa tham số
    for attr in [
        "#allow(S101)",
        "#allow(S101, \"\")",
        "#allow(S101, \"   \")",
        "#allow(S999, \"lý do\")",
        "#allow(E0601, \"lý do\")",
        "#allow(S101, \"a\", \"b\")",
    ] {
        let c = codes(&format!("{attr}\n{body}"));
        assert!(c.contains(&"E1401".to_string()), "{attr}: {c:?}");
        assert!(
            c.contains(&"S101".to_string()),
            "{attr}: allow sai KHÔNG được tắt cảnh báo: {c:?}"
        );
    }
}

#[test]
fn allow_scope_and_unused_allows() {
    let two = "ƒ f() → &str ! ∅ { v ≔ \"AKIAIOSFODNN7EXAMPLE\"; v }\nƒ g() → &str ! ∅ { w ≔ \"AKIAIOSFODNN7EXAMPLE\"; w }";
    // chỉ tắt trong phạm vi item được gắn
    let c = codes(&format!("#allow(S101, \"mẫu\")\n{two}"));
    assert_eq!(c, vec!["S101"], "chỉ f được tắt, g vẫn cảnh báo");
    // allow đúng mã khác: không tắt gì ⇒ N1401 + cảnh báo còn nguyên
    let c = codes("#allow(S102, \"mẫu\")\nƒ f() → &str ! ∅ { v ≔ \"AKIAIOSFODNN7EXAMPLE\"; v }");
    assert_eq!(c, vec!["N1401", "S101"]);
    // allow thừa trên item sạch ⇒ N1401
    assert_eq!(
        codes("#allow(S101, \"thừa\")\nƒ f() → u32 ! ∅ { 1 }"),
        vec!["N1401"]
    );
}

// ---------------------------------------------------------------- biên chính xác của từng mẫu

fn flagged(v: &str) -> bool {
    codes(&lit("v", v)) == vec!["S101"]
}

fn rep(c: char, n: usize) -> String {
    std::iter::repeat_n(c, n).collect()
}

#[test]
fn pattern_boundaries_are_exact() {
    // (tiền tố, độ dài đuôi tối thiểu, ký tự hợp lệ, ký tự KHÔNG hợp lệ)
    let cases: [(&str, usize, char, char); 8] = [
        ("AKIA", 16, 'A', 'a'),
        ("ASIA", 16, '7', 'a'),
        ("ghp_", 36, 'a', '-'),
        ("github_pat_", 22, 'a', '-'),
        ("xoxb-", 10, 'a', '_'),
        ("AIza", 35, 'a', '.'),
        ("sk_live_", 24, 'a', '_'),
        ("rk_live_", 24, 'Z', '-'),
    ];
    for (pre, n, good, bad) in cases {
        let ok = format!("{pre}{}", rep(good, n));
        assert!(flagged(&ok), "{pre}: đủ {n} ký tự phải bị bắt: {ok}");
        // dư thêm vẫn bắt (khớp theo tiền tố + đoạn đầu)
        assert!(flagged(&format!("{ok}zz")), "{pre}: dư ký tự");
        // thiếu một
        assert!(
            !flagged(&format!("{pre}{}", rep(good, n - 1))),
            "{pre}: thiếu 1 ký tự phải KHÔNG bị bắt"
        );
        // một ký tự sai trong đoạn đuôi
        let mut tail: Vec<char> = rep(good, n).chars().collect();
        tail[n / 2] = bad;
        let broken: String = tail.into_iter().collect();
        assert!(
            !flagged(&format!("{pre}{broken}")),
            "{pre}: ký tự `{bad}` không hợp lệ"
        );
    }
    // Slack: chữ cái loại token phải thuộc {b,a,p,r,s}
    for t in ['b', 'a', 'p', 'r', 's'] {
        assert!(flagged(&format!("xox{t}-{}", rep('a', 10))), "{t}");
    }
    assert!(!flagged(&format!("xoxz-{}", rep('a', 10))));
    assert!(!flagged(&format!("xoxb_{}", rep('a', 10))));
    // GitHub: mọi tiền tố
    for p in ["ghp_", "gho_", "ghu_", "ghs_", "ghr_"] {
        assert!(flagged(&format!("{p}{}", rep('a', 36))), "{p}");
    }
}

#[test]
fn jwt_and_pem_boundaries() {
    let seg = |n: usize| format!("eyJ{}", rep('a', n - 3));
    let jwt = |a: &str, b: &str, c: &str| format!("{a}.{b}.{c}");
    assert!(flagged(&jwt(&seg(10), &seg(10), &rep('a', 10))));
    assert!(!flagged(&jwt(&seg(9), &seg(10), &rep('a', 10))));
    assert!(!flagged(&jwt(&seg(10), &seg(9), &rep('a', 10))));
    assert!(!flagged(&jwt(&seg(10), &seg(10), &rep('a', 9))));
    // đoạn thứ hai phải bắt đầu eyJ
    assert!(!flagged(&jwt(&seg(10), &rep('a', 12), &rep('a', 10))));
    // chỉ ba đoạn
    assert!(!flagged(&format!(
        "{}.{}.{}.{}",
        seg(10),
        seg(10),
        rep('a', 10),
        rep('a', 10)
    )));
    // ký tự lạ cắt đoạn
    let cut = format!("{}!{}", rep('a', 5), rep('a', 5));
    assert!(!flagged(&format!("{}.{}.{}", seg(10), seg(10), cut)));
    // PEM cần cả hai vế
    assert!(flagged("-----BEGIN RSA PRIVATE KEY----- x"));
    assert!(!flagged("-----BEGIN CERTIFICATE-----"));
    assert!(!flagged("PRIVATE KEY----- thôi"));
    // biên đầu/cuối chuỗi: token là TOÀN BỘ literal
    assert!(flagged(&format!("AKIA{}", rep('Q', 16))));
}

#[test]
fn entropy_rule_boundaries() {
    let s102 = |name: &str, v: &str| codes(&lit(name, v)) == vec!["S102"];
    // 12 ký tự đôi một khác nhau (entropy = log2 12 ≈ 3,58): đúng ngưỡng độ dài
    assert!(s102("token", "abcdefghijkl"));
    assert!(!s102("token", "abcdefghijk"), "11 ký tự: chưa đủ");
    // mật khẩu: ≥ 8, entropy ≥ 2,5, có chữ số
    assert!(s102("password", "abcd1234"));
    assert!(s102("pwd", "abcd1234"));
    assert!(!s102("password", "abcdefgh"), "không có chữ số");
    assert!(!s102("password", "abc1234"), "7 ký tự");
    assert!(!s102("password", "11111111"), "entropy 0");
    // tên không phải mật khẩu thì không dùng ngưỡng thấp
    assert!(!s102("name", "abcd1234"));
    assert!(
        !s102("key", "abcd1234"),
        "key không phải tên mật khẩu: cần ≥ 12 ký tự"
    );
    // gán bằng `←` cũng được tính, và có kiểu giữa tên và dấu gán
    assert!(
        codes("ƒ f() → &str ! ∅ { μapi_key ≔ \"x\"; api_key ← \"x8F3kQ9zLp2Vw7Rm\"; api_key }")
            == vec!["S102"]
    );
    assert!(codes("□ API_KEY: &str ≔ \"x8F3kQ9zLp2Vw7Rm\";") == vec!["S102"]);
    // chuỗi đã bị S101 bắt thì không báo S102 thêm
    assert_eq!(codes(&lit("api_key", "AKIAIOSFODNN7EXAMPLE")), vec!["S101"]);
    // gán xa tên (qua dấu `;`) không tính
    assert!(codes("ƒ f() → &str ! ∅ { api_key ≔ 1; \"x8F3kQ9zLp2Vw7Rm\" }").is_empty());
}

// ---------------------------------------------------------------- taint chính xác

#[test]
fn taint_flows_through_many_shapes_and_only_real_declassify_counts() {
    let sink = "ƒ sink2(a: u32, b: u32) ! {io} { }\nƒ sink1(s: u32) ! {io} { }\n";
    // chỉ MỘT trong hai đối số bị nhiễm
    assert_eq!(
        codes(&format!(
            "{sink}ƒ f(s: Secret<u32>) ! {{io}} {{ x ≔ s.declassify(); sink2(1u32, x); }}"
        )),
        vec!["S201", "S202"]
    );
    assert_eq!(
        codes(&format!(
            "{sink}ƒ f(s: Secret<u32>) ! {{io}} {{ x ≔ s.declassify(); sink2(x, 1u32); }}"
        )),
        vec!["S201", "S202"]
    );
    // Secret nằm trong struct / enum / tuple / tham chiếu
    for ty_decl in [
        "▢ B { s: Secret<u32> }\nƒ sinkb(b: B) ! {io} { }\nƒ f(b: B) ! {io} { sinkb(b); }",
        "⊔ E { A(Secret<u32>), N }\nƒ sinke(e: E) ! {io} { }\nƒ f(e: E) ! {io} { sinke(e); }",
        "ƒ sinkt(t: (u8, Secret<u32>)) ! {io} { }\nƒ f(t: (u8, Secret<u32>)) ! {io} { sinkt(t); }",
        "ƒ sinkr(r: &Secret<u32>) ! {io} { }\nƒ f(r: &Secret<u32>) ! {io} { sinkr(r); }",
        "ƒ sinkg(o: Option<Secret<u32>>) ! {io} { }\nƒ f(o: Option<Secret<u32>>) ! {io} { sinkg(o); }",
        "▢ W<T> { v: T }\nƒ sinkw(w: W<Secret<u32>>) ! {io} { }\nƒ f(w: W<Secret<u32>>) ! {io} { sinkw(w); }",
    ] {
        assert_eq!(codes(ty_decl), vec!["S202"], "{ty_decl}");
    }
    // KHÔNG có Secret ở bất cứ đâu ⇒ im lặng, kể cả struct/enum/tuple lồng
    for ty_decl in [
        "▢ B { s: u32 }\nƒ sinkb(b: B) ! {io} { }\nƒ f(b: B) ! {io} { sinkb(b); }",
        "ƒ sinkt(t: (u8, u32)) ! {io} { }\nƒ f(t: (u8, u32)) ! {io} { sinkt(t); }",
        "⊔ E { A(u32), N }\nƒ sinke(e: E) ! {io} { }\nƒ f(e: E) ! {io} { sinke(e); }",
    ] {
        assert!(codes(ty_decl).is_empty(), "{ty_decl}");
    }
    // `declassify` của kiểu NGƯỜI DÙNG hay hàm tự do trùng tên không phải declassify của Secret
    assert!(codes("▢ S { v: u32 }\n⊐ S { ƒ declassify(self) → u32 ! ∅ { self.v } }\nƒ f(s: S) → u32 ! ∅ { s.declassify() }").is_empty());
    assert!(
        codes("ƒ declassify(x: u32) → u32 ! ∅ { x }\nƒ f() → u32 ! ∅ { declassify(1u32) }")
            .is_empty()
    );
    // phương thức khác của Secret (không phải declassify) không bị S201
    // taint ngược chiều trong vòng lặp: cần lặp tới điểm cố định
    assert_eq!(
        codes(&format!(
            "{sink}ƒ f(s: Secret<u32>) ! {{io}} {{ μa ≔ 0u32; μb ≔ 0u32; μc ≔ 0u32; ⟳ i in 0..3u32 {{ sink1(a); a ← b; b ← c; c ← s.declassify(); }} }}"
        )),
        vec!["S201", "S202"]
    );
    // phép toán và ép kiểu truyền taint
    assert_eq!(
        codes(&format!(
            "{sink}ƒ f(s: Secret<u32>) ! {{io}} {{ x ≔ s.declassify(); y ≔ x * 2u32; z ≔ y as u64; sink1(z as u32); }}"
        )),
        vec!["S201", "S202"]
    );
}

#[test]
fn taint_needs_only_one_tainted_local_among_many_operands() {
    let sink = "ƒ sink2(a: u32, b: u32) ! {io} { }\n";
    // hai đối số đều là biến cục bộ, chỉ một bị nhiễm
    assert_eq!(
        codes(&format!(
            "{sink}ƒ f(s: Secret<u32>) ! {{io}} {{ k ≔ 1u32; x ≔ s.declassify(); sink2(k, x); }}"
        )),
        vec!["S201", "S202"]
    );
    assert_eq!(
        codes(&format!(
            "{sink}ƒ f(s: Secret<u32>) ! {{io}} {{ k ≔ 1u32; x ≔ s.declassify(); sink2(x, k); }}"
        )),
        vec!["S201", "S202"]
    );
    // phép toán hai biến, chỉ một bị nhiễm
    assert_eq!(
        codes(&format!(
            "{sink}ƒ f(s: Secret<u32>) ! {{io}} {{ k ≔ 1u32; x ≔ s.declassify(); y ≔ k + x; sink2(y, 0u32); }}"
        )),
        vec!["S201", "S202"]
    );
    assert_eq!(
        codes(&format!(
            "{sink}ƒ f(s: Secret<u32>) ! {{io}} {{ k ≔ 1u32; x ≔ s.declassify(); y ≔ x + k; sink2(y, 0u32); }}"
        )),
        vec!["S201", "S202"]
    );
    // không có gì nhiễm thì im lặng dù có nhiều biến
    assert!(
        codes(&format!(
            "{sink}ƒ f() ! {{io}} {{ k ≔ 1u32; x ≔ 2u32; y ≔ k + x; sink2(y, k); }}"
        ))
        .is_empty()
    );
}

#[test]
fn scanning_continues_past_non_matching_names_and_invalid_allows() {
    // tên gợi ý nhưng giá trị không phải chuỗi, rồi tên khác có secret thật
    assert_eq!(
        codes(
            "ƒ f() → &str ! ∅ { key ≔ 5u32; token ≔ \"short\"; api_key ≔ \"x8F3kQ9zLp2Vw7Rm\"; api_key }"
        ),
        vec!["S102"]
    );
    // hai literal secret liên tiếp: cả hai được báo
    assert_eq!(
        codes(
            "ƒ f() → &str ! ∅ { a ≔ \"AKIAIOSFODNN7EXAMPLE\"; api_key ≔ \"x8F3kQ9zLp2Vw7Rm\"; a }"
        ),
        vec!["S101", "S102"]
    );
    // #allow sai rồi #allow đúng liền nhau: cả hai đều được xử lý
    let c = codes("#allow(S101)\n#allow(S102, \"không dùng\")\nƒ f() → u32 ! ∅ { 1u32 }");
    assert_eq!(c, vec!["E1401", "N1401"]);
    // item thứ hai vẫn được xét sau item có #allow sai
    let c = codes(
        "#allow(S101)\nƒ f() → u32 ! ∅ { 1u32 }\n#allow(S102, \"thừa\")\nƒ g() → u32 ! ∅ { 2u32 }",
    );
    assert_eq!(c, vec!["E1401", "N1401"]);
}
