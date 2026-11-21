# GRIT — KẾ HOẠCH KIẾN TRÚC TRƯỞNG v0.1

> Ngôn ngữ: **Grit** · Đuôi file: `.gt` · Trình biên dịch: `gritc` · Quản lý gói: `rig`
> Giai đoạn 1: **Seed** (`gritc0`, viết bằng Rust) · Giai đoạn 2: **Ascend** (nâng dần tới triết lý đầy đủ, rồi tự biên dịch chính mình)

---

## 0. CÁCH DÙNG TÀI LIỆU NÀY (dành cho agent code trong Claude Cowork)

Tài liệu này là **hợp đồng kỹ thuật**. Agent phải tuân thủ:

1. **Làm tuần tự theo milestone** (M0 → M4.5 → M12 → M13). Không nhảy cóc. Không bắt đầu milestone sau khi milestone trước chưa đạt đủ "Tiêu chí hoàn thành".
2. **Cấm làm qua loa.** Không `todo!()`, `unimplemented!()`, `// TODO` không có issue đi kèm, không stub giả vờ chạy được, không test bị `#[ignore]` để qua cửa. Nếu chưa làm được thì ghi rõ vào `docs/KNOWN_LIMITATIONS.md` và để compiler **từ chối** (báo đỏ) thay vì **cho qua**.
3. **Nguyên tắc fail-closed:** khi gritc không chắc → báo đỏ, không chạy. "Không biết" không bao giờ được coi là "an toàn".
4. **Mọi quyết định thiết kế** ghi vào `docs/DECISIONS.md` dạng ADR ngắn (bối cảnh – quyết định – hệ quả). Không tự ý đổi mục 4 và mục 5 của tài liệu này; muốn đổi phải viết ADR và chờ chủ dự án (Giang) duyệt,sau khi code chuẩn thì git commit lại
5. **Mỗi mã chẩn đoán (error code) phải có:** tài liệu `docs/errors/Exxxx.md`, ≥3 test **fail** (code sai phải bị chặn đúng mã đó), ≥3 test **pass** (code đúng không bị chặn nhầm).
6. **Mỗi milestone kết thúc bằng:** chạy toàn bộ test + clippy + fmt + cập nhật `docs/HISTORY.md` (làm gì, còn thiếu gì, rủi ro mới). Báo cáo trung thực, kể cả khi chưa đạt.
7. **Rust:** edition mới nhất ổn định, `#![forbid(unsafe_code)]` ở mọi crate (ngoại lệ duy nhất cần ADR: ranh giới FFI với LLVM), `cargo clippy -- -D warnings`, `cargo deny`, `cargo audit` chạy trong CI. Output biên dịch phải **deterministic** (cùng input → cùng output byte-for-byte).
8. Khi thấy mâu thuẫn hoặc lỗ hổng trong kế hoạch này: **dừng, ghi vào `docs/OPEN_QUESTIONS.md`, hỏi chủ dự án.** Không tự đoán rồi code tiếp.

---

## 1. TRIẾT LÝ — CÁC NGUYÊN TẮC BẤT BIẾN

| #   | Nguyên tắc                                      | Ý nghĩa thực thi                                                                                                                                                                                            |
| --- | ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P1  | **Máy là độc giả chính**                        | Cú pháp dùng ký hiệu toán học/logic, ngắn gọn. Comment, chuỗi, commit message dùng tiếng Anh/Việt tự do. Con người vẫn _đọc kiểm được_ (có `gritc explain`, `gritc fmt`, bảng ký hiệu), chỉ là khó học hơn. |
| P2  | **Compiler không tin lời, chỉ tin bằng chứng**  | Mọi khẳng định an toàn phải được chứng minh hoặc kiểm tra; `unsafe` không phải "lời cam kết" mà là "nghĩa vụ chứng minh".                                                                                   |
| P3  | **Fail-closed**                                 | Có lỗi đỏ hoặc compiler lỗi nội bộ (ICE) → không sinh binary, không chạy.                                                                                                                                   |
| P4  | **Đúng trước, nhanh sau — nhưng nhanh ngang C** | Mã sinh ra phải sát phần cứng. Mọi kiểm tra chứng minh được lúc biên dịch thì **xoá** khỏi runtime (zero-cost).                                                                                             |
| P5  | **Giảm tối đa lỗi, không hứa 100%**             | Thiết kế _sound nhưng không complete_: có thể từ chối chương trình đúng, nhưng không được chấp nhận chương trình vi phạm quy tắc đã nêu.                                                                    |
| P6  | **Cơ sở tin cậy (TCB) nhỏ và được liệt kê**     | Luôn biết phần nào ta _buộc phải tin_ (xem mục 3.3).                                                                                                                                                        |
| P7  | **Xác định (deterministic)**                    | Không timestamp, không thứ tự ngẫu nhiên, giới hạn solver theo tài nguyên chứ không theo thời gian thực.                                                                                                    |
| P8  | **Dị cũng được, miễn mạnh – logic – tối ưu**    | Không nhân nhượng độ chặt vì "cho dễ dùng".                                                                                                                                                                 |

---

## 2. ĐÍNH CHÍNH KỸ THUẬT & GIỚI HẠN TRUNG THỰC

Phần này quan trọng để tránh xây nhà trên giả định sai.

### 2.1 Đính chính về mô tả ban đầu

- **LLVM viết bằng C++, không phải Rust.** `rustc` (viết bằng Rust) _gọi_ LLVM làm backend. Vậy `gritc` viết bằng Rust sẽ gọi LLVM (C++) qua binding (`inkwell` hoặc `llvm-sys`). Ghim đúng một phiên bản LLVM, ghi vào ADR.
- **Rust không "bằng C" và không viết bằng C.** Rust nhanh ngang C vì dùng _cùng backend LLVM_ và có trừu tượng không tốn chi phí. Grit đạt tốc độ "sát C" theo đúng cơ chế đó: sinh LLVM IR tốt + không chèn kiểm tra thừa. **Mục tiêu đo được:** nằm trong ±5% so với C/Rust tương đương trên bộ benchmark (xem 8.G).
- "Rust an toàn hơn C" đúng nhờ ownership/borrowing. Grit đi xa hơn (bằng chứng cho unsafe) nhưng cái giá là độ phức tạp compiler tăng mạnh.

### 2.2 Những thứ **không thể** làm hoàn hảo (và cách Grit xử lý)

| Bài toán                          | Vì sao không thể                                            | Cách Grit xử lý                                                                                                                |
| --------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Chứng minh mọi chương trình dừng  | Bài toán dừng bất khả quyết định                            | Mọi vòng lặp/đệ quy phải kèm **biến thể giảm** (`↓`) hoặc khai báo hiệu ứng `div` (được phép phân kỳ có chủ đích)              |
| Chứng minh "logic đúng" tổng quát | Đúng theo _đặc tả nào_? Không có đặc tả thì không có "đúng" | Chỉ kiểm: hợp đồng (`⊳ ⊲ ⋈`) mà người/AI viết ra + các bất biến an toàn built-in (tràn số, biên, aliasing…)                    |
| Phát hiện mọi secret bị lộ        | Secret không có "dấu hiệu" chắc chắn                        | Kết hợp: mẫu nhận dạng, entropy, theo dõi luồng qua kiểu `Secret<T>`. Chấp nhận false positive/negative, ghi rõ trong tài liệu |
| Tự kiểm chứng compiler            | Compiler có bug thì "chứng minh" cũng sai                   | TCB nhỏ, kernel kiểm bằng chứng độc lập, differential test, bootstrap ba tầng (8.H)                                            |
| SMT solver luôn trả lời           | Có thể `unknown`/timeout                                    | `unknown` = **chưa chứng minh = không được qua** (hoặc hạ thành runtime check nếu kiểm tra được lúc chạy)                      |

### 2.3 Rủi ro thiết kế cần theo dõi

1. **Ký hiệu Unicode có thể tốn token hơn ASCII với LLM** (nhiều ký hiệu toán học bị tách thành nhiều token). Trái với mục tiêu "ngắn gọn cho AI". → Mỗi ký hiệu **bắt buộc có bí danh ASCII 1–1** (kiểu LaTeX, vì LLM đã quen), và milestone M0 phải **đo số token thực tế** trước khi chốt bảng ký hiệu.
2. **Tấn công bằng ký tự lạ** (ký tự điều khiển bidi, ký tự trông giống nhau – homoglyph, "Trojan Source"). Ngôn ngữ nhiều ký hiệu Unicode đặc biệt dễ dính → có luật cấm cứng (D15).
3. **Phạm vi quá lớn:** type system + borrow checker + proof engine + LLVM + package manager, mỗi cái là một dự án lớn. → Danh sách **cắt bỏ** tường minh ở 6.3; Phase 1 phải **chừa sẵn móc nối** cho Phase 2 để không phải viết lại.
4. **Bug nằm ngay trong gritc** là bug "tạo cảm giác an toàn giả" — nguy hiểm hơn bug thường. → Chiến lược test mục 9 là bắt buộc, không phải tuỳ chọn.

---

## 3. KIẾN TRÚC TỔNG THỂ

### 3.1 Pipeline `gritc`

```
.gt (UTF-8, NFC)
  │  [1] Nạp nguồn + chuẩn hoá (NFC, quy đổi bí danh ASCII → canonical, chặn ký tự cấm)
  ▼
Tokens ──[2] Lexer
  ▼
AST ─────[3] Parser (recursive descent + Pratt, khôi phục lỗi để báo nhiều lỗi/lần)
  ▼
AST+Names [4] Giải tên & đồ thị module
  ▼
HIR(typed)[5] Kiểm kiểu (bidirectional, generics đơn hình hoá), traits tĩnh
  │         [6] Kiểm cú pháp-ngữ nghĩa & đầy đủ: match đầy đủ, mọi nhánh trả về, khởi tạo chắc chắn
  ▼
MIR(CFG)  [7] Hạ xuống MIR: basic block, place, borrow, drop
  │         [8] Khung dataflow tổng quát (lattice solver) — dùng chung cho mọi phân tích
  │         [9] Kiểm sở hữu & mượn (borrow checker kiểu NLL)
  │         [10] Kiểm hiệu ứng & trạng thái (effects)
  │         [11] Kiểm đa luồng (Send/Sync-like, thứ tự khoá)
  │         [12] Kiểm biên/trường hợp biên (tràn số, chia 0, chỉ số, ép kiểu…) → sinh Obligation
  │         [13] Kiểm điều kiện dừng (termination)
  │         [14] Bộ giải Obligation (P1: miền khoảng + zone + hằng; P2: SMT + Kernel)
  │         [15] Kiểm bảo mật/secret (VÀNG)
  ▼
 [16] PHÁN QUYẾT: có ĐỎ hoặc ICE ⇒ dừng. Chỉ VÀNG/GHI CHÚ ⇒ đi tiếp, in cảnh báo.
  ▼
 [17] Codegen: MIR → LLVM IR (kèm metadata gắn Obligation-ID đã giải)
  ▼
 [18] IR GUARD (kiểm nhanh trước khi chạy: LLVM verifier + đối chiếu attestation + kiểm metadata đa luồng/unsafe)
  ▼
 LLVM opt → object → link → binary ──► chạy với gritrt (panic = abort tức thì)
```

### 3.2 Hạ tầng dùng chung (bắt buộc dựng ở Phase 1, dù Phase 1 chưa dùng hết)

- **Obligation IR**: biểu diễn "điều cần chứng minh" (`Oblig { id, kind, span, formula, context, dischargeable_at_runtime: bool }`). Phase 1 giải bằng miền khoảng/hằng; Phase 2 cắm SMT/Kernel **không đổi giao diện**.
- **Diagnostics engine**: mã lỗi ổn định, mức độ (ĐỎ/VÀNG/GHI CHÚ), span, gợi ý sửa dạng _edit có cấu trúc_, xuất người-đọc + JSON.
- **Dataflow framework**: một bộ giải lattice, các phân tích (init, liveness, interval, taint…) chỉ là instance.
- **Attestation**: mỗi phiên `gritc check` sinh bản ghi băm (hash nguồn + phiên bản gritc + danh sách pass đã chạy + kết quả). IR Guard từ chối IR không khớp attestation (chống chạy IR chưa qua kiểm tra / bị sửa tay).

### 3.3 Cơ sở tin cậy (TCB) — thứ ta _buộc_ phải tin

Phase 1: (1) mã gritc0, (2) `rustc` + thư viện chuẩn Rust, (3) LLVM, (4) linker, (5) OS/phần cứng, (6) các `intrinsic` trong `lib/core` (được đánh dấu `trusted`, mỗi cái kèm lý do và được người duyệt), (6b) danh sách **lang item** `#lang(…)` mà compiler gán ý nghĩa đặc biệt và chỉ lib/core được khai báo: `option`, `copy`, `range`, `secret`, `cap`, `arena`, `drop`, `range_inclusive` (ADR-018, ADR-019, ADR-020, ADR-022; mỗi mục mới cần Giang duyệt), và (M8, ADR-026/027, chỉ thị Giang 2026-10-08) `send`, `sync` (trait tự động, suy theo cấu trúc), `cell`, `mutex`, `atomic`, `atomic_int`, `ordering`; cùng marker hiệu ứng `poly`/`detach` (chỉ lib/core) và thuộc tính `#level("n")` cho cấp khoá. **Hợp đồng từng lang item** (bất biến, quyền truy cập, ordering bắt buộc tường minh) ở ADR-027 §1. **Thông dịch MIR cài các lang item này chỉ là công cụ test, NGOÀI TCB; M10 phải cài bằng atomics và khoá LLVM/hệ điều hành thật; ngữ nghĩa của thông dịch không được coi là bằng chứng cho ngữ nghĩa codegen.**
(6c) **Intrinsic/runtime trusted của Phase 1 được liệt kê đầy đủ, kèm hợp đồng (hiệu ứng, capability, điều kiện abort, bất biến, người duyệt) ở `docs/TCB_INTRINSICS.md`**; ranh giới `unsafe` của `runtime/gt-rt` ở ADR-031.
Phase 2 thêm/đổi: (7) SMT solver (giảm dần khi có Kernel), (8) Kernel kiểm bằng chứng. **Mục tiêu dài hạn: thu nhỏ TCB, không phải làm nó biến mất.**

---

## 4. QUY ĐỊNH NGÔN NGỮ (do kiến trúc sư quyết — Giang có quyền phủ quyết)

### 4.0 Bổ sung triết lý — Mục "Thứ ba" (đã được Giang duyệt, đưa vào Phase 1: Seed)

**(a) Mô hình bộ nhớ & cấp phát tường minh**

- Không garbage collector. Mọi cấp phát đi qua **allocator tường minh** truyền vào (tham số hoặc hiệu ứng), không có allocator ngầm toàn cục trừ `lib/core::GlobalAlloc` dùng cho chương trình đơn giản.
- Kiểu allocator lõi cần có ngay ở Phase 1: `Bump`/`Arena` (cấp phát tuyến tính, giải phóng cả khối), `GlobalAlloc` (bọc an toàn quanh allocator hệ thống). Pool/slab để Phase 2 khi cần.
- Hiệu ứng `mem` được gắn cận trên khi chứng minh được (ví dụ `! {mem ≤ n*8}`), hạ xuống runtime check khi không chứng minh được, giống cơ chế Obligation đã có (D24/D25) — **không phải hệ thống riêng**, chỉ là một họ Obligation mới.
- Vùng nhớ gắn với region/arena được borrow checker coi như một _lifetime được đặt tên_, cấm con trỏ thoát khỏi region khi region đóng (kiểm bằng cùng cơ chế NLL ở M6, mở rộng không viết lại).

**(b) Hệ thống quyền hạn (capability) & ranh giới module**

- Không có quyền truy cập ngầm định. Mọi năng lực nhạy cảm — đọc/ghi tệp, mở mạng, đọc biến môi trường, đọc đồng hồ hệ thống, đọc `Secret<T>` — biểu diễn bằng **kiểu capability** (`Cap<Fs>`, `Cap<Net>`, `Cap<Env>`, `Cap<Clock>`, `Cap<Secret>`…) phải được **truyền tường minh** vào hàm cần nó, không lấy qua biến toàn cục.
- `main` là nơi duy nhất nhận capability gốc từ runtime; từ đó capability được chia nhỏ/truyền xuống có kiểm soát (có thể giới hạn phạm vi, ví dụ `Cap<Fs>` chỉ cho một thư mục).
- Capability là **một họ hiệu ứng cụ thể hoá thành kiểu** — tái dùng hạ tầng hiệu ứng D10, không phải cơ chế song song. Thiếu capability trong chữ ký hàm mà thân hàm cần dùng ⇒ lỗi kiểu, không phải lỗi runtime (E03xx/E08xx).
- Đây là nền tảng bắt buộc để: `Secret<T>` (D13) chỉ đọc được khi có `Cap<Secret>`; Rig (8.F) kiểm chuỗi cung ứng bằng cách xem một gói khai báo đòi capability gì trước khi cho phép cài.

**Việc cần làm trong Phase 1 do (a)/(b):**

- Thêm M4.5 (xen giữa M4 và M5): thiết kế kiểu `Cap<T>`, `Arena`, lifetime đặt tên cho region — vào **cùng đợt** với hệ thống kiểu, vì capability và vùng nhớ đều là kiểu.
- Thêm luật D26–D29 (mục 4.4).
- `lib/core` (M12) phải có: `Arena`, `GlobalAlloc`, `Cap<Fs|Net|Env|Clock|Secret>`, và một hàm `entry` nhận capability gốc.
- 3 chương trình kiểm tra tay ở M12 phải dùng allocator tường minh và capability thay vì giả định quyền truy cập ngầm (ví dụ chương trình đọc JSON từ tệp phải nhận `Cap<Fs>` tường minh).

### 4.1 Hai bề mặt cú pháp, một ngữ nghĩa

- **Canonical** (Unicode ký hiệu) và **ASCII alias** (kiểu LaTeX: `\fn`, `\land`, …). Lexer chấp nhận cả hai; `gritc fmt` chuẩn hoá về canonical. Hai dạng phải **biên dịch ra cùng AST** (test round-trip).
- Nguyên tắc chọn ký hiệu (M0 chốt): (a) 1 codepoint phổ biến trong khối ký hiệu toán/logic; (b) có bí danh ASCII; (c) không gây nhầm lẫn thị giác với ký hiệu khác; (d) đo token LLM, loại ký hiệu quá tốn.

### 4.2 Bảng ký hiệu lõi (bản nháp — M0 phải đóng băng đầy đủ, gồm cả if/match/struct/enum/mod/use/pub/trait/impl…)

| Ý nghĩa                        | Canonical  | ASCII alias                |
| ------------------------------ | ---------- | -------------------------- |
| Định nghĩa hàm                 | `ƒ`        | `\fn`                      |
| Gán/bind bất biến              | `≔`        | `:=`                       |
| Biến khả biến (tiền tố)        | `μ`        | `\mut`                     |
| Ghi đè giá trị                 | `←`        | `<-`                       |
| Kiểu trả về                    | `→`        | `->`                       |
| Vòng lặp có biến thể           | `⟳`        | `\loop`                    |
| Biến thể giảm (termination)    | `↓`        | `\dec`                     |
| Tiền điều kiện / Hậu điều kiện | `⊳` / `⊲`  | `\pre` / `\post`           |
| Bất biến vòng lặp              | `⋈`        | `\inv`                     |
| Và / Hoặc / Phủ định / Suy ra  | `∧ ∨ ¬ ⇒`  | `\land \lor \neg \implies` |
| Với mọi / Tồn tại              | `∀ ∃`      | `\forall \exists`          |
| So sánh                        | `≤ ≥ ≠`    | `<= >= !=`                 |
| Thuộc                          | `∈`        | `\in`                      |
| Tập hiệu ứng rỗng (thuần)      | `∅`        | `\nil`                     |
| Khối unsafe                    | `⚠`        | `\unsafe`                  |
| Mệnh đề bằng chứng             | `⊢`        | `\proof`                   |
| Mượn chung / mượn duy nhất     | `&` / `&!` | `&` / `&!`                 |

Số học: mặc định **kiểm tràn**. Toán tử tường minh: `+%` (quấn vòng), `+^` (bão hoà), `+?` (trả `Option`).

### 4.3 Ví dụ minh hoạ ý đồ (bản nháp, ngữ pháp thật chốt ở M0)

```
ƒ sum(xs: &[i64]) → i64 ! ∅ {
  μs ≔ 0;
  μi ≔ 0;
  ⟳ i < xs.len() ↓ xs.len() - i {
    s ← s + xs[i];     // tràn số: không chứng minh được → hạ thành runtime check (GHI CHÚ N0601)
    i ← i + 1;         // chỉ số xs[i]: chứng minh được từ điều kiện vòng lặp → KHÔNG có check lúc chạy
  }
  s
}
```

### 4.4 Các quy định bổ sung (D1–D25)

**Kiểu & số học**

- **D1** Không ép kiểu ngầm giữa các kiểu số; ép kiểu tường minh và sinh Obligation (mất thông tin → phải chứng minh hoặc dùng toán tử có tên).
- **D2** Mọi phép số học nguyên đều có Obligation tràn số (xem D-edge bên dưới).
- **D3** Không `null`, không biến chưa khởi tạo, không exception, **không unwinding**. Lỗi khôi phục được dùng `Result`/`Option`. Lỗi không khôi phục = **abort tức thì**.
- **D4** Số thực `f32/f64` theo IEEE; **cấm `==` trực tiếp trên số thực** (đỏ), dùng hàm so sánh tường minh (`total_cmp`/ngưỡng sai số). NaN là nguồn bug phổ biến.

**Biến & luồng**

- **D5** Bất biến theo mặc định. **D6** `match` phải đầy đủ. **D7** Cấm shadowing (AI không cần, và nó là nguồn bug). **D8** Cấm macro, reflection, `eval`, inline assembly (Phase 1); inline asm có thể cân nhắc ở Phase 2 kèm Obligation.
- **D9** Biên dịch xác định/tái lập được (reproducible).

**Hiệu ứng & trạng thái**

- **D10** Mọi hàm **khai báo tập hiệu ứng** (`! ∅` = thuần; `! {io, mem, st, div, ffi, lock(Lk)}`…). Gritc kiểm: _khai báo ⊇ suy ra_. Hiệu ứng lan truyền lên hàm gọi.
- **D11** Không biến toàn cục khả biến, trừ qua kiểu đồng bộ (atomic/mutex có cấp khoá) và hiệu ứng `st` khai báo rõ.
- **D12** Vòng lặp/đệ quy: phải có `↓` hoặc nằm trong hiệu ứng `div` (chỉ cho vòng lặp chủ ý vô hạn: server, event loop, entry của luồng). **Biến thể của hàm đệ quy là contract `↓ expr` trong chữ ký hàm; đệ quy tương hỗ cần `↓` cho mọi hàm trong cùng SCC** (bổ sung theo quyết định của Giang, 2026-10-05, từ Q15h).

**Bảo mật**

- **D13** Dữ liệu nhạy cảm đi qua kiểu `Secret<T>`: không `Display`, không ghi log/mạng/tệp trừ khi `declassify` tường minh (ghi nhận và báo VÀNG).
- **D14** FFI: Phase 1 **cấm** ngoài `lib/core`. Phase 2: chỉ qua khai báo `extern` có hợp đồng, gọi FFI là hiệu ứng `ffi` cần bằng chứng/được ký trong danh sách tin cậy của `rig.toml`.
- **D15** Nguồn: UTF-8, NFC. **Cấm (ĐỎ):** ký tự điều khiển bidi, ký tự vô hình/zero-width ngoài chuỗi/comment có chú giải, định danh trộn nhiều bảng chữ (confusable/homoglyph). Comment/chuỗi cũng cấm ký tự bidi.

**Công cụ**

- **D16** Hệ thống mã: `E` + 4 số (ĐỎ), `S` + 3 số (VÀNG, bảo mật), `N` + 4 số (GHI CHÚ).
  - E01xx cú pháp · E02xx tên/module · E03xx kiểu · E04xx sở hữu/mượn · E05xx đa luồng · E06xx biên/trường hợp biên · E07xx điều kiện dừng · E08xx hiệu ứng/trạng thái · E09xx logic/Obligation · E10xx unsafe/bằng chứng · E11xx IR Guard · E12xx unicode/nguồn · E13xx định dạng (gt-fmt) · E99xx ICE.
- **D17** Exit code: `0` = qua (có thể kèm VÀNG), `1` = có ĐỎ, `2` = ICE. **Cả 1 và 2 đều không sinh binary và không chạy.** **Phân vùng mã thoát (Giang duyệt 2026-10-09, ADR-028 §3):** giai đoạn `check`/`build`/lỗi dùng: `0` qua, `1` ĐỎ (hoặc VÀNG khi `--deny-yellow`), `2` ICE (kể cả thiếu pass bắt buộc, pass panic), `3` lỗi dùng/I/O. Giai đoạn `run` (chương trình ĐÃ chạy): `0` chạy xong bình thường, `70` chương trình abort (D3), `71` bộ chạy tham chiếu hết giới hạn, `100+s` (s = 1..=100) trạng thái thoát do chính chương trình báo (dành riêng). Hai vùng KHÔNG chồng nhau nên "chương trình trả 1" không bao giờ lẫn với "gritc báo ĐỎ" (mã 1 chỉ do gritc ở giai đoạn check); mã thoát tuỳ ý của chương trình còn đi qua JSON (`phase`, `run.exit_status`). Mọi đầu ra ghi `backend` (M9: `"interpreter"`).
- **D18** Lệnh gritc: `check`, `build`, `run`, `explain <mã>`, `fmt`, `--json` (schema có version, ổn định). Phase 2 thêm: `prove`, `recheck`, `serve`.
- **D19** Báo **tất cả** lỗi (có giới hạn số lượng), thứ tự xác định.
- **D20** Giới hạn solver theo **tài nguyên** (số bước/bộ nhớ), không theo giây.

**Kiểm chứng & unsafe**

- **D21** `⚠` chỉ hợp lệ khi mỗi nghĩa vụ (hợp lệ con trỏ, căn lề, không alias, độ sống, không data race…) được **giải** bởi bộ giải hiện có, kèm mệnh đề `⊢`. Không giải được → ĐỎ E10xx. Không có cơ chế "tin tôi đi".
- **D22** Chỉ `lib/core` có `intrinsic trusted`. Mỗi intrinsic có tài liệu hợp đồng + người duyệt, và nằm trong danh sách TCB.
- **D23** Thư viện/gói: public API phải khai báo đủ kiểu + hiệu ứng + hợp đồng; Rig lưu kèm chứng nhận kiểm tra (Phase 2: bằng chứng).
- **D24** Mọi "chuyển mức" (hạ Obligation thành runtime check) phải **hiển thị** (GHI CHÚ Nxxxx) để người/AI biết chi phí.
- **D25** Một Obligation chỉ được hạ thành runtime check nếu **kiểm tra được lúc chạy** (biểu thức thuần, tính được). Những thứ _không kiểm lúc chạy được_ (độ hợp lệ con trỏ, alias, lifetime, termination, data race, lượng từ không biên) mà chưa giải → **ĐỎ**, không có đường hạ.

**Bộ nhớ & quyền hạn**

- **D26** Cấm allocator toàn cục ngầm định trong code người dùng; mọi cấp phát phải truy vết được tới một `Arena`/`GlobalAlloc` được truyền vào tường minh (tham số hoặc capability `Cap<Alloc>`).
- **D27** Con trỏ/tham chiếu trỏ vào một `Arena` không được thoát khỏi lifetime của `Arena` đó; borrow checker coi region là một lifetime có tên, kiểm cùng cơ chế M6 (E04xx mở rộng, không mã lỗi riêng).
- **D28** Năng lực nhạy cảm (tệp, mạng, biến môi trường, đồng hồ, `Secret<T>`) chỉ được dùng trong thân hàm nếu kiểu `Cap<...>` tương ứng xuất hiện trong chữ ký hàm đó; không có biến toàn cục cấp quyền. Thiếu capability ⇒ lỗi kiểu lúc biên dịch (không phải runtime).
- **D29** `main`/entrypoint là nguồn duy nhất phát capability gốc; chia nhỏ capability (ví dụ giới hạn `Cap<Fs>` vào một thư mục con) phải qua hàm tạo tường minh trong `lib/core`, không có cách "tự phong" capability trong code người dùng thường — chỉ intrinsic `trusted` (D22) được làm việc đó.

---

## 5. HỆ THỐNG PHÁN QUYẾT (theo đúng yêu cầu của Giang)

| Mức                 | Tác động                              | Gồm những gì                                                                                                                                                             |
| ------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 🔴 **ĐỎ** (E…)      | **Không sinh binary, không chạy**     | Mọi lỗi **không thuộc nhóm nhạy cảm**: cú pháp, tên, kiểu, sở hữu/mượn, đa luồng, biên, dừng, hiệu ứng, logic, unsafe thiếu bằng chứng, IR Guard, unicode nguy hiểm, ICE |
| 🟡 **VÀNG** (S…)    | **Vẫn cho chạy**, in cảnh báo nổi bật | **Chỉ** lỗi lộ nhạy cảm: key/mật khẩu/token nằm trong nguồn (S1xx), `Secret<T>` bị `declassify`/chảy ra sink log-mạng-tệp (S2xx)                                         |
| ⚪ **GHI CHÚ** (N…) | Không ảnh hưởng chạy                  | Thông tin: Obligation đã hạ thành runtime check, biến không dùng (không phải kiểu `must_use`), v.v.                                                                      |

**Quan trọng — hai điều kiến trúc sư khuyến nghị (không tự ý áp đặt, chờ Giang duyệt):**

1. Mặc định **giữ đúng quy tắc của Giang** (secret = vàng, vẫn chạy). Đồng thời cung cấp cờ `--deny-yellow` và hồ sơ `release` có thể nâng VÀNG thành ĐỎ. Lý do: secret cứng trong nguồn sẽ nằm nguyên văn trong binary. Mặc định dev thoải mái, build phát hành nên chặn.
2. Cho phép `#allow(S1xx, "lý do")` để tắt cảnh báo VÀNG có ghi lý do; **ĐỎ thì không có cách tắt.**

**Phát hiện secret (Phase 1):** (a) regex/mẫu cho định dạng phổ biến (khoá cloud, token VCS, khối khoá riêng PEM, JWT…); (b) entropy cao trong literal gán cho tên giống `key/token/secret/password/passwd/pwd/api`; (c) theo dõi taint từ `Secret<T>` tới sink bằng khung dataflow.

---

## 6. GIAI ĐOẠN 1 — SEED (`gritc0`, viết bằng Rust)

**Mục tiêu:** có compiler _chạy được thật_, kiểm tra thật, sinh mã máy qua LLVM, với một tập con Grit **nhỏ nhưng chặt**, và **chừa móc nối** cho Phase 2.

### 6.1 Cấu trúc repo

```
grit/
  Cargo.toml                 (workspace)
  crates/
    gt-span/                 bản đồ nguồn, span
    gt-diag/                 chẩn đoán, mã lỗi, mức độ, JSON
    gt-lexer/
    gt-ast/
    gt-parser/
    gt-resolve/              giải tên, module
    gt-types/                kiểm kiểu, traits tĩnh, đơn hình hoá
    gt-mir/                  biểu diễn trung gian (MIR); bước hạ MIR là nơi DUY NHẤT đọc AST + bảng của gt-types
    (gt-hir/)                HOÃN, không bỏ (ADR-020 §9, Q27): chưa cần lớp HIR riêng ở Phase 1
    gt-dataflow/             khung phân tích lattice
    gt-borrowck/             sở hữu & mượn
    gt-effects/
    gt-conc/
    gt-edge/
    gt-term/
    gt-secrets/
    gt-oblig/                Obligation IR + bộ giải P1 (interval/const)
    gt-codegen-llvm/         MIR → LLVM IR
    gt-guard/                IR Guard + attestation
    gt-driver/               binary `gritc`
    gt-fmt/
    rig/                     binary `rig`
  runtime/gt-rt/             runtime tối thiểu (staticlib, panic=abort)
  lib/core/*.gt              thư viện lõi bằng Grit
  tests/{pass,fail,golden,fuzz,bench}/
  docs/{spec,errors,DECISIONS.md,HISTORY.md,KNOWN_LIMITATIONS.md,OPEN_QUESTIONS.md}
```

### 6.2 Các milestone

**M0 — Đặc tả & nền móng** _(không viết compiler trước khi xong M0)_

- Viết `docs/spec/grammar.ebnf` đầy đủ, bảng ký hiệu đóng băng, bí danh ASCII, ngữ nghĩa lõi (kiểu, sở hữu, hiệu ứng, số học, đa luồng, unsafe, hợp đồng), mã lỗi dự kiến.
- **Đo token:** chạy ví dụ chuẩn qua tokenizer sẵn có, so sánh canonical vs ASCII, ghi kết quả vào ADR; chỉnh bảng ký hiệu nếu cần.
- Dựng workspace, CI, quy ước test, `DECISIONS.md`.
- _Hoàn thành khi:_ Giang duyệt đặc tả; có ≥30 chương trình mẫu `.gt` (đúng/sai) làm "đặc tả sống".

**M1 — Lexer (hai bề mặt)**

- Chuẩn hoá NFC, quy đổi alias→canonical, định danh Unicode (XID), chặn bidi/zero-width/homoglyph (E12xx).
- _Hoàn thành khi:_ round-trip alias↔canonical đúng 100% mẫu; fuzz lexer ≥1 giờ không panic/treo; mọi ký tự cấm có test fail.

**M2 — Parser + AST + Diagnostics**

- Recursive descent + Pratt, **khôi phục lỗi** (báo nhiều lỗi), span chính xác, in lỗi đẹp + JSON.
- _Hoàn thành khi:_ parse toàn bộ mẫu hợp lệ; `fmt(parse(x))` ổn định (idempotent); fuzz parser sạch; snapshot test cho thông báo lỗi.

**M3 — Giải tên & module**

- Module, `use`, khả kiến (`pub`), phát hiện trùng/thiếu/vòng phụ thuộc.
- _Hoàn thành khi:_ E02xx đầy đủ test; xử lý đúng thứ tự không phụ thuộc.

**M4 — Hệ thống kiểu**

- Kiểu nguyên thuỷ có kích thước, struct/enum/tuple/slice/array, `Option/Result`, generics (đơn hình hoá), traits tĩnh đơn giản (không `dyn`), suy luận cục bộ bidirectional, **kiểu `Secret<T>`**, hiệu ứng như một phần chữ ký hàm.
- _Hoàn thành khi:_ bộ test kiểu ≥200 ca; không có ép kiểu ngầm lọt; thông điệp lỗi có gợi ý sửa dạng edit.

**M4.5 — Kiểu vùng nhớ & kiểu capability**

- Thiết kế và cài `Arena`/region như lifetime có tên trong hệ kiểu (móc nối sẵn chỗ cho borrow checker M6 dùng); cài họ kiểu `Cap<T>` (`Fs|Net|Env|Clock|Secret|Alloc`) như kiểu affine không `Copy`, không `Default` (không tự tạo ra được).
- Kiểm tĩnh: hàm dùng năng lực nhạy cảm mà không có `Cap<...>` tương ứng trong chữ ký ⇒ lỗi kiểu (D28).
- _Hoàn thành khi:_ có ≥20 ca test ép buộc thiếu capability bị chặn đúng mã; ≥20 ca có capability hợp lệ qua; bản nháp ngữ nghĩa region sẵn sàng cho M6 dùng lại không phải thiết kế lại.

**M5 — HIR/MIR + khung Dataflow**

- Hạ xuống MIR (CFG, place, drop, borrow). Dựng bộ giải lattice chung; cài sẵn: khởi tạo chắc chắn, liveness.
- Kiểm đầy đủ: match đầy đủ, mọi đường trả về, mọi biến khởi tạo (E01xx/E03xx/E06xx).
- _Hoàn thành khi:_ in được MIR dạng text ổn định để debug; test đặc tả dataflow (kết quả cố định trên CFG mẫu).

**M6 — Sở hữu & mượn (borrow checker)**

- Move/copy, drop, mượn chung/duy nhất, region inference kiểu NLL (liveness + ràng buộc), reborrow, elision lifetime hạn chế, kiểm `drop` đúng một lần; mở rộng để kiểm **region/Arena** (D27): con trỏ vào region không thoát khỏi lifetime của region (dùng lại ngữ nghĩa region đã thiết kế ở M4.5, không thiết kế lại).
- _Hoàn thành khi:_ bộ **corpus chương trình xấu** (use-after-move, dangling, aliasing &!, trả tham chiếu tới local, mượn chồng chéo, con trỏ thoát khỏi Arena…) ≥150 ca **đều bị chặn đúng mã**; bộ chương trình đúng ≥150 ca **không bị chặn nhầm**. Mọi false negative phát hiện sau này = bug mức nghiêm trọng nhất, phải thêm vào corpus.

**M7 — Biên & trường hợp biên (miền khoảng)**

- Sinh Obligation cho: tràn số (cộng/trừ/nhân/âm số nhỏ nhất/shift), chia 0, chỉ số mảng, ép kiểu mất thông tin, `unwrap` trên `None`…
- Bộ giải P1: hằng số, **miền khoảng** (interval) **cộng zone** (ràng buộc hiệu `x − y ≤ c` giữa các biến nguyên và độ dài lát — cần thiết để chứng minh `xs[i]` từ `i < xs.len()` như ví dụ §4.3; miền khoảng thuần không làm được vì `xs.len()` không phải hằng), điều kiện nhánh/vòng lặp (widening/narrowing), `⊳` của hàm hiện tại (ADR-025, Giang duyệt 2026-10-07). Giải được → **xoá check**; không giải được nhưng kiểm lúc chạy được → **runtime check + GHI CHÚ** (D24/D25), ở MỌI hồ sơ (dev và release). Bộ giải thứ hai (liệt kê vét cạn trên độ rộng bit nhỏ) là bắt buộc ở Phase 1 để chứng minh giao diện cắm được và làm oracle.
- _Hoàn thành khi:_ test cả hai chiều (check bị xoá khi chứng minh được; check còn khi không); đo số check còn lại trên chương trình mẫu.

**M8 — Hiệu ứng, điều kiện dừng, đa luồng**

- Hiệu ứng: suy luận + so với khai báo, lan truyền gọi hàm (E08xx).
- Dừng: mỗi `⟳`/đệ quy có `↓` trên kiểu well-founded; kiểm giảm nghiêm ngặt qua miền khoảng/dạng mẫu `i ← i + c` hướng về cận; đệ quy tương hỗ xét theo SCC; `div` cho phân kỳ chủ ý (E07xx).
- Đa luồng: `spawn(hàm, đối_số)` (Phase 1 **chưa có closure** để giảm độ phức tạp), điều kiện `Send/Sync`-like tự suy, chia sẻ chỉ qua kiểu đồng bộ; **cấp khoá** (`Mutex<T, L>`): thứ tự khoá phải tăng nghiêm ngặt, kiểm xuyên hàm bằng tóm tắt hiệu ứng `lock(...)` → chặn deadlock theo thứ tự (E05xx). Atomics bắt buộc ghi rõ memory ordering.
- _Hoàn thành khi:_ corpus data race / deadlock / vòng lặp không biến thể đều bị chặn; chương trình đa luồng đúng chạy qua.

**M9 — Secret scanner + Cổng phán quyết**

- Cài S1xx/S2xx như mục 5; `#allow` có lý do; `--deny-yellow`; bộ tổng hợp phán quyết (D17), ICE bắt panic nội bộ → exit 2 và **không bao giờ** chạy.
- _Hoàn thành khi:_ có test chứng minh "ĐỎ ⇒ không có file output, không chạy" và "VÀNG ⇒ chạy + in cảnh báo"; danh sách mẫu secret có test pass/fail; tài liệu hoá giới hạn.

**M10 — Codegen LLVM + IR Guard + Runtime**

- MIR → LLVM IR bằng binding đã ghim; gắn metadata `!gt.obl` (Obligation-ID đã giải) và `noalias`/attribute suy ra từ borrow checker; hồ sơ `dev`/`release`.
- **IR Guard** (nhanh, không kiểm lại toàn bộ): (a) LLVM verifier; (b) đối chiếu attestation hash; (c) mọi thao tác nhạy cảm (raw memory, atomic, spawn) phải có Obligation-ID đã giải; (d) quét metadata đa luồng/ordering nhất quán. Không khớp → E11xx, không chạy.
- **`Unreachable` của MIR hạ thành abort (D3, fail-closed), KHÔNG BAO GIỜ thành `unreachable` của LLVM làm điều kiện tối ưu** (chứng minh của compiler sai ⇒ chương trình dừng, không chạy tiếp vào hành vi không xác định). **IR Guard từ chối IR chứa `unreachable` không do codegen cố ý sinh ra ở chỗ đã được chứng minh.** Lưu ý kỹ thuật (làm rõ lệnh): LLVM bắt buộc mỗi khối có terminator, nên `call @gritrt_abort()` (noreturn) phải được theo bởi một `unreachable` — đó là CHỖ DUY NHẤT hợp lệ (ngay sau lời gọi hàm abort của runtime, đã được chứng minh không trả về); IR Guard chấp nhận đúng mẫu đó và từ chối mọi `unreachable` khác.
- `gritrt`: `panic = abort`, handler tối thiểu in thông tin rồi dừng tiến trình **ngay lập tức** (không unwinding, không cleanup). Hồ sơ `dev` hỗ trợ bật ASan/TSan/UBSan để bắt thêm lỗi.
- _Hoàn thành khi:_ `gritc run` chạy được chương trình mẫu; mọi crash/abort dừng ngay với exit code riêng; golden test trên LLVM IR; benchmark nền (xem 8.G).

**M11 — Rig v0**

- `rig new | check | build | run | test | add (path) | lock | fmt | clean`. Manifest `rig.toml` + `rig.lock` (có băm nội dung). Phụ thuộc cục bộ theo đường dẫn. `rig build` luôn chạy `gritc check` trước. Chưa có registry.
- _Hoàn thành khi:_ dự án 3 gói phụ thuộc nhau build được, tái lập được (hai lần build → binary giống hệt).

**M12 — Lõi thư viện + Cổng "Seed 1.0"**

- `lib/core`: `Option`, `Result`, `Box`, `Vec`, slice, chuỗi UTF-8, `Mutex`, atomics, I/O tối thiểu, số học có tên (`+%`, `+^`, `+?`), `Secret`, `Arena`, `GlobalAlloc`, `Cap<Fs|Net|Env|Clock|Secret>`, hàm `entry` nhận capability gốc từ runtime.
- Viết **3 chương trình thật** bằng Grit làm bài kiểm tay, **bắt buộc dùng allocator tường minh và capability** (không giả định quyền ngầm): (1) bộ phân tích JSON nhỏ (nhận `Cap<Fs>` để đọc tệp, cấp phát bằng `Arena`), (2) nhân ma trận đa luồng (cấp phát bằng `Arena` dùng chung giữa các luồng), (3) hash/mã hoá nhỏ (dữ liệu khoá đi qua `Secret<T>` + `Cap<Secret>`). Mỗi chương trình phải chạy đúng và được benchmark.
- Cập nhật `KNOWN_LIMITATIONS.md` trung thực.

**M13 — Grit LSP (tooling, "Seed 1.1")**

- Mục tiêu: giữ đúng lời hứa "khó học, không dị biệt" — một ngôn ngữ ký hiệu dày đặc mà không có công cụ đọc tốt sẽ trôi dần thành dị biệt thật sự theo thời gian, nên tooling không phải phụ kiện.
- Làm sau M12, dùng trực tiếp `gritc check --json` (D18) đã có sẵn — **không viết lại logic kiểm tra**, LSP chỉ là lớp tiêu thụ dữ liệu, không chạm TCB (mục 3.3).
- Chức năng tối thiểu:
  - Chuẩn LSP (Language Server Protocol) để dùng được trên nhiều editor, không khoá riêng VSCode.
  - Hiển thị kép canonical ⇄ alias ASCII (gõ alias, hiện canonical, hoặc ngược lại tuỳ cấu hình — tái dùng bộ chuẩn hoá từ M1).
  - Inlay hint: kiểu suy luận được, hiệu ứng (`! {...}`) và capability (`Cap<...>`) của hàm đang xem, trạng thái từng Obligation (đã xoá / hạ runtime check / chưa giải — tái dùng dữ liệu M7/M8).
  - Gạch chân lỗi theo đúng mã (E/S/N), hover hiện link `gritc explain <mã>`.
  - Go-to-definition, tìm tham chiếu (dựa trên `gt-resolve` của M3).
- _Hoàn thành khi:_ mở được 3 chương trình thật ở M12 trong editor, inlay hint khớp đúng kết quả `gritc check --json`; có test "LSP không bao giờ tự ý đổi phán quyết của gritc" (LSP chỉ hiển thị, không tự kiểm tra song song để tránh hai nguồn sự thật lệch nhau).
- _Lưu ý phạm vi:_ chế độ `--oracle` ("vì sao chưa chứng minh được", gợi ý sửa tự động cho AI) **không** làm ở đây — để dành 8.E, vì nó cần dữ liệu từ SMT/Kernel (8.A/8.B) mới trả lời được "thiếu giả thiết gì".

### 6.3 DANH SÁCH CẮT BỎ khỏi Phase 1 (chủ ý, không phải sót)

Closure, `dyn`/trait object, macro, async, GAT/specialization, inline asm, FFI người dùng, `unsafe` do người dùng viết (chỉ có intrinsic trusted trong core), SMT, kernel, registry, tối ưu MIR nâng cao, tự biên dịch. **Bất cứ thứ gì khác muốn thêm ⇒ ghi OPEN_QUESTIONS, hỏi trước.**

> Trong Phase 1, `⚠` **có cú pháp và có đường kiểm tra Obligation**, nhưng bộ giải chỉ có miền khoảng/hằng nên thực tế mọi `⚠` của người dùng sẽ bị ĐỎ trừ khi nghĩa vụ rất đơn giản. Đó là hành vi đúng và đã được thiết kế (fail-closed). Sức mạnh thật đến ở Phase 2.

---

## 7. CỔNG CHUYỂN GIAI ĐOẠN (Seed 1.0 → Ascend)

Chỉ chuyển khi đạt **tất cả**:

1. Toàn bộ test xanh; corpus borrowck/đa luồng/biên không còn false negative đã biết.
2. **(Sửa bởi ADR-037, Giang 2026-10-14)** Fuzz **lexer và parser** mỗi cái ≥24 giờ TÍCH LUỸ (không cần liên tục, giữ corpus; cơ chế `tools/fuzz_session.sh` + `docs/FUZZ_LOG.jsonl`; tổng bằng `tools/fuzz_total.py`), không panic/treo. **Type checker: KHÔNG fuzz byte** (parser chặn trước); thay bằng bộ sinh có cấu trúc + oracle độc lập + đột biến (số chương trình và độ phủ theo loại ghi ở ADR-037). Đây là NỚI điều kiện so với bản gốc "lexer/parser/typechecker mỗi cái ≥24 giờ".
3. 3 chương trình thật (M12) chạy đúng; benchmark nền được ghi nhận.
4. Mọi mã lỗi có tài liệu + test đủ (mục 0.5).
5. Output tái lập 100% (build hai lần so băm).
6. `KNOWN_LIMITATIONS.md` và `HISTORY.md` được xem xét bởi Giang.
7. Có ADR chốt giao diện **Obligation IR** ở dạng ổn định (đây là "khớp nối" quan trọng nhất sang Phase 2).
8. M13 (LSP) hoàn thành và schema JSON chẩn đoán đóng băng (đây là "khớp nối" thứ hai sang 8.E).
9. **Điều kiện cứng về mutability bên trong (Giang, 2026-10-07):** đột biến 238/361/591/554 của `gt-borrowck/src/regions.rs` (xem `KNOWN_LIMITATIONS.md`) chỉ an toàn chừng nào ngôn ngữ chưa có mutability bên trong, nhưng M8 đưa `Mutex`/atomics vào trước M12. **Trước khi bất kỳ kiểu nào có mutability bên trong được đưa vào fixture hoặc lib**, bốn đột biến này phải được mở lại: mỗi cái hoặc bị giết bằng chương trình tấn công có dùng mutability bên trong (kèm kịch bản `interior_mut` trong bộ sinh, tắt bản sửa để kiểm), hoặc có lập luận tương đương mới tính cả mutability bên trong. Chưa xong thì M8 không được bắt đầu phần `Mutex`/atomics.
10. **Điều kiện cứng về đột biến chưa giải quyết của M7 (Giang, 2026-10-08):** M9 KHÔNG được coi là xong nếu còn đột biến của `gt-oblig/src/analysis.rs` (đường quyết định xoá/giữ) chưa giải quyết theo hướng "xoá thêm check" (làm bộ giải xoá nhiều check hơn đúng — nguy hiểm; hướng "giữ thừa check" chỉ tốn hiệu năng). Ngoài ra M9 PHẢI có chế độ **giữ mọi check** (không xoá check nào, kể cả check đã chứng minh), hoạt động đúng và có test, và chế độ này là MẶC ĐỊNH khi dùng thử Grit thật cho tới khi điều kiện trên được giải quyết. Đây là đề xuất của Giang để đánh giá trong ADR của M9 (chưa triển khai ở M7). Danh sách đột biến chưa giải quyết: `KNOWN_LIMITATIONS.md`, mục "gt-oblig".
11. **Điều kiện cứng về lỗ hổng đã biết của M4/M5 (Giang, 2026-10-06):** trước M7, literal ngoài phạm vi kiểu (`x: u8 ≔ 300`) LỌT qua M4 (E0601 thuộc M7), và thiếu đường trả về (E0307) thuộc M5. Vì vậy **M9 không được coi là xong, và `gritc` không được sinh binary**, nếu E0601 (M7) chưa được triển khai và qua test; tương tự nếu E0307 (M5) chưa triển khai. Đây là cổng chặn, không phải ghi chú.

12. **Điều kiện cứng về mô hình capability (Giang, 2026-10-13):** KHÔNG được tuyên bố mô hình capability là an toàn khi còn BẤT KỲ hàm không phải `main` cấp cao của tệp gọi được (hoặc nhắc tới) một hàm nguồn capability như `root_*`/`Host::fs()`. Hiện E0310 chặn điều này (kiểm 2026-10-13 cho `root_fs` và `root_out`, test hồi quy ở `tests/fail/E0310_*`); mọi nguồn capability MỚI thêm vào prelude/`lib/core` phải có mẫu `fail` E0310 (gọi từ hàm thường) và mẫu `pass` (nhận qua tham số) trước khi được coi là xong.

---

## 8. GIAI ĐOẠN 2 — ASCEND (hướng tới triết lý đầy đủ)

> Kế hoạch hợp nhất ĐỀ XUẤT (chưa duyệt) cho Giai đoạn 2: `docs/PHASE2_PLAN.md`.

Mỗi hạng mục dưới đây là một "đường ray" độc lập, mỗi cái có cổng hoàn thành riêng. Thứ tự đề xuất: A → B → C → D → E → F → G → H.

**8.A — Bộ giải Obligation đầy đủ (SMT)**

- Cắm Z3 (và cvc5 để **đối chứng chéo**: hai solver phải đồng ý) vào giao diện Obligation IR có sẵn. Lý thuyết: số nguyên tuyến tính, bit-vector, mảng, hàm không diễn giải; lượng từ có biên được mở rộng/instantiate có kiểm soát.
- Hợp đồng `⊳ ⊲ ⋈ ↓` được **kiểm tĩnh đầy đủ** (Phase 1 chỉ kiểm hậu điều kiện lúc chạy). Sinh verification condition theo weakest-precondition trên MIR.
- Kết quả `unknown`/hết tài nguyên = chưa chứng minh (xử lý theo D25).
- _Hoàn thành khi:_ bộ chương trình xác minh (sắp xếp, tìm kiếm nhị phân, cấp phát…) qua; tỉ lệ check runtime bị xoá tăng đo được so với Phase 1; kết quả xác định (D20).

**8.B — Kernel logic (bộ kiểm chứng bằng chứng nhỏ)**

- Đây là "nhân toán học duyệt nhanh" bạn mô tả, thiết kế thực tế: SMT _tìm_ bằng chứng, **Kernel nhỏ chỉ _kiểm_ bằng chứng** (rẻ hơn nhiều, dễ tin hơn). Kernel viết bằng Rust ít dòng, `forbid(unsafe_code)`, không phụ thuộc solver.
- Lộ trình theo mức độ khả thi: (1) số học tuyến tính qua **chứng chỉ Farkas** (kiểm bằng phép nhân-cộng đơn giản); (2) mệnh đề/bit-vector qua **LRAT/DRAT** (bit-blasting + SAT); (3) mở rộng dần các lý thuyết; phần chưa phủ tạm thời nằm trong TCB và được ghi rõ.
- `gritc recheck` kiểm lại toàn bộ chứng chỉ `.gtproof` **không cần solver**.
- _Hoàn thành khi:_ ≥X% Obligation (X do đo thực tế, ghi vào ADR) được xác nhận bằng Kernel; mọi chứng chỉ giả/sai bị Kernel từ chối (test độc hại).

**8.C — Unsafe có bằng chứng (hoàn thiện D21)**

- Mô hình vùng nhớ & quyền (ownership of memory/permissions kiểu separation logic rút gọn): hợp lệ con trỏ, căn lề, không alias, độ sống, kích thước. Thao tác raw pointer sinh Obligation thật, giải bằng 8.A/8.B.
- Mở khoá `⚠` cho người dùng; thu hẹp danh sách intrinsic trusted trong core (chuyển dần từ "tin" sang "chứng minh").
- FFI qua `extern` + hợp đồng; hiệu ứng `ffi`; danh sách tin cậy ký trong `rig.toml`.
- _Hoàn thành khi:_ cài đặt lại `Vec`/allocator trong Grit với `⚠` được chứng minh; corpus unsafe độc hại (use-after-free, OOB, aliasing) bị chặn.

**8.D — Đa luồng nâng cao**

- Closure + `scope` (luồng có phạm vi mượn), kênh (channel) có kiểu, mô hình bộ nhớ & kiểm tra thứ tự atomic, mở rộng kiểm deadlock/livelock, lượng từ trên bất biến luồng. Tuỳ chọn: model-checking nhỏ cho các primitive đồng bộ của core.
- _Hoàn thành khi:_ corpus race/deadlock mở rộng bị chặn; có test stress đa luồng lặp lớn.

**8.E — Giao thức AI**

- Build trực tiếp trên nền LSP đã có từ **M13** (Phase 1) — không viết lại kênh giao tiếp, chỉ mở rộng: `gritc serve` (JSON-RPC, biên dịch tăng dần), chẩn đoán JSON **schema ổn định có version** (đã đóng băng từ M13), mỗi lỗi kèm **bản sửa dạng edit có cấu trúc**.
- Chế độ `--oracle`: trả lời "vì sao chưa chứng minh được" (obligation nào, giả thiết nào thiếu) để AI bổ sung hợp đồng — phần này **phải** đợi 8.A/8.B vì cần dữ liệu từ SMT/Kernel mới trả lời được "thiếu giả thiết gì", đây là lý do nó không làm được ở M13.
- Vòng lặp "AI viết → gritc kiểm → AI sửa" tự động, chạy trong sandbox, tái dùng inlay-hint/Obligation-status đã có từ LSP M13 để AI "nhìn thấy" trạng thái chứng minh giống hệt người dùng IDE thấy.
- _Hoàn thành khi:_ benchmark "AI sửa lỗi theo chẩn đoán" đo tỉ lệ thành công; schema JSON đóng băng (hoặc giữ nguyên bản đã đóng băng từ M13 nếu không cần đổi).

**8.F — Rig đầy đủ**

- Registry, phiên bản ngữ nghĩa, khoá băm, **chữ ký**, tái lập bản dựng; gói phát hành kèm **chứng nhận kiểm tra/bằng chứng** — người dùng thư viện không phải tin lời tác giả. Chính sách tin cậy cho phụ thuộc; quét chuỗi cung ứng.
- _Hoàn thành khi:_ cài/đăng gói end-to-end; gói bị sửa nội dung bị từ chối.

**8.G — Tối ưu & hiệu năng**

- Dùng bằng chứng để: xoá bounds/overflow check, sinh `noalias`/`nonnull`/range metadata cho LLVM, hoán đổi vòng lặp an toàn. Tinh chỉnh pass LLVM, PGO tuỳ chọn.
- **Cổng hiệu năng:** bộ benchmark (số học, bộ nhớ, đa luồng, phân tích chuỗi, mã hoá) nằm trong **±5% so với C/Rust tương đương** hoặc có giải trình từng ca lệch. Benchmark tự động theo dõi hồi quy trong CI.

**8.H — Tự biên dịch (self-hosting)**

- Port dần gritc sang Grit theo thứ tự: lexer → parser → resolve → types → … → borrowck → solver glue. Mỗi phần có **differential test** với bản Rust (cùng input phải cho cùng chẩn đoán).
- Bootstrap ba tầng: `stage0` (Rust) → `stage1` (Grit biên dịch bởi stage0) → `stage2` (Grit biên dịch bởi stage1); **stage1 và stage2 phải giống hệt từng byte**.
- Phòng "trusting trust": **Diverse Double-Compiling** (biên dịch gritc bằng một toolchain độc lập thứ hai để đối chiếu).
- Thay `gt-rt` bằng runtime viết bằng Grit.
- _Hoàn thành khi:_ stage1≡stage2; toàn bộ test Phase 1 xanh khi chạy bằng gritc tự biên dịch.

**8.I — Thực thi tất định & cận tài nguyên (bổ sung triết lý — mục "Thứ ba", ý 3)**

- Một hồ sơ biên dịch tuỳ chọn `--profile rt` (realtime/bounded), dùng lại đúng hạ tầng đã có (Obligation, `↓`, hiệu ứng `mem`), **không phải hệ thống kiểm tra riêng**:
  - Cấm cấp phát động ngoài `Arena` cố định kích thước đã biết lúc biên dịch hoặc được cấp một lần ở khởi động (siết chặt D26 trong hồ sơ này).
  - Mọi hàm đạt hồ sơ `rt` phải có **cận trên về số bước thực thi** được chứng minh hoặc suy ra từ biến thể `↓` + độ phức tạp của các lệnh gọi con (tái dùng bộ giải 8.A, mở rộng từ "chứng minh dừng" sang "chứng minh cận", đây là phần việc chính của hạng mục).
  - Cấm đệ quy không có cận tường minh, cấm vòng lặp không biến thể (đã cấm sẵn ở D12, hồ sơ `rt` chỉ bỏ các trường hợp ngoại lệ `div`).
  - Hiệu ứng `mem` bắt buộc có cận tường minh (không được để "chưa chứng minh → hạ runtime check" như D24, vì hồ sơ `rt` ưu tiên từ chối biên dịch hơn là kiểm tra lúc chạy).
- Mục tiêu: hợp với hệ nhúng/thời gian thực — nơi "chạy xong trong thời gian hữu hạn với bộ nhớ cố định, kiểm được từ lúc biên dịch" quan trọng hơn tiện dụng.
- _Hoàn thành khi:_ ≥3 chương trình mẫu (một bộ điều khiển vòng lặp cảm biến giả lập, một bộ xử lý gói tin cỡ cố định, một bộ lập lịch đơn giản) biên dịch qua hồ sơ `rt`, có cận bộ nhớ/số bước được in ra, và có test chứng minh chương trình cố ý vi phạm (cấp phát động, đệ quy không cận) bị từ chối đúng mã.
- _Ghi chú phạm vi:_ đây là hạng mục mang tính nghiên cứu, phụ thuộc nhiều vào kết quả 8.A (giải Obligation cận); nên làm sau 8.A, có thể song song 8.D.

---

## 9. CHIẾN LƯỢC KIỂM THỬ & CHẤT LƯỢNG (xuyên suốt, bắt buộc)

| Lớp                            | Nội dung                                                                                                                                                               |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Unit                           | Mỗi crate, mỗi pass                                                                                                                                                    |
| Golden/Snapshot                | Thông điệp chẩn đoán, MIR text, LLVM IR (`insta`)                                                                                                                      |
| **UI test (pass/fail corpus)** | `tests/fail/Exxxx_*.gt` có chú thích mã lỗi kỳ vọng; `tests/pass/*.gt` phải qua. Test runner so khớp **đúng mã** (không chỉ "có lỗi")                                  |
| Property-based                 | `proptest`: round-trip lexer/parser/fmt, dataflow đơn điệu, bảo toàn ngữ nghĩa khi hạ MIR                                                                              |
| Fuzz                           | `cargo-fuzz` cho lexer, parser, typechecker; fuzz có cấu trúc sinh chương trình hợp lệ để tìm false negative                                                           |
| **Soundness hunting**          | Với mỗi nhóm kiểm tra: tự viết "chương trình tấn công" cố lách (alias bằng reborrow, race qua generics, tràn qua ép kiểu…). Mỗi lỗ hổng phát hiện thành test vĩnh viễn |
| Differential                   | Phase 1: so sánh kết quả thực thi với oracle (ví dụ chương trình C/Rust tương đương); Phase 2: bản Rust vs bản tự biên dịch                                            |
| Metamorphic                    | Đổi tên biến, đổi thứ tự khai báo, alias↔canonical ⇒ phán quyết **không đổi**                                                                                          |
| Mutation                       | Đột biến chính mã gritc (`cargo-mutants`): test phải bắt được đột biến ở các pass an toàn                                                                              |
| Tái lập                        | Build 2 lần, so băm                                                                                                                                                    |
| Hiệu năng                      | Benchmark nền từ M10, theo dõi hồi quy                                                                                                                                 |

**Định nghĩa bug nghiêm trọng:** (1) _false negative của kiểm tra an toàn_ (chương trình sai lọt qua) — mức cao nhất; (2) gritc crash/ICE; (3) output không tái lập; (4) false positive (chặn nhầm) — thấp hơn nhưng vẫn phải sửa.

---

## 10. BẢNG RỦI RO

| Rủi ro                                               | Mức        | Giảm thiểu                                                                                                                 |
| ---------------------------------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------- |
| Borrow checker có lỗ hổng sound                      | Cao        | Corpus tấn công, fuzz có cấu trúc, mutation test, port ý tưởng test từ tài liệu NLL/Polonius công khai (không sao chép mã) |
| Phạm vi phình to, không bao giờ xong                 | Cao        | Danh sách cắt bỏ 6.3, cổng milestone, ADR cho mọi thay đổi                                                                 |
| Ký hiệu Unicode làm AI viết sai/tốn token            | Trung bình | Alias ASCII 1–1, đo token ở M0, formatter chuẩn hoá                                                                        |
| Obligation IR thiết kế sai làm Phase 2 phải viết lại | Cao        | Chốt giao diện bằng ADR trước cổng chuyển giai đoạn; thử nghiệm bộ giải giả (mock SMT) ngay từ Phase 1                     |
| Quá nhiều false positive khiến Grit không dùng nổi   | Trung bình | Đo tỉ lệ chặn nhầm trên 3 chương trình thật; mở rộng bộ giải thay vì nới luật                                              |
| Phụ thuộc phiên bản LLVM/binding                     | Trung bình | Ghim phiên bản, lớp bọc mỏng quanh binding, test golden IR                                                                 |
| Solver `unknown` làm kết quả bất định                | Trung bình | Giới hạn theo tài nguyên, cache kết quả theo băm, fail-closed                                                              |
| Lộ secret vì quy tắc VÀNG vẫn cho chạy               | Trung bình | Cờ `--deny-yellow`, hồ sơ release (mục 5)                                                                                  |
| Tin nhầm vào chính gritc                             | Cao        | TCB nhỏ, Kernel (8.B), bootstrap + DDC (8.H)                                                                               |

**Ước lượng thô (không phải cam kết):** Phase 1 khoảng 40–80 nghìn dòng Rust và là dự án nhiều tháng làm đều đặn cùng agent; Phase 2 là dự án nhiều năm, riêng 8.A–8.C mang tính nghiên cứu. Nên coi mỗi milestone là một cổng chất lượng, không phải mốc ngày.

---

## 11. VIỆC CẦN CHỦ DỰ ÁN (GIANG) QUYẾT / XÁC NHẬN

1. ~~Mục "Thứ ba" còn thiếu~~ — **đã chốt**: mô hình bộ nhớ/capability vào Phase 1 (mục 4.0.a/b, D26–D29, M4.5), thực thi tất định có cận vào Phase 2 (8.I).
2. Duyệt hoặc phủ quyết các quy định D1–D25 (đặc biệt D4 cấm `==` trên số thực, D7 cấm shadowing, D12 bắt buộc biến thể cho vòng lặp, D14 cấm FFI ở Phase 1).
3. Duyệt hai khuyến nghị ở mục 5 (`--deny-yellow`, `#allow` có lý do).
4. Duyệt bảng ký hiệu sau khi M0 có kết quả đo token.
5. Cấp quyền cho agent ghi OPEN_QUESTIONS và dừng chờ trả lời khi gặp mâu thuẫn.

---

## PHỤ LỤC — PROMPT KHỞI ĐỘNG CHO CLAUDE COWORK

```
Đọc toàn bộ GRIT_PLAN.md. Bạn là kỹ sư trưởng triển khai Grit (Giai đoạn 1 – Seed).
Quy tắc: làm đúng mục 0, tuần tự từng milestone, fail-closed, không stub, không bỏ qua test.
Hôm nay chỉ làm M0:
1) Dựng workspace, CI, thư mục docs theo 6.1.
2) Viết grammar.ebnf, bảng ký hiệu + bí danh ASCII, đặc tả ngữ nghĩa lõi, danh sách mã lỗi.
3) Đo token canonical vs ASCII, ghi ADR.
4) Viết ≥30 chương trình mẫu .gt (đúng/sai) kèm kết quả kỳ vọng.
5) Cập nhật HISTORY.md và OPEN_QUESTIONS.md, rồi DỪNG chờ duyệt trước khi sang M1.
```
