# Tiến trình dự án Grit — đọc một lượt là hiểu

> Viết cho **Giang** (chủ dự án). Ngày lập: **2026-11-17** (cập nhật 2026-11-18), nhánh `integration`.
> Nguồn: đọc lại toàn bộ tài liệu của kho (HANDOFF, RUNLIST, RUN_REPORTS, READINESS, GRIT_PLAN, CAPSTONE, SEED_*, TCB_*, OPEN_QUESTIONS, FIELD_BUGS…).
> Số liệu là số **đã ghi trong tài liệu của dự án** (ngày ghi kèm theo) trừ khi nói rõ là đo lại; chỗ tài liệu tự mâu thuẫn được nêu rõ.

> **CẬP NHẬT 2026-11-19 (GÓI 8):** nhãn `1.0.0-rc6` (FB-C34 spawn qua generic, ICE 33 hợp đồng, E1001 oan tuple lồng); cổng đầy đủ Windows GREEN trên `7fa8647` (rc5 + FB-C34), cổng trên rc6 xem RUN_REPORTS; Actions vẫn khoá.
> **CẬP NHẬT 2026-11-19 (GÓI 7):** nhãn `1.0.0-rc5`; kho GitHub nay CÔNG KHAI (không thấy bí mật thật); sửa FB-C32 (vòng không dừng khi hết 24 biến bóng) và FB-C33 (đệ quy qua `spawn`); cổng đầy đủ rc5 CHƯA xanh liền mạch; Actions vẫn khoá.
> **CẬP NHẬT 2026-11-19 (GÓI 6):** nhãn `1.0.0-rc4` (E0702 nêu nguyên nhân khi hàm quá lớn); cổng đầy đủ Linux GREEN trên rc4; 90 nhánh quyết định chưa chạy (bảng từng nhánh); `docs/LIMITS_AUDIT.md`; ba app mẫu `apps/`; `docs/SEED_FINISH_LINE.md` liệt kê đích tới Seed. **GitHub Actions đang bị khoá** (Giang kiểm Billing).

> **CẬP NHẬT 2026-11-19 (GÓI 5):** `main` = `e450c69` (fast-forward 1 lần); TSan có đối chứng QUA pipeline Grit (tỉ lệ phát hiện yếu nhất 13/30; 0 báo cáo ở 400 lần chạy); độ phủ nhánh các hàm quyết định đã đo (730 nhánh, 100 chưa chạy; `docs/DECISION_COVERAGE.md`); `docs/START_CODING.md` để bắt đầu viết Grit thật. Vẫn rc3; A2 vẫn chưa.

> **CẬP NHẬT 2026-11-19 (GÓI 4, Giang chỉ dùng Claude):** `origin` đổi sang kho mới; **CI GitHub chạy XANH** (Linux đầy đủ + Windows rút gọn: `docs/CI_REPORT.md`); TSan: không phát hiện trong 8 mẫu × 50 lần (`docs/TSAN_RESULTS.md`); stack nhỏ nhất ≤ 4 MiB; đọc chéo khác họ chỉ ở `gt-rt` + `gt-guard`; vẫn `1.0.0-rc3`; A2 vẫn chưa.
> **CẬP NHẬT 2026-11-18 (GÓI 3, sau ủy quyền "Giang quyết"):** nhãn **`1.0.0-rc3`**, cổng XANH `89438fd` (1027/0/206); đã đẩy `windows-work` + thẻ `seed-1.0-rc3` lên `gn3305970-lang/Grit` (riêng tư), bundle ở `D:\Grit\.backup` và `C:\Users\PC\GritBackup`; chỉ ≈ 3,8 % dòng TCB được đọc chéo (`docs/CROSS_REVIEW_COVERAGE.md`); việc chờ Giang: `docs/PENDING_GIANG.md`, quyết định đã ủy quyền: `docs/GIANG_DECISIONS_2026-11-18.md`.
> **CẬP NHẬT 2026-11-18 (sau lệnh P0–P4)** — nội dung bên dưới là ảnh chụp 2026-11-17. Thay đổi chính: cổng liền mạch **XANH 1023/0/204** trên `13e2df9` (bundle `.backup`, thẻ cục bộ `seed-1.0-rc2`); **R11b XONG** (`docs/TCB_CODE_AUDIT_CLAUDE.md`: 17 mục, 1 xác nhận FB-C31, 9 bác bỏ, 7 chưa kiểm); G2.1 **XONG** (đột biến gt-term tương đương CÓ ĐO; rà fail-open CHƯA RÀ = 0; bằng chứng Arena chia sẻ đủ — `RUN_REPORTS` G2.1.c); LSP có hover/definition/symbol (CHƯA thử VS Code thật, không inlay hint ⇒ M13 chưa đủ); `tools/fuzz_loop.sh <giờ>` qua đêm; READINESS #7 ⇒ đạt, thêm bảng Seed A1–A6 (A3 đạt, A2 chưa ⇒ vẫn `1.0.0-rc2`); các chữ "Windows chưa kiểm" đã sửa. **Còn CHỜ GIANG:** `docs/PENDING_GIANG.md` (12 mục; quan trọng nhất #11: push remote báo "Repository not found").

---

## 0. Tóm tắt 30 giây

- **Grit** = ngôn ngữ lập trình "an toàn theo thiết kế", viết cho **AI** viết code. Trình biên dịch tên `gritc` (viết bằng Rust). Nếu `gritc` **không chứng minh được** chương trình an toàn thì **từ chối** (báo ĐỎ) và **không tạo ra chương trình chạy**.
- **Giai đoạn 1 ("Seed")** có **15 mốc** (M0 → M13, kèm M4.5). **Có mã cho cả 15**: 9 mốc "đạt", 4 mốc "đạt có điều kiện" (M6, M7, M8, M10), M12 (thư viện + chương trình thật) phần lớn có nhưng cổng Seed 1.0 chưa đạt, M13 (LSP) mới ở mức tối thiểu+. Chi tiết Phần 3.
- **Giai đoạn 2 ("Ascend")** có **9 hướng** (8.A → 8.I): **chưa bắt đầu** (cố ý, theo kế hoạch).
- Bản hiện tại là **`1.0.0-rc9`** (GÓI 14: rc8 → rc9 vì thông dịch đọc `□`/`■` và gt-term ICE tường minh cho nhánh phòng thủ; GÓI 12: rc7 → rc8 vì FB-C36, vòng tham chiếu qua hằng `□`; GÓI 11: rc6 → rc7 vì FB-C35, đệ quy qua hợp đồng; GÓI 8: rc5 → rc6 vì FB-C34; GÓI 7: rc4 → rc5 vì FB-C32, vòng không dừng bị chấp nhận khi hết biến bóng; trước đó rc4) (ứng viên phát hành; GÓI 6: rc3 → rc4 do thông báo E0702; tài liệu cũ ghi rc2 nhưng mã báo rc1 — đã sửa ở rc3), **chưa** gắn nhãn `1.0.0-seed`: còn thiếu điều kiện (Phần 3–5).
- **Cổng kiểm thử liền mạch gần nhất (Windows): XANH — 1027 test đạt / 0 lỗi / 206 bộ** (2026-11-18, commit `89438fd` = `1.0.0-rc3`; thẻ cục bộ và trên kho GitHub riêng tư `seed-1.0-rc3`).
- **Việc cần BẠN** (Phần 5 và `docs/PENDING_GIANG.md`): các quyết định/duyệt, hai phiên cần bạn mở (phiên sạch viết capstone; phiên model khác họ đọc lại TCB), vài chỗ tài liệu lỗi thời.

---

## 1. Dự án là gì

**Ý tưởng:** các ngôn ngữ cũ để con người viết; Grit để **AI viết** và **máy kiểm chặt** trước khi chạy. Ký hiệu toán/logic ngắn (mỗi ký hiệu có bí danh ASCII), hệ thống kiểu và kiểm tra rất nghiêm.

**Cách hoạt động (đường ống của `gritc`):**
```
mã .gt → đọc ký tự → đọc cú pháp → giải tên → kiểm kiểu → dạng trung gian (MIR)
       → kiểm mượn/sở hữu → kiểm hiệu ứng + đa luồng → kiểm tràn số/chia 0/chỉ số mảng
       → kiểm dừng (vòng lặp/đệ quy có kết thúc không) → quét bí mật (secret)
       → PHÁN QUYẾT:  ĐỎ (lỗi E…) hoặc lỗi nội bộ ⇒ DỪNG, không tạo chương trình
                      VÀNG (cảnh báo S…) ⇒ vẫn chạy + in cảnh báo
       → sinh mã LLVM → IR Guard (bộ kiểm độc lập kiểm lại IR) → chạy
```

**Nguyên tắc sống còn — "fail-closed" (đóng khi không chắc):** "không biết" = "chưa chứng minh" = **không cho qua**. Lỗi giả-an-toàn (compiler nói an toàn mà thực ra không — *false negative*) là lỗi nghiêm trọng nhất; mỗi lỗi tìm ra phải có test ĐỎ trước khi sửa, ghi vào `docs/FIELD_BUGS.md` và vào bộ test vĩnh viễn.

**Quy mô:** khoảng **90 000 dòng Rust**; **22 crate** + `runtime/gt-rt`; **~980 chương trình mẫu `.gt`**; **72 mã lỗi** (mỗi mã có tài liệu + ≥3 mẫu sai + ≥3 mẫu đúng); **42 ADR** + ADR-C (dòng Linux); nhật ký lỗi thật `FIELD_BUGS.md` (FB-1…FB-19, FB-C1…FB-C31).

**Ai làm gì:** Giang = chủ dự án. Claude = kiến trúc sư (và viết phần lớn mã). Codex/GPT-6 = thực thi gói lệnh trên Windows. Hai dòng làm việc đã hợp nhất (Windows và Linux đám mây) ở commit `c3ba7de` trên nhánh `integration`.

---

## 2. Dự án có bao nhiêu phần

### Phần A — Giai đoạn 1 "Seed" (đã có mã): **15 mốc**
M0 · M1 · M2 · M3 · M4 · **M4.5** · M5 · M6 · M7 · M8 · M9 · M10 · M11 · M12 · M13

### Phần B — Giai đoạn 2 "Ascend" (CHƯA làm): **9 hướng**
8.A bộ giải SMT · 8.B kernel kiểm chứng bằng chứng · 8.C `unsafe` có bằng chứng · 8.D đa luồng nâng cao · 8.E giao thức AI · 8.F Rig đầy đủ (kho gói) · 8.G tối ưu/hiệu năng · 8.H tự biên dịch · 8.I thực thi tất định/cận tài nguyên

### Phần C — Sản phẩm đi kèm
`gritc` (check/run/build/fmt/explain/doctor/lsp) · `rig` (quản lý gói) · `lib/core` + `lib/std` (thư viện viết bằng Grit: Vec, Text, HashMap, sắp xếp, I/O) · 6 công cụ dogfood (`examples/`) · 4 capstone (`capstone/`: csvstat, calc, kvstore, grep-lite) · bộ tài liệu cho người/AI dùng Grit · hệ thống kiểm chứng (corpus, oracle động, đột biến, so ba nguồn, fuzz).

---

## 3. Đã tới đâu — từng mốc của Giai đoạn 1

| Mốc | Làm gì | Trạng thái | Ghi chú / điều kiện còn lại |
|---|---|---|---|
| M0 | Đặc tả, ký hiệu, ngữ pháp | **đạt** | đã đóng băng (`spec/FREEZE.md`) |
| M1 | Lexer | **đạt** | fuzz mới ~1,2 giờ (cần 24) |
| M2 | Parser, AST, `fmt` | **đạt** | fuzz parser mới ~2,0 giờ (cần 24) |
| M3 | Giải tên, module | **đạt** | |
| M4 | Hệ thống kiểu | **đạt** | đột biến chỉ cho 4 kiểm tra an toàn chọn lọc |
| M4.5 | Kiểu vùng nhớ (`Arena`) + kiểu quyền (`Cap<…>`) | **đạt** | |
| M5 | MIR + khung dataflow | **đạt** | |
| M6 | Sở hữu & mượn (borrow checker) | **đạt có điều kiện** | 3 đột biến (361/591/554) chấp nhận bằng LẬP LUẬN + đo máy, không phải chứng minh hình thức; Arena chia sẻ (ADR-C03) có bảng bằng chứng ở `RUN_REPORTS` G2.1.c |
| M7 | Biên số học (miền khoảng + zone) | **đạt có điều kiện** | 191 đột biến nhóm nguy hiểm: 60 sống → 30 giết bằng test, 30 chỉ có lập luận tương đương |
| M8 | Dừng, hiệu ứng, đa luồng | **đạt có điều kiện** | giới hạn 24 biến/thân; không TSan/loom |
| M9 | `gritc` + quét secret + phán quyết | **đạt** | quét secret là heuristic (cảnh báo VÀNG) |
| M10 | Sinh mã LLVM + IR Guard + runtime | **đạt có điều kiện** | release/release-o2 chỉ có bằng chứng thực nghiệm; TCB chưa có model khác họ đọc độc lập đủ |
| M11 | Rig | **đạt** | chưa có kho gói |
| M12 | Thư viện + chương trình thật + cổng Seed 1.0 | **phần lớn có; cổng Seed 1.0 CHƯA đạt** | 6 dogfood + 4 capstone; xem bảng Seed A1–A6 ở `READINESS.md` |
| M13 | LSP | **mức tối thiểu+** | chẩn đoán, định dạng, hover, đi tới định nghĩa cấp item, ký hiệu; không inlay hint; VS Code CHƯA thử thật |

**Cổng chuyển giai đoạn `GRIT_PLAN.md` §7 — 12 điều kiện** (bảng đầy đủ `docs/READINESS.md`, cập nhật 2026-11-18):

| Kết quả | Điều kiện |
|---|---|
| **đạt** (4) | #4 mọi mã lỗi có tài liệu+test · #7 Obligation IR đóng băng · #11 E0601/E0307 · #12 mô hình capability |
| **đạt có điều kiện** (5) | #1 test xanh / không còn false-negative đã biết · #3 chương trình thật + benchmark · #5 build tái lập · #9 mutability bên trong · #10 đột biến M7 |
| **chưa** (3) | #2 fuzz ≥24 giờ mỗi cái (lexer 1,2 h, parser 2,0 h) · #6 Giang đọc `KNOWN_LIMITATIONS` + `PROGRESS` · #8 LSP đầy đủ (schema JSON đã khoá) |

**Nhãn "Seed 1.0"** chỉ khi đủ **sáu mục A1–A6** (`docs/archive/SEED_PLAN.md`). Hiện: A3 đạt; A1, A4, A6 có điều kiện; **A2 (an toàn) chưa** ⇒ giữ `1.0.0-rc3`. Không được tuyên bố "Phase 1 xong".

---

## 4. Hiện đang làm gì (tiến độ gần nhất)

Hai dãy "gói việc" đã hoàn thành rất nhiều; ghi chép mới nhất là **2026-11-12**.

**Gói 1 (G1.0–G1.6, trên Windows):**

- G1.1–G1.2 bảo toàn + cổng nền: **XANH 961/0/189**. G1.3 hợp nhất hai dòng làm việc (26 xung đột giải xong), kiểm "đỏ-khi-hoàn-nguyên" cho F-01…F-08: xong. G1.4 (flake, std, sortlines): xong. G1.6 cổng cuối: **GREEN 1017/0/202**, đã push lên `origin/windows-work`.
- **G1.5 (đọc mã đối kháng phần TCB) chưa đạt đúng yêu cầu**: người đọc cũng là GPT-6, cùng họ model với người viết. Tìm và sửa 3 lỗi thật (FB-C28/C29/C30).

**Gói 2 (G2.0–G2.7):**
| | Trạng thái | Ghi chú |
|---|---|---|
| G2.0 bảo toàn + cổng | XONG | cổng GREEN 1017/0/202; bundle sao lưu có |
| **G2.1** kiểm chứng nặng sau hợp nhất | **ĐANG** | đã xong: probe 20 000 seed (0 vi phạm), `GT_FULL8` (cả hai phép nặng), 6 bộ so sánh mã sinh ở dev + release-o2 (0 bất đồng). **Còn mở:** (a) đột biến `gt-term/src/lib.rs:361` sống sót, đã có lập luận tương đương nhưng chưa kết luận; (b) `fail_open_scan.py` còn **95 chỗ chưa rà**, chưa xác nhận đó có đúng phạm vi lệnh gốc không |
| **G2.2** đọc mã TCB | **ĐANG** | sổ 13 nghi vấn, thí nghiệm H10 trên Windows; **chờ phiên Claude** (R11b) |
| G2.3 benchmark `sortlines` | XONG | Grit chậm hơn Rust `-O` khoảng ×3,9 (dev) — xem `BENCHMARK.md` |
| G2.4 kiểm kê tài liệu | XONG | đủ 11/11 mục |
| G2.5 đánh giá HashMap/capstone | XONG CÓ GIỚI HẠN | thiết kế giữ nguyên (ADR-C06); **chưa đạt** ngưỡng ≥1.500 test riêng cho HashMap |
| G2.6 tài liệu thiếu | XONG | doc-test 207 khối / 0 sai |
| **G2.7** phiên sạch viết capstone | **CHỜ BẠN MỞ** | prompt + công cụ kiểm đã sẵn (chưa commit) |

**Danh sách R1–R12** (`docs/RUNLIST.md`): R1–R5 xong · R6 (Arena chia sẻ), R7 (HashMap), R8 (hai intrinsic I/O), R10 (đột biến M7) **đã làm ở dòng Linux** nhưng bảng của dòng Windows vẫn ghi CHƯA · R9 (fuzz) mới tích luỹ 1,2 h/2,0 h trên 24 h — xem Phần 5-C · R11 đang · R11b chờ · R12 đang (còn thiếu bundle, xem 5-A).

**Kho git:** nhánh `integration` = `origin/windows-work` (đã push, **không** có commit nào chưa đẩy). `main` **chưa đụng**, cách `integration` 129 commit. Không có tag nào trong kho này.

**Chưa commit trong cây làm việc** (do phiên trước để lại, tôi **không động vào**): sửa dở `docs/{E_PROMPT_CLAUDE,GLOSSARY,MUTANT_ARGUMENTS,READINESS,RUNLIST,RUN_REPORTS}.md`; tệp mới `docs/CAPSTONE_PROMPTS.md`, `tools/capstone_check.sh`. Thêm vào đó là các thay đổi dọn dẹp của lần này (Phần 7).

---

## 5. CẦN BẠN DUYỆT / QUYẾT / MỞ — gom hết ở đây

### A. Quyết định chỉ bạn làm được

| #   | Việc                                                                                                                                                                                            | Vì sao cần bạn                                                                                                                 | Gợi ý của tôi                                                                          |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| 1   | **Hợp nhất `integration` vào `main`?**                                                                                                                                                          | `main` còn là bản cũ; lệnh hiện hành cấm tự đụng `main`                                                                        | chờ xong G2.1/G2.2 rồi hợp nhất bằng tay, hoặc ra lệnh rõ                              |
| 2   | **Quyền ghi thư mục sao lưu `D:\Grit_backup`**                                                                                                                                                  | `git bundle` bị `Permission denied` (ACL chỉ cho đọc) ⇒ **chưa có bundle sao lưu mới**; 3 bundle cũ còn                        | cấp quyền ghi cho tài khoản chạy; sao lưu hiện chỉ dựa vào nhánh `origin/windows-work` |
| 3   | **Đọc `docs/KNOWN_LIMITATIONS.md` và `docs/PROGRESS.md`** (điều kiện #6)                                                                                                                        | chỉ bạn xác nhận được                                                                                                          | hai tệp rất dài (100 KB / 166 KB); đọc **Phần 6 dưới đây trước**                       |
| 4   | **Xác nhận chính thức các câu hỏi cũ còn ghi "CHỜ GIANG"** trong `OPEN_QUESTIONS.md`: **Q23** (match số nguyên bằng khoảng, literal ngoài phạm vi = E0601) và **Q35–Q42** (thiết kế bộ giải M7) | đã được cài đặt từ lâu nhưng tôi **không thấy ghi nhận bạn xác nhận** (chỉ thấy "ủy quyền kiến trúc sư duyệt TCB và đột biến") | nếu đồng ý, ghi "đã duyệt" để dọn danh sách mở                                         |
| 5   | **Chấp nhận (hoặc bác) điều kiện #9 và #10** (lập luận thay chứng minh cho 3 đột biến M6 và 30 đột biến M7)                                                                                     | là rủi ro an toàn có ý thức                                                                                                    | đã chấp nhận "có điều kiện" theo ủy quyền; bạn nên xác nhận lần cuối                   |
| 6   | **H7 `Atomic`: chấp nhận rủi ro không có ThreadSanitizer/loom**                                                                                                                                 | không có công cụ bắt đua dữ liệu                                                                                               | xác nhận là rủi ro chấp nhận cho Seed                                                  |
| 7   | **G2.5: có đòi ≥1.500 test riêng cho HashMap** (điều kiện ADR-038) không?                                                                                                                       | tốn công, giá trị thấp nếu đã có 35 seed/hồ sơ + 20/20 ca K3                                                                   | gợi ý: hạ ngưỡng hoặc ghi rõ "không đạt" trong READINESS                               |
| 8   | **G2.1: xác nhận phạm vi quét fail-open "E04xx/A.1"** (95 chỗ chưa rà)                                                                                                                          | lệnh gốc mơ hồ về phạm vi                                                                                                      | cho biết đúng phạm vi, hoặc cho phép rà hết 95 chỗ                                     |
| 9   | **Có làm tiếp Giai đoạn 2 / M13 đầy đủ không, và khi nào**                                                                                                                                      | hiện cấm làm khi chưa có lệnh                                                                                                  | sau khi nhãn `1.0.0-seed` đạt                                                          |

### B. Phiên mới do **bạn mở** (agent không tự làm được)

1. **R11b — phiên Claude/model khác họ** đọc diff TCB do Codex viết, theo `docs/E_PROMPT_CLAUDE.md`; kết quả lưu `docs/TCB_CODE_AUDIT_CLAUDE.md`. Cho tới lúc đó: "chưa có người thứ hai khác họ model đọc mã runtime/Guard". Nếu không có model khác họ, phải ghi thẳng điều đó thay vì tuyên bố đã duyệt.
2. **G2.7 — phiên sạch viết capstone K1–K4** chỉ từ `AI_GUIDE` + `STD_REFERENCE` + đề (`docs/CAPSTONE_PROMPTS.md`, `tools/capstone_check.sh`). Mục đích: kiểm "AI mới có viết đúng Grit chỉ từ tài liệu không" trên **Windows** (hiện mới có 8/8 trên Linux).

### C. Chỗ tài liệu đang **mâu thuẫn / lỗi thời** (tôi phát hiện khi rà; chưa sửa vì đa số đang có sửa dở chưa commit)

| #   | Chỗ                                                              | Vấn đề                                                                                                                                                                                                                                                            |
| --- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `docs/GLOSSARY.md` (bản **chưa commit**)                         | ghi "Arena chia sẻ **đã hoãn sang Phase 2**, `Vec` giữ `&!Arena` độc quyền" — **SAI**: ADR-C03 và mã hiện dùng `&'a Arena` chia sẻ; bản đã commit thì đúng. Nên **bỏ bản sửa dở** (`git checkout docs/GLOSSARY.md`) — bạn quyết                                   |
| 2   | `docs/READINESS.md` bảng §7                                      | dòng **#7** (ADR đóng băng Obligation IR) ghi "chưa" nhưng `ADR-039` ghi **đã đóng băng** và đã cài; dòng **#8** chưa tính LSP tối thiểu + schema JSON đã khoá bằng test (`spec/FREEZE.md`). Dòng "Kết luận (đếm lại)" cũng lệch (3/4/5, sau #10 đổi thành 3/5/4) |
| 3   | `docs/RUNLIST.md`                                                | có **hai bảng** R1–R12 (Windows và "Imported Claude branch") ghi trạng thái **khác nhau** cho R6–R10 (vd Windows: R6 CHƯA; Claude: R6 XONG)                                                                                                                       |
| 4   | `docs/RUN_REPORTS.md`                                            | còn nguyên một dòng `FUZZ_PLACEHOLDER` (mục "R9 fuzz tích luỹ" chưa điền); **thiếu mục S1 và S8** mà `CHANGELOG`, `spec/FREEZE.md` dẫn tới                                                                                                                        |
| 5   | `docs/READINESS.md`                                              | **không có** mục "Seed 1.0" mà `FREEZE.md`/`CHANGELOG.md` nói tới                                                                                                                                                                                                 |
| 6   | `docs/SAFETY_CLAIMS.md`, `SEED_LIMITATIONS.md`, `spec/FREEZE.md` | ghi "Windows: **chưa kiểm**" — nhưng cổng Windows đã GREEN (1017/0/202, 2026-11-12)                                                                                                                                                                               |
| 7   | `docs/HANDOFF.md` dòng "Cập nhật tại commit faddecc"             | đã lệch 23 commit (`tools/handoff_check.sh` cảnh báo); cần ghi lại hash sau khi commit                                                                                                                                                                            |
| 8   | `docs/RUNLIST.md` dòng R11                                       | còn nhắc `E_PROMPT.md` (đã chuyển vào `docs/archive/`)                                                                                                                                                                                                            |

### D. Rủi ro an toàn còn mở (nói thẳng, không né)

- **Chưa có người thứ hai là con người**, cũng chưa có model khác họ đọc mã TCB. Đọc mã đối kháng đã có bằng subagent **cùng họ** — hai đợt đó vẫn tìm thêm 13+3 lỗ hổng thật mà corpus/oracle/đột biến trước **không** bắt ⇒ khả năng còn lỗ hổng chưa tìm là có thật.
- Fuzz **≪ 24 giờ** (lexer 1,17 h, parser 2,00 h; typechecker không fuzz byte, thay bằng fuzz chuỗi kiểm).
- Không có oracle động cho **đua dữ liệu/deadlock** (chỉ corpus + chạy lặp 300 lần).
- Flake `llvm_sort`: không tái hiện (0/100 seed ⇒ giới hạn trên ≈ 3% ở tin cậy 95%) — **không được kết luận "đã hết"**.
- `release`/`release-o2`: bằng chứng thực nghiệm (so ba nguồn), không phải chứng minh.
- **Chưa kiểm:** CI Windows trên GitHub; tiện ích VS Code trong VS Code thật; capstone trên Windows; build tái lập cho `release`; `u128` nhân/chia ở backend LLVM (bị từ chối có chủ ý).
- Hiệu năng: `sortlines` 33 MB — Grit dev ≈ 3,0 s vs Rust `-O` ≈ 0,77 s (≈ ×3,9); stdout không đệm làm I/O lớn chậm ≈ ×5.

---

## 6. Nếu chỉ đọc 6 tệp (theo thứ tự)

1. **Tệp này** (`docs/PROJECT_STATUS.md`).
2. `docs/READINESS.md` — "dùng được thật chưa" + bảng 12 điều kiện _(đọc kèm Phần 5-C mục 2: bảng có chỗ lỗi thời)_.
3. `docs/KNOWN_LIMITATIONS.md` — mục lục các giới hạn thật (rất dài; đọc đề mục).
4. `docs/SEED_LIMITATIONS.md` + `docs/SAFETY_CLAIMS.md` — Grit làm được/không làm được gì, nói cho người dùng.
5. `docs/RUNLIST.md` + `docs/RUN_REPORTS.md` (đoạn cuối: G1.x, G2.x) — việc đã làm và số liệu thật.
6. `docs/TCB_CODE_AUDIT.md` — phần "cơ sở tin cậy" (thứ phải tin mà compiler không tự kiểm được).

**Bản đồ tài liệu (sau dọn dẹp):**
| Nhóm | Tệp |
|---|---|
| Kế hoạch gốc & luật | `GRIT_PLAN.md`, `AGENTS.md`, `.claude/skills/grit-contract.md`, `docs/OPEN_QUESTIONS.md`, `docs/DECISIONS.md` (ADR) |
| Trạng thái sống | `docs/HANDOFF.md` (đã rút từ 41 KB còn 22 KB), `docs/RUNLIST.md`, `docs/RUN_REPORTS.md`, `docs/READINESS.md`, `docs/PROGRESS.md` |
| Lỗi & giới hạn | `docs/FIELD_BUGS.md`, `docs/KNOWN_LIMITATIONS.md`, `docs/FAIL_OPEN_AUDIT.md`, `docs/MUTANT_ARGUMENTS.md`, `docs/M7_DANGER_MUTANTS.md`, `docs/INTRINSIC_AS_VALUE.md` |
| Cơ sở tin cậy (TCB) | `docs/TCB_INTRINSICS.md`, `docs/TCB_REVIEW_GUIDE.md`, `docs/TCB_CODE_AUDIT.md`, `docs/AUTHORSHIP.md`, `docs/E_PROMPT_CLAUDE.md` |
| Cho người/AI dùng Grit | `README.md`, `BUILDING.md`, `docs/QUICKSTART*.md`, `docs/GRIT_BOOK.md`, `docs/LANGUAGE_REFERENCE.md`, `docs/STD_REFERENCE.md`, `docs/AI_GUIDE.md`, `docs/GLOSSARY.md` |
| Capstone | `docs/CAPSTONE.md`, `docs/CAPSTONE_LOG.md`, `docs/CAPSTONE_PROMPTS.md`, `docs/G2_CLEAN_SESSION.md` |
| Số liệu đo (JSONL, công cụ ghi) | `docs/*.jsonl`, `docs/fail_open_*` |
| Đặc tả & mã lỗi | `docs/spec/`, `docs/errors/` |
| **Đã xong → lưu trữ** | `docs/archive/` (xem `docs/archive/README.md`), `docs/HANDOFF_ARCHIVE.md` |

---

## 7. Đợt dọn dẹp lần này (2026-11-17) — không xoá mã

**Đã làm** (tất cả **chưa commit**; xem lại bằng `git status`):

1. Tạo `docs/archive/` và `git mv` (giữ lịch sử) các tài liệu của việc đã xong: `MERGE_NOTES.md` (70 KB, hợp nhất đã xong), `SEED_PLAN.md`, `E_PROMPT.md`, thư mục `adversarial/` (82 tệp bằng chứng), và `scratch_edit2.py` (script tạm lỡ vào thư mục gốc). Có `docs/archive/README.md` giải thích. Đã sửa đường dẫn tham chiếu trong `CHANGELOG.md`, `FAIL_OPEN_AUDIT.md`, `QUICKSTART_CLOUD.md`, `spec/FREEZE.md`.
2. `docs/HANDOFF.md`: chuyển **nguyên văn** phần lịch sử tháng 10 (mục 0, 3, 5, 6.1–8, "kiểm toán ngoài") xuống cuối `docs/HANDOFF_ARCHIVE.md`; thêm con trỏ tới tệp này; sửa câu "đang ở đâu" lỗi thời.
3. Xoá tệp rỗng `{}` ở thư mục gốc (đã ghi `docs/DELETIONS.md`).
4. Thêm con trỏ tới tệp này vào `README.md`.

**Kiểm sau dọn dẹp** (chỉ các kiểm không cần dựng lại): `check_expectations` OK (72 mã/245 tham chiếu), `doc_test --list` vẫn **207 khối** (không khối nào nằm trong lưu trữ), `deletions_check` OK (191), `nul_check` OK (1939 tệp). **Chưa chạy** cổng đầy đủ (cần ~40 phút, bạn dặn không code).

**Cố ý chưa dọn** (và lý do):

- `audit/` (274 tệp, 740 KB): bộ kiểm toán ngoài đã chạy xong nhưng có script chạy được + 181 chương trình thử tấn công — là mã/dữ liệu. Nếu bạn muốn, có thể chuyển nguyên thư mục vào `docs/archive/` sau khi sửa đường dẫn trong `audit/run_all.sh`.
- `docs/DECISIONS.md` (333 KB), `PROGRESS.md` (166 KB), `KNOWN_LIMITATIONS.md` (99 KB), `FIELD_BUGS.md`: nhật ký lõi, dài nhưng được nhiều nơi dẫn; cắt ngắn cần quyết định của bạn.
- Các sổ `*.jsonl` (bằng chứng đo đạc), 5 tệp `TASK.md` giữ chỗ nhỏ (<1 KB mỗi cái), `t.sh` (được `preflight.sh` và test dùng), `mutants.out/` (đã bị `.gitignore`).
- 8 chỗ lỗi thời ở Phần 5-C: đa số nằm trong tệp **đang có sửa dở chưa commit** nên tôi không chạm.

---

## 8. Muốn tự kiểm nhanh (khi cần; không bắt buộc)

```
bash tools/preflight.sh D:/Grit                    # kho/máy có khoẻ không (nhanh)
PYTHONIOENCODING=utf-8 python tools/check_expectations.py   # corpus khớp bảng mã lỗi (nhanh)
bash tools/handoff_check.sh                        # HANDOFF có lỗi thời không
bash tools/gate.sh D:/Grit                         # CỔNG ĐẦY ĐỦ ≈ 40 phút; xanh liền mạch mới tính
```

c

## Cập nhật GÓI 10 (2026-11-20)
Nhãn vẫn `1.0.0-rc6`. Mã sản phẩm đóng băng (không đổi `crates/ runtime/ lib/`). Thêm: `tools/make_export.sh`, `tools/gate_clean.sh`; tài liệu `GITHUB_SUSPENSION.md`, `NEXT_PRODUCT_BATCH.md`, `PHASE2_MEASUREMENTS.md`; ADR-C12 (b) cho tài liệu; ADR-C13/N9 hoãn.
Cổng Linux cục bộ (WSL2) GREEN trên `16c37f0`, GATE_TREE khớp Windows (xem `docs/CLOUD_GATE_RESULT.md`); đột biến gt-term: 27/110 sống sót (`docs/MUT_GT_TERM_GOI10.md`).
