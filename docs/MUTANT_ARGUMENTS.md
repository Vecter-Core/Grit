# MUTANT_ARGUMENTS — bốn đột biến cuối của `gt-borrowck/src/regions.rs` (điều kiện cứng #9, GRIT_PLAN §7)

Viết cho Giang đọc và quyết định (2026-10-14). **Chưa được duyệt — tôi KHÔNG tự coi là đã duyệt.** Các đối chiếu mã dưới đây là ở commit hiện tại; số dòng có thể lệch khi `regions.rs` đổi. Nguồn số liệu: `docs/DECISIONS.md` ADR-027 §0, `docs/KNOWN_LIMITATIONS.md` (mục gt-borrowck và "Điều kiện cứng mutability bên trong"), `crates/gt-borrowck/tests/interior_mut.rs`.

**Nhắc lại bối cảnh.** `regions.rs` tính các ràng buộc "vùng A sống ít nhất bằng vùng B" (outlives) cho mọi phép gán/lời gọi. Một ràng buộc có thể một chiều (hiệp biến: `src ⊒ dst`) hoặc hai chiều (bất biến: thêm cả `dst ⊒ src`). Vị trí bất biến cần thiết khi có thể GHI qua nó (ví dụ `&!T`, hoặc ô `Cell`). Đột biến sống sót = đổi một dòng mà mọi test vẫn qua. "Chặt hơn" = bản đột biến từ chối nhiều chương trình hơn hoặc bằng bản gốc; "lỏng hơn" = có thể chấp nhận chương trình mà bản gốc từ chối (nguy hiểm: âm tính giả).

**Lập luận đơn điệu (dùng chung cho 361/591/554, nêu một lần):** borrowck tính vùng NHỎ NHẤT thoả mọi ràng buộc, rồi coi một khoản vay là "còn sống" ở mọi điểm mà vùng của nó chứa. Thêm ràng buộc chỉ làm vùng lớn lên ⇒ khoản vay sống thêm ⇒ xung đột và lỗi "không sống đủ lâu" chỉ tăng. Mọi phán quyết ĐỎ của borrowck đều là điều kiện dạng "tồn tại xung đột/vi phạm", không có phán quyết nào dạng "ĐỎ nếu ràng buộc X KHÔNG có". Vì vậy tập chương trình bị từ chối đơn điệu tăng theo tập ràng buộc. **Đây là lập luận đọc mã, không phải chứng minh hình thức** (borrowck chưa được chứng minh hình thức; xem KNOWN_LIMITATIONS). Điểm tựa còn có thể sai: nếu tồn tại một luật ĐỎ nào đó ở M6 phụ thuộc "vùng KHÔNG chứa điểm P" (phủ định), thì đơn điệu hỏng; tôi chưa tìm thấy luật nào như vậy trong `regions.rs`/`conflicts` nhưng KHÔNG đã quét hết.

---
## 238 — `ty_invariant` coi `&T`/mảng/lát là hiệp biến (regions.rs, nhánh `Ty::Ref(false, i) | Array | Slice`, dòng 265 hiện tại)
- **Trạng thái: ĐÃ GIẾT.**
- **(i) Đổi gì:** hàm `ty_invariant` quyết định một kiểu có chứa vị trí bất biến không. Bản gốc: `&T`, mảng, lát bất biến NẾU phần tử bên trong bất biến (đệ quy). Bản đột biến: coi `&T`/mảng/lát là hiệp biến (không đệ quy xuống phần tử), nên một `&Cell<&'a i32>` bị coi là hiệp biến ở vị trí `'a`.
- **(ii) Chặt hay lỏng:** **LỎNG hơn** — bỏ ràng buộc ngược `dst ⊒ src` ở các vị trí bất biến. Có thể cho phép gán vùng ngắn vào một ô được chia sẻ rồi đọc sau khi vùng đã chết (dangling) ⇒ âm tính giả nguy hiểm khi có mutability bên trong.
- **(iii) Lập luận "ràng buộc dư thừa":** KHÔNG dùng được. Ràng buộc KHÔNG dư thừa khi có `Cell`/`Mutex`/`Atomic`; nó chỉ dư thừa khi ngôn ngữ không có mutability bên trong (khi đó đường duy nhất làm thu hẹp một vùng bất biến là GHI qua nó, mà `&T` không cho ghi, nên không khai thác được độ lỏng). Vì ngôn ngữ ĐÃ có `Cell`/`Mutex`/`Atomic`, lập luận cũ ("chưa có mutability bên trong") hết hiệu lực; phải GIẾT bằng chương trình.
- **(iv) Khi có Mutex/Atomic\*:** đã xử lý bằng cách đặt quy tắc riêng: `adt_invariant` (regions.rs, dòng 236–253) coi MỌI ADT có lang item mutability bên trong (`Lang::interior_mutable`: `Cell`, `Mutex`, `Atomic`) là bất biến ở mọi vị trí region bên trong, kể cả khi chỉ có `&` tới nó. Quy tắc này nằm ở `adt_invariant`, KHÔNG phải ở dòng bị đột biến, nên đột biến 238 (đổi đệ quy ở `ty_invariant`) trở nên BẮT ĐƯỢC.
- **(v) Kiểm bằng chương trình tấn công và kịch bản `interior_mut`:** `gt-borrowck/tests/interior_mut.rs` (tấn công: trực tiếp qua `&Cell`, ADT giữ `&Cell`, hàm generic, tuple, `Option`, mảng, `&&Cell`, ghi đè muộn; đối chứng hợp lệ) và kịch bản `interior_mut` của bộ sinh với oracle động hiểu `Cell`/`Mutex`/`Atomic` (20 000 chương trình: 0 âm tính giả; 3 700 + 1 561 + 1 523 chương trình thuộc ba biến thể; độ phủ ≥ 300 mỗi loại). **Đã tắt bản sửa:** vô hiệu quy tắc bất biến của Cell ⇒ test tấn công đỏ và oracle bắt **53 âm tính giả trong 3 000 chương trình**; vô hiệu cả nhóm (Cell+Mutex+Atomic) ⇒ 24 âm tính giả/3 000.
- **Giới hạn nói thẳng:** chỉ tắt riêng Mutex/Atomic (giữ Cell) ⇒ **0** âm tính giả. Nghĩa là với Mutex/Atomic chưa chứng minh được quy tắc bất biến là CẦN, vì hiện không có đường nhét tham chiếu sống ngắn vào chúng (đối số của `with` phải `Send`, cấm tham chiếu; `Atomic` chỉ chứa số). Quy tắc cho Mutex/Atomic hiện là PHÒNG THỦ CHIỀU SÂU, chỉ thể hiện bằng `Cell`. Điều kiện mở khoá: khi 8.D cho phép tham chiếu trong `Arg`/luồng có phạm vi, thêm tấn công riêng cho Mutex vào `interior_mut` rồi chạy lại kiểm tắt bản sửa.

## 361 — `Rvalue::Repeat`: quan hệ hai chiều thay vì một chiều (regions.rs, dòng 369–374)
- **Trạng thái: TƯƠNG ĐƯƠNG CÓ LẬP LUẬN** (chưa phân biệt được bằng chương trình; hướng CHẶT hơn). Chờ Giang chấp nhận lập luận.
- **(i) Đổi gì:** với `[x; n]`, bản gốc gọi `self.relate(el, &s, &dslots, false)` (một chiều: nguồn sống ≥ phần tử mảng). Bản đột biến đổi `false` → `true` (hai chiều + coi phần bên trong bất biến).
- **(ii) Chặt hay lỏng:** **CHẶT hơn.** `relate(..., inv = true)` chỉ THÊM ràng buộc ngược `dst ⊒ src` cho mọi vị trí region và coi phần bên trong là bất biến; không bỏ ràng buộc nào. Theo lập luận đơn điệu ở đầu tài liệu, bản này chỉ có thể từ chối thêm.
- **(iii) Nếu lỏng hơn:** không áp dụng (không lỏng hơn). Tuy vậy lập luận "ràng buộc thêm vào có thể quan sát hay không": `[x; n]` có đúng MỘT nguồn `x` nên chiều ngược chỉ thêm các điểm của nguồn vào đích — thực tế hiếm khi tạo khác biệt quan sát được; mọi phần tử mảng đều là bản sao của một giá trị.
- **(iv) Khi có Mutex/Atomic\*:** lập luận đơn điệu KHÔNG dùng giả thiết gì về tính khả biến của kiểu (nó chỉ nói về việc thêm ràng buộc outlives), nên giữ nguyên với kiểu có mutability bên trong. Với kiểu chứa ô, bất biến đã được cài chính bằng các ràng buộc hai chiều này (đột biến chỉ làm thêm cho các kiểu khác).
- **(v) Kiểm:** đo trên 6 000 chương trình có kịch bản `interior_mut` (Cell/Mutex/Atomic): bản gốc và bản đột biến đều **0 âm tính giả và 127 dương tính giả** — oracle động KHÔNG phân biệt được ⇒ hướng dựa vào đọc mã chứ không dựa vào đo. Không có chương trình tấn công viết tay (vì đột biến không thể chấp nhận thêm). "Tắt bản sửa" không áp dụng (không có bản sửa riêng cho 361).
- **Rủi ro còn lại:** chỉ dương tính giả (chặn nhầm một chương trình đúng chứa `[x; n]` với tham chiếu bên trong). Tôi chưa viết được chương trình hợp lệ bị bản đột biến từ chối nhầm.

## 591 — kết quả lời gọi: quan hệ hai chiều (regions.rs, dòng 602, hàm `call_sig`)
- **Trạng thái: TƯƠNG ĐƯƠNG CÓ LẬP LUẬN** (hướng CHẶT hơn). Chờ Giang chấp nhận.
- **(i) Đổi gì:** `self.relate(&ret_inst, &rl, dslots, false)` — gắn các vị trí region của kết quả lời gọi với đích gán — đổi `false` → `true`.
- **(ii) Chặt hay lỏng:** **CHẶT hơn** (cùng cơ chế với 361: thêm chiều ngược). Không bỏ ràng buộc nào.
- **(iii) Nếu lỏng hơn:** không áp dụng. Quan sát thêm: biến region của callee được tạo MỚI cho mỗi lần gọi và cận dưới duy nhất của nó là chính kết quả, nên chiều ngược `dst ⊒ kết-quả` không thêm ràng buộc nào có thể quan sát từ bên ngoài ngoài việc bắt đích sống ít nhất bằng vị trí kết quả.
- **(iv) Khi có Mutex/Atomic\*:** như 361: lập luận đơn điệu không phụ thuộc tính khả biến.
- **(v) Kiểm:** cùng phép đo 6 000 chương trình: **0 âm tính giả, 127 dương tính giả**, giống hệt bản gốc. Không có chương trình tấn công (không thể chấp nhận thêm). Không có "bản sửa" để tắt.

## 554 — kiểu khai báo của `&self` là `&!Self` (regions.rs, dòng 565, hàm `call_sig`/`walk`)
- **Trạng thái: TƯƠNG ĐƯƠNG CÓ LẬP LUẬN** (hướng CHẶT hơn). Chờ Giang chấp nhận.
- **(i) Đổi gì:** `gt_ast::SelfKind::Ref => Ty::Ref(false, Box::new(sd.clone()))` đổi thành `Ty::Ref(true, …)`: phương thức khai báo `&self` bị coi là `&!self` khi tính ràng buộc.
- **(ii) Chặt hay lỏng:** **CHẶT hơn.** `relate` xử lý `Ref(m, inner)` bằng `relate(inner, …, inv ‖ m)` nên phần được trỏ tới bị coi bất biến. `walk` khớp vị trí region theo CẤU TRÚC (cùng số vị trí với `Ref(false)`) nên không đổi gì khác. Không bỏ ràng buộc nào.
- **(iii) Nếu lỏng hơn:** không áp dụng.
- **(iv) Khi có Mutex/Atomic\*:** với kiểu chứa ô, phần được trỏ tới ĐÃ bất biến nhờ `adt_invariant`, nên đột biến này không đổi gì cho chúng; với kiểu khác nó chỉ thêm ràng buộc. Lập luận đơn điệu không phụ thuộc tính khả biến.
- **(v) Kiểm:** cùng phép đo 6 000 chương trình: giống hệt bản gốc (0 âm tính giả, 127 dương tính giả). Không viết được chương trình hợp lệ bị bản đột biến từ chối nhầm trong giới hạn nỗ lực hợp lý. Không có "bản sửa" để tắt.

---
## Tóm tắt trạng thái và điều cần Giang quyết
| đột biến | hướng | trạng thái | cần gì |
|---|---|---|---|
| 238 | lỏng | **đã giết** (tắt bản sửa ⇒ 53 âm tính giả/3 000) | không |
| 361 | chặt | tương đương có lập luận | Giang chấp nhận lập luận đơn điệu |
| 591 | chặt | tương đương có lập luận | như trên |
| 554 | chặt | tương đương có lập luận | như trên |
**Không có đột biến nào ở trạng thái "chưa giải quyết = false negative tiềm ẩn" theo nghĩa nguy hiểm** — NHƯNG điều này dựa vào lập luận đơn điệu ở đầu tài liệu (đọc mã, chưa chứng minh hình thức; tôi chưa quét hết mọi luật ĐỎ để chắc không có luật phủ định) và vào việc oracle không phân biệt được bản gốc với bản đột biến. Nếu Giang không chấp nhận lập luận, ba đột biến này quay lại thành "false negative tiềm ẩn" của M6 và điều kiện #9 chưa đạt.
**Điểm mở khác (từ ADR-027):** quy tắc bất biến cho `Mutex`/`Atomic` chưa chứng minh được là cần bằng kiểm tắt bản sửa (chỉ `Cell` thể hiện); phải thêm tấn công riêng khi 8.D cho phép tham chiếu trong `Arg`/luồng có phạm vi.

---
## Kiểm lại bằng bộ test HIỆN TẠI (2026-10-15; `tools/mut.py`, 1 worker, `cargo test -p gt-borrowck`)
Chạy lại đúng các dòng của bốn đột biến (số dòng hiện tại 265/372/565/602): **238 (dòng 265, `false → true`): KILLED; 361 (372), 554 (565), 591 (602): SURVIVED** — khớp bảng ở trên và ADR-027 §0. Hai đột biến LỎNG hơn "xoá câu lệnh `relate`" ở dòng 372 và 602 (bỏ hẳn ràng buộc) đều KILLED, chứng tỏ bộ test nhạy với hướng nguy hiểm ở chính các điểm này. Kết quả thô: `target/tmp/mut_regions4.txt` (không commit). **Chưa kiểm:** chưa chạy so sánh phán quyết từng chương trình giữa bản gốc và ba bản đột biến để xác nhận tập từ chối của chúng là siêu tập (đo cũ trên 6 000 chương trình chỉ cho thấy "giống hệt").
