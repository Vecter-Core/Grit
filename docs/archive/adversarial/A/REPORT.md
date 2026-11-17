# Báo cáo đọc đối kháng nhóm A (Arena, Vec, Text, lát/&str, File, str::as_bytes, Vec::truncate)

Công cụ: /tmp/gritbin/gritc, chương trình tái hiện ở /tmp/eprompt/A/*.gt. Mã nguồn không bị sửa.

## Bảng kết luận
| intrinsic | kết luận | bằng chứng tệp:dòng | chương trình tái hiện |
|---|---|---|---|
| Arena::new / Cap<Alloc>::arena | LOẠI (runtime) / liên quan lỗ THẬT-1 | lower.rs:2305-2316; gt-rt lib.rs:537-544 | — |
| Arena::alloc | codegen+runtime ĐÚNG hợp đồng (align luỹ thừa 2, size lớn ⇒ Layout lỗi ⇒ abort 17, saturating_add). Nhưng con trỏ bị dùng-sau-reset khi Arena sao chép được: **THẬT-1** | lower.rs:2317-2349; lib.rs:548-584; validate.rs:207-210 (thiếu `Lang::Arena`) | arena_copy_silent.gt, arena_copy.gt |
| Arena::reset | runtime đúng (giữ chunk đầu, dealloc đúng layout lib.rs:620-636). Bảo vệ chỉ dựa M6, M6 bị vô hiệu bởi THẬT-1/THẬT-2; thông dịch coi reset là no-op (interp.rs:843-849) nên không thấy UAF | lib.rs:620; interp.rs:843 | arena_copy.gt (SIGSEGV), cell_arena.gt |
| Vec::new | LOẠI: không cấp phát, bố cục {arena,null,0,0} | lower.rs:2539-2550 | slice_empty.gt |
| Vec::push | ĐÚNG ở các đường kiểm: tràn 2·cap, tràn n·size (umul.with.overflow), align theo T, memcpy len·size không tràn (len≤cap). Bất biến "T không alias" bị phá bởi THẬT-2 | lower.rs:3361-3422, 3039-3093 | reserve_zst.gt (ZST OK) |
| Vec::reserve | ĐÚNG: n·size=2^64 ⇒ abort overflow ở dev và release-o2; n=usize::MAX với ZST OK | lower.rs:3323-3356 | reserve_huge.gt, reserve_zst.gt |
| Vec::len / as_slice / as_mut_slice | ĐÚNG về đọc trường; hai `&![T]` cùng bộ nhớ khả thi qua THẬT-2 | lower.rs:2551-2566 | cell_vec.gt |
| Vec::truncate | LOẠI: len←min(n,len), ULT + select, không chạm ptr/cap; n=usize::MAX/0 đúng, codegen ≡ thông dịch | lower.rs:2603-2618; interp.rs:712-726 | trunc_big.gt |
| Text::new / len / as_str / as_bytes | LOẠI: as_str trên Text rỗng trả ptr null+len 0 (xem NGHI-1) | lower.rs:2445-2459, 2522-2537 | slice_empty.gt |
| Text::push_str / push_char / vec_extend | ĐÚNG: len+n kiểm tràn, 2·cap kiểm tràn, ncap=max(2cap,sum,16), memcpy không chồng (khối mới hoặc nguồn khác); mã hoá UTF-8 4 byte đúng cho scalar hợp lệ. `u32 as char` bị E0301, `'\u{D800}'` bị E0101 ⇒ char luôn hợp lệ | lower.rs:3135-3248 | char_cast.gt, char_cast2.gt, char_lit.gt (đều bị từ chối: tốt) |
| Text::push_utf8 / from_utf8 | LOẠI: gritrt_utf8_valid n=0 ⇒ 1 không đọc con trỏ; không hợp lệ ⇒ không thêm byte | lower.rs:2469-2521; lib.rs:388-394 | — |
| str::slice | ĐÚNG: from≤to≤n, hai đầu ở biên ký tự, idx==n bỏ qua load; gep inbounds null+0 hợp lệ; codegen ≡ interp (interp.rs:538-545) | lower.rs:3251-3320 | slice_empty.gt |
| str::len | LOẠI (trích trường 1 của con trỏ béo) | lower.rs:2707-2710 | — |
| str::as_bytes | LOẠI: đồng nhất `vals[0]`; interp sao chép sang mảng (khác biểu diễn, cùng kết quả đọc) | lower.rs:2602; interp.rs:823-842 | — |
| &str == / != | LOẠI: gritrt_bytes_eq độ dài khác ⇒ 0, n=0 ⇒ 1 không đọc | lower.rs:1619-1650; lib.rs:401-416 | — |
| File::len / File::bytes | ĐÚNG; tay cầm không giả mạo được ngoài prelude (literal `File{x:5}` và `f.x` bị E0203); file_at tay cầm sai/âm ⇒ abort 14 | lower.rs:2642-2658; lib.rs:209-215, 340-347 | forge_file.gt, f_field_file.gt |
| Cap<Fs>::read / open / write, Cap<In>::read_all | ĐÚNG trừ **NGHI-1** (from_raw_parts con trỏ null, n=0) và **NGHI-2** (lệch tài liệu) | lib.rs:249-337, 306-316 | null_path.gt |
| Cap<Env>::arg / arg_count | ĐÚNG: lossy UTF-8, vượt số đối số ⇒ abort 15, kiểu u64 khớp | lower.rs:2619-2631, 2659-2663; lib.rs:177-200 | — |
| Cap<Out>/<ErrOut> write_* | ĐÚNG; n=0 không dựng slice từ null | lib.rs:350-380, 421-430 | — |
| Literal/field/pattern giả mạo Vec, Text, Arena, File | LOẠI: literal Vec/Text/Cap/Secret ⇒ E0313; Arena/File ⇒ E0203 (trường private); pattern và gán trường ⇒ E0203 | expr.rs:668-680 | forge_arena.gt, f_*.gt |
| IR Guard (RUNTIME) | xem mục Guard | gt-guard lib.rs:84-112, 222-231 | — |

## Danh sách THẬT (theo mức nghiêm trọng)

### THẬT-1 (CAO, an toàn bộ nhớ): người dùng viết được `⊐ Copy for Arena { }` ⇒ sao chép tay cầm Arena ⇒ dùng-sau-reset / use-after-free
- Nguyên nhân: `validate_copy_impl` chỉ cấm `Lang::Secret | Cap | Vec | Text` (crates/gt-types/src/validate.rs:207-210). `Arena` (lang `arena`, prelude.gt:81, trường `x: i32` Copy) không bị cấm, nên `⊐ Copy for Arena {}` hợp lệ trong mã người dùng. Khi đó `b ≔ a;` không move; M6 không còn thấy `b.reset()` (`&!b`) xung đột với tham chiếu sinh từ `a` (hai biến khác tên cùng tay cầm).
- Tái hiện 1 (sai thầm lặng): /tmp/eprompt/A/arena_copy_silent.gt. `r ≔ a.alloc(111u64); b.reset(); b.alloc(222u64); *r` ⇒ main trả về **222**, ở dev, release và release-o2, check xanh, không có abort (kiểm động arena dev không bắt vì reset xoá `live`).
- Tái hiện 2 (UAF thật): /tmp/eprompt/A/arena_copy.gt: Vec 3 triệu u64 trên `a`, `b.reset()` giải phóng chunk, đọc lại ⇒ `gritc: lỗi nội bộ khi chạy: chương trình bị hệ điều hành dừng` (SIGSEGV, exit 2). Đối chứng không có reset (arena_copy_ctrl.gt) trả 2999999.
- Thông dịch không thấy: Arena::alloc không có thân MIR trong thông dịch (ICE "không có thân MIR cho Arena::alloc"), Arena::reset là no-op (interp.rs:843-849).
- Sửa gợi ý (không thực hiện): thêm `Lang::Arena` vào danh sách cấm Copy; xem rộng hơn THẬT-2.

### THẬT-2 (CAO): `Cell<T>::get(&self) → T` không đòi `T: Copy` ⇒ nhân bản bất kỳ giá trị không-Copy (Arena, Vec, Text, ...) mà KHÔNG cần impl Copy
- Nguyên nhân: prelude.gt:54 `ƒ get(&self) → T ! ∅ { }` trong `⊐ <T> Cell<T>` không có ràng buộc; codegen hạ thành load thường (docs TCB_INTRINSICS dòng Cell). Kiểm kiểu chấp nhận.
- Tái hiện 3: /tmp/eprompt/A/cell_arena.gt. `c ≔ Cell::new(Arena::new()); a ≔ c.get(); b ≔ c.get();` rồi `r ≔ a.alloc(111u64); b.reset(); b.alloc(222u64); *r` ⇒ **222** (cùng hậu quả THẬT-1, không cần `⊐ Copy`).
- Tái hiện 4: /tmp/eprompt/A/cell_vec.gt. Hai bản sao Vec cùng khối: `sl ≔ v1.as_slice(); v2.truncate(0); v2.push(99); sl[0]` ⇒ exe trả 99010 (lát chung `&[u64]` bị ghi đè qua bản sao), thông dịch trả 10010 (sao chép giá trị) ⇒ **codegen và thông dịch khác nhau**, bất biến "lát `&` bất biến suốt khoản mượn" và mọi chứng minh M7 về nội dung lát bị phá. Hai `&![T]` cùng bộ nhớ cũng dựng được. Cũng nhân bản được `Cap<..>` (ngoài nhóm A, chưa thử).
- Sửa gợi ý: `get` đòi `T: Copy`, hoặc cấm instantiate `Cell` với kiểu không Copy.

### Không phải lỗi nhưng cần biết
- Mọi bảo vệ chống UAF của Arena dựa hoàn toàn vào M6 (E0405) + tính tuyến tính của tay cầm; THẬT-1 và THẬT-2 chứng tỏ tính tuyến tính có lỗ ở tầng kiểu.

## NGHI (đọc thấy, không có hậu quả quan sát)
- NGHI-1 (THẤP): `gritrt_fs_read`, `gritrt_fs_open`, `gritrt_fs_write` (đường dẫn) dựng `slice::from_raw_parts(p, n)` KHÔNG bảo vệ n=0 (lib.rs:250, 275, 324) trong khi dữ liệu `d` và `write_str/ewrite_str/utf8_valid/bytes_eq` đều có bảo vệ. `Text::new(&a).as_str()` có ptr null, len 0 (lower.rs:2528-2536) ⇒ UB về mặt Rust. null_path.gt chạy bình thường (open/write ⇒ Err, read ⇒ abort io_error) vì runtime build release, không có kiểm tiền điều kiện debug. Nếu gt-rt dựng với debug-assertions, sẽ thành panic/abort không kiểm soát.
- NGHI-2 (THẤP, lệch tài liệu): TCB_INTRINSICS dòng `Cap<Fs>::open` nói "chỉ abort nếu bảng tay cầm tràn i32", nhưng lib.rs:284 và 315 trả mã -4 (Other) khi tràn; chỉ `read` abort (lib.rs:260). Tài liệu B4 còn nói "một Arena một Text (E0402)" trong khi R6 đã cho chia sẻ Arena.
- NGHI-3 (THẤP): `llvm.memcpy` được gọi với nguồn có thể null và n=0 (vec_grow_to lower.rs:3089 khi len=0, vec_extend lower.rs:3189 khi n=0 và src null). Hợp lệ theo LLVM hiện hành, nhưng không có kiểm tra Guard nào bảo đảm; chưa thử O2 với LLVM suy luận nonnull.
- NGHI-4 (THẤP): `vec_extend` giả định nguồn không chồng đích (memcpy, không memmove). Hiện chỉ đúng nhờ M6 chặn `t.push_str(t.as_str())`; chưa thử tìm đường vòng qua M6 (nhưng THẬT-2 cho phép nhân bản Text: bản sao A.as_str() rồi B.push_str chỉ nối đuôi, không chồng, nên không thấy hậu quả).

## IR Guard (câu hỏi 5)
- Bảng RUNTIME lib.rs:84-112 khớp danh sách hàm gt-rt; thiếu thì từ chối (fail-closed) đúng.
- Guard chỉ kiểm "thẻ lớp nghĩa vụ trên lệnh = lớp trong bảng" (lib.rs:222-231, 390-400), không kiểm ngữ nghĩa: không thể phát hiện Arena::alloc được gọi trên tay cầm bị sao chép (THẬT-1/2 là lỗi tầng kiểu, Guard mù). Chấp nhận theo thiết kế ("theo lớp, chưa theo điểm").
- `gritrt_utf8_valid` và `gritrt_bytes_eq` không cần lớp (None), nên Guard không bảo đảm `push_utf8` thực sự kiểm trước `vec_extend` (lower.rs:2469-2480); chỉ test vi sai utf8_diff đảm bảo.
- `call_basic` gắn E0310 cho mọi lời gọi runtime có giá trị trả về, gồm `gritrt_bytes_eq` (bảng ghi None): vô hại nhưng lệch hợp đồng. `llvm.*` không bị kiểm đối số (memcpy tuỳ ý).
- Không dựng được chương trình Grit làm Guard từ chối sai/đúng khác kỳ vọng; chưa thử đột biến codegen.

## Câu hỏi (1)-(4) tóm tắt
1. Độ dài 0, null, tràn, rất lớn, căn lề, hết bộ nhớ: reserve/push/extend/truncate/slice/alloc đều kiểm đúng (xem bảng). Hết bộ nhớ ⇒ abort 17 (Layout lỗi hoặc alloc null).
2. Ngoài vùng hợp đồng / sống lâu hơn bộ nhớ: chỉ qua THẬT-1/THẬT-2 (UAF sau reset). Arena chỉ giải phóng ở reset, nên không có UAF khác.
3. Kiểu đối số codegen ≡ runtime: i32 tay cầm, u64 kích thước/chỉ số, u32 code cho exit, con trỏ + u64 cho lát; khớp.
4. Kiểm chỉ ở thông dịch: Vec::reserve/push không mô phỏng tràn kích thước hay hết Arena (interp.rs:689, 728-738); Arena::reset no-op; Arena::alloc không có thân; Cell sao chép giá trị (khác exe). Kiểm chỉ ở codegen: tràn n·size, arena_exhausted, kiểm động dev.
6. Hợp đồng thiếu điều kiện: "tay cầm không giả mạo/không nhân bản" chỉ được bảo đảm cho Vec/Text/Cap/Secret (E0313), không cho Arena, và Cell::get bỏ qua toàn bộ.

## Phần chưa đọc / chưa kiểm (nói thẳng)
- Chưa đọc M6 (gt-borrowck) nên không biết liệu có đường khác (struct chứa `&Arena`, static Arena, thread spawn với Arena) phá bất biến; chưa thử Mutex<Arena>/Atomic Copy.
- Chưa đọc kiểm chặn chỉ số `Assert` M5/M7 của lát `v[i]`, `&a[i..j]`, và hạ Place/Index (lower.rs phần place); chưa đọc phần Atomic/Mutex/spawn/Cell hạ (chỉ phát hiện Cell::get qua hành vi).
- Chưa chạy các chương trình std (lib/std/src/*.gt) và chưa audit chúng.
- Chưa đọc hết interp.rs (chỉ các đoạn Vec/Text/str/Arena) và chưa đọc các test gt-driver (utf8_diff, vec_overflow, arena_dynamic).
- Chưa thử GRITRT_ARENA_LIMIT, GRITRT_ARENA_CHECK, align lớn, đa luồng với Arena, f64, và chưa chạy hồ sơ release-o2 cho từng nghi vấn NGHI.
- Chưa xác định gt-rt được dựng với debug-assertions hay không (ảnh hưởng NGHI-1).
