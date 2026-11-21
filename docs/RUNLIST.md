# RUNLIST — danh sách chạy liên tục (lệnh Giang 2026-10-17 "CHẾ ĐỘ CHẠY LIÊN TỤC")

**Cách dùng (phiên mới):** đọc `docs/HANDOFF.md` rồi tệp này; làm mục CHƯA/ĐANG đầu tiên. Sau MỖI mục: cập nhật trạng thái + commit + dòng "Trạng thái/Việc tiếp theo" trong HANDOFF + báo cáo ngắn nối vào `docs/RUN_REPORTS.md`.
**Quy tắc cứng:** không dừng báo giữa chừng; dừng chỉ khi (1) cổng đỏ sau 2 lần sửa, (2) cần cú pháp/từ khoá mới hoặc đổi ngữ nghĩa ngoài ủy quyền, (3) preflight báo kho hỏng/ổ thiếu/WinError 1455 lặp 3 lần, (4) lỗ hổng điều kiện cứng #12/#9 không sửa được ngay. TRUNG THỰC hơn HOÀN TẤT: không XONG thì ghi KHÔNG XONG kèm lý do. Việc nặng theo LÔ NGẮN đồng bộ (≤ 15 phút; công cụ chỉ cho tối đa 10 phút một lệnh ⇒ dùng `--max-seconds` ≤ 480), `bash tools/preflight.sh D:/Grit` trước mỗi lô, không có tiến trình nền ngoại lệ DUY NHẤT là **cổng liền mạch** (giai đoạn `test` > 10 phút; chạy tách bằng `target/tmp/run_gate.sh` qua WMI; nếu phiên chết, cổng KHÔNG được coi là xanh ⇒ chạy lại TỪ ĐẦU).
**Cổng liền mạch ở:** sau R1, R3, R6, R8, R10 và cuối. Sau mỗi cổng xanh: `bash tools/backup_bundle.sh D:/Grit`.
**Môi trường:** không hai cargo cùng lúc, `CARGO_BUILD_JOBS=2`, `--test-threads=2`.

## Gói 2
| Giai đoạn | Trạng thái | Bằng chứng / việc mở |
|---|---|---|
| G2.0 | XONG | Không đụng `main`; bundle `grit-9359db8.bundle` và sau gate `grit-ed529c6.bundle` verify ở `.backup`; hai test G1.6 ghi trong PROGRESS; fallback `gritrt_start` fail-closed, runtime 8/8; authorship report có; gate đúng HEAD `ed529c6` GREEN 1017/0/202. |
| G2.1 | ĐANG LÀM | Probe 20k, GT_FULL8 và sáu bộ codegen xanh; mismatch 0. Mutation 7 killed + 1 survivor gt-term có lập luận candidate tương đương và test generic mới; E04xx/A.1 scan còn CHƯA RÀ 95. |
| G2.2 | ĐANG LÀM | H10 tests và sổ 13 nghi vấn có; authorship cập nhật tại HEAD 9e9b54c; prompt Claude đã tạo ở `docs/E_PROMPT_CLAUDE.md`; R11b CHỜ Claude/model khác họ, năm giới hạn còn mở. |
| G2.3 | XONG | Grit dev/release-o2 và Rust O0/-O đo trên cùng input; 2 run và scaling raw được ghi ở `docs/SORTLINES_BENCHMARK_G2.jsonl`; không thấy O(n) lần cấp Arena. |
| G2.4 | XONG | Kiểm kê đủ 11 mục; tất cả CÓ, xem `docs/INVENTORY.md`. |
| G2.5 | XONG CÓ GIỚI HẠN | ADR-C06: K3 234 dòng, một Arena caller; giữ HashMap hiện hành, không cần dừng. 1.500 input HashMap riêng vẫn chưa đạt. |
| G2.6 | XONG | Inventory đủ 11/11; doc-test sau cập nhật 207 khối/0 sai; sửa glossary stale và tạo prompt Claude/ADR cần thiết. |
| G2.7 | CHỜ PHIÊN SẠCH (GIANG MỞ) | `docs/CAPSTONE_PROMPTS.md` + `tools/capstone_check.sh` sẵn sàng; self-check fib check/run xanh, trả 55. Không author capstone trong phiên này. |

### G2 checkpoint (2026-11-12)
- G2.1.a: probe_diff 20.000 seed/5 lô; GT_FULL8 bốn test; sáu suite codegen dev/release-o2 có log tại `docs/*CODEGEN*RESULTS.jsonl`, 0 mismatch (xem RUN_REPORTS để biết số chương trình và nhóm đỏ kỳ vọng).
- G2.1.d: 8 mutation records; 7 killed, một survivor `gt-term/src/lib.rs:361` có phân tích tương đương có phạm vi + regression generic, đột biến đối ứng ở dòng 685 bị bắt. `fail_open_scan.py check` còn CHƯA RÀ 95 (sổ tổng quát; chưa đồng nhất với hoàn tất rà E04xx/A.1). G2.1 chưa kết luận cho tới khi rà đúng phạm vi theo lệnh gốc.
- Tiếp tục xác nhận phạm vi E04xx/A.1; không dùng JSONL lịch sử làm bằng chứng mới.
- G2.2 H10: hai test Windows mới xanh; sổ 13 nghi vấn, authorship tại 9e9b54c (503/64.768 TCB, 0,78%), prompt `docs/E_PROMPT_CLAUDE.md` sẵn sàng. R11b CHỜ Claude; Codex-authored TCB chưa được tính review độc lập.
- G2.5: K3 capstone có 234 dòng, một Arena caller; ADR-C06 giữ HashMap địa chỉ mở hiện hành. Điều kiện 1.500 seed HashMap riêng còn thiếu.

## Bảng R1–R12 HỢP NHẤT (2026-11-17; thay hai bảng cũ của dòng Windows và dòng Linux)
Trạng thái theo BẰNG CHỨNG trên mã hiện tại, không theo lời tuyên bố. "bằng chứng ghi" = đã chạy ở thời điểm ghi, CHƯA chạy lại ở đầu hiện tại (cổng liền mạch P1.e sẽ chạy lại tất cả test liệt kê). Dòng Windows = nhánh `windows-work`; dòng Linux = `claude/gallant-darwin-m251cq` (đã hợp nhất ở `c3ba7de`).

| ID | Việc | Trạng thái | Bằng chứng (test / tệp / commit) |
|---|---|---|---|
| R1 | Toàn vẹn kho: preflight, quét NUL, sao lưu | XONG (trừ bundle ở `D:\Grit_backup`) | `tools/preflight.sh`, `tools/nul_check.sh`+`nul_scan.py`, `tools/test_preflight.py`, `test_nul_check.py`; cổng G1.6: NUL 0/1925; commit 824ec22. Bundle: `D:\Grit_backup` bị ACL chặn ⇒ dùng `D:\Grit\.backup` (G2.0.b) |
| R2 | Hàm-làm-giá-trị: M6/M7 qua giá trị, `⊳/⊲`, hiệu ứng, bảng ĐO | XONG | Windows: `gt-borrowck --test fn_value_regions`, `gt-oblig --test oracle`, `gt-driver --test contract_via_value`, `--test fn_value_gen`; Linux: progen `fn_value_ref` 764/734 (`2f8fc92`), FB-C5 `fdce836`, FB-C6 `f000546`; `docs/INTRINSIC_AS_VALUE.md`; commit b80dffc |
| R3 | FAIL_OPEN_AUDIT rà hết | XONG (sổ cái `check` CHƯA RÀ 0; gate failopen 0 mới) | Windows: sổ 1292/1292 (`36ac79d`…`b97338d`, FB-17/18/19). Dòng Linux: 471 vị trí + `docs/fail_open_classification.tsv`. Bộ quét hợp nhất hiện báo CHƯA RÀ 95 ⇒ phải rà hết |
| R4 | `GRITRT_ARENA_CHECK` chỉ THÊM, không tắt được ở dev | XONG | `gt-driver --test arena_dynamic` 6/6 (`dev_check_cannot_be_switched_off_by_environment`); Windows `bcec465`, Linux `963da34→53bfb4d` |
| R5 | Flake llvm_sort, std release/release-o2, Ord kiểu ghép, sortlines | XONG | G1.4: `docs/FLAKE_LLVM_SORT.jsonl` (seed 1..100, 0/100 đỏ ⇒ giới hạn trên ≈3%, KHÔNG kết luận hết flake), `docs/STD_TEST_RESULTS.jsonl`, `docs/SORTLINES_BENCHMARK.jsonl`; commit faddecc |
| R6 | Arena chia sẻ `&'a Arena` (ADR-C03) | XONG CÓ ĐIỀU KIỆN (bảng bằng chứng đủ ở RUN_REPORTS G2.1.c; cổng xanh; dòng TCB H1–H6 ở CÓ ĐIỀU KIỆN cho tới khi Giang/đọc mã khác họ xác nhận) | `lib/core/prelude.gt` `Vec<'a,T>{arena:&'a Arena}`; ADR-C03/C07; oracle `arena_vec` + FB-C9 (`d7f5939`); dòng TCB H1–H6 để CÓ ĐIỀU KIỆN tới khi bảng điều kiện R6 đủ |
| R7 | HashMap trong `lib/std` | XONG (ngưỡng riêng: P1.d) | `gt-driver --test std2_lib` (35 seed/hồ sơ), K3 20/20, ADR-C06; ngưỡng ≥1.500 là toàn cục (ghi ở DECISIONS, P1.d) |
| R8 | Hai intrinsic I/O: `Cap<In>::read_all`, `Cap<Fs>::write` | XONG | `gt-driver --test io_intrinsics` (6/6 sau sửa Windows G1.6), `fs_read_io_error`; CAPSTONE G5/G6/G7; hợp đồng ở `docs/TCB_INTRINSICS.md` |
| R9 | `tools/fuzz_loop.sh` + fuzz tích luỹ | ĐANG TÍCH LUỸ | `tools/fuzz_total.py` (2026-11-17): lexer 1,17 h / 24, parser 2,00 h / 24, 0 phiên lỗi; `tools/fuzz_loop.sh <giờ>` MỘT lệnh qua đêm (P3.a XONG, thử 2+ phiên 1 phút: lex/parse ok; tổng theo băm nguồn); cần ≈ 48 giờ chạy để đạt 24 h/loại |
| R10 | Đột biến nguy hiểm của M7 | XONG CÓ ĐIỀU KIỆN | `docs/M7_DANGER_MUTANTS.md`: 191 đột biến, 60 sống, 30 giết bằng `gt-oblig --test danger_decisions`, 30 chỉ có lập luận tương đương (KHÔNG chứng minh). Số 69 cũ không ánh xạ 1-1 |
| R11 | Đọc mã đối kháng nhóm CAO H1–H10, Guard, gt-rt | ĐANG | `docs/TCB_CODE_AUDIT.md` (sổ 13 nghi vấn, H10 tests); subagent cùng họ model ở S7 (`docs/archive/adversarial/`); GPT-6 đọc phần Claude. Chưa có model KHÁC HỌ độc lập cho phần Codex-viết |
| R11b | Claude đọc diff TCB do Codex viết (`docs/E_PROMPT_CLAUDE.md`) | XONG 2026-11-17 (Claude Sonnet 5.5, HEAD f9f0f78) | `docs/TCB_CODE_AUDIT_CLAUDE.md`: 17 mục, 1 xác nhận (FB-C31), 9 bác bỏ trong phạm vi, 7 chưa kiểm; chưa nâng dòng TCB nào |
| R12 | Cổng cuối + READINESS + bundle + push + báo cáo | ĐANG | cổng GREEN 1017/0/202 (`c5aa45b`, `ed529c6`, 2026-11-12); push `windows-work` OK; cổng mới sau P1/P3: P1.e; READINESS: P4 |
| CAPSTONE WINDOWS | K1–K4 bằng phiên sạch trên Windows | CHỜ PHIÊN SẠCH (Giang mở) | `docs/CAPSTONE_PROMPTS.md`, `tools/capstone_check.sh`; Linux 8/8 (`capstone/verify.py`) |

## Lệnh P0–P4 (2026-11-17)
| Giai đoạn | Trạng thái |
|---|---|
| P0 đồng bộ + tài liệu | XONG (2026-11-17) |
| P1 G2.1, Arena chia sẻ, HashMap, cổng, bundle, thẻ | XONG (cổng XANH `13e2df9` 1023/0/204; bundle `.backup`; thẻ cục bộ; push THẤT BẠI "Repository not found") |
| P2 R11b, capstone | R11b XONG; CAPSTONE WINDOWS CHỜ PHIÊN SẠCH (Giang mở) |
| P3 fuzz_loop, LSP, tái lập, BufOut | a, b (mức tối thiểu+, VS Code chưa thử), c XONG; d KHÔNG làm |
| P4 READINESS + chữ "Windows chưa kiểm" + HANDOFF | XONG (2026-11-18) |

## GÓI 3 (2026-11-18; ủy quyền theo `docs/GIANG_DECISIONS_2026-11-18.md`)
| Mục | Trạng thái | Ghi chú |
|---|---|---|
| S0 sao lưu + chẩn đoán remote | XONG | bundle `D:\Grit\.backup` + `C:\Users\PC\GritBackup` (verify OK); `origin` cũ "Repository not found"; đã đẩy `windows-work` lên `gn3305970-lang/Grit` (riêng tư) bằng URL trực tiếp; thẻ KHÔNG đẩy |
| S1 nhãn quyết định A1–A5 | XONG | bằng chứng trích dẫn có đủ trong kho; nhãn sửa ở 5 tệp; `GIANG_DECISIONS_2026-11-18.md` |
| S2.a bảy nghi vấn R11b | XONG | ARENA-01, VEC-01, argv bác bỏ trong phạm vi; GUARD-02, ATTEST-01, UNSAFE-01 xác nhận (giới hạn/tài liệu); ATOMIC-01 KHÔNG KIỂM ĐƯỢC; FB-C mới: không |
| S2.b `spawn_failed` + stack + stress | XONG | hợp đồng, test đỏ→xanh, đầu-cuối dev + release-o2; stack GIỮ 256 MiB (đo); `tools/stress_spawn.sh` thủ công (tới K=100) |
| S2.c preflight tự từ chối | XONG | `tools/require_preflight.sh` + 5 test |
| S2.d điểm lưu cổng theo băm cây | XONG | `GATE_TREE`, `GATE_STAGE_TREE`, 4 test |
| S2.e khoá `--checks` mặc định release* | XONG | `release_profiles_default_to_all_checks` (mới) + `release_profiles_refuse_proven_mode`; lật mặc định trong bản sao tạm ⇒ 3 test ĐỎ |
| S3 đọc chéo | XONG | `CROSS_REVIEW_COVERAGE.md` (≈ 3,8 % dòng TCB đọc chéo), `E_PROMPT_GPT.md`; **R11c CHỜ PHIÊN GPT (Giang mở)** |
| S4.a–e | XONG | CAPSTONE_PROMPTS trung lập + LOG_TEMPLATE; PR bản thảo; GIANG_DOC6_READING; TSAN_FEASIBILITY; fuzz số thật (HANDOFF) |
| S4.f HashMap chuỗi thao tác ngẫu nhiên | KHÔNG làm (tuỳ chọn) | |
| S5 rc3 + cổng + sao lưu | XONG: `1.0.0-rc3`; cổng XANH `89438fd` (1027/0/206); bundle D: + C:; đã đẩy `windows-work` + thẻ `seed-1.0-rc3` lên kho mới |

## GÓI 4 (2026-11-19; Giang CHỈ dùng Claude; người thực thi Claude Sonnet 5.5)
| Mục | Trạng thái | Ghi chú |
|---|---|---|
| S0 remote + CI | ĐANG | `origin` = kho mới (`old-origin` giữ kho cũ); workflow viết lại; xem `docs/CI_REPORT.md` |
| S1 cách nói + đọc chéo | XONG (tài liệu) | `CROSS_REVIEW_COVERAGE`, `TCB_REVIEW_GUIDE`, `TCB_CODE_AUDIT*`, `READINESS`, `SAFETY_CLAIMS`; ADR-C11 |
| **R11c** | **CHỜ PHIÊN OPUS (Giang mở)** | `docs/E_PROMPT_OPUS.md` (CÙNG HỌ, ĐỘC LẬP YẾU); `E_PROMPT_GPT.md` ghi KHÔNG DÙNG |
| CAPSTONE | CHỜ PHIÊN SẠCH (Giang mở) | hai phiên Claude: Haiku 5.5 và Sonnet 5.5 |

| S0 (kết quả) | XONG | `origin` mới; CI Linux ✓ (74 phút) / Windows rút gọn ✓ (26 phút) / deny ✓ / determinism ✓; 3 vòng sửa; không lỗi thật của Grit; xem `docs/CI_REPORT.md` |
| S2.a stack | XONG | 10 chương trình × dev/release-o2 đạt ở 4 MiB; giữ 256 MiB: `docs/STACK_PROBE.md` |
| S2.b phiên bản tài liệu | XONG | `tools/version_docs.py` + 5 test (lệch ⇒ đỏ) trong giai đoạn expect |
| S2.c HashMap 100 chuỗi ngẫu nhiên | XONG | 100 chuỗi × {dev, release-o2}: 0 bất đồng (băm phân tán, 50–700 khoá, tăng dung lượng nhiều lần) |
| S3 TSan | XONG (mức nêu) | đối chứng dương BẮT (pthread 3/3; `gritrt_spawn` 2/3), âm không báo; 8 mẫu × 50: không phát hiện; `docs/TSAN_RESULTS.md`; job CI `tsan` |
| S4 độ phủ nhánh | CHƯA CHẠY | `tools/coverage.sh` + `docs/DECISION_COVERAGE.md` |
| S5 fuzz | CHUẨN BỊ XONG (Giang chạy) | đóng băng lexer/parser; `fuzz_loop.sh --parallel` (kiểm RAM) + test; lệnh ở HANDOFF |
| S6 phiên bản/cổng | GIỮ rc3 | không đổi mã sản phẩm (`crates/`, `runtime/` chỉ thêm test/công cụ); chạy giai đoạn công cụ + tài liệu của cổng |

## GÓI 5 — XONG trừ phần ghi CHƯA (2026-11-19)
S0 xong (main=e450c69). S1 xong (TSan đối chứng qua Grit). S2 xong (PLATFORM_TESTS, test_deny; Linux diff `--list` nằm trong CI, CHƯA có kết quả run). S3 chạy 2 lần; nhóm D CHƯA thí nghiệm. S4 xong. S5: giữ rc3.

## GÓI 6 (2026-11-19) — ĐANG CHẠY cổng CI trên rc4
S0 xong (main `17beed4`, sau khi Giang tự hợp nhất hai PR trên GitHub). S1 độ phủ: 104→100→90 nhánh chưa chạy, bảng từng nhánh. S2 đo cỡ hàm + LIMITS_AUDIT + sửa thông báo E0702 (rc4). S3 ba app (csv2json/todo/difflite) khớp tham chiếu. S4 SEED_FINISH_LINE. S5: cổng Linux CI chạy trên commit này.

## GÓI 7 (2026-11-19) — XONG trừ phần CHẶN/CHƯA (xem docs/RUN_REPORTS.md)
S0 xong (kho CÔNG KHAI, quét bí mật: không thấy bí mật thật; SECURITY.md). S1 **CHẶN** (Actions vẫn khoá). S2.a FB-C32 sửa (+ FB-C33 do phiên kiểm toán tìm, sửa). S2.b thí nghiệm biên 32 768 khối và ngân sách theo loại nghĩa vụ. S2.c nhóm D một phần, độ phủ CHƯA đo. S3 AI_PITFALLS. S4 workflow fuzz + fuzz_ingest viết xong, CHƯA chạy thử. S5 rc5: cổng đầy đủ CHƯA xanh liền mạch (preflight đỏ 3 lần: bộ nhớ ảo); không thẻ, không đẩy main.

## GÓI 13 (2026-11-21/22) — XONG MỘT PHẦN (xem docs/RUN_REPORTS.md "GÓI 13")
- [x] A1–A3. [x] B1 (21 sống, mục tiêu ở mức ĐO). [x] B2 (0 FB, bộ sinh +64). [ ] **B3 TSan CHƯA: thiếu clang (Giang cài).** [~] B4: gt-oblig XONG (20); nhóm D còn ≈ 21/23 chưa thí nghiệm.
- [x] C cổng đầy đủ rc8 cả hai nền, cùng `GATE_TREE=c7bdef26…`. [ ] D1, D2 CHƯA; [x] D3. [ ] thêm `gt-fmt` vào `GATE_SPLIT_PKGS` (công cụ).

## GÓI 12 (2026-11-20/21) — XONG MỘT PHẦN (xem docs/RUN_REPORTS.md "GÓI 12")
- [x] P0 nguồn gốc lần đẩy (`docs/REMOTE_STATE.md`; hai chỗ lệch ghi nhận; cây `main` khác cây đã qua cổng một dòng `license`).
- [x] P1 cổng Linux rc7 GREEN (WSL2 cục bộ; `GATE_TREE` khớp; 1051/0/215).
- [~] P2 fuzz WSL chạy thử XONG (2×5 phút/mục tiêu, băm khớp, đã nhập log, lệnh qua đêm ở HANDOFF); **TSan CHẶN: thiếu `clang` (việc của Giang)**.
- [x] P3 FB-C36 (hằng `□`) phát hiện, test ĐỎ trước, sửa, rc8, cổng Windows rc8 GREEN (1071/0/217); static init cycle ghim; đột biến `const_cycles` (12; còn 2 sống: 1 tương đương, 1 chưa có test). [ ] 25 ca cũ không chạy lại.
- [x] P4 gate chia `gt-driver` theo mục tiêu (dài nhất 581 s ⇒ không ngoại lệ).
- [~] P5 đo: release/dev (sha256 chậm hơn, chưa giải thích), memcpy Vec = 0 byte (sortlines), enum 4 capstone; `perf` không có.
- [~] P6 coverage chạy MỘT lần (5/6 crate; **gt-oblig hết 14 phút**); nhóm D: 1 nhánh thí nghiệm mới (`pat.rs:222` false); phần còn lại CHƯA LÀM (`docs/COVERAGE_GOI12.md`).
- [ ] Cổng Linux trên rc8; cổng trên cây `Vecter-Core/main`; TSan.

## GÓI 11 (2026-11-20) — XONG MỘT PHẦN (xem docs/RUN_REPORTS.md "GÓI 11")
- [x] S0 nền tái lập (ghim rustc, setup_cloud --check, pyfind, bench_profile, số test).
- [x] S1: FB-C35 phát hiện (kiểm kê MIR), test ĐỎ → sửa → bộ sinh 160 → rc7 → cổng đầy đủ Windows rc7 GREEN. [~] S1.c/g đột biến chạy lại một phần.
- [~] S2: cỡ enum, bench_profile; S2.a/b/e/f CHƯA.
- [ ] **S3 (fuzz WSL, TSan) CHƯA LÀM — đang dở: chưa bắt đầu.** [ ] **S4 (nhóm D, coverage) CHƯA LÀM.** Giao GÓI 12.
- [ ] Cổng Linux trên rc7 CHƯA.

## GÓI 10 (2026-11-20) — bản sao ngoài máy; cổng Linux WSL BỎ QUA; đo Giai đoạn 2 CHƯA ĐO (preflight đỏ); quyết định kiến trúc sư (chỉ tài liệu/công cụ/test)
- [x] S0 `tools/make_export.sh`; bản sao `D:\Grit_export` (bundle 3 977 668 B, tar cây `seed-1.0-rc6` 11 141 120 B, SHA256SUMS, RESTORE.md, REFS.txt); kiểm khôi phục thật XONG; `docs/GITHUB_SUSPENSION.md`; mục KHẨN 0 ở PENDING_GIANG.
- [x] S1.a `tools/gate_clean.sh` + `tools/test_gate_clean.py` (3 test OK, thêm vào cổng); quy tắc ở AGENTS.md/HANDOFF. [ ] S1.b–e BỎ QUA: `wsl -l -v` không có bản Ubuntu (chỉ `docker-desktop`, `OpenClawGateway` không phải của agent); bước cho Giang ở PENDING_GIANG. Cổng Linux CHƯA.
- [~] S2 đo: mục 1, 5, 8, 6 ĐÃ ĐO (build nguội 1 m 39,8 s; sortlines dev 2,83 / release-o2 1,25 / Rust -O 0,97 s; không-LLVM XANH, 433/433 pass); mục 2, 3, 4, 7, 9 CHƯA ĐO. `docs/PHASE2_MEASUREMENTS.md`.
- [~] S3: (a) FB-C34 tắt bản sửa ⇒ ĐỎ (4/20 ca chấp nhận); (b) đột biến gt-term 110: 80 bị bắt, 3 vô hiệu, 27 sống sót (110 tự động, 0 tay); (c) oracle spawn CHƯA LÀM. [x] S3.d: Cap affine (E0401, ĐO bằng gritc rc6), ADR-013 đối chiếu ĐÚNG, FACTCHECK #24 SAI; tổng 31/6/2/5.
- [x] S4 a–f: ADR-C12 (b) cho tài liệu + test khoá; ADR-C13 hoãn; N8; N9 hoãn; GIANG_DECISIONS; `docs/NEXT_PRODUCT_BATCH.md` (4 mục chờ).
- [x] S5: không đổi `crates/ runtime/ lib/` kể từ `16c37f0` (`git diff --name-only 16c37f0..HEAD -- crates runtime lib` rỗng); không đổi nhãn (1.0.0-rc6).

## GÓI 9 (2026-11-20) — CHỈ TÀI LIỆU: kế hoạch Giai đoạn 2 (ĐỀ XUẤT)
- [x] Q0 `docs/PHASE2_PROPOSAL_OPUS.md` (sao chép nguyên văn từ tệp chưa theo dõi `docs/DE_XUAT_GIAI_DOAN_2.md`; thân không sửa).
- [x] Q1 `docs/PHASE2_FACTCHECK.md` (44 khẳng định; đo sau cổng ở mục 10).
- [x] Q2 `docs/PHASE2_PLAN.md`; Q3 `docs/PHASE2_ADR_DRAFTS.md` (4 ADR nháp).
- [x] Q4 đồng bộ GRIT_PLAN (1 dòng), PENDING_GIANG, HANDOFF, RUNLIST, RUN_REPORTS, SEED_FINISH_LINE.
- [ ] Việc của Giang: duyệt 5 điểm ở `docs/PENDING_GIANG.md`; Giai đoạn 2 vẫn CẤM mã khi chưa có lệnh.

## GÓI 8 (2026-11-20) — XONG trừ phần CHƯA (xem docs/RUN_REPORTS.md "CẬP NHẬT KẾT QUẢ CUỐI")
Cổng đầy đủ Windows rc6 GREEN trên `16c37f0` (`GATE_TREE=b828b981…`, 1 045 test, 213 bộ). FB-C34 sửa; ICE 33 hợp đồng; E1001 oan; kit hộp đen; audit/fixture/reports_index. S2 (ADR-C12) chưa áp dụng; S3 (ADR-C13) KHÔNG XONG; S1d chưa làm; cổng Linux/CI chưa; không đẩy main.
