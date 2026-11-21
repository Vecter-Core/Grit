# Gói 2 — handoff cho phiên sạch tiếp theo

Ngày 2026-11-12; nhánh làm việc `integration`; `main` chưa đụng. Push dự phòng là `origin/windows-work`. Đọc `docs/STATUS.md`, `docs/HISTORY.md`, rồi file này. Worktree hiện không được coi là sạch cho tới khi kiểm tra `git status --short`.

## Không làm trong phiên chuẩn bị

- Không viết hoặc sửa lời giải trong `capstone/solutions/`; capstone hiện tại là bằng chứng đóng băng, không phải bài đang được giao lại.
- Không merge/fast-forward/push `main`.
- Không ghi kết quả lịch sử thành kết quả mới; mọi phép đo mới phải ghi HEAD, profile, seed/range, command và kết quả.
- Không chạy hai Cargo cùng lúc; `CARGO_BUILD_JOBS=2`.

## Việc còn mở, theo thứ tự

1. G2.1: giải thích/đóng survivor mutation `crates/gt-term/src/lib.rs:361` (mutation ở dòng 685 đã bị test bắt); chạy `python tools/fail_open_scan.py list --limit 0` theo lô, phân loại các 95 hit chưa rà, chỉ đánh dấu sau khi đọc từng hit; chưa gọi mục này xong.
2. G2.2 R11b: chuyển `docs/HISTORY.md` cho phiên Claude/model khác họ và lưu nguyên phản hồi; các Codex-authored TCB lines vẫn CHỜ review. Không giả lập phản hồi.
3. G2.5: ADR-C06 đã kết luận K3 có 1 caller Arena/234 dòng; giữ HashMap hiện hành. Điều kiện test riêng ≥1.500 seed vẫn mở; không tự đổi thiết kế.
4. G2.7: chỉ chuẩn bị/kiểm chứng môi trường sạch cho lần review tiếp; không author capstone.
5. Cuối Gói 2: khi các mục yêu cầu đã giải quyết, chạy gate đầy đủ đúng HEAD hiện hành một lần, cập nhật READINESS/HANDOFF/RUNLIST, bundle, push `HEAD:refs/heads/windows-work`. Không tuyên bố xong nếu điều kiện cứng chưa được xác nhận.

## Môi trường Windows

- Bash: `C:\Program Files\Git\bin\bash.exe` (không dùng WindowsApps bash).
- LLVM: `D:\Tools\llvm-20.1.8\bin` trong `PATH`; sau sửa `runtime/gt-rt`, dựng release runtime và đặt `GRIT_RT_LIB` đúng.
- Chạy `tools/preflight.sh D:/Grit` trước lô nặng; nếu PREFLIGHT_FAIL, dừng và báo số thật.

## Bằng chứng gần nhất

- `9e9b54c` là checkpoint được push lên `windows-work`; `main` không đổi.
- G2.1 codegen: sáu suite dev/release-o2, mismatch 0; mutation 7 killed / 1 survivor; fail-open scan còn 95 chưa rà.
- G2.2: H10 tests Windows xanh; 13 mục audit; prompt Claude sẵn sàng nhưng chưa được reviewer chạy.
- G2.5: ADR-C06; capstone historical 8/8 Linux, Windows chưa kiểm.
- G2.6 doc-test mới chạy: 207 khối Grit, 0 sai.
