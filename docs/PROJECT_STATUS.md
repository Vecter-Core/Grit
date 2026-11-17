cparser 2,00 h, typechecker chưa có) · #6 Giang đọc `KNOWN_LIMITATIONS` + `PROGRESS` · #7 ADR đóng băng Obligation IR _(xem Phần 5 mục C — có thể đã đạt nhưng bảng chưa cập nhật)_ · #8 LSP đầy đủ + schema JSON đóng băng _(cũng cần rà lại)_ |

**Nhãn "Seed 1.0"** chỉ được đặt khi đủ **sáu mục A1–A6** (`docs/archive/SEED_PLAN.md`): dùng được · an toàn · cổng xanh liền mạch · tái lập & di động · tài liệu kiểm được · đóng băng. Hiện đã có bản **`1.0.0-rc2`** (kèm danh sách mục thiếu). Không được tuyên bố "Phase 1 xong".

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
