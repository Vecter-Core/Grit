# RUNLIST — danh sách chạy liên tục (lệnh Giang 2026-10-17 "CHẾ ĐỘ CHẠY LIÊN TỤC")

**Cách dùng (phiên mới):** đọc `docs/HANDOFF.md` rồi tệp này; làm mục CHƯA/ĐANG đầu tiên. Sau MỖI mục: cập nhật trạng thái + commit + dòng "Trạng thái/Việc tiếp theo" trong HANDOFF + báo cáo ngắn nối vào `docs/RUN_REPORTS.md`.
**Quy tắc cứng:** không dừng báo giữa chừng; dừng chỉ khi (1) cổng đỏ sau 2 lần sửa, (2) cần cú pháp/từ khoá mới hoặc đổi ngữ nghĩa ngoài ủy quyền, (3) preflight báo kho hỏng/ổ thiếu/WinError 1455 lặp 3 lần, (4) lỗ hổng điều kiện cứng #12/#9 không sửa được ngay. TRUNG THỰC hơn HOÀN TẤT: không XONG thì ghi KHÔNG XONG kèm lý do. Việc nặng theo LÔ NGẮN đồng bộ (≤ 15 phút; công cụ chỉ cho tối đa 10 phút một lệnh ⇒ dùng `--max-seconds` ≤ 480), `bash tools/preflight.sh D:/Grit` trước mỗi lô, không có tiến trình nền ngoại lệ DUY NHẤT là **cổng liền mạch** (giai đoạn `test` > 10 phút; chạy tách bằng `target/tmp/run_gate.sh` qua WMI; nếu phiên chết, cổng KHÔNG được coi là xanh ⇒ chạy lại TỪ ĐẦU).
**Cổng liền mạch ở:** sau R1, R3, R6, R8, R10 và cuối. Sau mỗi cổng xanh: `bash tools/backup_bundle.sh D:/Grit`.
**Môi trường:** không hai cargo cùng lúc, `CARGO_BUILD_JOBS=2`, `--test-threads=2`.

| ID | Việc | Trạng thái | Lệnh chạy lại / bằng chứng | Commit |
|---|---|---|---|---|
| R1 | Toàn vẹn kho: `tools/preflight.sh`, giai đoạn `nul` của cổng, 4 tệp NUL, quan hệ git, sao lưu bundle, KNOWN_LIMITATIONS | XONG | `python tools/test_preflight.py`, `python tools/test_nul_check.py`, `bash tools/nul_check.sh D:/Grit`, `bash tools/backup_bundle.sh D:/Grit`; cổng sau R1 XANH 943/0/185 + `NUL_OK (1399 tệp quét)` | 824ec22 |
| R2 | Hàm-làm-giá-trị: M6/M7 progen/obgen + vùng qua kiểu hàm; ⊳/⊲ qua giá trị hàm; bảng ĐO vs SUY RA + cờ test; hiệu ứng qua kiểu hàm | XONG (cổng kế tiếp ở R3) | `cargo test -p gt-borrowck --test oracle random_programs`, `--test fn_value_regions`; `cargo test -p gt-oblig --test oracle pruned_programs_with_function`; `cargo test -p gt-driver --test contract_via_value`; feature đo `--features gt-types/allow-intrinsic-values` | b80dffc |
| R3 | FAIL_OPEN_AUDIT rà hết phần còn lại (`tools/fail_open_scan.py`, dấu vân tay) | XONG | `python tools/fail_open_scan.py check` (CHƯA RÀ 0; nghi vấn 0); `cargo test -p gt-types --test copy_impl_bounds`, `-p gt-mir --test interp_u128`, `-p gt-driver --test llvm_vec`; cổng liền mạch XANH 961/0/189 | 36ac79d, 2fa3b1b, 93aa345, 932a99f, cc3ba41, b97338d |
| R4 | GRITRT_ARENA_CHECK không tắt được ở dev; grit-contract §3.3 hai luật | ĐANG | | |
| R5 | Chạy nặng theo lô: flake llvm_sort ≥100, std release/release-o2, Ord kiểu ghép, sortlines 5 lần | CHƯA | | |
| R6 | Arena chia sẻ (ADR-038 Q83): phụ lục ADR trước, tấn công, oracle shared_arena, TCB H1–H6 | CHƯA | | |
| R7 | HashMap (Q86 iii) trong gói std | CHƯA | | |
| R8 | Hai intrinsic I/O (Q86 iv): `Cap<In>::read_all`, `Cap<Fs>::write` | CHƯA | | |
| R9 | `tools/fuzz_loop.sh` + tích luỹ ≤ 2 giờ | CHƯA | | |
| R10 | 69 đột biến nguy hiểm của M7 (điều kiện #10) | CHƯA | | |
| R11 | Mục E: `docs/E_PROMPT.md` + khung `docs/TCB_CODE_AUDIT.md` (CHỜ PHIÊN MỚI do Giang mở) | CHƯA | | |
| R12 | Kết thúc: cổng cuối, READINESS bảng §7 từng điều kiện, HANDOFF, bundle, báo cáo cuối | CHƯA | | |
