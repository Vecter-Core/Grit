# Tham chiếu ngôn ngữ Grit

Tài liệu tra cứu hợp nhất cho ngôn ngữ **Grit** (ngôn ngữ thiết kế để AI viết mã) và trình biên dịch `gritc`. Tổng hợp từ `docs/spec/*` (grammar, semantics, symbols, regions, error_codes), `docs/errors/*.md`, `GRIT_PLAN.md` (D1–D29), `lib/core/prelude.gt` và `docs/KNOWN_LIMITATIONS.md`. Khi tài liệu này và hành vi thật của `gritc` khác nhau, tài liệu ghi **hành vi thật** và nêu sự khác biệt ở [mục 18.3](#s18-3).

**Cách tài liệu được kiểm.** Mọi khối ` ```grit ` trong tệp này là chương trình đầy đủ được máy kiểm bằng `tools/doc_test.py`: khối thường phải qua `gritc check`; `value=N` nghĩa là `main` trả đúng `N`; `fail=Exxxx` nghĩa là chương trình **phải bị từ chối đúng mã đó**. Đoạn không phải chương trình đầy đủ dùng hàng rào trần và không được kiểm.

```text
GRITC=/tmp/gritbin/gritc python3 tools/doc_test.py docs/LANGUAGE_REFERENCE.md
```

**Mức đảm bảo (nói thẳng).** An toàn của Grit đến từ các bước **kiểm tra** (kiểu, mượn, nghĩa vụ, dừng/hiệu ứng/đa luồng, bí mật, IR Guard), cộng corpus kiểm thử, oracle động, đột biến và so sánh thông dịch với mã máy. Đó là bằng chứng thực nghiệm (kiểm tra, không phải bằng chứng toán học); còn đột biến chưa giải quyết được liệt kê ở [mục 18](#s18). Nền đã kiểm: **Linux x86_64**; Windows: **chưa kiểm**. Hai backend: `llvm` (mặc định khi bản dựng có) và `interpreter` (thông dịch MIR, chỉ là công cụ kiểm thử, không phải bằng chứng cho ngữ nghĩa codegen).

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
- `Copy`: số, `bool`, `char`, `&T`, tuple và mảng của kiểu `Copy`. Struct/enum của người dùng **không** `Copy` trừ khi khai tường minh `⊐ Copy for X { }` (mọi trường phải `Copy`: `E0311`).
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

<a id="s4-3"></a>
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

<a id="s10"></a>
## 10. Capability

Quyền nhạy cảm (tệp, mạng, môi trường, đồng hồ, stdout/stdin/stderr, cấp phát) là **giá trị kiểu `Cap<K>`** phải được truyền tường minh qua chữ ký (D28). Không có biến toàn cục cấp quyền.

- `K` trong prelude: `Fs`, `Net`, `Env`, `Clock`, `Alloc`, `Out`, `In`, `ErrOut`. Ví dụ `Cap<Out>::write_str/write_u64/write_bytes`, `Cap<Fs>::open/read/write/sub`, `Cap<Env>::arg/arg_count/var`, `Cap<Alloc>::arena`. Thao tác có hiệu ứng `io` (trừ `Alloc`: `mem`).
- `Cap<K>` là kiểu **affine**: không `Copy`, không `Default`, không literal (`E0313` nếu tạo bằng `Cap { … }` hay khai `⊐ Copy for Cap<…>`). Truyền hai lần theo giá trị là `E0401`; truyền `&Cap<K>` để chia sẻ.
- **Nguồn gốc** (D29): hàm trả `Cap<…>` mà không nhận `Cap<…>` nào (`root_fs() root_net() root_env() root_clock() root_alloc() root_out() root_in() root_err()`, `Host::fs()`). **Chỉ `main` cấp cao của một tệp** được gọi hoặc nhắc tới chúng; mọi nơi khác (hàm thường, phương thức, generic, nhánh/vòng lặp, khởi tạo `□`/`■`, `main` lồng trong `▦`) là `E0310`.
- Chia nhỏ qua hàm nhận `&Cap<…>` (ví dụ `fs.sub("data")`, intrinsic trusted trong `lib/core`).
- Mô hình "thiếu capability" thể hiện qua kiểu: không có `Cap<Out>` trong tay thì không có cách nào gọi `write_str`.

```grit
ƒ say(o: &Cap<Out>, s: &str) ! {io} { o.write_str(s); }
ƒ main() ! {io} {
  out ≔ root_out();
  say(&out, "hi\n");
  say(&out, "again\n");
}
```

```grit fail=E0310
ƒ helper() ! {io} { out ≔ root_out(); out.write_str("x"); }
ƒ main() ! {io} { helper(); }
```

```grit fail=E0313
ƒ main() ! ∅ { c ≔ Cap { v: Fs { x: 1 } }; }
```

```grit fail=E0401
ƒ use2(a: Cap<Out>, b: Cap<Out>) ! ∅ { }
ƒ main() ! ∅ { o ≔ root_out(); use2(o, o); }
```

---

<a id="s11"></a>
## 11. Bí mật (`Secret`)

- `Secret<T>`: không `Display`, không vào sink log/mạng/tệp. Không tạo bằng literal (`E0313`); nhận qua tham số hoặc từ thư viện. Không bao giờ `Copy`.
- `declassify(self) → T` là chỗ **duy nhất** lấy giá trị ra: mỗi lời gọi được ghi nhận và báo **VÀNG `S201`**. Giá trị chảy tới lời gọi có hiệu ứng `io` (sink): **`S202`** (taint theo thân hàm, không xuyên hàm, bảo thủ).
- Quét nguồn: **`S101`** literal giống khoá/token phổ biến (AWS `AKIA…`, GitHub `ghp_…`, Slack, Google `AIza…`, Stripe, khối PEM, JWT; cả trong comment); **`S102`** literal chuỗi entropy cao (≥ 12 ký tự, ≥ 3,3 bit/ký tự) gán cho tên gợi ý secret (`key token secret api passw pwd`; mật khẩu ≥ 8 ký tự, ≥ 2,5 bit, có chữ số).
- **VÀNG vẫn chạy** (exit 0). `--deny-yellow` nâng VÀNG thành ĐỎ (exit 1); nên dùng cho bản phát hành.
- Tắt có kiểm soát bằng `#allow(Sxxx, "lý do")` đặt **trước item**: lý do là chuỗi không rỗng; mã phải là VÀNG đã biết (`S101 S102 S201 S202`); ĐỎ không có cách tắt (`E1401`). `#allow` thừa là ghi chú `N1401`.
- Các bộ quét là **kinh nghiệm**: có dương tính giả (khoá mẫu trong tài liệu) và âm tính giả (secret ghép lúc chạy, tên không gợi ý). Không phải kiểm tra an toàn đầy đủ.

```grit
ƒ reveal(s: Secret<i32>) → i32 ! ∅ { s.declassify() }
ƒ main() ! ∅ { }
```

```grit
ƒ main() ! ∅ { k ≔ "AKIAIOSFODNN7EXAMPLE"; }
```

```grit
#allow(S101, "khoá mẫu trong tài liệu")
ƒ f() ! ∅ { k ≔ "AKIAIOSFODNN7EXAMPLE"; }
ƒ main() ! ∅ { f(); }
```

```grit fail=E1401
#allow(S101)
ƒ f() ! ∅ { k ≔ "AKIAIOSFODNN7EXAMPLE"; }
ƒ main() ! ∅ { f(); }
```

```grit fail=E0313
ƒ main() → i32 ! ∅ { s ≔ Secret { v: 3 }; 0 }
```

`unsafe` và con trỏ thô: `⚠ ⊢p … { }` có cú pháp và sinh nghĩa vụ, nhưng Phase 1 chưa giải được nghĩa vụ `⊢` nào nên **mọi `⚠` ở mã người dùng là `E1001`** (kể cả khối rỗng); con trỏ thô `*□ T`/`*μ T` ngoài `⚠` cũng `E1001`. `#lang(…)` ngoài `lib/core` là `E1002`. `\extern` (FFI) bị cấm (D14).

```grit fail=E1001
ƒ main() ! ∅ { ⚠ { } }
```

```grit fail=E1002
#lang(option)
▢ X { a: i32 }
```

---

<a id="s12"></a>
## 12. Đa luồng

- `spawn(f, arg)` là **hàm** trong prelude (không phải từ khoá); không closure: `f` là tên hàm kiểu `ƒ(A) ! …` (không bắt biến), `arg: A` chuyển sang luồng mới và phải `Send`. Không có `join`: chương trình kết thúc khi `main` trả về.
- `Send`/`Sync` tự suy theo cấu trúc. Phase 1 bảo thủ: mọi tham chiếu, con trỏ thô, `Arena` không `Send` ⇒ `E0501`. Chia sẻ giữa luồng chỉ qua `■` static kiểu `Sync` (`Atomic<…>`, `Mutex<…>`) **bằng tên**.
- `Atomic<T>` (`T` ∈ `bool i32 u32 i64 u64 usize`): `new load store swap fetch_add fetch_sub compare_exchange`. **Ordering** (`AtomicOrdering::{Relaxed, Acquire, Release, AcqRel, SeqCst}`) bắt buộc tường minh, viết thẳng (hoặc biến gán đúng một lần): `load` không nhận `Release`/`AcqRel`, `store` không nhận `Acquire`/`AcqRel`, `compare_exchange` thất bại không `Release`/`AcqRel`; sai là `E0503`.
- `Mutex<T, L>`: khoá chỉ qua `M.with(f, arg)` (có phạm vi, không guard), `f: ƒ(&!T, Arg) → R`. `L` là kiểu đánh dấu có `#level("n")`. Hiệu ứng `lock(L)` truyền xuyên hàm (kiểm bằng `E0801`). Tại mỗi `with`, mọi `lock(L')` bên trong `f` phải có cấp **lớn hơn nghiêm ngặt** cấp của `L`; đảo/cùng cấp/thiếu `#level` là `E0502` (thứ tự toàn cục tăng ⇒ không chu trình chờ).
- `Cell<T>`: ô khả biến đơn luồng (`new set get`), `&self`.

```grit
■ COUNT: Atomic<u32> ≔ Atomic::new(0u32);
ƒ worker(n: u32) ! ∅ { COUNT.fetch_add(n, AtomicOrdering::Relaxed); }
ƒ main() ! ∅ { spawn(worker, 1u32); spawn(worker, 2u32); }
```

```grit
#level("1")
▢ Lo { }
■ M: Mutex<u32, Lo> ≔ Mutex::new(0u32);
ƒ add(v: &!u32, n: u32) → u32 ! ∅ { *v ← *v +% n; *v }
ƒ worker(n: u32) ! {lock(Lo)} { M.with(add, n); }
ƒ main() ! {lock(Lo)} { spawn(worker, 1u32); spawn(worker, 2u32); }
```

```grit fail=E0502
#level("10")
▢ Lo { }
#level("20")
▢ Hi { }
■ LO: Mutex<u32, Lo> ≔ Mutex::new(0u32);
■ HI: Mutex<u32, Hi> ≔ Mutex::new(0u32);
ƒ leaf(v: &!u32, n: u32) → u32 ! ∅ { *v }
ƒ inner(v: &!u32, n: u32) → u32 ! {lock(Lo)} { LO.with(leaf, 0u32) }
ƒ go() → u32 ! {lock(Hi), lock(Lo)} { HI.with(inner, 0u32) }
```

```grit fail=E0503
ƒ f(a: &Atomic<u32>) → u32 ! ∅ { a.load(AtomicOrdering::Release) }
```

```grit fail=E0501
ƒ w(r: &u32) ! ∅ { }
ƒ main() ! ∅ { x ≔ 5u32; spawn(w, &x); }
```

```grit value=5
ƒ main() → i32 ! ∅ { c ≔ Cell::new(1); c.set(5); c.get() }
```

**Mức đảm bảo đa luồng:** không có oracle động cho đua dữ liệu/deadlock/memory ordering và **không có ThreadSanitizer hay loom**; bằng chứng là corpus (`E0501/E0502/E0503`), test cấu trúc, đột biến, chạy lặp mã máy thật và đối chiếu thông dịch ≡ exe. Thông dịch chạy `spawn` tuần tự, không phản ánh ngữ nghĩa luồng thật. Khoá `Mutex` trên Linux là khoá quay + nhường luồng, chưa đo tranh chấp.

---

<a id="s13"></a>
## 13. Module và visibility

- Mỗi **tệp `.gt`** là một module (tên tệp bỏ `.gt`; phải là định danh hợp lệ, duy nhất không phân biệt hoa/thường). `▦ tên { … }` tạo module con. `gritc` nhận một tệp hoặc một thư mục (mọi `*.gt`, sắp theo tên); `run` đòi đúng một `ƒ main()` cấp tệp.
- **Mặc định private.** `◈` làm item (hàm, struct, enum, trait, alias, const, static, module con, trường) thấy được ở mọi nơi; item private thấy ở module định nghĩa và **mọi module con** của nó, không thấy ở anh em hay tệp khác: `E0203`. `◈ ⇲ a::f;` tái xuất khẩu.
- Đường dẫn dùng `::` (`geo::area`). `⇲ đường::dẫn;` hoặc `⇲ đường::dẫn::{a, b};` nhập tên. Trong một tệp đơn tên module là `main` (nhập `⇲ main::geo::area;`).
- Tên đầu đường dẫn tìm theo: biến cục bộ → tham số generics → tên của module hiện tại → tên module-tệp → prelude. **Module `▦` lồng không kế thừa tên module cha**: dùng đường dẫn tuyệt đối (`main::g`) hoặc `⇲`. Không có `super`/`self::`/đổi tên/glob.
- Tên không có: `E0201`. Định nghĩa trùng (cùng một không gian tên item, kể cả tên nhập): `E0202`. Vòng phụ thuộc giữa module/tệp: `E0204`. Thứ tự khai báo item **không** quan trọng; xáo thứ tự tệp cho kết quả giống từng byte.
- `main` ở trong `▦` không phải điểm vào và không được gọi nguồn capability.

```grit value=12
▦ geo { ◈ ƒ area(w: i32, h: i32) → i32 ! ∅ { w * h } }
ƒ main() → i32 ! ∅ { geo::area(3, 4) }
```

```grit value=16
▦ geo {
  ◈ ƒ area(w: i32, h: i32) → i32 ! ∅ { w * h }
  ◈ ƒ perim(w: i32, h: i32) → i32 ! ∅ { 2 * (w + h) }
}
⇲ main::geo::{area, perim};
ƒ main() → i32 ! ∅ { area(3, 4) + perim(1, 1) }
```

```grit fail=E0203
▦ geo { ƒ area(w: i32, h: i32) → i32 ! ∅ { w * h } }
ƒ main() → i32 ! ∅ { geo::area(3, 4) }
```

```grit fail=E0201
ƒ g() → i32 ! ∅ { 5 }
▦ m { ◈ ƒ h() → i32 ! ∅ { g() } }
ƒ main() → i32 ! ∅ { m::h() }
```

```grit fail=E0202
ƒ f() ! ∅ { }
ƒ f() ! ∅ { }
```

Nhiều tệp (mỗi khối một tệp, cùng nhóm):

```grit value=12 file=main.gt group=multi1
ƒ main() → i32 ! ∅ { geo::area(3, 4) }
```

```grit file=geo.gt group=multi1
◈ ƒ area(w: i32, h: i32) → i32 ! ∅ { w * h }
```

```grit fail=E0203 file=main.gt group=multi2
ƒ main() → i32 ! ∅ { geo::area(3, 4) }
```

```grit file=geo.gt group=multi2
ƒ area(w: i32, h: i32) → i32 ! ∅ { w * h }
```

```grit fail=E0204 file=main.gt group=multi3
ƒ main() → i32 ! ∅ { a::f() }
```

```grit file=a.gt group=multi3
◈ ƒ f() → i32 ! ∅ { b::g() }
```

```grit file=b.gt group=multi3
◈ ƒ g() → i32 ! ∅ { a::f() }
```

---

<a id="s14"></a>
## 14. Không shadowing (D7)

Một tên mới **không được trùng** tên đang thấy được; vi phạm là `E0205`. Phạm vi cấm: tham số, biến `≔`, biến `⟳ x in`, binding trong pattern, biến lượng từ, tham số generics, `result` của `⊲`, đối với: biến cục bộ/tham số khác **trong cùng hàm (kể cả khối con)**, tham số generics, tên cấp module (item, tên nhập, prelude), và tên module-tệp (ở mọi gói trong đồ thị). Hai khối **anh em** có thể dùng lại tên (không cùng thấy nhau). Tên biến thể người dùng trùng `Ok/Err/Some/None` của prelude thường ra `E0301` kèm gợi ý `Result::Ok`.

Hệ quả thực hành: `lib/std` đặt hậu tố `_` cho tên cục bộ phổ biến (`base_`, `buf_`…); nếu gặp `E0205` trỏ vào `std.gt`, đổi tên tệp của bạn (ví dụ `base.gt` → `base_util.gt`).

```grit fail=E0205
ƒ main() → i32 ! ∅ { x ≔ 1; ⎇ true { x ≔ 2; x } ⇝ { 0 } }
```

```grit fail=E0205
ƒ f(x: i32) → i32 ! ∅ { x ≔ 2; x }
```

```grit fail=E0205
ƒ main() → u32 ! ∅ { i ≔ 1u32; μs ≔ 0u32; ⟳ i in 0u32..3u32 { s ← s + i; } s }
```

```grit value=3
ƒ main() → i32 ! ∅ {
  a ≔ ⎇ true { x ≔ 1; x } ⇝ { 0 };
  b ≔ ⎇ true { x ≔ 2; x } ⇝ { 0 };
  a + b
}
```

---

<a id="s15"></a>
## 15. Các pass của gritc

Mọi lỗi được báo (có giới hạn số lượng, thứ tự tất định theo offset, D19). Pipeline:

| Giai đoạn | Pass (`passes` trong JSON) | Việc | Mã chính |
|---|---|---|---|
| nạp + lexer + parser | — | NFC, quy bí danh ASCII, cấm ký tự nguy hiểm; recursive descent + Pratt, khôi phục lỗi | `E0101 E0102 E12xx` |
| giải tên | — | đồ thị module, tên, visibility, D7 | `E0201–E0205` |
| **M4 kiểu** | `m4.types` | kiểm kiểu hai chiều, generics đơn hình hoá, trait tĩnh, đầy đủ `⌥`, capability, chữ ký region | `E03xx` |
| **M5 MIR** | `m5.mir`, `m5.return_paths` | hạ xuống MIR (CFG); mọi đường trả về | `E0307` |
| **M6 mượn** | `m6.borrowck` | sở hữu, mượn kiểu NLL, region, Arena | `E04xx` |
| **M7 nghĩa vụ** | `m7.obligations`, `m7.const_bounds` | sinh Obligation (tràn, chia, biên, ép kiểu, hợp đồng) và giải bằng khoảng + zone; chưa giải được thì hạ runtime | `E06xx E0901 E0902 N06xx N09xx` |
| **M8 dừng/hiệu ứng/đa luồng** | `m8.termination`, `m8.effects`, `m8.conc` | biến thể `↓`, hiệu ứng, `Send/Sync`, thứ tự khoá, ordering | `E07xx E08xx E05xx` |
| **M9 bí mật** | `m9.secrets` | quét S1xx, theo dõi taint S2xx, `#allow`, `unsafe` | `S101 S102 S201 S202 E10xx E14xx` |
| **cổng phán quyết** | `selftest` + danh sách pass bắt buộc | gộp kết quả; canary `selftest` (`E0601`/`E0307` phải còn bị bắt) | exit 0/1/2 |
| codegen | — | MIR → LLVM IR kèm `!gt.obl`, `!gt.attest` | |
| **IR Guard** | — | LLVM verifier; attestation khớp; thao tác nhạy cảm có `!gt.obl` đúng lớp (atomic ⇒ `E0503`, `spawn` ⇒ `E0501`, mutex ⇒ `E0502`, I/O ⇒ `E0310`); `unreachable` chỉ ngay sau `call @gritrt_abort`; không cờ UB/fast-math/`llvm.assume` | `E1101` |

Phán quyết:

| Mức | Ý nghĩa | Tác động |
|---|---|---|
| ĐỎ (`E…`) | mọi lỗi không thuộc nhóm bí mật; kể cả pass hỏng/thiếu | không sinh binary, không chạy (exit 1) |
| VÀNG (`S…`) | chỉ lộ nhạy cảm | vẫn chạy, in cảnh báo; `--deny-yellow` ⇒ ĐỎ |
| GHI CHÚ (`N…`) | thông tin (chuyển mức Obligation ⇒ runtime check) | không ảnh hưởng |
| ICE (`E9901`) | lỗi nội bộ gritc (kể cả panic bị bắt) | exit 2; không bao giờ chạy |

Nguyên tắc **fail-closed**: "không biết" ≠ "an toàn". Pass hỏng ⇒ ICE; solver trả "chưa chứng minh" ⇒ giữ runtime check nếu kiểm được lúc chạy (D25), ngược lại ĐỎ. Mọi pass đệ quy chạy qua `run_with_big_stack`.

**Lệnh:** `gritc check <tệp|thư-mục>`, `build [-o ra] [--backend=llvm|interpreter] [--profile=dev|release|release-o2]`, `run`, `explain <mã>` (in tài liệu mã), `fmt <tệp> [--check]` (in dạng chuẩn canonical, ghi nguyên tử; từ chối tệp có lỗi parser, không sửa tệp), `--json` (schema có version; có `verdict`, `passes`, `checks`, `attestation`), `--deny-yellow`, `--checks=all|proven`.

- `--checks=all` (**mặc định**): giữ mọi check; `--checks=proven` là **thử nghiệm**, xoá check đã chứng minh (`N0606`), vì còn 69 đột biến "xoá thêm check" chưa giải quyết: không dùng cho bản phát hành.
- Hồ sơ `dev` (-O0, mọi Assert giữ) là mặc định; `release`/`release-o2` có bằng chứng thực nghiệm dev ≡ release, **không phải chứng minh**; bộ tối ưu LLVM nằm trong TCB. `gritc build` ghi exe (llvm) hoặc gói `*.gtmir` (interpreter).
- Cơ sở tin cậy (TCB): mã gritc, `rustc`, LLVM, linker, OS/phần cứng, các intrinsic trusted trong `lib/core` (`docs/TCB_INTRINSICS.md`), danh sách lang item `#lang(…)`.

---

<a id="s16"></a>
## 16. Mã thoát của gritc và rig

| `gritc` | Ý nghĩa |
|---|---|
| `0` | qua (có thể kèm VÀNG/GHI CHÚ) |
| `1` | ĐỎ (hoặc VÀNG với `--deny-yellow`) |
| `2` | ICE |
| `3` | lỗi dùng / I/O / môi trường / chưa hỗ trợ (ví dụ backend llvm chưa hạ được `f64 as`, `i128 *`, `Cap<Fs>::sub`) |

Riêng `gritc run` (chương trình **đã chạy**): `0` xong bình thường; `70` abort (D3: check lúc chạy hỏng, `core::abort`, chỉ số ngoài biên…); `71` bộ chạy tham chiếu hết giới hạn (thông dịch: 200 khung đệ quy); `100 + s` chương trình tự thoát bằng `core::exit(s)`. Vùng của gritc (≤ 3) và vùng chương trình (≥ 70) không chồng nhau. Mọi đầu ra ghi `backend`. `gritc run` trên chương trình không có `ƒ main()` cấp tệp trả `2` (báo là lỗi codegen), không phải `3` ([18.3](#s18-3)).

| `rig` | Ý nghĩa |
|---|---|
| mã của gritc | chuyển nguyên khi lỗi đến từ gritc |
| `10` | `rig.lock` thiếu (với `--locked`) hoặc lệch |
| `11` | dự án/manifest sai |
| `12` | dùng sai |
| `13` | I/O / môi trường |
| `1` | `rig test` có test hỏng |

Mỗi sửa đổi mã của gói Rig phải `rig lock` rồi mới `rig build/run`; rig từ chối khi lệch băm.

```grit exit=2
ƒ main() ! {div} { core::exit(2u32); }
```

---

<a id="s17"></a>
## 17. Mã lỗi

Quy ước (D16): `E` + 4 số = ĐỎ; `S` + 3 số = VÀNG (bí mật); `N` + 4 số = GHI CHÚ; `E99xx` = ICE. Dải: `E01xx` cú pháp · `E02xx` tên/module · `E03xx` kiểu · `E04xx` sở hữu/mượn · `E05xx` đa luồng · `E06xx` biên/trường hợp biên · `E07xx` dừng · `E08xx` hiệu ứng · `E09xx` logic/Obligation · `E10xx` unsafe/`#lang` · `E11xx` IR Guard · `E12xx` Unicode/nguồn · `E13xx` định dạng · `E14xx` `#allow` · `E99xx` ICE · `S1xx` bí mật trong nguồn · `S2xx` luồng bí mật · `N06xx/N09xx/N14xx` ghi chú. `E0306` đã nghỉ (không dùng lại). Chi tiết từng mã: `gritc explain <mã>` hoặc `docs/errors/<mã>.md`.

### 17.1 E01xx–E02xx: cú pháp, tên

| Mã | Mô tả |
|---|---|
| `E0101` | token bất ngờ/thiếu dấu; từ khoá cứng hoặc ký hiệu dành riêng dùng sai chỗ (`self`, `\extern`, `\where`, `μ`); `/* */`; `=` đứng riêng; `x as T < y` |
| `E0102` | lồng quá sâu (> 256 mức) |
| `E0201` | tên chưa định nghĩa |
| `E0202` | định nghĩa trùng |
| `E0203` | truy cập item/trường private |
| `E0204` | vòng phụ thuộc module |
| `E0205` | shadowing (D7) |

### 17.2 E03xx: kiểu

| Mã | Mô tả |
|---|---|
| `E0301` | sai kiểu / số đối số / biểu thức chưa có ngữ nghĩa (range mở độc lập, `==` trên tuple, không suy ra được kiểu) |
| `E0302` | ép kiểu số ngầm (D1) |
| `E0303` | `==`/`≠` trên số thực (D4) |
| `E0304` | trường struct/biến thể/tuple sai (thiếu, thừa, trùng, lạ, chỉ số tuple vượt) |
| `E0305` | `⌥` không đầy đủ (D6) |
| `E0307` | thiếu đường trả về |
| `E0308` | gán vào chỗ bất biến (D5) |
| `E0309` | ràng buộc trait không thoả / không có phương thức / mơ hồ |
| `E0310` | thiếu capability: gọi nguồn capability gốc ngoài `main` |
| `E0311` | `⊐` không hợp lệ (thiếu/thừa/khác chữ ký, chồng lấn, `Copy` sai) |
| `E0312` | đệ quy đa hình vượt 64 mức instance |
| `E0313` | kiểu lang dùng sai: tạo `Secret`/`Cap`/`Vec`/`Text` bằng literal, `⊐ Copy for` chúng, `⊐ Drop for` ngoài lib/core |
| `E0314` | `!` ở vị trí không hợp lệ (tham số kiểu, trường, phần tử) |
| `E0315` | lifetime/region trong chữ ký không hợp lệ (chưa khai báo, trùng, elision mơ hồ, trường thiếu lifetime) |
| `E0316` | lượng từ `∀ ∃` ngoài hợp đồng |
| `E0317` | quá 10 000 instance sau đơn hình hoá |
| `E0318` | `Vec<'a, T>` với `T` cần drop |
| `E0319` | phép số thực chưa hỗ trợ/bị cấm: `as` từ `f64` sang nguyên/`char`/`bool`, `f32`, `%` trên số thực |

### 17.3 E04xx–E05xx: mượn, đa luồng

| Mã | Mô tả |
|---|---|
| `E0401` | dùng sau move |
| `E0402` | hai `&!` chồng nhau |
| `E0403` | `&!` chồng `&` |
| `E0404` | tham chiếu sống lâu hơn giá trị bị mượn / vi phạm region chữ ký |
| `E0405` | tham chiếu thoát khỏi `Arena` (D27) |
| `E0406` | gán/move khi đang bị mượn |
| `E0407` | move ra khỏi tham chiếu/chỉ số/lát hoặc trường của giá trị có `Drop` |
| `E0501` | chia sẻ không `Send`/`Sync` qua luồng |
| `E0502` | vi phạm thứ tự khoá |
| `E0503` | memory ordering của atomic không hợp lệ hoặc không viết thẳng |

### 17.4 E06xx–E09xx: biên, dừng, hiệu ứng, hợp đồng

| Mã | Mô tả |
|---|---|
| `E0601` | tràn chắc chắn / literal ngoài phạm vi |
| `E0602` | chia/`%` cho 0 chắc chắn |
| `E0603` | chỉ số/khoảng cắt ngoài biên chắc chắn |
| `E0604` | ép kiểu mất thông tin chắc chắn |
| `E0701` | vòng lặp không chứng minh được dừng (thiếu `↓`; vô hạn thiếu `div`) |
| `E0702` | `↓` không giảm nghiêm ngặt hoặc không bị chặn dưới |
| `E0703` | đệ quy thiếu `↓` (hoặc `div`) |
| `E0801` | hiệu ứng suy ra ⊄ khai báo |
| `E0802` | thành viên impl khai hiệu ứng ngoài tập của trait |
| `E0901` | hợp đồng `⊳ ⊲ ⋈` chắc chắn sai |
| `E0902` | nghĩa vụ không kiểm được lúc chạy mà chưa giải (lượng từ không biên, đối số không `Copy`, `⊲` nhắc tham số bị gán lại) |

### 17.5 E10xx–E14xx, E99xx: unsafe, Guard, nguồn, ICE

| Mã | Mô tả |
|---|---|
| `E1001` | `⚠` thiếu bằng chứng (mọi `⚠` ở mã người dùng); con trỏ thô ngoài `⚠` |
| `E1002` | `#lang` ngoài lib/core |
| `E1101` | IR Guard từ chối IR (lỗi gritc; không có mẫu `.gt` kích hoạt) |
| `E1201` | ký tự bidi (Trojan Source) |
| `E1202` | ký tự vô hình/zero-width/điều khiển/khoảng trắng lạ, `\r` lẻ |
| `E1203` | định danh trộn bảng chữ (homoglyph); `µ` U+00B5 |
| `E1204` | nguồn không NFC |
| `E1301` | `gritc fmt` không đặt được comment (phương án cuối; là lỗi của fmt) |
| `E1401` | `#allow` không hợp lệ (thiếu/rỗng lý do, mã ĐỎ/lạ, thừa tham số) |
| `E9901` | ICE (panic bị bắt, bất biến nội bộ vỡ, pass hỏng): exit 2 |

### 17.6 VÀNG `S…` và GHI CHÚ `N…`

| Mã | Mô tả |
|---|---|
| `S101` | secret trong nguồn (mẫu nhận dạng) |
| `S102` | literal entropy cao gán cho tên gợi ý secret |
| `S201` | `declassify` trên `Secret<T>` |
| `S202` | giá trị nhạy cảm chảy tới sink `io` |
| `N0601` `N0602` `N0603` `N0604` | check tràn / biên / chia / ép kiểu hạ thành runtime |
| `N0605` | phân tích giảm độ chính xác (> 24 biến zone hoặc hết ngân sách bước) |
| `N0606` | `--checks=proven` là chế độ thử nghiệm |
| `N0901` `N0902` `N0903` `N0904` | `⊳` / `⊲` / lượng từ có biên / `⋈` hạ thành runtime |
| `N0905` | nhiên liệu còn sau vòng lặp hạ thành runtime (cạn ⇒ abort `fuel_exhausted`) |
| `N1401` | `#allow` không tắt cảnh báo nào |

---

<a id="s18"></a>
## 18. Giới hạn đã biết

Tóm tắt `docs/KNOWN_LIMITATIONS.md` (đọc bản gốc để có số liệu đột biến chi tiết). Nguyên tắc: nói thẳng; "không chứng minh được" luôn thành check lúc chạy hoặc ĐỎ, không bao giờ im lặng chấp nhận.

### 18.1 Hạn chế chủ ý của Phase 1 (không phải thiếu sót)

- Không có: closure và `|x|`, `dyn`/trait object, macro, async, GAT/specialization, hàm biến đổi tham số, `cfg`, nội suy chuỗi, chuỗi nhiều dòng, raw string, iterator tổng quát, `?`, destructure tuple ở `≔`, `+=`/`-=`, toán tử bit (dùng hàm `band bor bxor bnot shl shr rotr rotl`, chỉ `u32`), comment `/* */`, or-pattern, guard, `..base`, `■ μ`, outlives `'a: 'b`, trait generics, where-clause.
- Dành riêng, chưa có nghĩa: `\extern` (FFI, D14), `\where`, `f32`. `%` trên số thực, `as` từ số thực sang nguyên: `E0319`.
- `unsafe` do người dùng viết: mọi `⚠` là `E1001` (D21). Con trỏ thô chỉ trong `lib/core`.
- Range mở (`a..`, `..b`, `..`) chỉ làm chỉ số lát và trong pattern; range đóng làm giá trị độc lập chỉ dùng làm đối tượng `⟳ x in …`.
- Prelude (`lib/core/prelude.gt`) thân hàm **không được kiểm** (intrinsic trusted); hàm của prelude không có thân nên không thể bổ sung bằng Grit trong `lib/core` (hàm mới đặt ở `lib/std`).
- Giá trị `f64` là bí ẩn với bộ giải; vòng lặp có điều kiện/biến thể số thực không dùng được; dấu/payload NaN không đặc tả.
- `Drop` của người dùng bị bỏ qua ở codegen (Phase 1 chỉ có `Drop` giả trong prelude). `spawn` không có `join`.

### 18.2 Giới hạn kiểm tra và bằng chứng

<a id="s18-2"></a>

- **Chỉ là bằng chứng thực nghiệm.** Borrowck: NLL sound nhưng không complete (dương tính giả đo được 0,92% trên chương trình oracle-sạch); bằng chứng gồm corpus, đột biến toàn crate, oracle động trên 20 000 chương trình ngẫu nhiên; hai lỗ hổng soundness thật đã tìm thấy và vá; còn 4 đột biến chưa giải quyết (3 theo hướng chặt hơn, 1 phụ thuộc việc chưa có mutability bên trong ngoài `Cell`).
- **Bộ giải M7** (khoảng + zone) sound, không complete; **69 đột biến "xoá thêm check" còn mở** ⇒ mặc định giữ mọi check (`--checks=all`); "chắc chắn sai" dựa trên khả năng tới của phân tích nên có thể từ chối chương trình hợp lệ (hướng chặt hơn). Hàm > 24 biến nguyên: `N0605`; vòng thứ 13 trở đi có biến thể 2 biến có thể `E0702` oan: tách hàm.
- **Dừng (M8):** biến thể chỉ tuyến tính (±1 hệ số); bước tiến nhân đôi (`w ← w + w`) hay `lo ← lo + w` chưa chứng minh được (dùng mẫu [nhiên liệu](#s4-3) ở 4.3); đệ quy qua giá trị hàm/phương thức trait luôn cần `div`; kiểm dừng phụ thuộc `select_impl` (chọn sai impl có thể bỏ sót chu trình).
- **Đa luồng:** không TSan, không loom, không oracle động cho đua dữ liệu; thông dịch tuần tự không phải bằng chứng cho codegen; chỉ `Atomic` cho `bool i32 u32 i64 u64 usize`; khoá chỉ qua `Mutex::with`.
- **IR Guard:** nghĩa vụ nhạy cảm kiểm theo **lớp**, không theo từng điểm; Guard-post đếm abort theo (hàm, loại), không theo vị trí; `release`/`release-o2` đặt bộ tối ưu LLVM vào TCB.
- **Quét bí mật** (`S101 S102 S201 S202`) là kinh nghiệm, không đầy đủ; taint theo thân hàm, không xuyên hàm.
- **Unicode:** chỉ phát hiện trộn bảng chữ trong một định danh và `µ`; **không** phát hiện định danh thuần một bảng chữ lạ trông như Latin (cần bảng confusables UTS #39). Danh sách ký tự vô hình là hữu hạn.
- **Dòng trống/comment:** `gritc fmt` chuẩn hoá bố cục, không bảo toàn dòng trống; `E1301` là phương án cuối.
- **Nền tảng:** đã kiểm **Linux x86_64**; **Windows: chưa kiểm**. `main` chạy trên luồng 256 MiB; M8 chứng minh *dừng*, không chứng minh *cận ngăn xếp* (đệ quy quá sâu bị HĐH giết, `gritc run` báo mã 2). CI mới viết, chưa chạy trên GitHub.
- **I/O:** `Cap<Fs>::read/open` đọc nguyên tệp vào bộ nhớ (không giải phóng); không đệm stdout (flush sau mỗi lời gọi, gom đầu ra rồi ghi một lần nếu cần tốc độ). `Cap<Env>::arg` đổi đối số không-UTF-8 thành lossy.
- **Backend llvm chưa hạ được:** phép nhân/chia 128 bit, `f64 as` (đã bị `E0319`), `Cap<Fs>::sub`… ⇒ `gritc run`/`build` thoát `3`, không sinh exe.
- Các tệp trong `lib/std` đổi băm khiến `rig.lock` của gói phụ thuộc lệch (phải `rig lock` lại).

### 18.3 Khác biệt spec ↔ gritc đã quan sát

<a id="s18-3"></a>

Tại thời điểm viết, tài liệu này ghi hành vi thật của `gritc`. Các khác biệt với `docs/spec/*`:

| Điểm | Spec nói | `gritc` thực tế |
|---|---|---|
| `⇒` | chỉ dùng trong hợp đồng | được nhận trong biểu thức thường (`true ⇒ false` qua `check`) |
| `declassify` | cần `Cap<Secret>` (semantics §7, D13) | `declassify(self)` của prelude không nhận capability và prelude không có `Cap<Secret>`; chỉ báo `S201` |
| Tên hiệu ứng | `io mem st div ffi lock(L)`; `ffi` bị cấm Phase 1 | tên hiệu ứng không được kiểm chính tả; `{ffi}` và `{zzz}` vẫn qua `check` |
| Nhánh `⌥` dư thừa | `N0301` (đề xuất) | không có `N0301`; nhánh sau nhánh bắt-tất-cả im lặng |
| Move `Arena` khi còn khoản mượn | `regions.md` §4 ghi `E0406` | báo `E0405` (đúng với `docs/errors/E0405.md`) |
| `gritc run` không có `main` | `run` đòi đúng một `ƒ main()` (lỗi dùng ⇒ 3) | exit `2` với thông báo "lỗi nội bộ codegen" |
| Mảng → lát | for-in "slice/array qua `&`" | `⟳ x in &a` với mảng dùng được, nhưng đối số hàm `&[T]` không nhận `&a` của mảng (`E0301`): cắt `&a[..]` |
| Backend (semantics/KNOWN_LIMITATIONS cũ) | "backend duy nhất là thông dịch" | `llvm` là mặc định khi bản dựng có; `check` in `backend: interpreter` trong JSON |
| Tên module trong tệp đơn | — | `main` (cần `⇲ main::geo::area;`, `⇲ geo::area;` là `E0201`) |

**Nghi lỗi compiler (chưa sửa, đừng dựa vào):** chương trình `x ≔ 1.5; x as i32` (biến nhận literal số thực **không chú thích kiểu**) qua `gritc check` (chỉ ghi chú `N0604`) thay vì `E0319`, rồi `gritc run` thoát `3` ("backend llvm chưa hỗ trợ: kiểm ép kiểu Float → Int"). Khi chú thích `x: f64` hoặc dùng `1.5f64` thì đúng là `E0319`. Đây là kẽ hở của lệnh cấm "`as` từ số thực sang nguyên" (D1/Q70) chưa được chặn ở suy luận literal.

### 18.4 Seed 1.0 — giới hạn bề mặt ngôn ngữ/thư viện (nói thẳng; cách né)

- `Option::is_some` và `Option::unwrap` chỉ là **khai báo hợp đồng** của prelude, không có thân MIR: gọi chúng trong chương trình chạy thật thì backend llvm từ chối (mã 3, "hàm không có thân MIR"); và vì `unwrap` đòi `⊳ self.is_some()` với đối số không `Copy`, gritc thường báo `E0902` ngay ở `check`. **Né:** `⌥ x { Some(v) ↦ …, None ↦ … }`, `unwrap_or`, `is_none` (gói `std`).
- `==`/`≠` chỉ cho số nguyên, `bool`, `char`, `&str` (tuple, `&T`, `Option`, ADT: `E0301`). **Né:** so từng trường, `*a == *b`, `cmp(..).is_eq()` cho kiểu `Ord`.
- Hàm của prelude không có thân (bodiless) nên không bổ sung được bằng Grit trong `lib/core`; hàm mới đặt trong `lib/std` (gói Rig `std`).
- Mỗi lần sửa mã của gói Rig phải `rig lock` rồi mới `rig build/run`; rig từ chối khi lệch.
- **D7 cấm biến/tham số cục bộ trùng TÊN MODULE (tên tệp `.gt`) ở bất kỳ gói nào** trong đồ thị (không gian tên phẳng): nếu tệp của bạn là `base.gt` mà thư viện `std` có biến cục bộ `base` thì `E0205` trỏ vào `std.gt`. `lib/std` giảm nhẹ bằng hậu tố `_`; nếu vẫn gặp, **đổi tên tệp của bạn** (`base.gt` → `base_util.gt`).
- Khác: không có `Ord` cài sẵn trong prelude (người dùng tự `⊐ Ord for X`; `lib/std` cài cho các kiểu thường dùng, không cho `f64`, `str`, `&[T]`, `u128/i128`); `map`/`and_then` chỉ nhận con trỏ hàm `! ∅`; `Text` dựng rồi đóng băng (muốn nhiều chuỗi sống độc lập cần Arena riêng); `Vec<Vec<T>>`/`Vec<Text>` là `E0318`; `⌥` trên `u128` luôn cần `_`.

---

<a id="s19"></a>
## 19. Phụ lục: prelude tóm tắt

Nguồn: `lib/core/prelude.gt` (chữ ký có nghĩa; thân là intrinsic trusted). Tham chiếu chữ ký/hiệu ứng/abort đầy đủ của gói `std`: `docs/STD_REFERENCE.md` (sinh bằng `tools/gen_docs.py`).

| Nhóm | Nội dung |
|---|---|
| enum cơ bản | `Option<T> { None, Some(T) }`, `Result<T, E> { Ok(T), Err(E) }`, `Ordering { Less, Equal, Greater }`, `AtomicOrdering`, `IoError { NotFound, Denied, Invalid, Other }` |
| range | `Range<T>` (`a..b`), `RangeInclusive<T>` (`a..=b`) |
| trait lang | `Copy`, `Send`, `Sync`, `Drop`, `Ord { cmp(&self, &Self) → Ordering }`, `AtomicInt` |
| `core::` | `abort(&str) → !`, `exit(u32) → !` (đều `! {div}`), `fuel_left(u64)`, `f64_from_bits(u64) → f64` |
| bit (`u32`) | `band bor bxor bnot shl shr rotr rotl` (`shl shr rotr rotl` đòi `⊳ n < 32`) |
| `f64` | `sqrt to_i64 → Option<i64> to_bits is_nan` |
| `str` | `len as_bytes slice(from, to)` |
| `[T]` | `len` |
| capability | `Cap<Fs Net Env Clock Alloc Out In ErrOut>`; nguồn gốc `root_fs() root_net() root_env() root_clock() root_alloc() root_out() root_in() root_err()` |
| bộ nhớ | `Arena`, `Vec<'a, T>`, `Text<'a>` ([mục 9](#s9)) |
| đồng thời | `spawn`, `Atomic<T>`, `Mutex<T, L>`, `Cell<T>` ([mục 12](#s12)) |
| `File` | `len`, `bytes() → &'static [u8]` |
| `Secret<T>` | `declassify(self) → T` |
| `Option<T>` | `is_some`, `unwrap` (xem [18.4](#s18)) |

Gói `std` (`lib/std`, Grit thường qua cùng cổng gritc): `Ordering::reverse/then/is_lt/is_eq/is_gt`; `Option::is_none/unwrap_or/map/and_then/or/ok_or`; `Result::is_ok/is_err/unwrap_or/map/map_err/and_then/ok/err`; `std::min/max`; API của `Vec` (trên phần tử `Copy`); phân tích/định dạng số (`parse_digits`, `fmt_u64`…); `Ord` cho số nguyên/`char`/`bool`/tuple/`[T]`/`Text`; `Hash`, `HashMap`. Dùng bằng khối ` ```grit std ` trong tài liệu (thêm gói `std` vào đầu vào).

```grit std value=3
ƒ main() → u32 ! ∅ { std::min(3u32, 9u32) }
```

```grit std value=8
ƒ main() → i32 ! ∅ { Some(3).unwrap_or(0) + None.unwrap_or(5) }
```
