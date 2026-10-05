//! gt-resolve (M3) — E0201 (thiếu tên), E0202 (trùng), E0203 (private), E0204 (vòng module), E0205 (D7). Viết TRƯỚC (TDD).
mod common;
use common::*;

// ======================= E0201: tên chưa định nghĩa =======================

#[test]
fn e0201_undefined_names_in_every_position() {
    let cases = [
        "ƒ f() → i32 ! ∅ { y }",                                // biểu thức
        "ƒ f(x: Foo) ! ∅ { }",                                  // kiểu trong chữ ký
        "ƒ f() ! ∅ { P { x: 1 }; }",                            // struct literal
        "ƒ f() ! ∅ { g(); }",                                   // gọi hàm
        "ƒ f(o: i32) → i32 ! ∅ { ⌥ o { Nope(1) ↦ 1, _ ↦ 2 } }", // pattern đường dẫn
        "ƒ f() ! ∅ { x ← 1; }",                                 // đích gán
        "▢ P { a: Missing }",                                   // kiểu trong trường
        "⊔ E { A(Missing) }",                                   // kiểu trong biến thể
        "⊐ Missing { ƒ f(&self) ! ∅ { } }",                     // kiểu của impl
        "⊐ Missing for i32 { ƒ f(&self) ! ∅ { } }",             // trait của impl
        "ƒ f<T: Missing>(x: T) ! ∅ { }",                        // ràng buộc generics
        "□ C: i32 ≔ MISSING;",                                  // giá trị const
        "ƒ f() ! ∅ ⊳ nope > 0 { }",                             // hợp đồng
    ];
    for src in cases {
        assert_err(&[("main.gt", src)], "E0201", 1);
    }
}

#[test]
fn e0201_member_and_variant_not_found() {
    assert_err(
        &[("main.gt", "▦ m { ◈ ƒ a() ! ∅ { } } ƒ f() ! ∅ { m::b(); }")],
        "E0201",
        1,
    );
    // `Enum::tên` với tên không phải biến thể là mục kết hợp (impl/trait): M3 KHÔNG báo lỗi, ghi `Res::Assoc` cho M4 kiểm
    let out = one("⊔ E { A } ƒ f() ! ∅ { E::B; }");
    assert!(out.report.diagnostics.is_empty());
    assert!(
        out.uses
            .values()
            .any(|r| matches!(r, gt_resolve::Res::Assoc { member, .. } if member == "B"))
    );
    // `f::x` với f là hàm: f không có thành viên
    assert_err(
        &[("main.gt", "ƒ g() ! ∅ { } ƒ f() ! ∅ { g::x; }")],
        "E0201",
        1,
    );
}

#[test]
fn e0201_one_error_per_use_and_no_cascade() {
    assert_err(&[("main.gt", "ƒ f() → i32 ! ∅ { y + y }")], "E0201", 2);
    // sau tên thiếu, các đoạn sau và biểu thức bao quanh không sinh lỗi dây chuyền
    assert_err(
        &[("main.gt", "ƒ f() ! ∅ { nope::a::b(1, nope2); }")],
        "E0201",
        2,
    );
}

#[test]
fn e0201_self_type_and_self_value_only_where_they_exist() {
    assert_err(&[("main.gt", "ƒ f() → Self ! ∅ { }")], "E0201", 1);
    assert_err(&[("main.gt", "ƒ f() → i32 ! ∅ { self.x }")], "E0201", 1);
    // `self` trong hàm KHÔNG có tham số self của impl: vẫn thiếu
    assert_err(
        &[(
            "main.gt",
            "▢ P { x: i32 } ⊐ P { ƒ f() → i32 ! ∅ { self.x } }",
        )],
        "E0201",
        1,
    );
    // `Self` dùng được trong cả ⊏ lẫn ⊐
    assert_ok(&[(
        "main.gt",
        "⊏ T { ƒ me(&self) → Self ! ∅; } ▢ P { x: i32 } ⊐ P { ƒ new() → Self ! ∅ { Self { x: 0 } } ƒ get(&self) → i32 ! ∅ { self.x } }",
    )]);
}

#[test]
fn e0201_use_before_definition_in_a_block_but_not_across_items() {
    assert_err(
        &[("main.gt", "ƒ f() → i32 ! ∅ { y ≔ x; x ≔ 1; y }")],
        "E0201",
        1,
    );
    // thứ tự khai báo ITEM không quan trọng
    assert_ok(&[("main.gt", "ƒ f() → i32 ! ∅ { g() } ƒ g() → i32 ! ∅ { 1 }")]);
    assert_ok(&[("main.gt", "ƒ f(p: P) ! ∅ { } ▢ P { x: Q } ▢ Q { y: i32 }")]);
}

#[test]
fn e0201_names_are_not_inherited_from_the_parent_module() {
    assert_err(
        &[("main.gt", "ƒ g() ! ∅ { } ▦ m { ƒ h() ! ∅ { g(); } }")],
        "E0201",
        1,
    );
    // đường dẫn tuyệt đối từ tên tệp + nhập thì được
    assert_ok(&[(
        "main.gt",
        "◈ ƒ g() ! ∅ { } ▦ m { ⇲ main::g; ƒ h() ! ∅ { g(); } }",
    )]);
}

#[test]
fn e0201_import_of_missing_name_or_module() {
    assert_err(&[("main.gt", "⇲ nope::x;")], "E0201", 1);
    assert_err(
        &[("a.gt", "◈ ƒ f() ! ∅ { }"), ("b.gt", "⇲ a::g;")],
        "E0201",
        1,
    );
    assert_err(
        &[("a.gt", "◈ ƒ f() ! ∅ { }"), ("b.gt", "⇲ a::{f, g};")],
        "E0201",
        1,
    );
}

#[test]
fn name_resolution_positive_cases() {
    // prelude: tên trần, biến thể, module con của prelude
    assert_ok(&[(
        "main.gt",
        "ƒ f(o: Option<i32>) → i32 ! ∅ { ⌥ o { Some(v) ↦ v, None ↦ 0 } } ƒ g() ! ∅ { core::abort(); } ƒ h() → Option<i32> ! ∅ { Some(1) }",
    )]);
    // generics
    assert_ok(&[("main.gt", "ƒ id<T>(x: T) → T ! ∅ { x } ⊔ W<T> { A(T) }")]);
    // biến thể: qua enum, trần trong module định nghĩa enum (Q19), và qua nhập từ module khác
    assert_ok(&[(
        "main.gt",
        "⊔ E { A, B(i32) } ƒ f() → E ! ∅ { E::B(1) } ƒ g() → E ! ∅ { A }",
    )]);
    assert_ok(&[
        ("a.gt", "◈ ⊔ E { A, B(i32) }"),
        (
            "b.gt",
            "⇲ a::E::A; ⇲ a::B; ƒ g() → a::E ! ∅ { A } ƒ h() → a::E ! ∅ { B(1) }",
        ),
    ]);
    // nhập biến thể vào CHÍNH module định nghĩa enum là trùng tên (biến thể đã là tên trần ở đó)
    assert_err(&[("main.gt", "⊔ E { A } ⇲ main::E::A;")], "E0202", 1);
    // tên module-tệp thấy được ở mọi tệp
    assert_ok(&[
        ("a.gt", "◈ ƒ fa() → i32 ! ∅ { 1 }"),
        ("b.gt", "ƒ fb() → i32 ! ∅ { a::fa() }"),
    ]);
    // nhập nhóm
    assert_ok(&[
        ("a.gt", "◈ ƒ f() ! ∅ { } ◈ ƒ g() ! ∅ { }"),
        ("b.gt", "⇲ a::{f, g}; ƒ h() ! ∅ { f(); g(); }"),
    ]);
}

// ======================= E0202: định nghĩa trùng =======================

#[test]
fn e0202_duplicate_definitions_one_diagnostic_per_name_with_all_sites() {
    // 2 lần và 3 lần: MỘT chẩn đoán cho mỗi (module, tên)
    let out = assert_err(&[("main.gt", "ƒ f() ! ∅ { } ƒ f() ! ∅ { }")], "E0202", 1);
    assert_eq!(out.report.diagnostics[0].labels.len(), 1);
    let out = assert_err(
        &[("main.gt", "ƒ f() ! ∅ { } ƒ f() ! ∅ { } ƒ f() ! ∅ { }")],
        "E0202",
        1,
    );
    assert_eq!(out.report.diagnostics[0].labels.len(), 2);
    // thông điệp độc lập thứ tự: không nói "thứ hai"
    assert!(!out.report.diagnostics[0].message.contains("thứ hai"));
}

#[test]
fn e0202_all_kinds_share_one_namespace_per_module() {
    for src in [
        "▢ P { x: i32 } ƒ P() ! ∅ { }",
        "⊔ E { A } ▢ E { x: i32 }",
        "□ C: i32 ≔ 1; ■ C: i32 ≔ 2;",
        "≡ T ≔ i32; ▢ T { x: i32 }",
        "▦ m { } ƒ m() ! ∅ { }",
        "⊏ Tr { } ⊔ Tr { A }",
    ] {
        assert_err(&[("main.gt", src)], "E0202", 1);
    }
}

#[test]
fn e0202_import_clashes_with_item_or_other_import() {
    assert_err(
        &[
            ("a.gt", "◈ ƒ f() ! ∅ { }"),
            ("b.gt", "⇲ a::f; ƒ f() ! ∅ { }"),
        ],
        "E0202",
        1,
    );
    assert_err(
        &[
            ("a.gt", "◈ ƒ f() ! ∅ { }"),
            ("c.gt", "◈ ƒ f() ! ∅ { }"),
            ("b.gt", "⇲ a::f; ⇲ c::f;"),
        ],
        "E0202",
        1,
    );
}

#[test]
fn e0202_duplicates_inside_one_declaration() {
    for src in [
        "⊔ E { A, A }",
        "▢ P { x: i32, x: i32 }",
        "ƒ f(a: i32, a: i32) ! ∅ { }",
        "ƒ f<T, T>(x: T) ! ∅ { }",
        "ƒ f(p: (i32, i32)) ! ∅ { ⌥ p { (a, a) ↦ 1, _ ↦ 2 }; }",
        "⊏ Tr { ƒ m(&self) ! ∅; ƒ m(&self) ! ∅; }",
        "▢ P { x: i32 } ⊐ P { ƒ m(&self) ! ∅ { } ƒ m(&self) ! ∅ { } }",
    ] {
        assert_err(&[("main.gt", src)], "E0202", 1);
    }
}

#[test]
fn e0202_same_name_in_different_modules_is_fine() {
    assert_ok(&[
        ("a.gt", "ƒ f() ! ∅ { } ▦ m { ƒ f() ! ∅ { } }"),
        ("b.gt", "ƒ f() ! ∅ { }"),
    ]);
}

#[test]
fn e0202_duplicates_make_uses_unresolved_without_extra_errors() {
    // dùng một tên trùng: chỉ có MỘT E0202 (không thêm E0201/lỗi mơ hồ)
    assert_err(
        &[("main.gt", "ƒ f() ! ∅ { } ƒ f() ! ∅ { } ƒ g() ! ∅ { f(); }")],
        "E0202",
        1,
    );
}

// ======================= E0203: private =======================

#[test]
fn e0203_private_items_across_modules_and_files() {
    assert_err(
        &[(
            "main.gt",
            "▦ m { ƒ secret() → i32 ! ∅ { 1 } } ƒ f() → i32 ! ∅ { m::secret() }",
        )],
        "E0203",
        1,
    );
    let out = assert_err(
        &[
            ("a.gt", "ƒ secret() ! ∅ { }"),
            ("b.gt", "ƒ g() ! ∅ { a::secret(); }"),
        ],
        "E0203",
        1,
    );
    // nhãn trỏ tới định nghĩa Ở TỆP KHÁC (nhắc ADR-017a: nhãn có trường file)
    let d = &out.report.diagnostics[0];
    assert_eq!(d.file, "b.gt");
    assert_eq!(d.labels.len(), 1);
    assert_eq!(d.labels[0].file, "a.gt");
    // nhập một item private
    assert_err(
        &[("a.gt", "ƒ secret() ! ∅ { }"), ("b.gt", "⇲ a::secret;")],
        "E0203",
        1,
    );
}

#[test]
fn e0203_visibility_rules_positive() {
    // ◈ thấy ở mọi nơi
    assert_ok(&[
        ("a.gt", "◈ ƒ f() ! ∅ { }"),
        ("b.gt", "ƒ g() ! ∅ { a::f(); }"),
    ]);
    // private thấy ở module định nghĩa và module con
    assert_ok(&[(
        "a.gt",
        "▦ m { ƒ s() ! ∅ { } ▦ n { ⇲ a::m::s; ƒ t() ! ∅ { s(); } } ƒ u() ! ∅ { s(); } }",
    )]);
    // …nhưng không thấy ở module anh em
    assert_err(
        &[("a.gt", "▦ m { ƒ s() ! ∅ { } } ▦ n { ⇲ a::m::s; }")],
        "E0203",
        1,
    );
    // nhập private mặc định; ◈ ⇲ tái xuất khẩu
    assert_err(
        &[
            ("a.gt", "◈ ƒ f() ! ∅ { }"),
            ("b.gt", "⇲ a::f;"),
            ("c.gt", "ƒ g() ! ∅ { b::f(); }"),
        ],
        "E0203",
        1,
    );
    assert_ok(&[
        ("a.gt", "◈ ƒ f() ! ∅ { }"),
        ("b.gt", "◈ ⇲ a::f;"),
        ("c.gt", "ƒ g() ! ∅ { b::f(); }"),
    ]);
    // module con private với tệp khác
    assert_err(
        &[
            ("a.gt", "▦ m { ◈ ƒ f() ! ∅ { } }"),
            ("b.gt", "ƒ g() ! ∅ { a::m::f(); }"),
        ],
        "E0203",
        1,
    );
    // prelude: thành viên module con cần ◈
    assert_err(&[("main.gt", "ƒ g() ! ∅ { core::hidden(); }")], "E0203", 1);
}

// ======================= E0204: vòng phụ thuộc module =======================

#[test]
fn e0204_cycle_between_two_files_is_reported_once() {
    let out = assert_err(
        &[
            ("a.gt", "◈ ƒ fa() ! ∅ { b::fb(); }"),
            ("b.gt", "◈ ƒ fb() ! ∅ { a::fa(); }"),
        ],
        "E0204",
        1,
    );
    let d = &out.report.diagnostics[0];
    assert!(
        d.message.contains('a') && d.message.contains('b'),
        "{}",
        d.message
    );
    // nhãn ở cả hai tệp
    let mut files: Vec<&str> = d.labels.iter().map(|l| l.file.as_str()).collect();
    files.push(d.file.as_str());
    assert!(
        files.contains(&"a.gt") && files.contains(&"b.gt"),
        "{files:?}"
    );
}

#[test]
fn e0204_three_file_cycle_and_via_imports() {
    assert_err(
        &[
            ("a.gt", "◈ ƒ fa() ! ∅ { b::fb(); }"),
            ("b.gt", "◈ ƒ fb() ! ∅ { c::fc(); }"),
            ("c.gt", "◈ ƒ fc() ! ∅ { a::fa(); }"),
        ],
        "E0204",
        1,
    );
    assert_err(
        &[
            ("a.gt", "⇲ b::fb; ◈ ƒ fa() ! ∅ { }"),
            ("b.gt", "⇲ a::fa; ◈ ƒ fb() ! ∅ { }"),
        ],
        "E0204",
        1,
    );
    // hai vòng độc lập → hai chẩn đoán
    assert_err(
        &[
            ("a.gt", "◈ ƒ f() ! ∅ { b::f(); }"),
            ("b.gt", "◈ ƒ f() ! ∅ { a::f(); }"),
            ("c.gt", "◈ ƒ f() ! ∅ { d::f(); }"),
            ("d.gt", "◈ ƒ f() ! ∅ { c::f(); }"),
        ],
        "E0204",
        2,
    );
}

#[test]
fn e0204_acyclic_graphs_and_nesting_are_fine() {
    assert_ok(&[
        ("a.gt", "◈ ƒ fa() ! ∅ { }"),
        ("b.gt", "◈ ƒ fb() ! ∅ { a::fa(); }"),
        ("c.gt", "ƒ fc() ! ∅ { a::fa(); b::fb(); }"),
    ]);
    // cha → con và con → cha (đường dẫn tuyệt đối) không tạo vòng (ADR-017 mục 6)
    assert_ok(&[(
        "a.gt",
        "▦ m { ◈ ƒ x() ! ∅ { a::f(); } } ◈ ƒ f() ! ∅ { m::x(); }",
    )]);
    // tự tham chiếu / đệ quy trong cùng module
    assert_ok(&[("a.gt", "◈ ƒ f() ! ∅ { f(); }")]);
    // dùng prelude không tạo cạnh
    assert_ok(&[
        ("a.gt", "ƒ f() ! ∅ { core::abort(); }"),
        ("b.gt", "ƒ g() ! ∅ { core::abort(); }"),
    ]);
}

// ======================= E0205: cấm shadowing (D7) =======================

#[test]
fn e0205_shadowing_of_locals_and_params() {
    for src in [
        "ƒ f(x: i32) ! ∅ { x ≔ 2; }",                       // che tham số
        "ƒ f() ! ∅ { x ≔ 1; x ≔ 2; }",                      // cùng khối
        "ƒ f() ! ∅ { x ≔ 1; ⎇ true { x ≔ 2; } }",           // khối lồng
        "ƒ f(n: i32) ! ∅ { ⟳ n in 0..3 { } }",              // biến vòng lặp che tham số
        "ƒ f(o: i32) ! ∅ { x ≔ 1; ⌥ o { x ↦ 1, _ ↦ 2 }; }", // binding của pattern che biến
        "ƒ f() ! ∅ { x ≔ 1; ∀ x ∈ 0..3 : x ≥ 0; }",         // lượng từ che biến
        "ƒ f<T>(x: i32) ! ∅ { T ≔ 1; }",                    // che tham số generics
    ] {
        assert_err(&[("main.gt", src)], "E0205", 1);
    }
}

#[test]
fn e0205_local_shadowing_an_item_import_or_prelude_name() {
    assert_err(
        &[("main.gt", "ƒ g() ! ∅ { } ƒ f() ! ∅ { g ≔ 1; }")],
        "E0205",
        1,
    );
    assert_err(&[("main.gt", "ƒ f() ! ∅ { Option ≔ 1; }")], "E0205", 1);
    assert_err(&[("main.gt", "ƒ f(Result: i32) ! ∅ { }")], "E0205", 1);
    assert_err(
        &[
            ("a.gt", "◈ ƒ g() ! ∅ { }"),
            ("b.gt", "⇲ a::g; ƒ f() ! ∅ { g ≔ 1; }"),
        ],
        "E0205",
        1,
    );
}

#[test]
fn e0205_positive_cases() {
    // khối anh em dùng lại tên
    assert_ok(&[(
        "main.gt",
        "ƒ f(c: bool) ! ∅ { ⎇ c { x ≔ 1; } ⇝ { x ≔ 2; } }",
    )]);
    assert_ok(&[(
        "main.gt",
        "ƒ f() ! ∅ { ⎇ true { x ≔ 1; } ⎇ true { x ≔ 2; } }",
    )]);
    // hai hàm cùng tên tham số
    assert_ok(&[("main.gt", "ƒ f(x: i32) ! ∅ { } ƒ g(x: i32) ! ∅ { }")]);
    // tên trùng item ở MODULE KHÁC: không che
    assert_ok(&[
        ("a.gt", "◈ ƒ g() ! ∅ { }"),
        ("b.gt", "ƒ f() ! ∅ { g ≔ 1; }"),
    ]);
    // pattern ident là biến thể (không phải binding mới) → không che
    assert_ok(&[(
        "main.gt",
        "ƒ f(o: Option<i32>) → i32 ! ∅ { ⌥ o { None ↦ 0, Some(x) ↦ x } }",
    )]);
    // `result` ngầm trong ⊲ (Q18)
    assert_ok(&[("main.gt", "ƒ f(x: i32) → i32 ! ∅ ⊲ result > 0 { x }")]);
    assert_err(
        &[(
            "main.gt",
            "ƒ f(result: i32) → i32 ! ∅ ⊲ result > 0 { result }",
        )],
        "E0205",
        1,
    );
    // `result` ngoài ⊲ không tồn tại
    assert_err(
        &[("main.gt", "ƒ f(x: i32) → i32 ! ∅ ⊳ result > 0 { x }")],
        "E0201",
        1,
    );
}

#[test]
fn e0205_message_names_the_shadowed_binding_and_points_at_it() {
    let out = assert_err(&[("main.gt", "ƒ f(x: i32) ! ∅ { x ≔ 2; }")], "E0205", 1);
    let d = &out.report.diagnostics[0];
    assert!(d.message.contains('x'));
    assert_eq!(d.labels.len(), 1, "nhãn chỉ vào định nghĩa bị che");
}

// ======================= thứ tự phạm vi, biến thể, prelude (bổ sung sau đột biến) =======================

#[test]
fn binding_scopes_follow_evaluation_order() {
    // vế phải thấy phạm vi TRƯỚC khi biến mới được khai báo
    assert_err(&[("main.gt", "ƒ f() ! ∅ { x ≔ x; }")], "E0201", 1);
    // `⟳ x in iter`: iter được giải trước khi `x` vào phạm vi
    assert_err(&[("main.gt", "ƒ f() ! ∅ { ⟳ x in x { } }")], "E0201", 1);
    // lượng từ: miền được giải trước biến
    assert_err(&[("main.gt", "ƒ f() ! ∅ { ∀ x ∈ x : x ≥ 0; }")], "E0201", 1);
    // biến vòng lặp thấy được trong ↓, ⋈ và thân
    assert_ok(&[(
        "main.gt",
        "ƒ f(n: i32) ! ∅ { ⟳ i in 0..n ↓ n - i ⋈ i ≥ 0 { j ≔ i; } }",
    )]);
    // binding của một nhánh match KHÔNG rò sang nhánh sau
    assert_ok(&[("main.gt", "ƒ f(o: i32) → i32 ! ∅ { ⌥ o { a ↦ a, b ↦ b } }")]);
    assert_ok(&[("main.gt", "ƒ f(o: i32) → i32 ! ∅ { ⌥ o { a ↦ a, a ↦ a } }")]);
    // …và không rò ra ngoài match
    assert_err(
        &[(
            "main.gt",
            "ƒ f(o: i32) → i32 ! ∅ { ⌥ o { a ↦ 1, _ ↦ 2 }; a }",
        )],
        "E0201",
        1,
    );
    // biến khối trong khối không thấy ngoài khối
    assert_err(
        &[("main.gt", "ƒ f(c: bool) → i32 ! ∅ { ⎇ c { z ≔ 1; } z }")],
        "E0201",
        1,
    );
}

#[test]
fn a_local_is_not_a_module_and_pattern_idents_that_name_non_variants_are_bindings() {
    // `x::f()` với x là biến cục bộ: x không phải module → E0201 (đoạn đầu chỉ tra cục bộ khi đường dẫn có MỘT đoạn)
    assert_err(&[("main.gt", "ƒ f() ! ∅ { x ≔ 1; x::g(); }")], "E0201", 1);
    // pattern ident trùng tên một HÀM không phải biến thể → binding che item (E0205)
    assert_err(
        &[(
            "main.gt",
            "ƒ g() ! ∅ { } ƒ f(o: i32) ! ∅ { ⌥ o { g ↦ 1, _ ↦ 2 }; }",
        )],
        "E0205",
        1,
    );
    // pattern ident là biến thể của enum trong CÙNG module (Q19) hoặc nhập từ module khác → không phải binding
    assert_ok(&[(
        "main.gt",
        "⊔ E { A, B } ƒ f(e: E) → i32 ! ∅ { ⌥ e { A ↦ 1, B ↦ 2 } }",
    )]);
    assert_ok(&[
        ("a.gt", "◈ ⊔ E { A, B }"),
        (
            "b.gt",
            "⇲ a::{A, B}; ƒ f(e: a::E) → i32 ! ∅ { ⌥ e { A ↦ 1, B ↦ 2 } }",
        ),
    ]);
}

#[test]
fn variants_are_names_of_their_enums_module_with_the_enums_visibility() {
    // biến thể của hai enum cùng module trùng tên → E0202 (một không gian tên)
    assert_err(&[("main.gt", "⊔ E { A } ⊔ F { A }")], "E0202", 1);
    // biến thể trùng tên hàm
    assert_err(&[("main.gt", "⊔ E { A } ƒ A() ! ∅ { }")], "E0202", 1);
    // enum private: biến thể cũng private với module khác (cả dạng trần lẫn đường dẫn)
    assert_err(
        &[("a.gt", "⊔ E { A }"), ("b.gt", "ƒ f() ! ∅ { a::A; }")],
        "E0203",
        1,
    );
    assert_err(
        &[("a.gt", "⊔ E { A }"), ("b.gt", "ƒ f() ! ∅ { a::E::A; }")],
        "E0203",
        1,
    );
    // enum private dùng trong CHÍNH module định nghĩa qua đường dẫn đủ: hợp lệ
    assert_ok(&[("main.gt", "⊔ E { A } ƒ f() → E ! ∅ { E::A }")]);
}

#[test]
fn imports_from_the_prelude_and_group_expansion() {
    assert_ok(&[("main.gt", "⇲ core::abort; ƒ f() ! ∅ { abort(); }")]);
    assert_err(&[("main.gt", "⇲ core::hidden;")], "E0203", 1);
    // nhóm có 3 tên: mỗi tên được nhập
    assert_ok(&[
        ("a.gt", "◈ ƒ f() ! ∅ { } ◈ ƒ g() ! ∅ { } ◈ ƒ h() ! ∅ { }"),
        ("b.gt", "⇲ a::{f, g, h}; ƒ k() ! ∅ { f(); g(); h(); }"),
    ]);
    // nhập module rồi dùng qua tên module nhập
    assert_ok(&[
        ("a.gt", "◈ ▦ m { ◈ ƒ f() ! ∅ { } }"),
        ("b.gt", "⇲ a::m; ƒ k() ! ∅ { m::f(); }"),
    ]);
    // vòng nhập (a = ⇲ b::a, b = ⇲ a::a) không treo và không ICE
    let out = run(&[("a.gt", "◈ ⇲ b::x; ◈ ƒ y() ! ∅ { }"), ("b.gt", "◈ ⇲ a::x;")]);
    assert!(out.report.diagnostics.iter().all(|d| !d.code.is_ice()));
    assert!(!out.report.diagnostics.is_empty());
}

#[test]
fn self_parameter_and_result_scoping() {
    // `self` hợp lệ trong thành viên trait có tham số self và chỉ ở hàm đó
    assert_ok(&[("main.gt", "⊏ T { ƒ get(&self) → i32 ! ∅; }")]);
    assert_err(
        &[(
            "main.gt",
            "▢ P { x: i32 } ⊐ P { ƒ a(&self) → i32 ! ∅ { self.x } ƒ b() → i32 ! ∅ { self.x } }",
        )],
        "E0201",
        1,
    );
    // `Self` trong tiêu đề của impl chưa có nghĩa
    assert_err(&[("main.gt", "▢ P { x: i32 } ⊐ Self { }")], "E0201", 1);
}

#[test]
fn import_cycles_are_one_e0201_each_and_independent_of_file_order() {
    let two = [("a.gt", "◈ ⇲ b::x;"), ("b.gt", "◈ ⇲ a::x;")];
    let three = [
        ("a.gt", "◈ ⇲ b::x;"),
        ("b.gt", "◈ ⇲ c::x;"),
        ("c.gt", "◈ ⇲ a::x;"),
    ];
    for files in [&two[..], &three[..]] {
        let out = assert_err(files, "E0201", 1);
        let mut rev: Vec<(&str, &str)> = files.to_vec();
        rev.reverse();
        let other = run(&rev);
        assert_eq!(out.canonical_diagnostics(), other.canonical_diagnostics());
        assert_eq!(out.canonical(), other.canonical());
        // nhãn trỏ tới các thành viên còn lại của vòng
        assert_eq!(out.report.diagnostics[0].labels.len(), files.len() - 1);
    }
}

// ======================= bổ sung sau đột biến vòng 2 =======================

/// Mỗi vị trí con của AST đều được duyệt: một tên chưa định nghĩa đặt ở đó phải bị báo đúng một lần.
#[test]
fn an_undefined_name_is_found_in_every_walked_position() {
    let cases = [
        "ƒ f() ! ∅ { x ≔ [1; zz]; }",                // độ dài mảng (biểu thức)
        "ƒ f(a: [i32; zz]) ! ∅ { }",                 // độ dài mảng trong kiểu
        "ƒ f() ! ∅ { ⌥ zz { _ ↦ 1 }; }",             // scrutinee
        "ƒ f() ! ∅ { ∀ i ∈ 0..3 : zz; }",            // thân lượng từ
        "ƒ f() ! ∅ { ⟳ true ↓ zz { } }",             // variant của vòng lặp
        "ƒ f() ! ∅ { ⟳ true ⋈ zz { } }",             // invariant
        "ƒ f() ! ∅ { ⟳ zz { } }",                    // điều kiện while
        "≡ T ≔ Zz;",                                 // alias
        "▢ P { x: i32 } ƒ f() ! ∅ { P { x: zz }; }", // giá trị trường struct literal
        "ƒ f(x: i32) ! ∅ { x as Zz; }",              // kiểu của phép ép
        "ƒ f() ! ∅ { ⎇ zz { } }",                    // điều kiện if
        "ƒ f() ! ∅ { (1, zz); }",                    // tuple
        "ƒ f() ! ∅ { 0..zz; }",                      // range
        "ƒ f() ! ∅ { ⚠ ⊢ zz { } }",                  // bằng chứng unsafe
        "ƒ f(a: [i32; 3]) ! ∅ { a[zz]; }",           // chỉ số
        "ƒ f() ! ∅ { ⏎ zz; }",                       // return
        "ƒ f() ! ∅ { g(zz); } ƒ g(x: i32) ! ∅ { }",  // đối số
    ];
    for src in cases {
        assert_err(&[("main.gt", src)], "E0201", 1);
    }
}

#[test]
fn generics_are_scoped_to_their_item_and_checked_for_shadowing() {
    // tham số generics không rò sang item khác
    assert_err(
        &[("main.gt", "ƒ f<T>(x: T) ! ∅ { } ƒ g() ! ∅ { T; }")],
        "E0201",
        1,
    );
    // generics của fn che generics của impl, hoặc tên item
    assert_err(
        &[(
            "main.gt",
            "▢ P<T> { x: T } ⊐ <T> P<T> { ƒ f<T>(&self) ! ∅ { } }",
        )],
        "E0205",
        1,
    );
    assert_err(
        &[("main.gt", "▢ T { x: i32 } ƒ f<T>(x: i32) ! ∅ { }")],
        "E0205",
        1,
    );
}

#[test]
fn a_local_may_not_shadow_a_file_module_name() {
    assert_err(
        &[
            ("a.gt", "◈ ƒ f() ! ∅ { }"),
            ("b.gt", "ƒ g() ! ∅ { a ≔ 1; }"),
        ],
        "E0205",
        1,
    );
}

#[test]
fn associated_paths_are_recorded_not_reported_and_still_count_as_dependencies() {
    use gt_resolve::Res;
    // `P::new` và `m::P::new`: Assoc với đúng tên thành viên
    let out = one("▢ P { x: i32 } ƒ f() ! ∅ { P::new(); }");
    assert!(out.report.diagnostics.is_empty());
    let members: Vec<&String> = out
        .uses
        .values()
        .filter_map(|r| match r {
            Res::Assoc { member, .. } => Some(member),
            _ => None,
        })
        .collect();
    assert_eq!(members, ["new"]);
    let out = run(&[
        ("m.gt", "◈ ▢ P { x: i32 }"),
        ("main.gt", "ƒ f() ! ∅ { m::P::new::deep(); }"),
    ]);
    assert!(out.report.diagnostics.is_empty());
    let members: Vec<&String> = out
        .uses
        .values()
        .filter_map(|r| match r {
            Res::Assoc { member, .. } => Some(member),
            _ => None,
        })
        .collect();
    assert_eq!(members, ["new::deep"]);
    // tham chiếu mục kết hợp xuyên tệp vẫn là phụ thuộc module → vòng
    assert_err(
        &[
            ("a.gt", "◈ ▢ P { x: i32 } ƒ f() ! ∅ { b::Q::new(); }"),
            ("b.gt", "◈ ▢ Q { x: i32 } ƒ g() ! ∅ { a::P::new(); }"),
        ],
        "E0204",
        1,
    );
    // không nhập được mục kết hợp
    assert_err(&[("main.gt", "▢ P { x: i32 } ⇲ main::P::new;")], "E0201", 1);
}

#[test]
fn prelude_enum_variants_are_reachable_by_path_and_prelude_items_are_public() {
    assert_ok(&[(
        "main.gt",
        "ƒ f() → Option<i32> ! ∅ { Option::Some(1) } ƒ g() → Result<i32, i32> ! ∅ { Result::Ok(1) }",
    )]);
}

#[test]
fn canonical_form_lists_every_definition_with_its_visibility() {
    let out = one("◈ ƒ f() ! ∅ { } ƒ g() ! ∅ { } ⊔ E { A }");
    let c = out.canonical();
    assert!(c.contains("def main::f fn pub"), "{c}");
    assert!(c.contains("def main::g fn priv"), "{c}");
    assert!(c.contains("def main::E enum priv"), "{c}");
    assert!(c.contains("def main::E::A variant priv"), "{c}");
    // DefId theo thứ tự đường dẫn
    let paths: Vec<&str> = out.defs.iter().map(|d| d.path.as_str()).collect();
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    assert_eq!(paths, sorted);
}

/// Mọi hoán vị thứ tự item cho cùng kết quả: nhập từ module anh em bị chặn, từ module con của chủ sở hữu thì được.
/// (Bắt lỗi sửa DefId sau khi sắp xếp: module của khai báo nhập và `parent` phải được đánh lại.)
#[test]
fn every_item_permutation_gives_the_same_visibility_verdicts() {
    let items = [
        "▦ a { ƒ s() ! ∅ { } ▦ c { ⇲ z::a::s; ƒ u() ! ∅ { s(); } } }",
        "▦ b { ⇲ z::a::s; ƒ t() ! ∅ { s(); } }",
        "ƒ top() ! ∅ { }",
        "▦ d { ◈ ƒ q() ! ∅ { } }",
    ];
    let mut perms: Vec<Vec<usize>> = vec![vec![]];
    for _ in 0..items.len() {
        let mut next = Vec::new();
        for p in &perms {
            for i in 0..items.len() {
                if !p.contains(&i) {
                    let mut q = p.clone();
                    q.push(i);
                    next.push(q);
                }
            }
        }
        perms = next;
    }
    assert_eq!(perms.len(), 24);
    let mut base: Option<(String, Vec<String>)> = None;
    for p in perms {
        let src: Vec<&str> = p.iter().map(|&i| items[i]).collect();
        let out = run(&[("z.gt", &src.join("\n"))]);
        assert_eq!(codes(&out), ["E0203"], "{src:?}");
        let key = (out.canonical(), out.canonical_diagnostics());
        match &base {
            None => base = Some(key),
            Some(b) => {
                assert_eq!(&b.0, &key.0);
                assert_eq!(&b.1, &key.1);
            }
        }
    }
}

#[test]
fn import_cycle_member_choice_does_not_depend_on_declaration_order() {
    let a = one("▦ m { ◈ ⇲ main::n::x; } ▦ n { ◈ ⇲ main::m::x; }");
    let b = one("▦ n { ◈ ⇲ main::m::x; } ▦ m { ◈ ⇲ main::n::x; }");
    assert_eq!(codes(&a), ["E0201"]);
    assert_eq!(a.canonical_diagnostics(), b.canonical_diagnostics());
    assert_eq!(a.canonical(), b.canonical());
    // tự nhập chính mình là vòng một thành viên: mỗi cái một chẩn đoán
    assert_eq!(codes(&one("⇲ main::y; ⇲ main::x;")), ["E0201", "E0201"]);
}
