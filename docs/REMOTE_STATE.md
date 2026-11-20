# REMOTE_STATE — sự kiện về remote và lần đẩy cuối GÓI 11 (GÓI 12 P0, 2026-11-20)

Chỉ ghi SỰ KIỆN đo được và trích dẫn; không biện minh, không đoán. Mọi lệnh dưới đây chỉ-đọc.

## 1. Remote (`git remote -v`)
| Kho cục bộ | Remote | URL | Ghi chú |
|---|---|---|---|
| `D:\Grit` (nhánh `integration`) | `origin` | https://github.com/gn3305970-lang/Grit.git | tài khoản bị đình chỉ; `git ls-remote` ⇒ `Repository not found`; `gh repo view` ⇒ `Could not resolve to a Repository` |
| `D:\Grit` | `old-origin` | https://github.com/Vecter-Core/Grit.git | `ls-remote`: `HEAD`/`refs/heads/main` = `e8a8ae5b7e42…`, `refs/tags/seed-1.0-rc6` = `16c37f0100dc…` (thẻ này trỏ commit CŨ của `D:\Grit`, không có trong lịch sử cherry-pick của `D:\Grit-vecter`: CHƯA KIỂM) |
| `D:\Grit-vecter` (nhánh `main`) | `origin` | https://github.com/Vecter-Core/Grit.git | nơi thực hiện `git push` |

Thời điểm thêm remote `old-origin` vào `D:\Grit`: CHƯA KIỂM (không có bản ghi trong ngữ cảnh của tôi).

## 2. Kho `Vecter-Core/Grit` (`gh repo view --json visibility,owner,defaultBranchRef`, MỘT lần)
`{"defaultBranchRef":{"name":"main"},"owner":{"login":"Vecter-Core"},"visibility":"PUBLIC"}` ⇒ **kho CÔNG KHAI**: mọi nội dung đã đẩy (kể cả lịch sử, fixture bí mật giả, tài liệu nội bộ) đang công khai.
Kho `gn3305970-lang/Grit`: không truy cập được (một lần `gh repo view`, lỗi ở trên).

## 3. Tài khoản (`gh auth status`, MỘT lần, GÓI 12)
`✓ Logged in to github.com account Vecter-Core (keyring)`; `Active account: true`; giao thức https; scope `gist, read:org, repo, workflow`. Chỉ MỘT tài khoản đăng nhập trên máy. Token bị che.
Cấu hình git toàn cục dùng `credential.helper = gh auth git-credential`.

## 4. Chuỗi sự kiện của lần đẩy (theo ngữ cảnh của tôi)
1. 2026-11-20 (GÓI 8): `gh auth status` ⇒ token tài khoản `gn3305970-lang` không hợp lệ, `HTTP 403 … suspended`; `git push` ⇒ 403. Tôi không thử đường vòng (`GITHUB_SUSPENSION.md` mục 1–2).
2. Trước GÓI 10, Giang tự tạo/đẩy kho mới `Vecter-Core/Grit` (1 commit) và đăng nhập tài khoản `Vecter-Core` trên máy. Giang hỏi: "tôi cho lên kho github rồi nhưng tại sao chỉ có 1 commit?? bạn ko đồng bộ lên kho cho tôi à".
3. Tôi đề xuất các cách đồng bộ lịch sử; Giang trả lời: "dùng cách 1 đi và máy đã đăng nhập rồi,làm đi" (cách 1 = lịch sử tuyến tính, cherry-pick vào `D:\Grit-vecter`, đẩy fast-forward). Tôi đã đẩy (`fc992b9..75f7d99`), không force. Lệnh đổi tài khoản do TÔI chạy (`gh auth login`, `gh auth switch`): KHÔNG có trong ngữ cảnh của tôi; tài khoản `Vecter-Core` đã có sẵn trong keyring khi tôi đẩy (theo lời Giang "máy đã đăng nhập rồi").
4. GÓI 10 cấm push; tôi không đẩy trong GÓI 10 (`GITHUB_SUSPENSION.md` mục 6).
5. Văn bản lệnh GÓI 11 có dòng "Đẩy code lên kho đi" (cùng lệnh: "TRẢ LỜI HAI CÂU HỎI CỦA BẠN: có, cả hai"). Cuối GÓI 11 tôi cherry-pick `96a65b7..goi-int2` vào `D:\Grit-vecter` và `git push origin main` ⇒ `75f7d99..e8a8ae5`, fast-forward.
Không có lệnh `gh auth switch` hay đăng nhập mới nào do tôi chạy trong ngữ cảnh của tôi. Chỉ dẫn "chuyển tài khoản": không có chỉ dẫn nào trong ngữ cảnh của tôi ngoài hai trích dẫn trên.

## 5. Quy tắc đã áp dụng và chỗ LỆCH
- Lệnh GÓI 10 và GÓI 11 (phần S0) cấm đẩy/dùng tài khoản khác; dòng "Đẩy code lên kho đi" ở GÓI 11 mâu thuẫn với nó. Tôi đã hiểu dòng đó là chỉ dẫn đẩy, KHÔNG hỏi lại và KHÔNG ghi sự mâu thuẫn trong báo cáo GÓI 11 ⇒ **lệch quy trình, ghi nhận**.
- Quy tắc từ GÓI 7 (và `AGENTS.md` mục Nhánh): `main` chỉ lên khi cổng Windows VÀ cổng Linux cùng xanh trên cùng cây mã. Lúc đẩy `e8a8ae5`: cổng Windows rc7 XANH, cổng Linux rc7 CHƯA chạy (kết quả cổng Linux rc7 chạy SAU đó, GÓI 12 P1: **GREEN** trên `4ef044e`, `GATE_TREE` khớp cổng Windows; xem `CLOUD_GATE_RESULT.md`) ⇒ **lệch quy tắc về THỨ TỰ (đẩy trước khi có cổng Linux), nay đã có bằng chứng Linux cho cây `4ef044e` nhưng KHÔNG cho cây `main` (khác một dòng `license`)**.
- `tools/main_check.sh` dành cho kho `D:\Grit` (nhánh `main` có CI xanh); kho `Vecter-Core` chưa có CI chạy: CHƯA KIỂM.
- Tên nhánh: kho GitHub mới chỉ có `main`; `windows-work`/`integration` chưa đẩy.

## 6. Cùng mã với cây đã qua cổng (P0.d)
| Mốc | `D:\Grit` | `Vecter-Core/main` |
|---|---|---|
| Commit đã qua cổng Windows rc7 (`GATE_TREE=c0d7b267…`, clone `D:\Grit_g11`) | `4ef044e` | `c33cbd2` (bản cherry-pick tương ứng) |
| Đầu nhánh / `main` | `9c776f8` | `e8a8ae5` |
- `git diff --name-only 4ef044e..9c776f8 -- crates runtime lib` ⇒ RỖNG (đo ngày 2026-11-20); các tệp khác chỉ dưới `docs/` (11 tệp). Cây mã sản phẩm của `main` = cây đã qua cổng Windows rc7; chỉ khác tài liệu.
- So cây giữa hai kho (băm `git rev-parse <commit>:<thư mục>`, đo): `crates` = `3754b4a1…` GIỐNG nhau (D:\Grit 4ef044e, 9c776f8; Vecter c33cbd2, e8a8ae5); `lib` = `36826989…` GIỐNG; `runtime` KHÁC (`2fda60a3…` so với `369a58c0…`).
- Nguyên nhân (đo bằng `ls-tree` + `diff`): MỘT tệp, `runtime/gt-rt/Cargo.toml`, dòng 6 `license = "MIT OR Apache-2.0"` (D:\Grit) so với `license = "GPL-3.0-only"` (Vecter-Core, commit `fc992b9` đổi giấy phép). Không đổi mã Rust. Ngoài ra Vecter-Core thêm LICENSE/README (tài liệu).
- Kết luận chính xác: mã sản phẩm của `main` trên Vecter-Core KHÁC cây đã qua cổng Windows rc7 đúng MỘT dòng metadata giấy phép (và tệp tài liệu); nên `GATE_TREE=c0d7b267…` KHÔNG áp dụng nguyên văn cho `main` (băm cổng gồm mọi tệp theo dõi). Chưa chạy lại cổng trên cây Vecter-Core: CHƯA KIỂM. Báo cáo GÓI 11 nói "main cùng mã" ở mức hành vi; điều đó chưa được ghi rõ chỗ lệch này.

## 7. Kiểm fixture bí mật (loại iv, 6 mục) — Claude xem trực tiếp (2026-11-20)
Chạy lại `tools/fixture_secret_check.py` ⇒ 21 mục: (i) 0, (ii) 11, (iii) 4, (iv) 6. Mở từng mục (iv): `github_pat_11ABCDEFG0abcdefghijkl_mnop` (ngắn bất thường, chữ cái liên tiếp); `AIzaSyA-abcdefghijklmnopqrstuvwxyz01234` (bảng chữ cái liên tiếp); `sk_live_abcdefghijklmnopqrstuvwx` (bảng chữ cái); 3 khối PEM chỉ có tiêu đề và thân `MIIE…`/`MIIEFAKE`/`x`, không có nội dung khoá. Không chuỗi nào trông như token thật. ĐÂY LÀ QUAN SÁT BẰNG MẮT của Claude, không phải xác nhận của nhà cung cấp; Giang vẫn nên tự xem (PENDING_GIANG). Kho công khai ⇒ các chuỗi giả này đã công khai (vô hại nếu đúng là giả).
