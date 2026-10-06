# Region và Arena — bản nháp ngữ nghĩa cho M6 (M4.5, ADR-019)

Mục đích: M6 (borrow checker) **dùng lại** mô hình này, không thiết kế lại (plan M4.5/M6). Phần "Đã cài ở M4.5" có test; phần "Giao cho M6" là
hợp đồng M6 phải hiện thực, chưa có mã.

## 1. Mô hình
- Mọi kiểu tham chiếu `&T` / `&!T` mang một **region**: tên `'a` do người dùng ghi, `'static`, hoặc region ẩn (elision) mà compiler đặt tên.
- `Arena` là **chủ của một vùng nhớ**: `Arena::alloc(&self, v: T) → &T` cấp phát vào arena và trả tham chiếu mà region của nó **là region của lần mượn `&self`**
  (kết quả của luật elision: chữ ký chỉ có đúng một tham chiếu vào). Không có "region tự do" nào khác: sống của dữ liệu trong arena = sống của lần mượn arena.
- Hệ quả: D27 ("con trỏ vào Arena không thoát khỏi lifetime của Arena") KHÔNG cần cơ chế riêng; nó là quy tắc mượn thường áp lên `alloc`. Chỉ có **mã lỗi**
  khác (E0405 thay vì E0404) khi vật bị mượn là một `Arena` — nhờ `#lang(arena)` (`Lang::Arena`) M6 nhận ra nguồn gốc của khoản mượn.

## 2. Giới hạn của ngữ pháp Phase 1 (sự thật cần biết khi thiết kế M6)
- Lifetime chỉ xuất hiện: trong danh sách generics `<'a, T>` của hàm/struct/enum/trait/impl và sau `&`/`&!` (`&'a T`, `&!'a T`).
- **Không có đối số lifetime trong đường dẫn kiểu**: không viết được `Holder<'a>`; mỗi lần dùng `Holder` (struct có tham số lifetime) region của nó là ẩn.
  **Không có ràng buộc outlives** `'a: 'b`. Chữ ký như `ƒ f<'a, 'b>(x: &'a &'b T) → &'b T` dùng quan hệ ngầm "'b sống lâu hơn 'a" (well-formedness).
- Hệ quả đã biết, ghi KNOWN_LIMITATIONS và Q24: struct giữ tham chiếu dùng được nhưng không thể ràng buộc rõ region của nó ở chữ ký hàm ngoài elision.

## 3. Đã cài ở M4.5 (kiểm tĩnh trên CHỮ KÝ; mã E0315)
1. Lifetime dùng phải được khai báo (`'static` luôn hợp lệ) ở: tham số, kiểu trả về, trường struct/enum, chú thích kiểu trong thân, header/thành viên impl/trait.
2. Khai báo không trùng và không che lifetime của impl/trait chứa nó.
3. Trường struct/enum và alias: `&T` phải ghi lifetime (không elision).
4. **Elision ở kiểu trả về** (semantics §5, đề xuất cũ nay thành luật): đếm các vị trí tham chiếu ở đầu vào (mỗi `&` trong kiểu tham số; `&self`/`&!self` là một vị trí).
   Đúng một vị trí → mọi `&` không ghi lifetime ở đầu ra nhận region đó. Không vị trí nào → `'static` (chỉ có thể trả dữ liệu tĩnh như literal chuỗi; M6 kiểm thân hàm).
   Từ hai vị trí → E0315, phải ghi `'r`. Không có "luật `&self` thắng" (khác Rust) — xem Q24.
5. **Bảng `RegionSig`** (`TypesOutput.regions`, khoá = NodeId của hàm/thành viên): `declared` (lifetime khai báo ở chính hàm),
   `inputs` (với mỗi tham số, kể cả `self` đứng đầu: các vị trí region theo thứ tự duyệt trước, `Anon(n)` đánh số tăng dần qua mọi vị trí ẩn của đầu vào),
   `output` (các vị trí region của kiểu trả về, vị trí ẩn đã gán theo luật 4). Đây là hợp đồng dữ liệu M6 đọc để khởi tạo ràng buộc cho lời gọi.
6. Region KHÔNG nằm trong `Ty`: `&'a i32` và `&'b i32` là cùng một kiểu với bộ hợp nhất của M4 (region không ảnh hưởng suy kiểu). M6 suy region trên MIR.

## 4. Giao cho M6 (hợp đồng, chưa có mã)
- **Biến region:** mỗi vị trí tham chiếu trong MIR (local, rvalue `&`/`&!`, kết quả lời gọi) nhận một biến region; chữ ký hàm được gọi được khởi tạo từ `RegionSig`
  (`Named` → biến chung trong lời gọi, `Anon(n)` → biến mới theo vị trí đầu vào, `Static` → hằng).
- **Ràng buộc:** gán `&'a T → &'b T` đòi `'a: 'b` (hiệp biến theo region; `&!T` bất biến theo `T`, hiệp biến theo region); reborrow `&*p` có region ngắn hơn hoặc bằng region của `p`;
  gọi hàm: đối số ⊒ tham số, kết quả theo `output`.
- **Sống (NLL):** region = tập điểm chương trình nơi một tham chiếu mang region đó còn có thể được dùng sau này (liveness backward do M5 cung cấp).
- **Khoản mượn (loan):** mỗi `&`/`&!` tạo `Loan { place, kind, region }`; xung đột aliasing → E0402/E0403/E0406.
- **Thoát khỏi vùng:** kết quả hàm phải có mọi region ⊒ thân hàm trừ khi nó là region của một tham số (hoặc `'static`). Loan trên `place` cục bộ kết thúc ở `StorageDead(place)`:
  còn sống sau đó và `place` có kiểu `Arena` → **E0405**; kiểu khác → **E0404**.
- **Arena cục bộ:** `a ≔ Arena::new(); p ≔ a.alloc(x);` hợp lệ khi `p` không sống quá `a` (drop theo thứ tự ngược khai báo). Trả `p` ra khỏi hàm → E0405. `a` là tham số `&'r Arena` thì trả `&'r T` hợp lệ.
- **Di chuyển Arena khi còn khoản mượn** → E0406 (cùng luật gán khi đang bị mượn).

## 5. Điểm mở cho M6 (không quyết ở M4.5)
- Có "static promotion" cho `&1`/`&"x"` (hằng) không? M4.5 chấp nhận `ƒ f() → &i32 { &1 }` ở mức chữ ký; M6 quyết thân hàm.
- **Bắt buộc (Giang, Q24d):** chữ ký không có tham chiếu vào có đầu ra `Static`; M6 PHẢI kiểm thân hàm thật sự chỉ trả dữ liệu `'static` (literal, `□`/`■`); trả tham chiếu tới biến cục bộ hay tạm vẫn bị chặn (E0404).
