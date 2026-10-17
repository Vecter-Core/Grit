---
name: grit-contract
description: Hợp đồng làm việc dự án Grit (bản sao của CLAUDE.md gốc, file gốc đã mất khỏi đĩa). Đọc trước khi chạm vào code Grit.
---

# Grit — hợp đồng làm việc

> Bản sao nguyên văn của `D:\Grit\CLAUDE.md` (file gốc không còn trên đĩa và chưa từng được commit). Đây là hợp đồng, không phải gợi ý. Đọc trước khi chạm vào code.

## 1. Dự án này là gì

**Grit** là một ngôn ngữ lập trình mới, đuôi file `.gt`, **thiết kế cho AI viết code, không phải cho con người viết dễ dàng**. Cú pháp dùng ký hiệu toán học/logic ngắn gọn (có bí danh ASCII đi kèm để AI gõ được bằng bàn phím thường). Comment, chuỗi, commit message thì dùng ngôn ngữ tự nhiên (Việt/Anh) thoải mái.

Trình biên dịch tên `gritc`. Triết lý cốt lõi, nhớ nằm lòng:

> **Compiler không tin lời lập trình viên (hay AI) — mọi khẳng định an toàn phải có bằng chứng. Khi không chắc, compiler từ chối chạy (fail-closed), trừ một ngoại lệ duy nhất: lộ secret/key chỉ bị cảnh báo vàng và vẫn cho chạy.**

Mục tiêu kỹ thuật: an toàn hơn Rust (sở hữu/mượn + bằng chứng cho `unsafe`, không chỉ "lời hứa"), nhanh ngang C (biên dịch qua LLVM, xoá hết check nào chứng minh được lúc biên dịch).

Dự án chia **hai giai đoạn**:
- **Phase 1 — Seed**: viết `gritc` bằng Rust, chỉ hỗ trợ tập con ngôn ngữ nhưng kiểm tra **thật**, không giả.
- **Phase 2 — Ascend**: nâng dần (SMT, kernel chứng minh, unsafe thật, tự biên dịch bằng chính Grit…).

Bạn (Claude Code) hiện đang làm **Phase 1**. Không nhảy sang ý tưởng của Phase 2 trừ khi được yêu cầu rõ ràng.

## 2. Việc đầu tiên phải làm

1. Đọc toàn bộ `GRIT_PLAN.md` — nguồn sự thật duy nhất về kiến trúc, milestone, luật ngôn ngữ (D1–D29), hệ mã lỗi, tiêu chí hoàn thành.
2. Xác định milestone hiện tại bằng cách đọc `docs/PROGRESS.md` (nếu chưa có file này, nghĩa là đang ở M0).
3. **Không được làm milestone sau khi milestone trước chưa đạt "Tiêu chí hoàn thành"** như ghi trong `GRIT_PLAN.md` mục 6.2.
4. Nếu thấy mâu thuẫn, lỗ hổng, hoặc chỗ thiếu trong `GRIT_PLAN.md` — **dừng lại, ghi vào `docs/OPEN_QUESTIONS.md`, không tự đoán rồi code tiếp.**

## 3. Quy tắc bất biến — đọc lại mỗi khi bắt đầu phiên làm việc mới

### 3.1 Cấm làm qua loa
- Không `todo!()`, `unimplemented!()`, `// TODO` không gắn issue, không stub giả vờ chạy được.
- Không test bị tắt (`#[ignore]`) để "cho qua cổng CI".
- Chưa làm được → ghi thẳng vào `docs/KNOWN_LIMITATIONS.md`, để compiler **từ chối** case đó (báo lỗi rõ ràng) thay vì âm thầm bỏ qua.
- Đây không phải sở thích phong cách — đây là **yêu cầu cứng của chủ dự án**. Code dở dang ở Phase 1 sẽ thành bug ngầm ở mọi bản sau, vì các giai đoạn sau *xây trên* giả định rằng phần trước đã đúng.

### 3.2 Fail-closed là luật, không phải tuỳ chọn
- Lỗi đỏ (E...) hoặc lỗi nội bộ compiler (ICE) ⇒ **không sinh binary, không chạy**, bất kể lý do gì.
- Chỉ đúng một ngoại lệ: cảnh báo vàng (S...) — lộ secret/key — vẫn cho chạy, chỉ in cảnh báo.
- Khi một bộ giải (interval solver, SMT...) trả về "không biết" / hết tài nguyên ⇒ coi như **chưa chứng minh = không được qua**, không bao giờ coi "không biết" là "an toàn".

### 3.3 Test trước khi tin
Mỗi pass kiểm tra mới (borrow checker, hiệu ứng, biên, đa luồng...) phải có:
- Một **corpus chương trình xấu** cố tình vi phạm — phải bị chặn **đúng mã lỗi**.
- Một **corpus chương trình đúng** — không được chặn nhầm.
- Nếu sau này phát hiện một case lọt qua (false negative) — đây là **bug mức nghiêm trọng nhất có thể có** trong dự án này. Phải thêm ngay vào corpus vĩnh viễn, không chỉ sửa rồi thôi.

**Luật bộ sinh (Giang, 2026-10-07, từ bài học M6):** mỗi lỗ hổng soundness (false negative) tìm ra bằng cách KHÁC với bộ sinh chương trình ngẫu nhiên (viết tay, đột biến, hunting) thì PHẢI mở rộng bộ sinh để TỰ tìm được nó, rồi kiểm bằng cách TẮT bản sửa: bộ sinh phải tự phát hiện (ghi kết quả vào PROGRESS.md), và loại cấu trúc mới phải vào bảng độ phủ có ngưỡng. Một oracle/bộ sinh chỉ phủ những cấu trúc nó sinh ra — "oracle sạch" không phải chứng minh.
- **Điểm mù của oracle (Giang, 2026-10-08, từ lỗ hổng #4 của M7):** khi một lỗ hổng soundness CHỈ được tìm bằng cách mà oracle không quan sát được (ví dụ chương trình bị một kiểm tra khác từ chối nên oracle không bao giờ so được hành vi, như `⊲` nhắc tham số bị gán lại ⇒ E0902), thì phải (1) ghi LÝ DO oracle mù vào `KNOWN_LIMITATIONS.md` mục "những gì oracle không quan sát được", và (2) thêm một test golden (chương trình nhỏ + kết quả CHÍNH XÁC) giết được lỗ hổng đó — vì "tắt bản sửa" không thể dùng oracle để kiểm. Chỉ số "tỉ lệ lần chạy tới cuối hàm" của oracle phải được đo và báo trong PROGRESS.md; loại nghĩa vụ dưới ngưỡng (mặc định ≥ 100 check bị xoá mà đoạn sau chạy tới cuối hàm) chưa được tính là đã kiểm.
- **Luật "chỉ THÊM, không GỠ" (Giang, 2026-10-17, R4):** biến môi trường, cờ dòng lệnh hay cấu hình KHÔNG được gỡ bỏ một kiểm tra an toàn (kể cả ở hồ sơ dev); chúng chỉ được THÊM kiểm tra (vd `GRITRT_ARENA_CHECK=1` ở release). Công tắc tắt phục vụ đo hiệu năng, nếu cần, phải là cargo feature CHỈ-CHO-TEST không có trong `gritc` phát hành (có test khẳng định như `release_has_no_allow_intrinsic_values`).
- **Luật "kiểm tra gắn với điểm gọi trực tiếp phải được thử qua giá trị hàm" (Giang, 2026-10-17, R4; từ FB-15/E0313):** MỌI lớp kiểm tra mà cơ chế dựa vào việc nhìn thấy lời gọi TRỰC TIẾP (tiền điều kiện `⊳` của intrinsic, lớp nghĩa vụ IR Guard, E0310 capability, E0405 Arena, thứ tự atomic E0503, hiệu ứng, khoá…) PHẢI có test chạy cùng hành vi khi hàm được lấy làm GIÁ TRỊ rồi gọi gián tiếp (biến cục bộ, tham số kiểu hàm, qua `⌥`); thiếu test này thì lớp kiểm tra chưa được coi là đã kiểm thử.
- Bộ đếm phòng thủ (nhánh `return false`/kiểm độ dài chỉ-để-phòng-thủ) phải FAIL-CLOSED (ICE E99xx), không âm thầm chấp nhận hay bỏ qua (P3), và bằng 0 trên corpus + bộ sinh ngẫu nhiên là bằng chứng KIỂM THỬ, không phải chứng minh.

- **LUẬT "không có khẳng định tường minh thì không phải đã chứng minh" (Giang, 2026-10-15, từ B / FB-7):** mọi kiểm tra QUYẾT ĐỊNH CHẤP NHẬN (cho chạy, xoá check, báo "dừng", báo "hiệu ứng khớp", cho qua IR) phải dựa trên một KHẲNG ĐỊNH DƯƠNG của bước trước, KHÔNG dựa trên việc bước trước không báo gì (tập rỗng, `None`, `unwrap_or_default`, `all()` trên tập rỗng, `continue` khi thiếu dữ liệu). Thiếu khẳng định ⇒ từ chối (hoặc ICE E99xx), không phải chấp nhận. Khi viết một bước chấp nhận mới: nêu khẳng định dương nó dựa vào, và viết test TIÊM LỖI làm mất khẳng định đó (nút/nghĩa vụ mà Phase 1 không sinh ra) và đòi từ chối. Khẳng định dương mà từng crate quyết định hiện dựa vào:
  - `gt-oblig` (M7): xoá check / coi nghĩa vụ đã chứng minh CHỈ khi bộ giải trả `Verdict::Proved(cert)`; `Unknown`, hết ngân sách, nghĩa vụ không biểu diễn được (nguyên tử mờ) ⇒ giữ check; điều kiện dừng "không có cạnh lùi" là khẳng định tường minh `BodyReport::no_back_edge_heads`.
  - `gt-term` (M8): chấp nhận vòng lặp CHỈ khi (a) có ít nhất một nghĩa vụ biến thể và mọi nghĩa vụ (hoặc một ứng viên tự suy) `Status::Discharged`, hoặc (b) đầu vòng nằm trong `no_back_edge_heads` của M7; chấp nhận đệ quy CHỈ khi nghĩa vụ lời gọi đệ quy `Discharged` và đồ thị instance dựng được (lùi về đồ thị bảo thủ được ĐẾM).
  - `gt-effects` (M8): hiệu ứng của một lời gọi lấy từ chữ ký callee TÌM THẤY (`sig_by_node`/thành viên trait); không tìm thấy, khối không terminator, hàm đối số `with` không xác định ⇒ E9901.
  - `gt-borrowck` (M6): chấp nhận một thân CHỈ khi dataflow giải xong (`Ok`), không chốt chặn phòng thủ nào bị chạm (`Gen.ice` rỗng) và không có xung đột với khoản vay còn hiệu lực; `check_program` chỉ chạy khi M4 và M5 sạch (nếu không trả rỗng vì bước trước ĐÃ ĐỎ — cổng phán quyết dựa vào chẩn đoán của bước trước, không dựa vào rỗng của M6).
  - `gt-guard` (M10): chấp nhận IR CHỈ khi `check_module`/`check_post` trả rỗng: verifier LLVM sạch, metadata `!gt.attest` TỒN TẠI và bằng attestation của `check`, tập hàm định nghĩa BẰNG tập instance, mọi thao tác nhạy cảm mang lớp nghĩa vụ ĐÚNG, số `gritrt_abort` theo (hàm, loại) không giảm (Guard-post); thiếu metadata là vi phạm, không phải "không có gì để kiểm".

**Nguồn đột biến (Giang, 2026-10-17, từ FB-12):** MỖI lô đột biến kiểm tra phải có ÍT NHẤT MỘT NỬA là đột biến TỰ ĐỘNG (`tools/mut.py list` liệt kê; không phải tập do chính người viết mã chọn tay). Đột biến tay chỉ BỔ SUNG cho chỗ công cụ không sinh được (arm `match`, `matches!`, `.replace`, vòng lặp danh sách). Tệp kết quả `.jsonl` ghi nguồn từng dòng (`"source": "tự động"` hoặc `"tay"`); lô nào tay > tự động là chưa đủ quy tắc. Đột biến theo hướng CHẤP NHẬN thêm đầu vào/IR sống sót phải có test bắt hoặc lập luận tương đương cụ thể.

**Phạm vi đột biến (mutation testing) — Giang quyết 2026-10-06 (đã sửa lần hai), áp dụng từ M4:**
- Đã làm đột biến (tay hoặc cargo-mutants; mọi đột biến sống sót phải có test mới hoặc được chứng minh tương đương): M1 lexer, M2 (gt-ast, gt-diag, gt-parser, gt-fmt), M3 (gt-resolve).
- **KHÔNG làm đột biến TOÀN BỘ** gt-types, gt-hir, gt-mir, gt-dataflow (M4, M5).
- **NGOẠI LỆ có mục tiêu:** đột biến cho 4 kiểm tra an toàn trong gt-types — D1 (không ép kiểu ngầm), D4 (cấm `==` trên số thực), D5 (bất biến mặc định), phép ép kiểu `!` — khoảng 40 đột biến, làm ở CUỐI M4; mọi đột biến sống sót phải có test mới.
- **GIỮ đột biến đầy đủ** cho M6 (borrow checker), M7 (solver/biên), M8 (đa luồng/hiệu ứng) — nơi false negative là lỗ hổng an toàn.
- Phần không đột biến PHẢI bù bằng: TDD (test viết trước), corpus fail/pass đủ ≥3+≥3 mỗi mã, test thuộc tính/ngẫu nhiên khi có thể, và test "mọi vị trí con của AST đều được duyệt" (bài học M3: đột biến phát hiện 4 lỗi thật, trong đó 1 lỗi fail-open).
### 3.4 Minh bạch, không tự ý quyết định lớn
- Mọi quyết định thiết kế quan trọng ghi thành ADR ngắn trong `docs/DECISIONS.md` (bối cảnh – quyết định – hệ quả).
- Không tự ý đổi luật ngôn ngữ (mục 4 trong `GRIT_PLAN.md`) hoặc hệ mã lỗi mà không có ADR và chưa được chủ dự án duyệt.
- Kết thúc mỗi milestone: chạy hết test + clippy + fmt, cập nhật `docs/PROGRESS.md` — **trung thực**, kể cả khi chưa đạt. Báo cáo "làm được gì / còn thiếu gì / rủi ro mới phát hiện".

### 3.5 Rust code (crate Rust của chính gritc)
- `#![forbid(unsafe_code)]` ở mọi crate, trừ ranh giới FFI gọi LLVM (phải có ADR giải thích).
- `cargo clippy -- -D warnings`, `cargo fmt`, `cargo deny`, `cargo audit` sạch trước khi coi một milestone là xong.
- Output của `gritc` phải **deterministic**: cùng input → cùng output byte-for-byte (so băm để kiểm).

## 4. Cách tiếp cận một milestone (quy trình chuẩn)

1. Đọc kỹ mục tương ứng trong `GRIT_PLAN.md` (mô tả + "Hoàn thành khi").
2. Nếu milestone liên quan tới luật ngôn ngữ (D1–D29) — đọc đúng luật đó trước, đừng đoán.
3. Viết test trước khi viết logic chính khi có thể (đặc biệt với các pass kiểm tra an toàn — xem mục 3.3).
4. Code từng phần nhỏ, chạy test thường xuyên, không viết một khối lớn rồi mới test.
5. Khi milestone "xong" theo tiêu chí: chạy toàn bộ test suite + lint, cập nhật `docs/PROGRESS.md`, rồi **dừng lại báo cáo** — không tự ý nhảy sang milestone kế tiếp nếu đây là lần đầu hoàn thành (chờ xác nhận), trừ khi được dặn "cứ làm liên tục tới hết Phase 1".

## 5. Việc không được làm ở Phase 1 (xem mục 6.3 của GRIT_PLAN.md)

Không triển khai: closure, `dyn`/trait object, macro, async, GAT/specialization, inline asm, FFI cho người dùng, `unsafe` do người dùng viết thật sự chạy được (chỉ có intrinsic `trusted` trong `lib/core`), SMT solver, kernel chứng minh, registry gói, tự biên dịch. Những thứ này thuộc Phase 2 (mục 8 của `GRIT_PLAN.md`). Nếu thấy cần dùng tạm một thứ trong danh sách này để code tiếp — dừng lại, hỏi, đừng tự thêm vào.

## 6. Khi gặp bế tắc

Nếu một yêu cầu không rõ, một luật mâu thuẫn với luật khác, hoặc một thiết kế trong `GRIT_PLAN.md` không khả thi khi thực sự code — **không tự chế ra giải pháp rồi code tiếp như thể nó đã được duyệt**. Ghi câu hỏi cụ thể vào `docs/OPEN_QUESTIONS.md`, giải thích ngắn gọn vì sao bế tắc, đề xuất 1–2 phương án nếu có, rồi dừng chờ trả lời.

## 7. Tư duy tổng quát khi code dự án này

Đây là một compiler an toàn — lỗi trong chính compiler nguy hiểm hơn lỗi thường, vì nó tạo **cảm giác an toàn giả**: người dùng tin "gritc đã kiểm rồi" trong khi thực ra chưa. Khi phân vân giữa "code nhanh cho xong milestone" và "code chậm nhưng chắc chắn đúng với chính nó" — luôn chọn vế sau. Chủ dự án đã nói rõ: làm dở dang hay thất trách ở bản đầu, bug ngầm ở các bản sau là lỗi của người code, không phải lỗi của độ khó dự án.
