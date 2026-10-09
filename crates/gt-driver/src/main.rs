#![forbid(unsafe_code)]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(gt_driver::cli::main(args));
}
