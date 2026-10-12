// Tham chiếu Rust cho tests/bench/loop_sum.gt (cùng ngữ nghĩa: `i + 1` có kiểm tràn, cộng `s` bao quanh).
fn sum_to(n: u32) -> u64 {
    let mut s = 0u64;
    let mut i = 0u32;
    while i < n {
        s = s.wrapping_add(i as u64);
        i = i.checked_add(1).expect("overflow");
    }
    s
}
fn main() {
    println!("{}", sum_to(std::hint::black_box(400_000_000)));
}
