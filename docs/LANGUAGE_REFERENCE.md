# Tham chiếu ngôn ngữ Grit

Tài liệu tra cứu hợp nhất cho ngôn ngữ **Grit** (ngôn ngữ thiết kế để AI viết mã) và trình biên dịch `gritc`. Tổng hợp từ `docs/spec/*` (grammar, semantics, symbols, regions, error_codes), `docs/errors/*.md`, `GRIT_PLAN.md` (D1–D29), `lib/core/prelude.gt` và `docs/KNOWN_LIMITATIONS.md`. Khi tài liệu này và hành vi thật của `gritc` khác nhau, tài liệu ghi **hành vi thật** và nêu sự khác biệt ở [mục 18.3](#s18-3).

**Cách tài liệu được kiểm.** Mọi khối ` ```grit ` trong tệp này là chương trình đầy đủ được máy kiểm bằng `tools/doc_test.py`: khối thường phải qua `gritc check`; `value=N` nghĩa là `main` trả đúng `N`; `fail=Exxxx` nghĩa là chương trình **phải bị từ chối đúng mã đó**. Đoạn không phải chương trình đầy đủ dùng hàng rào trần và không được kiểm.

```text
GRITC=/tmp/gritbin/gritc python3 tools/doc_test.py docs/LANGUAGE_REFERENCE.md
```

**Mức đảm bảo (nói thẳng).** An toàn của Grit đến từ các bước **kiểm tra** (kiểu, mượn, nghĩa vụ, dừng/hiệu ứng/đa luồng, bí mật, IR Guard), cộng corpus kiểm thử, oracle động, đột biến và so sánh thông dịch với mã máy. Đó là bằng chứng thực nghiệm, **không phải chứng minh hình thức**; còn đột biến chưa giải quyết được liệt kê ở [mục 18](#s18). Nền đã kiểm: **Linux x86_64**; Windows: **chưa kiểm**. Hai backend: `llvm` (mặc định khi bản dựng có) và `interpreter` (thông dịch MIR, chỉ là công cụ kiểm thử, không phải bằng chứng cho ngữ nghĩa codegen).

## Mục lục

1. [Từ vựng và ký hiệu](#s1)
2. [Kiểu](#s2)
3. [Biểu thức và toán tử](#s3)
4. [Câu lệnh và vòng lặp](#s4)
5. [Khớp mẫu `⌥`](#s5)
6. [Hàm, hiệu ứng, hợp đồng](#s6)
7. [Struct, enum, generics, trait, impl](#s7)
8. [Sở hữu, mượn và vùng đời (M6)](#s8)
9. [Arena, Vec, Text](#s9)
10. [Capability](#s10)
11. [Bí mật (`Secret`)](#s11)
12. [Đa luồng](#s12)
13. [Module và visibility](#s13)
14. [Không shadowing (D7)](#s14)
15. [Các pass của gritc](#s15)
16. [Mã thoát của gritc và rig](#s16)
17. [Mã lỗi](#s17)
18. [Giới hạn đã biết](#s18)
19. [Phụ lục: prelude tóm tắt](#s19)

---

<a id="s1"></a>
## 1. Từ vựng và ký hiệu

### 1.1 Nguồn

- UTF-8, dạng chuẩn **NFC** (không phải NFC: `E1204`). Xuống dòng là `\n` hoặc `\r\n`; `\r` lẻ là `E1202`.
- Cấm cứng (D15, ở **mọi nơi**, kể cả trong chuỗi và comment): ký tự bidi `U+202A–202E`, `U+2066–2069` (và LRM/RLM/ALM) là `E1201`; ký tự vô hình/zero-width/điều khiển/khoảng trắng Unicode lạ (`U+200B–200D`, `U+2060`, `U+FEFF` kể cả BOM) là `E1202`; định danh trộn hai bảng chữ là `E1203`. Muốn có ký tự đặc biệt trong chuỗi, dùng escape `\u{…}`.
- Định danh: `XID_Start XID_Continue*`, trừ từ khoá cứng và ký hiệu dành riêng. Chưa phát hiện homoglyph "toàn một bảng chữ lạ" (ví dụ toàn Cyrillic trông như Latin): xem [18.2](#s18-2).
- Phân biệt hoa/thường. Thứ tự khai báo item không quan trọng.

### 1.2 Bảng ký hiệu: canonical ↔ ASCII (1–1)

Lexer chấp nhận cả hai dạng và quy về canonical **trước** khi parse; `gritc fmt` luôn in canonical. Hai dạng cho cùng AST. Bí danh `\tên` chỉ hợp lệ khi `\` đi liền chữ cái ASCII thường (`\fn`, `\loop`…); tên không có trong bảng là `E0101`. Toán tử ASCII dùng khớp dài nhất.

| Ý nghĩa | Canonical | ASCII | Ý nghĩa | Canonical | ASCII |
|---|---|---|---|---|---|
| hàm | `ƒ` | `\fn` | struct | `▢` | `\struct` |
| bind bất biến | `≔` | `:=` | enum | `⊔` | `\enum` |
| tiền tố khả biến | `μ` | `\mut` | module | `▦` | `\mod` |
| ghi đè | `←` | `<-` | use (nhập) | `⇲` | `\use` |
| kiểu trả về | `→` | `->` | pub | `◈` | `\pub` |
| vòng lặp | `⟳` | `\loop` | trait | `⊏` | `\trait` |
| biến thể giảm | `↓` | `\dec` | impl | `⊐` | `\impl` |
| tiền điều kiện | `⊳` | `\pre` | break | `⊠` | `\break` |
| hậu điều kiện | `⊲` | `\post` | continue | `⊡` | `\cont` |
| bất biến vòng | `⋈` | `\inv` | type alias | `≡` | `\type` |
| khối unsafe | `⚠` | `\unsafe` | hằng | `□` | `\const` |
| mệnh đề bằng chứng | `⊢` | `\proof` | static | `■` | `\static` |
| tập hiệu ứng rỗng | `∅` | `\nil` | và / hoặc / phủ định | `∧ ∨ ¬` | `\land \lor \neg` |
| if | `⎇` | `\if` | suy ra | `⇒` | `\implies` |
| else | `⇝` | `\else` | với mọi / tồn tại | `∀ ∃` | `\forall \exists` |
| match | `⌥` | `\match` | so sánh | `≤ ≥ ≠` | `<= >= !=` |
| nhánh match | `↦` | `=>` | thuộc | `∈` | `\in` |
| return | `⏎` | `\ret` | con trỏ thô | `*□ T`, `*μ T` | `*\const T`, `*\mut T` |

Không có canonical (chỉ ASCII): `&` `&!` (mượn chung/duy nhất), `*` (deref và nhân), `+ - * / %`, `+% -% *%` (quấn vòng), `+^ -^ *^` (bão hoà), `+? -? *?` (trả `Option`), `==`, `< >`, `..` `..=`, `'a` (lifetime), `#attr`, `//` `///`.

Cùng một hàm, hai bề mặt, cùng kết quả:

```grit value=55
\fn fib(n: u32) -> u64 ! \nil \dec n { \if n < 2u32 { n as u64 } \else { fib(n - 1u32) + fib(n - 2u32) } }
\fn main() -> u64 ! \nil { fib(10u32) }
```

```grit value=55
ƒ fib(n: u32) → u64 ! ∅ ↓ n { ⎇ n < 2u32 { n as u64 } ⇝ { fib(n - 1u32) + fib(n - 2u32) } }
ƒ main() → u64 ! ∅ { fib(10u32) }
```

Bí danh không tồn tại bị từ chối:

```grit fail=E0101
\fn main() -> i32 ! \nil { \foo 1 }
```

### 1.3 Từ khoá cứng và ký hiệu dành riêng

| Token | Vai trò |
|---|---|
| `self` | tham số đối tượng trong `⊏`/`⊐` (`self`, `&self`, `&!self`); cũng là biểu thức `self.x` |
| `Self` | kiểu của `⊏`/`⊐` đang viết |
| `as` | ép kiểu tường minh `e as T` |
| `true` `false` | hằng bool (không có `⊤ ⊥`) |
| `for` | trong `⊐ Trait for Kiểu { … }` |
| `in` | trong `⟳ x in xs { … }` (khác `∈`/`\in` của hợp đồng) |
| `μ` | luôn là `KW_MUT`: `μs` = `μ s`; không có định danh nào chứa `μ` (`µ` U+00B5 bị chặn) |
| `\extern`, `\where` | **dành riêng** (Phase 2), mọi chỗ dùng là `E0101` |

Dùng từ khoá làm tên (`self ≔ 1;`) là `E0101`. `ƒ` cũng là ký hiệu dành riêng, không nằm trong định danh.

```grit fail=E0101
ƒ main() → i32 ! ∅ { self ≔ 1; self }
```

### 1.4 Literal và comment

| Loại | Quy tắc | Ví dụ |
|---|---|---|
| số nguyên | thập phân, `0x`, `0o`, `0b`; `_` chỉ giữa các chữ số; hậu tố `i8…i128 isize u8…u128 usize` | `1_000u32`, `0xFF`, `0b1010` |
| số thực | `d.d`, `d e d`, hậu tố `f64` (`f32` bị `E0319`); `1.` và `.5` không hợp lệ | `1.5`, `2.5e-3` |
| ký tự | đúng một codepoint, `'x'` | `'a'`, `'\n'`, `'\u{e9}'` |
| chuỗi | UTF-8, không xuống dòng thô, không nội suy; escape `\n \r \t \0 \\ \' \" \u{hex}`, escape lạ là `E0101` | `"xin chào\n"` |
| lifetime | `'` + định danh ASCII, không có dấu đóng | `'a`, `'r`, `'static` |
| range | `a..b` (không gồm `b`), `a..=b` (gồm `b`) | `0..n` |
| comment | `//` (giữ trong bảng trivia cho `gritc fmt`), `///` doc comment gắn vào node kế tiếp; `/* */` **cấm** (`E0101`) | |

Quy tắc phân biệt: `'x'` (có dấu đóng) là ký tự, `'a` (không có dấu đóng) là lifetime; `'ab'` và `''` là `E0101`. `-128i8` hợp lệ (dấu `-` áp trực tiếp lên literal được gấp trước khi kiểm phạm vi); literal ngoài phạm vi (`256u8`) là `E0601`. `1000_u32` không hợp lệ (`_` không liền hậu tố).

```grit value=1280
ƒ main() → u32 ! ∅ {
  a ≔ 1_000u32;
  b ≔ 0xFFu32;
  c ≔ 0b1010u32;
  d ≔ 0o17u32;
  a + b + c + d
}
```

```grit fail=E0601
ƒ main() → u8 ! ∅ { 256u8 }
```

### 1.5 Quy tắc lexer dễ vấp

- **`*`** là một token; parser phân biệt theo vị trí: ở đầu toán hạng là deref (`*p * *q` = `(*p) * (*q)`), sau toán hạng đầy đủ là nhân.
- **`/*p`** (chia cho deref không khoảng trắng) bị đọc là mở comment khối ⇒ `E0101`; viết `a / *p`.
- **`x<-1`** lex thành `x <- 1` (gán); muốn so sánh với số âm viết `x < -1`.
- **Không có `+=`, `-=`…**: chỉ `x ← x + 1`. `=` đứng riêng là `E0101`; so sánh bằng là `==`.
- **Không có toán tử bit**: dùng hàm `band bor bxor bnot shl shr rotr rotl` (chỉ cho `u32`). `|`, `^`, `~` đứng riêng là `E0101`.
- **`x as T < y`**: sau `as`, `<` mở danh sách tham số kiểu ⇒ `E0101` (có gợi ý máy áp dụng được). Viết `(x as T) < y`.
- Chuỗi hợp lệ `"a\tb"`; tuple lồng `t.0.1` = `t . 0 . 1`.

```grit fail=E0101
ƒ main() → bool ! ∅ { x ≔ 1i32; x as i64 < 3i64 }
```

```grit value=1
ƒ main() → i32 ! ∅ { x ≔ 1i32; ⎇ (x as i64) < 3i64 { 1 } ⇝ { 0 } }
```

```grit fail=E0101
ƒ main() → i32 ! ∅ { μx ≔ 1; x += 1; x }
```

### 1.6 Độ sâu

Độ lồng (biểu thức, khối, kiểu, pattern, `▦`) cộng độ dài chuỗi toán tử không vượt `MAX_DEPTH = 256`; vượt là `E0102` (ĐỎ, không phải ICE): tách biểu thức con ra biến phụ bằng `≔`.

---

<a id="s2"></a>
## 2. Kiểu

| Loại | Cú pháp | Ghi chú |
|---|---|---|
| nguyên | `i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize` | `isize`/`usize` rộng bằng con trỏ (x86-64: 64 bit) |
| thực | `f64` | `f32` **dành riêng** (Phase 2): mọi dùng là `E0319`; `%` trên số thực cũng `E0319` |
| khác | `bool`, `char` (một Unicode scalar), `str` | `str` không biết kích thước, chỉ dùng sau `&`/`&!` |
| unit, tuple | `()`, `(A, B)`, `(A,)` | truy cập `t.0` |
| mảng | `[T; N]` | `N` là hằng lúc biên dịch |
| lát | `[T]` | chỉ hợp lệ sau `&`/`&!`: `&[T]`, `&![T]` |
| tham chiếu | `&T`, `&!T`, `&'a T`, `&!'a T` | chung / duy nhất, xem [mục 8](#s8) |
| hàm tĩnh | `ƒ(A, B) → R ! E` | không bắt biến, không có closure |
| never | `!` | không bao giờ trả về; gán được cho mọi kiểu |
| con trỏ thô | `*□ T`, `*μ T` | chỉ trong `⚠`; ngoài `⚠` là `E1001` |
| đường dẫn | `Option<T>`, `Result<T, E>`, `Vec<'a, T>`, `Cap<Fs>`, `Secret<T>`, `Mutex<T, L>`… | đối số lifetime **đứng trước** đối số kiểu: `Pair<'a, T>` |

Quy tắc nền:

- Không `null`, không biến chưa khởi tạo, không exception, không unwinding (D3). Lỗi khôi phục được dùng `Option`/`Result`; lỗi không khôi phục là **abort tức thì**.
- **Không ép kiểu ngầm giữa các kiểu số** (D1): `E0302`. Chữ ký hàm luôn tường minh; suy luận chỉ cục bộ.
- `Option<T>` và `Result<T, E>` là enum thường trong prelude, không phải kiểu đặc biệt của compiler. Chúng **không** `Copy` (kể cả `Option<i32>`): dùng hai lần là `E0401`.
- `≡ Tên<…> ≔ Kiểu;` là bí danh (không tạo kiểu mới). `□ TÊN: T ≔ e;` là hằng lúc biên dịch. `■ TÊN: T ≔ e;` là static **bất biến** (không có `■ μ`, D11).
- Bất cứ kiểu nào bên ngoài danh sách trên mà là `Copy` phải khai tường minh `⊐ Copy for X { }` (struct/enum của người dùng); số, `bool`, `char`, `&T`, tuple và mảng của kiểu `Copy` là `Copy`.
- `Arena`, `Cap<…>`, `Secret<T>` không bao giờ `Copy` (`E0313` nếu thử khai).

```grit value=13
≡ Id ≔ u32;
□ N: u32 ≔ 10u32;
■ M: u32 ≔ 3u32;
ƒ main() → u32 ! ∅ { x: Id ≔ N; x + M }
```

```grit fail=E0302
ƒ main() → i64 ! ∅ { x ≔ 1i32; y ≔ 2i64; x + y }
```

```grit fail=E0301
ƒ main() → i32 ! ∅ { x ≔ 1.5f64; ⎇ x { 1 } ⇝ { 0 } }
```

Mảng và lát: mảng **không** tự chuyển thành lát khi truyền `&a`; cắt `&a[..]` (hoặc `&a[1..]`) để có `&[T]`. Mảng không có phương thức `len`; lát có.

```grit value=3
ƒ len_of(xs: &[i32]) → usize ! ∅ { xs.len() }
ƒ main() → usize ! ∅ { a ≔ [1, 2, 3]; len_of(&a[..]) }
```

```grit fail=E0301
ƒ len_of(xs: &[i32]) → usize ! ∅ { xs.len() }
ƒ main() → usize ! ∅ { a ≔ [1, 2, 3]; len_of(&a) }
```

---

<a id="s3"></a>
## 3. Biểu thức và toán tử

### 3.1 Độ ưu tiên (giảm dần)

| Mức | Toán tử | Kết hợp |
|---|---|---|
| 1 | postfix: gọi `f(x)`, trường `.x`, chỉ số tuple `.0`, chỉ số/lát `[i]` | trái |
| 2 | prefix: `¬  -  *  &  &!` | |
| 3 | `as` | trái |
| 4 | `*  /  %  *%  *^  *?` | trái |
| 5 | `+  -  +%  -%  +^  -^  +?  -?` | trái |
| 6 | `==  ≠  <  >  ≤  ≥  ∈` | **không kết hợp** |
| 7 | `∧` (ngắn mạch) | trái |
| 8 | `∨` (ngắn mạch) | trái |
| 9 | `⇒` | **phải** |
| 10 | range `..  ..=` | |

`a < b < c`, `a ∈ b ∈ c`, `a < b == c` là lỗi cú pháp (`E0101`): dùng `∧` hoặc ngoặc. `⇒` theo spec chỉ dành cho hợp đồng; `gritc` thực tế vẫn nhận nó trong biểu thức thường ([18.3](#s18-3)). `∈` cần miền là range có ngoặc: `x ∈ (0..10)`.

```grit value=7
ƒ main() → i32 ! ∅ { ⎇ (true ∨ false ∧ false) { 1 + 2 * 3 } ⇝ { 0 } }
```

```grit fail=E0101
ƒ main() → bool ! ∅ { 1 < 2 < 3 }
```

### 3.2 Số học và kiểm tràn (D2)

| Họ | Toán tử | Hành vi |
|---|---|---|
| kiểm tràn (mặc định) | `+ - * / %`, `-x` | mỗi phép sinh Obligation tràn; `/ %` thêm "mẫu số ≠ 0" (và `MIN / -1`); `-x` thêm "x ≠ MIN"; dịch bit thêm `0 ≤ n < bits` |
| quấn vòng | `+% -% *%` | modulo 2ⁿ, không Obligation |
| bão hoà | `+^ -^ *^` | kẹt ở `MIN`/`MAX` |
| trả `Option` | `+? -? *?` | `None` khi tràn |

Số thực không có Obligation tràn; NaN/inf theo IEEE-754. Kết quả của Obligation ([mục 15](#s15)):

- **chắc chắn sai trên mọi đường** ⇒ ĐỎ: `E0601` (tràn, literal ngoài phạm vi), `E0602` (chia 0), `E0603` (chỉ số ngoài biên), `E0604` (ép kiểu mất thông tin);
- **chứng minh được** ⇒ xoá khỏi kiểm tra lúc chạy;
- **chưa chứng minh được mà kiểm được lúc chạy** ⇒ giữ kiểm tra + GHI CHÚ `N0601` (tràn), `N0602` (biên), `N0603` (chia), `N0604` (ép kiểu). Check chưa chứng minh được **được giữ ở mọi hồ sơ** (D24/D25, fail-closed); mặc định `--checks=all`.
- Lỗi lúc chạy không khôi phục (check hỏng) = abort tức thì, mã thoát 70.

```grit value=4
ƒ main() → u8 ! ∅ { x ≔ 250u8; x +% 10u8 }
```

```grit value=255
ƒ main() → u8 ! ∅ { x ≔ 250u8; x +^ 10u8 }
```

```grit value=0
ƒ main() → i32 ! ∅ { x ≔ 250u8; ⌥ x +? 10u8 { Some(v) ↦ 1, None ↦ 0 } }
```

```grit fail=E0601
ƒ main() → u8 ! ∅ { x ≔ 250u8; x + 10u8 }
```

```grit fail=E0602
ƒ main() → i32 ! ∅ { 1 / 0 }
```

```grit fail=E0603
ƒ main() → i32 ! ∅ { a ≔ [1, 2, 3]; a[5] }
```

### 3.3 Ép kiểu `as`

`e as T` giữa các kiểu số/`char`/`bool` (`char as u32` và `bool as i32` hợp lệ; `u32 as char` là `E0301`). `as` giữa số nguyên sinh Obligation "giá trị vừa kiểu đích": chắc chắn mất thông tin là `E0604`; chưa chứng minh là check + `N0604`. **Số thực sang số nguyên/`char`/`bool` bị cấm (`E0319`)**: dùng `f64::to_i64` (trả `Option<i64>`).

```grit value=3
ƒ main() → u64 ! ∅ { x ≔ 3u8; x as u64 }
```

```grit fail=E0604
ƒ main() → u8 ! ∅ { x ≔ 300i32; x as u8 }
```

```grit fail=E0319
ƒ f(x: f64) → i32 ! ∅ { x as i32 }
```

### 3.4 So sánh, logic, số thực

- `==`/`≠` chỉ cho số nguyên, `bool`, `char`, `&str`. Tuple, `&T`, `Option`, struct/enum: `E0301` (so từng trường, `*a == *b`, hoặc `cmp(..)` cho kiểu `Ord`).
- **`==`/`≠` trên số thực là ĐỎ** (`E0303`, D4); `< > ≤ ≥` vẫn dùng được.
- `∧ ∨` ngắn mạch, toán hạng `bool`. Điều kiện `⎇`/`⟳` phải là `bool`, không có truthiness ngầm.
- `f64`: IEEE-754, `+ - * /`, so sánh thứ tự, `-x`; phương thức `sqrt to_i64 to_bits is_nan`, hàm `core::f64_from_bits`. Bộ giải M7 coi giá trị `f64` là bí ẩn (không chứng minh được gì về chúng).

```grit fail=E0303
ƒ main() → bool ! ∅ { x ≔ 1.5f64; x == 1.5f64 }
```

```grit fail=E0301
ƒ main() → bool ! ∅ { (1, 2) == (1, 2) }
```

### 3.5 Thứ tự đánh giá và biến tạm

Trái sang phải. Toán hạng trái rồi phải (`∧ ∨` ngắn mạch); lời gọi: callee rồi đối số theo thứ tự; khoản mượn tự động của phương thức (`&self`/`&!self`) tạo **sau** đối số. Gán `chỗ ← giá_trị`: biểu thức con của chỗ (chỉ số, deref) trước, kiểm chỉ số, rồi giá trị, rồi ghi (khác Rust). Biến tạm chết ở cuối câu lệnh, trừ `tên ≔ &V;` / `≔ &!V;` với `V` không phải biểu thức chỗ (tạm sống tới cuối khối).

```grit value=5
ƒ main() → i32 ! ∅ {
  μa ≔ [0, 0, 0];
  μi ≔ 0;
  a[i] ← { i ← 2; 5 };
  a[0]
}
```

### 3.6 Khối và dấu `;`

Biểu thức cuối khối **không** `;` là giá trị khối; có `;` là câu lệnh, khối trả `()`. Hàm trả kiểu khác `()` mà mọi đường không kết thúc bằng giá trị/`⏎` là `E0307`. `⎇ ⌥ ⟳ ⚠` ở vị trí câu lệnh kết thúc tại `}`, không cần `;`.

```grit value=6
ƒ main() → i32 ! ∅ { x ≔ { y ≔ 2; y * 3 }; x }
```

```grit fail=E0307
ƒ f(x: i32) → i32 ! ∅ { x; }
ƒ main() → i32 ! ∅ { f(3) }
```

---

<a id="s4"></a>
## 4. Câu lệnh và vòng lặp

### 4.1 Binding, gán, `⎇`

- `tên [: T] ≔ e;` bind bất biến; `μ tên ≔ e;` khả biến. Chưa có destructure ở `≔` (dùng `t.0`, `t.1`).
- `chỗ ← e;` chỉ với `μ` hoặc qua `&!`; gán vào chỗ bất biến là `E0308` (D5). Đích gán: biến, trường, chỉ số, deref (`*p ← v`, `a[i] ← v`).
- `⎇ c { … } ⇝ { … }` là biểu thức; hai nhánh cùng kiểu; thiếu `⇝` thì nhánh `then` phải kiểu `()`. `⇝ ⎇ …` nối chuỗi. Điều kiện/đối tượng của `⎇ ⟳ ⌥` **không được là struct literal trần**: bọc ngoặc `(P { x: 0 })`.
- `⏎ e;` trả về sớm (kiểu `!`).

```grit value=1
ƒ f(x: i32) → i32 ! ∅ { ⎇ x > 0 { ⏎ 1; } ⇝ { } 0 }
ƒ main() → i32 ! ∅ { f(3) }
```

```grit fail=E0308
ƒ main() → i32 ! ∅ { x ≔ 1; x ← 2; x }
```

### 4.2 Ba dạng `⟳`

| Dạng | Cú pháp | Dừng |
|---|---|---|
| vô hạn | `⟳ { … }` | cần hiệu ứng `div` của hàm, không cần `↓` (thiếu `div`: `E0701`) |
| while | `⟳ cond [↓ m] [⋈ inv] { … }` | **tự suy** biến thể từ `cond` khi có dạng mẫu; không suy được và không có `↓`: `E0701` |
| for-in | `⟳ x in xs [↓ m] [⋈ inv] { … }` | `xs` hữu hạn (range đóng, `&[T]`, `&![T]`, `&[T; N]`) tự suy; iterator tổng quát chưa có |

Phần tử for-in: range cho giá trị (`i: u32`); `&[T]` cho `&T`; `&![T]` cho `&!T`. Biến lặp bất biến. `⊠` (break) và `⊡` (continue) dùng được trong cả ba dạng.

**Tự suy biến thể (while):** điều kiện có một vế `a < b`, `a ≤ b`, `a > b` hoặc `a ≥ b` (các vế nối bằng `∧`), và thân đưa biến về phía cận bằng bước hằng (`i ← i + c`, `i ← i - c`). `∨` và `≠` không suy được.

**`↓ m`** (biến thể): `m` phải **giảm nghiêm ngặt** mỗi vòng và **bị chặn dưới** (`0 ≤ m`) trên kiểu well-founded; sai là `E0702`. Biểu thức `↓` phải tuyến tính theo biến/tham số nguyên và `xs.len()` (hệ số ±1; phép nhân luôn `E0702`). Tối đa 24 biến biến thể mỗi thân hàm.

**`⋈ inv`** (bất biến vòng): kiểm lúc chạy ở đầu mỗi vòng nếu chưa chứng minh (`N0904`); chắc chắn sai là `E0901`. Phase 1 không dùng nó để chứng minh.

```grit value=10
ƒ main() → u32 ! ∅ {
  μi ≔ 0u32;
  ⟳ i < 10u32 ↓ 10u32 - i ⋈ i ≤ 10u32 { i ← i + 1u32; }
  i
}
```

```grit value=6
ƒ sum(xs: &[u32]) → u32 ! ∅ {
  μs ≔ 0u32;
  ⟳ x in xs { s ← s + *x; }
  s
}
ƒ main() → u32 ! ∅ { a ≔ [1u32, 2u32, 3u32]; sum(&a[..]) }
```

```grit value=12
ƒ main() → u32 ! ∅ {
  μs ≔ 0u32;
  ⟳ i in 0u32..10u32 {
    ⎇ i == 3u32 { ⊡; } ⇝ { }
    ⎇ i == 6u32 { ⊠; } ⇝ { }
    s ← s + i;
  }
  s
}
```

```grit fail=E0701
ƒ main() → u32 ! ∅ { μi ≔ 0u32; ⟳ i < 10u32 { } i }
```

```grit fail=E0701
ƒ main() ! ∅ { ⟳ { } }
```

```grit fail=E0702
ƒ main() → u32 ! ∅ {
  μi ≔ 0u32;
  ⟳ i < 10u32 ↓ 10u32 - i { i ← i + 0u32; }
  i
}
```

```grit
ƒ spin() ! {div} { ⟳ { } }
ƒ main() ! ∅ { }
```

### 4.3 Nhiên liệu (`fuel`)

Khi biến thể không chứng minh được bằng quan hệ chỉ số (bước tiến nhân đôi `w ← w + w`, `i ← i + f(..)`, đệ quy tương hỗ), dùng một bộ đếm `fuel` giảm 1 mỗi vòng cùng điều kiện `fuel > 0` và `↓ fuel`, rồi gọi `core::fuel_left(fuel)` **ngay sau vòng**: vòng thoát vì cạn nhiên liệu (`fuel == 0`) ⇒ abort `fuel_exhausted`. Đó là check lúc chạy (GHI CHÚ `N0905`); nếu bộ giải biết nhiên liệu **luôn** cạn khi thoát thì là `E0901`. Chương trình tự lo nhiên liệu đủ lớn.

```grit value=1024
ƒ main() → u64 ! ∅ {
  μx ≔ 1u64;
  μfuel ≔ 64u64;
  ⟳ x < 1000u64 ∧ fuel > 0u64 ↓ fuel { x ← x + x; fuel ← fuel - 1u64; }
  core::fuel_left(fuel);
  x
}
```

---

<a id="s5"></a>
## 5. Khớp mẫu `⌥`

`⌥ e { mẫu ↦ biểu_thức_hoặc_khối, … }` phải **đầy đủ** (D6): thiếu nhánh là `E0305`. Các nhánh cùng kiểu; nhánh kiểu `!` hợp nhất với kiểu của nhánh khác. Đối tượng `e` không được là struct literal trần (bọc ngoặc).

| Mẫu | Ví dụ | Ghi chú |
|---|---|---|
| wildcard / tên | `_`, `x` | tên bind giá trị (hoặc là biến thể không đối số nếu trùng tên) |
| literal | `1`, `-1`, `'a'`, `"s"`, `true` | |
| khoảng | `0..10` (không gồm 10), `0..=9`, `10..` (từ 10), `..0` (dưới 0) | số nguyên/`char` được vét cạn theo khoảng |
| biến thể | `Some(v)`, `Shape::Rect(w, h)` | có thể viết tên trần (`Circle(r)`) hoặc đủ đường dẫn |
| struct | `P { x, y: 0 }`, `P { x, .. }` | `x` = `x: x`; `..` đứng cuối, tối đa một lần |
| tuple | `(a, _)`, `(a, ..)` | `..` đơn lẻ là rest, tối đa một lần mỗi danh sách |

Đầy đủ: số nguyên/`char` phủ theo khoảng (`..0`, `0..=9`, `10..` phủ trọn `i32`); `bool` cần cả `true` và `false`; `str`, số thực và các kiểu mờ cần `_`/tên; **`u128` luôn cần `_`** (miền không được mô hình hoá; `i128` chính xác). Literal đơn lẻ không phủ cả miền.

**Khớp qua tham chiếu (ADR-040):** pattern phá cấu trúc (biến thể/struct/tuple) trên giá trị `&U`/`&!U` khớp `U` qua tham chiếu và biến bind có kiểu `&V`/`&!V` (không move, không copy). Chỉ **một tầng** (tham chiếu lồng là `E0301`); pattern hằng/khoảng trên `&U` là `E0301`; `_` và tên trần không đổi.

**Chưa có:** or-pattern `a | b`, guard, `μ x` trong pattern, destructure ở `≔`, `ref`. Nhánh dư thừa (nhánh sau nhánh bắt-tất-cả) **không** bị báo (`N0301` chỉ là đề xuất, chưa có).

```grit value=12
⊔ Shape { Circle(i32), Rect(i32, i32), Dot }
ƒ area(s: Shape) → i32 ! ∅ { ⌥ s { Circle(r) ↦ r * r, Rect(w, h) ↦ w * h, Dot ↦ 0 } }
ƒ main() → i32 ! ∅ { area(Shape::Rect(3, 4)) }
```

```grit value=1
ƒ f(x: i32) → i32 ! ∅ { ⌥ x { ..0 ↦ 0, 0..=9 ↦ 1, 10.. ↦ 2 } }
ƒ main() → i32 ! ∅ { f(5) }
```

```grit value=5
▢ P { x: i32, y: i32 }
ƒ f(p: P) → i32 ! ∅ { ⌥ p { P { x, y: 0 } ↦ x, P { x, .. } ↦ x + 4 } }
ƒ main() → i32 ! ∅ { f(P { x: 1, y: 2 }) }
```

```grit value=5
⊔ E { A(i32), B }
ƒ f(e: &E) → i32 ! ∅ { ⌥ e { A(v) ↦ *v, B ↦ 0 } }
ƒ main() → i32 ! ∅ { e ≔ E::A(5); f(&e) }
```

```grit fail=E0305
⊔ Shape { Circle(i32), Dot }
ƒ main() → i32 ! ∅ { s ≔ Shape::Dot; ⌥ s { Circle(r) ↦ r } }
```

```grit fail=E0305
ƒ f(x: i32) → i32 ! ∅ { ⌥ x { ..0 ↦ 0, 1.. ↦ 1 } }
```

```grit fail=E0301
ƒ f(x: &i32) → i32 ! ∅ { ⌥ x { 0 ↦ 0, _ ↦ 1 } }
```

**Bẫy đã biết:** tên biến thể **không đối số viết sai** (`Nonee`) không bị từ chối mà thành một biến mới bắt mọi giá trị; phần lớn bị bắt nhờ kiểm đầy đủ, nhưng khi nó đứng trước nhánh khác thì nhánh đó trở thành mã chết. Ví dụ dưới trả `0` dù đối số là `Some(3)`:

```grit value=0
ƒ f(o: Option<i32>) → i32 ! ∅ { ⌥ o { Nonee ↦ 0, Some(v) ↦ v } }
ƒ main() → i32 ! ∅ { f(Some(3)) }
```

---

<a id="s6"></a>
## 6. Hàm, hiệu ứng, hợp đồng

### 6.1 Chữ ký

```text
ƒ tên<generics>(μ? tham_số: kiểu, …) → kiểu  ! tập_hiệu_ứng  ⊳ pre  ⊲ post  ↓ biến_thể  { thân }
```

- Kiểu của mọi tham số và **tập hiệu ứng là bắt buộc** (D10): thiếu `! …` là `E0101`. Không có `→ T` nghĩa là trả `()`.
- Tham số khả biến viết `μ x: T`. `main` là điểm vào: một `ƒ main()` không tham số ở cấp tệp, có thể trả số nguyên (`gritc run` in giá trị).
- Gọi hàm là gọi tên hoặc giá trị kiểu `ƒ(A) → R ! E` (con trỏ hàm, không bắt biến, không closure).
- `self`, `&self`, `&!self` chỉ trong `⊏`/`⊐`.

### 6.2 Hiệu ứng (D10)

Tập hiệu ứng: `∅` (thuần) hoặc `{e₁, e₂, …}` với `io`, `mem`, `st`, `div`, `ffi`, `lock(L)`. Hiệu ứng **lan truyền lên hàm gọi**; gritc kiểm **khai báo ⊇ suy ra** (hợp hiệu ứng của mọi hàm được gọi, gồm qua giá trị hàm và phương thức trait): thiếu là `E0801`. Khai thừa thì được.

| Hiệu ứng | Nguồn điển hình |
|---|---|
| `io` | mọi thao tác của `Cap<Fs/Net/Env/Clock/Out/In/ErrOut>` |
| `mem` | `Arena::new`, `alloc`, `reset`, `Vec::new/push`, `Text::new/push_*` |
| `div` | `core::abort`, `core::exit`, vòng lặp vô hạn, đệ quy cố ý không dừng |
| `lock(L)` | `Mutex<T, L>::with` ([mục 12](#s12)) |
| `st`, `ffi` | tên được nhận; `ffi` bị cấm ở Phase 1 (D14) |

Kiểu hàm mang hiệu ứng trong kiểu (`ƒ(i32) ! {io}`) và so khớp **chính xác** (`ƒ(A) ! ∅` không nhận hàm `! {io}`: `E0301`). Thành viên `⊐ Trait for` chỉ được khai hiệu ứng nằm trong tập mà thành viên trait cho phép (khai ít hơn thì được): vượt là `E0802`. Tên hiệu ứng chưa được kiểm chính tả (tên lạ vẫn qua).

```grit
ƒ say(o: &Cap<Out>) ! {io} { o.write_str("hi\n"); }
ƒ main() ! {io} { out ≔ root_out(); say(&out); }
```

```grit fail=E0801
ƒ say(o: &Cap<Out>) ! {io} { o.write_str("hi\n"); }
ƒ main() ! ∅ { out ≔ root_out(); say(&out); }
```

```grit fail=E0802
⊏ A { ƒ a(&self) ! ∅; }
▢ S { v: i32 }
⊐ A for S { ƒ a(&self) ! {io} { } }
```

```grit value=4
ƒ add1(x: i32) → i32 ! ∅ { x + 1 }
ƒ apply(f: ƒ(i32) → i32 ! ∅, x: i32) → i32 ! ∅ { f(x) }
ƒ main() → i32 ! ∅ { apply(add1, 3) }
```

### 6.3 Hợp đồng: `⊳ ⊲ ↓ ⋈`

| Hợp đồng | Ý nghĩa | Kiểm | Hạ lúc chạy | ĐỎ khi chắc chắn sai |
|---|---|---|---|---|
| `⊳ P` | tiền điều kiện (người gọi phải bảo đảm); nhiều `⊳` được | tại **lời gọi trực tiếp** | `N0901` | `E0901` |
| `⊲ P` | hậu điều kiện; `result` là giá trị trả về | ở **mọi điểm thoát** | `N0902` | `E0901` |
| `↓ m` | biến thể của hàm đệ quy / vòng lặp | termination (M8) | không (D25) | `E0702` |
| `⋈ P` | bất biến vòng lặp | đầu mỗi vòng | `N0904` | `E0901` |

- Điều kiện kiểm lúc chạy phải là biểu thức **thuần, tính được** (D25). Cái không kiểm được lúc chạy mà chưa chứng minh (độ dừng, lượng từ không biên) là ĐỎ, không có đường hạ.
- **Lượng từ** `∀ i ∈ a..b : P` và `∃ i ∈ a..b : P` (dùng `:` thay `.`) chỉ hợp lệ **trong hợp đồng** (`E0316` ở nơi khác) và chỉ hạ được khi miền là khoảng nguyên **literal** `a..b`/`a..=b` (vòng lặp lúc chạy, `N0903`); miền là một giá trị `Range` hay lượng từ không biên là `E0902`.
- Các ràng buộc cố ý (fail-closed, `E0902`): `⊳` chưa chứng minh mà đối số không `Copy`; `⊲` nhắc tham số đã bị gán lại hoặc mượn `&!` (nghĩa "giá trị lúc vào" không có).
- `⊳` chỉ làm **giả thiết cho thân** khi hàm chỉ bị gọi trực tiếp; qua giá trị hàm hoặc là thành viên `⊐ Trait for` thì thân không được hưởng giả thiết (mỗi lời gọi trực tiếp vẫn bị kiểm).
- Bộ giải P1 (M7) = miền khoảng + zone (quan hệ hai biến) + điều kiện nhánh + `⊳`; không phải SMT. Sound, không complete: không chứng minh được quan hệ phi tuyến (`x*y`), đẳng thức giữa ba biến trở lên. Hàm có > 24 biến nguyên mất quan hệ giữa biến (`N0605`): tách hàm.

```grit value=5
ƒ half(x: u32) → u32 ! ∅ ⊳ x ≥ 2u32 { x / 2u32 }
ƒ main() → u32 ! ∅ { half(10u32) }
```

```grit fail=E0901
ƒ half(x: u32) → u32 ! ∅ ⊳ x ≥ 2u32 { x / 2u32 }
ƒ main() → u32 ! ∅ { half(1u32) }
```

Chưa chứng minh được tại điểm gọi thì `⊳` thành kiểm tra lúc chạy (ghi chú `N0901`), chương trình vẫn qua:

```grit value=5
ƒ half(x: u32) → u32 ! ∅ ⊳ x ≥ 2u32 { x / 2u32 }
ƒ g(y: u32) → u32 ! ∅ { half(y) }
ƒ main() → u32 ! ∅ { g(10u32) }
```

```grit value=6
ƒ inc(x: u32) → u32 ! ∅ ⊳ x < 100u32 ⊲ result > x { x + 1u32 }
ƒ main() → u32 ! ∅ { inc(5u32) }
```

```grit fail=E0901
ƒ inc(x: u32) → u32 ! ∅ ⊳ x < 100u32 ⊲ result > x + 5u32 { x + 1u32 }
ƒ main() → u32 ! ∅ { inc(5u32) }
```

```grit value=1
ƒ f(n: u32) → u32 ! ∅ ⊳ ∀ i ∈ 0..3 : i < 3 ⊲ ∃ j ∈ 0..2 : j == 1 { 1u32 }
ƒ main() → u32 ! ∅ { f(1u32) }
```

```grit fail=E0316
ƒ main() → bool ! ∅ { ∀ i ∈ 0..3 : i < 3 }
```

```grit fail=E0902
ƒ f(r: Range<u32>) → u32 ! ∅ ⊳ ∀ i ∈ r : i < 3u32 { 1u32 }
```

### 6.4 Đệ quy và dừng (D12)

Hàm nằm trong chu trình của đồ thị gọi (đệ quy trực tiếp hoặc tương hỗ, theo thành phần liên thông mạnh) phải có `↓ m` ở **mọi** hàm của chu trình, hoặc khai hiệu ứng `div`; thiếu là `E0703`. Mỗi lời gọi đệ quy phải làm `m` giảm nghiêm ngặt và `m ≥ 0` (`E0702` nếu không chứng minh được). Đệ quy qua giá trị hàm hoặc phương thức trait tính bảo thủ (cần `div`).

```grit value=120
ƒ fact(n: u32) → u64 ! ∅ ↓ n { ⎇ n == 0u32 { 1u64 } ⇝ { (n as u64) * fact(n - 1u32) } }
ƒ main() → u64 ! ∅ { fact(5u32) }
```

```grit value=1
ƒ ev(n: u32) → bool ! ∅ ↓ n { ⎇ n == 0u32 { true } ⇝ { od(n - 1u32) } }
ƒ od(n: u32) → bool ! ∅ ↓ n { ⎇ n == 0u32 { false } ⇝ { ev(n - 1u32) } }
ƒ main() → bool ! ∅ { ev(4u32) }
```

```grit fail=E0703
ƒ f(n: u32) → u32 ! ∅ { ⎇ n == 0u32 { 0u32 } ⇝ { f(n - 1u32) } }
ƒ main() → u32 ! ∅ { f(3u32) }
```

### 6.5 Abort, thoát, kiểu `!`

Không có macro `panic!`: dùng **hàm** `core::abort(msg: &str) → ! ! {div}` (dừng tức thì, không unwinding, mã thoát 70) và `core::exit(code: u32) → ! ! {div}` (thoát tiến trình với trạng thái `code`; `gritc run` báo `100 + code`). Hàm khai `→ !` phải phân kỳ trên **mọi** đường (gọi hàm `!` hoặc vòng vô hạn). `#noreturn` chỉ cho intrinsic trong `lib/core`.

```grit value=1
ƒ die() → ! ! {div} { core::abort("x") }
ƒ f(b: bool) → i32 ! {div} { ⎇ b { 1 } ⇝ { die() } }
ƒ main() → i32 ! {div} { f(true) }
```

---

<a id="s7"></a>
## 7. Struct, enum, generics, trait, impl

### 7.1 Struct và enum

- `▢ Tên<generics> { trường: Kiểu, … }`; literal `P { x: 1, y: 2 }` phải đủ trường (thiếu/thừa/trùng/lạ: `E0304`), không có `..base` và không có viết tắt `{ x, y }`.
- `⊔ Tên<generics> { V1, V2(T, U), … }`; tạo `Tên::V2(a, b)` (hoặc `V2(a, b)` trong cùng module); biến thể có tên trong không gian tên của module chứa enum, nên hai enum cùng module không được trùng tên biến thể (`E0202`).
- Trường private theo mặc định; `◈ trường: T` làm trường thấy được ngoài module (`E0203`).
- Suy luận generic cần đủ thông tin: `Either::Left(3)` mà không biết `R` là `E0301` ("thêm chú thích kiểu").

```grit fail=E0304
▢ P { x: i32, y: i32 }
ƒ main() → i32 ! ∅ { p ≔ P { x: 1 }; p.x }
```

### 7.2 Generics

`ƒ id<T>(x: T)`, `▢ Pair<A, B>`, `⊔ Either<L, R>`, `⊐ <T> Hộp<T> { … }`. Ràng buộc trait **inline**: `<T: A + B>` (không có where-clause; `\where` dành riêng). Lifetime là tham số: `<'a, T>`. Đơn hình hoá lazy: mỗi cặp (hàm, đối số kiểu) một bản; đệ quy đa hình vượt độ sâu 64 là `E0312`; tổng instance > 10 000 là `E0317`. Gọi hàm generic với kiểu không thoả ràng buộc: `E0309`.

```grit value=1
▢ Pair<A, B> { a: A, b: B }
⊐ <A, B> Pair<A, B> { ƒ first(&self) → &A ! ∅ { &self.a } }
ƒ id<T>(x: T) → T ! ∅ { x }
ƒ main() → i32 ! ∅ { p ≔ Pair { a: id(1), b: true }; *p.first() }
```

### 7.3 Trait và impl

- `⊏ Tên { ƒ m(&self, …) → T ! E ⊳ … ⊲ …; }` — trait **tĩnh** (không `dyn`). Trait generics (`⊏ T<U>`) chưa dùng được.
- `⊐ Trait for Kiểu { … }` cài trait; `⊐ Kiểu { … }` là impl kế thừa (inherent). Phương thức kế thừa được ưu tiên trước phương thức trait.
- `Self` là kiểu đang cài; `self` / `&self` / `&!self` là người nhận (theo giá trị / mượn chung / mượn duy nhất). Gọi `x.m(…)` tự mượn theo chữ ký.
- Impl phải khớp trait: thiếu hàm, thừa hàm, chữ ký khác, hai impl chồng lấn, hoặc `⊐ Copy for` kiểu có trường không `Copy` là `E0311`. Phương thức không tồn tại hoặc mơ hồ là `E0309`.
- Marker có sẵn: `Copy` (khai tường minh `⊐ Copy for P { }`), `Send`/`Sync` (tự suy theo cấu trúc, không được viết `⊐ Send for`), `Drop` (chỉ prelude), `Ord` (người dùng tự cài; `lib/std` cài sẵn cho các kiểu thường dùng, [mục 19](#s19)).

```grit value=9
⊏ Area { ƒ area(&self) → i32 ! ∅; }
▢ Sq { s: i32 }
⊐ Area for Sq { ƒ area(&self) → i32 ! ∅ { self.s * self.s } }
ƒ show<T: Area>(x: &T) → i32 ! ∅ { x.area() }
ƒ main() → i32 ! ∅ { q ≔ Sq { s: 3 }; show(&q) }
```

```grit value=7
▢ C { n: i32 }
⊐ C {
  ƒ new(n: i32) → Self ! ∅ { Self { n: n } }
  ƒ get(&self) → i32 ! ∅ { self.n }
  ƒ set(&!self, v: i32) ! ∅ { self.n ← v; }
}
ƒ main() → i32 ! ∅ { μc ≔ C::new(1); c.set(7); c.get() }
```

```grit fail=E0309
⊏ Area { ƒ area(&self) → i32 ! ∅; }
▢ Tri { s: i32 }
ƒ show<T: Area>(x: &T) → i32 ! ∅ { x.area() }
ƒ main() → i32 ! ∅ { q ≔ Tri { s: 3 }; show(&q) }
```

```grit fail=E0311
⊏ A { ƒ a(&self) → i32 ! ∅; ƒ b(&self) → i32 ! ∅; }
▢ S { v: i32 }
⊐ A for S { ƒ a(&self) → i32 ! ∅ { 1 } }
```

---

<a id="s8"></a>
## 8. Sở hữu, mượn và vùng đời (M6)

### 8.1 Quy tắc

- Giá trị kiểu không `Copy` bị **move** khi gán/truyền; dùng lại: `E0401` (trên bất kỳ đường nào có thể tới, kể cả cạnh lùi của vòng lặp hay khi chỉ một nhánh đã move). Gán lại toàn biến `μ` khởi tạo lại. Move một **trường** rồi dùng trường **khác** là hợp lệ (move một phần), trừ khi kiểu có `⊐ Drop for`/`Arena` (`E0407`).
- `&T` nhiều lượt cùng lúc; `&!T` **duy nhất**, loại trừ mọi `&`/`&!` khác trên cùng chỗ (hoặc chỗ chồng lấn: struct và trường của nó; chỉ số mảng luôn coi là chồng lấn mọi phần tử, nhưng hai **trường** khác nhau thì độc lập).
- **NLL:** khoản mượn sống đến **lần dùng cuối** của tham chiếu, không đến hết khối.
- Gán/move khi đang bị mượn mà khoản mượn còn dùng về sau: `E0406`. Move ra khỏi chỗ đi qua `*`, `[i]`, lát: `E0407`.
- Tham chiếu sống lâu hơn giá trị bị mượn (trả `&local`, trả `&tạm`, biến ngoài giữ tham chiếu tới biến trong, vi phạm region của chữ ký): `E0404`. Arena xem [mục 9](#s9).
- Biến tham chiếu là `Copy` (`&T`); truyền `&!` vào tham số `&!` là reborrow ngầm.

| Mã | Tình huống |
|---|---|
| `E0401` | dùng sau move |
| `E0402` | hai `&!` chồng nhau còn dùng |
| `E0403` | `&!` chồng `&` (hoặc đọc trực tiếp khi đang `&!` mượn) |
| `E0404` | tham chiếu sống lâu hơn giá trị bị mượn |
| `E0405` | tham chiếu thoát khỏi `Arena` (drop/move/gán đè/`reset` Arena khi còn tham chiếu cấp từ nó) |
| `E0406` | gán/move vào chỗ đang bị mượn |
| `E0407` | move ra khỏi tham chiếu/chỉ số/lát/trường của giá trị có `Drop` |

```grit value=2
ƒ main() → i32 ! ∅ { μx ≔ 1; a ≔ &!x; *a ← 2; b ≔ &x; *b }
```

```grit fail=E0403
ƒ main() → i32 ! ∅ { μx ≔ 1; a ≔ &x; b ≔ &!x; *b ← 2; *a }
```

```grit fail=E0402
ƒ main() → i32 ! ∅ { μx ≔ 1; a ≔ &!x; b ≔ &!x; *a ← 2; *b ← 3; x }
```

```grit fail=E0401
▢ B { v: i32 }
ƒ eat(b: B) → i32 ! ∅ { b.v }
ƒ main() → i32 ! ∅ { b ≔ B { v: 1 }; x ≔ eat(b); y ≔ eat(b); x + y }
```

```grit fail=E0406
ƒ main() → i32 ! ∅ { μx ≔ 1; a ≔ &x; x ← 2; *a }
```

```grit fail=E0407
▢ B { v: i32 }
ƒ take(r: &B) → B ! ∅ { *r }
```

```grit fail=E0308
ƒ f(x: &i32) ! ∅ { *x ← 1; }
```

### 8.2 Region và lifetime

- Mọi `&T`/`&!T` mang một region: tên `'a` do người dùng viết, `'static`, hoặc region ẩn (elision). Region **không** nằm trong kiểu (`&'a i32` và `&'b i32` là cùng một kiểu); M6 suy region trên MIR.
- Lifetime xuất hiện trong danh sách generics (`<'a, T>`) và sau `&`/`&!` (`&'a T`, `&!'a T`); trong đường dẫn kiểu đứng **trước** kiểu (`Holder<'a>`, `Pair<'a, T>`). Lifetime dùng phải được khai báo (`'static` luôn hợp lệ).
- **Elision** (chỉ ở chữ ký hàm, chặt hơn Rust): đếm vị trí tham chiếu ở đầu vào (mỗi `&` trong kiểu tham số; `&self`/`&!self` là một vị trí). **Đúng một** vị trí ⇒ mọi `&` đầu ra không ghi lifetime nhận region đó; **không** vị trí nào ⇒ `'static` (chỉ trả dữ liệu tĩnh như literal chuỗi; M6 kiểm thân hàm); **từ hai** ⇒ `E0315`, phải ghi `'r`. Không có luật "`&self` thắng".
- Trường struct/enum/alias: `&T` phải ghi lifetime tường minh (`E0315`).
- Chưa có ràng buộc outlives `'a: 'b`; quan hệ giữa hai region diễn đạt bằng cùng một tên.
- Phân tích NLL **sound, không complete**: có thể từ chối chương trình mà phân tích chính xác hơn chấp nhận (ví dụ kết quả `pick(&x, &y)` buộc với cả hai đối số).

```grit value=1
ƒ pick<'a>(x: &'a i32, y: &i32) → &'a i32 ! ∅ { x }
ƒ main() → i32 ! ∅ { a ≔ 1; b ≔ 2; *pick(&a, &b) }
```

```grit value=4
▢ H<'a> { r: &'a i32 }
ƒ get<'a>(h: H<'a>) → &'a i32 ! ∅ { h.r }
ƒ main() → i32 ! ∅ { x ≔ 4; *get(H { r: &x }) }
```

```grit value=4
ƒ name() → &str ! ∅ { "grit" }
ƒ main() → usize ! ∅ { name().len() }
```

```grit fail=E0315
ƒ pick(x: &i32, y: &i32) → &i32 ! ∅ { x }
```

```grit fail=E0315
▢ H { r: &i32 }
```

```grit fail=E0404
ƒ f() → &i32 ! ∅ { x ≔ 1; &x }
```

```grit fail=E0404
ƒ pick<'a, 'b>(x: &'a i32, y: &'b i32) → &'a i32 ! ∅ { y }
```

---

<a id="s9"></a>
## 9. Arena, Vec, Text

D26: không có allocator toàn cục ngầm; mọi cấp phát truy vết được tới một `Arena` truyền vào tường minh (tham số hoặc `Cap<Alloc>`). Hiệu ứng `mem` bắt buộc.

| Kiểu | API chính (đều `! {mem}` trừ khi ghi khác) |
|---|---|
| `Arena` | `Arena::new()`, `alloc<T>(&self, v: T) → &T`, `reset(&!self)`; `Cap<Alloc>::arena() → Arena` |
| `Vec<'a, T>` | `Vec::new(&'a Arena)`, `push(&!self, x)`, `reserve`, `len`, `as_slice() → &[T]`, `as_mut_slice(&!self) → &![T]`, `truncate(&!self, n)` |
| `Text<'a>` | `Text::new(&'a Arena)`, `push_str`, `push_char`, `push_utf8 → bool`, `from_utf8(a, &[u8]) → Option<Text<'a>>`, `len`, `as_str`, `as_bytes` |

Quy tắc:

- `Arena::alloc(&self, v) → &T`: kết quả mang region của lần mượn `&self`. Đó là quy tắc mượn thường; chỉ **mã lỗi** khác (`E0405` thay vì `E0404`) khi vật bị mượn là `Arena`. Tham chiếu cấp từ arena không được sống quá arena (drop, move, gán đè) và không được sống qua `reset` (`reset` là `&!`).
- Arena là tham số `&'r Arena` thì trả `&'r T` hợp lệ.
- `Vec<'a, T>`/`Text<'a>` giữ `&'a Arena` chia sẻ: **nhiều Vec/Text dùng chung một Arena**. Vec trên Arena **không chạy destructor**: `T` cần drop (`Arena`, có `Drop`, chứa thành phần như vậy, tham số kiểu không `Copy`) là `E0318`.
- `as_slice`/`as_str`/`as_bytes` mượn bằng `&self`: dựng thêm (`push*`, `&!self`) khi khung nhìn còn sống là `E0403` (mượn duy nhất chồng mượn chung). `Text` được **dựng rồi đóng băng**; bất biến UTF-8 do intrinsic bảo đảm (`push_utf8`/`from_utf8` kiểm RFC 3629 lúc chạy).
- Chỉ số `v.as_slice()[i]` có kiểm biên (`N0602` nếu chưa chứng minh). Chưa có `Vec<Vec<T>>`/`Vec<Text>` (`E0318`).
- Ngoài prelude còn có `&str`: `len`, `slice(from, to)` (cắt tại biên ký tự, sai thì abort `slice_range`), `as_bytes`.

```grit value=7
ƒ main() → u32 ! {mem} {
  a ≔ Arena::new();
  μv: Vec<u32> ≔ Vec::new(&a);
  v.push(3u32);
  v.push(4u32);
  s ≔ v.as_slice();
  s[0] + s[1]
}
```

```grit value=5
ƒ main() → usize ! {mem} {
  a ≔ Arena::new();
  μt ≔ Text::new(&a);
  t.push_str("xin ");
  t.push_char('c');
  t.len()
}
```

```grit value=2
ƒ main() → usize ! {mem} {
  a ≔ Arena::new();
  μv: Vec<u32> ≔ Vec::new(&a);
  μw: Vec<u8> ≔ Vec::new(&a);
  v.push(1u32);
  w.push(2u8);
  v.len() + w.len()
}
```

```grit fail=E0403
ƒ main() → u32 ! {mem} {
  a ≔ Arena::new();
  μv: Vec<u32> ≔ Vec::new(&a);
  v.push(3u32);
  s ≔ v.as_slice();
  v.push(4u32);
  s[0]
}
```

```grit fail=E0405
ƒ f() → &i32 ! {mem} { a ≔ Arena::new(); p ≔ a.alloc(5); p }
```

```grit fail=E0405
ƒ main() → i32 ! {mem} { μa ≔ Arena::new(); p ≔ a.alloc(5); a.reset(); *p }
```

```grit fail=E0318
ƒ main() ! {mem} { a ≔ Arena::new(); μv: Vec<Arena> ≔ Vec::new(&a); }
```

---

