<!-- TOKEN_PROXY: ~8960 (ước lượng = số byte UTF-8 / 3; giới hạn 12000; kiểm bởi tools/test_ai_guide.py) -->
# AI_GUIDE — viết chương trình Grit (gói ngữ cảnh TỰ CHỨA cho AI)
Gặp lỗi `E0702`/`E0902`/`E0301`/`E0403` mà logic có vẻ đúng? Xem `docs/AI_PITFALLS.md` (mẫu hay viết sai, xếp theo tần suất, có TRƯỚC/SAU chạy thật).

Grit là ngôn ngữ biên dịch (`gritc`, qua LLVM) **fail-closed**: chương trình chỉ chạy khi mọi kiểm tra an toàn đều qua. Mục tiêu của bạn: viết `src/main.gt` sao cho `rig check` XANH rồi `rig run` cho đúng kết quả. **KHÔNG BAO GIỜ nới kiểm tra** (không `--checks=proven`, không `#allow` để lách, không đổi đề bài); lỗi `E…` là thông tin để sửa MÃ của bạn. Danh sách hàm thư viện: `docs/STD_REFERENCE.md`. Nền đã kiểm: Linux x86_64 (Windows: chưa kiểm).

## 1. Quy trình (làm đúng thứ tự)
```text
rig new tên_gói        # tạo tên_gói/ (rig.toml, src/main.gt, rig.lock; ĐÃ có phụ thuộc std). Tên: chữ thường/số/_ , bắt đầu bằng chữ cái
cd tên_gói             # sửa src/main.gt; mỗi tệp src/x.gt là một module `x` (tên tệp = tên module)
rig lock               # BẮT BUỘC sau MỌI lần sửa mã (rig từ chối nếu rig.lock lệch, mã 10)
rig check              # kiểm, không dựng      →  gritc: phán quyết green|red
rig run                # dựng + chạy           →  target/tên_gói.exe (đối số chương trình: chạy tệp .exe trực tiếp)
```
- Thử nhanh một tệp đơn (KHÔNG có `std`): `gritc check f.gt`, `gritc run f.gt` (in giá trị `main` trả về). Máy đọc: thêm `--json`.
- Đọc kết quả: dòng `gritc: phán quyết green|red; N lỗi, …`. Chỉ `error[E…]` là lỗi phải sửa. `note[N06xx/N09xx]` (kiểm tra hạ thành lúc chạy) và `S…` (vàng) là BÌNH THƯỜNG, bỏ qua. Mã thoát: 0 xanh · 1 đỏ (lỗi mã của bạn) · 2 lỗi nội bộ · 3 backend/công cụ; chương trình chạy: 70 = abort có kiểm soát.
- Mã thoát của CHƯƠNG TRÌNH của bạn chỉ đặt được bằng `core::exit(code)`. Giá trị `main` trả về KHÔNG thành mã thoát.
- Sau mỗi lần biên dịch đỏ: đọc MỘT lỗi đầu tiên, sửa, chạy lại (lỗi sau thường là hệ quả).

## 2. Ký hiệu (canonical ↔ ASCII; viết kiểu nào cũng được, `rig fmt` in canonical)
| ý nghĩa | canonical | ASCII | | ý nghĩa | canonical | ASCII |
|---|---|---|---|---|---|---|
| hàm | `ƒ` | `\fn` | | struct / enum | `▢` / `⊔` | `\struct` / `\enum` |
| gán bất biến | `≔` | `:=` | | impl / trait | `⊐` / `⊏` | `\impl` / `\trait` |
| biến khả biến (tiền tố) | `μ` | `\mut` | | công khai (pub) | `◈` | `\pub` |
| ghi đè | `←` | `<-` | | module / use | `▦` / `⇲` | `\mod` / `\use` |
| kiểu trả về | `→` | `->` | | hằng | `□ N: T ≔ v;` | `\const` |
| if / else | `⎇` / `⇝` | `\if` / `\else` | | match / nhánh | `⌥` / `↦` | `\match` / `=>` |
| vòng lặp | `⟳` | `\loop` | | return | `⏎` | `\ret` |
| break / continue | `⊠;` / `⊡;` | `\break` / `\cont` | | biến thể giảm | `↓` | `\dec` |
| tiền / hậu điều kiện | `⊳` / `⊲` | `\pre` / `\post` | | hiệu ứng rỗng | `∅` | `\nil` |
| và / hoặc / không | `∧ ∨ ¬` | `\land \lor \neg` | | so sánh | `≤ ≥ ≠ == < >` | `<= >= != == < >` |
| mượn chung / độc quyền | `&` / `&!` | `&` / `&!` | | wrap / bão hoà / Option | `+% +^ +?` | (giống) |
`!` chỉ dùng cho: hiệu ứng (`! ∅`, `! {io}`) và kiểu never (`→ !`). Phủ định luận lý là `¬` (KHÔNG có `!x`, `&&`, `||`).

## 3. Khung chương trình, capability, hiệu ứng
- Mọi hàm khai báo **hiệu ứng**: `! ∅` (thuần), `{io}` (I/O), `{mem}` (cấp phát Arena), `{div}` (có thể không dừng/thoát). Hàm gọi hàm có hiệu ứng thì phải khai báo ⊇ (E0801). `main` thường là `! {io, div, mem}`.
- I/O cần **capability**: chỉ `main` được gọi `root_out() root_err() root_in() root_env() root_fs()`; truyền xuống bằng tham chiếu (`&Cap<Out>`). `main` không khai return type = trả `()`.
```grit std
ƒ greet(out: &Cap<Out>, name: &str) ! {io} {
  out.write_str("xin chào ");
  out.write_str(name);
  out.write_str("\n");
}

ƒ main() ! {io, div, mem} {
  out ≔ root_out();
  err ≔ root_err();
  env ≔ root_env();
  ⎇ env.arg_count() < 2usize {
    err.write_str("usage: prog NAME\n");
    core::exit(2u32)
  } ⇝ { }
  greet(&out, env.arg(1usize));   // env.arg(0) là tên chương trình; vượt số đối số = abort
}
```
- Đọc stdin / tệp (nội dung sống tới hết tiến trình, kiểu `&'static [u8]`):
```grit std run
ƒ main() ! {io, div, mem} {
  inp ≔ root_in();
  out ≔ root_out();
  fs ≔ root_fs();
  data ≔ ⌥ inp.read_all() { Ok(f) ↦ f.bytes(), Err(_) ↦ "".as_bytes() };   // stdin → &[u8]
  out.write_u64(data.len() as u64);
  out.write_str("\n");
  ⌥ fs.open("không_có_tệp_này") {                  // Result<File, IoError>; Cap<Fs>::read abort nếu lỗi, dùng open
    Ok(f) ↦ { out.write_bytes(f.bytes()); },
    Err(_) ↦ { out.write_str("cannot open\n"); },
  }
  fs.write("/tmp/grit_guide_probe.txt", data);   // Result<(), IoError>; kết quả bị bỏ qua ở đây
}
```
```output
0
cannot open
```

## 4. Kiểu, số, chuỗi
- Số nguyên: `u8 u16 u32 u64 usize i8 i16 i32 i64 isize` (+`u128/i128`). Literal nên có hậu tố (`1u64`); KHÔNG có ép kiểu ngầm: trộn kiểu = E0302 (`u32 + u64`). Ép tường minh `x as u64`: **abort `cast_lossy` nếu giá trị không vừa** (ví dụ `i64` âm `as u64`) — widening an toàn luôn đúng.
- `+ - *` abort `overflow` khi tràn; `/ %` abort khi chia 0 — gritc chứng minh được thì bỏ kiểm, không thì giữ (ghi chú N0601): đó là BÌNH THƯỜNG. Muốn không abort: `a +% b` (quấn vòng), `a +^ b` (bão hoà), `a +? b` → `Option` (None khi tràn); tương tự `-% *% -^ *^ -? *?`.
- Bool `∧ ∨ ¬` ngắn mạch. `==` CHỈ cho số nguyên, bool, char, **`&str`**; tuple/`&T`/Option/struct/lát: E0301 — dùng `*a == *b`, so từng trường, `cmp(..).is_eq()` (kiểu `Ord`) hoặc `a.equals(b)` cho `&[u8]`. Số thực `f64`: `==` bị cấm (E0303).
- `&str`: literal `"..."` (escape `\n \t \\ \"`), `s.len()`, `s.as_bytes()`, `s.slice(i, j)` (abort nếu cắt giữa ký tự UTF-8); std thêm `starts_with ends_with find parse_u64 parse_i64`. Xử lý văn bản = xử lý **BYTE** (`&[u8]`): `b[i]` là `u8`, `b'a'` KHÔNG có (dùng `97u8`).
- Mảng `[0u8; 24]`, `[1u32, 2u32]`; lát `&a[i..j]` / `&!a[i..j]` (độ dài kiểm lúc chạy). `&[T; N]` KHÔNG tự thành `&[T]` khi truyền vào hàm (E0301): viết `&a[0usize..N]` (N là hằng độ dài); `⟳ x in &a` thì được. Tuple `(1u32, true)`, truy cập `.0 .1`; KHÔNG có phân rã `(a, b) ≔ f();` — viết `t ≔ f(); a ≔ t.0;`.
- struct / enum / impl / const:
```grit run value=130
□ LIMIT: u32 ≔ 10;
▢ P { x: i32, y: i32 }
⊔ Shape { Dot, Circle(u32), Rect(u32, u32) }
⊐ P { ƒ sum(&self) → i32 ! ∅ { self.x + self.y } }
ƒ area(s: Shape) → u32 ! ∅ {
  ⌥ s {
    Shape::Dot ↦ 0u32,
    Shape::Circle(r) ↦ r * r * 3u32,
    Shape::Rect(w, h) ↦ w * h,
  }
}
ƒ main() → u64 ! ∅ {
  p ≔ P { x: 3, y: 4 };
  t ≔ (7u32, true);
  (p.sum() as u64) + (area(Shape::Rect(2u32, 3u32)) as u64) + 100u64 + (t.0 as u64) + (LIMIT as u64)
}
```
- `⌥` phải VÉT CẠN (E0305); số nguyên/chuỗi luôn cần nhánh `_`. Tên trong pattern là biến MỚI (không được trùng tên khác: §10).
- `⎇` là biểu thức: `x ≔ ⎇ c { 1u32 } ⇝ { 2u32 };`; nối chuỗi `⎇ a { … } ⇝ ⎇ b { … } ⇝ { … }`. Nhánh `⇝` có thể bỏ khi là câu lệnh.
- Struct/enum chỉ dùng lại sau khi chuyển (move) nếu `⊐ Copy for T { }` (kiểu `Copy`: số, bool, char, `&T`, struct toàn trường Copy đã khai `Copy`); không thì E0401 (dùng sau move). `Option<T>`/`Result` Copy khi `T` Copy.

## 5. Vòng lặp và đệ quy (nơi AI hay đỏ nhất) — mọi vòng lặp phải được CHỨNG MINH dừng
- `⟳ cond { … }` (while): gritc tự suy ra khi `cond` so một biến với một cận VÀ thân đổi biến đó bằng bước hằng: `⟳ i < n { …; i ← i + 1usize; }`. Có `⊠;` (break) bên trong cũng được.
- Khi không suy ra được (E0701) hoặc biến đổi không đều: thêm biến thể `↓ biểu_thức_không_âm_giảm_nghiêm_ngặt`:
```grit std run value=20
ƒ main() → u64 ! ∅ {
  b ≔ "a b  c d\n".as_bytes();
  n ≔ b.len();
  μpos ≔ 0usize;
  μwords ≔ 0u64;
  ⟳ pos < n ↓ n - pos {                          // ↓ n - pos: giảm mỗi vòng (pos tăng ≥ 1) và ≥ 0
    end ≔ b.find_byte_or_end(pos, 32u8);          // vị trí khoảng trắng kế tiếp (hoặc n); có hậu điều kiện result >= pos
    ⎇ end > pos { words ← words + 1u64; } ⇝ { }
    pos ← end + 1usize;                            // luôn > pos ⇒ biến thể giảm
  }
  words * 5u64
}
```
  **Tách dòng/từ**: dùng `b.find_byte_or_end(pos, byte)` (có `⊲ result >= pos` nên M8 chứng minh `pos ← end + 1usize` luôn tăng). Hàm TỰ VIẾT trả chỉ số cũng cần hậu điều kiện: `ƒ next(b: &[u8], from: usize) → usize ! ∅ ⊲ result >= from { … }` — KHÔNG có nó thì `pos ← f(pos) + 1` bị E0702 vì gritc không biết `f(pos) ≥ pos`.
  Quy tắc của biến thể: biểu thức tuyến tính theo biến số nguyên (và `xs.len()`); phải giảm NGHIÊM NGẶT mỗi vòng trên MỌI đường (E0702 nếu có đường không giảm, kể cả `i ← 0`/gán hằng); không dùng gán hằng để "thoát sớm" (dùng `⊠;` hoặc đẩy chỉ số tới `n` bằng phép CỘNG/BIẾN: `i ← n`là gán BIẾN nên được nếu có thể chứng minh giảm); cận dẫn xuất kiểu `n - m + 1` thường KHÔNG được: dùng `¬ ∧ ∨` trong điều kiện thay vì cận phức.
- Vòng lặp không có điều kiện `⟳ { … }` cần hiệu ứng `div`. Vòng không chứng minh được dừng → dùng nhiên liệu: `μfuel ≔ 100000u64; ⟳ fuel > 0u64 ↓ fuel { …; fuel ← fuel - 1u64; } core::fuel_left(fuel);` (cạn ⇒ abort `fuel_exhausted`).
- `⟳ i in 0..n { }` (biến lặp bất biến). 
- **Đệ quy** cần biến thể trên một tham số đếm (ngân sách) giảm ở MỌI cạnh của chu trình (E0703 nếu thiếu):
```grit run value=120
ƒ fact(n: u64, budget: u32) → u64 ! ∅ ↓ budget {
  ⎇ budget == 0u32 ∨ n <= 1u64 { 1u64 } ⇝ { n * fact(n - 1u64, budget - 1u32) }
}
ƒ main() → u64 ! ∅ { fact(5u64, 100u32) }
```
  Đệ quy tương hỗ (`expr→term→primary→expr`): mỗi hàm có `↓ budget` và mỗi lời gọi truyền `budget - 1`. `main` chạy trên ngăn xếp 256 MiB.

## 6. Arena, Vec, HashMap, sắp xếp (bộ nhớ động)
- `μar ≔ Arena::new();` (hiệu ứng `{mem}`). `Vec<'a, T>`/`Text<'a>`/`HashMap` đều trỏ vào Arena (CHIA SẺ: một Arena cho nhiều Vec). KHÔNG có free/drop từng phần; Arena sống tới hết hàm. **Phần tử Vec/HashMap không được cần drop** (E0318); các hàm của std (`get`, `pop`, `contains`, HashMap…) đòi thêm `T: Copy`: dùng số, bool, char, `&T`, struct có `⊐ Copy for T { }`, `std::Bytes`.
- Khung nhìn lấy từ Vec (`v.as_slice()`) vô hiệu khi `push`/`insert`/`clear`… tiếp theo (E0403) hoặc sau `ar.reset()` (E0405): lấy lại khung nhìn sau mỗi lần sửa.
```grit std run
ƒ main() ! {io, div, mem} {
  out ≔ root_out();
  μar ≔ Arena::new();
  μv ≔ Vec::new(&ar);                 // kiểu phần tử suy ra từ lần dùng đầu
  v.push(30u64); v.push(10u64); v.push(20u64);
  v.insert(1usize, 5u64);             // chèn tại min(i, len)
  ⌥ v.pop() { Some(x) ↦ { out.write_u64(x); out.write_str(" "); }, None ↦ { } }
  μsl ≔ v.as_mut_slice();
  sl.sort();                          // [T: Ord + Copy] heapsort, KHÔNG ổn định
  μi ≔ 0usize;
  ⟳ i < sl.len() { out.write_u64(sl[i]); out.write_str(","); i ← i + 1usize; }
  out.write_str("\n");
  ⌥ v.get(0usize) { Some(x) ↦ { out.write_u64(x); }, None ↦ { } }   // get/first/last trả Option
  out.write_str(" ");
  out.write_u64(v.len() as u64);
  out.write_str("\n");
}
```
```output
20 5,10,30,
5 3
```
- `Vec`: `new push pop get first last len is_empty clear truncate insert remove extend_from_slice contains as_slice as_mut_slice`. `Vec<u8>`: thêm `push_str push_u64 push_i64` (dựng đầu ra rồi `out.write_bytes(v.as_slice())`).
- **HashMap** (khoá `Copy + Ord + Hash`: `u64`, `usize`, `std::Bytes`; giá trị `Copy`). Khoá chuỗi/byte: bọc `std::Bytes { b: lát }`. Nên đặt kiểu qua suy luận (không cần viết `<'a, K, V>` trong `main`); hàm nhận map thì viết `m: &!std::HashMap<'a, std::Bytes<'static>, u64>`.
```grit std run
ƒ main() ! {io, div, mem} {
  out ≔ root_out();
  μar ≔ Arena::new();
  μm ≔ std::HashMap::new(&ar);
  words ≔ ["b", "a", "b", "c", "b", "a"];
  μi ≔ 0usize;
  ⟳ i < 6usize {
    k ≔ std::Bytes { b: words[i].as_bytes() };
    cur ≔ ⌥ m.get(&k) { Some(c) ↦ c, None ↦ 0u64 };
    m.insert(k, cur + 1u64);          // insert = thêm hoặc ghi đè
    i ← i + 1usize;
  }
  kb ≔ std::Bytes { b: "b".as_bytes() };
  ⌥ m.get(&kb) { Some(c) ↦ { out.write_u64(c); }, None ↦ { out.write_str("-"); } }
  out.write_str(" ");
  out.write_u64(m.len() as u64);
  out.write_str("\n");
}
```
```output
3 3
```
  API: `new insert get remove(&k) → bool contains_key len`; duyệt: `entry_count()` rồi `entry_at(i)` → `Option<(K, V)>` (thứ tự KHÔNG xác định; muốn thứ tự thì đưa vào Vec rồi `sort()` — cần struct có `⊐ Ord`).
- Số ⇄ chữ (std): `s.parse_u64()` / `s.parse_i64()` → `Result<_, std::ParseError>` (`Empty | Invalid | Overflow`; `[u8]` cũng có); in: `out.write_u64/write_i64/write_hex/write_byte`, hoặc dựng `Vec<u8>` bằng `push_u64`…; `fmt_u64(&!buf, n)` với `buf: [u8; 24]` trả chỉ số bắt đầu. Tìm kiếm byte: `find_byte(from, b)`, `find(pat)`, `starts_with`, `ends_with`, `trim_ascii`, `equals`. Số học không abort: `abs_i64`, `pow_u64` trả `Option`.
- `Option`/`Result` có `unwrap_or map and_then ok_or is_none is_ok is_err …` (nhận HÀM, không có closure). **`unwrap()` và `is_some()` KHÔNG dùng được** (chỉ là khai báo hợp đồng): dùng `⌥ x { Some(v) ↦ …, None ↦ … }`, `unwrap_or(d)`, `is_none()`.

## 7. Mượn (M6) tóm tắt
- `&x` mượn chung (nhiều cái), `&!x` mượn độc quyền (một cái, và không cùng lúc với `&x`); gán/di chuyển khi đang bị mượn: E0402/E0403/E0406. Biến muốn sửa phải khai `μ`: `μx ≔ 0u64;` (E0308 nếu sửa biến bất biến). Tham số `&!T` cho phép hàm sửa nơi gọi: gọi `f(&!x)`.
- Trả tham chiếu phải trỏ vào tham số/`'static`, không vào biến cục bộ (E0404). Hàm trả tham chiếu từ nhiều tham số tham chiếu cần vùng đời tường minh `ƒ f<'a>(x: &'a [u8], y: &[u8]) → &'a [u8]`.
- Một giá trị không-Copy dùng hai lần trong CÙNG câu lệnh (`(b, b)`, `f(b, b.x)`) là E0401.

## 8. Hợp đồng (tuỳ chọn) 
`ƒ f(x: u32) → u32 ! ∅ ⊳ x > 0 ⊲ result > 0 { … }`: `⊳` tiền điều kiện (kiểm tại chỗ gọi: chắc chắn sai ⇒ E0901; không chứng minh được ⇒ kiểm lúc chạy, abort `precondition`), `⊲` hậu điều kiện (`result` là giá trị trả về). Không cần cho chương trình thường; gọi hàm có `⊳` qua con trỏ hàm bị E0901.

## 9. 40 mã lỗi hay gặp → cách sửa
| mã | nghĩa | sửa |
|---|---|---|
| E0101 | cú pháp (token lạ/thiếu `;`/`}`; `=` đơn; `!x`; `/* */`) | gán là `←`/`≔`, so sánh `==`, phủ định `¬`, chú thích `//`; mỗi câu lệnh kết thúc `;`; trong `⌥` mỗi nhánh kết thúc bằng `,` |
| E0102 | lồng quá sâu (>256) | tách hàm |
| E0201 | tên chưa định nghĩa | sai chính tả; cần `std::tên` cho hàm tự do của std; hàm tự do của module khác cần `◈` và `module::tên` |
| E0202 | định nghĩa trùng | đổi tên |
| E0203 | item private | thêm `◈` trước item |
| E0204 | vòng module | gộp/tách module |
| E0205 | shadowing (D7) | mỗi tên (biến, tham số, binding pattern, **tên tệp module**, item, prelude như `Option`) chỉ khai MỘT lần trong phạm vi: đặt tên khác (`n1`, `n2`) |
| E0301 | sai kiểu / thiếu thông tin kiểu / `==` ngoài số-bool-char-&str | thêm chú thích `x: u64 ≔ …`; so từng trường |
| E0302 | ép kiểu ngầm số | `x as u64` hoặc hậu tố literal |
| E0303 | `==` trên số thực | so ngưỡng |
| E0304 | trường struct sai/thiếu/thừa | đủ mọi trường |
| E0305 | `⌥` không vét cạn | thêm nhánh `_ ↦ …` |
| E0307 | thiếu đường trả về / khối kết thúc bằng `;` mà hàm trả giá trị | bỏ `;` ở biểu thức cuối hoặc dùng `⏎ v;` |
| E0308 | gán vào biến bất biến / tham số | khai `μx ≔ …`; tham số cần sửa dùng `&!T` |
| E0309 | không có phương thức / ràng buộc trait không thoả | kiểm tên phương thức, kiểu nhận (`&self`/`&!self`), ràng buộc `<T: Ord + Copy>`; phương thức của std cần gói `std` trong rig.toml |
| E0310 | thiếu capability | gọi `root_*()` chỉ ở `main`, truyền `&Cap<…>` xuống |
| E0311 | `⊐` sai chữ ký/trùng | khớp chữ ký trait |
| E0313 | tạo `Cap`/`Secret` bằng literal | lấy `Cap` từ `root_*()` |
| E0315 | vùng đời `'a` sai | khai `<'a>` ở hàm; trả tham chiếu cần `'a` tường minh |
| E0318 | `Vec`/HashMap có phần tử cần drop | chỉ dùng phần tử `Copy`; khai `⊐ Copy for T { }` |
| E0401 | dùng sau move | giá trị không-Copy chỉ dùng một lần; truyền `&x`; hoặc khai `Copy` |
| E0402 | hai `&!` chồng nhau | thu hẹp phạm vi mượn thứ nhất |
| E0403 | `&!` chồng `&` (kể cả khung nhìn Vec qua `push`) | lấy lại `as_slice()` sau khi sửa |
| E0404 | tham chiếu sống lâu hơn giá trị | trả giá trị, hoặc tham chiếu tới tham số |
| E0405 | con trỏ thoát Arena / dùng sau `reset` | không giữ khung nhìn qua `ar.reset()` |
| E0406 | gán khi đang bị mượn | chờ mượn kết thúc |
| E0407 | move ra khỏi `*p`/`a[i]` | sao chép (`Copy`) hoặc đọc trường |
| E0501 | chia sẻ không `Send` qua luồng | (hiếm) |
| E0601 | tràn chắc chắn (hằng) | đổi kiểu rộng hơn / `+?` |
| E0602 | chia 0 chắc chắn | kiểm mẫu số trước |
| E0603 | chỉ số ngoài biên chắc chắn | kiểm `i < xs.len()` |
| E0701 | vòng lặp không chứng minh được dừng | thêm `↓ …` (§5) hoặc nhiên liệu |
| E0702 | `↓` không giảm nghiêm ngặt / không bị chặn dưới | mỗi đường phải giảm; tránh gán hằng; biểu thức tuyến tính Hàm có hơn 24 biến (hoặc hơn 24 vòng lặp) vượt giới hạn của bộ chứng minh và vòng ĐÚNG cũng bị E0702 (thông báo có lời nhắc): tách hàm. Thoát sớm bằng `⊠;`, không dùng cờ bool. |
| E0703 | đệ quy thiếu `↓` | tham số `budget` + `↓ budget` + gọi với `budget - 1` |
| E0801 | hiệu ứng suy ra ⊄ khai báo | thêm `{io}`/`{mem}`/`{div}` vào hàm (và hàm gọi nó) |
| E0802 | impl khai nhiều hiệu ứng hơn trait | khớp trait |
| E0901 | hợp đồng `⊳`/`⊲` chắc chắn sai (kể cả `⊳` qua con trỏ hàm) | sửa đối số hoặc gọi trực tiếp |
| E0902 | không kiểm được lúc chạy | tránh `⊳` trên tham số `&!`; hàm có `⊳`/`⊲` mà đối số là lát/`Vec` (không Copy) thì không kiểm được lúc chạy: bỏ hợp đồng (bounds check vẫn có) hoặc truyền chỉ số/độ dài |
| E1001 | `⚠`/con trỏ thô | không dùng |
| E1101 | IR Guard từ chối | lỗi compiler: báo, đừng lách |
| E9901 | lỗi nội bộ (ICE) | ghi lại tệp tối thiểu gây lỗi và báo |
| backend mã 3 "chưa hỗ trợ …" | phần chương trình compiler chưa cài | viết lại cách khác (vd `⌥` thay `unwrap`, so từng trường thay `==` tuple) |

## 10. Bẫy thường gặp
1. **Không shadowing**: dùng lại tên biến/tham số/binding trong phạm vi lồng, hoặc trùng **tên tệp module** của bất kỳ gói nào (kể cả `std`) ⇒ E0205 (nếu lỗi chỉ vào `std.gt`: đổi tên tệp của bạn, ví dụ `base.gt` → `base_util.gt`).
2. Mỗi lần sửa mã: `rig lock`. Quên ⇒ mã 10 "rig.lock không khớp".
3. Thứ tự đánh giá: đối số trái → phải; `∧`/`∨` ngắn mạch. Cùng một giá trị không-Copy hai lần trong một câu lệnh: E0401.
4. `as` abort khi mất thông tin; `+ - *` abort khi tràn (nếu không chứng minh được). Dữ liệu từ ngoài (số đọc từ stdin) nên dùng `+?`/so sánh trước.
5. Hàm tự do của module khác: `◈ ƒ f` và gọi `ten_module::f(…)`; hàm tự do của std: `std::tên`. Phương thức của kiểu (`v.push`, `s.parse_u64()`) không cần tiền tố.
6. `⌥` trên `u128` luôn cần nhánh `_`. Literal ≥ 2⁶⁴ cần hậu tố `u128`.
7. `check` XANH nhưng `rig run`/`gritc build` báo `backend llvm chưa hỗ trợ …` (mã 3): bạn dùng thứ chỉ-khai-báo (`Option::unwrap/is_some`, `Cap<Fs>::sub`, `==` trên kiểu ghép đã bị E0301): viết lại bằng cách khác. Bộ giải M8 không chứng minh được `a % b < b` hay `x ← x / 10` giảm: dùng nhiên liệu.
8. Không có: closure, `?`, phân rã tuple, `unwrap()`/`is_some()`, `==` trên kiểu ghép, `f32`, macro, `dyn`, async, `unsafe` cho người dùng, tham số lifetime lồng. Cách né ở các mục trên.
9. Văn bản là BYTE: tách dòng theo `\n` (10), cắt khoảng trắng 32/9/13. Đầu ra: `write_str` (literal), `write_bytes` (lát), `write_u64`/`write_i64` (số).
10. `main` không trả mã thoát: dùng `core::exit(2u32)` (hiệu ứng `{div}`) cho lỗi dùng sai; kết thúc bình thường ⇒ mã 0.
11. `spawn(f, a)` có hiệu ứng của worker `f`; `! ∅` KHÔNG nghĩa là "không có luồng".
12. Một Arena `Arena::new()` mỗi chương trình là đủ; dữ liệu nhập/tệp (`&'static [u8]`) KHÔNG cần Arena.

## 11. Mẫu hoàn chỉnh: đếm dòng chứa mẫu (đọc stdin, tách dòng, tìm chuỗi, thoát có mã)
```grit std run exit=1
ƒ main() ! {io, div, mem} {
  inp ≔ root_in();
  out ≔ root_out();
  data ≔ ⌥ inp.read_all() { Ok(f) ↦ f.bytes(), Err(_) ↦ "".as_bytes() };
  pat ≔ "ab".as_bytes();
  n ≔ data.len();
  μpos ≔ 0usize;
  μhits ≔ 0u64;
  ⟳ pos < n ↓ n - pos {
    end ≔ data.find_byte_or_end(pos, 10u8);
    line ≔ &data[pos..end];
    ⎇ line.find(pat).is_none() { } ⇝ { hits ← hits + 1u64; }
    pos ← end + 1usize;
  }
  out.write_u64(hits);
  out.write_str("\n");
  ⎇ hits == 0u64 { core::exit(1u32) } ⇝ { }
}
```
```output
0
```

## 12. Công thức hay dùng (rút từ bốn chương trình thật: csvstat, calc, kvstore, grep-lite)
- **Phân tích cờ đối số — thoát sớm bằng `⊠;`, KHÔNG dùng cờ bool** (cờ bool làm biến thể không giảm ⇒ E0702):
```grit std
ƒ main() ! {io, div, mem} {
  env ≔ root_env();
  argc ≔ env.arg_count();
  μcount ≔ false;
  μfirst ≔ argc;                       // chỉ số đối số vị trí đầu tiên
  μk ≔ 1usize;
  ⟳ k < argc ↓ argc - k {
    a ≔ env.arg(k);
    ⎇ a == "-c" { count ← true; k ← k + 1usize; } ⇝ {
      ⎇ a == "--" { first ← k + 1usize; ⊠; } ⇝ { first ← k; ⊠; }
    }
  }
  ⎇ count { root_out().write_str("count\n"); } ⇝ { }
}
```
- **Trạng thái dùng chung qua `&!Struct`**: `▢ St<'a> { b: &'a [u8], pos: usize }`; hàm nhận `st: &!St` ghi `st.pos ← st.pos + 1usize;` và truyền tiếp `st` cho hàm con (tự mượn lại, KHÔNG viết `&!st`). BẪY: `f(st, st.pos)` với `st: &!St` là **E0403** (đối số thứ nhất đã mượn độc quyền `st`, đối số thứ hai lại đọc `st.pos`; không có "mượn hai pha"): đọc trường vào biến trước (`p ≔ st.pos; f(st, p)`) hoặc để hàm con tự đọc `st.pos`.
- **Ghi vào lát khả biến**: `μsl ≔ v.as_mut_slice(); sl[i] ← x;` (lấy `as_mut_slice` SAU lần `push` cuối).
- **`i64::MIN`**: `-9223372036854775807i64 - 1i64`. Số học kiểm tràn không abort trả Option, dùng trực tiếp làm đối tượng `⌥`: `⌥ a *? b { Some(x) ↦ …, None ↦ { …overflow… } }`.
- **`core::exit` ở vị trí biểu thức**: kiểu `!` khớp mọi kiểu nên được dùng làm một nhánh: `data ≔ ⌥ inp.read_all() { Ok(f) ↦ f.bytes(), Err(_) ↦ { err.write_str("lỗi\n"); core::exit(2u32) } };`.
- **Byte → `&str` (tên tệp đọc từ dữ liệu)**: `⌥ Text::from_utf8(&ar, bytes) { Some(t) ↦ { fs.open(t.as_str()) … }, None ↦ … }` (cần Arena; trả `None` nếu không phải UTF-8 hợp lệ).
- **Sắp xếp struct**: `⊐ Ord for P { ƒ cmp(&self, o: &Self) → Ordering ! ∅ { self.k.cmp(&o.k) } }` (kèm `⊐ Copy for P { }`) rồi `sl.sort()`; `std::Bytes` đã là `Copy + Ord + Hash` (so theo byte).
- **Đệ quy tương hỗ (parser)**: mọi hàm trong chu trình có `↓ budget` và mỗi lời gọi truyền `budget - 1`; nhánh `budget == 0` xử lý riêng (báo lỗi). Ngân sách vài nghìn đủ cho đề bài; ngăn xếp 256 MiB.
- **Vòng lặp mà điều kiện dừng đổi qua hàm con** (`↓ n - pos` không chứng minh được): dùng nhiên liệu `μfuel ≔ len + 2usize; ⟳ fuel > 0usize ↓ fuel { …; fuel ← fuel - 1usize; }` rồi `core::fuel_left(fuel)`… hoặc thoát bằng `⊠;`.
- **Đầu ra nhiều dòng**: mỗi `write_*` là một lời gọi hệ thống (stdout không đệm). Với đầu ra lớn hãy dựng vào `Vec<u8>` (`push_str/push_u64/push_i64/extend_from_slice`) rồi `out.write_bytes(v.as_slice())` MỘT lần. `Cap<ErrOut>` (stderr) chỉ có `write_str` và `write_u64`.
- **`main` quá lớn** ⇒ ghi chú N0605 ("quá 24 biến cho miền quan hệ") và nhiều kiểm tra biên giữ lại lúc chạy (đúng nhưng chậm hơn): tách thành các hàm nhỏ. Dòng `backend: interpreter` của `check`/`rig build` chỉ là nhãn của giai đoạn kiểm; exe luôn là mã máy (llvm).
