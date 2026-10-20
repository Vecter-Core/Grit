//! leak_check (M1): lex 1_000_000 chuỗi giả ngẫu nhiên trong MỘT tiến trình, đo RSS trước/sau, fail nếu tăng quá 10 MB.
//!
//! Đo RSS không dùng `unsafe` (crate cấm): Linux đọc `/proc/self/status`, macOS gọi `ps`, Windows gọi PowerShell
//! (`(Get-Process -Id PID).PrivateMemorySize64` — bộ nhớ riêng đã cấp, không bị OS thu nhỏ như Working Set nên không che rò rỉ). Nếu môi trường không đo được → test FAIL (fail-closed, không âm thầm qua).
use gt_lexer::lex;
#[cfg(any(target_os = "macos", windows))]
use std::process::Command;
use std::sync::Mutex;

/// Hai test đo cùng một tiến trình nên phải chạy tuần tự (test song song sẽ làm sai mốc đo).
static SERIAL: Mutex<()> = Mutex::new(());

/// RSS hiện tại của tiến trình (byte), hoặc `None` nếu không đo được.
fn rss_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let kb: u64 = status
            .lines()
            .find_map(|l| l.strip_prefix("VmRSS:"))?
            .split_whitespace()
            .next()?
            .parse()
            .ok()?;
        Some(kb * 1024)
    }
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()?;
        let kb: u64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
        Some(kb * 1024)
    }
    #[cfg(windows)]
    {
        let cmd = format!(
            "(Get-Process -Id {}).PrivateMemorySize64",
            std::process::id()
        );
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &cmd])
            .output()
            .ok()?;
        String::from_utf8_lossy(&out.stdout).trim().parse().ok()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        let _ = Command::new("true");
        None
    }
}

const MB: u64 = 1024 * 1024;

fn alphabet() -> Vec<char> {
    "ƒ≔μ←→⟳↓⊳⊲⋈∧∨¬⇒∀∃≤≥≠∈∅⚠⊢⎇⇝⌥↦⏎▢⊔▦⇲◈⊏⊐⊠⊡≡□■ \t\n\r\\'\"/*.-+<>=!&%:;,#(){}[]_0123456789abcxeEuUfFiIsS\u{b5}\u{3bc}\u{430}\u{301}\u{200b}\u{202e}\u{a0}é😀"
        .chars()
        .collect()
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn random_string(rng: &mut Rng, alphabet: &[char]) -> String {
    let len = (rng.next() % 41) as usize;
    (0..len)
        .map(|_| alphabet[(rng.next() % alphabet.len() as u64) as usize])
        .collect()
}

/// Kiểm tra chính phép đo: cấp phát và chạm ~64 MB phải làm RSS tăng rõ rệt (nếu không, leak_check vô nghĩa).
#[test]
fn rss_probe_sees_growth() {
    let _guard = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let before = rss_bytes()
        .expect("không đo được RSS trên môi trường này — leak_check không thể chạy (fail-closed)");
    let mut v = vec![0u8; 64 * 1024 * 1024];
    for i in (0..v.len()).step_by(4096) {
        v[i] = 1; // chạm từng trang để chắc chắn được commit
    }
    let during = rss_bytes().expect("rss");
    assert!(
        during >= before + 40 * MB,
        "phép đo RSS không thấy mức tăng 64 MB (trước {before}, sau {during})"
    );
    drop(v);
}

#[test]
fn leak_check() {
    let _guard = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let alphabet = alphabet();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    // khởi động: để allocator/OS ổn định bộ nhớ nền trước khi lấy mốc
    for _ in 0..20_000 {
        let _ = lex(&random_string(&mut rng, &alphabet));
    }
    let before = rss_bytes()
        .expect("không đo được RSS trên môi trường này — leak_check không thể chạy (fail-closed)");
    let mut tokens = 0usize;
    let mut errors = 0usize;
    for _ in 0..1_000_000 {
        let out = lex(&random_string(&mut rng, &alphabet));
        tokens += out.tokens.len();
        errors += out.errors.len();
    }
    let after = rss_bytes().expect("rss");
    println!(
        "leak_check: RSS trước = {before} B, sau = {after} B, token = {tokens}, lỗi = {errors}"
    );
    assert!(tokens > 1_000_000, "vòng lặp không thực sự chạy lexer");
    assert!(
        after <= before + 10 * MB,
        "RSS tăng {} B (> 10 MB) sau 1_000_000 lần lex: nghi rò rỉ bộ nhớ",
        after.saturating_sub(before)
    );
}
