# Intrinsic / lang item dùng làm GIÁ TRỊ hàm — bảng thực nghiệm (2026-10-17, mục 2 lệnh Giang)

Phương pháp: với mỗi hàm intrinsic của `lib/core/prelude.gt` (và vài hàm thường làm đối chứng), `tools`-ngoài (`target/tmp/iv/gen.py`, 43 chương trình) viết `f: ƒ(…) ≔ Tên;` rồi gọi `f(...)` với đối số VI PHẠM điều kiện của hàm, chạy `gritc check`, `run --backend=interpreter`, `run --backend=llvm`. Cột "trước" = bản gritc SAU FB-15 (hiệu ứng của hàm kết hợp đã giữ) và TRƯỚC quy tắc E0313; cột "sau" = bản hiện tại. **"Đọc bảng":** ĐỎ ở cột trước nhưng mã KHÁC E0313 có thể do chính chương trình thử (lỗi kiểu/mượn của tôi), không phải check lớp chặn — các ô đó KHÔNG được coi là bằng chứng "check chặn qua con trỏ". Ô nói được điều chắc chắn: **XANH ở cột trước** = check bỏ lọt (fail-open của check; backend LLVM luôn từ chối "giá trị hàm không có thân MIR", thông dịch tuỳ hàm). **ĐO THẬT hai lần:** (1) lúc chưa có quy tắc E0313; (2) 2026-10-18 đo lại bằng feature đo `gt-types/allow-intrinsic-values` (tắt E0313; `cargo build -p gt-driver --features gt-types/allow-intrinsic-values`): 43 chương trình cho KẾT QUẢ Y HỆT (26 XANH, 0 khác biệt) ⇒ cột "trước" là ĐO, không phải suy ra.

| chương trình | check TRƯỚC | thông dịch (trước) | LLVM (trước) | check SAU |
|---|---|---|---|---|
| arena_alloc_reset | ĐỎ E0405 | — | — | ĐỎ E0313 |
| arena_new | XANH | chạy xong (backend: interpreter); main trả về Unit | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| atomic_load | ĐỎ E0503 | — | — | ĐỎ E0503 |
| band | XANH | chạy xong (backend: interpreter); main trả về Int(2, U32) | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| bnot | XANH | chạy xong (backend: interpreter); main trả về Int(4294967289 | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| buf_mk | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR ch | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| cap_alloc_arena | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR ch | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| cap_clock_now | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR ch | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| cap_out_write | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR ch | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| cell_get | XANH | chạy xong (backend: interpreter); main trả về Int(1, U32) | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| core_abort | XANH | abort: abort (mã thoát 70, backend: interpreter) | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| core_exit | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR ch | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| core_f64_from_bits | XANH | chạy xong (backend: interpreter); main trả về Bool(false) | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| core_fuel_left | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR ch | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| f64_sqrt | ĐỎ E0201 | — | — | ĐỎ E0201 |
| f64_to_i64 | ĐỎ E0201 | — | — | ĐỎ E0201 |
| file_bytes | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR ch | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| file_len | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR ch | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| fs_open | ĐỎ E0404 | — | — | ĐỎ E0313 |
| fs_read | ĐỎ E0404 | — | — | ĐỎ E0313 |
| host_fs | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR ch | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| mutex_with | ĐỎ E0301 | — | — | ĐỎ E0301,E0313 |
| option_is_some | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR ch | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| option_unwrap | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR ch | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| raw_read | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR ch | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| root_fs_in_helper | ĐỎ E0310 | — | — | ĐỎ E0310,E0313 |
| root_out_in_helper | ĐỎ E0310 | — | — | ĐỎ E0310,E0313 |
| rotl | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: rotl: n ≥ 32 không c | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| rotr | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: rotr: n ≥ 32 không c | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| secret_declassify | ĐỎ E0313 | — | — | ĐỎ E0313 |
| shl | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: shl: n ≥ 32 không có | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| shr | XANH | ICE khi chạy: bộ chạy phát hiện MIR không hợp lệ: shr: n ≥ 32 không có | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| slice_len | ĐỎ E0101 | — | — | ĐỎ E0101 |
| spawn_send | ĐỎ E0501,E1002 | — | — | ĐỎ E0313,E0501,E1002 |
| str_len | ĐỎ E0201 | — | — | ĐỎ E0201 |
| str_slice | ĐỎ E0201 | — | — | ĐỎ E0201 |
| text_from_utf8 | ĐỎ E0301 | — | — | ĐỎ E0301,E0313 |
| text_push_char | XANH | chạy xong (backend: interpreter); main trả về Unit | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| text_push_utf8 | ĐỎ E0301 | — | — | ĐỎ E0301,E0313 |
| vec_as_slice | XANH | chạy xong (backend: interpreter); main trả về Int(1, Usize) | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| vec_len | XANH | chạy xong (backend: interpreter); main trả về Int(0, Usize) | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |
| vec_new_push | ĐỎ E0403 | — | — | ĐỎ E0313 |
| vec_reserve_huge | XANH | chạy xong (backend: interpreter); main trả về Unit | LLVM: giá trị hàm không có thân MIR (hàm của lang/runtime) | ĐỎ E0313 |

**Tổng:** 26/43 chương trình XANH ở cột trước (check bỏ lọt): arena_new, band, bnot, buf_mk, cap_alloc_arena, cap_clock_now, cap_out_write, cell_get, core_abort, core_exit, core_f64_from_bits, core_fuel_left, file_bytes, file_len, host_fs, option_is_some, option_unwrap, raw_read, rotl, rotr, shl, shr, text_push_char, vec_as_slice, vec_len, vec_reserve_huge.
Cột sau: 43/43 chương trình không còn XANH (E0313 hoặc lỗi khác của chương trình thử); 0 chương trình XANH.

## Ba lớp bỏ lọt cụ thể (đều đạt tới được từ Grit, đều check XANH trước quy tắc)
- **Precondition `n < 32`** (`shl`/`shr`/`rotl`/`rotr` qua con trỏ): check xanh; thông dịch báo ICE "n ≥ 32 không có Assert"; LLVM từ chối. Không sinh mã sai nhưng check nói sai.
- **Hiệu ứng** (FB-15, mọi hàm kết hợp): hố THẬT, đã sửa riêng ở gt-types (hiệu ứng khai báo của thành viên nay đi theo kiểu hàm).
- **Nguồn capability** (`root_fs` làm giá trị trong `main`) — ĐO bằng feature đo (không còn "suy ra"): `ƒ h(g: ƒ() → Cap<Fs> ! ∅) ! {io} { c ≔ g(); x ≔ c.read("/etc/hostname"); }  ƒ main() ! {io} { h(root_fs); }` ⇒ check XANH khi tắt E0313: hàm KHÔNG phải `main` (`h`) đúc `Cap<Fs>` từ hư không qua tham số kiểu hàm (`check_caps` chỉ thấy đường dẫn `root_fs` ở nơi nó được NHẮC, không thấy lời gọi `g()`). LỖ HỔNG THẬT của điều kiện cứng #12 (FB-16), đã đóng bằng E0313; với E0313 bật: ĐỎ E0313.

## Quy tắc (Phase 1)
Mọi hàm của lib/core có thân prelude (không có MIR) ⇒ không dùng làm giá trị: M4 báo E0313 (`gt-types/src/check/call.rs::reject_intrinsic_value`), riêng `Atomic` giữ E0503 (gt-conc). Hàm Grit thường (kể cả hàm kết hợp của kiểu người dùng) dùng làm giá trị như trước. Gọi TRỰC TIẾP và cú pháp phương thức không đổi.

## IR Guard
G1 đã chặn địa chỉ `gritrt_*` ngoài vai trò callee trực tiếp. Mục 2d: `gt-driver/tests/intrinsic_value_codegen.rs` dựng `Const::FnItem` trỏ tới `shl` lọt xuống codegen ⇒ backend từ chối ("không có thân MIR") ở cả dev, release, release-o2, KHÔNG có exe. Giới hạn: kiểm bằng dữ liệu dựng trong bộ nhớ; đường thật bị M4 chặn trước.
