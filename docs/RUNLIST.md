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
