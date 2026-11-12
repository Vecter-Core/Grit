# G2.2 / R11b — yêu cầu đọc mã đối kháng độc lập

**Trạng thái:** chờ Claude (model khác họ GPT-6/Codex); không tính là hoàn tất cho tới khi có phản hồi được lưu và xử lý.

Bạn là reviewer độc lập cho dự án ngôn ngữ Grit. Chỉ đọc và thí nghiệm; không sửa mã. Mục tiêu là tìm đường chấp nhận sai, memory unsafety, UB, sai hành vi runtime hoặc tuyên bố an toàn vượt quá bằng chứng. Đọc `docs/TCB_REVIEW_GUIDE.md`, toàn bộ `docs/TCB_CODE_AUDIT.md`, `docs/TCB_INTRINSICS.md`, các tệp Rust trong IR Guard (`crates/gt-guard`), codegen LLVM có liên quan (`crates/gt-codegen-llvm`) và runtime (`runtime/gt-rt`). Đọc cả thay đổi Codex mới nhất được chỉ rõ bên dưới.

## Phạm vi bắt buộc

1. H1–H10 trong `docs/TCB_REVIEW_GUIDE.md`; nêu đường mã cụ thể, điều kiện kích hoạt, hậu quả và bằng chứng phản chứng.
2. IR Guard: verifier/attestation, allowlist intrinsic, obligation, abort/unreachable, call/invoke/callbr, kiểm tra trước/sau tối ưu. Xem đặc biệt các giới hạn GUARD-01/02, ATTEST-01 trong sổ audit.
3. Runtime `gt-rt`, đặc biệt H10 argv lossy UTF-8, `encode_args`, `arg_count/arg_ptr/arg_len`, H1–H9 và các FB-C28/C29/C30.
4. Mọi dòng TCB hiện do Codex phân loại trong `docs/AUTHORSHIP.md`; báo cáo này phân loại theo ancestry Git, không khẳng định tác giả người/model. Tại HEAD ghi trong báo cáo, tổng Codex là 503/64.768 dòng TCB (0,78%); ưu tiên `gt-rt` (164 dòng), `gt-guard` (68), `gt-driver` (231), `gt-types` (32), `gt-borrowck` (8).

## Cách làm và định dạng đầu ra

- Chỉ đọc mã và chạy thí nghiệm có mục tiêu. Không dùng kết quả thông dịch MIR làm bằng chứng cho hành vi LLVM/runtime nếu chưa so thực thi native.
- Với mỗi nghi vấn: mã/tệp/dòng hoặc hàm; giả thuyết; chương trình/lệnh tái hiện; kết quả quan sát; kết luận `xác nhận`, `bác bỏ trong phạm vi`, hoặc `chưa kiểm`; mức độ và giới hạn.
- Nếu phát hiện lỗi, cung cấp test tối thiểu đỏ trước sửa và đề xuất hướng sửa; không tự sửa.
- Nếu không tái hiện, không kết luận an toàn tổng quát. Phân biệt không tìm thấy với đã chứng minh.
- Đặc biệt đánh giá liệu `to_string_lossy` là hợp đồng phù hợp của argv trên Windows và liệu test hiện có đủ cho đường gọi thực tế; chưa có test end-to-end shell với surrogate lẻ.
- Đặc biệt phân tích các giới hạn: Arena/chồng lấn mọi kích thước; tràn Vec chưa ép được; Guard không đối chiếu số Assert theo vị trí; tối ưu target muộn; attestation không ràng buộc binary cuối; chưa có kiểm kê tự động `unsafe`; chưa có TSan/loom.

## Trả lời theo cấu trúc

1. Tóm tắt: số nghi vấn xác nhận/bác bỏ/chưa kiểm; lỗi nghiêm trọng nào cần dừng phát hành.
2. Bảng H1–H10, Guard và runtime với bằng chứng cụ thể.
3. Kết quả từng mục trong sổ 13 nghi vấn của `docs/TCB_CODE_AUDIT.md`; thêm hàng nếu phát hiện mới.
4. Codex lines: rà đặc biệt các tệp/dòng nêu ở trên; không gộp vào review các dòng Claude.
5. Các giới hạn còn lại, mức độ, bước kiểm chứng kế tiếp.

Không dùng nhãn “an toàn”, “đã chứng minh” hoặc “đạt” nếu bằng chứng chỉ là test hữu hạn. Không thay đổi trạng thái `TCB_INTRINSICS.md`; kiến trúc sư sẽ quyết định sau khi đọc báo cáo.
