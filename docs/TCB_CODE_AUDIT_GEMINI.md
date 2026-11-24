# Đọc mã đối kháng KHÁC HỌ model — Họ Gemini (nhánh `model-gemini-neon`, 2026-11-23/24)

**Reviewer:** Họ model **Gemini** (tham gia dự án với vai trò đọc chéo **KHÁC HỌ** so với tác giả chính Claude; nhánh làm việc `model-gemini-neon`).  
**HEAD đọc và thử nghiệm:** `f342ae4` (`main` tại thời điểm rẽ nhánh; không sửa bất kỳ tệp mã nguồn/test/runtime/tài liệu TCB nào).  
**Môi trường chạy thực nghiệm:** Linux Ubuntu 24.04 x86_64, `clang-20` / `llvm-20-dev` (20.1.2), Rust stable + nightly (`rustc 1.102.0-nightly 20fe935d5 2026-11-23`, `rust-src`), `cargo-fuzz 0.13.2`.  
**Phương pháp tuân thủ yêu cầu dự án:** Đọc thẳng mã nguồn TRƯỚC (checker M4–M9, `gt-kernel`, `gt-guard`, `gt-rt`, `gt-codegen-llvm`, `gt-driver`, `gt-dataflow`, `gt-mir`, `gt-parser`, `rig`) rồi mới đối chiếu tài liệu/ADR/sổ lỗi; mọi kết luận đều gắn lệnh chạy thật, mã thoát và đầu ra thực tế.

> **Tuyên bố trung thực (tuân thủ `docs/SAFETY_CLAIMS.md` §6 và `docs/TCB_REVIEW_GUIDE.md`):**  
> Đây là đợt đọc mã đối kháng có mục tiêu (~14 000+ dòng mã trọng yếu) kèm thực nghiệm tấn công, TSan và fuzz ngắn trên Linux — **KHÔNG phải chứng minh hình thức**, **KHÔNG phải kiểm toán toàn bộ ~65 000+ dòng mã**, và **KHÔNG đổi bất kỳ trạng thái nào** trong `docs/TCB_INTRINSICS.md` hay `docs/STATUS.md`. Vì phát hiện nhiều lỗ hổng soundness đạt tới được từ Grit an toàn (bao gồm **Use-After-Free thực sự trên stack và heap `Owned`**), báo cáo này là đầu vào đề nghị kiến trúc sư lập test ĐỎ trước rồi sửa từng lỗi thành commit + `FB-C…` riêng.

---

## 1. Tóm tắt kết quả

| Nhóm kết quả | Số lượng | Chi tiết ngắn |
|---|---:|---|
| **Lỗi Nghiêm trọng (Soundness / Memory Safety / D12 / D14)** | **3 nhóm (7 đường khai thác)** | **G-01**: Safe-Grit Use-After-Free (stack + heap `Owned`) qua `Callee::Value` bất biến; **G-02**: `pure_preds` bỏ qua `■ Atomic` → treo vĩnh viễn dưới `--checks=all` mặc định & xoá nhầm `⊳` dưới `--checks=proven`; **G-03**: 5 pass (`gt-term`, `gt-effects`, `gt-conc`, `gt-oblig::unsafe_check`, `gt-secrets`) chỉ duyệt `BodyKind::Fn`, bỏ sót `Static`/`Const`/`Pre`/`Post` |
| **Lỗi Cao (Bảo mật D13 / Post-Green ICE / Soundness đến Kernel)** | **3** | **G-04**: `Owned::into_inner` và `spawn` trampoline rò bản sao `Secret` trên heap chưa xoá (`gritrt_secret_wipe`); **G-05**: `Cell<T>`/`Mutex<T, L>` generic vượt E0318 → `check` XANH rồi sập ICE exit 2 cả interpreter & LLVM; **G-06**: `narrow` & `check_post_fixpoint` không kiểm `facts`/`defs` của Phase 2 `linear_facts` |
| **Lỗi Trung bình–Cao (Edition 2 C2/C2c, Typecheck, TCB FFI, Release surface)** | **6** | **G-07a**: vượt E1002 `! {detach}`/`! {poly}` trong `impl`/`trait`; **G-07b**: `..` bên trong `\|` vượt `check_single_rest`; **G-07c**: or-pattern nổ mũ treo `gritc check --edition=2`; **G-07d**: `?` chấp nhận trong `□` const; **G-08a**: `gritrt_alloc(0)` UB trong Rust (hiện chưa đạt tới từ Grit); **G-08b**: `GRITC_FAULT=mir:pat-field[-swap]` biên dịch sai im lặng trong bản **release** |
| **Thiếu sót công cụ / nhỏ** | **6** | **G-09a**: 3 fuzz target, `gt-secrets::scan_text`, `rig test --auto`, `lsp` cố định Edition 1; **G-09b**: tràn `usize` ở `rig::autotest::combos`, panic `i128::MIN` ở `gt-kernel::search`, `runner::find_main` thiếu lọc `BodyKind::Fn`, `attestation` thiếu `edition`, `gt-llvm/shim.rs` thiếu `#[cfg(target_arch = "x86_64")]` |
| **Giả thuyết tấn công đã BÁC BỎ (trình biên dịch chặn đúng)** | **14** | `⊲` trên `Owned` (E0902), `root_out()` trong `■` (E0310), `■ Mutex<Owned>` (E0501+E0310), `ƒ(&u32)→&'static u32` (E0404), `Vec<T>` generic với `Secret`/`Owned` (E0318), `gt-guard` `UB_FLAGS` tên hàm `fast`/`exact`, 8 ca Edition 2 (`?`/let-pat/or-pat với `Owned`/`Secret`) |
| **Kiểm thử động (Linux x86_64)** | **TSan XANH, 3/3 fuzz XANH (0 sập)** | `tools/tsan_check.sh 50` (0 data race trên 400 lượt chạy + 5/5 đối chứng dương bắt + 3/3 đối chứng âm sạch) + mẫu Phase 2 `Secret`+`spawn`+`Atomic` (0 race/20 lượt); fuzz `lex` (121s, 265 899 chạy, 0 sập), `parse` (121s, 73 118 chạy, 0 sập), `check` (nạp 1 067 seed trong 270s: 1 `slow-unit` 17s, 0 sập + đột biến thật 61s: 2 063 chạy, 285 mẫu mới, 0 sập) |
| **CHƯA KIỂM** | **5 mục lớn** | Windows x86_64 / MSVC / SRWLOCK / cổng Windows; phần cứng bộ nhớ yếu (ARM64); fuzz 24h mỗi đích; `tools/gate_clean.sh` đầy đủ; ~50 000 dòng mã ngoài phạm vi đọc sâu |

---

## 2. Chi tiết các lỗi đã xác nhận (mọi lỗi đều MỚI, chưa có trong `docs/FIELD_BUGS.md` hay `docs/KNOWN_LIMITATIONS.md`)

Tất cả các phát hiện dưới đây được chạy thử bằng `./target/release/gritc` dựng từ `f342ae4` (`LLVM 20`, `gritc doctor` → `doctor: XANH`, exit 0). Không có mã nào được chỉnh sửa.

### G-01 [NGHIÊM TRỌNG — MEMORY SAFETY]: Safe-Grit Use-After-Free (Stack & Heap `Owned`) qua con trỏ hàm (`Callee::Value`) trong M6 `gt-borrowck`

- **Vị trí:** `crates/gt-borrowck/src/regions.rs:546-566` (`Callee::Value` nhánh fallback).
- **Nguyên nhân:** Khi lời gọi đi qua giá trị/con trỏ hàm (`Callee::Value(_)`), `regions.rs` bỏ qua `call_sig` (`mapped = false`) và dùng nhánh "bảo thủ" chỉ thêm `a : b` giữa các region đầu vào và `a : x` từ đầu vào tới đầu ra — **thiếu hẳn cạnh ngược `x : a`** cho các vị trí **bất biến** (`&Cell<&T>`, `&!T`, `Mutex<T, L>`). Nhờ đó M6 cho phép thu ngắn lifetime bên trong `Cell<&T>` (coi `Cell` như hiệp biến), rồi ghi tham chiếu ngắn hạn (`&local` trên stack hoặc `o.get()` trên heap `Owned`) vào `Cell` sống lâu hơn!
- **Vi phạm hợp đồng:** Điều kiện cứng #10 của Giang (`GRIT_PLAN.md` §7: interior mutability phải bất biến) và tuyên bố an toàn bộ nhớ cốt lõi (D1/D2).
- **Mẫu tái hiện tối thiểu (Heap UAF — 100% Grit an toàn `! {mem}`, không `⚠`):**
  ```text
  ƒ id_cell<'a>(c: &'a Cell<&'a u32>) → &'a Cell<&'a u32> ! ∅ { c }

  ƒ main() → u32 ! {mem} {
    a ≔ root_alloc();
    outer: u32 ≔ 111u32;
    c: Cell<&u32> ≔ Cell::new(&outer);
    fp: ƒ(&Cell<&u32>) → &Cell<&u32> ! ∅ ≔ id_cell;
    {
      o ≔ Owned::new(&a, 999u32);
      c2 ≔ fp(&c);
      c2.set(o.get());
    };
    // `o` đã bị giải phóng bằng gritrt_heap_free! `*c.get()` đọc vùng heap đã free!
    r ≔ c.get();
    *r
  }
  ```
  (Bản stack UAF thay `o ≔ Owned::new(&a, 999u32)` bằng `local: u32 ≔ 999u32; c2.set(&local);`, hiệu ứng `! ∅` hoàn toàn thuần.)
- **Bằng chứng thực nghiệm:**
  - `gritc check --json`: **`verdict: "green"`, 0 lỗi, exit 0** (cả bản stack lẫn bản heap).
  - `gritc run --backend=interpreter`: **exit 2 (ICE)** — `đọc _6 chưa gán / đã chết` (stack) và `đọc _0 chưa gán / đã chết` (heap) — chứng tỏ interpreter bắt được việc đọc ô đã chết sau khi M6 đã cho qua!
  - `gritc build --backend=llvm --profile=dev`: **exit 0** (vượt qua cả IR Guard!), chạy binary với `GRITRT_HEAP_REPORT=1` báo `gritrt_heap_live=0` nhưng tiến trình đã đọc con trỏ heap/stack bị giải phóng!
- **Hướng sửa đề xuất (chỉ tham vấn):** Nhánh fallback của `Callee::Value` (và mọi fallback trong `regions.rs:548-566`) cần thêm cạnh hai chiều `self.add(*x, *a)` (đẳng trị `a = x`) hoặc đối sánh đầy đủ chữ ký `Ty::Fn` theo tính biến thiên (`Variance::Invariant`) giống `call_sig`.

---

### G-02 [NGHIÊM TRỌNG — SOUNDNESS]: `pure_preds` coi hàm `! ∅` dùng `■ Atomic` là hàm thuần tất định → vòng lặp vô hạn dưới `--checks=all` mặc định & xoá nhầm `⊳` dưới `--checks=proven`

- **Vị trí:** `crates/gt-oblig/src/driver.rs:510-544` (`build_prog_info`) kết hợp `lib/core/prelude.gt:36-42`.
- **Nguyên nhân:** Các phương thức `Atomic::{load, store, swap, fetch_add, fetch_sub, compare_exchange}` mang hiệu ứng `! ∅`. `build_prog_info` chỉ kiểm `sig.effects.is_empty()`, không nhận `&!`, và tham số không chứa interior mutability (`may_have_interior_mut`) — **bỏ quên hoàn toàn trạng thái biến thiên toàn cục qua `■` tĩnh `Atomic<T>`** (vốn là `Sync` nên hợp lệ trong `■`). Hệ quả là `gt-oblig`, `gt-kernel` và `gt-term` tin rằng `is_ok(x)` cho kết quả bất biến trong cùng một khối/nhánh!
- **Bằng chứng 1 — Vượt D12 (treo vĩnh viễn) ngay dưới chế độ MẶC ĐỊNH `--checks=all`:**
  ```text
  ■ A: Atomic<u32> ≔ Atomic::new(0u32);
  ƒ is_ok(x: u32) → bool ! ∅ { A.fetch_add(1u32, AtomicOrdering::SeqCst) == 0u32 }
  ƒ main() → u32 ! ∅ {
    x: u32 ≔ 1u32;
    ⎇ is_ok(x) {
      ⎇ ¬is_ok(x) {
        ⟳ true ↓ x { }
      }
    };
    0u32
  }
  ```
  - `gritc check --json` (mặc định `--checks=all`): **`verdict: "green"`, exit 0** (kernel suy ra nhánh `⎇ ¬is_ok(x)` vô lý từ giả thiết `is_ok(x)` và chứng minh sai rằng vòng lặp không bao giờ chạy/dừng!).
  - `gritc build --backend=llvm --profile=dev`: **exit 0**.
  - Chạy binary (`timeout 2 ...`): **treo vĩnh viễn, exit 124**!
- **Bằng chứng 2 — Xoá nhầm tiền điều kiện `⊳` dưới `--checks=proven`:**
  ```text
  ■ A: Atomic<u32> ≔ Atomic::new(0u32);
  ƒ is_ok(x: u32) → bool ! ∅ { A.fetch_add(1u32, AtomicOrdering::SeqCst) == 0u32 }
  ƒ need_ok(x: u32) → u32 ! ∅ ⊳ is_ok(x) { x }
  ƒ main() → u32 ! ∅ { x: u32 ≔ 1u32; ⎇ is_ok(x) { need_ok(x) } ⇝ { 0u32 } }
  ```
  - `--checks=all`: chạy binary abort `abort: precondition` (exit 70 — hành vi đúng vì lần gọi thứ 2 trả `false`).
  - `--checks=proven`: kernel xoá nhầm kiểm `⊳ is_ok(x)` → binary thoát **exit 0** dù tiền điều kiện thực tế là `false`!
- **Hướng sửa đề xuất:** Chỉ đưa hàm vào `pure_preds` khi kiểm tra thân hàm (và đồ thị gọi truyền tiếp) **không truy cập `■` static chứa interior mutability (`Atomic`/`Mutex`) và không chứa khối `⚠`**, hoặc gắn một hiệu ứng/cờ riêng cho thao tác `Atomic`/trạng thái toàn cục.

---

### G-03 [NGHIÊM TRỌNG — LỖI HỆ THỐNG]: 5 pass kiểm tra chỉ lọc `BodyKind::Fn`, bỏ lọt hoàn toàn `Static`, `Const`, `Pre` (`⊳`), `Post` (`⊲`)

Năm pass độc lập trong M7/M8/M9 đều dùng cùng một mẫu lọc `m.bodies.iter().filter(|b| b.kind == BodyKind::Fn)` trong khi MIR vẫn hạ đầy đủ các thân `Static`, `Const`, `Pre`, `Post` và **codegen LLVM biên dịch + chạy thật chúng**! (Ghi chú: `docs/CONTROL_TRANSFER_INVENTORY.md` dòng 9 và 13 đã sót vòng lặp trong `Pre`/`Post` và vòng lặp/lời gọi `! {div}` trong `Static`/`Const`.)

| Mã con | Pass & Vị trí | Lỗi bị bỏ lọt | Mẫu tối thiểu & Kết quả thật |
|---|---|---|---|
| **G-03a** | `gt-term`: `crates/gt-term/src/lib.rs:749,876` | Vượt hoàn toàn D12 (chứng minh dừng) | 5 mẫu: `■ BAD: u32 ≔ { ⟳ true { } 0u32 };`, `□ BAD: u32 ≔ { ⟳ true { } 0u32 };`, `■ BAD: u32 ≔ loop_fn();` (`loop_fn ! {div}`), `⊳ { ⟳ true { } true }`, `⊲ { ⟳ true { } true }`. **5/5 `gritc check` = `green` (exit 0), LLVM build exit 0, chạy binary `timeout 2` = exit 124 (treo vĩnh viễn)** trong `main() ! ∅`! |
| **G-03b** | `gt-effects`: `crates/gt-effects/src/lib.rs:49` | Vượt **E0502** (đảo ngược thứ tự khoá L2→L1) và **E0801** (hiệu ứng `! {io}` trong hàm thuần `! ∅`) | (1) `■ BAD: u32 ≔ M2.with(call1, 0u32);` và `⊳ M2.with(call1, 0u32) == 3u32` (khoá L2 rồi L1): **`green`**, LLVM build & chạy exit 0. (2) `ƒ pure_fn(o: &Cap<Out>) → u32 ! ∅ ⊳ write_it(o) { 0u32 }` gọi từ `main() ! ∅`: **`green`**, LLVM build exit 0, **chạy binary in `99` ra stdout**! |
| **G-03c** | `gt-conc`: `crates/gt-conc/src/lib.rs:144` | Vượt **E0503** (`AtomicOrdering` sai) trong `■` và `⊳` | `A.load(AtomicOrdering::Release)` trong `■`/`⊳` → `check` **`green`** (LLVM build exit 3). `A.compare_exchange(0u32, 1u32, Relaxed, SeqCst)` trong `■` và `⊳` → `check` **`green`**, rồi LLVM build sập **ICE exit 2** (`LLVM: Cmpxchg ordering error or mismatch`)! |
| **G-03d** | `gt-oblig::unsafe_check`: `crates/gt-oblig/src/unsafe_check.rs:170` | Vượt **E1001** (gọi `raw_read` ngoài `⚠`) trong `■`, `□`, `⊳` | `■ BAD: i32 ≔ raw_read(0usize);` (tương tự `□` và `⊳`): **`green`** (exit 0)! (LLVM build chỉ dừng exit 3 vì `raw_read` chưa được hạ, tức là cổng kiểm tĩnh E1001 hoàn toàn bị vượt qua.) |
| **G-03e** | `gt-secrets`: `crates/gt-secrets/src/lib.rs:423` | Vượt **S201/S202** (rò rỉ / `declassify` secret) trong `■` | `■ S: u32 ≔ Secret::new(42u32).declassify();`: **`green`**, **0 cảnh báo** ngay cả khi bật `--deny-yellow`, LLVM build exit 0! |

- **Hướng sửa đề xuất:** Duyệt TẤT CẢ các `Body` trong `m.bodies` ở cả 5 pass (quy ước rõ: `Static`/`Const` có tập hiệu ứng cho phép = `∅`, không `div`, không `lock`, không `unsafe` ngoài `⚠`; `Pre`/`Post` kế thừa giới hạn an toàn của hàm chủ hoặc bị siết chặt về `! ∅`, không `div`, không khoá mới).

---

### G-04 [CAO — BẢO MẬT D13 / ADR-P2-03 / TCB B6]: Không gọi `gritrt_secret_wipe` cho `Secret<T>` trên heap trong `Owned::into_inner` và `spawn` trampoline (và `■ Mutex<Secret<T>>`)

- **Vị trí:**
  - `crates/gt-codegen-llvm/src/lower.rs:2837-2850` (`Intr::OwnedIntoInner`) và `crates/gt-mir/src/interp.rs:859`
  - `crates/gt-codegen-llvm/src/lower.rs:3194-3228, 3894-3932` (`Intr::Spawn` & `trampoline`)
- **Nguyên nhân & Bằng chứng:**
  1. **`Owned::into_inner(o)`:** Đọc giá trị `T` từ con trỏ heap `p` (`build_load`) rồi gọi thẳng `self.heap_free(p, t)` **mà không gọi `gritrt_secret_wipe(p, t)`** khi `t` chứa `Secret`. Kiểm tra trực tiếp IR sinh ra (`bug4a_owned_secret_into_inner.ll`) và chạy với `GRITRT_SECRET_REPORT=1`: chỉ có **2 lần wipe** (8 byte, đều ở biến cục bộ trên stack), **0 lần wipe trên vùng heap đã trả về allocator**!
  2. **`spawn(worker, Secret::new(42u32))`:** `Secret<u32>` là `Send` (`crates/gt-types/src/send.rs`). `Intr::Spawn` cấp hộp heap `blk` qua `gritrt_alloc` (rò rỉ có chủ đích tới cuối tiến trình!), ghi `Secret` vào `blk`; hàm đệm `gt.tramp.0` đọc `Secret` ra rồi gọi `worker` mà **không bao giờ gọi `gritrt_secret_wipe` lên `ap` trong `blk`**. Kiểm tra IR (`bug4b_spawn_secret.ll`): `gt.tramp.0` có **0 lệnh gọi `gritrt_secret_wipe`**. Thử nghiệm TSan Phase 2 với 4 luồng (`tsan_p2_secret_spawn.gt`) ghi nhận `gritrt_secret_wipes=12` thay vì `16` — 4 bản sao `Secret` nằm nguyên dạng bản rõ trên heap rò rỉ cho tới khi tiến trình kết thúc!
  3. **`■ M: Mutex<Secret<u32>, L1>`:** `Mutex::new(Secret::new(42u32))` trong `■` tĩnh vượt `gritc check` (`green`), dựng & chạy LLVM với **0 lần wipe** (tài liệu/kiểm tĩnh cần nêu rõ hoặc cấm nếu D13 đòi hỏi mọi `Secret` phải được xoá).
- **Hướng sửa đề xuất:** Trong `OwnedIntoInner` (cả LLVM lẫn interpreter), nếu `contains_secret(t)` thì gọi `secret_wipe(p, t)` trước `heap_free(p, t)`. Trong `trampoline`, ngay sau `build_load(at, ap, "a")`, nếu `contains_secret(a_ty)` thì gọi `gritrt_secret_wipe(ap, sizeof(a_ty))`.

---

### G-05 [CAO — POST-GREEN ICE]: `Cell<T>` và `Mutex<T, L>` trong hàm generic vượt E0318 → `gritc check` XANH rồi sập ICE (exit 2) cả interpreter & LLVM

- **Vị trí:** `crates/gt-dataflow/src/elaborate.rs:304-315`.
- **Nguyên nhân:** Kiểm tra `E0318` cho kiểu lang đục (`Cell`, `Mutex`) chứa kiểu cần `Drop` được đặt ở `elaborate.rs` với điều kiện `!ty.has_param()`. Khi `Cell::new(x)` hoặc `Mutex::new(x)` nằm trong hàm generic `ƒ hide<T>(x: T)`, `ty` là `Cell<T0>` (có param) nên `elaborate` bỏ qua im lặng! (Trái lại, `Vec<'a, T>` đã có kiểm tra ở tầng `gt-types` nên `Vec<T>` bị chặn đúng `E0318`.)
- **Mẫu tái hiện:**
  ```text
  ƒ hide<T>(x: T) ! ∅ { c ≔ Cell::new(x); }
  ƒ main() → u32 ! {mem} { a ≔ root_alloc(); hide(Owned::new(&a, 42u32)); 0u32 }
  ```
  (Tương tự với `Secret::new(42u32)` trong `Cell<T>` và `Mutex<T, L1>`.)
- **Bằng chứng thực nghiệm (3 mẫu `bug5a`, `bug5f`, `bug5g`):**
  - `gritc check --json`: **`verdict: "green"`, exit 0**!
  - `gritc run --backend=interpreter`: **exit 2 (ICE)** — `drop glue: kiểu lang đục Cell/Mutex chứa giá trị có Drop người dùng: chưa hỗ trợ (ADR-C18)`.
  - `gritc build --backend=llvm --profile=dev`: **exit 2 (ICE)** với cùng thông báo!
- **Hướng sửa đề xuất:** Kiểm `E0318` cho `Cell<T>` và `Mutex<T, L>` tại `gt-types` (tương tự `Vec<T>`: yêu cầu `T: Copy` hoặc không cần drop trên từng instantiation trong `types.instances`), hoặc chạy lại kiểm `drop_glue` trên bảng monomorphized `types.instances` trước khi mở cổng.

---

### G-06 [CAO — SOUNDNESS ĐẾN KERNEL]: `narrow` và bộ kiểm độc lập `check_post_fixpoint` không kiểm `facts` và `defs` (Phase 2 `linear_facts`, ADR-P2-05)

- **Vị trí:** `crates/gt-oblig/src/analysis.rs:2326-2358` (kết hợp `analysis.rs:1211` nơi `s.facts` được nạp thẳng làm giả thiết tin cậy vào `gt-kernel`).
- **Nguyên nhân:**
  - `narrow` (`analysis.rs:2328`) thay thế `states[bb] = Some(i)` chỉ dựa trên điều kiện `i.env.leq(&old.env)` — không kiểm `i.facts` hay `i.defs` có thoả hậu-điểm-cố-định hay không.
  - Quan trọng hơn: `check_post_fixpoint` (`analysis.rs:2338-2358`) được thiết kế làm chốt chặn độc lập kiểm chứng trạng thái điểm cố định cuối cùng (trả ICE nếu sai), nhưng dòng 2347 **CHỈ kiểm `out.env.leq(&t.env)`**, hoàn toàn **không kiểm `t.facts ⊆ out.facts` và `t.defs ⊆ out.defs`** (thứ tự ngược tập hợp cho các bất đẳng thức giao nhau!).
  - Nếu có bất kỳ sai sót nào trong `join`/`widen`/`narrow` làm `t.facts` giữ thừa một ràng buộc tuyến tính không đúng trên một nhánh vào, `check_post_fixpoint` sẽ cho qua XANH và `gt-kernel` sẽ nhận giả thiết sai đó là đúng để chứng minh/xoá kiểm!
- **Trạng thái xác minh:** **Xác nhận lỗ hổng chốt kiểm tra qua đọc mã**; chưa tạo được phản ví dụ kích hoạt trực tiếp `t.facts ⊈ out.facts` với các toán tử `join`/`widen` hiện tại (`CHƯA KIỂM` khả năng khai thác ngay), nhưng là thiếu sót rõ ràng trong hàng rào soundness của Phase 2 `ADR-P2-05`.
- **Hướng sửa đề xuất:** Bổ sung kiểm `t.facts.iter().all(|f| out.facts.contains(f))` và kiểm tương ứng cho `t.defs` ngay trong `check_post_fixpoint` (và trong điều kiện chấp nhận của `narrow`).

---

### G-07 [TRUNG BÌNH–CAO — EDITION 2 (C2/C2c) & TYPECHECK]: 4 lỗi đã tái hiện

1. **G-07a — Vượt E1002 (`! {detach}` / `! {poly}`) trong `impl` và `trait` (`crates/gt-types/src/collect.rs:1204,1243`):**  
   `fill_fn` (`collect.rs:1187`) gọi `check_marker_effects`, nhưng `fill_trait` và `fill_impl` **không gọi**.  
   Mẫu `bug8_impl_detach_bypass.gt`: `▢ S {} ⊐ S { ƒ sneaky() ! {detach} {} ƒ sneaky2() ! {poly} {} }` → `gritc check` = **`green` (exit 0)**, LLVM build & chạy exit 0!
2. **G-07b — C2c Or-pattern cho phép `..` (`Rest`) bên trong `|` và nhiều `..` (`crates/gt-parser/src/pats.rs:184-205`):**  
   `parse_pattern_elem` gọi `parse_alts(true)`, nên `(1u32, .. | 2u32)` và `(.. | 3u32, .. | 4u32)` vượt qua cả parser lẫn `check_single_rest` (chỉ kiểm `PatternKind::Rest` cấp ngoài).  
   Mẫu `bug6_or_pattern_rest.gt` (`--edition=2`): `gritc check` = **`green`**, LLVM build & chạy trả về `20`!
3. **G-07c — C2c Or-pattern nổ mũ trong `exhaust.rs` gây treo `gritc check --edition=2` (`crates/gt-types/src/exhaust.rs:202-218`):**  
   `missing` khai triển từng `Pat::Or` thành nhiều hàng không giới hạn/không khử trùng. Một hàm 6 dòng khớp tuple 20 phần tử `(_|_, _|_, ..., _|_)` (`bug7c_or_wild_blowup.gt`) làm `gritc check --edition=2` **treo vượt quá 5 giây (`timeout 5` → exit 124)**!
4. **G-07d — Toán tử `?` (C2) được chấp nhận trong `□` const (`bug9a_try_in_const.gt`):**  
   `□ C: Option<u32> ≔ Some(None?);` với `--edition=2` đạt **`green` (exit 0)**, LLVM build & chạy exit 0 — trái ngược trực tiếp với mô tả giới hạn trong `docs/KNOWN_LIMITATIONS.md:394` (`? chỉ trong thân ƒ`).

---

### G-08 [TRUNG BÌNH — TCB RUNTIME FFI & RELEASE SURFACE]

1. **G-08a — `gritrt_alloc(0)` gọi `std::alloc::alloc` với `Layout` cỡ 0 (UB trong Rust) (`runtime/gt-rt/src/lib.rs:727-733`):**  
   `gritrt_alloc` là hàm `pub extern "C"` thuộc TCB runtime; khác với `gritrt_heap_alloc` (dùng `size.max(1)`), `gritrt_alloc` truyền thẳng `size=0` vào `std::alloc::alloc(Layout::from_size_align(0, 16))` — đây là hành vi bất định (UB) theo hợp đồng `GlobalAlloc::alloc` của Rust. Hiện tại từ Grit không đạt tới vì `Intr::Spawn` luôn cấp `{fn_ptr, arg}` có kích thước ≥ 8 byte (`lower.rs:3200`), nhưng cần chặn `size == 0` / `size.max(1)` giống `gritrt_heap_alloc` để giữ hợp đồng FFI an toàn (tương tự `FB-C31`).
2. **G-08b — `GRITC_FAULT=mir:pat-field` và `mir:pat-field-swap` mở trong binary RELEASE (`crates/gt-driver/src/pipeline.rs:355-362`):**  
   Mọi chốt `GRITC_FAULT` khác đều chỉ làm trình biên dịch từ chối THÊM (fail-closed). Riêng `mir:pat-field` và `mir:pat-field-swap` được biên dịch không điều kiện vào `target/release/gritc` và **cố tình sinh MIR sai (hoán đổi biến bind cùng kiểu)** nhưng vẫn đi qua M6, M7, M8 và IR Guard với phán quyết **`green`**! Thực nghiệm với `Pair { a: 10u32, b: 99u32 }` trả `*a` (`bug13_fault_swap.gt`): chạy thường ra `Int(10, U32)`; đặt `GRITC_FAULT=mir:pat-field-swap` trên `./target/release/gritc` cho phán quyết **`green` (0 lỗi, 0 cảnh báo)**, interpreter trả **`Int(99, U32)`**, LLVM build **exit 0** (qua IR Guard!). Nên khoá sau `#[cfg(feature = "test-faults")]` hoặc `debug_assertions` để binary release không thể bị biến môi trường làm biên dịch sai im lặng.

---

### G-09 [THẤP–TRUNG BÌNH — CÔNG CỤ, EDITION 2 & CHI TIẾT PHỤ]

- **G-09a — Edition 1 bị viết cứng ở 4 điểm công cụ/kiểm tra:**
  - Cả 3 fuzz target (`crates/gt-lexer/fuzz/fuzz_targets/lex.rs`, `crates/gt-parser/fuzz/fuzz_targets/parse.rs`, `crates/gt-driver/fuzz/fuzz_targets/check.rs`) chỉ gọi Edition 1 ⇒ **0% độ phủ fuzz cho toàn bộ cú pháp Edition 2 (`?`, `let` destructuring, `p | q`)**.
  - `crates/gt-secrets/src/lib.rs:171` (`scan_text`) gọi `lex(text)` (Edition 1).
  - `crates/rig/src/autotest.rs:156` (`targets`) gọi `gt_parser::parse(fname, text)` (Edition 1) — sẽ bỏ qua/lỗi cú pháp im lặng trên dự án Edition 2!
  - `crates/gt-driver/src/lsp.rs:171` dùng `Options::default()` (Edition 1; đã có ghi nhận ở `KNOWN_LIMITATIONS.md:395`).
- **G-09b — Các điểm nhỏ khác:**
  - `crates/rig/src/autotest.rs:287`: `vals.iter().map(Vec::len).product()` có thể tràn `usize` nếu hàm có nhiều tham số.
  - `crates/gt-kernel/src/search.rs:25,104`: `a.abs()` và `-q.lin.coef[&k]` có thể panic trên `i128::MIN` (nằm ngoài TCB kernel `check.rs`, không làm sai chứng chỉ nhưng gây panic trong pass tìm kiếm).
  - `crates/gt-driver/src/runner.rs:19-24`: `find_main` chỉ lọc tên `::main` và `arg_count == 0`, không lọc `b.kind == BodyKind::Fn`. Thực nghiệm với `■ main: u32 ≔ 42u32;` (`bug12_static_main.gt`): `gritc run --backend=interpreter` chạy và trả `Int(42, U32)` (exit 0), còn `gritc build --backend=llvm` sập **ICE exit 2** (`không có hàm main không tham số ở cấp tệp`) thay vì lỗi dùng exit 3.
  - `crates/gt-driver/src/pipeline.rs:485-504`: `attestation` không băm `edition`.
  - `crates/gt-llvm/src/shim.rs`: các ràng buộc thanh ghi x86_64/cỡ con trỏ chưa có `#[cfg(target_arch = "x86_64")]` (hoặc `compile_error!`) khi biên dịch chéo kiến trúc.

---

## 3. Các giả thuyết tấn công đã BÁC BỎ trong phạm vi thực nghiệm

| Giả thuyết tấn công | Mẫu thử | Kết quả thật | Kết luận |
|---|---|---|---|
| Hậu điều kiện `⊲` đọc/move `Owned<T>` không `Copy` | `bug1`, `bug1b` | `E0902` (ĐỎ, exit 1) | **Bác bỏ** |
| Dùng `root_out()` (`Cap<Out>`) trong `■` static | `bug3c` | `E0310` (ĐỎ, exit 1) | **Bác bỏ** |
| `■ M: Mutex<Owned<u32>, L1>` | `bug5c` | `E0501` (`Owned` không `Sync`) + `E0310` (ĐỎ, exit 1) | **Bác bỏ** |
| Con trỏ hàm `ƒ(&u32) → &'static u32` leo thang lifetime | `bug_uaf_static_fnptr` | `E0404` (ĐỎ, exit 1) | **Bác bỏ** |
| `Vec<'a, T>` generic chứa `Secret<u32>` / `Owned<u32>` | `bug5d`, `bug5e` | `E0318` ngay ở M4 `gt-types` (ĐỎ, exit 1) | **Bác bỏ** |
| Tên hàm `fast` / `exact` kích nhầm `UB_FLAGS` của `gt-guard` | `bug11` | `green`, LLVM build & chạy exit 0 (mangling bảo vệ) | **Bác bỏ** |
| Edition 2 `?`, `let` destructuring, or-pattern với `Owned` & `Secret`, bind trùng, ref ergonomics | 8 mẫu `ed2_*` (`try_owned`, `try_secret`, `letpat_owned_wild`, `letpat_secret_wild`, `letpat_dup_bind`, `or_ref_ergonomics`, `letpat_or`, `or_owned_partial`) | Bind trùng bị chặn `E0202`; 7 mẫu hợp lệ đều chạy khớp với `gritrt_heap_live=0` & wipe đầy đủ (`ed2_letpat_ref_temp` bị `E0404` — bảo thủ, an toàn) | **Bác bỏ** trong phạm vi 8 mẫu |

---

## 4. Độ phủ đọc mã theo bảng ưu tiên (`docs/E_PROMPT_OPUS.md`) & Phase 2

| Ưu tiên | Vùng / Tệp đã đọc | Phạm vi đọc thật (dòng) | Phát hiện / Kết luận |
|---|---|---:|---|
| Sau G1.5 | `runtime/gt-rt/src/lib.rs`, `crates/gt-guard/src/lib.rs`, `crates/gt-llvm/src/shim.rs`, `crates/gt-types/src/validate.rs`, `crates/gt-driver/src/lsp.rs`, `crates/gt-mir/src/interp.rs` | 1 257 + 522 + 61 + đọc phần sửa đổi (~2 400 dòng) | **G-04** (`interp.rs:859`), **G-08a** (`gritrt_alloc(0)`), **G-09a** (`lsp` ed1), **G-09b** (`shim.rs` arch guard). `gt-guard` & phần `gt-rt` khác sạch trong phạm vi đọc. |
| 1 | `crates/gt-oblig/src/{analysis,driver,unsafe_check}.rs` + `crates/gt-kernel/src/{lib,linear,check,search}.rs` | toàn bộ `gt-kernel` (590) + các hàm quyết định `gt-oblig` (~1 800) | **G-02** (`pure_preds` + `■ Atomic`), **G-03d** (`unsafe_check` bỏ `Static/Const/Pre`), **G-06** (`narrow`/`check_post_fixpoint` thiếu kiểm `facts`/`defs`), **G-09b** (`search.rs` `i128::MIN`). TCB `gt-kernel/check.rs` gọn, sạch. |
| 2 | `crates/gt-borrowck/src/{lib,check,regions,solve}.rs` | **100%** (1 865 dòng) | **G-01 (NGHIÊM TRỌNG — Safe-Grit UAF stack & heap `Owned`)** tại `regions.rs:546-566`. Các phần còn lại (WF, `conflicts`, `solve`) sạch trong phạm vi đọc. |
| 3 | `crates/gt-types/src/{send,validate,db,collect,exhaust,check/{call,pat,expr,mod}}.rs` | các hàm quyết định (~2 800 dòng) | **G-07a** (`collect.rs` thiếu `check_marker_effects`), **G-07c** (`exhaust.rs` nổ mũ or-pattern), **G-07d** (`?` trong `□` const). `send.rs` và `Vec<T>` E0318 chặt. |
| 4 | `crates/gt-term/src/lib.rs` | hàm chính & vòng lặp (~300 dòng) | **G-03a** (`BodyKind::Fn` bỏ lọt `Static`/`Const`/`Pre`/`Post` → 5 đường treo vĩnh viễn) + ảnh hưởng gián tiếp từ **G-02**. |
| 5 | `crates/gt-effects/src/lib.rs`, `crates/gt-conc/src/lib.rs`, `crates/gt-secrets/src/lib.rs` | 395 + 234 + 450 = 1 079 dòng | **G-03b** (vượt E0502 & E0801), **G-03c** (vượt E0503 → LLVM ICE), **G-03e** (vượt S201/S202 trong `■`), **G-09a** (`scan_text` ed1). |
| 6 | `crates/gt-guard/src/lib.rs` | **100%** (522 dòng) | Không tìm thấy lỗ hổng vượt Guard mới; giới hạn `GUARD-02` giữ nguyên như `TCB_CODE_AUDIT_CLAUDE.md`. |
| 7 | `crates/gt-driver/src/{verdict,pipeline,native,cli,runner,lsp}.rs` | `verdict.rs` (113) + `pipeline.rs` (505) + `native.rs` (446) + phần chính `cli`/`runner`/`lsp` (~1 500) | `verdict.rs` và `native.rs` đóng chặt; tìm thấy **G-08b** (`GRITC_FAULT=mir:pat-field[-swap]` ở release), **G-09b** (`find_main`, `attestation`). |
| Mở rộng (Phase 2 & hạ tầng) | `crates/gt-dataflow/src/{elaborate,analyses}.rs` (100%: 1 197), `crates/gt-mir/src/validate.rs` (100%: 499), `crates/gt-codegen-llvm/src/lower.rs` (các đường `Owned`/`Secret`/`Spawn`/`Drop`/`gep`), `crates/gt-parser/src/{pats,exprs}.rs`, `crates/rig/src/{autotest,cli,project}.rs`, 3 fuzz target, `lib/core/prelude.gt` | ~3 200 dòng | `validate.rs` sạch; tìm thấy **G-04** (`OwnedIntoInner` & `Spawn` trampoline), **G-05** (`elaborate.rs` generic `Cell`/`Mutex` ICE), **G-07b** (`pats.rs` `..` trong `\|`), **G-09a/b** (`rig`, fuzz target). |

### Đối chiếu 7 nghi vấn mở từ R11b (`docs/E_PROMPT_OPUS.md`)

| ID | Trạng thái tại `f342ae4` (đối chiếu từ góc nhìn Gemini trên Linux) |
|---|---|
| `ARENA-01` | Đã có test ngẫu nhiên 10 800 cấp phát ở GÓI 3; vẫn giữ nguyên giới hạn: release mặc định không bật kiểm chồng lấn. Không phát hiện lỗi mới. |
| `VEC-01` | Đọc `gt-rt` và `lower.rs`: không phát hiện đường tràn mới đạt tới từ Grit. |
| `ATOMIC-01` | **Đã chạy TSan thật trên `f342ae4` (Linux x86_64):** 0 data race trên 400 lượt chính thức + 20 lượt mẫu Phase 2 `Secret`+`spawn`+`Atomic` (xem mục 5). **VẪN `CHƯA KIỂM`** trên phần cứng bộ nhớ yếu (ARM64) và Windows SRWLOCK. |
| `GUARD-02` | Xác nhận vẫn là giới hạn thiết kế đã khoá bằng test. |
| `ATTEST-01` | Xác nhận vẫn là giới hạn thiết kế; phát hiện thêm `attestation` chưa băm `edition` (**G-09b**). |
| `UNSAFE-01` | Đã chạy `python3 tools/unsafe_inventory.py --check` → `UNSAFE_INVENTORY_OK 103 chỗ trong 3 tệp` (exit 0). Phát hiện thêm `gritrt_alloc(0)` chưa bảo vệ cỡ 0 (**G-08a**). |
| `argv` đầu-cuối | `CHƯA KIỂM` trên Windows (`cmd`/`PowerShell`/`Git Bash`) vì phiên này chạy trên Linux. |

---

## 5. Kết quả thực nghiệm động trên Linux (`f342ae4`)

### 5.1. Preflight & các bộ kiểm tĩnh Python

| Lệnh | Exit code | Kết quả thực tế |
|---|---:|---|
| `bash tools/preflight.sh /home/antigravity/Vecter-Core/Grit` | 0 | `PREFLIGHT_OK` |
| `./target/release/gritc doctor` | 0 | `doctor: XANH` |
| `python3 tools/check_expectations.py` | 0 | `Đã kiểm 1108 file .gt: fail=588, warn=13, pass=450, lexer=9, ed2/pass=13, ed2/fail=17; OK` |
| `python3 tools/unsafe_inventory.py --check` | 0 | `UNSAFE_INVENTORY_OK 103 chỗ trong 3 tệp` |
| `python3 tools/version_docs.py` | 0 | `VERSION_DOCS_OK` |
| `python3 tools/doc_test.py` | 0 | `DOC_TEST: 220 khối grit, 0 sai` |
| `python3 tools/fuzz_total.py` | 0 | Lịch sử cũ: `lex` 1,22h (0,00h trên hash hiện tại `a6b1cf08994d`), `parse` 2,02h (0,00h trên hash hiện tại `65c7be973acf`) |

### 5.2. ThreadSanitizer (TSan) trên HEAD `f342ae4` (Linux x86_64, nightly + `-Zbuild-std`)

1. Dựng runtime TSan: `RUSTFLAGS="-Zsanitizer=thread" cargo +nightly build --manifest-path runtime/gt-rt/Cargo.toml -Zbuild-std=std,panic_abort --target x86_64-unknown-linux-gnu --release` → **exit 0** (55,95s).
2. Chạy bộ kiểm chính thức `GRITC=.../target/release/gritc bash tools/tsan_check.sh 50` → **exit 0**:
   - **5 đối chứng dương (30 lần/mẫu — đều BẮT được đua dữ liệu):** `race_pthread=30/30`, `race_gritrt_spawn=29/30`, `race_narrow_ir=18/30`, `grit_two_locks_nolock=21/30`, `grit_narrow_droplock_nolock=18/30`.
   - **3 đối chứng âm (30 lần/mẫu — 0 báo động giả):** `norace_atomic_pthread=0/30`, `grit_two_locks_lock=0/30`, `grit_narrow_droplock_lock=0/30`.
   - **8 mẫu Grit thật × 50 lần = 400 lượt chạy** (`threads`, `two_locks`, 6 mẫu `E0501_ok_*`): **0 `bad_exit`, 0 `data_race_reports`** (`TSAN_RESULT reports_total=0`).
3. **Thử nghiệm TSan mở rộng cho Phase 2 (`Secret` + `spawn` + `Atomic`, 4 luồng × 20 lượt):**
   - **0 `bad_exit`, 0 `data_race_reports`** — nhưng báo cáo `GRITRT_SECRET_REPORT=1` cho thấy `gritrt_secret_wipes=12 bytes=96` thay vì `16` (xác nhận thêm **G-04**).

### 5.3. Fuzzing (libFuzzer / ASan trên `f342ae4`)

> Lưu ý: Không ghi đè `docs/FUZZ_LOG.jsonl` vì phiên này chỉ đọc (không sửa tệp theo dõi của kho). Cả 3 đích fuzz hiện chỉ kiểm Edition 1 (**G-09a**).

| Đích fuzz | Thời gian | Số lượt chạy | Tốc độ TB | Peak RSS | Exit code | Sập / Treo / Artifact |
|---|---:|---:|---:|---:|---:|---|
| `gt-lexer` (`lex`, seed toàn bộ corpus `tests/`) | 121s | 265 899 | 2 197 exec/s | 79 MB | 0 | 0 |
| `gt-parser` (`parse`, seed toàn bộ corpus `tests/`) | 121s | 73 118 | 604 exec/s | 116 MB | 0 | 0 |
| `gt-driver` (`check`, lượt 1 — nạp toàn bộ 1 067 seed `tests/`) | 270s | 1 068 | 3 exec/s | 250 MB | 0 | 0 sập; 1 `slow-unit` 17s (mẫu hồi quy `E0702` 24 biến bóng dưới ASan) |
| `gt-driver` (`check`, lượt 2 — đột biến thật trên 25 seed ngắn) | 61s | 2 063 (285 mẫu mới) | 33 exec/s | 127 MB | 0 | 0 |

---

## 6. Phần KHÔNG đọc và các mục `CHƯA KIỂM`

- **Windows (`CHƯA KIỂM`):** Môi trường này là Linux Ubuntu 24.04 x86_64. Toàn bộ đường mã đặc thù Windows (`SRWLOCK` trong `gt-rt`, liên kết MSVC/COFF, xử lý đường dẫn/argv trên `cmd`/`PowerShell`/`Git Bash`, và `tools/gate.sh` trên Windows) đều **`CHƯA KIỂM`**.
- **Phần cứng bộ nhớ yếu / ARM64 (`CHƯA KIỂM`):** x86_64 (TSO) và TSan không thay thế được kiểm tra thực tế trên ARM64 hoặc model checker (loom/CDSChecker) cho các thứ tự `Relaxed`/`Acquire`/`Release`.
- **Fuzz dài hạn (`CHƯA KIỂM`):** Phiên này chỉ chạy 2 phút/đích (tổng ~6 phút) làm kiểm tra khói trên `f342ae4`, chưa đạt chuẩn ≥ 24h mỗi đích trên hash hiện tại và chưa fuzz được Edition 2 do **G-09a**.
- **Cổng đầy đủ `tools/gate_clean.sh` (`CHƯA KIỂM`):** Tuân thủ `E_PROMPT_OPUS.md` quy tắc 3 (không chạy cổng đầy đủ trong phiên đọc mã).
- **Mã nguồn ngoài phạm vi đọc sâu (~50 000 dòng):** Phần lớn `crates/gt-codegen-llvm/src/lower.rs` ngoài các đường `Owned`/`Secret`/`Spawn`/`Drop`/`gep`, `crates/gt-mir/src/build/expr.rs` và `interp.rs` ngoài các đường đã kiểm, `crates/gt-fmt`, `crates/gt-resolve`, `crates/gt-ast`, `crates/gt-lexer`, `crates/gt-diag`, và hầu hết mã test trong `crates/*/tests/`.

---

## 7. Đề xuất thứ tự xử lý cho kiến trúc sư (KHÔNG sửa mã trong nhánh này)

1. **Khẩn cấp (chặn phát hành / hỏng an toàn bộ nhớ):** Sửa **G-01** (`regions.rs:546-566` — Safe-Grit UAF qua `Callee::Value`), thêm test ĐỎ tái hiện cả stack lẫn heap `Owned` + mở rộng bộ sinh kiểm tra con trỏ hàm với `Cell`/`&!T`, ghi `FB-C36`.
2. **Ưu tiên 1 (Soundness M7/M8/M9):**
   - Sửa **G-02** (`pure_preds` không nhận hàm chạm `■ Atomic` hoặc `⚠`), ghi `FB-C37`.
   - Sửa **G-03** (bỏ lọc `BodyKind::Fn`-only ở 5 pass `gt-term`, `gt-effects`, `gt-conc`, `gt-oblig::unsafe_check`, `gt-secrets`; bổ sung ma trận test E2E cho `Static`/`Const`/`Pre`/`Post`), ghi `FB-C38`.
   - Sửa **G-06** (thêm kiểm `facts`/`defs` vào `check_post_fixpoint` và `narrow`).
3. **Ưu tiên 2 (D13 Secret & Post-Green ICE):**
   - Sửa **G-04** (`gritrt_secret_wipe` trong `Owned::into_inner` và `spawn` trampoline; chốt chính sách cho `■ Mutex<Secret<T>>`).
   - Sửa **G-05** (kiểm `E0318` cho `Cell<T>`/`Mutex<T, L>` trên generic / bảng `instances`).
4. **Ưu tiên 3 (Edition 2, TCB FFI & Hardening):**
   - Sửa **G-07a–d** (`check_marker_effects` trong `fill_trait`/`fill_impl`, cấm `..` trong `|`, chặn nổ mũ `exhaust.rs`, khoá `?` trong `□` const).
   - Sửa **G-08a–b** (`size.max(1)` trong `gritrt_alloc`; đưa `GRITC_FAULT=mir:pat-field[-swap]` ra khỏi build release).
   - Sửa **G-09a–b** (cập nhật 3 fuzz target, `gt-secrets`, `rig test --auto` hỗ trợ Edition 2; chạy lại cổng Windows + fuzz dài hạn trên mã sau khi sửa).
