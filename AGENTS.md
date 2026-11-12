# Grit contributor instructions

## Project contract
- Trạng thái hiện tại: xem `docs/HANDOFF.md`.
- Kiến trúc sư là Claude qua Giang. Codex thực thi gói lệnh đã được ủy quyền; không chờ duyệt lại trong phạm vi gói.
- Fail-closed: lỗi đỏ E... hoặc ICE thì không sinh binary, không chạy. Chưa chứng minh thì không được coi là an toàn. Cảnh báo vàng S... về secret vẫn cho chạy và cảnh báo.
- Chỉ báo số liệu thật từ lệnh đã chạy. Ghi `CHƯA KIỂM` cho phần chưa kiểm; không báo xong khi chưa có bằng chứng.
- Lỗi soundness/false negative: thêm test tái hiện ĐỎ trước, sửa sau trong commit riêng, ghi FB vào `docs/FIELD_BUGS.md`, mở rộng generator/oracle và kiểm bằng cách tắt bản sửa.
- Gate đỏ: sửa theo test-trước rồi chạy lại TOÀN BỘ gate. Không nới/tắt gate, không `#[ignore]`, `todo!()` hoặc stub để qua.
- Không xóa file tracked nếu chưa ghi đường dẫn, commit và lý do vào `docs/DELETIONS.md` cùng commit. Không chạy `git reset --hard`, `git clean`, `git stash drop`, force-push hay push `main`.
- Không chạy hai Cargo cùng lúc. Dùng `CARGO_BUILD_JOBS=2`, `--test-threads=2`. Lô nặng tối đa 15 phút; chạy preflight trước mỗi lô và ghi kết quả sau từng đơn vị.
- Đột biến M6/M7/M8/Guard: ít nhất một nửa tự động qua `tools/mut.py`; ghi nguồn từng kết quả là `tự động` hoặc `tay`.
- Duyệt hợp đồng TCB không phải kiểm toán mã. Chỉ đổi trạng thái TCB/READINESS khi có bằng chứng và gate liền mạch xanh trên đúng mã đó.
- Không thêm cú pháp/từ khóa hoặc đổi ngữ nghĩa nếu chưa có quyết định kiến trúc. Intrinsic mới cần hợp đồng TCB trước.

## Tài liệu phải đọc trước khi làm
`GRIT_PLAN.md`, `docs/HANDOFF.md`, `docs/RUNLIST.md`, `docs/RUN_REPORTS.md`, `.claude/skills/grit-contract.md`, `docs/KNOWN_LIMITATIONS.md`, `docs/READINESS.md`, `docs/ENVIRONMENT.md`, `docs/TCB_INTRINSICS.md`, `docs/TCB_REVIEW_GUIDE.md`.

## Chạy lệnh
- Trên Windows dùng Git Bash tại `C:\Program Files\Git\bin\bash.exe`; không dùng `bash` trong WindowsApps/WSL shim.
- Preflight: `bash tools/preflight.sh D:/Grit`.
- Gate đầy đủ: `bash tools/gate.sh D:/Grit`; chỉ một lần chạy liền mạch đầy đủ mới được gọi là xanh.
- Dựng runtime trước test LLVM: `cargo build --release --manifest-path runtime/gt-rt/Cargo.toml`.
- Môi trường/test recipes: `docs/HANDOFF.md` mục 8 và `docs/ENVIRONMENT.md`.
- Đột biến dùng `tools/mut.py`, tôn trọng phạm vi, resume, giới hạn worker và sổ kết quả.

## Mẫu báo cáo cho mỗi mục
Ghi trạng thái/ngày; thay đổi và commit; lệnh đã chạy cùng kết quả/exit code; regression/FB; phần chưa chạy; rủi ro và việc kế. Ghi `CHƯA KIỂM` cho khẳng định chưa xác minh. Cập nhật `docs/RUN_REPORTS.md`, `docs/HANDOFF.md` và trạng thái mục trong `docs/RUNLIST.md` theo yêu cầu gói.
