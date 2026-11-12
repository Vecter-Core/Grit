# RUNLIST — danh sách chạy liên tục (lệnh Giang 2026-10-17 "CHẾ ĐỘ CHẠY LIÊN TỤC")

**Cách dùng (phiên mới):** đọc `docs/HANDOFF.md` rồi tệp này; làm mục CHƯA/ĐANG đầu tiên. Sau MỖI mục: cập nhật trạng thái + commit + dòng "Trạng thái/Việc tiếp theo" trong HANDOFF + báo cáo ngắn nối vào `docs/RUN_REPORTS.md`.
**Quy tắc cứng:** không dừng báo giữa chừng; dừng chỉ khi (1) cổng đỏ sau 2 lần sửa, (2) cần cú pháp/từ khoá mới hoặc đổi ngữ nghĩa ngoài ủy quyền, (3) preflight báo kho hỏng/ổ thiếu/WinError 1455 lặp 3 lần, (4) lỗ hổng điều kiện cứng #12/#9 không sửa được ngay. TRUNG THỰC hơn HOÀN TẤT: không XONG thì ghi KHÔNG XONG kèm lý do. Việc nặng theo LÔ NGẮN đồng bộ (≤ 15 phút; công cụ chỉ cho tối đa 10 phút một lệnh ⇒ dùng `--max-seconds` ≤ 480), `bash tools/preflight.sh D:/Grit` trước mỗi lô, không có tiến trình nền ngoại lệ DUY NHẤT là **cổng liền mạch** (giai đoạn `test` > 10 phút; chạy tách bằng `target/tmp/run_gate.sh` qua WMI; nếu phiên chết, cổng KHÔNG được coi là xanh ⇒ chạy lại TỪ ĐẦU).
**Cổng liền mạch ở:** sau R1, R3, R6, R8, R10 và cuối. Sau mỗi cổng xanh: `bash tools/backup_bundle.sh D:/Grit`.
**Môi trường:** không hai cargo cùng lúc, `CARGO_BUILD_JOBS=2`, `--test-threads=2`.

## Gói 2
| Giai đoạn | Trạng thái | Bằng chứng / việc mở |
|---|---|---|
| G2.0 | XONG | Không đụng `main`; bundle `grit-9359db8.bundle` verify ở `.backup`; thay đổi hai test G1.6 ghi trong PROGRESS; fallback `gritrt_start` fail-closed, runtime 8/8; authorship script/report có ở `tools/authorship_report.py` và `docs/AUTHORSHIP.md`. |
| G2.1–G2.7 | CHƯA KIỂM | Chưa bắt đầu; G2.1 trước tiên cần gate xanh trên đầu G2.0 rồi chạy lô nặng tuần tự. |

| ID | Việc | Trạng thái | Lệnh chạy lại / bằng chứng | Commit |
|---|---|---|---|---|
| R1 | Toàn vẹn kho: `tools/preflight.sh`, giai đoạn `nul` của cổng, 4 tệp NUL, quan hệ git, sao lưu bundle, KNOWN_LIMITATIONS | XONG | `python tools/test_preflight.py`, `python tools/test_nul_check.py`, `bash tools/nul_check.sh D:/Grit`, `bash tools/backup_bundle.sh D:/Grit`; cổng sau R1 XANH 943/0/185 + `NUL_OK (1399 tệp quét)` | 824ec22 |
| R2 | Hàm-làm-giá-trị: M6/M7 progen/obgen + vùng qua kiểu hàm; ⊳/⊲ qua giá trị hàm; bảng ĐO vs SUY RA + cờ test; hiệu ứng qua kiểu hàm | XONG (cổng kế tiếp ở R3) | `cargo test -p gt-borrowck --test oracle random_programs`, `--test fn_value_regions`; `cargo test -p gt-oblig --test oracle pruned_programs_with_function`; `cargo test -p gt-driver --test contract_via_value`; feature đo `--features gt-types/allow-intrinsic-values` | b80dffc |
| R3 | FAIL_OPEN_AUDIT rà hết phần còn lại (`tools/fail_open_scan.py`, dấu vân tay) | XONG | `python tools/fail_open_scan.py check` (CHƯA RÀ 0; nghi vấn 0); `cargo test -p gt-types --test copy_impl_bounds`, `-p gt-mir --test interp_u128`, `-p gt-driver --test llvm_vec`; cổng liền mạch XANH 961/0/189 | 36ac79d, 2fa3b1b, 93aa345, 932a99f, cc3ba41, b97338d |
| R4 | GRITRT_ARENA_CHECK không tắt được ở dev; grit-contract §3.3 hai luật | XONG | `cargo build --release --manifest-path runtime/gt-rt/Cargo.toml` rồi `cargo test -p gt-driver --test arena_dynamic` (6/6; cần LLVM trên PATH) | bcec465 |
| R5 | Chạy nặng theo lô: flake llvm_sort ≥100, std release/release-o2, Ord kiểu ghép, sortlines 5 lần | XONG (2026-11-12, G1.4) | `docs/FLAKE_LLVM_SORT.jsonl`: 100/100 seed 1..100 tuần tự xanh; `std_lib` dev/release/release-o2 0 lỗi, 35 chương trình/mục; 4 kiểu Ord ghép × 35 chương trình × 346 phép; `docs/SORTLINES_BENCHMARK.jsonl`: 5 lần mỗi bản, đầu ra khớp | faddecc |
| R6 | Arena chia sẻ (ADR-038 Q83): phụ lục ADR trước, tấn công, oracle shared_arena, TCB H1–H6 | CHƯA | | |
| R7 | HashMap (Q86 iii) trong gói std | CHƯA | | |
| R8 | Hai intrinsic I/O (Q86 iv): `Cap<In>::read_all`, `Cap<Fs>::write` | CHƯA | | |
| R9 | `tools/fuzz_loop.sh` + tích luỹ ≤ 2 giờ | CHƯA | | |
| R10 | 69 đột biến nguy hiểm của M7 (điều kiện #10) | CHƯA | | |
| R11 | Mục E: đọc mã H1–H10, IR Guard, `gt-rt`; ghi thí nghiệm và giới hạn ở `docs/TCB_CODE_AUDIT.md` | CHƯA ĐẠT, G1.5 | Đọc mã/thí nghiệm và FB-C28/C29/C30 đã ghi; reader GPT-6 cùng họ với Codex, nên điều kiện model khác chưa đạt. Cần review khác họ; H10 không có thí nghiệm riêng |  |
| R12 | Kết thúc: cổng cuối liền mạch, READINESS bảng §7 từng điều kiện, HANDOFF, bundle, push, báo cáo cuối | ĐANG, G1.6 | Gate GREEN (1.017/0/202), READINESS cập nhật, push `windows-work` thành công; bundle bị ACL `D:\Grit_backup` chặn, còn mở tới khi cấp quyền ghi và verify bundle |  |

---

## Imported Claude branch record: RUNLIST — danh sách việc chạy (tạo mới 2026-10-21 trên nhánh `claude/gallant-darwin-m251cq`)

Trước đây tham chiếu "RUNLIST" nằm ở bản Windows chưa lên origin; tệp này dựng lại từ lệnh Seed 1.0 (bản sửa) và HANDOFF §6. Trạng thái: XONG / ĐANG / CHƯA / KHÔNG XONG (kèm phần đã làm). Cập nhật sau mỗi mục.

| Mã | Việc | Trạng thái | Ghi chú |
|---|---|---|---|
| R1 | preflight + quét NUL chạy được Linux và Windows; bundle sao lưu | ĐANG | `tools/preflight.sh`, `tools/nul_scan.py` (+ test), giai đoạn `nul` của `gate.sh`: XONG phần Linux; Windows chưa kiểm (`preflight.sh` có nhánh PowerShell cho RAM); bundle: làm ở mốc rc1 |
| R2 | hàm-làm-giá-trị: M6 qua giá trị, `⊳` qua giá trị, hiệu ứng, bảng INTRINSIC_AS_VALUE | XONG (2026-10-21, Linux) | bảng ĐO trước/sau; FB-C5, FB-C6 đã sửa (đỏ trước); M6 qua giá trị: bộ sinh `fn_value_ref` + 7 mẫu; ADR-C01/C02 |
| R3 | FAIL_OPEN_AUDIT rà hết bằng `tools/fail_open_scan.py` | CHƯA | S7.a |
| R4 | `GRITRT_ARENA_CHECK` chỉ bật thêm, không tắt được ở dev | XONG (2026-10-21, Linux) | test đảo `dev_check_cannot_be_switched_off_by_environment` (đỏ trước `963da34`, xanh `53bfb4d`); arena_dynamic 6/6, gt-rt 6/6 |
| R5 | flake `llvm_sort` ≥ 100 lần; std ở release/release-o2; Ord kiểu ghép; đo sortlines 5 lần | CHƯA | S7.b |
| R6 | Arena chia sẻ `&'a Arena` (ADR-038 / Q83) | XONG (2026-10-21, Linux) | ADR-C03; oracle `arena_vec`; FB-C9 tìm thấy và sửa |
| R7 | HashMap | XONG (std::HashMap trong lib/std bằng Grit; std2_lib 3 hồ sơ) | S2 |
| R8 | READINESS: bảng §7 và sáu mục SEED 1.0 | CHƯA | S8.d |
| R9 | `tools/fuzz_loop.sh` tích luỹ có giới hạn (tối đa 2 giờ tổng; điều kiện #2 ghi "ĐANG TÍCH LUỸ") | CHƯA | S7.d |
| R10 | 69 đột biến "xoá thêm check" của M7 (`analysis.rs` 62, `driver.rs` 7) | CHƯA | S7.c |
| R11 | đọc mã đối kháng nhóm CAO bằng subagent (E_PROMPT.md; trạng thái TCB: "đã qua đọc mã đối kháng bằng subagent cùng họ model") | CHƯA | S7.e |

## Đã XONG trước nhánh này (giữ nguyên, từ HANDOFF)
M0–M11 có mã; std (47 hàm, 35 chương trình ba nguồn mỗi hàm); 6 công cụ dogfood; B1/B2-B3/D1; đột biến G1+FB-9.

## Đã XONG trên nhánh này (kiểm toán ngoài, xem FIELD_BUGS FB-C1…FB-C8)
F-01…F-08, port Linux, ngăn xếp 256 MiB, clippy sạch, CI/deny (chưa chạy trên GitHub).
