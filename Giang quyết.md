# Giang quyết — các điểm cần chủ dự án quyết, kèm lựa chọn của kiến trúc sư

> Viết ngày 2026-11-18 bởi Claude Opus 5.5 (vai kiến trúc sư), cho **Giang**. Nhánh `integration`, HEAD `869e819`.
> Phạm vi: chỉ đọc **tài liệu chữ**, không đọc mã, không sửa mã, không commit. Nguồn: `docs/PENDING_GIANG.md`, `docs/PROJECT_STATUS.md` §5, `docs/OPEN_QUESTIONS.md`, `docs/READINESS.md`, `docs/DECISIONS.md` (ADR-018/022/025/036/037/038/C09), `docs/TCB_REVIEW_GUIDE.md`, `docs/TCB_CODE_AUDIT.md`, `docs/TCB_CODE_AUDIT_CLAUDE.md`, `docs/AUTHORSHIP.md`, `docs/KNOWN_LIMITATIONS_DIGEST.md`, `GRIT_PLAN.md` §7.
> Mỗi mục có: **bối cảnh**, **các lựa chọn**, **lựa chọn của tôi** và lý do, rồi một dòng để bạn ký. Chỗ nào tôi chưa kiểm bằng lệnh thật thì ghi **CHƯA KIỂM**.

---

## 0. Đọc trong 1 phút

Có **17 điểm**. Chia làm ba loại:

- **Loại A, chỉ là ghi sổ (5 điểm):** bạn **đã duyệt từ trước**, nhưng tài liệu vẫn ghi "chờ". Bạn chỉ cần ký "đúng, sửa nhãn". Mất khoảng 5 phút.
- **Loại B, quyết định thật (9 điểm):** mỗi điểm đổi hướng dự án hoặc mức rủi ro bạn chấp nhận.
- **Loại C, việc chỉ bạn làm được (3 điểm):** đẩy repo, thử VS Code, mở phiên sạch.

**Hai phát hiện mới mà báo cáo của Sonnet 5.5 chưa nêu:**

1. **Đọc chéo khác họ model có lẽ đã đạt cho phần mã intrinsic/runtime/Guard.** Theo `docs/AUTHORSHIP.md`, Claude viết **99,2 %** số dòng TCB, còn Codex chỉ viết 0,78 % (503 dòng).
   - G1.5 do GPT-6 đọc, tức khác họ với Claude, nên phủ gần hết số dòng đó.
   - R11b do Claude đọc, phủ phần 503 dòng của Codex.
   - Tài liệu hiện ghi "G1.5 không đạt" vì chỉ so người đọc với Codex. Xem mục **B3**.
2. **Có hai cổng khác nhau đang bị gộp làm một:**
   - Nhãn **`1.0.0-seed`**: 6 mục A1–A6, định nghĩa ở `docs/archive/SEED_PLAN.md`.
   - **Chuyển sang Giai đoạn 2**: 12 điều kiện của `GRIT_PLAN.md` §7.
   - M13 đầy đủ (inlay hint) chỉ bắt buộc cho **cổng thứ hai**. Xem mục **B8**.

**Thứ tự tôi đề xuất** (chi tiết ở Phần 4):

1. Hôm nay: đẩy repo và có bản sao ở ổ khác (**C1**). Hiện **toàn bộ 23 commit mới chỉ nằm trên một ổ đĩa**.
2. Ký 5 mục loại A.
3. Tuần này: B1, B2, B4, B5, B6, C2, C3; chạy fuzz.
4. Sau khi CI xanh: hợp nhất vào `main` (B7).

---

## Loại A — chỉ ghi sổ (bạn đã duyệt, tài liệu còn ghi "chờ")

| # | Điểm | Bằng chứng đã duyệt | Đề xuất |
|---|---|---|---|
| A1 | **Q23**: match số nguyên bằng khoảng; literal ngoài phạm vi là E0601; E0307 thuộc M5 | `docs/PROGRESS.md:171`: "M4 HOÀN THÀNH (Giang duyệt Q23 và 4 điểm sai khác, 2026-10-06)" | sửa nhãn ở `OPEN_QUESTIONS.md:25` thành ĐÃ DUYỆT 2026-10-06 |
| A2 | **Q34** (ADR-022) | tiêu đề ADR-022: "Giang DUYỆT 2026-10-06, Q34". Dòng Q34 tự mâu thuẫn: vừa "ĐÃ DUYỆT" vừa "[CHỜ GIANG XÁC NHẬN]" | xoá đuôi "[CHỜ GIANG XÁC NHẬN]" |
| A3 | **Q35–Q42** (bộ giải M7, ADR-025) | `DECISIONS.md` ADR-025 §12: "Duyệt của Giang (2026-10-07) … mục này THẮNG" | sửa nhãn `OPEN_QUESTIONS.md:40` thành ĐÃ DUYỆT 2026-10-07 (kèm §12) |
| A4 | **Q79–Q82** (`f32`, `%` số thực, `instcombine`, O2) | cả bốn đã ghi "ĐÃ DUYỆT 2026-10-14" trong `OPEN_QUESTIONS.md`. `READINESS.md:56` và `BENCHMARK.md:47` vẫn ghi "chờ Giang" | sửa hai chỗ đó |
| A5 | **ADR-038** (thư viện: Arena chia sẻ, HashMap, I/O…) | `HANDOFF.md:93`: "ADR-038 duyệt theo từng bước (Q83–Q86, Q85 hoãn)". Tiêu đề ADR vẫn ghi "CHỜ GIANG DUYỆT, CHƯA CÓ DÒNG MÃ NÀO" (sai: mã đã có) | thêm dòng trạng thái đầu ADR-038. Tương tự Q52 ở `KNOWN_LIMITATIONS.md:189` ("duyệt cho `spawn` thuần") |

**Lựa chọn của tôi:** ký cả năm. Không có rủi ro, vì mã đã cài và có test trong cổng xanh `13e2df9`. Đây chỉ là dọn nhãn, để danh sách "đang chờ" chỉ còn việc thật.

**Ký:** ☐ Đồng ý cả A1–A5 ☐ Khác: ______

---

## Loại B — quyết định thật

### B1. Điều kiện #9: 3 đột biến M6 (361/591/554) sống sót, chấp nhận bằng lập luận cộng đo đạc
- **Bối cảnh:** đo trên 24 143 chương trình, số chương trình "đột biến chấp nhận mà bản gốc từ chối" = **0**. Nghĩa là ba đột biến này chỉ **chặn nhiều hơn**, không bao giờ cho qua thêm. Kiến trúc sư đã chấp nhận có điều kiện (2026-10-15), và điều kiện mở lại đã được ghi.
- **Lựa chọn:**
  - (a) Chấp nhận có điều kiện, chốt lần cuối.
  - (b) Đòi chứng minh hình thức: tốn nhiều tuần, cần công cụ Phase 2.
  - (c) Đòi viết test giết từng đột biến.
- **Tôi chọn (a).** Hướng sai của các đột biến này là "từ chối oan", không phải "cho qua sai". Vậy rủi ro thật là hiệu năng/tiện dụng, không phải an toàn. Giữ hai điều kiện mở lại đã ghi:
  - thêm luật ĐỎ mới của M6 ⇒ cập nhật A.1 và chạy lại `probe_diff`;
  - chạy `probe_diff` với `GT_PROBE_N=4000` trước mỗi lần đóng milestone.
- **Ký:** ☐ (a) ☐ (b) ☐ (c)

### B2. Điều kiện #10: 30 đột biến M7 "xoá thêm check" chỉ có lập luận tương đương
- **Bối cảnh:** các đột biến này nằm ở hàm **quyết định xoá check**.
  - Theo ADR-029 và `DECISIONS.md:729/781/1095`, mặc định **mọi hồ sơ** (kể cả release) là `--checks=all`: mọi `Assert` được giữ, bộ giải không xoá gì.
  - Vậy trong chế độ mặc định, 30 đột biến này **không làm mất check nào**. Rủi ro chỉ có khi người dùng tự gõ `--checks=proven` (đang ghi THỬ NGHIỆM).
- **Lựa chọn:**
  - (a) Chấp nhận có điều kiện, kèm ràng buộc cứng "`--checks=proven` giữ THỬ NGHIỆM cho tới khi 30 đột biến bị giết hoặc có chứng minh".
  - (b) Bắt giết hết 30 trước nhãn Seed.
- **Tôi chọn (a).** Thêm một việc nhỏ cho agent: một test khoá "release/release-o2 mặc định là `--checks=all`". **CHƯA KIỂM** test đó đã tồn tại chưa; `DECISIONS.md:1095` nói "Có test khẳng định" cho báo cáo check còn lại, nhưng tôi chưa đọc mã.
- **Ký:** ☐ (a) ☐ (b)

### B3. TCB: coi yêu cầu "đọc mã khác họ model" là ĐẠT CÓ ĐIỀU KIỆN?
- **Bối cảnh:**
  - Tài liệu đang nói "chưa có model khác họ đọc TCB", vì G1.5 dùng GPT-6 đọc, cùng họ với Codex.
  - Nhưng `AUTHORSHIP.md` cho thấy Claude viết 64 265/64 768 dòng TCB, Codex chỉ viết 503 dòng.
  - Vậy G1.5 (GPT-6 đọc mã Claude viết) **là đọc chéo khác họ** cho phần lớn mã. R11b (Claude Sonnet 5.5 đọc 503 dòng Codex) phủ phần còn lại.
- **Ba lỗ hổng của lập luận này (nói thẳng):**
  1. `AUTHORSHIP.md` đo ở HEAD cũ `9e9b54c`. Mã sửa **sau** G1.5 (vd FB-C31 do Claude viết) chưa có model khác họ đọc. **CHƯA KIỂM** con số tại HEAD hiện tại.
  2. G1.5 đọc H1–H10, IR Guard và `gt-rt`. Nó **không** đọc các checker (M6 borrowck, M7 bộ giải, M8), mà các checker cũng thuộc TCB (`READINESS.md` mục d).
  3. Người đọc đều là model, chưa có con người. 7/13 nghi vấn của G2.2 còn "chưa kiểm": ARENA-01, VEC-01, ATOMIC-01, GUARD-02, ATTEST-01, UNSAFE-01 và argv end-to-end.
- **Lựa chọn:**
  - (a) Giữ "chưa đạt" như hiện tại.
  - (b) Ghi "đọc chéo khác họ: ĐẠT CÓ ĐIỀU KIỆN cho intrinsic/runtime/Guard", sau khi agent đo lại `AUTHORSHIP` tại HEAD và cho một phiên GPT/Codex đọc các diff TCB **sau** G1.5.
  - (c) Đòi người thật đọc.
- **Tôi chọn (b).** Đây là cách đúng sự thật nhất: tài liệu hiện đánh giá **thấp hơn** thực tế. Nhưng **không** được nâng thành "TCB đã kiểm toán": các checker M6/M7 vẫn chỉ có oracle, đột biến và corpus, chưa có lượt đọc mã khác họ. Điều này ảnh hưởng thẳng tới **A2 (an toàn)**, mục duy nhất đang "chưa" của nhãn Seed.
- **Ký:** ☐ (a) ☐ (b) ☐ (c)

### B4. H7 `Atomic` (và H8 `Mutex`/`spawn`): không có ThreadSanitizer/loom
- **Bối cảnh:** không có công cụ nào bắt đua dữ liệu lúc chạy. Hiện chỉ có kiểu `Send`/`Sync`, hằng thứ tự bộ nhớ (E0503), corpus, và chạy lặp 300 lần.
- **Lựa chọn:**
  - (a) Chấp nhận là rủi ro có ý thức cho Seed, ghi `SAFETY_CLAIMS`, xem lại ở Phase 2 (8.D).
  - (b) Chặn nhãn Seed tới khi có TSan.
  - (c) Như (a), cộng một job CI **Linux** dựng vài chương trình đa luồng Grit với pass ThreadSanitizer của LLVM. TSan chạy tốt trên Linux; trên Windows thì không.
- **Tôi chọn (c)**, nhưng job CI làm **sau** khi repo đẩy được (C1). Lời hứa với người dùng phải nói rõ: "đa luồng an toàn theo kiểu; chưa có kiểm động đua dữ liệu".
- **Ký:** ☐ (a) ☐ (b) ☐ (c)

### B5. ADR-C09: hạ ngưỡng bằng chứng `HashMap` từ "≥1500 chương trình" xuống "≥30 chương trình mỗi hàm API + va chạm băm + K3 20/20"
- **Bối cảnh:** ngưỡng này do **chính agent** (lệnh "kiến trúc sư" P1.d) quyết ngày 2026-11-17. Đây là **nới điều kiện**. Tiền lệ: ADR-037 nới fuzz "theo lệnh Giang". Vì vậy ADR-C09 cần chữ ký của bạn mới đúng luật dự án.
- **Lập luận của tôi:**
  - `HashMap` viết **bằng Grit** trong `lib/std`, không phải intrinsic. Compiler kiểm nó như mọi chương trình.
  - Lỗi trong HashMap vì thế gây **sai kết quả**, không gây lỗi bộ nhớ hay UB. An toàn bộ nhớ nằm ở intrinsic `Vec` (H4/H5), đã có bằng chứng riêng.
  - Con số 1500 vốn là ngưỡng **toàn cục** dev≡release (Q77), bị chép nhầm sang.
- **Lựa chọn:**
  - (a) Duyệt ADR-C09 như hiện có.
  - (b) Duyệt và thêm một bộ "chuỗi thao tác ngẫu nhiên" so với mô hình Rust (rẻ, vài trăm dòng test).
  - (c) Giữ 1500.
- **Tôi chọn (a).** (b) đáng làm nếu HashMap sau này dùng cho việc quan trọng, nhưng không chặn Seed.
- **Ký:** ☐ (a) ☐ (b) ☐ (c)

### B6. Điều kiện #6: bạn "xem" `KNOWN_LIMITATIONS.md` (≈100 KB) và `PROGRESS.md` (≈166 KB)
- **Bối cảnh:** đọc hết khoảng 266 KB là không thực tế, và cũng không phải mục đích của điều kiện. Mục đích là chủ dự án **biết rủi ro nào đang được chấp nhận**.
- **Lựa chọn:**
  - (a) Đọc hết.
  - (b) Đọc theo danh sách rút gọn dưới đây (khoảng 60–90 phút), rồi ghi một ADR "điều kiện #6 được hiểu là đọc các mục rủi ro sau…". Đây là bạn tự định nghĩa điều kiện của chính mình, nên hợp lệ, nhưng phải ghi rõ là định nghĩa lại.
  - (c) Bỏ điều kiện.
- **Tôi chọn (b).** Danh sách đọc theo số dòng của `KNOWN_LIMITATIONS.md`, theo mục lục ở `KNOWN_LIMITATIONS_DIGEST.md`:

  | Dòng | Mục |
  |---|---|
  | 93 | lỗ hổng M4/M5 |
  | 127 | đột biến M6 |
  | 135, 195 | mutability bên trong |
  | 152 | oracle M7 mù với gì |
  | 161 | đột biến M7 |
  | 207 | M8 có điều kiện |
  | 274 | âm tính giả tìm muộn |
  | 310 | Arena check chỉ ở dev |
  | 327 | fail-open |
  | 333 | sự cố ghi NUL chưa rõ nguyên nhân |
  | 340–357 | nền tảng và Seed 1.0 |

  `PROGRESS.md`: chỉ đọc dòng tiêu đề và trạng thái của từng mốc. Cuối cùng đọc `docs/SAFETY_CLAIMS.md` (lời hứa với người dùng) và `docs/SEED_LIMITATIONS.md`.
- **Ký:** ☐ (a) ☐ (b) ☐ (c)

### B7. Hợp nhất `integration` vào `main`: khi nào, bằng cách nào
- **Bối cảnh:** `main` ở `cc23c24`, chậm khoảng 130 commit. Cổng xanh ở `13e2df9`; sau đó chỉ có commit tài liệu.
- **Lựa chọn:**
  - (a) Hợp nhất ngay.
  - (b) Hợp nhất khi đủ ba điều kiện: (1) repo đẩy được (C1); (2) CI GitHub chạy xanh **một lần** trên Linux và Windows (hiện chưa chạy lần nào); (3) loại A đã ký.
  - (c) Chờ tới nhãn `1.0.0-seed`.
- **Tôi chọn (b)**, làm qua PR, giữ lịch sử, **không squash** (FIELD_BUGS và RUN_REPORTS trỏ tới hash commit). Lý do không chọn (c): `main` cũ quá lâu thì người khác clone sẽ nhận bản sai. Lý do không chọn (a): chưa có CI nào xác nhận ngoài máy này.
- **Ký:** ☐ (a) ☐ (b) ☐ (c)

### B8. M13 (inlay hint) và hai cổng: nhãn Seed hay Giai đoạn 2
- **Bối cảnh:**
  - Sonnet đề xuất "dừng ở mức hiện tại".
  - Nhưng §7 điều kiện #8 **bắt buộc** "M13 hoàn thành" mới được sang Giai đoạn 2. Dừng hẳn thì Giai đoạn 2 không bao giờ mở được, trừ khi sửa #8.
  - Nhãn `1.0.0-seed` (A1–A6) **không** đòi M13 đầy đủ; A6 chỉ để trống phần "LSP".
- **Lựa chọn:**
  - (a) Dừng M13 vĩnh viễn và sửa #8 bằng ADR.
  - (b) Làm theo thứ tự: nhãn Seed trước (không cần M13); sau đó làm M13 tối thiểu theo kế hoạch (inlay hint kiểu và hiệu ứng; trạng thái nghĩa vụ để sau) **chỉ khi** bạn đã thử VS Code (C2) và thấy LSP có ích.
  - (c) Làm M13 ngay.
- **Tôi chọn (b).** Không sửa #8 bây giờ. Cứ để nó là việc thật của cổng Giai đoạn 2.
- **Ký:** ☐ (a) ☐ (b) ☐ (c)

### B9. Tên phiên bản: hai bản mã khác nhau cùng tên `rc2`
- **Bối cảnh:** thẻ cục bộ `seed-1.0-rc2` gắn trên `13e2df9`, nhưng `READINESS.md` ghi "mã đã đổi sau rc2 gốc, KHÔNG đổi số phiên bản để khỏi lệch băm chứng thực/golden IR". Hai bản mã khác nhau mang cùng nhãn sẽ làm hỏng chính lời hứa tái lập (A4): người khác không biết "rc2" là bản nào.
- **Lựa chọn:**
  - (a) Giữ `rc2`.
  - (b) Đổi lên `1.0.0-rc3` trong **một commit riêng** (cập nhật chuỗi phiên bản, bless lại golden chứng thực, chạy cổng), rồi gắn thẻ `seed-1.0-rc3`. Thẻ cục bộ `seed-1.0-rc2` đổi tên trước khi đẩy.
  - (c) Không gắn thẻ tới khi có nhãn Seed.
- **Tôi chọn (b).** Làm cùng lần chạy cổng kế tiếp, khỏi tốn thêm một cổng 2 giờ. **Chú ý:** đừng đẩy thẻ `seed-1.0-rc2` hiện tại lên remote khi C1 sửa xong.
- **Ký:** ☐ (a) ☐ (b) ☐ (c)

---

## Loại C — việc chỉ bạn làm được

### C1. Remote báo "Repository not found", và mọi thứ đang nằm trên một ổ đĩa (KHẨN)
- **Hiện trạng:**
  - 23 commit (từ `19204c2` tới `869e819`) **chỉ có** ở `D:\Grit`.
  - Bundle `D:\Grit\.backup\grit-13e2df9.bundle` cũng nằm trên ổ D:. Ổ hỏng là mất cả hai.
  - `D:\Grit_backup` cũng ở ổ D:, nên cấp quyền ghi cho nó (PENDING #2) **không** giải quyết rủi ro này.
- **Việc bạn làm, theo thứ tự, khoảng 15 phút:**
  1. Chép `D:\Grit\.backup\grit-13e2df9.bundle` sang ổ khác: USB, ổ C:, hoặc OneDrive/Google Drive.
  2. Mở trình duyệt, xem `github.com/Vecter-Core/Grit` còn tồn tại không (đổi tên, chuyển owner, chuyển private?).
  3. Nếu repo vẫn còn: chạy `gh auth login` lại, hoặc tạo token mới có quyền repo. Lỗi "not found" của GitHub thường là **thiếu quyền**, không phải repo mất thật.
  4. Báo URL đúng cho agent. Agent sẽ đẩy `windows-work`, nhưng **chưa** đẩy thẻ rc2 (xem B9).
- **Lựa chọn của tôi về PENDING #2 (`D:\Grit_backup`):** không cần cấp quyền nữa. Thay bằng "bundle cộng một bản ở ổ hoặc máy khác".

### C2. Thử tiện ích VS Code (10 phút)
Làm 5 bước ở `docs/QUICKSTART_VSCODE.md`. Kết quả quyết định B8. Chỉ cần báo lại một câu: "chạy / không chạy / lỗi gì".

### C3. Mở phiên sạch viết capstone K1–K4 trên Windows (điều kiện A1)
- **Lựa chọn của tôi:** mở **hai** phiên sạch.
  - Phiên 1 là Claude, chỉ đưa `docs/AI_GUIDE.md`, `docs/STD_REFERENCE.md` và đề ở `docs/CAPSTONE_PROMPTS.md`.
  - Phiên 2 (tuỳ chọn, giá trị cao) là một model **khác họ**, ví dụ GPT.
- **Lý do:** Grit được làm "cho AI viết". Nếu chỉ Claude viết được thì đó là bằng chứng yếu, vì tài liệu do Claude viết cho chính Claude đọc.
- Chấm bằng `tools/capstone_check.sh`.

---

## Các điểm nhỏ (agent tự làm được sau khi bạn gật đầu một lần)

- **Fuzz 24 giờ (điều kiện #2):** hiện lexer 1,2 h, parser 2,0 h. Có hai cách:
  - (a) Chạy `tools/fuzz_loop.sh 10` qua đêm trên máy bạn, khoảng 3–5 đêm.
  - (b) **Tôi chọn:** khi repo đẩy được, chạy trên máy Linux đám mây. Trên Linux libFuzzer không bị giới hạn phiên ASan như Windows, nên chạy liên tục được, cả hai mục tiêu song song, xong trong khoảng một ngày.
  - Dù chọn cách nào, phải chạy trên **băm nguồn hiện tại** (công cụ đã kiểm).
- **Thất bại khi tạo quá nhiều luồng** (3000 luồng × 256 MiB làm cạn tài nguyên máy, R11b): đề xuất `spawn` thất bại ⇒ abort có tên rõ (`spawn_failed`), tức là fail-closed. Đây là đổi hợp đồng runtime (H8), nên theo `AGENTS.md` phải ghi hợp đồng TCB trước, code sau. **Tôi chọn:** làm, mức ưu tiên thấp.
- **Kho `audit/`, cắt `DECISIONS.md`/`PROGRESS.md`:** **tôi chọn không đụng.** Các tệp đó dài nhưng được nhiều nơi trỏ tới; cắt sẽ làm gãy liên kết bằng chứng.
- **Giai đoạn 2:** không bắt đầu trước nhãn Seed. Khi bắt đầu, giữ thứ tự của kế hoạch: 8.A (SMT) rồi 8.B (kernel kiểm chứng). Hai hướng này giảm check lúc chạy **và** thu nhỏ TCB, đúng điểm yếu A2 hiện tại.

---

## Phần 4. Kế hoạch tốt nhất (sau khi bạn ký)

| Bước | Ai | Việc | Mở khoá |
|---|---|---|---|
| 1 (hôm nay) | Giang | C1: chép bundle sang ổ khác, sửa quyền remote | chống mất dữ liệu; CI; B7 |
| 2 (hôm nay) | Giang | ký A1–A5, B1, B2, B5 | dọn danh sách chờ |
| 3 | agent | sửa nhãn loại A; ghi ADR cho B1/B2/B5/B6; đổi rc3 (B9); **một** cổng liền mạch | tài liệu khớp sự thật |
| 4 | agent | đẩy `windows-work`, bật CI GitHub (Linux + Windows) | B7 |
| 5 | agent + máy | fuzz 24 h (Linux đám mây hoặc qua đêm) | điều kiện #2 |
| 6 | Giang | C2 (VS Code), C3 (phiên sạch capstone), đọc danh sách B6 | A1, #6, quyết B8 |
| 7 | agent | B3: đo lại AUTHORSHIP tại HEAD, phiên GPT đọc diff TCB sau G1.5; B4: job TSan Linux | A2 |
| 8 | Giang | khi CI xanh: PR `integration` vào `main` (B7) | `main` đúng |
| 9 | Giang | xét nhãn `1.0.0-seed` khi A1–A6 đều đạt hoặc đạt có điều kiện được ký | Seed 1.0 |

**Điều có thể chặn nhãn Seed lâu nhất:** A2 (an toàn). Nó phụ thuộc B3 (đọc chéo), fuzz 24 h và B4 (TSan). Các việc khác đều nhỏ.

---

## Phần 5. CHƯA KIỂM (trong chính tài liệu này)

- Tôi **không đọc mã** và không chạy lệnh. Mọi khẳng định ở trên lấy từ tài liệu, có ghi số dòng.
- Con số 99,2 % của `AUTHORSHIP.md` đo ở HEAD cũ `9e9b54c`, chưa đo lại ở `869e819`.
- Chưa xác minh có test khoá "release mặc định `--checks=all`" (B2).
- Chưa xác minh repo `Vecter-Core/Grit` còn tồn tại hay không (C1).

---

## Phiếu ký nhanh (đánh dấu rồi gửi lại cho agent)

| Mục | Lựa chọn của tôi | Ký |
|---|---|---|
| A1–A5 | sửa nhãn | ☐ |
| B1 | (a) chấp nhận có điều kiện | ☐ |
| B2 | (a), kèm `--checks=proven` giữ thử nghiệm | ☐ |
| B3 | (b) đạt có điều kiện, sau khi đo lại | ☐ |
| B4 | (c) chấp nhận, kèm TSan Linux sau | ☐ |
| B5 | (a) duyệt ADR-C09 | ☐ |
| B6 | (b) đọc theo danh sách, ghi ADR | ☐ |
| B7 | (b) hợp nhất sau CI xanh, qua PR | ☐ |
| B8 | (b) Seed trước, M13 sau khi thử VS Code | ☐ |
| B9 | (b) đổi lên rc3 | ☐ |
| C1 | chép bundle và sửa remote (bạn làm) | ☐ |
| C2 | thử VS Code (bạn làm) | ☐ |
| C3 | hai phiên sạch Claude + GPT (bạn mở) | ☐ |
