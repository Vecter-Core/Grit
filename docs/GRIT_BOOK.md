# Sách Grit

Hướng dẫn theo ví dụ cho lập trình viên chưa biết Grit. Bạn nên đã quen một ngôn ngữ kiểu Rust hoặc C; ví dụ so sánh dùng Rust vì Grit mượn nhiều ý của nó (sở hữu, mượn, `match`, trait) nhưng siết chặt hơn.

**Cách đọc.** Mỗi chương một ý chính, ví dụ trước lời giải thích, cuối chương có một bài tập nhỏ kèm đáp án. Mọi khối mã Grit đánh dấu ```` ```grit ```` trong tệp này đều được máy kiểm bằng `tools/doc_test.py` (xem cuối sách). Khối `fail=E0xxx` là ví dụ *cố ý sai*: compiler phải từ chối đúng mã đó. Đoạn trích không phải chương trình đầy đủ nằm trong khối `text`, không được kiểm.

**Grit là gì, và không là gì.**

- Grit là ngôn ngữ thiết kế để *AI viết mã* và người đọc kiểm. Compiler `gritc` (viết bằng Rust) đòi bạn khai báo nhiều thứ mà ngôn ngữ khác để ngầm: hiệu ứng, quyền truy cập, hàm đo giảm của vòng lặp.
- Độ an toàn của Grit đến từ **kiểm tra**: kiểm kiểu, kiểm mượn, kiểm nghĩa vụ số học, kiểm dừng và hiệu ứng, kiểm bí mật, và một lớp kiểm IR (IR Guard) trước khi sinh mã. Bên cạnh đó là một kho mẫu (corpus), so sánh với bản tham chiếu độc lập (oracle) và kiểm thử đột biến. Nó **không** phải một bằng chứng toán học đầy đủ rằng compiler đúng. Compiler có thể có lỗi; mục "Giới hạn" ở cuối nói thẳng những gì đã biết.
- Nền đã kiểm: **Linux x86_64**. Windows: chưa kiểm.
- Nguyên tắc nền: *fail-closed*. Khi `gritc` không chắc, nó báo ĐỎ chứ không cho qua. Nó có thể từ chối một chương trình đúng (bạn sẽ gặp ví dụ ở chương 4), nhưng nó không được chấp nhận chương trình vi phạm quy tắc đã nêu.

**Dụng cụ.** Bạn cần `gritc` (compiler) và `rig` (công cụ dự án, chương 11):

```text
gritc check <tệp.gt|thư-mục>    kiểm, không chạy (thoát 0 nếu qua)
gritc run   <tệp.gt|thư-mục>    kiểm rồi chạy, in giá trị main trả về
gritc explain E0701             giải thích một mã lỗi
gritc fmt <tệp.gt>              ghi lại tệp theo dạng canonical (SỬA TỆP TẠI CHỖ); thêm --check để chỉ kiểm
gritc check --json <tệp>        đầu ra máy đọc (chương 10)
```

---

## Chương 1. Chương trình đầu tiên

```grit run
ƒ main() ! {io} {
  out ≔ root_out();
  out.write_str("Xin chào, Grit!\n");
}
```

```output
Xin chào, Grit!
```

Đọc từng phần:

- `ƒ main()` khai báo hàm `main` (`ƒ` là `fn`). Chữ ký luôn tường minh.
- `! {io}` là **tập hiệu ứng**: hàm này có thể làm I/O. Phần này **bắt buộc** với mọi hàm (chương 5).
- `root_out()` đưa cho bạn quyền ghi stdout, kiểu `Cap<Out>` (chương 6). Chỉ `main` được gọi hàm `root_*`.
- `out.write_str(...)` ghi một chuỗi. Mọi câu lệnh kết thúc bằng `;`.
- `≔` gắn một tên cho một giá trị (như `let`). Mặc định bất biến.

Hàm không có `→ kiểu` trả về `()`. Một hàm có thể trả giá trị bằng biểu thức cuối khối, *không* có `;`:

```grit value=42
ƒ main() → i32 ! ∅ {
  6 * 7
}
```

`→ i32` là kiểu trả về. `! ∅` nói hàm này **thuần**: không I/O, không cấp phát, không chạy mãi. `gritc run` in giá trị `main` trả về (`42`). Lưu ý: giá trị này **không** thành mã thoát của tiến trình; xem chương 12.

**Bài tập 1.** In ba dòng: `Grit`, rồi `la`, rồi số `2026` (dùng `write_u64` cho số).

Đáp án:

```grit run
ƒ main() ! {io} {
  out ≔ root_out();
  out.write_str("Grit\n");
  out.write_str("la\n");
  out.write_u64(2026);
  out.write_str("\n");
}
```

```output
Grit
la
2026
```

---

## Chương 2. Bảng ký hiệu

Grit dùng ký hiệu toán/logic ngắn thay từ khoá dài. Quy tắc bất biến: **mỗi ký hiệu canonical có đúng một bí danh ASCII**, 1–1. Lexer nhận cả hai và cho cùng AST; `gritc fmt` luôn in dạng canonical. Bạn không cần gõ Unicode: viết bằng bí danh ASCII rồi chạy `gritc fmt`.

**Vì sao ký hiệu ngắn?** Mục tiêu thiết kế là máy làm độc giả chính: một khái niệm một ký hiệu, ít nhiễu, ít chỗ mơ hồ (mỗi ký hiệu là *một* codepoint). Người vẫn đọc kiểm được nhờ bảng này, `gritc fmt` và `gritc explain`. Vì ký hiệu Unicode có thể tốn token hơn ASCII với mô hình ngôn ngữ, bí danh ASCII là bắt buộc chứ không phải tuỳ chọn. Đổi lại, Unicode bị siết: cấm ký tự bidi, ký tự vô hình, định danh trộn bảng chữ, và nguồn phải là NFC (lỗi E1201–E1204), để chặn kiểu tấn công "Trojan Source".

### Khai báo và luồng điều khiển

| Canonical | ASCII | Nghĩa (so với Rust) |
|---|---|---|
| `ƒ` | `\fn` | định nghĩa hàm (`fn`) |
| `≔` | `:=` | gắn tên, bất biến (`let`) |
| `μ` | `\mut` | tiền tố khả biến (`mut`), viết sát tên: `μi ≔ 0;` |
| `←` | `<-` | ghi đè (`=` của Rust); chỉ cho biến `μ` |
| `→` | `->` | kiểu trả về |
| `⎇` / `⇝` | `\if` / `\else` | `if` / `else` (là biểu thức) |
| `⌥` / `↦` | `\match` / `=>` | `match` / mũi tên nhánh; phải đầy đủ |
| `⟳` | `\loop` | vòng lặp duy nhất: vô hạn, `while`, hoặc `for x in` |
| `⏎` | `\ret` | `return` |
| `⊠` / `⊡` | `\break` / `\cont` | `break` / `continue` |
| `▢` | `\struct` | `struct` |
| `⊔` | `\enum` | `enum` |
| `⊏` / `⊐` | `\trait` / `\impl` | `trait` / `impl` |
| `▦` | `\mod` | module |
| `⇲` | `\use` | `use` |
| `◈` | `\pub` | `pub` (mặc định mọi thứ là private) |
| `≡` | `\type` | bí danh kiểu |
| `□` / `■` | `\const` / `\static` | hằng lúc biên dịch / static bất biến |

### Hiệu ứng, hợp đồng, dừng

| Canonical | ASCII | Nghĩa |
|---|---|---|
| `!` | `!` | mở tập hiệu ứng: `! ∅`, `! {io, mem}` |
| `∅` | `\nil` | tập hiệu ứng rỗng (hàm thuần) |
| `↓` | `\dec` | hàm đo giảm (variant) của vòng lặp/đệ quy |
| `⊳` / `⊲` | `\pre` / `\post` | tiền / hậu điều kiện; trong `⊲` dùng tên `result` |
| `⋈` | `\inv` | bất biến vòng lặp |
| `⚠` / `⊢` | `\unsafe` / `\proof` | khối unsafe và mệnh đề bằng chứng (Phase 1: gần như luôn bị E1001) |

### Toán tử

| Canonical | ASCII | Nghĩa |
|---|---|---|
| `∧` `∨` `¬` | `\land` `\lor` `\neg` | và, hoặc (ngắn mạch), phủ định |
| `⇒` | `\implies` | suy ra (chỉ trong hợp đồng) |
| `∀` `∃` `∈` | `\forall` `\exists` `\in` | lượng từ có biên, thuộc (chỉ trong hợp đồng) |
| `≤` `≥` `≠` | `<=` `>=` `!=` | so sánh; còn `==` `<` `>` |
| `+ - * / %` | giống | số học **kiểm tràn** |
| `+% -% *%` | giống | quấn vòng (chương 3) |
| `+^ -^ *^` | giống | bão hoà (chương 3) |
| `+? -? *?` | giống | trả `Option` (chương 3) |
| `&` `&!` | giống | mượn chung / mượn độc quyền (chương 8) |

### Chỉ có dạng ASCII (không có canonical)

`self`, `Self`, `as`, `for` (trong `⊐ Trait for Kiểu`), `in` (trong `⟳ x in xs`), `true`, `false`, vùng đời `'a`, chú thích `//` và `///`. Không có `/* */`, không có `+=`, không có `|` hay `^` đứng riêng.

Cùng một chương trình, hai bề mặt. Bản ASCII (gõ được trên mọi bàn phím):

```grit value=11
\const LIMIT: u32 := 10;
\struct Pair { a: u32, b: u32 }
\enum Kind { Small, Big }
\trait Describe { \fn kind(&self) -> Kind ! \nil; }
\impl Describe for Pair {
  \fn kind(&self) -> Kind ! \nil {
    \if self.a + self.b < LIMIT { Kind::Small } \else { Kind::Big }
  }
}
\fn sum_odd(n: u32) -> u32 ! \nil \pre n < 100 {
  \mut i := 0u32;
  \mut s := 0u32;
  \loop i < n \dec n - i {
    i <- i + 1;
    \if i % 2 == 0 { \cont; } \else { }
    \if i > 50 { \break; } \else { }
    s <- s + i;
  }
  s
}
\fn main() -> u32 ! \nil {
  p := Pair { a: 3, b: 9 };
  r := \match p.kind() { Kind::Small => 1, Kind::Big => 2 };
  \if r == 2 \land \neg (r == 3) { sum_odd(6) + r } \else { 0 }
}
```

Và dạng canonical tương đương:

```grit value=11
□ LIMIT: u32 ≔ 10;
▢ Pair { a: u32, b: u32 }
⊔ Kind { Small, Big }
⊏ Describe { ƒ kind(&self) → Kind ! ∅; }
⊐ Describe for Pair {
  ƒ kind(&self) → Kind ! ∅ { ⎇ self.a + self.b < LIMIT { Kind::Small } ⇝ { Kind::Big } }
}
ƒ sum_odd(n: u32) → u32 ! ∅ ⊳ n < 100 {
  μi ≔ 0u32;
  μs ≔ 0u32;
  ⟳ i < n ↓ n - i {
    i ← i + 1;
    ⎇ i % 2 == 0 { ⊡; } ⇝ { }
    ⎇ i > 50 { ⊠; } ⇝ { }
    s ← s + i;
  }
  s
}
ƒ main() → u32 ! ∅ {
  p ≔ Pair { a: 3, b: 9 };
  r ≔ ⌥ p.kind() { Kind::Small ↦ 1, Kind::Big ↦ 2 };
  ⎇ r == 2 ∧ ¬(r == 3) { sum_odd(6) + r } ⇝ { 0 }
}
```

Hai tệp này cho cùng giá trị, và `gritc fmt` biến cả hai thành một tệp giống hệt nhau (đã thử): đó là ý nghĩa của "cùng AST".

Chi tiết lexer đáng nhớ:

- `μ` là ký hiệu dành riêng: không định danh nào chứa `μ`. Viết `μs`, không phải tên `s` có `μ` ở giữa.
- Bí danh dạng `\tên` khớp theo dãy dài nhất; toán tử ASCII cũng khớp dài nhất (`<-`, `->`, `:=`, `=>`, `..=`, `+%`…). Hệ quả: `x<-1` đọc là gán `x <- 1`; muốn so sánh với số âm hãy viết `x < -1`.
- Số: `1_000_000`, `0xFF`, `0b1010`; hậu tố kiểu `250u8`, `7i64`. Dấu `_` chỉ nằm *giữa* chữ số (`1000_u32` bị từ chối).
- Chuỗi một dòng, UTF-8; không nội suy, không chuỗi nhiều dòng.

**Bài tập 2.** Viết lại hàm sau bằng ký hiệu canonical (có thể chạy `gritc fmt` để so):

```text
\fn is_even(n: u32) -> bool ! \nil { n % 2 == 0 }
\fn main() -> u32 ! \nil { \if is_even(10) \land \neg is_even(7) { 1 } \else { 0 } }
```

Đáp án:

```grit value=1
ƒ is_even(n: u32) → bool ! ∅ { n % 2 == 0 }
ƒ main() → u32 ! ∅ { ⎇ is_even(10) ∧ ¬is_even(7) { 1 } ⇝ { 0 } }
```

---

## Chương 3. Biến, kiểu và số nguyên

Bất biến là mặc định; `μ` mới cho phép ghi đè bằng `←`. Không có `+=`: viết `x ← x + 1`.

```grit value=19
▢ Point { x: i32, y: i32 }

ƒ main() → i32 ! ∅ {
  μcount ≔ 0;
  p ≔ Point { x: 4, y: 5 };
  pair ≔ (p.x, p.y);
  arr ≔ [10, 20, 30];
  ⎇ pair.0 > 3 { count ← count + 1; } ⇝ { }
  big ≔ ⎇ arr[1] > 15 { 10 } ⇝ { 0 };
  count + pair.1 + big + 3
}
```

`⎇` là biểu thức nên có thể đứng ở vế phải của `≔`. Hai nhánh phải cùng kiểu; thiếu `⇝` thì nhánh `then` phải có kiểu `()`.

**Kiểu có sẵn:** `i8 i16 i32 i64 i128 isize`, `u8 u16 u32 u64 u128 usize`, `f64`, `bool`, `char`, `&str`, tuple `(A, B)`, mảng `[T; N]`, lát `&[T]`, cùng `▢` và `⊔`. `f32` là dành riêng (chưa dùng được). Không có `null`, không có biến chưa khởi tạo, không có ngoại lệ.

**Không có ép kiểu ngầm.** Hai số khác kiểu không cộng được; viết `as`:

```grit fail=E0302
ƒ main() → i64 ! ∅ {
  a ≔ 5i32;
  b ≔ 7i64;
  a + b
}
```

```text
error[E0302]: ép kiểu số ngầm bị cấm (D1): giá trị kiểu `i32` được dùng ở chỗ cần `i64`; viết `as i64` nếu đó là ý định
  = help: ép kiểu tường minh: ` as i64` (maybe-incorrect)
```

Sửa: `(a as i64) + b`. Literal không hậu tố (`5`) tự suy theo ngữ cảnh.

### Kiểm tràn: nghĩa vụ ở mọi phép tính

Trong Grit, `+ - * / %` trên số nguyên **kiểm tràn**. Mỗi phép tính sinh một *nghĩa vụ* ("kết quả vừa kiểu", với chia/`%` thêm "mẫu số ≠ 0"). Có ba kết cục, bạn sẽ gặp lại ở chương 9:

1. Bộ giải chứng minh được nghĩa vụ ⇒ không còn kiểm tra nào lúc chạy.
2. Chắc chắn sai trên mọi đường ⇒ ĐỎ ngay lúc biên dịch.
3. Không chứng minh được ⇒ giữ một kiểm tra lúc chạy và báo ghi chú `N0601` (tràn). Nếu nó kích hoạt, chương trình `abort` (mã thoát 70), không bao giờ cho kết quả sai lặng lẽ.

```grit fail=E0601
ƒ main() → u8 ! ∅ {
  250u8 + 10u8
}
```

```grit
ƒ add(a: u8, b: u8) → u8 ! ∅ {
  a + b
}

ƒ main() ! {io} {
  out ≔ root_out();
  out.write_u64(add(250u8, 10u8) as u64);
}
```

Khối trên qua `check` nhưng với ghi chú `N0601` ("không chứng minh được không tràn: giữ kiểm tra lúc chạy"). Chạy nó sẽ in `abort: overflow` và thoát với mã 70.

Khi bạn *muốn* hành vi khác, chọn toán tử có tên: `+% -% *%` quấn vòng, `+^ -^ *^` bão hoà (dừng ở min/max của kiểu), `+? -? *?` trả `Option<T>` (`None` nếu tràn).

```grit run
ƒ show(out: &Cap<Out>, o: Option<u8>) ! {io} {
  ⌥ o {
    Some(v) ↦ { out.write_u64(v as u64); },
    None ↦ { out.write_str("None"); },
  }
  out.write_str("\n");
}

ƒ main() ! {io} {
  out ≔ root_out();
  a ≔ 250u8;
  out.write_u64((a +% 10u8) as u64);
  out.write_str("\n");
  out.write_u64((a +^ 10u8) as u64);
  out.write_str("\n");
  show(&out, a +? 10u8);
  show(&out, 200u8 +? 10u8);
}
```

```output
4
255
None
210
```

`250 +% 10` quấn thành `4`; `+^` kẹt ở `255`; `+?` cho `None` khi tràn và `Some(210)` khi vừa. Chỉ số mảng ngoài biên cũng là nghĩa vụ: `a[5]` trên mảng 3 phần tử là E0603, còn chỉ số chưa biết thì giữ kiểm tra lúc chạy (`N0602`).

**Bài tập 3.** Viết `add_or_minus_one(a: u8, b: u8) → i32` trả tổng nếu không tràn, ngược lại `-1`, dùng `+?`.

Đáp án (`main` trả `-1` cho `250 + 10`):

```grit value=-1
ƒ add_or_minus_one(a: u8, b: u8) → i32 ! ∅ {
  ⌥ a +? b {
    Some(s) ↦ s as i32,
    None ↦ -1,
  }
}

ƒ main() → i32 ! ∅ {
  add_or_minus_one(250u8, 10u8)
}
```

---

## Chương 4. Vì sao có `↓`: dừng, hàm đo giảm và nhiên liệu

Không compiler nào quyết định được mọi chương trình có dừng hay không (bài toán dừng). Grit chọn: **mọi vòng lặp và mọi đệ quy phải chỉ ra một lý do dừng**, hoặc khai báo mình được phép chạy mãi bằng hiệu ứng `div` (chương 5). Lý do dừng là một **hàm đo giảm** (variant): một biểu thức số nguyên không âm mà mỗi vòng/mỗi lần gọi đệ quy **giảm nghiêm ngặt**. Một dãy số tự nhiên không thể giảm mãi, nên vòng lặp phải dừng.

Với `⟳ điều-kiện { … }` (kiểu `while`), `gritc` thử **tự suy** hàm đo từ điều kiện: một vế so sánh một biến với một cận, và thân đẩy biến về phía cận bằng bước hằng.

```grit value=55
ƒ sum_to(n: u64) → u64 ! ∅ ⊳ n < 1000 {
  μi ≔ 0u64;
  μs ≔ 0u64;
  ⟳ i < n {
    i ← i + 1;
    s ← s + i;
  }
  s
}

ƒ main() → u64 ! ∅ { sum_to(10) }
```

Ở đây `n - i` giảm mỗi vòng và không âm, nên qua. Vòng `⟳ x in xs` trên range hoặc lát cũng tự dừng. Nếu thân không tiến về cận, bạn bị từ chối (E0701), không có đường lui lúc chạy:

```grit fail=E0701
ƒ stuck(n: u32) → u32 ! ∅ {
  μi ≔ 0u32;
  ⟳ i < n {
  }
  i
}

ƒ main() → u32 ! ∅ { stuck(3) }
```

Khi suy luận không đủ, bạn viết hàm đo bằng `↓`. Với **đệ quy** thì `↓` là bắt buộc (không có đoán), đặt ngay sau chữ ký:

```grit value=120
ƒ fact(n: u64) → u64 ! ∅ ⊳ n <= 10 ↓ n {
  ⎇ n == 0 { 1 } ⇝ { n * fact(n - 1) }
}

ƒ main() → u64 ! ∅ { fact(5) }
```

Quên `↓` là E0703:

```grit fail=E0703
ƒ fact(n: u64) → u64 ! ∅ ⊳ n <= 10 {
  ⎇ n == 0 { 1 } ⇝ { n * fact(n - 1) }
}

ƒ main() → u64 ! ∅ { fact(5) }
```

Đệ quy tương hỗ cần `↓` ở *mọi* hàm trong chu trình.

### Khi bộ kiểm chưa chứng minh được

Compiler **sound nhưng không complete**: nó có thể từ chối chương trình đúng. Thuật toán Euclid dừng vì `a % b < b`, nhưng bộ giải của Grit hiện không biết điều đó, nên `↓ b` bị từ chối:

```grit fail=E0702
ƒ gcd(a: u64, b: u64) → u64 ! ∅ ↓ b {
  ⎇ b == 0 { a } ⇝ { gcd(b, a % b) }
}

ƒ main() → u64 ! ∅ { gcd(48, 18) }
```

```text
error[E0702]: lời gọi đệ quy không làm biến thể `↓` giảm nghiêm ngặt (hoặc không bị chặn dưới ≥ 0)
```

Chương trình hợp lệ nhưng chưa chứng minh được: dùng mẫu **nhiên liệu** (fuel). Bạn cấp một ngân sách số vòng, đưa nó vào điều kiện và vào `↓`; nó giảm đúng 1 mỗi vòng nên *luôn* chứng minh được. Sau vòng lặp gọi `core::fuel_left(fuel)`: nếu nhiên liệu cạn (bằng 0) thì chương trình abort với `fuel_exhausted` thay vì chạy mãi hay trả kết quả cụt.

Ví dụ kinh điển: dãy Collatz (chưa ai chứng minh nó luôn dừng). Không có hàm đo nào:

```grit fail=E0701
ƒ steps(n: u64) → u64 ! ∅ {
  μx ≔ n;
  μc ≔ 0u64;
  ⟳ x > 1 {
    ⎇ x % 2 == 0 { x ← x / 2; } ⇝ { x ← x * 3 + 1; }
    c ← c + 1;
  }
  c
}

ƒ main() → u64 ! ∅ { steps(27) }
```

Với nhiên liệu:

```grit value=111
ƒ steps(n: u64) → u64 ! ∅ {
  μx ≔ n;
  μc ≔ 0u64;
  μfuel ≔ 1000u64;
  ⟳ x > 1 ∧ fuel > 0 ↓ fuel {
    ⎇ x % 2 == 0 { x ← x / 2; } ⇝ { x ← x * 3 + 1; }
    c ← c + 1;
    fuel ← fuel - 1;
  }
  core::fuel_left(fuel);
  c
}

ƒ main() → u64 ! ∅ { steps(27) }
```

Đọc kỹ cái giá: kiểm tra `fuel_left` là kiểm tra lúc chạy (ghi chú `N0905`). Bạn phải chọn nhiên liệu *đủ lớn* và có lý lẽ cho nó (ví dụ "số chữ số của `u64` ≤ 20, nên 24 là đủ"). Nhiên liệu bằng đúng số vòng cần thiết sẽ abort sai vào vòng cuối; hãy cho dư.

**Bài tập 4.** Đếm số chữ số thập phân của `n: u64` (với `0` cho kết quả `0`). Vòng `x ← x / 10` không tự chứng minh được; hãy dùng nhiên liệu.

Đáp án (`digits(12345) = 5`; `u64` có tối đa 20 chữ số):

```grit value=5
ƒ digits(n: u64) → u64 ! ∅ {
  μx ≔ n;
  μc ≔ 0u64;
  μfuel ≔ 24u64;
  ⟳ x > 0 ∧ fuel > 0 ↓ fuel {
    x ← x / 10;
    c ← c + 1;
    fuel ← fuel - 1;
  }
  core::fuel_left(fuel);
  c
}

ƒ main() → u64 ! ∅ { digits(12345) }
```

---

## Chương 5. Hiệu ứng: `! ∅`, `{io}`, `{mem}`, `{div}`

Mỗi hàm khai báo những gì nó *có thể làm* ngoài tính toán thuần. Bốn hiệu ứng bạn dùng hằng ngày:

| Hiệu ứng | Nghĩa |
|---|---|
| `∅` | thuần: chỉ tính toán |
| `io` | đọc/ghi thế giới ngoài (stdout, tệp, môi trường…) |
| `mem` | cấp phát bộ nhớ (Arena, `Vec`, `Text`) |
| `div` | có thể không dừng hoặc thoát tiến trình (`⟳ { }` vô hạn, `core::exit`, `core::abort`) |

(Còn `st`, `ffi`, `lock(L)` và vài hiệu ứng của đa luồng; sách này không đi vào.)

**Luật:** hiệu ứng *khai báo* phải bao hàm hiệu ứng *suy ra* từ thân hàm và từ mọi hàm được gọi. Hiệu ứng lan truyền lên các hàm gọi, kể cả `main`. Vi phạm là E0801.

```grit fail=E0801
ƒ hello(out: &Cap<Out>) ! ∅ {
  out.write_str("hi\n");
}

ƒ main() ! {io} {
  out ≔ root_out();
  hello(&out);
}
```

```text
error[E0801]: `main::hello` khai báo hiệu ứng ∅ nhưng suy ra `io` (lời gọi `write_str`)
```

Sửa bằng `! {io}` ở `hello`. **Vì sao bắt khai báo?** Để chỉ nhìn chữ ký là biết một hàm có đụng thế giới ngoài, có cấp phát, có thể chạy mãi hay không, mà không phải đọc thân và mọi hàm nó gọi. Hàm `! ∅` là hàm bạn gọi mà không phải lo gì thêm. Với trait, một `⊐` không được khai nhiều hiệu ứng hơn chữ ký trong `⊏` (E0802).

`div` là chỗ duy nhất vòng lặp vô hạn `⟳ { … }` được phép (nó không cần `↓`). Hiệu ứng lan lên: gọi hàm có `div` thì `main` cũng phải khai `div`.

```grit run
ƒ serve(out: &Cap<Out>) ! {io, div} {
  μn ≔ 0u64;
  ⟳ {
    out.write_u64(n);
    out.write_str("\n");
    ⎇ n >= 2 { core::exit(0); } ⇝ { }
    n ← n + 1;
  }
}

ƒ main() ! {io, div} {
  out ≔ root_out();
  serve(&out);
}
```

```output
0
1
2
```

**Bài tập 5.** Viết `print_squares(out: &Cap<Out>, n: u32)` in `0 1 4 9 …` (mỗi số một dòng, `n` số) bằng một `Vec` tạm trên một `Arena`. Khai báo đúng hiệu ứng (gợi ý: có cả I/O và cấp phát). Đáp án:

```grit run
ƒ print_squares(out: &Cap<Out>, n: u32) ! {io, mem} {
  μa ≔ Arena::new();
  μv ≔ Vec::new(&a);
  ⟳ i in 0..n {
    v.push(i * i);
  }
  ⟳ x in v.as_slice() {
    out.write_u64(*x as u64);
    out.write_str("\n");
  }
}

ƒ main() ! {io, mem} {
  out ≔ root_out();
  print_squares(&out, 4);
}
```

```output
0
1
4
9
```

---

## Chương 6. Capability: quyền là một giá trị

Trong Grit không có hàm toàn cục "mở tệp" hay "in ra". Muốn làm I/O, bạn cần cầm một giá trị **capability** có kiểu `Cap<…>`:

| Kiểu | Cho phép |
|---|---|
| `Cap<Out>` | ghi stdout: `write_str`, `write_u64`, `write_bytes` (+ `write_i64`, `write_hex` của `std`) |
| `Cap<ErrOut>` | ghi stderr: `write_str`, `write_u64` |
| `Cap<In>` | đọc stdin: `read_all` |
| `Cap<Fs>` | tệp: `open`, `read`, `write` |
| `Cap<Env>` | đối số dòng lệnh: `arg_count`, `arg(i)` |
| `Cap<Alloc>` | tạo Arena: `arena()` |
| `Cap<Net>`, `Cap<Clock>` | mạng, đồng hồ |

Quyền gốc chỉ có một nguồn: các hàm `root_out()`, `root_in()`, `root_err()`, `root_fs()`, `root_env()`, `root_alloc()`, … mà **chỉ `main` được gọi**. Mọi hàm khác phải *nhận* capability qua tham số. Vậy chữ ký hàm là danh sách đầy đủ những gì nó chạm tới:

```grit fail=E0310
ƒ log(msg: &str) ! {io} {
  out ≔ root_out();
  out.write_str(msg);
}

ƒ main() ! {io} { log("x"); }
```

```text
error[E0310]: thiếu capability: lời gọi này tạo `Cap<Out>` từ hư không (nguồn capability gốc); chỉ `main` được gọi nó — các hàm khác phải nhận `Cap<…>` qua tham số (D28, D29)
```

Cách đúng: `main` truyền xuống, mượn bằng `&`:

```grit run
ƒ greet(out: &Cap<Out>, name: &str) ! {io} {
  out.write_str("Xin chào, ");
  out.write_str(name);
  out.write_str("!\n");
}

ƒ main() ! {io} {
  out ≔ root_out();
  env ≔ root_env();
  ⎇ env.arg_count() > 1 { greet(&out, env.arg(1)); } ⇝ { greet(&out, "thế giới"); }
}
```

```output
Xin chào, thế giới!
```

Capability là kiểu *affine*: không `Copy`, không thể tạo bằng literal. Gán nó sang tên khác là **move** và dùng lại tên cũ là E0401 (xem thêm chương 8):

```grit fail=E0401
ƒ main() ! {io} {
  out ≔ root_out();
  out2 ≔ out;
  out.write_str("x");
}
```

Ví dụ với tệp: `Cap<Fs>` ghi nguyên tệp và đọc lại. Lỗi I/O là `Result<_, IoError>` (`NotFound`, `Denied`, `Invalid`, `Other`), không phải ngoại lệ:

```grit run
ƒ save(fs: &Cap<Fs>, out: &Cap<Out>) ! {io} {
  ⌥ fs.write("ghi_chu.txt", "xin chao\n".as_bytes()) {
    Ok(_) ↦ { out.write_str("da ghi\n"); },
    Err(_) ↦ { out.write_str("loi ghi\n"); },
  }
}

ƒ load(fs: &Cap<Fs>, out: &Cap<Out>) ! {io} {
  ⌥ fs.open("ghi_chu.txt") {
    Ok(f) ↦ { out.write_bytes(f.bytes()); },
    Err(IoError::NotFound) ↦ { out.write_str("khong thay tep\n"); },
    Err(_) ↦ { out.write_str("loi khac\n"); },
  }
}

ƒ main() ! {io} {
  out ≔ root_out();
  fs ≔ root_fs();
  save(&fs, &out);
  load(&fs, &out);
}
```

```output
da ghi
xin chao
```

Đọc stdin theo cùng kiểu (bộ kiểm tài liệu chạy với stdin rỗng, nên in `0 byte`; chạy thật với `echo abc | ./chuongtrinh` sẽ in số byte đọc được):

```grit run
ƒ main() ! {io} {
  out ≔ root_out();
  inp ≔ root_in();
  ⌥ inp.read_all() {
    Ok(f) ↦ {
      out.write_u64(f.bytes().len() as u64);
      out.write_str(" byte\n");
    },
    Err(_) ↦ { out.write_str("loi doc stdin\n"); },
  }
}
```

```output
0 byte
```

**Vì sao thiết kế thế?** Một thư viện bạn gọi mà chữ ký không có `Cap<Fs>` thì không thể đọc tệp của bạn: đó là lỗi kiểu lúc biên dịch, không phải hành vi lúc chạy. Rig (chương 11) dựa vào đây để biết một gói đòi quyền gì. Giới hạn trung thực: `Cap<Fs>::sub` (thu hẹp vào một thư mục con) có trong chữ ký ở prelude nhưng backend LLVM hiện chưa chạy được (đã thử); đừng dựa vào nó.

**Bài tập 6.** Viết `banner(out: &Cap<Out>, n: u32)` in `n` dấu `*` rồi xuống dòng, và `main` gọi `banner(&out, 5)`. Đáp án:

```grit run
ƒ banner(out: &Cap<Out>, n: u32) ! {io} {
  ⟳ i in 0..n {
    out.write_str("*");
  }
  out.write_str("\n");
}

ƒ main() ! {io} {
  out ≔ root_out();
  banner(&out, 5);
}
```

```output
*****
```

---

## Chương 7. Arena, `Vec` và `Text`

Grit không có bộ cấp phát toàn cục ngầm. Mọi bộ nhớ động đến từ một **`Arena`** (bạn tạo bằng `Arena::new()`, hoặc xin từ `Cap<Alloc>`), cho nên hàm cấp phát luôn mang hiệu ứng `mem`. Arena giải phóng *cả khối một lần* khi nó ra khỏi phạm vi: **không có destructor cho từng phần tử**.

```grit value=60
ƒ main() → u64 ! {mem} {
  μa ≔ Arena::new();
  μv ≔ Vec::new(&a);
  v.push(10u32);
  v.push(20u32);
  v.push(30u32);
  s ≔ v.as_slice();
  μi ≔ 0usize;
  μtotal ≔ 0u64;
  ⟳ i < s.len() {
    total ← total + (s[i] as u64);
    i ← i + 1;
  }
  total
}
```

- `Vec<'a, T>` là mảng tăng trưởng nằm trong arena; `Vec::new(&a)` giữ một tham chiếu *chia sẻ* tới `a`. Hệ quả: **nhiều `Vec`/`Text` dùng chung một Arena**.
- `as_slice()` cho `&[T]` (đọc), `as_mut_slice()` cho `&![T]` (ghi), `len()`, `push`, `reserve`, `truncate`.
- `Text<'a>` là bộ đệm UTF-8 cũng nằm trong arena: dựng bằng `push_str`/`push_char`, đọc bằng `as_str()`/`as_bytes()`. `len()` đếm **byte**.

```grit run
ƒ main() ! {io, mem} {
  out ≔ root_out();
  μa ≔ Arena::new();
  μt ≔ Text::new(&a);
  t.push_str("Xin ");
  t.push_str("chào");
  t.push_char('!');
  out.write_str(t.as_str());
  out.write_str("\n");
  out.write_u64(t.len() as u64);
  out.write_str("\n");
}
```

```output
Xin chào!
10
```

(`chào` có 5 byte vì `à` chiếm 2 byte UTF-8, nên tổng là 10.)

Hai `Vec` cùng một Arena là bình thường:

```grit value=2
ƒ main() → u64 ! {mem} {
  μa ≔ Arena::new();
  μv ≔ Vec::new(&a);
  μw ≔ Vec::new(&a);
  v.push(1u64);
  w.push(2u64);
  v.len() as u64 + w.len() as u64
}
```

**Phần tử phải không cần dọn dẹp.** Vì arena không chạy destructor, `Vec<T>` với `T` cần drop (chứa `Arena`, có `⊐ Drop`, …) bị từ chối ở E0318. Các hàm tiện ích của `std` (`get`, `pop`, `insert`, `contains`, …) còn đòi `T: Copy`; muốn một struct của bạn dùng được, viết `⊐ Copy for P { }`.

```grit fail=E0318
ƒ main() ! {mem} {
  μa ≔ Arena::new();
  μb ≔ Arena::new();
  μv ≔ Vec::new(&a);
  v.push(b);
}
```

**Arena và mượn.** `a.reset()` (giải phóng toàn bộ, cần `μa`) bị chặn khi còn `Vec` sống trong nó (E0405), và đừng giữ lát `as_slice()` qua một `push` (E0403), vì `push` có thể dời bộ nhớ:

```grit fail=E0405
ƒ main() → u32 ! {mem} {
  μa ≔ Arena::new();
  μv ≔ Vec::new(&a);
  v.push(1u32);
  a.reset();
  v.len() as u32
}
```

```grit fail=E0403
ƒ main() ! {mem} {
  μa ≔ Arena::new();
  μv ≔ Vec::new(&a);
  v.push(1u32);
  s ≔ v.as_slice();
  v.push(2u32);
  x ≔ s[0];
}
```

**Bài tập 7.** Tính tổng bình phương của `0..5` (`0+1+4+9+16 = 30`) bằng một `Vec<u32>`. Đáp án:

```grit value=30
ƒ main() → u64 ! {mem} {
  μa ≔ Arena::new();
  μv ≔ Vec::new(&a);
  ⟳ i in 0..5u32 {
    v.push(i * i);
  }
  μt ≔ 0u64;
  ⟳ x in v.as_slice() {
    t ← t + (*x as u64);
  }
  t
}
```

---

## Chương 8. Mượn: `&T`, `&!T` và vùng đời `'a`

Giá trị không `Copy` (capability, `Arena`, `Vec`, struct không `⊐ Copy`) bị **move** khi gán hoặc truyền: dùng lại là E0401. Muốn dùng mà không chuyển quyền, hãy **mượn**:

- `&T`: mượn chung, nhiều lượt cùng lúc, chỉ đọc.
- `&!T`: mượn **độc quyền**, một lượt, được ghi. Nó loại trừ mọi `&`/`&!` khác còn sống.

```grit value=42
ƒ bump(x: &!i32) ! ∅ {
  *x ← *x + 1;
}

ƒ peek(x: &i32) → i32 ! ∅ { *x }

ƒ main() → i32 ! ∅ {
  μn ≔ 41;
  bump(&!n);
  peek(&n)
}
```

`*x` giải tham chiếu (đứng đầu toán hạng là deref, đứng giữa hai toán hạng là phép nhân). Một khoản mượn sống đến **lần dùng cuối** của nó, không phải đến hết khối. Vi phạm:

```grit fail=E0403
ƒ main() → i32 ! ∅ {
  μn ≔ 1;
  r ≔ &n;
  w ≔ &!n;
  *w ← 2;
  *r
}
```

```text
error[E0403]: mượn duy nhất `&!` `n` trong khi còn khoản mượn chung `&` đang được dùng về sau
```

```grit fail=E0402
ƒ main() → i32 ! ∅ {
  μn ≔ 1;
  a ≔ &!n;
  b ≔ &!n;
  *a ← 2;
  *b ← 3;
  n
}
```

Trả tham chiếu tới biến cục bộ là E0404:

```grit fail=E0404
ƒ bad() → &i32 ! ∅ {
  x ≔ 5;
  &x
}

ƒ main() → i32 ! ∅ { *bad() }
```

### Vùng đời `'a`

Mỗi tham chiếu mang một vùng đời (region) cho biết nó hợp lệ bao lâu. Thường bạn không phải viết; chữ ký tự suy nếu **đúng một** vị trí tham chiếu ở đầu vào (kết quả lấy vùng đời đó), và là `'static` nếu không có vị trí nào. Từ **hai vị trí trở lên** bạn phải nêu `'a` (khác Rust: ở đây `&self` không tự "thắng"):

```grit fail=E0315
ƒ pick(a: &i32, b: &i32) → &i32 ! ∅ { a }

ƒ main() → i32 ! ∅ {
  x ≔ 1;
  y ≔ 2;
  *pick(&x, &y)
}
```

```grit value=1
ƒ pick<'a>(a: &'a i32, b: &i32) → &'a i32 ! ∅ { a }

ƒ main() → i32 ! ∅ {
  x ≔ 1;
  y ≔ 2;
  *pick(&x, &y)
}
```

Chữ ký nói: kết quả sống lâu bằng `a`, không bằng `b`. Struct chứa tham chiếu phải ghi vùng đời ở trường và ở tham số kiểu (vùng đời đứng *trước* kiểu: `Pair<'a, T>`):

```grit value=42
▢ Holder<'a> { r: &'a i32 }

ƒ get<'a>(h: &Holder<'a>) → &'a i32 ! ∅ { h.r }

ƒ main() → i32 ! ∅ {
  x ≔ 41;
  h ≔ Holder { r: &x };
  *get(&h) + 1
}
```

Vùng đời của dữ liệu trong Arena chính là vùng đời của khoản mượn Arena. Vì thế một con trỏ cấp từ Arena cục bộ không thoát được hàm (E0405); nhận Arena qua tham số thì được:

```grit fail=E0405
ƒ leak() → &i32 ! {mem} {
  μa ≔ Arena::new();
  a.alloc(5)
}

ƒ main() → i32 ! {mem} { *leak() }
```

```grit value=5
ƒ make<'a>(a: &'a Arena) → &'a i32 ! {mem} {
  a.alloc(5)
}

ƒ main() → i32 ! {mem} {
  a ≔ Arena::new();
  *make(&a)
}
```

**Bài tập 8.** Viết `swap(a: &!i32, b: &!i32)` hoán đổi hai số, rồi `main` trả `x * 10 + y` sau khi hoán đổi `x = 1`, `y = 2` (kết quả `21`). Đáp án:

```grit value=21
ƒ swap(a: &!i32, b: &!i32) ! ∅ {
  t ≔ *a;
  *a ← *b;
  *b ← t;
}

ƒ main() → i32 ! ∅ {
  μx ≔ 1;
  μy ≔ 2;
  swap(&!x, &!y);
  x * 10 + y
}
```

---

## Chương 9. Hợp đồng `⊳`/`⊲` và nghĩa vụ M7

Bạn có thể ghi điều kiện lên chữ ký:

- `⊳ điều-kiện` (tiền điều kiện): người gọi phải bảo đảm.
- `⊲ điều-kiện` (hậu điều kiện): hàm bảo đảm; dùng `result` cho giá trị trả về.
- `⋈ điều-kiện` đặt trên vòng lặp (bất biến). Lượng từ `∀ i ∈ 0..n : …` và `∃` chỉ dùng được trong hợp đồng và phải có biên.

```grit value=14
ƒ safe_div(a: u32, b: u32) → u32 ! ∅ ⊳ b > 0 ⊲ result <= a {
  a / b
}

ƒ main() → u32 ! ∅ {
  safe_div(100, 7)
}
```

**Nghĩa vụ M7.** Pass M7 của `gritc` biến mỗi thứ cần đúng thành một *nghĩa vụ* (obligation): không tràn của mỗi phép số học, mẫu số khác 0, chỉ số trong biên, ép kiểu không mất thông tin, mỗi `⊳` tại chỗ gọi, mỗi `⊲` ở chỗ thoát, mỗi `⋈`. Rồi một bộ giải (khoảng giá trị, quan hệ giữa biến, điều kiện nhánh, và các `⊳` của hàm hiện tại) cố chứng minh từng cái. Bốn kết cục:

| Kết cục | Thấy gì |
|---|---|
| Chứng minh được | kiểm tra bị xoá, không báo gì |
| Chắc chắn sai | ĐỎ: `E0601` tràn, `E0602` chia 0, `E0603` ngoài biên, `E0901` hợp đồng sai |
| Chưa chứng minh nhưng kiểm được lúc chạy | giữ kiểm tra + ghi chú `N0601`/`N0602`/`N0603`/`N0604`, `N0901` (`⊳`), `N0902` (`⊲`), `N0903` (lượng từ), `N0904` (`⋈`), `N0905` (nhiên liệu) |
| Chưa chứng minh **và** không kiểm được lúc chạy | ĐỎ `E0902` (không có đường lui) |

Các ghi chú `N…` là chi phí được *hiển thị*: kiểm tra giữ lại ở mọi hồ sơ biên dịch (dev lẫn release), không im lặng bị bỏ. Ví dụ `⊳` chắc chắn sai tại chỗ gọi:

```grit fail=E0901
ƒ safe_div(a: u32, b: u32) → u32 ! ∅ ⊳ b > 0 { a / b }

ƒ main() → u32 ! ∅ { safe_div(1, 0) }
```

Một điều kiện nhánh chứng minh được `⊳` cho người gọi:

```grit value=4
ƒ safe_div(a: u32, b: u32) → u32 ! ∅ ⊳ b > 0 { a / b }

ƒ run(x: u32, y: u32) → u32 ! ∅ {
  ⎇ y > 0 { safe_div(x, y) } ⇝ { 0 }
}

ƒ main() → u32 ! ∅ { run(9, 0) + run(9, 2) }
```

Còn nếu `run` gọi `safe_div(x, y)` mà không có điều kiện bảo vệ, `gritc` không chứng minh được `y > 0`: nó vẫn cho qua, nhưng giữ kiểm tra lúc chạy (`N0901`):

```grit
ƒ safe_div(a: u32, b: u32) → u32 ! ∅ ⊳ b > 0 { a / b }

ƒ run(x: u32, y: u32) → u32 ! ∅ { safe_div(x, y) }

ƒ main() → u32 ! ∅ { run(9, 0) }
```

Khối trên qua `check` (ghi chú `N0901`), nhưng chạy sẽ in `abort: precondition` và thoát mã 70. Bất biến vòng lặp `⋈` cũng chỉ là một nghĩa vụ như vậy:

```grit value=50
ƒ count(n: u32) → u32 ! ∅ ⊳ n < 100 ⊲ result == n {
  μi ≔ 0u32;
  ⟳ i < n ↓ n - i ⋈ i <= n {
    i ← i + 1;
  }
  i
}

ƒ main() → u32 ! ∅ { count(50) }
```

Hợp đồng không phải chứng minh tính đúng đắn của chương trình theo ý bạn muốn: nó chỉ kiểm điều *bạn viết ra* cộng các bất biến an toàn có sẵn (tràn, biên, mượn, dừng, hiệu ứng). Viết ít, viết đúng chỗ.

**Bài tập 9.** Viết `diff(a: u32, b: u32) → u32` với `⊳ a >= b` và `⊲ result <= a`, thân `a - b`; `main` gọi `diff(10, 3)` (kết quả `7`). Đáp án:

```grit value=7
ƒ diff(a: u32, b: u32) → u32 ! ∅ ⊳ a >= b ⊲ result <= a {
  a - b
}

ƒ main() → u32 ! ∅ { diff(10, 3) }
```

Ở đây phép trừ không tràn vì `⊳ a >= b`, nên bộ giải xoá kiểm tra tràn.

---

## Chương 10. Đọc thông báo lỗi và cách sửa

Một thông báo có dạng:

```text
error[E0302]: ép kiểu số ngầm bị cấm (D1): giá trị kiểu `i32` được dùng ở chỗ cần `i64`; viết `as i64` nếu đó là ý định
 --> p5.gt:4:3
  |
4 |   a + b
  |   ^^^^^
  = help: ép kiểu tường minh: ` as i64` (maybe-incorrect)
          - 4:8-4:8: `` -> ` as i64`
gritc: phán quyết red; 2 lỗi, 0 cảnh báo, 0 ghi chú; backend: interpreter
```

- **Mã** `E0302`: ổn định, tra bằng `gritc explain E0302` hoặc `docs/errors/E0302.md`. Chữ đầu là mức: `E` ĐỎ (không chạy, thoát 1), `S` VÀNG (cảnh báo bảo mật như `S101` bí mật trong nguồn, `S201` `declassify`; chương trình vẫn chạy, trừ khi `--deny-yellow`), `N` ghi chú (không ảnh hưởng, nhưng cho thấy chi phí, ví dụ `N0601`), `E99xx` lỗi nội bộ của compiler (thoát 2, không bao giờ chạy).
- **Vị trí** `tệp:dòng:cột` và đoạn mã có `^^^` chỉ chỗ.
- **`= help`** là gợi ý sửa có cấu trúc: danh sách *edit* (`vị trí -> thay bằng`) và độ tin cậy: `machine-applicable` (áp dụng máy móc được) hay `maybe-incorrect` (cần bạn xét).
- **Phán quyết** cuối: `green` / `yellow` / `red` / `ice`.

Mã thoát của `gritc`: `0` qua; `1` ĐỎ; `2` lỗi nội bộ; `3` lỗi dùng hoặc I/O hoặc backend chưa hỗ trợ. Với `gritc run`, chương trình *đã* chạy: `0` bình thường, `70` abort (tràn, ngoài biên, `⊳` sai, hết nhiên liệu…), `71` hết giới hạn bộ chạy, `100+s` chương trình tự thoát với mã `s` (chương 12).

Máy và AI nên đọc `--json`: mỗi chẩn đoán có `code`, `level`, `message`, `span`, và `suggestions[].edits[]` (vị trí byte, chuỗi thay thế). Schema ở `docs/spec/diag_schema.json`.

```text
gritc check --json tệp.gt
{"schema_version":1,"tool":"gritc","diagnostics":[{"code":"E0302","level":"error",
 "message":"…","file":"p5.gt","span":{"start":{"offset":56,"line":4,"col":3},…},
 "suggestions":[{"applicability":"maybe-incorrect","edits":[{"replacement":" as i64",…}]}]}],…}
```

**Các mã bạn gặp nhiều nhất:**

| Mã | Nghĩa | Cách sửa thường gặp |
|---|---|---|
| E0101 | lỗi cú pháp | xem ký hiệu bị thiếu/thừa; `x as T < y` cần ngoặc; không có `+=`; `=` đứng riêng |
| E0201 / E0203 | tên chưa định nghĩa / item private | `⇲` đúng module; thêm `◈` |
| E0205 | shadowing | đổi tên (chương 12) |
| E0301 / E0302 | sai kiểu / ép kiểu ngầm | `as` tường minh; `⌥` thay `==` cho `Option` |
| E0305 | `⌥` không đầy đủ | thêm nhánh `_` |
| E0307 | thiếu đường trả về (hay gặp: `;` thừa ở cuối) | bỏ `;` hoặc dùng `⏎` |
| E0308 | gán vào biến bất biến | thêm `μ` |
| E0310 | gọi `root_*` ngoài `main` | nhận `&Cap<…>` qua tham số |
| E0315 | không suy ra vùng đời | ghi `'a` |
| E0318 | `Vec` chứa kiểu cần drop | dùng kiểu `Copy` |
| E0401–E0406 | move/mượn/Arena | rút ngắn khoản mượn; đừng dùng sau move |
| E0601–E0603, E0901/2 | số học, biên, hợp đồng | thêm điều kiện bảo vệ hoặc `⊳` |
| E0701–E0703 | không chứng minh được dừng | `↓`, hoặc nhiên liệu (chương 4) |
| E0801 | thiếu hiệu ứng khai báo | thêm hiệu ứng vào chữ ký (và vào hàm gọi) |

**Một phiên sửa lỗi thật.** Hàm tính tổng một lát:

```grit fail=E0301
ƒ total(xs: &[u32]) → u64 ! ∅ {
  μt ≔ 0u64;
  ⟳ x in xs {
    t ← t + x;
  }
  t;
}

ƒ main() → u64 ! ∅ {
  arr ≔ [1u32, 2, 3];
  total(&arr[..])
}
```

Lỗi đầu: `E0301 sai kiểu: cần u64, gặp &u32`. Phần tử của `⟳ x in xs` trên `&[u32]` là `&u32`; phải giải tham chiếu và ép kiểu: `t + (*x as u64)`. Sửa xong, compiler (theo từng giai đoạn) báo lỗi kế tiếp:

```grit fail=E0307
ƒ total(xs: &[u32]) → u64 ! ∅ {
  μt ≔ 0u64;
  ⟳ x in xs {
    t ← t + (*x as u64);
  }
  t;
}

ƒ main() → u64 ! ∅ {
  arr ≔ [1u32, 2, 3];
  total(&arr[..])
}
```

`E0307`: khối kết thúc bằng `t;` nên không trả giá trị. Bỏ `;`:

```grit value=6
ƒ total(xs: &[u32]) → u64 ! ∅ {
  μt ≔ 0u64;
  ⟳ x in xs {
    t ← t + (*x as u64);
  }
  t
}

ƒ main() → u64 ! ∅ {
  arr ≔ [1u32, 2, 3];
  total(&arr[..])
}
```

Hai bài học: lỗi hiện theo giai đoạn (kiểu trước, rồi đường trả về, rồi mượn, nghĩa vụ, dừng/hiệu ứng), nên sửa xong một lớp có thể lộ lớp sau; và `&[u32; 3]` **không** tự thành `&[u32]` khi truyền vào hàm, hãy cắt `&arr[..]`.

**Bài tập 10.** Chương trình sau bị từ chối với mã nào, và sửa thế nào?

```text
ƒ main() → i32 ! ∅ {
  μn ≔ 5;
  n += 1;
  n
}
```

Đáp án: `E0101` (`=` đứng riêng không hợp lệ). Grit không có gán kết hợp; viết `n ← n + 1`.

```grit value=6
ƒ main() → i32 ! ∅ {
  μn ≔ 5;
  n ← n + 1;
  n
}
```

---

## Chương 11. Nhiều tệp với Rig

**Rig** (`rig`) quản lý một dự án: `rig.toml` (tên, phụ thuộc), `rig.lock` (băm nội dung từng gói), thư mục `src/`. Quy ước: **tên tệp là tên module**. `src/geom.gt` là module `geom`; `src/main.gt` là gốc có `main`.

```text
rig new demo        tạo gói demo (rig.toml, src/main.gt, rig.lock), tự thêm phụ thuộc `std`
cd demo
# sửa/thêm tệp trong src/
rig lock            ghi lại rig.lock (BẮT BUỘC sau mỗi lần sửa mã)
rig check           lock + gritc check cả đồ thị gói
rig run             build rồi chạy
rig build           dựng target/<tên>.exe
rig test            mỗi tests/*.gt là một chương trình có main; thoát 0 là qua
rig fmt             định dạng src/ và tests/
```

Rig từ chối build khi `rig.lock` lệch nội dung (mã thoát 10); thông báo nhắc bạn chạy `rig lock`. Đó là chủ ý, không phải lỗi. Tệp `target/<tên>.exe` có đuôi `.exe` cả trên Linux. Phụ thuộc `std` trong `rig.toml` là đường dẫn tới thư mục `lib/std` trên máy bạn:

```text
[package]
name = "demo"
version = "0.1.0"

[dependencies]
std = { path = "/đường/dẫn/tới/Grit/lib/std" }
```

Ví dụ hai tệp, dùng `std`. Tệp `tok.gt` (module `tok`) tách các số cách nhau bởi khoảng trắng:

```grit file=tok.gt group=ch11 std
// module `tok` (tên tệp = tên module): tách các số thập phân cách nhau bởi khoảng trắng.
// Trả false nếu có token không phải số; các số đọc được vẫn được đẩy vào `dst`.
◈ ƒ parse_all<'a>(text: &str, dst: &!Vec<'a, u64>) → bool ! {mem} {
  b ≔ text.as_bytes();
  n ≔ b.len();
  μstart ≔ 0usize;
  μin_tok ≔ false;
  μok ≔ true;
  ⟳ i in 0..n {
    ws ≔ std::is_ascii_ws(b[i]);
    ⎇ ws ∧ in_tok {
      ⌥ b[start..i].parse_u64() {
        Ok(v) ↦ { dst.push(v); },
        Err(_) ↦ { ok ← false; },
      }
      in_tok ← false;
    } ⇝ { }
    ⎇ ¬ws ∧ ¬in_tok {
      start ← i;
      in_tok ← true;
    } ⇝ { }
  }
  ⎇ in_tok {
    ⌥ b[start..n].parse_u64() {
      Ok(v) ↦ { dst.push(v); },
      Err(_) ↦ { ok ← false; },
    }
  } ⇝ { }
  ok
}
```

Tệp `main.gt` đếm tần suất bằng `std::HashMap` và in số có dấu bằng `write_i64`:

```grit file=main.gt group=ch11 std run
⇲ tok::parse_all;

ƒ main() ! {io, mem, div} {
  out ≔ root_out();
  μa ≔ Arena::new();
  μnums ≔ Vec::new(&a);
  ok ≔ parse_all("3 1 3 2 3 1", &!nums);
  ⎇ ¬ok { core::exit(2); } ⇝ { }

  μcounts ≔ std::HashMap::new(&a);
  s ≔ nums.as_slice();
  ⟳ x in s {
    prev ≔ counts.get(x).unwrap_or(0);
    counts.insert(*x, prev + 1);
  }
  ⟳ j in 0..counts.entry_count() {
    ⌥ counts.entry_at(j) {
      Some((k, c)) ↦ {
        out.write_u64(k);
        out.write_str(" x");
        out.write_u64(c);
        out.write_str("\n");
      },
      None ↦ { },
    }
  }
  first ≔ s[0];
  last ≔ s[s.len() - 1];
  out.write_i64((last as i64) - (first as i64));
  out.write_str("\n");
}
```

```output
3 x3
1 x2
2 x1
-2
```

Những điều rút ra từ ví dụ:

- `main` có `div` vì gọi `core::exit` (chương 5). Thiếu nó là E0801.
- Một Arena `a` phục vụ cả `Vec` lẫn `HashMap`; `dst: &!Vec<'a, u64>` là mượn độc quyền nên `parse_all` được đẩy phần tử vào.
- Tên của `std` gọi là `std::tên` (hàm tự do như `std::is_ascii_ws`, `std::max`, `std::HashMap::new`); *phương thức* trên kiểu có sẵn (`parse_u64`, `unwrap_or`, `get`) dùng trực tiếp khi `std` có trong đồ thị gói.
- Thứ tự duyệt `HashMap` bằng `entry_at` là thứ tự chèn, không phải thứ tự khoá.

**Gói `std` (viết bằng Grit thường)** gồm: `Vec` (`get first last pop clear extend_from_slice insert remove contains is_empty`), `[u8]`/`str` (`starts_with ends_with find find_byte equals trim_ascii parse_u64 parse_i64`, trả `Result<_, std::ParseError>` với `Empty`/`Invalid`/`Overflow`), in số (`Cap<Out>::write_i64 write_hex write_byte`, `Vec<u8>::push_str push_u64 push_i64`), `[T]::sort` (heapsort, *không* ổn định), `min`/`max`, `abs_i64`, `pow_u64` (trả `Option`), `HashMap<K, V>` (khoá/giá trị `Copy`, khoá có `Ord` và `Hash`), và hàm trợ giúp cho `Option`/`Result`/`Ordering`. Tra chữ ký ở `docs/STD_REFERENCE.md`.

```grit run std
ƒ show(out: &Cap<Out>, r: Result<i64, std::ParseError>) ! {io} {
  ⌥ r {
    Ok(n) ↦ { out.write_i64(n); },
    Err(std::ParseError::Empty) ↦ { out.write_str("rong"); },
    Err(std::ParseError::Invalid) ↦ { out.write_str("sai"); },
    Err(std::ParseError::Overflow) ↦ { out.write_str("tran"); },
  }
  out.write_str("\n");
}

ƒ main() ! {io, mem} {
  out ≔ root_out();
  show(&out, "-42".parse_i64());
  show(&out, "".parse_i64());
  show(&out, "12a".parse_i64());
  show(&out, "99999999999999999999".parse_i64());

  μa ≔ Arena::new();
  μv ≔ Vec::new(&a);
  v.push(5u32);
  v.push(2u32);
  v.push(9u32);
  v.push(1u32);
  v.as_mut_slice().sort();
  ⟳ x in v.as_slice() {
    out.write_u64(*x as u64);
    out.write_str(" ");
  }
  out.write_str("\n");
}
```

```output
-42
rong
sai
tran
1 2 5 9 
```

Một lỗi hay gặp khi tách module: item mặc định private. Tệp `geom.gt` thiếu `◈`:

```grit file=geom.gt group=priv fail=E0203
▢ Point { x: i64, y: i64 }

◈ ƒ origin() → Point ! ∅ { Point { x: 0, y: 0 } }
```

```grit file=main.gt group=priv
⇲ geom::{Point, origin};

ƒ main() → i64 ! ∅ {
  p ≔ origin();
  p.x
}
```

```text
error[E0203]: `Point` là private trong module `geom` (thiếu `◈`)
```

Thêm `◈` vào `▢ Point` (và `◈` vào từng trường nếu module khác cần đọc `p.x`).

**Bài tập 11.** Trong dự án hai tệp, đặt hàm `cube(x: u64) → u64` vào `math.gt` (module `math`), công khai, và gọi nó từ `main.gt` (`cube(3) = 27`). Đáp án:

```grit file=math.gt group=ex11
◈ ƒ cube(x: u64) → u64 ! ∅ ⊳ x < 1000 { x * x * x }
```

```grit file=main.gt group=ex11 value=27
⇲ math::cube;

ƒ main() → u64 ! ∅ { cube(3) }
```

---

## Chương 12. Bẫy thường gặp

**1. Không có shadowing (D7).** Khai báo lại một tên (kể cả trong khối con) là E0205, và tên của item cấp module hay của prelude cũng không được che. Đây là lý do mã `std` đặt tên như `buf_`, `first_`.

```grit fail=E0205
ƒ main() → i32 ! ∅ {
  x ≔ 1;
  y ≔ ⎇ x > 0 { x ≔ 2; x } ⇝ { 0 };
  x + y
}
```

```grit fail=E0205
ƒ main() → i32 ! ∅ {
  band ≔ 1;
  band
}
```

(`band` là hàm của prelude.) Cũng đừng tự định nghĩa enum có biến thể tên `Ok`/`Err`/`Some`/`None`: chúng che biến thể của prelude.

**2. Thứ tự đánh giá là trái sang phải, và gán khác Rust.** Với `f() + g() * h()` thì `f`, `g`, `h` chạy theo thứ tự viết. Với gán `chỗ ← giá_trị`, các biểu thức con của *chỗ* (chỉ số) chạy trước, rồi mới đến giá trị. Rust làm ngược (vế phải trước).

```grit run
ƒ say(out: &Cap<Out>, tag: &str, v: u64) → u64 ! {io} {
  out.write_str(tag);
  v
}

ƒ main() ! {io} {
  out ≔ root_out();
  r ≔ say(&out, "a", 1) + say(&out, "b", 2) * say(&out, "c", 3);
  out.write_str("=");
  out.write_u64(r);
  out.write_str("\n");
  μarr ≔ [0u64; 3];
  arr[say(&out, "i", 1) as usize] ← say(&out, "v", 9);
  out.write_str("|");
  out.write_u64(arr[1]);
  out.write_str("\n");
}
```

```output
abc=7
iv|9
```

**3. Tên tệp là tên module.** `src/geom.gt` là `geom` (chương 11), item mặc định private, thứ tự khai báo không quan trọng, và vòng phụ thuộc module là E0204. Với `gritc run tệp.gt` lẻ, tệp đó là module gốc bất kể tên.

**4. `main` trả giá trị KHÔNG thành mã thoát.** `gritc run` in giá trị `main` trả về, nhưng tiến trình dựng ra (`gritc build`) thoát `0`. Muốn có mã thoát, gọi `core::exit(code)` (kiểu `→ !`, hiệu ứng `div`, mã bị kẹp ở 255).

```grit run exit=3
ƒ main() ! {div} {
  core::exit(3);
}
```

Tiến trình dựng từ khối trên thoát với mã `3`. Dưới `gritc run`, mã đó hiện thành `103` (`100 + 3`) để không lẫn với mã ĐỎ của `gritc`.

**5. `==` chỉ cho số, `bool`, `char` và `&str`.** Không có `==` cho `Option`, tuple, struct (E0301); số thực cấm hẳn (E0303, vì NaN). So sánh kiểu ghép bằng `⌥`, số thực bằng ngưỡng sai số.

```grit fail=E0301
ƒ main() → bool ! ∅ {
  a ≔ Some(1);
  b ≔ Some(1);
  a == b
}
```

```grit fail=E0303
ƒ main() → bool ! ∅ {
  x ≔ 1.5;
  x == 1.5
}
```

**6. `Option::unwrap` và `is_some` không chạy được.** Chúng là khai báo trong prelude (kèm `⊳ self.is_some()`) chưa có thân để sinh mã. Khối sau qua `check` nhưng `gritc run` báo "backend llvm chưa hỗ trợ: hàm không có thân MIR" (thoát 3). Dùng `⌥`, hoặc `unwrap_or`/`is_none` của `std`.

```grit
ƒ main() → i32 ! ∅ {
  o ≔ Some(5);
  ⎇ o.is_some() { o.unwrap() } ⇝ { 0 }
}
```

```grit value=5
ƒ main() → i32 ! ∅ {
  o ≔ Some(5);
  ⌥ o { Some(v) ↦ v, None ↦ 0 }
}
```

**7. Vài cú pháp dễ vấp.**

- `x as i64 < y` bị đọc là tham số kiểu; viết `(x as i64) < y` (E0101, có gợi ý máy áp dụng được).
- `x<-1` là gán; viết `x < -1`.
- Điều kiện/đối tượng của `⎇ ⟳ ⌥` không được là literal struct trần: bọc ngoặc `⌥ (P { x: 1 }) { … }`.
- Biểu thức cuối có `;` là câu lệnh, hàm trả giá trị mà kết thúc bằng `;` là E0307.

```grit fail=E0101
ƒ main() → bool ! ∅ {
  a ≔ 5i32;
  b ≔ 7i64;
  a as i64 < b
}
```

```grit fail=E0101
▢ P { x: i32 }

ƒ main() → i32 ! ∅ {
  ⌥ P { x: 1 } { P { x: v } ↦ v }
}
```

**8. Quan hệ kiểu khác.** `⌥` trên số nguyên cần nhánh `_` (E0305). Literal số không hậu tố tự suy kiểu, nhưng hai biến khác kiểu thì không bao giờ trộn được (E0302). Một `Vec<T>` chỉ dùng được hàm của `std` khi `T: Copy`.

---

## Giới hạn, nói thẳng

- **Kiểm tra, không phải bằng chứng toán học về compiler.** Một chương trình qua `gritc` nghĩa là nó qua các kiểm tra đã liệt kê (kiểu, mượn, nghĩa vụ số học, dừng và hiệu ứng, bí mật, IR Guard). Độ tin cậy của chính compiler dựa vào kho mẫu, so sánh với bản tham chiếu độc lập (oracle) và kiểm thử đột biến, và vẫn có thể có lỗi. Nhớ rằng một kiểm tra *không chứng minh được* được giữ lại lúc chạy chứ không bị bỏ qua, và có một số nghĩa vụ nằm ngoài tầm bộ giải hiện tại.
- **Sound, không complete.** Có chương trình đúng bị từ chối (chương 4: `gcd` đệ quy, Collatz). Cách né là viết lại (nhiên liệu, điều kiện bảo vệ), không phải tắt kiểm tra.
- **Chưa có:** closure, macro, `dyn`/trait object, iterator tổng quát (`⟳ x in` chỉ nhận range và lát/mảng), or-pattern, guard trong `⌥`, destructure ở `≔`, `f32`, toán tử bit (dùng hàm `band bor bxor bnot shl shr` trên `u32`), `+=`, chuỗi nhiều dòng, `/* */`, FFI cho người dùng, `⚠` (Phase 1 gần như luôn E1001).
- **Giới hạn đã gặp khi viết sách này** (đã thử): `Cap<Fs>::sub` và `Option::unwrap`/`is_some` có chữ ký nhưng chưa chạy được; `&[T; N]` không tự thành `&[T]`; số vòng của một vòng lặp dựa trên phép chia hay nhân không tự chứng minh được dừng.
- **Nền đã kiểm:** Linux x86_64. Windows: chưa kiểm. Hai backend (thông dịch MIR và LLVM) và các hồ sơ `dev`, `release`, `release-o2` có đối chiếu thực nghiệm; đó là bằng chứng thực nghiệm, không phải chứng minh.

**Đọc tiếp:** `docs/spec/symbols.md` (bảng ký hiệu đầy đủ), `docs/spec/semantics.md` (ngữ nghĩa), `docs/spec/regions.md` (vùng đời), `docs/STD_REFERENCE.md` (thư viện), `docs/errors/` (mỗi mã lỗi một trang), `docs/KNOWN_LIMITATIONS.md`, và các chương trình thật trong `examples/` (wordfreq, json, sha256, sortlines, stats) cùng đề bài ở `docs/CAPSTONE.md`.

**Kiểm tài liệu này.** Mỗi khối ```` ```grit ```` ở trên là một chương trình đầy đủ và được chạy qua:

```text
cd /đường/dẫn/tới/Grit
GRITC=/đường/dẫn/tới/gritc python3 tools/doc_test.py docs/GRIT_BOOK.md
```

Kết quả mong đợi: `DOC_TEST: N khối grit, 0 sai`. Các thuộc tính trên dòng mở hàng rào: không có = phải qua `gritc check`; `value=N` = `main` phải trả `N`; `run` + khối `output` = stdout phải khớp từng byte; `fail=E0xxx` = ví dụ lỗi cố ý, phải bị từ chối đúng mã đó; `std` = thêm gói `std`; `file=… group=…` = nhiều khối ghép thành một chương trình nhiều tệp.
