# Intrinsic / runtime TRUSTED của Phase 1 (GRIT_PLAN §3.3 mục 6 và 6b, D22)

Mọi hàm ở đây là phần của TCB: compiler KHÔNG kiểm thân của chúng (prelude chỉ có chữ ký; backend LLVM hạ thành lệnh LLVM hoặc lời gọi runtime). Mỗi mục ghi: chữ ký Grit, hiệu ứng, capability cần có, điều kiện abort, bất biến, hạ xuống đâu, và **người duyệt (D22)**. Trạng thái người duyệt: `CHỜ GIANG` = tác giả (Claude) đã viết lý do, CHƯA có chữ ký duyệt của Giang — không được coi là đã duyệt.
IR Guard (gt-guard) giữ bảng đóng `RUNTIME` của các hàm runtime tương ứng; thêm hàm runtime mới mà quên thêm vào bảng ⇒ Guard từ chối (fail-closed).

## A. Năng lực / I/O (thêm 2026-10-13, dogfood) — lớp nghĩa vụ IR Guard: **E0310**
| intrinsic (prelude) | hiệu ứng | capability cần | hạ thành | điều kiện abort | bất biến | người duyệt |
|---|---|---|---|---|---|---|
| `root_fs() root_net() root_env() root_clock() root_alloc() root_out()` → `Cap<K>` | `∅` | KHÔNG (là NGUỒN gốc) — nhưng CHỈ `main` cấp cao của tệp được gọi/nhắc tới (E0310, D29) | giá trị zero của `Cap<K>` (struct rỗng về nghĩa; không có runtime) | không | không có quyền thực: quyền nằm ở việc sở hữu giá trị + hiệu ứng `io` của thao tác dùng nó | CHỜ GIANG |
| `Cap<Env>::arg(&self, i: usize) → &str` | `io` | `&Cap<Env>` | `gritrt_arg_ptr(i)`, `gritrt_arg_len(i)` ⇒ lát `{ptr,len}` | `i` ≥ số đối số ⇒ abort `arg_out_of_range` | lát trỏ vào bộ nhớ sống tới hết tiến trình, không bao giờ bị ghi; đối số không-UTF-8 bị chuyển lossy (U+FFFD) nên luôn hợp lệ `&str` | CHỜ GIANG |
| `Cap<Fs>::read(&self, p: &str) → File` | `io` | `&Cap<Fs>` | `gritrt_fs_read(ptr,len) → i32` (tay cầm) | đường dẫn không UTF-8 hoặc lỗi hệ điều hành (không có tệp, quyền, …) ⇒ abort `io_error` + dòng chi tiết ở stderr | đọc NGUYÊN tệp, bộ nhớ rò rỉ có chủ đích (sống tới hết tiến trình ⇒ lát từ `bytes` không bao giờ treo); tay cầm = chỉ số trong bảng toàn cục có khoá | CHỜ GIANG |
| `File::len(&self) → usize` | `∅` | không (File đã là bằng chứng quyền) | `gritrt_file_len(h)` | tay cầm sai ⇒ abort `io_error` (không xảy ra với giá trị do `read` tạo vì `File` không giả mạo được ngoài prelude) | `len` = số byte đã đọc, bất biến sau `read` | CHỜ GIANG |
| `File::bytes(&self) → &[u8]` | `∅` | không | `gritrt_file_ptr(h)` + `gritrt_file_len(h)` ⇒ lát | như trên | lát chỉ đọc, đúng `len`, sống `'static` thực tế (mượn từ `&self` an toàn hơn cần thiết) | CHỜ GIANG |
| `Cap<Out>::write_u64(&self, n: u64)` | `io` | `&Cap<Out>` | `gritrt_write_u64(n)` | ghi stdout thất bại ⇒ abort `io_error` | in thập phân của `n`, flush ngay; không chèn gì khác | CHỜ GIANG |
| `Cap<Out>::write_str(&self, s: &str)` | `io` | `&Cap<Out>` | `gritrt_write_str(ptr,len)` | ghi thất bại ⇒ abort `io_error` | ghi nguyên các byte của `s`, flush ngay | CHỜ GIANG |

## B. Đồng thời / bộ nhớ (M10) — lớp nghĩa vụ: E0503 (atomic), E0502 (khoá), E0501 (spawn)
| intrinsic | hiệu ứng | capability | hạ thành | abort | bất biến | người duyệt |
|---|---|---|---|---|---|---|
| `Atomic<T>::{new,load,store,swap,fetch_add,fetch_sub,compare_exchange}` | `∅` | không (kiểu `Sync`) | `atomicrmw`/`cmpxchg`/`load atomic`/`store atomic` thật với ordering LÀ HẰNG tại chỗ gọi (không hằng ⇒ chưa hỗ trợ, thoát 3) | không | ordering đúng như mã nguồn (E0503 đòi tường minh); `load` không Release/AcqRel, `store` không Acquire/AcqRel, failure CAS không Release/AcqRel (không hợp lệ ⇒ từ chối) | CHỜ GIANG |
| `Mutex<T,L>::{new,with}` | `lock(L)` | không | `gritrt_mutex_lock/unlock` (SRWLOCK) quanh lời gọi hàm | không | khoá giữ suốt lời gọi `f`; thứ tự khoá kiểm tĩnh E0502; không có `drop` giữa chừng | CHỜ GIANG |
| `Cell<T>::{new,get,set}` | `∅` | không (không `Sync`) | load/store thường | không | chỉ một luồng thấy (Send/Sync suy cấu trúc) | CHỜ GIANG |
| `spawn(f, a)` | `detach` (chỉ prelude) | không | hộp `{f,a}` qua `gritrt_alloc` + `gritrt_spawn(tramp, hộp)` | tạo luồng thất bại ⇒ abort | `a: Send` (E0501); không có `join`; hộp không giải phóng | CHỜ GIANG |
| `<[T]>::len`, `core::abort` | `∅` / `div` | không | trích độ dài lát / `gritrt_abort(10)` | `core::abort` luôn abort | — | CHỜ GIANG |

## C. Runtime `gt-rt` (staticlib, `panic = "abort"`): hàm xuất ra cho mã sinh
`gritrt_start`, `gritrt_abort(kind)` (mã thoát **70**, kind = tên `AssertKind` hoặc `io_error`/`arg_out_of_range`/`abort`), `gritrt_report_int` (chỉ khi `GRITRT_RESULT_FILE`), `gritrt_alloc`, `gritrt_spawn`, `gritrt_mutex_lock/unlock`, `gritrt_arg_ptr/len`, `gritrt_fs_read`, `gritrt_file_len/ptr`, `gritrt_write_u64/str`. Danh sách đóng của IR Guard (`gt_guard::RUNTIME`).
Ranh giới `unsafe` của gt-rt: ADR-031.
