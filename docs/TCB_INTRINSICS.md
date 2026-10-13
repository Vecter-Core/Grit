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
| `Cap<Env>::arg_count(&self) → usize` | `io` | `&Cap<Env>` | `gritrt_arg_count()` | không | số đối số kể cả tên chương trình (chỉ số 0); không đổi suốt tiến trình | CHỜ GIANG |
| `Cap<ErrOut>::write_u64(&self, n: u64)`, `write_str(&self, s: &str)` (marker `ErrOut`, KHÔNG đặt tên `Err` vì trùng biến thể `Result::Err`) | `io` | `&Cap<ErrOut>` (từ `root_err()` chỉ ở `main`, E0310) | `gritrt_ewrite_u64(n)`, `gritrt_ewrite_str(ptr,len)` — ghi STDERR, flush ngay | ghi thất bại ⇒ abort `io_error` | stderr tách khỏi stdout; cùng quy tắc nội dung như `Cap<Out>` | CHỜ GIANG |
| `core::exit(code: u32) → !` | `div` | không | `gritrt_exit(code)` (flush stdout, thoát với `min(code,255)`) | không (đây là thoát CÓ KIỂM SOÁT, khác abort mã 70) | không bao giờ trả về; sau lời gọi codegen luôn có khối `trap.*` | CHỜ GIANG |
| `core::fuel_left(n: u64)` (lang của M5) | `∅` | không | KHÔNG phải hàm runtime: M5 hạ thành `Assert(n > 0, kind=Fuel)` ⇒ `gritrt_abort(16)` "fuel_exhausted" khi `n == 0` | `n == 0` ⇒ abort `fuel_exhausted`; M7 xoá nếu chứng minh được, nếu không giữ + ghi chú N0905 | dùng SAU vòng kiểu nhiên liệu; không có tác dụng phụ | CHỜ GIANG |
| `Cap<Out>::write_u64(&self, n: u64)` | `io` | `&Cap<Out>` | `gritrt_write_u64(n)` | ghi stdout thất bại ⇒ abort `io_error` | in thập phân của `n`, flush ngay; không chèn gì khác | CHỜ GIANG |
| `Cap<Out>::write_str(&self, s: &str)` | `io` | `&Cap<Out>` | `gritrt_write_str(ptr,len)` | ghi thất bại ⇒ abort `io_error` | ghi nguyên các byte của `s`, flush ngay | CHỜ GIANG |
| `Cap<Fs>::open(&self, p: &str) → Result<File, IoError>` | `io` | `&Cap<Fs>` | `gritrt_fs_open(ptr,len) → i32` rồi dựng `Result` (Ok(File{x:h}) khi h ≥ 0; Err(IoError k) với mã âm −(k+1)) | KHÔNG abort vì lỗi I/O (đó là điểm khác `read`); chỉ abort nếu bảng tay cầm tràn `i32` | `IoError`: NotFound(−1) / Denied(−2) / Invalid(−3, đường dẫn không UTF-8) / Other(−4); ánh xạ cố định, thứ tự biến thể của prelude là hợp đồng với runtime | CHỜ GIANG |

## B. Đồng thời / bộ nhớ (M10) — lớp nghĩa vụ: E0503 (atomic), E0502 (khoá), E0501 (spawn)
| intrinsic | hiệu ứng | capability | hạ thành | abort | bất biến | người duyệt |
|---|---|---|---|---|---|---|
| `Atomic<T>::{new,load,store,swap,fetch_add,fetch_sub,compare_exchange}` | `∅` | không (kiểu `Sync`) | `atomicrmw`/`cmpxchg`/`load atomic`/`store atomic` thật với ordering LÀ HẰNG tại chỗ gọi (không hằng ⇒ chưa hỗ trợ, thoát 3) | không | ordering đúng như mã nguồn (E0503 đòi tường minh); `load` không Release/AcqRel, `store` không Acquire/AcqRel, failure CAS không Release/AcqRel (không hợp lệ ⇒ từ chối) | CHỜ GIANG |
| `Mutex<T,L>::{new,with}` | `lock(L)` | không | `gritrt_mutex_lock/unlock` (SRWLOCK) quanh lời gọi hàm | không | khoá giữ suốt lời gọi `f`; thứ tự khoá kiểm tĩnh E0502; không có `drop` giữa chừng | CHỜ GIANG |
| `Cell<T>::{new,get,set}` | `∅` | không (không `Sync`) | load/store thường | không | chỉ một luồng thấy (Send/Sync suy cấu trúc) | CHỜ GIANG |
| `spawn(f, a)` | `detach` (chỉ prelude) | không | hộp `{f,a}` qua `gritrt_alloc` + `gritrt_spawn(tramp, hộp)` | tạo luồng thất bại ⇒ abort | `a: Send` (E0501); không có `join`; hộp không giải phóng | CHỜ GIANG |
| `<[T]>::len`, `core::abort` | `∅` / `div` | không | trích độ dài lát / `gritrt_abort(10)` | `core::abort` luôn abort | — | CHỜ GIANG |

## B2. Arena (thêm 2026-10-13) — lớp nghĩa vụ IR Guard: **E0405** (tham chiếu không thoát Arena; kiểm ở M6)
| intrinsic | hiệu ứng | capability | hạ thành | abort | bất biến | người duyệt |
|---|---|---|---|---|---|---|
| `Cap<Alloc>::arena(&self) → Arena`, `Arena::new() → Arena` | `mem` | `&Cap<Alloc>` (arena) / không (new) | `gritrt_arena_new() → i32` (tay cầm trong `Arena{x}`) | bảng arena tràn `i32` | mỗi arena độc lập; tay cầm không giả mạo được ngoài prelude | CHỜ GIANG |
| `Arena::alloc<T>(&self, v: T) → &T` | `mem` | `&Arena` | `gritrt_arena_alloc(h, size_of T, align_of T)` + `store v` | hết bộ nhớ, `align` không phải luỹ thừa 2 hoặc > 4096 ⇒ abort `unknown` | con trỏ trả về trỏ vào chunk sống tới `reset` hoặc hết tiến trình; KHÔNG chạy destructor của `T` (Phase 1 không có `Drop` người dùng); borrowck M6 (E0405) bảo đảm tham chiếu không sống quá arena | CHỜ GIANG |
| `Arena::reset(&!self)` | `mem` | `&!Arena` | `gritrt_arena_reset(h)` | tay cầm sai | `&!` ⇒ không còn tham chiếu tới nội dung (M6); chunk đầu được giữ, phần còn lại trả cho allocator | CHỜ GIANG |

## C. Runtime `gt-rt` (staticlib, `panic = "abort"`): hàm xuất ra cho mã sinh
`gritrt_start`, `gritrt_abort(kind)` (mã thoát **70**, kind = tên `AssertKind` hoặc `io_error`/`arg_out_of_range`/`abort`), `gritrt_report_int` (chỉ khi `GRITRT_RESULT_FILE`), `gritrt_alloc`, `gritrt_spawn`, `gritrt_mutex_lock/unlock`, `gritrt_arg_count`, `gritrt_arg_ptr/len`, `gritrt_exit`, `gritrt_ewrite_u64/str`, `gritrt_fs_read`, `gritrt_fs_open`, `gritrt_arena_new/alloc/reset`, `gritrt_file_len/ptr`, `gritrt_write_u64/str`. Danh sách đóng của IR Guard (`gt_guard::RUNTIME`).
Ranh giới `unsafe` của gt-rt: ADR-031.
