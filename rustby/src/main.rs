use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if let Err(e) = rustby::run(&args) {
        eprintln!("\x1b[31merror:\x1b[0m {}", e);
        std::process::exit(1);
    }
}
