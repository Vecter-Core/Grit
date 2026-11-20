# NEXT_PRODUCT_BATCH — thay đổi MÃ SẢN PHẨM đang chờ (lập GÓI 10, 2026-11-20)

Mã sản phẩm (`crates/`, `runtime/`, `lib/`) ĐÓNG BĂNG sau cổng xanh rc6 (`16c37f0`). Danh sách này gom mọi thay đổi cần làm; áp dụng MỘT LẦN khi có kích hoạt, rồi MỘT nhãn rc7 và MỘT cổng.
**Kích hoạt:** (a) phát hiện hỏng bộ nhớ hoặc âm tính giả (false negative) của kiểm tra an toàn: sửa NGAY (test đỏ trước, `FIELD_BUGS.md`), nâng rc7, cổng lại; hoặc (b) quyết định của kiến trúc sư/Giang.

| # | Thay đổi | Lý do | Nguồn | Trạng thái |
|---|---|---|---|---|
| 1 | Chú thích `{detach}` trong chữ ký prelude `spawn` (`lib/core/prelude.gt:15`) không khớp hành vi (không được thi hành) | tài liệu đã đồng bộ theo hành vi (GÓI 10); mã vẫn mang chú thích sai | ADR-C12 | CHỜ — sửa prelude đổi hợp đồng TCB (cần duyệt) |
| 2 | Đường siết spawn: cho phép khai `{detach}` ở mã người dùng (E1002 hiện chặn) rồi bắt buộc hàm gọi `spawn` khai `detach` | `! ∅` hiện không bảo đảm "không có luồng" | ADR-C12 | CHỜ — cần ADR riêng, làm TRƯỚC kênh có kiểu |
| 3 | `declassify` không có `Cap<Secret>` ⇒ lỗi kiểu ĐỎ (hiện chỉ cảnh báo S201) | D28: dùng năng lực cần `Cap` | ADR-C13 | HOÃN — thay đổi phá vỡ; bán kính chưa đo bằng chạy thật (27 tệp theo `grep`) |
| 4 | Xoá `Secret<T>` có bảo đảm (`secret_wipe`) | hiện không có chỗ nào xoá | `PHASE2_ADR_DRAFTS.md` ADR-P2-03 | ĐỀ XUẤT |
| 5 | Phát hiện từ S1 (cổng Linux WSL) | — | GÓI 10 S1 | KHÔNG có lỗi mã: cổng Linux GREEN. Chỉ lệch môi trường (toolchain, `python`, `libffi-dev`) — sửa `tools/setup_cloud.sh` là việc công cụ, không phải mã sản phẩm |
| 6 | Phát hiện từ S3: FB-C34 tắt bản sửa ⇒ ĐỎ (đúng). Đột biến S1d: 27/110 sống sót; cần THÊM TEST (tests/ của gt-term) để giết ~22 ca hướng chấp-nhận-thêm | GÓI 10 S3.a/b | `docs/MUT_GT_TERM_GOI10.md` | CHỜ (thêm test không đổi mã sản phẩm; làm được ngay khi có phiên) |
| 7 | Kết quả R11c (Opus đọc mã) | — | sau này | CHƯA CÓ |

Số mục đang chờ thật (1–4): **4** (mục 5–7 là chỗ giữ chỗ).

## Bổ sung GÓI 11 (2026-11-20)
| # | Thay đổi | Lý do | Nguồn | Trạng thái |
|---|---|---|---|---|
| 8 | Vòng khởi tạo static qua lời gọi hàm (`■ X ≔ f()`, `f` đọc `X`) hiện `gritc check` XANH nhưng backend LLVM từ chối và thông dịch báo lỗi nội bộ: thêm kiểm đồ thị khởi tạo static ở M4/M8 để ĐỎ ngay ở `check` | nhất quán D12/fail-closed ở giai đoạn kiểm, không chờ backend | `docs/CONTROL_TRANSFER_INVENTORY.md` hàng 13 | CHỜ (không phải âm tính giả chạy được: backend từ chối) |
| 9 | 25 đột biến gt-term còn sống: thêm test giết (đặc biệt nhánh `None => true` ở `dyn_fn_arg`, `takes_fn_param`, `has_back_edge` 0 lần trong 974 đầu vào) | `docs/MUT_GT_TERM_GOI10.md` | GÓI 11 S1 | CHỜ (chỉ test) |
| (đã làm) | FB-C35 sửa trong GÓI 11 (commit `7c6334c`, rc7 `41ae5e7`) — KHÔNG còn chờ | — | FIELD_BUGS | XONG |

## Bổ sung GÓI 12 (2026-11-20)
| # | Thay đổi | Lý do | Nguồn | Trạng thái |
|---|---|---|---|---|
| 10 | **FB-C36**: vòng tham chiếu QUA HẰNG `□` (`□ C ≔ C`; `□ A ≔ B; □ B ≔ A`; `□ C ≔ f()` với `f` đọc `C`) bị gt-term bỏ sót (chỉ dựng đồ thị từ thân `BodyKind::Fn`) | LLVM hạ `□ C` thành LỜI GỌI hàm ở mỗi chỗ dùng ⇒ đệ quy vô hạn, `check` XANH, exe tràn ngăn xếp | `CONTROL_TRANSFER_INVENTORY` hàng 13 (mới đo ở GÓI 12 P3.a) | XỬ LÝ trong GÓI 12 theo luật đóng băng (test đỏ trước, rc8) — xem FIELD_BUGS |
| 11 | Đổi hai `_ => {}` ở phần dựng cạnh của gt-term (`lib.rs` `instance_graph` ~371 và `check_termination` ~779, nhánh `Rvalue` ngoài Use/Cast/Repeat/Aggregate) sang `match` vét cạn | khi thêm biến thể `Rvalue`/toán hạng mang `Const::FnItem`/`Const::Item` mới, trình biên dịch buộc xem lại (FB-C35/36 đều do "bỏ sót nguồn cạnh") | GÓI 12 P3.d | ĐỀ XUẤT (không đổi hành vi) |
| 12 | Kiểm đồ thị khởi tạo static ở M4/M8 để `check` ĐỎ (mục 8) kèm mã lỗi mới | ghim hành vi hiện tại ở GÓI 12; mã lỗi đóng băng nên cần ADR | mục 8 | CHỜ ADR; gom vào lô rc9 hoặc sau |

### P3.d — các `_ =>`/mặc định trong phần dựng cạnh và kiểm dừng của gt-term (đọc `lib.rs`, GÓI 12)
| Dòng | Ngữ cảnh | An toàn vì | Đề xuất |
|---|---|---|---|
| 47 | `index_fns`: item không phải hàm/impl/mod | các item đó không có thân hàm (hàm mặc định trong `⊏` không viết được: hàng 14) | giữ; liệt kê biến thể để buộc xem lại |
| 71, 100, 118, 138 | `Tx::term` (biến thể `↓`): dạng không nhận ra ⇒ `None` | `None` ⇒ E0702/E0701 (đỏ), tức hướng fail-closed | giữ |
| 174 | `inferred_candidates`: phép so sánh khác `< ≤ > ≥` ⇒ bỏ ứng viên | thiếu ứng viên ⇒ vòng phải có `↓` (đỏ nếu thiếu) | giữ |
| ~371, ~779 | `Rvalue` khác Use/Cast/Repeat/Aggregate ⇒ không phát hiện hàm dùng như giá trị | kiểu hàm không thể là toán hạng của BinaryOp/UnaryOp/Ref/Len/Discriminant (type-check M4; hàng 6) — NHƯNG lý do này chỉ đúng với `Const::FnItem`, KHÔNG đúng với `Const::Item` (FB-C36 nằm ở đây) | **đổi sang match vét cạn** (mục 11) |
| callee match | `Callee::{Fn,Value,TraitMethod}` | đã vét cạn, không có `_` | giữ |

## Bổ sung GÓI 13 (2026-11-21)
| # | Thay đổi | Lý do | Nguồn | Trạng thái |
|---|---|---|---|---|
| 13 | Thông dịch: hỗ trợ khởi tạo `□`/`■` (đọc giá trị), HOẶC `check`/`run` báo "chưa hỗ trợ" rõ ràng (mã thoát 3) thay vì ICE (mã 2) | ĐO hai nền: MỌI chương trình đọc `□`/`■` (kể cả literal) ⇒ ICE mã 2 ở thông dịch | `docs/SEED_LIMITATIONS.md` GÓI 13 | CHỜ — gom lô rc9 |

## D3 (GÓI 13) — thứ tự đề xuất của MỘT lô rc9 (CHƯA áp dụng; mọi mục dưới đây đổi mã sản phẩm ⇒ cần kích hoạt: lỗi âm tính giả HOẶC quyết định kiến trúc sư)
Đã XONG, không còn trong lô: FB-C35 (rc7), FB-C36 (rc8).
| Thứ tự | Mục | Đổi gì | Rủi ro (đề xuất) | Ghi chú |
|---|---|---|---|---|
| 1 | #11 match vét cạn ở dựng cạnh gt-term (2 chỗ `_ => {}`) | chỉ dạng mã, không đổi hành vi | **thấp** | chạy gate gt-term + đột biến lại; lợi: biến thể MIR mới buộc phải xem |
| 2 | #13 thông dịch đọc `□`/`■` | hoặc hỗ trợ khởi tạo (`gt-interp`, trung bình), hoặc báo "chưa hỗ trợ" mã 3 thay ICE (thấp) | thấp (báo rõ) / trung bình (hỗ trợ) | đề xuất làm bản "báo rõ" trước; ĐO: ICE ở cả hai nền, kể cả literal |
| 3 | #12 kiểm vòng khởi tạo static ở M4/M8 ⇒ `check` ĐỎ | thêm mã lỗi mới (đóng băng ⇒ cần ADR) + test ≥ 3 fail/3 pass | trung bình | `static_const_init_pin.rs` phải đổi theo (chủ ý) |
| 4 | gt-term nhánh phòng thủ 567/584/687/699 | đổi `None => true` thành ICE tường minh (E9901) thay vì lặng lẽ | thấp–trung bình | biến false negative tiềm ẩn thành lỗi nếu bất biến bị phá; có thể làm cùng #11 |
| 5 | #1, #2 (spawn/`detach`), #3, #4 (`declassify`, `secret_wipe`) | đổi hợp đồng TCB/ngữ nghĩa | **cao**, cần duyệt kiến trúc sư/Giang | KHÔNG gom vào rc9 trừ khi có quyết định |
Ước lượng: mục 1+4 cùng gt-term (một cổng gt-term + toàn bộ); mục 2 gt-driver; mục 3 gt-types/gt-term/docs spec. Một nhãn rc9 + MỘT cổng đầy đủ hai nền (≈ 3 giờ).
