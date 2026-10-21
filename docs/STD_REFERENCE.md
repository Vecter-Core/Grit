# STD_REFERENCE — tham chiếu thư viện chuẩn (SINH TỰ ĐỘNG)

> Tệp này được SINH bởi `python3 tools/gen_docs.py` từ `lib/core/prelude.gt` và `lib/std/src/std.gt`; test `std_reference_matches_generator` khoá từng byte.
> KHÔNG sửa tay: sửa nguồn (chữ ký, chú thích `//` ngay trên mục) hoặc bảng điều kiện abort trong `tools/gen_docs.py`, rồi sinh lại.

## Cách đọc
- Chữ ký: `ƒ tên(tham số) → kiểu trả về ! hiệu-ứng [⊳ tiền-điều-kiện]`. Hiệu ứng `∅` = thuần; `{io}` = I/O (cần capability); `{mem}` = cấp phát Arena; `{div}` = có thể không dừng/thoát.
- **abort**: cách chương trình dừng KHẨN CẤP (mã thoát 70) khi vi phạm — `overflow` (số học tràn), `bounds` (chỉ số/lát ngoài biên), `precondition` (`⊳` bị vi phạm), `slice_range`, `io_error`, `arg_out_of_range`, `fuel_exhausted`, `cast_lossy`. "không" = hàm toàn phần.
- Hàm **prelude** có sẵn không cần nhập. Hàm của gói **`std`** là gói Rig `lib/std`: `rig new` tự thêm; tên tự do phải gọi `std::tên` (`std::HashMap::new`, `std::fmt_u64`); phương thức trên kiểu có sẵn (`Vec`, `[u8]`, `str`, `Option`, …) dùng được ngay khi gói std có trong đồ thị.
- Mọi tham chiếu đến `Arena`: `Vec<'a, T>`/`Text<'a>` giữ `&'a Arena` CHIA SẺ; phần tử `Vec` phải không cần drop (`Copy`): E0318.
- Mã lỗi giải thích ở `docs/errors/` và `docs/spec/error_codes.md`.

## Prelude (lib/core/prelude.gt)

### Kiểu

- `⊔ Option<T> { None, Some(T) }`
- `⊔ Result<T, E> { Ok(T), Err(E) }`
- `▢ Range<T> { start: T, end: T }`
- `▢ RangeInclusive<T> { start: T, end: T }`
- `⊏ Copy { }`
- `⊏ Send { }`
- `⊏ Sync { }`
- `⊔ AtomicOrdering { Relaxed, Acquire, Release, AcqRel, SeqCst }`
- `⊔ Ordering { Less, Equal, Greater }` — Kết quả so sánh ba chiều (ADR-032, Q71): enum thay cho số để không có giá trị không hợp lệ. Enum nguyên tử là `AtomicOrdering`.
- `⊏ Ord { ƒ cmp(&self, o: &Self) → Ordering ! ∅; }` — Thứ tự toàn phần do NGƯỜI DÙNG cài (`⊐ Ord for X`); lib/core không cài sẵn cho kiểu nào (thân prelude không có MIR).
- `⊏ AtomicInt { }`
- `▢ Atomic<T: AtomicInt> { v: T }`
- `▢ Mutex<T, L> { v: T }`
- `▢ Cell<T> { v: T }`
- `⊏ Drop { ƒ drop(&!self) ! ∅; }`
- `▢ Buf { n: i32 }`
- `▢ Guard { ◈ inner: Buf, ◈ n: i32 }`
- `▢ Secret<T> { v: T }`
- `▢ Cap<T> { v: T }`
- `▢ Fs { x: i32 }`
- `▢ Net { x: i32 }`
- `▢ Env { x: i32 }`
- `▢ Clock { x: i32 }`
- `▢ Alloc { x: i32 }`
- `▢ Out { x: i32 }`
- `▢ In { x: i32 }` — stdin (Seed 1.0 G5): capability đọc đầu vào chuẩn; `Cap<In>::read_all` đọc NGUYÊN stdin tới EOF (lỗi ⇒ `Err`)
- `▢ ErrOut { x: i32 }` — stderr riêng. Tên `ErrOut` (không phải `Err`) vì `Err` là biến thể của `Result` và sẽ bị che/che khuất (xem DOGFOOD #11).
- `▢ Host { x: i32 }`
- `▢ Arena { x: i32 }`
- `▢ Vec<'a, T> { arena: &'a Arena, ptr: *μ T, len: usize, cap: usize }`
- `▢ File { x: i32 }`
- `⊔ IoError { NotFound, Denied, Invalid, Other }`
- `▢ Text<'a> { bytes: Vec<'a, u8> }`

### Hàm tự do

- `ƒ spawn<A: Send>(f: ƒ(A) ! {detach}, a: A) ! {detach}`
  - hiệu ứng: `{detach}`; abort: nếu không tạo được luồng; không có `join`; đối số phải `Send` (E0501)
- `ƒ mk_buf() → Buf ! ∅`
  - hiệu ứng: `∅`; abort: (chỉ cho test)
- `ƒ mk_guard() → Guard ! ∅`
  - hiệu ứng: `∅`; abort: (chỉ cho test)
- `ƒ shl(x: u32, n: u32) → u32 ! ∅ ⊳ n < 32`
  - hiệu ứng: `∅`; abort: `⊳ n < 32`: vi phạm ⇒ E0901 lúc biên dịch hoặc abort `precondition` lúc chạy
- `ƒ shr(x: u32, n: u32) → u32 ! ∅ ⊳ n < 32`
  - hiệu ứng: `∅`; abort: `⊳ n < 32`: vi phạm ⇒ E0901 lúc biên dịch hoặc abort `precondition` lúc chạy
- `ƒ band(a: u32, b: u32) → u32 ! ∅` — phép bit trên u32 (Q9: là HÀM của lib/core, không phải toán tử): AND/OR/XOR/NOT, xoay phải/trái; xoay và dịch đòi n < 32 (⊳)
  - hiệu ứng: `∅`; abort: không
- `ƒ bor(a: u32, b: u32) → u32 ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ bxor(a: u32, b: u32) → u32 ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ bnot(a: u32) → u32 ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ rotr(x: u32, n: u32) → u32 ! ∅ ⊳ n < 32`
  - hiệu ứng: `∅`; abort: `⊳ n < 32`: vi phạm ⇒ E0901 lúc biên dịch hoặc abort `precondition` lúc chạy
- `ƒ rotl(x: u32, n: u32) → u32 ! ∅ ⊳ n < 32`
  - hiệu ứng: `∅`; abort: `⊳ n < 32`: vi phạm ⇒ E0901 lúc biên dịch hoặc abort `precondition` lúc chạy
- `ƒ root_fs() → Cap<Fs> ! ∅`
  - hiệu ứng: `∅`; abort: không (chỉ `main` được gọi: E0310)
- `ƒ root_net() → Cap<Net> ! ∅`
  - hiệu ứng: `∅`; abort: không (chỉ `main`)
- `ƒ root_env() → Cap<Env> ! ∅`
  - hiệu ứng: `∅`; abort: không (chỉ `main`)
- `ƒ root_clock() → Cap<Clock> ! ∅`
  - hiệu ứng: `∅`; abort: không (chỉ `main`)
- `ƒ root_alloc() → Cap<Alloc> ! ∅`
  - hiệu ứng: `∅`; abort: không (chỉ `main`)
- `ƒ root_out() → Cap<Out> ! ∅`
  - hiệu ứng: `∅`; abort: không (chỉ `main`)
- `ƒ root_in() → Cap<In> ! ∅`
  - hiệu ứng: `∅`; abort: không (chỉ `main`)
- `ƒ root_err() → Cap<ErrOut> ! ∅`
  - hiệu ứng: `∅`; abort: không (chỉ `main`)
- `ƒ raw_read(p: usize) → i32 ! ∅`
  - hiệu ứng: `∅`; abort: (chỉ cho thử nghiệm bộ nhớ thô)

### `⊐ <T: AtomicInt> Atomic<T>`

- `ƒ new(v: T) → Atomic<T> ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ load(&self, o: AtomicOrdering) → T ! ∅`
  - hiệu ứng: `∅`; abort: không (thứ tự bộ nhớ phải là hằng tại chỗ gọi, E0503)
- `ƒ store(&self, v: T, o: AtomicOrdering) ! ∅`
  - hiệu ứng: `∅`; abort: không (thứ tự là hằng, E0503)
- `ƒ swap(&self, v: T, o: AtomicOrdering) → T ! ∅`
  - hiệu ứng: `∅`; abort: không (thứ tự là hằng, E0503)
- `ƒ fetch_add(&self, v: T, o: AtomicOrdering) → T ! ∅`
  - hiệu ứng: `∅`; abort: không (thứ tự là hằng, E0503)
- `ƒ fetch_sub(&self, v: T, o: AtomicOrdering) → T ! ∅`
  - hiệu ứng: `∅`; abort: không (thứ tự là hằng, E0503)
- `ƒ compare_exchange(&self, cur: T, new: T, s: AtomicOrdering, f: AtomicOrdering) → Result<T, T> ! ∅`
  - hiệu ứng: `∅`; abort: không (thứ tự là hằng, E0503)

### `⊐ <T, L> Mutex<T, L>`

- `ƒ new(v: T) → Mutex<T, L> ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ with<R: Send, Arg: Send>(&self, f: ƒ(&!T, Arg) → R ! {poly}, a: Arg) → R ! {poly, lock(L)}`
  - hiệu ứng: `{poly, lock(L)}`; abort: không (khoá giữ suốt lời gọi `f`; thứ tự khoá kiểm tĩnh E0502)

### `⊐ <T> Cell<T>`

- `ƒ new(v: T) → Cell<T> ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ set(&self, x: T) ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ get(&self) → T ! ∅`
  - hiệu ứng: `∅`; abort: không

### `⊐ File`

- `ƒ len(&self) → usize ! ∅`
  - hiệu ứng: `∅`; abort: không (tay cầm do `read`/`open` tạo luôn hợp lệ)
- `ƒ bytes(&self) → &'static [u8] ! ∅` — nội dung tệp: bộ nhớ RÒ RỈ CÓ CHỦ ĐÍCH sống tới hết tiến trình (gt-rt) nên là `'static` — có thể lưu trong cấu trúc dài hơn `File` (Seed 1.0 G10)
  - hiệu ứng: `∅`; abort: không

### `⊐ <T> Option<T>`

- `ƒ is_some(&self) → bool ! ∅`
  - hiệu ứng: `∅`; abort: không chạy được (chỉ là khai báo): dùng `⌥` hoặc `is_none` của std
- `ƒ unwrap(self) → T ! ∅ ⊳ self.is_some()`
  - hiệu ứng: `∅`; abort: không chạy được trong exe/thông dịch (chỉ là khai báo hợp đồng `⊳ self.is_some()`): dùng `⌥` hoặc `unwrap_or`

### `⊐ Cap<Fs>`

- `ƒ read(&self, p: &str) → File ! {io}`
  - hiệu ứng: `{io}`; abort: `io_error` nếu không đọc được (dùng `open` để nhận `Result`)
- `ƒ open(&self, p: &str) → Result<File, IoError> ! {io}`
  - hiệu ứng: `{io}`; abort: không (lỗi I/O trả `Err(IoError)`)
- `ƒ sub(&self, p: &str) → Cap<Fs> ! {io}`
  - hiệu ứng: `{io}`; abort: không
- `ƒ write(&self, p: &str, b: &[u8]) → Result<(), IoError> ! {io}` — ghi NGUYÊN tệp (tạo/cắt cụt); lỗi ⇒ `Err` (Seed 1.0 G6)
  - hiệu ứng: `{io}`; abort: không (lỗi I/O trả `Err(IoError)`)

### `⊐ Cap<Env>`

- `ƒ var(&self, n: &str) → usize ! {io}`
  - hiệu ứng: `{io}`; abort: xem docs/TCB_INTRINSICS.md (giá trị trả về là chỉ số, chưa dùng trong Seed 1.0)
- `ƒ arg(&self, i: usize) → &str ! {io}`
  - hiệu ứng: `{io}`; abort: `arg_out_of_range` nếu `i ≥ arg_count()`
- `ƒ arg_count(&self) → usize ! {io}`
  - hiệu ứng: `{io}`; abort: không

### `⊐ Cap<Out>`

- `ƒ write_u64(&self, n: u64) ! {io}`
  - hiệu ứng: `{io}`; abort: `io_error` nếu ghi stdout thất bại
- `ƒ write_str(&self, s: &str) ! {io}`
  - hiệu ứng: `{io}`; abort: `io_error` nếu ghi stdout thất bại
- `ƒ write_bytes(&self, b: &[u8]) ! {io}`
  - hiệu ứng: `{io}`; abort: `io_error` nếu ghi stdout thất bại

### `⊐ Cap<In>`

- `ƒ read_all(&self) → Result<File, IoError> ! {io}`
  - hiệu ứng: `{io}`; abort: không (lỗi I/O trả `Err(IoError)`)

### `⊐ Cap<ErrOut>`

- `ƒ write_u64(&self, n: u64) ! {io}`
  - hiệu ứng: `{io}`; abort: không (lỗi ghi stderr bị bỏ qua)
- `ƒ write_str(&self, s: &str) ! {io}`
  - hiệu ứng: `{io}`; abort: không (lỗi ghi stderr bị bỏ qua)

### `⊐ <'a, T> Vec<'a, T>`

- `ƒ new(a: &'a Arena) → Vec<'a, T> ! {mem}`
  - hiệu ứng: `{mem}`; abort: không (không cấp phát)
- `ƒ push(&!self, x: T) ! {mem}`
  - hiệu ứng: `{mem}`; abort: `overflow` nếu `len + 1` hoặc dung lượng tràn; abort nếu cấp phát arena thất bại
- `ƒ reserve(&!self, n: usize) ! {mem}`
  - hiệu ứng: `{mem}`; abort: `overflow` nếu dung lượng tràn; abort nếu cấp phát thất bại
- `ƒ len(&self) → usize ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ as_slice(&self) → &[T] ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ as_mut_slice(&!self) → &![T] ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ truncate(&!self, n: usize) ! ∅` — cắt bớt còn `min(n, len)` phần tử (không bao giờ tăng; T không cần drop nên không chạy destructor) — Seed 1.0 G3
  - hiệu ứng: `∅`; abort: không (cắt còn `min(n, len)`, không bao giờ tăng)

### `⊐ <'a> Text<'a>`

- `ƒ new(a: &'a Arena) → Text<'a> ! {mem}`
  - hiệu ứng: `{mem}`; abort: không
- `ƒ push_str(&!self, s: &str) ! {mem}`
  - hiệu ứng: `{mem}`; abort: `overflow`/cấp phát thất bại như `Vec::push`
- `ƒ push_char(&!self, c: char) ! {mem}`
  - hiệu ứng: `{mem}`; abort: `overflow`/cấp phát thất bại như `Vec::push`
- `ƒ push_utf8(&!self, b: &[u8]) → bool ! {mem}`
  - hiệu ứng: `{mem}`; abort: như `push_str`; trả `false` (không ghi gì) nếu `b` không phải UTF-8 hợp lệ
- `ƒ from_utf8(a: &'a Arena, b: &[u8]) → Option<Text<'a>> ! {mem}`
  - hiệu ứng: `{mem}`; abort: trả `None` nếu không phải UTF-8 hợp lệ; abort nếu cấp phát thất bại
- `ƒ len(&self) → usize ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ as_str(&self) → &str ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ as_bytes(&self) → &[u8] ! ∅`
  - hiệu ứng: `∅`; abort: không

### `⊐ f64`

- `ƒ sqrt(self) → f64 ! ∅`
  - hiệu ứng: `∅`; abort: không (NaN cho số âm)
- `ƒ to_i64(self) → Option<i64> ! ∅`
  - hiệu ứng: `∅`; abort: không (trả `None` nếu NaN/∞/ngoài miền)
- `ƒ to_bits(self) → u64 ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ is_nan(self) → bool ! ∅`
  - hiệu ứng: `∅`; abort: không

### `⊐ str`

- `ƒ as_bytes(&self) → &[u8] ! ∅` — khung nhìn byte của chuỗi (cùng bộ nhớ, không sao chép) — Seed 1.0 G4
  - hiệu ứng: `∅`; abort: không
- `ƒ len(&self) → usize ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ slice(&self, from: usize, to: usize) → &str ! ∅`
  - hiệu ứng: `∅`; abort: `slice_range` nếu `from > to`, `to > len` hoặc một đầu nằm giữa ký tự nhiều byte

### `▦ core`

- `ƒ abort(msg: &str) → ! ! {div}`
  - hiệu ứng: `{div}`; abort: luôn abort (mã 70) kèm thông điệp
- `ƒ exit(code: u32) → ! ! {div}`
  - hiệu ứng: `{div}`; abort: không bao giờ trả về; thoát CÓ KIỂM SOÁT với `min(code, 255)` (không phải abort)
- `ƒ f64_from_bits(b: u64) → f64 ! ∅` — bitcast u64 → f64 (mẫu bit IEEE-754; mọi mẫu hợp lệ, kể cả NaN/±∞/dưới chuẩn)
  - hiệu ứng: `∅`; abort: không
- `ƒ fuel_left(n: u64) ! ∅` — nhiên liệu của vòng lặp: gọi SAU vòng lặp `⟳ fuel > 0 … ↓ fuel`; `fuel == 0` (cạn) ⇒ abort `fuel_exhausted` (check lúc chạy, M7 có thể xoá, ghi chú N0905)
  - hiệu ứng: `∅`; abort: `fuel_exhausted` nếu `n == 0` (M7 xoá nếu chứng minh được)
- `ƒ hidden() ! ∅` — KHÔNG ◈: hàm riêng của module, chỉ để test E0203 (quy tắc nhìn thấy) của gt-resolve dùng chính prelude thật
  - hiệu ứng: `∅`; abort: (chỉ để test)

## Gói `std` (lib/std/src/std.gt)

Mặc định cho hàm không có ghi chú riêng: TOÀN PHẦN theo thiết kế: mọi chỉ số/tràn trong thân đều được bảo vệ logic; kiểm tra lúc chạy còn lại (ghi chú N0601/N0602 khi biên dịch gói std) chỉ là lưới an toàn — chạm vào nó nghĩa là lỗi của std (hãy báo). Hàm cần Arena có thể abort khi cấp phát thất bại.

### Kiểu

- `◈ ⊔ ParseError { Empty, Invalid, Overflow }` — lỗi phân tích số nguyên: Empty (không có ký tự), Invalid (có ký tự không phải chữ số), Overflow (ngoài miền kiểu). Ưu tiên: Empty > Invalid > Overflow.
- `◈ ⊏ Hash { ƒ hash(&self) → u64 ! ∅; }`
- `◈ ▢ Bytes<'a> { ◈ b: &'a [u8] }` — khoá/giá trị là lát byte (vd. nội dung `File::bytes()` hoặc một `&[u8]` nào đó): dùng được làm khoá của HashMap, so theo thứ tự từ điển byte
- `◈ ▢ HashMap<'a, K, V> { slots: Vec<'a, usize>, ents: Vec<'a, Ent<K, V>>, live: usize }`

### Hàm tự do

- `ƒ min<T: Ord + Copy>(a: T, b: T) → T ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ max<T: Ord + Copy>(a: T, b: T) → T ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ is_ascii_ws(c: u8) → bool ! ∅` — ký tự khoảng trắng ASCII: ` `, `\t`, `\n`, `\r`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ fmt_u64(buf_: &![u8; 24], n: u64) → usize ! ∅` — các chữ số thập phân của `n` căn PHẢI trong `buf_` (24 byte); trả chỉ số bắt đầu (0 ở đầu không in; n = 0 ⇒ một chữ số `0`)
  - hiệu ứng: `∅`; abort: không (`buf` 24 byte đủ cho mọi u64)
- `ƒ fmt_i64(buf_: &![u8; 24], n: i64) → usize ! ∅` — như `fmt_u64` nhưng có dấu `-` cho số âm (`i64::MIN` đúng)
  - hiệu ứng: `∅`; abort: không
- `ƒ fmt_hex(buf_: &![u8; 24], n: u64) → usize ! ∅` — 16 chữ số hex (chữ thường) của `n` căn phải trong `buf_`; trả chỉ số bắt đầu (n = 0 ⇒ một chữ số `0`)
  - hiệu ứng: `∅`; abort: không
- `ƒ abs_i64(x: i64) → Option<i64> ! ∅` — số học kiểm tràn không abort
  - hiệu ứng: `∅`; abort: không (trả `None` cho `i64::MIN`)
- `ƒ pow_u64(base_: u64, exp_: u32) → Option<u64> ! ∅`
  - hiệu ứng: `∅`; abort: không (trả `None` khi tràn)
- `ƒ hash_bytes(b: &[u8]) → u64 ! ∅` — băm đa thức cuộn trên byte (không có xor u64 ở Phase 1) + trộn cuối
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)

### `⊐ Ordering`

- `ƒ reverse(self) → Ordering ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ then(self, o: Ordering) → Ordering ! ∅` — `a.cmp(b).then(c.cmp(d))`: nếu bằng thì lấy `o`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ is_lt(self) → bool ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ is_eq(self) → bool ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ is_gt(self) → bool ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)

### `⊐ <T> Option<T>`

- `ƒ is_none(&self) → bool ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ unwrap_or(self, d: T) → T ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ map<U>(self, f: ƒ(T) → U ! ∅) → Option<U> ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ and_then<U>(self, f: ƒ(T) → Option<U> ! ∅) → Option<U> ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ or(self, o: Option<T>) → Option<T> ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ ok_or<E>(self, e: E) → Result<T, E> ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)

### `⊐ <T, E> Result<T, E>`

- `ƒ is_ok(&self) → bool ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ is_err(&self) → bool ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ unwrap_or(self, d: T) → T ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ map<U>(self, f: ƒ(T) → U ! ∅) → Result<U, E> ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ map_err<F>(self, f: ƒ(E) → F ! ∅) → Result<T, F> ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ and_then<U>(self, f: ƒ(T) → Result<U, E> ! ∅) → Result<U, E> ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ ok(self) → Option<T> ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ err(self) → Option<E> ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)

### `⊐ <'a, T: Copy> Vec<'a, T>`

- `ƒ is_empty(&self) → bool ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ get(&self, i: usize) → Option<T> ! ∅` — phần tử thứ `i` (None nếu ngoài biên)
  - hiệu ứng: `∅`; abort: không (trả `None` nếu ngoài biên)
- `ƒ first(&self) → Option<T> ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ last(&self) → Option<T> ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ pop(&!self) → Option<T> ! ∅` — bỏ và trả phần tử cuối
  - hiệu ứng: `∅`; abort: không (trả `None` khi rỗng)
- `ƒ clear(&!self) ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ extend_from_slice(&!self, s: &[T]) ! {mem}` — thêm mọi phần tử của `s` vào cuối
  - hiệu ứng: `{mem}`; abort: như `push` cho từng phần tử
- `ƒ insert(&!self, i: usize, x: T) ! {mem}` — chèn `x` tại `min(i, len)` (dịch phần sau sang phải). Không dùng `⊳ i ≤ len`: hợp đồng trên `&!self` không kiểm được lúc chạy (E0902) nên sẽ đòi chứng minh ở MỌI chỗ gọi; kẹp chỉ số làm hàm TOÀN PHẦN, không bao giờ abort
  - hiệu ứng: `{mem}`; abort: kẹp chỉ số `min(i, len)`; không abort ngoài cấp phát (như `push`)
- `ƒ remove(&!self, i: usize) → Option<T> ! ∅` — bỏ và trả phần tử thứ `i` (dịch phần sau sang trái); None nếu `i ≥ len`
  - hiệu ứng: `∅`; abort: không (trả `None` nếu `i ≥ len`)

### `⊐ <'a, T: Ord + Copy> Vec<'a, T>`

- `ƒ contains(&self, x: &T) → bool ! ∅`
  - hiệu ứng: `∅`; abort: không

### `⊐ [u8]`

- `ƒ starts_with(&self, p: &[u8]) → bool ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ ends_with(&self, p: &[u8]) → bool ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ equals(&self, o: &[u8]) → bool ! ∅` — bằng nhau từng byte (`==` KHÔNG có cho lát; dùng hàm này)
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ find_byte(&self, from_: usize, b: u8) → Option<usize> ! ∅` — vị trí byte đầu tiên bằng `b` tại hoặc sau `from_`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ find(&self, p: &[u8]) → Option<usize> ! ∅` — vị trí đầu tiên của dãy `p` (mẫu rỗng ⇒ Some(0)); tìm ngây thơ O(n·m)
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ trim_ascii(&self) → &[u8] ! ∅` — bỏ khoảng trắng ASCII ở hai đầu
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ parse_u64(&self) → Result<u64, ParseError> ! ∅` — u64 thập phân không dấu
  - hiệu ứng: `∅`; abort: không (trả `Err(ParseError)`: Empty > Invalid > Overflow)
- `ƒ parse_i64(&self) → Result<i64, ParseError> ! ∅` — i64 thập phân với `-` tuỳ chọn (không có `+`)
  - hiệu ứng: `∅`; abort: không (trả `Err(ParseError)`)

### `⊐ str`

- `ƒ starts_with(&self, p: &str) → bool ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ ends_with(&self, p: &str) → bool ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ find(&self, p: &str) → Option<usize> ! ∅`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ parse_u64(&self) → Result<u64, ParseError> ! ∅`
  - hiệu ứng: `∅`; abort: không (trả `Err(ParseError)`)
- `ƒ parse_i64(&self) → Result<i64, ParseError> ! ∅`
  - hiệu ứng: `∅`; abort: không (trả `Err(ParseError)`)

### `⊐ Cap<Out>`

- `ƒ write_i64(&self, n: i64) ! {io}`
  - hiệu ứng: `{io}`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ write_hex(&self, n: u64) ! {io}`
  - hiệu ứng: `{io}`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ write_byte(&self, b: u8) ! {io}`
  - hiệu ứng: `{io}`; abort: toàn phần theo thiết kế (xem mặc định ở trên)

### `⊐ <'a> Vec<'a, u8>`

- `ƒ push_str(&!self, s: &str) ! {mem}`
  - hiệu ứng: `{mem}`; abort: như `Vec::push` cho từng byte
- `ƒ push_u64(&!self, n: u64) ! {mem}`
  - hiệu ứng: `{mem}`; abort: như `Vec::push`
- `ƒ push_i64(&!self, n: i64) ! {mem}`
  - hiệu ứng: `{mem}`; abort: như `Vec::push`

### `⊐ <T: Ord + Copy> [T]`

- `ƒ sort(&!self) ! ∅`
  - hiệu ứng: `∅`; abort: không (sắp xếp tại chỗ; heapsort, KHÔNG ổn định)

### `⊐ <'a, K: Copy + Ord + Hash, V: Copy> HashMap<'a, K, V>`

- `ƒ new(a: &'a Arena) → HashMap<'a, K, V> ! {mem}`
  - hiệu ứng: `{mem}`; abort: không
- `ƒ len(&self) → usize ! ∅` — số khoá đang sống
  - hiệu ứng: `∅`; abort: không
- `ƒ find(&self, k: &K) → Option<usize> ! ∅` — chỉ số trong `ents` của entry sống có khoá `k`
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ get(&self, k: &K) → Option<V> ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ contains_key(&self, k: &K) → bool ! ∅`
  - hiệu ứng: `∅`; abort: không
- `ƒ place(&!self, idx0: usize) ! ∅` — đặt `ents[idx0]` vào ô trống đầu tiên của chuỗi dò (bảng luôn còn ô trống: tải < 1/2)
  - hiệu ứng: `∅`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ rebuild(&!self, newcap_: usize) ! {mem}` — dựng lại với `newcap_` ô: bỏ entry đã xoá, rồi đặt lại mọi entry sống
  - hiệu ứng: `{mem}`; abort: toàn phần theo thiết kế (xem mặc định ở trên)
- `ƒ insert(&!self, k: K, v: V) ! {mem}` — thêm hoặc ghi đè
  - hiệu ứng: `{mem}`; abort: nếu cấp phát arena thất bại (bảng nở khi cần)
- `ƒ remove(&!self, k: &K) → bool ! ∅` — xoá khoá; true nếu có
  - hiệu ứng: `∅`; abort: không
- `ƒ entry_count(&self) → usize ! ∅` — duyệt: số ô entry (kể cả đã xoá) và entry thứ `i` nếu còn sống
  - hiệu ứng: `∅`; abort: không
- `ƒ entry_at(&self, i: usize) → Option<(K, V)> ! ∅`
  - hiệu ứng: `∅`; abort: không (trả `None` nếu ngoài biên hoặc entry đã xoá)

### Nội bộ (công khai vì các impl dùng chung; KHÔNG dựa vào, có thể đổi)

`parse_digits`, `first_non_ws`, `end_after_last_non_ws`, `swap_at`, `sift_down`, `next_cap`, `Ent`
