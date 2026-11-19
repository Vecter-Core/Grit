> **TRẠNG THÁI (GÓI 9): ĐỀ XUẤT, CHƯA DUYỆT; đối chiếu mã: docs/PHASE2_FACTCHECK.md**

# ĐỀ XUẤT BỔ SUNG GIAI ĐOẠN 2 (ASCEND)

> **Opus 5.5 đề xuất thêm vào Giai đoạn 2.** Ngày 2026-11-19 (cập nhật lần 2 cùng ngày: thêm §6 tính năng học từ ngôn ngữ khác, §7 backend nhỏ gọn kiểu Zig, §8 không khoá cứng LLVM). Trạng thái: **ĐỀ XUẤT, CHƯA DUYỆT**. Mọi mục "CÓ" vẫn cần ADR riêng và Giang duyệt trước khi viết mã (AGENTS.md: không đổi ngữ nghĩa/cú pháp, không thêm intrinsic khi chưa có quyết định kiến trúc và hợp đồng TCB).
>
> Nguồn đánh giá: một bản phân tích của AI khác đề xuất học từ Goose và các compiler viết bằng Rust. Tài liệu đối chiếu: `GRIT_PLAN.md` (§3, §6.3, §8), `docs/DECISIONS.md` (ADR-013, 020, 029, 036, 038), `docs/TCB_INTRINSICS.md` (B2/B3), `docs/BENCHMARK.md`, `docs/KNOWN_LIMITATIONS.md`; mã chỉ mở đúng chỗ cần: `crates/gt-mir/src/mir.rs` (cấu trúc `Body`), `crates/gt-codegen-llvm/src/lower.rs` dòng 4 (bố cục enum), `runtime/gt-rt/src/lib.rs` (tìm xoá bộ nhớ).

## 0. Ba sự thật nền phải biết trước khi đọc

1. **Số liệu của Goose là tự báo.** Tìm kiếm chỉ thấy benchmark trong chính kho của tác giả, không có đo độc lập. Chính các nguồn cũng ghi đánh đổi: phần tử cỡ biến thiên không truy cập theo chỉ số được (phải duyệt tuần tự hoặc giữ mảng chỉ mục phụ), dung lượng tối đa cố định lúc tạo, mô hình vòng đời gắn stack khó dùng cho dữ liệu chia sẻ sống lâu. ⇒ Câu "nhanh hơn C++/Rust 1,12–1,16×" **CHƯA KIỂM**, không được dùng làm lý do.
2. **Khoảng cách hiệu năng thật của Grit nằm ở backend, không ở bố cục dữ liệu.** `docs/BENCHMARK.md` (G2.3, sortlines 33 MB): Grit dev 2,67 s; Grit release-o2 1,14 s; Rust `-O` 0,78 s (theo min). Mặc định của Grit vẫn là `dev` (LLVM -O0, mọi check giữ) vì backend O2 nằm sau IR Guard (ADR-036, Q82). Tối ưu bố cục chỉ có ý nghĩa SAU khi chuyện này được giải quyết.
3. **Hiện trạng đã có sẵn:** MIR đã phẳng theo chỉ số (`Body { locals: Vec<LocalDecl>, blocks: Vec<BlockData> }`); Arena của `gt-rt` là bump theo chunk, cỡ không cố định; `Vec::push` cấp khối mới + `memcpy`, khối cũ giữ tới `reset` (TCB B3); enum hạ thành `{ i64 tag, [k x i64] payload }` (rất thô); runtime **không có** cơ chế xoá bộ nhớ Secret nào; `spawn` truyền đối số bằng hộp sở hữu, chưa có `join`, chưa có kênh; Giai đoạn 2 đã có sẵn `gritc serve` biên dịch tăng dần (8.E) và kênh có kiểu (8.D).

## 1. Bảng kết luận

| # | Kỹ thuật | Kết luận | Gắn vào | Lý do chính |
|---|---|---|---|---|
| 1 | Khởi tạo tại chỗ (destination-passing) | **CÓ** (vì hiệu năng) — **KHÔNG** dùng làm biện pháp bảo vệ Secret | 8.G | Giảm `memcpy` là thật; nhưng không ngăn được bản sao Secret (xem §2.1) |
| 1b | Xoá bộ nhớ Secret có bảo đảm (mới, thay cho lập luận sai ở #1) | **CÓ** | 8.C + M9/S2xx | Lỗ hổng thật: hiện không có cơ chế xoá nào |
| 2 | Enum cỡ biến thiên | **KHÔNG** (Phase 2 hiện tại) | — | Mất truy cập chỉ số O(1), bố cục mới phải vào TCB/Guard, lợi ích chưa đo độc lập |
| 3 | Tham chiếu tương đối 1/2/4 byte | **KHÔNG** (là tính năng ngôn ngữ) | — | Arena nhiều chunk, cỡ không cố định ⇒ M7 không có cận; "đọc thẳng từ đĩa" = giải tuần tự không kiểm |
| 4 | Arena đặt trước không gian địa chỉ ảo | **CÓ, CÓ ĐIỀU KIỆN** | 8.G (runtime) | Bỏ `memcpy` và lãng phí ~2× của `Vec::push`; KHÔNG được nới luật M6 |
| 5 | Biên dịch tăng dần theo truy vấn (kiểu Salsa) | **CÓ, giữ đúng chỗ 8.E**, không ưu tiên sớm | 8.E | Đã có trong kế hoạch; rủi ro cache sai = false negative |
| 6 | AST/MIR phẳng theo chỉ số | **KHÔNG** (MIR đã phẳng; AST là quyết định ADR-013) | — | Không có số đo cho thấy thời gian biên dịch là nút thắt |
| 7a | Niche cơ bản + tag nhỏ cho enum | **CÓ** | 8.G | Enum đang dùng tag 8 byte + payload theo từ 8 byte |
| 7b | Niche/nén kiểu theo miền giá trị của M7 | **KHÔNG** | — | Biến lỗi bộ giải thành lỗi hỏng bộ nhớ; M7 chứng minh theo điểm chương trình, không theo kiểu |
| 8 | Luồng không chia sẻ + kênh có kiểu | **CÓ** (làm trong 8.D, không thay `Mutex`) | 8.D | Đã có `Send`-like (E0501) làm nền |

## 2. Lập luận từng mục

### 2.1 Khởi tạo tại chỗ — CÓ cho hiệu năng, sai cho Secret
- **Khả thi:** hạ MIR→LLVM theo kiểu "truyền đích": hàm trả aggregate lớn nhận con trỏ đích (`sret`), `v.push(f(x))` truyền thẳng ô `ptr[len]` làm đích sau khi đã đảm bảo `len < cap`. Không đổi ngữ nghĩa nguồn, không cú pháp mới.
- **Ràng buộc an toàn:** thứ tự đánh giá của Grit là "chỗ trước, giá trị sau" (semantics §3a, KNOWN_LIMITATIONS M5). Ghi thẳng vào đích chỉ hợp lệ khi `f` abort (D3) không để lại trạng thái nửa vời quan sát được — đúng với Grit vì abort dừng tiến trình, nhưng `len` chỉ được tăng SAU khi ghi xong. Cần oracle so dev ≡ release trên chương trình có abort giữa chừng.
- **Vì sao KHÔNG phải biện pháp Secret:** LLVM vẫn tự do sao chép giá trị vào thanh ghi, tràn ra stack khi cấp phát thanh ghi, gộp/tách `memcpy`; hệ điều hành có thể đưa trang ra swap. Khởi tạo tại chỗ chỉ giảm SỐ bản sao trong IR, không bảo đảm bằng 0. Tuyên bố "khoá không bao giờ bị nhân bản" là **sai** và nếu ghi vào tài liệu sẽ tạo cảm giác an toàn giả — đúng loại lỗi `grit-contract §7` cấm.
- **Cổng:** benchmark trước/sau trên 6 dogfood (`tools/bench_profile.py`), differential codegen ≡ thông dịch, IR Guard vẫn sạch.

### 2.1b Xoá Secret có bảo đảm — CÓ (đề xuất mới)
- `Secret<T>` khi hết phạm vi/`reset` Arena phải được ghi đè bằng thao tác **volatile** + rào compiler (`llvm.memset` với cờ volatile), là intrinsic `trusted` mới ⇒ cần dòng hợp đồng trong `TCB_INTRINSICS.md` trước khi viết mã.
- Nói rõ giới hạn trong tài liệu: chỉ xoá ô nhớ do Grit quản lý; KHÔNG xoá bản sao trong thanh ghi đã tràn, swap, core dump. Muốn mạnh hơn (khoá trang `VirtualLock`/`mlock`) là bước sau.
- Phụ thuộc: `Drop` người dùng chưa hạ (KNOWN_LIMITATIONS) — intrinsic này phải do codegen chèn, không dựa vào `Drop`.

### 2.2 Enum cỡ biến thiên — KHÔNG
- Mảng phần tử cỡ biến thiên không có `ptr + i·size` ⇒ mọi chỉ số `xs[i]` (và toàn bộ bằng chứng biên `i < xs.len()` của M7) không áp dụng được; phải thêm kiểu tập hợp mới, đường duyệt mới, nghĩa vụ mới.
- Bố cục mới phải được IR Guard hiểu và thêm vào TCB; oracle bố cục Arena của M6 phải mở rộng. Chi phí kiểm chứng lớn, lợi ích chỉ cho dữ liệu bất biến duyệt tuần tự, số đo duy nhất là tự báo.
- Thay thế đã làm được ngay: người dùng tách "bảng nhỏ + bảng payload lớn" bằng chỉ số. Xem lại khi có Phase 2 ổn định và có ứng dụng thật cần.

### 2.3 Tham chiếu tương đối — KHÔNG
- Arena của `gt-rt` cấp nhiều chunk **không liên tục** và không giới hạn cỡ (`GRITRT_ARENA_LIMIT` mặc định không giới hạn) ⇒ khoảng cách giữa hai nút không có cận tĩnh; M7 không thể chứng minh offset vừa 1/2/4 byte.
- "Ghi/đọc cả cấu trúc xuống đĩa bằng một lệnh kèm kiểm" thực chất là ép kiểu byte → cấu trúc có con trỏ: đó là `unsafe` thật, cần bộ kiểm cấu trúc đầy đủ (mỗi offset, mỗi tag, căn lề) — thuộc 8.C và rất lớn.
- Thay thế: chỉ số `u32` vào `Vec` (mã Grit thường, M7 đã chứng minh được biên). Đạt phần lớn lợi ích bộ nhớ mà không thêm TCB.

### 2.4 Arena đặt trước không gian địa chỉ ảo — CÓ, CÓ ĐIỀU KIỆN
- Chỉ đổi **runtime** (`gt-rt`): đặt trước dải địa chỉ lớn (`VirtualAlloc MEM_RESERVE` / `mmap PROT_NONE`), commit dần. `Vec` ở cuối Arena có thể lớn tại chỗ, bỏ `memcpy` và lãng phí ~2× ghi ở TCB B3.
- **Điều kiện cứng:** KHÔNG nới luật "`push` vô hiệu mọi tham chiếu cũ" của M6, dù về vật lý chúng còn hợp lệ. Nới luật = đổi ngữ nghĩa mượn = phải mở lại đột biến/oracle M6. Lợi ích ở đây chỉ là tốc độ.
- **Rủi ro:** commit charge trên Windows (chính máy dự án thường thiếu bộ nhớ ảo, preflight đỏ GÓI 7/8); phải có đường lùi về chunk khi đặt trước thất bại, và test cả hai đường. Kiểm `arena_overlap` của hồ sơ dev phải giữ nguyên.
- **Cổng:** hợp đồng TCB mới cho `gritrt_arena_*`, đột biến hàm kiểm Arena (`tools/mut_arena.py`), `shared_arena_diff`, benchmark sortlines trước/sau.

### 2.5 Biên dịch tăng dần — CÓ, ở 8.E, không làm sớm
- Thời gian dựng hiện 0,2–1,1 s (gritc **debug**, BENCHMARK.md); chưa đo bản release của gritc — đo việc đó rẻ hơn và nên làm trước.
- Rủi ro đặc thù Grit: cache trả kết quả cũ = chấp nhận chương trình sai = **false negative**. Điều kiện bắt buộc nếu làm: cổng differential "tăng dần ≡ biên dịch từ đầu" trên bộ sinh chương trình, attestation (§3.2) phải băm theo đầu vào thật của từng truy vấn, D9 (tất định) phải giữ.

### 2.6 AST/MIR phẳng — KHÔNG
- MIR đã là `Vec` + chỉ số. AST dùng `Box` là quyết định có lập luận (ADR-013: khớp mẫu, so sánh round-trip). Viết lại AST đụng mọi crate từ parser tới M8, phải đột biến/oracle lại, trong khi chưa có hồ sơ (profile) nào chỉ ra AST là nút thắt. Chỉ xem lại khi có số đo.

### 2.7 Niche — tách đôi
- **7a CÓ:** enum hiện là `{ i64 tag, [k x i64] payload }`. Tag `u8`/`u16` theo số biến thể, payload theo bố cục tự nhiên, và niche cơ bản đã chứng minh trong Rust (`Option<&T>` dùng null; Grit không có null nên tham chiếu luôn khác 0 theo D3). Đây là tối ưu chuẩn, không phụ thuộc bộ giải.
- **7b KHÔNG:** M7 chứng minh "biến `x` tại điểm P nằm trong [0,1000]", không phải "mọi giá trị của trường này mãi mãi trong [0,1000]". Nén bố cục theo đó cần bất biến của KIỂU, mọi lần ghi phải được chứng minh, và một lỗi của M7 (còn đột biến nhóm NGUY HIỂM được chấp nhận bằng lập luận — `docs/M7_DANGER_MUTANTS.md`; mặc định vẫn `--checks=all`) sẽ biến thành **hỏng bộ nhớ** thay vì chỉ thừa/thiếu một check. Ngược với P6 (TCB nhỏ). Nếu muốn, đường đúng là kiểu tinh chỉnh tường minh do người viết khai báo (`u32 ∈ 0..1000`) và chỉ sau 8.A/8.B — nhưng đó là cú pháp mới, cần quyết định kiến trúc.

### 2.8 Luồng không chia sẻ + kênh có kiểu — CÓ, trong 8.D
- 8.D đã có "kênh (channel) có kiểu". Đề xuất cụ thể: kênh chỉ nhận kiểu `Send`-like (đã kiểm bằng E0501), gửi = chuyển sở hữu (move, M6 đã chặn dùng lại), nhận có hiệu ứng, cùng `join` còn thiếu.
- **Không** biến nó thành "mặc định thay Mutex": đổi mặc định là đổi ngữ nghĩa; `Mutex` có cấp khoá (E05xx) vẫn cần cho trạng thái chia sẻ thật.
- Lưu ý trung thực: "triệt tiêu hoàn toàn data race" chỉ đúng cho dữ liệu đi qua kênh; chương trình vẫn có thể dùng `Mutex`/`Atomic`. Kênh vẫn cần đồng bộ phần cứng bên trong (hàng đợi có khoá hoặc atomic) ⇒ là intrinsic TCB mới, cần TSan như H7/H8.

## 3. Thứ tự ưu tiên đề xuất (trong Giai đoạn 2)

1. **Khép khoảng cách backend** (đã có trong 8.G/ADR-036): đưa release-o2 qua đủ bằng chứng để bớt chậm ~1,5× so với Rust `-O`. Lợi nhiều nhất, không đụng ngữ nghĩa.
2. **7a** tag nhỏ + niche cơ bản.
3. **1b** xoá Secret có bảo đảm (lỗ hổng an toàn, không phải tối ưu).
4. **1** destination-passing, **4** Arena đặt trước.
5. **8** kênh có kiểu (theo lịch 8.D).
6. **5** tăng dần (theo lịch 8.E), sau khi đo gritc release.

Không làm: 2, 3, 6, 7b.

Bổ sung từ lần cập nhật 2 (chi tiết ở §6–§8), xếp chung vào thứ tự trên:
- Ngay sau mục 1: **L0–L2 của §8** (GLIR + tách IR Guard). Không đổi hành vi, nhưng là điều kiện để mọi tối ưu bố cục (7a) và mọi backend mới không phải làm hai lần.
- Ngay sau 7a: **N1** (sinh test tự động từ hợp đồng) và **N7** (`?`): lợi lớn cho AI, rủi ro an toàn thấp.
- **N8** (edition + `gritc fix`) phải có TRƯỚC thay đổi cú pháp đầu tiên của Giai đoạn 2.
- **L3–L5 của §8** (backend x64 riêng) chạy song song với 8.A sau khi L2 xong.
- N2–N5 đi cùng 8.A/8.B; N6 cùng 8.C.

## 4. Điều kiện chung cho mọi mục "CÓ"
- ADR viết TRƯỚC mã; intrinsic mới có dòng hợp đồng TCB trước.
- Test ĐỎ trước khi sửa; differential codegen ≡ thông dịch ở dev/release/release-o2; IR Guard cập nhật nếu đổi bố cục; đột biến tự động ≥ một nửa (`tools/mut.py`).
- Benchmark trước/sau bằng công cụ có sẵn, báo **min** và trung vị, nêu rõ máy và nhiễu.
- Chỉ gọi là xong khi cổng liền mạch `bash tools/gate.sh D:/Grit` xanh trên đúng mã đó.

## 5. Phần CHƯA KIỂM trong đề xuất này
- Số liệu Goose (chỉ có nguồn tự báo).
- Mức lợi thật của 1, 4, 7a trên Grit: chưa đo, chỉ là suy luận từ bố cục hiện tại.
- Thời gian biên dịch của `gritc` bản release.
- Chưa đọc toàn bộ `lower.rs`; nhận định bố cục enum dựa trên chú thích đầu tệp (dòng 4).
- (Lần 2) Ước lượng cỡ backend x64 (8–15 nghìn dòng) là suy đoán, chưa có nguyên mẫu.
- (Lần 2) Tốc độ chạy của mã do backend tự viết so với LLVM -O0 trên chương trình Grit: chưa đo. Số "chậm 1,5–3× nhưng dịch nhanh 5–20×" của Zig lấy từ nguồn thứ cấp.
- (Lần 2) Lợi ích của N1, N7, N9 cho AI: suy luận từ loại lỗi thường gặp, chưa đo trên `apps/` hay `USER_BUGS.md`.

## 6. Tính năng học từ ngôn ngữ khác (giữ triết lý an toàn, giúp AI viết)

**Tiêu chí lọc:** (1) không mở lỗ an toàn, không đổi kiểm tĩnh thành kiểm lúc chạy (P2, P4); (2) giúp AI viết đúng ngay lần đầu hoặc tự sửa nhanh; (3) không vi phạm D1–D29, nhất là D8 (cấm macro/eval/reflection); (4) càng ít cú pháp mới càng tốt.

**Ràng buộc thật:** cú pháp và ngữ nghĩa lõi đang ĐÓNG BĂNG (`docs/spec/FREEZE.md`, ADR-C04), và lexer/parser còn khoá tới khi đủ 24 giờ fuzz. Mọi mục "cần cú pháp" dưới đây chỉ được làm SAU khi gỡ khoá, có ADR và có tăng phiên bản. Các mục "không cần cú pháp" làm được sớm hơn.

### 6.1 Nên lấy

| # | Tính năng | Học từ | Giúp Grit thế nào | Cần cú pháp? | Kết luận | Gắn vào |
|---|---|---|---|---|---|---|
| N1 | **Sinh test tự động từ hợp đồng**: `⊳` sinh đầu vào hợp lệ; `⊲` và "không abort" làm oracle | Eiffel AutoTest, QuickCheck/Hypothesis, `go test -fuzz` | AI có vòng "viết → kiểm → chạy thử" mà không phải viết test tay; hợp đồng vừa là đặc tả vừa là oracle. Công cụ nằm NGOÀI TCB (chỉ tìm lỗi, không cấp phép chạy) | Không (lệnh `rig test --auto`) | **CÓ, ưu tiên cao** | 8.E |
| N2 | **`old(e)`** trong `⊲` (giá trị lúc vào hàm) | Eiffel `old`, SPARK `'Old`, Dafny `old()` | Gỡ giới hạn đã ghi: `⊲` nhắc tham số bị gán lại hiện bị E0902 (KNOWN_LIMITATIONS M7). Hạ thành: chép giá trị lúc vào (chỉ khi `Copy`) | Có (từ khoá mới) | **CÓ** | 8.A |
| N3 | **Bất biến kiểu** (`⋈` gắn vào struct) | Eiffel class invariant, Ada `Type_Invariant`, Dafny `Valid()` | Đây là đường ĐÚNG thay cho 7b: bất biến do người viết khai báo, mỗi chỗ dựng/ghi phải chứng minh, nên bố cục có thể dựa vào bất biến mà không tin ngầm vào M7 | Có | **CÓ, sau 8.A** | 8.A |
| N4 | **Kiểu khoảng tường minh** (`u16 ∈ 0..1000`) | Ada/SPARK range subtype | Chặn giá trị sai ngay ở ranh giới kiểu; là nền an toàn cho niche theo miền giá trị (thay 7b) | Có | **CÓ, sau 8.A/8.B** | 8.A, 8.G |
| N5 | **Mã ma (ghost) và bổ đề** | Dafny, SPARK Ghost, Why3 | Bổ sung dữ kiện cho bộ giải mà không tốn thời gian chạy; codegen bỏ hẳn. IR Guard phải khẳng định không còn mã ma trong IR | Có | **CÓ** | 8.A, 8.B |
| N6 | **Kiểu tuyến tính "phải tiêu thụ"** | Austral (tuyến tính + capability, rất gần Grit), Rust `#[must_use]` (bản yếu) | Tệp/giao dịch/khoá phải được đóng; quên dọn ⇒ ĐỎ thay vì rò rỉ. `Cap<T>` hiện là affine (dùng tối đa một lần), tuyến tính là bắt buộc dùng đúng một lần | Có (thuộc tính hoặc lang item) | **CÓ, có điều kiện**: cần `Drop` đã hạ, oracle M6 mở rộng thêm loại cấu trúc | 8.C |
| N7 | **Toán tử lan truyền lỗi `?`** | Rust `?`, Zig `try`, Swift `try` | `SEED_LIMITATIONS` ghi "chưa có `?`"; AI phải lồng `⌥` nhiều tầng, dễ sai. Chỉ là đường tắt cú pháp: hạ thành `⌥` + trả về sớm, không thêm ngữ nghĩa mới | Có | **CÓ** | đầu Giai đoạn 2 |
| N8 | **Edition ngôn ngữ + `gritc fix`** | Rust editions, `go fix` | Mã AI viết năm trước vẫn dịch được; mỗi lần đổi cú pháp có công cụ tự chuyển (dùng lại "bản sửa dạng edit có cấu trúc" đã có) | Không (khoá `edition` trong `rig.toml`; FREEZE cho phép THÊM khoá) | **CÓ** | 8.F, trước thay đổi cú pháp đầu tiên |
| N9 | **Bắt buộc nhãn đối số** khi hàm có ≥ 2 tham số liền nhau cùng kiểu | Swift argument labels, Kotlin/Python đối số có tên | Chặn lỗi đảo đối số (`copy(src, dst)` ↔ `copy(dst, src)`), lỗi máy kiểu không bắt được và rất hay gặp ở mã AI | Có | **CÓ, có điều kiện**: đo trước tần suất lỗi này trên `apps/` và `USER_BUGS.md` | ADR riêng |
| N10 | **Newtype trong suốt** (struct một trường có bố cục y hệt trường) | Haskell `newtype`, Rust `repr(transparent)`, F# units of measure (bản rút gọn) | AI gói `Meters`, `UserId` mà không tốn chi phí; D1 đã cấm ép ngầm nên không trộn nhầm đơn vị được | Không (chỉ là bảo đảm bố cục trong `gt-layout`) | **CÓ** | cùng 7a |
| N11 | **Gắn chứng nhận theo băm của từng định nghĩa** | Unison (mã định danh theo nội dung) | Rig gắn `.gtproof`/chứng nhận kiểm tra vào băm từng hàm; chỉ kiểm lại phần đổi; nền cho biên dịch tăng dần (mục 5) | Không | **CÓ phần lưu trữ**, KHÔNG đổi mô hình mã nguồn | 8.F, 8.E |

### 6.2 Không lấy

| Tính năng | Học từ | Lý do từ chối |
|---|---|---|
| `comptime` (chạy mã lúc biên dịch) | Zig | Gần với macro/eval (D8); phá ngân sách tài nguyên theo bước (D20); tạo đường mã sinh ra không qua kiểm tra như mã thường. Generics đơn hình hoá đã đủ cho Phase 1–2 |
| Tham chiếu thế hệ (kiểm lúc chạy) | Vale | Đổi chứng minh tĩnh lấy kiểm lúc chạy, trái P2/P4 |
| Effect handlers (bắt và tiếp tục hiệu ứng) | Koka, Effekt | Cần closure + continuation, luồng điều khiển phức tạp, khó kiểm. Grit chỉ cần mở `poly` (đa hình hiệu ứng) cho người dùng sau khi có closure (8.D) |
| GC / ARC | Go, Swift | Trái mục 4.0(a) (không GC, cấp phát tường minh) |
| Ngoại lệ có unwinding | C++, Java | Trái D3 |
| Reference capabilities (`iso`/`val`/`ref`) | Pony | Trùng với sở hữu + `Send`/`Sync`-like đã có; thêm một hệ thứ hai làm AI khó viết hơn |

## 7. Backend nhỏ gọn cho Grit (kiểu Zig)

### 7.1 Zig đã làm gì (đã kiểm qua nguồn)
- Zig 0.15.1 đặt backend x86_64 tự viết làm **mặc định cho Debug** trên x86_64 Linux; ghi chú phát hành báo dịch debug nhanh hơn 5×. **Release vẫn dùng LLVM.** Windows chưa đổi mặc định vì linker COFF chưa xong. Vẫn ép LLVM được bằng `-fllvm`.
- Bài học: backend riêng **không thay LLVM cho bản phát hành**; nó thay LLVM cho vòng lặp debug, nơi tốc độ dịch quan trọng hơn tốc độ chạy.

### 7.2 Vì sao Grit hợp với hướng này hơn cả Zig
1. **Mặc định của Grit đã là LLVM -O0** với mọi check giữ nguyên (`dev`, `--checks=all`). Đó đúng là loại mã một backend không tối ưu sinh ra được, nên mất ít tốc độ chạy.
2. **TCB nhỏ hơn hẳn (P6).** GRIT_PLAN §3.3 liệt kê LLVM (hàng triệu dòng C++) trong TCB. Backend không có bộ tối ưu thì không có chỗ "lợi dụng hành vi không xác định". Với hồ sơ dev, đây là bước **tăng** độ tin cậy, không chỉ tăng tốc.
3. **Có hai backend độc lập** thì chạy được kiểm thử vi sai (differential) giữa chúng và với bộ thông dịch. Hạ tầng so codegen ≡ thông dịch đã có (refpat, obgen, fn_value, std...). Đây cũng là nền cho Diverse Double-Compiling ở 8.H.
4. **Khối lượng có thể ước được:** codegen LLVM hiện tại chỉ ~3 650 dòng (`lower.rs`) cộng 517 dòng Guard. Backend x64 riêng phải làm thêm chọn lệnh, cấp ô stack, mã hoá lệnh máy, ghi tệp object. Ước lượng 8–15 nghìn dòng Rust (**CHƯA KIỂM**). Tham chiếu: QBE (backend C có cả tối ưu) đạt cỡ 10% mã của trình biên dịch công nghiệp.

### 7.3 Thiết kế đề xuất: `gt-codegen-x64`
- **Đầu vào:** GLIR (§8.2), không đọc MIR trực tiếp.
- **Đích:** x86-64. Linux ELF trước (giống Zig, tránh vướng linker COFF), Windows COFF sau.
- **Chọn lệnh:** mẫu cố định cho từng thao tác GLIR; mọi local nằm trong ô stack (như -O0); không tối ưu ngoài gập hằng tầm thường. Đơn giản là để kiểm được.
- **Check và abort:** `cmp` + `jcc` tới khối gọi `gritrt_abort`; giữ nguyên D3 và luật "không có `unreachable` ngoài sau abort".
- **Quy ước gọi:** C ABI (SysV/Win64) chỉ ở ranh giới với `gt-rt`; bên trong dùng một quy ước riêng, đơn giản, có tài liệu.
- **Liên kết:** dùng linker hệ thống như hiện nay. Không viết linker riêng.
- **Phần chưa hỗ trợ** (atomic, spawn...): trả lỗi backend, mã thoát 3 (đã đóng băng là "lỗi backend"), **không bao giờ** sinh mã sai.
- **Debug info:** không có ở bản đầu.

### 7.4 Kiểm chứng (thay cho LLVM verifier)
- GLIR Guard dùng chung cho mọi backend (§8.2).
- Vi sai: x64 ≡ LLVM-dev ≡ thông dịch trên toàn bộ corpus và các bộ sinh hiện có; bất đồng ⇒ ĐỎ.
- Bộ mã hoá lệnh được kiểm bằng một bộ dịch ngược độc lập (`llvm-objdump` hoặc crate disassembler) làm oracle NGOÀI TCB; đột biến tự động bộ mã hoá bằng `tools/mut.py`.
- Về sau: kiểm chứng luật chọn lệnh bằng SMT theo hướng Crocus của Cranelift (gắn đặc tả vào luật hạ lệnh, SMT kiểm tương đương) khi 8.A đã có SMT.

### 7.5 So sánh các phương án backend thứ hai

| Phương án | Ưu | Nhược | Kết luận |
|---|---|---|---|
| **Tự viết x64** | TCB nhỏ nhất, kiểm soát hoàn toàn, không phụ thuộc C/C++ | Công sức lớn, lúc đầu chỉ một kiến trúc | **CÓ — khuyến nghị** |
| Cranelift | Rust thuần; có x64/aarch64/riscv64; dịch nhanh; luật hạ lệnh ISLE đang được kiểm chứng hình thức (Crocus) | Phụ thuộc lớn vào TCB; có tối ưu riêng mà Guard không thấy | Phương án dự phòng tốt nếu không đủ sức tự viết |
| QBE | Nhỏ, có tối ưu vừa phải | Viết bằng C; theo chính ghi chú phát hành ~63% hiệu năng gcc -O2 trên coremark gốc; thêm C vào TCB mà không hơn LLVM về bằng chứng | KHÔNG |
| Sinh mã C | Chạy được mọi nơi | UB của C; trình biên dịch C vào TCB; mất kiểm soát việc xoá bộ nhớ Secret | KHÔNG làm backend chính |
| WebAssembly | Có đặc tả hình thức, nhỏ, chạy trong sandbox (hợp chạy mã AI chưa tin) | Cần runtime để chạy, không phải mã máy gốc | **CÓ làm đích phụ**, sau x64 |

## 8. Không khoá cứng Grit vào LLVM

### 8.1 Hiện trạng (đã đo trong mã)
**Đã tốt:**
- `gritc` dựng được không cần LLVM: `gt-driver` có feature `llvm` tuỳ chọn; `--no-default-features` cho `gritc` chỉ có backend thông dịch.
- LLVM nạp động (LLVM-C 20.1.8); FFI thô gom vào `gt-llvm` (66 dòng).
- Đầu ra JSON đã có trường `backend`; mã thoát 3 đã dành cho lỗi backend.
- Mọi pass kiểm tra (M1–M9) không biết gì về LLVM.

**Còn khoá:**
1. **IR Guard kiểm trên LLVM IR** (`gt-guard` phụ thuộc `gt-llvm`). Đây là chỗ khoá nặng nhất, vì Guard là một lớp an toàn.
2. **Attestation và lớp nghĩa vụ gắn bằng metadata LLVM** (`!gt.attest`, `!gt.obl`).
3. **Bố cục dữ liệu** (enum, con trỏ béo, Mutex) quyết định ngay trong `lower.rs`.
4. **Hồ sơ release** gắn với danh sách pass LLVM (ADR-036).
5. **TCB** liệt kê LLVM.

### 8.2 Kiến trúc đề xuất

```
MIR (sau M7)
  ▼
GLIR — Grit Low IR (MỚI, Grit sở hữu)   ← bố cục từ gt-layout, Assert → nhánh tới abort,
  │                                        Obligation-ID trên thao tác nhạy cảm, attestation
  ▼
GLIR Guard (MỚI, độc lập backend)
  ▼
Backend (trait, mỗi cái một cargo feature)
  ├── llvm  → LLVM Guard-post (verifier + kiểm sau tối ưu, như hiện nay)
  ├── x64   → x64 Guard-post tối thiểu (đếm lời gọi abort, không có lệnh ngoài danh sách)
  ├── wasm  (sau)
  └── interp (đã có)
```

- **GLIR:** thấp hơn MIR, tất định, in văn bản ổn định, có golden test. Chốt giao diện bằng ADR giống cách Obligation IR được đóng băng (ADR-039).
- **`gt-layout`:** Grit tự quyết bố cục (làm cùng 7a và N10); mọi backend chỉ làm theo. Bố cục không còn là "chi tiết của LLVM".
- **Chia IR Guard làm hai:** phần độc lập backend (attestation, tập hàm = tập instance, lớp nghĩa vụ, số lời gọi abort không giảm) chạy trên GLIR; phần riêng từng backend chỉ kiểm điều backend đó có thể làm hỏng.
- **Runtime `gt-rt`** giữ C ABI làm giao diện ổn định cho mọi backend.

### 8.3 Lộ trình (dừng ở bất kỳ bước nào cũng không để lại mã dở)

| Bước | Việc | Đổi hành vi? | Cổng phải qua |
|---|---|---|---|
| L0 | ADR; khoanh mọi chỗ phụ thuộc LLVM; đo `gritc` bản release | Không | — |
| L1 | `gt-layout` + GLIR; backend LLVM hạ từ GLIR thay vì MIR | Không (IR LLVM sinh ra tương đương) | vi sai toàn bộ + golden IR + cổng liền mạch |
| L2 | Tách Guard: GLIR Guard + LLVM Guard-post | Không | test tiêm lỗi Guard hiện có, đột biến Guard |
| L3 | `gt-codegen-x64` cho tập con (số nguyên, rẽ nhánh, gọi hàm, struct), Linux; `--backend=x64` thử nghiệm | Thêm tuỳ chọn | vi sai x64 ≡ LLVM ≡ thông dịch |
| L4 | Đủ tập con Phase 1 (lát, Arena/Vec qua gt-rt, enum, I/O); Windows COFF | Thêm | toàn bộ corpus + bộ sinh |
| L5 | Cân nhắc x64 làm mặc định cho `dev` | **Có** (đổi mặc định) | cổng liền mạch xanh trên CẢ HAI backend; Giang duyệt |
| L6 | LLVM giữ cho release; tuỳ chọn Cranelift/Wasm | Thêm | như L3 |

**Điểm mấu chốt:** LLVM không bị bỏ, nó trở thành một backend trong số nhiều backend. **Chỉ cần xong L2 là đã đạt mục tiêu "không khoá cứng"**: từ đó thêm hay thay backend không còn đụng tới các lớp an toàn.

### 8.4 Rủi ro
- Hai backend là hai nơi có bug. Vi sai bắt buộc biến điều đó thành lợi thế: bất đồng ⇒ ĐỎ, không chọn bên nào.
- GLIR thiết kế sai làm phải sửa mọi backend ⇒ chốt bằng ADR và golden test trước khi viết backend thứ hai.
- Công sức lớn trên một máy hay thiếu bộ nhớ ảo (preflight đỏ GÓI 7/8) ⇒ mỗi bước nhỏ, mỗi bước một cổng xanh, không làm song song với việc nặng khác.
- Mặc định đổi ở L5 là thay đổi người dùng thấy được ⇒ cần Giang duyệt riêng.

## Nguồn ngoài đã tham khảo
- Goose: https://github.com/aardappel/goose/wiki ; https://news.ycombinator.com/item?id=50224852 ; https://zeli.app/story/50224300
- Tham chiếu tương đối: https://www.gingerbill.org/article/2020/05/17/relative-pointers/
- Zig backend x86 tự viết: https://ziglang.org/download/0.15.1/release-notes.html ; https://ziggit.dev/t/self-hosted-x86-backend-is-now-default-in-debug-mode/10447?page=2
- QBE: https://c9x.me/compile ; https://bittide.aicompass.dev/article/c6325ab7-cf06-4029-952b-d7751ed13a77?locale=en ; https://briancallahan.net/blog/20210829.html
- Cranelift/Crocus: https://par.nsf.gov/biblio/10546563 ; https://cs.wellesley.edu/~avh/veri-isle-preprint.pdf
- Các tính năng ở §6 (Eiffel, SPARK/Ada, Dafny, Austral, Unison, Koka, Vale, Pony) lấy từ hiểu biết chung về các ngôn ngữ đó, chưa tra lại tài liệu gốc trong phiên này.
