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

## gt-fmt và comment (ADR-012) — quy tắc gắn comment, ghi trước khi viết gt-fmt
- Comment `//` nằm trong bảng trivia của lexer, **không** nằm trong AST. Từ Q16b (ADR-016 v2) gt-fmt gắn **theo token gần nhất**, không theo node: comment dòng riêng → token KẾ TIẾP; comment cuối dòng → token LIỀN TRƯỚC; E1301 chỉ khi bước xác minh thất bại. (Quy tắc theo node 1–4 của v1 đã bị thay thế.)
- **Bất biến kiểm bằng test khi viết gt-fmt:** `fmt(x)` giữ nguyên **số comment và nội dung từng comment**; `fmt(fmt(x)) == fmt(x)`.
- Dòng trống giữa các node không nằm trong AST nên gt-fmt chưa bảo toàn (sẽ chuẩn hoá về tối đa một dòng trống); ghi nhận để Giang xem lại.
- `///` (doc comment) là token `DOC_COMMENT` gắn vào node AST nên được gt-fmt giữ theo node.
- gt-fmt ở M2 là **thư viện** (`gt_fmt::format`); chưa có lệnh `gritc fmt` — binary `gritc` (gt-driver) chỉ có từ M9, và chỉ ở đó gt-fmt mới được nối thành lệnh.

## gt-ast (ADR-013)
- **`NodeId` không ổn định qua chỉnh sửa:** ổn định cho cùng một nguồn (cấp tuần tự tất định), nhưng sửa nguồn rồi parse lại sẽ cấp lại. LSP tăng dần (M13) phải ánh xạ lại bảng kết quả theo span, không giữ NodeId cũ.
- **`NodeId` chỉ duy nhất trong một tệp:** M3 (nhiều tệp/module) cần khoá `(FileId, NodeId)`; `gt-span` chưa có `FileId`.
- Ngoặc nhóm không vào AST; số literal giữ văn bản nguyên (giá trị/phạm vi kiểm ở tầng sau).

## gt-fmt: dòng trống (Giang duyệt 2026-10-05)
- Grit muốn **đúng một dạng chuẩn**: gt-fmt tự quy định dòng trống — **một dòng trống giữa hai item cấp cao nhất liền kề, không dòng trống thừa** (không dòng trống đầu/cuối tệp, không hai dòng trống liên tiếp). Dòng trống người dùng viết không được bảo toàn (không nằm trong AST).

## gt-parser (ADR-014)
- **Mọi pass đệ quy trên AST PHẢI chạy qua `gt_stack::run_with_big_stack`** (ADR-015; `MAX_DEPTH` chỉ bảo đảm cây không sâu hơn 256, không bảo đảm stack của luồng gọi). Types/borrowck/effects/… (M4+) chưa tồn tại nên chưa có gì để vi phạm; mỗi pass mới phải có test độ sâu tương tự `depth_law`.
- Khôi phục lỗi chỉ ở mức câu lệnh / item / nhánh match: lỗi trong danh sách tham số, đối số, trường vẫn làm mất cả câu lệnh/item chứa nó (một lỗi, bỏ cả câu lệnh).
- Sau lỗi `E0102` (quá sâu) parser dừng hẳn: không báo thêm lỗi nào ở phần còn lại của tệp.
- Biểu thức phẳng quá 256 toán hạng/postfix bị từ chối (`E0102`) — hệ quả của Q15e.
- AST không có node lỗi: chỉ dùng AST khi không có ĐỎ (fail-closed).
- Parser đang theo các đề xuất Q15a/b/c/h/i chưa được Giang duyệt (OPEN_QUESTIONS); nếu bị bác, phải sửa parser và các mẫu liên quan.
- Số mức lồng tối đa theo từng loại khác nhau (127 cho khối `{ }`/`⎇`/`⟳`/`⚠`, 253–255 cho biểu thức/kiểu/pattern) vì mỗi loại tốn độ sâu khác nhau cho mỗi mức; giới hạn chung vẫn là 256.

## gt-fmt: giới hạn đã biết (M2, ADR-016)
- **Comment được đặt theo token (v2); hệ quả bố cục:** comment cuối dòng ép xuống dòng nên một danh sách tham số/đối số ngắn bị ngắt tại comment (không tự chuyển thành mỗi phần tử một dòng); nhiều lần ép dòng liên tiếp tạo bậc thang thụt +2; comment đứng trước dấu phẩy nguồn đẩy dấu phẩy xuống dòng riêng. Hợp lệ và idempotent nhưng không đẹp; cải thiện bố cục (vd tham số mỗi dòng khi có comment) là việc có chủ đích sau này, cần ADR.
- **E1301 chỉ khi xác minh thất bại** (phương án cuối). Hiện chưa có đầu vào hợp lệ nào kích hoạt (bài đo chèn comment sau từng token của 33 file corpus: 0 từ chối); nếu gặp là lỗi của gt-fmt.
- **Một dạng chuẩn duy nhất, không bảo toàn bố cục người viết:** xuống dòng/thụt lề/khoảng trắng bị chuẩn hoá; ngoặc thừa bị bỏ, ngoặc cần thiết được thêm (theo độ ưu tiên của parser); `INLINE_MAX = 60` ký tự quyết định khối một dòng hay nhiều dòng; không ngắt dòng giữa các toán tử của một biểu thức dài (chỉ khối/match/danh sách mới nhiều dòng).
- **Từ chối khi nguồn có bất kỳ chẩn đoán nào của parser** (fail-closed): gt-fmt không định dạng tệp hỏng và không có chế độ "cố gắng".
- **`x as T < y`:** quy tắc NGÔN NGỮ (như Rust, `semantics.md`), không phải workaround của fmt: `T <` mở tham số kiểu; parser báo lỗi kèm gợi ý máy áp dụng được bọc `(x as T)`; fmt in `(x as T) < y`.
- **gt-fmt chỉ là thư viện** (`gt_fmt::format`); lệnh `gritc fmt` có từ M9 (gt-driver).
