# Intrinsic-as-value investigation records

## Existing Windows measurements

## Intrinsic / lang item dùng làm GIÁ TRỊ hàm — bảng thực nghiệm (2026-10-17, mục 2 lệnh Giang)

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

## Imported Linux measurements from Claude branch

## INTRINSIC / HÀM LÀM GIÁ TRỊ — bảng ĐO (2026-10-21, nền tảng: Linux x86_64, thông dịch `gritc check`)

Câu hỏi: khi một hàm (intrinsic của prelude, hàm kết hợp `Kiểu::hàm`, hàm tự do, hàm có `⊳`) được dùng làm **giá trị** (`f ≔ g;`, truyền làm đối số) rồi gọi qua giá trị, các kiểm tra gắn với CHỮ KÝ của nó (capability D28/D29, tiền điều kiện `⊳`, hiệu ứng, vùng mượn) còn hiệu lực không?

Cách đo: `python3 tools/intrinsic_value_probe.py "<nhãn>=<gritc>" ...` (mỗi probe là một chương trình nhỏ, tập mã lấy từ `gritc check --json`). Nhãn **ĐO** = do chạy thật; **SUY RA** = chỉ đọc mã. Cột "trước F-05" là gritc dựng từ commit `05b4488` (trước sửa F-05 của kiểm toán ngoài) — ĐO bằng bản dựng riêng, không phải suy ra.

| probe | ý nghĩa | trước F-05 (05b4488) | HEAD 4178c51 |
|---|---|---|---|
| `cap_source_called_in_main_via_value` | main tự đặt nguồn capability vào biến rồi gọi (main được phép) | green [] | green [] |
| `cap_source_passed_to_helper_which_calls` | main đưa nguồn capability làm giá trị cho hàm khác GỌI nó (đúc capability ngoài main) | green [] | red ['E0310'] |
| `cap_source_in_nonmain_local` | hàm không phải main đặt nguồn capability vào biến rồi gọi | red ['E0310'] | red ['E0310'] |
| `cap_param_value_ok` | hàm giá trị nhận Cap qua tham số (không phải nguồn) | green [] | green [] |
| `pre_shl_direct_violated` | đối chứng: gọi TRỰC TIẾP shl với n=40 vi phạm ⊳ n < 32 | red ['E0901'] | red ['E0901'] |
| `pre_shl_value_violated` | gọi shl QUA GIÁ TRỊ hàm với n=40 (⊳ n < 32 có còn được kiểm?) | green [] | green [] |
| `pre_rotr_value_violated` | như trên với rotr | green [] | green [] |
| `pre_user_fn_value_violated` | hàm người dùng có ⊳ gọi qua giá trị với đối số vi phạm | green [] | green [] |
| `pre_option_unwrap_value_none` | Option::unwrap (⊳ self.is_some()) qua giá trị với None | green [] | green [] |
| `effect_io_into_pure_param` | hàm có io truyền vào tham số ƒ(u64) ! ∅ | red ['E0301'] | red ['E0301'] |
| `effect_div_into_pure_param` | hàm có div (core::abort) truyền vào tham số ƒ(&str) ! ∅ | red ['E0301'] | red ['E0301'] |
| `effect_mem_into_pure_param` | hàm có mem truyền vào tham số thuần | green [] | green [] |
| `effect_subset_ok` | đối chứng: hàm thuần truyền vào tham số cho phép io | red ['E0301'] | red ['E0301'] |
| `spawn_as_value` | spawn làm giá trị | red ['E1002'] | red ['E1002'] |
| `arena_new_value_ok` | Arena::new làm giá trị rồi gọi trong hàm khai báo mem | green [] | green [] |
| `assoc_fn_value_mem_into_pure_param` | hàm kết hợp Arena::new ({mem}) truyền vào tham số ƒ() → Arena ! ∅ | green [] | green [] |
| `assoc_fn_value_io_local` | hàm kết hợp người dùng có {io} đặt vào biến rồi gọi trong hàm thuần | green [] | green [] |
| `assoc_fn_value_io_into_pure_param` | hàm kết hợp có {io} truyền vào tham số thuần | green [] | green [] |
| `free_fn_value_io_local` | đối chứng: hàm TỰ DO có {io} đặt vào biến rồi gọi trong hàm thuần | red ['E0801'] | red ['E0801'] |
| `assoc_fn_direct_io_in_pure` | đối chứng: gọi TRỰC TIẾP hàm kết hợp có {io} trong hàm thuần | red ['E0801'] | red ['E0801'] |
| `ref_fn_value_two_refs` | hàm hai tham chiếu trả một trong hai, gọi qua giá trị rồi dùng kết quả sau khi nguồn chết | red ['E0301'] | red ['E0301'] |

## Đọc bảng (tất cả ĐO, trừ khi nói khác)
- **Capability (điều kiện #12)**: trước F-05, `helper(root_out)` rồi `helper` gọi `mk()` là XANH — hàm không phải `main` đúc được `Cap<Out>` tuỳ ý (LỖ HỔNG THẬT, F-05). Sau F-05 (`gt-types/src/check/call.rs`: lời gọi qua giá trị hàm ghi `CapUse`) là E0310. `main` tự đặt nguồn capability vào biến rồi gọi vẫn được phép (D29: chỉ `main` gọi nguồn gốc). Hàm giá trị nhận `Cap` qua tham số là hợp lệ. **Điều kiện #12 đạt cho đường này** (nhưng xem hiệu ứng dưới: lớp lỗi cùng họ còn mở ở hàm kết hợp).
- **Tiền điều kiện `⊳` (ĐÃ SỬA ở S1 — FB-C6; mô tả LỖI trước sửa)**: `shl(1, 40)` gọi TRỰC TIẾP là E0901, nhưng QUA GIÁ TRỊ (`f ≔ shl; f(1, 40)`) là XANH; cùng cho `rotr`, hàm người dùng có `⊳`, và `Option::unwrap` (`⊳ self.is_some()`). ⊳ chỉ được kiểm ở chỗ gọi TRỰC TIẾP; qua giá trị hàm nghĩa vụ không được sinh ⇒ false negative của kiểm tra an toàn (mức nghiêm trọng nhất theo grit-contract §3.3). Hành vi lúc chạy khi vi phạm: xem FB-C6 sau khi sửa.
- **Hiệu ứng qua kiểu hàm**: hàm TỰ DO có `{io}`/`{div}` truyền vào tham số `ƒ(…) ! ∅` hoặc gọi qua biến trong hàm thuần: ĐỎ (E0301/E0801) — đúng. Nhưng **hàm KẾT HỢP (`Kiểu::hàm`) dùng làm giá trị MẤT hiệu ứng** (`call.rs`: nhánh `Ty::Fn(ps, ret, Vec::new())` cho đường dẫn `Kiểu::hàm`): `Arena::new` (`{mem}`) và `S::mk` (`{io}`) qua giá trị vào hàm thuần là XANH, trong khi gọi trực tiếp là E0801. LỖ HỔNG THẬT (S1.iii) ⇒ FB-C5 (ĐÃ SỬA ở S1).
- **Vùng mượn qua giá trị hàm (S1.i)**: probe `ref_fn_value_two_refs` của bảng trên bị E0301 ở bước kiểu (cú pháp probe sai — không dùng được làm bằng chứng). Đo lại bằng chương trình đúng cú pháp ở mục "Sau S1" (4 mẫu fail E0404/E0405, 3 mẫu pass) và bằng bộ sinh `fn_value_ref`.
- Hiệu ứng là bất biến (không có subtyping): hàm thuần KHÔNG truyền được vào tham số `ƒ(…) ! {io}` (E0301) — bảo thủ, an toàn, hơi bất tiện; ghi vào SEED_LIMITATIONS.
- `spawn` làm giá trị: bị E1002 (đúng — chưa hỗ trợ).

## Sau S1 (ĐO trên HEAD, Linux)
Cột dưới là kết quả của CÙNG bộ probe sau khi sửa FB-C5 (hiệu ứng hàm kết hợp làm giá trị: `Pick.effects` → `Ty::Fn`) và FB-C6 (`⊳` + giá trị hàm ⇒ E0901 ở `gt-oblig/src/driver.rs`):

| probe | ý nghĩa | HEAD 53bfb4d (sau S1) |
|---|---|---|
| `cap_source_called_in_main_via_value` | main tự đặt nguồn capability vào biến rồi gọi (main được phép) | green [] |
| `cap_source_passed_to_helper_which_calls` | main đưa nguồn capability làm giá trị cho hàm khác GỌI nó (đúc capability ngoài main) | red ['E0310'] |
| `cap_source_in_nonmain_local` | hàm không phải main đặt nguồn capability vào biến rồi gọi | red ['E0310'] |
| `cap_param_value_ok` | hàm giá trị nhận Cap qua tham số (không phải nguồn) | green [] |
| `pre_shl_direct_violated` | đối chứng: gọi TRỰC TIẾP shl với n=40 vi phạm ⊳ n < 32 | red ['E0901'] |
| `pre_shl_value_violated` | gọi shl QUA GIÁ TRỊ hàm với n=40 (⊳ n < 32 có còn được kiểm?) | red ['E0901'] |
| `pre_rotr_value_violated` | như trên với rotr | red ['E0901'] |
| `pre_user_fn_value_violated` | hàm người dùng có ⊳ gọi qua giá trị với đối số vi phạm | red ['E0901'] |
| `pre_option_unwrap_value_none` | Option::unwrap (⊳ self.is_some()) qua giá trị với None | red ['E0901'] |
| `effect_io_into_pure_param` | hàm có io truyền vào tham số ƒ(u64) ! ∅ | red ['E0301'] |
| `effect_div_into_pure_param` | hàm có div (core::abort) truyền vào tham số ƒ(&str) ! ∅ | red ['E0301'] |
| `effect_mem_into_pure_param` | hàm có mem truyền vào tham số thuần | red ['E0301'] |
| `effect_subset_ok` | đối chứng: hàm thuần truyền vào tham số cho phép io | red ['E0301'] |
| `spawn_as_value` | spawn làm giá trị | red ['E1002'] |
| `arena_new_value_ok` | Arena::new làm giá trị rồi gọi trong hàm khai báo mem | green [] |
| `assoc_fn_value_mem_into_pure_param` | hàm kết hợp Arena::new ({mem}) truyền vào tham số ƒ() → Arena ! ∅ | red ['E0301'] |
| `assoc_fn_value_io_local` | hàm kết hợp người dùng có {io} đặt vào biến rồi gọi trong hàm thuần | red ['E0801'] |
| `assoc_fn_value_io_into_pure_param` | hàm kết hợp có {io} truyền vào tham số thuần | red ['E0301'] |
| `free_fn_value_io_local` | đối chứng: hàm TỰ DO có {io} đặt vào biến rồi gọi trong hàm thuần | red ['E0801'] |
| `assoc_fn_direct_io_in_pure` | đối chứng: gọi TRỰC TIẾP hàm kết hợp có {io} trong hàm thuần | red ['E0801'] |
| `ref_fn_value_two_refs` | hàm hai tham chiếu trả một trong hai, gọi qua giá trị rồi dùng kết quả sau khi nguồn chết | red ['E0301'] |

Ghi chú sau S1:
- Mọi probe "chương trình sai" ở trên giờ ĐỎ; các đối chứng hợp lệ (`cap_param_value_ok`, `arena_new_value_ok`, `free`/`assoc` trong hàm khai báo hiệu ứng) vẫn XANH. `effect_subset_ok` (hàm thuần truyền vào tham số cho phép `io`) vẫn bị E0301: hiệu ứng là bất biến (không có subtyping) — bảo thủ, an toàn (SEED_LIMITATIONS).
- M6 qua giá trị hàm có tham chiếu: kiểu hàm không mang region nên M6 dùng đường BẢO THỦ cho `Callee::Value` (mọi region đầu vào outlive nhau và outlive đích — `gt-borrowck/src/regions.rs`, `call`). Khoá bằng 4 mẫu fail (`E0404_fn_value_*`, `E0405_fn_value_arena_reset`), 3 mẫu pass (`207…209_bc_fn_value_*`) và kịch bản `fn_value_ref` của bộ sinh `progen` (20 000 chương trình: 1 532 có kịch bản, 734 chấp nhận+sạch, 764 oracle thấy vi phạm). KIỂM TẮT-BẢN-SỬA: đổi `Callee::Value(_) => false` thành `true` (không ràng buộc) ⇒ oracle ĐỎ (`#213`, `#367` … "borrowck CHẤP NHẬN nhưng oracle thấy vi phạm", cả `semantic_attacks`), hoàn nguyên ⇒ xanh.
