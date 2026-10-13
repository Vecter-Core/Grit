#![forbid(unsafe_code)]
fn main() {
    std::process::exit(rig::cli::main(std::env::args().skip(1).collect()));
}
