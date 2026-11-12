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
| H8 Mutex/spawn | Kiểm khóa thứ tự, `Send`/`Sync`, hộp tham số worker và thực thi thread thật; test `stack_depth` 3/3 sau FB-C29, gồm mảng cục bộ worker 3 MB. G2.0.d ép lỗi tạo thread main trong subprocess. | Mỗi thread reserve 256 MiB; quá nhiều thread hoặc stack cần hơn mức đó vẫn có thể thất bại. Tạo thread main thất bại nay thoát riêng mã 71, test xác nhận thông báo và mã thoát. |
| H9 Cell | Kiểm `Cell::get: T: Copy`, cấm `Copy` cho các kiểu mang Arena/interior mutability; `interior_mut` 3/3 và `traits_generics fb_c17` 2/2. | M6 vẫn là hàng rào chính cho vùng chứa mutability bên trong. |
| H10 đối số môi trường | `encode_args` chuyển từng `OsString` bằng `to_string_lossy` sang byte UTF-8; `arg_count` đếm cả argv[0], `arg_ptr/arg_len` cùng đọc từ bảng giữ suốt tiến trình. G2.2 thêm test Windows `arguments_are_utf8_and_preserve_count_for_empty_long_and_multiple_values` với surrogate lẻ, chuỗi rỗng, chuỗi 32.000 ký tự, Unicode, nhiều đối số; kiểm `arg_count`, `arg_ptr`, `arg_len`, `str::from_utf8`. | Test Windows xanh. Surrogate lẻ được thay bằng U+FFFD, không giữ byte không hợp lệ nguyên trạng; đây là hành vi lossy hiện hành, không phải lossless argv. Không có test tích hợp end-to-end gọi chương trình grit từ shell với surrogate lẻ; giả định của `to_string_lossy` vẫn là điểm hợp đồng cần Claude đọc. |

## Sổ nghi vấn G2.2 (G1.5 mở rộng; trạng thái có căn cứ)

| ID | H-line / nghi vấn cụ thể | Thí nghiệm / bằng chứng | Kết quả | FB |
|---|---|---|---|---|
| H10-01 | Chuyển đối số OS sang UTF-8 có thể tạo `&str` không hợp lệ hoặc làm lệch độ dài/con trỏ | `arguments_are_utf8_preserve_count...`; `lone_surrogate...`; kiểm mọi phần tử qua `str::from_utf8`, count/pointer/len | Bác bỏ lỗi UTF-8 không hợp lệ trong chuyển đổi đã kiểm; xác nhận dữ liệu OS không hợp lệ bị thay U+FFFD (lossy). Chưa có test tích hợp CLI surrogate lẻ | — |
| GUARD-01 | Guard có thể nhận intrinsic LLVM ngoài tập codegen, gồm `llvm.abs` với cờ poison | `rejects_non_allowlisted_llvm_intrinsic_with_poison_flag`, trước/sau FB-C30 | Xác nhận trước sửa; sau sửa test bắt và guard 35/35 | FB-C30 |
| RT-01 | FFI slice null + zero có thể UB tại `from_raw_parts` | `fs_empty_paths_accept_null_zero_ffi_slices`, trước/sau | Xác nhận trước sửa, xanh sau | FB-C28 |
| THREAD-01 | Main/worker stack thiếu có thể lặng lẽ chạy trên stack nhỏ | `stack_depth` trước/sau; G2.0 fault injection `GRITRT_TEST_FORCE_MAIN_STACK_FAILURE` | Xác nhận fallback worker; main nay fail-closed, code 71 | FB-C29 |
| ARENA-01 | Các test vùng không chứng minh mọi kích thước/căn lề và release mặc định không bật overlap checker | `arena_dynamic` 6/6; không có exhaustive oracle mọi kích thước | Chưa chứng minh tính tổng quát; giới hạn checker release được giữ rõ | — |
| VEC-01 | Nhánh `len+n` / `2*cap` có thể tràn với kiểu chưa hỗ trợ | `vec_overflow` 2/2; không ép được kích thước phần tử chưa hỗ trợ | Chưa kiểm bằng phản ví dụ thực thi; phạm vi hiện chưa đạt được qua kiểu hiện hỗ trợ | — |
| SLICE-01 | Có thể có đường tạo lát dùng độ dài cũ / biên UTF-8 sai | FB-C20 `slice_stale_len` 1/1; 902 ca so với Rust | Không tái hiện ở tập đã thử; không bao phủ mọi đường tạo lát | FB-C20 |
| UTF8-01 | Oracle UTF-8 runtime lỗi có thể làm sai toàn đường kiểm | 100.000 chuỗi và 902 boundary cases so Rust ở dev/release-o2 | Không bất đồng quan sát được trong tập đã thử | — |
| ATOMIC-01 | Thứ tự atomic/liveness có thể sai dù test ngắn xanh | `threads_repeat`, dev/release-o2 ×300/mẫu; không có TSan/loom | Không có lỗi quan sát được; race/deadlock nói chung chưa được loại trừ | — |
| CELL-01 | `Cell` có thể cho phép Copy/alias qua kiểu mang interior mutability | `interior_mut` 3/3, `traits_generics fb_c17` 2/2 | Không tái hiện; phụ thuộc M6 | — |
| GUARD-02 | Số abort theo hàm/loại có thể che mất vị trí Assert bị bỏ; target late-pass ngoài Guard-post | Không có thí nghiệm khẳng định tương đương ở mức vị trí/obj cuối | Chưa kiểm; giới hạn cấu trúc đã xác định, không được ghi đạt độc lập | — |
| ATTEST-01 | Attestation có thể không ràng buộc profile/runtime/toolchain/object cuối | Đọc cấu trúc metadata; chưa có kiểm tra liên kết binary cuối | Chưa kiểm; giới hạn thiết kế hiện hữu | — |
| UNSAFE-01 | `unsafe` trong `gt-rt` có thể tăng mà không có kiểm kê | Đọc đường liên quan; chưa có bộ đếm khớp danh sách cho phép | Chưa kiểm; cần kiểm kê tự động riêng | — |

**Phạm vi sổ:** liệt kê các nghi vấn nêu rõ ở H1–H10 và phần “còn mở” của audit G1.5; đây không phải tuyên bố mọi rủi ro của TCB đã được liệt kê. Có 13 mục; 8 có thí nghiệm/bằng chứng gắn trực tiếp nhưng hữu hạn, 5 vẫn là giới hạn chưa kiểm được bằng thí nghiệm quyết định. Claude cần xem lại những hàng chưa kết luận và mọi mã Codex bên dưới.

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
5. G2.0.d đã bỏ fallback chạy entry trên stack hiện tại: lỗi tạo thread main 256 MiB ghi thông báo và thoát mã 71; unit test subprocess ép nhánh này. Việc cấp stack 256 MiB vẫn có thể thất bại vì giới hạn tài nguyên.
6. `gt-rt` có các khối `unsafe`; G1.5 đọc các đường đã nêu nhưng **chưa** có test tự động đếm `unsafe` khớp danh sách TCB.

## Lệnh và kết quả G1.5 liên quan

- `tools/preflight.sh D:/Grit` qua Git Bash chỉ định: `PREFLIGHT_OK` trước từng lô được chạy trong lượt này.
- Test hồi quy mới: trước sửa `0 passed; 1 failed`, panic tại assertion với `[]`; sau sửa `1 passed`.
- `cargo test -p gt-guard --test guard -- --test-threads=2`: 35 passed, 0 failed.
- G2.0.d `cargo test --manifest-path runtime/gt-rt/Cargo.toml`: 8 passed/0 failed; subprocess ép `GRITRT_TEST_FORCE_MAIN_STACK_FAILURE`, kiểm đúng mã 71 và stderr.
- Các test H1–H10, runtime và driver bổ sung được ghi theo lệnh/kết quả trong mục G1.5 của `docs/RUN_REPORTS.md` và `docs/HANDOFF.md`.
- **G1.6 full gate:** chạy liền mạch lần 2 trên HEAD `c5aa45b`, `GATE_RESULT=GREEN`, 1.017/0/202; runtime 7/7; fail-open 470/0 mới. READINESS §7 không được tự suy rộng từ kết quả gate; các giới hạn trên vẫn mở. Chi tiết cổng ở `docs/RUN_REPORTS.md`; tình trạng bundle/push ở HANDOFF/RUNLIST.
