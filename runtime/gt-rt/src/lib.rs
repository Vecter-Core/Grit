//! gt-rt: runtime tối thiểu cho chương trình do `gritc` sinh (ADR-029 §5). Chỉ x86_64-pc-windows-msvc ở Phase 1.
//!
//! Hợp đồng (ABI C), codegen gọi đúng các ký hiệu sau:
//! * `gritrt_start(main)` chạy `main` rồi trả 0 (mã thoát bình thường).
//! * `gritrt_abort(kind)` ghi `abort: <loại>` lên stderr rồi dừng tiến trình NGAY với mã **70** (D3: không unwinding, không cleanup).
//! * `gritrt_mutex_lock/unlock(lock)` — `lock` trỏ tới ô 8 byte KHỞI TẠO BẰNG 0 (SRWLOCK thật của Windows).
//! * `gritrt_spawn(f, arg)` tạo luồng HĐH thật chạy `f(arg)`; không có `join` (đúng ngữ nghĩa M8).
#![allow(unsafe_code)]

#[cfg(not(all(any(windows, target_os = "linux"), target_arch = "x86_64")))]
compile_error!(
    "gt-rt Phase 1 chỉ hỗ trợ x86_64 Windows (ADR-029) và x86_64 Linux; target khác phải được thêm có chủ đích"
);

use std::io::Write;

/// Mã thoát của chương trình khi abort (bảng phân vùng D17: tách khỏi mã 0–3 của `gritc`).
pub const ABORT_EXIT_CODE: i32 = 70;

/// Loại abort; số thứ tự là hợp đồng với codegen; TÊN trùng `AssertKind::name` của MIR (đối chiếu với thông dịch).
pub const ABORT_KINDS: [&str; 19] = [
    "unknown",
    "overflow",
    "index_out_of_bounds",
    "div_by_zero",
    "assertion",
    "unreachable",
    "cast_lossy",
    "precondition",
    "postcondition",
    "invariant",
    "abort",
    "overflow_neg",
    "rem_by_zero",
    "slice_range",
    "io_error",
    "arg_out_of_range",
    "fuel_exhausted",
    "arena_exhausted",
    "arena_overlap",
];

#[unsafe(no_mangle)]
pub extern "C" fn gritrt_abort(kind: i32) -> ! {
    let name = usize::try_from(kind)
        .ok()
        .and_then(|k| ABORT_KINDS.get(k).copied())
        .unwrap_or("unknown");
    let mut err = std::io::stderr().lock();
    let _ = writeln!(err, "abort: {name}");
    let _ = err.flush();
    std::process::exit(ABORT_EXIT_CODE)
}

/// `core::exit(code)`: dừng tiến trình NGAY với trạng thái `code` (không unwinding); khác `abort` (mã 70) — là thoát CÓ KIỂM SOÁT của chương trình.
#[unsafe(no_mangle)]
pub extern "C" fn gritrt_exit(code: u32) -> ! {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(i32::try_from(code.min(255)).unwrap_or(255))
}

/// Ngăn xếp của luồng chạy `main`: 256 MiB (chỉ giữ chỗ địa chỉ; trang được cấp khi chạm tới). Ngăn xếp mặc định của tiến trình (1 MiB ở Windows,
/// 8 MiB ở Linux) quá nhỏ cho chương trình HỢP LỆ và đã được M8 chứng minh dừng (đệ quy sâu có `↓`, mảng lớn cục bộ) — chúng bị HĐH giết và
/// bị báo như lỗi nội bộ (F-06 của bộ kiểm toán). Tràn quá 256 MiB vẫn là giới hạn tài nguyên của chương trình (docs/KNOWN_LIMITATIONS.md).
const MAIN_STACK_BYTES: usize = 256 << 20;

#[unsafe(no_mangle)]
pub extern "C" fn gritrt_start(main: extern "C" fn()) -> i32 {
    match std::thread::Builder::new()
        .name("grit-main".to_string())
        .stack_size(MAIN_STACK_BYTES)
        .spawn(move || main())
    {
        // `panic = "abort"` nên `join` chỉ Err khi luồng chết bất thường; không để lỗi đó thành "thành công"
        Ok(h) => match h.join() {
            Ok(()) => 0,
            Err(_) => gritrt_abort(ABORT_KINDS.len().try_into().unwrap_or(i32::MAX)),
        },
        // không giữ chỗ được 256 MiB (giới hạn bộ nhớ ảo của môi trường): lùi về ngăn xếp tiến trình thay vì không chạy được
        Err(_) => {
            main();
            0
        }
    }
}

/// Báo kết quả của `main` (số nguyên/bool) cho `gritc run`: ghi `<giá trị>` vào tệp có đường dẫn trong biến môi trường
/// `GRITRT_RESULT_FILE`; KHÔNG đặt biến ⇒ không làm gì (chương trình chạy độc lập không in gì thêm). Chỉ để đối chứng với thông dịch.
#[unsafe(no_mangle)]
pub extern "C" fn gritrt_report_int(value: i64, signed: i32) {
    let Some(path) = std::env::var_os("GRITRT_RESULT_FILE") else {
        return;
    };
    let text = if signed != 0 {
        value.to_string()
    } else {
        (value as u64).to_string()
    };
    if std::fs::write(path, text).is_err() {
        gritrt_abort(0);
    }
}

#[cfg(windows)]
#[repr(C)]
struct SrwLock(*mut u8);

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn AcquireSRWLockExclusive(l: *mut SrwLock);
    fn ReleaseSRWLockExclusive(l: *mut SrwLock);
}

/// # Safety
/// `lock` trỏ tới ô 8 byte căn lề 8, khởi tạo 0 và sống suốt thời gian khoá (codegen đảm bảo: ô nằm trong `Mutex`).
#[cfg(windows)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gritrt_mutex_lock(lock: *mut u8) {
    unsafe { AcquireSRWLockExclusive(lock.cast()) }
}

/// # Safety
/// Như `gritrt_mutex_lock`; luồng gọi đang giữ khoá (M8 đảm bảo `with` bao cặp lock/unlock).
#[cfg(windows)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gritrt_mutex_unlock(lock: *mut u8) {
    unsafe { ReleaseSRWLockExclusive(lock.cast()) }
}

/// Linux: khoá quay + nhường luồng trên ô 8 byte khởi tạo 0 (cùng hợp đồng ô khoá với SRWLOCK của Windows; không cần phụ thuộc ngoài).
///
/// # Safety
/// `lock` trỏ tới ô 8 byte căn lề 8, khởi tạo 0.
#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gritrt_mutex_lock(lock: *mut u8) {
    let a = unsafe { &*lock.cast::<std::sync::atomic::AtomicU64>() };
    while a
        .compare_exchange_weak(
            0,
            1,
            std::sync::atomic::Ordering::Acquire,
            std::sync::atomic::Ordering::Relaxed,
        )
        .is_err()
    {
        std::thread::yield_now();
    }
}

/// # Safety
/// Như `gritrt_mutex_lock`.
#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gritrt_mutex_unlock(lock: *mut u8) {
    let a = unsafe { &*lock.cast::<std::sync::atomic::AtomicU64>() };
    a.store(0, std::sync::atomic::Ordering::Release);
}

/// Con trỏ thô gửi qua luồng; tính `Send` do kiểm tĩnh E0501 ở compiler bảo đảm (đối số của `spawn` là `Send`).
struct Arg(*mut u8);
unsafe impl Send for Arg {}

// ---- I/O tối thiểu cho dogfood (docs/DOGFOOD.md): đối số dòng lệnh, đọc nguyên tệp, ghi stdout. Mọi lỗi ⇒ abort (chưa có Result/IoError).

fn cmdline() -> &'static Vec<Vec<u8>> {
    static ARGS: std::sync::OnceLock<Vec<Vec<u8>>> = std::sync::OnceLock::new();
    ARGS.get_or_init(|| {
        std::env::args_os()
            .map(|a| a.to_string_lossy().into_owned().into_bytes())
            .collect()
    })
}

fn arg_at(i: u64) -> &'static [u8] {
    match usize::try_from(i).ok().and_then(|k| cmdline().get(k)) {
        Some(a) => a.as_slice(),
        None => {
            eprintln!("arg_out_of_range: đối số #{i} không tồn tại");
            gritrt_abort(15)
        }
    }
}

/// Số đối số dòng lệnh (kể cả tên chương trình ở chỉ số 0).
#[unsafe(no_mangle)]
pub extern "C" fn gritrt_arg_count() -> u64 {
    cmdline().len() as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn gritrt_arg_ptr(i: u64) -> *const u8 {
    arg_at(i).as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn gritrt_arg_len(i: u64) -> u64 {
    arg_at(i).len() as u64
}

fn files() -> &'static std::sync::Mutex<Vec<&'static [u8]>> {
    static FILES: std::sync::OnceLock<std::sync::Mutex<Vec<&'static [u8]>>> =
        std::sync::OnceLock::new();
    FILES.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

fn file_at(h: i32) -> &'static [u8] {
    let g = files().lock().unwrap_or_else(|e| e.into_inner());
    match usize::try_from(h).ok().and_then(|k| g.get(k)) {
        Some(b) => b,
        None => gritrt_abort(14),
    }
}

/// Đọc TOÀN BỘ nội dung từ `r`, kỳ vọng đúng `expected` byte (kích thước lúc mở). Lỗi đọc giữa chừng ⇒ `Err`; đọc ĐƯỢC ÍT/NHIỀU HƠN kỳ vọng (tệp bị cắt hoặc
/// tăng trong lúc đọc) ⇒ `Err` — KHÔNG BAO GIỜ trả nội dung cụt như thể đầy đủ (D.1: `Cap<Fs>::read` đọc nửa chừng phải abort `io_error`).
fn read_checked<R: std::io::Read>(mut r: R, expected: u64) -> std::io::Result<Vec<u8>> {
    let mut v = Vec::new();
    r.read_to_end(&mut v)?;
    if v.len() as u64 != expected {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            format!(
                "tệp đổi kích thước lúc đọc: đọc {} byte, kỳ vọng {expected}",
                v.len()
            ),
        ));
    }
    Ok(v)
}

/// Thay `std::fs::read`: mở tệp, lấy kích thước, đọc bằng [`read_checked`]. Chỉ TỆP THƯỜNG có nghĩa: thư mục, tệp bị khoá, ống/thiết bị (kích thước 0 ≠ nội dung) ⇒ `Err`.
fn read_file_checked(path: &str) -> std::io::Result<Vec<u8>> {
    let f = std::fs::File::open(path)?;
    let meta = f.metadata()?;
    if !meta.is_file() {
        return Err(std::io::Error::other("không phải tệp thường"));
    }
    read_checked(f, meta.len())
}

/// Đọc NGUYÊN tệp vào bộ nhớ (rò rỉ có chủ đích, sống tới hết tiến trình); trả tay cầm. Lỗi (không có tệp, quyền, đường dẫn không UTF-8) ⇒ abort `io_error`.
///
/// # Safety
/// `p` trỏ tới `n` byte đọc được.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gritrt_fs_read(p: *const u8, n: u64) -> i32 {
    let bytes = unsafe { std::slice::from_raw_parts(p, usize::try_from(n).unwrap_or(0)) };
    let Ok(path) = std::str::from_utf8(bytes) else {
        eprintln!("io_error: đường dẫn không phải UTF-8");
        gritrt_abort(14)
    };
    match read_file_checked(path) {
        Ok(v) => {
            let leaked: &'static [u8] = Box::leak(v.into_boxed_slice());
            let mut g = files().lock().unwrap_or_else(|e| e.into_inner());
            g.push(leaked);
            i32::try_from(g.len() - 1).unwrap_or_else(|_| gritrt_abort(14))
        }
        Err(e) => {
            eprintln!("io_error: {path}: {e}");
            gritrt_abort(14)
        }
    }
}

/// Như `gritrt_fs_read` nhưng KHÔNG abort: trả tay cầm (≥ 0) hoặc mã lỗi âm: -1 không có tệp, -2 bị từ chối quyền, -3 đường dẫn không hợp lệ (không UTF-8), -4 lỗi khác.
///
/// # Safety
/// `p` trỏ tới `n` byte đọc được.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gritrt_fs_open(p: *const u8, n: u64) -> i32 {
    let bytes = unsafe { std::slice::from_raw_parts(p, usize::try_from(n).unwrap_or(0)) };
    let Ok(path) = std::str::from_utf8(bytes) else {
        return -3;
    };
    match read_file_checked(path) {
        Ok(v) => {
            let leaked: &'static [u8] = Box::leak(v.into_boxed_slice());
            let mut g = files().lock().unwrap_or_else(|e| e.into_inner());
            g.push(leaked);
            i32::try_from(g.len() - 1).unwrap_or(-4)
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => -1,
            std::io::ErrorKind::PermissionDenied => -2,
            _ => -4,
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gritrt_file_len(h: i32) -> u64 {
    file_at(h).len() as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn gritrt_file_ptr(h: i32) -> *const u8 {
    file_at(h).as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn gritrt_write_u64(n: u64) {
    let mut o = std::io::stdout().lock();
    if write!(o, "{n}").and_then(|()| o.flush()).is_err() {
        gritrt_abort(14);
    }
}

/// Như `gritrt_write_u64` nhưng ra stderr (không đệm).
#[unsafe(no_mangle)]
pub extern "C" fn gritrt_ewrite_u64(n: u64) {
    let mut o = std::io::stderr().lock();
    if write!(o, "{n}").and_then(|()| o.flush()).is_err() {
        gritrt_abort(14);
    }
}

/// Như `gritrt_write_str` nhưng ra stderr.
///
/// # Safety
/// `p` trỏ tới `n` byte đọc được (`n = 0`: `p` có thể null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gritrt_ewrite_str(p: *const u8, n: u64) {
    if n == 0 {
        return; // lát rỗng của `Vec` có thể mang con trỏ null: không dựng `&[u8]` từ null (UB)
    }
    let bytes = unsafe { std::slice::from_raw_parts(p, usize::try_from(n).unwrap_or(0)) };
    let mut o = std::io::stderr().lock();
    if o.write_all(bytes).and_then(|()| o.flush()).is_err() {
        gritrt_abort(14);
    }
}

/// Kiểm `n` byte có phải UTF-8 hợp lệ (RFC 3629: không overlong, không surrogate, ≤ U+10FFFF) bằng `std::str::from_utf8` — nguồn kiểm
/// ĐỘC LẬP với bộ sinh. Trả 1 nếu hợp lệ, 0 nếu không.
///
/// # Safety
/// `p` trỏ tới `n` byte đọc được (`n = 0`: `p` có thể null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gritrt_utf8_valid(p: *const u8, n: u64) -> i32 {
    if n == 0 {
        return 1;
    }
    let bytes = unsafe { std::slice::from_raw_parts(p, usize::try_from(n).unwrap_or(0)) };
    i32::from(std::str::from_utf8(bytes).is_ok())
}

/// # Safety
/// `p` trỏ tới `n` byte đọc được (`n = 0`: `p` có thể null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gritrt_write_str(p: *const u8, n: u64) {
    if n == 0 {
        return; // lát rỗng của `Vec` có thể mang con trỏ null: không dựng `&[u8]` từ null (UB)
    }
    let bytes = unsafe { std::slice::from_raw_parts(p, usize::try_from(n).unwrap_or(0)) };
    let mut o = std::io::stdout().lock();
    if o.write_all(bytes).and_then(|()| o.flush()).is_err() {
        gritrt_abort(14);
    }
}

// ---- Arena (bump allocator có khoá): `Arena::{new,alloc,reset}` của prelude. Bộ nhớ trả ra sống tới `reset`/hết tiến trình.

struct ArenaState {
    chunks: Vec<(usize, usize, usize)>, // (địa chỉ, dung lượng, đã dùng)
    /// kiểm động (hồ sơ dev): các vùng `[bắt đầu, kết thúc)` đã cấp TRONG THẾ HỆ hiện tại (xoá khi `reset`), khoá = địa chỉ bắt đầu
    live: std::collections::BTreeMap<usize, usize>,
}

/// Loại abort `arena_overlap`: vùng do `alloc` cấp chồng lấn vùng khác cùng thế hệ, lệch căn lề, hoặc nằm ngoài chunk của arena.
const ARENA_OVERLAP: i32 = 18;

/// Kiểm động Arena (ADR-040 phụ lục / hợp đồng H2): BẬT ở hồ sơ `dev` (mã khởi động do codegen phát gọi `gritrt_dev_checks`) hoặc khi `GRITRT_ARENA_CHECK=1`.
/// Biến môi trường chỉ BẬT THÊM: KHÔNG có công tắc tắt kiểm ở dev (R4, Seed 1.0; trước đây `GRITRT_ARENA_CHECK=0` tắt được). Là BẰNG CHỨNG (bắt lỗi cấp phát lúc chạy), KHÔNG phải chứng minh; hồ sơ release/release-o2 KHÔNG kiểm.
static ARENA_CHECK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn arena_check_on() -> bool {
    ARENA_CHECK.load(std::sync::atomic::Ordering::Relaxed)
}

/// Gọi một lần ở đầu `gt_entry` của exe hồ sơ dev.
#[unsafe(no_mangle)]
pub extern "C" fn gritrt_dev_checks() {
    ARENA_CHECK.store(true, std::sync::atomic::Ordering::Relaxed);
}

fn arena_check_env_on() -> bool {
    static E: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *E.get_or_init(|| std::env::var("GRITRT_ARENA_CHECK").is_ok_and(|v| v == "1"))
}

/// Chèn lỗi (CHỈ có tác dụng khi kiểm động BẬT, để chứng minh kiểm bắt được), `GRITRT_FAULT=`:
/// `arena_overlap` trả lại vùng của lần cấp trước (trùng đầu); `arena_overlap_tail` (align 1) trả con trỏ chồng 1 byte vào ĐUÔI vùng trước (kiểm "vùng đứng trước");
/// `arena_overlap_head` (align 1, size ≥ 2) trả vùng chồng đúng 1 byte vào ĐẦU vùng trước (kiểm "vùng đứng sau"); `arena_misalign` lệch 1 byte khi `align > 1`;
/// `arena_outside` trả con trỏ ngoài chunk (+256 MiB, vẫn đúng căn lề).
fn arena_fault() -> Option<&'static str> {
    static F: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    F.get_or_init(|| std::env::var("GRITRT_FAULT").ok())
        .as_deref()
        .filter(|f| {
            matches!(
                *f,
                "arena_overlap"
                    | "arena_overlap_tail"
                    | "arena_overlap_head"
                    | "arena_misalign"
                    | "arena_outside"
            )
        })
}

/// Sau khi cấp `[p, p+size)` căn lề `align` trong arena `st`: kiểm (1) căn lề, (2) nằm trọn trong một chunk, (3) không chồng vùng nào cùng thế hệ; rồi ghi nhận.
fn arena_verify(st: &mut ArenaState, p: usize, size: usize, align: usize) {
    let fail = |why: &str| -> ! {
        let mut err = std::io::stderr().lock();
        let _ = writeln!(err, "arena: {why}");
        drop(err);
        gritrt_abort(ARENA_OVERLAP)
    };
    if !p.is_multiple_of(align) {
        fail("con trỏ lệch căn lề");
    }
    let Some(end) = p.checked_add(size) else {
        fail("vùng tràn không gian địa chỉ")
    };
    if size > 0 && !st.chunks.iter().any(|c| p >= c.0 && end <= c.0 + c.1) {
        fail("vùng nằm ngoài chunk của arena");
    }
    if size == 0 {
        return;
    }
    if let Some((_, e)) = st.live.range(..=p).next_back()
        && *e > p
    {
        fail("vùng chồng lấn vùng đã cấp (cùng thế hệ)");
    }
    if let Some((s, _)) = st.live.range(p..).next()
        && *s < end
    {
        fail("vùng chồng lấn vùng đã cấp (cùng thế hệ)");
    }
    st.live.insert(p, end);
}

/// Loại abort `arena_exhausted` (hết bộ nhớ của Arena hoặc vượt giới hạn); số thứ tự khớp `ABORT_KINDS`/`gt-codegen-llvm`.
const ARENA_EXHAUSTED: i32 = 17;

/// Giới hạn TỔNG dung lượng chunk của MỘT arena (byte), đọc một lần từ biến môi trường `GRITRT_ARENA_LIMIT`; mặc định không giới hạn.
/// Chỉ làm `alloc` thất bại SỚM hơn (fail-closed): dùng để kiểm thử `arena_exhausted` và để chặn ngốn bộ nhớ.
fn arena_limit() -> Option<usize> {
    static L: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    *L.get_or_init(|| {
        std::env::var("GRITRT_ARENA_LIMIT")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
    })
}

fn arenas() -> &'static std::sync::Mutex<Vec<ArenaState>> {
    static A: std::sync::OnceLock<std::sync::Mutex<Vec<ArenaState>>> = std::sync::OnceLock::new();
    A.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

const CHUNK: usize = 1 << 20;

#[unsafe(no_mangle)]
pub extern "C" fn gritrt_arena_new() -> i32 {
    let mut g = arenas().lock().unwrap_or_else(|e| e.into_inner());
    g.push(ArenaState {
        chunks: Vec::new(),
        live: std::collections::BTreeMap::new(),
    });
    i32::try_from(g.len() - 1).unwrap_or_else(|_| gritrt_abort(0))
}

/// Cấp `size` byte căn lề `align` (luỹ thừa 2 ≤ 4096), KHÔNG khởi tạo. Hết bộ nhớ ⇒ abort.
#[unsafe(no_mangle)]
pub extern "C" fn gritrt_arena_alloc(h: i32, size: u64, align: u64) -> *mut u8 {
    let (size, align) = (
        usize::try_from(size).unwrap_or(usize::MAX),
        usize::try_from(align).unwrap_or(0),
    );
    if !align.is_power_of_two() || align > 4096 {
        gritrt_abort(0);
    }
    let mut g = arenas().lock().unwrap_or_else(|e| e.into_inner());
    let Some(st) = usize::try_from(h).ok().and_then(|k| g.get_mut(k)) else {
        gritrt_abort(0)
    };
    let check = arena_check_on() || arena_check_env_on();
    if let Some((base, cap, used)) = st.chunks.last_mut() {
        let start = (*base + *used).next_multiple_of(align) - *base;
        if start.checked_add(size).is_some_and(|e| e <= *cap) {
            *used = start + size;
            let p = *base + start;
            return finish_alloc(st, p, size, align, check);
        }
    }
    let cap = CHUNK.max(size.saturating_add(align));
    let held: usize = st.chunks.iter().map(|c| c.1).fold(0, usize::saturating_add);
    if arena_limit().is_some_and(|l| held.saturating_add(cap) > l) {
        gritrt_abort(ARENA_EXHAUSTED);
    }
    let Ok(layout) = std::alloc::Layout::from_size_align(cap, 4096) else {
        gritrt_abort(ARENA_EXHAUSTED)
    };
    let base = unsafe { std::alloc::alloc(layout) } as usize;
    if base == 0 {
        gritrt_abort(ARENA_EXHAUSTED);
    }
    let start = base.next_multiple_of(align) - base;
    st.chunks.push((base, cap, start + size));
    finish_alloc(st, base + start, size, align, check)
}

/// Kết thúc `alloc`: áp chèn lỗi (khi kiểm BẬT), kiểm động, trả con trỏ.
fn finish_alloc(st: &mut ArenaState, p: usize, size: usize, align: usize, check: bool) -> *mut u8 {
    if !check {
        return p as *mut u8;
    }
    let mut p = p;
    match arena_fault() {
        Some("arena_overlap") if size > 0 => {
            if let Some((s, _)) = st.live.iter().next_back() {
                p = *s; // trả lại vùng của lần cấp trước
            }
        }
        Some("arena_overlap_tail") if align == 1 => {
            if let Some((_, e)) = st.live.iter().next_back() {
                p = *e - 1;
            }
        }
        Some("arena_overlap_head") if align == 1 && size >= 2 => {
            if let Some((s, _)) = st.live.iter().next_back()
                && *s + 1 >= size
            {
                p = *s + 1 - size;
            }
        }
        Some("arena_misalign") if align > 1 => p += 1,
        Some("arena_outside") => p += 1 << 28,
        _ => {}
    }
    arena_verify(st, p, size, align);
    p as *mut u8
}

/// Quay lại đầu arena (giữ chunk đầu, trả các chunk còn lại). `&!Arena` ⇒ borrowck đảm bảo không còn tham chiếu tới nội dung.
#[unsafe(no_mangle)]
pub extern "C" fn gritrt_arena_reset(h: i32) {
    let mut g = arenas().lock().unwrap_or_else(|e| e.into_inner());
    let Some(st) = usize::try_from(h).ok().and_then(|k| g.get_mut(k)) else {
        gritrt_abort(0)
    };
    while st.chunks.len() > 1 {
        if let Some((base, cap, _)) = st.chunks.pop()
            && let Ok(layout) = std::alloc::Layout::from_size_align(cap, 4096)
        {
            unsafe { std::alloc::dealloc(base as *mut u8, layout) };
        }
    }
    if let Some(c) = st.chunks.first_mut() {
        c.2 = 0;
    }
    st.live.clear(); // thế hệ mới
}

/// Cấp `size` byte (căn lề 16, KHÔNG khởi tạo) cho hộp tham số của `spawn`; hộp thuộc về luồng mới (không giải phóng: rò rỉ có chủ đích, nhỏ).
#[unsafe(no_mangle)]
pub extern "C" fn gritrt_alloc(size: u64) -> *mut u8 {
    let Ok(layout) =
        std::alloc::Layout::from_size_align(usize::try_from(size).unwrap_or(usize::MAX), 16)
    else {
        gritrt_abort(0)
    };
    let p = unsafe { std::alloc::alloc(layout) };
    if p.is_null() {
        gritrt_abort(0);
    }
    p
}

/// # Safety
/// `f` là hàm hợp lệ nhận `arg`; `arg` sở hữu bởi luồng mới (chuyển quyền sở hữu).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gritrt_spawn(f: extern "C" fn(*mut u8), arg: *mut u8) {
    let a = Arg(arg);
    let r = std::thread::Builder::new().spawn(move || {
        let a = a;
        f(a.0);
    });
    if r.is_err() {
        gritrt_abort(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn start_returns_zero_and_runs_main() {
        static HIT: AtomicU64 = AtomicU64::new(0);
        extern "C" fn m() {
            HIT.fetch_add(1, Ordering::SeqCst);
        }
        assert_eq!(gritrt_start(m), 0);
        assert_eq!(HIT.load(Ordering::SeqCst), 1);
    }

    /// Nhiều luồng thật tăng một bộ đếm dưới khoá thật: tổng phải đúng (không mất cập nhật).
    #[test]
    fn mutex_serializes_real_threads() {
        struct Shared {
            lock: u64,
            count: u64,
        }
        let sh = Box::into_raw(Box::new(Shared { lock: 0, count: 0 })) as usize;
        let hs: Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(move || {
                    let p = sh as *mut Shared;
                    for _ in 0..20_000 {
                        unsafe {
                            gritrt_mutex_lock(std::ptr::addr_of_mut!((*p).lock).cast());
                            let c = std::ptr::read_volatile(std::ptr::addr_of!((*p).count));
                            std::ptr::write_volatile(std::ptr::addr_of_mut!((*p).count), c + 1);
                            gritrt_mutex_unlock(std::ptr::addr_of_mut!((*p).lock).cast());
                        }
                    }
                })
            })
            .collect();
        for h in hs {
            h.join().expect("luồng");
        }
        let s = unsafe { Box::from_raw(sh as *mut Shared) };
        assert_eq!(s.count, 80_000);
    }

    /// Kiểm động Arena BẬT: nhiều lần cấp ngẫu nhiên (kích thước 0..300, căn lề 1..64) qua nhiều chunk và nhiều lần `reset` KHÔNG báo nhầm (0 abort),
    /// và mọi vùng trả ra đúng căn lề, rời nhau trong cùng thế hệ (đối chiếu độc lập bằng danh sách).
    #[test]
    fn arena_dynamic_check_accepts_valid_allocation_sequences() {
        ARENA_CHECK.store(true, std::sync::atomic::Ordering::Relaxed);
        let h = gritrt_arena_new();
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for _gen in 0..4 {
            let mut seen: Vec<(usize, usize)> = Vec::new();
            for _ in 0..4_000 {
                let size = next() % 301;
                let align = 1u64 << (next() % 7);
                let big = next() % 500 == 0;
                let size = if big { size * 5000 } else { size }; // vài khối lớn để sang chunk mới
                let p = gritrt_arena_alloc(h, size, align) as usize;
                assert_eq!(p % align as usize, 0);
                let e = p + size as usize;
                assert!(
                    seen.iter().all(|(s, t)| e <= *s || p >= *t || size == 0),
                    "chồng lấn"
                );
                if size > 0 {
                    seen.push((p, e));
                }
            }
            gritrt_arena_reset(h);
        }
    }

    /// Đọc nửa chừng (D.1): lỗi giữa chừng, tệp cụt, tệp tăng ⇒ `Err`; đúng kích thước ⇒ `Ok` đủ byte.
    #[test]
    fn read_checked_never_returns_partial_content() {
        struct Flaky {
            sent: bool,
        }
        impl std::io::Read for Flaky {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if self.sent {
                    return Err(std::io::Error::other("đĩa lỗi giữa chừng"));
                }
                self.sent = true;
                buf[..5].copy_from_slice(b"hello");
                Ok(5)
            }
        }
        assert!(read_checked(Flaky { sent: false }, 10).is_err()); // lỗi sau 5/10 byte
        assert!(read_checked(Flaky { sent: false }, 5).is_err()); // lỗi SAU khi đã đủ byte kỳ vọng: vẫn là lỗi, không nuốt
        assert!(read_checked(&b"hello"[..], 10).is_err()); // tệp bị cắt: 5 < 10
        assert!(read_checked(&b"hello world"[..], 5).is_err()); // tệp tăng: 11 > 5
        assert_eq!(read_checked(&b"hello"[..], 5).expect("đủ"), b"hello");
        assert_eq!(read_checked(&b""[..], 0).expect("rỗng"), b"");
    }

    #[test]
    fn read_file_checked_rejects_directories_and_missing_files_and_reads_regular_files() {
        let d = std::env::temp_dir().join(format!("gtrt-fs-{}", std::process::id()));
        std::fs::create_dir_all(&d).expect("tạo");
        assert!(read_file_checked(d.to_str().expect("utf8")).is_err());
        let f = d.join("a.txt");
        std::fs::write(&f, b"abc").expect("ghi");
        assert_eq!(
            read_file_checked(f.to_str().expect("utf8")).expect("đọc"),
            b"abc"
        );
        assert!(read_file_checked(d.join("khong_co").to_str().expect("utf8")).is_err());
        #[cfg(windows)]
        assert!(
            read_file_checked("NUL").is_err(),
            "thiết bị NUL không phải tệp thường"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn spawn_runs_function_with_argument() {
        static GOT: AtomicU64 = AtomicU64::new(0);
        extern "C" fn f(a: *mut u8) {
            GOT.store(a as u64, Ordering::SeqCst);
        }
        unsafe { gritrt_spawn(f, 0x1234 as *mut u8) };
        for _ in 0..2000 {
            if GOT.load(Ordering::SeqCst) == 0x1234 {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("luồng spawn không chạy");
    }
}
