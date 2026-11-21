# QUICKSTART_CLOUD — viết Grit từ điện thoại/trình duyệt (Claude Code đám mây)

Nền đã kiểm: container Linux x86_64. Windows: chưa kiểm. **origin (GitHub) là nguồn sự thật duy nhất**: container tạm thời, mọi thứ cần giữ phải `git push`.

```text
1. Mở phiên Claude Code đám mây trên repo Grit (`https://github.com/gn3305970-lang/Grit.git`, nhánh `windows-work` hoặc nhánh bạn chọn; kho này riêng tư, cần quyền).
2. Lần đầu mỗi phiên mới:  bash tools/setup_cloud.sh   (≈ 25 phút nếu dựng từ đầu; xem mục thời gian ở docs/HISTORY.md)
3. export PATH="$PWD/target/release:$PATH"; export GRIT_LLVM_LIB_DIR=/usr/lib/x86_64-linux-gnu; gritc doctor
4. Dặn AI (mẫu câu bên dưới). AI đọc docs/AI_GUIDE.md rồi tạo gói bằng `rig new`.
5. AI lặp: `rig lock && rig check` → sửa lỗi E… → `rig run`. KHÔNG được nới kiểm tra.
6. Kiểm kết quả với dữ liệu thật của bạn; yêu cầu AI `git commit` + `git push` khi ổn.
7. Muốn tiếp phiên sau: nói AI đọc docs/STATUS.md và docs/ROADMAP.md (lịch sử từng gói: docs/HISTORY.md).
8. Gặp `LỖI` ở `gritc doctor`: làm theo dòng "→" nó in ra (hoặc BUILDING.md).
9. Chương trình lớn: chia thành nhiều tệp `src/*.gt` (tên tệp = tên module).
10. Không dùng được Windows: chưa kiểm — dùng Linux/đám mây.
```

**Câu mẫu giao việc cho AI:**
> Đọc docs/AI_GUIDE.md. Viết bằng Grit chương trình … (mô tả: đầu vào, đầu ra, mã thoát). Chạy `gritc check` và `gritc run` (hoặc `rig check`/`rig run`), sửa tới khi đúng, KHÔNG nới kiểm tra. So kết quả với ví dụ tôi đưa và báo cáo số lần biên dịch và lỗi gặp.
