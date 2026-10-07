//! Soundness hunting (ADR-023 §10c): chương trình TẤN CÔNG — thử qua mặt borrowck bằng alias qua reborrow/generic/match/vòng
//! lặp/Arena/move một phần/struct giữ tham chiếu/lời gọi theo `RegionSig`. Mỗi ca là một test VĨNH VIỄN:
//! `attack` = borrowck PHẢI chặn (mã mong đợi) VÀ — khi chạy được — oracle động phải thấy vi phạm;
//! `fine` = hợp lệ: borrowck PHẢI chấp nhận VÀ oracle phải sạch.
//! Hai phía bất đồng ở `attack` = lỗ hổng (âm tính giả) hoặc oracle yếu; ở `fine` = dương tính giả hoặc oracle sai.
mod common;
mod support;
use common::*;
use support::oracle::{Oracle, Outcome, V};
use support::progen::PRELUDE;

fn eval(src: &str) -> (Vec<String>, Option<Outcome>) {
    let t = one(src);
    assert!(t.report.diagnostics.is_empty(), "M4: {:?}\n{src}", show(&t));
    let m = gt_mir::lower_program(&t);
    assert!(m.report.diagnostics.is_empty(), "M5 báo lỗi\n{src}");
    let r = gt_borrowck::check_program(&t, &m);
    let mut bc: Vec<String> = r
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    bc.sort();
    bc.dedup();
    // chạy `entry` nếu mọi tham số là bool
    let run = m.body("main::entry").and_then(|b| {
        if (1..=b.arg_count).any(|i| !matches!(b.locals[i].ty, gt_types::Ty::Bool))
            || b.arg_count > 3
        {
            return None;
        }
        let n = b.arg_count;
        let mut worst = None;
        for mask in 0..(1u32 << n) {
            let args: Vec<V> = (0..n).map(|i| V::Bool((mask >> i) & 1 == 1)).collect();
            let o = Oracle::new(&m, 200_000).call_key("main::entry", args);
            if matches!(o, Outcome::Violation(_) | Outcome::Bug(_)) {
                return Some(o);
            }
            worst = Some(o);
        }
        worst
    });
    (bc, run)
}

/// `body` là thân của `entry(c: bool, d: bool)`; PRELUDE có sẵn.
fn program(extra_items: &str, body: &str) -> String {
    format!(
        "{PRELUDE}{extra_items}\nƒ entry(c: bool, d: bool) → i32 ! {{mem}} {{\n{body}\n  0\n}}\n"
    )
}

#[track_caller]
fn attack(name: &str, extra: &str, body: &str, code: &str) {
    let src = program(extra, body);
    let (bc, run) = eval(&src);
    assert!(
        bc.iter().any(|c| c == code),
        "[{name}] borrowck phải chặn bằng {code}, nhận {bc:?}\n{src}"
    );
    match run {
        Some(Outcome::Violation(_)) => {}
        Some(Outcome::Bug(m)) => panic!("[{name}] oracle Bug: {m}"),
        Some(o) => panic!(
            "[{name}] oracle không thấy vi phạm ({o:?}) — oracle yếu hoặc ca không tới được\n{src}"
        ),
        None => {}
    }
}

/// Borrowck PHẢI chặn; oracle KHÔNG thấy (dự kiến) vì vi phạm nằm ở chữ ký chứ không ở thân thật — nhưng oracle không được `Bug`.
#[track_caller]
fn attack_sig(name: &str, extra: &str, body: &str, code: &str) {
    let src = program(extra, body);
    let (bc, run) = eval(&src);
    assert!(
        bc.iter().any(|c| c == code),
        "[{name}] borrowck phải chặn bằng {code}, nhận {bc:?}
{src}"
    );
    assert!(!matches!(run, Some(Outcome::Bug(_))), "[{name}] oracle Bug");
}

/// Chỉ kiểm borrowck (ca không chạy được: tham số tham chiếu, v.v.).
#[track_caller]
fn attack_static(name: &str, extra: &str, items: &str, code: &str) {
    let src = format!("{PRELUDE}{extra}\n{items}\n");
    let (bc, _) = eval(&src);
    assert!(
        bc.iter().any(|c| c == code),
        "[{name}] borrowck phải chặn bằng {code}, nhận {bc:?}\n{src}"
    );
}

#[track_caller]
fn fine(name: &str, extra: &str, body: &str) {
    let src = program(extra, body);
    let (bc, run) = eval(&src);
    assert!(bc.is_empty(), "[{name}] dương tính giả: {bc:?}\n{src}");
    if let Some(o) = run {
        assert!(
            !matches!(o, Outcome::Violation(_) | Outcome::Bug(_)),
            "[{name}] oracle: {o:?}\n{src}"
        );
    }
}

// ----------------------------------------------------------------------------- alias qua reborrow

#[test]
fn alias_through_reborrow_chains() {
    attack(
        "reborrow ghi gốc rồi dùng con",
        "",
        "μx ≔ 1; a ≔ &!x; b ≔ &!*a; *a ← 2; *b ← 3;",
        "E0406",
    );
    attack(
        "hai reborrow rồi dùng cả hai",
        "",
        "μx ≔ 1; a ≔ &!x; b ≔ &!*a; k2 ≔ &!*a; *b ← 1; *k2 ← 2;",
        "E0402",
    );
    attack(
        "reborrow chung rồi ghi qua gốc",
        "",
        "μx ≔ 1; a ≔ &!x; b ≔ &*a; *a ← 2; use_i(*b);",
        "E0406",
    );
    attack(
        "chuỗi ba tầng, ghi tầng giữa",
        "",
        "μx ≔ 1; a ≔ &!x; b ≔ &!*a; k2 ≔ &!*b; *b ← 9; *k2 ← 1;",
        "E0406",
    );
    attack(
        "dùng gốc khi con &! còn sống",
        "",
        "μx ≔ 1; a ≔ &!x; b ≔ &!*a; use_i(*a); *b ← 1;",
        "E0403",
    );
    fine(
        "con rồi gốc nối tiếp",
        "",
        "μx ≔ 1; a ≔ &!x; b ≔ &!*a; *b ← 1; *a ← 2; use_i(x);",
    );
    fine(
        "reborrow chung nhiều con",
        "",
        "μx ≔ 1; a ≔ &!x; b ≔ &*a; k2 ≔ &*a; use_i(*b +% *k2); *a ← 2;",
    );
}

#[test]
fn alias_through_function_calls_and_returned_references() {
    attack(
        "kết quả idr giữ khoản vay",
        "",
        "μx ≔ 1; r ≔ idr(&x); x ← 2; use_i(*r);",
        "E0406",
    );
    // chữ ký `pick<'a>(x: &'a, y: &'a)` buộc kết quả với CẢ HAI đối số; thân thật chỉ trả `x` nên oracle không thấy ⇒ chỉ kiểm borrowck
    attack_sig(
        "pick giữ cả hai khoản vay",
        "",
        "μx ≔ 1; μy ≔ 2; r ≔ pick(&x, &y); y ← 3; use_i(*r);",
        "E0406",
    );
    attack(
        "hai &! cùng biến trong một lời gọi",
        "",
        "μx ≔ 1; two(&!x, &!x);",
        "E0402",
    );
    attack("mix &x và &!x", "", "μx ≔ 1; mix(&x, &!x);", "E0403");
    attack(
        "trả tham chiếu tới local qua hàm",
        "ƒ leak() → &i32 ! ∅ { x ≔ 1; idr(&x) }",
        "use_i(*leak());",
        "E0404",
    );
    attack(
        "trả tham chiếu tới tham số theo giá trị",
        "ƒ leak2(x: i32) → &i32 ! ∅ { &x }",
        "use_i(*leak2(1));",
        "E0404",
    );
    attack(
        "trả khoản vay từ thân lồng",
        "ƒ leak3() → &i32 ! ∅ { { y ≔ 2; idr(&y) } }",
        "use_i(*leak3());",
        "E0404",
    );
    fine(
        "pick rồi dùng hết trước khi ghi",
        "",
        "μx ≔ 1; μy ≔ 2; r ≔ pick(&x, &y); use_i(*r); y ← 3; x ← 4;",
    );
    fine(
        "pick2 chỉ buộc đối số đầu",
        "",
        "μx ≔ 1; μy ≔ 2; r ≔ pick2(&x, &y); y ← 3; use_i(*r);",
    );
}

// ----------------------------------------------------------------------------- generic, match, vòng lặp

#[test]
fn alias_through_generics() {
    let id = "ƒ idg2<T>(x: T) → T ! ∅ { x }";
    attack(
        "generic trả lại tham chiếu mượn",
        id,
        "μx ≔ 1; r ≔ idg2(&x); x ← 2; use_i(*r);",
        "E0406",
    );
    attack(
        "generic với &!",
        id,
        "μx ≔ 1; r ≔ idg2(&!x); use_i(x); *r ← 1;",
        "E0403",
    );
    attack(
        "generic move rồi dùng",
        id,
        "b ≔ mk(); k2 ≔ idg2(b); use_i(b.v);",
        "E0401",
    );
    fine(
        "generic với tham chiếu dùng xong",
        id,
        "μx ≔ 1; r ≔ idg2(&x); use_i(*r); x ← 2;",
    );
    let wrap = "▢ Wr<T> { t: T }\nƒ wr<T>(x: T) → Wr<T> ! ∅ { Wr { t: x } }";
    attack(
        "bọc tham chiếu trong generic struct",
        wrap,
        "μx ≔ 1; w ≔ wr(&x); x ← 2; use_i(*w.t);",
        "E0406",
    );
    fine(
        "bọc rồi dùng xong",
        wrap,
        "μx ≔ 1; w ≔ wr(&x); use_i(*w.t); x ← 2;",
    );
}

#[test]
fn alias_through_match_and_partial_moves() {
    attack(
        "match move payload rồi dùng lại Option",
        "",
        "μo ≔ ⎇ c { Some(mk()) } ⇝ { None }; ⌥ o { Some(p) ↦ { eat(p); }, None ↦ { } }; ⌥ o { Some(q) ↦ { use_i(q.v); }, None ↦ { } };",
        "E0401",
    );
    attack(
        "mượn payload rồi move cả Option",
        "",
        "μo ≔ ⎇ c { Some(mk()) } ⇝ { None }; ⌥ o { Some(p) ↦ { r ≔ &p; eat(p); use_i(r.v); }, None ↦ { } };",
        "E0406",
    );
    attack(
        "move trường rồi dùng cả struct",
        "",
        "μw ≔ mkw(); x ≔ w.b; eatw(w);",
        "E0401",
    );
    attack(
        "move trường rồi mượn trường đó",
        "",
        "μw ≔ mkw(); x ≔ w.b; r ≔ &w.b; use_i(r.v);",
        "E0401",
    );
    attack(
        "mượn trường rồi move cả struct",
        "",
        "μw ≔ mkw(); r ≔ &w.n; eatw(w); use_i(*r);",
        "E0406",
    );
    attack(
        "tuple: move một phần rồi dùng cả",
        "ƒ pair(t: (B, B)) ! ∅ { }",
        "t ≔ (mk(), mk()); x ≔ t.0; pair(t);",
        "E0401",
    );
    fine(
        "move trường rồi dùng trường khác",
        "",
        "μw ≔ mkw(); x ≔ w.b; use_i(w.n); eat(x);",
    );
    fine(
        "match mượn payload Copy",
        "",
        "o ≔ ⎇ c { Some(5) } ⇝ { None }; ⌥ o { Some(p) ↦ { r ≔ &p; use_i(*r); }, None ↦ { } };",
    );
}

#[test]
fn alias_across_loop_back_edges() {
    attack(
        "giữ &! qua vòng lặp, mượn lại trong vòng",
        "",
        "μx ≔ 0; r ≔ &!x; ⟳ i in 0..2 { w ≔ &!x; *w ← 1; } *r ← 2;",
        "E0402",
    );
    attack(
        "move trong vòng lặp",
        "",
        "b ≔ mk(); ⟳ i in 0..2 { eat(b); }",
        "E0401",
    );
    attack(
        "tham chiếu gán ở vòng trước, ghi gốc vòng sau",
        "",
        "μx ≔ 0; μr ≔ &x; ⟳ i in 0..2 { x ← i; use_i(*r); }",
        "E0406",
    );
    attack(
        "khoản vay sống qua cạnh lùi",
        "",
        "μx ≔ 0; r ≔ &x; ⟳ i in 0..3 { use_i(*r); x ← 1; }",
        "E0406",
    );
    attack(
        "gán ref trong vòng tới biến của vòng",
        "",
        "μz ≔ 0; μp ≔ &z; ⟳ i in 0..2 { y ≔ i; p ← &y; } use_i(*p);",
        "E0404",
    );
    fine(
        "mượn mới mỗi vòng",
        "",
        "μx ≔ 0; ⟳ i in 0..3 { w ≔ &!x; *w ← 1; } use_i(x);",
    );
    fine(
        "move rồi gán lại trong vòng",
        "",
        "μb ≔ mk(); ⟳ i in 0..3 { eat(b); b ← mk(); } eat(b);",
    );
}

// ----------------------------------------------------------------------------- Arena

#[test]
fn arena_versions_are_invalidated_by_every_operation() {
    attack(
        "reset",
        "",
        "μa ≔ Arena::new(); p ≔ a.alloc(1); a.reset(); use_i(*p);",
        "E0405",
    );
    attack(
        "move Arena",
        "",
        "a ≔ Arena::new(); p ≔ a.alloc(1); eata(a); use_i(*p);",
        "E0405",
    );
    attack(
        "gán đè Arena",
        "",
        "μa ≔ Arena::new(); p ≔ a.alloc(1); a ← Arena::new(); use_i(*p);",
        "E0405",
    );
    attack(
        "ref thoát phạm vi Arena",
        "",
        "μp ≔ &0; { a ≔ Arena::new(); p ← a.alloc(1); }; use_i(*p);",
        "E0405",
    );
    attack(
        "ref thoát qua trả về",
        "ƒ esc() → &i32 ! {mem} { a ≔ Arena::new(); a.alloc(1) }",
        "use_i(*esc());",
        "E0405",
    );
    attack(
        "reset trong nhánh, dùng sau nhánh",
        "",
        "μa ≔ Arena::new(); p ≔ a.alloc(1); ⎇ c { a.reset(); } use_i(*p);",
        "E0405",
    );
    attack(
        "reset trong vòng, ref giữ ngoài",
        "",
        "μa ≔ Arena::new(); p ≔ a.alloc(1); ⟳ i in 0..2 { a.reset(); } use_i(*p);",
        "E0405",
    );
    attack(
        "ref trong struct rồi reset",
        "",
        "μa ≔ Arena::new(); h ≔ Hs { r: a.alloc(1) }; a.reset(); use_i(*h.r);",
        "E0405",
    );
    attack(
        "reset qua &!Arena tham số",
        "ƒ rs(a: &!Arena) ! {mem} { a.reset(); }",
        "μa ≔ Arena::new(); p ≔ a.alloc(1); rs(&!a); use_i(*p);",
        "E0405",
    );
    fine(
        "dùng xong rồi reset",
        "",
        "μa ≔ Arena::new(); p ≔ a.alloc(1); use_i(*p); a.reset(); q ≔ a.alloc(2); use_i(*q);",
    );
    fine(
        "Arena tham số trả ref",
        "ƒ okk<'r>(a: &'r Arena) → &'r i32 ! {mem} { a.alloc(1) }",
        "a ≔ Arena::new(); use_i(*okk(&a));",
    );
}

// ----------------------------------------------------------------------------- struct giữ tham chiếu, region trong ADT

#[test]
fn invariance_survives_adt_construction() {
    // ADT giữ `&!` của một tham chiếu: DỰNG ADT từ `&!p` phải đồng nhất region trong (bất biến), nếu không ghi một tham chiếu
    // ngắn qua ADT làm `p` treo (lỗ hổng tìm thấy khi đột biến `adt_invariant` sống sót)
    let sl = "▢ SlT<'a> { s: &!'a &'a i32 }
ƒ putt2<'a>(sl: SlT<'a>, v: &'a i32) ! ∅ { *sl.s ← v; }";
    attack(
        "dựng SlT từ &!p rồi ghi tham chiếu ngắn",
        sl,
        "z ≔ 1; μp ≔ &z; { y ≔ 7; sl ≔ SlT { s: &!p }; putt2(sl, &y); }; use_i(*p);",
        "E0404",
    );
    fine(
        "dựng SlT, không dùng p sau đó (Rust cũng từ chối dùng p: &!'a &'a ghim p suốt 'a)",
        sl,
        "z ≔ 1; y ≔ 7; μp ≔ &z; sl ≔ SlT { s: &!p }; putt2(sl, &y);",
    );
}

#[test]
fn references_stored_in_structs_and_collections() {
    attack(
        "Hs giữ & rồi ghi gốc",
        "",
        "μx ≔ 1; h ≔ Hs { r: &x }; x ← 2; use_i(*h.r);",
        "E0406",
    );
    attack(
        "Hs trong tuple",
        "",
        "μx ≔ 1; t ≔ (Hs { r: &x }, 1); x ← 2; use_i(*t.0.r);",
        "E0406",
    );
    attack(
        "Option giữ &",
        "",
        "μx ≔ 1; o ≔ Some(&x); x ← 2; ⌥ o { Some(r) ↦ { use_i(*r); }, None ↦ { } };",
        "E0406",
    );
    attack(
        "mảng giữ &",
        "",
        "μx ≔ 1; a ≔ [&x]; x ← 2; use_i(*a[0]);",
        "E0406",
    );
    attack(
        "ghi tham chiếu local vào holder ngoài",
        "",
        "μz ≔ 0; μh ≔ Hs { r: &z }; { y ≔ 5; h ← Hs { r: &y }; }; use_i(*h.r);",
        "E0404",
    );
    attack(
        "Hs trả về giữ tham chiếu local",
        "ƒ lk() → Hs ! ∅ { x ≔ 1; Hs { r: &x } }",
        "use_i(*lk().r);",
        "E0404",
    );
    fine(
        "Hs dùng xong rồi ghi",
        "",
        "μx ≔ 1; h ≔ Hs { r: &x }; use_i(*h.r); x ← 2;",
    );
    fine(
        "Hs của tham số",
        "ƒ gt<'a>(h: Hs<'a>) → &'a i32 ! ∅ { h.r }",
        "μx ≔ 1; r ≔ gt(Hs { r: &x }); use_i(*r);",
    );
}

// ----------------------------------------------------------------------------- move ra khỏi tham chiếu / nguồn alias khác

#[test]
fn moves_out_of_places_that_cannot_be_moved_from() {
    attack_static("move qua &", "", "ƒ f(r: &B) → B ! ∅ { *r }", "E0407");
    attack_static("move qua &!", "", "ƒ f(r: &!B) ! ∅ { eat(*r); }", "E0407");
    attack_static(
        "move trường qua &",
        "",
        "ƒ f(r: &W) → B ! ∅ { r.b }",
        "E0407",
    );
    attack_static(
        "move phần tử mảng",
        "",
        "ƒ f(a: [B; 2]) ! ∅ { eat(a[0]); }",
        "E0407",
    );
    attack_static(
        "move phần tử lát",
        "",
        "ƒ f(s: &[B]) ! ∅ { eat(s[0]); }",
        "E0407",
    );
    attack_static(
        "move trong match qua &",
        "",
        "ƒ f(o: &Option<B>) ! ∅ { ⌥ *o { Some(b) ↦ eat(b), None ↦ { } } }",
        "E0407",
    );
    attack_static(
        "move trường khỏi kiểu Drop",
        "",
        "ƒ f(g: Guard) → Buf ! ∅ { g.inner }",
        "E0407",
    );
}

#[test]
fn unique_references_cannot_be_duplicated_by_copying_or_moving_around() {
    attack(
        "&! sao chép bằng gán rồi dùng cả hai",
        "",
        "μx ≔ 1; a ≔ &!x; b ≔ a; *b ← 1; *a ← 2;",
        "E0401",
    );
    fine(
        "&! truyền reborrow ngầm hai lần",
        "ƒ takes2(r: &!i32) ! ∅ { *r ← 1; }",
        "μx ≔ 1; a ≔ &!x; takes2(a); takes2(a); *a ← 2;",
    );
}
