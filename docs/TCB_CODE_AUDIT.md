# G1.5 — đọc mã đối kháng TCB (Windows, 2026-11-12)

## Phạm vi và người đọc

Đọc đối kháng các đường mã H1–H10 trong `docs/TCB_REVIEW_GUIDE.md`, IR Guard (`crates/gt-guard`), phần hạ LLVM liên quan và runtime `runtime/gt-rt`. Mục tiêu là tìm đường chấp nhận sai hoặc hành vi runtime không đúng hợp đồng; đây là đọc mã và thí nghiệm có mục tiêu, không phải chứng minh hình thức hay chứng nhận an toàn.

- Codebase trước đó chủ yếu do Claude Sonnet/Opus các đời viết (Git không lưu model theo từng commit); các sửa FB-C28/C29/C30 trong lượt này do Codex GPT-6 viết. Người đọc chính: GPT-6/Codex; reader tách lượt GPT-6 chỉ đọc, cùng họ model. Vì vậy **đã có lượt đọc mã, nhưng chưa đáp ứng yêu cầu G1.5 về reader khác họ model**.
- Reader đã xem mã H1–H10, Guard và runtime; không chạy test/build, không sửa tệp. Phát hiện được kiểm lại tại chỗ bằng test trước/sau. Phần thảo luận “không tìm thấy” chỉ có nghĩa trong phạm vi mã đã đọc.
- **Đếm phát hiện:** H1–H10 đều có hàng trong bảng (10/10 đã đọc); ba giả thuyết được chuyển thành hồi quy đỏ-trước (FB-C28/C29/C30) đều xác nhận, 0 bác bỏ trong nhóm ba này. Tài liệu chưa có sổ đếm toàn bộ nghi vấn/bác bỏ, nên tổng chính thức **CHƯA KIỂM**. H10 chưa có thí nghiệm riêng; không xem các dòng không có phát hiện mới là đã chứng minh an toàn.
- Nền kiểm tra: Windows x86_64, LLVM 20.1.8, Rust 1.98.1. Cổng G1.6 là điều kiện riêng để cập nhật READINESS cuối cùng.

## Kết quả H1–H10

| Nhóm | Đường mã đối kháng và bằng chứng | Kết luận / giới hạn |
|---|---|---|
| H1 Arena | Đọc cấp phát chunk, căn lề, handle và reset; `cargo test -p gt-driver --test arena_dynamic` 6/6 gồm chuỗi hợp lệ và lỗi chèn chồng lấn/lệch căn lề/ngoài chunk. | Không thấy đường ghi tràn mới. Đây không chứng minh mọi tổ hợp kích thước/căn lề; H2 chỉ kiểm Arena ở dev hoặc khi ép bật. |
| H2 `Arena::alloc` | Kiểm `GRITRT_ARENA_CHECK` và đường phát hiện overlap; test trên chấp nhận chuỗi hợp lệ, bắt lỗi chèn. | Release mặc định không bật kiểm động; bằng chứng này không thay thế chứng minh không chồng lấn. |
| H3 `Arena::reset` | Đọc hợp đồng `&!self`/M6; các test vùng và borrowck giữ việc reset không qua khoản mượn sống. | An toàn phụ thuộc M6 và biểu diễn region của các kiểu hiện hành; kiểu chứa mutability mới phải mở lại audit. |
| H4 Vec | Kiểm phép nhân kích thước trước cấp phát và cập nhật `len/cap`; `cargo test -p gt-driver --test vec_overflow` 2/2, gồm trường hợp tràn thật và biên sát tràn. | `len+n`/`2*cap` có nhánh phòng thủ nhưng không ép riêng được với kích thước phần tử hiện hỗ trợ; xem `KNOWN_LIMITATIONS.md`. |
| H5 lát/File | Kiểm lát lấy độ dài mới sau gán lại; `slice_stale_len` 1/1 (FB-C20), `str::slice` 902 trường hợp khớp `is_char_boundary` của Rust. | Đối chiếu runtime không bao phủ mọi đường tạo lát. Interpreter không phải nguồn phán quyết cho LLVM. |
| H6 Text/UTF-8 | `text_push_utf8_agrees_with_rust_from_utf8_on_100k_strings`: 100.000 đầu vào, 33.409 hợp lệ/66.591 không hợp lệ, dev và release-o2 khớp Rust; cắt chuỗi 902 trường hợp khớp Rust. | `gritrt_utf8_valid` bọc `std::str::from_utf8`; lỗi oracle ở runtime có thể ảnh hưởng toàn đường. Đã xử lý thêm ranh giới FFI rỗng tại FB-C28. |
| H7 Atomic | Kiểm thứ tự nguyên tử tại M8/codegen và dữ liệu lệnh IR; `threads_repeat`: với dev và release-o2, mỗi mẫu `threads`/`two_locks` chạy 300 lượt, 0 lỗi quan sát được, không treo. | Không có ThreadSanitizer/loom và không có oracle đua dữ liệu/deadlock; 300 lượt không chứng minh vắng race. |
| H8 Mutex/spawn | Kiểm khóa thứ tự, `Send`/`Sync`, hộp tham số worker và thực thi thread thật; test `stack_depth` 3/3 sau FB-C29, gồm mảng cục bộ worker 3 MB. | Mỗi thread reserve 256 MiB; quá nhiều thread hoặc stack cần hơn mức đó vẫn có thể thất bại. Fallback khi tạo thread `main` 256 MiB thất bại chưa được ép thử. |
| H9 Cell | Kiểm `Cell::get: T: Copy`, cấm `Copy` cho các kiểu mang Arena/interior mutability; `interior_mut` 3/3 và `traits_generics fb_c17` 2/2. | M6 vẫn là hàng rào chính cho vùng chứa mutability bên trong. |
| H10 đối số môi trường | Đọc đường chuyển đối số thành UTF-8 lossy và lưu trữ tới hết tiến trình; không tạo `&str` từ byte hệ điều hành chưa kiểm. | Không có thí nghiệm mới chỉ dành cho H10 trong G1.5; kết luận phần này chủ yếu là đọc mã và kiểm chứng dogfood sẵn có. |

## IR Guard và runtime

- Guard kiểm verifier, attestation, instance, lớp nghĩa vụ, đích gọi, `invoke`/`callbr`, abort/unreachable, cờ poison/fast-math và các luật Guard-post. Bảng runtime hiện có 27 symbol.
- **FB-C30:** trước sửa, Guard chấp nhận lời gọi hợp lệ theo verifier tới `llvm.abs.i32(1, true)` và trả danh sách vi phạm `[]`. Đối số `is_int_min_poison=true` có thể làm kết quả poison. Test `rejects_non_allowlisted_llvm_intrinsic_with_poison_flag` đỏ trước sửa, xanh sau sửa. Guard nay chỉ nhận các intrinsic codegen đang dùng theo tên/họ độ rộng đóng; `llvm.assume` và FMA vẫn bị luật chuyên biệt từ chối. Cả bộ `gt-guard/tests/guard.rs` sau sửa: 35/35.
- FB-C28: `gritrt_fs_read/open/write` không còn gọi `slice::from_raw_parts(null, 0)`; helper `ffi_bytes` trả `&[]` cho length 0, và từ chối length không biểu diễn được thành Rust slice. Unit test runtime đỏ trước sửa, xanh sau sửa.
- FB-C29: main và worker dùng `GRIT_THREAD_STACK_BYTES = 256 MiB`; hồi quy worker 3 MB đỏ trước sửa (exit 101), xanh sau sửa.

## Phần còn mở, không được diễn giải thành “đã an toàn”

1. Guard **không đối chiếu số `Assert` biên trong MIR với số `gritrt_abort` trong IR**. Kiểm dev so LLVM với interpreter là bằng chứng ngoài Guard, không phải kiểm chứng bất biến này.
2. Attestation là chuỗi metadata; nó không ràng buộc đầy đủ hồ sơ build, thư viện runtime, phiên bản LLVM, object/linker hay binary cuối.
3. Guard-post đếm abort theo `(hàm, loại)` chứ không theo từng vị trí; phép biến đổi có thể đổi vị trí nhưng giữ số đếm. Các tối ưu muộn phía target sau Guard-post chưa được ràng buộc bằng chứng tương đương.
4. Không có ThreadSanitizer/loom. Stress test luồng hữu hạn không chứng minh không có race/deadlock.
5. `gritrt_start` vẫn có fallback chạy entry trên stack hiện tại nếu không tạo được thread main 256 MiB; chưa có fault injection cho nhánh thiếu tài nguyên đó.
6. `gt-rt` có các khối `unsafe`; G1.5 đọc các đường đã nêu nhưng **chưa** có test tự động đếm `unsafe` khớp danh sách TCB.

## Lệnh và kết quả G1.5 liên quan

- `tools/preflight.sh D:/Grit` qua Git Bash chỉ định: `PREFLIGHT_OK` trước từng lô được chạy trong lượt này.
- Test hồi quy mới: trước sửa `0 passed; 1 failed`, panic tại assertion với `[]`; sau sửa `1 passed`.
- `cargo test -p gt-guard --test guard -- --test-threads=2`: 35 passed, 0 failed.
- Các test H1–H10, runtime và driver bổ sung được ghi theo lệnh/kết quả trong mục G1.5 của `docs/RUN_REPORTS.md` và `docs/HANDOFF.md`.
- **G1.6 full gate:** chạy liền mạch lần 2 trên HEAD `c5aa45b`, `GATE_RESULT=GREEN`, 1.017/0/202; runtime 7/7; fail-open 470/0 mới. READINESS §7 không được tự suy rộng từ kết quả gate; các giới hạn trên vẫn mở. Chi tiết cổng ở `docs/RUN_REPORTS.md`; tình trạng bundle/push ở HANDOFF/RUNLIST.
