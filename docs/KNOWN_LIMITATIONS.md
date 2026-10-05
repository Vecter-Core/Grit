# KNOWN LIMITATIONS (cập nhật theo milestone)

## Sau M0
- Chưa có compiler: `docs/spec/` là đặc tả M0 FINAL (Q1–Q14 đã được Giang duyệt) nhưng chưa được compiler nào kiểm chứng.
- Token đo bằng proxy (tiktoken), không phải tokenizer Claude (ADR-002).
- `tools/check_expectations.py` chỉ kiểm tính nhất quán của corpus (header, tên file, mã lỗi, Unicode, `.tokens`); **chưa** chạy gritc. Các mẫu **chưa được parse bởi công cụ nào**, nên có thể lệch grammar ở chỗ nhỏ — lỗi lộ ở M2, khi đó phải sửa mẫu hoặc grammar và ghi lại.
- Đã chấp nhận hệ quả của Q5: mọi `μ` là token `MUT` nên tên biến không được chứa `μ` ở bất kỳ vị trí nào (kể cả `aμb`, `xμ`); `aμb` lex thành `IDENT MUT IDENT` và bị parser từ chối.
- `tests/lexer/mu_split.tokens` đặt tên token (`MUT`, `ASSIGN_LET`...) tạm; tên chính thức do M1 chốt, khi đó phải cập nhật file này cùng lexer.
- Một số mẫu fail dựa vào hàm chưa định nghĩa trong core (`root_fs`, `raw_read`, `Arena::new`) vì lib/core viết ở M12; mẫu chỉ nhằm minh hoạ mã lỗi, cần rà lại ở milestone sở hữu mã đó.
- Chưa có mẫu đa file (E0204 vòng phụ thuộc module) — thuộc M3.
- Grammar M0 đã đóng hết Q1–Q14; không còn điểm mở. Tên token trong `.tokens` là tạm cho tới khi M1 chốt.
- `'a'` vs `'a` (semantics §15), `*` prefix/infix (§14), literal/comment (§19) mới là đặc tả + mẫu; chưa có lexer/parser kiểm.

## Hạn chế chủ ý của Phase 1 (không phải thiếu sót)
- **Toán tử bit không có cú pháp trực tiếp ở Phase 1, dùng hàm trong `lib/core`:** `band`, `bor`, `bxor`, `bnot`, `shl`, `shr` (Q9). Không chặn M1–M11; hàm viết ở M12. Do đó `|`, `^`, `~` đứng riêng là E0101.
- **Không có toán tử gán kết hợp** (`+=`, `-=`, `*=`, …): chủ ý, vì `x ← x + 1` đã đủ và nhất quán với triết lý không dư thừa. `x += 1` là E0101.
- **Comment nhiều dòng `/* */` bị CẤM** (E0101 khi gặp `/*` hoặc `*/`); chỉ có `//` và `///`. Dành riêng cho Phase 2 nếu cần. Hệ quả: `a /*p` phải viết `a / *p`.
- **`■` (static) chỉ bất biến** (D11); không có `■ μ`; static chứa kiểu đồng bộ chờ M8.
- **Ràng buộc trait chỉ inline** `<T: A + B>`; không có where-clause (`\where` dành riêng cho Phase 2).
- **Dành riêng, chưa có nghĩa:** `\extern` (FFI, D14), `\where`. Mọi chỗ dùng → E0101.
- **Phase 2 hoặc chưa có:** closure (và `|x|`), hàm biến đổi tham số (variadic), `cfg`/biên dịch có điều kiện, nội suy chuỗi, chuỗi nhiều dòng,
  raw string, macro, `dyn`/trait object. Chưa cần từ khoá dành riêng thêm: các cú pháp này hiện không đụng token nào (`|` chưa dùng; `#cfg` đi qua cú pháp thuộc tính).
- **Con trỏ thô `*□ T` / `*μ T`** chỉ hợp lệ trong khối `⚠`; ngoài `⚠` → E1001 (trừ intrinsic trusted trong `lib/core`). Phase 1 gần như mọi `⚠` của người dùng bị ĐỎ (D21).
- **Range mở trong biểu thức độc lập (không phải index/pattern) chưa có ngữ nghĩa — chờ iterator Phase 2.** `a..`, `..b`, `..` chỉ dùng được làm chỉ số slice và trong pattern; đứng độc lập bị từ chối (E0301). Range đóng `a..b`/`a..=b` làm giá trị độc lập chỉ dùng làm đối tượng của `⟳ x in …`.
- **Pattern** chưa có: or-pattern, guard, `μ` trong pattern, destructure ở `let`. Literal struct chưa có `..base` và viết tắt `{ x, y }`.
- **Iterator tổng quát chưa có:** `⟳ x in …` chỉ nhận range đóng và slice/array (qua `&`); iterable khác → E0701 cho tới M12/Phase 2.
