# Grit contributor instructions

## Project contract
- Trạng thái hiện tại: xem `docs/STATUS.md`.
- Kiến trúc sư là Claude qua Giang. Người thực thi là Claude Sonnet 5.5 (từ GÓI 4, 2026-11-19 Giang CHỈ dùng Claude; không dùng GPT/Codex); thực thi gói lệnh đã được ủy quyền, không chờ duyệt lại trong phạm vi gói.
- Fail-closed: lỗi đỏ E... hoặc ICE thì không sinh binary, không chạy. Chưa chứng minh thì không được coi là an toàn. Cảnh báo vàng S... về secret vẫn cho chạy và cảnh báo.
- Chỉ báo số liệu thật từ lệnh đã chạy. Ghi `CHƯA KIỂM` cho phần chưa kiểm; không báo xong khi chưa có bằng chứng.
- Lỗi soundness/false negative: thêm test tái hiện ĐỎ trước, sửa sau trong commit riêng, ghi FB vào `docs/FIELD_BUGS.md`, mở rộng generator/oracle và kiểm bằng cách tắt bản sửa.
- Gate đỏ: sửa theo test-trước rồi chạy lại TOÀN BỘ gate. Không nới/tắt gate, không `#[ignore]`, `todo!()` hoặc stub để qua.
- Không xóa file tracked nếu chưa ghi đường dẫn, commit và lý do vào `docs/DELETIONS.md` cùng commit. Không chạy `git reset --hard`, `git clean`, `git stash drop`, force-push hay push `main`.
- Không chạy hai Cargo cùng lúc. Dùng `CARGO_BUILD_JOBS=2`, `--test-threads=2`. Lô nặng tối đa 15 phút; chạy preflight trước mỗi lô và ghi kết quả sau từng đơn vị.
- Đột biến M6/M7/M8/Guard: ít nhất một nửa tự động qua `tools/mut.py`; ghi nguồn từng kết quả là `tự động` hoặc `tay`.
- Duyệt hợp đồng TCB không phải kiểm toán mã. Chỉ đổi trạng thái TCB/READINESS khi có bằng chứng và gate liền mạch xanh trên đúng mã đó.
- Nhánh (GÓI 6): `main` = commit có CI xanh, hoặc commit CHỈ-TÀI-LIỆU (hay công cụ/test/CI, không đổi hành vi sản phẩm) ngay sau đó — kiểm bằng `bash tools/main_check.sh <commit CI xanh> <đầu main>`; chỉ fast-forward (không force, không squash). `windows-work` = nhánh làm việc. Chỉ Giang (hoặc gói lệnh nêu rõ) được đẩy `main`: `git push origin <commit>:refs/heads/main`.
- Phiên Claude được giao VIẾT CHƯƠNG TRÌNH GRIT cho người dùng (`docs/START_CODING.md`) KHÔNG được sửa compiler (`crates/`, `runtime/`), test, spec (`docs/spec/`) hay tài liệu ngoài phạm vi: chỉ viết ứng dụng trong `apps/` (hoặc `examples/`) và ghi lỗi vào `docs/USER_BUGS.md`. `gritc` từ chối chương trình bạn tin là đúng, hoặc chấp nhận chương trình bạn tin là sai: DỪNG và ghi `docs/USER_BUGS.md`; không nới/tắt kiểm tra.
- Không thêm cú pháp/từ khóa hoặc đổi ngữ nghĩa nếu chưa có quyết định kiến trúc. Intrinsic mới cần hợp đồng TCB trước.
- Test (quyết định Giang 2026-11-21): E2E trước. Hành vi mới = mẫu `.gt` trong `tests/{pass,fail,warn}` (dòng `// expect:`), chạy bởi `crates/gt-driver/tests/corpus_e2e.rs` qua `gritc check --json` (đòi ĐÚNG mã kỳ vọng). Test hồi quy chi tiết CHỈ khi có lỗi thật (FB/USER_BUG); không viết test chỉ kiểm cấu trúc dữ liệu nội bộ (hình dạng AST, `NodeId`, tên trường) hay lặp lại logic của hàm; tìm sập bằng fuzz. Không xoá oracle, bộ sinh, test giết đột biến, test hồi quy FB khi dọn test. Chi tiết: `docs/ROADMAP.md` mục 4.

## Tài liệu phải đọc trước khi làm
`GRIT_PLAN.md`, `docs/STATUS.md`, `docs/ROADMAP.md`, `.claude/skills/grit-contract.md`, `docs/KNOWN_LIMITATIONS.md`, `docs/ENVIRONMENT.md`, `docs/TCB_INTRINSICS.md`, `docs/TCB_REVIEW_GUIDE.md`. Lịch sử từng gói: `docs/HISTORY.md` (chỉ đọc khi cần).

## Chạy lệnh
- Trên Windows dùng Git Bash tại `C:\Program Files\Git\bin\bash.exe`; không dùng `bash` trong WindowsApps/WSL shim.
- Preflight: `bash tools/preflight.sh D:/Grit`.
- Gate đầy đủ: `bash tools/gate.sh D:/Grit`; chỉ một lần chạy liền mạch đầy đủ mới được gọi là xanh.
- Cổng đầy đủ luôn chạy trong CLONE sạch: `bash tools/gate_clean.sh <commit> <thư_mục_ngoài_kho>` (băm cổng gồm tệp chưa theo dõi; một tệp lạ trong kho gốc đã làm vô hiệu cổng nhiều lần). Không chạy cổng đầy đủ trực tiếp trong kho làm việc.
- Dựng runtime trước test LLVM: `cargo build --release --manifest-path runtime/gt-rt/Cargo.toml`.
- Môi trường/test recipes: `docs/STATUS.md` mục 9 và `docs/ENVIRONMENT.md`.
- Đột biến dùng `tools/mut.py`, tôn trọng phạm vi, resume, giới hạn worker và sổ kết quả.

## Mẫu báo cáo cho mỗi mục
Ghi trạng thái/ngày; thay đổi và commit; lệnh đã chạy cùng kết quả/exit code; regression/FB; phần chưa chạy; rủi ro và việc kế. Ghi `CHƯA KIỂM` cho khẳng định chưa xác minh. Cập nhật `docs/HISTORY.md` (mục của gói) và `docs/STATUS.md` (nhãn, cổng, việc chờ) theo yêu cầu gói.
