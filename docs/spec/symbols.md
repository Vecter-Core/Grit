# Bảng ký hiệu Grit — bản ĐỀ XUẤT M0 (chờ Giang duyệt, xem ADR-002)

Quy tắc: mỗi ký hiệu canonical có **đúng một** bí danh ASCII (1–1). Lexer chấp nhận cả hai;
`gritc fmt` in canonical. Hai dạng phải cho **cùng AST** (test round-trip, M1).
Bí danh dạng `\tên` chỉ hợp lệ khi ngay sau `\` là chữ cái ASCII thường và kết thúc ở ranh giới
không phải chữ/số/`_`. Bí danh toán tử (`<-`, `->`, `:=`, `<=`...) dùng quy tắc "khớp dài nhất".

> Trạng thái: **ĐỀ XUẤT**. Các dòng đánh dấu (M0★) là phần plan §4.2 chưa nêu — do agent đề xuất,
> cần Giang duyệt. Chưa có số đo token (xem `docs/OPEN_QUESTIONS.md` Q2) nên chưa "đóng băng".

## Từ khoá / khai báo

| Ý nghĩa | Canonical | ASCII | Nguồn |
|---|---|---|---|
| Định nghĩa hàm | `ƒ` | `\fn` | plan |
| Bind bất biến | `≔` | `:=` | plan |
| Tiền tố khả biến | `μ` | `\mut` | plan |
| Ghi đè | `←` | `<-` | plan |
| Kiểu trả về | `→` | `->` | plan |
| Vòng lặp | `⟳` | `\loop` | plan |
| Biến thể giảm | `↓` | `\dec` | plan |
| Tiền / hậu điều kiện | `⊳` / `⊲` | `\pre` / `\post` | plan |
| Bất biến vòng lặp | `⋈` | `\inv` | plan |
| Khối unsafe | `⚠` | `\unsafe` | plan |
| Mệnh đề bằng chứng | `⊢` | `\proof` | plan |
| Tập hiệu ứng rỗng | `∅` | `\nil` | plan |
| if | `⎇` | `\if` | M0★ |
| else | `⇝` | `\else` | M0★ |
| match | `⌥` | `\match` | M0★ |
| nhánh match | `↦` | `=>` | M0★ |
| return | `⏎` | `\ret` | M0★ |
| struct | `▢` | `\struct` | M0★ |
| enum | `⊔` | `\enum` | M0★ |
| module | `▦` | `\mod` | M0★ |
| use | `⇲` | `\use` | M0★ |
| pub | `◈` | `\pub` | M0★ |
| trait | `⊏` | `\trait` | M0★ |
| impl | `⊐` | `\impl` | M0★ |
| break / continue | `⊠` / `⊡` | `\break` / `\cont` | M0★ (kế thừa seed cũ) |
| true / false | `⊤` / `⊥` | `true` / `false` | M0★ |

## Toán tử

| Ý nghĩa | Canonical | ASCII |
|---|---|---|
| Và / Hoặc / Phủ định (ngắn mạch) | `∧ ∨ ¬` | `\land \lor \neg` |
| Suy ra (chỉ trong hợp đồng) | `⇒` | `\implies` |
| Với mọi / Tồn tại (chỉ trong hợp đồng, có biên) | `∀ ∃` | `\forall \exists` |
| So sánh | `≤ ≥ ≠ == < >` | `<= >= != == < >` |
| Thuộc (chỉ trong hợp đồng) | `∈` | `\in` |
| Mượn chung / duy nhất | `&` / `&!` | `&` / `&!` |
| Số học kiểm tràn (mặc định) | `+ - * / %` | giống |
| Quấn vòng / bão hoà / trả Option | `+% -% *%` / `+^ -^ *^` / `+? -? *?` | giống (đã ASCII) |
| Region có tên | `@r` | `@r` |

## Ràng buộc đã chốt từ plan
- Không ép kiểu ngầm (D1): ép kiểu dùng `e as T` (từ khoá ASCII `as`, không ký hiệu — hiếm gặp, ngắn sẵn).
- `==` trên `f32/f64` là ĐỎ (D4).
- Không shadowing (D7).
- Mỗi ký hiệu canonical là **một codepoint** (ngoại lệ: `&!`, `+%`… là toán tử ASCII sẵn).
- Cấm cứng (D15): bidi (U+202A–202E, U+2066–2069), zero-width (U+200B–200D, U+2060, U+FEFF),
  định danh trộn bảng chữ. Nguồn phải là NFC.
- Định danh: XID_Start/XID_Continue, **ngoại trừ** mọi codepoint nằm trong bảng trên (để `ƒ`, `μ`
  không bị nuốt thành định danh; `μ` là chữ Hy Lạp nên lexer coi `μ` đứng riêng + ranh giới là
  tiền tố khả biến — xem grammar §Lexical, đây là điểm cần test kỹ ở M1).
