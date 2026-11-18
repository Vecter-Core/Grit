# TSAN_FEASIBILITY — đánh giá khả thi kiểm động đua dữ liệu cho Grit (GÓI 3 S4.e, 2026-11-18)

**Trạng thái: bản đánh giá ban đầu (GÓI 3); ĐÃ CÀI ĐẶT đường (b) ở GÓI 4 — xem `docs/TSAN_RESULTS.md`, `tools/tsan_check.sh`, job CI `tsan`.** Chưa chạy thí nghiệm nào ở đây; mọi ước lượng công sức là phán đoán của tác giả (`CHƯA KIỂM`). Bối cảnh: điều kiện rủi ro H7/H8 (`Atomic`, `Mutex`, `spawn`) đang được chấp nhận bằng kiểu (`Send`/`Sync`, E0501/E0502/E0503) + corpus + chạy lặp 300 lần ở dev và release-o2; **chưa có kiểm động đua dữ liệu** (`docs/GIANG_DECISIONS_2026-11-18.md` B4).

## ThreadSanitizer (TSan) cần gì
1. **Nền Linux (hoặc macOS).** Runtime TSan (`libclang_rt.tsan`) không có cho Windows/MSVC; bản LLVM 20.1.8 dựng sẵn ở `D:\Tools` không kèm nó. ⇒ chạy trên máy Linux đám mây (dòng Linux của dự án đã có `docs/QUICKSTART_CLOUD.md`).
2. **Đưa chỉ thị vào IR do `gritc` sinh:** hàm phải mang thuộc tính `sanitize_thread` và pass `tsan`/`tsan-module` của LLVM phải chạy. Hai đường: (a) *trong `gritc`* (đổi TCB: Guard-post phải cho phép lời gọi `__tsan_*`; bảng `RUNTIME` của Guard đang ĐÓNG ⇒ cần hợp đồng TCB trước, theo `AGENTS.md` "Intrinsic mới cần hợp đồng TCB trước"); (b) **ngoài `gritc` (khuyến nghị)**: lấy tệp `.ll` đã qua Guard, chèn thuộc tính bằng một script, rồi `clang -fsanitize=thread tệp.ll libgt_rt.a`. Đường (b) không đụng TCB; phải kiểm gritc có đường xuất `.ll` ổn định (thư mục `tests/golden_ll` cho thấy có đường xuất, CHƯA kiểm tại sao dùng).
3. **Dựng lại `gt-rt` có sanitizer:** Rust nightly `-Zsanitizer=thread` (+ `-Zbuild-std` để `std` cũng được dò); nếu không, `std` (mutex/thread) bị TSan coi là không đồng bộ ⇒ báo sai nhiều.
4. **Khoá tự cài:** `gt-rt` trên Linux dùng khoá quay + nhường (tự viết bằng atomic) thay `pthread_mutex`. TSan hiểu atomic nên hiểu được quan hệ happens-before nếu thứ tự bộ nhớ đúng — nhưng đó chính là thứ cần kiểm; nếu khoá dùng `Relaxed` ở chỗ cần `Acquire/Release` TSan sẽ báo (là phát hiện thật, không phải báo sai). Trên Windows khoá là SRWLOCK (TSan không có ở Windows ⇒ không kiểm được trên máy chính).

## TSan KHÔNG làm được
- Chỉ thấy đua dữ liệu **trên các lịch thực sự xảy ra** (không vét cạn); chương trình ngắn, 2–4 luồng: xác suất bỏ sót thấp nhưng không bằng 0.
- Không chứng minh thứ tự bộ nhớ đúng cho phần cứng yếu (ARM): trên x86 vẫn chạy mô hình TSO; TSan mô hình hoá happens-before theo C++ nên bắt được nhiều lỗi thứ tự, nhưng không thay thế kiểm trên ARM.
- Không bắt deadlock (có chế độ phát hiện đảo thứ tự khoá, chỉ với `pthread`; khoá tự cài thì không).

## Ước lượng công sức (phán đoán; CHƯA KIỂM)
| việc | công sức |
|---|---|
| Thử nghiệm đường (b) thủ công cho 2 mẫu `threads`, `two_locks` trên Linux | 1–2 ngày (gồm dựng `gt-rt` nightly + sanitizer) |
| Tự động hoá thành job CI Linux (chạy mọi mẫu đa luồng trong corpus ×N dưới TSan, 0 báo cáo mới = xanh) | thêm 1–2 ngày, sau khi CI GitHub chạy được (B7) |
| Đường (a) trong `gritc` (hồ sơ `--sanitize=thread`): hợp đồng TCB + Guard + test | 1–2 tuần; KHÔNG khuyến nghị lúc này |

## Phương án rẻ thay thế / bổ sung
1. **Chạy lặp tăng cường + nhiễu lịch:** nâng `threads_repeat` từ 300 lên vài chục nghìn lượt ở Linux, chèn `yield`/đặt affinity ngẫu nhiên (chỉ trong test). Rẻ, bắt được lỗi hiếm hơn; vẫn là bằng chứng thực nghiệm.
2. **Mô hình `loom` cho các nguyên thuỷ của `gt-rt`** (khoá quay, hộp tham số `spawn`) viết lại bằng kiểu atomic của `loom` trong test Rust: vét cạn mọi lịch của một mô hình nhỏ. Giới hạn: kiểm MÔ HÌNH chứ không phải mã thật (SRWLOCK/pthread không mô hình hoá được); chỉ có giá trị với phần khoá quay của Linux.
3. **Bộ lập lịch vét cạn trên MIR** (thông dịch nhiều luồng với mọi xen kẽ, mô hình nhất quán tuần tự) cho chương trình rất nhỏ: tìm deadlock/vi phạm bất biến ở mức Grit; công sức 1–2 tuần; không thấy lỗi bộ nhớ yếu.

## Khuyến nghị
1. Giữ B4: chấp nhận rủi ro cho Seed, nói rõ trong `SAFETY_CLAIMS` "an toàn theo kiểu; chưa có kiểm động đua dữ liệu".
2. **Làm đường (b) trên Linux** sau khi (i) repo đẩy được và (ii) CI chạy; bắt đầu bằng hai mẫu `threads`, `two_locks`. Đây là bước rẻ nhất cho bằng chứng thật đầu tiên.
3. Chạy lặp tăng cường (phương án 1) song song vì gần như không tốn công.
4. Chỉ làm đường (a)/loom/lập lịch vét cạn nếu bước 2 bắt được lỗi thật hoặc Giang muốn nhãn "kiểm động đua dữ liệu".
