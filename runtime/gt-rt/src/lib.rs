//! gt-rt: runtime tối thiểu cho chương trình do `gritc` sinh (ADR-029 §5). Chỉ x86_64-pc-windows-msvc ở Phase 1.
//!
//! Hợp đồng (ABI C), codegen gọi đúng các ký hiệu sau:
//! * `gritrt_start(main)` chạy `main` rồi trả 0 (mã thoát bình thường).
//! * `gritrt_abort(kind)` ghi `abort: <loại>` lên stderr rồi dừng tiến trình NGAY với mã **70** (D3: không unwinding, không cleanup).
//! * `gritrt_mutex_lock/unlock(lock)` — `lock` trỏ tới ô 8 byte KHỞI TẠO BẰNG 0 (SRWLOCK thật của Windows).
//! * `gritrt_spawn(f, arg)` tạo luồng HĐH thật chạy `f(arg)`; không có `join` (đúng ngữ nghĩa M8).
#![allow(unsafe_code)]

#[cfg(not(all(windows, target_arch = "x86_64")))]
compile_error!(
    "gt-rt Phase 1 chỉ hỗ trợ x86_64 Windows (ADR-029); target khác phải được thêm có chủ đích"
);

use std::io::Write;

/// Mã thoát của chương trình khi abort (bảng phân vùng D17: tách khỏi mã 0–3 của `gritc`).
pub const ABORT_EXIT_CODE: i32 = 70;

/// Loại abort; số thứ tự là hợp đồng với codegen; TÊN trùng `AssertKind::name` của MIR (đối chiếu với thông dịch).
pub const ABORT_KINDS: [&str; 16] = [
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

#[unsafe(no_mangle)]
pub extern "C" fn gritrt_start(main: extern "C" fn()) -> i32 {
    main();
    0
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

#[repr(C)]
struct SrwLock(*mut u8);

#[link(name = "kernel32")]
unsafe extern "system" {
    fn AcquireSRWLockExclusive(l: *mut SrwLock);
    fn ReleaseSRWLockExclusive(l: *mut SrwLock);
}

/// # Safety
/// `lock` trỏ tới ô 8 byte căn lề 8, khởi tạo 0 và sống suốt thời gian khoá (codegen đảm bảo: ô nằm trong `Mutex`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gritrt_mutex_lock(lock: *mut u8) {
    unsafe { AcquireSRWLockExclusive(lock.cast()) }
}

/// # Safety
/// Như `gritrt_mutex_lock`; luồng gọi đang giữ khoá (M8 đảm bảo `with` bao cặp lock/unlock).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gritrt_mutex_unlock(lock: *mut u8) {
    unsafe { ReleaseSRWLockExclusive(lock.cast()) }
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
    match std::fs::read(path) {
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

/// # Safety
/// `p` trỏ tới `n` byte đọc được.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gritrt_write_str(p: *const u8, n: u64) {
    let bytes = unsafe { std::slice::from_raw_parts(p, usize::try_from(n).unwrap_or(0)) };
    let mut o = std::io::stdout().lock();
    if o.write_all(bytes).and_then(|()| o.flush()).is_err() {
        gritrt_abort(14);
    }
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
