// Tham chiếu Rust cho tests/bench/fib.gt (n - 1, n - 2 có kiểm tràn).
fn fib(n: u32) -> u64 {
    if n < 2 {
        n as u64
    } else {
        fib(n.checked_sub(1).expect("overflow")).wrapping_add(fib(n.checked_sub(2).expect("overflow")))
    }
}
fn main() {
    println!("{}", fib(std::hint::black_box(35)));
}
