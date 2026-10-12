// Tham chiếu Rust cho tests/bench/slice_sum.gt (xs[i] có kiểm biên, cộng bao quanh).
fn sum(xs: &[i64]) -> i64 {
    let mut s = 0i64;
    let mut i = 0usize;
    while i < xs.len() {
        s = s.wrapping_add(xs[i]);
        i = i.checked_add(1).expect("overflow");
    }
    s
}
fn main() {
    let a = [1i64; 1000];
    let mut t = 0i64;
    let mut k = 0u32;
    while k < std::hint::black_box(200_000) {
        t = t.wrapping_add(sum(std::hint::black_box(&a[..])));
        k += 1;
    }
    println!("{t}");
}
