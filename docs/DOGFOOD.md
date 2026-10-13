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
| 5 | 〃 | không có kênh báo lỗi I/O (`read` trả `File`, không `Result`) ⇒ mọi lỗi I/O = abort mã 70 + dòng `io_error: …` ở stderr | lib/spec | KHÔNG (chấp nhận cho v0) | Chưa: cần `Result<File, IoError>` + `⌥`/`?` quen thuộc; KNOWN_LIMITATIONS |
| 6 | 〃 | (ĐÃ ĐÍNH CHÍNH 2026-10-13) báo cáo bước 5 viết sai rằng `root_*` gọi ở đâu cũng được. Đo thật: hàm thường gọi `root_fs()`/`root_out()` bị `E0310` (chỉ `main` cấp cao được gọi nguồn capability, D29). Phần còn lại đúng: capability gốc là hàm prelude (giả lập), chưa do runtime cấp cho `main` | lib/spec | KHÔNG | Đúng sẵn từ M4.5; thêm test hồi quy cho `root_out`: fail/E0310_root_out_in_helper, pass/E0310_ok_out_passed_down |
| 7 | 〃 | `N0602`: `b[i]` với `b ≔ f.bytes()` và vòng `i < b.len()` KHÔNG được chứng minh (giữ check lúc chạy); cùng mẫu với tham số `xs: &[T]` thì chứng minh được | gritc (bộ giải M7, quan hệ zone chưa gắn `b.len()` của biến cục bộ mượn từ lời gọi) | KHÔNG (check runtime đúng, chỉ chậm) | Chưa; ghi KNOWN_LIMITATIONS |
| 8 | 〃 | In số/chuỗi chưa có định dạng: chỉ `write_u64` và `write_str`; không có `String`/`Vec`/format | lib | KHÔNG | Chưa (M12) |
| 9 | 〃 | IR Guard không phân lớp nghĩa vụ cho lời gọi runtime I/O (`gritrt_fs_read/write_*/arg_*`) | gritc (gt-guard) | KHÔNG | Chưa; KNOWN_LIMITATIONS |
| 10 | 〃 | tên tệp của người dùng với dấu cách, UTF-8 ok; đường dẫn không phải UTF-8 ⇒ `arg` chuyển lossy (U+FFFD) | runtime | KHÔNG | Chưa |

## Những thứ ĐÃ nghĩ có thể chặn nhưng KHÔNG chặn wc (đã kiểm bằng cách viết và chạy)
- Gọi qua trait (`Callee::TraitMethod`): wc không dùng. Số thực: không dùng. 128 bit: không dùng. `String`/`Vec`: không dùng (đếm trên lát byte). Chúng sẽ chặn công cụ lớn hơn (xử lý văn bản, đếm tần suất từ ⇒ cần `Vec`/`HashMap` + trait).
- `⌥`/enum, tuple, lát, `u8`, vòng `↓`: đều dùng được trong dự án thật.
