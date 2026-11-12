# Báo cáo đối kháng nhóm B (I/O và capability) — Grit

Chương trình tái hiện nằm trong /tmp/eprompt/B/. Chạy bằng /tmp/gritbin/gritc. Backend LLVM, hồ sơ dev trừ khi ghi khác.
Thông dịch (crates/gt-mir/src/interp.rs) KHÔNG cài bất kỳ intrinsic I/O/capability nào (không có root_*, read, open, exit, write_*). Vì vậy mục (4) "kiểm ở thông dịch mà không ở codegen" không áp dụng cho nhóm B. Thông dịch chỉ so sánh `&str ==` bằng `==` của Rust (interp.rs:1063-1101).

## Danh sách THẬT theo mức nghiêm trọng

1. **THẬT-CAO — điều kiện cứng #12 bị phá: `main` không phải entry duy nhất được miễn E0310.**
   - Nguyên nhân: crates/gt-types/src/check/mod.rs:369-372. `entry` = không thuộc impl, có thân, khoá `a::main` (đúng 2 đoạn). Không kiểm `arg_count == 0` và không kiểm tệp là tệp gốc.
   - `find_main` của codegen (lower.rs:144-158) và runner (gt-driver/src/runner.rs:19) chỉ coi `main` KHÔNG tham số là entry. Một `util::main(x: u64)` ở tệp thứ hai được check miễn E0310 mà vẫn không bị tính là entry, nên không có lỗi "nhiều main".
   - Tái hiện mf2/: `util::main(x)` gọi `root_out()` và `root_fs()`, `helper()` (không phải main) gọi `util::main(5u64)`. `gritc check .` xanh, `gritc run .` in "util::main(x) used root_out+root_fs outside entry; x=5" rồi in độ dài /etc/hostname (3).
   - Tái hiện mf3/: `util::main(x) → W` với `W { c: Cap<Fs> }`. `helper()` lấy `w.c` rồi `w.c.read(...)`; chạy được, in 3. Đây là lấy `Cap<Fs>` thật ở hàm ngoài main.
   - mf4/ (trả thẳng `Cap<Fs>`) bị E0310 ở điểm gọi, nên chỉ bọc trong struct mới lọt (xem mục 2).
   - Guard không bắt: nó chỉ kiểm nhãn `!gt.obl=E0310` theo LỚP (gt-guard/src/lib.rs:7-9, 222-231), không kiểm chỗ gọi.
2. **THẬT-CAO (đồng phạm của 1) — `has_cap` không đi vào trường của struct người dùng.**
   - check/mod.rs:459-470: `go` chỉ đệ quy qua `Adt` args, tuple, array, slice, ref, ptr, fn. Hàm trả `W{c:Cap<Fs>}` mà không nhận Cap thì không bị E0310 ở điểm gọi.
   - Tự nó chưa đủ để lấy Cap (vẫn cần một hàm miễn trừ để dựng W), nhưng cùng mục 1 thì phá hoàn toàn mô hình.
3. **THẬT-TRUNG — `io_result` đặt `alloca` tại chỗ gọi (lower.rs:3438), không ở khối entry.**
   - Mỗi lần chạy `Cap<Fs>::open`/`write`/`Cap<In>::read_all` trong vòng lặp cấp thêm ngăn xếp (alloca động), không bao giờ giải phóng trước khi hàm trả về.
   - Tái hiện r02_loop_open.gt (40 triệu lần `fs.open("/nonexistent_zzz")`): SIGSEGV (exit 139) ở dev và release-o2 sau ~7 giây. Cùng chương trình với 5 triệu lần chạy bình thường (r02b.gt).
   - Không phải ghi ngoài vùng (chặn bởi trang bảo vệ) nhưng chương trình hợp lệ, đã được chứng minh dừng, chết bằng SIGSEGV thay vì abort có kiểm soát; hợp đồng "không bao giờ abort vì I/O" bị phá.
   - Cùng kiểu: lower.rs:2819 (`cas.res`) thuộc nhóm khác, chưa thử.
4. **THẬT-THẤP — ghi vào stdout đã đóng (`>&-`) thành công im lặng.** `std::io::stdout` của Rust nuốt EBADF.
   - runtime lib.rs:350-355, 421-430 mong `write_all` lỗi ⇒ abort `io_error` (TCB_INTRINSICS dòng 21, 22).
   - r03.exe x >&- ⇒ exit=255 (từ `core::exit(300)`), không có abort. `> /dev/full` thì abort 70 đúng.
5. **THẬT-THẤP — `check` xanh nhưng build/run đỏ cho hàm prelude không có lowering.**
   - `Host::fs()`, `Cap<Fs>::sub`, `Cap<Net>::connect`, `Cap<Env>::var`, `Cap<Clock>::now`, `raw_read`: intr_of trả None (lower.rs:2003-2142) nên codegen báo "chưa hỗ trợ".
   - t05_host_in_main.gt: check xanh, run lỗi "hàm không có thân MIR: $prelude::Host::fs"; thông dịch cũng "không có thân MIR". Fail-closed, không phải lỗ hổng, nhưng `check` nói dối về khả năng build.
   - Cũng áp dụng cho chương trình 2 `main` (mf1/): check xanh; codegen/runner từ chối "nhiều hàm main".

## Bảng theo mục

| mục | kết luận | bằng chứng | chương trình |
|---|---|---|---|
| `root_*` / E0310 / điều kiện #12 | **THẬT** (mục 1, 2) | check/mod.rs:369-372, 459-470, 524-541; call.rs:102,231,281,406,505 | mf2/, mf3/ |
| Cap qua literal / trường private | LOẠI: E0313 + E0203 chặn `Cap{..}`, `Fs{x:..}`, `File{x:..}` | thông báo thật | t01, t02, t03 |
| `Host::fs()` ngoài main | LOẠI: E0310 bắt (call.rs:281); trong main thì codegen từ chối | t04, t05 | t04, t05 |
| con trỏ hàm / giá trị hàm / trait / generic | LOẠI (đã đọc): call.rs:406-413 ghi CapUse cho gọi qua giá trị; nhắc `root_fs` làm giá trị cũng là CapUse (call.rs:102); trait: call.rs:231. Không dựng được lối vòng ngoài mục 1 | (chưa viết thêm chương trình riêng) | — |
| `spawn` đóng gói Cap | LOẠI/không thử: `Cap` Send theo thiết kế, chuyền xuống là hợp lệ, không phải nguồn | — | — |
| `Cap<Fs>::read` (FsRead) | ĐÚNG hợp đồng, có lưu ý nhỏ | lower.rs:2632-2641; runtime 235-267 (`read_file_checked`: thư mục/thiết bị/`/proc` ⇒ Err ⇒ abort) | r01 |
| `Cap<Fs>::open` | ĐÚNG; NUL trong đường dẫn ⇒ `Other` (không phải `Invalid`, tài liệu chỉ nói không-UTF-8); đường dẫn rỗng ⇒ NotFound | lower.rs:2569-2576; runtime 274-292 | r01 (nul: Other, empty: NotFound, dir/dev: Other) |
| `from_raw_parts(null,0)` ở fs_read/fs_open/fs_write cho đường dẫn (runtime 250, 275, 324) | NGHI-thấp (UB lý thuyết khi `&str` rỗng mang con trỏ null, ví dụ `Text::new().as_str()`); khác `write_str` đã có `n == 0` return. Thực nghiệm không lỗi (runtime dựng release, không bật ub_checks) | runtime lib.rs:250, 275, 324 vs 373, 389, 422 | r01 "emptyText" ⇒ NotFound, không crash |
| `Cap<Fs>::write` | ĐÚNG ở đường đã đọc (n==0 xử lý, mã âm ánh xạ qua `io_result`); không chạy ca ghi lỗi riêng | lower.rs:2583-2601; runtime 323-337 | — |
| `File::len/bytes` | ĐÚNG: tay cầm i32 (lower.rs:2642-2658) khớp `gritrt_file_len(i32)->u64`; File không giả mạo được (E0203). Tay cầm tràn i32 chỉ sau 2^31 tệp, bỏ qua | runtime 203-215, 340-347 | t02 (bị E0203) |
| `Cap<In>::read_all` | ĐÚNG: 100 MB stdin ⇒ `100000000,0`; stdin đóng ⇒ `0,0`; stdin là thư mục ⇒ `Err,Err` | lower.rs:2577-2582; runtime 306-316 | r05 |
| `Cap<Env>::arg/arg_count` | ĐÚNG: đối số không-UTF-8 chuyển lossy (`a\xffb` ⇒ 5 byte); `usize::MAX` ⇒ abort `arg_out_of_range` mã 70 | lower.rs:2619-2631; runtime 168-200 | r03, r04 |
| `Cap<Out>/ErrOut` write_* | ĐÚNG ở lỗi thường (/dev/full ⇒ abort 70); SAI ở stdout đóng (mục 4) | lower.rs:2664-2700 | r03 |
| `core::exit` | ĐÚNG: `exit(300u32)` ⇒ exit 255 (min 255) | lower.rs:2294-2304; runtime 57-61 | r03 |
| `&str == &str` (`gritrt_bytes_eq`) | ĐÚNG: khác độ dài ⇒ 0 không đọc; len 0 ⇒ 1 không đọc; `Ne` đảo đúng | lower.rs:1619-1650; runtime 401-416 | (chỉ đọc mã; chưa viết chương trình mới, đã có `str_eq.rs` của tác giả) |
| `io_result` (dựng Result) | SAI về stack (mục 3); tag Ok=0/Err=1, mã lỗi `-(h+1)` khớp thứ tự `IoError` | lower.rs:3426-3483 | r02 |
| bảng `RUNTIME` của Guard (5) | Đủ 27 hàm khớp với runtime; fail-closed với `gritrt_*` lạ; chặn địa chỉ hàm runtime (lib.rs:140-178). Nhưng chỉ kiểm NHÃN lớp, nên không chặn mục 1 (codegen gắn E0310 cho mọi lời gọi `call_basic`, lower.rs:2984; `gritrt_file_len/ptr` bị gán E0310 dù hợp đồng nói "không cần cap") | gt-guard/src/lib.rs:84-112, 222-231 | — |
| `intr_of` khớp bằng `key.contains(...)` | NGHI-thấp: chỉ áp dụng cho hàm không có thân (lower.rs:2172) nên người dùng không chèn được hàm trùng tên; không tái hiện | lower.rs:2003-2142 | — |

## Phần không kịp đọc / chưa thử
- Không chạy ca lỗi giữa chừng của `Cap<Fs>::write` (đĩa đầy, đường dẫn là thư mục) và không đo tay cầm đạt 2^31.
- Không viết chương trình riêng cho `&str ==` với con trỏ null / chuỗi lớn, và không thử `spawn` đóng gói Cap.
- Không đọc mã tests/fail E0310 từng tệp ngoài hai tệp mẫu; không đọc `gt-driver/tests/io_intrinsics.rs`.
- Chưa kiểm xem thêm đường vòng nào cho mục 2 ngoài `main` miễn trừ (ví dụ trait mặc định, hằng/static) — các tệp tests/fail E0310_* nói đã chặn.
- Không chạy ThreadSanitizer hay Windows (nền kiểm duy nhất: Linux x86_64).
