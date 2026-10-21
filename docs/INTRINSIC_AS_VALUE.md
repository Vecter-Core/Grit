# INTRINSIC / HÀM LÀM GIÁ TRỊ — bảng ĐO (2026-10-21, nền tảng: Linux x86_64, thông dịch `gritc check`)

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
- **Tiền điều kiện `⊳` (CÒN MỞ — S1.ii)**: `shl(1, 40)` gọi TRỰC TIẾP là E0901, nhưng QUA GIÁ TRỊ (`f ≔ shl; f(1, 40)`) là XANH; cùng cho `rotr`, hàm người dùng có `⊳`, và `Option::unwrap` (`⊳ self.is_some()`). ⊳ chỉ được kiểm ở chỗ gọi TRỰC TIẾP; qua giá trị hàm nghĩa vụ không được sinh ⇒ false negative của kiểm tra an toàn (mức nghiêm trọng nhất theo grit-contract §3.3). Hành vi lúc chạy khi vi phạm: xem FB-C6 sau khi sửa.
- **Hiệu ứng qua kiểu hàm**: hàm TỰ DO có `{io}`/`{div}` truyền vào tham số `ƒ(…) ! ∅` hoặc gọi qua biến trong hàm thuần: ĐỎ (E0301/E0801) — đúng. Nhưng **hàm KẾT HỢP (`Kiểu::hàm`) dùng làm giá trị MẤT hiệu ứng** (`call.rs`: nhánh `Ty::Fn(ps, ret, Vec::new())` cho đường dẫn `Kiểu::hàm`): `Arena::new` (`{mem}`) và `S::mk` (`{io}`) qua giá trị vào hàm thuần là XANH, trong khi gọi trực tiếp là E0801. LỖ HỔNG THẬT (S1.iii) ⇒ FB-C5.
- **Vùng mượn qua giá trị hàm (S1.i)**: probe `ref_fn_value_two_refs` bị E0301 ở bước kiểu (vì cú pháp/kiểu của probe), CHƯA đo được M6 qua giá trị hàm có tham chiếu; sẽ đo bằng probe viết lại ở S1.
- Hiệu ứng là bất biến (không có subtyping): hàm thuần KHÔNG truyền được vào tham số `ƒ(…) ! {io}` (E0301) — bảo thủ, an toàn, hơi bất tiện; ghi vào SEED_LIMITATIONS.
- `spawn` làm giá trị: bị E1002 (đúng — chưa hỗ trợ).
