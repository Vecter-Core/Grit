# Ngữ nghĩa lõi Grit — M0 DRAFT

Nguồn sự thật là GRIT_PLAN.md §4 (D1–D29). File này chỉ **cụ thể hoá** để M1–M7 có chỗ bám.
Chỗ nào plan chưa nói và agent phải chọn → đánh dấu **[ĐỀ XUẤT]** và có mục trong OPEN_QUESTIONS.

## 1. Kiểu
- **Kiểu nguyên thuỷ (danh sách chính thức):** `i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 isize usize f32 f64 bool char str`.
  (Giang yêu cầu ghi ở "§3"; §3 là biến & luồng nên danh sách nằm ở §1 — nội dung đúng như yêu cầu.)
  `isize/usize` rộng bằng con trỏ của mục tiêu (Phase 1: x86-64 ⇒ 64 bit). `char` = một Unicode scalar value (không phải surrogate).
  `str` không biết kích thước, chỉ dùng sau `&`/`&!` (`&str`). Unit là `()`.
- Phức hợp (cú pháp như Rust): tuple `(A, B)`, 1-tuple `(A,)`, unit `()`; array `[T; N]` (N là hằng lúc biên dịch);
  slice `[T]` (không biết kích thước — chỉ hợp lệ sau `&`/`&!`, tức `&[T]`, `&![T]`); `▢` struct, `⊔` enum.
  Truy cập trường tuple `t.0`, trường struct `p.x`; literal struct `Point { x: 1, y: 2 }`; literal array `[1, 2, 3]`, `[0; 8]`.
- `Option<T>`, `Result<T,E>` là enum thường trong `lib/core` (M12), không phải kiểu đặc biệt của compiler.
- Không có `null`, không có biến chưa khởi tạo, không exception, không unwinding (D3).
- Không ép kiểu ngầm (D1). `e as T` chỉ cho phép khi không mất thông tin; ngược lại sinh Obligation
  (M7) hoặc phải dùng toán tử có tên. **[ĐỀ XUẤT]** `as` giữa số nguyên: sinh Obligation "giá trị vừa kiểu đích".
- `f32/f64`: IEEE 754; `==`/`≠` trực tiếp là ĐỎ (D4).
- Suy luận chỉ cục bộ, bidirectional; chữ ký hàm luôn tường minh.

## 2. Số học (D2)
- `+ - * / %` trên nguyên: **kiểm tràn**. Mỗi phép sinh một Obligation `OVERFLOW`; chia/`%` thêm
  `DIV_NONZERO`; `-x` trên kiểu có dấu thêm "x ≠ MIN"; shift thêm "0 ≤ n < bits".
- Bộ giải P1 (M7) chứng minh bằng hằng + miền khoảng + điều kiện nhánh + `⊳`. Chứng minh được → xoá;
  không chứng minh được mà kiểm lúc chạy được → runtime check + **N0601** (D24/D25).
- `+% -% *%` quấn vòng (không Obligation); `+^ -^ *^` bão hoà; `+? -? *?` trả `Option<T>`.
- Số thực không có Obligation tràn; NaN/inf theo IEEE.
- Lỗi runtime không khôi phục (check thất bại) = abort tức thì, exit code riêng (M10).

## 3. Biến & luồng điều khiển
- Bất biến mặc định (D5). `μ` mới cho phép `←`. Cấm shadowing (D7): khai báo trùng tên trong cùng
  hàm (kể cả khối con) → E0205.
- `⎇` là biểu thức; hai nhánh phải cùng kiểu; thiếu `⇝` thì nhánh `then` phải kiểu `()`.
- **Khối & dấu `;` (Q8, đã duyệt):** biểu thức cuối khối không `;` = giá trị khối; có `;` = câu lệnh, khối trả `()`. Hàm có kiểu trả
  khác `()` mà khối kết thúc bằng `;` không có `⏎` → E0307. `⎇ ⌥ ⟳ ⚠` ở vị trí câu lệnh kết thúc tại `}`, không cần `;`.
- **Điều kiện/đối tượng không được là struct literal trần:** trong `⎇ cond`, `⟳ cond`, `⌥ scrut` biểu thức ngoài cùng không được là
  `Tên { … }` (mơ hồ với khối); phải bọc `( Tên { … } )`. Ví dụ `⎇ (p == Point { x: 0, y: 0 }) { … }`.
- **Vòng lặp (Q10, Q11 — đã duyệt):** `⟳` là ký hiệu vòng lặp duy nhất, ba dạng: `⟳ { … }` vô hạn (bắt buộc hiệu ứng `div`, D12); `⟳ cond { … }` là while;
  `⟳ x in xs { … }` là for-in (`in` từ khoá cứng ASCII; biến lặp bất biến). Đều nhận `↓ m` và `⋈ inv` tuỳ chọn. Chi tiết termination ở §21.
- `⌥` phải đầy đủ (D6) → E0305. Mẫu trùng thừa → N0301 **[ĐỀ XUẤT]**.
- `⟳` — xem §21 (termination của while / for-in / vòng vô hạn). `⋈` (bất biến): Phase 1 kiểm lúc chạy trong `dev`, không dùng để chứng minh (chờ 8.A).
- Mọi đường điều khiển của hàm có kiểu trả về khác `()` phải trả giá trị (E0307).

## 3a. Thứ tự đánh giá và biến tạm (Giang, 2026-10-06, ADR-020 §9)
- **Trái sang phải.** Toán hạng trái rồi phải (`∧ ∨ ⇒` ngắn mạch); lời gọi: callee rồi đối số theo thứ tự; phương thức: toán hạng của chỗ người nhận rồi đối số, khoản mượn tự động (`&self`/`&!self`) tạo SAU đối số (autoref muộn); tuple/mảng/struct literal: theo thứ tự viết trong nguồn.
- **Gán `chỗ ← giá_trị`:** biểu thức con của chỗ (chỉ số, deref) trước, kiểm chỉ số, rồi giá trị, rồi ghi. Khác Rust (Rust đánh giá vế phải trước).
- **Biến tạm** chết ở cuối câu lệnh, trừ `tên [: T] ≔ &V;` / `≔ &!V;` với V không phải biểu thức chỗ: tạm sống tới cuối khối chứa câu lệnh.
- Lỗi lúc chạy (tràn, chia 0, chỉ số, ép kiểu mất thông tin) là `Assert` ⇒ abort tức thì, không unwind (D3).

## 4. Hiệu ứng (D10)
- Chữ ký hàm **luôn** có `! ∅` hoặc `! {…}`. Hiệu ứng đã định nghĩa: `io, mem, st, div, ffi, lock(L)`.
- Kiểm: khai báo ⊇ suy ra (thân hàm + hiệu ứng các hàm được gọi) → E0801/E0802.

## 5. Sở hữu, mượn, region (M6 + D26/D27)
- Giá trị có kiểu không `Copy` bị **move** khi gán/truyền; dùng lại → E0401.
- `&T` nhiều lượt cùng lúc; `&!T` duy nhất, loại trừ mọi `&`/`&!` khác còn sống (NLL: sống theo lần
  dùng cuối) → E0402/E0403. Trả tham chiếu tới local → E0404. Gán khi đang bị mượn → E0406.
- `Arena` là một region có tên `'r`; tham chiếu `&'r T` không được thoát khỏi phạm vi của arena → E0405.
- Elision lifetime (đã cài ở M4.5, E0315): chỉ khi chữ ký có đúng một vị trí tham chiếu vào; không có thì `'static`; còn lại phải ghi `'r`. Mô hình region đầy đủ và hợp đồng cho M6: `docs/spec/regions.md`.

## 6. Capability (D28/D29, M4.5)
- `Cap<Fs|Net|Env|Clock|Secret|Alloc>` là kiểu affine: không `Copy`, không `Default`, không có literal.
- Hàm dùng thao tác cần capability mà `Cap<…>` không có trong chữ ký → E0310 (lỗi kiểu).
- Chỉ `main` (hàm cấp cao `main` của một tệp) được gọi NGUỒN capability gốc (hàm trả `Cap<…>` mà không nhận `Cap<…>`, vd `root_fs()`); chia nhỏ qua hàm nhận `&Cap<…>` trong `lib/core` (intrinsic `trusted`). Mọi nơi khác gọi nguồn gốc → E0310 (ADR-019).

## 7. Secret (D13, M9)
- `Secret<T>`: không `Display`, không vào sink log/mạng/tệp. `declassify` đọc cần `Cap<Secret>` và
  gây **S201** (VÀNG). Literal giống key/token trong nguồn → **S101** (VÀNG). VÀNG vẫn chạy.

## 8. Hợp đồng (Phase 1)
- `⊳` (pre): người gọi phải chứng minh bằng P1; không chứng minh được → hạ runtime check (kiểm được)
  + N0901, hoặc ĐỎ E0901 nếu **chắc chắn sai**. `⊲` (post): kiểm lúc chạy ở `dev`. Kiểm tĩnh đầy đủ = 8.A.
- Lượng từ `∀ ∃` chỉ hợp lệ có biên (`∀ i ∈ 0..n : a[i] ≥ 0` — dùng `:` thay `.` vì `0..n.` mơ hồ với range/truy cập trường; Q14). Lượng từ không biên mà chưa giải → ĐỎ (D25).

## 9. Unsafe (D21)
- `⚠ ⊢p … { }` có cú pháp + sinh Obligation; Phase 1 gần như luôn E1001. Chỉ `lib/core` có `intrinsic trusted` (D22).

## 10. Module
- `▦` module, `⇲` use, `◈` pub. Mặc định private. Cấm vòng phụ thuộc (E0204). Thứ tự khai báo không quan trọng.

## 11. Đa luồng (M8)
- `spawn(f, arg)` — không closure ở Phase 1. Đối số chuyển sang luồng phải `Send`-like (tự suy).
- `Mutex<T, L>` có cấp khoá `L`; thứ tự khoá phải tăng nghiêm ngặt (E0502). Atomics ghi rõ ordering.

## 12. Token hoá (lexer) — quy tắc đã duyệt
- **[Q5] `μ` là ký hiệu dành riêng.** Test bắt buộc cho lexer (M1): nguồn `μs` phải cho đúng hai token
  `KW_MUT`, `IDENT(s)` — không bao giờ một `IDENT("μs")`; `μ s` cho cùng kết quả. Mọi `μ` (U+03BC) đều là `KW_MUT`,
  nên không có định danh nào chứa `μ`. Chuỗi token kỳ vọng đầy đủ: `tests/lexer/mu_split.tokens`.
- `true`/`false` là từ khoá ASCII (Q2); không có ký hiệu canonical.
- Alias và canonical phải cho cùng chuỗi token (M1 round-trip).

## 13. Từ khoá cứng & tên dành riêng (Giang, 2026-10-05)
- **Từ khoá cứng ASCII:** `self`, `Self`, `as`, `true`, `false`, `for` (phân biệt hoa/thường: `self` ≠ `Self`). Không dùng làm tên
  biến/hàm/tham số/kiểu/trường. `self` chỉ xuất hiện như tham số đầu trong `⊏`/`⊐` (`self`, `&self`, `&!self`); `Self` là kiểu của
  `⊏`/`⊐` đang viết, dùng ở vị trí kiểu. Dùng làm tên (vd `self ≔ 1;`, `self := 1;`, `Self ≔ 1;`) → **E0101** (parser; không thêm mã mới).
  Test: `tests/fail/E0101_self_as_var*.gt`, `tests/fail/E0101_selftype_as_var.gt`.
- **Ký hiệu dành riêng:** `μ` (Q5, luôn `KW_MUT`), `\extern` (token `KW_EXTERN`), `\where` (token `KW_WHERE`, Q7).
  `\extern`/`\where` chưa có nghĩa ở Phase 1: mọi chỗ dùng → E0101 (D14 cấm FFI người dùng; where-clause là Phase 2).
  Test: `tests/fail/E0101_extern_reserved.gt`, `tests/fail/E0101_where_reserved.gt`.
- Lexer nhận ra các từ khoá/ký hiệu này **trước** khi hình thành định danh, nên chúng không bao giờ thành `IDENT`.

## 14. `*`: giải tham chiếu (prefix) vs nhân (infix)
- Lexer **không** phân biệt: mọi `*` đơn là token `STAR`. (`*%`, `*^`, `*?` là token riêng theo khớp-dài-nhất.)
  `**` không phải token: `a ** b` = `STAR STAR` = `a * (*b)`.
- Parser (Pratt) phân biệt theo **vị trí**: ở nơi mong đợi một toán hạng (đầu biểu thức/câu lệnh, sau toán tử, sau `(`, `,`,
  `[`, `≔`, `←`, `↦`, `⏎`, `⊳`, `⊲`…) thì `STAR` là **prefix deref**, độ ưu tiên unary. Ngay sau một toán hạng đầy đủ
  thì là **infix nhân**.
- Ví dụ: `*p * *q` = `(*p) * (*q)`; `*r ← *r * 2;` (câu lệnh bắt đầu bằng `*` → đích gán là deref); `a * ;` → E0101 (thiếu toán hạng).
- Vì câu lệnh kết thúc bằng `;` (Q8, đã duyệt) nên `a;` rồi `*p ← 1;` không bao giờ bị đọc thành `a * p`. Đây là một lý do
  chống phương án "xuống dòng làm dấu kết thúc" (xem Q8).
- Mẫu: `tests/pass/19_deref_vs_mul.gt`, `tests/lexer/star_ctx.gt`, `tests/fail/E0101_star_dangling.gt`.

## 15. Lifetime `'a` vs literal ký tự `'x'` (lexer)
- Lifetime/region: `'` + định danh (ASCII, không có canonical). Ví dụ `'a`, `'r`.
- Quy tắc lexer (như Rust): gặp `'`. Nếu ngay sau là **đúng một ký tự** (hoặc một escape) rồi `'` đóng ⇒ literal ký tự (`'a'`, `'\n'`).
  Ngược lại, nếu sau `'` là XID_Start ⇒ lifetime (`'a`, `'abc`). Còn lại ⇒ E0101.
- **Thuật toán (quyết định bằng nhìn trước, không quay lui):** sau `'` đọc một codepoint `c` (hoặc một escape trọn vẹn `\n`, `\'`, `\u{…}`).
  Nếu ký tự kế tiếp là `'` ⇒ `CHAR(c)`. Ngược lại nếu `c` là XID_Start (không phải escape) ⇒ `LIFETIME`: đọc hết XID_Continue.
  Còn lại ⇒ E0101. Quy tắc tóm gọn của Giang: **`'x'` (có dấu đóng) = char; `'a` (không có dấu đóng) = lifetime.**
- Ví dụ: `'a'` CHAR · `'\n'` CHAR · `'\''` CHAR · `'é'` CHAR (một codepoint) · `'a` LIFETIME · `'abc` LIFETIME · `<'a, 'b>` hai LIFETIME ·
  `'ab'` ⇒ `LIFETIME(ab)` rồi `'` lẻ ⇒ **E0101** (char nhiều ký tự không hợp lệ) · `''` ⇒ E0101.
- Hệ quả: `<'a>`/`&'a T` đọc đúng nhờ không có `'` đóng ngay sau một ký tự. Test: `tests/lexer/quote_ctx.*`, `tests/lexer/lifetime_vs_char.*`,
  `tests/fail/E0101_char_multi.gt`.
- Elision (đề xuất cũ) giữ nguyên: chỉ khi có đúng một tham số tham chiếu; còn lại phải ghi `'a`.

## 16. `≡` type alias, `□` const, `■` static (Giang)
- `≡ Tên<…> ≔ Kiểu;` — bí danh kiểu, không tạo kiểu mới (không phải newtype).
- `□ TÊN: Kiểu ≔ biểu_thức;` — hằng, biểu thức phải tính được lúc biên dịch (M4 kiểm; không hằng được → lỗi kiểu E03xx).
- `■ TÊN: Kiểu ≔ biểu_thức;` — static **bất biến**, một vùng nhớ duy nhất. **[Đã xác nhận — Giang]** Phase 1 không có `■ μ`
  (D11 cấm biến toàn cục khả biến, trừ kiểu đồng bộ/hiệu ứng `st` thuộc M8); static chứa atomic/mutex để M8 quyết định.
- Cả ba có thể đứng sau `◈` (pub). Mẫu: `tests/pass/20_const_static_type.gt`.

## 17. Dừng chương trình tường minh (`abort`)
- Không có macro `panic!` và không có ký hiệu riêng (D8). Lỗi không khôi phục dùng **hàm** `core::abort(msg: &str)` trong `lib/core` (M12):
  abort tức thì, không unwinding, không cleanup, exit code riêng (D3, M10).
- Hiệu ứng: `abort` khai báo `! {div}` (Giang) — hàm gọi nó phải có `div` trong tập hiệu ứng (D10) hoặc dùng nó trong nhánh mà bộ kiểm hiệu ứng
  cho phép theo M8.
- **Never (Q12, đã duyệt):** `abort` trả về kiểu `!` (never) — xem §22. Nhánh `⇝ { core::abort("…") }` có kiểu `!` nên hợp nhất được với nhánh kia.

## 18. Nguyên thủy đồng thời (concurrency primitives)
- `spawn(f, args)` là **hàm** trong `lib/core`, không phải từ khoá. Phase 1 không có closure: `f` là tên hàm (không bắt biến), `args` là giá trị
  chuyển sang luồng mới và phải `Send`-like (M8). Chữ ký dùng kiểu hàm tĩnh `ƒ(A) → R ! E` (Q12, đã duyệt): không bắt biến, không có closure.
- **Atomic ordering** là enum thường trong `lib/core`, không phải từ khoá: `Relaxed`, `Acquire`, `Release`, `AcqRel`, `SeqCst`.
  Mọi thao tác atomic bắt buộc truyền ordering tường minh (plan M8); không có ordering mặc định.
- `Mutex<T, L>` và cấp khoá: xem §11.

## 19. Đặc tả lexer cho literal và comment
- **Số nguyên:** `dec_int`/`0x`/`0o`/`0b`, hậu tố `i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize`. `_` chỉ nằm giữa các chữ số (không đầu,
  không cuối, không liền hậu tố: `1_000u32` hợp lệ vì `_` giữa chữ số; `1000_u32` **không** hợp lệ ở bản nháp này — Q14). Cần ≥1 chữ số thật sau tiền tố.
- **Số thực:** `d.d`, `d e d`, hậu tố `f32/f64`; `1.` và `.5` không hợp lệ; `1f64` là float. `0x1e3` là hex int (không phải float).
- **Literal ngoài phạm vi kiểu:** `128i8` → lỗi biên chắc chắn (E0601, M7). Dấu `-` là toán tử prefix, nhưng `-` áp trực tiếp lên một literal số
  được gấp trước khi kiểm phạm vi, nên `-128i8` hợp lệ (Q14).
- **Range:** `1..2` lex thành `INT DOTDOT INT` (không phải float). `..=` là `DOTDOTEQ`.
- **Chỉ số tuple:** sau token `.`, lexer đọc **chỉ chữ số thập phân** thành `INT_LIT` (không bao giờ float) ⇒ `t.0.1` = `t . 0 . 1`. Test: `tests/lexer/tuple_index.*`.
- **Chuỗi:** UTF-8, không xuống dòng thô (không có chuỗi nhiều dòng — Phase 2/chưa có), không nội suy; escape `\n \r \t \0 \\ \' \" \u{hex}`; escape lạ → E0101.
- **Comment:** `//` → **không vào luồng token và không vào AST, nhưng được giữ trong bảng trivia `LexOutput.trivia`** (văn bản nguyên văn + span + `own_line`) để `gt-fmt` gắn lại, không xoá comment của người dùng (ADR-012); `///` → `DOC_COMMENT` giữ lại, gắn vào node kế tiếp; `/*`/`*/` → **E0101** (Phase 1). `///` ngay trước `}` hoặc hết file
  (không có node để gắn) → E0101. `////…` (≥4 dấu `/`) là `//` thường. Văn bản `DOC_COMMENT` bỏ đúng một khoảng trắng đầu (nếu có).
  Test: `tests/pass/22_comments.gt`, `tests/lexer/comments.*`, `tests/fail/E0101_block_comment.gt`, `tests/fail/E0101_doc_comment_dangling.gt`.
- Không có `+= -= …`: `x += 1` lex thành `x`, `+`, `=` và `=` đứng riêng là E0101. Test: `tests/fail/E0101_compound_assign.gt`.

## 20. Biểu thức: range, tuple, struct, pattern
- **Range (Q14):** `a..b` (không gồm `b`), `a..=b` (gồm `b`), và dạng **mở**: `a..` (đến hết), `..b` (từ đầu, không gồm `b`), `..` (toàn bộ). Độ ưu tiên thấp nhất (dưới `⇒`).
  Dạng mở có nghĩa **chỉ** làm chỉ số slice (`xs[1..]`, `xs[..2]`, `xs[..]`) và trong pattern. Range mở trong biểu thức độc lập (ví dụ `r ≔ 1..;`) **chưa có ngữ nghĩa — chờ iterator Phase 2**:
  compiler từ chối (E0301). Range đóng `a..b`/`a..=b` làm giá trị độc lập chỉ dùng được làm đối tượng của `⟳ x in a..b`.
- **Tuple:** `()` unit, `(e)` nhóm, `(e,)` 1-tuple, `(a, b)` tuple. `t.0` truy cập phần tử.
- **Struct:** literal `Point { x: 1, y: 2 }` (đủ mọi trường, không có `..base`, không có viết tắt `{ x, y }` ở literal); truy cập `p.x`.
- **Pattern:** `_` wildcard · `x` gán tên · literal (kể cả `-1`) · `Variant(p, …)` · `Path { x, y: p }` (destructure struct; `x` = `x: x`) · tuple `(p, q)`
  · range đóng `0..10`, `0..=9` · range mở `10..`, `..0` · rest `..` (một lần mỗi danh sách, vd `(a, ..)`, `Variant(.., z)`, `Path { x, .. }`).
  Chưa có: or-pattern `a | b`, guard, binding `μ x` trong pattern, destructure ở `let` (KNOWN_LIMITATIONS).
- **Con trỏ thô (Q13, đã duyệt):** xem §23.

## 21. Termination của vòng lặp (D12, M8) — Q10, Q11 đã duyệt
(Giang yêu cầu "§7"; §7 là Secret nên phần này là §21 — nội dung đúng như yêu cầu.)
- **Vô hạn** `⟳ { … }` (không cond): bắt buộc hiệu ứng `div` của hàm (D12), không cần `↓`. Thiếu `div` → E0701.
- **while** `⟳ cond { … }`: termination được **suy từ cond** — không bắt buộc `↓`. Bộ kiểm M8 tìm biến thể từ dạng mẫu: cond so sánh một biến với một cận,
  và thân đưa biến đó về phía cận bằng bước hằng (`i ← i + c`, `i ← i - c`). Suy ra được → qua. **Không suy ra được và không có `↓` → E0701 (fail-closed, D12/P3;
  "không biết" ≠ "an toàn")**. Nếu có `↓ m` thì compiler **xác minh** `m` giảm nghiêm ngặt mỗi vòng và bị chặn dưới trên kiểu well-founded; sai → E0702.
  (Cách hiểu của agent về "suy từ cond, không bắt buộc ↓" — nhất quán với D12 vì suy luận thất bại vẫn bị từ chối. Xem ADR-008.)
- **for-in** `⟳ x in xs { … }`: với đối tượng hữu hạn — range đóng `a..b`/`a..=b`, slice/array (qua `&`) — termination **tự suy** (số phần tử còn lại hữu hạn và giảm
  mỗi vòng), không cần `↓`. `↓` nếu có vẫn được xác minh. Đối tượng không chứng minh được hữu hạn (Iterator tổng quát — M12/Phase 2) → E0701.
- **Phần tử của for-in:** trên range cho giá trị (`⟳ i in 0..n` → `i: u32`); trên `&[T]`/`&[T; N]` cho `&T`; trên `&![T]` cho `&!T`. Biến lặp bất biến.
- Phạm vi tên: `break`/`continue` (`⊠ ⊡`) hợp lệ trong cả ba dạng. Đệ quy giữ nguyên: cần `↓` (E0703) hoặc `div`.

## 22. Kiểu never `!` (Q12 đã duyệt)
- `!` là một kiểu nguyên thuỷ ở vị trí kiểu, không có giá trị, **gán được cho mọi kiểu** (bottom). Cần cho: nhánh gọi hàm không trả về trong `⎇`/`⌥`, và `match` đầy đủ
  khi một nhánh không bao giờ trả về (nhánh `!` hợp nhất với kiểu của các nhánh còn lại). `⏎`, `⊠`, `⊡`, vòng vô hạn không `⊠` cũng có kiểu `!`.
- Cú pháp: `ƒ abort(msg: &str) → ! ! {div}` — `→ !` là kiểu trả về never, `! {div}` là tập hiệu ứng. Sau `→` parser luôn đọc một kiểu nên không mơ hồ.
  `&!` là token mượn-duy-nhất; "tham chiếu tới never" không được hỗ trợ (và vô nghĩa).
- **`#noreturn`:** thuộc tính chỉ dành cho **intrinsic trusted trong `lib/core`** (D22): khẳng định hàm không trả về mà compiler không thể kiểm (thân không phải Grit).
  Người dùng không được viết `#noreturn` (E1001: tự khẳng định mà không có bằng chứng). Hàm Grit thường có thể khai báo `→ !` nếu **mọi đường đi đều phân kỳ**
  (kết thúc bằng gọi hàm `!` hoặc vòng vô hạn); compiler chứng minh điều đó (M5), không tin lời. (Cách hiểu của agent về quan hệ `#noreturn` ↔ `→ !` — ADR-008.)
- `core::abort(msg: &str) → !`, hiệu ứng `div` (xem §17).

## 23. Con trỏ thô (Q13 đã duyệt)
- Kiểu: `*□ T` (con trỏ tới T chỉ đọc) và `*μ T` (con trỏ tới T ghi được); ASCII: `*` + `\const` / `*` + `\mut`. Cú pháp con trỏ thô chỉ được **dùng bên trong khối `⚠`**.
- Dùng ngoài `⚠` → **E1001** (tái dùng, không thêm E1002). Ngoại lệ duy nhất: intrinsic trusted trong `lib/core` (D22), vì chúng cần con trỏ thô trong chữ ký.
- Trong `⚠`, Phase 1 mọi thao tác con trỏ thô sinh Obligation (hợp lệ, căn lề, không alias…) và hầu hết ĐỎ E1001 (D21).

## 24. Lexer: xuống dòng, khoảng trắng, ký tự điều khiển, alias — D15 (chi tiết M1; ADR-009, ADR-010)
- **Xuống dòng:** `\n` hoặc `\r\n` (CRLF coi là một xuống dòng — repo có thể được checkout với CRLF trên Windows). `\r` lẻ không đi kèm `\n` → **E1202**.
- **Khoảng trắng hợp lệ:** dấu cách, tab, xuống dòng. Mọi khoảng trắng Unicode khác (NBSP, U+2028/2029, …) → **E1202** (có thể là ký tự vô hình/giả dạng).
- **Ký tự điều khiển/vô hình:** C0 (trừ tab/xuống dòng), DEL, C1, và `U+200B–200D, U+2060, U+FEFF` → **E1202**, ở mọi nơi kể cả trong chuỗi/comment (fail-closed; dùng
  escape `\u{…}` trong chuỗi nếu cần). BOM đầu file (U+FEFF) cũng là E1202. Bidi `U+202A–202E, U+2066–2069` → **E1201** ở mọi nơi (kể cả comment/chuỗi, D15).
- **NFC:** nguồn không phải NFC → **E1204**. **Định danh trộn bảng chữ** (hai script khác nhau, không kể Common/Inherited) → **E1203**. Chưa phát hiện được
  homoglyph "cả định danh cùng một script khác" (vd toàn Cyrillic trông như Latin): xem KNOWN_LIMITATIONS.
- **Alias ASCII:** `\` + `[a-z][A-Za-z0-9_]*` (chạy hết dãy chữ/số/`_`), tra bảng; không có trong bảng → E0101. Toán tử ASCII dùng khớp dài nhất:
  `-> := <- <= >= != => == &! ..= ..` và `+% -% *% +^ -^ *^ +? -? *?`. Hệ quả: `x<-1` lex là `x <- 1` (gán), viết `x < -1` có khoảng trắng.
- **Ký tự không thuộc ngôn ngữ** (`@ $ ? ~ ^ | ` hoặc `=` đứng riêng…) → E0101.
- Token `EOF` luôn có ở cuối (độ dài 0). Lexer không bao giờ panic; mỗi lỗi có span, lexer tiếp tục để báo nhiều lỗi (D19).
- **Token `UNKNOWN`:** mọi ký tự/đoạn không hợp lệ (ký tự lạ, `=` đứng riêng, alias `\…` không tồn tại, `/*`, `*/`, literal số/chuỗi/char hỏng, `µ`) phát đúng **một** token `UNKNOWN`
  phủ đoạn đó, kèm một `LexError` — để parser (M2) có điểm neo khôi phục lỗi. Chuỗi hợp lệ không bao giờ chứa `UNKNOWN`. Ký tự bị cấm ở prescan (bidi/vô hình/điều khiển)
  chỉ báo lỗi, không phát token (coi như khoảng trắng).

## 25. gt-fmt — dạng chuẩn (Giang duyệt 2026-10-05)
- Một dạng chuẩn duy nhất: `gritc fmt` in canonical Unicode; hai bề mặt (canonical/ASCII) của cùng chương trình cho **cùng đầu ra**.
- **Dòng trống:** một dòng trống giữa hai item cấp cao nhất liền kề; không dòng trống thừa; dòng trống người dùng không được bảo toàn.
- **Comment (Q16b):** `//` giữ nguyên số lượng, nội dung và thứ tự (bảng trivia, ADR-012). Gắn theo **token gần nhất**: dòng riêng → token kế tiếp, cuối dòng → token liền trước; sau comment cuối dòng ép xuống dòng. E1301 chỉ khi xác minh sau khi chèn thất bại (phương án cuối). Chi tiết và ví dụ trước/sau: ADR-016.
- AST không có ngoặc nhóm nên printer thêm ngoặc theo độ ưu tiên và theo luật `expr_ns` (struct literal trần trong điều kiện `⎇ ⌥ ⟳` phải bọc ngoặc).

## 25b. `x as T < y` (Q16c, Giang duyệt 2026-10-06) — quy tắc NGÔN NGỮ như Rust
- Sau `as`, một kiểu đường dẫn theo sau bởi `<` được đọc là **danh sách tham số kiểu** của kiểu đó (`x as Vec<u8>`). Vì vậy `x as i64 < y` KHÔNG phải "so sánh `x as i64` với `y`": nó là lỗi cú pháp (E0101). Phải viết **`(x as i64) < y`**. Các toán tử so sánh khác (`>`, `≤`, `≥`, `==`, `≠`, `∈`) không bị ảnh hưởng.
- Parser phát MỘT E0101 trỏ vào `<`, kèm gợi ý **`machine-applicable`** gồm hai edit nguyên tử: chèn `(` ở đầu vế trái của chuỗi `as` và `)` ngay sau kiểu (`a + b as i64 < c` → `a + (b as i64) < c`; `x as i32 as i64 < a` → `(x as i32 as i64) < a`). Gợi ý chỉ được phát khi phần sau `<` parse được thành biểu thức (nếu không thì là lỗi tham số kiểu thường, không gợi ý). Mơ hồ vốn có: `x as Vec<i64 ;` trùng chuỗi token với `(x as Vec) < i64;` nên được coi là so sánh.
- gt-fmt không "giải quanh": nó in `(x as T) < y` vì dạng không ngoặc không còn là chương trình hợp lệ.
## 26. Độ sâu, stack và an toàn đệ quy (luật — ADR-014, ADR-015)
- **`MAX_DEPTH` = 256** nằm đúng MỘT chỗ: hằng `gt_ast::MAX_DEPTH`. Mọi bộ phận (parser và các pass đệ quy sau này) dùng hằng này, không tự đặt giới hạn riêng.
- **Độ sâu** = số mức lồng nhau của biểu thức, khối, kiểu, pattern, `▦` (tính chung một bộ đếm khi trộn lẫn) **cộng độ dài chuỗi toán tử nhị phân/postfix/`as`** (chuỗi phẳng tạo cây lệch trái sâu như nhau). Vượt `MAX_DEPTH` → ĐỎ **E0102** ("tách biểu thức con ra biến phụ bằng `≔`") và dừng phân tích. E0102 là lỗi người dùng, không phải ICE.
- **Hợp đồng stack:** một chương trình sâu đúng `MAX_DEPTH` mức ở MỌI loại lồng phải được xử lý **trong bản debug** mà không tràn stack. Mọi pass đệ quy trên AST phải chạy qua helper `gt_stack::run_with_big_stack` (một chỗ chung); panic nội bộ thành ICE **E9901** (exit 2, không chạy). Test `depth_law` của parser kiểm: với mỗi loại lồng, độ sâu cực đại tìm được parse thành công (không E9901) và cực đại + 1 cho E0102.
