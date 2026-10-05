# Ngữ nghĩa lõi Grit — M0 DRAFT

Nguồn sự thật là GRIT_PLAN.md §4 (D1–D29). File này chỉ **cụ thể hoá** để M1–M7 có chỗ bám.
Chỗ nào plan chưa nói và agent phải chọn → đánh dấu **[ĐỀ XUẤT]** và có mục trong OPEN_QUESTIONS.

## 1. Kiểu
- Nguyên thuỷ có kích thước: `i8..i64, isize, u8..u64, usize, f32, f64, bool, char, str`; unit `()`.
- Phức hợp: tuple, array `[T; N]` (N là hằng lúc biên dịch), slice `&[T]`, `▢` struct, `⊔` enum.
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
- `⌥` phải đầy đủ (D6) → E0305. Mẫu trùng thừa → N0301 **[ĐỀ XUẤT]**.
- `⟳ cond ↓ m ⋈ inv { }`: `m` là biểu thức kiểu số nguyên không âm (well-founded) phải giảm nghiêm
  ngặt mỗi vòng (D12, kiểm ở M8). Vòng không `cond` chỉ hợp lệ khi hàm khai báo hiệu ứng `div`.
  `⋈` (bất biến): Phase 1 kiểm lúc chạy trong `dev`, không dùng để chứng minh (chờ 8.A).
- Mọi đường điều khiển của hàm có kiểu trả về khác `()` phải trả giá trị (E0307).

## 4. Hiệu ứng (D10)
- Chữ ký hàm **luôn** có `! ∅` hoặc `! {…}`. Hiệu ứng đã định nghĩa: `io, mem, st, div, ffi, lock(L)`.
- Kiểm: khai báo ⊇ suy ra (thân hàm + hiệu ứng các hàm được gọi) → E0801/E0802.

## 5. Sở hữu, mượn, region (M6 + D26/D27)
- Giá trị có kiểu không `Copy` bị **move** khi gán/truyền; dùng lại → E0401.
- `&T` nhiều lượt cùng lúc; `&!T` duy nhất, loại trừ mọi `&`/`&!` khác còn sống (NLL: sống theo lần
  dùng cuối) → E0402/E0403. Trả tham chiếu tới local → E0404. Gán khi đang bị mượn → E0406.
- `Arena` là một region có tên `@r`; tham chiếu `&@r T` không được thoát khỏi phạm vi của arena → E0405.
- **[ĐỀ XUẤT]** Elision lifetime: chỉ cho trường hợp một tham số tham chiếu duy nhất; còn lại phải ghi `@r`.

## 6. Capability (D28/D29, M4.5)
- `Cap<Fs|Net|Env|Clock|Secret|Alloc>` là kiểu affine: không `Copy`, không `Default`, không có literal.
- Hàm dùng thao tác cần capability mà `Cap<…>` không có trong chữ ký → E0310 (lỗi kiểu).
- Chỉ `entry` nhận capability gốc từ runtime; chia nhỏ qua hàm tạo trong `lib/core` (intrinsic `trusted`).

## 7. Secret (D13, M9)
- `Secret<T>`: không `Display`, không vào sink log/mạng/tệp. `declassify` đọc cần `Cap<Secret>` và
  gây **S201** (VÀNG). Literal giống key/token trong nguồn → **S101** (VÀNG). VÀNG vẫn chạy.

## 8. Hợp đồng (Phase 1)
- `⊳` (pre): người gọi phải chứng minh bằng P1; không chứng minh được → hạ runtime check (kiểm được)
  + N0901, hoặc ĐỎ E0901 nếu **chắc chắn sai**. `⊲` (post): kiểm lúc chạy ở `dev`. Kiểm tĩnh đầy đủ = 8.A.
- Lượng từ `∀ ∃` chỉ hợp lệ có biên (`∀ i ∈ 0..n.`). Lượng từ không biên mà chưa giải → ĐỎ (D25).

## 9. Unsafe (D21)
- `⚠ ⊢p … { }` có cú pháp + sinh Obligation; Phase 1 gần như luôn E1001. Chỉ `lib/core` có `intrinsic trusted` (D22).

## 10. Module
- `▦` module, `⇲` use, `◈` pub. Mặc định private. Cấm vòng phụ thuộc (E0204). Thứ tự khai báo không quan trọng.

## 11. Đa luồng (M8)
- `spawn(f, arg)` — không closure ở Phase 1. Đối số chuyển sang luồng phải `Send`-like (tự suy).
- `Mutex<T, L>` có cấp khoá `L`; thứ tự khoá phải tăng nghiêm ngặt (E0502). Atomics ghi rõ ordering.
