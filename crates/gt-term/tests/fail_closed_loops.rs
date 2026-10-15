//! Rà soát FAIL-OPEN của gt-term (lệnh Giang 2026-10-15, mục B): một vòng lặp KHÔNG có nghĩa vụ dừng nào được xét (tập nghĩa vụ RỖNG hoặc BỊ CẮT vì vượt
//! `GHOST_MAX_VARS = 8` biến bóng của một thân) KHÔNG BAO GIỜ được coi là "đã chứng minh dừng". Mỗi vòng sai phải bị E0701/E0702 đúng mã; vòng đúng phải qua.
#[path = "../../gt-mir/tests/common/mod.rs"]
mod common;
use gt_oblig::driver::build_prog_info;
use gt_oblig::prover::ZoneProver;
use gt_term::check_termination;

fn codes(src: &str) -> Vec<String> {
    let (t, m) = common::mir(src);
    assert!(
        m.report.diagnostics.is_empty(),
        "{:?}\n{src}",
        common::mir_codes(&m)
    );
    let info = build_prog_info(&t, &m);
    let o = check_termination(&t, &m, &info, &mut ZoneProver::new(24));
    let mut v: Vec<String> = o
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect();
    v.sort();
    v
}

fn exactly(src: &str, want: &[&str]) {
    let c = codes(src);
    let w: Vec<String> = want.iter().map(|x| (*x).to_string()).collect();
    assert_eq!(c, w, "\n{src}");
}

/// hàm có `n` vòng lặp: vòng `k` dùng biến RIÊNG `ik`/`nk` (mỗi vòng cần 2 biến bóng ⇒ vượt 8 sau 4 vòng). `bad[k]` ⇒ vòng đứng yên (sai).
fn many_loops(n: usize, bad: &[usize], explicit: bool) -> String {
    let params: Vec<String> = (1..=n).map(|k| format!("n{k}: u32")).collect();
    let mut body = String::new();
    for k in 1..=n {
        let step = if bad.contains(&k) {
            String::new()
        } else {
            format!("i{k} ← i{k} + 1;")
        };
        if explicit {
            body.push_str(&format!(
                "  μi{k} ≔ 0u32; ⟳ i{k} < n{k} ↓ n{k} - i{k} {{ {step} }}\n"
            ));
        } else {
            body.push_str(&format!("  μi{k} ≔ 0u32; ⟳ i{k} < n{k} {{ {step} }}\n"));
        }
    }
    format!("ƒ f({}) → () ! ∅ {{\n{body}}}\n", params.join(", "))
}

#[test]
fn many_valid_loops_with_distinct_variables_are_accepted() {
    // đối chứng pass: 3, 6, 10 vòng đúng, mỗi vòng biến riêng (kể cả khi vượt giới hạn biến bóng của một thân)
    for n in [3, 6, 10] {
        exactly(&many_loops(n, &[], true), &[]);
        exactly(&many_loops(n, &[], false), &[]);
    }
}

#[test]
fn stationary_variant_loops_past_the_ghost_limit_are_rejected_with_e0702() {
    // vòng 5, 6 (vượt 8 biến bóng) đứng yên ⇒ E0702 cho từng vòng sai; trước đây vòng bị cắt nghĩa vụ được chấp nhận
    exactly(&many_loops(6, &[5, 6], true), &["E0702", "E0702"]);
    exactly(&many_loops(8, &[8], true), &["E0702"]);
    exactly(
        &many_loops(10, &[1, 9, 10], true),
        &["E0702", "E0702", "E0702"],
    );
}

#[test]
fn inferred_variant_loops_without_progress_past_the_ghost_limit_are_rejected_with_e0701() {
    exactly(&many_loops(6, &[5, 6], false), &["E0701", "E0701"]);
    exactly(&many_loops(9, &[9], false), &["E0701"]);
}

#[test]
fn a_variant_that_decreases_but_is_overwritten_in_the_body_is_rejected() {
    // giảm rồi bị ghi đè ngược trong thân
    exactly(
        "ƒ f(n: u32) → () ! ∅ { μi ≔ 0u32; μm ≔ n; ⟳ i < m ↓ m - i { i ← i + 1; m ← m + 5; } }",
        &["E0702"],
    );
}

#[test]
fn nested_loops_each_need_their_own_proof() {
    // vòng trong đúng, vòng ngoài đứng yên
    exactly(
        "ƒ f(n: u32) → () ! ∅ { μi ≔ 0u32; ⟳ i < n ↓ n - i { μj ≔ 0u32; ⟳ j < 3 { j ← j + 1; } } }",
        &["E0702"],
    );
    // vòng ngoài đúng, vòng trong đứng yên: vòng trong E0702. Sau vòng trong KHÔNG tới được (vòng trong không thoát) nên M7 KHẲNG ĐỊNH vòng ngoài không có cạnh lùi
    // khả thi (chạy tối đa một lượt) ⇒ vòng ngoài không báo thêm; cả chương trình vẫn bị từ chối vì vòng trong
    exactly(
        "ƒ f(n: u32) → () ! ∅ { μi ≔ 0u32; ⟳ i < n { μj ≔ 0u32; ⟳ j < 3 ↓ 3 - j { } i ← i + 1; } }",
        &["E0702"],
    );
    // cả hai đúng
    exactly(
        "ƒ f(n: u32) → () ! ∅ { μi ≔ 0u32; ⟳ i < n ↓ n - i { μj ≔ 0u32; ⟳ j < 3 ↓ 3 - j { j ← j + 1; } i ← i + 1; } }",
        &[],
    );
}

#[test]
fn mutual_recursion_with_one_decreasing_edge_is_rejected() {
    // chỉ một cạnh của chu trình giảm biến thể
    exactly(
        "ƒ a(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { b(n - 1) } }\nƒ b(n: u32) → u32 ! ∅ ↓ n { ⎇ n == 0 { 0 } ⇝ { a(n) } }",
        &["E0702"],
    );
}

#[test]
fn beyond_the_ghost_limit_valid_loops_are_rejected_not_silently_accepted() {
    // 20 vòng đúng = 40 biến của biến thể > 24 (ZONE_MAX_VARS = GHOST_MAX_VARS): các vòng thừa KHÔNG có bóng ⇒ biến thể không chứng minh được ⇒ E0702 (chặn nhầm, fail-closed);
    // KHÔNG được chấp nhận trong im lặng. Ghi ở KNOWN_LIMITATIONS: tối đa 24 biến của biến thể mỗi thân (12 vòng có biến thể 2 biến).
    let c = codes(&many_loops(20, &[], true));
    assert!(
        !c.is_empty() && c.iter().all(|x| x == "E0702"),
        "kỳ vọng chỉ E0702 (chặn nhầm có kiểm soát), nhận {c:?}"
    );
    // và vòng SAI vẫn bị bắt dù nằm ngoài giới hạn
    let c = codes(&many_loops(20, &[20], true));
    assert!(c.len() >= 2 && c.iter().all(|x| x == "E0702"), "{c:?}");
}

#[test]
fn a_self_looping_head_block_counts_as_a_back_edge() {
    // vòng vô hạn có `↓ n` đứng yên, thân rỗng (khối đầu có thể tự quay lại chính nó: rpo_pos[tgt] == rpo_pos[nguồn]):
    // KHÔNG được coi là "không có cạnh lùi khả thi" (đột biến `<` thay `<=` làm chấp nhận vòng vô hạn)
    for body in ["{ }", "{ μk ≔ 0u32; }"] {
        let c = codes(&format!("ƒ f(n: u32) → () ! {{div}} {{ ⟳ ↓ n {body} }}"));
        assert!(
            c.iter().any(|x| x == "E0702" || x == "E0701"),
            "vòng vô hạn đứng yên bị CHẤP NHẬN ({c:?}) với thân {body}"
        );
    }
}

/// Khối nhảy về CHÍNH NÓ (vòng một khối) CÓ tồn tại (`⟳ { }`): vì vậy tiêu chí cạnh lùi của `no_back_edge_heads` phải dùng `<=` (không phải `<`).
/// Với vòng lặp có biến thể mà đầu vòng tự quay về chính nó, KHÔNG được coi là "không có cạnh lùi".
#[test]
fn single_block_loops_exist_and_keep_their_back_edge() {
    let mut selfloops = Vec::new();
    for src in [
        "ƒ f() → () ! {div} { ⟳ { } }",
        "ƒ f(n: u32) → () ! {div} { ⟳ ↓ n { } }",
        "ƒ f(n: u32) → () ! {div} { ⟳ ↓ n { μk ≔ 0u32; } }",
    ] {
        let (_, m) = common::mir(src);
        for b in &m.bodies {
            for (i, bb) in b.blocks.iter().enumerate() {
                if let Some(t) = &bb.terminator
                    && t.kind.successors().iter().any(|s| s.index() == i)
                {
                    selfloops.push((src, i));
                }
            }
        }
    }
    println!("vòng một khối: {selfloops:?}");
    // chương trình vô hạn có `↓ n` đứng yên: dù đầu vòng tự quay về nó, PHẢI bị E0702/E0701
    for src in [
        "ƒ f(n: u32) → () ! {div} { ⟳ ↓ n { } }",
        "ƒ f(n: u32) → () ! {div} { ⟳ ↓ n { μk ≔ 0u32; } }",
    ] {
        let c = codes(src);
        assert!(
            c.iter().any(|x| x == "E0702" || x == "E0701"),
            "vòng vô hạn đứng yên bị CHẤP NHẬN ({c:?}): {src}"
        );
    }
}

/// hàm có ĐÚNG số biến của biến thể: `pairs` vòng hai biến (`↓ nK - iK`), thêm (tuỳ chọn) một vòng MỘT biến (`↓ m`)
fn exact_variant_vars(pairs: usize, extra_single_var_loop: bool) -> String {
    let mut params: Vec<String> = (1..=pairs).map(|k| format!("n{k}: u32")).collect();
    let mut body = String::new();
    for k in 1..=pairs {
        body.push_str(&format!(
            "  μi{k} ≔ 0u32; ⟳ i{k} < n{k} ↓ n{k} - i{k} {{ i{k} ← i{k} + 1; }}
"
        ));
    }
    if extra_single_var_loop {
        params.push("m0: u32".to_string());
        body.push_str(
            "  μm ≔ m0; ⟳ m > 0 ↓ m { m ← m - 1; }
",
        );
    }
    format!(
        "ƒ f({}) → () ! ∅ {{
{body}}}
",
        params.join(", ")
    )
}

/// GIỚI HẠN THẬT = `ZONE_MAX_VARS` = 24 biến của biến thể mỗi thân (12 vòng hai biến): biến thể cần quan hệ zone và chỉ 24 biến đầu (ưu tiên biến của biến thể) vào
/// zone; `GHOST_MAX_VARS` được đặt bằng 24 cho khớp. Đúng 24 ⇒ mọi vòng được chứng minh; 25 ⇒ vòng có biến thứ 25 bị E0702 (chặn nhầm CÓ KIỂM SOÁT), không bị
/// chấp nhận hay bỏ qua im lặng.
#[test]
fn exactly_24_variant_variables_are_handled_and_25_are_rejected_in_a_controlled_way() {
    let t = std::time::Instant::now();
    exactly(&exact_variant_vars(12, false), &[]); // 24 biến
    let t24 = t.elapsed();
    let t = std::time::Instant::now();
    let c = codes(&exact_variant_vars(12, true)); // 25 biến
    let t25 = t.elapsed();
    assert!(
        !c.is_empty() && c.iter().all(|x| x == "E0702"),
        "25 biến của biến thể phải bị E0702 có kiểm soát, nhận {c:?}"
    );
    // 33 biến (16 vòng hai biến + 1 vòng một biến): cũng bị từ chối, không bị chấp nhận
    let c = codes(&exact_variant_vars(16, true));
    assert!(!c.is_empty() && c.iter().all(|x| x == "E0702"), "{c:?}");
    println!("VARIANT_LIMIT_TIMING 24 biến: {t24:?}; 25 biến: {t25:?}");
}
