# GITHUB_SUSPENSION — sự kiện đình chỉ tài khoản GitHub (GÓI 10, 2026-11-20)

Tài liệu CHỈ ghi sự kiện, việc đã làm/không làm, và trình tự sau khi khôi phục. Không phải hướng dẫn lách.

## 1. Lỗi nguyên văn và ngày
```text
GÓI 6  : Actions has been disabled for this user        (HTTP 422, khi chạy workflow)
GÓI 8  : gh auth status → The token in keyring is invalid
         Error: HTTP 403: Sorry. Your account was suspended
         git push origin integration:windows-work → remote: Your account is suspended. Please visit
         https://support.github.com ... returned error: 403      (2026-11-20)
```
Tài khoản bị ảnh hưởng: `gn3305970-lang`. `origin/windows-work` trên GitHub dừng ở `4a520f1`; rc6, GÓI 7, 8, 9 chỉ có trên máy (xem bản sao ngoài máy ở mục 5).

## 2. Việc agent đã làm
- Đẩy tự động bằng `gh`/`git push` và chạy CI (workflow_dispatch) trên kho `gn3305970-lang/Grit` qua các gói trước.
- Sau lỗi 403: KHÔNG thử đường vòng (không đổi tài khoản, không dùng token khác, không remote tự tạo).
- Theo yêu cầu TRỰC TIẾP của Giang (2026-11-20, trước GÓI 10), một kho mới `Vecter-Core/Grit` được tạo cục bộ (`D:\Grit-vecter`, giấy phép GPL-3.0-only) và đẩy bằng tài khoản mới mà Giang tự đăng nhập trên máy. Đó là việc của Giang, không phải đường vòng do agent chọn; từ GÓI 10 agent KHÔNG đẩy thêm.

## 3. Nguyên nhân: CHƯA KIỂM (chỉ là giả thuyết)
- Khối lượng thao tác tự động (đẩy/chạy workflow) từ tài khoản còn mới — CHƯA KIỂM.
- Việc dùng GitHub Actions (số lần chạy, thời lượng) — CHƯA KIỂM.
- Nội dung kho (vd chuỗi giống khoá bí mật trong fixture, mã bảo mật) — CHƯA KIỂM.
- Thanh toán/xác minh tài khoản — CHƯA KIỂM.
GitHub chưa cho agent biết lý do; chỉ Giang đọc được thông báo trên web/email.

## 4. Việc của Giang
1. Đọc thông báo đình chỉ (web và email đăng ký).
2. Kháng nghị qua GitHub Support (https://support.github.com); trả lời trung thực những gì đã làm (mục 2).
3. Sau khi khôi phục, theo trình tự: `gh auth refresh -h github.com`; `git push origin integration:windows-work`; chạy workflow_dispatch `only=all full=true` MỘT lần.

## 5. Bản sao ngoài máy
`tools/make_export.sh` ⇒ `D:\Grit_export` (bundle `--all`, tar cây mã của `seed-1.0-rc6`, SHA256SUMS, RESTORE.md, REFS.txt). Giang cần chép ra ngoài máy (xem `docs/PENDING_GIANG.md`).

## 6. Trạng thái cuối gói (`gh auth status` chạy MỘT lần)
`gh auth status` (2026-11-20, cuối GÓI 10, MỘT lần, nguyên văn; token đã bị che bởi chính `gh`):
```text
github.com
  ✓ Logged in to github.com account Vecter-Core (keyring)
  - Active account: true
  - Git operations protocol: https
  - Token: gho_************************************
  - Token scopes: 'gist', 'read:org', 'repo', 'workflow'
```
Máy hiện đăng nhập tài khoản MỚI `Vecter-Core` (do Giang tự đăng nhập), không còn tài khoản `gn3305970-lang` bị đình chỉ. GÓI 10 cấm đẩy ⇒ agent KHÔNG đẩy gì trong gói này. Kho `gn3305970-lang/Grit` vẫn ở `4a520f1` (CHƯA KIỂM: không gọi API).

## 7. Trạng thái cuối GÓI 11 (`gh auth status`, MỘT lần, 2026-11-20)
```text
github.com
  ✓ Logged in to github.com account Vecter-Core (keyring)
  - Active account: true
  - Git operations protocol: https
  - Token: gho_************************************
  - Token scopes: 'gist', 'read:org', 'repo', 'workflow'
```
Trong GÓI 11 có đẩy lên kho MỚI `Vecter-Core/Grit` theo lệnh trực tiếp của Giang (fast-forward `fc992b9..75f7d99`, rồi lần cuối gói); kho cũ `gn3305970-lang/Grit` không đụng.
