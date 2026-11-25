# Đọc mã đối kháng KHÁC HỌ model — Đợt 2: kiểm chéo bản sửa FB-C36..C51, GLIR/Layout, IDE/LSP, Fuzz & CI (Họ Gemini, 2026-11-25)

**Reviewer:** Họ model **Gemini** (**KHÁC HỌ** với tác giả chính Claude; nhánh `model-gemini-neon`).  
**HEAD đọc và thử nghiệm:** `911dc25` (`origin/main` sau PR #18; fast-forward sạch từ đợt 1 `83366cf` — không sửa bất kỳ mã sản phẩm, test hay trạng thái TCB/STATUS nào).  
**Môi trường cục bộ:** Linux Ubuntu 24.04 x86_64, `clang-20` / `llvm-20-dev` (20.1.2), Rust stable + nightly (`rustc 1.102.0-nightly 20fe935d5`), `cargo-fuzz 0.13.2`, `gritc doctor` → `doctor: XANH` (exit 0).  
**Môi trường CI đám mây (GitHub Actions):**
- `ci.yml` (`workflow_dispatch`, `only=all`, `full=true`): run [`45613112950`](https://github.com/Vecter-Core/Grit/actions/runs/45613112950)
- `fuzz.yml` (`workflow_dispatch`, `minutes=330` song song `lex` + `parse`): run [`45613108859`](https://github.com/Vecter-Core/Grit/actions/runs/45613108859)

> **Tuyên bố trung thực (`docs/SAFETY_CLAIMS.md` §6):** Đợt đọc chéo này chỉ kiểm tra phạm vi mã mới từ `f342ae4..911dc25` (bản sửa `FB-C36..C51`, `gt-layout`, `gt-glir`, `gt-glirgen`, `S203`), giao thức `gritc lsp` + `editors/vscode`, fuzz cục bộ 5 phút cho `gt-driver check` (Edition 1 & 2) và CI GitHub Actions — **KHÔNG phải chứng minh hình thức**, **KHÔNG đổi trạng thái TCB/STATUS**, và **KHÔNG thay thế việc mở cửa sổ VS Code thật trên màn hình**.

---

## 1. Tóm tắt phát hiện mới (Đợt 2 trên `911dc25`)

| Nhóm | Mã | Mức độ | Tóm tắt ngắn | Trạng thái xác minh |
|---|---|---|---|---|
| **Compiler Soundness** | **G2-01** | **NGHIÊM TRỌNG (D12 + Segfault)** | Chu trình hằng `□ A: u32 ≔ get_a(); ƒ get_a() → u32 ! ∅ { A }` vượt `gritc check` (`green`), dựng LLVM exit 0 (qua IR Guard), chạy **Segmentation fault (exit 139)** trong Grit thuần `! ∅` | **Xác nhận thực nghiệm** |
| **Compiler Soundness** | **G2-02** | **NGHIÊM TRỌNG (Vượt FB-C39)** | `□ TICK: u32 ≔ A.fetch_add(1u32, SeqCst);` vượt hoàn toàn `impure_fns` (`FB-C39`): **treo vĩnh viễn (exit 124)** dưới `--checks=all` mặc định & xoá nhầm `⊳` dưới `--checks=proven` (exit 0 thay vì abort 70) | **Xác nhận thực nghiệm** |
| **Compiler Soundness** | **G2-03** | **CAO (Vô hiệu hoá FB-C40)** | Commit `7d26f53` truyền `strict_meta = rep.narrowing_rejected` vào `check_post_fixpoint`: khi narrowing được chấp nhận (trường hợp thông thường), `facts`/`defs` của Phase 2 `linear_facts` **vẫn không được kiểm** trước khi đưa vào `gt-kernel` | **Xác nhận qua đọc mã** |
| **GLIR / Oracle (Phase 2)** | **G2-04** | **CAO (Oracle & Verifier mới)** | `gt-glir::interp` không huỷ/đầu độc slot stack của hàm con sau khi `ret` (bỏ sót stack UAF/dangling ptr); `gt-glir::verify` chấp nhận `Phi` ở khối vào `bb0` có cạnh lùi nhưng `interp` báo `Ub("phi ở khối vào")`; thiếu kiểm trùng `decl`/`func`, `Call` gán `Void`, `Ty::Void` ở param/slot, tràn `off+total`/`round_up`; `parse.rs` nuốt `%x = store` và nhận `slot $` giữa khối | **Xác nhận qua đọc mã** |
| **IDE / LSP** | **G2-05** | **CAO (False-green im lặng trong IDE)** | Mở bất kỳ tệp nào trùng tên biến cục bộ/tham số của `lib/std/src/std.gt` (đã đo **8/17 tên phổ biến**: `ok.gt`, `res.gt`, `a.gt`, `b.gt`, `s.gt`, `v.gt`, `n.gt`, `i.gt`) làm `std.gt` lỗi `E0205`, `m4.types` dừng sớm và `lsp.rs:176` lọc `d.file == me` ⇒ IDE báo **`[]` (0 lỗi)** kể cả trên mã sai rõ rệt! | **Xác nhận thực nghiệm (JSON-RPC)** |
| **IDE / LSP** | **G2-06** | **TRUNG BÌNH–CAO (UX IDE)** | `my-app.gt`, `123.gt` báo `E9901: BadModuleName` vì `lsp.rs:137-144` thiếu `module_file_name_for`; mở tệp Edition 2 (`examples/rpn/src/rpn.gt`) báo 18 lỗi `E0101`, 0 symbol, `formatting: null`; `uri_to_path` trên Windows giữ `/` đầu `/c:/...`; `grit.tmLanguage.json` thiếu các kiểu/toán tử Phase 2 (`Owned`, `Cell`, `Mutex`, `Atomic`, `⚠`, `∅`, `?`, `\|`, `#level`) | **Xác nhận thực nghiệm + đọc mã** |
| **CI / Fuzz 24h** | **G2-07** | **CAO (4 lỗi CI thật)** | (a) `.github/workflows/fuzz.yml` chạy `cargo fuzz run` thiếu `+nightly-2026-10-04` ⇒ bị `rust-toolchain.toml` (`1.98.1`) ghi đè, **đỏ sau 0s** trên cả `lex` & `parse` (`45613108859`); (b) `deny.toml` chưa cho phép `GPL-3.0-only` của workspace ⇒ job `deny` đỏ ở mọi push; (c) `linux-gate`: `tools/gate.sh` **XANH**, nhưng bước diff danh sách test đỏ do `docs/platform_tests_windows.list` cũ (1 009 vs 1 026 test Linux); (d) `windows-core (full=true)`: 23/24 crate + 215/216 test `gt-driver` **XANH**, duy nhất `llvm.rs:354::built_exe_does_not_depend_on_llvm_dll` **ĐỎ** vì `bcryptprimitives.dll` trên Rust 1.98 Windows | **Xác nhận thực nghiệm trên GitHub Actions** |
| **Secrets (nhỏ)** | **G2-08** | **THẤP** | `□ C: Secret<u32> ≔ Secret::new(42u32);` nhúng hằng secret trong binary như `■` nhưng không phát cảnh báo vàng `S203` (`gt-secrets/src/lib.rs:428` chỉ kiểm `BodyKind::Static`) | **Xác nhận thực nghiệm** |

---

## 2. Kiểm tra chéo bản sửa `FB-C36..C51` (từ đợt Gemini 1) và các lỗi mới

### 2.1. G2-01 [NGHIÊM TRỌNG]: Đệ quy vô hạn qua `□` hằng (`Const`) → Segfault (exit 139) trong Grit thuần `! ∅`
- **Vị trí:** `crates/gt-term/src/lib.rs:749,877` (sau `FB-C38`) + `crates/gt-codegen-llvm/src/lower.rs` (chỉ kiểm chu trình khởi tạo cho `Static`, không kiểm `Const`).
- **Đối chiếu `docs/KNOWN_LIMITATIONS.md:389`:** Tài liệu chỉ ghi nhận chu trình cho `■` static (được LLVM chặn không sinh binary), nhưng **bỏ sót hoàn toàn `□` const** (được hạ thành lời gọi hàm mỗi lần đọc nên vượt qua cả `gt-term` lẫn LLVM và IR Guard!).
- **Mẫu tái hiện (3 dòng, `! ∅` thuần):**
  ```text
  □ A: u32 ≔ get_a();
  ƒ get_a() → u32 ! ∅ { A }
  ƒ main() → u32 ! ∅ { A }
  ```
- **Kết quả thật trên `911dc25`:**
  - `./target/release/gritc check --json`: **`verdict: "green"`, 0 lỗi, exit 0**.
  - `./target/release/gritc run --backend=interpreter`: **ICE exit 2** (`hằng ngoài phạm vi thông dịch`).
  - `./target/release/gritc build --backend=llvm --profile=dev`: **exit 0** (vượt IR Guard!).
  - Chạy binary: **`Segmentation fault` (exit 139)** do tràn stack đệ quy vô hạn.

### 2.2. G2-02 [NGHIÊM TRỌNG]: Vượt bản sửa `FB-C39` (`impure_fns`) bằng `□` const gọi `■ Atomic`
- **Vị trí:** `crates/gt-oblig/src/driver.rs:502-534` (`impure_fns`).
- **Nguyên nhân:** `impure_fns` chỉ duyệt `b.kind == BodyKind::Fn` (dòng 525) và chỉ kiểm `Operand::Const(Const::Item { node })` nằm trong tập `hot` các `DefKind::Static` có `may_have_interior_mut`. Hằng `□ TICK: u32 ≔ A.fetch_add(1u32, AtomicOrdering::SeqCst);` có kiểu `u32` (không nằm trong `hot`) và là `BodyKind::Const` (không bị duyệt, không thêm cạnh gọi), nên hàm `is_ok` đọc `TICK` vẫn được đưa vào `pure_preds`!
- **Đối chiếu `docs/KNOWN_LIMITATIONS.md:399`:** Phiên trước chỉ thử `□ R: &Atomic ≔ &A` (bị `E0404`), chưa thử hằng `u32` gọi phương thức trên `■ Atomic`.
- **Mẫu 1 — Treo vĩnh viễn ngay dưới mặc định `--checks=all`:**
  ```text
  ■ A: Atomic<u32> ≔ Atomic::new(0u32);
  □ TICK: u32 ≔ A.fetch_add(1u32, AtomicOrdering::SeqCst);
  ƒ is_ok(x: u32) → bool ! ∅ { TICK == 0u32 }
  ƒ main() → u32 ! ∅ {
    x: u32 ≔ 1u32;
    ⎇ is_ok(x) { ⎇ ¬is_ok(x) { ⟳ true ↓ x { } } };
    0u32
  }
  ```
  - `gritc check`: **`verdict: "green"`, exit 0**; `gritc build --backend=llvm --profile=dev`: **exit 0**; chạy binary (`timeout 2`): **treo vĩnh viễn, exit 124**.
- **Mẫu 2 — Xoá nhầm `⊳ is_ok(x)` dưới `--checks=proven`:**
  - `--checks=all`: abort `abort: precondition` (**exit 70**).
  - `--checks=proven`: kernel xoá nhầm kiểm tiền điều kiện → binary thoát **exit 0**!

### 2.3. G2-03 [CAO]: `FB-C40` (`7d26f53`) vô hiệu hoá kiểm `facts`/`defs` khi narrowing thành công
- **Vị trí:** `crates/gt-oblig/src/analysis.rs:2336-2361, 2534-2542`.
- **Nguyên nhân:** Sau `narrow(&cx, &mut states)`, mã chỉ gọi `check_post_fixpoint(&cx, &states, rep.narrowing_rejected)`. Khi narrowing không bị bác (`rep.narrowing_rejected == false` — phần lớn hàm thực tế), `strict_meta` là `false`, nên `t.facts` và `t.defs` **không hề được kiểm tra** trước khi nạp vào `gt-kernel` (ngay cả trên `pre` trước narrowing cũng không được kiểm).

### 2.4. Trạng thái các bản sửa `FB-C36..C51` khác đã kiểm chéo
- **FB-C36 (`6c65f29`, `regions.rs:564`):** Đã thử thêm biến thể 2 đối số (`fp: ƒ(&Cell<&u32>, &u32) ! ∅ ≔ put; fp(&c, &local);`) → bị chặn đúng **`E0404`**. Bản sửa vững trong phạm vi đã thử.
- **FB-C35/C38 (`r2_contract_self_rec.gt`, `r2_contract_mutual.gt`):** Đệ quy qua `⊳`/`⊲` trực tiếp và tương hỗ đều bị chặn đúng **`E0703`**.
- **FB-C41 (`3994452`):** `Owned::into_inner` và `spawn` trampoline đã gọi `secret_wipe` đúng chỗ cả LLVM và interpreter.
- **FB-C42..C51 (`f6ef87c`, `b891504`, `e27ff6b`, `90e5379`):** Đã đọc toàn bộ diff; sạch ngoại trừ ghi chú nhỏ **G2-08** (`□ C: Secret<u32>` chưa phát `S203`) và **G2-07** (`fuzz.yml` chưa chạy được trên CI).

---

## 3. Đọc chéo 3 crate Phase 2 mới (`gt-layout`, `gt-glir`, `gt-glirgen`) — G2-04

Đã đọc 100% mã nguồn mới (`crates/gt-layout/src/lib.rs` 175 dòng; `crates/gt-glir/src/{lib,layout,parse,print,verify,interp}.rs` 1 783 dòng; `crates/gt-glirgen/src/{lib,body}.rs` 1 118 dòng; tổng **3 076 dòng**):

1. **`gt-glir/src/interp.rs:313-316, 366, 506-511` (Oracle bỏ sót stack UAF / dangling pointer):** Slot cục bộ của hàm được gọi (`exec`) được `push` vào `mem: &mut Vec<Vec<Byte>>` dùng chung nhưng **không bao giờ đánh dấu đã giải phóng / đầu độc sau khi hàm trả về**. Nếu bước hạ sinh con trỏ treo trỏ tới slot cục bộ đã trả về, oracle `gt-glir::interp` vẫn cho phép đọc/ghi hợp lệ thay vì trả `Outcome::Ub`.
2. **`gt-glir/src/verify.rs:186-198, 347-358` lệch với `interp.rs:325-328`:** Verifier chấp nhận `Inst::Phi` đặt trong khối vào `bb0` nếu có cạnh lùi về `bb0` (`pred[0]` không rỗng), nhưng `interp.rs` chạy tới sẽ trả ngay `Outcome::Ub("phi ở khối vào")` vì lúc vào hàm `prev == None`.
3. **Các lỗ hổng phòng thủ khác trong `gt-glir::verify`:**
   - `verify.rs:58-68`: không kiểm trùng tên trong `m.decls` và không kiểm va chạm tên giữa `m.decls` với `m.funcs` (dòng 312 ưu tiên im lặng `decls`).
   - `verify.rs:161-165, 309-346`: `Inst::Call { d: Some(x), ret: Ty::Void, .. }` không bị từ chối (gán kiểu `Void` cho biến SSA `%x`).
   - `verify.rs:154-158, 293-297`: không cấm `Ty::Void` làm kiểu tham số, slot, `Load`/`Store`/`Select`/`Phi`; `Opd::Int(t, _)` (dòng 171) không kiểm `t.is_int()`.
4. **Tràn số & parser trong `gt-glir` / `gt-layout`:**
   - `gt-glir/src/layout.rs:12, 61`: `Array` dùng `el.size.saturating_mul(*n)` (bão hoà im lặng thành `u64::MAX`), sau đó `round_up(n, a)` tính `n.div_ceil(a) * a` tràn `u64` (khác với `gt-layout/src/lib.rs:110` dùng `checked_mul`, tuy `struct_layout` của cả hai vẫn dùng `+` không kiểm tràn).
   - `gt-glir/src/interp.rs:462`: `off + total > st.mem[a].len() as u64` dùng cộng chưa kiểm tràn (so với dòng 101 dùng `checked_add`).
   - `gt-glir/src/parse.rs:133, 201-211`: `%x = store i32 ...` bị bỏ qua `%x` im lặng không báo lỗi; `parse.rs:368-377` chấp nhận `slot $s: T` đặt xen giữa các khối/lệnh.
5. **`gt-glirgen/src/body.rs`:**
   - Dòng 835: `ok.insert(f.name.clone(), f)` ghi đè im lặng nếu va chạm băm tên 24-bit `.{:06x}`.
   - Dòng 913–914: bỏ qua `StorageLive`/`StorageDead`, nên biến cục bộ vòng lặp giữ nguyên byte từ lượt trước sang lượt sau trong `gt-glir::interp` (không bắt được đọc chưa khởi tạo giữa các lượt lặp).
   - Dòng 943–996 (`check_abort_cover`): giống giới hạn `GUARD-02`, `CondBr` luôn đẩy cả 2 nhánh kể cả khi điều kiện là hằng `i1 1`/`i1 0`, và chỉ so tổng số khối `Abort` theo từng loại.

---

## 4. Kết quả thử IDE (`gritc lsp` + `editors/vscode`) — G2-05, G2-06

### 4.1. Thử nghiệm giao thức JSON-RPC thật với `./target/release/gritc lsp`
- **Các luồng hoạt động tốt (trên tên tệp Edition 1 không trùng biến của `std.gt`, vd `demo.gt`, `foo.gt`, `main.gt`):**
  - `initialize` → trả đúng 5 capability (`textDocumentSync`, `documentFormattingProvider`, `hoverProvider`, `definitionProvider`, `documentSymbolProvider`), `serverInfo: gritc 1.0.0-rc7`.
  - `didOpen` / `didChange` / `didClose` + `publishDiagnostics`: bắt đúng `E0601`, `E0301`, `E0404` (severity 1) và `S201` (severity 2) với cột UTF-16 chính xác sau các ký tự Unicode đa byte (`ƒ`, `→`, `≔`, `∅`).
  - `textDocument/hover`: trả markdown chứa đúng mã lỗi + tài liệu `gritc explain`.
  - `textDocument/definition` & `textDocument/documentSymbol`: trả đúng định nghĩa cấp item (`helper`, `main`).
  - `textDocument/formatting`: định dạng được mã Edition 1 hợp lệ.
  - Yêu cầu không hỗ trợ (`textDocument/completion`) trả `-32601`; `shutdown` + `exit` thoát mã `0`, `stderr` rỗng.
- **Lỗi nghiêm trọng phát hiện khi thử thật:**
  1. **G2-05 (`lsp.rs:161-177`) — Mất trắng toàn bộ chẩn đoán theo tên tệp:** `build_inputs` nạp `lib/std/src/*.gt` vào chung không gian tên module với tệp đang mở. Khi tên tệp trùng biến cục bộ/tham số trong `std.gt` — đã đo thực tế **8/17 tên phổ biến bị hỏng: `ok.gt`, `res.gt`, `a.gt`, `b.gt`, `s.gt`, `v.gt`, `n.gt`, `i.gt`** — `gt-resolve` báo `E0205` gắn trên `file: "std.gt"`, `m4.types` dừng sớm, và bộ lọc `d.file == me` xoá luôn lỗi đó ⇒ **IDE trả `diagnostics: []` trên chương trình sai kiểu/sai mượn/tràn số!**
  2. **G2-06a (`lsp.rs:137-144`) — Tên tệp có dấu `-` hoặc chữ số đầu (`my-app.gt`, `123.gt`):** `build_inputs` truyền tên gốc vào `check()` mà không gọi `gt_resolve::module_file_name_for` (trái với `cli.rs:160` và `lsp.rs:311`), khiến IDE báo lỗi nội bộ **`E9901: BadModuleName`**.
  3. **G2-06b (`lsp.rs:171, 319, 519`) — Edition 2 hỏng hoàn toàn trong IDE:** Mở tệp thật của dự án (`examples/rpn/src/rpn.gt`) ⇒ 18 chẩn đoán `E0101` giả, `documentSymbol` = `0`, `formatting` = `null`.
  4. **G2-06c (`lsp.rs:62-82`) — Đường dẫn Windows URI:** `uri_to_path("file:///c%3A/Users/test/main.gt")` để lại dấu `/` đầu (`/c:/Users/test/main.gt`), làm hỏng `add_dir`/`is_file` khi mở dự án nhiều tệp trên Windows (đọc mã; Windows GUI `CHƯA KIỂM`).
- **Kiểm tra tĩnh `editors/vscode/`:**
  - `package.json`, `language-configuration.json`, `syntaxes/grit.tmLanguage.json`: hợp lệ cú pháp JSON (`json.load` exit 0).
  - `extension.js`: hợp lệ cú pháp JS (`node --check` exit 0).
  - Thiếu sót tô màu cú pháp trong `grit.tmLanguage.json`: thiếu các kiểu chuẩn (`Owned`, `Cell`, `Mutex`, `Atomic`, `AtomicOrdering`, `Ordering`, `File`, `IoError`, `Range`, `RangeInclusive`) và ký hiệu (`⚠`/`\unsafe`, `∅`, `?`, `|`, `..`, `#level`/`#allow`/`#lang`, hằng ký tự `'a'`).
  - **Mở cửa sổ VS Code GUI thật:** **`CHƯA KIỂM`** (VM Linux không có giao diện đồ họa).

---

## 5. Kết quả Fuzz & GitHub Actions CI (`911dc25`)

### 5.1. Fuzz cục bộ trong VM — `gt-driver` (`check`, Edition 1 + Edition 2)
- Preflight: `bash tools/preflight.sh` → `PREFLIGHT_OK` (exit 0).
- Seed: **67 tệp** (toàn bộ `tests/ed2/{pass,fail}/*.gt` + 25 tệp Edition 1 đa dạng).
- Lệnh: `cargo +nightly fuzz run check ... -max_total_time=300 -max_len=1024 -timeout=20 -rss_limit_mb=4096 -print_final_stats=1` (đã khôi phục `crates/gt-driver/fuzz/Cargo.lock` sau khi chạy).
- **Kết quả thật:** `exit=0`, **5 563 lượt chạy trong 301 giây** (5 495 đột biến thật, 524 mẫu corpus mới, 18 exec/s, peak RSS 222 MB; mỗi lượt chạy cả Edition 1 lẫn Edition 2 × 2 lần kiểm tất định = **22 252 lần gọi `gt_driver::check`**) — **0 sập, 0 ICE (`E9901`), 0 treo, 0 mất tất định**.

### 5.2. GitHub Actions `fuzz.yml` (lượt 1 — 330 phút) — run [`45613108859`](https://github.com/Vecter-Core/Grit/actions/runs/45613108859)
- **Kết quả:** **THẤT BẠI (exit 1 sau ~45 giây dựng, `seconds: 0`, `runs: 0`)** trên cả `fuzz (lex)` và `fuzz (parse)`.
- **Nguyên nhân xác nhận từ artifact (`G2-07a`):** `.github/workflows/fuzz.yml:46,57` chạy `rustup default nightly-2026-10-04` rồi gọi `cargo fuzz run` **không kèm `+nightly-2026-10-04`**; tệp `rust-toolchain.toml` (`channel = "1.98.1"`) ghi đè `rustup default` nên `rustc 1.98.1` từ chối cờ `-Zsanitizer=address`.
- **Hệ quả cho điều kiện #2 (24h fuzz):** Chưa tích luỹ được giờ nào từ CI; cần sửa 1 dòng trong `fuzz.yml` (`cargo +nightly-2026-10-04 fuzz run ...` hoặc `RUSTUP_TOOLCHAIN=nightly-2026-10-04`) rồi chạy 5 lượt × 330 phút.

### 5.3. GitHub Actions `ci.yml` (`only=all`, `full=true`) — run [`45613112950`](https://github.com/Vecter-Core/Grit/actions/runs/45613112950) (ĐÃ HOÀN TẤT)
Kết quả cuối cùng của toàn bộ 8 job trên HEAD `911dc25` (`12:22:38Z → 13:25:49Z`):

| Job | Kết luận cuối | Thời lượng | Chi tiết thực tế |
|---|---|---:|---|
| `changes` | **XANH (`success`)** | 11s | `tsan=true` |
| `linux-nollvm` | **XANH (`success`)** | 1m 50s | `gt-driver --no-default-features` qua |
| `determinism` | **XANH (`success`)** | 1m 24s | Dựng `gritc` release 2 lần khớp từng byte SHA-256 |
| `tsan` | **XANH (`success`)** | 5m 01s | ThreadSanitizer trên Linux `911dc25` qua sạch |
| `coverage` | `skipped` | — | Chỉ chạy khi `only=coverage` |
| `deny` | **ĐỎ (`failure`)** | 41s | **G2-07b:** `cargo-deny` từ chối `license = "GPL-3.0-only"` của chính các crate trong workspace vì `deny.toml` chưa liệt kê `GPL-3.0-only` (làm bỏ qua bước đối chứng `test_deny.py`) |
| `linux-gate` | **ĐỎ (`failure`)** | 43m 54s | **Cổng đầy đủ `bash tools/gate.sh .` ĐẠT XANH (`GATE_PLATFORM=linux`, `GATE_RESULT=GREEN`)!** Chỉ đỏ ở bước sau đó (`Diff danh sách test Linux và Windows` — **G2-07c**): `list_linux.txt` có **1 026** test còn `docs/platform_tests_windows.list` cũ chỉ có **1 009** test (chưa cập nhật sau khi thêm GLIR/layout/FB-C36..C51) ⇒ bỏ qua bước Capstone K1–K4 |
| `windows-core` (`full=true`) | **ĐỎ (`failure`)** | 1h 03m 09s | `runtime gt-rt`, `gritc doctor`, cổng nhẹ (`nul gtrt fmt clippy expect doc deletions failopen track`), test lõi, `audit`, **toàn bộ 23/24 crate khác** (`gt-span`…`gt-codegen-llvm`, `rig`), và **215/216 test `gt-driver`** đều **XANH**! Duy nhất **1 test ĐỎ** (**G2-07d**): `crates/gt-driver/tests/llvm.rs:354::built_exe_does_not_depend_on_llvm_dll` báo `DLL ngoài danh sách hệ thống/CRT: bcryptprimitives.dll` (do `std` Rust 1.98 trên Windows liên kết DLL hệ thống `bcryptprimitives.dll` cho `ProcessPrng`, chưa nằm trong allowlist của test) |

---

## 6. Mục `CHƯA KIỂM` & Đề xuất ưu tiên

- **`CHƯA KIỂM`:** (1) Fuzz 24h `lex`/`parse` trên CI (bị chặn bởi lỗi `G2-07a` trong `fuzz.yml`); (2) Bước Capstone K1–K4 trong `linux-gate` (bị bỏ qua sau khi `diff platform_tests_windows.list` đỏ); (3) Bước đối chứng `test_deny.py` (bị bỏ qua sau khi `cargo-deny` từ chối `GPL-3.0-only`); (4) Mở cửa sổ VS Code GUI thật với `editors/vscode`; (5) Phần cứng bộ nhớ yếu ARM64.
- **Đề xuất thứ tự sửa (KHÔNG sửa trong nhánh đọc kiểm này):**
  1. **Khẩn cấp (Soundness):** Sửa **G2-01** (đưa cạnh gọi của `BodyKind::Const` vào đồ thị `gt-term` / kiểm chu trình hằng), **G2-02** (đưa `BodyKind::Const` và `Const::Item` vào `impure_fns`), và **G2-03** (kiểm `strict_meta` cho `facts`/`defs` trên trạng thái widening `pre` và/hoặc trong `narrow`).
  2. **Ưu tiên 1 (CI & Fuzz 24h):** Sửa **G2-07a** (`fuzz.yml`: thêm `+nightly-2026-10-04` hoặc `RUSTUP_TOOLCHAIN`) để chạy đủ 5 lượt × 330 phút = 24h+ song song `lex`/`parse`; sửa **G2-07b** (`deny.toml`: cho phép `GPL-3.0-only` của workspace); sửa **G2-07c** (cập nhật `docs/platform_tests_windows.list` từ bản chạy Windows mới nhất); sửa **G2-07d** (thêm `bcryptprimitives.dll` vào danh sách DLL hệ thống Windows trong `crates/gt-driver/tests/llvm.rs`).
  3. **Ưu tiên 2 (GLIR & IDE):** Sửa **G2-04** (đánh dấu giải phóng slot stack khi `ret` trong `gt-glir::interp`, cấm `Phi` ở khối vào `bb0` và các kiểm thiếu trong `verify.rs`, chặn `%x = store` trong `parse.rs`); sửa **G2-05/G2-06** (`gritc lsp`: tách lỗi của tệp khác/tránh va chạm `E0205` với `std.gt`, gọi `module_file_name_for`, đọc `edition` từ `rig.toml`, chuẩn hoá URI Windows, cập nhật `grit.tmLanguage.json`).
