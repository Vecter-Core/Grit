<!-- TOKEN_PROXY: ~3281 (đo bằng số byte UTF-8 / 3; AI_GUIDE vẫn giữ <= 12000) -->
# AI_PITFALLS — mẫu AI hay viết nhưng `gritc` từ chối (GÓI 7 S3, 2026-11-19)

Xếp theo TẦN SUẤT gặp trong `docs/CAPSTONE_LOG.md`, `docs/APPS_LOG.md`, `docs/USER_BUGS.md` (đếm mã lỗi: E0702 ×11, E0902 ×4, E0301 ×3, E0403 ×2, E0309 ×2, E0302 ×1, E0101 ×1). Mỗi mục: mã, VÌ SAO, cách viết đúng. Khối `grit` dưới đây được `tools/doc_test.py` chạy thật bằng `gritc check` trong cổng (khối `fail=` PHẢI bị từ chối đúng mã; khối còn lại PHẢI xanh). Đọc kèm `docs/AI_GUIDE.md`; đây là phần bổ sung, không thay thế. Mọi thứ ở đây là FAIL-CLOSED: `gritc` từ chối chương trình có thể đúng vì không chứng minh được; KHÔNG phải lỗi của bạn nếu bạn viết đúng logic. Không đổi compiler để nới.

## 1. E0702 — cờ bool trong điều kiện vòng lặp (gặp nhiều nhất)
Vì sao: biến thể `↓` phải GIẢM NGHIÊM NGẶT trên MỌI đường qua thân vòng (không có `↓` thì `gritc` tự suy và báo E0701 nếu không suy được). Cờ như `done` làm vòng dừng "vì cờ", không phải vì một đại lượng số nguyên giảm: bộ kiểm không suy ra được.
Viết đúng: điều kiện so một biến số nguyên với cận, và thân đổi biến đó bằng bước hằng; thoát sớm dùng `⊠;`; vòng không rõ cận dùng nhiên liệu (`core::fuel_left`, kiểu `u64`).
TRƯỚC (bị từ chối):
```grit fail=E0702
ƒ count(n: u32) → u32 ! ∅ {
  μi ≔ 0u32; μdone ≔ false;
  ⟳ done == false ↓ n - i {
    ⎇ i >= n { done ← true; } ⇝ { i ← i + 1u32; }
  }
  i
}
ƒ main() → i32 ! ∅ { count(3u32); 0 }
```
SAU:
```grit
ƒ count(n: u32) → u32 ! ∅ {
  μi ≔ 0u32;
  ⟳ i < n ↓ n - i { i ← i + 1u32; }
  i
}
ƒ main() → i32 ! ∅ { count(3u32); 0 }
```
Cần thoát giữa chừng (không có `break` kiểu khác):
```grit
ƒ find(n: u32) → u32 ! ∅ {
  μi ≔ 0u32;
  ⟳ i < n ↓ n - i {
    ⎇ i == 7u32 { ⊠; } ⇝ { }
    i ← i + 1u32;
  }
  i
}
ƒ main() → i32 ! ∅ { find(20u32); 0 }
```

## 2. E0702 — từ 25 vòng lặp (hoặc > 24 biến biến-thể) trong MỘT hàm
Vì sao: bộ chứng minh chỉ cấp tối đa 24 biến theo dõi quan hệ và 24 biến "bóng" của biến thể vòng lặp mỗi hàm; vòng thừa không chứng minh được nên bị từ chối (fail-closed; thông báo có ghi nguyên nhân). Hàm lớn kiểu "main làm hết" gặp trước tiên.
Viết đúng: TÁCH hàm, mỗi hàm vài vòng (đây cũng là lối viết dễ đọc hơn).
TRƯỚC (25 vòng trong một hàm):
```grit fail=E0702
ƒ f() → () ! ∅ {
  μi1 ≔ 0u32; ⟳ i1 < 3 ↓ 3 - i1 { i1 ← i1 + 1u32; }
  μi2 ≔ 0u32; ⟳ i2 < 3 ↓ 3 - i2 { i2 ← i2 + 1u32; }
  μi3 ≔ 0u32; ⟳ i3 < 3 ↓ 3 - i3 { i3 ← i3 + 1u32; }
  μi4 ≔ 0u32; ⟳ i4 < 3 ↓ 3 - i4 { i4 ← i4 + 1u32; }
  μi5 ≔ 0u32; ⟳ i5 < 3 ↓ 3 - i5 { i5 ← i5 + 1u32; }
  μi6 ≔ 0u32; ⟳ i6 < 3 ↓ 3 - i6 { i6 ← i6 + 1u32; }
  μi7 ≔ 0u32; ⟳ i7 < 3 ↓ 3 - i7 { i7 ← i7 + 1u32; }
  μi8 ≔ 0u32; ⟳ i8 < 3 ↓ 3 - i8 { i8 ← i8 + 1u32; }
  μi9 ≔ 0u32; ⟳ i9 < 3 ↓ 3 - i9 { i9 ← i9 + 1u32; }
  μi10 ≔ 0u32; ⟳ i10 < 3 ↓ 3 - i10 { i10 ← i10 + 1u32; }
  μi11 ≔ 0u32; ⟳ i11 < 3 ↓ 3 - i11 { i11 ← i11 + 1u32; }
  μi12 ≔ 0u32; ⟳ i12 < 3 ↓ 3 - i12 { i12 ← i12 + 1u32; }
  μi13 ≔ 0u32; ⟳ i13 < 3 ↓ 3 - i13 { i13 ← i13 + 1u32; }
  μi14 ≔ 0u32; ⟳ i14 < 3 ↓ 3 - i14 { i14 ← i14 + 1u32; }
  μi15 ≔ 0u32; ⟳ i15 < 3 ↓ 3 - i15 { i15 ← i15 + 1u32; }
  μi16 ≔ 0u32; ⟳ i16 < 3 ↓ 3 - i16 { i16 ← i16 + 1u32; }
  μi17 ≔ 0u32; ⟳ i17 < 3 ↓ 3 - i17 { i17 ← i17 + 1u32; }
  μi18 ≔ 0u32; ⟳ i18 < 3 ↓ 3 - i18 { i18 ← i18 + 1u32; }
  μi19 ≔ 0u32; ⟳ i19 < 3 ↓ 3 - i19 { i19 ← i19 + 1u32; }
  μi20 ≔ 0u32; ⟳ i20 < 3 ↓ 3 - i20 { i20 ← i20 + 1u32; }
  μi21 ≔ 0u32; ⟳ i21 < 3 ↓ 3 - i21 { i21 ← i21 + 1u32; }
  μi22 ≔ 0u32; ⟳ i22 < 3 ↓ 3 - i22 { i22 ← i22 + 1u32; }
  μi23 ≔ 0u32; ⟳ i23 < 3 ↓ 3 - i23 { i23 ← i23 + 1u32; }
  μi24 ≔ 0u32; ⟳ i24 < 3 ↓ 3 - i24 { i24 ← i24 + 1u32; }
  μi25 ≔ 0u32; ⟳ i25 < 3 ↓ 3 - i25 { i25 ← i25 + 1u32; }
}
ƒ main() → i32 ! ∅ { f(); 0 }
```
SAU (tách làm hai hàm):
```grit
ƒ part_a() → () ! ∅ {
  μi1 ≔ 0u32; ⟳ i1 < 3 ↓ 3 - i1 { i1 ← i1 + 1u32; }
  μi2 ≔ 0u32; ⟳ i2 < 3 ↓ 3 - i2 { i2 ← i2 + 1u32; }
  μi3 ≔ 0u32; ⟳ i3 < 3 ↓ 3 - i3 { i3 ← i3 + 1u32; }
  μi4 ≔ 0u32; ⟳ i4 < 3 ↓ 3 - i4 { i4 ← i4 + 1u32; }
  μi5 ≔ 0u32; ⟳ i5 < 3 ↓ 3 - i5 { i5 ← i5 + 1u32; }
  μi6 ≔ 0u32; ⟳ i6 < 3 ↓ 3 - i6 { i6 ← i6 + 1u32; }
  μi7 ≔ 0u32; ⟳ i7 < 3 ↓ 3 - i7 { i7 ← i7 + 1u32; }
  μi8 ≔ 0u32; ⟳ i8 < 3 ↓ 3 - i8 { i8 ← i8 + 1u32; }
  μi9 ≔ 0u32; ⟳ i9 < 3 ↓ 3 - i9 { i9 ← i9 + 1u32; }
  μi10 ≔ 0u32; ⟳ i10 < 3 ↓ 3 - i10 { i10 ← i10 + 1u32; }
  μi11 ≔ 0u32; ⟳ i11 < 3 ↓ 3 - i11 { i11 ← i11 + 1u32; }
  μi12 ≔ 0u32; ⟳ i12 < 3 ↓ 3 - i12 { i12 ← i12 + 1u32; }
  μi13 ≔ 0u32; ⟳ i13 < 3 ↓ 3 - i13 { i13 ← i13 + 1u32; }
}
ƒ part_b() → () ! ∅ {
  μi14 ≔ 0u32; ⟳ i14 < 3 ↓ 3 - i14 { i14 ← i14 + 1u32; }
  μi15 ≔ 0u32; ⟳ i15 < 3 ↓ 3 - i15 { i15 ← i15 + 1u32; }
  μi16 ≔ 0u32; ⟳ i16 < 3 ↓ 3 - i16 { i16 ← i16 + 1u32; }
  μi17 ≔ 0u32; ⟳ i17 < 3 ↓ 3 - i17 { i17 ← i17 + 1u32; }
  μi18 ≔ 0u32; ⟳ i18 < 3 ↓ 3 - i18 { i18 ← i18 + 1u32; }
  μi19 ≔ 0u32; ⟳ i19 < 3 ↓ 3 - i19 { i19 ← i19 + 1u32; }
  μi20 ≔ 0u32; ⟳ i20 < 3 ↓ 3 - i20 { i20 ← i20 + 1u32; }
  μi21 ≔ 0u32; ⟳ i21 < 3 ↓ 3 - i21 { i21 ← i21 + 1u32; }
  μi22 ≔ 0u32; ⟳ i22 < 3 ↓ 3 - i22 { i22 ← i22 + 1u32; }
  μi23 ≔ 0u32; ⟳ i23 < 3 ↓ 3 - i23 { i23 ← i23 + 1u32; }
  μi24 ≔ 0u32; ⟳ i24 < 3 ↓ 3 - i24 { i24 ← i24 + 1u32; }
  μi25 ≔ 0u32; ⟳ i25 < 3 ↓ 3 - i25 { i25 ← i25 + 1u32; }
}
ƒ f() → () ! ∅ { part_a(); part_b(); }
ƒ main() → i32 ! ∅ { f(); 0 }
```

## 3. E0902 — hợp đồng `⊳`/`⊲` không chứng minh được ở chỗ gọi, mà lời gọi có đối số KHÔNG Copy
Vì sao: hợp đồng không chứng minh được thì `gritc` giữ nó thành kiểm tra lúc chạy (ghi chú N0901, vẫn xanh); nhưng nếu lời gọi di chuyển một đối số không Copy (`Vec`, struct, `Buf`) thì không thể gọi lại hợp đồng với giá trị đó ⇒ E0902 (cũng gặp với `⊲` gọi tên tham số bị mượn `&!` hoặc bị gán lại).
Viết đúng: kiểm trong thân hàm (trả `Option`/giá trị mặc định), hoặc để hợp đồng trên hàm chỉ nhận số nguyên/Copy.
TRƯỚC:
```grit fail=E0902
ƒ take(v: Vec<u32>, n: u32) → u32 ! ∅
  ⊳ n > 0u32
{ n }
ƒ f(v: Vec<u32>, k: u32) → u32 ! ∅ { take(v, k) }
ƒ main() → i32 ! ∅ { 0 }
```
SAU:
```grit
ƒ take(v: Vec<u32>, n: u32) → u32 ! ∅ {
  ⎇ n > 0u32 { n } ⇝ { 0u32 }
}
ƒ f(v: Vec<u32>, k: u32) → u32 ! ∅ { take(v, k) }
ƒ main() → i32 ! ∅ { 0 }
```

## 4. E0301 — so sánh `Option` bằng `==`/`!=`
Vì sao: `==` chưa hỗ trợ cho `Option<T>`.
Viết đúng: dùng `⌥` (match).
TRƯỚC:
```grit fail=E0301
ƒ has(o: Option<usize>) → bool ! ∅ { o != None }
ƒ main() → i32 ! ∅ { has(Some(1usize)); 0 }
```
SAU:
```grit
ƒ has(o: Option<usize>) → bool ! ∅ { ⌥ o { Some(_) ↦ true, None ↦ false } }
ƒ main() → i32 ! ∅ { has(Some(1usize)); 0 }
```

## 5. E0403 — truyền `st` và đọc `st.field` trong cùng lời gọi (mượn hai pha)
Vì sao: `f(st, st.pos + 1)` với `st: &!St` mượn `st` độc quyền rồi đọc `st.pos` khi mượn còn sống.
Viết đúng: tính giá trị ra biến cục bộ trước.
TRƯỚC:
```grit fail=E0403
▢ St { pos: u32 }
ƒ step(st: &!St, p: u32) → u32 ! ∅ { p }
ƒ go(st: &!St) → u32 ! ∅ { step(st, st.pos + 1u32) }
ƒ main() → i32 ! ∅ { μs ≔ St { pos: 1u32 }; go(&!s); 0 }
```
SAU:
```grit
▢ St { pos: u32 }
ƒ step(st: &!St, p: u32) → u32 ! ∅ { p }
ƒ go(st: &!St) → u32 ! ∅ { p ≔ st.pos + 1u32; step(st, p) }
ƒ main() → i32 ! ∅ { μs ≔ St { pos: 1u32 }; go(&!s); 0 }
```

## 6. E0301 — đối số `{poly}` (như `Mutex::with`) phải là TÊN hàm
Vì sao: hiệu ứng của đối số hàm được lấy từ tên hàm; kết quả của một lời gọi (hoặc trường) không được chấp nhận làm đối số (GÓI 7 S2.c).
Viết đúng: truyền tên hàm trực tiếp (hoặc gán vào biến cục bộ kiểu hàm).
TRƯỚC:
```grit fail=E0301
#level("10")
▢ Lo { }
■ LO: Mutex<u32, Lo> ≔ Mutex::new(0u32);
ƒ leaf(v: &!u32, n: u32) → u32 ! ∅ { *v }
ƒ pick() → ƒ(&!u32, u32) → u32 ! ∅ ! ∅ { leaf }
ƒ go() ! {lock(Lo)} { LO.with(pick(), 0u32); }
ƒ main() → i32 ! ∅ { 0 }
```
SAU:
```grit
#level("10")
▢ Lo { }
■ LO: Mutex<u32, Lo> ≔ Mutex::new(0u32);
ƒ leaf(v: &!u32, n: u32) → u32 ! ∅ { *v }
ƒ go() ! {lock(Lo)} { LO.with(leaf, 0u32); }
ƒ main() → i32 ! ∅ { 0 }
```

## 7. E0101 — chuỗi không được xuống dòng thô
Vì sao: literal chuỗi một dòng; xuống dòng thật trong chuỗi là lỗi từ vựng (hay xảy ra khi tạo tệp qua heredoc/script làm `
` thành xuống dòng).
Viết đúng: viết escape `
` hai ký tự trong literal; kiểm tệp không bị shell biến đổi.

## 8. Mẫu ngắn khác (không có khối chạy được)
* **E0302 `usize` ≠ `u64`**: `core::fuel_left` nhận `u64`; đừng dùng bộ đếm `usize` làm nhiên liệu.
* **E0309 "không có phương thức `push_str`"** khi tự viết `rig.toml`: thiếu phụ thuộc gói `std`; dùng `rig new` (tự có `std`).
* **E0301 `&![T]` ≠ `&[T]`**: `&!` không tự hạ xuống `&`; với `Vec` dùng `v.as_slice()`.
* **Quá 24 biến cục bộ**: quan hệ giữa biến bị bỏ (ghi chú N0605 ẩn), vòng đúng có thể bị E0702: tách hàm.
* **`! ∅` vẫn có thể tạo luồng:** `spawn(f, a)` có hiệu ứng của worker `f`; hàm `! ∅` gọi `spawn` với worker `! ∅` được chấp nhận. Đừng coi `! ∅` là "không có luồng"; worker `mem` trong hàm `! ∅` bị E0801.
