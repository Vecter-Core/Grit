# ROADMAP — hoàn thiện Grit (ĐỀ XUẤT, CHƯA DUYỆT; lập 2026-11-21, GÓI 12)

> Trạng thái: **đề xuất của người thực thi cho Giang.** Không mục nào ở đây là quyết định cho tới khi Giang duyệt. Mọi mục đụng `crates/`, `runtime/`, `lib/` đều cần lệnh bằng văn bản của Giang
> (AGENTS.md: Giai đoạn 2 và cú pháp/intrinsic mới bị cấm khi chưa có lệnh/ADR/hợp đồng TCB). Việc đã làm cùng gói này chỉ là tài liệu + test (xem `docs/STATUS.md` mục 8).
> Kế hoạch Giai đoạn 2 chi tiết (bốn dòng việc A–D, ADR nháp) vẫn ở `docs/PHASE2_PLAN.md` và `docs/PHASE2_ADR_DRAFTS.md`; tệp này là kế hoạch GẦN: từ `1.0.0-rc7` tới "viết được chương trình thật".

## 0. Phản biện danh sách "còn thiếu" của Giang (đã đối chiếu với mã, không chỉ với tài liệu)

| Ý trong danh sách | Kết luận | Bằng chứng đã kiểm |
|---|---|---|
| `Cap<Net>::connect`, `Cap<Env>::var`, `Cap<Clock>::now`, `Cap<Fs>::sub` "có chữ ký, check xanh, không chạy được" | **ĐÚNG** | khai báo ở `lib/core/prelude.gt:113,117,119,135`; backend chỉ hạ `arg/arg_count/open/read/write/read_all/write_*` (`gt-codegen-llvm/src/lower.rs:2156–2201`); còn lại rơi vào "hàm không có thân MIR" (`lower.rs:2252`) ⇒ thoát 3 |
| `Option::unwrap/is_some` cũng vậy | **ĐÚNG, thiếu trong danh sách** | `docs/SEED_LIMITATIONS.md` ("`check` XANH nhưng backend từ chối") |
| Backend từ chối `f32`, nhân/chia/kiểm tràn 128 bit, `Atomic<bool>`, `MulSat` | **ĐÚNG** | `lower.rs:718,1431` (f32), `1738,1745,1850` (128 bit), `1794` (MulSat), `2350` (Atomic bool). Riêng `f32`: **Giang đã cắt khỏi Phase 1 (Q79, 2026-10-14)** nên không phải thiếu sót |
| Không đọc stdin theo dòng | ĐÚNG | chỉ có `Cap<In>::read_all` |
| "Không có `?`" là lỗ hổng | **CẦN CHỈNH** | Giang đã **HOÃN** `?` (Q85) vì nhật ký capstone ghi "không cần": cả bốn capstone viết được không có `?` và không có phân rã tuple |
| "Không FFI là lỗ lớn nhất" | **ĐÚNG về tác động, SAI về thứ tự làm** | `⚠` ở mã người dùng hiện bị E1001 từ chối hoàn toàn (D21). Mở FFI trước khi có "unsafe có bằng chứng" (8.C) là phá fail-closed. FFI phải đứng SAU 8.C, không trước |
| `async`, `dyn`, macro | **KHÔNG nên làm** (hoặc để ngỏ vô hạn) | GRIT_PLAN D8 cấm macro/reflection; `dyn`/`async` đẩy đồng thời vào borrowck + effects + term + send, đổi mức tin cậy nhiều nhất cho ít lợi nhất |
| "Không kho gói" | ĐÚNG, nhưng **rủi ro chuỗi cung ứng** | kho gói cần chữ ký + capability manifest (GRIT_PLAN 8.F) trước khi cho phép cài; không làm sớm |
| "stdout không đệm" | ĐÚNG, nhưng **là quyết định của Giang (Q75)** | `KNOWN_LIMITATIONS.md:269`: đệm sẽ đổi ngữ nghĩa lỗi/abort. Cách đúng: thêm `flush` tường minh + bộ ghi đệm opt-in trong `lib/std`, KHÔNG đổi mặc định của `gt-rt` |
| "Máy này không dựng được vì không có rustc" | **KHÔNG ĐÚNG ở môi trường hiện tại** | `cargo 1.98.1` (đúng bản ghim) dựng được `gritc` không-LLVM trong 25 giây và chạy 160+ bộ test. Chỉ thiếu LLVM 20 (máy có 17/18) nên backend `llvm` và các test chạy nhị phân thật không dựng được ở đây |
| "Compiler lõi không đứng đầu danh sách" | **ĐỒNG Ý** | lõi đã có corpus 989 mẫu + oracle + đột biến; khoảng cách lớn nhất là bề mặt dùng được, không phải soundness |

Hai mục tôi **thêm** vào vì danh sách bỏ sót: (a) khoảng cách `check` xanh ↔ `build` đỏ (mục 1.1) là vấn đề trung thực của sản phẩm, không chỉ thiếu tính năng; (b) fuzz typechecker: đã có target `crates/gt-driver/fuzz/fuzz_targets/check.rs` nhưng `.github/workflows/fuzz.yml` chỉ chạy `lex` và `parse` ⇒ điều kiện cứng #2 chưa thể đạt.

## 0b. Kế hoạch hoàn thiện Giai đoạn 1 THEO TỪNG M (lệnh Giang 2026-11-21: "chỗ nào thừa gộp, chỗ nào thiếu viết", Giai đoạn 2 để sau)
Cách đo: viết chương trình theo idiom của Rust (thứ người viết sẽ thử trước), chạy `gritc check`, đếm cái bị từ chối oan; đọc nhật ký lỗi thật
(capstone/apps/USER_BUGS: E0702 ×11, E0902 ×4, E0301 ×3, E0403 ×2); dò mã lặp (cửa sổ 8 dòng chuẩn hoá trên `crates/*/src`: **2,6%** dòng —
mã nguồn KHÔNG thừa nhiều; phần thừa thật nằm ở test/tài liệu và đã dọn ở GÓI 12 phần đầu).

| M | Trạng thái | ĐÃ LÀM trong GÓI 12 (có test) | CÒN THIẾU — việc kế, theo thứ tự | Thừa / gộp |
|---|---|---|---|---|
| M1 lexer | đạt, đóng băng | golden `tests/lexer/all_tokens` (91 token) thay 2 test nội bộ | fuzz 24 giờ (điều kiện #2) | — |
| M2 parser/fmt | đạt, đóng băng | 11 mẫu compile-fail cho nhánh thông báo lỗi; bỏ test hình-dạng-AST | (edition) or-pattern `A \| B`, guard `⎇` trong nhánh, phân rã `(a, b) ≔`, `?` (Q85 hoãn): mỗi cái một ADR, gom MỘT đợt | — |
| M3 giải tên | đạt | — | — | — |
| M4 kiểu | đạt | **ép `&[T; N]` → `&[T]`** (ADR-C15) | `==`/`≠` cho tuple/`Option`/struct (E0301 ×3 trong nhật ký): suy `Eq` theo cấu trúc như `Copy` (ADR); `Ord` dùng toán tử `<` trên `T: Ord`; ép ở đuôi khối thân hàm | `TypesOutput` rỗng dựng 1 chỗ (`empty_output`) thay 2 |
| M5 MIR | đạt | hạ MIR khớp đặc tả §3a cho autoref/reborrow (gộp `stabilize`/`snapshot`) | — | — |
| **M6 mượn** | đạt có điều kiện | **mượn hai pha** (ADR-C14). Đo: 14/15 idiom Rust NLL đã qua TRƯỚC khi sửa; lỗ duy nhất là hai pha. Nay 15/15 | bộ sinh oracle sinh dạng `x.m(x.f)`; "problem case #3" (Polonius; Rust cũng từ chối) KHÔNG làm; region theo trường ADT (đang gộp mọi region của ADT: chặn oan hiếm) | — |
| M7 nghĩa vụ | đạt có điều kiện | quan hệ `/`, `%` (ADR-C16) | E0902 ×4: hợp đồng trên đối số KHÔNG Copy không hạ được lúc chạy ⇒ cho phép khi chứng minh được ở chỗ gọi, còn lại ĐỎ như cũ; nhân hằng (`x * 2`) bằng quan hệ tuyến tính có hệ số; đột biến tự động cho mã mới (đang chạy) | — |
| **M8 dừng** | đạt có điều kiện | vòng `x ← x / k`, Euclid `gcd(b, a % b)` | E0702 vòng nhân đôi (`x ← x * 2` tới cận); ngân sách 24 biến/hàm (AI_PITFALLS #2): cấp chỗ zone theo vòng thay vì theo hàm; vòng theo cờ bool (đúng là phải ĐỎ trừ khi có nhiên liệu) | — |
| M9 gritc/secret | đạt | — | — | — |
| M10 LLVM | đạt có điều kiện | — | **khai báo mà không hạ được** (`check` XANH, `build` mã 3): `Env::var`, `Clock::now`, `Fs::sub` (hạ), `Net::connect` (gỡ tới Giai đoạn 2), `Option::is_some/unwrap`; nhân/chia 128 bit; vòng khởi tạo `■` (NEXT_PRODUCT_BATCH #8) | — |
| M11 rig | đạt | — | — (kho gói là Giai đoạn 2) | — |
| M12 std | dở | `std::Slab` (ADR-C17) | `GlobalAlloc` (GRIT_PLAN §4.0a, thiếu) + `Drop` người dùng được hạ (ADR-C18 đề xuất) ⇒ `Vec`/`HashMap` phần tử không-Copy; `split/trim/find/parse`, in/đọc `f64`, `read_line`, bộ ghi đệm opt-in (Q75 giữ mặc định) | — |
| M13 LSP | tối thiểu | — | hover kiểu, đi tới biến cục bộ (điều kiện #8) | — |

Những gì KHÔNG làm trong Giai đoạn 1 (đúng lệnh "Giai đoạn 2 cứ từ từ"): FFI, closure bắt biến, `async`/`dyn`, kho gói, SMT/kernel, backend x64, tối ưu ±5%.

## 1. Nguyên tắc xếp thứ tự
1. **Đóng khoảng "khai báo ≠ hạ được" trước khi thêm khai báo mới.** Một hàm trong prelude hoặc chạy được ở `build`, hoặc `check` phải ĐỎ.
2. **An toàn/bằng chứng trước tính năng**; cú pháp mới chỉ vào MỘT đợt "edition" (ADR-P2-01, đã ủy quyền 2026-11-20) sau khi lexer/parser mở băng và fuzz 24 giờ.
3. **Mỗi mục có tiêu chí xong đo được và test E2E** (mẫu corpus `.gt` + `gritc`; xem mục 4). Không báo xong khi chưa có cổng liền mạch xanh trên đúng mã đó.
4. Mỗi intrinsic/lang item mới cần dòng TCB + hợp đồng (`docs/TCB_INTRINSICS.md`) TRƯỚC khi viết mã.

## 2. Các đợt

### Đợt 1 — Khớp lời khai với thực tế (Phase 1, không cú pháp mới) — làm TRƯỚC
| # | Việc | Tiêu chí xong (đo được) | Phụ thuộc / TCB |
|---|---|---|---|
| 1.1 | Quyết định cho từng hàm khai báo mà chưa hạ: **hạ** (đề xuất: `Env::var`, `Clock::now`, `Fs::sub`) hoặc **gỡ khỏi prelude** (đề xuất: `Net::connect`, để Giai đoạn 2); `Option::is_some/unwrap` cần quyết định riêng (hợp đồng `⊳ self.is_some()` không kiểm được lúc chạy, FB-6) | Test E2E duy nhất: quét `lib/core/prelude.gt` và `lib/std`; mỗi hàm không thân phải có mẫu `tests/pass` chạy được ở `gritc build` HOẶC không còn trong prelude. `SEED_LIMITATIONS` hết dòng "`check` XANH nhưng backend từ chối" | mỗi intrinsic được hạ = một dòng TCB mới + hợp đồng; việc ĐỌC mã runtime cần reviewer |
| 1.1b | Cùng loại "check xanh, build đỏ": vòng khởi tạo `■ X ≔ f()` mà `f` đọc `X` (`NEXT_PRODUCT_BATCH` #8, GÓI 11) ⇒ thêm kiểm đồ thị khởi tạo static để ĐỎ ngay ở `check`; kèm `NEXT_PRODUCT_BATCH` #1–#4 (`{detach}`, `declassify`, xoá `Secret`) | mẫu `tests/fail` mới; `check` ĐỎ | M4/M8; ADR-C12/C13 |
| 1.2 | `Cap<In>::read_line`, `Cap<Out>::flush` (stdout **vẫn không đệm** theo Q75) và bộ ghi đệm opt-in trong `lib/std` | mẫu đọc stdin từng dòng chạy ở dev và release-o2; benchmark in 1 triệu số: ≥ 3× nhanh hơn không đệm | `gt-rt` (nhóm CAO) |
| 1.3 | Backend: nhân/chia/kiểm tràn 128 bit và `MulSat` (hoặc ĐỎ ở `check` nếu không hạ) | mẫu `u128` chạy và khớp mô hình Rust trong `llvm_diff*`; không còn "cần `__multi3`" | `lower.rs`; IR Guard phải biết lời gọi mới |
| 1.4 | `==`/`≠` cho tuple/struct/`Option` bằng derive tường minh hoặc impl `Eq` trong std | cần ADR trước (đổi ngữ nghĩa so sánh) | **cần Giang**: ADR |
| 1.5 | Thông báo E0702/E0701 khi vượt 24 biến nêu đúng nguyên nhân (đã có `limit_hint`): kiểm lại với `apps/` | 3 app mẫu không còn cần "nhiên liệu" ngoài ý muốn | `gt-term` |

### Đợt 2 — Thư viện chuẩn đủ cho công cụ ~500 dòng (`lib/std` viết bằng Grit; KHÔNG cú pháp mới)
`Drop` người dùng được hạ (ADR) → `Vec`/`HashMap` chứa phần tử không-Copy, xoá từng phần tử; in/đọc `f64`; `split/trim/find/parse`; JSON ra cây giá trị; `BTreeMap`. Tiêu chí: viết lại 2 capstone dùng `HashMap<K, V>` không-Copy, khớp bản tham chiếu Rust từng byte; thư viện đi qua CÙNG cổng như mã người dùng (không thuộc TCB).
Phụ thuộc: `Drop` (hiện "bị bỏ qua", `KNOWN_LIMITATIONS.md:233`) — **việc lớn nhất của đợt này**, cần ADR và oracle M6 mở rộng.

### Đợt 3 — "Edition": cú pháp mới trong MỘT đợt (cần ADR + Giang duyệt từng ký hiệu)
Thứ tự đề xuất theo rủi ro tăng dần: toán tử bit (đã có hàm, chỉ thêm cú pháp) → phân rã tuple trong `≔` → `?` (desugar thành `⌥`+`return`, **Q85 đang hoãn**) → closure KHÔNG bắt biến (con trỏ hàm có tên viết gọn) → closure bắt biến (đụng borrowck + effects + term + send: lớn nhất, đặt cuối).
Điều kiện vào (ADR-P2-01): lexer/parser mở băng, fuzz 24 giờ MỘT lần; corpus pass/fail + round-trip `fmt` cho từng cú pháp; không đổi phán quyết chương trình cũ.

### Đợt 4 — Ra ngoài thế giới (Giai đoạn 2; mỗi mục một ADR)
8.C `unsafe` có bằng chứng (mở `⚠`, nghĩa vụ `⊢`) → FFI/`extern` (libc, SQLite, OpenSSL…) → `Cap<Net>` thật → kênh/`join`/thread pool (8.D, sau ADR-C12). **FFI là mục tăng TCB nhiều nhất: không bắt đầu trước 8.C.**

### Đợt 5 — Hệ sinh thái
Kho gói có chữ ký + khai báo capability (8.F); `gritc lsp` đủ hover/định nghĩa cục bộ/hoàn thành; bản cài một lệnh (`tools/install.sh` đã có cho Linux); REPL (thấp nhất).

### Đợt 6 — Hiệu năng
Đưa `release-o2` thành mặc định chỉ khi cổng xanh cả hai backend (Q82, ADR-036); mục tiêu ±5% so với C/Rust (8.G) là Giai đoạn 2, chưa đo được.

## 3. Việc chặn mọi đợt (không phải tính năng)
- Cổng Linux rc7 CHƯA chạy; GitHub bị đình chỉ (Actions khoá, không push) — `docs/STATUS.md` mục 5.
- Fuzz 24 giờ cho lexer + parser (+ typechecker, chưa có trong `fuzz.yml`); đọc mã chéo cho ≈ 63.000 dòng checker; hai phiên capstone sạch. Đây là điều kiện nhãn `1.0.0-seed`, không phải của ROADMAP này.

## 4. Cách kiểm thử (áp dụng cho mọi mục ở trên)
Quyết định của Giang 2026-11-21: **E2E trước, test nội bộ sau cùng.**
1. Mỗi hành vi ngôn ngữ/thư viện mới = một hoặc vài mẫu `.gt` trong `tests/{pass,fail,warn}` (dòng `// expect:`). `crates/gt-driver/tests/corpus_e2e.rs` chạy `gritc check --json` trên TOÀN BỘ corpus và đòi **đúng một** mã lỗi kỳ vọng. Không viết Rust mới cho việc này.
2. Hành vi chạy (đúng/sai kết quả) = chương trình chạy ở `llvm`/`interpreter` so với MÔ HÌNH ĐỘC LẬP (bộ sinh + oracle đã có: `llvm_diff*`, `std_lib`, capstone).
3. Test hồi quy chi tiết CHỈ viết khi có lỗi thật (FB-xx hoặc USER_BUG): test ĐỎ trước, sửa sau trong commit riêng (AGENTS.md).
4. Tìm sập bằng fuzz (lexer/parser/**check**), không bằng test viết tay thêm.
5. Cấm: test chỉ kiểm cấu trúc dữ liệu nội bộ (hình dạng AST, tên trường, `NodeId`), test lặp lại logic của hàm vừa viết, test `#[ignore]` chỉ để in ra.
