//! Mutex, Atomic, Sync (M8 nửa 2, ADR-027): kiểm kiểu cơ bản và quy tắc Send/Sync cho các lang item mới.
mod common;
use common::*;

fn only(src: &str, code: &str) {
    let out = one(src);
    let c = codes(&out);
    assert!(
        !c.is_empty() && c.iter().all(|x| x == code),
        "kỳ vọng chỉ {code}, có {c:?} {:?}\n{src}",
        show(&out)
    );
}

const LV: &str = "#level(\"10\")\n▢ Lo { }\n#level(\"20\")\n▢ Hi { }\n";

#[test]
fn basic_mutex_and_atomic_typecheck() {
    ok(&format!(
        "{LV}ƒ bump(v: &!u32, n: u32) → u32 ! ∅ {{ *v ← *v +% n; *v }}\nƒ go(m: &Mutex<u32, Lo>) → u32 ! {{lock(Lo)}} {{ m.with(bump, 1u32) }}"
    ));
    ok("ƒ go(a: &Atomic<u32>) → u32 ! ∅ { a.fetch_add(1u32, AtomicOrdering::SeqCst) }");
    ok("ƒ go(a: &Atomic<bool>) ! ∅ { a.store(true, AtomicOrdering::Release); }");
}

#[test]
fn sync_and_send_rules_for_the_new_lang_items() {
    // Mutex<u32,_> và Atomic đều Send+Sync: chia sẻ qua static được
    ok(&format!(
        "{LV}■ M: Mutex<u32, Lo> ≔ Mutex::new(0u32);\n■ A: Atomic<u32> ≔ Atomic::new(0u32);"
    ));
    // Cell không Sync ⇒ static bị từ chối
    only("■ C: Cell<u32> ≔ Cell::new(0u32);", "E0501");
    // Mutex chứa tham chiếu: không Send ⇒ không Sync
    only(
        &format!("{LV}■ M: Mutex<&u32, Lo> ≔ Mutex::new(&5u32);"),
        "E0501",
    );
    // Cell Send nhưng không Sync: spawn cho phép, static thì không
    ok("ƒ w(c: Cell<u32>) ! ∅ { }\nƒ main() ! ∅ { spawn(w, Cell::new(1u32)); }");
    // ⊐ Sync for … bị cấm (trait tự động)
    only("▢ S { v: i32 }\n⊐ Sync for S { }", "E0313");
}

#[test]
fn polymorphic_effects_flow_from_the_function_argument() {
    // spawn nhận worker có io; Mutex::with nhận hàm có hiệu ứng bất kỳ
    ok("ƒ out(n: u32) ! {io} { }\nƒ main() ! {io} { spawn(out, 1u32); }");
    // người dùng không được tự dùng marker poly/detach
    only("ƒ f(g: ƒ() ! {poly}) ! ∅ { }", "E1002");
    only("ƒ f() ! {detach} { }", "E1002");
}

const SHARE: &str = "ƒ share<T: Sync>(x: &T) ! ∅ { }\n";

#[test]
fn sync_is_structural() {
    // nguyên thuỷ, tuple, mảng, ADT sở hữu
    ok(&format!(
        "{SHARE}▢ P {{ a: u32, b: (u8, [bool; 2]) }}\nƒ main() ! ∅ {{ share(&1u32); share(&(1u8, 2u8)); share(&[1u32, 2u32]); share(&P {{ a: 1, b: (2, [true, false]) }}); }}"
    ));
    // &T Sync iff T Sync; Cell không Sync, kể cả khi nằm sau tham chiếu hoặc trong ADT/tuple/mảng/Option
    only(
        &format!("{SHARE}ƒ main() ! ∅ {{ c ≔ Cell::new(1u32); r ≔ &c; share(&r); }}"),
        "E0501",
    );
    only(
        &format!(
            "{SHARE}▢ H {{ n: u32, c: Cell<u32> }}\nƒ main() ! ∅ {{ share(&H {{ n: 1, c: Cell::new(2u32) }}); }}"
        ),
        "E0501",
    );
    only(
        &format!("{SHARE}⊔ E {{ A(u32), B(Cell<u32>) }}\nƒ main() ! ∅ {{ share(&E::A(1)); }}"),
        "E0501",
    );
    only(
        &format!("{SHARE}ƒ main() ! ∅ {{ share(&(1u8, Cell::new(2u32))); }}"),
        "E0501",
    );
    only(
        &format!("{SHARE}ƒ main() ! ∅ {{ share(&[Cell::new(2u32)]); }}"),
        "E0501",
    );
    // Mutex<T,_> Sync iff T Send: Cell Send ⇒ Mutex<Cell> Sync; tham chiếu thì không
    ok(&format!(
        "{LV}{SHARE}ƒ main() ! ∅ {{ m: Mutex<Cell<u32>, Lo> ≔ Mutex::new(Cell::new(1u32)); share(&m); }}"
    ));
    only(
        &format!(
            "{LV}{SHARE}ƒ main() ! ∅ {{ n ≔ 5u32; m: Mutex<&u32, Lo> ≔ Mutex::new(&n); share(&m); }}"
        ),
        "E0501",
    );
    // Atomic Sync; Arena không
    ok(&format!(
        "{SHARE}ƒ main() ! ∅ {{ share(&Atomic::new(1u32)); share(&Atomic::new(true)); }}"
    ));
    only(
        &format!("{SHARE}ƒ run(a: Arena) ! ∅ {{ share(&a); }}"),
        "E0501",
    );
    // tham số kiểu cần ràng buộc Sync
    ok(&format!("{SHARE}ƒ run<T: Sync>(x: &T) ! ∅ {{ share(x); }}"));
    only(
        &format!("{SHARE}ƒ run<T>(x: &T) ! ∅ {{ share(x); }}"),
        "E0501",
    );
    // Send ≠ Sync: bound Send không đủ cho Sync
    only(
        &format!("{SHARE}ƒ run<T: Send>(x: &T) ! ∅ {{ share(x); }}"),
        "E0501",
    );
}

#[test]
fn send_rules_for_cell_mutex_atomic() {
    let w = "ƒ w<T: Send>(x: T) ! ∅ { }\n";
    ok(&format!(
        "{LV}{w}ƒ main() ! ∅ {{ w(Cell::new(1u32)); w(Atomic::new(1u32)); m: Mutex<u32, Lo> ≔ Mutex::new(1u32); w(m); }}"
    ));
    only(
        &format!("{w}ƒ main() ! ∅ {{ n ≔ 1u32; w(Cell::new(&n)); }}"),
        "E0501",
    );
    only(
        &format!("{LV}{w}ƒ main() ! ∅ {{ n ≔ 1u32; m: Mutex<&u32, Lo> ≔ Mutex::new(&n); w(m); }}"),
        "E0501",
    );
}
