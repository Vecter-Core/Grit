# G2.2 / R11b — review tấn công phần TCB do Codex viết

**Trạng thái: CHỜ PHIÊN CLAUDE (Giang mở).** Không sửa mã và không tự cập nhật trạng thái duyệt TCB. Kết quả reviewer lưu ở `docs/TCB_CODE_AUDIT_CLAUDE.md`.

## Vai trò

Bạn là reviewer tấn công độc lập, chỉ được đọc diff TCB do Codex viết theo danh sách dưới đây và phần hợp đồng liên quan trong `docs/TCB_INTRINSICS.md` / `docs/TCB_REVIEW_GUIDE.md`. Không cần đọc toàn bộ mã Claude-authored hoặc lặp lại G1.5. Báo cáo authorship là phân loại theo ancestry Git; không suy ra model/tác giả cá nhân ngoài quy tắc đó.

## Phạm vi diff bắt buộc

- **FB-C28** — commit `1c7deef`, `runtime/gt-rt/src/lib.rs`: `ffi_bytes` và các caller `gritrt_fs_open/read/write`; xem null/zero, độ dài không biểu diễn được, kiểm con trỏ và hậu quả FFI.
- **FB-C29** — commit `9da739e` (worker stack) và `7080925` (main fail-closed), `runtime/gt-rt/src/lib.rs`: helper/tạo thread và `gritrt_start`; xem cấp stack, fallback, thông báo/mã thoát và quyền tiếp tục.
- **FB-C30** — commit `8eb17ba`, `crates/gt-guard/src/lib.rs`: `allowed_llvm_intrinsic`, kiểm lời gọi trong `check_module`; xem tên/họ intrinsic, overload, cờ poison và mọi đường bỏ qua Guard.
- **FB-C6** — commits `a5c4295`, `f000546`, tập trung `crates/gt-oblig/src/driver.rs::analyze_program_with` và `escaped_sites`; xem hàm có tiền điều kiện `⊳` bị dùng như giá trị và mọi cách có thể thoát khỏi phát hiện.
- **Giải xung đột merge** — merge commit `c3ba7de`, diff kết hợp trong các hàm TCB sau: `gt-borrowck/src/regions.rs` (WF/outlives ở `call`); `gt-types/src/check/call.rs` (callee-value/capability tracking); `gt-types/src/check/mod.rs` (`Checker` state/finalization); `gt-types/src/validate.rs` (Copy bounds); `gt-types/src/db.rs` (trait/Copy database); `gt-oblig/src/driver.rs` (precondition/escaped fn-value). Không có tệp TCB `gt-effects` trong diff kết hợp của merge này.

HEAD tham chiếu lúc tạo prompt: `9e9b54c` (các commit sau chỉ cập nhật docs/tests, trừ khi HANDOFF chỉ rõ khác). Xem `docs/AUTHORSHIP.md` để biết phép đếm tại HEAD cập nhật: Codex 503/64.768 dòng TCB, 0,78%; ưu tiên đúng diff/commit nêu ở trên.

## Câu hỏi bắt buộc cho mỗi nghi vấn

1. Hợp đồng ghi gì và mã thực hiện gì? Nêu giả định ngầm, kiểm tra bị bỏ qua, đường dữ liệu/tác vụ tới điểm nguy hiểm.
2. Có đường tới hỏng bộ nhớ, UB, soundness fail-open, hoặc hành vi runtime sai không? Nêu điều kiện cụ thể và hậu quả.
3. Có thể viết test/chương trình tấn công tối thiểu nào? Nếu chạy được, ghi command và kết quả; nếu chưa chạy, ghi rõ `chưa kiểm` và lý do.

Mỗi mục phải có commit/file/hàm; kết luận `xác nhận`, `bác bỏ trong phạm vi`, hoặc `chưa kiểm`; bằng chứng/thí nghiệm; mức độ; giới hạn; test tái hiện/đề xuất. Với nghi vấn đã được test hiện có phủ, kiểm xem mutation/đường tắt liên quan có lọt hay không. Không biến một test hữu hạn thành tuyên bố tổng quát.

## Giới hạn thao tác và định dạng giao kết

- Phạm vi mã: CHỈ diff do Codex viết (danh sách trên). Ngoài ra bắt buộc xử lý **sổ 13 nghi vấn** trong `docs/TCB_CODE_AUDIT.md`: mỗi nghi vấn một thí nghiệm hoặc ghi `chưa kiểm` kèm lý do. Không đọc/đánh giá dòng Claude ngoài ngữ cảnh tối thiểu để hiểu hợp đồng. Không chạy full gate.
- Không đổi trạng thái `docs/TCB_INTRINSICS.md` / READINESS bằng tay ngoài dòng nêu ở mục sau. Cấm kết luận "an toàn" khi chỉ có test hữu hạn hoặc không tái hiện được; phân biệt "không tìm thấy" với "đã chứng minh".
- Ghi báo cáo ở `docs/TCB_CODE_AUDIT_CLAUDE.md` với: **tên model thật của reviewer và hash HEAD đọc**; tóm tắt (số nghi vấn xác nhận / bác bỏ trong phạm vi / chưa kiểm); bảng FB-C28/C29/C30/C6; bảng các vùng merge-conflict; từng nghi vấn kèm bằng chứng; lỗi cần dừng phát hành; giới hạn và bước kiểm kế tiếp.
- Lỗi thật: theo quy trình dự án — test ĐỎ trước (commit riêng), sửa (commit riêng), ghi FB-C trong `docs/FIELD_BUGS.md`. Lỗi có thể hỏng bộ nhớ/UB: sửa ngay theo cách đó rồi báo.
- Dòng TCB chỉ được ghi "ĐÃ QUA ĐỌC MÃ ĐỐI KHÁNG BẰNG MODEL KHÁC (<tên model>, <hash>)" cho dòng KHÔNG còn thay đổi chưa được đọc độc lập.
