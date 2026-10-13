# DOGFOOD — dùng Grit thật (ghi từ 2026-10-13)

Mỗi dòng = một khoảng trống/bug gặp khi viết công cụ thật. Cột "gốc": gritc | lib | codegen | runtime | spec. "Chặn" = chặn công cụ `wc` hay không. Bug thật sửa theo `docs/FIELD_BUGS.md`.

## Công cụ đầu tiên: `examples/wc` (dự án Rig, 1 gói, ~30 dòng Grit)
`wc.exe <đường-dẫn>` in `<dòng> <từ> <byte> <đường-dẫn>`. Từ = dãy byte không phải ` `, `\n`, `\t`, `\r`. Kiểm bằng `crates/rig/tests/dogfood.rs` trên 10 tệp (rỗng, 1 xuống dòng, không xuống dòng cuối, CRLF, chỉ khoảng trắng, đủ 256 byte, NUL/byte cao, văn bản UTF-8, 100 KB ngẫu nhiên, đường dẫn có dấu cách) + tệp 32 MiB + 2 đường lỗi. Tất cả khớp bản tham chiếu Rust. **Đo thật:** 32 MiB trong 0.44 s ≈ 72 MiB/s (LLVM -O0, mọi check giữ; gồm cả đọc tệp). Lỗi: không có tệp ⇒ `abort: io_error` mã 70; thiếu đối số ⇒ `abort: arg_out_of_range` mã 70; không in gì ra stdout.

## Khoảng trống THEO THỨ TỰ CHẶN (thứ tự gặp khi build tool)
| # | ngày | triệu chứng | gốc | chặn wc? | đã sửa? / test |
|---|---|---|---|---|---|
| 1 | 2026-10-13 | `E0201 root_out chưa được định nghĩa`: không có cách in ra | lib (prelude) | CÓ | Có: `▢ Out`, `Cap<Out>::write_u64/write_str`, `root_out()` (prelude + fixture gt-types khoá cùng nhau); test dogfood |
| 2 | 〃 | không có tham số dòng lệnh để biết tệp nào | lib | CÓ | Có: `Cap<Env>::arg(i) → &str` + runtime `gritrt_arg_*`; test "thiếu đối số" |
| 3 | 〃 | `File` chỉ có `len`, không đọc được byte | lib | CÓ | Có: `File::bytes(&self) → &[u8]` (đọc NGUYÊN tệp vào bộ nhớ, rò rỉ có chủ đích) |
| 4 | 〃 | `gritc: backend llvm chưa hỗ trợ: hàm không có thân MIR: $prelude::root_env` — MỌI hàm prelude của `Cap<…>`/`File` chưa hạ được | codegen | CÓ | Có: intrinsic `RootCap`, `EnvArg`, `FsRead`, `FileLen`, `FileBytes`, `OutU64`, `OutStr` (+ `gritrt_fs_read`, `gritrt_file_*`, `gritrt_write_*`) |
| 5 | 〃 | không có kênh báo lỗi I/O (`read` trả `File`, không `Result`) ⇒ mọi lỗi I/O = abort mã 70 + dòng `io_error: …` ở stderr | lib/spec | KHÔNG cho wc v0 | **ĐÃ SỬA 2026-10-13 (lệnh 7a):** thêm `Cap<Fs>::open → Result<File, IoError>` (`IoError {NotFound, Denied, Invalid, Other}`) + `core::exit(code) → !`; `read` (abort) giữ để không vỡ corpus; wc dùng `open`, lỗi ⇒ in lý do, thoát mã 2; test `wc_error_paths_are_controlled`. Còn thiếu: stderr riêng (chưa có capability), `?` |
| 6 | 〃 | (ĐÃ ĐÍNH CHÍNH 2026-10-13) báo cáo bước 5 viết sai rằng `root_*` gọi ở đâu cũng được. Đo thật: hàm thường gọi `root_fs()`/`root_out()` bị `E0310` (chỉ `main` cấp cao được gọi nguồn capability, D29). Phần còn lại đúng: capability gốc là hàm prelude (giả lập), chưa do runtime cấp cho `main` | lib/spec | KHÔNG | Đúng sẵn từ M4.5; thêm test hồi quy cho `root_out`: fail/E0310_root_out_in_helper, pass/E0310_ok_out_passed_down |
| 7 | 〃 | `N0602`: `b[i]` với `b ≔ f.bytes()` và vòng `i < b.len()` KHÔNG được chứng minh (giữ check lúc chạy); cùng mẫu với tham số `xs: &[T]` thì chứng minh được | gritc (bộ giải M7, quan hệ zone chưa gắn `b.len()` của biến cục bộ mượn từ lời gọi) | KHÔNG (check runtime đúng, chỉ chậm) | Chưa; ghi KNOWN_LIMITATIONS |
| 8 | 〃 | In số/chuỗi chưa có định dạng: chỉ `write_u64` và `write_str`; không có `String`/`Vec`/format | lib | KHÔNG | Chưa (M12) |
| 9 | 〃 | IR Guard không phân lớp nghĩa vụ cho lời gọi runtime I/O (`gritrt_fs_read/write_*/arg_*`) | gritc (gt-guard) | KHÔNG | Chưa; KNOWN_LIMITATIONS |
| 10 | 〃 | tên tệp của người dùng với dấu cách, UTF-8 ok; đường dẫn không phải UTF-8 ⇒ `arg` chuyển lossy (U+FFFD) | runtime | KHÔNG | Chưa |

## Những thứ ĐÃ nghĩ có thể chặn nhưng KHÔNG chặn wc (đã kiểm bằng cách viết và chạy)
- Gọi qua trait (`Callee::TraitMethod`): wc không dùng. Số thực: không dùng. 128 bit: không dùng. `String`/`Vec`: không dùng (đếm trên lát byte). Chúng sẽ chặn công cụ lớn hơn (xử lý văn bản, đếm tần suất từ ⇒ cần `Vec`/`HashMap` + trait).
- `⌥`/enum, tuple, lát, `u8`, vòng `↓`: đều dùng được trong dự án thật.

## Công cụ 2: bộ phân tích JSON (`examples/json`) — khoảng trống THEO THỨ TỰ CHẶN, ghi TRƯỚC khi code (2026-10-13)
Đã thử bằng chương trình thật (`gritc check`/`run`) trước khi kết luận:
| # | khoảng trống | kiểm bằng | chặn? | quyết định |
|---|---|---|---|---|
| J1 | **Cây giá trị đệ quy** `enum Value<'a>` chứa `&'a Node<'a>`; cấp phát nút | thử `j2.gt`: enum đệ quy qua tham chiếu arena + `ar.alloc(..)` ⇒ gritc check XANH (borrowck/termination) và chạy đúng | không còn | cần `Arena::{new,alloc,reset}` ở codegen (đã thêm: intrinsic + `gritrt_arena_*`, lớp IR Guard E0405) |
| J2 | `Vec` | không có trong prelude | KHÔNG nếu dùng arena + danh sách liên kết | tool dùng `Node::Cons(&Value, &Node)`; `Vec` chưa làm (ghi lại: chặn công cụ cần truy cập ngẫu nhiên/sắp xếp) |
| J3 | `String` | không có | KHÔNG nếu giữ chuỗi/số dạng khoảng `(đầu, dài)` trên bộ đệm tệp | tool KHÔNG giải mã escape/số; chỉ kiểm cú pháp (đủ cho yêu cầu: đếm, độ sâu, hợp lệ). Ghi rõ là hạn chế |
| J4 | `Result`/`IoError` | thiếu | không chặn nhưng yêu cầu | thêm `open` (xem hàng 5 phía trên) và đổi wc |
| J5 | Số thực | `Ty::Float` ⇒ `Unsupported` ở codegen (đã thử ở nhóm test "chưa hỗ trợ") | KHÔNG nếu không cần GIÁ TRỊ số | số JSON được kiểm cú pháp, không đổi sang `f64`; chặn mọi tool cần giá trị thập phân |
| J6 | Nhân/chia 128 bit | `Unsupported` | KHÔNG | không dùng |
| J7 | Gọi qua trait / generic có ràng buộc | `Callee::TraitMethod` ⇒ `Unsupported` | KHÔNG nếu tránh trait | tool không dùng trait. **Codegen HIỆN hạ được generic KHÔNG ràng buộc** (đơn hình hoá hàm + ADT, đã test `id<T>`, `Option<i32>`, `Atomic<T>`, `Result<T,T>`); KHÔNG hạ được lời gọi phương thức trait (cả `⊐ Trait for T` lẫn `T: Bound` gọi `t.method()`) |
| J8 | Đệ quy sâu làm tràn stack gốc | thiết kế | PHẢI xử lý | tham số ngân sách độ sâu `depth` (↓ depth, M8 chứng minh dừng) ⇒ lồng quá `MAX_DEPTH` trả `Err(TooDeep)` có kiểm soát, KHÔNG đệ quy sâu hơn ngân sách; stack gốc của exe 1 MiB mặc định (link.exe) đủ cho ngân sách |
