//! Test viết để GIẾT đột biến mã nguồn sống sót (M6, ADR-023 §10a; công cụ `tools/mut.py`). Mỗi test ghim một hành vi mà
//! đột biến làm sai: số chẩn đoán (khử trùng, mỗi khoản vay một lần), vị trí nhãn "bị move ở đây", chọn mức truy cập nông/sâu,
//! luật "gán cả biến giết khoản vay qua `*biến`", ràng buộc region qua tuple/mảng/Option/ADT/`&&`/giá trị hàm/trait/self.
mod common;
mod support;
use common::*;
use support::progen::PRELUDE;

fn items(extra: &str) -> String {
    format!(
        "{PRELUDE}ƒ add2(a: i32, b: i32) ! ∅ {{ }}\nƒ two_b(x: B, y: B) ! ∅ {{ }}\nƒ k2(a: i32, b: B) ! ∅ {{ }}\nƒ idu<'a>(x: &!'a i32) → &!'a i32 ! ∅ {{ x }}\n{extra}\n"
    )
}

fn run(extra: &str) -> (String, Run) {
    let src = items(extra);
    let r = bc(&src);
    (src, r)
}

#[track_caller]
fn expect_codes(extra: &str, code: &str, n: usize) {
    let (src, r) = run(extra);
    assert_eq!(
        bc_codes(&r),
        vec![code.to_string(); n],
        "{extra}\n{:?}\n{src}",
        bc_show(&r)
    );
}

#[track_caller]
fn expect_ok(extra: &str) {
    let (src, r) = run(extra);
    assert!(
        r.bc.diagnostics.is_empty(),
        "{extra}\n{:?}\n{src}",
        bc_show(&r)
    );
}

fn text(src: &str, s: gt_span::Span) -> &str {
    &src[s.start as usize..s.end as usize]
}

// ------------------------------------------------------------------ gán cả biến giết khoản vay qua `*biến`

#[test]
fn assigning_a_whole_pointer_kills_loans_through_it() {
    // `p ← &!y` làm khoản vay `&!*p` (trỏ vào `x`) không còn là khoản vay của `*p` mới
    expect_ok("ƒ f() ! ∅ { μx ≔ 1; μy ≔ 2; μp ≔ &!x; q ≔ &!*p; p ← &!y; *p ← 3; use_i(*q); }");
    // cùng luật khi gán bằng kết quả của LỜI GỌI
    expect_ok("ƒ f() ! ∅ { μx ≔ 1; μy ≔ 2; μp ≔ &!x; q ≔ &!*p; p ← idu(&!y); *p ← 3; use_i(*q); }");
    // nhưng ghi QUA `*p` không giết khoản vay: hai lần ghi, hai chẩn đoán
    expect_codes(
        "ƒ f() ! ∅ { μx ≔ 1; p ≔ &!x; q ≔ &!*p; *p ← 1; *p ← 2; use_i(*q); }",
        "E0406",
        2,
    );
}

// ------------------------------------------------------------------ mức truy cập nông/sâu

#[test]
fn moving_or_borrowing_a_pointer_conflicts_with_a_live_reborrow_through_it() {
    expect_codes("ƒ f(p: &!i32) ! ∅ { r ≔ &!*p; q ≔ p; *r ← 1; }", "E0406", 1);
    expect_codes(
        "ƒ f(p: &!i32) ! ∅ { r ≔ &!*p; s ≔ &p; *r ← 1; use_i(**s); }",
        "E0403",
        1,
    );
}

#[test]
fn a_reborrow_through_a_pointer_may_outlive_the_pointer_variable() {
    // `p` chết ở cuối khối trong khi `q` (reborrow qua `*p`) còn dùng sau: hợp lệ (StorageDead là truy cập NÔNG)
    expect_ok("ƒ f() ! ∅ { μx ≔ 1; μy ≔ 0; μq ≔ &!y; { p ≔ &!x; q ← &!*p; }; *q ← 2; }");
}

// ------------------------------------------------------------------ khử trùng và đếm chẩn đoán

#[test]
fn the_same_conflict_is_reported_once() {
    expect_codes(
        "ƒ f() ! ∅ { μx ≔ 1; r ≔ &!x; add2(x, x); *r ← 1; }",
        "E0403",
        1,
    );
    // hai khoản vay KHÁC nhau cùng bị xung đột ⇒ hai chẩn đoán
    expect_codes(
        "ƒ f() ! ∅ { μx ≔ 1; r ≔ &x; s ≔ &x; x ← 2; use_i(*r); use_i(*s); }",
        "E0406",
        2,
    );
}

#[test]
fn every_move_out_of_a_reference_in_one_call_is_reported() {
    expect_codes("ƒ f(x: i32, r: &B) ! ∅ { k2(x, *r); }", "E0407", 1);
    expect_codes("ƒ f(r1: &B, r2: &B) ! ∅ { two_b(*r1, *r2); }", "E0407", 2);
    // một chỗ bị move ra khỏi tuple (không có Drop) là hợp lệ
    expect_ok("ƒ f() ! ∅ { t ≔ (mk(), 1); x ≔ t.0; use_i(t.1); eat(x); }");
}

// ------------------------------------------------------------------ nhãn và thông điệp

#[test]
fn use_after_move_points_at_the_move_site_of_that_very_variable() {
    let cases: [(&str, &str, &str); 4] = [
        // (thân, văn bản của nhãn, văn bản đứng trước mà nhãn phải nằm SAU)
        (
            "a ≔ mk(); c ≔ mk(); eat(a); eat(c); use_i(c.v);",
            "c",
            "eat(a)",
        ),
        (
            "a ≔ mk(); c ≔ mk(); two_b(a, c); use_i(c.v);",
            "c",
            "two_b(a",
        ),
        (
            "a ≔ mk(); c ≔ mk(); y ≔ a; z ≔ c; use_i(c.v);",
            "c",
            "y ≔ a",
        ),
        (
            "a ≔ mk(); c ≔ mk(); t ≔ (a, 1); u ≔ (c, 2); use_i(c.v);",
            "(c, 2)",
            "t ≔ (a, 1)",
        ),
    ];
    for (body, label_text, after) in cases {
        let (src, r) = run(&format!("ƒ f() ! ∅ {{ {body} }}"));
        assert_eq!(bc_codes(&r), vec!["E0401"], "{body}");
        let d = &r.bc.diagnostics[0];
        assert_eq!(d.labels.len(), 1, "{body}: {d:?}");
        let l = &d.labels[0];
        assert_eq!(l.message, "giá trị bị move ở đây", "{body}");
        assert_eq!(text(&src, l.span), label_text, "{body}");
        let prior = src.find(after).expect("văn bản đứng trước");
        assert!(
            l.span.start as usize > prior,
            "{body}: nhãn phải ở SAU `{after}`"
        );
    }
}

#[test]
fn conflict_messages_name_the_kind_of_the_existing_loan() {
    let (_, r) = run("ƒ f() ! ∅ { μx ≔ 1; r ≔ &!x; x ← 2; *r ← 3; }");
    assert!(
        bc_show(&r)[0].contains("mượn duy nhất `&!`"),
        "{:?}",
        bc_show(&r)
    );
    let (_, r) = run("ƒ f() ! ∅ { μx ≔ 1; r ≔ &x; x ← 2; use_i(*r); }");
    assert!(
        bc_show(&r)[0].contains("mượn chung `&`"),
        "{:?}",
        bc_show(&r)
    );
    // nhãn trỏ vào chỗ tạo khoản mượn
    let (src, r) = run("ƒ f() ! ∅ { μx ≔ 1; r ≔ &x; x ← 2; use_i(*r); }");
    let d = &r.bc.diagnostics[0];
    assert_eq!(d.labels.len(), 1);
    assert_eq!(text(&src, d.labels[0].span), "&x");
}

// ------------------------------------------------------------------ ràng buộc region qua kiểu và lời gọi

#[test]
fn references_flow_through_tuples_arrays_options_and_adts() {
    expect_codes(
        "ƒ f() ! ∅ { μx ≔ 1; t ≔ (&x, 1); u ≔ t; x ← 2; use_i(*u.0); }",
        "E0406",
        1,
    );
    expect_codes(
        "ƒ f() ! ∅ { μx ≔ 1; a ≔ [&x; 2]; x ← 2; use_i(*a[0]); }",
        "E0406",
        1,
    );
    expect_codes(
        "ƒ f() ! ∅ { μx ≔ 1; a ≔ [&x, &x]; b ≔ a; x ← 2; use_i(*b[0]); }",
        "E0406",
        2,
    );
    expect_codes(
        "ƒ f() ! ∅ { μx ≔ 1; o ≔ Some(&x); p ≔ o; x ← 2; ⌥ p { Some(r) ↦ { use_i(*r); }, None ↦ { } }; }",
        "E0406",
        1,
    );
    expect_codes(
        "ƒ f() ! ∅ { μx ≔ 1; h ≔ Hs { r: &x }; k ≔ h; x ← 2; use_i(*k.r); }",
        "E0406",
        1,
    );
    // `&!` trong ADT là bất biến: chuyển cả ADT rồi dùng gốc là lỗi
    expect_codes(
        "ƒ f() ! ∅ { μx ≔ 1; h ≔ Hm { r: &!x }; k ≔ h; use_i(x); *k.r ← 2; }",
        "E0403",
        1,
    );
    expect_codes(
        "ƒ f() ! ∅ { μx ≔ 1; r ≔ &x; rr ≔ &r; q ≔ rr; x ← 2; use_i(**q); }",
        "E0406",
        1,
    );
    expect_codes(
        "ƒ f() ! ∅ { μx ≔ 1; r ≔ &x; rr ≔ &r; q ≔ &*rr; x ← 2; use_i(**q); }",
        "E0406",
        1,
    );
    // hợp lệ: dùng xong rồi ghi
    expect_ok("ƒ f() ! ∅ { μx ≔ 1; t ≔ (&x, 1); u ≔ t; use_i(*u.0); x ← 2; }");
    expect_ok("ƒ f() ! ∅ { μx ≔ 1; h ≔ Hm { r: &!x }; k ≔ h; *k.r ← 2; use_i(x); }");
}

#[test]
fn calls_through_function_values_and_trait_methods_keep_the_loans_alive() {
    let id = "ƒ ident(x: &i32) → &i32 ! ∅ { x }";
    expect_codes(
        &format!("{id}\nƒ f() ! ∅ {{ μx ≔ 1; h ≔ ident; r ≔ h(&x); x ← 2; use_i(*r); }}"),
        "E0406",
        1,
    );
    let tr = "⊏ G { ƒ g(&self) → &i32 ! ∅; }\n▢ S { n: i32 }\n⊐ G for S { ƒ g(&self) → &i32 ! ∅ { &self.n } }\nƒ via<T: G>(t: &T) → &i32 ! ∅ { t.g() }";
    expect_codes(
        &format!("{tr}\nƒ f() ! ∅ {{ μs ≔ S {{ n: 1 }}; r ≔ via(&s); s.n ← 2; use_i(*r); }}"),
        "E0406",
        1,
    );
    expect_ok(&format!(
        "{tr}\nƒ f() ! ∅ {{ μs ≔ S {{ n: 1 }}; r ≔ via(&s); use_i(*r); s.n ← 2; }}"
    ));
}

#[test]
fn method_receivers_follow_their_declared_self_kind() {
    let imp = "⊐ B { ƒ refv(&self) → &i32 ! ∅ { &self.v } ƒ incr(&!self) → &!i32 ! ∅ { &!self.v } ƒ val(self) → i32 ! ∅ { self.v } }";
    expect_codes(
        &format!("{imp}\nƒ f() ! ∅ {{ μb ≔ mk(); r ≔ b.refv(); b.v ← 2; use_i(*r); }}"),
        "E0406",
        1,
    );
    expect_codes(
        &format!("{imp}\nƒ f() ! ∅ {{ μb ≔ mk(); r ≔ b.incr(); use_i(b.v); *r ← 1; }}"),
        "E0403",
        1,
    );
    expect_ok(&format!(
        "{imp}\nƒ f() ! ∅ {{ μb ≔ mk(); r ≔ b.incr(); *r ← 1; use_i(b.v); use_i(b.val()); }}"
    ));
}

#[test]
fn signature_violations_are_located_at_the_returning_expression() {
    let (src, r) = run("ƒ pk<'a, 'b>(x: &'a i32, y: &'b i32) → &'a i32 ! ∅ { y }");
    assert_eq!(bc_codes(&r), vec!["E0404"], "{:?}", bc_show(&r));
    assert_eq!(text(&src, r.bc.diagnostics[0].span), "y");
    let (src, r) = run("ƒ pk<'a, 'b>(x: &'a i32, y: &'b i32) → &'a i32 ! ∅ { z ≔ y; w ≔ z; w }");
    assert_eq!(bc_codes(&r), vec!["E0404"], "{:?}", bc_show(&r));
    eprintln!("span chuỗi: {:?}", text(&src, r.bc.diagnostics[0].span));
}

#[test]
fn assigning_a_pointer_does_not_conflict_with_a_loan_on_a_field_behind_it() {
    // khoản vay `&!(*p).n` (sau `*`, rồi một trường): gán lại `p` là truy cập NÔNG, không đụng dữ liệu sau `*`
    expect_ok(
        "ƒ f() ! ∅ { μw ≔ mkw(); μw2 ≔ mkw(); μp ≔ &!w; q ≔ &!p.n; p ← &!w2; p.n ← 3; use_i(*q); }",
    );
}

#[test]
fn recursive_adts_terminate_and_carry_their_regions() {
    // `adt_invariant` đệ quy qua `Option<&Lst<'a>>`: phải có chốt chặn vòng
    expect_codes(
        "▢ Lst<'a> { v: i32, n: Option<&'a Lst<'a>> }\nƒ f() ! ∅ { μx ≔ Lst { v: 1, n: None }; m ≔ Lst { v: 2, n: Some(&x) }; k ≔ m; x.v ← 3; use_i(k.v); }",
        "E0406",
        1,
    );
}

// ------------------------------------------------------------------ bất biến (invariance) của `&!` và ADT

#[test]
fn unique_references_are_invariant_in_what_they_point_to() {
    // kinh điển: `store` ghi một tham chiếu NGẮN vào chỗ đang giữ tham chiếu DÀI; nếu `&!` hiệp biến thì `p` bị thu hẹp
    let store = "ƒ store<'a>(slot: &!&'a i32, v: &'a i32) ! ∅ { *slot ← v; }";
    expect_codes(
        &format!(
            "{store}\nƒ f() ! ∅ {{ z ≔ 1; μp ≔ &z; {{ y ≔ 7; store(&!p, &y); }}; use_i(*p); }}"
        ),
        "E0404",
        1,
    );
    // hợp lệ: cả hai cùng sống đủ lâu
    expect_ok(&format!(
        "{store}\nƒ f() ! ∅ {{ z ≔ 1; y ≔ 7; μp ≔ &z; store(&!p, &y); use_i(*p); }}"
    ));
    // phương thức `&!self` trên ADT có tham số lifetime: cùng luật (kiểu `self` khai báo là `&!Cr<'a>`)
    let cr = "▢ Cr<'a> { r: &'a i32 }\n⊐ <'a> Cr<'a> { ƒ put(&!self, v: &'a i32) ! ∅ { self.r ← v; } ƒ get(&self) → &'a i32 ! ∅ { self.r } }";
    expect_codes(
        &format!(
            "{cr}\nƒ f() ! ∅ {{ z ≔ 1; μc ≔ Cr {{ r: &z }}; {{ y ≔ 5; c.put(&y); }}; use_i(*c.r); }}"
        ),
        "E0404",
        1,
    );
    expect_ok(&format!(
        "{cr}\nƒ f() ! ∅ {{ z ≔ 1; y ≔ 5; μc ≔ Cr {{ r: &z }}; c.put(&y); use_i(*c.r); }}"
    ));
    // ADT giữ `&!` của tham chiếu: bất biến theo cả ADT
    let sl = "▢ SlT<'a> { s: &!'a &'a i32 }
ƒ putt2<'a>(sl: SlT<'a>, v: &'a i32) ! ∅ { *sl.s ← v; }";
    let (src, r) = run(&format!(
        "{sl}
ƒ f() ! ∅ {{ z ≔ 1; μp ≔ &z; {{ y ≔ 7; sl ≔ SlT {{ s: &!p }}; putt2(sl, &y); }}; use_i(*p); }}"
    ));
    assert!(
        bc_codes(&r).iter().any(|c| c == "E0404"),
        "{:?}
{src}",
        bc_show(&r)
    );
}

#[test]
fn reborrow_through_references_is_bounded_by_the_right_regions() {
    // qua tham chiếu CHUNG (Copy) kết quả chỉ bị chặn bởi region của tham chiếu trong cùng: `&**pp : 'l`
    expect_ok("ƒ rb<'s, 'l>(pp: &'s &'l i32) → &'l i32 ! ∅ { &**pp }");
    // qua `&!` thì bị chặn bởi CẢ HAI: không thể lấy `&!'l` từ `&!'s`
    expect_codes(
        "ƒ bad<'s, 'l>(pp: &!'s &!'l i32) → &!'l i32 ! ∅ { &!**pp }",
        "E0404",
        1,
    );
}

// ------------------------------------------------------------------ độ chính xác của ánh xạ chữ ký (nếu rơi về đường bảo thủ sẽ báo nhầm)

#[test]
fn call_signatures_tie_the_result_only_to_the_declared_arguments() {
    // generic: kết quả chỉ buộc với đối số `a`, không buộc với `b`
    let g = "ƒ first_of<P1, P2>(a: P1, b: P2) → P1 ! ∅ { a }";
    expect_ok(&format!(
        "{g}\nƒ f() ! ∅ {{ μx ≔ 1; μy ≔ 2; r ≔ first_of(&x, &y); y ← 3; use_i(*r); }}"
    ));
    expect_codes(
        &format!("{g}\nƒ f() ! ∅ {{ μx ≔ 1; μy ≔ 2; r ≔ first_of(&x, &y); x ← 3; use_i(*r); }}"),
        "E0406",
        1,
    );
    // tham số đầu có `'a` riêng: kết quả không buộc với `o`
    let m = "ƒ pick_n<'a>(w: &'a W, o: &i32) → &'a i32 ! ∅ { &w.n }";
    expect_ok(&format!(
        "{m}\nƒ f() ! ∅ {{ μw ≔ mkw(); μy ≔ 2; r ≔ pick_n(&w, &y); y ← 3; use_i(*r); }}"
    ));
    expect_codes(
        &format!(
            "{m}\nƒ f() ! ∅ {{ μw ≔ mkw(); μy ≔ 2; r ≔ pick_n(&w, &y); w.n ← 3; use_i(*r); }}"
        ),
        "E0406",
        1,
    );
    // ADT/tuple/Option trong chữ ký
    let a = "ƒ uo<'a>(o: Option<&'a i32>, d: &'a i32) → &'a i32 ! ∅ { d }\nƒ tl<'a>(t: (&'a i32, i32)) → &'a i32 ! ∅ { t.0 }\nƒ gt<'a>(h: Hs<'a>) → &'a i32 ! ∅ { h.r }";
    expect_codes(
        &format!("{a}\nƒ f() ! ∅ {{ μx ≔ 1; μy ≔ 2; r ≔ uo(Some(&x), &y); x ← 3; use_i(*r); }}"),
        "E0406",
        1,
    );
    expect_codes(
        &format!("{a}\nƒ f() ! ∅ {{ μx ≔ 1; r ≔ tl((&x, 1)); x ← 3; use_i(*r); }}"),
        "E0406",
        1,
    );
    expect_codes(
        &format!("{a}\nƒ f() ! ∅ {{ μx ≔ 1; r ≔ gt(Hs {{ r: &x }}); x ← 3; use_i(*r); }}"),
        "E0406",
        1,
    );
    expect_ok(&format!(
        "{a}\nƒ f() ! ∅ {{ μx ≔ 1; r ≔ tl((&x, 1)); use_i(*r); x ← 3; }}"
    ));
    // `'static` trong chữ ký là `'static` thật: không thể đưa tham chiếu tới local vào
    expect_codes(
        "ƒ need(x: &'static i32) ! ∅ { }\nƒ f() ! ∅ { x ≔ 1; need(&x); }",
        "E0404",
        1,
    );
}

// ------------------------------------------------------------------ gom nhiều nguồn vào CÙNG một nhóm region

#[test]
fn flows_into_one_region_group_do_not_pull_in_unrelated_loans() {
    // mảng: mọi phần tử chung một region; nhưng ràng buộc là MỘT chiều (nguồn → đích), nên `t1` dùng sau cùng không kéo dài `&y`
    expect_ok(
        "ƒ f() ! ∅ { μx ≔ 1; μy ≔ 2; t1 ≔ &x; a ≔ [t1, &y]; use_i(*a[0]); y ← 3; use_i(*t1); }",
    );
    // tuple: ghi đè phần tử rồi dùng giá trị cũ ở biến khác
    expect_ok(
        "ƒ f() ! ∅ { μx ≔ 1; μy ≔ 2; t1 ≔ &x; μtup ≔ (t1, 1); tup.0 ← &y; use_i(*tup.0); y ← 3; use_i(*t1); }",
    );
}

#[test]
fn adts_are_invariant_exactly_when_a_field_type_is() {
    // trường là TUPLE có `&!` ⇒ ADT bất biến (kiểm tra `ty_invariant` qua tuple)
    let tu = "▢ Tu<'a> { t: (&!'a &'a i32, i32) }\nƒ put3<'a>(x: Tu<'a>, v: &'a i32) ! ∅ { *x.t.0 ← v; }";
    let (src, r) = run(&format!(
        "{tu}\nƒ f() ! ∅ {{ z ≔ 1; μp ≔ &z; {{ y ≔ 7; x ≔ Tu {{ t: (&!p, 1) }}; put3(x, &y); }}; use_i(*p); }}"
    ));
    assert!(
        bc_codes(&r).iter().any(|c| c == "E0404"),
        "{:?}\n{src}",
        bc_show(&r)
    );
    // trường là MẢNG có `&!`
    let ar =
        "▢ Ar<'a> { a: [&!'a &'a i32; 1] }\nƒ put4<'a>(x: Ar<'a>, v: &'a i32) ! ∅ { *x.a[0] ← v; }";
    let (src, r) = run(&format!(
        "{ar}\nƒ f() ! ∅ {{ z ≔ 1; μp ≔ &z; {{ y ≔ 7; x ≔ Ar {{ a: [&!p] }}; put4(x, &y); }}; use_i(*p); }}"
    ));
    assert!(
        bc_codes(&r).iter().any(|c| c == "E0404"),
        "{:?}\n{src}",
        bc_show(&r)
    );
    // ADT chỉ có `&` thì HIỆP BIẾN: gán `Hs<'dài>` vào `Hs<'ngắn>` hợp lệ
    expect_ok(
        "ƒ f() ! ∅ { μx ≔ 1; h1 ≔ Hs { r: &x }; { y ≔ 2; μh ≔ Hs { r: &y }; h ← Hs { r: h1.r }; use_i(*h.r); }; use_i(*h1.r); }",
    );
    // ADT bọc ADT bất biến cũng bất biến (qua đối số kiểu)
    let wr = "▢ Wr2<T> { t: T }\nƒ put5<'a>(x: Wr2<SlT<'a>>, v: &'a i32) ! ∅ { *x.t.s ← v; }\n▢ SlT<'a> { s: &!'a &'a i32 }";
    let (src, r) = run(&format!(
        "{wr}\nƒ f() ! ∅ {{ z ≔ 1; μp ≔ &z; {{ y ≔ 7; x ≔ Wr2 {{ t: SlT {{ s: &!p }} }}; put5(x, &y); }}; use_i(*p); }}"
    ));
    // bị từ chối (E0403: `p` còn bị `&!` giữ tới lần dùng sau; hoặc E0404: `y` treo)
    assert!(!bc_codes(&r).is_empty(), "{src}");
}

#[test]
fn call_arguments_flow_one_way_into_the_signature() {
    // `t` còn dùng SAU khi `y` được ghi: hợp lệ vì đối số chảy MỘT chiều vào tham số (`'a` không kéo dài theo `t`)
    let pk = "ƒ pk<'a>(x: &'a i32, y: &'a i32) → &'a i32 ! ∅ { x }";
    expect_ok(&format!(
        "{pk}\nƒ f() ! ∅ {{ μx ≔ 1; μy ≔ 2; t ≔ &x; r ≔ pk(t, &y); use_i(*r); y ← 3; use_i(*t); }}"
    ));
}

#[test]
fn an_adt_with_one_invariant_field_among_covariant_ones_is_invariant() {
    // `Sl2` có THÊM một trường `i32` (không bất biến): bất biến vẫn do `any`, không phải `all`, các trường
    let sl2 = "▢ Sl2<'a> { s: &!'a &'a i32, n: i32 }\nƒ put6<'a>(x: Sl2<'a>, v: &'a i32) ! ∅ { *x.s ← v; }";
    let (src, r) = run(&format!(
        "{sl2}\nƒ f() ! ∅ {{ z ≔ 1; μp ≔ &z; {{ y ≔ 7; x ≔ Sl2 {{ s: &!p, n: 0 }}; put6(x, &y); }}; use_i(*p); }}"
    ));
    assert!(
        bc_codes(&r).iter().any(|c| c == "E0404"),
        "{:?}\n{src}",
        bc_show(&r)
    );
    // bọc trong ADT hiệp biến: bất biến truyền lên qua đối số kiểu (`||`, không phải `&&`)
    let wr = "▢ IdT<T> { t: T }\nƒ put7<'a>(x: IdT<Sl2<'a>>, v: &'a i32) ! ∅ { *x.t.s ← v; }";
    let (src, r) = run(&format!(
        "{sl2}\n{wr}\nƒ f() ! ∅ {{ z ≔ 1; μp ≔ &z; {{ y ≔ 7; sl ≔ Sl2 {{ s: &!p, n: 0 }}; put7(IdT {{ t: sl }}, &y); }}; use_i(*p); }}"
    ));
    assert!(
        !bc_codes(&r).is_empty(),
        "phải bị từ chối
{src}"
    );
}

#[test]
fn recursive_adts_stay_covariant() {
    // `adt_invariant` đệ quy qua `Option<&Lst<'a>>`: chốt chặn vòng phải trả `false` (hiệp biến), nếu không `Lst` thành bất biến và
    // `y` bị mượn lây theo `t`. Hợp lệ: `h ← Lst { n: Some(t) }` rồi ghi `y` sau khi `h` hết dùng, `t` còn dùng sau.
    expect_ok(
        "▢ Lst<'a> { v: i32, n: Option<&'a Lst<'a>> }\nƒ f() ! ∅ { μa ≔ Lst { v: 1, n: None }; μy ≔ Lst { v: 2, n: None }; t ≔ &a; μh ≔ Lst { v: 3, n: Some(&y) }; h ← Lst { v: 4, n: Some(t) }; use_i(h.v); y.v ← 5; use_i(t.v); }",
    );
}

#[test]
fn raw_pointers_are_invariant_in_what_they_point_to() {
    // không thể thu hẹp `'a` của `*μ &'a i32` (con trỏ thô bất biến): `y` ngắn hơn region phổ quát của `p`
    expect_codes(
        "ƒ g<'a>(p: *μ &'a i32, v: &'a i32) ! ∅ { }\nƒ f<'x>(p: *μ &'x i32) ! ∅ { y ≔ 1; g(p, &y); }",
        "E0404",
        1,
    );
}

#[test]
fn a_static_requirement_reports_every_region_the_source_fails_to_outlive() {
    // `'static` outlive MỌI region phổ quát: trả `&'a` thay `&'static` vi phạm với `'static` VÀ với `'b`
    let (_, r) = run("ƒ st<'a, 'b>(x: &'a i32, y: &'b i32) → &'static i32 ! ∅ { x }");
    assert_eq!(bc_codes(&r), vec!["E0404", "E0404"], "{:?}", bc_show(&r));
}

#[test]
fn a_covariant_wrapper_of_an_invariant_adt_is_invariant() {
    // `IdU<T>` hiệu biến, nhưng `IdU<SlU<'a>>` bất biến theo `'a`: `sl` là THAM SỐ (region phổ quát `'a`), `y` cục bộ ngắn hơn ⇒ ĐỎ;
    // nếu coi `IdU` hiệp biến thì `'a` bị thu hẹp xuống region của `y` và chương trình bị chấp nhận
    let adts = "▢ SlU<'a> { s: &!'a &'a i32 }\n▢ IdU<T> { t: T }\nƒ putu<'a>(x: IdU<SlU<'a>>, v: &'a i32) ! ∅ { *x.t.s ← v; }";
    expect_codes(
        &format!(
            "{adts}\nƒ attack<'a>(sl: SlU<'a>) ! ∅ {{ y ≔ 7; w ≔ IdU {{ t: sl }}; putu(w, &y); }}"
        ),
        "E0404",
        1,
    );
}

#[test]
fn function_typed_parameters_are_mapped() {
    let ap = "ƒ ident(x: &i32) → &i32 ! ∅ { x }\nƒ ap<'a>(fun: ƒ(&'a i32) → &'a i32 ! ∅, x: &'a i32) → &'a i32 ! ∅ { fun(x) }";
    expect_ok(&format!(
        "{ap}\nƒ f() ! ∅ {{ μx ≔ 1; r ≔ ap(ident, &x); use_i(*r); x ← 2; }}"
    ));
}
