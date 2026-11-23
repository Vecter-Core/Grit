# Bảng ký hiệu Grit — bản M0 FINAL (Q1–Q14 đã đóng); tên token chính thức đã chốt (ADR-010)

Quy tắc: mỗi ký hiệu canonical có **đúng một** bí danh ASCII (1–1). Lexer chấp nhận cả hai;
`gritc fmt` in canonical. Hai dạng phải cho **cùng AST** (test round-trip, M1).
Bí danh dạng `\tên` chỉ hợp lệ khi ngay sau `\` là chữ cái ASCII thường và kết thúc ở ranh giới
không phải chữ/số/`_`. Bí danh toán tử (`<-`, `->`, `:=`, `<=`...) dùng quy tắc "khớp dài nhất".

> Cột "Nguồn": **plan** = GRIT_PLAN §4.2 · **M0★✓** = agent đề xuất, Giang đã duyệt · **Giang** = Giang yêu cầu bổ sung ·
> **ASCII-only** = không có canonical Unicode, từ khoá/ký hiệu ASCII duy nhất.
> Số đo token là proxy tiktoken (ADR-002).

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
| if | `⎇` | `\if` | M0★✓ |
| else | `⇝` | `\else` | M0★✓ |
| match | `⌥` | `\match` | M0★✓ |
| nhánh match | `↦` | `=>` | M0★✓ |
| return | `⏎` | `\ret` | M0★✓ |
| struct | `▢` | `\struct` | M0★✓ |
| enum | `⊔` | `\enum` | M0★✓ |
| module | `▦` | `\mod` | M0★✓ |
| use | `⇲` | `\use` | M0★✓ |
| pub | `◈` | `\pub` | M0★✓ |
| trait | `⊏` | `\trait` | M0★✓ |
| impl | `⊐` | `\impl` | M0★✓ |
| break / continue | `⊠` / `⊡` | `\break` / `\cont` | M0★✓ |
| type alias | `≡` | `\type` | Giang |
| const (hằng lúc biên dịch) | `□` | `\const` | Giang |
| static (bất biến, Phase 1) | `■` | `\static` | Giang |
| extern (**dành riêng**, chưa có nghĩa — Phase 2) | — | `\extern` (token `KW_EXTERN`) | Giang, ASCII-only |
| where (**dành riêng**, chưa có nghĩa — Phase 2; Q7) | — | `\where` (token `KW_WHERE`) | Giang, ASCII-only |

## Từ khoá cứng ASCII (không có canonical, không dùng làm tên — semantics §13)

| Từ khoá | Vai trò |
|---|---|
| `self` | tham số đối tượng trong trait/impl (`self`, `&self`, `&!self`) — **đã duyệt: giữ ASCII** |
| `Self` | kiểu của chính `⊐`/`⊏` đang viết (viết hoa, phân biệt với `self`) — **đã duyệt** |
| `as` | ép kiểu tường minh `e as T` — **đã duyệt: giữ ASCII** |
| `true` / `false` | hằng boolean (Q2; không có `⊤ ⊥`) |
| `for` | trong `⊐ Trait for Type { … }` — **đã duyệt (Q7)** |
| `in` | trong `⟳ x in xs { … }` (for-in) — **đã duyệt (Q10)**. Khác `∈`/`\in` (toán tử thuộc trong hợp đồng): `in` trần là từ khoá, `\in` là bí danh của `∈` |

## Kiểu never và con trỏ thô (ở vị trí kiểu)

| Ý nghĩa | Cú pháp | Ghi chú |
|---|---|---|
| Kiểu never (không bao giờ trả về) | `!` (ASCII-only) | **đã duyệt (Q12)**. Chỉ ở vị trí kiểu: `ƒ abort(msg: &str) → ! ! {div}` (`→ !` là kiểu, `! {div}` là hiệu ứng). Xem semantics §22 |
| Con trỏ thô | `*□ T` / `*μ T` (canonical); `*\const T` / `*\mut T` (ASCII) | **đã duyệt (Q13)**. Chỉ hợp lệ trong `⚠`; ngoài `⚠` → E1001. Xem semantics §23 |
| Kiểu hàm tĩnh | `ƒ(A) → R ! E` | **đã duyệt (Q12)**; không bắt biến; dùng cho `spawn(f, args)` |
| Thuộc tính `#noreturn` | `#noreturn` | **đã duyệt (Q12)**; chỉ trên intrinsic trusted trong `lib/core` (D22) |

## Toán tử

| Ý nghĩa | Canonical | ASCII |
|---|---|---|
| Và / Hoặc / Phủ định (ngắn mạch) | `∧ ∨ ¬` | `\land \lor \neg` |
| Suy ra (chỉ trong hợp đồng) | `⇒` | `\implies` |
| Với mọi / Tồn tại (chỉ trong hợp đồng, có biên) | `∀ ∃` | `\forall \exists` |
| So sánh | `≤ ≥ ≠ == < >` | `<= >= != == < >` |
| Thuộc (chỉ trong hợp đồng) | `∈` | `\in` |
| Mượn chung / duy nhất (prefix) | `&` / `&!` | `&` / `&!` |
| Giải tham chiếu (prefix) **và** nhân (infix) | `*` | `*` — cùng một token `STAR`, parser phân biệt (semantics §14); **đã duyệt: giữ ASCII** |
| Số học kiểm tràn (mặc định) | `+ - * / %` | giống |
| Quấn vòng / bão hoà / trả Option | `+% -% *%` / `+^ -^ *^` / `+? -? *?` | giống (đã ASCII) |
| Lan truyền lỗi `e?` (**edition 2**, ADR-P2-09) | `?` hậu tố (token `QUESTION`, đứng riêng; `+?` `-?` `*?` vẫn là một token theo khớp-dài-nhất) | giống (đã ASCII) |

## Lifetime / region có tên

| Ý nghĩa | Canonical | ASCII |
|---|---|---|
| Lifetime/region có tên, vd `'a` | **không có canonical Unicode — chỉ dùng ASCII** | `'a`, `'r`, … (`'` + định danh) |

Thay thế `@r` của bản nháp M0 đầu (Giang yêu cầu giữ cú pháp `'a` như Rust). Phân biệt với literal ký tự `'x'`:
xem semantics §15.

## Dấu kết thúc câu lệnh (Q8 — đã duyệt)

`;` tường minh, quy tắc kiểu Rust: biểu thức cuối khối **không** `;` = giá trị khối; có `;` = câu lệnh, khối trả `()`.
Hàm trả kiểu khác `()` mà khối kết thúc bằng `;` không có `⏎` → **E0307**. Câu lệnh dạng khối
(`⎇ ⌥ ⟳ ⚠`) kết thúc tại `}` và **không** cần `;`.

## Comment (ASCII-only, không có canonical Unicode)

| Dạng | Token | Xử lý |
|---|---|---|
| `// …` đến cuối dòng | `LINE_COMMENT` | không vào luồng token/AST nhưng **được giữ** trong bảng trivia (`LexOutput.trivia`: văn bản nguyên văn, span, `own_line`) cho `gt-fmt`. `////…` (≥4 dấu `/`) cũng là `LINE_COMMENT` |
| `/// …` đến cuối dòng | `DOC_COMMENT` | lexer **giữ lại**, gắn vào node AST kế tiếp (item / field / variant / chữ ký trait); dùng ở Rig 8.F. Không có node kế tiếp → E0101 |
| `/* … */` | — | **CẤM ở Phase 1**: gặp `/*` hoặc `*/` → E0101. Dành riêng cho Phase 2 nếu cần |

Hệ quả: `a /*p` (chia cho deref không có khoảng trắng) bị đọc là mở comment khối → E0101; viết `a / *p`.

## Literal & dấu câu ASCII (không có canonical)

| Dạng | Quy tắc tóm tắt (chi tiết: grammar §Lexical, semantics §19) |
|---|---|
| số nguyên | thập phân, `0x` hex, `0o` bát phân, `0b` nhị phân; dấu `_` giữa chữ số (`1_000_000`); hậu tố `i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize` |
| số thực | `1.5`, `1e3`, `2.5e-3`, hậu tố `f32`/`f64`; `1.` không hợp lệ (xung đột với range) |
| ký tự / chuỗi | `'x'` (một codepoint), `"…"` (UTF-8, không xuống dòng thô), escape `\n \r \t \0 \\ \' \" \u{…}` |
| range | `a..b` (không gồm b), `a..=b` (gồm b); **dạng mở (Q14):** `a..` (đến hết), `..b` (từ đầu, không gồm b), `..` (toàn bộ) — chỉ có nghĩa làm chỉ số slice và trong pattern; token `DOTDOT`, `DOTDOTEQ` |
| thuộc tính | `#` + định danh (plan §5: `#allow(S101, "lý do")`) |

## Ràng buộc đã chốt
- Không ép kiểu ngầm (D1): ép kiểu dùng `e as T`.
- `==` trên `f32/f64` là ĐỎ (D4). Không shadowing (D7).
- Mỗi ký hiệu canonical là **một codepoint** (ngoại lệ: `&!`, `+%`… là toán tử ASCII sẵn).
- Cấm cứng (D15): bidi (U+202A–202E, U+2066–2069, cộng LRM/RLM/ALM) → E1201; zero-width/vô hình/điều khiển (U+200B–200D, U+2060, U+FEFF, …) → E1202;
  định danh trộn bảng chữ → E1203; nguồn phải là NFC → E1204. Chi tiết và cách hiểu: semantics §24, ADR-009.
- `ƒ` (U+0192) và `μ` (U+03BC) là chữ cái nhưng là ký hiệu dành riêng: không nằm trong bất kỳ định danh nào. `µ` (U+00B5) bị chặn vì trông giống `μ`.
- Định danh: XID_Start/XID_Continue, **ngoại trừ** mọi codepoint trong bảng trên và mọi từ khoá cứng.
- **`μ` là ký hiệu dành riêng (Q5):** mọi codepoint `μ` (U+03BC) đều là token `KW_MUT`; lexer không bao giờ
  đưa `μ` vào một định danh, dù dính liền ký tự khác. `μs` → `KW_MUT`, `IDENT(s)` (giống hệt `μ s`).
  Hệ quả: không định danh nào chứa `μ`; `aμb` → `IDENT(a) KW_MUT IDENT(b)` và bị parser từ chối (E0101, M2).
  `μ` ngoài vị trí khai báo (vd `μ ≔ 1`) cũng là E0101.
  Test: `tests/lexer/mu_split.gt` + `.tokens`, `tests/fail/E0101_mu_as_ident.gt`.
- `true` / `false` là từ khoá ASCII duy nhất cho hằng boolean (Q2); không có ký hiệu `⊤ ⊥`.
- `\extern` và `\where` được lexer nhận ra thành token riêng (`KW_EXTERN`, `KW_WHERE`) để không dùng làm tên;
  mọi chỗ dùng ở Phase 1 là E0101 (không production nào chấp nhận — D14 cấm FFI người dùng; where là Phase 2).
  Test: `tests/fail/E0101_extern_reserved.gt`, `tests/fail/E0101_where_reserved.gt`.
- **Toán tử bit/dịch bit (Q9 — đã duyệt):** không có ký hiệu riêng. Dùng hàm `band bor bxor bnot shl shr` trong `lib/core` (M12).
  Hệ quả: `|` không được dùng ở Phase 1 (để dành cho closure Phase 2); lone `|`, `^`, `~` là E0101.
- Không có toán tử gán kết hợp `+= -= …` (chủ ý): chỉ `x ← x + 1`. `=` đứng riêng không phải token hợp lệ (E0101); so sánh bằng là `==`.
- **Tên token chính thức (ADR-010):** từ khoá `KW_*`; ký hiệu `WALRUS ≔`, `FAT_ARROW ↦`, `LOOP_SYM ⟳`, `PROOF_SYM ⊢`, `EMPTY_SET ∅`, `LEQ GEQ NEQ`, `LAND LOR LNOT`, `BORROW &`, `BORROW_MUT &!`, `EQ_EQ`; literal `INT_LIT FLOAT_LIT STR_LIT CHAR_LIT`; `LIFETIME DOC_COMMENT IDENT EOF UNKNOWN`. Danh sách đầy đủ và mã nguồn: `crates/gt-lexer/src/token.rs`; test khoá: `crates/gt-lexer/tests/token_names.rs`.
