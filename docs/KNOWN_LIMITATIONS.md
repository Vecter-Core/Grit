# KNOWN LIMITATIONS (cập nhật theo milestone)

## Sau M0
- Chưa có compiler: `docs/spec/` là đặc tả M0 FINAL (Q1–Q14 đã được Giang duyệt) nhưng chưa được compiler nào kiểm chứng.
- Token đo bằng proxy (tiktoken), không phải tokenizer Claude (ADR-002).
- `tools/check_expectations.py` chỉ kiểm tính nhất quán của corpus (header, tên file, mã lỗi, Unicode, `.tokens`); **chưa** chạy gritc. Các mẫu **chưa được parse bởi công cụ nào**, nên có thể lệch grammar ở chỗ nhỏ — lỗi lộ ở M2, khi đó phải sửa mẫu hoặc grammar và ghi lại.
- Đã chấp nhận hệ quả của Q5: mọi `μ` là token `KW_MUT` nên tên biến không được chứa `μ` ở bất kỳ vị trí nào (kể cả `aμb`, `xμ`); `aμb` lex thành `IDENT MUT IDENT` và bị parser từ chối.
- Tên token đã chốt chính thức ở M1 (ADR-010); `tests/lexer/*.tokens` đã cập nhật.
- Một số mẫu fail dựa vào hàm chưa định nghĩa trong core (`root_fs`, `raw_read`, `Arena::new`) vì lib/core viết ở M12; mẫu chỉ nhằm minh hoạ mã lỗi, cần rà lại ở milestone sở hữu mã đó.
- Chưa có mẫu đa file (E0204 vòng phụ thuộc module) — thuộc M3.
- Grammar M0 đã đóng hết Q1–Q14; không còn điểm mở. Tên token đã chốt (ADR-010).
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

## Lexer (M1)
- **Homoglyph chưa phủ hết:** chỉ phát hiện (a) trộn hai bảng chữ trong một định danh/lifetime (E1203) và (b) `µ` (U+00B5) giả `μ`. **Không** phát hiện định danh thuần một bảng chữ nhưng trông như tên Latin (vd `х` Cyrillic thay `x`; `аррӏе` toàn Cyrillic) — cần bảng confusables UTS #39; chưa làm. Rủi ro: hai định danh khác nhau nhưng trông giống nhau.
- Quy tắc trộn bảng chữ cũng từ chối cả tiếng Nhật trộn Han + Hiragana/Katakana trong một định danh (fail-closed).
- Danh sách ký tự "vô hình" là danh sách hữu hạn (bidi, zero-width, định dạng, tag, variation selector…), không phải mọi ký tự Cf; ký tự mới của Unicode có thể lọt cho tới khi cập nhật bảng.
- `x<-1` lex thành `x <- 1` (gán) theo khớp-dài-nhất; muốn so sánh với số âm phải viết `x < -1`.
- Văn bản `STR_LIT`/`CHAR_LIT` trong token là văn bản nguồn nguyên văn (chưa giải mã escape); giải mã/kiểm giá trị ở tầng sau.
- Literal số chưa kiểm phạm vi (`128i8` → E0601 ở M7). `ƒ` và `μ` đều là ký hiệu dành riêng nên không có định danh nào chứa `ƒ` hay `μ`.
- Fuzz lexer: 6 phiên × 10 phút sạch; chưa có phiên liên tục ≥1 giờ (ASan Windows) và chưa đạt ≥24 giờ tích luỹ của cổng chuyển giai đoạn.
- **leak_check** (`crates/gt-lexer/tests/leak_check.rs`) đo bộ nhớ qua tiến trình con/`/proc` vì crate cấm `unsafe` (không gọi được `GetProcessMemoryInfo`): Windows dùng PowerShell (`PrivateMemorySize64`), Linux `/proc/self/status` (VmRSS), macOS `ps`. Môi trường khác, hoặc máy không có PowerShell/`ps`, **không đo được → test FAIL** (fail-closed). Không có phương án đếm allocation vì cần `unsafe impl GlobalAlloc`. Test chạy ~12–25 s (1 triệu lần lex) và không bị `#[ignore]`.
- **Fuzz trên Windows:** ASan hết bộ nhớ nội bộ khi quarantine mặc định 256 MB (đã gặp ở lần chạy 1 giờ). `tools/fuzz_lexer.sh` đặt `ASAN_OPTIONS=quarantine_size_mb=8` và chạy nhiều phiên ngắn, giữ corpus. Cần nightly, `cargo-fuzz`, và `clang_rt.asan_dynamic-x86_64.dll` trong PATH.
