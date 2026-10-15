# TCB_REVIEW_GUIDE — hướng dẫn Giang duyệt các dòng intrinsic (cột "người duyệt" của `docs/TCB_INTRINSICS.md`)

Viết 2026-10-15 cho Giang. **Chỉ cần đọc kỹ nhóm CAO.** Nhóm TRUNG BÌNH đọc lướt, nhóm THẤP chỉ cần nhìn một dòng. Tôi KHÔNG đổi cột "người duyệt" (vẫn `CHỜ GIANG` ở cả 42 dòng của bảng gốc); tài liệu này chỉ giải thích để Giang quyết.

Mỗi mục: (i) làm gì · (ii) chạm vào gì · (iii) nếu hợp đồng SAI thì lỗi gì · (iv) đã kiểm bằng gì · (v) mức rủi ro. Số nhóm ("A", "B2"…) là nhóm của `TCB_INTRINSICS.md`. Mục "sắp thêm" (ADR-038) ở cuối; chưa có mã.

**Bằng chứng chung (áp dụng nhiều mục):** (E1) **so sánh ba nguồn** — mô hình độc lập của bộ sinh, thông dịch MIR, exe LLVM — trên các lô `llvm_diff_*` (hàng nghìn chương trình, ghi ở `docs/CODEGEN_DIFF_RESULTS.jsonl`, `RELEASE_DIFF_RESULTS.jsonl`, `RELEASE_O2_DIFF_RESULTS.jsonl`); (E2) **IR Guard** (`gt-guard`): bảng đóng `RUNTIME` 23 hàm, mọi thao tác nhạy cảm phải mang lớp nghĩa vụ (`E0310`/`E0405`/`E0501`/`E0502`/`E0503`), đột biến 34/34 bị bắt cho Guard-post; (E3) **sáu công cụ dogfood** (`examples/`) khớp bản tham chiếu Rust; (E4) **bẫy abort** đúng loại ở dev/release/release-o2 (`release_traps.rs`). Chưa có: ThreadSanitizer/loom cho đa luồng; đột biến toàn bộ phần hạ của `gt-codegen-llvm` (mới có đột biến cho Guard và hàm quyết định mới).

---
# NHÓM CAO — bộ nhớ, Arena, luồng, mutability bên trong (đọc kỹ)

### H1. `Arena::new()`, `Cap<Alloc>::arena()` (B2)
- (i) Tạo một vùng bộ nhớ bump (tay cầm số nguyên trỏ vào bảng toàn cục của runtime). (ii) Bảng tay cầm của `gt-rt`, bộ nhớ hệ điều hành. (iii) Bảng tay cầm tràn `i32` ⇒ abort; tay cầm giả mạo (không thể ngoài prelude) ⇒ truy cập vùng của arena khác. (iv) E1 (kịch bản `vec_*`, `arena_*`), abort `arena_exhausted` kiểm riêng (`GRITRT_ARENA_LIMIT`), E2 lớp `E0405`, M6 oracle động theo dõi Arena. (v) **CAO**.

### H2. `Arena::alloc<T>(&self, v: T) → &T` (B2) — **lời khẳng định tin cậy nhất của dự án**
- (i) Cấp một vùng MỚI cho `T`, ghi `v` vào và trả tham chiếu tới vùng đó. **Lưu ý đối chiếu lệnh Giang:** chữ ký hiện tại trả `&T` (chia sẻ), KHÔNG phải `&!T`; cùng một lời khẳng định áp dụng: "mọi vùng trả về rời nhau". (ii) Bộ nhớ trong chunk của arena. (iii) Nếu hai lần `alloc` trả vùng CHỒNG LẤN ⇒ ghi qua một tham chiếu làm hỏng giá trị của tham chiếu kia (hỏng bộ nhớ không phát hiện được bởi M6 vì M6 tin lời khẳng định này); `align` sai ⇒ truy cập lệch. (iv) E1, M6 oracle động theo dõi khoản vay/Arena (tôi CHƯA thấy cơ chế theo dõi tính rời nhau của các vùng do `alloc` cấp, cả ở oracle lẫn ở runtime), kiểm abort `arena_exhausted`; **CHƯA có kiểm động "không chồng lấn"** — ADR-038 §C.1b yêu cầu thêm trước khi cài Arena chia sẻ. (v) **CAO — lời khẳng định bộ nhớ quan trọng nhất.**

### H3. `Arena::reset(&!self)` (B2)
- (i) Thu hồi mọi vùng đã cấp (chunk đầu giữ lại). (ii) Chunk của arena. (iii) Nếu còn tham chiếu/Vec/lát sống tới nội dung ⇒ dùng-sau-giải-phóng. An toàn dựa vào `&!` (M6 chặn mọi khoản mượn còn sống: `E0405`). (iv) E0405 corpus + M6 oracle (kịch bản `arena_reset`), mutation M6. (v) **CAO**.

### H4. `Vec::new`, `Vec::push`, `Vec::reserve` (B3)
- (i) `new`: dựng Vec rỗng không cấp phát; `push`: tăng dung lượng (×2, tối thiểu 4) bằng khối MỚI trong Arena + `memcpy`, rồi ghi phần tử; `reserve`: cấp một lần. (ii) Bộ nhớ của Arena; ptr/len/cap của Vec. (iii) Tính sai `ncap·size` (tràn) ⇒ cấp thiếu rồi `memcpy`/ghi tràn bộ nhớ; `len ≤ cap` bị phá ⇒ ghi ngoài khối. (iv) Kiểm tràn trong chính intrinsic (abort `overflow`), E1 (`vec_ops`, `vec_index_oob`, `vec_arena_exhausted`, `elem_*`), E3 (`wordfreq`, `sortlines`), E0318 (T cần drop bị cấm). (v) **CAO**.

### H5. `Vec::as_slice` / `as_mut_slice` (B3), `Text::as_bytes` / `as_str` (B4), `File::bytes` (A)
- (i) Trả lát `{ptr, len}` nhìn vào bộ nhớ của Vec/Text/tệp đã đọc. (ii) Bộ nhớ Arena/heap rò rỉ có chủ đích. (iii) `len` sai ⇒ đọc/ghi ngoài vùng qua lát (truy cập phần tử có `Assert` biên nhưng `Assert` tin `len` của lát). (iv) E1, E3, kiểm biên `index_out_of_bounds`/`slice_range`. (v) **CAO** (độ dài của lát là gốc của mọi kiểm biên).

### H6. `Text::push_str/push_char/push_utf8/from_utf8` (B4), `str::slice` (B4)
- (i) Thêm byte vào Text (đảm bảo UTF-8), cắt `&str` ở biên ký tự. (ii) Bộ nhớ Arena. (iii) Bất biến UTF-8 sai ⇒ `&str` không hợp lệ ⇒ mã giả định UTF-8 (ví dụ `slice` cắt giữa ký tự) có thể đọc sai; tràn khi thêm ⇒ như Vec. (iv) E1 (`text_*`), E3 (`wordfreq`, 9 ca UTF-8 sai), `gritrt_utf8_valid` dùng `std::str::from_utf8`. (v) **CAO** (bộ nhớ + bất biến kiểu).

### H7. `Atomic<T>::{new,load,store,swap,fetch_add,fetch_sub,compare_exchange}` (B)
- (i) Thao tác nguyên tử thật với thứ tự bộ nhớ là hằng. (ii) Bộ nhớ dùng chung giữa luồng. (iii) Thứ tự bộ nhớ sai ⇒ đua dữ liệu/kết quả sai hiếm, rất khó tái hiện. (iv) corpus E0503 (thứ tự phải truy ngược về hằng), test `threads_ir_uses_real_atomics_with_requested_orderings`, `real_threads_atomic_and_static_mutex`; **KHÔNG có ThreadSanitizer/loom**. (v) **CAO**.

### H8. `Mutex<T,L>::{new,with}` (B), `spawn(f, a)` (B)
- (i) `with`: khoá SRWLOCK quanh một lời gọi; `spawn`: tạo luồng HĐH thật chạy `f(a)`. (ii) Luồng, khoá, hộp `{f,a}` cấp bằng `gritrt_alloc`. (iii) Khoá không giữ đúng ⇒ đua dữ liệu; thứ tự khoá sai ⇒ deadlock; `a: Send` bị phá ⇒ chia sẻ tham chiếu giữa luồng. (iv) corpus E0501/E0502, test cấu trúc `Send/Sync`, đột biến `send.rs`/`ordering.rs`, test luồng thật; **không có oracle động cho đua dữ liệu/deadlock**; thông dịch tuần tự KHÔNG phải bằng chứng. (v) **CAO**.

### H9. `Cell<T>::{new,get,set}` (B)
- (i) Ô ghi được qua `&self` (load/store thường). (ii) Bộ nhớ của ô. (iii) Nếu quy tắc "mọi vị trí region bên trong Cell bất biến" (M6) hỏng ⇒ nhét tham chiếu sống ngắn vào ô sống lâu ⇒ treo. (iv) `interior_mut.rs`, oracle `interior_mut` (tắt bản sửa ⇒ 53 âm tính giả/3 000), 238 đã giết; **điều kiện #9 (361/591/554) chờ Giang** (`MUTANT_ARGUMENTS.md`). (v) **CAO** (điều kiện cứng).

### H10. `Cap<Env>::arg(i)` (A)
- (i) Trả `&str` trỏ tới đối số dòng lệnh đã chuyển lossy sang UTF-8. (ii) Bộ nhớ sống tới hết tiến trình. (iii) Trả chuỗi không UTF-8 ⇒ vi phạm bất biến `&str`. (iv) 6 công cụ dogfood, test tham số lạ. (v) **CAO nhẹ** (là `&str` từ bên ngoài; đã chuyển lossy nên luôn hợp lệ).

---
# NHÓM TRUNG BÌNH — I/O và năng lực (đọc lướt)
### M1. `root_fs/net/env/clock/alloc/out()` → `Cap<K>` (A)
- (i) Nguồn năng lực gốc; giá trị zero. (ii) Không có gì thực (quyền = sở hữu + hiệu ứng `io`). (iii) Nếu hàm không phải `main` gọi được ⇒ phá mô hình capability (điều kiện #12). (iv) E0310 chặn, fixtures `E0310_*`, lớp E0310 của Guard. (v) **TRUNG BÌNH** (an ninh, không phải bộ nhớ).
### M2. `Cap<Fs>::read(p)`, `Cap<Fs>::open(p)`, `File::len` (A)
- (i) Đọc nguyên tệp vào bộ nhớ (rò rỉ có chủ đích); `open` trả `Result`. (ii) Tệp trên đĩa, heap. (iii) Đọc nửa chừng/lỗi bị bỏ qua ⇒ dữ liệu cắt cụt không báo; `read` abort `io_error`. (iv) 6 công cụ (lỗi: không tồn tại, quyền, đường dẫn). (v) **TRUNG BÌNH**.
### M3. `Cap<Out>::write_u64/write_str/write_bytes`, `Cap<ErrOut>::write_u64/write_str` (A)
- (i) Ghi stdout/stderr, flush ngay. (ii) Luồng ra chuẩn. (iii) Ghi thất bại ⇒ abort `io_error`; byte không biến đổi (kiểm byte-by-byte ở dogfood). (iv) tất cả công cụ dogfood (so từng byte). (v) **TRUNG BÌNH**.
### M4. `Cap<Env>::arg_count()` (A) — (i) số đối số; (ii) tiến trình; (iii) sai ⇒ `arg(i)` abort `arg_out_of_range`; (iv) dogfood; (v) **TRUNG BÌNH nhẹ**.
### M5. `core::exit(code)` (A) — (i) thoát có kiểm soát sau flush stdout; (ii) tiến trình; (iii) mất dữ liệu nếu không flush; (iv) dogfood (mã 2); (v) **TRUNG BÌNH nhẹ**.

---
# NHÓM THẤP — tính toán thuần (nhìn một dòng)
### L1. `band/bor/bxor/bnot`, `shl/shr/rotr/rotl` (A) — lệnh LLVM thuần; `shl/shr` kiểm `n < 32` (abort `precondition`) để không sinh poison; kiểm bằng `llvm_diff_bits` ba nguồn (hàng trăm biểu thức ngẫu nhiên). **THẤP**.
### L2. `core::fuel_left(n)` (A), `<[T]>::len`, `core::abort` (B), `Vec::len`, `Text::len`, `str::len` (B3/B4) — `Assert`/đọc trường/abort thuần. **THẤP**.
### L3. `x as f64` (≤ 32 bit và 64 bit có kiểm 2^53), `f64::sqrt/to_i64/to_bits/f64_from_bits/is_nan` (B5) — lệnh `sitofp/sqrt/fptosi/bitcast/fcmp` thuần; `to_i64` chỉ chạy `fptosi` trong miền; kiểm: bộ sinh f64 100 chương trình + 30 bẫy IEEE + bẫy FMA (release/release-o2), đột biến 23/27 bắt (3 tương đương có lập luận). **THẤP**.

---
# SẮP THÊM (ADR-038, chưa có mã) — dự kiến mức rủi ro
- `Vec::pop/clear/truncate/extend_from_slice/insert/remove/swap_remove` (7): **CAO** (bộ nhớ như `push`). `str::as_bytes`: **THẤP** (cùng con trỏ béo). `root_in`, `Cap<In>::read`: **TRUNG BÌNH** (ghi vào bộ đệm do người gọi cấp, ≤ len); `Cap<Fs>::write`: **TRUNG BÌNH**. Arena chia sẻ (đổi `Vec<'a,T>` giữ `&'a Arena`): đổi hợp đồng của H1–H6, **CAO**.
